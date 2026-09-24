use tracing::info;

use crate::builder::registers::Registers;
use crate::fpu::Fpu;
use crate::rcp::Rcp;
use crate::{cache, events, exceptions, tlb};

use crate::mmu::{MMU_INSTRUCTIONS, MMU_SECONDARY_INSTRUCTIONS, write_mmu_registers, Mmu};
use crate::{mmu, fpu};

use super::mmu::{add_cycles, MmuRegister};
use super::events::EventType;
use super::{rcp, memory, N64};
use crate::cpu;

type InstrFn = for<'a> fn(&'a mut N64, u32);

const MAIN_OPS: [(usize, InstrFn); 48] = [
    (2, j),
    (3, jal),
    (4, beq),
    (5, bne),
    (6, blez),
    (7, bgtz),
    (8, addi),
    (9, addiu),
    (10, slti),
    (11, sltiu),
    (12, andi),
    (13, ori),
    (14, xori),
    (15, lui),
    (20, beql),
    (21, bnel),
    (22, blezl),
    (23, bgtzl),
    (24, daddi),
    (25, daddiu),
    (26, ldl),
    (27, ldr),
    (32, lb),
    (33, lh),
    (34, lwl),
    (35, lw),
    (36, lbu),
    (37, lhu),
    (38, lwr),
    (39, lwu),
    (40, sb),
    (41, sh),
    (42, swl),
    (43, sw),
    (44, sdl),
    (45, sdr),
    (46, swr),
    (47, cache),
    (48, ll),
    (49, fpu::lwc1),
    (52, lld),
    (53, fpu::ldc1),
    (55, ld),
    (56, sc),
    (57, fpu::swc1),
    (60, scd),
    (61, fpu::sdc1),
    (63, sd),
];

const SPECIAL_OPS: [(usize, InstrFn); 52] = [
    (0, sll),
    (2, srl),
    (3, sra),
    (4, sllv),
    (6, srlv),
    (7, srav),
    (8, jr),
    (9, jalr),
    (12, mmu::syscall),
    (13, mmu::break_),
    (15, sync),
    (16, mfhi),
    (17, mthi),
    (18, mflo),
    (19, mtlo),
    (20, dsllv),
    (22, dsrlv),
    (23, dsrav),
    (24, mult),
    (25, multu),
    (26, div),
    (27, divu),
    (28, dmult),
    (29, dmultu),
    (30, ddiv),
    (31, ddivu),
    (32, add),
    (33, addu),
    (34, sub),
    (35, subu),
    (36, and),
    (37, or),
    (38, xor),
    (39, nor),
    (42, slt),
    (43, sltu),
    (44, dadd),
    (45, daddu),
    (46, dsub),
    (47, dsubu),
    (48, tge),
    (49, tgeu),
    (50, tlt),
    (51, tltu),
    (52, teq),
    (54, tne),
    (56, dsll),
    (58, dsrl),
    (59, dsra),
    (60, dsll32),
    (62, dsrl32),
    (63, dsra32),
];

const REGIMM_OPS: [(usize, InstrFn); 14] = [
    (0, bltz),
    (1, bgez),
    (2, bltzl),
    (3, bgezl),
    (8, tgei),
    (9, tgeiu),
    (10, tlti),
    (11, tltiu),
    (12, teqi),
    (14, tnei),
    (16, bltzal),
    (17, bgezal),
    (18, bltzall),
    (19, bgezall),
];

pub fn rd(opcode: u32) -> u32 {
    (opcode >> 11) & 0x1F
}
pub fn rs(opcode: u32) -> u32 {
    (opcode >> 21) & 0x1F
}
pub fn rt(opcode: u32) -> u32 {
    (opcode >> 16) & 0x1F
}
fn sa(opcode: u32) -> u32 {
    (opcode >> 6) & 0x1F
}
pub fn imm(opcode: u32) -> u16 {
    opcode as u16
}
pub fn se32(value: i32) -> u64 {
    value as i64 as u64
}
pub fn se16(value: i16) -> u64 {
    value as i64 as u64
}
fn se8(value: i8) -> u64 {
    value as i64 as u64
}

fn bshift<T: Into<u64>>(address: T) -> u64 {
    ((address.into() & 3) ^ 3) << 3
}
fn hshift<T: Into<u64>>(address: T) -> u64 {
    ((address.into() & 2) ^ 2) << 3
}
fn bits_below_mask<T: Into<u64>>(x: T) -> u64 {
    (1 << x.into()) - 1
}
fn bits_above_mask<T: Into<u64>>(x: T) -> u64 {
    !bits_below_mask(x)
}

