use super::{
    compile_block, supported_memory_opcode, supported_opcode, NativeMemoryContext,
    NativeMemoryExit, MAX_BLOCK_INSTRUCTIONS,
};

use super::{
    encoding::{displacement, Condition, GUEST_INSTRUCTION_BYTES, GUEST_REGISTER_BYTES},
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

/// Emits aligned KSEG0 RAM cache hits, exiting before a failing access.
/// The caller checks privilege, endianness and events before resuming any side exit.
pub fn compile_memory_block(opcodes: &[u32]) -> Option<Vec<u8>> {
    if !cfg!(target_arch = "x86_64")
        || opcodes.len() < 2
        || opcodes.len() > MAX_BLOCK_INSTRUCTIONS
        || !opcodes
            .iter()
            .any(|opcode| supported_memory_opcode(*opcode))
        || opcodes
            .iter()
            .any(|opcode| !supported_opcode(*opcode) && !supported_memory_opcode(*opcode))
    {
        return None;
    }

    let line_stride = i8::try_from(size_of::<Line>()).ok()? as u8;
    let mut code = Vec::with_capacity(MEMORY_BLOCK_CODE_CAPACITY);
    // RBX is callee-saved on both native ABIs and holds the borrowed context.
    code.push(0x53);
    code.extend_from_slice(if cfg!(windows) {
        &[0x48, 0x89, 0xCB]
    } else {
        &[0x48, 0x89, 0xFB]
    });
    code.extend_from_slice(&[0x48, 0x8B, 0x93]);
    displacement(&mut code, GPR);
    let mut address_exits = Vec::new();
    let mut miss_exits = Vec::new();
    let mut cursor = 0;

    while cursor < opcodes.len() {
        let opcode = opcodes[cursor];

        if supported_opcode(opcode) {
            let start = cursor;

            while cursor < opcodes.len() && supported_opcode(opcodes[cursor]) {
                cursor += 1;
            }

            interlock(&mut code, opcode);
            let segment = compile_block(&opcodes[start..cursor])?;
            // Preserve RDX/RBX pointers; splice out the ALU argument setup and RET.
            code.extend_from_slice(&segment[ALU_ARGUMENT_SETUP_BYTES..segment.len() - 1]);
            retire(&mut code, cursor - start);
            code.extend_from_slice(&[0x48, 0x8B, 0x83]);
            displacement(&mut code, PC);
            code.extend_from_slice(&[
                0x48,
                0x83,
                0xC0,
                ((cursor - 1) * GUEST_INSTRUCTION_BYTES) as u8,
            ]);
            code.extend_from_slice(&[0x48, 0x89, 0x83]);
            displacement(&mut code, DATA);
            continue;
        }

        let operands = Operands::decode(opcode);
        let (rs, rt) = (operands.rs as u32, operands.rt as u32);
        let doubleword = matches!(
            operands.primary,
            Some(PrimaryOpcode::Ld | PrimaryOpcode::Sd)
        );
        load_gpr(&mut code, rs, false, false);
        let immediate = opcode as i16 as i32;
        code.extend_from_slice(&[0x48, 0x81, 0xC0]);
        code.extend_from_slice(&immediate.to_le_bytes());
        code.extend_from_slice(&[0x49, 0x89, 0xC2]); // r10 = effective virtual address
        code.extend_from_slice(&[0xA8, if doubleword { 7 } else { 3 }]);
        address_exits.push(jump(&mut code, Some(Condition::NotEqual)));
        // Canonical KSEG0 upper bits, including sign extension.
        code.extend_from_slice(&[
            0x49,
            0x89,
            0xC0,
            0x49,
            0xC1,
            0xE8,
            KSEG_ADDRESS_BITS,
            0x48,
            0xB9,
        ]);
        code.extend_from_slice(&(KSEG0_BASE >> KSEG_ADDRESS_BITS).to_le_bytes());
        code.extend_from_slice(&[0x49, 0x39, 0xC8]);
        address_exits.push(jump(&mut code, Some(Condition::NotEqual)));
        code.extend_from_slice(&[0x41, 0x89, 0xC0, 0x41, 0x81, 0xE0]);
        code.extend_from_slice(&PHYSICAL_ADDRESS_MASK.to_le_bytes());
        code.extend_from_slice(&[0x41, 0x81, 0xF8]);
        code.extend_from_slice(&(crate::rdram::RDRAM_SIZE as u32).to_le_bytes());
        address_exits.push(jump(&mut code, Some(Condition::AboveOrEqual)));
        // VIPT line index uses the effective virtual address.
        code.extend_from_slice(&[
            0x4D,
            0x89,
            0xD1,
            0x49,
            0xC1,
            0xE9,
            DCACHE_LINE_SHIFT,
            0x41,
            0x81,
            0xE1,
        ]);
        code.extend_from_slice(&DCACHE_INDEX_MASK.to_le_bytes());
        code.extend_from_slice(&[0x4D, 0x6B, 0xC9, line_stride]);
        code.extend_from_slice(&[0x4C, 0x8B, 0x9B]);
        displacement(&mut code, DCACHE);
        code.extend_from_slice(&[0x4D, 0x01, 0xD9, 0x41, 0x80, 0xB9]);
        displacement(&mut code, offset_of!(Line, valid));
        code.push(0);
        miss_exits.push(jump(&mut code, Some(Condition::Equal)));
        code.extend_from_slice(&[0x41, 0x8B, 0x89]);
        displacement(&mut code, offset_of!(Line, tag));
        code.extend_from_slice(&[0x81, 0xE1]);
        code.extend_from_slice(&DCACHE_TAG_ADDRESS_MASK.to_le_bytes());
        code.extend_from_slice(&[0x41, 0x81, 0xE0]);
        code.extend_from_slice(&DCACHE_PAGE_MASK.to_le_bytes());
        code.extend_from_slice(&[0x44, 0x39, 0xC1]);
        miss_exits.push(jump(&mut code, Some(Condition::NotEqual)));
        code.extend_from_slice(&[
            0x4D,
            0x89,
            0xD3,
            0x41,
            0x83,
            0xE3,
            if doubleword { 8 } else { 12 },
        ]);
        // Consume the pending load and interlock only after every address/cache guard passes.
        interlock(&mut code, opcode);
        if matches!(
            operands.primary,
            Some(PrimaryOpcode::Lw | PrimaryOpcode::Ld)
        ) {
            code.extend_from_slice(&[0x4B, if doubleword { 0x8B } else { 0x63 }, 0x84, 0x19]);
            displacement(&mut code, offset_of!(Line, words));
            // DCache words are semantic big-endian words even on a little-endian
            // host. Swap their 32-bit halves without reversing either word.
            if doubleword {
                code.extend_from_slice(&[0x48, 0xC1, 0xC0, 32]);
            }

            if rt != 0 {
                code.extend_from_slice(&[0x48, 0x89, 0x82]);
                displacement(&mut code, rt as usize * GUEST_REGISTER_BYTES);
            }

            code.extend_from_slice(&[0xC6, 0x83]);
            displacement(&mut code, PENDING);
            code.push(rt as u8);
        } else {
            load_gpr(&mut code, rt, true, !doubleword);
            if doubleword {
                code.extend_from_slice(&[0x48, 0xC1, 0xC1, 32]);
            }

            code.extend_from_slice(&[if doubleword { 0x4B } else { 0x43 }, 0x89, 0x8C, 0x19]);
            displacement(&mut code, offset_of!(Line, words));

            for offset in [offset_of!(Line, dirty), offset_of!(Line, store_dirty)] {
                code.extend_from_slice(&[0x41, 0xC6, 0x81]);
                displacement(&mut code, offset);
                code.push(1);
            }
        }

        retire(&mut code, 1);
        code.extend_from_slice(&[0x4C, 0x89, 0x93]);
        displacement(&mut code, DATA);
        cursor += 1;
    }

    let complete = jump(&mut code, None);
    let address_exit = code.len();
    code.extend_from_slice(&[0xC6, 0x83]);
    displacement(&mut code, EXIT);
    code.push(NativeMemoryExit::Address as u8);
    let address_done = jump(&mut code, None);
    let miss_exit = code.len();
    code.extend_from_slice(&[0xC6, 0x83]);
    displacement(&mut code, EXIT);
    code.push(NativeMemoryExit::CacheMiss as u8);
    let done = code.len();
    code.extend_from_slice(&[0x5B, 0xC3]); // pop rbx; ret
    for at in address_exits {
        patch(&mut code, at, address_exit);
    }

    for at in miss_exits {
        patch(&mut code, at, miss_exit);
    }

    patch(&mut code, complete, done);
    patch(&mut code, address_done, done);

    Some(code)
}
