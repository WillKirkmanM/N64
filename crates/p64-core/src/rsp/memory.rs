use super::ADDR_MASK;
use crate::builder::memory::Memory;

#[derive(PartialEq, Copy, Clone, Default)]
pub enum DmaDir {
    #[default]
    None,
    Write,
    Read,
}

#[derive(Copy, Clone, Default)]
pub struct RspDma {
    pub dir: DmaDir,
    pub length: u32,
    pub memaddr: u32,
    pub dramaddr: u32,
}

pub struct MemoryState {
    pub mem: Memory,
    pub fifo: [RspDma; 2],
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            mem: Memory::new().with_size(0x2000).build(),
            fifo: [RspDma::default(); 2],
        }
    }
}

impl MemoryState {
    /// Scalar accesses wrap within DMEM, including unaligned multi-byte transfers.
    pub fn read_scalar<const N: usize>(&self, address: u32) -> [u8; N] {
        core::array::from_fn(|index| {
            self.mem[address.wrapping_add(index as u32) as usize & ADDR_MASK as usize]
        })
    }

    pub fn write_scalar<const N: usize>(&mut self, address: u32, bytes: [u8; N]) {
        for (offset, byte) in bytes.into_iter().enumerate() {
            self.mem[(address as usize + offset) & ADDR_MASK as usize] = byte;
        }
    }
}
