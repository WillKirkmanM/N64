use super::{compile_block, supported_branch_opcode, supported_opcode, MAX_BLOCK_INSTRUCTIONS};
use crate::opcode::PrimaryOpcode;

use super::{encoding::{load_operand, set_condition, Condition}, operands::Operands};

/// Returns the branch condition after the ALU prefix; the dispatcher handles targets
/// and delay slots.
pub fn compile_branch_block(opcodes: &[u32]) -> Option<Vec<u8>> {
    if !cfg!(target_arch = "x86_64") || opcodes.len() < 2 || opcodes.len() > MAX_BLOCK_INSTRUCTIONS
    {
        return None;
    }

    let branch = *opcodes.last()?;
    let prefix = &opcodes[..opcodes.len() - 1];

    if !supported_branch_opcode(branch) || prefix.iter().any(|opcode| !supported_opcode(*opcode)) {
        return None;
    }

    let mut code = compile_block(prefix)?;
    code.pop(); // Keep the prefix writeback epilogue, omitting only RET.
    let operands = Operands::decode(branch);
    let (primary, rs, rt) = (operands.primary, operands.rs, operands.rt);

    match primary {
        Some(PrimaryOpcode::J) => code.extend_from_slice(&[0xB8, 1, 0, 0, 0]),
        Some(PrimaryOpcode::Beq | PrimaryOpcode::Bne) => {
            load_operand(&mut code, rs, false);
            load_operand(&mut code, rt, true);
            code.extend_from_slice(&[0x4c, 0x39, 0xc0]);
            let condition = if primary == Some(PrimaryOpcode::Beq) {
                Condition::Equal
            } else {
                Condition::NotEqual
            };

            set_condition(&mut code, condition);
        }
        Some(PrimaryOpcode::Blez | PrimaryOpcode::Bgtz) => {
            load_operand(&mut code, rs, false);
            code.extend_from_slice(&[0x48, 0x83, 0xf8, 0]);
            let condition = if primary == Some(PrimaryOpcode::Blez) {
                Condition::LessOrEqual
            } else {
                Condition::Greater
            };

            set_condition(&mut code, condition);
        }
        _ => unreachable!(),
    }

    code.push(0xC3);

    Some(code)
}
