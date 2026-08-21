use core::fmt::Error;

use littlefs_rust_core::LfsDir;

use crate::{Filesystem, Storage};

pub struct ReadDir<'a, 'b, S: Storage> {
    fs: &'b Filesystem<'a, S>,
    dir: LfsDir,
}

impl<'a, 'b, S: Storage> ReadDir<'a, 'b, S> {
    pub(crate) fn new(fs: &'b Filesystem<'a, S>) -> Self {
        Self {
            fs,
            dir: unsafe { core::mem::MaybeUninit::zeroed().assume_init() },
        }
    }
}

impl<S: Storage> Iterator for ReadDir<'_, '_, S> {
    type Item = Result<(), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
