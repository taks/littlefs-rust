use core::cell::{RefCell, UnsafeCell};
use core::ffi::c_void;
use littlefs_rust_core::error::Error;
use typenum::Unsigned;

use littlefs_rust_core::{Lfs, LfsConfig, LfsInfo};

use crate::config::{self, Config};
use crate::dir::{dir_entry_from_info, ReadDir};
use crate::file::File;
use crate::metadata::{DirEntry, Metadata, OpenFlags};
use crate::storage::Storage;

pub(crate) type Bytes<SIZE> = hybrid_array::Array<u8, SIZE>;

struct Cache<S: Storage> {
    read: Bytes<S::CACHE_SIZE>,
    write: Bytes<S::CACHE_SIZE>, // lookahead: aligned::Aligned<aligned::A4, Bytes<Storage::LOOKAHEAD_SIZE>>,
    lookahead: hybrid_array::Array<u64, S::LOOKAHEAD_SIZE>,
}

impl<S: Storage> Default for Cache<S> {
    fn default() -> Self {
        Self {
            read: Default::default(),
            write: Default::default(),
            lookahead: Default::default(),
        }
    }
}

pub struct Allocation<S: Storage> {
    pub(crate) lfs: Lfs,
    pub(crate) config: UnsafeCell<LfsConfig>,
    cache: Cache<S>,
}

/// A mounted LittleFS filesystem.
///
/// All methods take `&self` via interior mutability, so multiple [`File`] and
/// [`ReadDir`] handles can coexist. The internal state is heap-allocated and
/// pinned so that core pointers remain stable across moves.
///
/// Use [`Filesystem::format`] to initialize storage, then [`Filesystem::mount`]
/// to obtain a `Filesystem`. Call [`Filesystem::unmount`] to cleanly unmount
/// and recover the storage, or let [`Drop`] handle it automatically.
///
/// `Filesystem` is `!Send` and `!Sync` (due to interior `RefCell`). This is
/// appropriate for single-threaded embedded use. If you need cross-thread
/// access, wrap it in a `Mutex`.
pub struct Filesystem<'a, S: Storage> {
    pub(crate) alloc: RefCell<&'a mut Allocation<S>>,
    storage: &'a mut S,
}

// ── Trampolines ─────────────────────────────────────────────────────────────

fn trampoline_read<S: Storage>(
    cfg: &LfsConfig,
    block: u32,
    off: u32,
    buffer: &mut [u8],
) -> Result<(), Error> {
    let storage = unsafe { &mut *(cfg.context as *mut S) };
    storage.read(block, off, buffer)
}

fn trampoline_prog<S: Storage>(
    cfg: &LfsConfig,
    block: u32,
    off: u32,
    buffer: &[u8],
) -> Result<(), Error> {
    let storage = unsafe { &mut *(cfg.context as *mut S) };
    storage.write(block, off, buffer)
}

fn trampoline_erase<S: Storage>(cfg: &LfsConfig, block: u32) -> Result<(), Error> {
    let storage = unsafe { &mut *(cfg.context as *mut S) };
    storage.erase(block)
}

fn trampoline_sync<S: Storage>(cfg: &LfsConfig) -> Result<(), Error> {
    let storage = unsafe { &mut *(cfg.context as *mut S) };
    storage.sync()
}

// ── FsInner construction ────────────────────────────────────────────────────
impl<S: Storage> Allocation<S> {
    pub fn new() -> Self {
        const {
            assert!(S::BLOCK_CYCLES >= -1);
            assert!(S::BLOCK_CYCLES != 0);
            assert!(S::BLOCK_SIZE >= 128);

            assert!(S::CACHE_SIZE::USIZE.is_multiple_of(S::READ_SIZE));
            assert!(S::CACHE_SIZE::USIZE.is_multiple_of(S::WRITE_SIZE));
            assert!(S::BLOCK_SIZE.is_multiple_of(S::CACHE_SIZE::USIZE));
        }

        let config = Config::new(S::BLOCK_SIZE as _, S::BLOCK_COUNT as _);

        let cache = Cache::<S>::default();

        let lfs_config = LfsConfig {
            context: core::ptr::null_mut(),
            read: Some(trampoline_read::<S>),
            prog: Some(trampoline_prog::<S>),
            erase: Some(trampoline_erase::<S>),
            sync: Some(trampoline_sync::<S>),
            read_size: S::READ_SIZE as _,
            prog_size: S::WRITE_SIZE as _,
            block_size: config.block_size,
            block_count: config.block_count,
            block_cycles: S::BLOCK_CYCLES as _,
            cache_size: S::CACHE_SIZE::U32,
            lookahead_size: 8 * S::LOOKAHEAD_SIZE::U32,
            compact_thresh: u32::MAX,
            read_buffer: core::ptr::null_mut(),
            prog_buffer: core::ptr::null_mut(),
            lookahead_buffer: core::ptr::null_mut(),
            name_max: config.name_max,
            file_max: config.file_max,
            attr_max: config.attr_max,
            metadata_max: 0,
            inline_max: 0,
        };

        Allocation {
            lfs: unsafe { core::mem::zeroed() },
            config: RefCell::new(lfs_config),
            cache,
        }
    }
}

