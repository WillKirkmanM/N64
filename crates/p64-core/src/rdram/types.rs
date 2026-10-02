use super::{RDRAM_ALIGNMENT, RDRAM_MASK, RDRAM_SIZE};
use crate::builder::registers::register_index;
use crate::{
    builder::{memory::Memory, registers::Registers},
    memory::AccessSize,
};

const CHIP_COUNT: usize = 4;
const CHIP_REGISTER_COUNT: usize = 16;
const CHIP_SELECT_SHIFT: u32 = 13;
const CHIP_SELECT_MASK: u64 = (CHIP_COUNT - 1) as u64;
const CHIP_REGISTER_MASK: u64 = 0x3f;
const REGISTER_SHIFT: u32 = 2;
const BOOT_SELECT: u32 = 0x14;
const BOOT_REFRESH: u32 = 0x0006_3634;
const CHIP_RESET_REGISTERS: [u32; CHIP_REGISTER_COUNT] = [
    0xb4190010, 0, 0x2b3b1a0b, 0, 0, 0, 0x101c0a04, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub struct Rdram {
    pub mem: Memory,
    pub regs: [[u32; CHIP_REGISTER_COUNT]; CHIP_COUNT],
}

impl Default for Rdram {
    fn default() -> Self {
        Self {
            mem: Memory::new()
                .with_size(RDRAM_SIZE)
                .with_alignment(RDRAM_ALIGNMENT)
                .with_mask(RDRAM_MASK)
                .is_little_endian()
                .is_rdram()
                .calculate_cycles_from_addr_size()
                .build(),
            regs: [CHIP_RESET_REGISTERS; CHIP_COUNT],
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum RiReg {
    Select = 3,
    Refresh = 4,
    RegsCount = 8,
}

pub struct RdramInterface {
    pub regs: Registers,
    pub(crate) timing: super::timing::RdramTiming,
}

impl Default for RdramInterface {
    fn default() -> Self {
        let mut regs = Registers::new()
            .with_size(RiReg::RegsCount as usize)
            .build();
        // Skip RDRAM initialisation in the replacement bootcode.
        regs[RiReg::Select as usize] = BOOT_SELECT;

        regs[RiReg::Refresh as usize] = BOOT_REFRESH;
        Self {
            regs,
            timing: super::timing::RdramTiming::default(),
        }
    }
}

impl RdramInterface {
    pub fn read(&self, address: u64, _access_size: AccessSize) -> u32 {
        let index = register_index(address);

        self.regs.regs.get(index).copied().unwrap_or(0)
    }

    pub fn write(&mut self, address: u64, value: u32, mask: u32) {
        let index = register_index(address);

        if let Some(reg) = self.regs.regs.get_mut(index) {
            crate::memory::masked_write_32(reg, value, mask);
        }
    }
}

impl Rdram {
    pub fn register(&self, address: u64) -> u32 {
        let (chip, index) = register_indices(address);

        self.regs[chip][index]
    }

    pub fn write_register(&mut self, address: u64, value: u32, mask: u32) {
        let (chip, index) = register_indices(address);
        crate::memory::masked_write_32(&mut self.regs[chip][index], value, mask);
    }
}

fn register_indices(address: u64) -> (usize, usize) {
    (
        ((address >> CHIP_SELECT_SHIFT) & CHIP_SELECT_MASK) as usize,
        ((address & CHIP_REGISTER_MASK) >> REGISTER_SHIFT) as usize,
    )
}
