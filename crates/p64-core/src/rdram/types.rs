use super::*;

pub struct Rdram {
    pub mem: Memory,
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum RiReg {
    Select = 3,
    RegsCount = 8,
}

#[derive(Clone)]
pub struct RdramInterface {
    pub regs: Memory,
}

impl Default for Rdram {
    fn default() -> Self {
        const ALIGNMENT: usize = 64 * 1024;

        Self {
            mem: Memory::new()
                .with_size(RDRAM_SIZE)
                .with_alignment(ALIGNMENT)
                .with_mask(RDRAM_MASK)
                .is_little_endian()
                .is_rdram()
                .calculate_cycles_from_addr_size()
                .build(),
        }
    }
}

impl Default for RdramInterface {
    fn default() -> Self {
        Self {
            regs: Memory::new().with_size(RiReg::RegsCount as usize),
        }
    }
}

impl RdramInterface {
    pub fn read(self, address: u64, _access_size: AccessSize) -> u32 {
        let index = ((address & 0xFFFF) >> 2) as u32;
        if index == RiReg::Select as u32 {
            return 0x14;
        }

        self.regs.read_u32(index as u64)
    }

    pub fn write(&mut self, address: u64, value: u32, mask: u32) {
        self.regs.write_u32((address & 0xFFFF) >> 2, value, mask);
    }
}
