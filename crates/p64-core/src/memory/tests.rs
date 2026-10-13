use super::*;
use crate::mmu::MmuCause;
use crate::mmu::MmuRegister;

#[test]
fn speculative_instruction_translation_never_raises_or_mutates_a_fault() {
    let mut n64 = N64::default();
    n64.cpu.execution.data_address = 0x12345678;
    n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] = 0xABCD;
    n64.cpu.mmu.registers.regs[MmuRegister::BadVAddr as usize] = 0x98765432;
    for address in [
        0xFFFF_FFFF_8000_0001,
        0x8000_0000,
        0x0000_0000_1000_0000,
        0x9000_0001_0000_0000,
    ] {
        assert_eq!(peek_translation(&n64, address), None);
    }

    assert_eq!(
        peek_translation(&n64, 0xFFFF_FFFF_8000_0040),
        Some((64, true))
    );
    assert_eq!(
        peek_translation(&n64, 0xFFFF_FFFF_A000_0040),
        Some((64, false))
    );
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0x10;

    assert_eq!(peek_translation(&n64, 0xFFFF_FFFF_8000_0040), None);
    assert_eq!(n64.cpu.execution.data_address, 0x12345678);
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
        0xABCD
    );
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::BadVAddr as usize],
        0x98765432
    );
    assert!(matches!(
        n64.cpu.branch_state.state,
        crate::cpu::State::Step
    ));
}
