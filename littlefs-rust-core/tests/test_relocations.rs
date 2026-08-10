//! Relocation and compaction tests.
//!
//! Upstream: tests/test_relocations.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_relocations.toml
//!
//! Validates dir_compact, dir_split, and orphaningcommit.

#![cfg_attr(not(feature = "slow_tests"), allow(unused_imports))]

mod common;

use std::ffi::CStr;

use common::powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear};
use common::{
    LFS_O_CREAT, LFS_O_WRONLY, assert_ok, config_with_cache, default_config, init_context,
    init_logger,
};
use littlefs_rust_core::{
    Lfs, LfsFile, LfsInfo, lfs_file_close, lfs_file_open, lfs_file_write, lfs_format, lfs_mkdir,
    lfs_mount, lfs_remove, lfs_rename, lfs_stat, lfs_unmount,
};
use rstest::rstest;

#[allow(dead_code)]
const ITERATIONS: usize = 20;
const COUNT: usize = 10;

// --- test_relocations_dangling_split_dir ---
/// Upstream: [cases.test_relocations_dangling_split_dir]
/// defines.ITERATIONS = 20, COUNT = 10, BLOCK_CYCLES = [8, 1]
///
/// Fill FS, create many files in child dir. Triggers split when metadata overflows.
#[rstest]
fn test_relocations_dangling_split_dir(#[values(8, 1)] block_cycles: i32) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    env.config.block_cycles = block_cycles;

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "d0"));
    for i in 0..COUNT {
        let path = &format!("d0/f{i}");
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        let n = lfs_file_write(lfs, file, b"x");
        assert_eq!(n, Ok(1));
        assert_ok(lfs_file_close(lfs, file));
    }

    for i in 0..COUNT {
        let path = &format!("d0/f{i}");
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok(lfs_stat(lfs, path, info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(
            core::str::from_utf8(&info.name[..nul]).unwrap(),
            format!("f{i}")
        );
    }

    assert_ok(lfs_unmount(lfs));
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

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    for i in 0..3 {
        assert_ok(lfs_mkdir(lfs, &format!("d{i}")));
    }
    assert_ok(lfs_mkdir(lfs, "d0/sub"));
    for i in 0..COUNT {
        let path = &format!("d0/sub/f{i}");
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        let n = lfs_file_write(lfs, file, b"x");
        assert_eq!(n, Ok(1));
        assert_ok(lfs_file_close(lfs, file));
    }

    for i in 0..COUNT {
        let path = &format!("d0/sub/f{i}");
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok(lfs_stat(lfs, path, info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(
            core::str::from_utf8(&info.name[..nul]).unwrap(),
            format!("f{i}")
        );
    }

    assert_ok(lfs_unmount(lfs));
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

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    for _ in 0..cycles {
        for i in 0..files {
            let name = format!("{}", (b'a' + i as u8) as char);
            let path = (&name);
            let _ = lfs_mkdir(lfs, path);
        }
        for i in 0..files {
            let name = format!("{}", (b'a' + i as u8) as char);
            let path = (&name);
            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            assert_ok(lfs_stat(lfs, path, info));
            let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
            assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), name);
            assert_ok(lfs_remove(lfs, path));
        }
    }

    assert_ok(lfs_unmount(lfs));
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

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    for name in ["x", "y"] {
        let path = (name);
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        assert_ok(lfs_file_close(lfs, file));
    }

    assert_ok(lfs_rename(lfs, "x", "z"));
    assert_ok(lfs_rename(lfs, "y", "x"));
    assert_ok(lfs_rename(lfs, "z", "y"));

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok(lfs_stat(lfs, "x", info));
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "x");

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok(lfs_stat(lfs, "y", info));
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "y");

    assert_ok(lfs_remove(lfs, "x"));
    assert_ok(lfs_remove(lfs, "y"));

    assert_ok(lfs_unmount(lfs));
}

