//! Upstream: tests/test_compat.toml
//!
//! Version edge-case tests. The 14 forward/backward compat tests live in
//! littlefs-rust-core-compat where they test actual C ↔ Rust interop.
//! These 3 remaining tests exercise superblock version field handling.

mod common;

use common::{assert_err, assert_ok, default_config, init_context, init_logger, path_bytes};
use littlefs_rust_core::LfsFile;
use littlefs_rust_core::lfs_type::lfs_type::LFS_TYPE_INLINESTRUCT;
use littlefs_rust_core::{
    LFS_DISK_VERSION, Lfs, LfsFsinfo, LfsMdir, LfsSuperblock, error::Error, lfs_dir_commit,
    lfs_dir_fetch, lfs_format, lfs_fs_stat, lfs_mattr, lfs_mktag, lfs_mount, lfs_superblock_tole32,
    lfs_unmount,
};
use zerocopy::IntoBytes;

/// Upstream: [cases.test_compat_major_incompat]
///
/// Bump major version in superblock, verify mount rejects with LFS_ERR_INVAL.
#[test]
fn test_compat_major_incompat() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let cfg = &env.config;

    let mut lfs = unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(&mut lfs, cfg));
    assert_ok(lfs_mount(&mut lfs, cfg));

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
    assert_ok(lfs_dir_fetch(&mut lfs, &mut mdir, &root_pair));

    let mut superblock = LfsSuperblock {
        version: LFS_DISK_VERSION + 0x0001_0000,
        block_size: cfg.block_size,
        block_count: cfg.block_count,
        name_max: lfs.name_max,
        file_max: lfs.file_max,
        attr_max: lfs.attr_max,
    };
    lfs_superblock_tole32(&mut superblock);
    let attrs = [lfs_mattr {
        tag: lfs_mktag(
            LFS_TYPE_INLINESTRUCT,
            0,
            core::mem::size_of::<LfsSuperblock>() as u32,
        ),
        buffer: superblock.as_bytes(),
    }];
    assert_ok(lfs_dir_commit(&mut lfs, &mut mdir, &attrs));
    assert_ok(lfs_unmount(&mut lfs));

    assert_err(Error::Invalid, lfs_mount(&mut lfs, cfg));
}

/// Upstream: [cases.test_compat_minor_incompat]
///
/// Bump minor version in superblock beyond what we support, verify mount rejects.
#[test]
fn test_compat_minor_incompat() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let cfg = &env.config;

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, cfg));
    assert_ok(lfs_mount(lfs, cfg));

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
    assert_ok(lfs_dir_fetch(lfs, &mut mdir, &root_pair));

    let mut superblock = LfsSuperblock {
        version: LFS_DISK_VERSION + 0x0000_0001,
        block_size: cfg.block_size,
        block_count: cfg.block_count,
        name_max: lfs.name_max,
        file_max: lfs.file_max,
        attr_max: lfs.attr_max,
    };
    lfs_superblock_tole32(&mut superblock);
    let attrs = [lfs_mattr {
        tag: lfs_mktag(
            LFS_TYPE_INLINESTRUCT,
            0,
            core::mem::size_of::<LfsSuperblock>() as u32,
        ),
        buffer: superblock.as_bytes(),
    }];
    assert_ok(lfs_dir_commit(lfs, &mut mdir, &attrs));
    assert_ok(lfs_unmount(lfs));

    assert_err(Error::Invalid, lfs_mount(lfs, cfg));
}

/// Upstream: [cases.test_compat_minor_bump]
///
/// Downgrade minor version in superblock, mount works, write triggers minor bump.
#[test]
fn test_compat_minor_bump() {
    use littlefs_rust_core::lfs_type::lfs_open_flags::{
        LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_TRUNC, LFS_O_WRONLY,
    };
    use littlefs_rust_core::{lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write};

    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let cfg = &env.config;

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, cfg));
    assert_ok(lfs_mount(lfs, cfg));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"test",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    assert_eq!(
        lfs_file_write(
            lfs,
            file,
            b"testtest".as_ptr() as *const core::ffi::c_void,
            8,
        ),
        Ok(8)
    );
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    // Write old minor version to superblock
    assert_ok(lfs_mount(lfs, cfg));
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
    assert_ok(lfs_dir_fetch(lfs, &mut mdir, &root_pair));

    let cfg = unsafe { &*lfs.cfg };
    let mut superblock = LfsSuperblock {
        version: LFS_DISK_VERSION - 1,
        block_size: cfg.block_size,
        block_count: cfg.block_count,
        name_max: lfs.name_max,
        file_max: lfs.file_max,
        attr_max: lfs.attr_max,
    };
    lfs_superblock_tole32(&mut superblock);
    let attrs = [lfs_mattr {
        tag: lfs_mktag(
            LFS_TYPE_INLINESTRUCT,
            0,
            core::mem::size_of::<LfsSuperblock>() as u32,
        ),
        buffer: &superblock.as_bytes(),
    }];
    assert_ok(lfs_dir_commit(lfs, &mut mdir, &attrs));
    assert_ok(lfs_unmount(lfs));

    // Mount should work
    assert_ok(lfs_mount(lfs, cfg));

    let fsinfo = &mut unsafe { core::mem::MaybeUninit::<LfsFsinfo>::zeroed().assume_init() };
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(unsafe { (*fsinfo).disk_version }, LFS_DISK_VERSION - 1);

    assert_ok(lfs_file_open(lfs, file, c"test", LFS_O_RDONLY));
    let mut buf = [0u8; 8];
    assert_eq!(
        lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 8,),
        Ok(8)
    );
    assert_eq!(&buf, b"testtest");
    assert_ok(lfs_file_close(lfs, file));

    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!({ (*fsinfo).disk_version }, LFS_DISK_VERSION - 1);
    assert_ok(lfs_unmount(lfs));

    // Write should bump minor version
    assert_ok(lfs_mount(lfs, cfg));
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!({ (*fsinfo).disk_version }, LFS_DISK_VERSION - 1);

    assert_ok(lfs_file_open(
        lfs,
        file,
        c"test",
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    assert_eq!(
        lfs_file_write(
            lfs,
            file,
            b"teeeeest".as_ptr() as *const core::ffi::c_void,
            8,
        ),
        Ok(8)
    );
    assert_ok(lfs_file_close(lfs, file));

    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.disk_version, LFS_DISK_VERSION);
    assert_ok(lfs_unmount(lfs));

    // Remount, verify version stayed bumped
    assert_ok(lfs_mount(lfs, cfg));
    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(unsafe { (*fsinfo).disk_version }, LFS_DISK_VERSION);

    assert_ok(lfs_file_open(lfs, file, c"test", LFS_O_RDONLY));
    assert_eq!(
        lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 8,),
        Ok(8)
    );
    assert_eq!(&buf, b"teeeeest");
    assert_ok(lfs_file_close(lfs, file));

    assert_ok(lfs_fs_stat(lfs, fsinfo));
    assert_eq!(fsinfo.disk_version, LFS_DISK_VERSION);
    assert_ok(lfs_unmount(lfs));
}
