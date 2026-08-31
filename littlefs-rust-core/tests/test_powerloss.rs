//! Power-loss simulation tests.
//!
//! Upstream: tests/test_powerloss.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_powerloss.toml

#![allow(unused)]

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
    Lfs, LfsDir, LfsFile, LfsInfo, error::Error, lfs_dir_close, lfs_dir_open, lfs_file_close,
    lfs_file_open, lfs_file_read, lfs_file_sync, lfs_file_write, lfs_format, lfs_mkdir, lfs_mount,
    lfs_unmount,
};

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
