//! Rust (littlefs-rust-core) wrapper for compat tests.

use std::mem::MaybeUninit;

use littlefs_rust_core::{error::Error, lfs_type::OpenFlags};

use crate::storage::{prng_verify, test_prng, SharedStorage};

#[allow(unused)]
const LFS_O_RDONLY: i32 = 1;
const LFS_O_WRONLY: i32 = 2;
const LFS_O_CREAT: i32 = 0x0100;
const LFS_O_EXCL: i32 = 0x0200;
// const LFS_ERR_EXIST: i32 = -17;

// ── Operation-level helpers (phase 2) ───────────────────────────────────

pub fn format(storage: &SharedStorage) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    littlefs_rust_core::lfs_format(lfs, &env.config)?;
    littlefs_rust_core::lfs_mount(lfs, &env.config)?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub fn mount_dir_names(storage: &SharedStorage, path: &str) -> Result<Vec<String>, Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    littlefs_rust_core::lfs_mount(lfs, &env.config)?;
    let names = dir_names_mounted(lfs, path)?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(names)
}

pub fn mount_read_file(storage: &SharedStorage, path: &str) -> Result<Vec<u8>, Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    littlefs_rust_core::lfs_mount(lfs, &env.config)?;
    let data = read_file_mounted(lfs, path)?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(data)
}

pub fn format_mkdir_unmount(storage: &SharedStorage, dir_name: &str) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    littlefs_rust_core::lfs_format(lfs, &env.config)?;
    littlefs_rust_core::lfs_mount(lfs, &env.config)?;
    mkdir_mounted(lfs, dir_name)?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub fn format_mkdir_file_unmount(
    storage: &SharedStorage,
    dir_name: &str,
    file_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    littlefs_rust_core::lfs_format(lfs, &env.config)?;
    littlefs_rust_core::lfs_mount(lfs, &env.config)?;
    mkdir_mounted(lfs, dir_name)?;
    create_empty_file_mounted(lfs, file_name)?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub fn format_file_mkdir_unmount(
    storage: &SharedStorage,
    file_name: &str,
    dir_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    littlefs_rust_core::lfs_format(lfs, &env.config)?;
    littlefs_rust_core::lfs_mount(lfs, &env.config)?;
    create_empty_file_mounted(lfs, file_name)?;
    mkdir_mounted(lfs, dir_name)?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub fn format_create_three_unmount(storage: &SharedStorage) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    littlefs_rust_core::lfs_format(lfs, &env.config)?;
    littlefs_rust_core::lfs_mount(lfs, &env.config)?;
    for name in ["aaa", "zzz", "mmm"] {
        create_empty_file_mounted(lfs, name)?;
    }
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub fn format_create_rename_unmount(
    storage: &SharedStorage,
    old_name: &str,
    new_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    littlefs_rust_core::lfs_format(lfs, &env.config)?;
    littlefs_rust_core::lfs_mount(lfs, &env.config)?;
    create_empty_file_mounted(lfs, old_name)?;
    (littlefs_rust_core::lfs_rename(lfs, old_name, new_name))?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn format_create_remove_unmount(storage: &SharedStorage, path: &str) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_format(lfs, &env.config))?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    create_empty_file_mounted(lfs, path)?;
    (littlefs_rust_core::lfs_remove(lfs, path))?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn format_create_write_unmount(
    storage: &SharedStorage,
    path: &str,
    content: &[u8],
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_format(lfs, &env.config))?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    write_file_mounted(lfs, path, content)?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn format_nested_dir_file_unmount(
    storage: &SharedStorage,
    parent: &str,
    child: &str,
    file_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_format(lfs, &env.config))?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    mkdir_mounted(lfs, parent)?;
    let child_path = format!("{parent}/{child}");
    mkdir_mounted(lfs, &child_path)?;
    let file_path = format!("{child_path}/{file_name}");
    create_empty_file_mounted(lfs, &file_path)?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn format_mkdir_file_rmdir_unmount(
    storage: &SharedStorage,
    dir_name: &str,
    file_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_format(lfs, &env.config))?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    mkdir_mounted(lfs, dir_name)?;
    let file_path = format!("{dir_name}/{file_name}");
    create_empty_file_mounted(lfs, &file_path)?;
    (littlefs_rust_core::lfs_remove(lfs, &file_path))?;
    (littlefs_rust_core::lfs_remove(lfs, dir_name))?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn mount_mkdir_expect_exist(storage: &SharedStorage, path: &str) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    let res = littlefs_rust_core::lfs_mkdir(lfs, path);
    (littlefs_rust_core::lfs_unmount(lfs))?;
    if res == Err(Error::Exists) {
        Ok(())
    } else if res.is_ok() {
        Err(Error::Invalid)
    } else {
        Err(res.unwrap_err())
    }
}

// ── Compat-level helpers (phase 3) ──────────────────────────────────────

pub fn format_only(storage: &SharedStorage) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };
    (littlefs_rust_core::lfs_format(lfs, &env.config))?;
    Ok(())
}

