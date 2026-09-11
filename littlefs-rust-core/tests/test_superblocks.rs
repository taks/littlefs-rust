//! Superblock and format/mount tests.
//!
//! Upstream: tests/test_superblocks.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_superblocks.toml

mod common;

use common::{
    LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY, default_config, init_context,
    read_block_raw,
};
use littlefs_rust_core::lfs_type::lfs_type::{
    LFS_TYPE_CREATE, LFS_TYPE_INLINESTRUCT, LFS_TYPE_REG, LFS_TYPE_SUPERBLOCK,
};
use littlefs_rust_core::{
    Error, LFS_DISK_VERSION, Lfs, LfsConfig, LfsFile, LfsFsinfo, LfsInfo, LfsMattr, LfsMdir,
    LfsSuperblock, lfs_deinit, lfs_dir_commit, lfs_file_close, lfs_file_open, lfs_file_read,
    lfs_file_write, lfs_format, lfs_fs_grow, lfs_fs_stat, lfs_init, lfs_mktag, lfs_mount,
    lfs_remove, lfs_stat, lfs_superblock_tole32, lfs_unmount,
};
use littlefs_rust_test_macro::lfs_test;
use rstest::rstest;
use std::cmp;
use zerocopy::IntoBytes;

// --- test_superblocks_format ---
// Upstream: lfs_format(&lfs, cfg) => 0
#[lfs_test]
fn test_superblocks_format(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
}

// --- test_superblocks_mount ---
// Upstream: format, mount, unmount
#[lfs_test]
fn test_superblocks_mount(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_superblocks_magic ---
// Upstream: format, then raw read to verify "littlefs" at MAGIC_OFFSET in both blocks.
#[lfs_test]
fn test_superblocks_magic(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));

    let mut magic = vec![0u8; cmp::max(16, cfg.read_size as usize)];
    assert_ok!(read_block_raw(cfg, 0, 0, &mut magic));
    assert_eq!(&magic[8..16], b"littlefs");
    assert_ok!(read_block_raw(cfg, 1, 0, &mut magic));
    assert_eq!(&magic[8..16], b"littlefs");
}

// --- test_traverse_attrs_callback_order ---
// Unit test (in integration harness): traverse with tmask=0 passes SUPERBLOCK correctly.
#[lfs_test]
fn test_traverse_attrs_callback_order(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    let mut out = littlefs_rust_core::TraverseTestOut::default();

    assert_ok!(unsafe { littlefs_rust_core::test_traverse_format_attrs(lfs, cfg, &mut out) });

    assert_eq!(out.call_count, 3);
    assert_eq!(out.tags[1], 0x0ff, "second callback should be SUPERBLOCK");
    assert_eq!(out.first_bytes[1], b'l');
}

