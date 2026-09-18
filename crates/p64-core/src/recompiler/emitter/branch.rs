use super::{supported_branch_opcode, supported_opcode, MAX_BLOCK_INSTRUCTIONS};
use crate::opcode::PrimaryOpcode;

use super::{
    encoding::{load_operand, set_condition, Condition},
    operands::Operands,
};
