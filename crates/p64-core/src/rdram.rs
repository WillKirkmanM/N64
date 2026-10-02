mod types;
mod timing;
#[cfg(feature = "accuracy-diagnostics")]
mod diagnostics;
#[cfg(feature = "accuracy-diagnostics")]
pub(crate) use diagnostics::{trace_write, write_watched};
pub use types::{Rdram, RdramInterface, RiReg};

use super::memory::AccessSize;

use super::mmu::add_cycles;
use super::N64;

pub const RDRAM_MASK: usize = 0xFFFFFF;
pub const RDRAM_SIZE: usize = 0x800000;
const RDRAM_ALIGNMENT: usize = 64 * 1024;
const REPEAT_OFFSET_MASK: u64 = 0x7ff;
const TRANSFER_START_CYCLES: u64 = 31;
const TRANSFER_BYTES_PER_CYCLE: u64 = 3;
const REGISTER_READ_CYCLES: u64 = 20;
const REGISTER_WINDOW_MASK: u64 = 0x3ff;
const BROADCAST_READ_OFFSET: u64 = 0x200;

// Bulk DMA service time; CPU requests also arbitrate with refresh and scanout.
pub fn rdram_calculate_cycles(length: u64) -> u64 {
    TRANSFER_START_CYCLES + (length / TRANSFER_BYTES_PER_CYCLE)
}

pub fn read_mem_fast(n64: &mut N64, address: u64, _access_size: AccessSize) -> u32 {
    // Unpopulated RDRAM does not mirror the installed RAM.
    if address >= RDRAM_SIZE as u64 {
        return 0;
    }

    n64.rdram.mem.read_mem_fast(address)
}

pub fn read_mem(n64: &mut N64, address: u64, access_size: AccessSize) -> u32 {
    timing::read(n64, address, access_size);

    read_mem_fast(n64, address, access_size)
}

pub(crate) fn count_changed(n64: &mut N64, previous: u64, count: u64) {
    n64.rdram_interface.timing.rebase(previous, count);
}

pub(crate) fn record_write(n64: &mut N64, address: u64) {
    timing::write(n64, address);
}

pub fn write_mem(n64: &mut N64, address: u64, value: u32, mask: u32) {
    #[cfg(feature = "accuracy-diagnostics")]
    trace_write(n64, "rdram", address, value, mask);
    if crate::mips::repeat_enabled(n64) {
        let start = address + (mask.leading_zeros() / 8) as u64;
        write_repeat(n64, start, &value.to_be_bytes());
        return;
    }

    if address < RDRAM_SIZE as u64 {
        timing::write(n64, address);
        n64.rdram.mem.write_u32(address, value, mask);
    }
}

fn write_repeat(n64: &mut N64, start: u64, pattern: &[u8]) {
    let length = crate::mips::consume_repeat(n64)
        .saturating_sub(crate::bits::alignment_offset(start, 8) as u32);

    for offset in 0..length as u64 {
        let address = crate::bits::replace_bits(start, start + offset, REPEAT_OFFSET_MASK);

        if address < RDRAM_SIZE as u64 {
            timing::write(n64, address);
            n64.rdram.mem[address as usize ^ n64.byte_swap] =
                pattern[(address as usize) % pattern.len()];
        }
    }
}

pub fn write_mem_double_repeat(n64: &mut N64, address: u64, value: u64) {
    write_repeat(n64, address, &value.to_be_bytes());
}

pub fn read_rdram_regs(n64: &mut N64, address: u64, _size: AccessSize) -> u32 {
    add_cycles(n64, REGISTER_READ_CYCLES);
    if address & REGISTER_WINDOW_MASK == BROADCAST_READ_OFFSET {
        return 0;
    }

    n64.rdram.register(address)
}

pub fn write_rdram_regs(n64: &mut N64, address: u64, value: u32, mask: u32) {
    n64.rdram.write_register(address, value, mask);
}

pub fn get_rdram_shared_pointer(n64: &mut N64) -> (*mut u8, usize) {
    let data_ptr = n64.rdram.mem.data_mut().as_mut_ptr();

    (data_ptr, RDRAM_SIZE)
}

pub fn read_regs(n64: &mut N64, address: u64, access_size: AccessSize) -> u32 {
    n64.rdram_interface.read(address, access_size)
}

pub fn write_regs(n64: &mut N64, address: u64, value: u32, mask: u32) {
    n64.rdram_interface.write(address, value, mask)
}