pub fn format_create_n_dirs(storage: &SharedStorage, count: usize) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_format(lfs, &env.config))?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    for i in 0..count {
        mkdir_mounted(lfs, &format!("dir{i}"))?;
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn format_create_n_files_prng(
    storage: &SharedStorage,
    count: usize,
    size: u32,
    chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_format(lfs, &env.config))?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    for i in 0..count {
        write_prng_file_mounted(lfs, &format!("file{i}"), size, chunk, (i + 1) as u32)?;
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn format_create_n_dirs_with_files_prng(
    storage: &SharedStorage,
    count: usize,
    size: u32,
    chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_format(lfs, &env.config))?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    for i in 0..count {
        let dir = format!("dir{i}");
        mkdir_mounted(lfs, &dir)?;
        write_prng_file_mounted(lfs, &format!("{dir}/file"), size, chunk, (i + 1) as u32)?;
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn mount_verify_n_empty_dirs(storage: &SharedStorage, count: usize) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    let root = dir_names_mounted(lfs, "/")?;
    assert_eq!(
        root.len(),
        count,
        "expected {count} dirs, got {}",
        root.len()
    );
    for i in 0..count {
        let name = format!("dir{i}");
        assert!(root.contains(&name), "missing {name}");
        let contents = dir_names_mounted(lfs, &name)?;
        assert!(
            contents.is_empty(),
            "dir {name} should be empty, got {contents:?}"
        );
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn mount_verify_n_files_prng(
    storage: &SharedStorage,
    count: usize,
    size: u32,
    _chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    let root = dir_names_mounted(lfs, "/")?;
    assert_eq!(
        root.len(),
        count,
        "expected {count} files, got {}",
        root.len()
    );
    for i in 0..count {
        let path = format!("file{i}");
        let data = read_file_mounted(lfs, &path)?;
        assert_eq!(data.len(), size as usize, "file {path} size mismatch");
        prng_verify(&data, (i + 1) as u32);
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn mount_verify_n_dirs_with_files_prng(
    storage: &SharedStorage,
    count: usize,
    size: u32,
    _chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    let root = dir_names_mounted(lfs, "/")?;
    assert_eq!(
        root.len(),
        count,
        "expected {count} dirs, got {}",
        root.len()
    );
    for i in 0..count {
        let dir = format!("dir{i}");
        let contents = dir_names_mounted(lfs, &dir)?;
        assert_eq!(contents.len(), 1, "dir {dir} should have 1 file");
        let data = read_file_mounted(lfs, &format!("{dir}/file"))?;
        assert_eq!(data.len(), size as usize);
        prng_verify(&data, (i + 1) as u32);
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn mount_create_dirs_and_list(
    storage: &SharedStorage,
    start: usize,
    count: usize,
    expected: usize,
) -> Result<Vec<String>, Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    for i in start..(start + count) {
        mkdir_mounted(lfs, &format!("dir{i}"))?;
    }
    let root = dir_names_mounted(lfs, "/")?;
    assert_eq!(
        root.len(),
        expected,
        "expected {expected} entries, got {}",
        root.len()
    );
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(root)
}

pub fn mount_create_files_prng_and_verify_all(
    storage: &SharedStorage,
    start: usize,
    count: usize,
    total: usize,
    size: u32,
    chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    for i in start..(start + count) {
        write_prng_file_mounted(lfs, &format!("file{i}"), size, chunk, (i + 1) as u32)?;
    }
    let root = dir_names_mounted(lfs, "/")?;
    assert_eq!(
        root.len(),
        total,
        "expected {total} files, got {}",
        root.len()
    );
    for i in 0..total {
        let data = read_file_mounted(lfs, &format!("file{i}"))?;
        assert_eq!(data.len(), size as usize);
        prng_verify(&data, (i + 1) as u32);
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub fn mount_create_dirs_files_prng_and_verify_all(
    storage: &SharedStorage,
    start: usize,
    count: usize,
    total: usize,
    size: u32,
    chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut unsafe { MaybeUninit::<littlefs_rust_core::Lfs>::zeroed().assume_init() };

    (littlefs_rust_core::lfs_mount(lfs, &env.config))?;
    for i in start..(start + count) {
        let dir = format!("dir{i}");
        mkdir_mounted(lfs, &dir)?;
        write_prng_file_mounted(lfs, &format!("{dir}/file"), size, chunk, (i + 1) as u32)?;
    }
    let root = dir_names_mounted(lfs, "/")?;
    assert_eq!(
        root.len(),
        total,
        "expected {total} dirs, got {}",
        root.len()
    );
    for i in 0..total {
        let dir = format!("dir{i}");
        let data = read_file_mounted(lfs, &format!("{dir}/file"))?;
        assert_eq!(data.len(), size as usize);
        prng_verify(&data, (i + 1) as u32);
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

// ── Internal helpers ────────────────────────────────────────────────────

fn mkdir_mounted(lfs: &mut littlefs_rust_core::Lfs, path: &str) -> Result<(), Error> {
    littlefs_rust_core::lfs_mkdir(lfs, path)
}

fn create_empty_file_mounted(lfs: &mut littlefs_rust_core::Lfs, path: &str) -> Result<(), Error> {
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = unsafe { MaybeUninit::<littlefs_rust_core::LfsFile>::zeroed().assume_init() };
    littlefs_rust_core::lfs_file_open(
        lfs,
        &mut file,
        path,
        OpenFlags::from_bits_retain(flags as u32),
    )?;
    littlefs_rust_core::lfs_file_close(lfs, &mut file)
}

fn write_file_mounted(
    lfs: &mut littlefs_rust_core::Lfs,
    path: &str,
    content: &[u8],
) -> Result<(), Error> {
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let file = &mut unsafe { MaybeUninit::<littlefs_rust_core::LfsFile>::zeroed().assume_init() };
    (littlefs_rust_core::lfs_file_open(
        lfs,
        file,
        path,
        OpenFlags::from_bits_retain(flags as u32),
    ))?;
    let n = littlefs_rust_core::lfs_file_write(lfs, file, content);
    (littlefs_rust_core::lfs_file_close(lfs, file))?;
    if let Err(err) = n {
        return Err(err);
    }
    assert_eq!(n.unwrap() as usize, content.len(), "short write");
    Ok(())
}

fn write_prng_file_mounted(
    lfs: &mut littlefs_rust_core::Lfs,
    path: &str,
    size: u32,
    chunk: u32,
    seed: u32,
) -> Result<(), Error> {
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let file = &mut unsafe { MaybeUninit::<littlefs_rust_core::LfsFile>::zeroed().assume_init() };
    (littlefs_rust_core::lfs_file_open(
        lfs,
        file,
        path,
        OpenFlags::from_bits_retain(flags as u32),
    ))?;

    let mut prng = seed;
    let mut buf = vec![0u8; chunk as usize];
    let mut i: u32 = 0;
    while i < size {
        let c = std::cmp::min(chunk, size - i);
        for slot in buf[..c as usize].iter_mut() {
            *slot = (test_prng(&mut prng) & 0xff) as u8;
        }
        let n = littlefs_rust_core::lfs_file_write(lfs, file, &buf[..c as usize]);
        assert_eq!(n.unwrap(), c as u32, "short write at offset {i}");
        i += c;
    }
    littlefs_rust_core::lfs_file_close(lfs, file)
}

fn read_file_mounted(lfs: &mut littlefs_rust_core::Lfs, path: &str) -> Result<Vec<u8>, Error> {
    let file = &mut unsafe { MaybeUninit::<littlefs_rust_core::LfsFile>::zeroed().assume_init() };
    (littlefs_rust_core::lfs_file_open(lfs, file, path, OpenFlags::READ))?;

    let mut buf = Vec::new();
    let mut chunk = [0u8; 256];
    loop {
        let n = littlefs_rust_core::lfs_file_read(lfs, file, &mut chunk);
        if let Err(err) = n {
            let _ = littlefs_rust_core::lfs_file_close(lfs, file);
            return Err(err);
        }
        if n == Ok(0) {
            break;
        }
        buf.extend_from_slice(&chunk[..(n.unwrap()) as usize]);
    }
    (littlefs_rust_core::lfs_file_close(lfs, file))?;
    Ok(buf)
}

fn dir_names_mounted(lfs: &mut littlefs_rust_core::Lfs, path: &str) -> Result<Vec<String>, Error> {
    let dir = &mut unsafe { MaybeUninit::<littlefs_rust_core::LfsDir>::zeroed().assume_init() };
    (littlefs_rust_core::lfs_dir_open(lfs, dir, path))?;

    let mut names = Vec::new();
    let info = &mut unsafe { MaybeUninit::<littlefs_rust_core::LfsInfo>::zeroed().assume_init() };
    loop {
        let res = littlefs_rust_core::lfs_dir_read(lfs, dir, info);
        if res == Ok(0) {
            break;
        }
        if let Err(err) = res {
            let _ = littlefs_rust_core::lfs_dir_close(lfs, dir);
            return Err(err);
        }
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        let name = core::str::from_utf8(&info.name[..nul])
            .unwrap_or("")
            .to_string();
        if name != "." && name != ".." {
            names.push(name);
        }
    }
    let _ = littlefs_rust_core::lfs_dir_close(lfs, dir);
    Ok(names)
}
