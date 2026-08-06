//! Move/rename tests.
//!
//! Upstream: tests/test_move.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_move.toml
//!
//! Corruption and powerloss tests; cross-dir rename implemented via lfs_rename_.
#![allow(clippy::manual_c_str_literals)]

mod common;

use common::{
    LFS_O_CREAT, LFS_O_RDONLY, LFS_O_TRUNC, LFS_O_WRONLY, assert_err, assert_ok,
    config_with_wear_leveling, corrupt_block, default_config, dir_block, dir_entry_names, dir_pair,
    init_context, init_logger, init_wear_leveling_context, path_bytes,
    powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear},
};
use littlefs_rust_core::{
    Lfs, LfsConfig, LfsDir, LfsFile, LfsInfo, lfs_dir_close, lfs_dir_open, lfs_dir_read,
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write, lfs_format, lfs_mkdir, lfs_mount,
    lfs_remove, lfs_rename, lfs_stat, lfs_unmount,
};
use littlefs_rust_core::{
    error::Error,
    lfs_type::lfs_type::{LFS_TYPE_DIR, LFS_TYPE_REG},
};

// --- test_move_nop ---
// Rename to self is legal
#[test]
fn test_move_nop() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let hi = path_bytes("hi");
    assert_ok(lfs_mkdir(lfs, hi.as_c_str()));
    assert_ok(lfs_rename(lfs, hi.as_c_str(), hi.as_c_str()));

    let hi_hi = path_bytes("hi/hi");
    assert_ok(lfs_mkdir(lfs, hi_hi.as_c_str()));
    assert_ok(lfs_rename(lfs, hi_hi.as_c_str(), hi_hi.as_c_str()));

    let hi_hi_hi = c"hi/hi/hi";
    assert_ok(lfs_mkdir(lfs, hi_hi_hi));
    assert_ok(lfs_rename(lfs, hi_hi_hi, hi_hi_hi));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, hi_hi_hi, info));
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "hi");
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    assert_ok(lfs_unmount(lfs));
}

// --- test_move_file ---
// Cross-dir rename a/hello -> c/hello
#[test]
fn test_move_file() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("d").as_c_str()));

    let a_hello = path_bytes("a/hello");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        a_hello.as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n1 = lfs_file_write(lfs, file, b"hola\n".as_ptr() as *const core::ffi::c_void, 5);
    assert_eq!(n1, Ok(5));
    let n2 = lfs_file_write(
        lfs,
        file,
        b"bonjour\n".as_ptr() as *const core::ffi::c_void,
        8,
    );
    assert_eq!(n2, Ok(8));
    let n3 = lfs_file_write(
        lfs,
        file,
        b"ohayo\n".as_ptr() as *const core::ffi::c_void,
        6,
    );
    assert_eq!(n3, Ok(6));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        c"a/hello",
        path_bytes("c/hello").as_c_str(),
    ));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let a_names = dir_entry_names(lfs, &env.config, "a").unwrap();
    assert_eq!(a_names.len(), 0);
    let c_names = dir_entry_names(lfs, &env.config, "c").unwrap();
    assert_eq!(c_names.len(), 1);
    assert_eq!(c_names[0], "hello");

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, path_bytes("c/hello").as_c_str(), info));
    assert_eq!(info.size, 5 + 8 + 6);

    let info_dummy = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    let err_a = lfs_stat(lfs, c"a/hello", info_dummy);
    assert_err(Error::NoEntry, err_a);
    let err_b = lfs_stat(lfs, path_bytes("b/hello").as_c_str(), info_dummy);
    assert_err(Error::NoEntry, err_b);

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("c/hello").as_c_str(),
        LFS_O_RDONLY,
    ));
    let mut buf = [0u8; 32];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(5 + 8 + 6));
    assert_eq!(&buf[..5], b"hola\n");
    assert_eq!(&buf[5..13], b"bonjour\n");
    assert_eq!(&buf[13..19], b"ohayo\n");
    assert_ok(lfs_file_close(lfs, file));

    let file_dummy = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    let err_d = lfs_file_open(
        lfs,
        file_dummy,
        path_bytes("d/hello").as_c_str(),
        LFS_O_RDONLY,
    );
    assert_err(Error::NoEntry, err_d);
    assert_ok(lfs_unmount(lfs));
}

