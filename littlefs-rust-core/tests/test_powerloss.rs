//! Power-loss simulation tests.
//!
//! Upstream: tests/test_powerloss.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_powerloss.toml

mod common;

use common::{
    LFS_O_APPEND, LFS_O_CREAT, LFS_O_RDONLY, LFS_O_WRONLY, assert_ok_at, default_config,
    init_context, init_logger,
    powerloss::{
        PowerLossBehavior, init_powerloss_context, powerloss_config,
        powerloss_config_with_behavior, run_powerloss_exhaustive, run_powerloss_linear,
        run_powerloss_log,
    },
    read_block_raw, write_block_raw,
};
use littlefs_rust_core::{
    Lfs<T> LfsDir, LfsFile, LfsInfo, error::Error, lfs_dir_close, lfs_dir_open, lfs_file_close,
    lfs_file_open, lfs_file_read, lfs_file_sync, lfs_file_write, lfs_format, lfs_mkdir, lfs_mount,
    lfs_unmount,
};

// --- test_powerloss_only_rev ---
// Upstream: write rev+1 to one block of dir pair; mount picks higher rev, read/write still works.
#[test]
fn test_powerloss_only_rev() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    assert_ok_at("mount", lfs_mount(lfs, &env.config));

    let path_nb = "notebook";
    let path_paper = "notebook/paper";
    assert_ok_at("mkdir notebook", lfs_mkdir(lfs, path_nb));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open paper create",
        lfs_file_open(
            lfs,
            file,
            path_paper,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        ),
    );
    let buf = b"hello";
    for i in 0..5 {
        let n = lfs_file_write(lfs, file, buf);
        assert_eq!(n, Ok(buf.len() as u32));
        assert_ok_at(
            &format!("file_sync #{} (first loop)", i + 1),
            lfs_file_sync(lfs, file),
        );
    }
    assert_ok_at("file_close", lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open paper read",
        lfs_file_open(lfs, file, path_paper, LFS_O_RDONLY),
    );
    let mut rbuf = [0u8; 256];
    for _ in 0..5 {
        let n = lfs_file_read(lfs, file, &mut rbuf[..5]);
        assert_eq!(n, Ok(5));
        assert_eq!(&rbuf[..5], b"hello");
    }
    assert_ok_at("file_close", lfs_file_close(lfs, file));
    assert_ok_at("unmount", lfs_unmount(lfs));

    // Get dir pair and rev from a fresh mount, then corrupt rev
    assert_ok_at("mount before corrupt", lfs_mount(lfs, &env.config));
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok_at("dir_open notebook", lfs_dir_open(lfs, dir, path_nb));
    let pair = dir.m.pair;
    let rev = dir.m.rev;
    assert_ok_at("dir_close", lfs_dir_close(lfs, dir));
    assert_ok_at("unmount before corrupt", lfs_unmount(lfs));

    // Partial write: rev+1 in block
    let block_size = env.config.block_size as usize;
    let mut block_buf = vec![0u8; block_size];
    let read_fn = env.config.read.expect("read");

    let _ = read_fn(&env.config, pair[1], 0, &mut block_buf);

    block_buf[0..4].copy_from_slice(&(rev + 1).to_le_bytes());
    let erase_fn = env.config.erase.expect("erase");
    let prog_fn = env.config.prog.expect("prog");

    let _ = erase_fn(&env.config, pair[1]);
    let _ = prog_fn(&env.config, pair[1], 0, &block_buf);

    assert_ok_at("mount after corrupt", lfs_mount(lfs, &env.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open paper read after corrupt",
        lfs_file_open(lfs, file, path_paper, LFS_O_RDONLY),
    );
    for _ in 0..5 {
        let n = lfs_file_read(lfs, file, &mut rbuf[..5]);
        assert_eq!(n, Ok(5));
        assert_eq!(&rbuf[..5], b"hello");
    }
    assert_ok_at("file_close", lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open paper append",
        lfs_file_open(lfs, file, path_paper, LFS_O_WRONLY | LFS_O_APPEND),
    );
    let buf2 = b"goodbye";
    for i in 0..5 {
        let n = lfs_file_write(lfs, file, buf2);
        assert_eq!(n, Ok(buf2.len() as u32));
        assert_ok_at(
            &format!("file_sync #{} (after corrupt)", i + 1),
            lfs_file_sync(lfs, file),
        );
    }
    assert_ok_at("file_close", lfs_file_close(lfs, file));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open paper read final",
        lfs_file_open(lfs, file, path_paper, LFS_O_RDONLY),
    );
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
    assert_ok_at("file_close", lfs_file_close(lfs, file));
    assert_ok_at("unmount final", lfs_unmount(lfs));
}

