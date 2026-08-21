use core::fmt::Error;

use crate::{Filesystem, Storage};

pub struct ReadDir<'a, 'b, S: Storage> {
    fs: &'b Filesystem<'a, S>,
}

impl<'a, 'b, S: Storage> ReadDir<'a, 'b, S> {
    pub(crate) fn new(fs: &'b Filesystem<'a, S>) -> Result<Self, Error> {
        Ok(Self { fs })
    }
}
