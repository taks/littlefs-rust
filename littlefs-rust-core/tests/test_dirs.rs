//! Directory iteration tests.
//!
//! Upstream: tests/test_dirs.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_dirs.toml

mod common;

#[cfg(feature = "slow_tests")]
use common::powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear};
use common::{
    LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY, default_config, dir_entry_names,
    init_context, init_logger,
};
use littlefs_rust_core::error::Error;
use littlefs_rust_core::lfs_type::lfs_type::{LFS_TYPE_DIR, LFS_TYPE_REG};
use littlefs_rust_core::{
    Lfs, LfsConfig, LfsDir, LfsFile, LfsInfo, lfs_dir_close, lfs_dir_open, lfs_dir_read,
    lfs_dir_rewind, lfs_dir_seek, lfs_dir_tell, lfs_file_close, lfs_file_open, lfs_format,
    lfs_mkdir, lfs_mount, lfs_remove, lfs_rename, lfs_stat, lfs_unmount,
};
use littlefs_rust_test_macro::lfs_test;
use rstest::rstest;

/// Root path: "/" null-terminated.
static ROOT_PATH: &str = "/"; // [b'/', 0];

// --- test_dirs_root ---
// Upstream: dir_open("/"), dir_read returns ".", "..", then 0
#[test]
fn test_dirs_root() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok!(lfs_dir_open(lfs, dir, ROOT_PATH));

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    let n = lfs_dir_read(lfs, dir, info);
    assert_eq!(n, Ok(true));
    assert_eq!(info.name[0], b'.');
    assert_eq!(info.name[1], 0);
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    let n = lfs_dir_read(lfs, dir, info);
    assert_eq!(n, Ok(true));
    assert_eq!(info.name[0], b'.');
    assert_eq!(info.name[1], b'.');
    assert_eq!(info.name[2], 0);
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    let n = lfs_dir_read(lfs, dir, info);
    assert_eq!(n, Ok(false));

    assert_ok!(lfs_dir_close(lfs, dir));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_dirs_one_mkdir ---
// Upstream: [cases.test_dirs_one_mkdir] mkdir("d0"), stat, dir_read
#[test]
fn test_dirs_one_mkdir() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let path = "d0";
    assert_ok!(lfs_mkdir(lfs, path));

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok!(lfs_stat(lfs, path, info));
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), "d0");
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let names = dir_entry_names(lfs, &env.config, "/").expect("dir_entry_names");
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], "d0");

    assert_ok!(lfs_unmount(lfs));
}

// --- test_dirs_many_creation ---
/// Upstream: [cases.test_dirs_many_creation]
/// defines.N = range(3, 100, 3), if = 'N < BLOCK_COUNT/2'
///
/// Create N dirs dir000..dir{N-1}, unmount, mount, verify dir_read.
#[rstest]
fn test_dirs_many_creation(
    #[values(
        3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36, 39, 42, 45, 48, 51, 54, 57, 60, 63, 66, 69,
        72, 75, 78, 81, 84, 87, 90, 93, 96, 99
    )]
    n: usize,
) {
    init_logger();
    let block_count = 256u32;
    if n >= block_count as usize / 2 {
        return;
    }
    let mut env = default_config(block_count);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    for i in 0..n {
        let path = &format!("dir{i:03}");
        let err = lfs_mkdir(lfs, path);
        assert_ok!(err);
    }

    let names = dir_entry_names(lfs, &env.config, "/").expect("dir_entry_names");
    assert_eq!(names.len(), n);
    let mut names_sorted = names.clone();
    names_sorted.sort();
    let expected: Vec<String> = (0..n).map(|i| format!("dir{i:03}")).collect();
    let mut expected_sorted = expected.clone();
    expected_sorted.sort();
    assert_eq!(names_sorted, expected_sorted);

    assert_ok!(lfs_unmount(lfs));
}