// --- test_move_dir ---
// Cross-dir rename a/hi -> c/hi
#[test]
fn test_move_dir() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("d").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/hola").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/bonjour").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/ohayo").as_c_str()));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("a/hi").as_c_str(),
        path_bytes("c/hi").as_c_str(),
    ));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let names = dir_entry_names(lfs, &env.config, "c/hi").unwrap();
    assert!(names.contains(&"bonjour".to_string()));
    assert!(names.contains(&"hola".to_string()));
    assert!(names.contains(&"ohayo".to_string()));
    assert_ok(lfs_unmount(lfs));
}

// --- test_move_state_stealing ---
// Chain a->b->c->d then remove b,c
#[test]
fn test_move_state_stealing() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("d").as_c_str()));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"a/hello",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n1 = lfs_file_write(lfs, file, b"hola\n".as_ptr() as *const core::ffi::c_void, 5);
    assert_eq!(n1, Ok(5));
    let n2 = lfs_file_write(
        lfs,
        file,
        b"bonjour\n".as_ptr() as *const core::ffi::c_void,
        8,
    );
    assert_eq!(n2, Ok(8));
    let n3 = lfs_file_write(
        lfs,
        file,
        b"ohayo\n".as_ptr() as *const core::ffi::c_void,
        6,
    );
    assert_eq!(n3, Ok(6));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        c"a/hello",
        path_bytes("b/hello").as_c_str(),
    ));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("b/hello").as_c_str(),
        path_bytes("c/hello").as_c_str(),
    ));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("c/hello").as_c_str(),
        path_bytes("d/hello").as_c_str(),
    ));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_remove(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_remove(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("d/hello").as_c_str(),
        LFS_O_RDONLY,
    ));
    let mut buf = [0u8; 32];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(5 + 8 + 6));
    assert_eq!(&buf[..5], b"hola\n");
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

// --- test_move_create_delete_same ---
// Same-dir rename while files open
#[test]
fn test_move_create_delete_same() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let f1 = path_bytes("1.move_me");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        f1.as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_close(lfs, file));

    let f0 = path_bytes("0.before");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        f0.as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"test.1".as_ptr() as *const core::ffi::c_void, 6);
    assert_eq!(n, Ok(6));
    assert_ok(lfs_file_close(lfs, file));

    let f2 = path_bytes("2.in_between");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        f2.as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"test.2".as_ptr() as *const core::ffi::c_void, 6);
    assert_eq!(n, Ok(6));
    assert_ok(lfs_file_close(lfs, file));

    let f4 = path_bytes("4.after");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        f4.as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"test.3".as_ptr() as *const core::ffi::c_void, 6);
    assert_eq!(n, Ok(6));
    assert_ok(lfs_file_close(lfs, file));

    let fa = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    let fb = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    let fc = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        fa,
        f0.as_c_str(),
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    assert_ok(lfs_file_open(
        lfs,
        fb,
        f2.as_c_str(),
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    assert_ok(lfs_file_open(
        lfs,
        fc,
        f4.as_c_str(),
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    let _ = lfs_file_write(lfs, fa, b"test.4".as_ptr() as *const core::ffi::c_void, 6);
    let _ = lfs_file_write(lfs, fb, b"test.5".as_ptr() as *const core::ffi::c_void, 6);
    let _ = lfs_file_write(lfs, fc, b"test.6".as_ptr() as *const core::ffi::c_void, 6);

    assert_ok(lfs_rename(
        lfs,
        path_bytes("1.move_me").as_c_str(),
        path_bytes("3.move_me").as_c_str(),
    ));

    assert_ok(lfs_file_close(lfs, fa));
    assert_ok(lfs_file_close(lfs, fb));
    assert_ok(lfs_file_close(lfs, fc));

    let names = dir_entry_names(lfs, &env.config, "/").unwrap();
    assert!(names.contains(&"0.before".to_string()));
    assert!(names.contains(&"2.in_between".to_string()));
    assert!(names.contains(&"3.move_me".to_string()));
    assert!(names.contains(&"4.after".to_string()));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("0.before").as_c_str(),
        LFS_O_RDONLY,
    ));
    let mut buf = [0u8; 16];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 16);
    assert_eq!(n, Ok(6));
    assert_eq!(&buf[..6], b"test.4");
    assert_ok(lfs_file_close(lfs, file));

    assert_ok(lfs_unmount(lfs));
}

