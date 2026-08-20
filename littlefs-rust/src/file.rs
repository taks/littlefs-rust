use core::cell::{RefCell, UnsafeCell};

use littlefs_rust_core::error::Error;

use littlefs_rust_core::lfs_type::OpenFlags;
use littlefs_rust_core::{LfsFile, LfsFileConfig};

use crate::filesystem::{Bytes, Filesystem};
use crate::metadata::SeekFrom;
use crate::storage::Storage;

pub struct FileAllocation<'a, S: Storage> {
    pub(crate) file: LfsFile<'a>,
    cache: UnsafeCell<Bytes<S::CACHE_SIZE>>,
    pub(crate) file_config: LfsFileConfig<'a>,
}

impl<S: Storage> FileAllocation<'_, S> {
    pub fn new() -> Self {
        Self {
            file: unsafe { core::mem::MaybeUninit::zeroed().assume_init() },
            cache: Default::default(),
            file_config: unsafe { core::mem::MaybeUninit::zeroed().assume_init() },
        }
    }
}

/// An open file handle.
///
/// Obtained from [`Filesystem::open`]. Automatically closed on drop; call
/// [`File::close`] explicitly to check for errors.
pub struct File<'a, 'b, 'c, S: Storage> {
    fs: &'b Filesystem<'a, S>,
    alloc: RefCell<&'b mut FileAllocation<'c, S>>,
}

impl<'a, 'b, 'c, S: Storage> File<'a, 'b, 'c, S> {
    pub(crate) fn open(
        fs: &'b Filesystem<'a, S>,
        alloc: &'b mut FileAllocation<'c, S>,
        path: &str,
        flags: OpenFlags,
    ) -> Result<Self, Error> {
        alloc.file_config.buffer = unsafe { alloc.cache.get().as_mut_unchecked().as_mut_slice() };

        littlefs_rust_core::lfs_file_opencfg(
            &mut fs.alloc.borrow_mut().lfs,
            &mut alloc.file,
            path,
            flags,
            &mut alloc.file_config,
        )?;

        Ok(File {
            fs,
            alloc: RefCell::new(alloc),
        })
    }

    /// Read up to `buf.len()` bytes from the current position.
    /// Returns the number of bytes actually read.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<u32, Error> {
        let mut inner = self.fs.alloc.borrow_mut();
        let file = &mut self.alloc.borrow_mut().file;
        littlefs_rust_core::lfs_file_read(&mut inner.lfs, file, buf)
    }

    /// Write `data` at the current position. Returns the number of bytes written.
    pub fn write(&mut self, data: &[u8]) -> Result<u32, Error> {
        let mut inner = self.fs.alloc.borrow_mut();
        let file = &mut self.alloc.borrow_mut().file;
        littlefs_rust_core::lfs_file_write(&mut inner.lfs, file, data)
    }

    /// Seek to a position. Returns the new absolute offset.
    pub fn seek(&mut self, pos: SeekFrom) -> Result<u32, Error> {
        let (off, whence) = match pos {
            SeekFrom::Start(n) => (
                n as i32,
                littlefs_rust_core::lfs_type::lfs_whence_flags::LFS_SEEK_SET,
            ),
            SeekFrom::Current(n) => (
                n,
                littlefs_rust_core::lfs_type::lfs_whence_flags::LFS_SEEK_CUR,
            ),
            SeekFrom::End(n) => (
                n,
                littlefs_rust_core::lfs_type::lfs_whence_flags::LFS_SEEK_END,
            ),
        };
        let mut inner = self.fs.alloc.borrow_mut();
        let file = &mut self.alloc.borrow_mut().file;
        littlefs_rust_core::lfs_file_seek(&mut inner.lfs, file, off, whence)
    }

    /// Return the current read/write position.
    pub fn tell(&self) -> u32 {
        let mut inner = self.fs.alloc.borrow_mut();
        let file = &mut self.alloc.borrow_mut().file;
        let rc = littlefs_rust_core::lfs_file_tell(&mut inner.lfs, file);
        drop(inner);
        rc as u32
    }

    /// Return the file size in bytes.
    pub fn size(&self) -> u32 {
        let mut inner = self.fs.alloc.borrow_mut();
        let file = &mut self.alloc.borrow_mut().file;
        let rc = littlefs_rust_core::lfs_file_size(&mut inner.lfs, file);
        drop(inner);
        rc as u32
    }

    /// Flush cached writes to storage.
    pub fn sync(&mut self) -> Result<(), Error> {
        let mut inner = self.fs.alloc.borrow_mut();
        let file = &mut self.alloc.borrow_mut().file;
        let rc = littlefs_rust_core::lfs_file_sync(&mut inner.lfs, file);
        drop(inner);
        rc
    }

    /// Truncate or extend the file to `size` bytes.
    pub fn truncate(&mut self, size: u32) -> Result<(), Error> {
        let mut inner = self.fs.alloc.borrow_mut();
        let file = &mut self.alloc.borrow_mut().file;
        littlefs_rust_core::lfs_file_truncate(&mut inner.lfs, file, size)
    }

    /// Close the file, flushing any pending writes. Consumes `self`.
    ///
    /// Dropping a [`File`] also closes it, but errors are silently ignored.
    pub fn close(mut self) -> Result<(), Error> {
        let mut inner = self.fs.alloc.borrow_mut();
        let file = &mut self.alloc.borrow_mut().file;
        littlefs_rust_core::lfs_file_close(&mut inner.lfs, file)
    }
}

impl<S: Storage> Drop for File<'_, '_, '_, S> {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.fs.alloc.try_borrow_mut() {
            let file = &mut self.alloc.borrow_mut().file;
            let _ = littlefs_rust_core::lfs_file_close(&mut inner.lfs, file);
        }
    }
}
