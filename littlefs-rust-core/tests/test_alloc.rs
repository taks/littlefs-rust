//! Allocator and block allocation tests.
//!
//! Upstream: tests/test_alloc.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_alloc.toml

mod common;

use common::{
    LFS_O_APPEND, LFS_O_CREAT, LFS_O_RDONLY, LFS_O_TRUNC, LFS_O_WRONLY,
    clone_config_with_block_count, config_badblock, config_with_geometry, default_config,
    init_badblock_context, init_context, init_logger, run_with_timeout,
};
use littlefs_rust_core::{
    Lfs, LfsFile, LfsInfo, error::Error, lfs_file_close, lfs_file_open, lfs_file_read,
    lfs_file_size, lfs_file_sync, lfs_file_truncate, lfs_file_write, lfs_format, lfs_fs_gc,
    lfs_mkdir, lfs_mount, lfs_remove, lfs_stat, lfs_unmount,
};
use rstest::rstest;

const FILES: u32 = 3;
const NAMES: &[&[u8]] = &[b"bacon", b"eggs", b"pancakes"];

fn compact_thresh_u32(val: i32) -> u32 {
    if val == -1 { u32::MAX } else { val as u32 }
}

// --- test_alloc_parallel ---
/// Upstream: [cases.test_alloc_parallel]
/// defines.FILES = 3, SIZE = (((BLOCK_SIZE-8)*(BLOCK_COUNT-6))/FILES)
/// defines.GC = [false, true], COMPACT_THRESH = [-1, 0, BLOCK_SIZE/2], INFER_BC = [false, true]
///
/// Create breakfast dir, open 3 files in parallel, write SIZE bytes to each (optional GC),
/// close, unmount, remount, read and verify.
#[rstest]
fn test_alloc_parallel(
    #[values(false, true)] gc: bool,
    #[values(-1, 0, 256)] compact_thresh_val: i32,
    #[values(false, true)] infer_bc: bool,
) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let block_size = env.config.block_size;
    let block_count = env.config.block_count;
    let size: usize = ((block_size - 8) as usize * (block_count - 6) as usize) / FILES as usize;

    env.config.compact_thresh = compact_thresh_u32(compact_thresh_val);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));

    let mount_cfg = clone_config_with_block_count(&env, if infer_bc { 0 } else { block_count });
    assert_ok!(lfs_mount(lfs, &mount_cfg.config));
    assert_ok!(lfs_mkdir(lfs, "breakfast"));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &mount_cfg.config));
    let mut files: [LfsFile; 3] =
        core::array::from_fn(|_| unsafe { core::mem::MaybeUninit::zeroed().assume_init() });
    for n in 0..FILES {
        let path = &format!(
            "breakfast/{}",
            core::str::from_utf8(NAMES[n as usize]).unwrap()
        );
        assert_ok!(lfs_file_open(
            lfs,
            &mut files[n as usize],
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        ));
    }
    for n in 0..FILES {
        if gc {
            assert_ok!(lfs_fs_gc(lfs));
        }
        let name = NAMES[n as usize];
        for i in (0..size).step_by(name.len()) {
            let chunk = (size - i).min(name.len());
            let nw = lfs_file_write(lfs, &mut files[n as usize], &name[..chunk]);
            assert_eq!(nw, Ok(chunk as u32));
        }
    }
    for n in 0..FILES {
        assert_ok!(lfs_file_close(lfs, &mut files[n as usize]));
    }
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &mount_cfg.config));
    for n in 0..FILES {
        let path = &format!(
            "breakfast/{}",
            core::str::from_utf8(NAMES[n as usize]).unwrap()
        );
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let name = NAMES[n as usize];
        let mut buf = [0u8; 16];
        for i in (0..size).step_by(name.len()) {
            let chunk = (size - i).min(name.len());
            let nr = lfs_file_read(lfs, file, &mut buf[..chunk]);
            assert_eq!(nr, Ok(chunk as u32));
            assert_eq!(&buf[..chunk], &name[..chunk]);
        }
        assert_ok!(lfs_file_close(lfs, file));
    }
    assert_ok!(lfs_unmount(lfs));
}

