use std::string::String;

use littlefs_rust::{Config, Error, FileType, Filesystem, OpenFlags, SeekFrom};

type RamStorage = littlefs_rust::RamStorage<512, 128>;

async fn format_and_mount() -> Filesystem<RamStorage> {
    let mut storage = RamStorage::new();
    let config = Config::new(512, 128);
    Filesystem::format(&mut storage, &config)
        .await
        .expect("format");
    Filesystem::mount(storage, config)
        .await
        .map_err(|(e, _)| e)
        .expect("mount")
}

#[tokio::test]
async fn test_format_mount_unmount() {
    let mut storage = RamStorage::new();
    let config = Config::new(512, 128);
    Filesystem::format(&mut storage, &config).await.unwrap();
    let fs = Filesystem::mount(storage, config)
        .await
        .map_err(|(e, _)| e)
        .unwrap();
    let _storage = fs.unmount().unwrap();
}

#[tokio::test]
async fn test_mount_unformatted_fails() {
    let storage = RamStorage::new();
    let config = Config::new(512, 128);
    let result = Filesystem::mount(storage, config).await;
    let (err, recovered) = result.err().expect("mount should fail");
    assert_eq!(err, Error::Corrupt);
    assert_eq!(recovered.block_size(), 512);
}

#[tokio::test]
async fn test_drop_unmounts() {
    let mut storage = RamStorage::new();
    let config = Config::new(512, 128);
    Filesystem::format(&mut storage, &config).await.unwrap();
    {
        let _fs = Filesystem::mount(storage, config)
            .await
            .map_err(|(e, _)| e)
            .unwrap();
    }
    // No panic — Drop ran unmount
}

#[tokio::test]
async fn test_format_does_not_consume_storage() {
    let mut storage = RamStorage::new();
    let config = Config::new(512, 128);
    Filesystem::format(&mut storage, &config).await.unwrap();
    assert_eq!(storage.block_size(), 512);
}

#[tokio::test]
async fn test_write_read_roundtrip() {
    let fs = format_and_mount().await;
    let data = b"Hello, littlefs!";

    let mut file = fs
        .open("/hello.txt", OpenFlags::WRITE | OpenFlags::CREATE)
        .await
        .unwrap();
    file.write(data).await.unwrap();
    file.close().await.unwrap();

    let mut file = fs.open("/hello.txt", OpenFlags::READ).await.unwrap();
    let mut buf = vec![0u8; 64];
    let n = file.read(&mut buf).await.unwrap();
    assert_eq!(&buf[..n as usize], data);
    file.close().await.unwrap();

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_multiple_open_files() {
    let fs = format_and_mount().await;

    let mut f1 = fs
        .open("/a.txt", OpenFlags::WRITE | OpenFlags::CREATE)
        .await
        .unwrap();
    let mut f2 = fs
        .open("/b.txt", OpenFlags::WRITE | OpenFlags::CREATE)
        .await
        .unwrap();

    f1.write(b"aaa").await.unwrap();
    f2.write(b"bbb").await.unwrap();

    f1.close().await.unwrap();
    f2.close().await.unwrap();

    let data_a = fs.read_to_vec("/a.txt").await.unwrap();
    let data_b = fs.read_to_vec("/b.txt").await.unwrap();
    assert_eq!(data_a, b"aaa");
    assert_eq!(data_b, b"bbb");

    fs.unmount().unwrap();
}

#[tokio::test]
fn test_seek_tell_size() {
    let fs = format_and_mount().await;

    let mut file = fs
        .open("/data.bin", OpenFlags::WRITE | OpenFlags::CREATE)
        .await
        .unwrap();
    file.write(b"0123456789").await.unwrap();
    file.close().await.unwrap();

    let mut file = fs.open("/data.bin", OpenFlags::READ).await.unwrap();
    assert_eq!(file.size(), 10);
    assert_eq!(file.tell(), 0);

    file.seek(SeekFrom::Start(5)).await.unwrap();
    assert_eq!(file.tell(), 5);

    let mut buf = [0u8; 5];
    let n = file.read(&mut buf).await.unwrap();
    assert_eq!(n, 5);
    assert_eq!(&buf, b"56789");

    file.seek(SeekFrom::End(-3)).await.unwrap();
    assert_eq!(file.tell(), 7);

    file.close().await.unwrap();
    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_truncate() {
    let fs = format_and_mount().await;

    let mut file = fs
        .open("/trunc.txt", OpenFlags::WRITE | OpenFlags::CREATE)
        .await
        .unwrap();
    file.write(b"hello world").unwrap();
    file.close().unwrap();

    let mut file = fs.open("/trunc.txt", OpenFlags::WRITE).unwrap();
    file.truncate(5).unwrap();
    assert_eq!(file.size(), 5);
    file.close().unwrap();

    let data = fs.read_to_vec("/trunc.txt").unwrap();
    assert_eq!(data, b"hello");

    fs.unmount().unwrap();
}

#[tokio::test]
fn test_file_drop_closes() {
    let fs = format_and_mount().await;

    {
        let mut file = fs
            .open("/drop.txt", OpenFlags::WRITE | OpenFlags::CREATE)
            .unwrap();
        file.write(b"dropped").unwrap();
        // file dropped here without explicit close
    }

    let data = fs.read_to_vec("/drop.txt").unwrap();
    assert_eq!(data, b"dropped");

    fs.unmount().unwrap();
}

#[tokio::test]
fn test_open_nonexistent_fails() {
    let fs = format_and_mount().await;
    let result = fs.open("/nope.txt", OpenFlags::READ);
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), Error::NoEntry);
    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_mkdir_and_list() {
    let fs = format_and_mount().await;

    fs.mkdir("/docs").unwrap();
    fs.mkdir("/src").unwrap();

    let entries = fs.list_dir("/").unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"docs"));
    assert!(names.contains(&"src"));

    for entry in &entries {
        assert_eq!(entry.file_type, FileType::Dir);
    }

    fs.unmount().unwrap();
}

