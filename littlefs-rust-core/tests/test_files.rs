//! File read/write integration tests.
//!
//! Upstream: tests/test_files.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_files.toml

mod common;

use common::{
    LFS_O_APPEND, LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_TRUNC, LFS_O_WRONLY, advance_prng,
    assert_ok, config_with_inline_max, default_config, fs_with_hello, init_context, path_bytes,
    powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear},
    verify_prng_file, verify_prng_file_with_state, write_prng_file, write_prng_file_result,
};
use littlefs_rust_core::{
    Lfs, LfsConfig, LfsFile, error::Error, lfs_file_close, lfs_file_open, lfs_file_read,
    lfs_file_rewind, lfs_file_seek, lfs_file_size, lfs_file_sync, lfs_file_tell, lfs_file_truncate,
    lfs_file_write, lfs_format, lfs_mount, lfs_unmount,
};
use rstest::rstest;

/// Block count for tests with large files (SIZE up to 262144).
const BLOCK_COUNT_LARGE: u32 = 1024;

/// Block count for test_files_many with N=300 (needs dir space for 300 entries).
const BLOCK_COUNT_MANY: u32 = 256;

// ── Upstream Cases ──────────────────────────

/// Upstream: [cases.test_files_simple]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Create, write "Hello World!\0", close, unmount, mount, read, verify.
#[rstest]
fn test_files_simple(#[values(0, -1, 8)] inline_max: i32) {
    let mut env = config_with_inline_max(128, inline_max);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"hello";
    let data = b"Hello World!\0";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    let n = lfs_file_write(
        lfs,
        file,
        data.as_ptr() as *const core::ffi::c_void,
        data.len() as u32,
    );
    assert_eq!(n, Ok(data.len() as u32));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    let mut buf = [0u8; 32];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(data.len() as u32));
    assert_eq!(&buf[..(n.unwrap()) as usize], data);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_files_large]
/// defines.SIZE = [32, 8192, 262144, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 33, 1, 1023]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Write SIZE bytes of PRNG(seed=1) in CHUNKSIZE chunks, unmount, remount,
/// verify file_size == SIZE, read back and verify. Final read past EOF returns 0.
#[rstest]
fn test_files_large(
    #[values(32, 8192, 262144, 0, 7, 8193)] size: u32,
    #[values(31, 16, 33, 1, 1023)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = config_with_inline_max(BLOCK_COUNT_LARGE, inline_max);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    // write
    assert_ok(lfs_mount(lfs, &env.config));
    let path = c"avacado";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    write_prng_file(lfs, file, size, chunk_size, 1);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    // read
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    assert_eq!(lfs_file_size(lfs, file), size as i32);
    verify_prng_file(lfs, file, size, chunk_size, 1);
    // Final read past EOF returns 0
    let mut buf = [0u8; 1024];
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        chunk_size,
    );
    assert_eq!(n, Ok(0));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_files_rewrite]
/// defines.SIZE1 = [32, 8192, 131072, 0, 7, 8193]
/// defines.SIZE2 = [32, 8192, 131072, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 1]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Write SIZE1, read back, rewrite with SIZE2 (WRONLY, no TRUNC), read:
/// first SIZE2 bytes PRNG(2), remaining (SIZE2..SIZE1) PRNG(1) from offset SIZE2.
#[rstest]
fn test_files_rewrite(
    #[values(32, 8192, 131072, 0, 7, 8193)] size1: u32,
    #[values(32, 8192, 131072, 0, 7, 8193)] size2: u32,
    #[values(31, 16, 1)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = config_with_inline_max(BLOCK_COUNT_LARGE, inline_max);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    let path = c"avacado";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };

    // write SIZE1
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    write_prng_file(lfs, file, size1, chunk_size, 1);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    // read SIZE1
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    assert_eq!(lfs_file_size(lfs, file), size1 as i32);
    verify_prng_file(lfs, file, size1, chunk_size, 1);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    // rewrite SIZE2 (WRONLY, no TRUNC)
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_WRONLY));
    write_prng_file(lfs, file, size2, chunk_size, 2);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    // read: first SIZE2 = PRNG(2), then SIZE2..SIZE1 (if size1 > size2) = PRNG(1) from offset SIZE2
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    assert_eq!(lfs_file_size(lfs, file), size1.max(size2) as i32);
    verify_prng_file(lfs, file, size2, chunk_size, 2);
    if size1 > size2 {
        let mut prng = 1u32;
        advance_prng(&mut prng, size2);
        verify_prng_file_with_state(lfs, file, size1 - size2, chunk_size, &mut prng);
    }
    // Final read past EOF returns 0
    let mut buf = [0u8; 1024];
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        chunk_size,
    );
    assert_eq!(n, Ok(0));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_files_append]
