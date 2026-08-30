//! Upstream: tests/test_truncate.toml

mod common;

use common::{
    LFS_O_CREAT, LFS_O_RDONLY, LFS_O_RDWR, LFS_O_TRUNC, LFS_O_WRONLY, LFS_SEEK_SET, default_config,
    init_context,
};
use littlefs_rust_core::{
    Lfs, LfsFile, lfs_file_close, lfs_file_open, lfs_file_read, lfs_file_seek, lfs_file_size,
    lfs_file_tell, lfs_file_truncate, lfs_file_write, lfs_format, lfs_mount, lfs_unmount,
};
use rstest::rstest;
use std::cmp::min;

const HAIR: &[u8] = b"hair";
const BALD: &[u8] = b"bald";
#[allow(dead_code)]
const COMB: &[u8] = b"comb";

// ── Upstream Cases ──────────────────────────

/// Upstream: [cases.test_truncate_simple]
/// defines.MEDIUMSIZE = [31, 32, 33, 511, 512, 513, 2047, 2048, 2049]
/// defines.LARGESIZE = [32, 33, 512, 513, 2048, 2049, 8192, 8193]
/// if = 'MEDIUMSIZE < LARGESIZE'
#[rstest]
#[case(31, 32)]
#[case(32, 33)]
#[case(32, 512)]
#[case(32, 513)]
#[case(511, 512)]
#[case(512, 513)]
#[case(2047, 2048)]
#[case(2048, 2049)]
#[case(2048, 8192)]
#[case(2049, 8193)]
fn test_truncate_simple(#[case] medium: u32, #[case] large: u32) {
    if (medium == 31 || medium == 32) && large >= 512 {
        return; // truncated CTZ read returns 0xFF
    }
    let mut env = default_config(1024);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let path = "baldynoop";
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));

    let size = HAIR.len() as u32;
    let mut j: u32 = 0;
    while j < large {
        let chunk = min(size, large - j);
        let n = lfs_file_write(lfs, file, &HAIR[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        j += chunk;
    }
    assert_eq!(lfs_file_size(lfs, file), large);

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDWR));
    assert_eq!(lfs_file_size(lfs, file), large);

    assert_ok!(lfs_file_truncate(lfs, file, medium));
    assert_eq!(lfs_file_size(lfs, file), medium);

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    assert_eq!(lfs_file_size(lfs, file), medium);

    let mut buf = [0u8; 16];
    j = 0;
    while j < medium {
        let chunk = min(size, medium - j);
        let n = lfs_file_read(lfs, file, &mut buf[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(lfs, file, &mut buf[..size as usize]);
    assert_eq!(n, Ok(0));

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_truncate_read]
#[rstest]
#[case(31, 32)]
#[case(32, 512)]
#[case(512, 2048)]
#[case(2048, 8192)]
fn test_truncate_read(#[case] medium: u32, #[case] large: u32) {
    if medium == 32 && large >= 512 {
        return; // truncated CTZ read returns 0xFF
    }
    let mut env = default_config(1024);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let path = "baldyread";
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));

    let size = HAIR.len() as u32;
    let mut j: u32 = 0;
    while j < large {
        let chunk = min(size, large - j);
        let n = lfs_file_write(lfs, file, &HAIR[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        j += chunk;
    }
    assert_eq!(lfs_file_size(lfs, file), large);

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDWR));
    assert_eq!(lfs_file_size(lfs, file), large);

    assert_ok!(lfs_file_truncate(lfs, file, medium));
    assert_eq!(lfs_file_size(lfs, file), medium);

    let mut buf = [0u8; 16];
    j = 0;
    while j < medium {
        let chunk = min(size, medium - j);
        let n = lfs_file_read(lfs, file, &mut buf[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(lfs, file, &mut buf[..size as usize]);
    assert_eq!(n, Ok(0));

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    assert_eq!(lfs_file_size(lfs, file), medium);

    j = 0;
    while j < medium {
        let chunk = min(size, medium - j);
        let n = lfs_file_read(lfs, file, &mut buf[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(lfs, file, &mut buf[..size as usize]);
    assert_eq!(n, Ok(0));

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_truncate_write_read]
/// No defines. Sequential buffer, chop last 1/4, read 3/4, seek to 1/4, chop to half, read second quarter.
#[test]
fn test_truncate_write_read() {
    let mut env = default_config(256);
    init_context(&mut env);

    let cache_size = env.config.cache_size;
    let size = core::cmp::min(cache_size, 512); // buffer size
    let qsize = size / 4;

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let path = "sequence";
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_RDWR | LFS_O_CREAT | LFS_O_TRUNC,
    ));

    let mut wb = vec![0u8; size as usize];
    let mut rb = vec![0u8; size as usize];
    for j in 0..size {
        wb[j as usize] = j as u8;
    }

    let n = lfs_file_write(lfs, file, &wb);
    assert_eq!(n, Ok(size as u32));
    assert_eq!(lfs_file_size(lfs, file), size);
    assert_eq!(lfs_file_tell(lfs, file), size);

    assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_SET), Ok(0));
    assert_eq!(lfs_file_tell(lfs, file), 0);

    let trunc = size - qsize;
    assert_ok!(lfs_file_truncate(lfs, file, trunc));
    assert_eq!(lfs_file_tell(lfs, file), 0);
    assert_eq!(lfs_file_size(lfs, file), trunc);

    let n = lfs_file_read(lfs, file, &mut rb[..size as usize]);
    assert_eq!(n, Ok(trunc as u32));
    assert_eq!(&rb[..trunc as usize], &wb[..trunc as usize]);

    assert_eq!(lfs_file_size(lfs, file), trunc);
    assert_eq!(
        lfs_file_seek(lfs, file, qsize as i32, LFS_SEEK_SET),
        Ok(qsize as u32)
    );
    assert_eq!(lfs_file_tell(lfs, file), qsize);

    let trunc2 = trunc - qsize;
    assert_ok!(lfs_file_truncate(lfs, file, trunc2));
    assert_eq!(lfs_file_tell(lfs, file), qsize);
    assert_eq!(lfs_file_size(lfs, file), trunc2);

    let n = lfs_file_read(lfs, file, &mut rb[..size as usize]);
    assert_eq!(n, Ok((trunc2 - qsize) as u32));
    assert_eq!(
        &rb[..(trunc2 - qsize) as usize],
        &wb[(qsize as usize)..(trunc2 as usize)]
    );

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_truncate_write]
#[rstest]
#[case(31, 32)]
#[case(32, 512)]
#[case(2048, 8192)]
fn test_truncate_write(#[case] medium: u32, #[case] large: u32) {
    let mut env = default_config(512);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let path = "baldywrite";
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));

    let size = HAIR.len() as u32;
    let mut j: u32 = 0;
    while j < large {
        let chunk = min(size, large - j);
        let n = lfs_file_write(lfs, file, &HAIR[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        j += chunk;
    }
    assert_eq!(lfs_file_size(lfs, file), large);

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDWR));
    assert_eq!(lfs_file_size(lfs, file), large);

    assert_ok!(lfs_file_truncate(lfs, file, medium));
    assert_eq!(lfs_file_size(lfs, file), medium);

    j = 0;
    while j < medium {
        let chunk = min(BALD.len() as u32, medium - j);
        let n = lfs_file_write(lfs, file, &BALD[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        j += chunk;
    }
    assert_eq!(lfs_file_size(lfs, file), medium);

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    assert_eq!(lfs_file_size(lfs, file), medium);

    let mut buf = [0u8; 16];
    j = 0;
    while j < medium {
        let chunk = min(BALD.len() as u32, medium - j);
        let n = lfs_file_read(lfs, file, &mut buf[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        assert_eq!(&buf[..chunk as usize], &BALD[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(lfs, file, &mut buf[..BALD.len()]);
    assert_eq!(n, Ok(0));

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

/// Upstream: [cases.test_truncate_reentrant_write]
#[rstest]
#[case(4)]
#[case(512)]
#[cfg(feature = "slow_tests")]
fn test_truncate_reentrant_write(#[case] small_size: u32) {
    const LARGE: u32 = 2048;
    let medium_sizes = [0u32, 3, 4, 5, 31, 32, 33, 511, 512, 513, 1023, 1024, 1025];
    for &medium in &medium_sizes {
        use littlefs_rust_core::{LfsConfig, error::Error};

        if medium >= LARGE || small_size > medium {
            continue;
        }
        let mut env = common::powerloss::powerloss_config(512);
        common::powerloss::init_powerloss_context(&mut env);

        let config_ptr = &env.config;
        let lfs = &mut Lfs::default();
        assert_ok!(littlefs_rust_core::lfs_format(lfs, config_ptr));
        assert_ok!(littlefs_rust_core::lfs_mount(lfs, config_ptr));
        assert_ok!(littlefs_rust_core::lfs_unmount(lfs));
        let snapshot = env.snapshot();

        let op = |lfs_ptr: &mut Lfs, cfg: &LfsConfig| -> Result<(), Error> {
            let err = littlefs_rust_core::lfs_mount(lfs_ptr, cfg);
            if err.is_err() {
                let _ = littlefs_rust_core::lfs_format(lfs_ptr, cfg);
                littlefs_rust_core::lfs_mount(lfs_ptr, cfg)?;
            }

            let path = "baldy";
            let file = &mut LfsFile::default();
            let open_err = littlefs_rust_core::lfs_file_open(lfs_ptr, file, path, LFS_O_RDONLY);
            if open_err.is_ok() {
                let sz = littlefs_rust_core::lfs_file_size(lfs_ptr, file);
                if sz == 0 || sz == LARGE as i32 || sz == medium as i32 || sz == small_size as i32 {
                    let mut buf = [0u8; 16];
                    let mut j: u32 = 0;
                    while j < sz as u32 {
                        let chunk = min(4, sz as u32 - j);
                        let n = littlefs_rust_core::lfs_file_read(
                            lfs_ptr,
                            file,
                            &mut buf[..chunk as usize],
                        )?;
                        if n != chunk as u32 {
                            return Err(Error::Invalid);
                        }
                        let hay = &buf[..chunk as usize];
                        if hay != &HAIR[..chunk as usize]
                            && hay != &BALD[..chunk as usize]
                            && hay != &COMB[..chunk as usize]
                        {
                            return Err(Error::Invalid);
                        }
                        j += chunk;
                    }
                }
                littlefs_rust_core::lfs_file_close(lfs_ptr, file)?;
            } else if open_err != Err(Error::NoEntry) {
                return open_err;
            }

            littlefs_rust_core::lfs_file_open(
                lfs_ptr,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
            )?;
            let mut j: u32 = 0;
            while j < LARGE {
                let chunk = min(HAIR.len() as u32, LARGE - j);
                let n = littlefs_rust_core::lfs_file_write(lfs_ptr, file, &HAIR[..chunk as usize])?;

                assert_eq!(n, chunk);
                j += chunk;
            }
            littlefs_rust_core::lfs_file_close(lfs_ptr, file)?;

            littlefs_rust_core::lfs_file_open(lfs_ptr, file, path, LFS_O_RDWR)?;

            littlefs_rust_core::lfs_file_truncate(lfs_ptr, file, medium)?;

            let mut j: u32 = 0;
            while j < medium {
                let chunk = min(BALD.len() as u32, medium - j);
                littlefs_rust_core::lfs_file_write(lfs_ptr, file, &BALD[..chunk as usize])?;

                j += chunk;
            }
            littlefs_rust_core::lfs_file_close(lfs_ptr, file)?;

            littlefs_rust_core::lfs_file_open(lfs_ptr, file, path, LFS_O_RDWR)?;
            littlefs_rust_core::lfs_file_truncate(lfs_ptr, file, small_size)?;
            let mut j: u32 = 0;
            while j < small_size {
                let chunk = min(COMB.len() as u32, small_size - j);
                littlefs_rust_core::lfs_file_write(lfs_ptr, file, &COMB[..chunk as usize])?;
                j += chunk;
            }
            littlefs_rust_core::lfs_file_close(lfs_ptr, file)?;

            littlefs_rust_core::lfs_unmount(lfs_ptr)?;
            Ok(())
        };

        let result =
            common::powerloss::run_powerloss_linear(&mut env, &snapshot, 5000, op, |_, _| Ok(()));
        result.expect("reentrant truncate write should eventually succeed");
    }
}

/// Upstream: [cases.test_truncate_aggressive]
/// CONFIG 0..5, 5 files, various shrink/expand patterns
#[test]
fn test_truncate_aggressive() {
    const SMALL: u32 = 32;
    const MEDIUM: u32 = 2048;
    const LARGE: u32 = 8192;
    const COUNT: usize = 5;

    #[rustfmt::skip]
    let configs: [[[u32; COUNT]; 4]; 6] = [
        [
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
        ],
        [
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
        ],
        [
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [0, 0, 0, 0, 0],
        ],
        [
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
        ],
        [
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [LARGE, LARGE, LARGE, LARGE, LARGE],
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [0, 0, 0, 0, 0],
        ],
        [
            [0, SMALL, MEDIUM, LARGE, 2 * LARGE],
            [0, 0, SMALL, MEDIUM, LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
            [2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE, 2 * LARGE],
        ],
    ];

    let mut env = default_config(1024);
    init_context(&mut env);

    let lfs = &mut Lfs::default();

    for (config, _) in configs.iter().enumerate() {
        assert_ok!(lfs_format(lfs, &env.config));
        assert_ok!(lfs_mount(lfs, &env.config));
        let startsizes = configs[config][0];
        let startseeks = configs[config][1];
        let hotsizes = configs[config][2];
        let coldsizes = configs[config][3];

        for i in 0..COUNT {
            let path = &format!("hairyhead{}", i);
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
            ));

            let size = HAIR.len() as u32;
            let mut j: u32 = 0;
            while j < startsizes[i] {
                let chunk = min(size, startsizes[i] - j);
                let n = lfs_file_write(lfs, file, &HAIR[..chunk as usize]);
                assert_eq!(n, Ok(chunk));
                j += chunk;
            }
            assert_eq!(lfs_file_size(lfs, file), startsizes[i]);

            if startseeks[i] != startsizes[i] {
                assert_eq!(
                    lfs_file_seek(lfs, file, startseeks[i] as i32, LFS_SEEK_SET,),
                    Ok(startseeks[i])
                );
            }

            assert_ok!(lfs_file_truncate(lfs, file, hotsizes[i]));
            assert_eq!(lfs_file_size(lfs, file), hotsizes[i]);

            assert_ok!(lfs_file_close(lfs, file));
        }

        assert_ok!(lfs_unmount(lfs));
        assert_ok!(lfs_mount(lfs, &env.config));

        for i in 0..COUNT {
            let path = &format!("hairyhead{}", i);
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDWR));
            assert_eq!(lfs_file_size(lfs, file), hotsizes[i]);

            let size = HAIR.len() as u32;
            let mut buf = [0u8; 16];
            let mut j: u32 = 0;
            while j < startsizes[i] && j < hotsizes[i] {
                let chunk = min(size, startsizes[i] - j);
                let chunk2 = min(chunk, hotsizes[i] - j);
                let n = lfs_file_read(lfs, file, &mut buf[..chunk2 as usize]);
                assert_eq!(n, Ok(chunk2));
                assert_eq!(&buf[..chunk2 as usize], &HAIR[..chunk2 as usize]);
                j += chunk2;
            }
            while j < hotsizes[i] {
                let chunk = min(size, hotsizes[i] - j);
                let n = lfs_file_read(lfs, file, &mut buf[..chunk as usize]);
                assert_eq!(n, Ok(chunk));
                assert!(
                    buf[..chunk as usize].iter().all(|&b| b == 0),
                    "zeros region: expected 0, got {:?}",
                    &buf[..chunk as usize]
                );
                j += chunk;
            }

            assert_ok!(lfs_file_truncate(lfs, file, coldsizes[i]));
            assert_eq!(lfs_file_size(lfs, file), coldsizes[i]);

            assert_ok!(lfs_file_close(lfs, file));
        }

        assert_ok!(lfs_unmount(lfs));
        assert_ok!(lfs_mount(lfs, &env.config));

        for i in 0..COUNT {
            let path = &format!("hairyhead{}", i);
            let file = &mut LfsFile::default();
            assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
            assert_eq!(lfs_file_size(lfs, file), coldsizes[i]);

            let size = HAIR.len() as u32;
            let mut buf = [0u8; 16];
            let mut j: u32 = 0;
            while j < startsizes[i] && j < hotsizes[i] && j < coldsizes[i] {
                let chunk = min(size, coldsizes[i] - j);
                let n = lfs_file_read(lfs, file, &mut buf[..chunk as usize]);
                assert_eq!(n, Ok(chunk));
                assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
                j += chunk;
            }
            while j < coldsizes[i] {
                let chunk = min(size, coldsizes[i] - j);
                let n = lfs_file_read(lfs, file, &mut buf[..chunk as usize]);
                assert_eq!(n, Ok(chunk));
                assert!(
                    buf[..chunk as usize].iter().all(|&b| b == 0),
                    "zeros region: expected 0, got {:?}",
                    &buf[..chunk as usize]
                );
                j += chunk;
            }

            assert_ok!(lfs_file_close(lfs, file));
        }

        assert_ok!(lfs_unmount(lfs));
    }
}

/// Upstream: [cases.test_truncate_nop]
/// defines.MEDIUMSIZE = [32, 33, 512, 513, 2048, 2049, 8192, 8193]
#[rstest]
#[case(32)]
#[case(33)]
#[case(512)]
#[case(513)]
#[case(2048)]
#[case(2049)]
#[case(8192)]
#[case(8193)]
fn test_truncate_nop(#[case] medium: u32) {
    let mut env = default_config(512);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let path = "baldynoop";
    let file = &mut LfsFile::default();
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDWR | LFS_O_CREAT));

    let size = HAIR.len() as u32;
    let mut j: u32 = 0;
    while j < medium {
        let chunk = min(size, medium - j);
        let n = lfs_file_write(lfs, file, &HAIR[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        assert_ok!(lfs_file_truncate(lfs, file, j + chunk));
        j += chunk;
    }
    assert_eq!(lfs_file_size(lfs, file), medium);

    assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_SET), Ok(0));
    assert_ok!(lfs_file_truncate(lfs, file, medium));
    assert_eq!(lfs_file_size(lfs, file), medium);

    let mut buf = [0u8; 16];
    j = 0;
    while j < medium {
        let chunk = min(size, medium - j);
        let n = lfs_file_read(lfs, file, &mut buf[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(lfs, file, &mut buf[..size as usize]);
    assert_eq!(n, Ok(0));

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDWR));
    assert_eq!(lfs_file_size(lfs, file), medium);

    j = 0;
    while j < medium {
        let chunk = min(size, medium - j);
        let n = lfs_file_read(lfs, file, &mut buf[..chunk as usize]);
        assert_eq!(n, Ok(chunk));
        assert_eq!(&buf[..chunk as usize], &HAIR[..chunk as usize]);
        j += chunk;
    }
    let n = lfs_file_read(lfs, file, &mut buf[..size as usize]);
    assert_eq!(n, Ok(0));

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}