// --- test_move_create_delete_delete_same ---
#[test]
fn test_move_create_delete_delete_same() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("1.move_me").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("3.move_me").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(
        lfs,
        file,
        b"remove me".as_ptr() as *const core::ffi::c_void,
        9,
    );
    assert_eq!(n, Ok(9));
    assert_ok(lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("0.before").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"test.1".as_ptr() as *const core::ffi::c_void, 6);
    assert_eq!(n, Ok(6));
    assert_ok(lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("2.in_between").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"test.2".as_ptr() as *const core::ffi::c_void, 6);
    assert_eq!(n, Ok(6));
    assert_ok(lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("4.after").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"test.3".as_ptr() as *const core::ffi::c_void, 6);
    assert_eq!(n, Ok(6));
    assert_ok(lfs_file_close(lfs, file));

    let fa = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    let fb = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    let fc = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        fa,
        path_bytes("0.before").as_c_str(),
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    assert_ok(lfs_file_open(
        lfs,
        fb,
        path_bytes("2.in_between").as_c_str(),
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    assert_ok(lfs_file_open(
        lfs,
        fc,
        path_bytes("4.after").as_c_str(),
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    let _ = lfs_file_write(lfs, fa, b"test.4".as_ptr() as *const core::ffi::c_void, 6);
    let _ = lfs_file_write(lfs, fb, b"test.5".as_ptr() as *const core::ffi::c_void, 6);
    let _ = lfs_file_write(lfs, fc, b"test.6".as_ptr() as *const core::ffi::c_void, 6);

    assert_ok(lfs_rename(
        lfs,
        path_bytes("1.move_me").as_c_str(),
        path_bytes("3.move_me").as_c_str(),
    ));

    assert_ok(lfs_file_close(lfs, fa));
    assert_ok(lfs_file_close(lfs, fb));
    assert_ok(lfs_file_close(lfs, fc));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, c"3.move_me", info));
    assert_eq!(info.size, 0);

    assert_ok(lfs_unmount(lfs));
}

// --- test_move_create_delete_different ---
// Cross-dir rename with overwrite
#[test]
fn test_move_create_delete_different() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("dir.1").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("dir.2").as_c_str()));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("dir.1/1.move_me").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("dir.2/1.move_me").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(
        lfs,
        file,
        b"remove me".as_ptr() as *const core::ffi::c_void,
        9,
    );
    assert_eq!(n, Ok(9));
    assert_ok(lfs_file_close(lfs, file));

    assert_ok(lfs_rename(
        lfs,
        path_bytes("dir.1/1.move_me").as_c_str(),
        path_bytes("dir.2/1.move_me").as_c_str(),
    ));

    let names = dir_entry_names(lfs, &env.config, "dir.2").unwrap();
    assert!(names.contains(&"1.move_me".to_string()));
    assert_ok(lfs_unmount(lfs));
}

// --- Corruption: file rename ---

// Upstream: test_move_file_corrupt_source
// Corrupt source dir after rename; rename should stick.
#[test]
fn test_move_file_corrupt_source() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("d").as_c_str()));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"a/hello",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    assert_eq!(
        lfs_file_write(lfs, file, b"hola\n"),
        Ok(5)
    );
    assert_eq!(
        lfs_file_write(
            lfs,
            file,
            b"bonjour\n",
        ),
        Ok(8)
    );
    assert_eq!(
        lfs_file_write(
            lfs,
            file,
            b"ohayo\n",
        ),
        Ok(6)
    );
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        c"a/hello",
        path_bytes("c/hello").as_c_str(),
    ));

    let ablock = dir_block(lfs, "a");
    assert_ok(lfs_unmount(lfs));
    corrupt_block(&mut env, ablock);

    assert_ok(lfs_mount(lfs, &env.config));
    let a_names = dir_entry_names(lfs, &env.config, "a").unwrap();
    assert_eq!(a_names.len(), 0);
    let c_names = dir_entry_names(lfs, &env.config, "c").unwrap();
    assert_eq!(c_names.len(), 1);
    assert_eq!(c_names[0], "hello");

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, c"c/hello", info));
    assert_eq!(unsafe { (*info).size }, 5 + 8 + 6);

    assert_err(Error::NoEntry, lfs_stat(lfs, c"a/hello", info));
    assert_err(
        Error::NoEntry,
        lfs_stat(lfs, path_bytes("b/hello").as_c_str(), info),
    );

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("c/hello").as_c_str(),
        LFS_O_RDONLY,
    ));
    let mut buf = [0u8; 32];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(5 + 8 + 6));
    assert_eq!(&buf[..5], b"hola\n");
    assert_eq!(&buf[5..13], b"bonjour\n");
    assert_eq!(&buf[13..19], b"ohayo\n");
    assert_ok(lfs_file_close(lfs, file));

    assert_err(
        Error::NoEntry,
        lfs_file_open(lfs, file, path_bytes("d/hello").as_c_str(), LFS_O_RDONLY),
    );
    assert_ok(lfs_unmount(lfs));
}

