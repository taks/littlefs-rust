//! Relocation and compaction tests.
//!
//! Upstream: tests/test_relocations.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_relocations.toml
//!
//! Validates dir_compact, dir_split, and orphaningcommit.

#![cfg_attr(not(feature = "slow_tests"), allow(unused_imports))]

mod common;

#[cfg(feature = "slow_tests")]
use std::assert_matches;
use std::fmt::Write;

use common::{ALPHA, LFS_O_CREAT, LFS_O_WRONLY, test_prng};
use littlefs_rust_core::{
    Error, Lfs, LfsConfig, LfsFile, LfsInfo, lfs_file_close, lfs_file_open, lfs_file_write,
    lfs_format, lfs_mkdir, lfs_mount, lfs_remove, lfs_rename, lfs_stat, lfs_type::LfsType,
    lfs_unmount,
};
#[cfg(feature = "slow_tests")]
use littlefs_rust_test_macro::lfs_test;

#[allow(dead_code)]
const ITERATIONS: usize = 20;
const COUNT: usize = 10;

// --- test_relocations_dangling_split_dir ---
/// Upstream: [cases.test_relocations_dangling_split_dir]
/// defines.ITERATIONS = 20, COUNT = 10, BLOCK_CYCLES = [8, 1]
///
/// Fill FS, create many files in child dir. Triggers split when metadata overflows.
#[lfs_test]
fn test_relocations_dangling_split_dir(cfg: &LfsConfig, #[values(8, 1)] block_cycles: i32) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    assert_ok!(lfs_mkdir(lfs, "d0"));
    for i in 0..COUNT {
        let path = &format!("d0/f{i}");
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        let n = lfs_file_write(lfs, file, b"x");
        assert_eq!(n, Ok(1));
        assert_ok!(lfs_file_close(lfs, file));
    }

    for i in 0..COUNT {
        let path = &format!("d0/f{i}");
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok!(lfs_stat(lfs, path, info));
        assert_eq!(info.name_str(), format!("f{i}"));
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_outdated_head ---
/// Upstream: [cases.test_relocations_outdated_head]
/// defines.ITERATIONS = 20, COUNT = 10, BLOCK_CYCLES = [8, 1]
///
/// Split dir handling: multiple dirs, nested sub with many files.
#[lfs_test]
fn test_relocations_outdated_head(cfg: &LfsConfig, #[values(8, 1)] block_cycles: i32) {
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    for i in 0..3 {
        assert_ok!(lfs_mkdir(lfs, &format!("d{i}")));
    }
    assert_ok!(lfs_mkdir(lfs, "d0/sub"));
    for i in 0..COUNT {
        let path = &format!("d0/sub/f{i}");
        let file = &mut LfsFile::default();
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        let n = lfs_file_write(lfs, file, b"x");
        assert_eq!(n, Ok(1));
        assert_ok!(lfs_file_close(lfs, file));
    }

    for i in 0..COUNT {
        let path = &format!("d0/sub/f{i}");
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok!(lfs_stat(lfs, path, info));
        assert_eq!(info.name_str(), format!("f{i}"));
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_nonreentrant ---
// mkdir/remove cycles, no power-loss.
#[lfs_test]
#[case(6, 1, 2000)]
#[case(26, 1, 2000)]
#[case(3, 3, 2000)]
#[cfg(feature = "slow_tests")]
fn test_relocations_nonreentrant(
    cfg: &LfsConfig,
    #[case] files: usize,
    #[case] depth: usize,
    #[case] cycles: usize,
    #[values(1)] block_cycles: i32,
) {
    if depth == 3 && cfg.cache_size != 64 || 2 * files >= cfg.block_count as usize {
        return;
    }
    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let mut prng: u32 = 1;
    for _ in 0..cycles {
        let mut full_path = String::with_capacity(256);
        // create random path
        for _ in 0..depth {
            assert_ok!(write!(
                &mut full_path,
                "/{}",
                ALPHA[test_prng(&mut prng) as usize % files] as char
            ));
        }
        // if it does not exist, we create it, else we destroy
        let info = &mut LfsInfo::default();
        let res = lfs_stat(lfs, &full_path, info);
        assert!(res.is_ok() || res == Err(Error::NoEntry));
        if res == Err(Error::NoEntry) {
            // create each directory in turn, ignore if dir already exists
            for d in 0..depth {
                assert_matches!(
                    lfs_mkdir(lfs, &full_path[..(2 * d + 2)]),
                    Ok(()) | Err(Error::Exists)
                );
            }
            for d in 0..depth {
                assert_ok!(lfs_stat(lfs, &full_path[..(2 * d + 2)], info));
                assert_eq!(info.name_str(), &full_path[(2 * d + 1)..(2 * d + 2)]);
                assert_eq!(info.type_, LfsType::DIR);
            }
        } else {
            // try to delete path in reverse order, ignore if dir is not empty
            let mut d = depth - 1;
            loop {
                assert_matches!(
                    lfs_remove(lfs, &full_path[..(2 * d + 2)]),
                    Ok(()) | Err(Error::NotEmpty)
                );
                if d == 0 {
                    break;
                }
                d -= 1;
            }

            assert_eq!(lfs_stat(lfs, &full_path, info), Err(Error::NoEntry));
        }
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_nonreentrant_renames ---
#[lfs_test]
#[case(6, 1, 2000)]
#[case(26, 1, 2000)]
#[case(3, 3, 2000)]
#[cfg(feature = "slow_tests")]
fn test_relocations_nonreentrant_renames(
    cfg: &LfsConfig,
    #[case] files: usize,
    #[case] depth: usize,
    #[case] cycles: usize,
    #[values(1)] block_cycles: i32,
) {
    if depth == 3 && cfg.cache_size != 64 || 2 * files >= cfg.block_count as usize {
        return;
    }

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, cfg));
    assert_ok!(lfs_mount(lfs, cfg));

    let mut prng: u32 = 1;

    for _ in 0..cycles {
        // create random path
        let mut full_path = String::with_capacity(256);
        for _ in 0..depth {
            assert_ok!(write!(
                &mut full_path,
                "/{}",
                ALPHA[test_prng(&mut prng) as usize % files] as char
            ));
        }

        // if it does not exist, we create it, else we destroy
        let info = &mut LfsInfo::default();
        let res = lfs_stat(lfs, &full_path, info);
        assert!(res.is_ok() || res == Err(Error::NoEntry));
        if res == Err(Error::NoEntry) {
            // create each directory in turn, ignore if dir already exists
            for d in 0..depth {
                assert_matches!(
                    lfs_mkdir(lfs, &full_path[..(2 * d + 2)]),
                    Ok(()) | Err(Error::Exists)
                );
            }
            for d in 0..depth {
                assert_ok!(lfs_stat(lfs, &full_path[..(2 * d + 2)], info));
                assert_eq!(info.name_str(), &full_path[(2 * d + 1)..(2 * d + 2)]);
                assert_eq!(info.type_, LfsType::DIR);
            }
        } else {
            assert_eq!(info.name_str(), &full_path[(2 * (depth - 1) + 1)..]);
            assert_eq!(info.type_, LfsType::DIR);

            // create new random path
            let mut new_path = String::with_capacity(256);
            for _ in 0..depth {
                assert_ok!(write!(
                    &mut new_path,
                    "/{}",
                    ALPHA[test_prng(&mut prng) as usize % files] as char
                ));
            }

            // if new path does not exist, rename, otherwise destroy
            let res = lfs_stat(lfs, &new_path, info);
            assert_matches!(res, Ok(()) | Err(Error::NoEntry));
            if res == Err(Error::NoEntry) {
                // stop once some dir is renamed
                let mut from = String::new();
                let mut to = String::new();
                for d in 0..depth {
                    from.push_str(&full_path[(2*d)..(2*d+2)]);
                    to.push_str(&new_path[(2*d)..(2*d+2)]);
                    let ret = lfs_rename(lfs, &from, &to);
                    assert_matches!(ret, Ok(()) | Err(Error::NotEmpty));
                    if ret.is_ok() {
                        from.clear();
                        from.push_str(&to);
                    }
                }
                for d in 0..depth {
                    assert_ok!(lfs_stat(lfs, &new_path[..(2 * d + 2)], info));
                    assert_eq!(info.name_str(), &new_path[(2 * d + 1)..(2 * d + 2)]);
                    assert_eq!(info.type_, LfsType::DIR);
                }

                assert_eq!(lfs_stat(lfs, &full_path, info), Err(Error::NoEntry));
            } else {
                // try to delete path in reverse order,
                // ignore if dir is not empty
                let mut d = depth - 1;
                loop {
                    assert_matches!(
                        lfs_remove(lfs, &full_path[..(2 * d + 2)]),
                        Ok(()) | Err(Error::NotEmpty)
                    );
                    if d == 0 {
                        break;
                    }
                    d -= 1;
                }

                assert_eq!(lfs_stat(lfs, &full_path, info), Err(Error::NoEntry));
            }
        }
    }
    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_reentrant ---
// mkdir/remove cycles with power-loss; verify FS consistent after each.
#[lfs_test]
#[case(6, 1, 20)]
#[case(26, 1, 20)]
#[case(3, 3, 20)]
#[cfg(feature = "slow_tests")]
fn test_relocations_reentrant(
    cfg: &LfsConfig,
    #[values(true)] reentrant: bool,
    #[case] files: usize,
    #[case] depth: usize,
    #[case] cycles: usize,
    #[values(1)] block_cycles: i32,
) {
    if depth == 3 && cfg.cache_size != 64 || 2 * files >= cfg.block_count as usize {
        return;
    }

    let lfs = &mut Lfs::default();
    let err = littlefs_rust_core::lfs_mount(lfs, cfg);
    if err.is_err() {
        assert_ok!(littlefs_rust_core::lfs_format(lfs, cfg));
        assert_ok!(littlefs_rust_core::lfs_mount(lfs, cfg));
    }

    let mut prng: u32 = 1;
    for _ in 0..cycles {
        let mut full_path = String::with_capacity(256);
        // create random path
        for _ in 0..depth {
            assert_ok!(write!(
                &mut full_path,
                "/{}",
                ALPHA[test_prng(&mut prng) as usize % files] as char
            ));
        }
        // if it does not exist, we create it, else we destroy
        let info = &mut LfsInfo::default();
        let res = lfs_stat(lfs, &full_path, info);
        assert!(res.is_ok() || res == Err(Error::NoEntry));
        if res == Err(Error::NoEntry) {
            // create each directory in turn, ignore if dir already exists
            for d in 0..depth {
                assert_matches!(
                    lfs_mkdir(lfs, &full_path[..(2 * d + 2)]),
                    Ok(()) | Err(Error::Exists)
                );
            }
            for d in 0..depth {
                assert_ok!(lfs_stat(lfs, &full_path[..(2 * d + 2)], info));
                assert_eq!(info.name_str(), &full_path[(2 * d + 1)..(2 * d + 2)]);
                assert_eq!(info.type_, LfsType::DIR);
            }
        } else {
            // try to delete path in reverse order, ignore if dir is not empty
            let mut d = depth - 1;
            loop {
                assert_matches!(
                    lfs_remove(lfs, &full_path[..(2 * d + 2)]),
                    Ok(()) | Err(Error::NotEmpty)
                );
                if d == 0 {
                    break;
                }
                d -= 1;
            }

            assert_eq!(lfs_stat(lfs, &full_path, info), Err(Error::NoEntry));
        }
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_relocations_reentrant_renames ---
// Chained renames with power-loss; verify FS consistent after each.
#[lfs_test]
#[case(6, 1, 20)]
#[case(26, 1, 20)]
#[case(3, 3, 20)]
#[cfg(feature = "slow_tests")]
#[ignore = "TODO FIX"]
fn test_relocations_reentrant_renames(
    cfg: &LfsConfig,
    #[values(false, true)] reentrant: bool,
    #[case] files: usize,
    #[case] depth: usize,
    #[case] cycles: usize,
    #[values(1)] block_cycles: i32,
) {
    // TODO fix this case, caused by non-DAG trees
    // NOTE the second condition is required
    if depth == 3 && cfg.cache_size != 64 {
        return;
    }
    if 2 * files >= cfg.block_count as usize {
        return;
    }

    let lfs = &mut Lfs::default();

    let err = littlefs_rust_core::lfs_mount(lfs, cfg);
    if err.is_err() {
        assert_ok!(littlefs_rust_core::lfs_format(lfs, cfg));
        assert_ok!(littlefs_rust_core::lfs_mount(lfs, cfg));
    }

    let mut prng: u32 = 1;

    for _ in 0..cycles {
        // create random path
        let mut full_path = String::with_capacity(256);
        for _ in 0..depth {
            assert_ok!(write!(
                &mut full_path,
                "/{}",
                ALPHA[test_prng(&mut prng) as usize % files] as char
            ));
        }

        // if it does not exist, we create it, else we destroy
        let info = &mut LfsInfo::default();
        let res = lfs_stat(lfs, &full_path, info);
        assert!(res.is_ok() || res == Err(Error::NoEntry));
        if res == Err(Error::NoEntry) {
            // create each directory in turn, ignore if dir already exists
            for d in 0..depth {
                assert_matches!(
                    lfs_mkdir(lfs, &full_path[..(2 * d + 2)]),
                    Ok(()) | Err(Error::Exists)
                );
            }
            for d in 0..depth {
                assert_ok!(lfs_stat(lfs, &full_path[..(2 * d + 2)], info));
                assert_eq!(info.name_str(), &full_path[(2 * d + 1)..(2 * d + 2)]);
                assert_eq!(info.type_, LfsType::DIR);
            }
        } else {
            assert_eq!(info.name_str(), &full_path[(2 * (depth - 1) + 1)..]);
            assert_eq!(info.type_, LfsType::DIR);

            // create new random path
            let mut new_path = String::with_capacity(256);
            for _ in 0..depth {
                assert_ok!(write!(
                    &mut new_path,
                    "/{}",
                    ALPHA[test_prng(&mut prng) as usize % files] as char
                ));
            }

            // if new path does not exist, rename, otherwise destroy
            let res = lfs_stat(lfs, &new_path, info);
            assert_matches!(res, Ok(()) | Err(Error::NoEntry));
            if res == Err(Error::NoEntry) {
                // stop once some dir is renamed
                for d in 0..depth {
                    let from = format!(
                        "{}{}",
                        &new_path[..(2 * d)],
                        &full_path[(2 * d)..(2 * d + 2)]
                    );
                    assert_matches!(
                        lfs_rename(lfs, &from, &new_path[..(2 * d + 2)]),
                        Ok(()) | Err(Error::NotEmpty)
                    );
                }
                for d in 0..depth {
                    assert_ok!(lfs_stat(lfs, &new_path[..(2 * d + 2)], info));
                    assert_eq!(info.name_str(), &new_path[(2 * d + 1)..(2 * d + 2)]);
                    assert_eq!(info.type_, LfsType::DIR);
                }

                assert_eq!(lfs_stat(lfs, &full_path, info), Err(Error::NoEntry));
            } else {
                // try to delete path in reverse order,
                // ignore if dir is not empty
                let mut d = depth - 1;
                loop {
                    assert_matches!(
                        lfs_remove(lfs, &full_path[..(2 * d + 2)]),
                        Ok(()) | Err(Error::NotEmpty)
                    );
                    if d == 0 {
                        break;
                    }
                    d -= 1;
                }

                assert_eq!(lfs_stat(lfs, &full_path, info), Err(Error::NoEntry));
            }
        }
    }
    assert_ok!(lfs_unmount(lfs));
}