// --- test_alloc_serial ---
/// Upstream: [cases.test_alloc_serial]
/// defines.FILES = 3, SIZE = (((BLOCK_SIZE-8)*(BLOCK_COUNT-6))/FILES)
/// defines.GC = [false, true], COMPACT_THRESH = [-1, 0, BLOCK_SIZE/2], INFER_BC = [false, true]
///
/// Create breakfast dir, then for each file: mount, open, write SIZE bytes (optional GC per write),
/// close, unmount. Remount and verify all files.
#[rstest]
fn test_alloc_serial(
    #[values(false, true)] gc: bool,
    #[values(-1, 0, 256)] compact_thresh_val: i32,
    #[values(false, true)] infer_bc: bool,
) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let block_size = env.config.block_size;
    let block_count = env.config.block_count;
    let size: usize = ((block_size - 8) as usize * (block_count - 6) as usize) / FILES as usize;

    env.config.compact_thresh = compact_thresh_u32(compact_thresh_val);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));

    let mount_cfg = clone_config_with_block_count(&env, if infer_bc { 0 } else { block_count });
    assert_ok!(lfs_mount(lfs, &mount_cfg.config));
    assert_ok!(lfs_mkdir(lfs, "breakfast"));
    assert_ok!(lfs_unmount(lfs));

    for n in 0..FILES {
        assert_ok!(lfs_mount(lfs, &mount_cfg.config));
        let path = &format!(
            "breakfast/{}",
            core::str::from_utf8(NAMES[n as usize]).unwrap()
        );
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok!(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        ));
        let name = NAMES[n as usize];
        let mut buf = [0u8; 16];
        buf[..name.len()].copy_from_slice(name);
        for i in (0..size).step_by(name.len()) {
            if gc {
                assert_ok!(lfs_fs_gc(lfs));
            }
            let chunk = (size - i).min(name.len());
            let nw = lfs_file_write(lfs, file, &buf[..chunk]);
            assert_eq!(nw, Ok(chunk as u32));
        }
        assert_ok!(lfs_file_close(lfs, file));
        assert_ok!(lfs_unmount(lfs));
    }

    assert_ok!(lfs_mount(lfs, &mount_cfg.config));
    for n in 0..FILES {
        let path = &format!(
            "breakfast/{}",
            core::str::from_utf8(NAMES[n as usize]).unwrap()
        );
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let name = NAMES[n as usize];
        let mut buf = [0u8; 16];
        for i in (0..size).step_by(name.len()) {
            let chunk = (size - i).min(name.len());
            let nr = lfs_file_read(lfs, file, &mut buf[..chunk]);
            assert_eq!(nr, Ok(chunk as u32));
            assert_eq!(&buf[..chunk], &name[..chunk]);
        }
        assert_ok!(lfs_file_close(lfs, file));
    }
    assert_ok!(lfs_unmount(lfs));
}

