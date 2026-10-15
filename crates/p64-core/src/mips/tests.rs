use super::*;

#[test]
fn asserting_rcp_interrupt_preserves_active_exception() {
    let mut n64 = N64::new();
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = crate::mmu::MmuStatus::EXL as u64;
    n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] = MmuCause::ExccodeSys as u64;
    n64.mips.regs[MI_INTR_MASK_REG as usize] = MI_INTR_SP;
    set_rcp_interrupt(&mut n64, MI_INTR_SP);

    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
        MmuCause::ExccodeSys as u64 | MmuCause::IP2 as u64
    );
    clear_rcp_interrupt(&mut n64, MI_INTR_SP);

    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
        MmuCause::ExccodeSys as u64
    );
}

#[test]
fn unmasking_rcp_interrupt_preserves_active_exception() {
    let mut n64 = N64::new();
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = crate::mmu::MmuStatus::EXL as u64;
    n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] = MmuCause::ExccodeBP as u64;
    set_rcp_interrupt(&mut n64, MI_INTR_SP);
    write_regs(&mut n64, (MI_INTR_MASK_REG * 4) as u64, MI_SET_SP, u32::MAX);

    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
        MmuCause::ExccodeBP as u64 | MmuCause::IP2 as u64
    );
}
