use super::{compile_block, supported_memory_opcode, supported_opcode, NativeMemoryContext, NativeMemoryExit, MAX_BLOCK_INSTRUCTIONS};

use super::{encoding::{displacement, Condition, GUEST_INSTRUCTION_BYTES, GUEST_REGISTER_BYTES}, operands::Operands};

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

fn jump(code: &mut Vec<u8>, condition: Option<Condition>) -> usize {
    if let Some(condition) = condition {
        code.extend_from_slice(&[0x0F, condition.jump_opcode()]);
    } else {
        code.push(0xE9);
    }

    let patch = code.len();
    code.extend_from_slice(&[0; RELATIVE_JUMP_BYTES]);

    patch
}

fn patch(code: &mut [u8], at: usize, target: usize) {
    code[at..at + RELATIVE_JUMP_BYTES].copy_from_slice(
        &((target as isize - (at + RELATIVE_JUMP_BYTES) as isize) as i32).to_le_bytes(),
    );
}

fn load_gpr(code: &mut Vec<u8>, guest: u32, rcx: bool, word: bool) {
    if guest == 0 {
        code.extend_from_slice(if rcx { &[0x31, 0xC9] } else { &[0x31, 0xC0] });
    } else {
        if !word {
            code.push(0x48);
        }

        code.extend_from_slice(&[0x8B, if rcx { 0x8A } else { 0x82 }]);
        displacement(code, guest as usize * GUEST_REGISTER_BYTES);
    }
}

fn interlock(code: &mut Vec<u8>, opcode: u32) {
    let operands = Operands::decode(opcode);
    let (rs, rt) = (operands.rs as u8, operands.rt as u8);
    code.extend_from_slice(&[0x0F, 0xB6, 0x83]);
    displacement(code, PENDING);
    code.extend_from_slice(&[0x85, 0xC0]);
    let zero = jump(code, Some(Condition::Equal));
    code.extend_from_slice(&[0x83, 0xF8, rs]);
    let rs_match = jump(code, Some(Condition::Equal));
    code.extend_from_slice(&[0x83, 0xF8, rt]);
    let mismatch = jump(code, Some(Condition::NotEqual));
    let increment = code.len();

    if EXTRA <= SHORT_DISPLACEMENT_MAX {
        code.extend_from_slice(&[0x48, 0x83, 0x43, EXTRA as u8, 1]);
    } else {
        code.extend_from_slice(&[0x48, 0x83, 0x83]);
        displacement(code, EXTRA);
        code.push(1);
    }

    let clear = code.len();

    if PENDING <= SHORT_DISPLACEMENT_MAX {
        code.extend_from_slice(&[0xC6, 0x43, PENDING as u8, 0]);
    } else {
        code.extend_from_slice(&[0xC6, 0x83]);
        displacement(code, PENDING);
        code.push(0);
    }

    patch(code, zero, clear);
    patch(code, rs_match, increment);
    patch(code, mismatch, clear);
}

fn retire(code: &mut Vec<u8>, count: usize) {
    if RETIRED <= SHORT_DISPLACEMENT_MAX {
        code.extend_from_slice(&[0x48, 0x83, 0x43, RETIRED as u8, count as u8]);
    } else {
        code.extend_from_slice(&[0x48, 0x83, 0x83]);
        displacement(code, RETIRED);
        code.push(count as u8);
    }
}