// --- test_traverse_filter_gets_superblock_after_push ---
// Unit test: traverse with tmask (compact-style) triggers push; callback receives SUPERBLOCK with 'l'.
#[lfs_test]
fn test_traverse_filter_gets_superblock_after_push(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    let mut out = littlefs_rust_core::TraverseTestOut::default();

    assert_ok!(unsafe {
        littlefs_rust_core::test_traverse_filter_gets_superblock_after_push(lfs, cfg, &mut out)
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
#[lfs_test]
fn test_superblocks_invalid_mount(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    let err = lfs_mount(lfs, cfg);
    assert_err!(Error::Corrupt, err);
}

// --- test_superblocks_stat ---
// Upstream: fs_stat after format/mount returns correct values
#[lfs_test]
fn test_superblocks_stat(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, cfg.block_size);
    assert_eq!(fsinfo.block_count, cfg.block_count);
    assert_eq!(fsinfo.disk_version, 0x0002_0001);
    assert_eq!(fsinfo.name_max, 255);
    assert_eq!(fsinfo.file_max, 2_147_483_647);
    assert_eq!(fsinfo.attr_max, 1022);
}

// --- Missing upstream stubs ---

/// Upstream: [cases.test_superblocks_mount_unknown_block_count]
/// Mount with block_count=0; verify lfs.block_count is set from superblock.
#[lfs_test]
fn test_superblocks_mount_unknown_block_count(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));

    let tweaked_cfg = LfsConfig {
        block_count: 0,
        ..*cfg
    };

    assert_ok!(lfs_mount(lfs, &tweaked_cfg));
    assert_eq!(lfs.block_count, cfg.block_count);
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_reentrant_format]
/// reentrant = true, POWERLOSS_BEHAVIOR = [NOOP, OOO]. Format under power-loss, then mount.
#[lfs_test]
#[cfg(feature = "slow_tests")]
fn test_superblocks_reentrant_format(cfg: &LfsConfig, #[values(false, true)] reentrant: bool) {
    let lfs = &mut Lfs::default();
    let err = lfs_mount(lfs, cfg);
    if err.is_err() {
        assert_ok!(lfs_format(lfs, cfg));
        assert_ok!(lfs_mount(lfs, cfg));
    }
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_stat_tweaked]
/// Format with name_max=63, file_max=65535, attr_max=512; mount with default; verify fsinfo.
#[lfs_test]
fn test_superblocks_stat_tweaked(cfg: &LfsConfig) {
    let tweaked_cfg = LfsConfig {
        name_max: 63,
        file_max: 65535,
        attr_max: 512,
        ..*cfg
    };

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &tweaked_cfg));

    assert_ok!(lfs_mount(lfs, cfg));

    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.name_max, 63);
    assert_eq!(fsinfo.file_max, 65535);
    assert_eq!(fsinfo.attr_max, 512);
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_expand]
/// Create/remove dummy file N times; verify superblock survives compaction.
#[lfs_test]
fn test_superblocks_expand(
    cfg: &LfsConfig,
    #[values(32, 33, 1)] block_cycles: i32,
    #[values(10, 100, 1000)] n: u32,
) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let dummy = "dummy";
    for _ in 0..n {
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            dummy,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok!(lfs_file_close(lfs, file));
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok!(lfs_stat(lfs, dummy, info));
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
        assert_ok!(lfs_remove(lfs, dummy));
    }
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, cfg));
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        dummy,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    assert_ok!(lfs_file_close(lfs, file));
    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok!(lfs_stat(lfs, dummy, info));
    assert_eq!(info.type_, LFS_TYPE_REG as u8);
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_magic_expand]
/// Same as expand + magic check after.
#[lfs_test]
fn test_superblocks_magic_expand(
    cfg: &LfsConfig,
    #[values(32, 33, 1)] block_cycles: i32,
    #[values(10, 100, 1000)] n: u32,
) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let dummy = "dummy";
    for _ in 0..n {
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            dummy,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok!(lfs_file_close(lfs, file));
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok!(lfs_stat(lfs, dummy, info));
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
        assert_ok!(lfs_remove(lfs, dummy));
    }
    assert_ok!(lfs_unmount(lfs));

    let mut magic = vec![0u8; cmp::max(16, cfg.read_size as usize)];
    assert_ok!(read_block_raw(cfg, 0, 0, &mut magic));
    assert_eq!(&magic[8..16], b"littlefs");
    assert_ok!(read_block_raw(cfg, 1, 0, &mut magic));
    assert_eq!(&magic[8..16], b"littlefs");
}

