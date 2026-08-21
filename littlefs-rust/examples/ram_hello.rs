//! Write and read a file on a RAM-backed littlefs filesystem.

use littlefs_rust::{Allocation, Config, FileAllocation, Filesystem, OpenFlags, RamStorage};

fn main() {
    // RamStorage is an in-memory block device — useful for tests and examples.
    // 128 blocks of 512 bytes each = 64 KB.
    const BLOCK_SIZE: usize = 512;
    const BLOCK_COUNT: usize = 128;
    let mut storage = RamStorage::<BLOCK_SIZE, BLOCK_COUNT>::new();

    let mut alloc = Allocation::new();

    // Format lays down the superblock; mount opens the filesystem for use.
    Filesystem::format(&mut storage, &mut alloc).expect("format failed");
    let fs = Filesystem::mount(&mut storage, &mut alloc).expect("mount failed");

    let mut falloc = FileAllocation::new();

    {
        let mut f = fs
            .open(
                &mut falloc,
                "/hello.txt",
                OpenFlags::WRITE | OpenFlags::CREATE,
            )
            .expect("file open failed");
        f.write(b"Hello, littlefs!").expect("write failed");
    }

    {
        let mut f = fs
            .open(&mut falloc, "/hello.txt", OpenFlags::READ)
            .expect("file open failed");
        let mut data = [0u8; 100];
        let n = f.read(&mut data).expect("read failed");
        println!("{}", core::str::from_utf8(&data[..n as usize]).unwrap());
    }
}
