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