#[macro_export]
macro_rules! check_idle_loop {
    ($n64: expr, $condition:expr) => {
        if $condition
            && $n64.memory.fast_read[($n64.cpu.pc_phys >> 16) as usize](
                $n64,
                $n64.cpu.pc_phys + 4,
                memory::AccessSize::Word,
            ) == 0
        {
            $n64.cpu.mmu.regs[MmuRegister::Count as usize] = $n64.cpu.next_event_count
        }
    };
}

macro_rules! branch_instruction {
    ($name:ident, $condition:expr) => {
        fn $name(n64: &mut N64, opcode: u32) {
            if $condition(n64, opcode) {
                check_relative_idle_loop(n64, opcode);
                n64.cpu.branch_state.state = State::Take;
                n64.cpu.branch_state.pc =
                    n64.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
            } else {
                n64.cpu.branch_state.state = State::NotTaken;
            }
        }
    };
}

branch_instruction!(beq, |n64: &mut N64, opcode| n64.cpu.gpr
    [rs(opcode) as usize]
    == n64.cpu.gpr[rt(opcode) as usize]);
branch_instruction!(bne, |n64: &mut N64, opcode| n64.cpu.gpr
    [rs(opcode) as usize]
    != n64.cpu.gpr[rt(opcode) as usize]);
branch_instruction!(
    blez,
    |n64: &mut N64, opcode| n64.cpu.gpr[rs(opcode) as usize] as i64 <= 0
);
branch_instruction!(
    bgtz,
    |n64: &mut N64, opcode| n64.cpu.gpr[rs(opcode) as usize] as i64 > 0
);

macro_rules! impl_immediate_op {
    ($name:ident, $op:expr) => {
        fn $name(n64: &mut N64, opcode: u32) {
            n64.cpu.gpr[rt(opcode) as usize] = $op(n64.cpu.gpr[rs(opcode) as usize], imm(opcode));
        }
    };
}

impl_immediate_op!(addi, |rs, imm| se32(rs as i32)
    .wrapping_add(se16(imm as i16)));
impl_immediate_op!(addiu, |rs, imm| se32(rs as i32)
    .wrapping_add(se16(imm as i16)));
impl_immediate_op!(slti, |rs, imm| ((rs as i64) < (imm as i16 as i64)) as u64);
impl_immediate_op!(sltiu, |rs, imm| (rs < se16(imm as i16)) as u64);

macro_rules! cpu_arithmetic_op {
    ($name:ident, $op:tt) => {
        fn $name(n64: &mut N64, opcode: u32) {
            n64.cpu.gpr[rd(opcode) as usize] = se32(
                (n64.cpu.gpr[rs(opcode) as usize] as u32)
                    .$op(n64.cpu.gpr[rt(opcode) as usize] as u32) as i32,
            )
        }
    };
}

// Generate all arithmetic operations
cpu_arithmetic_op!(add, wrapping_add);
cpu_arithmetic_op!(addu, wrapping_add);
cpu_arithmetic_op!(sub, wrapping_sub);
cpu_arithmetic_op!(subu, wrapping_sub);

macro_rules! impl_bitwise_op {
    ($name:ident, $op:tt) => {
        fn $name(n64: &mut N64,opcode: u32) {
            n64.cpu.gpr[rd(opcode) as usize] =
                n64.cpu.gpr[rs(opcode) as usize] $op n64.cpu.gpr[rt(opcode) as usize]
        }
    };
}

impl_bitwise_op!(and, &);
impl_bitwise_op!(or, |);
impl_bitwise_op!(xor, ^);

// NOR needs special handling due to the NOT operation

macro_rules! def_d_arithmetic {
    ($name:ident, $op:tt) => {
        fn $name(n64: &mut N64, opcode: u32) {
            n64.cpu.gpr[rd(opcode) as usize] =
                n64.cpu.gpr[rs(opcode) as usize].$op(n64.cpu.gpr[rt(opcode) as usize])
        }
    };
}

// Define arithmetic instructions
def_d_arithmetic!(dadd, wrapping_add);
def_d_arithmetic!(daddu, wrapping_add);
def_d_arithmetic!(dsub, wrapping_sub);
def_d_arithmetic!(dsubu, wrapping_sub);

macro_rules! def_trap_reg {
    ($name:ident, $op:tt) => {
        fn $name(n64: &mut N64,opcode: u32) {
            if (n64.cpu.gpr[rs(opcode) as usize] as i64) $op (n64.cpu.gpr[rt(opcode) as usize] as i64) {
                exceptions::trap_exception(n64)
            }
        }
    };
    ($name:ident, $op:tt, unsigned) => {
        fn $name(n64: &mut N64,opcode: u32) {
            if n64.cpu.gpr[rs(opcode) as usize] $op n64.cpu.gpr[rt(opcode) as usize] {
                exceptions::trap_exception(n64)
            }
        }
    };
}