// --- test_dirs_many_removal ---
/// Upstream: [cases.test_dirs_many_removal]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
///
/// Create N dirs removeme000.., verify, remove all, verify empty.
#[rstest]
fn test_dirs_many_removal(#[values(3, 14, 25, 36, 47, 58, 69, 80, 91)] n: usize) {
    init_logger();
    let block_count = 256u32;
    if n >= block_count as usize / 2 {
        return;
    }
    let mut env = default_config(block_count);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    for i in 0..n {
        let path = &format!("removeme{i:03}");
        assert_ok!(lfs_mkdir(lfs, path));
    }
    for i in 0..n {
        let path = &format!("removeme{i:03}");
        assert_ok!(lfs_remove(lfs, path));
    }

    let names = dir_entry_names(lfs, &env.config, "/").expect("dir_entry_names");
    assert!(names.is_empty());

    assert_ok!(lfs_unmount(lfs));
}

// --- test_dirs_many_rename ---
/// Upstream: [cases.test_dirs_many_rename]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
///
/// Create N dirs test000.., rename to tedd000.., verify.
#[rstest]
fn test_dirs_many_rename(#[values(3, 14, 25, 36, 47, 58, 69, 80, 91)] n: usize) {
    init_logger();
    let block_count = 256u32;
    if n >= block_count as usize / 2 {
        return;
    }
    let mut env = default_config(block_count);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    for i in 0..n {
        let path = &format!("test{i:03}");
        assert_ok!(lfs_mkdir(lfs, path));
    }
    for i in 0..n {
        let old_path = &format!("test{i:03}");
        let new_path = &format!("tedd{i:03}");
        let err = lfs_rename(lfs, old_path, new_path);
        assert_ok!(err);
    }

    let names = dir_entry_names(lfs, &env.config, "/").expect("dir_entry_names");
    assert_eq!(names.len(), n);
    let mut names_sorted = names.clone();
    names_sorted.sort();
    let expected: Vec<String> = (0..n).map(|i| format!("tedd{i:03}")).collect();
    let mut expected_sorted = expected.clone();
    expected_sorted.sort();
    assert_eq!(names_sorted, expected_sorted);

    assert_ok!(lfs_unmount(lfs));
}

// --- Implemented upstream cases ---

/// Upstream: [cases.test_dirs_many_rename_append]
/// defines.N = range(5, 13, 2), if = 'N < BLOCK_COUNT/2'
/// Format, create N dirs a00..a{N-1}, unmount, mount, rename a→z, unmount,
/// mount, verify dir_read shows z00..z{N-1} in order.
#[test]
fn test_dirs_many_rename_append() {
    init_logger();
    for n in [5usize, 7, 9, 11] {
        let mut env = default_config(128);
        init_context(&mut env);

        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, &env.config));
        assert_ok!(lfs_mount(lfs, &env.config));

        for i in 0..n {
            let path = &format!("a{i:02}");
            assert_ok!(lfs_mkdir(lfs, path));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &env.config));
        for i in 0..n {
            let old = &format!("a{i:02}");
            let new = &format!("z{i:02}");
            assert_ok!(lfs_rename(lfs, old, new));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &env.config));
        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_ok!(lfs_dir_open(lfs, dir, ROOT_PATH));

        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
        assert_eq!(info.name[0], b'.');
        assert_eq!(info.name[1], 0);

        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
        assert_eq!(info.name[0], b'.');
        assert_eq!(info.name[1], b'.');
        assert_eq!(info.name[2], 0);

        for i in 0..n {
            let expected = format!("z{i:02}");
            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            assert_eq!(
                lfs_dir_read(lfs, dir, info),
                Ok(true),
                "N={n}, expected entry {i}"
            );
            assert_eq!(info.type_, LFS_TYPE_DIR as u8);
            let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
            let name = core::str::from_utf8(&info.name[..nul]).unwrap();
            assert_eq!(name, expected, "N={n}, entry {i}");
        }

        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(false));

        assert_ok!(lfs_dir_close(lfs, dir));
        assert_ok!(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_dirs_many_reentrant]
