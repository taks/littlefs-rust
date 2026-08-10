//! Entry/inline file corner case tests.
//!
//! Upstream: tests/test_entries.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_entries.toml
//!
//! Metadata spill (4 files × 200B inline) and directory compaction.

mod common;

use common::{
    LFS_O_CREAT, LFS_O_RDONLY, LFS_O_TRUNC, LFS_O_WRONLY, assert_ok, config_with_cache,
    init_context, init_logger, path_bytes,
};
use littlefs_rust_core::{
    Lfs, LfsFile, lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_write, lfs_format,
    lfs_mount, lfs_remove, lfs_unmount,
};

fn env_with_cache_512() -> common::TestEnv {
    config_with_cache(512, 128)
}

/// 2048 blocks matches upstream C test geometry (ERASE_COUNT=1M/512).
fn env_with_cache_512_2048_blocks() -> common::TestEnv {
    config_with_cache(512, 2048)
}

// --- test_entries_grow ---
#[test]
fn test_entries_grow() {
    init_logger();
    let mut env = env_with_cache_512();
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let buf = [b'c'; 1024];
    for i in 0..4 {
        let path = path_bytes(&format!("hi{i}"));
        let size = 20usize;
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_ok(lfs_file_close(lfs, file));
    }

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, c"hi1", LFS_O_RDONLY));
    let mut rb = [0u8; 256];
    let n = lfs_file_read(lfs, file, &mut rb[..20]);
    assert_eq!(n, Ok(20));
    assert_ok(lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"hi1",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..200]);
    assert_eq!(n, Ok(200));
    assert_ok(lfs_file_close(lfs, file));

    for i in 0..4 {
        let path = path_bytes(&format!("hi{i}"));
        let size = if i == 1 { 200 } else { 20 };
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_eq!(&rb[..size], &buf[..size]);
        assert_ok(lfs_file_close(lfs, file));
    }

    assert_ok(lfs_unmount(lfs));
}

// --- test_entries_shrink ---
#[test]
fn test_entries_shrink() {
    init_logger();
    let mut env = env_with_cache_512();
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let buf = [b'c'; 1024];
    for i in 0..4 {
        let path = path_bytes(&format!("hi{i}"));
        let size = if i == 1 { 200 } else { 20 };
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_ok(lfs_file_close(lfs, file));
    }

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, c"hi1", LFS_O_RDONLY));
    let mut rb = [0u8; 256];
    let n = lfs_file_read(lfs, file, &mut rb[..200]);
    assert_eq!(n, Ok(200));
    assert_ok(lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"hi1",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..20]);
    assert_eq!(n, Ok(20));
    assert_ok(lfs_file_close(lfs, file));

    for i in 0..4 {
        let path = path_bytes(&format!("hi{i}"));
        let size = 20;
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_eq!(&rb[..size], &buf[..size]);
        assert_ok(lfs_file_close(lfs, file));
    }

    assert_ok(lfs_unmount(lfs));
}

// --- test_entries_spill ---
#[test]
fn test_entries_spill() {
    init_logger();
    let mut env = env_with_cache_512();
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let buf = [b'c'; 256];
    for i in 0..4 {
        let path = path_bytes(&format!("hi{i}"));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..200]);
        assert_eq!(n, Ok(200));
        assert_ok(lfs_file_close(lfs, file));
    }

    let mut rb = [0u8; 256];
    for i in 0..4 {
        let path = path_bytes(&format!("hi{i}"));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..200]);
        assert_eq!(n, Ok(200));
        assert_eq!(&rb[..200], &buf[..200]);
        assert_ok(lfs_file_close(lfs, file));
    }

    assert_ok(lfs_unmount(lfs));
}

