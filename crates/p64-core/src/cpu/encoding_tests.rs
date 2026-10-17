use super::*;
use super::{alu::*, cache_ops::cache, control_flow::*, hilo::*, memory_ops::*};
use crate::{fpu, mmu, rcp};
use crate::mmu::MmuRegister;
use mmu::reserved;

// Raw positions are the NEC VR4300 opcode tables, independent of opcode enums.
const PRIMARY: [InstrFn; 64] = [
    reserved,
    reserved,
    j,
    jal,
    beq,
    bne,
    blez,
    bgtz,
    addi,
    addiu,
    slti,
    sltiu,
    andi,
    ori,
    xori,
    lui,
    reserved,
    reserved,
    reserved,
    reserved,
    beql,
    bnel,
    blezl,
    bgtzl,
    daddi,
    daddiu,
    ldl,
    ldr,
    reserved,
    reserved,
    reserved,
    reserved,
    lb,
    lh,
    lwl,
    lw,
    lbu,
    lhu,
    lwr,
    lwu,
    sb,
    sh,
    swl,
    sw,
    sdl,
    sdr,
    swr,
    cache,
    ll,
    fpu::lwc1,
    rcp::lwc2,
    reserved,
    lld,
    fpu::ldc1,
    rcp::ldc2,
    ld,
    sc,
    fpu::swc1,
    rcp::swc2,
    reserved,
    scd,
    fpu::sdc1,
    rcp::sdc2,
    sd,
];

const SPECIAL: [InstrFn; 64] = [
    sll,
    reserved,
    srl,
    sra,
    sllv,
    reserved,
    srlv,
    srav,
    jr,
    jalr,
    reserved,
    reserved,
    mmu::syscall,
    mmu::break_,
    reserved,
    sync,
    mfhi,
    mthi,
    mflo,
    mtlo,
    dsllv,
    reserved,
    dsrlv,
    dsrav,
    mult,
    multu,
    div,
    divu,
    dmult,
    dmultu,
    ddiv,
    ddivu,
    add,
    addu,
    sub,
    subu,
    and,
    or,
    xor,
    nor,
    reserved,
    reserved,
    slt,
    sltu,
    dadd,
    daddu,
    dsub,
    dsubu,
    traps::tge,
    traps::tgeu,
    traps::tlt,
    traps::tltu,
    traps::teq,
    reserved,
    traps::tne,
    reserved,
    dsll,
    reserved,
    dsrl,
    dsra,
    dsll32,
    reserved,
    dsrl32,
    dsra32,
];

const REGIMM: [InstrFn; 32] = [
    bltz,
    bgez,
    bltzl,
    bgezl,
    reserved,
    reserved,
    reserved,
    reserved,
    traps::tgei,
    traps::tgeiu,
    traps::tlti,
    traps::tltiu,
    traps::teqi,
    reserved,
    traps::tnei,
    reserved,
    bltzal,
    bgezal,
    bltzall,
    bgezall,
    reserved,
    reserved,
    reserved,
    reserved,
    reserved,
    reserved,
    reserved,
    reserved,
    reserved,
    reserved,
    reserved,
    reserved,
];

#[test]
fn every_integer_dispatch_slot_matches_the_independent_isa_table() {
    let mut machine = N64::new();
    start_cpu(&mut machine);
    for (table, expected) in [
        (&machine.cpu.instructions.instrs[..], &PRIMARY[..]),
        (&machine.cpu.instructions.special_instrs[..], &SPECIAL[..]),
        (&machine.cpu.instructions.regimm_instrs[..], &REGIMM[..]),
    ] {
        for (encoding, (&actual, &expected)) in table.iter().zip(expected).enumerate() {
            assert!(
                std::ptr::fn_addr_eq(actual, expected),
                "encoding {encoding:#x}"
            );
        }
    }
}