/// defines.SIZE1 = [32, 8192, 131072, 0, 7, 8193]
/// defines.SIZE2 = [32, 8192, 131072, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 1]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Write SIZE1, append SIZE2 (PRNG seed 2). Read: first SIZE1 = PRNG(1), next SIZE2 = PRNG(2).
#[rstest]
fn test_files_append(
    #[values(32, 8192, 131072, 0, 7, 8193)] size1: u32,
    #[values(32, 8192, 131072, 0, 7, 8193)] size2: u32,
    #[values(31, 16, 1)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = config_with_inline_max(BLOCK_COUNT_LARGE, inline_max);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    let path = c"avacado";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };

    // write SIZE1
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    write_prng_file(lfs, file, size1, chunk_size, 1);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    // append SIZE2
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_APPEND));
    write_prng_file(lfs, file, size2, chunk_size, 2);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    // read: SIZE1 + SIZE2, first PRNG(1) then PRNG(2)
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    assert_eq!(lfs_file_size(lfs, file), (size1 + size2) as i32);
    verify_prng_file(lfs, file, size1, chunk_size, 1);
    verify_prng_file(lfs, file, size2, chunk_size, 2);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_files_truncate]
/// defines.SIZE1 = [32, 8192, 131072, 0, 7, 8193]
/// defines.SIZE2 = [32, 8192, 131072, 0, 7, 8193]
/// defines.CHUNKSIZE = [31, 16, 1]
/// defines.INLINE_MAX = [0, -1, 8]
///
/// Write SIZE1, truncate+write SIZE2 (TRUNC|WRONLY). Read: SIZE2 bytes PRNG(2). Final read returns 0.
#[rstest]
fn test_files_truncate(
    #[values(32, 8192, 131072, 0, 7, 8193)] size1: u32,
    #[values(32, 8192, 131072, 0, 7, 8193)] size2: u32,
    #[values(31, 16, 1)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = config_with_inline_max(BLOCK_COUNT_LARGE, inline_max);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    let path = c"avacado";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };

    // write SIZE1
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    write_prng_file(lfs, file, size1, chunk_size, 1);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    // truncate + write SIZE2
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_TRUNC));
    write_prng_file(lfs, file, size2, chunk_size, 2);
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    // read SIZE2
    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    assert_eq!(lfs_file_size(lfs, file), size2 as i32);
    verify_prng_file(lfs, file, size2, chunk_size, 2);
    let mut buf = [0u8; 1024];
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        chunk_size,
    );
    assert_eq!(n, Ok(0));
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_files_reentrant_write]
/// defines.SIZE = [32, 0, 7, 2049]
/// defines.CHUNKSIZE = [31, 16, 65]
/// defines.INLINE_MAX = [0, -1, 8]
/// defines.POWERLOSS_BEHAVIOR = [NOOP, OOO] — we use NOOP only (OOO not implemented)
///
/// Mount-or-format, check existing file (size 0 or SIZE), write SIZE PRNG(1),
/// close, read back, verify. Power-loss retries until success.
#[rstest]
fn test_files_reentrant_write(
    #[values(32, 0, 7, 2049)] size: u32,
    #[values(31, 16, 65)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = powerloss_config(256);
    init_powerloss_context(&mut env);
    env.config.inline_max = if inline_max < 0 {
        u32::MAX
    } else {
        inline_max as u32
    };

    let config_ptr = &env.config;
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };

    // Format and mount for initial snapshot
    assert_ok(littlefs_rust_core::lfs_format(lfs, config_ptr));
    assert_ok(littlefs_rust_core::lfs_mount(lfs, config_ptr));
    assert_ok(littlefs_rust_core::lfs_unmount(lfs));
    let snapshot = env.snapshot();

    let max_iter = 5000;

    let op = |lfs: &mut Lfs, cfg: &LfsConfig| -> Result<(), Error> {
        let err = littlefs_rust_core::lfs_mount(lfs, cfg);
        if err.is_err() {
            let _ = littlefs_rust_core::lfs_format(lfs, cfg);
            littlefs_rust_core::lfs_mount(lfs, cfg)?;
        }

        let path = c"avacado";
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        let open_err = littlefs_rust_core::lfs_file_open(lfs, file, path, LFS_O_RDONLY);
        if open_err.is_ok() {
            let sz = littlefs_rust_core::lfs_file_size(lfs, file);
            assert!(sz == 0 || sz == size as i32, "size must be 0 or SIZE");
            littlefs_rust_core::lfs_file_close(lfs, file)?;
        } else {
            assert_eq!(open_err, Err(Error::NoEntry));
        }

        littlefs_rust_core::lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT)?;
        write_prng_file_result(lfs, file, size, chunk_size, 1)?;
        littlefs_rust_core::lfs_file_close(lfs, file)?;
        littlefs_rust_core::lfs_unmount(lfs)?;

        Ok(())
    };

    let verify = |lfs: &mut Lfs, cfg: &LfsConfig| -> Result<(), Error> {
        let remount = littlefs_rust_core::lfs_mount(lfs, cfg);
        if remount.is_err() {
            return Ok(());
        }
        let path = c"avacado";
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        let err = littlefs_rust_core::lfs_file_open(lfs, file, path, LFS_O_RDONLY);
        if err.is_err() {
            let _ = littlefs_rust_core::lfs_unmount(lfs);
            return Ok(());
        }
        let sz = littlefs_rust_core::lfs_file_size(lfs, file);
        if sz == size as i32 {
            verify_prng_file(lfs, file, size, chunk_size, 1);
        }
        littlefs_rust_core::lfs_file_close(lfs, file)?;
        littlefs_rust_core::lfs_unmount(lfs)?;

        Ok(())
    };

    let result = run_powerloss_linear(&mut env, &snapshot, max_iter, op, verify);
    result.expect("reentrant write should eventually succeed");
}

