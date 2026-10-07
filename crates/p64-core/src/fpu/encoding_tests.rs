use super::*;

fn supported_arithmetic(format: u32, function: u32) -> bool {
    match format {
        16 => matches!(function, 0..=15 | 0x21 | 0x24 | 0x25 | 0x30..=0x3f),
        17 => matches!(function, 0..=15 | 0x20 | 0x24 | 0x25 | 0x30..=0x3f),
        20 | 21 => matches!(function, 0x20 | 0x21),
        _ => false,
    }
}

#[test]
fn all_arithmetic_format_function_pairs_obey_the_independent_isa_matrix() {
    let mut machine = N64::new();
    for enabled in [false, true] {
        for format in 16..32 {
            for function in 0..64 {
                machine
                    .set_cp0_register(MmuRegister::Status, (1 << 26) | (u64::from(enabled) << 29));
                machine.set_cp0_register(MmuRegister::Cause, 0);
                machine.fpu_mut().control.fcr31 = 0;
                machine.fpu_mut().registers.fgr64.fill([0; 8]);
                machine.cpu.execution.pc = 0xffff_ffff_8000_1000;
                machine.cpu.branch_state.state = cpu::State::Step;
                let word = (0x11 << 26) | (format << 21) | (3 << 6) | function;
                cpu::decode_opcode(&machine, word)(&mut machine, word);
                let cause = machine.cp0_register(MmuRegister::Cause) & 0x3000_007c;
                if !enabled {
                    assert_eq!(cause, (11 << 2) | (1 << 28), "{word:#x}");
                } else if supported_arithmetic(format, function) {
                    assert_eq!(cause, 0, "{word:#x}");
                    assert!(!matches!(
                        machine.cpu.branch_state.state,
                        cpu::State::Exception
                    ));
                } else {
                    assert_eq!(cause & 0x7c, 15 << 2, "{word:#x}");
                    assert_eq!(
                        machine.fpu().control.fcr31 & FCR31_CAUSE_UNIMP_BIT,
                        FCR31_CAUSE_UNIMP_BIT
                    );
                }
            }
        }
    }
}
