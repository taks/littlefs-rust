//! Upstream: tests/test_interspersed.toml
//!
//! Multi-file I/O interspersed with directory operations.

#![allow(clippy::needless_range_loop)]

mod common;

#[cfg(feature = "slow_tests")]
use common::LFS_O_APPEND;
use common::{LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY, default_config, init_context};
#[cfg(feature = "slow_tests")]
use littlefs_rust_core::lfs_file_size;
use littlefs_rust_core::{
    Lfs, LfsConfig, LfsDir, LfsFile, LfsInfo, lfs_dir_close, lfs_dir_open, lfs_dir_read,
    lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_sync, lfs_file_write, lfs_format,
    lfs_mount, lfs_remove, lfs_unmount,
};
use littlefs_rust_test_macro::lfs_test;
use rstest::rstest;

const ALPHAS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const LFS_TYPE_DIR: u8 = 0x02;
const LFS_TYPE_REG: u8 = 0x01;

/// Upstream: [cases.test_interspersed_files]
/// defines.SIZE = [10, 100]
/// defines.FILES = [4, 10, 26]
///
/// Open FILES files ("a","b",...), write SIZE bytes to each in round-robin
/// (1 byte per iteration), close all. Verify directory listing (FILES + 2
/// for . and ..). Check each file has SIZE bytes, read back first 10 bytes.
#[lfs_test]
fn test_interspersed_files(
    cfg: &LfsConfig,
    #[values(10, 100)] size: usize,
    #[values(4, 10, 26)] files: usize,
) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let mut file_handles: Vec<LfsFile> = (0..files).map(|_| LfsFile::default()).collect();

    for j in 0..files {
        let path = &String::from(ALPHAS[j] as char);
        assert_ok!(lfs_file_open(
            lfs,
            &mut file_handles[j],
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
    }

    for _i in 0..size {
        for j in 0..files {
            let byte = [ALPHAS[j]];
            let n = lfs_file_write(lfs, &mut file_handles[j], &byte);
            assert_eq!(n, Ok(1));
        }
    }

    for j in 0..files {
        assert_ok!(lfs_file_close(lfs, &mut file_handles[j]));
    }

    // Verify directory listing
    let root = "/";
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok!(lfs_dir_open(lfs, dir, root));

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    assert_eq!(&info.name[..1], b".");
    assert_eq!(info.type_, LFS_TYPE_DIR);

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    assert_eq!(&info.name[..2], b"..");
    assert_eq!(info.type_, LFS_TYPE_DIR);

    for j in 0..files {
        let expected_name = String::from(ALPHAS[j] as char);
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        let name = core::str::from_utf8(&info.name[..nul]).unwrap();
        assert_eq!(name, expected_name);
        assert_eq!(info.type_, LFS_TYPE_REG);
        assert_eq!(info.size, size as u32);
    }

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(false));
    assert_ok!(lfs_dir_close(lfs, dir));

    // Re-open for reading and verify first 10 bytes
    let mut file_handles: Vec<LfsFile> = (0..files).map(|_| LfsFile::default()).collect();

    for j in 0..files {
        let path = &String::from(ALPHAS[j] as char);
        assert_ok!(lfs_file_open(lfs, &mut file_handles[j], path, LFS_O_RDONLY));
    }

    for _i in 0..10 {
        for j in 0..files {
            let mut buffer = [0u8; 1];
            let n = lfs_file_read(lfs, &mut file_handles[j], &mut buffer);
            assert_eq!(n, Ok(1));
            assert_eq!(buffer[0], ALPHAS[j]);
        }
    }

    for j in 0..files {
        assert_ok!(lfs_file_close(lfs, &mut file_handles[j]));
    }

    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_interspersed_remove_files]