// --- test_alloc_parallel_reuse ---
/// Upstream: [cases.test_alloc_parallel_reuse]
/// defines.FILES = 3, SIZE = (((BLOCK_SIZE-8)*(BLOCK_COUNT-6))/FILES)
/// defines.CYCLES = [1, 10], INFER_BC = [false, true]
///
/// CYCLES iterations: create breakfast, write 3 files, read back, remove all.
#[rstest]
fn test_alloc_parallel_reuse(#[values(1, 10)] cycles: u32, #[values(false, true)] infer_bc: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let block_size = env.config.block_size;
    let block_count = env.config.block_count;
    let size: usize = ((block_size - 8) as usize * (block_count - 6) as usize) / FILES as usize;

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));

    let mount_cfg = clone_config_with_block_count(&env, if infer_bc { 0 } else { block_count });

    for _c in 0..cycles {
        assert_ok!(lfs_mount(lfs, &mount_cfg.config));
        assert_ok!(lfs_mkdir(lfs, "breakfast"));
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &mount_cfg.config));
        let mut files: [LfsFile; 3] =
            core::array::from_fn(|_| unsafe { core::mem::MaybeUninit::zeroed().assume_init() });
        for n in 0..FILES {
            let path = &format!(
                "breakfast/{}",
                core::str::from_utf8(NAMES[n as usize]).unwrap()
            );
            assert_ok!(lfs_file_open(
                lfs,
                &mut files[n as usize],
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
            ));
        }
        for n in 0..FILES {
            let name = NAMES[n as usize];
            for i in (0..size).step_by(name.len()) {
                let chunk = (size - i).min(name.len());
                let nw = lfs_file_write(lfs, &mut files[n as usize], &name[..chunk]);
                assert_eq!(nw, Ok(chunk as u32));
            }
        }
        for n in 0..FILES {
            assert_ok!(lfs_file_close(lfs, &mut files[n as usize]));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &mount_cfg.config));
        for n in 0..FILES {
            let path = &format!(
                "breakfast/{}",
                core::str::from_utf8(NAMES[n as usize]).unwrap()
            );
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
            let name = NAMES[n as usize];
            let mut buf = [0u8; 16];
            for i in (0..size).step_by(name.len()) {
                let chunk = (size - i).min(name.len());
                let nr = lfs_file_read(lfs, file, &mut buf[..chunk]);
                assert_eq!(nr, Ok(chunk as u32));
                assert_eq!(&buf[..chunk], &name[..chunk]);
            }
            assert_ok!(lfs_file_close(lfs, file));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &mount_cfg.config));
        for n in 0..FILES {
            let path = &format!(
                "breakfast/{}",
                core::str::from_utf8(NAMES[n as usize]).unwrap()
            );
            assert_ok!(lfs_remove(lfs, path));
        }
        assert_ok!(lfs_remove(lfs, "breakfast"));
        assert_ok!(lfs_unmount(lfs));
    }
}

// --- test_alloc_serial_reuse ---
/// Upstream: [cases.test_alloc_serial_reuse]
/// defines.FILES = 3, SIZE = (((BLOCK_SIZE-8)*(BLOCK_COUNT-6))/FILES)
/// defines.CYCLES = [1, 10], INFER_BC = [false, true]
///
/// CYCLES iterations: create breakfast, write each file serially, read back, remove all.
#[rstest]
fn test_alloc_serial_reuse(#[values(1, 10)] cycles: u32, #[values(false, true)] infer_bc: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let block_size = env.config.block_size;
    let block_count = env.config.block_count;
    let size: usize = ((block_size - 8) as usize * (block_count - 6) as usize) / FILES as usize;

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));

    let mount_cfg = clone_config_with_block_count(&env, if infer_bc { 0 } else { block_count });

    for _c in 0..cycles {
        assert_ok!(lfs_mount(lfs, &mount_cfg.config));
        assert_ok!(lfs_mkdir(lfs, "breakfast"));
        assert_ok!(lfs_unmount(lfs));

        for n in 0..FILES {
            assert_ok!(lfs_mount(lfs, &mount_cfg.config));
            let path = &format!(
                "breakfast/{}",
                core::str::from_utf8(NAMES[n as usize]).unwrap()
            );
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok!(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
            ));
            let name = NAMES[n as usize];
            let mut buf = [0u8; 16];
            buf[..name.len()].copy_from_slice(name);
            for i in (0..size).step_by(name.len()) {
                let chunk = (size - i).min(name.len());
                let nw = lfs_file_write(lfs, file, &buf[..chunk]);
                assert_eq!(nw, Ok(chunk as u32));
            }
            assert_ok!(lfs_file_close(lfs, file));
            assert_ok!(lfs_unmount(lfs));
        }

        assert_ok!(lfs_mount(lfs, &mount_cfg.config));
        for n in 0..FILES {
            let path = &format!(
                "breakfast/{}",
                core::str::from_utf8(NAMES[n as usize]).unwrap()
            );
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok!(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
            let name = NAMES[n as usize];
            let mut buf = [0u8; 16];
            for i in (0..size).step_by(name.len()) {
                let chunk = (size - i).min(name.len());
                let nr = lfs_file_read(lfs, file, &mut buf[..chunk]);
                assert_eq!(nr, Ok(chunk as u32));
                assert_eq!(&buf[..chunk], &name[..chunk]);
            }
            assert_ok!(lfs_file_close(lfs, file));
        }
        assert_ok!(lfs_unmount(lfs));

        assert_ok!(lfs_mount(lfs, &mount_cfg.config));
        for n in 0..FILES {
            let path = &format!(
                "breakfast/{}",
                core::str::from_utf8(NAMES[n as usize]).unwrap()
            );
            assert_ok!(lfs_remove(lfs, path));
        }
        assert_ok!(lfs_remove(lfs, "breakfast"));
        assert_ok!(lfs_unmount(lfs));
    }
}