/// defines.N = [5, 11], BLOCK_COUNT >= 4*N, reentrant, POWERLOSS_BEHAVIOR = [NOOP, OOO]
#[lfs_test]
#[cfg(feature = "slow_tests")]
#[ignore = "bug: power-loss iteration returns LFS_ERR_CORRUPT (-84)"]
fn test_dirs_many_reentrant(cfg: &LfsConfig, #[values(5, 11)] n: usize) {
    init_logger();

    let err = lfs_mount(lfs_ptr, config);
    if err.is_err() {
        let _ = lfs_format(lfs_ptr, config);
        lfs_mount(lfs_ptr, config)?;
    }

    for i in 0..n {
        let path = &format!("hi{i:03}");
        let err = lfs_mkdir(lfs_ptr, path);
        if err.is_err() && err != Err(Error::Exists) {
            return err;
        }
    }
    for i in 0..n {
        let path = &format!("hello{i:03}");
        let err = lfs_remove(lfs_ptr, path);
        if err.is_err() && err != Err(Error::NoEntry) {
            return err;
        }
    }

    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    if lfs_dir_open(lfs_ptr, dir, ROOT_PATH).is_err() {
        return Err(Error::Invalid);
    }
    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    let _ = lfs_dir_read(lfs_ptr, dir, info);
    let _ = lfs_dir_read(lfs_ptr, dir, info);
    for i in 0..n {
        let expected = format!("hi{i:03}");
        let r = lfs_dir_read(lfs_ptr, dir, info);
        if r != Ok(true) {
            let _ = lfs_dir_close(lfs_ptr, dir);
            return Err(if let Err(r) = r { r } else { Error::Invalid });
        }
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        let name = core::str::from_utf8(&info.name[..nul]).unwrap();
        if name != expected {
            let _ = lfs_dir_close(lfs_ptr, dir);
            return Err(Error::Invalid);
        }
    }
    if lfs_dir_read(lfs_ptr, dir, info).is_err() {
        let _ = lfs_dir_close(lfs_ptr, dir);
        return Err(Error::Invalid);
    }
    if lfs_dir_close(lfs_ptr, dir).is_err() {
        return Err(Error::Invalid);
    }

    for i in 0..n {
        let old = &format!("hi{i:03}");
        let new = &format!("hello{i:03}");
        if lfs_rename(lfs_ptr, old, new).is_err() {
            return Err(Error::Invalid);
        }
    }

    if lfs_dir_open(lfs_ptr, dir, ROOT_PATH).is_err() {
        return Err(Error::Invalid);
    }
    let _ = lfs_dir_read(lfs_ptr, dir, info);
    let _ = lfs_dir_read(lfs_ptr, dir, info);
    for i in 0..n {
        let expected = format!("hello{i:03}");
        let r = lfs_dir_read(lfs_ptr, dir, info);
        if r != Ok(true) {
            let _ = lfs_dir_close(lfs_ptr, dir);
            return Err(if let Err(r) = r { r } else { Error::Invalid });
        }
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        let name = core::str::from_utf8(&info.name[..nul]).unwrap();
        if name != expected {
            let _ = lfs_dir_close(lfs_ptr, dir);
            return Err(Error::Invalid);
        }
    }
    if lfs_dir_read(lfs_ptr, dir, info).is_err() {
        let _ = lfs_dir_close(lfs_ptr, dir);
        return Err(Error::Invalid);
    }
    if lfs_dir_close(lfs_ptr, dir).is_err() {
        return Err(Error::Invalid);
    }

    for i in 0..n {
        let path = &format!("hello{i:03}");
        if lfs_remove(lfs_ptr, path).is_err() {
            return Err(Error::Invalid);
        }
    }

    if lfs_dir_open(lfs_ptr, dir, ROOT_PATH).is_err() {
        return Err(Error::Invalid);
    }
    let _ = lfs_dir_read(lfs_ptr, dir, info);
    let _ = lfs_dir_read(lfs_ptr, dir, info);
    if lfs_dir_read(lfs_ptr, dir, info).is_err() {
        let _ = lfs_dir_close(lfs_ptr, dir);
        return Err(Error::Invalid);
    }
    if lfs_dir_close(lfs_ptr, dir).is_err() {
        return Err(Error::Invalid);
    }

    if lfs_unmount(lfs_ptr).is_err() {
        return Err(Error::Invalid);
    }
}

