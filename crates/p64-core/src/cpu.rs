mod alu;
mod backend;
mod cache_ops;
mod control_flow;
mod dispatch;
mod execution;
mod hilo;
mod instruction;
mod memory_ops;
mod native;
mod runtime;
mod traps;
mod types;

use crate::N64;
type InstrFn = fn(&mut N64, u32);

use instruction::gpr_read_mask;
pub(crate) use instruction::Instruction;
use memory_ops::translate_aligned_address;
use runtime::in_delay_slot_taken;

pub use control_flow::{check_idle_loop, check_relative_idle_loop};
pub use dispatch::{decode_opcode, start_cpu};
pub use execution::{execute_instruction, execute_prepared_instruction};
pub use instruction::{imm, rd, rs, rt, se16, se32};
pub use native::{execute_native_block, execute_native_branch_block, execute_native_memory_block};
pub use runtime::{in_delay_slot, run};
pub use types::{
    BranchState, Cpu, CpuExecutionState, CpuInstructionTables, CpuScheduler, IntegerRegisters,
    State,
};

pub(crate) use execution::execute_decoded_instruction;
pub(crate) use memory_ops::effective_address;
pub(crate) use native::{execute_native_branch_nop_block, native_branch_target};
