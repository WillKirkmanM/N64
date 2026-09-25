use crate::builder::memory::Memory;
use super::memory::AccessSize;

use super::mmu::add_cycles;
use super::N64;

pub const RDRAM_MASK: usize = 0xFFFFFF;
pub const RDRAM_SIZE: usize = 0x800000;

pub fn rdram_calculate_cycles(length: u64) -> u64 {
    31 + (length / 3)
}

pub fn read_mem_fast(n64: &mut N64, address: u64, _access_size: AccessSize) -> u32 {
    n64.rdram.mem.read_mem_fast(address)
}

pub fn read_mem(n64: &mut N64, address: u64, access_size: AccessSize) -> u32 {
    add_cycles(
        n64,
        rdram_calculate_cycles(access_size as u64) / (access_size as u64 / 4),
    );
    n64.rdram.mem.read_mem_fast(address)
}

pub fn write_mem(n64: &mut N64, address: u64, value: u32, _mask: u32) {
    n64.rdram.mem.write_mem(address, value);
}

pub fn get_rdram_shared_pointer(n64: &mut N64) -> (*mut u8, usize) {
    let data_ptr = n64.rdram.mem.data_mut().as_mut_ptr();

    (data_ptr, RDRAM_SIZE)
}

pub fn read_regs(n64: &mut N64, address: u64, access_size: AccessSize) -> u32 {
    n64.rdram_interface.clone().read(address, access_size)
}

pub fn write_regs(n64: &mut N64, address: u64, value: u32, mask: u32) {
    n64.rdram_interface.write(address, value, mask)
}

mod types;
pub use types::{Rdram, RiReg, RdramInterface};