/// Upstream: [cases.test_dirs_file_creation]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
/// Create N empty files, unmount, mount, verify dir_read shows all with LFS_TYPE_REG.
#[lfs_test]
fn test_dirs_file_creation(cfg: &LfsConfig) {
    for n in [3usize, 14, 25, 36, 47, 58, 69, 80, 91] {
        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, cfg));
        assert_ok!(lfs_mount(lfs, cfg));

        for i in 0..n {
            let path = &format!("file{i:03}");
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
            assert_ok!(lfs_file_close(lfs, file));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, cfg));
        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_ok!(lfs_dir_open(lfs, dir, ROOT_PATH));

        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
        assert_eq!({ info.type_ }, LFS_TYPE_DIR as u8);

        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(true));
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);

        for i in 0..n {
            let expected = format!("file{i:03}");
            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            assert_eq!(
                lfs_dir_read(lfs, dir, info),
                Ok(true),
                "N={n}, expected entry {i}"
            );
            assert_eq!(info.type_, LFS_TYPE_REG as u8, "N={n}, entry {i} type");
            let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
            let name = core::str::from_utf8(&info.name[..nul]).unwrap();
            assert_eq!(name, expected, "N={n}, entry {i} name");
        }

        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_eq!(lfs_dir_read(lfs, dir, info), Ok(false));

        assert_ok!(lfs_dir_close(lfs, dir));
        assert_ok!(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_dirs_file_removal]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
/// Create N files, verify present, remove all, verify empty.
#[test]
fn test_dirs_file_removal() {
    init_logger();
    for n in [3usize, 14, 25, 36, 47, 58, 69, 80, 91] {
        let mut env = default_config(128);
        init_context(&mut env);

        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, &env.config));
        assert_ok!(lfs_mount(lfs, &env.config));

        for i in 0..n {
            let path = &format!("removeme{i:03}");
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
            assert_ok!(lfs_file_close(lfs, file));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &env.config));
        let names = dir_entry_names(lfs, &env.config, "/").expect("dir_entry_names");
        let mut names_sorted = names.clone();
        names_sorted.sort();
        let mut expected: Vec<String> = (0..n).map(|i| format!("removeme{i:03}")).collect();
        expected.sort();
        assert_eq!(names_sorted, expected, "N={n} before removal");
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &env.config));
        for i in 0..n {
            let path = &format!("removeme{i:03}");
            assert_ok!(lfs_remove(lfs, path));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &env.config));
        let names = dir_entry_names(lfs, &env.config, "/").expect("dir_entry_names");
        assert!(names.is_empty(), "N={n} after removal: {names:?}");
        assert_ok!(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_dirs_file_rename]
/// defines.N = range(3, 100, 11), if = 'N < BLOCK_COUNT/2'
/// Create N files test000.., rename to tedd000.., verify.
#[test]
fn test_dirs_file_rename() {
    init_logger();
    for n in [3usize, 14, 25, 36, 47, 58, 69, 80, 91] {
        let mut env = default_config(128);
        init_context(&mut env);

        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, &env.config));
        assert_ok!(lfs_mount(lfs, &env.config));

        for i in 0..n {
            let path = &format!("test{i:03}");
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
            assert_ok!(lfs_file_close(lfs, file));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &env.config));
        let names = dir_entry_names(lfs, &env.config, "/").expect("dir_entry_names");
        let mut names_sorted = names.clone();
        names_sorted.sort();
        let mut expected: Vec<String> = (0..n).map(|i| format!("test{i:03}")).collect();
        expected.sort();
        assert_eq!(names_sorted, expected, "N={n} before rename");
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &env.config));
        for i in 0..n {
            let old = &format!("test{i:03}");
            let new = &format!("tedd{i:03}");
            assert_ok!(lfs_rename(lfs, old, new));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &env.config));
        let names = dir_entry_names(lfs, &env.config, "/").expect("dir_entry_names");
        let mut names_sorted = names.clone();
        names_sorted.sort();
        let mut expected: Vec<String> = (0..n).map(|i| format!("tedd{i:03}")).collect();
        expected.sort();
        assert_eq!(names_sorted, expected, "N={n} after rename");
        assert_ok!(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_dirs_file_reentrant]