// Define trap instructions using a macro
def_trap_reg!(tge, >=);
def_trap_reg!(tgeu, >=, unsigned);
def_trap_reg!(tlt, <);
def_trap_reg!(tltu, <, unsigned);
def_trap_reg!(teq, ==, unsigned);
def_trap_reg!(tne, !=, unsigned);

macro_rules! branch_instruction {
    ($name:ident, $cmp:expr, $not_taken_state:expr) => {
        fn $name(n64: &mut N64, opcode: u32) {
            if $cmp(n64.cpu.gpr[rs(opcode) as usize] as i64) {
                check_relative_idle_loop(n64, opcode);
                n64.cpu.branch_state.state = State::Take;
                n64.cpu.branch_state.pc =
                    n64.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
            } else {
                n64.cpu.branch_state.state = $not_taken_state;
            }
        }
    };
}

// Define branch instructions using a macro
branch_instruction!(bltz, |x| x < 0, State::NotTaken);
branch_instruction!(bgez, |x| x >= 0, State::NotTaken);
branch_instruction!(bltzl, |x| x < 0, State::Discard);
branch_instruction!(bgezl, |x| x >= 0, State::Discard);

macro_rules! def_trap_imm {
    ($name:ident, $op:tt) => {
        fn $name(n64: &mut N64,opcode: u32) {
            if (n64.cpu.gpr[rs(opcode) as usize] as i64) $op (imm(opcode) as i16 as i64) {
                exceptions::trap_exception(n64)
            }
        }
    };
    ($name:ident, $op:tt, unsigned) => {
        fn $name(n64: &mut N64,opcode: u32) {
            if n64.cpu.gpr[rs(opcode) as usize] $op se16(imm(opcode) as i16) {
                exceptions::trap_exception(n64)
            }
        }
    };
}

// Define all trap immediate instructions
def_trap_imm!(tgei, >=);
def_trap_imm!(tgeiu, >=, unsigned);
def_trap_imm!(tlti, <);
def_trap_imm!(tltiu, <, unsigned);
def_trap_imm!(teqi, ==);
def_trap_imm!(tnei, !=);

macro_rules! impl_branch_and_link {
    ($name:ident, $condition:expr, $discard_on_false:literal) => {
        fn $name(n64: &mut N64, opcode: u32) {
            let rs_val = n64.cpu.gpr[rs(opcode) as usize] as i64;
            let in_delay_slot = cpu::in_delay_slot(n64);

            if $condition(rs_val) {
                check_relative_idle_loop(n64, opcode);
                n64.cpu.branch_state.state = State::Take;
                n64.cpu.branch_state.pc =
                    n64.cpu.pc.wrapping_add(se16(imm(opcode) as i16) << 2) + 4;
            } else {
                n64.cpu.branch_state.state = if $discard_on_false {
                    State::Discard
                } else {
                    State::NotTaken
                };
            }

            n64.cpu.gpr[31] = if in_delay_slot {
                n64.cpu.branch_state.pc + 4
            } else {
                n64.cpu.pc + 8
            };
        }
    };
}

// Define the branch conditions
const BLTZ: fn(i64) -> bool = |x| x < 0;
const BGEZ: fn(i64) -> bool = |x| x >= 0;

// Implement the instructions
impl_branch_and_link!(bltzal, BLTZ, false);
impl_branch_and_link!(bgezal, BGEZ, false);
impl_branch_and_link!(bltzall, BLTZ, true);
impl_branch_and_link!(bgezall, BGEZ, true);

mod instruction;
pub(crate) use instruction::Instruction;

mod alu;
use alu::{
    andi, ori, xori, lui, daddi, daddiu, sll, srl, sra, sllv, srlv, srav, dsllv, dsrlv, dsrav, nor,
    slt, sltu, dsll, dsrl, dsra, dsll32, dsrl32, dsra32,
};

mod dispatch;
pub use dispatch::{decode_opcode, start_cpu};

mod types;
pub use types::{State, BranchState, Cpu};

mod cache_ops;
use cache_ops::{cache};

mod runtime;
pub use runtime::{in_delay_slot, run};
use runtime::{in_delay_slot_taken};

mod control_flow;
pub use control_flow::{check_relative_idle_loop};
use control_flow::{check_absolute_idle_loop, j, jal, beql, bnel, blezl, bgtzl, jr, jalr};

mod hilo;
use hilo::{sync, mfhi, mthi, mflo, mtlo, div, divu, mult, multu, dmult, dmultu, ddiv, ddivu};

mod memory_ops;
use memory_ops::{
    ldl, ldr, lb, lh, lwl, lw, lbu, lhu, lwr, lwu, sb, sh, swl, sw, sdl, sdr, swr, ll, lld, ld, sc,
    scd, sd,
};