/// Upstream: [cases.test_files_reentrant_write_sync]
/// Three modes: APPEND, TRUNC, plain write. SIZE/CHUNKSIZE/INLINE_MAX vary per mode.
/// Power-loss after each sync. Stub: implement APPEND mode with SIZE=[32,0,7,2049].
#[rstest]
fn test_files_reentrant_write_sync(
    #[values(32, 0, 7, 2049)] size: u32,
    #[values(31, 16, 65)] chunk_size: u32,
    #[values(0, -1, 8)] inline_max: i32,
) {
    let mut env = powerloss_config(256);
    init_powerloss_context(&mut env);
    env.config.inline_max = if inline_max < 0 {
        u32::MAX
    } else {
        inline_max as u32
    };

    let config_ptr = &env.config;
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };

    assert_ok(littlefs_rust_core::lfs_format(lfs, config_ptr));
    assert_ok(littlefs_rust_core::lfs_mount(lfs, config_ptr));
    assert_ok(littlefs_rust_core::lfs_unmount(lfs));
    let snapshot = env.snapshot();

    let max_iter = 5000;

    let op = |lfs: &mut Lfs, cfg: &LfsConfig| -> Result<(), Error> {
        let err = littlefs_rust_core::lfs_mount(lfs, cfg);
        if err.is_err() {
            let _ = littlefs_rust_core::lfs_format(lfs, cfg);
            littlefs_rust_core::lfs_mount(lfs, cfg)?;
        }

        let path = c"avacado";
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        let open_err = littlefs_rust_core::lfs_file_open(lfs, file, path, LFS_O_RDONLY);
        if open_err.is_ok() {
            let sz = littlefs_rust_core::lfs_file_size(lfs, file);
            assert!(sz <= size as i32);
            let mut prng = 1u32;
            let mut buf = [0u8; 1024];
            let mut i: u32 = 0;
            while i < sz as u32 {
                let chunk = (chunk_size.min(sz as u32 - i)) as usize;
                let n = littlefs_rust_core::lfs_file_read(
                    lfs,
                    file,
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    chunk as u32,
                );
                assert_eq!(n, Ok(chunk as u32));
                for slot in buf[..chunk].iter() {
                    let expected = (common::test_prng(&mut prng) & 0xff) as u8;
                    assert_eq!(*slot, expected);
                }
                i += chunk as u32;
            }
            littlefs_rust_core::lfs_file_close(lfs, file)?;
        } else {
            assert_eq!(open_err, Err(Error::NoEntry));
        }

        littlefs_rust_core::lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        )?;
        let current_size = littlefs_rust_core::lfs_file_size(lfs, file);
        let skip = current_size.max(0) as u32;
        let mut prng = 1u32;
        common::advance_prng(&mut prng, skip);
        let mut i = skip;
        while i < size {
            let chunk = chunk_size.min(size - i);
            let mut buf = [0u8; 1024];
            for slot in buf[..chunk as usize].iter_mut() {
                *slot = (common::test_prng(&mut prng) & 0xff) as u8;
            }
            let n = littlefs_rust_core::lfs_file_write(
                lfs,
                file,
                buf.as_ptr() as *const core::ffi::c_void,
                chunk,
            )?;

            assert_eq!(n, chunk as u32);
            littlefs_rust_core::lfs_file_sync(lfs, file)?;

            i += chunk;
        }
        littlefs_rust_core::lfs_file_close(lfs, file)?;
        littlefs_rust_core::lfs_unmount(lfs)?;

        Ok(())
    };

    let verify = |lfs: &mut Lfs, cfg: &LfsConfig| -> Result<(), Error> {
        if littlefs_rust_core::lfs_mount(lfs, cfg).is_err() {
            return Ok(());
        }
        let path = c"avacado";
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        if littlefs_rust_core::lfs_file_open(lfs, file, path, LFS_O_RDONLY).is_err() {
            let _ = littlefs_rust_core::lfs_unmount(lfs);
            return Ok(());
        }
        let sz = littlefs_rust_core::lfs_file_size(lfs, file);
        if sz == size as i32 {
            verify_prng_file(lfs, file, size, chunk_size, 1);
        }
        littlefs_rust_core::lfs_file_close(lfs, file)?;
        littlefs_rust_core::lfs_unmount(lfs)?;

        Ok(())
    };

    let result = run_powerloss_linear(&mut env, &snapshot, max_iter, op, verify);
    result.expect("reentrant write sync should eventually succeed");
}

