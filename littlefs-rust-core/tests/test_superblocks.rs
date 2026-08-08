//! Superblock and format/mount tests.
//!
//! Upstream: tests/test_superblocks.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_superblocks.toml

mod common;

#[cfg(feature = "slow_tests")]
use common::powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear};
use common::{
    LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY, assert_err, assert_ok,
    assert_superblock_magic, clone_config_with_block_count, default_config, init_context,
    path_bytes,
};
use littlefs_rust_core::error::Error;
use littlefs_rust_core::lfs_type::lfs_type::LFS_TYPE_REG;
use littlefs_rust_core::{
    Lfs, LfsFile, LfsFsinfo, LfsInfo, lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write,
    lfs_format, lfs_fs_grow, lfs_fs_stat, lfs_mount, lfs_remove, lfs_stat, lfs_unmount,
};
use rstest::rstest;

use crate::common::init_logger;

// --- test_superblocks_format ---
// Upstream: lfs_format(&lfs, cfg) => 0
#[test]
fn test_superblocks_format() {
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    let err = lfs_format(lfs, &env.config);
    assert_ok(err);
}

// --- test_superblocks_mount ---
// Upstream: format, mount, unmount
#[test]
fn test_superblocks_mount() {
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_unmount(lfs));
}

// --- test_superblocks_magic ---
// Upstream: format, then raw read to verify "littlefs" at MAGIC_OFFSET in both blocks.
#[test]
fn test_superblocks_magic() {
    common::init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    assert_superblock_magic(&env.config);
}

// --- test_traverse_attrs_callback_order ---
// Unit test (in integration harness): traverse with tmask=0 passes SUPERBLOCK correctly.
#[test]
fn test_traverse_attrs_callback_order() {
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    let mut out = littlefs_rust_core::TraverseTestOut::default();

    assert_ok(unsafe {
        littlefs_rust_core::test_traverse_format_attrs(lfs, &env.config, &mut out as *mut _)
    });

    assert_eq!(out.call_count, 3);
    assert_eq!(out.tags[1], 0x0ff, "second callback should be SUPERBLOCK");
    assert_eq!(out.first_bytes[1], b'l');
}

// --- test_traverse_filter_gets_superblock_after_push ---
// Unit test: traverse with tmask (compact-style) triggers push; callback receives SUPERBLOCK with 'l'.
#[test]
fn test_traverse_filter_gets_superblock_after_push() {
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    let mut out = littlefs_rust_core::TraverseTestOut::default();

    assert_ok(unsafe {
        littlefs_rust_core::test_traverse_filter_gets_superblock_after_push(
            lfs,
            &env.config,
            &mut out as *mut _,
        )
    });

    let has_superblock = out.tags[..out.call_count as usize].contains(&0x0ff);
    assert!(
        has_superblock,
        "callback should receive SUPERBLOCK (type3=0x0ff)"
    );
    let superblock_idx = out.tags[..out.call_count as usize]
        .iter()
        .position(|&t| t == 0x0ff)
        .unwrap();
    assert_eq!(
        out.first_bytes[superblock_idx], b'l',
        "SUPERBLOCK buffer first byte should be 'l'"
    );
}

// --- test_superblocks_invalid_mount ---
// Upstream: mount on blank device => LFS_ERR_CORRUPT
#[test]
fn test_superblocks_invalid_mount() {
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    let err = lfs_mount(lfs, &env.config);
    assert_err(Error::Corrupt, err);
}

// --- test_superblocks_stat ---
// Upstream: fs_stat after format/mount returns correct values
#[test]
fn test_superblocks_stat() {
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, env.config.block_size);
    assert_eq!(fsinfo.block_count, env.config.block_count);
    assert_eq!(fsinfo.disk_version, 0x0002_0001);
    assert_eq!(fsinfo.name_max, 255);
    assert_eq!(fsinfo.file_max, 2_147_483_647);
    assert_eq!(fsinfo.attr_max, 1022);
}

// --- Missing upstream stubs ---

