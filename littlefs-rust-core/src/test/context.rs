//! TestContext: env + Lfs<T, U> ready for format/mount. Single setup for unit tests.

use alloc::vec::Vec;

use crate::borrow_unchecked::borrow_unchecked;
use crate::test::ram::{BLOCK_SIZE, RamStorage, make_config};
use crate::{Lfs, LfsConfig, lfs_format, lfs_mount, lfs_unmount};
use core::mem::MaybeUninit;

const DEFAULT_BLOCK_COUNT: u32 = 128;

/// Env + Lfs in one. Owns RAM BD, config, buffers. Use new(), then format_fs/mount_fs.
pub struct TestContext {
    pub ram: RamStorage,
    pub config: LfsConfig<Vec<u8>>,
    lfs: MaybeUninit<Lfs<Vec<u8>>>,
    _read_buf: alloc::vec::Vec<u8>,
    _prog_buf: alloc::vec::Vec<u8>,
    _lookahead_buf: alloc::vec::Vec<u8>,
}

#[allow(unused)]
impl TestContext {
    /// Fresh env with block_count blocks. Ready for format.
    pub fn new(block_count: u32) -> Self {
        let block_size = BLOCK_SIZE;
        let ram = RamStorage::new(block_size, block_count);
        let read_buf = alloc::vec![0u8; block_size as usize];
        let prog_buf = alloc::vec![0u8; block_size as usize];
        let lookahead_buf = alloc::vec![0u8; block_size as usize];

        let mut config = make_config(block_count, &ram);
        config.read_buffer = read_buf;
        config.prog_buffer = prog_buf;
        config.lookahead_buffer = lookahead_buf.as_ptr() as *mut core::ffi::c_void;

        let mut ctx = Self {
            ram,
            config,
            lfs: MaybeUninit::zeroed(),
            _read_buf: read_buf,
            _prog_buf: prog_buf,
            _lookahead_buf: lookahead_buf,
        };
        ctx.config.context = &mut ctx.ram as *mut RamStorage as *mut core::ffi::c_void;
        ctx
    }

    /// Default block_count 128. Matches upstream.
    pub fn default_blocks() -> Self {
        Self::new(DEFAULT_BLOCK_COUNT)
    }

    pub fn config(&self) -> &LfsConfig<Vec<u8>> {
        &self.config
    }

    pub fn lfs_mut(&mut self) -> &mut Lfs<Vec<u8>> {
        unsafe { self.lfs.as_mut_ptr().as_mut().unwrap() }
    }

    /// Format the filesystem. Panics on error.
    pub fn format(&mut self) {
        let config = unsafe { borrow_unchecked(self.config()) };
        let err = lfs_format(self.lfs_mut(), config);
        assert_eq!(err, Ok(()), "lfs_format failed: {:?}", err);
    }

    /// Mount the filesystem. Panics on error.
    pub fn mount(&mut self) {
        let config = unsafe { borrow_unchecked(self.config()) };
        let err = lfs_mount(self.lfs_mut(), config);
        assert_eq!(err, Ok(()), "lfs_mount failed: {:?}", err);
    }

    /// Unmount. Panics on error.
    pub fn unmount(&mut self) {
        let err = lfs_unmount(self.lfs_mut());
        assert_eq!(err, Ok(()), "lfs_unmount failed: {:?}", err);
    }
}