/// defines.N = [5, 25], N < BLOCK_COUNT/2, reentrant, POWERLOSS_BEHAVIOR = [NOOP, OOO]
#[test]
#[cfg(feature = "slow_tests")]
#[ignore = "bug: power-loss iteration returns LFS_ERR_CORRUPT (-84)"]
fn test_dirs_file_reentrant() {
    init_logger();
    for n in [5usize, 25] {
        let block_count = 128u32;
        let mut env = powerloss_config(block_count);
        init_powerloss_context(&mut env);
        let snapshot = env.snapshot();

        let result = run_powerloss_linear(
            &mut env,
            &snapshot,
            3000,
            |lfs_ptr, config| {
                let err = lfs_mount(lfs_ptr, config);
                if err.is_err() {
                    let _ = lfs_format(lfs_ptr, config);
                    lfs_mount(lfs_ptr, config)?;
                }

                let file = &mut LfsFile::default();
                for i in 0..n {
                    let path = &format!("hi{i:03}");
                    if lfs_file_open(lfs_ptr, file, path, LFS_O_CREAT | LFS_O_WRONLY).is_err() {
                        return Err(Error::Invalid);
                    }
                    if lfs_file_close(lfs_ptr, file).is_err() {
                        return Err(Error::Invalid);
                    }
                }
                for i in 0..n {
                    let path = &format!("hello{i:03}");
                    let err = lfs_remove(lfs_ptr, path);
                    if err.is_err() && err != Err(Error::NoEntry) {
                        return err;
                    }
                }

                let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
                if lfs_dir_open(lfs_ptr, dir, ROOT_PATH).is_err() {
                    return Err(Error::Invalid);
                }
                let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
                let _ = lfs_dir_read(lfs_ptr, dir, info);
                let _ = lfs_dir_read(lfs_ptr, dir, info);
                for i in 0..n {
                    let expected = format!("hi{i:03}");
                    let r = lfs_dir_read(lfs_ptr, dir, info);
                    if r != Ok(true) {
                        let _ = lfs_dir_close(lfs_ptr, dir);
                        return Err(if let Err(r) = r { r } else { Error::Invalid });
                    }
                    if info.type_ != LFS_TYPE_REG as u8 {
                        let _ = lfs_dir_close(lfs_ptr, dir);
                        return Err(Error::Invalid);
                    }
                    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
                    let name = core::str::from_utf8(&info.name[..nul]).unwrap();
                    if name != expected {
                        let _ = lfs_dir_close(lfs_ptr, dir);
                        return Err(Error::Invalid);
                    }
                }
                if lfs_dir_read(lfs_ptr, dir, info).is_err() {
                    let _ = lfs_dir_close(lfs_ptr, dir);
                    return Err(Error::Invalid);
                }
                if lfs_dir_close(lfs_ptr, dir).is_err() {
                    return Err(Error::Invalid);
                }

                for i in 0..n {
                    let old = &format!("hi{i:03}");
                    let new = &format!("hello{i:03}");
                    if lfs_rename(lfs_ptr, old, new).is_err() {
                        return Err(Error::Invalid);
                    }
                }

                if lfs_dir_open(lfs_ptr, dir, ROOT_PATH).is_err() {
                    return Err(Error::Invalid);
                }
                let _ = lfs_dir_read(lfs_ptr, dir, info);
                let _ = lfs_dir_read(lfs_ptr, dir, info);
                for i in 0..n {
                    let expected = format!("hello{i:03}");
                    let r = lfs_dir_read(lfs_ptr, dir, info);
                    if r != Ok(true) {
                        let _ = lfs_dir_close(lfs_ptr, dir);
                        return Err(if let Err(r) = r { r } else { Error::Invalid });
                    }
                    if info.type_ != LFS_TYPE_REG as u8 {
                        let _ = lfs_dir_close(lfs_ptr, dir);
                        return Err(Error::Invalid);
                    }
                    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
                    let name = core::str::from_utf8(&info.name[..nul]).unwrap();
                    if name != expected {
                        let _ = lfs_dir_close(lfs_ptr, dir);
                        return Err(Error::Invalid);
                    }
                }
                if lfs_dir_read(lfs_ptr, dir, info).is_err() {
                    let _ = lfs_dir_close(lfs_ptr, dir);
                    return Err(Error::Invalid);
                }
                if lfs_dir_close(lfs_ptr, dir).is_err() {
                    return Err(Error::Invalid);
                }

                for i in 0..n {
                    let path = &format!("hello{i:03}");
                    if lfs_remove(lfs_ptr, path).is_err() {
                        return Err(Error::Invalid);
                    }
                }

                if lfs_unmount(lfs_ptr).is_err() {
                    return Err(Error::Invalid);
                }
                Ok(())
            },
            |_, _| Ok(()),
        );
        result.unwrap_or_else(|_| panic!("test_dirs_file_reentrant N={n} should complete"));
    }
}