// Upstream: test_move_file_corrupt_source_dest
// Corrupt both source and dest dirs; rename should roll back.
#[test]
fn test_move_file_corrupt_source_dest() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("d").as_c_str()));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"a/hello",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    assert_eq!(
        lfs_file_write(lfs, file, b"hola\n"),
        Ok(5)
    );
    assert_eq!(
        lfs_file_write(
            lfs,
            file,
            b"bonjour\n"
        ),
        Ok(8)
    );
    assert_eq!(
        lfs_file_write(
            lfs,
            file,
            b"ohayo\n"
        ),
        Ok(6)
    );
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        c"a/hello",
        path_bytes("c/hello").as_c_str(),
    ));

    let ablock = dir_block(lfs, "a");
    let cblock = dir_block(lfs, "c");
    assert_ok(lfs_unmount(lfs));
    corrupt_block(&mut env, ablock);
    corrupt_block(&mut env, cblock);

    assert_ok(lfs_mount(lfs, &env.config));
    let a_names = dir_entry_names(lfs, &env.config, "a").unwrap();
    assert_eq!(a_names.len(), 1);
    assert_eq!(a_names[0], "hello");
    let c_names = dir_entry_names(lfs, &env.config, "c").unwrap();
    assert_eq!(c_names.len(), 0);

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, c"a/hello", info));
    assert_eq!(unsafe { (*info).size }, 5 + 8 + 6);

    assert_err(Error::NoEntry, lfs_stat(lfs, c"b/hello", info));
    assert_err(Error::NoEntry, lfs_stat(lfs, c"c/hello", info));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, c"a/hello", LFS_O_RDONLY));
    let mut buf = [0u8; 32];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(5 + 8 + 6));
    assert_eq!(&buf[..5], b"hola\n");
    assert_eq!(&buf[5..13], b"bonjour\n");
    assert_eq!(&buf[13..19], b"ohayo\n");
    assert_ok(lfs_file_close(lfs, file));

    assert_err(
        Error::NoEntry,
        lfs_file_open(lfs, file, path_bytes("d/hello").as_c_str(), LFS_O_RDONLY),
    );
    assert_ok(lfs_unmount(lfs));
}

// Upstream: test_move_file_after_corrupt
// Corrupt both, then redo rename; rename should succeed.
#[test]
fn test_move_file_after_corrupt() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("d").as_c_str()));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"a/hello",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    assert_eq!(
        lfs_file_write(lfs, file, b"hola\n"),
        Ok(5)
    );
    assert_eq!(
        lfs_file_write(
            lfs,
            file,
            b"bonjour\n"
        ),
        Ok(8)
    );
    assert_eq!(
        lfs_file_write(
            lfs,
            file,
            b"ohayo\n"
        ),
        Ok(6)
    );
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        c"a/hello",
        path_bytes("c/hello").as_c_str(),
    ));

    let ablock = dir_block(lfs, "a");
    let cblock = dir_block(lfs, "c");
    assert_ok(lfs_unmount(lfs));
    corrupt_block(&mut env, ablock);
    corrupt_block(&mut env, cblock);

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        c"a/hello",
        path_bytes("c/hello").as_c_str(),
    ));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let a_names = dir_entry_names(lfs, &env.config, "a").unwrap();
    assert_eq!(a_names.len(), 0);
    let c_names = dir_entry_names(lfs, &env.config, "c").unwrap();
    assert_eq!(c_names.len(), 1);
    assert_eq!(c_names[0], "hello");

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, c"c/hello", info));
    assert_eq!({ (*info).size }, 5 + 8 + 6);

    assert_err(Error::NoEntry, lfs_stat(lfs, c"a/hello", info));
    assert_err(Error::NoEntry, lfs_stat(lfs, c"b/hello", info));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("c/hello").as_c_str(),
        LFS_O_RDONLY,
    ));
    let mut buf = [0u8; 32];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(5 + 8 + 6));
    assert_eq!(&buf[..5], b"hola\n");
    assert_eq!(&buf[5..13], b"bonjour\n");
    assert_eq!(&buf[13..19], b"ohayo\n");
    assert_ok(lfs_file_close(lfs, file));

    assert_err(
        Error::NoEntry,
        lfs_file_open(lfs, file, path_bytes("d/hello").as_c_str(), LFS_O_RDONLY),
    );
    assert_ok(lfs_unmount(lfs));
}

