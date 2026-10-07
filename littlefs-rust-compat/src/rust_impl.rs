//! Rust (littlefs-rust-core) wrapper for compat tests.

use std::mem::MaybeUninit;

use littlefs_rust_core::{Error, LfsFile, Storage, lfs_type::OpenFlags};

use crate::storage::{SharedStorage, prng_verify, test_prng};

#[allow(unused)]
const LFS_O_RDONLY: i32 = 1;
const LFS_O_WRONLY: i32 = 2;
const LFS_O_CREAT: i32 = 0x0100;
const LFS_O_EXCL: i32 = 0x0200;
// const LFS_ERR_EXIST: i32 = -17;

// ── Operation-level helpers (phase 2) ───────────────────────────────────

pub async fn format(storage: &mut SharedStorage) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_format(lfs, &env.config).await?;
    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub async fn mount_dir_names(
    storage: &mut SharedStorage,
    path: &str,
) -> Result<Vec<String>, Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    let names = dir_names_mounted(lfs, path).await?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(names)
}

pub async fn mount_read_file(storage: &mut SharedStorage, path: &str) -> Result<Vec<u8>, Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    let data = read_file_mounted(lfs, path).await?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(data)
}

pub async fn format_mkdir_unmount(
    storage: &mut SharedStorage,
    dir_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_format(lfs, &env.config).await?;
    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    mkdir_mounted(lfs, dir_name).await?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub async fn format_mkdir_file_unmount(
    storage: &mut SharedStorage,
    dir_name: &str,
    file_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_format(lfs, &env.config).await?;
    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    mkdir_mounted(lfs, dir_name).await?;
    create_empty_file_mounted(lfs, file_name).await?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub async fn format_file_mkdir_unmount(
    storage: &mut SharedStorage,
    file_name: &str,
    dir_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_format(lfs, &env.config).await?;
    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    create_empty_file_mounted(lfs, file_name).await?;
    mkdir_mounted(lfs, dir_name).await?;
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub async fn format_create_three_unmount(storage: &mut SharedStorage) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_format(lfs, &env.config).await?;
    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    for name in ["aaa", "zzz", "mmm"] {
        create_empty_file_mounted(lfs, name).await?;
    }
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

pub async fn format_create_rename_unmount(
    storage: &mut SharedStorage,
    old_name: &str,
    new_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_format(lfs, &env.config).await?;
    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    create_empty_file_mounted(lfs, old_name).await?;
    (littlefs_rust_core::lfs_rename(lfs, old_name, new_name)).await?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn format_create_remove_unmount(
    storage: &mut SharedStorage,
    path: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_format(lfs, &env.config)).await?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    create_empty_file_mounted(lfs, path).await?;
    (littlefs_rust_core::lfs_remove(lfs, path)).await?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn format_create_write_unmount(
    storage: &mut SharedStorage,
    path: &str,
    content: &[u8],
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_format(lfs, &env.config)).await?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    write_file_mounted(lfs, path, content).await?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn format_nested_dir_file_unmount(
    storage: &mut SharedStorage,
    parent: &str,
    child: &str,
    file_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_format(lfs, &env.config)).await?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    mkdir_mounted(lfs, parent).await?;
    let child_path = format!("{parent}/{child}");
    mkdir_mounted(lfs, &child_path).await?;
    let file_path = format!("{child_path}/{file_name}");
    create_empty_file_mounted(lfs, &file_path).await?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn format_mkdir_file_rmdir_unmount(
    storage: &mut SharedStorage,
    dir_name: &str,
    file_name: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_format(lfs, &env.config)).await?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    mkdir_mounted(lfs, dir_name).await?;
    let file_path = format!("{dir_name}/{file_name}");
    create_empty_file_mounted(lfs, &file_path).await?;
    (littlefs_rust_core::lfs_remove(lfs, &file_path)).await?;
    (littlefs_rust_core::lfs_remove(lfs, dir_name)).await?;
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn mount_mkdir_expect_exist(
    storage: &mut SharedStorage,
    path: &str,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    let res = littlefs_rust_core::lfs_mkdir(lfs, path).await;
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

pub async fn format_only(storage: &mut SharedStorage) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();
    (littlefs_rust_core::lfs_format(lfs, &env.config)).await?;
    Ok(())
}

pub async fn format_create_n_dirs(storage: &mut SharedStorage, count: usize) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_format(lfs, &env.config)).await?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    for i in 0..count {
        mkdir_mounted(lfs, &format!("dir{i}")).await?;
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn format_create_n_files_prng(
    storage: &mut SharedStorage,
    count: usize,
    size: u32,
    chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_format(lfs, &env.config)).await?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    for i in 0..count {
        write_prng_file_mounted(lfs, &format!("file{i}"), size, chunk, (i + 1) as u32).await?;
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn format_create_n_dirs_with_files_prng(
    storage: &mut SharedStorage,
    count: usize,
    size: u32,
    chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_format(lfs, &env.config)).await?;
    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    for i in 0..count {
        let dir = format!("dir{i}");
        mkdir_mounted(lfs, &dir).await?;
        write_prng_file_mounted(lfs, &format!("{dir}/file"), size, chunk, (i + 1) as u32).await?;
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn mount_verify_n_empty_dirs(
    storage: &mut SharedStorage,
    count: usize,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    let root = dir_names_mounted(lfs, "/").await?;
    assert_eq!(
        root.len(),
        count,
        "expected {count} dirs, got {}",
        root.len()
    );
    for i in 0..count {
        let name = format!("dir{i}");
        assert!(root.contains(&name), "missing {name}");
        let contents = dir_names_mounted(lfs, &name).await?;
        assert!(
            contents.is_empty(),
            "dir {name} should be empty, got {contents:?}"
        );
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn mount_verify_n_files_prng(
    storage: &mut SharedStorage,
    count: usize,
    size: u32,
    _chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    (littlefs_rust_core::lfs_mount(lfs, &env.config)).await?;
    let root = dir_names_mounted(lfs, "/").await?;
    assert_eq!(
        root.len(),
        count,
        "expected {count} files, got {}",
        root.len()
    );
    for i in 0..count {
        let path = format!("file{i}");
        let data = read_file_mounted(lfs, &path).await?;
        assert_eq!(data.len(), size as usize, "file {path} size mismatch");
        prng_verify(&data, (i + 1) as u32);
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn mount_verify_n_dirs_with_files_prng(
    storage: &mut SharedStorage,
    count: usize,
    size: u32,
    _chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    let root = dir_names_mounted(lfs, "/").await?;
    assert_eq!(
        root.len(),
        count,
        "expected {count} dirs, got {}",
        root.len()
    );
    for i in 0..count {
        let dir = format!("dir{i}");
        let contents = dir_names_mounted(lfs, &dir).await?;
        assert_eq!(contents.len(), 1, "dir {dir} should have 1 file");
        let data = read_file_mounted(lfs, &format!("{dir}/file")).await?;
        assert_eq!(data.len(), size as usize);
        prng_verify(&data, (i + 1) as u32);
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn mount_create_dirs_and_list(
    storage: &mut SharedStorage,
    start: usize,
    count: usize,
    expected: usize,
) -> Result<Vec<String>, Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    for i in start..(start + count) {
        mkdir_mounted(lfs, &format!("dir{i}")).await?;
    }
    let root = dir_names_mounted(lfs, "/").await?;
    assert_eq!(
        root.len(),
        expected,
        "expected {expected} entries, got {}",
        root.len()
    );
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(root)
}

pub async fn mount_create_files_prng_and_verify_all(
    storage: &mut SharedStorage,
    start: usize,
    count: usize,
    total: usize,
    size: u32,
    chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    for i in start..(start + count) {
        write_prng_file_mounted(lfs, &format!("file{i}"), size, chunk, (i + 1) as u32).await?;
    }
    let root = dir_names_mounted(lfs, "/").await?;
    assert_eq!(
        root.len(),
        total,
        "expected {total} files, got {}",
        root.len()
    );
    for i in 0..total {
        let data = read_file_mounted(lfs, &format!("file{i}")).await?;
        assert_eq!(data.len(), size as usize);
        prng_verify(&data, (i + 1) as u32);
    }
    (littlefs_rust_core::lfs_unmount(lfs))?;
    Ok(())
}

pub async fn mount_create_dirs_files_prng_and_verify_all(
    storage: &mut SharedStorage,
    start: usize,
    count: usize,
    total: usize,
    size: u32,
    chunk: u32,
) -> Result<(), Error> {
    let env = storage.build_rust_env();
    let lfs = &mut littlefs_rust_core::Lfs::default();

    littlefs_rust_core::lfs_mount(lfs, &env.config).await?;
    for i in start..(start + count) {
        let dir = format!("dir{i}");
        mkdir_mounted(lfs, &dir).await?;
        write_prng_file_mounted(lfs, &format!("{dir}/file"), size, chunk, (i + 1) as u32).await?;
    }
    let root = dir_names_mounted(lfs, "/").await?;
    assert_eq!(
        root.len(),
        total,
        "expected {total} dirs, got {}",
        root.len()
    );
    for i in 0..total {
        let dir = format!("dir{i}");
        let data = read_file_mounted(lfs, &format!("{dir}/file")).await?;
        assert_eq!(data.len(), size as usize);
        prng_verify(&data, (i + 1) as u32);
    }
    littlefs_rust_core::lfs_unmount(lfs)?;
    Ok(())
}

// ── Internal helpers ────────────────────────────────────────────────────

async fn mkdir_mounted<S: Storage>(
    lfs: &mut littlefs_rust_core::Lfs<S>,
    path: &str,
) -> Result<(), Error> {
    littlefs_rust_core::lfs_mkdir(lfs, path).await
}

async fn create_empty_file_mounted<S: Storage>(
    lfs: &mut littlefs_rust_core::Lfs<S>,
    path: &str,
) -> Result<(), Error> {
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let mut file = LfsFile::default();
    littlefs_rust_core::lfs_file_open(
        lfs,
        &mut file,
        path,
        OpenFlags::from_bits_retain(flags as u32),
    )
    .await?;
    littlefs_rust_core::lfs_file_close(lfs, &mut file).await
}

async fn write_file_mounted<S: Storage>(
    lfs: &mut littlefs_rust_core::Lfs<S>,
    path: &str,
    content: &[u8],
) -> Result<(), Error> {
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let file = &mut littlefs_rust_core::LfsFile::default();
    littlefs_rust_core::lfs_file_open(lfs, file, path, OpenFlags::from_bits_retain(flags as u32))
        .await?;
    let n = littlefs_rust_core::lfs_file_write(lfs, file, content).await;
    littlefs_rust_core::lfs_file_close(lfs, file).await?;
    if let Err(err) = n {
        return Err(err);
    }
    assert_eq!(n.unwrap() as usize, content.len(), "short write");
    Ok(())
}

async fn write_prng_file_mounted<S: Storage>(
    lfs: &mut littlefs_rust_core::Lfs<S>,
    path: &str,
    size: u32,
    chunk: u32,
    seed: u32,
) -> Result<(), Error> {
    let flags = LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL;
    let file = &mut littlefs_rust_core::LfsFile::default();
    littlefs_rust_core::lfs_file_open(lfs, file, path, OpenFlags::from_bits_retain(flags as u32))
        .await?;

    let mut prng = seed;
    let mut buf = vec![0u8; chunk as usize];
    let mut i: u32 = 0;
    while i < size {
        let c = std::cmp::min(chunk, size - i);
        for slot in buf[..c as usize].iter_mut() {
            *slot = (test_prng(&mut prng) & 0xff) as u8;
        }
        let n = littlefs_rust_core::lfs_file_write(lfs, file, &buf[..c as usize]).await;
        assert_eq!(n.unwrap(), c as u32, "short write at offset {i}");
        i += c;
    }
    littlefs_rust_core::lfs_file_close(lfs, file).await
}

async fn read_file_mounted<S: Storage>(
    lfs: &mut littlefs_rust_core::Lfs<S>,
    path: &str,
) -> Result<Vec<u8>, Error> {
    let file = &mut littlefs_rust_core::LfsFile::default();
    littlefs_rust_core::lfs_file_open(lfs, file, path, OpenFlags::READ).await?;

    let mut buf = Vec::new();
    let mut chunk = [0u8; 256];
    loop {
        let n = littlefs_rust_core::lfs_file_read(lfs, file, &mut chunk).await;
        if let Err(err) = n {
            let _ = littlefs_rust_core::lfs_file_close(lfs, file).await;
            return Err(err);
        }
        if n == Ok(0) {
            break;
        }
        buf.extend_from_slice(&chunk[..(n.unwrap()) as usize]);
    }
    littlefs_rust_core::lfs_file_close(lfs, file).await?;
    Ok(buf)
}

async fn dir_names_mounted<S: Storage>(
    lfs: &mut littlefs_rust_core::Lfs<S>,
    path: &str,
) -> Result<Vec<String>, Error> {
    let dir = &mut unsafe { MaybeUninit::<littlefs_rust_core::LfsDir>::zeroed().assume_init() };
    (littlefs_rust_core::lfs_dir_open(lfs, dir, path).await)?;

    let mut names = Vec::new();
    let info = &mut unsafe { MaybeUninit::<littlefs_rust_core::LfsInfo>::zeroed().assume_init() };
    loop {
        let res = littlefs_rust_core::lfs_dir_read(lfs, dir, info).await;
        if res == Ok(false) {
            break;
        }
        if let Err(err) = res {
            let _ = littlefs_rust_core::lfs_dir_close(lfs, dir);
            return Err(err);
        }
        let name = info.name_str();
        if name != "." && name != ".." {
            names.push(name.to_string());
        }
    }
    let _ = littlefs_rust_core::lfs_dir_close(lfs, dir);
    Ok(names)
}