/// Upstream: [cases.test_superblocks_mount_unknown_block_count]
/// Mount with block_count=0; verify lfs.block_count is set from superblock.
#[test]
fn test_superblocks_mount_unknown_block_count() {
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    let cfg0 = clone_config_with_block_count(&env, 0);
    assert_ok(lfs_mount(lfs, &cfg0.config));
    assert_eq!(
        lfs.block_count, 128,
        "lfs.block_count should match format config"
    );
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_reentrant_format]
/// reentrant = true, POWERLOSS_BEHAVIOR = [NOOP, OOO]. Format under power-loss, then mount.
#[test]
#[cfg(feature = "slow_tests")]
fn test_superblocks_reentrant_format() {
    let mut env = powerloss_config(128);
    init_powerloss_context(&mut env);
    let snapshot = env.snapshot();

    let result = run_powerloss_linear(
        &mut env,
        &snapshot,
        500,
        |lfs_ptr, config| {
            let err = lfs_mount(lfs_ptr, config);
            if err.is_err() {
                lfs_format(lfs_ptr, config)?;

                lfs_mount(lfs_ptr, config)?;
            }
            lfs_unmount(lfs_ptr)?;
            Ok(())
        },
        |_, _| Ok(()),
    );
    result.expect("test_superblocks_reentrant_format should complete");
}

/// Upstream: [cases.test_superblocks_stat_tweaked]
/// Format with name_max=63, file_max=65535, attr_max=512; mount with default; verify fsinfo.
#[test]
fn test_superblocks_stat_tweaked() {
    let mut env = default_config(128);
    init_context(&mut env);
    env.config.name_max = 63;
    env.config.file_max = 65535;
    env.config.attr_max = 512;

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    env.config.name_max = 255;
    env.config.file_max = 2_147_483_647;
    env.config.attr_max = 1022;
    assert_ok(lfs_mount(lfs, &env.config));

    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.name_max, 63);
    assert_eq!(fsinfo.file_max, 65535);
    assert_eq!(fsinfo.attr_max, 512);
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_expand]
/// Create/remove dummy file N times; verify superblock survives compaction.
#[test]
fn test_superblocks_expand() {
    for &block_cycles in &[32i32, 33, 1] {
        for &n in &[10u32, 100, 1000] {
            let mut env = default_config(128);
            init_context(&mut env);
            env.config.block_cycles = block_cycles;

            let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
            assert_ok(lfs_format(lfs, &env.config));
            assert_ok(lfs_mount(lfs, &env.config));

            let dummy = c"dummy";
            for _ in 0..n {
                let file =
                    &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
                assert_ok(lfs_file_open(
                    lfs,
                    file,
                    dummy,
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ));
                assert_ok(lfs_file_close(lfs, file));
                let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
                assert_ok(lfs_stat(lfs, dummy, info));
                assert_eq!(info.type_, LFS_TYPE_REG as u8);
                assert_ok(lfs_remove(lfs, dummy));
            }
            assert_ok(lfs_unmount(lfs));

            assert_ok(lfs_mount(lfs, &env.config));
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                dummy,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            assert_ok(lfs_stat(lfs, dummy, info));
            assert_eq!(info.type_, LFS_TYPE_REG as u8);
            assert_ok(lfs_unmount(lfs));
        }
    }
}

/// Upstream: [cases.test_superblocks_magic_expand]
/// Same as expand + magic check after.
#[test]
fn test_superblocks_magic_expand() {
    common::init_logger();
    for &block_cycles in &[32i32, 33, 1] {
        for &n in &[10u32, 100, 1000] {
            let mut env = default_config(128);
            init_context(&mut env);
            env.config.block_cycles = block_cycles;

            let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
            assert_ok(lfs_format(lfs, &env.config));
            assert_ok(lfs_mount(lfs, &env.config));

            let dummy = c"dummy";
            for _ in 0..n {
                let file =
                    &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
                assert_ok(lfs_file_open(
                    lfs,
                    file,
                    dummy,
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ));
                assert_ok(lfs_file_close(lfs, file));
                let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
                assert_ok(lfs_stat(lfs, dummy, info));
                assert_eq!(info.type_, LFS_TYPE_REG as u8);
                assert_ok(lfs_remove(lfs, dummy));
            }
            assert_ok(lfs_unmount(lfs));

            assert_superblock_magic(&env.config);
        }
    }
}