// --- test_move_reentrant_file ---
// Power-loss at rename points; verify FS consistent after each simulated power loss.
#[test]
fn test_move_reentrant_file() {
    init_logger();
    let mut env = powerloss_config(128);
    init_powerloss_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_mkdir(lfs, path_bytes("dir.1").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("dir.2").as_c_str()));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("dir.1/1.move_me").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    let snapshot = env.snapshot();
    let path_src = path_bytes("dir.1/1.move_me");
    let path_dst = path_bytes("dir.2/1.move_me");

    let result = run_powerloss_linear(
        &mut env,
        &snapshot,
        128,
        |lfs_ptr, config| {
            lfs_mount(lfs_ptr, config)?;

            let err = lfs_rename(lfs_ptr, path_src.as_c_str(), path_dst.as_c_str());
            if let Err(err) = err {
                let _ = lfs_unmount(lfs_ptr);
                return Err(err);
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
    result.expect("test_move_reentrant_file should complete");
}

// Upstream: test_move_dir_corrupt_source
// Corrupt source dir after dir rename; rename should stick.
#[test]
fn test_move_dir_corrupt_source() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_mkdir(lfs, c"a"));
    assert_ok(lfs_mkdir(lfs, c"b"));
    assert_ok(lfs_mkdir(lfs, c"c"));
    assert_ok(lfs_mkdir(lfs, c"d"));
    assert_ok(lfs_mkdir(lfs, c"a/hi"));
    assert_ok(lfs_mkdir(lfs, c"a/hi/hola"));
    assert_ok(lfs_mkdir(lfs, c"a/hi/bonjour"));
    assert_ok(lfs_mkdir(lfs, c"a/hi/ohayo"));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("a/hi").as_c_str(),
        path_bytes("c/hi").as_c_str(),
    ));

    let ablock = dir_block(lfs, "a");
    assert_ok(lfs_unmount(lfs));
    corrupt_block(&mut env, ablock);

    assert_ok(lfs_mount(lfs, &env.config));
    let a_names = dir_entry_names(lfs, &env.config, "a").unwrap();
    assert_eq!(a_names.len(), 0);
    let c_names = dir_entry_names(lfs, &env.config, "c").unwrap();
    assert_eq!(c_names.len(), 1);
    assert_eq!(c_names[0], "hi");

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, c"c/hi", info));
    assert_eq!( info.type_ , LFS_TYPE_DIR as u8);

    assert_err(Error::NoEntry, lfs_stat(lfs, c"a/hi", info));
    assert_err(Error::NoEntry, lfs_stat(lfs, c"b/hi", info));

    let hi_names = dir_entry_names(lfs, &env.config, "c/hi").unwrap();
    assert!(hi_names.contains(&"hola".to_string()));
    assert!(hi_names.contains(&"bonjour".to_string()));
    assert!(hi_names.contains(&"ohayo".to_string()));

    assert_err(Error::NoEntry, lfs_stat(lfs, c"d/hi", info));
    assert_ok(lfs_unmount(lfs));
}

// Upstream: test_move_dir_corrupt_source_dest
// Corrupt both source and dest; dir rename should roll back.
#[test]
fn test_move_dir_corrupt_source_dest() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("d").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/hola").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/bonjour").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/ohayo").as_c_str()));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("a/hi").as_c_str(),
        path_bytes("c/hi").as_c_str(),
    ));

    let ablock = dir_block(lfs, "a");
    let cblock = dir_block(lfs, "c");
    assert_ok(lfs_unmount(lfs));
    corrupt_block(&mut env, ablock);
    corrupt_block(&mut env, cblock);

    assert_ok(lfs_mount(lfs, &env.config));
    let a_names = dir_entry_names(lfs, &env.config, "a").unwrap();
    assert_eq!(a_names.len(), 1);
    assert_eq!(a_names[0], "hi");
    let c_names = dir_entry_names(lfs, &env.config, "c").unwrap();
    assert_eq!(c_names.len(), 0);

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, c"a/hi", info));
    assert_eq!( { (*info).type_ }, LFS_TYPE_DIR as u8);

    assert_err(Error::NoEntry, lfs_stat(lfs, c"b/hi", info));
    assert_err(Error::NoEntry, lfs_stat(lfs, c"c/hi", info));

    let hi_names = dir_entry_names(lfs, &env.config, "a/hi").unwrap();
    assert!(hi_names.contains(&"hola".to_string()));
    assert!(hi_names.contains(&"bonjour".to_string()));
    assert!(hi_names.contains(&"ohayo".to_string()));

    assert_err(Error::NoEntry, lfs_stat(lfs, c"d/hi", info));
    assert_ok(lfs_unmount(lfs));
}