/// Upstream: [cases.test_dirs_nested]
/// Create dirs, files, rename chains, cross-dir renames, then cleanup.
#[test]
fn test_dirs_nested() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_mkdir(lfs, "potato"));
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "burito",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_mkdir(lfs, "potato/baked"));
    assert_ok!(lfs_mkdir(lfs, "potato/sweet"));
    assert_ok!(lfs_mkdir(lfs, "potato/fried"));

    let names = dir_entry_names(lfs, &env.config, "potato").expect("potato dir_entry_names");
    let mut names_sorted = names.clone();
    names_sorted.sort();
    assert_eq!(names_sorted, vec!["baked", "fried", "sweet"]);

    assert_err!(Error::NotEmpty, lfs_remove(lfs, "potato"));

    assert_ok!(lfs_rename(lfs, "potato", "coldpotato"));
    assert_ok!(lfs_rename(lfs, "coldpotato", "warmpotato"));
    assert_ok!(lfs_rename(lfs, "warmpotato", "hotpotato"));

    assert_err!(Error::NoEntry, lfs_remove(lfs, "potato"));
    assert_err!(Error::NoEntry, lfs_remove(lfs, "coldpotato"));
    assert_err!(Error::NoEntry, lfs_remove(lfs, "warmpotato"));
    assert_err!(Error::NotEmpty, lfs_remove(lfs, "hotpotato"));

    assert_ok!(lfs_mkdir(lfs, "coldpotato"));
    assert_ok!(lfs_rename(lfs, "hotpotato/baked", "coldpotato/baked"));
    assert_ok!(lfs_rename(lfs, "hotpotato/fried", "coldpotato/fried"));
    assert_ok!(lfs_rename(lfs, "hotpotato/sweet", "coldpotato/sweet"));

    assert_ok!(lfs_remove(lfs, "hotpotato"));
    assert_ok!(lfs_rename(lfs, "coldpotato", "hotpotato"));

    assert_ok!(lfs_remove(lfs, "hotpotato/baked"));
    assert_ok!(lfs_remove(lfs, "hotpotato/fried"));
    assert_ok!(lfs_remove(lfs, "hotpotato/sweet"));
    assert_ok!(lfs_remove(lfs, "hotpotato"));

    let names = dir_entry_names(lfs, &env.config, "/").expect("root dir_entry_names");
    assert_eq!(names, vec!["burito"]);

    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_dirs_recursive_remove]
/// defines.N = [10, 100], if = 'N < BLOCK_COUNT/2'
/// Create parent dir with N subdirs, remove children during dir iteration, then parent.
#[test]
fn test_dirs_recursive_remove() {
    init_logger();
    for n in [10usize, 100] {
        let mut env = default_config(256);
        init_context(&mut env);

        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, &env.config));
        assert_ok!(lfs_mount(lfs, &env.config));

        assert_ok!(lfs_mkdir(lfs, "prickly-pear"));
        for i in 0..n {
            let path = &format!("prickly-pear/cactus{i:03}");
            assert_ok!(lfs_mkdir(lfs, path));
        }

        let names = dir_entry_names(lfs, &env.config, "prickly-pear")
            .expect("prickly-pear dir_entry_names");
        assert_eq!(names.len(), n, "N={n} subdir count");

        assert_err!(Error::NotEmpty, lfs_remove(lfs, "prickly-pear"));

        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_ok!(lfs_dir_open(lfs, dir, "prickly-pear"));
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        loop {
            let rc = lfs_dir_read(lfs, dir, info);
            if rc == Ok(false) {
                break;
            }
            assert_eq!(rc, Ok(true), "N={n}, unexpected dir_read result");
            if info.name[0] == b'.' {
                continue;
            }
            let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
            let name = core::str::from_utf8(&info.name[..nul]).unwrap();
            let child_path = &format!("prickly-pear/{name}");
            assert_ok!(lfs_remove(lfs, child_path));
        }
        assert_ok!(lfs_dir_close(lfs, dir));

        assert_ok!(lfs_remove(lfs, "prickly-pear"));
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &env.config));
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_err!(Error::NoEntry, lfs_stat(lfs, "prickly-pear", info));
        assert_ok!(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_dirs_remove_read]
