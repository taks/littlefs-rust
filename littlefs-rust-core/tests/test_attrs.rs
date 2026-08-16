//! Custom attributes tests.
//!
//! Upstream: tests/test_attrs.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_attrs.toml

mod common;

use common::{LFS_O_CREAT, LFS_O_RDONLY, LFS_O_WRONLY, default_config, init_context, init_logger};
use littlefs_rust_core::{
    Lfs, LfsAttr, LfsFile, LfsFileConfig, error::Error, lfs_file_close, lfs_file_open,
    lfs_file_opencfg, lfs_file_read, lfs_file_sync, lfs_file_write, lfs_format, lfs_getattr,
    lfs_mkdir, lfs_mount, lfs_removeattr, lfs_setattr, lfs_unmount,
};
use zerocopy::IntoBytes;

/// attr_max from config; tests use ATTR_MAX+1 for NOSPC check.
const ATTR_MAX: usize = 1022;

// --- test_attrs_get_set ---
#[test]
fn test_attrs_get_set() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_mkdir(lfs, "hello"));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "hello/hello",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"hello");
    assert_eq!(n, Ok(5));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let mut buffer = [0u8; 1024];

    assert_ok!(lfs_setattr(lfs, "hello", b'A', b"aaaa", 4));
    assert_ok!(lfs_setattr(lfs, "hello", b'B', b"bbbbbb", 6));
    assert_ok!(lfs_setattr(lfs, "hello", b'C', b"ccccc", 5));

    let n = lfs_getattr(lfs, "hello", b'A', &mut buffer[..4]);
    assert_eq!(n, Ok(4));
    let n = lfs_getattr(lfs, "hello", b'B', &mut buffer[4..10]);
    assert_eq!(n, Ok(6));
    let n = lfs_getattr(lfs, "hello", b'C', &mut buffer[10..15]);
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    assert_ok!(lfs_setattr(lfs, "hello", b'B', b"", 0));
    let n = lfs_getattr(lfs, "hello", b'B', &mut buffer[4..10]);
    assert_eq!(n, Ok(0));
    assert_eq!(&buffer[4..10], b"\0\0\0\0\0\0");

    assert_ok!(lfs_removeattr(lfs, "hello", b'B'));
    let err = lfs_getattr(lfs, "hello", b'B', &mut buffer[4..10]);
    assert_err!(Error::NoAttribute, err);

    assert_ok!(lfs_setattr(lfs, "hello", b'B', b"dddddd", 6));
    assert_ok!(lfs_setattr(lfs, "hello", b'B', b"eee", 3));

    let oversized = vec![0u8; ATTR_MAX + 1];
    let err = lfs_setattr(lfs, "hello", b'A', &oversized, (ATTR_MAX + 1) as u32);
    assert_err!(Error::NoSpace, err);

    assert_ok!(lfs_setattr(lfs, "hello", b'B', b"fffffffff", 9));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let n = lfs_getattr(lfs, "hello", b'B', &mut buffer[4..13]);
    assert_eq!(n, Ok(9));
    assert_eq!(&buffer[4..13], b"fffffffff");

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(lfs, file, "hello/hello", LFS_O_RDONLY));
    let n = lfs_file_read(lfs, file, &mut buffer[..32]);
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[..5], b"hello");
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_attrs_get_set_root ---
#[test]
fn test_attrs_get_set_root() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_mkdir(lfs, "hello"));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "hello/hello",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"hello");
    assert_eq!(n, Ok(5));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let mut buffer = [0u8; 1024];

    assert_ok!(lfs_setattr(lfs, "/", b'A', b"aaaa", 4));
    assert_ok!(lfs_setattr(lfs, "/", b'B', b"bbbbbb", 6));
    assert_ok!(lfs_setattr(lfs, "/", b'C', b"ccccc", 5));

    let n = lfs_getattr(lfs, "/", b'A', &mut buffer[..4]);
    assert_eq!(n, Ok(4));
    let n = lfs_getattr(lfs, "/", b'B', &mut buffer[4..10]);
    assert_eq!(n, Ok(6));
    let n = lfs_getattr(lfs, "/", b'C', &mut buffer[10..15]);
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    assert_ok!(lfs_setattr(lfs, "/", b'B', b"", 0));
    assert_ok!(lfs_removeattr(lfs, "/", b'B'));
    assert_ok!(lfs_setattr(lfs, "/", b'B', b"fffffffff", 9));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let mut buffer = [0u8; 1024];
    let n = lfs_getattr(lfs, "/", b'A', &mut buffer[..4]);
    assert_eq!(n, Ok(4));
    let n = lfs_getattr(lfs, "/", b'B', &mut buffer[4..13]);
    assert_eq!(n, Ok(9));
    assert_eq!(&buffer[4..13], b"fffffffff");

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(lfs, file, "hello/hello", LFS_O_RDONLY));
    let n = lfs_file_read(lfs, file, &mut buffer[..32]);
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[..5], b"hello");
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_attrs_get_set_file ---
// Uses lfs_file_opencfg with attrs: WRONLY writes attrs on close, RDONLY reads on open.
#[test]
fn test_attrs_get_set_file() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_mkdir(lfs, "hello"));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "hello/hello",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"hello");
    assert_eq!(n, Ok(5));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let mut buffer = [0u8; 1024];
    buffer[0..4].copy_from_slice(b"aaaa");
    buffer[4..10].copy_from_slice(b"bbbbbb");
    buffer[10..15].copy_from_slice(b"ccccc");

    let mut attrs = [
        LfsAttr {
            type_: b'A',
            buffer: unsafe { core::mem::transmute::<&mut [u8], &mut [u8]>(&mut buffer[0..4]) },
        },
        LfsAttr {
            type_: b'B',
            buffer: unsafe { core::mem::transmute::<&mut [u8], &mut [u8]>(&mut buffer[4..10]) },
        },
        LfsAttr {
            type_: b'C',
            buffer: &mut buffer[10..15],
        },
    ];
    let mut cfg = LfsFileConfig {
        buffer: &mut [],
        attrs: &mut attrs,
    };
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_opencfg(
        lfs,
        file,
        "hello/hello",
        LFS_O_WRONLY,
        &mut cfg,
    ));
    assert_ok!(lfs_file_close(lfs, file));

    buffer.fill(0);
    let mut attrs_read = [
        LfsAttr {
            type_: b'A',
            buffer: unsafe { core::mem::transmute::<&mut [u8], &mut [u8]>(&mut buffer[0..4]) },
            // size: 4,
        },
        LfsAttr {
            type_: b'B',
            buffer: unsafe { core::mem::transmute::<&mut [u8], &mut [u8]>(&mut buffer[4..10]) },
            // size: 6,
        },
        LfsAttr {
            type_: b'C',
            buffer: &mut buffer[10..],
            // size: 5,
        },
    ];
    let mut cfg_read = LfsFileConfig {
        buffer: &mut [],
        attrs: &mut attrs_read,
    };
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_opencfg(
        lfs,
        file,
        "hello/hello",
        LFS_O_RDONLY,
        &mut cfg_read,
    ));
    assert_ok!(lfs_file_close(lfs, file));
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(lfs, file, "hello/hello", LFS_O_RDONLY));
    let n = lfs_file_read(lfs, file, &mut buffer[..32]);
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[..5], b"hello");
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_attrs_deferred_file ---
// Uses lfs_file_opencfg with deferred attrs (synced on file_sync).
#[test]
fn test_attrs_deferred_file() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_mkdir(lfs, "hello"));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "hello/hello",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"hello");
    assert_eq!(n, Ok(5));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_setattr(lfs, "hello/hello", b'B', b"fffffffff", 9));
    assert_ok!(lfs_setattr(lfs, "hello/hello", b'C', b"ccccc", 5));

    let mut buffer = [0u8; 1024];
    let n = lfs_getattr(lfs, "hello/hello", b'B', &mut buffer[..9]);
    assert_eq!(n, Ok(9));
    assert_eq!(&buffer[..9], b"fffffffff");

    let mut attr_buf = [0u8; 16];
    attr_buf[0..4].copy_from_slice(b"gggg");
    attr_buf[8..12].copy_from_slice(b"hhhh");
    let mut attrs = [
        LfsAttr {
            type_: b'B',
            buffer: unsafe {
                core::mem::transmute::<&mut [u8], &mut [u8]>(attr_buf[0..4].as_mut_bytes())
            },
            // size: 4,
        },
        LfsAttr {
            type_: b'C',
            buffer: &mut [],
            //size: 0,
        },
        LfsAttr {
            type_: b'D',
            buffer: attr_buf[8..].as_mut_bytes(),
            // size: 4,
        },
    ];
    let mut cfg = LfsFileConfig {
        buffer: &mut [],
        attrs: &mut attrs,
    };
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_opencfg(
        lfs,
        file,
        "hello/hello",
        LFS_O_WRONLY,
        &mut cfg,
    ));

    assert_ok!(lfs_file_sync(lfs, file));

    let n = lfs_getattr(lfs, "hello/hello", b'B', &mut buffer[..9]);
    assert_eq!(n, Ok(4));
    assert_eq!(&buffer[..9], b"gggg\0\0\0\0\0");

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}
