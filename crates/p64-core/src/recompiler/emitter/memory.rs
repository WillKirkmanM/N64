use super::{NativeMemoryContext, NativeMemoryExit, MAX_BLOCK_INSTRUCTIONS};

use super::{
    encoding::{Condition, GUEST_INSTRUCTION_BYTES, GUEST_REGISTER_BYTES},
    operands::Operands,
};

use crate::opcode::PrimaryOpcode;
use std::mem::{offset_of, size_of};
const RELATIVE_JUMP_BYTES: usize = std::mem::size_of::<i32>();
const SHORT_DISPLACEMENT_MAX: usize = i8::MAX as usize;
const KSEG0_BASE: u64 = 0xFFFF_FFFF_8000_0000;
const KSEG_ADDRESS_BITS: u8 = 29;
const PHYSICAL_ADDRESS_MASK: u32 = 0x1FFF_FFFF;
const DCACHE_INDEX_MASK: u32 = 511;
const DCACHE_TAG_ADDRESS_MASK: u32 = 0x1FFF_FFFC;
const DCACHE_PAGE_MASK: u32 = 0xFFFF_F000;
const DCACHE_LINE_SHIFT: u8 = 4;
const ALU_ARGUMENT_SETUP_BYTES: usize = 3;
const MEMORY_BLOCK_CODE_CAPACITY: usize = MAX_BLOCK_INSTRUCTIONS * 256;

type Context = NativeMemoryContext<'static>;
type Line = crate::cache::DCache;
const GPR: usize = offset_of!(Context, gpr);
const DCACHE: usize = offset_of!(Context, dcache);
const PC: usize = offset_of!(Context, start_pc);
const RETIRED: usize = offset_of!(Context, retired_count);
const EXTRA: usize = offset_of!(Context, extra_cycles);
const DATA: usize = offset_of!(Context, last_data_address);
const PENDING: usize = offset_of!(Context, pending_load);
const EXIT: usize = offset_of!(Context, side_exit);
