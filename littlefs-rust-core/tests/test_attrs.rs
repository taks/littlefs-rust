//! Custom attributes tests.
//!
//! Upstream: tests/test_attrs.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_attrs.toml

mod common;

use common::{
    LFS_O_CREAT, LFS_O_RDONLY, LFS_O_WRONLY, assert_err, assert_ok, default_config, init_context,
    init_logger, path_bytes,
};
use littlefs_rust_core::{
    Lfs, LfsAttr, LfsFile, LfsFileConfig, error::Error, lfs_file_close, lfs_file_open,
    lfs_file_opencfg, lfs_file_read, lfs_file_sync, lfs_file_write, lfs_format, lfs_getattr,
    lfs_mkdir, lfs_mount, lfs_removeattr, lfs_setattr, lfs_unmount,
};

/// attr_max from config; tests use ATTR_MAX+1 for NOSPC check.
const ATTR_MAX: usize = 1022;

// --- test_attrs_get_set ---
#[test]
fn test_attrs_get_set() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, c"hello"));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("hello/hello").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"hello".as_ptr() as *const core::ffi::c_void, 5);
    assert_eq!(n, Ok(5));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let mut buffer = [0u8; 1024];

    assert_ok(lfs_setattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'A',
        b"aaaa".as_ptr() as *const core::ffi::c_void,
        4,
    ));
    assert_ok(lfs_setattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'B',
        b"bbbbbb".as_ptr() as *const core::ffi::c_void,
        6,
    ));
    assert_ok(lfs_setattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'C',
        b"ccccc".as_ptr() as *const core::ffi::c_void,
        5,
    ));

    let n = lfs_getattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'A',
        buffer[..4].as_mut_ptr() as *mut core::ffi::c_void,
        4,
    );
    assert_eq!(n, Ok(4));
    let n = lfs_getattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'B',
        buffer[4..10].as_mut_ptr() as *mut core::ffi::c_void,
        6,
    );
    assert_eq!(n, Ok(6));
    let n = lfs_getattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'C',
        buffer[10..15].as_mut_ptr() as *mut core::ffi::c_void,
        5,
    );
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    assert_ok(lfs_setattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'B',
        b"".as_ptr() as *const core::ffi::c_void,
        0,
    ));
    let n = lfs_getattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'B',
        buffer[4..10].as_mut_ptr() as *mut core::ffi::c_void,
        6,
    );
    assert_eq!(n, Ok(0));
    assert_eq!(&buffer[4..10], b"\0\0\0\0\0\0");

    assert_ok(lfs_removeattr(lfs, c"hello", b'B'));
    let err = lfs_getattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'B',
        buffer[4..10].as_mut_ptr() as *mut core::ffi::c_void,
        6,
    );
    assert_err(Error::NoAttribute, err);

    assert_ok(lfs_setattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'B',
        b"dddddd".as_ptr() as *const core::ffi::c_void,
        6,
    ));
    assert_ok(lfs_setattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'B',
        b"eee".as_ptr() as *const core::ffi::c_void,
        3,
    ));

    let oversized = vec![0u8; ATTR_MAX + 1];
    let err = lfs_setattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'A',
        oversized.as_ptr() as *const core::ffi::c_void,
        (ATTR_MAX + 1) as u32,
    );
    assert_err(Error::NoSpace, err);

    assert_ok(lfs_setattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'B',
        b"fffffffff".as_ptr() as *const core::ffi::c_void,
        9,
    ));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let n = lfs_getattr(
        lfs,
        path_bytes("hello").as_c_str(),
        b'B',
        buffer[4..13].as_mut_ptr() as *mut core::ffi::c_void,
        9,
    );
    assert_eq!(n, Ok(9));
    assert_eq!(&buffer[4..13], b"fffffffff");

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("hello/hello").as_c_str(),
        LFS_O_RDONLY,
    ));
    let n = lfs_file_read(lfs, file, buffer.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[..5], b"hello");
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

// --- test_attrs_get_set_root ---
#[test]
fn test_attrs_get_set_root() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, c"hello"));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("hello/hello").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"hello".as_ptr() as *const core::ffi::c_void, 5);
    assert_eq!(n, Ok(5));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let mut buffer = [0u8; 1024];

    assert_ok(lfs_setattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'A',
        b"aaaa".as_ptr() as *const core::ffi::c_void,
        4,
    ));
    assert_ok(lfs_setattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'B',
        b"bbbbbb".as_ptr() as *const core::ffi::c_void,
        6,
    ));
    assert_ok(lfs_setattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'C',
        b"ccccc".as_ptr() as *const core::ffi::c_void,
        5,
    ));

    let n = lfs_getattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'A',
        buffer[..4].as_mut_ptr() as *mut core::ffi::c_void,
        4,
    );
    assert_eq!(n, Ok(4));
    let n = lfs_getattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'B',
        buffer[4..10].as_mut_ptr() as *mut core::ffi::c_void,
        6,
    );
    assert_eq!(n, Ok(6));
    let n = lfs_getattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'C',
        buffer[10..15].as_mut_ptr() as *mut core::ffi::c_void,
        5,
    );
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    assert_ok(lfs_setattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'B',
        b"".as_ptr() as *const core::ffi::c_void,
        0,
    ));
    assert_ok(lfs_removeattr(lfs, c"/", b'B'));
    assert_ok(lfs_setattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'B',
        b"fffffffff".as_ptr() as *const core::ffi::c_void,
        9,
    ));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let mut buffer = [0u8; 1024];
    let n = lfs_getattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'A',
        buffer[..4].as_mut_ptr() as *mut core::ffi::c_void,
        4,
    );
    assert_eq!(n, Ok(4));
    let n = lfs_getattr(
        lfs,
        path_bytes("/").as_c_str(),
        b'B',
        buffer[4..13].as_mut_ptr() as *mut core::ffi::c_void,
        9,
    );
    assert_eq!(n, Ok(9));
    assert_eq!(&buffer[4..13], b"fffffffff");

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("hello/hello").as_c_str(),
        LFS_O_RDONLY,
    ));
    let n = lfs_file_read(lfs, file, buffer.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[..5], b"hello");
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