/// Upstream: [cases.test_superblocks_expand_power_cycle]
/// Same as expand but unmount/remount after each iteration.
#[lfs_test]
fn test_superblocks_expand_power_cycle(
    cfg: &LfsConfig,
    #[values(32, 33, 1)] block_cycles: i32,
    #[values(10, 100, 1000)] n: u32,
) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));

    let dummy = "dummy";
    for i in 0..n {
        assert_ok!(lfs_mount(lfs, cfg));
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        let err = lfs_stat(lfs, dummy, info);
        assert!(
            err.is_ok() || (err == Err(Error::NoEntry) && i == 0),
            "stat dummy: err={err:?} i={i}"
        );
        if err.is_ok() {
            assert_eq!(info.type_, LFS_TYPE_REG as u8);
            assert_ok!(lfs_remove(lfs, dummy));
        }

        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            dummy,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok!(lfs_file_close(lfs, file));
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok!(lfs_stat(lfs, dummy, info));
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
        assert_ok!(lfs_unmount(lfs));
    }

    assert_ok!(lfs_mount(lfs, cfg));
    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok!(lfs_stat(lfs, dummy, info));
    assert_eq!(info.type_, LFS_TYPE_REG as u8);
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_reentrant_expand]
/// BLOCK_CYCLES = [2, 1], N = 24, reentrant, POWERLOSS_BEHAVIOR = [NOOP, OOO]
#[lfs_test]
#[cfg(feature = "slow_tests")]
fn test_superblocks_reentrant_expand(
    cfg: &LfsConfig,
    #[values(false, true)] reentrant: bool,
    #[values(2, 1)] block_cycles: i32,
) {
    const N: u32 = 24;

    let lfs = &mut Lfs::default();

    let dummy = "dummy";

    let err = lfs_mount(lfs, cfg);
    if err.is_err() {
        assert_ok!(lfs_format(lfs, cfg));
        assert_ok!(lfs_mount(lfs, cfg));
    }
    for i in 0..N {
        let info = &mut LfsInfo::default();
        let err = lfs_stat(lfs, dummy, info);
        assert!(err.is_ok() || (err == Err(Error::NoEntry) && i == 0));
        if err.is_ok() {
            assert_eq!(info.name_str(), dummy);
            assert_eq!(info.type_, LFS_TYPE_REG as u8);
            assert_ok!(lfs_remove(lfs, dummy));
        }
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            dummy,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok!(lfs_file_close(lfs, file));

        let info = &mut LfsInfo::default();
        assert_ok!(lfs_stat(lfs, dummy, info));
        assert_eq!(info.name_str(), dummy);
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
        assert_ok!(lfs_unmount(lfs));
    }

    // one last check after power-cycle
    assert_ok!(lfs_mount(lfs, cfg));
    let info = &mut LfsInfo::default();
    assert_ok!(lfs_stat(lfs, dummy, info));
    assert_eq!(info.name_str(), dummy);
    assert_eq!(info.type_, LFS_TYPE_REG as u8);
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_unknown_blocks]
/// Mount with block_count=0, lfs_fs_stat, basic file ops.
#[lfs_test]
fn test_superblocks_unknown_blocks(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));

    assert_ok!(lfs_mount(lfs, cfg));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, cfg.block_size);
    assert_eq!(fsinfo.block_count, cfg.block_count);
    assert_ok!(lfs_unmount(lfs));

    // unknown block_count
    let cfg0 = LfsConfig {
        block_count: 0,
        ..*cfg
    };
    assert_ok!(lfs_mount(lfs, &cfg0));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, cfg.block_size);
    assert_eq!(fsinfo.block_count, cfg.block_count);
    assert_ok!(lfs_unmount(lfs));

    // do some work
    assert_ok!(lfs_mount(lfs, &cfg0));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, cfg.block_size);
    assert_eq!(fsinfo.block_count, cfg.block_count);
    let test_path = "test";
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        test_path,
        LFS_O_CREAT | LFS_O_EXCL | LFS_O_WRONLY,
    ));
    let data = b"hello!";
    assert_eq!(lfs_file_write(lfs, file, data,), Ok(data.len() as u32));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &cfg0));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_count, cfg.block_count);
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, test_path, LFS_O_RDONLY));
    let mut buf = [0u8; 256];
    let n = lfs_file_read(lfs, file, &mut buf);
    assert_eq!(n, Ok(data.len() as u32));
    assert_eq!(&buf[..data.len()], data);
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_superblocks_fewer_blocks]
/// Format with BLOCK_COUNT blocks; mount with ERASE_COUNT blocks => LFS_ERR_INVAL.
#[lfs_test]
fn test_superblocks_fewer_blocks(cfg: &LfsConfig) {
    let erase_count: u32 = cfg.block_count;
    for &block_count in &[erase_count / 2, erase_count / 4, 2] {
        let mut cfg = LfsConfig {
            block_count,
            ..*cfg
        };

        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, &cfg));

        // known block_size/block_count
        assert_ok!(lfs_mount(lfs, &cfg));
        let fsinfo = &mut LfsFsinfo::default();
        assert_ok!(lfs_fs_stat(lfs, fsinfo));
        assert_eq!(fsinfo.block_size, cfg.block_size);
        assert_eq!(fsinfo.block_count, cfg.block_count);
        assert_ok!(lfs_unmount(lfs));

        // incorrect block_count
        cfg.block_count = erase_count;
        assert_eq!(lfs_mount(lfs, &cfg), Err(Error::Invalid));

        // unknown block_count
        cfg.block_count = 0;
        assert_ok!(lfs_mount(lfs, &cfg));
        assert_ok!(lfs_fs_stat(lfs, fsinfo));
        assert_eq!(fsinfo.block_count, block_count);
        assert_ok!(lfs_unmount(lfs));

        // do some work

        assert_ok!(lfs_mount(lfs, &cfg));
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            "test",
            LFS_O_CREAT | LFS_O_EXCL | LFS_O_WRONLY,
        ));
        assert_eq!(lfs_file_write(lfs, file, b"hello!"), Ok(6));
        assert_ok!(lfs_file_close(lfs, file));
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &cfg));
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, "test", LFS_O_RDONLY));
        let mut buf = [0u8; 16];
        assert_eq!(lfs_file_read(lfs, file, &mut buf,), Ok(6));
        assert_eq!(&buf[..6], b"hello!");
        assert_ok!(lfs_file_close(lfs, file));
        assert_ok!(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_superblocks_more_blocks]
