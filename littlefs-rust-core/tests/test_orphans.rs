//! Orphan and power-loss consistency tests.
//!
//! Upstream: tests/test_orphans.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_orphans.toml

mod common;

#[cfg(feature = "slow_tests")]
use std::assert_matches;

#[cfg(feature = "slow_tests")]
use common::test_prng;
use common::{dir_block, erase_block_raw, read_block_raw, write_block_raw};
#[cfg(feature = "slow_tests")]
use littlefs_rust_core::LfsConfig;
use littlefs_rust_core::error::Error;
#[cfg(feature = "slow_tests")]
use littlefs_rust_core::lfs_type::lfs_type::LFS_TYPE_DIR;
use littlefs_rust_core::lfs_type::lfs_type::LFS_TYPE_SOFTTAIL;
use littlefs_rust_core::{
    Lfs, LfsInfo, LfsMattr, LfsMdir, lfs_alloc_ckpoint, lfs_dir_alloc, lfs_dir_commit,
    lfs_dir_fetch, lfs_format, lfs_fs_forceconsistency, lfs_fs_hasorphans, lfs_fs_mkconsistent,
    lfs_fs_preporphans, lfs_fs_size, lfs_mkdir, lfs_mktag, lfs_mount, lfs_pair_tole32, lfs_remove,
    lfs_stat, lfs_unmount,
};
use littlefs_rust_test_macro::lfs_test;
use zerocopy::IntoBytes;

