//! Power-loss simulation tests.
//!
//! Upstream: tests/test_powerloss.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_powerloss.toml

mod common;

use common::{
    LFS_O_APPEND, LFS_O_CREAT, LFS_O_RDONLY, LFS_O_WRONLY, erase_block_raw, read_block_raw,
    write_block_raw,
};
use littlefs_rust_core::{
    Lfs, LfsConfig, LfsDir, LfsFile, lfs_dir_close, lfs_dir_open, lfs_file_close, lfs_file_open,
    lfs_file_read, lfs_file_sync, lfs_file_write, lfs_format, lfs_mkdir, lfs_mount, lfs_unmount,
};
use littlefs_rust_test_macro::lfs_test;

// --- test_powerloss_only_rev ---
// Upstream: write rev+1 to one block of dir pair; mount picks higher rev, read/write still works.
#[lfs_test]
fn test_powerloss_only_rev(cfg: &LfsConfig) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let path_nb = "notebook";
    let path_paper = "notebook/paper";
    assert_ok!(lfs_mkdir(lfs, path_nb));

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        path_paper,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    let buf = b"hello";
    for _ in 0..5 {
        let n = lfs_file_write(lfs, file, buf);
        assert_eq!(n, Ok(buf.len() as u32));
        assert_ok!(lfs_file_sync(lfs, file));
    }
    assert_ok!(lfs_file_close(lfs, file));

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path_paper, LFS_O_RDONLY));
    let mut rbuf = [0u8; 256];
    for _ in 0..5 {
        let n = lfs_file_read(lfs, file, &mut rbuf[..5]);
        assert_eq!(n, Ok(5));
        assert_eq!(&rbuf[..5], b"hello");
    }
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    // Get dir pair and rev from a fresh mount, then corrupt rev
    assert_ok!(lfs_mount(lfs, cfg));
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok!(lfs_dir_open(lfs, dir, path_nb));
    let pair = dir.m.pair;
    let rev = dir.m.rev;
    assert_ok!(lfs_dir_close(lfs, dir));
    assert_ok!(lfs_unmount(lfs));

    // Partial write: rev+1 in block
    let block_size = cfg.block_size as usize;
    let mut block_buf = vec![0u8; block_size];
    let _ = unsafe {
        cfg.context
            .unwrap()
            .as_mut()
            .read(pair[1], 0, &mut block_buf)
    };

    block_buf[0..4].copy_from_slice(&(rev + 1).to_le_bytes());

    let _ = unsafe { cfg.context.unwrap().as_mut().erase(pair[1]) };
    let _ = unsafe { cfg.context.unwrap().as_mut().write(pair[1], 0, &block_buf) };

    assert_ok!(lfs_mount(lfs, cfg));

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path_paper, LFS_O_RDONLY));
    for _ in 0..5 {
        let n = lfs_file_read(lfs, file, &mut rbuf[..5]);
        assert_eq!(n, Ok(5));
        assert_eq!(&rbuf[..5], b"hello");
    }
    assert_ok!(lfs_file_close(lfs, file));

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        path_paper,
        LFS_O_WRONLY | LFS_O_APPEND
    ));
    let buf2 = b"goodbye";
    for _ in 0..5 {
        let n = lfs_file_write(lfs, file, buf2);
        assert_eq!(n, Ok(buf2.len() as u32));
        assert_ok!(lfs_file_sync(lfs, file));
    }
    assert_ok!(lfs_file_close(lfs, file));

    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path_paper, LFS_O_RDONLY));
    for _ in 0..5 {
        let n = lfs_file_read(lfs, file, &mut rbuf[..5]);
        assert_eq!(n, Ok(5));
        assert_eq!(&rbuf[..5], b"hello");
    }
    for _ in 0..5 {
        let n = lfs_file_read(lfs, file, &mut rbuf[..7]);
        assert_eq!(n, Ok(7));
        assert_eq!(&rbuf[..7], b"goodbye");
    }
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_powerloss_partial_prog]
/// defines.PROG_SIZE < BLOCK_SIZE, BYTE_OFF = [0, PROG_SIZE-1, PROG_SIZE/2], BYTE_VALUE = [0x33, 0xcc].
/// Corrupt one byte in a directory block at BYTE_OFF with BYTE_VALUE. Verify mount and read/write still work.
#[lfs_test]
fn test_powerloss_partial_prog(cfg: &LfsConfig) {
    if cfg.prog_size >= cfg.block_size {
        return;
    }

    let byte_offs: [u32; 3] = [0, cfg.prog_size - 1, cfg.prog_size / 2];
    let byte_values: [u8; 2] = [0x33, 0xcc];
    // const DIR_BLOCK: u32 = 1; // second superblock block has root dir data

    for &byte_off in &byte_offs {
        for &byte_value in &byte_values {
            let lfs = &mut Lfs::default();
            assert_ok!(lfs_format(lfs, cfg));
            assert_ok!(lfs_mount(lfs, cfg));
            assert_ok!(lfs_mkdir(lfs, "notebook"));
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(
                lfs,
                file,
                "notebook/paper",
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND
            ));
            for _ in 0..5 {
                assert_eq!(lfs_file_write(lfs, file, b"hello"), Ok(5));
                assert_ok!(lfs_file_sync(lfs, file));
            }
            assert_ok!(lfs_file_close(lfs, file));

            assert_ok!(lfs_file_open(lfs, file, "notebook/paper", LFS_O_RDONLY));
            for _ in 0..5 {
                let mut rbuffer = [0u8; 5];
                assert_eq!(lfs_file_read(lfs, file, &mut rbuffer), Ok(5));
                assert_eq!(&rbuffer, b"hello");
            }
            assert_ok!(lfs_file_close(lfs, file));
            assert_ok!(lfs_unmount(lfs));

            // imitate a partial prog, value should not matter, if littlefs
            // doesn't notice the partial prog testbd will assert

            // get offset to next prog
            assert_ok!(lfs_mount(lfs, cfg));
            let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
            assert_ok!(lfs_dir_open(lfs, dir, "notebook"));
            let block = dir.m.pair[0];
            let off = dir.m.off;
            assert_ok!(lfs_dir_close(lfs, dir));
            assert_ok!(lfs_unmount(lfs));

            // tweak byte
            let mut bbuffer = vec![0u8; cfg.block_size as usize];
            assert_ok!(read_block_raw(cfg, block, 0, &mut bbuffer));
            bbuffer[(off + byte_off) as usize] = byte_value;

            assert_ok!(erase_block_raw(cfg, block));
            assert_ok!(write_block_raw(cfg, block, 0, &bbuffer));

            assert_ok!(lfs_mount(lfs, cfg));

            // can read?
            assert_ok!(lfs_file_open(lfs, file, "notebook/paper", LFS_O_RDONLY));
            for _ in 0..5 {
                let mut rbuffer = [0u8; 5];
                assert_eq!(lfs_file_read(lfs, file, &mut rbuffer), Ok(5));
                assert_eq!(&rbuffer, b"hello");
            }
            assert_ok!(lfs_file_close(lfs, file));

            // can write?
            assert_ok!(lfs_file_open(
                lfs,
                file,
                "notebook/paper",
                LFS_O_WRONLY | LFS_O_APPEND
            ));
            for _ in 0..5 {
                assert_eq!(lfs_file_write(lfs, file, b"goodbye"), Ok(7));
                assert_ok!(lfs_file_sync(lfs, file));
            }
            assert_ok!(lfs_file_close(lfs, file));

            assert_ok!(lfs_file_open(lfs, file, "notebook/paper", LFS_O_RDONLY));
            for _ in 0..5 {
                let mut rbuffer = [0u8; 5];
                assert_eq!(lfs_file_read(lfs, file, &mut rbuffer), Ok(5));
                assert_eq!(&rbuffer, b"hello");
            }
            for _ in 0..5 {
                let mut rbuffer = [0u8; 7];
                assert_eq!(lfs_file_read(lfs, file, &mut rbuffer), Ok(7));
                assert_eq!(&rbuffer, b"goodbye");
            }
            assert_ok!(lfs_file_close(lfs, file));
            assert_ok!(lfs_unmount(lfs));
        }
    }
}
