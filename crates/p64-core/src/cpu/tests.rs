use super::*;
use super::{alu::*, cache_ops::cache, control_flow::*, hilo::*, memory_ops::*};
use crate::memory;
use crate::mmu::MmuRegister;
use crate::mmu::{MmuCause, MmuStatus};

fn register_opcode(destination: u32) -> u32 {
    (1 << 21) | (2 << 16) | (destination << 11)
}

#[test]
fn integer_interlock_compares_unused_rs_rt_and_i_type_destinations() {
    let mut n64 = N64::default();
    start_cpu(&mut n64);
    let cases = [
        ((8 << 26) | (1 << 21) | (6 << 16), true), // ADDI writes loaded rt
        ((6 << 21) | (1 << 6), true),              // SLL with unused rs matching load
        ((6 << 16) | 18, true),                    // MFLO with unused rt matching load
        ((6 << 11) | 18, false),                   // R-type rd does not interlock
        ((49 << 26) | (1 << 21) | (6 << 16), false), // LWC1 ft is not a GPR
    ];

    for (opcode, stalls) in cases {
        n64.cpu.execution.pending_gpr_load = 6;
        n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] = 0;
        execute_instruction(&mut n64, opcode, sync);

        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::Count as usize],
            stalls as u64
        );
    }
}

#[test]
fn annulled_likely_slot_counts_a_cycle_and_clears_load_hazard() {
    let mut n64 = N64::default();
    start_cpu(&mut n64);
    n64.cpu.registers.gpr[1] = 1;
    let opcode = (20 << 26) | (1 << 21) | 1;
    let instruction = decode_opcode(&n64, opcode);
    execute_instruction(&mut n64, opcode, instruction);

    assert_eq!(n64.cpu.mmu.registers.regs[MmuRegister::Count as usize], 1);
    assert_eq!(n64.cpu.mmu.clock.instruction_clock, 2);
    assert_eq!(n64.cpu.execution.pending_gpr_load, 0);
    assert!(matches!(n64.cpu.branch_state.state, State::Discard));
}

#[test]
fn word_multiply_uses_signed_35_bit_rt_and_full_rs() {
    let mut n64 = N64::default();

    for (lhs, rhs, expected_hi, expected_lo) in [
        (0x0000_0000_FFFF_FFFF, 0x10, 0xF, 0xFFFF_FFFF_FFFF_FFF0),
        (0x0000_00EE_FFFF_FFFF, 0x10, 0xEEF, 0xFFFF_FFFF_FFFF_FFF0),
        (
            0x10,
            0x0000_00EE_FFFF_FFFF,
            0xFFFF_FFFF_FFFF_FFEF,
            0xFFFF_FFFF_FFFF_FFF0,
        ),
        (1, 0x0000_0004_0000_0000, 0xFFFF_FFFF_FFFF_FFFC, 0),
    ] {
        n64.cpu.registers.gpr[1] = lhs;
        n64.cpu.registers.gpr[2] = rhs;
        mult(&mut n64, register_opcode(3));

        assert_eq!(n64.cpu.registers.hi, expected_hi);
        assert_eq!(n64.cpu.registers.lo, expected_lo);
    }
}

#[test]
fn successful_sc_does_not_clear_the_reservation() {
    let mut n64 = N64::default();
    reset_data_access(&mut n64, 0);
    sc(&mut n64, (1 << 21) | (2 << 16));

    assert!(n64.cpu.registers.llbit);
    n64.cpu.registers.gpr[2] = 0x1234_5678_9ABC_DEF0;
    scd(&mut n64, (1 << 21) | (2 << 16));

    assert_eq!(n64.cpu.registers.gpr[2], 1);
    assert_eq!(n64.cpu.registers.gpr[8], 0x1234_5678);
    assert_eq!(n64.cpu.registers.gpr[9], 0x9ABC_DEF0);
    assert!(n64.cpu.registers.llbit);
}

#[test]
fn wide_instruction_guard_is_checked_at_execution_time() {
    let mut n64 = N64::default();
    start_cpu(&mut n64);
    let opcode = (1 << 21) | (2 << 16) | (3 << 11) | 0x2D;
    let instruction = decode_opcode(&n64, opcode);
    n64.cpu.registers.gpr[1] = 1;
    n64.cpu.registers.gpr[2] = 2;
    n64.cpu.registers.gpr[3] = 0xBADDECAF;
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0x10;
    instruction(&mut n64, opcode);

    assert_eq!(n64.cpu.registers.gpr[3], 0xBADDECAF);
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
        MmuCause::ExccodeRI as u64
    );
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0x30;
    instruction(&mut n64, opcode);

    assert_eq!(n64.cpu.registers.gpr[3], 3);
}

fn software_interrupt_program(_n64: &mut N64, address: u64, _size: memory::AccessSize) -> u32 {
    match address {
        0 => (16 << 26) | (4 << 21) | (1 << 16) | (13 << 11),
        4 => 0,
        0x1FC0_0380 => (9 << 26) | (4 << 21) | (4 << 16) | 1,
        0x1FC0_0384 => 63 << 26,
        _ => panic!("Interrupt must be taken after the next instruction"),
    }
}

#[test]
fn software_interrupt_retires_next_instruction_and_enters_handler_once() {
    let mut n64 = N64::default();
    start_cpu(&mut n64);
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = MmuStatus::BEV as u64 | 0x101;
    n64.cpu.execution.pc = 0xFFFF_FFFF_A000_0000;
    n64.cpu.registers.gpr[1] = 0x100;
    n64.memory.bus.memory_map_read[0] = software_interrupt_program;
    n64.memory.bus.memory_map_read[0x1FC0] = software_interrupt_program;
    n64.cpu.instructions.instrs[63] = halt;
    run(&mut n64);

    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::EPC as usize],
        0xFFFF_FFFF_A000_0008
    );
    assert_eq!(n64.cpu.registers.gpr[4], 1);
}

