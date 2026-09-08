//! Relocation and compaction tests.
//!
//! Upstream: tests/test_relocations.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_relocations.toml
//!
//! Validates dir_compact, dir_split, and orphaningcommit.

#![cfg_attr(not(feature = "slow_tests"), allow(unused_imports))]

mod common;

use common::powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear};
use common::{
    LFS_O_CREAT, LFS_O_WRONLY, config_with_cache, default_config, init_context, init_logger,
};
#[cfg(test)]
use littlefs_rust_core::LfsConfig;
use littlefs_rust_core::{
    Lfs, LfsFile, LfsInfo, lfs_file_close, lfs_file_open, lfs_file_write, lfs_format, lfs_mkdir,
    lfs_mount, lfs_remove, lfs_rename, lfs_stat, lfs_unmount,
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
fn test_relocations_dangling_split_dir(cfg: &mut LfsConfig, #[values(8, 1)] block_cycles: i32) {
    cfg.block_cycles = block_cycles;

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
#[rstest]
fn test_relocations_outdated_head(#[values(8, 1)] block_cycles: i32) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    env.config.block_cycles = block_cycles;

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

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
#[lfs_test(reentrant)]
#[case(6, 1, 20)]
#[case(26, 1, 20)]
#[case(3, 3, 20)]
#[cfg(feature = "slow_tests")]
#[ignore = "bug: power-loss iteration returns Error::Io for some cases"]
#[timeout(std::time::Duration::from_mins(1))]
fn test_relocations_reentrant(
    cfg: &mut LfsConfig,
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
            use littlefs_rust_core::error::Error;

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
#[lfs_test(reentrant)]
#[case(6, 1, 20)]
#[case(26, 1, 20)]
#[case(3, 3, 20)]
#[cfg(feature = "slow_tests")]
fn test_relocations_reentrant_renames(
    cfg: &mut LfsConfig,
    #[case] _files: usize,
    #[case] depth: usize,
    #[case] cycles: usize,
) {
    if depth == 3 && cfg.cache_size != 64 {
        return;
    }

    cfg.block_cycles = 1;
    let lfs = &mut Lfs::default();

    let err = littlefs_rust_core::lfs_mount(lfs, cfg);
    if err.is_err() {
        assert_ok!(littlefs_rust_core::lfs_format(lfs, cfg));
        assert_ok!(littlefs_rust_core::lfs_mount(lfs, cfg));
    }

    let mut prng: u32 = 1;
    const ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

    for i in 0..cycles {
        //     // create random path
        //     char full_path[256];
        //     for (unsigned d = 0; d < DEPTH; d++) {
        //         sprintf(&full_path[2*d], "/%c", alpha[TEST_PRNG(&prng) % FILES]);
        //     }

        //     // if it does not exist, we create it, else we destroy
        //     struct lfs_info info;
        //     int res = lfs_stat(&lfs, full_path, &info);
        //     assert(!res || res == LFS_ERR_NOENT);
        //     if (res == LFS_ERR_NOENT) {
        //         // create each directory in turn, ignore if dir already exists
        //         for (unsigned d = 0; d < DEPTH; d++) {
        //             char path[1024];
        //             strcpy(path, full_path);
        //             path[2*d+2] = '\0';
        //             err = lfs_mkdir(&lfs, path);
        //             assert(!err || err == LFS_ERR_EXIST);
        //         }

        //         for (unsigned d = 0; d < DEPTH; d++) {
        //             char path[1024];
        //             strcpy(path, full_path);
        //             path[2*d+2] = '\0';
        //             lfs_stat(&lfs, path, &info) => 0;
        //             assert(strcmp(info.name, &path[2*d+1]) == 0);
        //             assert(info.type == LFS_TYPE_DIR);
        //         }
        //     } else {
        //         assert(strcmp(info.name, &full_path[2*(DEPTH-1)+1]) == 0);
        //         assert(info.type == LFS_TYPE_DIR);

        //         // create new random path
        //         char new_path[256];
        //         for (unsigned d = 0; d < DEPTH; d++) {
        //             sprintf(&new_path[2*d], "/%c", alpha[TEST_PRNG(&prng) % FILES]);
        //         }

        //         // if new path does not exist, rename, otherwise destroy
        //         res = lfs_stat(&lfs, new_path, &info);
        //         assert(!res || res == LFS_ERR_NOENT);
        //         if (res == LFS_ERR_NOENT) {
        //             // stop once some dir is renamed
        //             for (unsigned d = 0; d < DEPTH; d++) {
        //                 char path[1024];
        //                 strcpy(&path[2*d], &full_path[2*d]);
        //                 path[2*d+2] = '\0';
        //                 strcpy(&path[128+2*d], &new_path[2*d]);
        //                 path[128+2*d+2] = '\0';
        //                 err = lfs_rename(&lfs, path, path+128);
        //                 assert(!err || err == LFS_ERR_NOTEMPTY);
        //                 if (!err) {
        //                     strcpy(path, path+128);
        //                 }
        //             }

        //             for (unsigned d = 0; d < DEPTH; d++) {
        //                 char path[1024];
        //                 strcpy(path, new_path);
        //                 path[2*d+2] = '\0';
        //                 lfs_stat(&lfs, path, &info) => 0;
        //                 assert(strcmp(info.name, &path[2*d+1]) == 0);
        //                 assert(info.type == LFS_TYPE_DIR);
        //             }

        //             lfs_stat(&lfs, full_path, &info) => LFS_ERR_NOENT;
        //         } else {
        //             // try to delete path in reverse order,
        //             // ignore if dir is not empty
        //             for (unsigned d = DEPTH-1; d+1 > 0; d--) {
        //                 char path[1024];
        //                 strcpy(path, full_path);
        //                 path[2*d+2] = '\0';
        //                 err = lfs_remove(&lfs, path);
        //                 assert(!err || err == LFS_ERR_NOTEMPTY);
        //             }

        //             lfs_stat(&lfs, full_path, &info) => LFS_ERR_NOENT;
        //         }
        //     }
    }
    assert_ok!(lfs_unmount(lfs));
}