// --- test_attrs_get_set_file ---
// Uses lfs_file_opencfg with attrs: WRONLY writes attrs on close, RDONLY reads on open.
#[test]
fn test_attrs_get_set_file() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("hello").as_c_str()));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("hello/hello").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"hello".as_ptr() as *const core::ffi::c_void, 5);
    assert_eq!(n, Ok(5));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let mut buffer = [0u8; 1024];
    buffer[0..4].copy_from_slice(b"aaaa");
    buffer[4..10].copy_from_slice(b"bbbbbb");
    buffer[10..15].copy_from_slice(b"ccccc");

    let mut attrs = [
        LfsAttr {
            type_: b'A',
            buffer: &mut buffer[0..4],
        },
        LfsAttr {
            type_: b'B',
            buffer: &mut buffer[4..10],
        },
        LfsAttr {
            type_: b'C',
            buffer: &mut buffer[10..],
        },
    ];
    let cfg = LfsFileConfig {
        buffer: core::ptr::null_mut(),
        attrs: attrs.as_mut_ptr(),
        attr_count: 3,
    };
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_opencfg(
        lfs,
        file,
        c"hello/hello",
        LFS_O_WRONLY,
        &cfg,
    ));
    assert_ok(lfs_file_close(lfs, file));

    buffer.fill(0);
    let mut attrs_read = [
        LfsAttr {
            type_: b'A',
            buffer: buffer.as_mut_ptr() as *mut core::ffi::c_void,
            size: 4,
        },
        LfsAttr {
            type_: b'B',
            buffer: buffer[4..].as_mut_ptr() as *mut core::ffi::c_void,
            size: 6,
        },
        LfsAttr {
            type_: b'C',
            buffer: buffer[10..].as_mut_ptr() as *mut core::ffi::c_void,
            size: 5,
        },
    ];
    let cfg_read = LfsFileConfig {
        buffer: core::ptr::null_mut(),
        attrs: attrs_read.as_mut_ptr(),
        attr_count: 3,
    };
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_opencfg(
        lfs,
        file,
        path_bytes("hello/hello").as_c_str(),
        LFS_O_RDONLY,
        &cfg_read,
    ));
    assert_ok(lfs_file_close(lfs, file));
    assert_eq!(&buffer[0..4], b"aaaa");
    assert_eq!(&buffer[4..10], b"bbbbbb");
    assert_eq!(&buffer[10..15], b"ccccc");

    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("hello/hello").as_c_str(),
        LFS_O_RDONLY,
    ));
    let n = lfs_file_read(lfs, file, buffer.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(5));
    assert_eq!(&buffer[..5], b"hello");
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

// --- test_attrs_deferred_file ---
// Uses lfs_file_opencfg with deferred attrs (synced on file_sync).
#[test]
fn test_attrs_deferred_file() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("hello").as_c_str()));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("hello/hello").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"hello".as_ptr() as *const core::ffi::c_void, 5);
    assert_eq!(n, Ok(5));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_setattr(
        lfs,
        path_bytes("hello/hello").as_c_str(),
        b'B',
        b"fffffffff".as_ptr() as *const core::ffi::c_void,
        9,
    ));
    assert_ok(lfs_setattr(
        lfs,
        path_bytes("hello/hello").as_c_str(),
        b'C',
        b"ccccc".as_ptr() as *const core::ffi::c_void,
        5,
    ));

    let mut buffer = [0u8; 1024];
    let n = lfs_getattr(
        lfs,
        path_bytes("hello/hello").as_c_str(),
        b'B',
        buffer[..9].as_mut_ptr() as *mut core::ffi::c_void,
        9,
    );
    assert_eq!(n, Ok(9));
    assert_eq!(&buffer[..9], b"fffffffff");

    let mut attr_buf = [0u8; 16];
    attr_buf[0..4].copy_from_slice(b"gggg");
    attr_buf[8..12].copy_from_slice(b"hhhh");
    let mut attrs = [
        LfsAttr {
            type_: b'B',
            buffer: attr_buf.as_mut_ptr() as *mut core::ffi::c_void,
            size: 4,
        },
        LfsAttr {
            type_: b'C',
            buffer: core::ptr::null_mut(),
            size: 0,
        },
        LfsAttr {
            type_: b'D',
            buffer: attr_buf[8..].as_mut_ptr() as *mut core::ffi::c_void,
            size: 4,
        },
    ];
    let cfg = LfsFileConfig {
        buffer: core::ptr::null_mut(),
        attrs: attrs.as_mut_ptr(),
        attr_count: 3,
    };
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_opencfg(
        lfs,
        file,
        path_bytes("hello/hello").as_c_str(),
        LFS_O_WRONLY,
        &cfg,
    ));

    assert_ok(lfs_file_sync(lfs, file));

    let n = lfs_getattr(
        lfs,
        path_bytes("hello/hello").as_c_str(),
        b'B',
        buffer[..9].as_mut_ptr() as *mut core::ffi::c_void,
        9,
    );
    assert_eq!(n, Ok(4));
    assert_eq!(&buffer[..9], b"gggg\0\0\0\0\0");

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}