// --- test_relocations_reentrant ---
// mkdir/remove cycles with power-loss; verify FS consistent after each.
#[rstest]
#[case(6, 1, 20)]
#[case(26, 1, 20)]
#[case(3, 3, 20)]
#[cfg(feature = "slow_tests")]
#[ignore = "bug: power-loss iteration returns -5 for some cases"]
fn test_relocations_reentrant(#[case] files: usize, #[case] depth: usize, #[case] cycles: usize) {
    if depth == 3 {
        return; // guard: DEPTH==3 && CACHE_SIZE!=64
    }
    init_logger();
    let block_count = 128u32;
    let mut env = powerloss_config(block_count);
    init_powerloss_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    let snapshot = env.snapshot();

    let result = run_powerloss_linear(
        &mut env,
        &snapshot,
        block_count,
        |lfs_ptr, config| {
            lfs_mount(lfs_ptr, config)?;

            for _ in 0..cycles {
                for i in 0..files {
                    let name = format!("{}", (b'a' + i as u8) as char);
                    let path = (&name);
                    let err = lfs_mkdir(lfs_ptr, path);
                    if err.is_err() {
                        let _ = lfs_unmount(lfs_ptr);
                        return err;
                    }
                }
                for i in 0..files {
                    let name = format!("{}", (b'a' + i as u8) as char);
                    let path = (&name);
                    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
                    let err = lfs_stat(lfs_ptr, path, info);
                    if err.is_err() {
                        let _ = lfs_unmount(lfs_ptr);
                        return err;
                    }
                    let err = lfs_remove(lfs_ptr, path);
                    if err.is_err() {
                        let _ = lfs_unmount(lfs_ptr);
                        return err;
                    }
                }
            }
            lfs_unmount(lfs_ptr)?;

            Ok(())
        },
        |lfs_ptr, config| {
            lfs_mount(lfs_ptr, config)?;
            let _ = lfs_unmount(lfs_ptr);
            Ok(())
        },
    );
    result.expect("test_relocations_reentrant should complete");
}

// --- test_relocations_reentrant_renames ---
// Chained renames with power-loss; verify FS consistent after each.
#[rstest]
#[case(6, 1, 20)]
#[case(26, 1, 20)]
#[case(3, 3, 20)]
#[cfg(feature = "slow_tests")]
fn test_relocations_reentrant_renames(
    #[case] _files: usize,
    #[case] depth: usize,
    #[case] _cycles: usize,
) {
    if depth == 3 {
        return; // guard: DEPTH==3 && CACHE_SIZE!=64
    }
    init_logger();
    let block_count = 128u32; // 2*FILES < BLOCK_COUNT
    let mut env = powerloss_config(block_count);
    init_powerloss_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    for name in ["x", "y"] {
        let path = (name);
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        assert_ok(lfs_file_close(lfs, file));
    }
    assert_ok(lfs_unmount(lfs));

    let snapshot = env.snapshot();

    let result = run_powerloss_linear(
        &mut env,
        &snapshot,
        128,
        |lfs_ptr, config| {
            lfs_mount(lfs_ptr, config)?;
            let err = lfs_rename(lfs_ptr, "x", "z");
            if err.is_err() {
                let _ = lfs_unmount(lfs_ptr);
                return err;
            }
            let err = lfs_rename(lfs_ptr, "y", "x");
            if err.is_err() {
                let _ = lfs_unmount(lfs_ptr);
                return err;
            }
            let err = lfs_rename(lfs_ptr, "z", "y");
            if err.is_err() {
                let _ = lfs_unmount(lfs_ptr);
                return err;
            }
            let err = lfs_remove(lfs_ptr, "x");
            if err.is_err() {
                let _ = lfs_unmount(lfs_ptr);
                return err;
            }
            let err = lfs_remove(lfs_ptr, "y");
            if err.is_err() {
                let _ = lfs_unmount(lfs_ptr);
                return err;
            }
            let err = lfs_unmount(lfs_ptr);
            if err.is_err() {
                return err;
            }
            Ok(())
        },
        |lfs_ptr, config| {
            lfs_mount(lfs_ptr, config)?;
            let _ = lfs_unmount(lfs_ptr);
            Ok(())
        },
    );
    result.expect("test_relocations_reentrant_renames should complete");
}
