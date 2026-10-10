use super::super::Instruction;
use crate::{exceptions, memory, N64};

const CARTRIDGE_START: u64 = 0x1000_0000;
const CARTRIDGE_END: u64 = 0x1fc0_0000;
const WORD_ALIGNMENT: u64 = 4;

#[derive(Clone, Copy)]
pub(super) struct MemoryOperand {
    pub register: usize,
    pub address: u64,
}

impl MemoryOperand {
    pub fn decode(n64: &N64, opcode: u32) -> Self {
        let instruction = Instruction::decode(opcode);

        Self {
            register: instruction.rt,
            address: n64
                .cpu()
                .gpr(instruction.rs)
                .wrapping_add(instruction.immediate_signed() as u64),
        }
    }

    pub fn aligned(
        self,
        n64: &mut N64,
        access: memory::AccessType,
        width: u64,
    ) -> Option<PhysicalAccess> {
        if !check_alignment(n64, self.address, width, access) {
            return None;
        }

        PhysicalAccess::translate(n64, self.address, access, width)
    }

    pub fn merged(
        self,
        n64: &mut N64,
        access: memory::AccessType,
        width: u64,
    ) -> Option<(PhysicalAccess, u64)> {
        let offset = memory::reverse_endian_address(n64, self.address, 1) & (width - 1);
        let mut physical = PhysicalAccess::translate(n64, self.address, access, 1)?;
        physical.address = crate::bits::align_down(physical.address, width);

        Some((physical, offset))
    }
}

#[derive(Clone, Copy)]
pub(super) struct PhysicalAccess {
    pub address: u64,
    pub cached: bool,
}

impl PhysicalAccess {
    pub fn translate(
        n64: &mut N64,
        address: u64,
        access: memory::AccessType,
        width: u64,
    ) -> Option<Self> {
        let (physical, cached, error) = memory::translate_address(n64, address, access);

        if error {
            return None;
        }

        Some(Self {
            address: memory::reverse_endian_address(n64, physical, width),
            cached,
        })
    }

    pub fn read<const WIDTH: u64>(self, n64: &mut N64) -> u64 {
        match WIDTH {
            1 => {
                (read_subword(
                    n64,
                    crate::bits::align_down(self.address, WORD_ALIGNMENT),
                    self.cached,
                ) >> byte_shift(self.address)) as u64
            }
            2 => {
                (read_subword(
                    n64,
                    crate::bits::align_down(self.address, WORD_ALIGNMENT),
                    self.cached,
                ) >> halfword_shift(self.address)) as u64
            }
            4 => memory::read_data(n64, self.address, memory::AccessSize::Word, self.cached) as u64,
            8 => memory::read_doubleword(n64, self.address, self.cached),
            _ => unreachable!("unsupported scalar width"),
        }
    }

    pub fn write<const WIDTH: u64>(self, n64: &mut N64, value: u64) {
        let (address, shift, mask) = match WIDTH {
            1 => (
                crate::bits::align_down(self.address, WORD_ALIGNMENT),
                byte_shift(self.address),
                0xff,
            ),
            2 => (
                crate::bits::align_down(self.address, WORD_ALIGNMENT),
                halfword_shift(self.address),
                0xffff,
            ),
            4 => (self.address, 0, u32::MAX),
            8 => {
                self.write_doubleword(n64, value, u64::MAX);
                return;
            }
            _ => unreachable!("unsupported scalar width"),
        };

        memory::write_data(
            n64,
            address,
            (value as u32) << shift,
            mask << shift,
            self.cached,
        );
    }

    pub fn write_doubleword(self, n64: &mut N64, value: u64, mask: u64) {
        self.write_word(n64, (value >> 32) as u32, (mask >> 32) as u32);
        self.write_next_word(n64, value as u32, mask as u32);
    }

    pub fn write_next_word(self, n64: &mut N64, value: u32, mask: u32) {
        Self {
            address: self.address + WORD_ALIGNMENT,
            ..self
        }
        .write_word(n64, value, mask);
    }

    pub fn write_word(self, n64: &mut N64, value: u32, mask: u32) {
        memory::write_data(n64, self.address, value, mask, self.cached);
    }
}

pub(super) fn read_subword(n64: &mut N64, address: u64, cached: bool) -> u32 {
    if !cached && (CARTRIDGE_START..CARTRIDGE_END).contains(&address) {
        crate::cartridge::read_subword(n64, address)
    } else {
        memory::read_data(n64, address, memory::AccessSize::Word, cached)
    }
}

pub(in crate::cpu) fn check_alignment(
    n64: &mut N64,
    address: u64,
    alignment: u64,
    access: memory::AccessType,
) -> bool {
    if !crate::bits::is_aligned(address, alignment) {
        exceptions::address_error_exception(n64, address, access);

        false
    } else {
        true
    }
}

fn byte_shift(address: u64) -> u64 {
    ((address & 3) ^ 3) * 8
}

fn halfword_shift(address: u64) -> u64 {
    ((address & 2) ^ 2) * 8
}