/// Upstream: [cases.test_superblocks_expand_power_cycle]
/// Same as expand but unmount/remount after each iteration.
#[test]
fn test_superblocks_expand_power_cycle() {
    init_logger();

    for &block_cycles in &[32i32, 33, 1] {
        for &n in &[10u32, 100, 1000] {
            let mut env = default_config(128);
            init_context(&mut env);
            env.config.block_cycles = block_cycles;

            let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
            assert_ok(lfs_format(lfs, &env.config));

            let dummy = c"dummy";
            for i in 0..n {
                assert_ok(lfs_mount(lfs, &env.config));
                let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
                let err = lfs_stat(lfs, dummy, info);
                assert!(
                    err.is_ok() || (err == Err(Error::NoEntry) && i == 0),
                    "stat dummy: err={err:?} i={i}"
                );
                if err.is_ok() {
                    assert_eq!(info.type_, LFS_TYPE_REG as u8);
                    assert_ok(lfs_remove(lfs, dummy));
                }

                let file =
                    &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
                assert_ok(lfs_file_open(
                    lfs,
                    file,
                    dummy,
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ));
                assert_ok(lfs_file_close(lfs, file));
                let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
                assert_ok(lfs_stat(lfs, dummy, info));
                assert_eq!(info.type_, LFS_TYPE_REG as u8);
                assert_ok(lfs_unmount(lfs));
            }

            assert_ok(lfs_mount(lfs, &env.config));
            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            assert_ok(lfs_stat(lfs, dummy, info));
            assert_eq!(info.type_, LFS_TYPE_REG as u8);
            assert_ok(lfs_unmount(lfs));
        }
    }
}

/// Upstream: [cases.test_superblocks_reentrant_expand]
/// BLOCK_CYCLES = [2, 1], N = 24, reentrant, POWERLOSS_BEHAVIOR = [NOOP, OOO]
#[test]
#[cfg(feature = "slow_tests")]
fn test_superblocks_reentrant_expand() {
    const N: u32 = 24;
    for &block_cycles in &[2i32, 1] {
        let mut env = powerloss_config(128);
        init_powerloss_context(&mut env);
        env.config.block_cycles = block_cycles;

        let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
        assert_ok(lfs_format(lfs, &env.config));
        assert_ok(lfs_mount(lfs, &env.config));
        assert_ok(lfs_unmount(lfs));
        let snapshot = env.snapshot();

        let dummy = path_bytes("dummy");
        let result = run_powerloss_linear(
            &mut env,
            &snapshot,
            3000,
            |lfs_ptr, config| {
                let err = lfs_mount(lfs_ptr, config);
                if err.is_err() {
                    lfs_format(lfs_ptr, config)?;
                    lfs_mount(lfs_ptr, config)?;
                }
                for i in 0..N {
                    let info =
                        &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
                    let err = lfs_stat(lfs_ptr, dummy.as_c_str(), info);
                    if err.is_ok() {
                        if info.type_ == LFS_TYPE_REG as u8 {
                            let e = lfs_remove(lfs_ptr, dummy.as_c_str());
                            if e.is_err() {
                                let _ = lfs_unmount(lfs_ptr);
                                return e;
                            }
                        }
                    } else if err != Err(Error::NoEntry) || i != 0 {
                        let _ = lfs_unmount(lfs_ptr);
                        return Err(err.unwrap_err());
                    }
                    let file =
                        &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
                    let e = lfs_file_open(
                        lfs_ptr,
                        file,
                        dummy.as_c_str(),
                        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                    );
                    if e.is_err() {
                        let _ = lfs_unmount(lfs_ptr);
                        return e;
                    }
                    let e = lfs_file_close(lfs_ptr, file);
                    if e.is_err() {
                        let _ = lfs_unmount(lfs_ptr);
                        return e;
                    }
                    let info =
                        &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
                    let e = lfs_stat(lfs_ptr, dummy.as_c_str(), info);
                    if e.is_err() {
                        let _ = lfs_unmount(lfs_ptr);
                        return e;
                    }
                }
                lfs_unmount(lfs_ptr)?;
                Ok(())
            },
            |_, _| Ok(()),
        );
        result.unwrap_or_else(|_| {
            panic!("test_superblocks_reentrant_expand block_cycles={block_cycles} should complete")
        });
    }
}