// ── Filesystem ──────────────────────────────────────────────────────────────

impl<'a, S: Storage> Filesystem<'a, S> {
    fn new(storage: &'a mut S, alloc: &'a mut Allocation<S>) -> Self {
        {
            let config = alloc.config.get_mut();
            config.context = storage as *mut _ as *mut c_void;
            config.read_buffer = alloc.cache.read.as_mut_ptr() as *mut c_void;
            config.prog_buffer = alloc.cache.write.as_mut_ptr() as *mut c_void;
            config.lookahead_buffer = alloc.cache.lookahead.as_mut_ptr() as *mut c_void;
        }

        Self {
            alloc: RefCell::new(alloc),
            storage,
        }
    }

    /// Format `storage` with a fresh LittleFS filesystem.
    ///
    /// This erases any existing data. The storage can be mounted afterwards
    /// with [`Filesystem::mount`].
    pub fn format(storage: &mut S) -> Result<(), Error> {
        let alloc = Allocation::new();
        let fs = Self::new(storage, &mut alloc);

        let mut alloc = fs.alloc.borrow_mut();
        let config = alloc.config.get();
        littlefs_rust_core::lfs_format(&mut alloc.lfs, unsafe { &*config })
    }

    /// Mount an existing filesystem. Takes ownership of the storage.
    ///
    /// On failure the storage is returned alongside the error so the caller
    /// can retry (e.g. format + mount).
    pub fn mount(storage: &'a mut S, alloc: &'a mut Allocation<S>) -> Result<Self, Error> {
        let fs = Self::new(storage, alloc);

        {
            let mut alloc = fs.alloc.borrow_mut();
            let config = unsafe { core::mem::transmute(&alloc.config) };
            littlefs_rust_core::lfs_mount(&mut alloc.lfs, config)?;
        }

        Ok(fs)
    }

    pub(crate) fn cache_size(&self) -> u32 {
        self.alloc.borrow().config.cache_size
    }

    // ── File access ─────────────────────────────────────────────────────

    /// Open a file with the given [`OpenFlags`].
    ///
    /// Common combinations: `READ`, `WRITE | CREATE | TRUNC`,
    /// `WRITE | CREATE | APPEND`.
    pub fn open(&self, path: &str, flags: OpenFlags) -> Result<File<'_, S>, Error> {
        File::open(self, path, flags)
    }

    // ── Convenience file I/O ────────────────────────────────────────────

    /// Read an entire file into a `Vec<u8>`.
    pub fn read_to_vec(&self, path: &str) -> Result<Vec<u8>, Error> {
        let mut file = self.open(path, OpenFlags::READ)?;
        let size = file.size() as usize;
        let mut buf = vec![0u8; size];
        if size > 0 {
            let n = file.read(&mut buf)?;
            buf.truncate(n as usize);
        }
        Ok(buf)
    }

    /// Write `data` to a file, creating or truncating it.
    pub fn write_file(&self, path: &str, data: &[u8]) -> Result<(), Error> {
        let mut file = self.open(
            path,
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNC,
        )?;
        let mut offset = 0;
        while offset < data.len() {
            let n = file.write(&data[offset..])? as usize;
            offset += n;
        }
        Ok(())
    }

    // ── Path operations ─────────────────────────────────────────────────

    /// Create a directory. Fails if it already exists.
    pub fn mkdir(&self, path: &str) -> Result<(), Error> {
        let mut alloc = self.alloc.borrow_mut();
        littlefs_rust_core::lfs_mkdir(&mut alloc.lfs, path)
    }

    /// Remove a file or empty directory.
    pub fn remove(&self, path: &str) -> Result<(), Error> {
        let mut alloc = self.alloc.borrow_mut();
        littlefs_rust_core::lfs_remove(&mut alloc.lfs, path)
    }

    /// Rename or move a file or directory.
    pub fn rename(&self, from: &str, to: &str) -> Result<(), Error> {
        let mut alloc = self.alloc.borrow_mut();
        littlefs_rust_core::lfs_rename(&mut alloc.lfs, from, to)
    }

    /// Get metadata for a file or directory.
    pub fn stat(&self, path: &str) -> Result<Metadata, Error> {
        let mut info = unsafe { core::mem::zeroed::<LfsInfo>() };
        {
            let mut alloc = self.alloc.borrow_mut();
            littlefs_rust_core::lfs_stat(&mut alloc.lfs, path, &mut info)?;
        }
        let entry = dir_entry_from_info(&info);
        Ok(Metadata {
            name: entry.name,
            file_type: entry.file_type,
            size: entry.size,
        })
    }

    /// Returns `true` if `path` exists.
    pub fn exists(&self, path: &str) -> bool {
        self.stat(path).is_ok()
    }

    // ── Directory listing ───────────────────────────────────────────────

    /// Open a directory for iteration. The returned [`ReadDir`] is an
    /// [`Iterator`] that skips `.` and `..` entries.
    pub fn read_dir(&self, path: &str) -> Result<ReadDir<'_, S>, Error> {
        ReadDir::open(self, path)
    }

    // ── FS-level ────────────────────────────────────────────────────────

    /// Return the number of allocated blocks.
    pub fn fs_size(&self) -> Result<u32, Error> {
        let mut alloc = self.alloc.borrow_mut();
        littlefs_rust_core::lfs_fs_size(&mut alloc.lfs)
    }

    /// Run garbage collection to reclaim unused blocks.
    pub fn gc(&mut self) -> Result<(), Error> {
        let mut alloc = self.alloc.borrow_mut();
        littlefs_rust_core::lfs_fs_gc(&mut alloc.lfs)
    }
}

