use crate::builder::registers::Registers;

#[repr(u32)]
pub enum SerialRegister {
    DramAddr = 0,
    PifAddrRd64b = 1,
    PifAddrWr64b = 4,
    Status = 6,
    RegsCount = 7,
}

bitflags::bitflags! {
    pub struct SerialStatus: u32 {
        const DMA_BUSY = 1 << 0;
        const IO_BUSY = 1 << 1;
        const INTERRUPT = 1 << 12;
    }
}

#[derive(PartialEq)]
pub enum DmaDir {
    None,
    Write,
    Read,
}

pub struct Serial {
    pub regs: Registers,
    pub dma_dir: DmaDir,
}

impl Default for Serial {
    fn default() -> Self {
        Self {
            regs: Registers::new()
                .with_size(SerialRegister::RegsCount as usize)
                .build(),
            dma_dir: DmaDir::None,
        }
    }
}