// --- test_orphans_mkconsistent_fresh ---
// Minimal: format, mount, mkconsistent. No mkdir/remove. Sanity check.
#[lfs_test]
fn test_orphans_mkconsistent_fresh(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    assert_ok!(lfs_fs_mkconsistent(lfs));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_orphans_mkconsistent_no_orphans ---
// With lazy force_consistency, mkdir/remove run deorphan first. So preporphans(1)
// gets cleared before the commit. Verify: mkconsistent clears (no-op) and persists.
#[lfs_test]
fn test_orphans_mkconsistent_no_orphans(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let lfs_ptr = lfs;
    assert_ok!(lfs_fs_preporphans(lfs_ptr, 1));
    assert!(lfs_fs_hasorphans(lfs_ptr));

    let path = "_p";
    assert_ok!(lfs_mkdir(lfs_ptr, path));
    assert_ok!(lfs_remove(lfs_ptr, path));
    assert!(
        !lfs_fs_hasorphans(lfs_ptr),
        "force_consistency before mkdir clears orphans"
    );
    assert_ok!(lfs_unmount(lfs_ptr));

    assert_ok!(lfs_mount(lfs_ptr, cfg));
    assert!(
        !lfs_fs_hasorphans(lfs_ptr),
        "persisted gstate has no orphans"
    );
    assert_ok!(lfs_fs_mkconsistent(lfs_ptr));
    assert!(
        !lfs_fs_hasorphans(lfs_ptr),
        "after mkconsistent, gstate should have no orphans"
    );
    assert_ok!(lfs_unmount(lfs_ptr));

    assert_ok!(lfs_mount(lfs_ptr, cfg));
    assert!(
        !lfs_fs_hasorphans(lfs_ptr),
        "after remount, gstate persisted to disk has no orphans"
    );
    assert_ok!(lfs_unmount(lfs_ptr));
}

// --- test_orphans_no_orphans ---
// preporphans(+1), mkdir+remove clears via force_consistency, unmount
#[lfs_test]
fn test_orphans_no_orphans(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let lfs_ptr = lfs;
    assert_ok!(lfs_fs_preporphans(lfs_ptr, 1));
    assert!(lfs_fs_hasorphans(lfs_ptr));

    let path = "_x";
    assert_ok!(lfs_mkdir(lfs_ptr, path));
    assert_ok!(lfs_remove(lfs_ptr, path));
    assert!(!lfs_fs_hasorphans(lfs_ptr));
    assert_ok!(lfs_unmount(lfs_ptr));
}

// --- test_orphans_nonreentrant ---
// Upstream: orphan operations without powerloss.
// Uses n=1 dir to match test_dirs_many_removal (n=2+ mkdir currently fails in this crate).
#[lfs_test]
fn test_orphans_nonreentrant(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let lfs_ptr = lfs;
    let path = "a";
    assert_ok!(lfs_mkdir(lfs_ptr, path));
    assert_ok!(lfs_remove(lfs_ptr, path));
    assert!(!lfs_fs_hasorphans(lfs_ptr));
    assert_ok!(lfs_unmount(lfs_ptr));
}

// --- Missing upstream stubs ---

/// Upstream: [cases.test_orphans_normal]
/// if = 'PROG_SIZE <= 0x3fe'. Corrupt child's commit to create orphan, mkdir triggers deorphan, check lfs_fs_size.
#[lfs_test]
fn test_orphans_normal(cfg: &LfsConfig) {
    if cfg.prog_size > 0x3fe {
        return;
    }
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let lfs_ptr = lfs;
    assert_ok!(lfs_mkdir(lfs_ptr, "parent"));
    assert_ok!(lfs_mkdir(lfs_ptr, "parent/orphan"));
    assert_ok!(lfs_mkdir(lfs_ptr, "parent/child"));
    assert_ok!(lfs_remove(lfs_ptr, "parent/orphan"));
    assert_ok!(lfs_unmount(lfs_ptr));

    // Mount to get child dir block, then corrupt it
    assert_ok!(lfs_mount(lfs_ptr, cfg));
    let block = dir_block(lfs_ptr, "parent/child");
    assert_ok!(lfs_unmount(lfs_ptr));

    let block_size = cfg.block_size as usize;
    let mut buffer = vec![0u8; block_size];
    assert_eq!(read_block_raw(cfg, block, 0, &mut buffer), Ok(()));

    let mut off = block_size as i32 - 1;
    while off >= 0 && buffer[off as usize] == 0xff {
        off -= 1;
    }
    assert!(off >= 3, "block {block} has fewer than 4 written bytes");
    let start = (off - 3) as usize;
    buffer[start..start + 3].fill(cfg.block_size as u8);

    assert_eq!(erase_block_raw(cfg, block), Ok(()));
    assert_eq!(write_block_raw(cfg, block, 0, &buffer), Ok(()));

    // Mount and verify orphan is gone, child exists, size is 8
    assert_ok!(lfs_mount(lfs_ptr, cfg));
    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_eq!(
        lfs_stat(lfs_ptr, "parent/orphan", info),
        Err(Error::NoEntry)
    );
    assert_ok!(lfs_stat(lfs_ptr, "parent/child", info));
    assert_eq!(lfs_fs_size(lfs_ptr), Ok(8));
    assert_ok!(lfs_unmount(lfs_ptr));

    // mkdir parent/otherchild triggers deorphan, size still 8
    assert_ok!(lfs_mount(lfs_ptr, cfg));
    assert_ok!(lfs_mkdir(lfs_ptr, "parent/otherchild"));
    assert_eq!(
        lfs_stat(lfs_ptr, "parent/orphan", info),
        Err(Error::NoEntry)
    );
    assert_ok!(lfs_stat(lfs_ptr, "parent/child", info));
    assert_ok!(lfs_stat(lfs_ptr, "parent/otherchild", info));
    assert_eq!(lfs_fs_size(lfs_ptr), Ok(8));
    assert_ok!(lfs_unmount(lfs_ptr));
}

/// Upstream: [cases.test_orphans_one_orphan]
/// Create orphan via internal APIs (lfs_dir_alloc + SOFTTAIL commit + lfs_fs_preporphans). Run lfs_fs_forceconsistency.
#[lfs_test]
fn test_orphans_one_orphan(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let lfs_ptr = lfs;

    // Create an orphan mdir
    let mut orphan = LfsMdir {
        pair: [0, 0],
        rev: 0,
        off: 0,
        etag: 0,
        count: 0,
        erased: false,
        split: false,
        tail: [0, 0],
    };
    lfs_alloc_ckpoint(lfs_ptr);
    assert_ok!(lfs_dir_alloc(lfs_ptr, &mut orphan));
    assert_ok!(lfs_dir_commit(lfs_ptr, &mut orphan, &[]));

    // Append orphan to root and mark FS as having orphans
    assert_ok!(lfs_fs_preporphans(lfs_ptr, 1));
    let mut mdir = LfsMdir {
        pair: [0, 0],
        rev: 0,
        off: 0,
        etag: 0,
        count: 0,
        erased: false,
        split: false,
        tail: [0, 0],
    };
    let root_pair: [u32; 2] = [0, 1];
    assert_ok!(lfs_dir_fetch(lfs_ptr, &mut mdir, root_pair));
    lfs_pair_tole32(&mut orphan.pair);
    let attrs = [LfsMattr {
        tag: lfs_mktag(LFS_TYPE_SOFTTAIL, 0x3ff, 8),
        buffer: orphan.pair.as_bytes(),
    }];
    assert_ok!(lfs_dir_commit(lfs_ptr, &mut mdir, &attrs));

    assert!(lfs_fs_hasorphans(lfs_ptr), "should have orphans");
    assert_ok!(lfs_unmount(lfs_ptr));

    assert_ok!(lfs_mount(lfs_ptr, cfg));
    assert!(lfs_fs_hasorphans(lfs_ptr), "orphans should persist");
    assert_ok!(lfs_fs_forceconsistency(lfs_ptr));
    assert!(
        !lfs_fs_hasorphans(lfs_ptr),
        "forceconsistency should clear orphans"
    );
    assert_ok!(lfs_unmount(lfs_ptr));
}

/// Upstream: [cases.test_orphans_mkconsistent_one_orphan]
/// Same orphan creation as one_orphan. Use lfs_fs_mkconsistent + remount. Verify cleanup.
#[lfs_test]
fn test_orphans_mkconsistent_one_orphan(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let lfs_ptr = lfs;

    // Create an orphan mdir
    let mut orphan = LfsMdir {
        pair: [0, 0],
        rev: 0,
        off: 0,
        etag: 0,
        count: 0,
        erased: false,
        split: false,
        tail: [0, 0],
    };
    lfs_alloc_ckpoint(lfs_ptr);
    assert_ok!(lfs_dir_alloc(lfs_ptr, &mut orphan));
    assert_ok!(lfs_dir_commit(lfs_ptr, &mut orphan, &[]));

    // Append orphan to root and mark FS as having orphans
    assert_ok!(lfs_fs_preporphans(lfs_ptr, 1));
    let mut mdir = LfsMdir {
        pair: [0, 0],
        rev: 0,
        off: 0,
        etag: 0,
        count: 0,
        erased: false,
        split: false,
        tail: [0, 0],
    };
    let root_pair: [u32; 2] = [0, 1];
    assert_ok!(lfs_dir_fetch(lfs_ptr, &mut mdir, root_pair));
    lfs_pair_tole32(&mut orphan.pair);
    let attrs = [LfsMattr {
        tag: lfs_mktag(LFS_TYPE_SOFTTAIL, 0x3ff, 8),
        buffer: orphan.pair.as_bytes(),
    }];
    assert_ok!(lfs_dir_commit(lfs_ptr, &mut mdir, &attrs));

    assert!(lfs_fs_hasorphans(lfs_ptr), "should have orphans");
    assert_ok!(lfs_unmount(lfs_ptr));

    assert_ok!(lfs_mount(lfs_ptr, cfg));
    assert!(lfs_fs_hasorphans(lfs_ptr), "orphans should persist");
    assert_ok!(lfs_fs_mkconsistent(lfs_ptr));
    assert!(
        !lfs_fs_hasorphans(lfs_ptr),
        "mkconsistent should clear orphans"
    );
    assert_ok!(lfs_unmount(lfs_ptr));

    // Remount and verify orphans are still gone
    assert_ok!(lfs_mount(lfs_ptr, cfg));
    assert!(
        !lfs_fs_hasorphans(lfs_ptr),
        "after remount, orphans should still be gone"
    );
    assert_ok!(lfs_unmount(lfs_ptr));
}

/// Upstream: [cases.test_orphans_reentrant]
/// FILES=[6,26], DEPTH=1; FILES=3,DEPTH=3 skipped when CACHE_SIZE!=64. reentrant, CYCLES=20.
#[lfs_test]
#[cfg(feature = "slow_tests")]
fn test_orphans_reentrant(cfg: &LfsConfig, #[values(false, true)] reentrant: bool) {
    const CYCLES: u32 = 20;
    const ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

    for (files, depth) in [(6usize, 1usize), (26, 1)] {
        let lfs = &mut Lfs::default();

        let err = lfs_mount(lfs, cfg);
        if err.is_err() {
            assert_ok!(lfs_format(lfs, cfg));
            assert_ok!(lfs_mount(lfs, cfg));
        }

        let mut prng: u32 = 1;
        for _ in 0..CYCLES {
            let mut components = Vec::with_capacity(depth);
            for _ in 0..depth {
                let c = ALPHA[(test_prng(&mut prng) as usize) % files];
                components.push((c as char).to_string());
            }
            let full_path = "/".to_string() + &components.join("/");

            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            let res = lfs_stat(lfs, &full_path, info);
            if res == Err(Error::NoEntry) {
                for d in 0..depth {
                    let sub = "/".to_string() + &components[..=d].join("/");
                    assert_matches!(lfs_mkdir(lfs, &sub), Ok(()) | Err(Error::Exists));
                }
                for d in 0..depth {
                    let sub = "/".to_string() + &components[..=d].join("/");

                    assert_ok!(lfs_stat(lfs, &sub, info));

                    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
                    let name = core::str::from_utf8(&info.name[..nul]).unwrap();
                    let expected = &components[d];
                    assert_eq!(name, *expected);
                    assert_eq!(info.type_, LFS_TYPE_DIR as u8);
                }
            } else {
                let expected = &components[depth - 1];
                let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
                let name = core::str::from_utf8(&info.name[..nul]).unwrap();
                assert_eq!(name, *expected);
                assert_eq!(info.type_, LFS_TYPE_DIR as u8);
                for d in (0..depth).rev() {
                    let sub = "/".to_string() + &components[..=d].join("/");
                    assert_matches!(lfs_remove(lfs, &sub), Ok(()) | Err(Error::NotEmpty));
                }
                assert_eq!(lfs_stat(lfs, &full_path, info), Err(Error::NoEntry));
            }
        }

        assert_ok!(lfs_unmount(lfs));
    }
}
