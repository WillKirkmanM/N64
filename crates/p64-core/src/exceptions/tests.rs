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