// --- test_alloc_exhaustion ---
/// Upstream: [cases.test_alloc_exhaustion]
/// defines.INFER_BC = [false, true]
///
/// Create file "exhaustion", write "exhaustion" then "blahblahblahblah" until NOSPC, GC, close,
/// remount, read back and verify.
#[rstest]
fn test_alloc_exhaustion(#[values(false, true)] infer_bc: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));

    let mount_cfg =
        clone_config_with_block_count(&env, if infer_bc { 0 } else { env.config.block_count });
    assert_ok!(lfs_mount(lfs, &mount_cfg.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let exhaustion = b"exhaustion";
    let n = lfs_file_write(lfs, file, exhaustion);
    assert_eq!(n, Ok(exhaustion.len() as u32));
    assert_ok!(lfs_file_sync(lfs, file));

    let blah = b"blahblahblahblah";
    loop {
        let res = lfs_file_write(lfs, file, blah);
        if res.is_err() {
            use littlefs_rust_core::error::Error;

            assert_err!(Error::NoSpace, res);
            break;
        }
        assert_eq!(res, Ok(blah.len() as u32));
    }

    assert_ok!(lfs_fs_gc(lfs));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &mount_cfg.config));
    assert_ok!(lfs_file_open(lfs, file, "exhaustion", LFS_O_RDONLY));
    let fsize = lfs_file_size(lfs, file);
    assert!(fsize >= exhaustion.len() as i32);
    let mut buf = [0u8; 16];
    let n = lfs_file_read(lfs, file, &mut buf[..exhaustion.len()]);
    assert_eq!(n, Ok(exhaustion.len() as u32));
    assert_eq!(&buf[..exhaustion.len()], exhaustion);
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_alloc_split_dir ---
/// Upstream: [cases.test_alloc_split_dir]
/// if = 'ERASE_SIZE == 512', defines.ERASE_COUNT = 1024
///
/// Create dir with files, verify stat. (Geometry-specific; uses default_config.)
#[test]
fn test_alloc_split_dir() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_mkdir(lfs, "d"));
    for i in 0..8 {
        let path = &format!("d/f{i}");
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok!(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
        let n = lfs_file_write(lfs, file, b"x");
        assert_eq!(n, Ok(1));
        assert_ok!(lfs_file_close(lfs, file));
    }
    for i in 0..8 {
        let path = &format!("d/f{i}");
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok!(lfs_stat(lfs, path, info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(
            core::str::from_utf8(&info.name[..nul]).unwrap(),
            format!("f{i}")
        );
    }

    assert_ok!(lfs_unmount(lfs));
}

// --- test_alloc_exhaustion_wraparound ---
/// Upstream: [cases.test_alloc_exhaustion_wraparound]
/// defines.SIZE = (((BLOCK_SIZE-8)*(BLOCK_COUNT-4))/3), INFER_BC = [false, true]
///
/// Fill padding file, remove, create exhaustion file, write until NOSPC, GC, remount, verify.
#[rstest]
fn test_alloc_exhaustion_wraparound(#[values(false, true)] infer_bc: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let block_size = env.config.block_size;
    let block_count = env.config.block_count;
    let size: usize = ((block_size - 8) as usize * (block_count - 4) as usize) / 3;

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));

    let mount_cfg = clone_config_with_block_count(&env, if infer_bc { 0 } else { block_count });
    assert_ok!(lfs_mount(lfs, &mount_cfg.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "padding",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let buffering = b"buffering";
    for i in (0..size).step_by(buffering.len()) {
        let chunk = (size - i).min(buffering.len());
        let n = lfs_file_write(lfs, file, &buffering[..chunk]);
        assert_eq!(n, Ok(chunk as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_remove(lfs, "padding"));

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let exhaustion = b"exhaustion";
    let n = lfs_file_write(lfs, file, exhaustion);
    assert_eq!(n, Ok(exhaustion.len() as u32));
    assert_ok!(lfs_file_sync(lfs, file));

    let blah = b"blahblahblahblah";
    loop {
        let res = lfs_file_write(lfs, file, blah);
        if let Err(err) = res {
            assert_eq!(err, littlefs_rust_core::error::Error::NoSpace);
            break;
        }
        assert_eq!(res, Ok(blah.len() as u32));
    }

    assert_ok!(lfs_fs_gc(lfs));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &mount_cfg.config));
    assert_ok!(lfs_file_open(lfs, file, "exhaustion", LFS_O_RDONLY));
    let fsize = lfs_file_size(lfs, file);
    assert!(fsize >= exhaustion.len() as i32);
    let mut buf = [0u8; 16];
    let n = lfs_file_read(lfs, file, &mut buf[..exhaustion.len()]);
    assert_eq!(n, Ok(exhaustion.len() as u32));
    assert_eq!(&buf[..exhaustion.len()], exhaustion);
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_remove(lfs, "exhaustion"));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_alloc_dir_exhaustion ---
/// Upstream: [cases.test_alloc_dir_exhaustion]
/// defines.INFER_BC = [false, true]
///
/// Find max file size, verify mkdir fits with count writes, fails with count+1.
#[rstest]
fn test_alloc_dir_exhaustion(#[values(false, true)] infer_bc: bool) {
    use littlefs_rust_core::error::Error;

    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let block_count = env.config.block_count;
    let mount_cfg = clone_config_with_block_count(&env, if infer_bc { 0 } else { block_count });

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &mount_cfg.config));

    assert_ok!(lfs_mkdir(lfs, "exhaustiondir"));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    let blah = b"blahblahblahblah";
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    let mut count = 0i32;
    loop {
        let err = lfs_file_write(lfs, file, blah);
        if err.is_err() {
            assert_err!(Error::NoSpace, err);
            break;
        }
        assert_eq!(err, Ok(blah.len() as u32));
        count += 1;
    }

    assert_ok!(lfs_fs_gc(lfs));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_remove(lfs, "exhaustion"));
    assert_ok!(lfs_remove(lfs, "exhaustiondir"));

    // Recreate with count writes; mkdir should succeed
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    for _ in 0..count {
        let n = lfs_file_write(lfs, file, blah);
        assert_eq!(n, Ok(blah.len() as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_mkdir(lfs, "exhaustiondir"));
    assert_ok!(lfs_remove(lfs, "exhaustiondir"));
    assert_ok!(lfs_remove(lfs, "exhaustion"));

    // Recreate with count+1 writes; mkdir should fail NOSPC
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    for _ in 0..(count + 1) {
        let n = lfs_file_write(lfs, file, blah);
        assert_eq!(n, Ok(blah.len() as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));

    let err = lfs_mkdir(lfs, "exhaustiondir");
    assert_err!(Error::NoSpace, err);

    assert_ok!(lfs_remove(lfs, "exhaustion"));
    assert_ok!(lfs_unmount(lfs));
}

// --- Deferred ---

// --- test_alloc_two_files_ctz ---
// Reproduces dir corruption: pacman fill+shrink, ghost fill to NOSPC, GC, read pacman.
#[test]
fn test_alloc_two_files_ctz() {
    init_logger();
    let mut env = default_config(48);
    init_context(&mut env);
    let block_size = env.config.block_size;

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "pacman",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let waka = b"waka";
    let mut filesize: usize = 0;
    loop {
        let res = lfs_file_write(lfs, file, waka);
        if res == Err(Error::NoSpace) {
            break;
        }
        assert_eq!(res, Ok(waka.len() as u32));
        filesize += waka.len();
    }
    assert_ok!(lfs_file_close(lfs, file));

    filesize = filesize.saturating_sub(3 * block_size as usize);
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "pacman",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    for _ in (0..filesize).step_by(waka.len()) {
        let n = lfs_file_write(lfs, file, waka);
        assert_eq!(n, Ok(waka.len() as u32));
    }
    assert_ok!(lfs_file_sync(lfs, file));
    let pacman_head = file.ctz.head;
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "ghost",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let chomp = b"chomp";
    loop {
        let res = lfs_file_write(lfs, file, chomp);
        if res == Err(Error::NoSpace) {
            break;
        }
        assert_eq!(res, Ok(chomp.len() as u32));
    }
    assert_ok!(lfs_fs_gc(lfs));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(lfs, file, "pacman", LFS_O_RDONLY));
    let open_head = file.ctz.head;
    assert_eq!(
        open_head, pacman_head,
        "pacman ctz.head after ghost fill+GC: expected {} got {}",
        pacman_head, open_head
    );
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

// Max iterations for write-until-NOSPC/CORRUPT loops. With 48 blocks * 512 = 24KB,
// ~5000 writes suffice. 50_000 caps infinite loops and causes a fast failure.
const MAX_FILL_ITER: u32 = 50_000;

// --- test_alloc_bad_blocks ---
/// Upstream: [cases.test_alloc_bad_blocks]
/// defines.ERASE_CYCLES = 0xffffffff, defines.BADBLOCK_BEHAVIOR = LFS_EMUBD_BADBLOCK_READERROR
///
/// Fill pacman, shrink, mark block bad, ghost write until CORRUPT, clear bad, ghost to NOSPC, GC, verify pacman.
#[test]
fn test_alloc_bad_blocks() {
    init_logger();
    run_with_timeout(30, || {
        test_alloc_bad_blocks_body();
    });
}

fn test_alloc_bad_blocks_body() {
    // Match upstream C: block_count 128 (test_alloc.toml default)
    let mut env = config_badblock(128);
    init_badblock_context(&mut env);

    let block_size = env.config.block_size;

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "pacman",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    let waka = b"waka";
    let mut filesize: usize = 0;
    let mut iter: u32 = 0;
    loop {
        assert!(
            iter < MAX_FILL_ITER,
            "pacman fill exceeded {} iterations",
            MAX_FILL_ITER
        );
        iter += 1;
        let res = lfs_file_write(lfs, file, waka);
        if res == Err(Error::NoSpace) {
            break;
        }
        assert_eq!(res, Ok(waka.len() as u32));
        filesize += waka.len();
    }
    assert_ok!(lfs_file_close(lfs, file));

    filesize = filesize.saturating_sub(3 * block_size as usize);

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "pacman",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC,
    ));
    for _ in (0..filesize).step_by(waka.len()) {
        let n = lfs_file_write(lfs, file, waka);
        assert_eq!(n, Ok(waka.len() as u32));
    }

    assert_ok!(lfs_file_sync(lfs, file));
    let fileblock = { file.ctz.head };
    let block_count = env.config.block_count;
    assert!(
        fileblock < block_count,
        "fileblock {} must be < block_count {}",
        fileblock,
        block_count
    );
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    env.badblock_ram.set_bad_block(fileblock);

    // Open ghost, write until CORRUPT (alloc hits bad block), close.
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "ghost",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let chomp = b"chomp";
    let mut iter: u32 = 0;
    loop {
        assert!(
            iter < MAX_FILL_ITER,
            "ghost fill (until CORRUPT/NOSPC) exceeded {} iterations",
            MAX_FILL_ITER
        );
        iter += 1;
        let res = lfs_file_write(lfs, file, chomp);
        if res == Err(Error::Corrupt) || res == Err(Error::NoSpace) {
            break;
        }
        assert_eq!(res, Ok(chomp.len() as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));

    env.badblock_ram.clear_bad_block(fileblock);

    // Write ghost to NOSPC, then GC, close, unmount.
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "ghost",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let mut iter: u32 = 0;
    loop {
        assert!(
            iter < MAX_FILL_ITER,
            "ghost fill (until NOSPC) exceeded {} iterations",
            MAX_FILL_ITER
        );
        iter += 1;
        let res = lfs_file_write(lfs, file, chomp);
        if res == Err(Error::NoSpace) {
            break;
        }
        assert_eq!(res, Ok(chomp.len() as u32));
    }
    assert_ok!(lfs_fs_gc(lfs));
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));

    assert_ok!(lfs_mount(lfs, &env.config));
    assert_ok!(lfs_file_open(lfs, file, "pacman", LFS_O_RDONLY));
    let open_head = { file.ctz.head };
    assert!(
        open_head < env.config.block_count,
        "pacman ctz.head={} must be < block_count {} (dir corruption when ghost present)",
        open_head,
        env.config.block_count
    );
    let mut rbuf = [0u8; 4];
    for _ in (0..filesize).step_by(waka.len()) {
        let n = lfs_file_read(lfs, file, &mut rbuf[..waka.len()]);
        if n != Ok(waka.len() as u32) {
            common::dump::dump_fs(
                &env.badblock_ram.ram.data,
                env.config.block_size,
                env.config.block_count,
            );
            panic!(
                "lfs_file_read returned {:?} (expected {}; LFS_ERR_CORRUPT={:?})",
                n,
                waka.len(),
                Error::Corrupt
            );
        }
        assert_eq!(&rbuf[..waka.len()], waka);
    }
    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_alloc_chained_dir_exhaustion ---
