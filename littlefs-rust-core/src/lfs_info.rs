//! File and filesystem info. Per lfs.h struct lfs_info, lfs_fsinfo, lfs_attr, lfs_file_config.

use zerocopy_derive::{FromBytes, Immutable, IntoBytes, KnownLayout, TryFromBytes};

use crate::types::lfs_size_t;
use core::ffi::c_void;

/// Per lfs.h struct lfs_info
#[repr(C)]
pub struct LfsInfo {
    pub type_: u8,
    pub size: lfs_size_t,
    pub name: [u8; 256], // LFS_NAME_MAX+1
}

/// Per lfs.h struct lfs_fsinfo
#[repr(C)]
pub struct LfsFsinfo {
    pub disk_version: u32,
    pub block_size: lfs_size_t,
    pub block_count: lfs_size_t,
    pub name_max: lfs_size_t,
    pub file_max: lfs_size_t,
    pub attr_max: lfs_size_t,
}

/// Per lfs.h struct lfs_attr
#[repr(C)]
#[derive(Immutable, KnownLayout, TryFromBytes)]
pub struct LfsAttr<'a> {
    pub type_: u8,
    pub buffer: &'a mut [u8],
}

unsafe impl<'a> zerocopy::IntoBytes for LfsAttr<'a> {
    fn as_bytes(&self) -> &[u8]
    where
        Self: zerocopy::Immutable,
    {
        todo!()
    }

    fn as_mut_bytes(&mut self) -> &mut [u8]
    where
        Self: zerocopy::FromBytes,
    {
        todo!()
    }

    fn write_to(&self, dst: &mut [u8]) -> Result<(), zerocopy::SizeError<&Self, &mut [u8]>>
    where
        Self: zerocopy::Immutable,
    {
        todo!()
    }

    fn write_to_prefix(&self, dst: &mut [u8]) -> Result<(), zerocopy::SizeError<&Self, &mut [u8]>>
    where
        Self: zerocopy::Immutable,
    {
        todo!()
    }

    fn write_to_suffix(&self, dst: &mut [u8]) -> Result<(), zerocopy::SizeError<&Self, &mut [u8]>>
    where
        Self: zerocopy::Immutable,
    {
        todo!()
    }

    fn only_derive_is_allowed_to_implement_this_trait()
    where
        Self: Sized,
    {
        todo!()
    }
}

/// Per lfs.h struct lfs_file_config
#[repr(C)]
pub struct LfsFileConfig<'a> {
    pub buffer: &'a mut [u8],
    pub attrs: &'a mut [LfsAttr<'a>],
    // pub attr_count: lfs_size_t,
}

// Safe: default config (all nulls) is shareable. Callers must not mutate.
unsafe impl Sync for LfsFileConfig<'_> {}