#[test]
fn every_reserved_integer_encoding_raises_ri_without_register_writes() {
    let mut machine = N64::new();
    start_cpu(&mut machine);
    let words = [0x13_u32, 0x1c, 0x1d, 0x1e, 0x1f, 0x33, 0x3b]
        .into_iter()
        .map(|raw| raw << 26)
        .chain([
            0x01, 0x05, 0x0a, 0x0b, 0x0e, 0x15, 0x28, 0x29, 0x35, 0x37, 0x39, 0x3d,
        ])
        .chain(
            [
                0x04_u32, 0x05, 0x06, 0x07, 0x0d, 0x0f, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a,
                0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
            ]
            .into_iter()
            .map(|raw| (1 << 26) | (raw << 16)),
        );
    for word in words {
        machine.set_cp0_register(MmuRegister::Status, 0);
        machine.set_cp0_register(MmuRegister::Cause, 0);
        machine.cpu.execution.pc = 0xffff_ffff_8000_1000;
        machine.cpu.branch_state.state = State::Step;
        machine.cpu.registers.gpr.fill(0x0123_4567_89ab_cdef);
        let registers = machine.cpu.registers.gpr;
        decode_opcode(&machine, word)(&mut machine, word);
        assert_eq!(
            machine.cp0_register(MmuRegister::Cause) & 0x3000_007c,
            10 << 2,
            "{word:#x}"
        );
        assert_eq!(machine.cpu.registers.gpr, registers);
        assert_eq!(
            machine.cp0_register(MmuRegister::EPC),
            0xffff_ffff_8000_1000
        );
    }
}

#[test]
fn width_guards_cover_every_extended_integer_instruction() {
    let primary = [
        0x18_u32, 0x19, 0x1a, 0x1b, 0x27, 0x2c, 0x2d, 0x34, 0x37, 0x3c, 0x3f,
    ];
    let special = [
        0x14_u32, 0x16, 0x17, 0x1c, 0x1d, 0x1e, 0x1f, 0x2c, 0x2d, 0x2e, 0x2f, 0x38, 0x3a, 0x3b,
        0x3c, 0x3e, 0x3f,
    ];
    let mut machine = N64::new();
    start_cpu(&mut machine);
    for word in primary.into_iter().map(|raw| raw << 26).chain(special) {
        for mode in [1_u64, 2] {
            machine.set_cp0_register(MmuRegister::Status, mode << 3);
            machine.set_cp0_register(MmuRegister::Cause, 0);
            machine.cpu.branch_state.state = State::Step;
            machine.cpu.execution.pc = 0x1000;
            decode_opcode(&machine, word)(&mut machine, word);
            assert_eq!(
                machine.cp0_register(MmuRegister::Cause) & 0x7c,
                10 << 2,
                "{word:#x}"
            );
        }
    }
}

#[test]
fn coprocessor_memory_operands_never_create_integer_load_dependencies() {
    for primary in [0x31_u32, 0x32, 0x35, 0x36, 0x39, 0x3a, 0x3d, 0x3e] {
        for base in 0..32 {
            for target in 0..32 {
                let instruction =
                    Instruction::decode((primary << 26) | (base << 21) | (target << 16));
                assert_eq!(instruction.interlock_read_mask(), 1 << base);
                assert_eq!(instruction.load_destination(), None);
                assert!(!instruction.requires_wide());
            }
        }
    }
}

#[test]
fn branch_and_jump_operands_wrap_at_address_boundaries() {
    for pc in [0_u64, u64::MAX - 3, 0xffff_ffff_8fff_fffc] {
        for immediate in [i16::MIN, -1, 0, 1, i16::MAX] {
            let instruction = Instruction::decode(immediate as u16 as u32);
            let expected = pc
                .wrapping_add(4)
                .wrapping_add((i64::from(immediate) * 4) as u64);
            assert_eq!(instruction.branch_target(pc), expected);
        }
        for index in [0_u32, 1, 0x03ff_ffff] {
            let expected = (pc.wrapping_add(4) & 0xffff_ffff_f000_0000) | (u64::from(index) * 4);
            assert_eq!(Instruction::decode(index).jump_target(pc), expected);
        }
    }
}
