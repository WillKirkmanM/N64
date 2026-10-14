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

#[test]
fn register_word_joins_preserve_raw_bits_in_both_fr_modes() {
    let mut n64 = N64::new();
    let mut bits = 0x8f01_2345_7f80_abcd_u64;

    for iteration in 0..1024 {
        bits ^= bits << 13;
        bits ^= bits >> 7;
        bits ^= bits << 17;
        let bytes = bits.to_le_bytes();
        let low: [u8; 4] = bytes[..4].try_into().unwrap();
        let high: [u8; 4] = bytes[4..].try_into().unwrap();
        let index = (iteration % 16) * 2;

        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0;
        n64.cpu.fpu.registers.fgr32[index] = low;
        n64.cpu.fpu.registers.fgr32[index + 1] = high;
        // FR=0 aliases odd double register indices to the even pair.
        assert_eq!(get_fpr_double(&n64, index).to_bits(), bits);
        assert_eq!(get_fpr_double(&n64, index + 1).to_bits(), bits);
        n64.fpu_mut().set_register_mode(true);

        assert_eq!(n64.cpu.fpu.registers.fgr64[index], bytes);
        n64.cpu.fpu.registers.fgr32[index] = [0; 4];
        n64.cpu.fpu.registers.fgr32[index + 1] = [0; 4];
        n64.fpu_mut().set_register_mode(false);

        assert_eq!(n64.cpu.fpu.registers.fgr32[index], low);
        assert_eq!(n64.cpu.fpu.registers.fgr32[index + 1], high);

        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = MmuStatus::FR as u64;

        for clear_high in [false, true] {
            n64.cpu.fpu.registers.fgr64[index + 1] = bytes;
            let single_bits = (bits >> 17) as u32;
            set_fpr_single(&mut n64, index + 1, f32::from_bits(single_bits), clear_high);
            let result = n64.cpu.fpu.registers.fgr64[index + 1];

            assert_eq!(&result[..4], &single_bits.to_le_bytes());
            assert_eq!(&result[4..], &if clear_high { [0; 4] } else { high });
            assert_eq!(n64.cpu.fpu.registers.fgr64[index], bytes);
        }
    }
}