// --- test_entries_push_spill ---
#[test]
fn test_entries_push_spill() {
    init_logger();
    let mut env = env_with_cache_512();
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let buf = [b'c'; 256];
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"hi0",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..200]);
    assert_eq!(n, Ok(200));
    assert_ok(lfs_file_close(lfs, file));

    for i in 1..4 {
        let path = path_bytes(&format!("hi{i}"));
        let size = if i == 1 { 20 } else { 200 };
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_ok(lfs_file_close(lfs, file));
    }

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, c"hi1", LFS_O_RDONLY));
    let mut rb = [0u8; 256];
    let n = lfs_file_read(lfs, file, &mut rb[..20]);
    assert_eq!(n, Ok(20));
    assert_ok(lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"hi1",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..200]);
    assert_eq!(n, Ok(200));
    assert_ok(lfs_file_close(lfs, file));

    for i in 0..4 {
        let path = path_bytes(&format!("hi{i}"));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..200]);
        assert_eq!(n, Ok(200));
        assert_ok(lfs_file_close(lfs, file));
    }

    assert_ok(lfs_unmount(lfs));
}

// --- test_entries_drop ---
#[test]
fn test_entries_drop() {
    init_logger();
    let mut env = env_with_cache_512();
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let buf = [b'c'; 256];
    for i in 0..4 {
        let path = path_bytes(&format!("hi{i}"));
        let size = if i == 1 { 200 } else { 20 };
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
        ));
        let n = lfs_file_write(lfs, file, &buf[..size]);
        assert_eq!(n, Ok(size as u32));
        assert_ok(lfs_file_close(lfs, file));
    }

    assert_ok(lfs_remove(lfs, c"hi1"));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        c"hi1",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &buf[..20]);
    assert_eq!(n, Ok(20));
    assert_ok(lfs_file_close(lfs, file));

    let mut rb = [0u8; 256];
    for i in 0..4 {
        let path = path_bytes(&format!("hi{i}"));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let n = lfs_file_read(lfs, file, &mut rb[..20]);
        assert_eq!(n, Ok(20));
        assert_ok(lfs_file_close(lfs, file));
    }

    assert_ok(lfs_unmount(lfs));
}

// --- test_entries_create_too_big ---
// Upstream: [cases.test_entries_create_too_big]
#[test]
fn test_entries_create_too_big() {
    init_logger();
    let mut env = env_with_cache_512();
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = path_bytes(&"m".repeat(200));
    let size = 400usize;
    let wbuf = [b'c'; 1024];
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &wbuf[..size]);
    assert_eq!(n, Ok(size as u32));
    assert_ok(lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    let mut rbuf = [0u8; 1024];
    let n = lfs_file_read(lfs, file, &mut rbuf[..size]);
    assert_eq!(n, Ok(size as u32));
    assert_eq!(&rbuf[..size], &wbuf[..size]);
    assert_ok(lfs_file_close(lfs, file));

    assert_ok(lfs_unmount(lfs));
}

// --- test_entries_resize_too_big ---
// Upstream: [cases.test_entries_resize_too_big]
// 200-byte path needs ample blocks; 2048 matches upstream geometry (ERASE_COUNT=1M/512).
#[test]
fn test_entries_resize_too_big() {
    init_logger();
    let mut env = env_with_cache_512_2048_blocks();
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = path_bytes(&"m".repeat(200));
    let wbuf = [b'c'; 1024];
    let mut rbuf = [0u8; 1024];

    // Create with 40 bytes
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &wbuf[..40]);
    assert_eq!(n, Ok(40));
    assert_ok(lfs_file_close(lfs, file));

    // Read 40 bytes
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    let n = lfs_file_read(lfs, file, &mut rbuf[..40]);
    assert_eq!(n, Ok(40));
    assert_eq!(&rbuf[..40], &wbuf[..40]);
    assert_ok(lfs_file_close(lfs, file));

    // Truncate and write 400 bytes
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    let n = lfs_file_write(lfs, file, &wbuf[..400]);
    assert_eq!(n, Ok(400));
    assert_ok(lfs_file_close(lfs, file));

    // Read 400 bytes
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    let n = lfs_file_read(lfs, file, &mut rbuf[..400]);
    assert_eq!(n, Ok(400));
    assert_eq!(&rbuf[..400], &wbuf[..400]);
    assert_ok(lfs_file_close(lfs, file));

    assert_ok(lfs_unmount(lfs));
}