// --- test_powerloss_trigger_first_write ---
// Unit test: fail_after_writes=1 causes first prog/erase to return Err(Error::Io).
#[test]
fn test_powerloss_trigger_first_write() {
    init_logger();
    let mut env = powerloss_config(128);
    init_powerloss_context(&mut env);
    env.set_fail_after_writes(1);

    let lfs = &mut Lfs::default();
    let err = lfs_format(lfs, &env.config);
    assert_eq!(
        err,
        Err(Error::Io),
        "format should fail on first write with fail_after_writes=1"
    );
}

// --- test_powerloss_runner_smoke ---
// Smoke test: run_powerloss_linear with mkdir op; verify mount works after power loss.
#[test]
fn test_powerloss_runner_smoke() {
    init_logger();
    let mut env = powerloss_config(128);
    init_powerloss_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    let snapshot = env.snapshot();

    let path_d = "d";
    let result = run_powerloss_linear(
        &mut env,
        &snapshot,
        64,
        |lfs, config| {
            lfs_mount(lfs, config)?;
            let err = lfs_mkdir(lfs, path_d);
            if let Err(err) = err {
                let _ = lfs_unmount(lfs);
                return Err(err);
            }
            lfs_unmount(lfs)?;
            Ok(())
        },
        |lfs, config| {
            lfs_mount(lfs, config)?;
            let _ = lfs_unmount(lfs);
            Ok(())
        },
    );
    result.expect("run_powerloss_linear should complete");
}

/// Upstream: [cases.test_powerloss_partial_prog]
/// defines.PROG_SIZE < BLOCK_SIZE, BYTE_OFF = [0, PROG_SIZE-1, PROG_SIZE/2], BYTE_VALUE = [0x33, 0xcc].
/// Corrupt one byte in a directory block at BYTE_OFF with BYTE_VALUE. Verify mount and read/write still work.
#[test]
fn test_powerloss_partial_prog() {
    init_logger();
    const PROG_SIZE: u32 = 16;
    const BLOCK_SIZE: u32 = 512;
    let byte_offs: [u32; 3] = [0, PROG_SIZE - 1, PROG_SIZE / 2];
    let byte_values: [u8; 2] = [0x33, 0xcc];
    const DIR_BLOCK: u32 = 1; // second superblock block has root dir data

    for &byte_off in &byte_offs {
        for &byte_value in &byte_values {
            let mut env = default_config(128);
            init_context(&mut env);
            let cfg = &env.config;

            let lfs = &mut Lfs::default();
            assert_ok_at("format", lfs_format(lfs, cfg));
            assert_ok_at("mount", lfs_mount(lfs, cfg));
            let path_a = "a";
            assert_ok_at("mkdir a", lfs_mkdir(lfs, path_a));
            assert_ok_at("unmount", lfs_unmount(lfs));

            let mut block = vec![0u8; BLOCK_SIZE as usize];
            assert_eq!(
                Ok(()),
                read_block_raw(cfg, DIR_BLOCK, 0, &mut block),
                "read_block_raw block {DIR_BLOCK}"
            );
            block[byte_off as usize] = byte_value;
            assert_eq!(
                Ok(()),
                write_block_raw(cfg, DIR_BLOCK, 0, &block),
                "write_block_raw block {DIR_BLOCK}"
            );

            assert_ok_at(
                &format!("mount after corrupt off={byte_off} val=0x{byte_value:02x}"),
                lfs_mount(lfs, cfg),
            );
            let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
            let r = littlefs_rust_core::lfs_stat(lfs, path_a, info);
            assert!(r.is_ok(), "lfs_stat a after corrupt: {r:?}");
            assert_ok_at("unmount after verify", lfs_unmount(lfs));
        }
    }
}

// --- test_powerloss_snapshot_restore ---
// Unit test: snapshot and restore preserve BD state.
#[test]
fn test_powerloss_snapshot_restore() {
    init_logger();
    let mut env = powerloss_config(128);
    init_powerloss_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    let snapshot = env.snapshot();

    // Mutate ram
    env.ctx.ram.data[0] = 0;
    assert_ne!(env.ctx.ram.data[0], snapshot[0]);

    env.restore(&snapshot);
    assert_eq!(&env.ctx.ram.data[..], &snapshot[..]);

    assert_ok_at("mount after restore", lfs_mount(lfs, &env.config));
    assert_ok_at("unmount", lfs_unmount(lfs));
}

