use core::mem::MaybeUninit;

use littlefs_rust_core::error::Error;

use littlefs_rust_core::{LfsDir, LfsInfo};

use crate::filesystem::Filesystem;
use crate::metadata::{DirEntry, FileType};
use crate::storage::Storage;

pub(crate) struct DirAllocation {
    pub(crate) dir: LfsDir,
}

impl DirAllocation {
    pub(crate) fn new() -> Self {
        Self {
            dir: unsafe { MaybeUninit::zeroed().assume_init() },
        }
    }
}

/// An open directory iterator.
///
/// Obtained from [`Filesystem::read_dir`]. Yields [`DirEntry`] items,
/// automatically skipping `.` and `..`. Closed on drop, or explicitly
/// via [`ReadDir::close`].
pub struct ReadDir<'a, 'b, S: Storage> {
    fs: &'b Filesystem<'a, S>,
    alloc: DirAllocation,
    closed: bool,
}

impl<'a, 'b, S: Storage> ReadDir<'a, 'b, S> {
    pub(crate) fn open(fs: &'a Filesystem<'a, S>, path: &str) -> Result<Self, Error> {
        let mut alloc = Box::new(DirAllocation::new());
        {
            let mut inner = fs.alloc.borrow_mut();
            littlefs_rust_core::lfs_dir_open(&mut inner.lfs, &mut alloc.dir, path)?;
        }
        Ok(ReadDir {
            fs,
            alloc,
            closed: false,
        })
    }

    /// Close the directory handle. Consumes `self`.
    ///
    /// Dropping a [`ReadDir`] also closes it, but errors are silently ignored.
    pub fn close(mut self) -> Result<(), Error> {
        self.closed = true;
        let mut inner = self.fs.alloc.borrow_mut();
        littlefs_rust_core::lfs_dir_close(&mut inner.lfs, &mut self.alloc.dir)
    }
}

impl<S: Storage> Iterator for ReadDir<'_, S> {
    type Item = Result<DirEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut info = unsafe { core::mem::zeroed::<LfsInfo>() };
            let rc = {
                let mut inner = self.fs.alloc.borrow_mut();
                littlefs_rust_core::lfs_dir_read(&mut inner.lfs, &mut self.alloc.dir, &mut info)
            };

            return match rc {
                Ok(0) => None,
                Err(e) => Some(Err(e)),
                _ => {
                    let entry = dir_entry_from_info(&info);
                    if entry.name == "." || entry.name == ".." {
                        continue;
                    }
                    Some(Ok(entry))
                }
            };
        }
    }
}

impl<S: Storage> Drop for ReadDir<'_, S> {
    fn drop(&mut self) {
        if !self.closed {
            if let Ok(mut inner) = self.fs.alloc.try_borrow_mut() {
                let _ = littlefs_rust_core::lfs_dir_close(&mut inner.lfs, &mut self.alloc.dir);
            }
        }
    }
}

pub(crate) fn dir_entry_from_info(info: &LfsInfo) -> DirEntry {
    let nul = info
        .name
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(info.name.len());
    let name = core::str::from_utf8(&info.name[..nul]).unwrap_or("").into();
    let file_type = if info.type_ == littlefs_rust_core::lfs_type::lfs_type::LFS_TYPE_DIR as u8 {
        FileType::Dir
    } else {
        FileType::File
    };
    DirEntry {
        name,
        file_type,
        size: info.size,
    }
}
