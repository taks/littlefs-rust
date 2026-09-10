//! Upstream: tests/test_badblocks.toml
//!
//! Bad-block handling: single, region, alternating corruption, and superblock corruption.
//! All cases require block_cycles == -1 (no FS-level wear leveling).

mod common;

use common::{BadblockBehavior, LFS_O_CREAT, LFS_O_RDONLY, LFS_O_WRONLY};
use littlefs_rust_core::{
    Error, Lfs, LfsConfig, LfsFile, LfsInfo, lfs_file_close, lfs_file_open, lfs_file_read,
    lfs_file_write, lfs_format, lfs_mkdir, lfs_mount, lfs_stat, lfs_unmount,
};
use littlefs_rust_test_macro::lfs_test;

use crate::common::lfs_emubd_setwear;

const LFS_TYPE_DIR: u8 = 0x02;
const NAMEMULT: usize = 64;
const FILEMULT: usize = 1;

/// Upstream: [cases.test_badblocks_single]
/// defines.ERASE_COUNT = 256
/// defines.ERASE_CYCLES = 0xffffffff
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
/// defines.NAMEMULT = 64
/// defines.FILEMULT = 1
///
/// For each block b in 2..BLOCK_COUNT: mark block b as worn (0xffffffff) and
/// block b-1 as fresh (0). Format, mount, create 9 dirs with files, unmount,
/// remount, stat/read all dirs and files.
#[lfs_test]
fn test_badblocks_single(
    cfg: &LfsConfig,
    #[values(Some(0x00), Some(0xff), None)] erase_value: Option<u8>,
    #[values(
        BadblockBehavior::ProgError,
        BadblockBehavior::EraseError,
        BadblockBehavior::ReadError,
        BadblockBehavior::ProgNoop,
        BadblockBehavior::EraseNoop
    )]
    badblock_behavior: BadblockBehavior,
) {
    let block_count: u32 = 256;

    for badblock in 2..block_count {
        lfs_emubd_setwear(cfg, badblock - 1, 0);
        lfs_emubd_setwear(cfg, badblock, 0xffffffff);

        let lfs = &mut Lfs::default();
        assert_ok!(lfs_format(lfs, cfg));
        assert_ok!(lfs_mount(lfs, cfg));

        for i in 1..10 {
            let mut buffer = [0u8; 1024];
            for b in buffer.iter_mut().take(NAMEMULT) {
                *b = b'0' + i as u8;
            }
            buffer[NAMEMULT] = 0;

            // mkdir
            assert_ok!(lfs_mkdir(lfs, unsafe {
                str::from_utf8_unchecked(&buffer[..NAMEMULT])
            }));

            // Build file path: "dirname/dirname"
            buffer[NAMEMULT] = b'/';
            for j in 0..NAMEMULT {
                buffer[j + NAMEMULT + 1] = b'0' + i as u8;
            }
            buffer[2 * NAMEMULT + 1] = 0;

            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(
                lfs,
                file,
                unsafe { str::from_utf8_unchecked(&buffer[..(2 * NAMEMULT + 1)]) },
                LFS_O_WRONLY | LFS_O_CREAT,
            ));

            let size = NAMEMULT as u32;
            for _j in 0..(i * FILEMULT) {
                let n = lfs_file_write(lfs, file, &buffer[..size as usize]);
                assert_eq!(n, Ok(size));
            }

            assert_ok!(lfs_file_close(lfs, file));
        }
        assert_ok!(lfs_unmount(lfs));

        // Remount and verify
        assert_ok!(lfs_mount(lfs, cfg));

        for i in 1..10 {
            let mut buffer = [0u8; 1024];
            for b in buffer.iter_mut().take(NAMEMULT) {
                *b = b'0' + i as u8;
            }

            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            assert_ok!(lfs_stat(
                lfs,
                unsafe { str::from_utf8_unchecked(&buffer[..NAMEMULT]) },
                info,
            ));
            assert_eq!(info.type_, LFS_TYPE_DIR);

            buffer[NAMEMULT] = b'/';
            for j in 0..NAMEMULT {
                buffer[j + NAMEMULT + 1] = b'0' + i as u8;
            }
            buffer[2 * NAMEMULT + 1] = 0;

            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(
                lfs,
                file,
                unsafe { str::from_utf8_unchecked(&buffer[..(2 * NAMEMULT + 1)]) },
                LFS_O_RDONLY,
            ));

            let size = NAMEMULT as u32;
            for _j in 0..(i * FILEMULT) {
                let mut rbuffer = [0u8; 1024];
                let n = lfs_file_read(lfs, file, &mut rbuffer[..size as usize]);
                assert_eq!(n, Ok(size));
                assert_eq!(&rbuffer[..size as usize], &buffer[..size as usize]);
            }

            assert_ok!(lfs_file_close(lfs, file));
        }
        assert_ok!(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_badblocks_region_corruption]