// =============================================================================
// Debug tests. test_powerloss_only_rev / test_debug_powerloss_after_corrupt still
// fail with NOSPC on sync #5 after rev corruption; lfs_dir_split is now implemented.
// Remaining issue may be in compact/relocate when reading from corrupted block.
// =============================================================================

/// Minimal: file in root, write "hello" once, sync. No mkdir, no subdir.
#[test]
fn test_debug_file_root_single_write_sync() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    assert_ok_at("mount", lfs_mount(lfs, &env.config));

    let path = "paper";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open create",
        lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND),
    );
    let buf = b"hello";
    let n = lfs_file_write(lfs, file, buf);
    assert_eq!(n, Ok(buf.len() as u32));
    assert_ok_at("file_sync", lfs_file_sync(lfs, file));
    assert_ok_at("file_close", lfs_file_close(lfs, file));
    assert_ok_at("unmount", lfs_unmount(lfs));
}

/// File in root, write "hello" 5x with sync each (like powerloss but no mkdir). Bisects root vs subdir.
#[test]
fn test_debug_file_root_repeated_write_sync() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    assert_ok_at("mount", lfs_mount(lfs, &env.config));

    let path = "paper";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open create",
        lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND),
    );
    let buf = b"hello";
    for i in 0..5 {
        let n = lfs_file_write(lfs, file, buf);
        assert_eq!(n, Ok(buf.len() as u32));
        assert_ok_at(&format!("file_sync #{}", i + 1), lfs_file_sync(lfs, file));
    }
    assert_ok_at("file_close", lfs_file_close(lfs, file));
    assert_ok_at("unmount", lfs_unmount(lfs));
}

/// Exact powerloss pattern (mkdir + file in subdir) but bisects which sync fails.
#[test]
fn test_debug_file_subdir_which_sync_fails() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    assert_ok_at("mount", lfs_mount(lfs, &env.config));

    let path_nb = "notebook";
    let path_paper = "notebook/paper";
    assert_ok_at("mkdir notebook", lfs_mkdir(lfs, path_nb));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open paper create",
        lfs_file_open(
            lfs,
            file,
            path_paper,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        ),
    );
    let buf = b"hello";
    for i in 0..5 {
        let n = lfs_file_write(lfs, file, buf);
        assert_eq!(n, Ok(buf.len() as u32));
        let err = lfs_file_sync(lfs, file);
        assert_ok_at(&format!("file_sync #{}", i + 1), err);
    }
    assert_ok_at("file_close", lfs_file_close(lfs, file));
    assert_ok_at("unmount", lfs_unmount(lfs));
}

/// Reproduces powerloss flow: setup, corrupt rev, then append. Bisects which sync fails after corrupt.
#[test]
fn test_debug_powerloss_after_corrupt_append() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    assert_ok_at("mount", lfs_mount(lfs, &env.config));

    let path_nb = "notebook";
    let path_paper = "notebook/paper";
    assert_ok_at("mkdir notebook", lfs_mkdir(lfs, path_nb));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open paper create",
        lfs_file_open(
            lfs,
            file,
            path_paper,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        ),
    );
    let buf = b"hello";
    for i in 0..5 {
        let n = lfs_file_write(lfs, file, buf);
        assert_eq!(n, Ok(buf.len() as u32));
        assert_ok_at(&format!("file_sync #{}", i + 1), lfs_file_sync(lfs, file));
    }
    assert_ok_at("file_close", lfs_file_close(lfs, file));
    assert_ok_at("unmount", lfs_unmount(lfs));

    assert_ok_at("mount before corrupt", lfs_mount(lfs, &env.config));
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok_at("dir_open notebook", lfs_dir_open(lfs, dir, path_nb));
    let pair = dir.m.pair;
    let rev = dir.m.rev;
    assert_ok_at("dir_close", lfs_dir_close(lfs, dir));
    assert_ok_at("unmount before corrupt", lfs_unmount(lfs));

    let block_size = env.config.block_size as usize;
    let mut block_buf = vec![0u8; block_size];
    let read_fn = env.config.read.expect("read");

    let _ = read_fn(&env.config, pair[1], 0, &mut block_buf);

    block_buf[0..4].copy_from_slice(&(rev + 1).to_le_bytes());
    let erase_fn = env.config.erase.expect("erase");
    let prog_fn = env.config.prog.expect("prog");

    let _ = erase_fn(&env.config, pair[1]);
    let _ = prog_fn(&env.config, pair[1], 0, &block_buf);

    assert_ok_at("mount after corrupt", lfs_mount(lfs, &env.config));
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open paper append",
        lfs_file_open(lfs, file, path_paper, LFS_O_WRONLY | LFS_O_APPEND),
    );
    let buf2 = b"goodbye";
    for i in 0..5 {
        let n = lfs_file_write(lfs, file, buf2);
        assert_eq!(n, Ok(buf2.len() as u32));
        assert_ok_at(
            &format!("file_sync #{} (after corrupt)", i + 1),
            lfs_file_sync(lfs, file),
        );
    }
    assert_ok_at("file_close", lfs_file_close(lfs, file));
    assert_ok_at("unmount", lfs_unmount(lfs));
}

