use std::ptr::NonNull;

use littlefs_rust_core::{Storage, error::Error};

struct EmubdBlock {
    rc: u32,
    wear: u32,
    data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BadblockBehavior {
    Prog,
    Erase,
    Read,
    ProgNoop,
    EraseNoop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PowerLossBehavior {
    Noop,
    Ooo,
}

struct EmubdConfig {
    pub read_size: u32,
    pub prog_size: u32,
    pub erase_size: u32,
    pub erase_count: u32,
    pub erase_value: Option<u8>,
    pub erase_cycle: u32,

    pub badblock_behavior: BadblockBehavior,
    pub powerloss_behavior: PowerLossBehavior,
}

struct Emubd<'a> {
    blocks: Vec<*mut EmubdBlock>,

    readed: usize,
    proged: usize,
    erased: usize,

    ooo_block: i32,
    ooo_data: *mut EmubdBlock,

    cfg: &'a EmubdConfig,
}

impl Emubd<'_> {
    fn decblock(block: *mut EmubdBlock) {
        if let Some(block) = unsafe { block.as_mut() } {
            block.rc -= 1;
            if block.rc == 0 {
                // TODO: free the block
                // free(block);
            }
        }
    }
}

impl Storage for Emubd<'_> {
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        assert!(block < self.cfg.erase_count);
        assert!(offset % self.cfg.read_size == 0);
        assert!(buf.len() % self.cfg.read_size as usize == 0);
        assert!(offset + buf.len() as u32 <= self.cfg.erase_size);

        if let Some(b) = unsafe { self.blocks[block as usize].as_mut() } {
            // block bad?
            if self.cfg.erase_cycle > 0
                && b.wear >= self.cfg.erase_cycle
                && self.cfg.badblock_behavior == BadblockBehavior::Read
            {
                return Err(Error::Corrupt);
            }

            // read data
            buf.copy_from_slice(&b.data[(offset as usize)..(offset as usize + buf.len())]);
        } else {
            // zero for consistency
            buf.fill(self.cfg.erase_value.unwrap_or_default());
        }

        // track reads
        self.readed += buf.len();

        todo!()
    }

    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error> {
        todo!()
    }

    fn erase(&mut self, block: u32) -> Result<(), Error> {
        todo!()
    }

    fn sync(&mut self) -> Result<(), Error> {
        if self.cfg.powerloss_behavior == PowerLossBehavior::Ooo {
            Self::decblock(self.ooo_data);
            self.ooo_block = -1;
            self.ooo_data = std::ptr::null_mut();
        }

        Ok(())
    }
}
