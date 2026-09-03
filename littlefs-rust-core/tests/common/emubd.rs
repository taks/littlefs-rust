use std::{ops::Deref, ptr::NonNull, rc::Rc};

use littlefs_rust_core::{Storage, error::Error};

struct EmubdBlock {
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
    pub erase_cycles: u32,

    pub badblock_behavior: BadblockBehavior,
    pub powerloss_behavior: PowerLossBehavior,
}

struct Emubd<'a> {
    blocks: Vec<Option<Rc<EmubdBlock>>>,

    readed: usize,
    proged: usize,
    erased: usize,

    ooo_block: Option<u32>,
    ooo_data: Option<Rc<EmubdBlock>>,

    cfg: &'a EmubdConfig,
}

impl<'d> Emubd<'d> {
    fn mutblock<'a>(&mut self, block: usize) -> &'a mut EmubdBlock {
        let block = &mut self.blocks[block];
        let block = if let Some(b) = block {
            if Rc::strong_count(b) == 1 {
                b
            } else {
                // rc > 1? need to create a copy
                let nblock = Rc::new(EmubdBlock {
                    wear: b.wear,
                    data: b.data.clone(),
                });
                *b = nblock;
                b
            }
        } else {
            // no block? need to allocate
            let nblock = Rc::new(EmubdBlock {
                wear: 0,
                data: vec![self.cfg.erase_value.unwrap_or_default(); self.cfg.erase_size as usize],
            });

            *block = Some(nblock);
            block.as_ref().unwrap()
        };
        #[expect(mutable_transmutes)]
        unsafe {
            std::mem::transmute(block.deref())
        }
    }
}

impl Storage for Emubd<'_> {
    fn read(&mut self, block: u32, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        assert!(block < self.cfg.erase_count);
        assert!(offset % self.cfg.read_size == 0);
        assert!(buf.len() % self.cfg.read_size as usize == 0);
        assert!(offset + buf.len() as u32 <= self.cfg.erase_size);

        if let Some(b) = self.blocks[block as usize].as_mut() {
            // block bad?
            if self.cfg.erase_cycles > 0
                && b.wear >= self.cfg.erase_cycles
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
        Ok(())
    }

    fn write(&mut self, block: u32, offset: u32, data: &[u8]) -> Result<(), Error> {
        todo!()
    }

    fn erase(&mut self, block: u32) -> Result<(), Error> {
        assert!(block < self.cfg.erase_count);

        // emulate out-of-order writes? save first write
        if self.cfg.powerloss_behavior == PowerLossBehavior::Ooo && self.ooo_block.is_none() {
            self.ooo_block = Some(block);
            self.ooo_data = self.blocks[block as usize].as_ref().map(|x| x.clone());
        }

        // get the block
        let b = self.mutblock(block as usize);

        // block bad?
        if self.cfg.erase_cycles > 0 && b.wear >= self.cfg.erase_cycles {
            if self.cfg.badblock_behavior == BadblockBehavior::Prog {
                return Err(Error::Corrupt);
            } else if self.cfg.badblock_behavior == BadblockBehavior::ProgNoop
                || self.cfg.badblock_behavior == BadblockBehavior::EraseNoop
            {
                return Ok(());
            }
        }

        todo!()
    }

    fn sync(&mut self) -> Result<(), Error> {
        // emulate out-of-order writes? reset first write, writes
        // cannot be out-of-order across sync
        if self.cfg.powerloss_behavior == PowerLossBehavior::Ooo {
            self.ooo_block = None;
            self.ooo_data = None;
        }

        Ok(())
    }
}
