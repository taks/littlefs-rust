use std::{ops::Deref, rc::Rc};

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
    /// Progs are atomic
    Noop,
    /// Blocks are written out-of-order
    Ooo,
}

pub struct EmubdConfig<'d> {
    pub read_size: u32,
    pub prog_size: u32,
    pub erase_size: u32,
    pub erase_count: u32,
    pub erase_value: Option<u8>,
    pub erase_cycles: u32,

    pub badblock_behavior: BadblockBehavior,
    pub power_cycles: u32,
    pub powerloss_behavior: PowerLossBehavior,

    pub powerloss_cb: &'d dyn Fn() -> (),
}

pub struct Emubd<'a> {
    blocks: Vec<Option<Rc<EmubdBlock>>>,

    readed: usize,
    proged: usize,
    erased: usize,
    power_cycles: u32,

    ooo_block: Option<usize>,
    ooo_data: Option<Rc<EmubdBlock>>,

    cfg: &'a EmubdConfig<'a>,
}

impl<'d> Emubd<'d> {
    pub fn new(bdcfg: &'d EmubdConfig) -> Self {
        Self {
            blocks: vec![None; bdcfg.erase_count as usize],
            readed: 0,
            proged: 0,
            erased: 0,
            power_cycles: bdcfg.power_cycles,
            ooo_block: None,
            ooo_data: None,
            cfg: bdcfg,
        }
    }

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

    fn powerloss(&mut self) -> Result<(), Error> {
        let mut ooo_data = None;

        // emulate out-of-order writes?
        if self.cfg.powerloss_behavior == PowerLossBehavior::Ooo
            && let Some(block) = self.ooo_block
        {
            // since writes between syncs are allowed to be out-of-order, it
            // shouldn't hurt to restore the first write on powerloss, right?
            ooo_data = std::mem::replace(&mut self.blocks[block], self.ooo_data.clone());
        }

        // simulate power loss
        (self.cfg.powerloss_cb)();

        // if we continue, undo out-of-order write emulation
        if self.cfg.powerloss_behavior == PowerLossBehavior::Ooo
            && let Some(block) = self.ooo_block
        {
            self.blocks[block] = ooo_data;
        }

        Ok(())
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
        // check if write is valid
        assert!(block < self.cfg.erase_count);
        assert!(offset % self.cfg.prog_size == 0);
        assert!(data.len() % self.cfg.prog_size as usize == 0);
        assert!(offset + data.len() as u32 <= self.cfg.erase_size);

        // get the block
        let b = self.mutblock(block as usize);

        // block bad?
        if self.cfg.erase_cycles > 0 && b.wear >= self.cfg.erase_cycles {
            match self.cfg.badblock_behavior {
                BadblockBehavior::Prog => return Err(Error::Corrupt),
                BadblockBehavior::ProgNoop | BadblockBehavior::EraseNoop => return Ok(()),
                _ => (),
            };
        }

        // were we erased properly?
        if let Some(v) = self.cfg.erase_value {
            assert!(
                b.data
                    .iter()
                    .skip(offset as usize)
                    .take(data.len())
                    .all(|&x| x == v)
            );
        }

        // prog data
        b.data[(offset as usize)..(offset as usize + data.len())].copy_from_slice(data);

        // track progs
        self.proged += data.len();

        // lose power?
        if self.power_cycles > 0 {
            self.power_cycles -= 1;
            if self.power_cycles == 0 {
                self.powerloss()?;
            }
        }

        Ok(())
    }

    fn erase(&mut self, block: u32) -> Result<(), Error> {
        assert!(block < self.cfg.erase_count);

        // emulate out-of-order writes? save first write
        if self.cfg.powerloss_behavior == PowerLossBehavior::Ooo && self.ooo_block.is_none() {
            self.ooo_block = Some(block as usize);
            self.ooo_data = self.blocks[block as usize].as_ref().map(|x| x.clone());
        }

        // get the block
        let b = self.mutblock(block as usize);

        // block bad?
        if self.cfg.erase_cycles > 0 {
            if b.wear >= self.cfg.erase_cycles {
                match self.cfg.badblock_behavior {
                    BadblockBehavior::Erase => return Err(Error::Corrupt),
                    BadblockBehavior::EraseNoop => return Ok(()),
                    _ => (),
                };
            } else {
                b.wear += 1;
            }
        }

        // emulate an erase value?
        if let Some(e) = self.cfg.erase_value {
            b.data.fill(e);
        }

        // track erases
        self.erased += self.cfg.erase_size as usize;

        // lose power?
        if self.power_cycles > 0 {
            self.power_cycles -= 1;
            if self.power_cycles == 0 {
                self.powerloss()?;
            }
        }

        Ok(())
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