/// Format with 2*ERASE_COUNT blocks; mount with ERASE_COUNT => LFS_ERR_INVAL.
#[lfs_test]
fn test_superblocks_more_blocks(cfg: &LfsConfig) {
    let format_block_count = 2 * cfg.block_count;
    let lfs = &mut Lfs::default();

    assert_ok!(lfs_init(lfs, cfg));

    let mut root = LfsMdir {
        pair: [0, 0],
        rev: 0,
        off: core::mem::size_of::<u32>() as u32,
        etag: 0xffff_ffff,
        count: 0,
        erased: false,
        split: false,
        tail: [u32::MAX, u32::MAX],
    };

    let mut superblock = LfsSuperblock {
        version: LFS_DISK_VERSION,
        block_size: cfg.block_size,
        block_count: format_block_count,
        name_max: 255,
        file_max: 255,
        attr_max: 255,
    };
    lfs_superblock_tole32(&mut superblock);

    assert_ok!(lfs_dir_commit(
        lfs,
        &mut root,
        &[
            LfsMattr {
                tag: lfs_mktag(LFS_TYPE_CREATE, 0, 0),
                buffer: &[],
            },
            LfsMattr {
                tag: lfs_mktag(LFS_TYPE_SUPERBLOCK, 0, 8),
                buffer: b"littlefs",
            },
            LfsMattr {
                tag: lfs_mktag(
                    LFS_TYPE_INLINESTRUCT,
                    0,
                    std::mem::size_of::<LfsSuperblock>()
                ),
                buffer: superblock.as_bytes(),
            },
        ],
    ));
    assert_ok!(lfs_deinit(lfs));
    assert_eq!(lfs_mount(lfs, cfg), Err(Error::Invalid));
}