/// defines.ERASE_COUNT = 256
/// defines.ERASE_CYCLES = 0xffffffff
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
/// defines.NAMEMULT = 64
/// defines.FILEMULT = 1
///
/// Mark first half of blocks (starting at 2) as worn. Format, create
/// 9 dirs+files, unmount, remount, verify.
#[lfs_test]
fn test_badblocks_region_corruption(
    cfg: &LfsConfig,
    #[values(Some(0x00), Some(0xff), None)] erase_value: Option<u8>,
    #[values(256)] erase_count: u32,
    #[values(0xffffffff)] erase_cycles: u32,
    #[values(
        BadblockBehavior::ProgError,
        BadblockBehavior::EraseError,
        BadblockBehavior::ReadError,
        BadblockBehavior::ProgNoop,
        BadblockBehavior::EraseNoop
    )]
    badblock_behavior: BadblockBehavior,
) {
    let block_count: u32 = 256;

    // C: for (lfs_block_t i = 0; i < (BLOCK_COUNT-2)/2; i++) {
    //        lfs_emubd_setwear(cfg, i+2, 0xffffffff)
    for i in 0..((block_count - 2) / 2) {
        lfs_emubd_setwear(cfg, i + 2, 0xffffffff);
    }

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));

    assert_ok!(lfs_mount(lfs, cfg));
    badblocks_create_dirs_and_files(lfs);
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, cfg));
    badblocks_verify_dirs_and_files(lfs);
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_badblocks_alternating_corruption]
/// defines.ERASE_COUNT = 256
/// defines.ERASE_CYCLES = 0xffffffff
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
/// defines.NAMEMULT = 64
/// defines.FILEMULT = 1
///
/// Mark every other block starting at 2 as worn. Format, create
/// 9 dirs+files, unmount, remount, verify.
#[lfs_test]
fn test_badblocks_alternating_corruption(
    cfg: &LfsConfig,
    #[values(Some(0x00), Some(0xff), None)] erase_value: Option<u8>,
    #[values(
        BadblockBehavior::ProgError,
        BadblockBehavior::EraseError,
        BadblockBehavior::ReadError,
        BadblockBehavior::ProgNoop,
        BadblockBehavior::EraseNoop
    )]
    badblock_behavior: BadblockBehavior,
) {
    let block_count: u32 = 256;

    // C: for (lfs_block_t i = 0; i < (BLOCK_COUNT-2)/2; i++) {
    //        lfs_emubd_setwear(cfg, (2*i) + 2, 0xffffffff)
    for i in 0..((block_count - 2) / 2) {
        lfs_emubd_setwear(cfg, (2 * i) + 2, 0xffffffff)
    }

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));

    assert_ok!(lfs_mount(lfs, cfg));
    badblocks_create_dirs_and_files(lfs);
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, cfg));
    badblocks_verify_dirs_and_files(lfs);
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_badblocks_superblocks]
/// defines.ERASE_CYCLES = 0xffffffff
/// defines.ERASE_VALUE = [0x00, 0xff, -1]
/// defines.BADBLOCK_BEHAVIOR = [PROGERROR, ERASEERROR, READERROR, PROGNOOP, ERASENOOP]
///
/// Mark blocks 0 and 1 (superblocks) as worn.
/// Expect lfs_format to fail with LFS_ERR_NOSPC.
/// Expect lfs_mount to fail with LFS_ERR_CORRUPT.
#[lfs_test]
fn test_badblocks_superblocks(
    cfg: &LfsConfig,
    #[values(0xffffffff)] erase_cycles: u32,
    #[values(Some(0x00), Some(0xff), None)] erase_value: Option<u8>,
    #[values(
        BadblockBehavior::ProgError,
        BadblockBehavior::EraseError,
        BadblockBehavior::ReadError,
        BadblockBehavior::ProgNoop,
        BadblockBehavior::EraseNoop
    )]
    badblock_behavior: BadblockBehavior,
) {
    lfs_emubd_setwear(cfg, 0, 0xffffffff);
    lfs_emubd_setwear(cfg, 1, 0xffffffff);

    let lfs = &mut Lfs::default();
    assert_eq!(lfs_format(lfs, cfg), Err(Error::NoSpace));
    assert_eq!(lfs_mount(lfs, cfg), Err(Error::Corrupt));
}