/// defines.N = 10, if = 'N < BLOCK_COUNT/2'
/// Create N dirs under prickly-pear/. Nested loop: open dir, iterate to j, remove dir k, iterate rest,
/// close, recreate k, unmount. Requires lfs_dir_seek.
#[lfs_test]
fn test_dirs_remove_read(cfg: &LfsConfig) {
    const N: usize = 10;

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    assert_ok!(lfs_mkdir(lfs, "prickly-pear"));
    for i in 0..N {
        let path = &format!("prickly-pear/cactus{i:03}");
        assert_ok!(lfs_mkdir(lfs, path));
    }

    for k in 0..N {
        for j in 0..=N {
            let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
            assert_ok!(lfs_dir_open(lfs, dir, "prickly-pear"));
            assert_ok!(lfs_dir_rewind(lfs, dir));
            assert_ok!(lfs_dir_seek(lfs, dir, j as _));
            assert_ok!(lfs_remove(lfs, &format!("prickly-pear/cactus{k:03}")));
            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            while lfs_dir_read(lfs, dir, info) == Ok(true) {}
            assert_ok!(lfs_dir_close(lfs, dir));
            assert_ok!(lfs_mkdir(lfs, &format!("prickly-pear/cactus{k:03}")));
        }
        assert_ok!(lfs_unmount(lfs));
        assert_ok!(lfs_mount(lfs, cfg));
    }

    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_dirs_other_errors]