/// defines.SIZE = [10, 100]
/// defines.FILES = [4, 10, 26]
///
/// Create FILES files with SIZE bytes each. Open "zzz", write one byte
/// and sync, remove one of the FILES-lettered files, repeat. After removing
/// all, verify "zzz" has FILES bytes and directory listing is correct.
#[rstest]
fn test_interspersed_remove_files(
    #[values(10, 100)] size: usize,
    #[values(4, 10, 26)] files: usize,
) {
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    // Create FILES files with SIZE bytes each
    for j in 0..files {
        let path = &String::from(ALPHAS[j] as char);
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        for _i in 0..size {
            let byte = [ALPHAS[j]];
            let n = lfs_file_write(lfs, file, &byte);
            assert_eq!(n, Ok(1));
        }
        assert_ok!(lfs_file_close(lfs, file));
    }
    assert_ok!(lfs_unmount(lfs));

    // Remount, open "zzz", interleave writes+syncs with removes
    assert_ok!(lfs_mount(lfs, &env.config));
    let zzz_path = "zzz";
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        zzz_path,
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    for j in 0..files {
        let tilde = b"~";
        let n = lfs_file_write(lfs, file, tilde);
        assert_eq!(n, Ok(1));
        assert_ok!(lfs_file_sync(lfs, file));

        let path = &String::from(ALPHAS[j] as char);
        assert_ok!(lfs_remove(lfs, path));
    }
    assert_ok!(lfs_file_close(lfs, file));

    // Verify directory: only "zzz" left
    let root = "/";
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok!(lfs_dir_open(lfs, dir, root));

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    assert_eq!(&info.name[..1], b".");
    assert_eq!(info.type_, LFS_TYPE_DIR);

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    assert_eq!(&info.name[..2], b"..");
    assert_eq!(info.type_, LFS_TYPE_DIR);

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    let name = core::str::from_utf8(&info.name[..nul]).unwrap();
    assert_eq!(name, "zzz");
    assert_eq!(info.type_, LFS_TYPE_REG);
    assert_eq!(info.size, files as u32);

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(false));
    assert_ok!(lfs_dir_close(lfs, dir));

    // Verify "zzz" content
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, zzz_path, LFS_O_RDONLY));
    for _i in 0..files {
        let mut buffer = [0u8; 1];
        let n = lfs_file_read(lfs, file, &mut buffer);
        assert_eq!(n, Ok(1));
        assert_eq!(buffer[0], b'~');
    }
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_interspersed_remove_inconveniently]
/// defines.SIZE = [10, 100]
///
/// Open three files "e","f","g". Write SIZE/2 bytes to each. Remove "f"
/// while all three are still open. Write another SIZE/2 bytes to all three
/// (including removed "f"). Close all. Verify directory: "e" and "g"
/// present, "f" absent. Read "e" and "g", verify SIZE bytes.
#[rstest]
fn test_interspersed_remove_inconveniently(#[values(10, 100)] size: usize) {
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let mut files: [LfsFile; 3] = Default::default();

    let path_e = "e";
    let path_f = "f";
    let path_g = "g";

    assert_ok!(lfs_file_open(
        lfs,
        &mut files[0],
        path_e,
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok!(lfs_file_open(
        lfs,
        &mut files[1],
        path_f,
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok!(lfs_file_open(
        lfs,
        &mut files[2],
        path_g,
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    // Write SIZE/2 bytes to each
    for _i in 0..(size / 2) {
        assert_eq!(lfs_file_write(lfs, &mut files[0], b"e"), Ok(1));
        assert_eq!(lfs_file_write(lfs, &mut files[1], b"f"), Ok(1));
        assert_eq!(lfs_file_write(lfs, &mut files[2], b"g"), Ok(1));
    }

    // Remove "f" while it's still open
    assert_ok!(lfs_remove(lfs, path_f));

    // Write another SIZE/2 bytes to all three
    for _i in 0..(size / 2) {
        assert_eq!(lfs_file_write(lfs, &mut files[0], b"e"), Ok(1));
        assert_eq!(lfs_file_write(lfs, &mut files[1], b"f"), Ok(1));
        assert_eq!(lfs_file_write(lfs, &mut files[2], b"g"), Ok(1));
    }

    assert_ok!(lfs_file_close(lfs, &mut files[0]));
    assert_ok!(lfs_file_close(lfs, &mut files[1]));
    assert_ok!(lfs_file_close(lfs, &mut files[2]));

    // Verify directory: "e" and "g" present, "f" absent
    let root = "/";
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok!(lfs_dir_open(lfs, dir, root));

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    assert_eq!(&info.name[..1], b".");
    assert_eq!(info.type_, LFS_TYPE_DIR);

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    assert_eq!(&info.name[..2], b"..");
    assert_eq!(info.type_, LFS_TYPE_DIR);

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "e");
    assert_eq!(info.type_, LFS_TYPE_REG);
    assert_eq!(info.size, size as u32);

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "g");
    assert_eq!(info.type_, LFS_TYPE_REG);
    assert_eq!(info.size, size as u32);

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(false));
    assert_ok!(lfs_dir_close(lfs, dir));

    // Read "e" and "g", verify SIZE bytes
    let mut files_r: [LfsFile; 2] = Default::default();
    assert_ok!(lfs_file_open(lfs, &mut files_r[0], path_e, LFS_O_RDONLY));
    assert_ok!(lfs_file_open(lfs, &mut files_r[1], path_g, LFS_O_RDONLY));

    for _i in 0..size {
        let mut buffer = [0u8; 1];
        assert_eq!(lfs_file_read(lfs, &mut files_r[0], &mut buffer), Ok(1));
        assert_eq!(buffer[0], b'e');
        assert_eq!(lfs_file_read(lfs, &mut files_r[1], &mut buffer), Ok(1));
        assert_eq!(buffer[0], b'g');
    }
    assert_ok!(lfs_file_close(lfs, &mut files_r[0]));
    assert_ok!(lfs_file_close(lfs, &mut files_r[1]));

    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_interspersed_reentrant_files]
/// defines.SIZE = [10, 100]
/// defines.FILES = [4, 10, 26]
/// defines.POWERLOSS_BEHAVIOR = [NOOP, OOO]
/// reentrant = true
///
/// Power-loss test. Mount-or-format. Open FILES files for append. Write
/// SIZE bytes per file with sync after each byte when size <= i. Close.
/// Verify directory and read 10 bytes from each.
#[rstest]
#[cfg(feature = "slow_tests")]
fn test_interspersed_reentrant_files(
    #[values(10, 100)] size: usize,
    #[values(4, 10, 26)] files: usize,
) {
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();

    // Mount-or-format
    let err = lfs_mount(lfs, &env.config);
    if err.is_err() {
        assert_ok!(lfs_format(lfs, &env.config));
        assert_ok!(lfs_mount(lfs, &env.config));
    }

    let mut file_handles: Vec<LfsFile> = (0..files).map(|_| LfsFile::default()).collect();

    for j in 0..files {
        let path = &String::from(ALPHAS[j] as char);
        assert_ok!(lfs_file_open(
            lfs,
            &mut file_handles[j],
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        ));
    }

    for i in 0..size {
        for j in 0..files {
            let file_sz = lfs_file_size(lfs, &file_handles[j]);
            if (file_sz as usize) <= i {
                let byte = [ALPHAS[j]];
                let n = lfs_file_write(lfs, &mut file_handles[j], &byte);
                assert_eq!(n, Ok(1));
                assert_ok!(lfs_file_sync(lfs, &mut file_handles[j]));
            }
        }
    }

    for j in 0..files {
        assert_ok!(lfs_file_close(lfs, &mut file_handles[j]));
    }

    // Verify directory
    let root = "/";
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok!(lfs_dir_open(lfs, dir, root));

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    assert_eq!(&info.name[..1], b".");
    assert_eq!(info.type_, LFS_TYPE_DIR);

    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
    assert_eq!(&info.name[..2], b"..");
    assert_eq!(info.type_, LFS_TYPE_DIR);

    for j in 0..files {
        let expected_name = String::from(ALPHAS[j] as char);
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        let name = core::str::from_utf8(&info.name[..nul]).unwrap();
        assert_eq!(name, expected_name);
        assert_eq!(info.type_, LFS_TYPE_REG);
        assert_eq!(info.size, size as u32);
    }
    assert_eq!(lfs_dir_read(lfs, dir, info), Ok(false));
    assert_ok!(lfs_dir_close(lfs, dir));

    // Read first 10 bytes from each
    let mut file_handles: Vec<LfsFile> = (0..files).map(|_| LfsFile::default()).collect();

    for j in 0..files {
        let path = &String::from(ALPHAS[j] as char);
        assert_ok!(lfs_file_open(lfs, &mut file_handles[j], path, LFS_O_RDONLY));
    }

    for _i in 0..10 {
        for j in 0..files {
            let mut buffer = [0u8; 1];
            let n = lfs_file_read(lfs, &mut file_handles[j], &mut buffer);
            assert_eq!(n, Ok(1));
            assert_eq!(buffer[0], ALPHAS[j]);
        }
    }

    for j in 0..files {
        assert_ok!(lfs_file_close(lfs, &mut file_handles[j]));
    }

    assert_ok!(lfs_unmount(lfs));
}