#[tokio::test]
fn test_remove_file() {
    let fs = format_and_mount().await;

    fs.write_file("/temp.txt", b"temp").unwrap();
    assert!(fs.exists("/temp.txt"));

    fs.remove("/temp.txt").unwrap();
    assert!(!fs.exists("/temp.txt"));

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_remove_dir() {
    let fs = format_and_mount().await;

    fs.mkdir("/empty").unwrap();
    assert!(fs.exists("/empty"));

    fs.remove("/empty").unwrap();
    assert!(!fs.exists("/empty"));

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_rename() {
    let fs = format_and_mount().await;

    fs.write_file("/old.txt", b"content").unwrap();
    fs.rename("/old.txt", "/new.txt").unwrap();

    assert!(!fs.exists("/old.txt"));
    let data = fs.read_to_vec("/new.txt").unwrap();
    assert_eq!(data, b"content");

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_stat() {
    let fs = format_and_mount().await;

    fs.write_file("/info.txt", b"12345").unwrap();
    let meta = fs.stat("/info.txt").unwrap();
    assert_eq!(meta.file_type, FileType::File);
    assert_eq!(meta.size, 5);

    fs.mkdir("/subdir").unwrap();
    let meta = fs.stat("/subdir").unwrap();
    assert_eq!(meta.file_type, FileType::Dir);

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_exists() {
    let fs = format_and_mount().await;
    assert!(!fs.exists("/nope"));
    fs.write_file("/yes.txt", b"y").unwrap();
    assert!(fs.exists("/yes.txt"));
    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_read_dir_iterator() {
    let fs = format_and_mount().await;

    fs.write_file("/a.txt", b"a").await.unwrap();
    fs.write_file("/b.txt", b"b").await.unwrap();
    fs.mkdir("/c").await.unwrap();

    let names: Vec<String> = {
        let dir = fs.read_dir("/").unwrap();
        let mut names = Vec::new();
        for entry in dir {
            names.push(entry.unwrap().name);
        }
        names
    };

    assert!(names.contains(&"a.txt".into()));
    assert!(names.contains(&"b.txt".into()));
    assert!(names.contains(&"c".into()));
    assert!(!names.contains(&".".into()));
    assert!(!names.contains(&"..".into()));

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_read_dir_interleaved_with_file_ops() {
    let fs = format_and_mount().await;

    fs.write_file("/x.txt", b"data-x").await.unwrap();
    fs.write_file("/y.txt", b"data-y").await.unwrap();

    {
        let dir = fs.read_dir("/").await.unwrap();
        for entry in dir {
            let entry = entry.unwrap();
            if entry.file_type == FileType::File {
                let data = fs.read_to_vec(&format!("/{}", entry.name)).await.unwrap();
                assert!(!data.is_empty());
            }
        }
    }

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_read_to_vec() {
    let fs = format_and_mount().await;

    fs.write_file("/hello.txt", b"Hello!").await.unwrap();
    let data = fs.read_to_vec("/hello.txt").await.unwrap();
    assert_eq!(data, b"Hello!");

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_read_to_vec_empty() {
    let fs = format_and_mount().await;

    fs.write_file("/empty.txt", b"").await.unwrap();
    let data = fs.read_to_vec("/empty.txt").await.unwrap();
    assert!(data.is_empty());

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_write_file_overwrites() {
    let fs = format_and_mount().await;

    fs.write_file("/f.txt", b"first").await.unwrap();
    fs.write_file("/f.txt", b"second").await.unwrap();
    let data = fs.read_to_vec("/f.txt").await.unwrap();
    assert_eq!(data, b"second");

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_fs_size() {
    let fs = format_and_mount().await;
    let s1 = fs.fs_size().await.unwrap();
    assert!(s1 > 0);

    fs.write_file("/big.bin", &vec![0xAB; 4096]).await.unwrap();
    let s2 = fs.fs_size().await.unwrap();
    assert!(s2 > s1);

    fs.unmount().unwrap();
}

#[tokio::test]
async fn test_nested_dirs() {
    let fs = format_and_mount().await;

    fs.mkdir("/a").await.unwrap();
    fs.mkdir("/a/b").await.unwrap();
    fs.write_file("/a/b/file.txt", b"nested").await.unwrap();

    let entries = fs.list_dir("/a/b").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "file.txt");

    let data = fs.read_to_vec("/a/b/file.txt").await.unwrap();
    assert_eq!(data, b"nested");

    fs.unmount().unwrap();
}
