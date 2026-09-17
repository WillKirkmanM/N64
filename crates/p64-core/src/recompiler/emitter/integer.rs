use super::{MAX_BLOCK_INSTRUCTIONS};
use crate::opcode::{PrimaryOpcode, SpecialOpcode};

use super::{
    encoding::{ArithmeticOperation, Condition, GUEST_REGISTER_BYTES, GUEST_REGISTER_COUNT},
    operands::Operands,
};

mod encoding;

const CACHED_HOST_REGISTER_COUNT: usize = 4;
const FIRST_CACHED_HOST_REGISTER: u8 = 8; // Volatile R8..R11 on both native ABIs.
const SCALAR_CODE_CAPACITY: usize = 64;
const BLOCK_CODE_CAPACITY: usize = MAX_BLOCK_INSTRUCTIONS * 32;
const IMMEDIATE_MASK: u32 = u16::MAX as u32;
const IMMEDIATE_HIGH_SHIFT: u32 = u16::BITS;
const HIGH_SHIFT_OFFSET: u8 = u32::BITS as u8;