/// Upstream: [cases.test_superblocks_unknown_blocks]
/// Mount with block_count=0, lfs_fs_stat, basic file ops.
#[test]
fn test_superblocks_unknown_blocks() {
    const BLOCK_COUNT: u32 = 128;
    let mut env = default_config(BLOCK_COUNT);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    assert_ok(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok(lfs_unmount(lfs));

    let cfg0 = clone_config_with_block_count(&env, 0);
    assert_ok(lfs_mount(lfs, &cfg0.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &cfg0.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    let test_path = path_bytes("test");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        test_path.as_c_str(),
        LFS_O_CREAT | LFS_O_EXCL | LFS_O_WRONLY,
    ));
    let data = b"hello!";
    assert_eq!(lfs_file_write(lfs, file, data,), Ok(data.len() as u32));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &cfg0.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, test_path.as_c_str(), LFS_O_RDONLY));
    let mut buf = [0u8; 256];
    let n = lfs_file_read(lfs, file, &mut buf);
    assert_eq!(n, Ok(data.len() as u32));
    assert_eq!(&buf[..data.len()], data);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_fewer_blocks]
/// Format with BLOCK_COUNT blocks; mount with ERASE_COUNT blocks => LFS_ERR_INVAL.
#[test]
fn test_superblocks_fewer_blocks() {
    const ERASE_COUNT: u32 = 128;
    for &block_count in &[ERASE_COUNT / 2, ERASE_COUNT / 4, 2u32] {
        let mut env = default_config(ERASE_COUNT);
        init_context(&mut env);
        env.config.block_count = block_count;

        let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
        assert_ok(lfs_format(lfs, &env.config));

        let cfg_full = clone_config_with_block_count(&env, ERASE_COUNT);
        let err = lfs_mount(lfs, &cfg_full.config);
        assert_err(Error::Invalid, err);

        let cfg0 = clone_config_with_block_count(&env, 0);
        assert_ok(lfs_mount(lfs, &cfg0.config));
        let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
        assert_ok(lfs_fs_stat(lfs, fsinfo));
        assert_eq!(fsinfo.block_count, block_count);
        assert_ok(lfs_unmount(lfs));

        let test_path = path_bytes("test");
        assert_ok(lfs_mount(lfs, &cfg0.config));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            test_path.as_c_str(),
            LFS_O_CREAT | LFS_O_EXCL | LFS_O_WRONLY,
        ));
        assert_eq!(lfs_file_write(lfs, file, b"hello!"), Ok(6));
        assert_ok(lfs_file_close(lfs, file));
        assert_ok(lfs_unmount(lfs));

        assert_ok(lfs_mount(lfs, &cfg0.config));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, test_path.as_c_str(), LFS_O_RDONLY));
        let mut buf = [0u8; 16];
        assert_eq!(lfs_file_read(lfs, file, &mut buf,), Ok(6));
        assert_eq!(&buf[..6], b"hello!");
        assert_ok(lfs_file_close(lfs, file));
        assert_ok(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_superblocks_more_blocks]
/// Format with 2*ERASE_COUNT blocks; mount with ERASE_COUNT => LFS_ERR_INVAL.
#[test]
fn test_superblocks_more_blocks() {
    const ERASE_COUNT: u32 = 128;
    let mut env = default_config(2 * ERASE_COUNT);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    let cfg_half = clone_config_with_block_count(&env, ERASE_COUNT);
    let err = lfs_mount(lfs, &cfg_half.config);
    assert_err(Error::Invalid, err);
}

const ERASE_COUNT_GROW: u32 = 128;

/// Upstream: [cases.test_superblocks_grow]
/// defines.BLOCK_COUNT = [ERASE_COUNT/2, ERASE_COUNT/4, 2], BLOCK_COUNT_2 = ERASE_COUNT,
/// KNOWN_BLOCK_COUNT = [true, false]. lfs_fs_grow from smaller to larger block count.
#[rstest]
fn test_superblocks_grow(
    #[values(
        ERASE_COUNT_GROW / 2,
        ERASE_COUNT_GROW / 4,
        2u32
    )]
    small_count: u32,
    #[values(false, true)] known_block_count: bool,
) {
    let mut env = default_config(ERASE_COUNT_GROW);
    init_context(&mut env);

    let large_count = ERASE_COUNT_GROW;
    env.config.block_count = small_count;

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    // Create a file to verify after grow
    let path = path_bytes("x");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path.as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    let buf = b"hello";
    assert_eq!(lfs_file_write(lfs, file, buf,), Ok(buf.len() as u32),);
    assert_ok(lfs_file_close(lfs, file));

    assert_ok(lfs_fs_grow(lfs, large_count));
    assert_ok(lfs_unmount(lfs));

    // Mount with full block_count and verify (or block_count=0 when known_block_count is false)
    let mount_block_count = if known_block_count { large_count } else { 0 };
    let mount_cfg = clone_config_with_block_count(&env, mount_block_count);
    env.config.block_count = large_count;
    assert_ok(lfs_mount(lfs, &mount_cfg.config));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path.as_c_str(), LFS_O_RDONLY));
    let mut rbuf = [0u8; 16];
    let n = lfs_file_read(lfs, file, &mut rbuf);
    assert_eq!(n, Ok(buf.len() as u32));
    assert_eq!(&rbuf[..buf.len()], buf);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

