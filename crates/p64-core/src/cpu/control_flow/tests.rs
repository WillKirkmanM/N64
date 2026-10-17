use super::{bgezal, bltzall, jalr};
use crate::{cpu::State, N64};

const PC: u64 = 0xffff_ffff_8000_1000;
const TARGET: u64 = 0xffff_ffff_8000_2000;

#[test]
fn jalr_reads_an_overlapping_source_before_writing_the_link() {
    let mut n64 = N64::default();
    n64.cpu.execution.pc = PC;
    n64.cpu.registers.gpr[31] = TARGET;
    jalr(&mut n64, (31 << 21) | (31 << 11) | 9);

    assert_eq!(n64.cpu.branch_state.pc, TARGET);
    assert_eq!(n64.cpu.registers.gpr[31], PC + 8);
    assert!(matches!(n64.cpu.branch_state.state, State::Take));

    n64.cpu.execution.pc = PC;
    n64.cpu.branch_state.state = State::DelaySlotTaken;
    n64.cpu.branch_state.pc = TARGET;
    n64.cpu.registers.gpr[31] = 0;
    jalr(&mut n64, (31 << 21) | (31 << 11) | 9);

    assert_eq!(n64.cpu.branch_state.pc, TARGET);
    assert_eq!(n64.cpu.registers.gpr[31], TARGET + 4);
    assert!(matches!(n64.cpu.branch_state.state, State::DelaySlotTaken));
}

#[test]
fn regimm_links_after_testing_r31_even_when_a_likely_branch_is_untaken() {
    let mut n64 = N64::default();
    n64.cpu.execution.pc = PC;
    n64.cpu.registers.gpr[31] = 0;
    bgezal(&mut n64, (31 << 21) | 1);

    assert_eq!(n64.cpu.branch_state.pc, PC + 8);
    assert_eq!(n64.cpu.registers.gpr[31], PC + 8);
    assert!(matches!(n64.cpu.branch_state.state, State::Take));

    n64.cpu.execution.pc = PC;
    n64.cpu.branch_state.state = State::Step;
    n64.cpu.registers.gpr[31] = 0;
    bltzall(&mut n64, (31 << 21) | 1);

    assert_eq!(n64.cpu.registers.gpr[31], PC + 8);
    assert!(matches!(n64.cpu.branch_state.state, State::Discard));
}