fn data_read(n64: &mut N64, address: u64, _size: memory::AccessSize) -> u32 {
    n64.cpu.registers.gpr[10] += 1;

    n64.cpu.registers.gpr[8 + ((address & 4) >> 2) as usize] as u32
}

fn data_write(n64: &mut N64, address: u64, value: u32, mask: u32) {
    n64.cpu.registers.gpr[10] += 1;
    let register = &mut n64.cpu.registers.gpr[8 + ((address & 4) >> 2) as usize];
    *register = ((*register as u32 & !mask) | (value & mask)) as u64;
}

fn reset_data_access(n64: &mut N64, offset: u64) {
    n64.cpu.execution.pc = 0xFFFF_FFFF_8000_1004;
    n64.cpu.branch_state.state = State::Step;
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0;
    n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] = 0;
    n64.cpu.registers.gpr[1] = 0xFFFF_FFFF_A000_0000 + offset;
    n64.cpu.registers.gpr[2] = 0x1122_3344_5566_7788;
    n64.cpu.registers.gpr[8] = 0x89AB_CDEF;
    n64.cpu.registers.gpr[9] = 0x1234_5678;
    n64.cpu.registers.gpr[10] = 0;
    n64.cpu.registers.llbit = true;
    n64.cpu.mmu.registers.regs[MmuRegister::LLAddr as usize] = 0xDEAD;
    n64.memory.bus.memory_map_read[0] = data_read;
    n64.memory.bus.memory_map_write[0] = data_write;
}

fn setup_reverse_data(n64: &mut N64, offset: u64, cached: bool) {
    reset_data_access(n64, 0);
    n64.cpu.registers.gpr[1] = 0x2000 + offset;
    n64.cpu.registers.gpr[8] = 0x0807_0605;
    n64.cpu.registers.gpr[9] = 0x0403_0201;
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0x0200_0030;
    n64.cpu.mmu.translations.tlb_entries[1] = crate::tlb::TlbEntry {
        hi: crate::tlb::EntryHi::decode(0x2000),
        global: true,
        pages: [
            crate::tlb::TlbPage {
                valid: true,
                dirty: true,
                coherency: if cached { 3 } else { 2 },
                ..Default::default()
            },
            crate::tlb::TlbPage::default(),
        ],
        ..Default::default()
    };
    n64.memory.caches.dcache.fill(Default::default());
    crate::cache::init(n64);
    n64.memory.bus.memory_map_read[0] = reverse_data_read;
    n64.memory.bus.memory_map_write[0] = reverse_data_write;
}

fn reverse_data_read(n64: &mut N64, address: u64, size: memory::AccessSize) -> u32 {
    if address & 15 >= 8 {
        0
    } else {
        data_read(n64, address, size)
    }
}

fn reverse_data_write(n64: &mut N64, address: u64, value: u32, mask: u32) {
    if address & 15 < 8 {
        data_write(n64, address, value, mask)
    }
}

#[test]
fn reverse_endian_user_loads_select_physical_bus_lanes() {
    let mut n64 = N64::default();
    let cases: [(InstrFn, u64); 9] = [
        (lb, 1),
        (lbu, 1),
        (lh, 0x0201),
        (lhu, 0x0201),
        (lw, 0x0403_0201),
        (lwu, 0x0403_0201),
        (ll, 0x0403_0201),
        (ld, 0x0807_0605_0403_0201),
        (lld, 0x0807_0605_0403_0201),
    ];

    for cached in [false, true] {
        for (instruction, expected) in cases {
            setup_reverse_data(&mut n64, 0, cached);
            instruction(&mut n64, (1 << 21) | (2 << 16));

            assert_eq!(n64.cpu.registers.gpr[2], expected);
            assert_eq!(n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize], 0);
        }
    }
}

fn reverse_fetch_program(_n64: &mut N64, address: u64, _size: memory::AccessSize) -> u32 {
    match address {
        0 => 0,
        4 => (9 << 26) | (3 << 21) | (3 << 16) | 1,
        8 => 12,
        12 => (9 << 26) | (3 << 21) | (3 << 16) | 4,
        0x1FC0_0380 => 63 << 26,
        _ => 0,
    }
}

fn unaligned_fetch_program(_n64: &mut N64, address: u64, _size: memory::AccessSize) -> u32 {
    match address {
        0 => (1 << 21) | 8, // JR $1
        4 => (13 << 26) | (3 << 21) | (3 << 16) | 2,
        0x1FC0_0380 => (9 << 26) | (4 << 21) | (4 << 16) | 1,
        0x1FC0_0384 => 63 << 26,
        _ => panic!("Unaligned address must fault before instruction fetch"),
    }
}

fn nested_branch_program(_n64: &mut N64, address: u64, _size: memory::AccessSize) -> u32 {
    match address {
        0 => (2 << 26) | 8, // J 0x20
        4 => (4 << 26) | 2, // BEQ $0, $0, +2 (nested target 0x28)
        0x20 => (13 << 26) | (3 << 21) | (3 << 16) | 1,
        0x24 => (13 << 26) | (3 << 21) | (3 << 16) | 2,
        0x28 => (13 << 26) | (3 << 21) | (3 << 16) | 4,
        0x2C => 63 << 26,
        _ => 0,
    }
}

fn halt(n64: &mut N64, _opcode: u32) {
    n64.cpu.execution.running = 0;
}
