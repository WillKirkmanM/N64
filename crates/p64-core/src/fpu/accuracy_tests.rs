use super::*;

#[test]
fn undocumented_control_register_transfers_preserve_defined_registers() {
    let mut n64 = N64::new();
    n64.set_cp0_register(MmuRegister::Status, MmuStatus::CU1 as u64);
    n64.fpu_mut().control.fcr31 = FCR31_CMP_BIT | 3;
    let identification = n64.fpu().control.fcr0;
    let control = n64.fpu().control.fcr31;

    for index in 1..31 {
        n64.cpu_mut().set_gpr(2, u64::MAX);
        ctc1(&mut n64, (2 << 16) | (index << 11));
        cfc1(&mut n64, (2 << 16) | (index << 11));
        assert_eq!(n64.cpu().gpr(2), 0);
        assert_eq!(n64.fpu().control.fcr0, identification);
        assert_eq!(n64.fpu().control.fcr31, control);
        assert!(!matches!(
            n64.cpu().branch_state.state,
            cpu::State::Exception
        ));
    }

    n64.cpu_mut().set_gpr(2, u64::MAX);
    ctc1(&mut n64, 2 << 16);
    cfc1(&mut n64, 2 << 16);
    assert_eq!(n64.cpu().gpr(2), u64::from(identification));
    n64.cpu_mut().set_gpr(2, u64::from(FCR31_CMP_BIT));
    ctc1(&mut n64, (2 << 16) | (31 << 11));
    cfc1(&mut n64, (2 << 16) | (31 << 11));
    assert_eq!(n64.cpu().gpr(2), u64::from(FCR31_CMP_BIT));
}

#[test]
fn undocumented_control_register_transfers_still_check_coprocessor_access() {
    for handler in [cfc1, ctc1] {
        let mut n64 = N64::new();
        n64.set_cp0_register(MmuRegister::Status, 0);
        n64.cpu_mut().set_gpr(2, 0x1234);
        handler(&mut n64, (2 << 16) | (1 << 11));
        assert_eq!(n64.cpu().gpr(2), 0x1234);
        assert_eq!(
            n64.cp0_register(MmuRegister::Cause),
            MmuCause::ExccodeCPU as u64 | MmuCause::CE1 as u64
        );
    }
}