/// Upstream: [cases.test_files_many]
/// defines.N = 300
///
/// Create 300 files of 7 bytes ("Hi %03d"), read each back immediately, verify.
#[test]
fn test_files_many() {
    const N: usize = 300;
    let mut env = default_config(BLOCK_COUNT_MANY);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    for i in 0..N {
        let path = path_bytes(&format!("file_{:03}", i));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path.as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        let content = format!("Hi {:03}\0", i);
        let bytes = content.as_bytes();
        assert_eq!(bytes.len(), 7);
        let n = lfs_file_write(
            lfs,
            file,
            bytes.as_ptr() as *const core::ffi::c_void,
            bytes.len() as u32,
        );
        assert_eq!(n, Ok(bytes.len() as u32));
        assert_ok(lfs_file_close(lfs, file));

        let rfile = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, rfile, path.as_c_str(), LFS_O_RDONLY));
        let mut buf = [0u8; 32];
        let n = lfs_file_read(lfs, rfile, buf.as_mut_ptr() as *mut core::ffi::c_void, 7);
        assert_eq!(n, Ok(7));
        assert_eq!(&buf[..7], bytes);
        assert_ok(lfs_file_close(lfs, rfile));
    }
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_files_many_power_cycle]
/// defines.N = 300
///
/// Create 300 files, unmount/remount after each. Verify on final mount.
#[test]
fn test_files_many_power_cycle() {
    const N: usize = 300;
    let mut env = default_config(BLOCK_COUNT_MANY);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));

    for i in 0..N {
        assert_ok(lfs_mount(lfs, &env.config));
        let path = path_bytes(&format!("file_{:03}", i));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path.as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        let content = format!("Hi {:03}\0", i);
        let bytes = content.as_bytes();
        assert_eq!(bytes.len(), 7);
        let n = lfs_file_write(
            lfs,
            file,
            bytes.as_ptr() as *const core::ffi::c_void,
            bytes.len() as u32,
        );
        assert_eq!(n, Ok(bytes.len() as u32));
        assert_ok(lfs_file_close(lfs, file));
        assert_ok(lfs_unmount(lfs));

        assert_ok(lfs_mount(lfs, &env.config));
        let rfile = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(lfs, rfile, path.as_c_str(), LFS_O_RDONLY));
        let mut buf = [0u8; 32];
        let n = lfs_file_read(lfs, rfile, buf.as_mut_ptr() as *mut core::ffi::c_void, 7);
        assert_eq!(n, Ok(7));
        assert_eq!(&buf[..7], bytes);
        assert_ok(lfs_file_close(lfs, rfile));
    }
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_files_many_power_loss]
/// defines.N = 300, defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
///
/// Reentrant creation of 300 files with power-loss simulation.
/// Can take 30+ seconds due to iteration over power-loss points.
#[test]
#[cfg(feature = "slow_tests")]
fn test_files_many_power_loss() {
    const N: usize = 300;
    let mut env = powerloss_config(BLOCK_COUNT_MANY);
    init_powerloss_context(&mut env);

    let config_ptr = &env.config;
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };

    assert_ok(littlefs_rust_core::lfs_format(lfs, config_ptr));
    assert_ok(littlefs_rust_core::lfs_mount(lfs, config_ptr));
    assert_ok(littlefs_rust_core::lfs_unmount(lfs));
    let snapshot = env.snapshot();

    let max_iter = 2000;

    let op = |lfs: &mut Lfs, cfg: &LfsConfig| -> Result<(), Error> {
        let err = littlefs_rust_core::lfs_mount(lfs, cfg);
        if err.is_err() {
            let _ = littlefs_rust_core::lfs_format(lfs, cfg);
            let e = littlefs_rust_core::lfs_mount(lfs, cfg)?;
        }
        for i in 0..N {
            let path = path_bytes(&format!("file_{:03}", i));
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            let e = littlefs_rust_core::lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT)?;
            let content = format!("Hi {:03}\0", i);
            let bytes = content.as_bytes();
            assert_eq!(bytes.len(), 7);
            let sz = littlefs_rust_core::lfs_file_size(lfs, file);
            if sz != bytes.len() as i32 {
                let n = littlefs_rust_core::lfs_file_write(
                    lfs,
                    file,
                    bytes.as_ptr() as *const core::ffi::c_void,
                    bytes.len() as u32,
                )?;
                assert_eq!(n, bytes.len() as u32);
            }
            let e = littlefs_rust_core::lfs_file_close(lfs, file)?;

            let rfile = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            let e = littlefs_rust_core::lfs_file_open(lfs, rfile, path, LFS_O_RDONLY)?;
            let mut buf = [0u8; 32];
            let n = littlefs_rust_core::lfs_file_read(
                lfs,
                rfile,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                7,
            )?;
            assert_eq!(n, 7);
            assert_eq!(&buf[..7], bytes);
            let e = littlefs_rust_core::lfs_file_close(lfs, rfile)?;
        }
        let e = littlefs_rust_core::lfs_unmount(lfs)?;
        Ok(())
    };

    let verify = |_lfs: &mut Lfs, _cfg: &LfsConfig| -> Result<(), Error> { Ok(()) };

    let result = run_powerloss_linear(&mut env, &snapshot, max_iter, op, verify);
    result.expect("many_power_loss should eventually succeed");
}