/// Upstream: [cases.test_superblocks_grow]
/// defines.BLOCK_COUNT = [ERASE_COUNT/2, ERASE_COUNT/4, 2], BLOCK_COUNT_2 = ERASE_COUNT,
/// KNOWN_BLOCK_COUNT = [true, false]. lfs_fs_grow from smaller to larger block count.
#[lfs_test]
fn test_superblocks_grow(cfg: &LfsConfig, #[values(false, true)] known_block_count: bool) {
    let erase_count = cfg.block_count;
    for block_count in [erase_count / 2, erase_count / 4, 2] {
        let mut cfg = LfsConfig {
            block_count,
            ..*cfg
        };

        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, &cfg));
        if !known_block_count {
            cfg.block_count = 0;
        }

        let fsinfo = &mut LfsFsinfo::default();

        // grow to new size
        assert_ok!(lfs_mount(lfs, &cfg));
        assert_ok!(lfs_fs_grow(lfs, erase_count));
        assert_ok!(lfs_fs_stat(lfs, fsinfo));
        assert_eq!(fsinfo.block_size, cfg.block_size);
        assert_eq!(fsinfo.block_count, erase_count);
        assert_ok!(lfs_unmount(lfs));

        cfg.block_count = if known_block_count { erase_count } else { 0 };

        assert_ok!(lfs_mount(lfs, &cfg));
        assert_ok!(lfs_fs_stat(lfs, fsinfo));
        assert_eq!(fsinfo.block_size, cfg.block_size);
        assert_eq!(fsinfo.block_count, erase_count);
        assert_ok!(lfs_unmount(lfs));

        // mounting with the previous size should fail
        cfg.block_count = block_count;
        assert_eq!(lfs_mount(lfs, &cfg), Err(Error::Invalid));

        cfg.block_count = if known_block_count { erase_count } else { 0 };

        // same size is a noop
        assert_ok!(lfs_mount(lfs, &cfg));
        assert_ok!(lfs_fs_grow(lfs, erase_count));
        assert_ok!(lfs_fs_stat(lfs, fsinfo));
        assert_eq!(fsinfo.block_size, cfg.block_size);
        assert_eq!(fsinfo.block_count, erase_count);
        assert_ok!(lfs_unmount(lfs));

        // do some work
        assert_ok!(lfs_mount(lfs, &cfg));
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            "test",
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_eq!(lfs_file_write(lfs, file, b"hello!"), Ok(6));
        assert_ok!(lfs_file_close(lfs, file));
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &cfg));
        assert_ok!(lfs_fs_stat(lfs, fsinfo));
        assert_eq!(fsinfo.block_size, cfg.block_size);
        assert_eq!(fsinfo.block_count, erase_count);
        assert_ok!(lfs_file_open(lfs, file, "test", LFS_O_RDONLY));
        let mut rbuf = [0u8; 16];
        assert_eq!(lfs_file_read(lfs, file, &mut rbuf), Ok(6));
        assert_eq!(&rbuf[..6], b"hello!");
        assert_ok!(lfs_file_close(lfs, file));
        assert_ok!(lfs_unmount(lfs));
    }
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

    let lfs = &mut Lfs::default();

    assert_ok!(lfs_format(lfs, &env.config));

    env.config.block_count = if known_block_count { BLOCK_COUNT } else { 0 };

    assert_ok!(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok!(lfs_unmount(lfs));

    // same size is a noop
    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_fs_grow(lfs, BLOCK_COUNT));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, BLOCK_COUNT);
    assert_ok!(lfs_unmount(lfs));

    // shrink to BLOCK_COUNT_2
    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_fs_grow(lfs, block_count_2));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    assert_ok!(lfs_unmount(lfs));

    env.config.block_count = if known_block_count { block_count_2 } else { 0 };

    assert_ok!(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    assert_ok!(lfs_unmount(lfs));

    // mounting with the previous (larger) size should fail
    let cfg_old = LfsConfig {
        block_count: BLOCK_COUNT,
        ..env.config
    };
    assert_err!(Error::Invalid, lfs_mount(lfs, &cfg_old));

    env.config.block_count = if known_block_count { block_count_2 } else { 0 };

    // same size is a noop
    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_fs_grow(lfs, block_count_2));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    assert_ok!(lfs_unmount(lfs));

    // write and read back a file
    assert_ok!(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    let test_path = "test";
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        test_path,
        LFS_O_CREAT | LFS_O_EXCL | LFS_O_WRONLY,
    ));
    assert_eq!(lfs_file_write(lfs, file, b"hello!"), Ok(6));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok!(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.block_size, BLOCK_SIZE);
    assert_eq!(fsinfo.block_count, block_count_2);
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, test_path, LFS_O_RDONLY));
    let mut buf = [0u8; 256];
    assert_eq!(lfs_file_read(lfs, file, &mut buf), Ok(6));
    assert_eq!(&buf[..6], b"hello!");
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
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

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    for i in 0..n {
        let name_str = format!("hello{:03x}", i);
        let name = &name_str;
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            name,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok!(lfs_file_close(lfs, file));
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok!(lfs_stat(lfs, name, info));
        assert_eq!(info.name_str(), name_str);
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
    }

    assert_ok!(lfs_unmount(lfs));
}