// Upstream: test_move_dir_after_corrupt
// Corrupt both, then redo dir rename; rename should succeed.
#[test]
fn test_move_dir_after_corrupt() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("d").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/hola").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/bonjour").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/ohayo").as_c_str()));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("a/hi").as_c_str(),
        path_bytes("c/hi").as_c_str(),
    ));

    let ablock = dir_block(lfs, "a");
    let cblock = dir_block(lfs, "c");
    assert_ok(lfs_unmount(lfs));
    corrupt_block(&mut env, ablock);
    corrupt_block(&mut env, cblock);

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("a/hi").as_c_str(),
        path_bytes("c/hi").as_c_str(),
    ));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let a_names = dir_entry_names(lfs, &env.config, "a").unwrap();
    assert_eq!(a_names.len(), 0);
    let c_names = dir_entry_names(lfs, &env.config, "c").unwrap();
    assert_eq!(c_names.len(), 1);
    assert_eq!(c_names[0], "hi");

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, c"c/hi", info));
    assert_eq!( info.type_ , LFS_TYPE_DIR as u8);

    assert_err(Error::NoEntry, lfs_stat(lfs, c"a/hi", info));
    assert_err(Error::NoEntry, lfs_stat(lfs, c"b/hi", info));

    let hi_names = dir_entry_names(lfs, &env.config, "c/hi").unwrap();
    assert!(hi_names.contains(&"hola".to_string()));
    assert!(hi_names.contains(&"bonjour".to_string()));
    assert!(hi_names.contains(&"ohayo".to_string()));

    assert_err(Error::NoEntry, lfs_stat(lfs, c"d/hi", info));
    assert_ok(lfs_unmount(lfs));
}

