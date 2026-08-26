use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use littlefs_rust_core::error::Error;

use littlefs_rust_core::lfs_type::OpenFlags;
use littlefs_rust_core::{LfsFile, LfsFileConfig};

use crate::filesystem::Filesystem;
use crate::metadata::SeekFrom;
use crate::storage::StorageWithConfig;

pub(crate) struct FileAllocation<'a> {
    pub(crate) file: LfsFile<'a>,
    _cache: Vec<u8>,
    pub(crate) file_config: LfsFileConfig<'a>,
}

impl FileAllocation<'_> {
    pub(crate) fn new(cache_size: u32) -> Self {
        let mut cache = vec![0u8; cache_size as usize];
        let file_config = LfsFileConfig {
            buffer: unsafe { core::mem::transmute::<&mut [u8], &mut [u8]>(cache.as_mut_slice()) },
            attrs: &mut [],
        };
        Self {
            file: LfsFile::default(),
            _cache: cache,
            file_config,
        }
    }
}

/// An open file handle.
///
/// Obtained from [`Filesystem::open`]. Automatically closed on drop; call
/// [`File::close`] explicitly to check for errors.
pub struct File<'a, S: StorageWithConfig> {
    fs: &'a Filesystem<S>,
    alloc: Box<FileAllocation<'a>>,
    closed: bool,
}

impl<'a, S: StorageWithConfig> File<'a, S> {
    pub(crate) fn open(fs: &'a Filesystem<S>, path: &str, flags: OpenFlags) -> Result<Self, Error> {
        let mut alloc = Box::new(FileAllocation::new(fs.cache_size()));
        {
            let mut inner = fs.inner.borrow_mut();
            littlefs_rust_core::lfs_file_opencfg(
                &mut inner.lfs,
                &mut alloc.file,
                path,
                flags,
                &mut alloc.file_config,
            )?;
        }
        Ok(File {
            fs,
            alloc,
            closed: false,
        })
    }

    /// Read up to `buf.len()` bytes from the current position.
    /// Returns the number of bytes actually read.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<u32, Error> {
        let mut inner = self.fs.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_file_read(&mut inner.lfs, &mut self.alloc.file, buf);
        drop(inner);
        rc
    }

    /// Write `data` at the current position. Returns the number of bytes written.
    pub fn write(&mut self, data: &[u8]) -> Result<u32, Error> {
        let mut inner = self.fs.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_file_write(&mut inner.lfs, &mut self.alloc.file, data);
        drop(inner);
        rc
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
        let mut inner = self.fs.inner.borrow_mut();
        littlefs_rust_core::lfs_file_seek(&mut inner.lfs, &mut self.alloc.file, off, whence)
    }

    /// Return the current read/write position.
    pub fn tell(&self) -> u32 {
        let mut inner = self.fs.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_file_tell(&mut inner.lfs, &self.alloc.file);
        drop(inner);
        rc as u32
    }

    /// Return the file size in bytes.
    pub fn size(&self) -> u32 {
        let mut inner = self.fs.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_file_size(&mut inner.lfs, &self.alloc.file);
        drop(inner);
        rc as u32
    }

    /// Flush cached writes to storage.
    pub fn sync(&mut self) -> Result<(), Error> {
        let mut inner = self.fs.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_file_sync(&mut inner.lfs, &mut self.alloc.file);
        drop(inner);
        rc
    }

    /// Truncate or extend the file to `size` bytes.
    pub fn truncate(&mut self, size: u32) -> Result<(), Error> {
        let mut inner = self.fs.inner.borrow_mut();
        let rc = littlefs_rust_core::lfs_file_truncate(&mut inner.lfs, &mut self.alloc.file, size);
        drop(inner);
        rc
    }

    /// Close the file, flushing any pending writes. Consumes `self`.
    ///
    /// Dropping a [`File`] also closes it, but errors are silently ignored.
    pub fn close(mut self) -> Result<(), Error> {
        self.closed = true;
        let mut inner = self.fs.inner.borrow_mut();
        littlefs_rust_core::lfs_file_close(&mut inner.lfs, &mut self.alloc.file)
    }
}

impl<S: StorageWithConfig> Drop for File<'_, S> {
    fn drop(&mut self) {
        if !self.closed
            && let Ok(mut inner) = self.fs.inner.try_borrow_mut()
        {
            let _ = littlefs_rust_core::lfs_file_close(&mut inner.lfs, &mut self.alloc.file);
        }
    }
}