/// Tests various error conditions for dirs and files.
#[test]
fn test_dirs_other_errors() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_mkdir(lfs, "potato"));
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "burito",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok!(lfs_file_close(lfs, file));

    assert_err!(Error::Exists, lfs_mkdir(lfs, "potato"));
    assert_err!(Error::Exists, lfs_mkdir(lfs, "burito"));

    let file = &mut LfsFile::default();
    assert_err!(
        Error::Exists,
        lfs_file_open(lfs, file, "burito", LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
    );
    let file = &mut LfsFile::default();
    assert_err!(
        Error::Exists,
        lfs_file_open(lfs, file, "potato", LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
    );

    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_err!(Error::NoEntry, lfs_dir_open(lfs, dir, "tomato"));
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_err!(Error::NotDir, lfs_dir_open(lfs, dir, "burito"));

    let file = &mut LfsFile::default();
    assert_err!(
        Error::NoEntry,
        lfs_file_open(lfs, file, "tomato", LFS_O_RDONLY),
    );
    let file = &mut LfsFile::default();
    assert_err!(
        Error::IsDir,
        lfs_file_open(lfs, file, "potato", LFS_O_RDONLY),
    );

    let file = &mut LfsFile::default();
    assert_err!(
        Error::NoEntry,
        lfs_file_open(lfs, file, "tomato", LFS_O_WRONLY),
    );
    let file = &mut LfsFile::default();
    assert_err!(
        Error::IsDir,
        lfs_file_open(lfs, file, "potato", LFS_O_WRONLY),
    );

    let file = &mut LfsFile::default();
    assert_err!(
        Error::IsDir,
        lfs_file_open(lfs, file, "potato", LFS_O_WRONLY | LFS_O_CREAT),
    );

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "tacoto",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    assert_ok!(lfs_file_close(lfs, file));

    assert_err!(Error::IsDir, lfs_rename(lfs, "tacoto", "potato"));
    assert_err!(Error::NotDir, lfs_rename(lfs, "potato", "tacoto"));

    assert_err!(Error::Exists, lfs_mkdir(lfs, "/"));
    let file = &mut LfsFile::default();
    assert_err!(
        Error::Exists,
        lfs_file_open(lfs, file, "/", LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
    );
    let file = &mut LfsFile::default();
    assert_err!(Error::IsDir, lfs_file_open(lfs, file, "/", LFS_O_RDONLY));
    let file = &mut LfsFile::default();
    assert_err!(Error::IsDir, lfs_file_open(lfs, file, "/", LFS_O_WRONLY));
    let file = &mut LfsFile::default();
    assert_err!(
        Error::IsDir,
        lfs_file_open(lfs, file, "/", LFS_O_WRONLY | LFS_O_CREAT),
    );

    let names = dir_entry_names(lfs, &env.config, "/").expect("root listing");
    let mut names_sorted = names.clone();
    names_sorted.sort();
    assert_eq!(names_sorted, vec!["burito", "potato", "tacoto"]);

    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    let names = dir_entry_names(lfs, &env.config, "/").expect("root listing after remount");
    let mut names_sorted = names.clone();
    names_sorted.sort();
    assert_eq!(names_sorted, vec!["burito", "potato", "tacoto"]);
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_dirs_seek]
/// defines.COUNT = [4, 128, 132], if = 'COUNT < BLOCK_COUNT/2'
/// Create COUNT entries in a child dir. Exercise lfs_dir_seek, lfs_dir_tell, lfs_dir_rewind.
#[lfs_test]
fn test_dirs_seek(cfg: &LfsConfig) {
    for count in [4usize, 128, 132] {
        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, cfg));
        assert_ok!(lfs_mount(lfs, cfg));

        assert_ok!(lfs_mkdir(lfs, "child"));
        for i in 0..count {
            let path = &format!("child/entry{i:03}");
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok!(lfs_file_close(lfs, file));
        }

        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_ok!(lfs_dir_open(lfs, dir, "child"));
        assert_ok!(lfs_dir_rewind(lfs, dir));
        let pos0 = lfs_dir_tell(lfs, dir);
        assert!(pos0 >= 0, "tell after rewind");

        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        let mut n = 0usize;
        while lfs_dir_read(lfs, dir, info) == Ok(true) {
            n += 1;
        }
        assert_eq!(n, count + 2, "COUNT={count}: . and .. plus {count} entries");

        assert_ok!(lfs_dir_rewind(lfs, dir));
        let half = (count + 2) / 2;
        assert_ok!(lfs_dir_seek(lfs, dir, half as u32));
        let pos_half = lfs_dir_tell(lfs, dir);
        assert!(pos_half >= 0, "tell after seek");

        assert_ok!(lfs_dir_rewind(lfs, dir));
        let pos_rewind = lfs_dir_tell(lfs, dir);
        assert_eq!(pos_rewind, pos0, "tell after rewind matches initial");

        assert_ok!(lfs_dir_close(lfs, dir));
        assert_ok!(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_dirs_toot_seek]
/// defines.COUNT = [4, 128, 132]
/// Same as seek but on root directory.
#[test]
fn test_dirs_toot_seek() {
    init_logger();
    for count in [4usize, 128, 132] {
        let mut env = default_config(512);
        init_context(&mut env);

        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, &env.config));
        assert_ok!(lfs_mount(lfs, &env.config));

        for i in 0..count {
            let path = &format!("entry{i:03}");
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok!(lfs_file_close(lfs, file));
        }

        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_ok!(lfs_dir_open(lfs, dir, ROOT_PATH));
        assert_ok!(lfs_dir_rewind(lfs, dir));
        let pos0 = lfs_dir_tell(lfs, dir);
        assert!(pos0 >= 0, "tell after rewind");

        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        let mut n = 0usize;
        while lfs_dir_read(lfs, dir, info) == Ok(true) {
            n += 1;
        }
        assert_eq!(n, count + 2, "COUNT={count}: . and .. plus {count} entries");

        assert_ok!(lfs_dir_rewind(lfs, dir));
        let half = (count + 2) / 2;
        assert_ok!(lfs_dir_seek(lfs, dir, half as u32));
        let _pos_half = lfs_dir_tell(lfs, dir);

        assert_ok!(lfs_dir_rewind(lfs, dir));
        let pos_rewind = lfs_dir_tell(lfs, dir);
        assert_eq!(pos_rewind, pos0, "tell after rewind matches initial");

        assert_ok!(lfs_dir_close(lfs, dir));
        assert_ok!(lfs_unmount(lfs));
    }
}