// --- test_reentrant_dir ---
// Power-loss at cross-dir dir rename points; verify FS consistent after each.
#[test]
fn test_reentrant_dir() {
    init_logger();
    let mut env = powerloss_config(128);
    init_powerloss_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("b").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("c").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("d").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("a/hi/hola").as_c_str()));
    assert_ok(lfs_unmount(lfs));

    let snapshot = env.snapshot();
    let path_src = path_bytes("a/hi");
    let path_dst = path_bytes("c/hi");

    let result = run_powerloss_linear(
        &mut env,
        &snapshot,
        128,
        |lfs_ptr, config| {
            lfs_mount(lfs_ptr, config)?;

            let err = lfs_rename(lfs_ptr, path_src.as_c_str(), path_dst.as_c_str());
            if let Err(err) = err {
                let _ = lfs_unmount(lfs_ptr);
                return Err(err);
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
    result.expect("test_reentrant_dir should complete");
}

// --- Missing upstream stubs ---

/// Upstream: [cases.test_move_fix_relocation]
/// RELOCATIONS in 0..4, ERASE_CYCLES=0xffffffff. Force dir relocation via set_wear, then rename.
#[test]
fn test_move_fix_relocation() {
    init_logger();
    const ERASE_CYCLES: u32 = 0xffffffff;
    let mut env = config_with_wear_leveling(256, ERASE_CYCLES);
    init_wear_leveling_context(&mut env);

    for relocations in 0..4u32 {
        let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
        assert_ok(lfs_format(lfs, &env.config));
        assert_ok(lfs_mount(lfs, &env.config));

        assert_ok(lfs_mkdir(lfs, path_bytes("parent").as_c_str()));
        assert_ok(lfs_mkdir(lfs, path_bytes("parent/child").as_c_str()));

        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path_bytes("parent/1.move_me").as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ));
        assert_eq!(
            lfs_file_write(
                lfs,
                file,
                b"move me\0"
            ),
            Ok(8)
        );
        assert_ok(lfs_file_close(lfs, file));

        for (path, content) in [
            ("parent/0.before", b"test.1\0"),
            ("parent/2.after", b"test.2\0"),
            ("parent/child/0.before", b"test.3\0"),
            ("parent/child/2.after", b"test.4\0"),
        ] {
            assert_ok(lfs_file_open(
                lfs,
                file,
                path_bytes(path).as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ));
            assert_eq!(
                lfs_file_write(lfs, file, content),
                Ok(7)
            );
            assert_ok(lfs_file_close(lfs, file));
        }

        let mut files = unsafe {
            [
                core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init(),
                core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init(),
                core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init(),
                core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init(),
            ]
        };
        let paths = [
            "parent/0.before",
            "parent/2.after",
            "parent/child/0.before",
            "parent/child/2.after",
        ];
        for (f, p) in files.iter_mut().zip(paths) {
            assert_ok(lfs_file_open(
                lfs,
                f,
                path_bytes(p).as_c_str(),
                LFS_O_WRONLY | LFS_O_TRUNC,
            ));
        }
        for (f, content) in
            files
                .iter_mut()
                .zip([b"test.5\0", b"test.6\0", b"test.7\0", b"test.8\0"])
        {
            assert_eq!(
                lfs_file_write(lfs, f, content),
                Ok(7)
            );
        }

        if relocations & 1 != 0 {
            let pair = dir_pair(lfs, "parent");
            env.bd.set_wear(pair[0], 0xffffffff);
            env.bd.set_wear(pair[1], 0xffffffff);
        }
        if relocations & 2 != 0 {
            let pair = dir_pair(lfs, "parent/child");
            env.bd.set_wear(pair[0], 0xffffffff);
            env.bd.set_wear(pair[1], 0xffffffff);
        }

        assert_ok(lfs_rename(
            lfs,
            path_bytes("parent/1.move_me").as_c_str(),
            path_bytes("parent/child/1.move_me").as_c_str(),
        ));

        for f in &mut files {
            assert_ok(lfs_file_close(lfs, f));
        }

        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_ok(lfs_dir_open(lfs, dir, path_bytes("parent").as_c_str()));
        let expect_parent = ["0.before", "2.after", "child"];
        let mut idx = 0;
        loop {
            let n = lfs_dir_read(lfs, dir, info);
            assert!(n.is_ok());
            if n == Ok(0) {
                break;
            }
            let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
            let name = core::str::from_utf8(&info.name[..nul]).unwrap();
            if name == "." || name == ".." {
                continue;
            }
            assert!(idx < expect_parent.len(), "extra entry: {name}");
            assert_eq!(name, expect_parent[idx]);
            if idx < 2 {
                assert_eq!(info.type_, LFS_TYPE_REG as u8);
                assert_eq!(info.size, 7);
            } else {
                assert_eq!(info.type_, LFS_TYPE_DIR as u8);
            }
            idx += 1;
        }
        assert_eq!(idx, expect_parent.len());
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(0));
        assert_ok(lfs_dir_close(lfs, dir));

        assert_ok(lfs_dir_open(
            lfs,
            dir,
            path_bytes("parent/child").as_c_str(),
        ));
        let expect_child = ["0.before", "1.move_me", "2.after"];
        let mut idx = 0;
        loop {
            let n = lfs_dir_read(lfs, dir, info);
            assert!(n.is_ok());
            if n == Ok(0) {
                break;
            }
            let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
            let name = core::str::from_utf8(&info.name[..nul]).unwrap();
            if name == "." || name == ".." {
                continue;
            }
            assert!(idx < expect_child.len(), "extra entry: {name}");
            assert_eq!(name, expect_child[idx]);
            assert_eq!(info.type_, LFS_TYPE_REG as u8);
            assert_eq!(info.size, if name == "1.move_me" { 8 } else { 7 });
            idx += 1;
        }
        assert_eq!(idx, expect_child.len());
        assert_ok(lfs_dir_close(lfs, dir));

        let mut buf = [0u8; 32];
        for (path, expected) in [
            ("parent/0.before", b"test.5\0"),
            ("parent/2.after", b"test.6\0"),
            ("parent/child/0.before", b"test.7\0"),
            ("parent/child/2.after", b"test.8\0"),
        ] {
            assert_ok(lfs_file_open(
                lfs,
                file,
                path_bytes(path).as_c_str(),
                LFS_O_RDONLY,
            ));
            assert_eq!(
                lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 7,),
                Ok(7)
            );
            assert_eq!(&buf[..6], &expected[..6]);
            assert_ok(lfs_file_close(lfs, file));
        }

        assert_ok(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_move_fix_relocation_predecessor]