#[cfg(feature = "shrink")]
const ERASE_COUNT_SHRINK: u32 = 128;

/// Upstream: [cases.test_superblocks_shrink]
/// BLOCK_COUNT = ERASE_COUNT, BLOCK_COUNT_2 = [ERASE_COUNT/2, ERASE_COUNT/4, 2],
/// KNOWN_BLOCK_COUNT = [true, false]. Shrink via lfs_fs_grow to smaller size.
#[cfg(feature = "shrink")]
#[rstest]
fn test_superblocks_shrink(
    #[values(ERASE_COUNT_SHRINK / 2, ERASE_COUNT_SHRINK / 4, 2u32)] block_count_2: u32,
    #[values(true, false)] known_block_count: bool,
) {
    const BLOCK_COUNT: u32 = ERASE_COUNT_SHRINK;
    const BLOCK_SIZE: u32 = 512;

    let mut env = default_config(ERASE_COUNT_SHRINK);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };

    assert_ok(lfs_format(lfs, &env.config));

    env.config.block_count = if known_block_count { BLOCK_COUNT } else { 0 };

    assert_ok(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok(lfs_unmount(lfs));

    // same size is a noop
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_fs_grow(lfs, BLOCK_COUNT));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok(lfs_unmount(lfs));

    // shrink to BLOCK_COUNT_2
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_fs_grow(lfs, block_count_2));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    assert_ok(lfs_unmount(lfs));

    env.config.block_count = if known_block_count { block_count_2 } else { 0 };

    assert_ok(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    assert_ok(lfs_unmount(lfs));

    // mounting with the previous (larger) size should fail
    let cfg_old = clone_config_with_block_count(&env, BLOCK_COUNT);
    assert_err(Error::Invalid, lfs_mount(lfs, &cfg_old.config));

    env.config.block_count = if known_block_count { block_count_2 } else { 0 };

    // same size is a noop
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_fs_grow(lfs, block_count_2));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    assert_ok(lfs_unmount(lfs));

    // write and read back a file
    assert_ok(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    let test_path = path_bytes("test");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        test_path.as_c_str(),
        LFS_O_CREAT | LFS_O_EXCL | LFS_O_WRONLY,
    ));
    assert_eq!(lfs_file_write(lfs, file, b"hello!"), Ok(6));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, test_path.as_c_str(), LFS_O_RDONLY));
    let mut buf = [0u8; 256];
    assert_eq!(lfs_file_read(lfs, file, &mut buf), Ok(6));
    assert_eq!(&buf[..6], b"hello!");
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_metadata_max]
/// Exercises superblock compaction under different metadata_max constraints.
/// METADATA_MAX = [lfs_max(512, PROG_SIZE), lfs_max(BLOCK_SIZE/2, PROG_SIZE), BLOCK_SIZE]
/// With BLOCK_SIZE=512, PROG_SIZE=16: [512, 256, 512]. N = [10, 100, 1000].
#[rstest]
fn test_superblocks_metadata_max(
    #[values(512, 256, 512)] metadata_max: u32,
    #[values(10, 100, 1000)] n: u32,
) {
    // Upstream default: ERASE_COUNT=2048, BLOCK_SIZE=512 → 1MB.
    // Need enough blocks for 1000 files with directory splitting.
    let mut env = default_config(1024);
    init_context(&mut env);
    env.config.metadata_max = metadata_max;

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    for i in 0..n {
        let name_str = format!("hello{:03x}", i);
        let name = path_bytes(&name_str);
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            name.as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok(lfs_stat(lfs, name.as_c_str(), info));
        let nul = info
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(info.name.len());
        let info_name = core::str::from_utf8(&info.name[..nul]).unwrap();
        assert_eq!(info_name, name_str);
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
    }

    assert_ok(lfs_unmount(lfs));
}
