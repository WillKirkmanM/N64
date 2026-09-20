use super::{supported_branch_opcode, supported_memory_opcode, supported_opcode, NativeMemoryContext, NativeMemoryExit, MAX_BLOCK_INSTRUCTIONS};

mod branch;
mod encoding;
mod integer;
mod memory;
mod operands;

pub use branch::compile_branch_block;
pub use integer::{compile_block, compile_opcode};

