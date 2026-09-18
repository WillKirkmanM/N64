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

/// Emits a nontrapping instruction over a GPR array, without embedded addresses or CPU offsets.
pub fn compile_opcode(opcode: u32) -> Option<Vec<u8>> {
    if !cfg!(target_arch = "x86_64") || !supported_opcode(opcode) {
        return None;
    }

    let operands = Operands::decode(opcode);
    let (rs, rt) = (operands.rs, operands.rt);
    let (primary, function) = (operands.primary, operands.function);
    let destination = operands.destination();
    let mut code = Vec::with_capacity(SCALAR_CODE_CAPACITY);
    // Keep the GPR pointer in volatile RDX, leaving RCX for shift counts.
    code.extend_from_slice(if cfg!(windows) {
        &[0x48, 0x89, 0xca]
    } else {
        &[0x48, 0x89, 0xfa]
    });
    let compare = |code: &mut Vec<u8>, unsigned: bool| {
        code.extend_from_slice(&[0x4c, 0x39, 0xc0]);
        set_condition(
            code,
            if unsigned {
                Condition::Below
            } else {
                Condition::Less
            },
        );
    };

    if primary == Some(PrimaryOpcode::Special) {
        if operands.is_shift() {
            return None;
        } else {
            load_operand(&mut code, rs, false);
            load_operand(&mut code, rt, true);

            match function {
                Some(SpecialOpcode::Addu | SpecialOpcode::Subu) => {
                    code.extend_from_slice(&[
                        0x44,
                        if function == Some(SpecialOpcode::Addu) {
                            0x01
                        } else {
                            0x29
                        },
                        0xc0,
                    ]);
                    sign_extend_word(&mut code);
                }
                Some(SpecialOpcode::Daddu | SpecialOpcode::Dsubu) => code.extend_from_slice(&[
                    0x4c,
                    if function == Some(SpecialOpcode::Daddu) {
                        0x01
                    } else {
                        0x29
                    },
                    0xc0,
                ]),
                Some(SpecialOpcode::And) => code.extend_from_slice(&[0x4c, 0x21, 0xc0]),
                Some(SpecialOpcode::Or | SpecialOpcode::Nor) => {
                    code.extend_from_slice(&[0x4c, 0x09, 0xc0]);

                    if function == Some(SpecialOpcode::Nor) {
                        code.extend_from_slice(&[0x48, 0xf7, 0xd0]);
                    }
                }
                Some(SpecialOpcode::Xor) => code.extend_from_slice(&[0x4c, 0x31, 0xc0]),

                _ => return None,
            }
        }
    } else {
        load_operand(&mut code, rs, false);
        let immediate = if matches!(
            primary,
            Some(PrimaryOpcode::Andi | PrimaryOpcode::Ori | PrimaryOpcode::Xori)
        ) {
            (opcode & IMMEDIATE_MASK) as u64
        } else {
            opcode as i16 as i64 as u64
        };

        code.extend_from_slice(&[0x49, 0xb8]);
        code.extend_from_slice(&immediate.to_le_bytes());

        match primary {
            _ => return None,
        }
    }

    if destination != 0 {
        code.extend_from_slice(&[0x48, 0x89, 0x82]);
        code.extend_from_slice(&((destination * GUEST_REGISTER_BYTES) as u32).to_le_bytes());
    }

    code.push(0xc3);

    Some(code)
}