/// RELOCATIONS in 0..8. Move sibling/1.move_me -> child/1.move_me with forced relocations.
#[test]
fn test_move_fix_relocation_predecessor() {
    init_logger();
    const ERASE_CYCLES: u32 = 0xffffffff;
    let mut env = config_with_wear_leveling(256, ERASE_CYCLES);
    init_wear_leveling_context(&mut env);

    for relocations in 0..8u32 {
        let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
        assert_ok(lfs_format(lfs, &env.config));
        assert_ok(lfs_mount(lfs, &env.config));

        assert_ok(lfs_mkdir(lfs, path_bytes("parent").as_c_str()));
        assert_ok(lfs_mkdir(lfs, path_bytes("parent/child").as_c_str()));
        assert_ok(lfs_mkdir(lfs, path_bytes("parent/sibling").as_c_str()));

        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path_bytes("parent/sibling/1.move_me").as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT,
        ));
        assert_eq!(
            lfs_file_write(
                lfs,
                file,
                b"move me\0",
            ),
            Ok(8)
        );
        assert_ok(lfs_file_close(lfs, file));

        for (path, content) in [
            ("parent/sibling/0.before", b"test.1\0"),
            ("parent/sibling/2.after", b"test.2\0"),
            ("parent/child/0.before", b"test.3\0"),
            ("parent/child/2.after", b"test.4\0"),
        ] {
            assert_ok(lfs_file_open(
                lfs,
                file,
                path_bytes(path).as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ));
            assert_eq!(
                lfs_file_write(lfs, file, content),
                Ok(7)
            );
            assert_ok(lfs_file_close(lfs, file));
        }

        let mut files = unsafe {
            [
                core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init(),
                core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init(),
                core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init(),
                core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init(),
            ]
        };
        let paths = [
            "parent/sibling/0.before",
            "parent/sibling/2.after",
            "parent/child/0.before",
            "parent/child/2.after",
        ];
        for (f, p) in files.iter_mut().zip(paths) {
            assert_ok(lfs_file_open(
                lfs,
                f,
                path_bytes(p).as_c_str(),
                LFS_O_WRONLY | LFS_O_TRUNC,
            ));
        }
        for (f, content) in
            files
                .iter_mut()
                .zip([b"test.5\0", b"test.6\0", b"test.7\0", b"test.8\0"])
        {
            assert_eq!(
                lfs_file_write(lfs, f, content),
                Ok(7)
            );
        }

        if relocations & 1 != 0 {
            let pair = dir_pair(lfs, "parent");
            env.bd.set_wear(pair[0], 0xffffffff);
            env.bd.set_wear(pair[1], 0xffffffff);
        }
        if relocations & 2 != 0 {
            let pair = dir_pair(lfs, "parent/sibling");
            env.bd.set_wear(pair[0], 0xffffffff);
            env.bd.set_wear(pair[1], 0xffffffff);
        }
        if relocations & 4 != 0 {
            let pair = dir_pair(lfs, "parent/child");
            env.bd.set_wear(pair[0], 0xffffffff);
            env.bd.set_wear(pair[1], 0xffffffff);
        }

        assert_ok(lfs_rename(
            lfs,
            path_bytes("parent/sibling/1.move_me").as_c_str(),
            path_bytes("parent/child/1.move_me").as_c_str(),
        ));

        for f in &mut files {
            assert_ok(lfs_file_close(lfs, f));
        }

        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_ok(lfs_dir_open(
            lfs,
            dir,
            path_bytes("parent/sibling").as_c_str(),
        ));
        // Skip . and ..
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(1));
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(1));
        let expect_sibling = ["0.before", "2.after"];
        for name in expect_sibling {
            assert_eq!(lfs_dir_read(lfs, dir, info), Ok(1));
            let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
            assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), name);
            assert_eq!(info.type_, LFS_TYPE_REG as u8);
            assert_eq!(info.size, 7);
        }
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(0));
        assert_ok(lfs_dir_close(lfs, dir));

        assert_ok(lfs_dir_open(
            lfs,
            dir,
            path_bytes("parent/child").as_c_str(),
        ));
        // Skip . and ..
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(1));
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(1));
        let expect_child = ["0.before", "1.move_me", "2.after"];
        for name in expect_child.iter() {
            assert_eq!(lfs_dir_read(lfs, dir, info), Ok(1));
            let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
            assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
            assert_eq!(info.type_, LFS_TYPE_REG as u8);
            if *name == "1.move_me" {
                assert_eq!(info.size, 8);
            } else {
                assert_eq!(info.size, 7);
            }
        }
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(0));
        assert_ok(lfs_dir_close(lfs, dir));

        let mut buf = [0u8; 32];
        for (path, expected) in [
            ("parent/sibling/0.before", b"test.5\0"),
            ("parent/sibling/2.after", b"test.6\0"),
            ("parent/child/0.before", b"test.7\0"),
            ("parent/child/2.after", b"test.8\0"),
        ] {
            assert_ok(lfs_file_open(
                lfs,
                file,
                path_bytes(path).as_c_str(),
                LFS_O_RDONLY,
            ));
            assert_eq!(
                lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 7,),
                Ok(7)
            );
            assert_eq!(&buf[..6], &expected[..6]);
            assert_ok(lfs_file_close(lfs, file));
        }

        assert_ok(lfs_unmount(lfs));
    }
}