// ── Helpers shared by region/alternating tests ──────────────────────────────

fn badblocks_create_dirs_and_files(lfs: &mut Lfs) {
    for i in 1..10 {
        let mut buffer = [0u8; 1024];
        for b in buffer.iter_mut().take(NAMEMULT) {
            *b = b'0' + i as u8;
        }
        buffer[NAMEMULT] = 0;

        assert_ok!(lfs_mkdir(lfs, unsafe {
            str::from_utf8_unchecked(&buffer[..NAMEMULT])
        }));

        buffer[NAMEMULT] = b'/';
        for j in 0..NAMEMULT {
            buffer[j + NAMEMULT + 1] = b'0' + i as u8;
        }
        buffer[2 * NAMEMULT + 1] = 0;

        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            unsafe { str::from_utf8_unchecked(&buffer[..(2 * NAMEMULT + 1)]) },
            LFS_O_WRONLY | LFS_O_CREAT,
        ));

        let size = NAMEMULT as u32;
        for _j in 0..(i * FILEMULT) {
            let n = lfs_file_write(lfs, file, &buffer[..size as usize]);
            assert_eq!(n, Ok(size));
        }

        assert_ok!(lfs_file_close(lfs, file));
    }
}

fn badblocks_verify_dirs_and_files(lfs: &mut Lfs) {
    for i in 1..10 {
        let mut buffer = [0u8; 1024];
        for b in buffer.iter_mut().take(NAMEMULT) {
            *b = b'0' + i as u8;
        }
        buffer[NAMEMULT] = 0;

        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok!(lfs_stat(
            lfs,
            unsafe { str::from_utf8_unchecked(&buffer[..(NAMEMULT)]) },
            info,
        ));
        assert_eq!(info.type_, LFS_TYPE_DIR);

        buffer[NAMEMULT] = b'/';
        for j in 0..NAMEMULT {
            buffer[j + NAMEMULT + 1] = b'0' + i as u8;
        }
        buffer[2 * NAMEMULT + 1] = 0;

        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(
            lfs,
            file,
            unsafe { str::from_utf8_unchecked(&buffer[..(2 * NAMEMULT + 1)]) },
            LFS_O_RDONLY,
        ));

        let size = NAMEMULT as u32;
        for _j in 0..(i * FILEMULT) {
            let mut rbuffer = [0u8; 1024];
            let n = lfs_file_read(lfs, file, &mut rbuffer[..size as usize]);
            assert_eq!(n, Ok(size));
            assert_eq!(&rbuffer[..size as usize], &buffer[..size as usize]);
        }

        assert_ok!(lfs_file_close(lfs, file));
    }
}
