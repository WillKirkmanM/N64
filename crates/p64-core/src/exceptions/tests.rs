use super::*;

#[test]
fn synchronous_exception_preserves_pending_interrupts() {
    let mut n64 = N64::new();
    n64.cpu.execution.pc = 0xFFFFFFFF80001234;
    let pending = MmuCause::IP2 as u64 | MmuCause::IP7 as u64 | (1 << 8);
    n64.set_cp0_register(MmuRegister::Cause, pending | MmuCause::CE2 as u64);
    syscall_exception(&mut n64);

    assert_eq!(
        n64.cp0_register(MmuRegister::Cause),
        pending | MmuCause::ExccodeSys as u64
    );
    assert_eq!(n64.cp0_register(MmuRegister::EPC), 0xFFFFFFFF80001234);
}

#[test]
fn delay_slot_exception_wraps_branch_address() {
    let mut n64 = N64::new();
    n64.cpu.execution.pc = 0;
    n64.cpu.branch_state.state = State::DelaySlotTaken;
    syscall_exception(&mut n64);

    assert_eq!(n64.cp0_register(MmuRegister::EPC), u64::MAX - 3);
    assert_ne!(
        n64.cp0_register(MmuRegister::Cause) & MmuCause::BD as u64,
        0
    );
}

#[test]
fn nested_exception_preserves_epc_and_branch_delay() {
    let mut n64 = N64::new();
    n64.set_cp0_register(MmuRegister::Status, MmuStatus::EXL as u64);
    n64.set_cp0_register(
        MmuRegister::Cause,
        MmuCause::BD as u64 | MmuCause::IP7 as u64,
    );
    n64.set_cp0_register(MmuRegister::EPC, 0xFFFFFFFF80002000);
    break_exception(&mut n64);

    assert_eq!(
        n64.cp0_register(MmuRegister::Cause),
        MmuCause::BD as u64 | MmuCause::IP7 as u64 | MmuCause::ExccodeBP as u64
    );
    assert_eq!(n64.cp0_register(MmuRegister::EPC), 0xFFFFFFFF80002000);
}

#[test]
fn interrupt_entry_clears_old_exception_code() {
    let mut n64 = N64::new();
    n64.set_cp0_register(
        MmuRegister::Status,
        MmuStatus::IE as u64 | MmuCause::IP7 as u64,
    );
    n64.set_cp0_register(
        MmuRegister::Cause,
        MmuCause::IP7 as u64 | MmuCause::ExccodeSys as u64,
    );
    check_pending_interrupts(&mut n64);

    assert_eq!(n64.cp0_register(MmuRegister::Cause), MmuCause::IP7 as u64);
    assert_ne!(
        n64.cp0_register(MmuRegister::Status) & MmuStatus::EXL as u64,
        0
    );
}

#[test]
fn context_update_preserves_pte_base() {
    // nemu64-test's Context::from_virtual_address uses only bits 4..22,
    // whereas XContext's BadVPN2 field extends through bit 30.
    let mut n64 = N64::new();
    let context_base = 0xFEDCBA987F800000;
    let xcontext_base = 0xFEDCBA9800000000;
    let address = 0xC00000ABCDE12345;
    n64.set_cp0_register(MmuRegister::Context, context_base);
    n64.set_cp0_register(MmuRegister::XContext, xcontext_base);
    update_context_registers(&mut n64, address);

    assert_eq!(
        n64.cp0_register(MmuRegister::Context),
        context_base | ((address >> 9) & 0x007FFFF0)
    );
    assert_eq!(
        n64.cp0_register(MmuRegister::XContext),
        xcontext_base
            | ((address >> 9) & BadVPN2Mask as u64)
            | ((address >> 31) & RegionMask as u64)
    );
}

#[test]
fn address_error_updates_fault_context_and_entry_hi() {
    for (access, expected_cause) in [
        (AccessType::Read, MmuCause::ExccodeAdEL),
        (AccessType::Write, MmuCause::ExccodeAdES),
    ] {
        let mut n64 = N64::new();
        let address = 0xFFFFFFFF81234562;
        let context_base = 0xFFFFFFFFFF800000;
        let xcontext_base = 0xFFFFFFFE00000000;
        n64.set_cp0_register(MmuRegister::Context, context_base);
        n64.set_cp0_register(MmuRegister::XContext, xcontext_base);
        n64.set_cp0_register(MmuRegister::EntryHi, 0x1234405A);
        n64.set_cp0_register(MmuRegister::Cause, MmuCause::IP7 as u64);
        address_error_exception(&mut n64, address, access);

        assert_eq!(n64.cp0_register(MmuRegister::BadVAddr), address);
        assert_eq!(
            n64.cp0_register(MmuRegister::Cause),
            MmuCause::IP7 as u64 | expected_cause as u64
        );
        assert_eq!(
            n64.cp0_register(MmuRegister::Context),
            context_base | ((address >> 9) & 0x007FFFF0)
        );
        assert_eq!(
            n64.cp0_register(MmuRegister::XContext),
            xcontext_base
                | ((address >> 9) & BadVPN2Mask as u64)
                | ((address >> 31) & RegionMask as u64)
        );
        assert_eq!(
            n64.cp0_register(MmuRegister::EntryHi),
            (address & crate::mmu::MmuMask::EntryHi as u64 & !0xFF) | 0x5A
        );
        assert_eq!(n64.cpu.execution.pc, 0xFFFFFFFF80000180);
    }
}

#[test]
fn tlb_exception_preserves_pending_interrupts() {
    let mut n64 = N64::new();
    n64.set_cp0_register(MmuRegister::Cause, MmuCause::IP7 as u64);
    tlb_miss_exception(&mut n64, 0x12345000, AccessType::Read);

    assert_eq!(
        n64.cp0_register(MmuRegister::Cause),
        MmuCause::IP7 as u64 | MmuCause::ExccodeTLBL as u64
    );
    assert_eq!(n64.cpu.execution.pc, 0xFFFFFFFF80000000);
}
