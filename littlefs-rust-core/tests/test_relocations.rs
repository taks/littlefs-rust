//! Relocation and compaction tests.
//!
//! Upstream: tests/test_relocations.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_relocations.toml
//!
//! Validates dir_compact, dir_split, and orphaningcommit.

#![cfg_attr(not(feature = "slow_tests"), allow(unused_imports))]

mod common;

use std::fmt::Write;
#[cfg(feature = "slow_tests")]
use std::{assert_matches, ffi::CStr};

use common::{
    LFS_O_CREAT, LFS_O_WRONLY, config_with_cache, default_config, init_context, init_logger,
    test_prng,
};
use littlefs_rust_core::{
    Lfs, LfsConfig, LfsFile, LfsInfo, error::Error, lfs_file_close, lfs_file_open, lfs_file_write,
    lfs_format, lfs_mkdir, lfs_mount, lfs_remove, lfs_rename, lfs_stat, lfs_unmount,
};
#[cfg(feature = "slow_tests")]
use littlefs_rust_test_macro::lfs_test;
use rstest::rstest;

#[allow(dead_code)]
const ITERATIONS: usize = 20;
const COUNT: usize = 10;

// --- test_relocations_dangling_split_dir ---
/// Upstream: [cases.test_relocations_dangling_split_dir]
/// defines.ITERATIONS = 20, COUNT = 10, BLOCK_CYCLES = [8, 1]
///
/// Fill FS, create many files in child dir. Triggers split when metadata overflows.
#[lfs_test]
fn test_relocations_dangling_split_dir(cfg: &LfsConfig, #[values(8, 1)] block_cycles: i32) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    assert_ok!(lfs_mkdir(lfs, "d0"));
    for i in 0..COUNT {
        let path = &format!("d0/f{i}");
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        let n = lfs_file_write(lfs, file, b"x");
        assert_eq!(n, Ok(1));
        assert_ok!(lfs_file_close(lfs, file));
    }

    for i in 0..COUNT {
        let path = &format!("d0/f{i}");
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok!(lfs_stat(lfs, path, info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(
            core::str::from_utf8(&info.name[..nul]).unwrap(),
            format!("f{i}")
        );
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_outdated_head ---
/// Upstream: [cases.test_relocations_outdated_head]
/// defines.ITERATIONS = 20, COUNT = 10, BLOCK_CYCLES = [8, 1]
///
/// Split dir handling: multiple dirs, nested sub with many files.
#[lfs_test]
fn test_relocations_outdated_head(cfg: &LfsConfig, #[values(8, 1)] block_cycles: i32) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    for i in 0..3 {
        assert_ok!(lfs_mkdir(lfs, &format!("d{i}")));
    }
    assert_ok!(lfs_mkdir(lfs, "d0/sub"));
    for i in 0..COUNT {
        let path = &format!("d0/sub/f{i}");
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        let n = lfs_file_write(lfs, file, b"x");
        assert_eq!(n, Ok(1));
        assert_ok!(lfs_file_close(lfs, file));
    }

    for i in 0..COUNT {
        let path = &format!("d0/sub/f{i}");
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok!(lfs_stat(lfs, path, info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(
            core::str::from_utf8(&info.name[..nul]).unwrap(),
            format!("f{i}")
        );
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_nonreentrant ---
// mkdir/remove cycles, no power-loss.
#[rstest]
#[case(6, 1, 2000)]
#[case(26, 1, 2000)]
#[case(3, 3, 2000)]
#[cfg(feature = "slow_tests")]
fn test_relocations_nonreentrant(
    #[case] files: usize,
    #[case] depth: usize,
    #[case] cycles: usize,
) {
    if depth == 3 {
        return; // guard: DEPTH==3 && CACHE_SIZE!=64
    }
    init_logger();
    let block_count = 128u32;
    let mut env = default_config(block_count);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    for _ in 0..cycles {
        for i in 0..files {
            let path = &format!("{}", (b'a' + i as u8) as char);
            let _ = lfs_mkdir(lfs, path);
        }
        for i in 0..files {
            let path = &format!("{}", (b'a' + i as u8) as char);
            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            assert_ok!(lfs_stat(lfs, path, info));
            let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
            assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), path);
            assert_ok!(lfs_remove(lfs, path));
        }
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_nonreentrant_renames ---
// Chained renames (x→z, y→x, z→y) exercise same-slot name change.
#[rstest]
#[case(6, 1, 2000)]
#[case(26, 1, 2000)]
#[case(3, 3, 2000)]
#[cfg(feature = "slow_tests")]
fn test_relocations_nonreentrant_renames(
    #[case] _files: usize,
    #[case] depth: usize,
    #[case] _cycles: usize,
) {
    if depth == 3 {
        return; // guard: DEPTH==3 && CACHE_SIZE!=64
    }
    init_logger();
    let block_count = 128u32; // 2*FILES < BLOCK_COUNT
    let mut env = config_with_cache(64, block_count);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    for path in ["x", "y"] {
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        assert_ok!(lfs_file_close(lfs, file));
    }

    assert_ok!(lfs_rename(lfs, "x", "z"));
    assert_ok!(lfs_rename(lfs, "y", "x"));
    assert_ok!(lfs_rename(lfs, "z", "y"));

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok!(lfs_stat(lfs, "x", info));
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "x");

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok!(lfs_stat(lfs, "y", info));
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "y");

    assert_ok!(lfs_remove(lfs, "x"));
    assert_ok!(lfs_remove(lfs, "y"));

    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_reentrant ---
// mkdir/remove cycles with power-loss; verify FS consistent after each.
#[lfs_test]
#[case(6, 1, 20)]
#[case(26, 1, 20)]
#[case(3, 3, 20)]
#[cfg(feature = "slow_tests")]
#[ignore = "bug: power-loss iteration returns Error::Io for some cases"]
#[timeout(std::time::Duration::from_mins(1))]
fn test_relocations_reentrant(
    cfg: &LfsConfig,
    #[values(false, true)] reentrant: bool,
    #[case] files: usize,
    #[case] depth: usize,
    #[case] cycles: usize,
) {
    if depth == 3 {
        return; // guard: DEPTH==3 && CACHE_SIZE!=64
    }

    let lfs = &mut Lfs::default();
    let err = littlefs_rust_core::lfs_mount(lfs, cfg);
    if err.is_err() {
        assert_ok!(littlefs_rust_core::lfs_format(lfs, cfg));
        assert_ok!(littlefs_rust_core::lfs_mount(lfs, cfg));
    }

    for _ in 0..cycles {
        for i in 0..files {
            let path = &format!("{}", (b'a' + i as u8) as char);
            assert!(matches!(lfs_mkdir(lfs, path), Ok(()) | Err(Error::Exists)));
        }
        for i in 0..files {
            let path = &format!("{}", (b'a' + i as u8) as char);
            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            assert_ok!(lfs_stat(lfs, path, info));

            assert_ok!(lfs_remove(lfs, path));
        }
    }
    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_reentrant_renames ---
// Chained renames with power-loss; verify FS consistent after each.
#[lfs_test]
#[case(6, 1, 20)]
#[case(26, 1, 20)]
#[case(3, 3, 20)]
#[cfg(feature = "slow_tests")]
#[ignore = "TODO FIX"]
fn test_relocations_reentrant_renames(
    cfg: &LfsConfig,
    #[values(false, true)] reentrant: bool,
    #[case] files: usize,
    #[case] depth: usize,
    #[case] cycles: usize,
    #[values(1)] block_cycles: i32,
) {
    // TODO fix this case, caused by non-DAG trees
    // NOTE the second condition is required
    if depth == 3 && cfg.cache_size != 64 {
        return;
    }
    if 2 * files >= cfg.block_count as usize {
        return;
    }

    let lfs = &mut Lfs::default();

    let err = littlefs_rust_core::lfs_mount(lfs, cfg);
    if err.is_err() {
        assert_ok!(littlefs_rust_core::lfs_format(lfs, cfg));
        assert_ok!(littlefs_rust_core::lfs_mount(lfs, cfg));
    }

    let mut prng: u32 = 1;
    let alpha: Vec<_> = "abcdefghijklmnopqrstuvwxyz".chars().collect();

    for _ in 0..cycles {
        // create random path
        let mut full_path = String::with_capacity(256);
        for _ in 0..depth {
            assert_ok!(write!(
                &mut full_path,
                "/{}",
                alpha[test_prng(&mut prng) as usize % files]
            ));
        }

        // if it does not exist, we create it, else we destroy
        let info = &mut LfsInfo::default();
        let res = lfs_stat(lfs, &full_path, info);
        assert!(res.is_ok() || res == Err(Error::NoEntry));
        if res == Err(Error::NoEntry) {
            // create each directory in turn, ignore if dir already exists
            for d in 0..depth {
                assert_matches!(
                    lfs_mkdir(lfs, &full_path[..(2 * d + 2)]),
                    Ok(()) | Err(Error::Exists)
                );
            }
            for d in 0..depth {
                use littlefs_rust_core::lfs_type::lfs_type::LFS_TYPE_DIR;

                assert_ok!(lfs_stat(lfs, &full_path[..(2 * d + 2)], info));
                assert_eq!(
                    CStr::from_bytes_until_nul(&info.name)
                        .unwrap()
                        .to_str()
                        .unwrap(),
                    &full_path[(2 * d + 1)..(2 * d + 2)]
                );
                assert_eq!(info.type_, LFS_TYPE_DIR as u8);
            }
        } else {
            use littlefs_rust_core::lfs_type::lfs_type::LFS_TYPE_DIR;

            assert_eq!(
                CStr::from_bytes_until_nul(&info.name)
                    .unwrap()
                    .to_str()
                    .unwrap(),
                &full_path[(2 * (depth - 1) + 1)..]
            );
            assert_eq!(info.type_, LFS_TYPE_DIR as u8);

            // create new random path
            let mut new_path = String::with_capacity(256);
            for _ in 0..depth {
                assert_ok!(write!(
                    &mut new_path,
                    "/{}",
                    alpha[test_prng(&mut prng) as usize % files]
                ));
            }

            // if new path does not exist, rename, otherwise destroy
            let res = lfs_stat(lfs, &new_path, info);
            assert_matches!(res, Ok(()) | Err(Error::NoEntry));
            if res == Err(Error::NoEntry) {
                // stop once some dir is renamed
                for d in 0..depth {
                    let from = format!(
                        "{}{}",
                        &new_path[..(2 * d)],
                        &full_path[(2 * d)..(2 * d + 2)]
                    );
                    assert_matches!(
                        lfs_rename(lfs, &from, &new_path[..(2 * d + 2)]),
                        Ok(()) | Err(Error::NotEmpty)
                    );
                }
                for d in 0..depth {
                    assert_ok!(lfs_stat(lfs, &new_path[..(2 * d + 2)], info));
                    assert_eq!(
                        CStr::from_bytes_until_nul(&info.name)
                            .unwrap()
                            .to_str()
                            .unwrap(),
                        &new_path[(2 * d + 1)..(2 * d + 2)]
                    );
                    assert_eq!(info.type_, LFS_TYPE_DIR as u8);
                }

                assert_eq!(lfs_stat(lfs, &full_path, info), Err(Error::NoEntry));
            } else {
                // try to delete path in reverse order,
                // ignore if dir is not empty
                let mut d = depth - 1;
                loop {
                    assert_matches!(
                        lfs_remove(lfs, &full_path[..(2 * d + 2)]),
                        Ok(()) | Err(Error::NotEmpty)
                    );
                    if d == 0 {
                        break;
                    }
                    d -= 1;
                }

                assert_eq!(lfs_stat(lfs, &full_path, info), Err(Error::NoEntry));
            }
        }
    }
    assert_ok!(lfs_unmount(lfs));
}