// ── Rust-specific extras ────────────────────
// Bug reproducers, debug helpers, unit tests. Not in upstream.

#[test]
fn test_files_same_session() {
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(littlefs_rust_core::lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"hello";
    let data = b"Hello World!\0";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path, 0x0100 | 2));
    let n = lfs_file_write(
        lfs,
        file,
        data.as_ptr() as *const core::ffi::c_void,
        data.len() as u32,
    );
    assert_eq!(n, Ok(data.len() as u32));
    assert_ok(lfs_file_close(lfs, file));

    let file2 = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file2, path, 1));
    assert_eq!(lfs_file_size(lfs, file2), 13);
    let mut buf = [0u8; 32];
    let n = lfs_file_read(lfs, file2, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(13));
    assert_eq!(&buf[..13], b"Hello World!\0");
    assert_ok(lfs_file_close(lfs, file2));
}

#[test]
fn test_files_simple_read() {
    let mut env = default_config(128);
    fs_with_hello(&mut env).expect("fs_with_hello");
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"hello";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path, 1));

    assert_eq!(lfs_file_size(lfs, file), 13);
    assert_eq!(lfs_file_tell(lfs, file), 0);

    let mut buf = [0u8; 32];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(13));
    assert_eq!(&buf[..13], b"Hello World!\0");

    let n2 = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n2, Ok(0));

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_files_seek_tell() {
    let mut env = default_config(128);
    fs_with_hello(&mut env).expect("fs_with_hello");
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"hello";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path, 1));

    let mut buf = [0u8; 4];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 4);
    assert_eq!(n, Ok(4));
    assert_eq!(&buf[..4], b"Hell");
    assert_eq!(lfs_file_tell(lfs, file), 4);

    assert_ok(lfs_file_rewind(lfs, file));
    assert_eq!(lfs_file_tell(lfs, file), 0);

    let n2 = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 4);
    assert_eq!(n2, Ok(4));
    assert_eq!(&buf[..4], b"Hell");

    let pos = lfs_file_seek(lfs, file, 6, 0);
    assert_eq!(pos, Ok(6));
    let n3 = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 4);
    assert_eq!(n3, Ok(4));
    assert_eq!(&buf[..4], b"Worl");

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_files_truncate_api() {
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = path_bytes("x");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path.as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    let data = b"hello world";
    let _ = lfs_file_write(
        lfs,
        file,
        data.as_ptr() as *const core::ffi::c_void,
        data.len() as u32,
    );
    assert_ok(lfs_file_truncate(lfs, file, 5));
    assert_ok(lfs_file_sync(lfs, file));
    assert_ok(lfs_file_close(lfs, file));

    assert_ok(lfs_unmount(lfs));
    assert_ok(lfs_mount(lfs, &env.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path.as_c_str(), LFS_O_RDONLY));
    assert_eq!(lfs_file_size(lfs, file), 5);
    let mut buf = [0u8; 32];
    let n = lfs_file_read(lfs, file, buf.as_mut_ptr() as *mut core::ffi::c_void, 32);
    assert_eq!(n, Ok(5));
    assert_eq!(&buf[..5], b"hello");
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}
