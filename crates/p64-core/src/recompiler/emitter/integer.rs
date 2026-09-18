use super::{
    supported_branch_opcode, supported_memory_opcode, supported_opcode, MAX_BLOCK_INSTRUCTIONS,
};
use crate::opcode::{PrimaryOpcode, SpecialOpcode};

use super::{
    encoding::{
        load_operand, set_condition, sign_extend_word, ArithmeticOperation, Condition,
        GUEST_REGISTER_BYTES, GUEST_REGISTER_COUNT,
    },
    operands::Operands,
};

mod encoding;
use encoding::{
    arithmetic_immediate, compare_result, load_guest, load_memory, mov_register, store_memory,
};

const CACHED_HOST_REGISTER_COUNT: usize = 4;
const FIRST_CACHED_HOST_REGISTER: u8 = 8; // Volatile R8..R11 on both native ABIs.
const SCALAR_CODE_CAPACITY: usize = 64;
const BLOCK_CODE_CAPACITY: usize = MAX_BLOCK_INSTRUCTIONS * 32;
const IMMEDIATE_MASK: u32 = u16::MAX as u32;
const IMMEDIATE_HIGH_SHIFT: u32 = u16::BITS;
const HIGH_SHIFT_OFFSET: u8 = u32::BITS as u8;