/// Upstream: [cases.test_alloc_chained_dir_exhaustion]
/// if = 'ERASE_SIZE == 512', defines.ERASE_COUNT = 1024
///
/// Find max file size, chained dir fails, truncate until mkdir succeeds.
#[test]
fn test_alloc_chained_dir_exhaustion() {
    init_logger();
    let mut env = config_with_geometry(512, 1024);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_mkdir(lfs, "exhaustiondir"));
    for i in 0..10 {
        assert_ok!(lfs_mkdir(
            lfs,
            &format!("dirwithanexhaustivelylongnameforpadding{i}"),
        ));
    }

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    let blah = b"blahblahblahblah";
    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));

    let mut count = 0i32;
    loop {
        let err = lfs_file_write(lfs, file, blah);
        if err.is_err() {
            assert_err!(Error::NoSpace, err);
            break;
        }
        count += 1;
    }

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_remove(lfs, "exhaustion"));
    assert_ok!(lfs_remove(lfs, "exhaustiondir"));
    for i in 0..10 {
        assert_ok!(lfs_remove(
            lfs,
            &format!("dirwithanexhaustivelylongnameforpadding{i}"),
        ));
    }

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    for _ in 0..(count + 1) {
        let n = lfs_file_write(lfs, file, blah);
        assert_eq!(n, Ok(blah.len() as u32));
    }
    assert_ok!(lfs_file_sync(lfs, file));

    for i in 0..10 {
        assert_ok!(lfs_mkdir(
            lfs,
            &format!("dirwithanexhaustivelylongnameforpadding{i}"),
        ));
    }

    let mut err = lfs_mkdir(lfs, "exhaustiondir");
    assert_err!(Error::NoSpace, err);

    loop {
        err = lfs_mkdir(lfs, "exhaustiondir");
        if err != Err(Error::NoSpace) {
            break;
        }
        let filesize = lfs_file_size(lfs, file);
        assert!(filesize > 0, "need positive file size to truncate");
        let new_size = (filesize - blah.len() as i32).max(0) as u32;
        assert_ok!(lfs_file_truncate(lfs, file, new_size));
        assert_ok!(lfs_file_sync(lfs, file));
    }
    assert_ok!(err);

    err = lfs_mkdir(lfs, "exhaustiondir2");
    assert_err!(Error::NoSpace, err);

    assert_ok!(lfs_file_close(lfs, file));
    assert_ok!(lfs_unmount(lfs));
}

