use super::{supported_branch_opcode, supported_memory_opcode, supported_opcode, MAX_BLOCK_INSTRUCTIONS};
use crate::opcode::{PrimaryOpcode, SpecialOpcode};

use super::{encoding::{load_operand, set_condition, sign_extend_word, ArithmeticOperation, Condition, GUEST_REGISTER_BYTES, GUEST_REGISTER_COUNT}, operands::Operands};

mod encoding;
use encoding::{arithmetic_immediate, compare_result, load_guest, load_memory, mov_register, store_memory};

const CACHED_HOST_REGISTER_COUNT: usize = 4;
const FIRST_CACHED_HOST_REGISTER: u8 = 8; // Volatile R8..R11 on both native ABIs.
const SCALAR_CODE_CAPACITY: usize = 64;
const BLOCK_CODE_CAPACITY: usize = MAX_BLOCK_INSTRUCTIONS * 32;
const IMMEDIATE_MASK: u32 = u16::MAX as u32;
const IMMEDIATE_HIGH_SHIFT: u32 = u16::BITS;
const HIGH_SHIFT_OFFSET: u8 = u32::BITS as u8;

/// Generated leaves preserve callee-saved registers and write back modified GPRs.
pub fn compile_block(opcodes: &[u32]) -> Option<Vec<u8>> {
    const ARGUMENT_SETUP_BYTES: usize = 3;
    if opcodes.is_empty()
        || opcodes.len() > MAX_BLOCK_INSTRUCTIONS
        || opcodes.iter().any(|opcode| !supported_opcode(*opcode))
    {
        return None;
    }
    let mut code = Vec::with_capacity(BLOCK_CODE_CAPACITY);
    for (index, opcode) in opcodes.iter().copied().enumerate() {
        let mut leaf = compile_opcode(opcode)?;
        leaf.pop();
        let start = if index == 0 { 0 } else { ARGUMENT_SETUP_BYTES };
        code.extend_from_slice(&leaf[start..]);
    }
    code.push(0xc3);
    Some(code)
}

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
            load_operand(&mut code, rt, false);
            let variable = operands.is_variable_shift();
            let wide = operands.is_wide_shift();
            let arithmetic = matches!(
                function,
                Some(
                    SpecialOpcode::Sra
                        | SpecialOpcode::Srav
                        | SpecialOpcode::Dsrav
                        | SpecialOpcode::Dsra
                        | SpecialOpcode::Dsra32
                )
            );
            let left = matches!(
                function,
                Some(
                    SpecialOpcode::Sll
                        | SpecialOpcode::Sllv
                        | SpecialOpcode::Dsllv
                        | SpecialOpcode::Dsll
                        | SpecialOpcode::Dsll32
                )
            );
            let operation = if left {
                0xe0
            } else if arithmetic {
                0xf8
            } else {
                0xe8
            };

            if variable {
                load_operand(&mut code, rs, true);
                code.extend_from_slice(&[0x44, 0x89, 0xc1]); // ecx = r8d
                if !wide {
                    code.extend_from_slice(&[0x83, 0xe1, 0x1f]);
                }
            }
            // VR4300 word arithmetic shifts consume the full signed 64-bit
            // operand before truncating/sign extending the result.
            if wide || arithmetic {
                code.push(0x48);
            }

            code.extend_from_slice(&[if variable { 0xd3 } else { 0xc1 }, operation]);
            if !variable {
                code.push(
                    operands.shift
                        + if operands.is_high_shift() {
                            HIGH_SHIFT_OFFSET
                        } else {
                            0
                        },
                );
            }

            if !wide {
                sign_extend_word(&mut code);
            }
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
                Some(SpecialOpcode::Slt | SpecialOpcode::Sltu) => {
                    compare(&mut code, function == Some(SpecialOpcode::Sltu))
                }
                _ => unreachable!(),
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
            Some(PrimaryOpcode::Addiu) => {
                code.extend_from_slice(&[0x44, 0x01, 0xc0]);
                sign_extend_word(&mut code);
            }
            Some(PrimaryOpcode::Slti | PrimaryOpcode::Sltiu) => {
                compare(&mut code, primary == Some(PrimaryOpcode::Sltiu))
            }
            Some(PrimaryOpcode::Andi) => code.extend_from_slice(&[0x4c, 0x21, 0xc0]),
            Some(PrimaryOpcode::Ori) => code.extend_from_slice(&[0x4c, 0x09, 0xc0]),
            Some(PrimaryOpcode::Xori) => code.extend_from_slice(&[0x4c, 0x31, 0xc0]),
            Some(PrimaryOpcode::Lui) => {
                code.push(0xb8);
                code.extend_from_slice(
                    &((opcode & IMMEDIATE_MASK) << IMMEDIATE_HIGH_SHIFT).to_le_bytes(),
                );
                sign_extend_word(&mut code);
            }
            Some(PrimaryOpcode::Daddiu) => code.extend_from_slice(&[0x4c, 0x01, 0xc0]),
            _ => unreachable!(),
        }
    }

    if destination != 0 {
        code.extend_from_slice(&[0x48, 0x89, 0x82]);
        code.extend_from_slice(&((destination * GUEST_REGISTER_BYTES) as u32).to_le_bytes());
    }

    code.push(0xc3);

    Some(code)
}
