//! Native code is keyed by fetched opcodes. The CPU dispatcher handles timing,
//! privilege checks and exceptions.
mod emitter;
pub use emitter::{compile_block, compile_branch_block, compile_memory_block, compile_opcode};

#[cfg(target_arch = "x86_64")]
mod executable;
#[cfg(target_arch = "x86_64")]
use executable::ExecutablePage;

#[cfg(all(test, target_arch = "x86_64"))]
mod tests;

use crate::opcode::{PrimaryOpcode, SpecialOpcode};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_BLOCK_INSTRUCTIONS: usize = 8;
#[cfg(target_arch = "x86_64")]
const MAX_NATIVE_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_SCALAR_RECORDS: usize = 65_536;
pub(crate) const MAX_BLOCK_RECORDS: usize = 262_144;

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

fn next_generation() -> u64 {
    NEXT_GENERATION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .expect("Native engine generation exhausted")
}

mod engine;
mod types;
pub use engine::{Engine, ExecutionStats};
pub(crate) use types::NativeCodeRecord;
pub use types::NativeReject;
pub use types::{
    Backend, NativeBlock, NativeBranchBlock, NativeInstruction, NativeMemoryBlock,
    NativeMemoryContext, NativeMemoryExit,
};
use types::{BlockMetadata, NativeBranchFunction, NativeFunction, NativeMemoryFunction};

pub fn supported_opcode(opcode: u32) -> bool {
    match PrimaryOpcode::from_encoding(crate::cpu::Instruction::decode(opcode).primary() as usize) {
        Some(PrimaryOpcode::Special) => matches!(
            SpecialOpcode::from_encoding(crate::cpu::Instruction::decode(opcode).function()),
            Some(
                SpecialOpcode::Sll
                    | SpecialOpcode::Srl
                    | SpecialOpcode::Sra
                    | SpecialOpcode::Sllv
                    | SpecialOpcode::Srlv
                    | SpecialOpcode::Srav
                    | SpecialOpcode::Dsllv
                    | SpecialOpcode::Dsrlv
                    | SpecialOpcode::Dsrav
                    | SpecialOpcode::Addu
                    | SpecialOpcode::Subu
                    | SpecialOpcode::And
                    | SpecialOpcode::Or
                    | SpecialOpcode::Xor
                    | SpecialOpcode::Nor
                    | SpecialOpcode::Slt
                    | SpecialOpcode::Sltu
                    | SpecialOpcode::Daddu
                    | SpecialOpcode::Dsubu
                    | SpecialOpcode::Dsll
                    | SpecialOpcode::Dsrl
                    | SpecialOpcode::Dsra
                    | SpecialOpcode::Dsll32
                    | SpecialOpcode::Dsrl32
                    | SpecialOpcode::Dsra32
            )
        ),
        Some(
            PrimaryOpcode::Addiu
            | PrimaryOpcode::Slti
            | PrimaryOpcode::Sltiu
            | PrimaryOpcode::Andi
            | PrimaryOpcode::Ori
            | PrimaryOpcode::Xori
            | PrimaryOpcode::Lui
            | PrimaryOpcode::Daddiu,
        ) => true,
        _ => false,
    }
}

/// Nonlinking, nonlikely branches with an ordinary scalar delay slot.
pub fn supported_branch_opcode(opcode: u32) -> bool {
    matches!(
        PrimaryOpcode::from_encoding(crate::cpu::Instruction::decode(opcode).primary() as usize),
        Some(
            PrimaryOpcode::J
                | PrimaryOpcode::Beq
                | PrimaryOpcode::Bne
                | PrimaryOpcode::Blez
                | PrimaryOpcode::Bgtz
        )
    )
}

pub fn supported_memory_opcode(opcode: u32) -> bool {
    matches!(
        PrimaryOpcode::from_encoding(crate::cpu::Instruction::decode(opcode).primary() as usize),
        Some(PrimaryOpcode::Lw | PrimaryOpcode::Sw | PrimaryOpcode::Ld | PrimaryOpcode::Sd)
    )
}

fn is_wide_opcode(opcode: u32) -> bool {
    matches!(
        PrimaryOpcode::from_encoding(crate::cpu::Instruction::decode(opcode).primary() as usize),
        Some(PrimaryOpcode::Daddiu | PrimaryOpcode::Ld | PrimaryOpcode::Sd)
    ) || (PrimaryOpcode::from_encoding(crate::cpu::Instruction::decode(opcode).primary() as usize)
        == Some(PrimaryOpcode::Special)
        && matches!(
            SpecialOpcode::from_encoding(crate::cpu::Instruction::decode(opcode).function()),
            Some(
                SpecialOpcode::Dsllv
                    | SpecialOpcode::Dsrlv
                    | SpecialOpcode::Dsrav
                    | SpecialOpcode::Daddu
                    | SpecialOpcode::Dsubu
                    | SpecialOpcode::Dsll
                    | SpecialOpcode::Dsrl
                    | SpecialOpcode::Dsra
                    | SpecialOpcode::Dsll32
                    | SpecialOpcode::Dsrl32
                    | SpecialOpcode::Dsra32
            )
        ))
}
