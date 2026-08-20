use core::cell::{RefCell, UnsafeCell};
use core::ffi::c_void;
use core::mem;
use littlefs_rust_core::error::Error;
use littlefs_rust_core::lfs_type::OpenFlags;
use typenum::Unsigned;

use littlefs_rust_core::{Lfs, LfsConfig, LfsInfo};

use crate::config::Config;
use crate::dir::{dir_entry_from_info, ReadDir};
use crate::file::{File, FileAllocation};
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
            lfs: unsafe { mem::MaybeUninit::zeroed().assume_init() },
            config: UnsafeCell::new(lfs_config),
            cache,
        }
    }
}

// ── Filesystem ──────────────────────────────────────────────────────────────

impl<'a, S: Storage> Filesystem<'a, S> {
    fn new(storage: &'a mut S, alloc: &'a mut Allocation<S>) -> Self {
        alloc.config.get_mut().context = storage as *mut _ as *mut c_void;
        alloc.config.get_mut().read_buffer = alloc.cache.read.as_mut_ptr() as *mut c_void;
        alloc.config.get_mut().prog_buffer = alloc.cache.write.as_mut_ptr() as *mut c_void;
        alloc.config.get_mut().lookahead_buffer = alloc.cache.lookahead.as_mut_ptr() as *mut c_void;

        Self {
            alloc: RefCell::new(alloc),
            storage,
        }
    }

    /// Format `storage` with a fresh LittleFS filesystem.
    ///
    /// This erases any existing data. The storage can be mounted afterwards
    /// with [`Filesystem::mount`].
    pub fn format(storage: &'a mut S, alloc: &'a mut Allocation<S>) -> Result<(), Error> {
        let fs = Self::new(storage, alloc);

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
        unsafe { (*self.alloc.borrow().config.get()).cache_size }
    }

    // ── File access ─────────────────────────────────────────────────────

    /// Open a file with the given [`OpenFlags`].
    ///
    /// Common combinations: `READ`, `WRITE | CREATE | TRUNC`,
    /// `WRITE | CREATE | APPEND`.
    pub fn open<'b, 'c>(
        &'b self,
        alloc: &'b mut FileAllocation<'c, S>,
        path: &str,
        flags: OpenFlags,
    ) -> Result<File<'a, 'b, 'c, S>, Error> {
        File::open(self, alloc, path, flags)
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