// --- test_alloc_outdated_lookahead ---
/// Upstream: [cases.test_alloc_outdated_lookahead]
/// if = 'ERASE_SIZE == 512', defines.ERASE_COUNT = 1024
///
/// Fill two files, remount, truncate+rewrite both; verify lookahead uses fresh population.
#[test]
fn test_alloc_outdated_lookahead() {
    init_logger();
    let mut env = config_with_geometry(512, 1024);
    init_context(&mut env);

    let block_size = env.config.block_size as usize;
    let block_count = env.config.block_count as usize;
    let size1 = ((block_count - 2) / 2) * (block_size - 8);
    let size2 = (block_count - 2).div_ceil(2) * (block_size - 8);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    let blah = b"blahblahblahblah";
    let chunk = blah.len();

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion1",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    for _ in (0..size1).step_by(chunk) {
        let n = lfs_file_write(lfs, file, &blah[..chunk]);
        assert_eq!(n, Ok(chunk as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion2",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    for _ in (0..size2).step_by(chunk) {
        let n = lfs_file_write(lfs, file, &blah[..chunk]);
        assert_eq!(n, Ok(chunk as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_unmount(lfs));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion1",
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    assert_ok!(lfs_file_sync(lfs, file));
    for _ in (0..size1).step_by(chunk) {
        let n = lfs_file_write(lfs, file, &blah[..chunk]);
        assert_eq!(n, Ok(chunk as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion2",
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    assert_ok!(lfs_file_sync(lfs, file));
    for _ in (0..size2).step_by(chunk) {
        let n = lfs_file_write(lfs, file, &blah[..chunk]);
        assert_eq!(n, Ok(chunk as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_unmount(lfs));
}

// --- test_alloc_outdated_lookahead_split_dir ---
/// Upstream: [cases.test_alloc_outdated_lookahead_split_dir]
/// if = 'ERASE_SIZE == 512', defines.ERASE_COUNT = 1024
///
/// Fill two files, remount, truncate one with hole; mkdir fails NOSPC, file create succeeds.
#[test]
fn test_alloc_outdated_lookahead_split_dir() {
    init_logger();
    let mut env = config_with_geometry(512, 1024);
    init_context(&mut env);

    let block_size = env.config.block_size as usize;
    let block_count = env.config.block_count as usize;
    let size1_full = ((block_count - 2) / 2) * (block_size - 8);
    let size2 = (block_count - 2).div_ceil(2) * (block_size - 8);
    let size1_hole = ((block_count - 2) / 2 - 1) * (block_size - 8);

    let lfs = &mut Lfs::default();
    assert_ok!(lfs_format(lfs, &env.config));
    assert_ok!(lfs_mount(lfs, &env.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    let blah = b"blahblahblahblah";
    let chunk = blah.len();

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion1",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    for _ in (0..size1_full).step_by(chunk) {
        let n = lfs_file_write(lfs, file, &blah[..chunk]);
        assert_eq!(n, Ok(chunk as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion2",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    for _ in (0..size2).step_by(chunk) {
        let n = lfs_file_write(lfs, file, &blah[..chunk]);
        assert_eq!(n, Ok(chunk as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_unmount(lfs));
    assert_ok!(lfs_mount(lfs, &env.config));

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "exhaustion1",
        LFS_O_WRONLY | LFS_O_TRUNC,
    ));
    assert_ok!(lfs_file_sync(lfs, file));
    for _ in (0..size1_hole).step_by(chunk) {
        let n = lfs_file_write(lfs, file, &blah[..chunk]);
        assert_eq!(n, Ok(chunk as u32));
    }
    assert_ok!(lfs_file_close(lfs, file));

    let err = lfs_mkdir(lfs, "split");
    assert_err!(Error::NoSpace, err);

    assert_ok!(lfs_file_open(
        lfs,
        file,
        "notasplit",
        LFS_O_WRONLY | LFS_O_CREAT,
    ));
    let n = lfs_file_write(lfs, file, b"hi");
    assert_eq!(n, Ok(2));
    assert_ok!(lfs_file_close(lfs, file));

    assert_ok!(lfs_unmount(lfs));
}
