const PRIMARY_SHIFT: u32 = 26;
const FUNCTION_MASK: u32 = 0x3f;
const REGISTER_MASK: u32 = 0x1f;
const RS_SHIFT: u32 = 21;
const RT_SHIFT: u32 = 16;
const RD_SHIFT: u32 = 11;
const SHIFT_AMOUNT_SHIFT: u32 = 6;
const INSTRUCTION_BYTES: u64 = 4;
const INSTRUCTION_ADDRESS_SHIFT: u32 = 2;
const JUMP_INDEX_MASK: u32 = 0x03ff_ffff;
const JUMP_REGION_MASK: u64 = 0xffff_ffff_f000_0000;

#[derive(Clone, Copy)]
pub(crate) struct Instruction {
    bits: u32,
    pub rs: usize,
    pub rt: usize,
    pub rd: usize,
}

impl Instruction {
    #[inline]
    pub fn decode(opcode: u32) -> Self {
        Self {
            bits: opcode,
            rs: ((opcode >> RS_SHIFT) & REGISTER_MASK) as usize,
            rt: ((opcode >> RT_SHIFT) & REGISTER_MASK) as usize,
            rd: ((opcode >> RD_SHIFT) & REGISTER_MASK) as usize,
        }
    }

    #[inline]
    pub fn primary(self) -> u32 {
        self.bits >> PRIMARY_SHIFT
    }

    #[inline]
    pub fn function(self) -> usize {
        (self.bits & FUNCTION_MASK) as usize
    }

    #[inline]
    pub fn immediate_unsigned(self) -> u64 {
        u64::from(self.bits as u16)
    }

    #[inline]
    pub fn immediate_signed(self) -> i64 {
        i64::from(self.bits as i16)
    }

    #[inline]
    pub fn shift_amount(self) -> u32 {
        (self.bits >> SHIFT_AMOUNT_SHIFT) & REGISTER_MASK
    }

    #[inline]
    pub fn branch_target(self, pc: u64) -> u64 {
        pc.wrapping_add((self.immediate_signed() as u64) << INSTRUCTION_ADDRESS_SHIFT)
            .wrapping_add(INSTRUCTION_BYTES)
    }

    #[inline]
    pub fn jump_target(self, pc: u64) -> u64 {
        (pc.wrapping_add(INSTRUCTION_BYTES) & JUMP_REGION_MASK)
            | (u64::from(self.bits & JUMP_INDEX_MASK) << INSTRUCTION_ADDRESS_SHIFT)
    }

    #[inline]
    pub fn requires_wide(self) -> bool {
        matches!(self.primary(), 24..=27 | 39 | 44 | 45 | 52 | 55 | 60 | 63)
            || (self.primary() == 0
                && matches!(self.function(),
                20 | 22 | 23 | 28..=31 | 44..=47 | 56 | 58..=60 | 62 | 63))
    }

    #[inline]
    pub fn native_requires_wide(self) -> bool {
        self.primary() == 25
            || (self.primary() == 0
                && matches!(
                    self.function(),
                    20 | 22 | 23 | 45 | 47 | 56 | 58..=60 | 62 | 63
                ))
    }

}

pub fn rd(opcode: u32) -> u32 {
    (opcode >> RD_SHIFT) & REGISTER_MASK
}

pub fn rs(opcode: u32) -> u32 {
    (opcode >> RS_SHIFT) & REGISTER_MASK
}

pub fn rt(opcode: u32) -> u32 {
    (opcode >> RT_SHIFT) & REGISTER_MASK
}

pub fn imm(opcode: u32) -> u16 {
    opcode as u16
}

pub fn se32(value: i32) -> u64 {
    value as i64 as u64
}

pub fn se16(value: i16) -> u64 {
    value as i64 as u64
}