// --- test_powerloss_runner_smoke_log ---
// Same as test_powerloss_runner_smoke but using run_powerloss_log (exponential stepping).
#[test]
fn test_powerloss_runner_smoke_log() {
    init_logger();
    let mut env = powerloss_config(128);
    init_powerloss_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    let snapshot = env.snapshot();

    let path_d = "d";
    let result = run_powerloss_log(
        &mut env,
        &snapshot,
        64,
        |lfs, config| {
            lfs_mount(lfs, config)?;
            let err = lfs_mkdir(lfs, path_d);
            if let Err(err) = err {
                let _ = lfs_unmount(lfs);
                return Err(err);
            }
            lfs_unmount(lfs)?;
            Ok(())
        },
        |lfs, config| {
            lfs_mount(lfs, config)?;
            let _ = lfs_unmount(lfs);
            Ok(())
        },
    );
    result.expect("run_powerloss_log should complete");
}

// --- test_powerloss_runner_smoke_exhaustive ---
// Same as test_powerloss_runner_smoke but using run_powerloss_exhaustive with depth=2.
#[test]
fn test_powerloss_runner_smoke_exhaustive() {
    init_logger();
    let mut env = powerloss_config(128);
    init_powerloss_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    let snapshot = env.snapshot();

    let path_d = "d";
    let result = run_powerloss_exhaustive(
        &mut env,
        &snapshot,
        64,
        2,
        |lfs, config| {
            lfs_mount(lfs, config)?;
            let err = lfs_mkdir(lfs, path_d);
            if let Err(err) = err {
                let _ = lfs_unmount(lfs);
                return Err(err);
            }
            lfs_unmount(lfs)?;
            Ok(())
        },
        |lfs, config| {
            lfs_mount(lfs, config)?;
            let _ = lfs_unmount(lfs);
            Ok(())
        },
    );
    result.expect("run_powerloss_exhaustive depth=2 should complete");
}

// --- test_powerloss_ooo_smoke ---
// OOO behaviour: writes between syncs may be reordered. Verify FS recovers correctly.
#[test]
fn test_powerloss_ooo_smoke() {
    init_logger();
    let mut env = powerloss_config_with_behavior(128, PowerLossBehavior::Ooo);
    init_powerloss_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    let snapshot = env.snapshot();

    let path_d = "d";
    let result = run_powerloss_linear(
        &mut env,
        &snapshot,
        64,
        |lfs, config| {
            lfs_mount(lfs, config)?;
            let err = lfs_mkdir(lfs, path_d);
            if let Err(err) = err {
                let _ = lfs_unmount(lfs);
                return Err(err);
            }
            lfs_unmount(lfs)?;
            Ok(())
        },
        |lfs, config| {
            lfs_mount(lfs, config)?;
            let _ = lfs_unmount(lfs);
            Ok(())
        },
    );
    result.expect("OOO powerloss linear should complete");
}

/// Minimal subdir: mkdir + file, single write + sync.
#[test]
fn test_debug_file_subdir_single_write_sync() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut Lfs::default();
    assert_ok_at("format", lfs_format(lfs, &env.config));
    assert_ok_at("mount", lfs_mount(lfs, &env.config));

    assert_ok_at("mkdir notebook", lfs_mkdir(lfs, "notebook"));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok_at(
        "file_open paper create",
        lfs_file_open(
            lfs,
            file,
            "notebook/paper",
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
        ),
    );
    let buf = b"hello";
    let n = lfs_file_write(lfs, file, buf);
    assert_eq!(n, Ok(buf.len() as u32));
    assert_ok_at("file_sync", lfs_file_sync(lfs, file));
    assert_ok_at("file_close", lfs_file_close(lfs, file));
    assert_ok_at("unmount", lfs_unmount(lfs));
}