impl<'a, S: Storage> Drop for Filesystem<'a, S> {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.alloc.try_borrow_mut() {
            let _ = littlefs_rust_core::lfs_unmount(&mut inner.lfs);
        }
    }
}

// ── format helper (borrows storage instead of taking ownership) ─────────────

struct BorrowedFsInner<'a, S: Storage> {
    lfs: Lfs,
    config: LfsConfig,
    storage: &'a mut S,
    _read_buf: Vec<u8>,
    _prog_buf: Vec<u8>,
    _lookahead_buf: Vec<u8>,
}

fn build_inner_borrowed<'a, S: Storage>(
    storage: &'a mut S,
    config: &Config,
) -> BorrowedFsInner<'a, S> {
    let cache_size = config.resolve_cache_size() as usize;
    let lookahead_size = config.resolve_lookahead_size() as usize;

    let mut read_buf = vec![0u8; cache_size];
    let mut prog_buf = vec![0u8; cache_size];
    let mut lookahead_buf = vec![0u8; lookahead_size];

    let lfs_config = LfsConfig {
        context: core::ptr::null_mut(),
        read: Some(trampoline_read::<S>),
        prog: Some(trampoline_prog::<S>),
        erase: Some(trampoline_erase::<S>),
        sync: Some(trampoline_sync::<S>),
        read_size: config.read_size,
        prog_size: config.prog_size,
        block_size: config.block_size,
        block_count: config.block_count,
        block_cycles: config.block_cycles,
        cache_size: config.resolve_cache_size(),
        lookahead_size: config.resolve_lookahead_size(),
        compact_thresh: u32::MAX,
        read_buffer: read_buf.as_mut_ptr() as *mut c_void,
        prog_buffer: prog_buf.as_mut_ptr() as *mut c_void,
        lookahead_buffer: lookahead_buf.as_mut_ptr() as *mut c_void,
        name_max: config.name_max,
        file_max: config.file_max,
        attr_max: config.attr_max,
        metadata_max: 0,
        inline_max: 0,
    };

    BorrowedFsInner {
        lfs: unsafe { core::mem::zeroed() },
        config: lfs_config,
        storage,
        _read_buf: read_buf,
        _prog_buf: prog_buf,
        _lookahead_buf: lookahead_buf,
    }
}

fn wire_context_borrowed<S: Storage>(inner: &mut BorrowedFsInner<'_, S>) {
    inner.config.context = inner.storage as *mut S as *mut c_void;
    inner.config.read_buffer = inner._read_buf.as_mut_ptr() as *mut c_void;
    inner.config.prog_buffer = inner._prog_buf.as_mut_ptr() as *mut c_void;
    inner.config.lookahead_buffer = inner._lookahead_buf.as_mut_ptr() as *mut c_void;
}
