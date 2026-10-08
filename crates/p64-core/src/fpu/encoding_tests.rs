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

#[test]
fn cop1_transfer_and_branch_selector_space_preserves_exception_priority() {
    let mut machine = N64::new();
    for enabled in [false, true] {
        for selector in 0..16 {
            for target in 0..32 {
                machine
                    .set_cp0_register(MmuRegister::Status, (1 << 26) | (u64::from(enabled) << 29));
                machine.set_cp0_register(MmuRegister::Cause, 0);
                machine.fpu_mut().control.fcr31 = 0;
                machine.cpu.execution.pc = 0xffff_ffff_8000_1000;
                machine.cpu.branch_state.state = cpu::State::Step;
                machine.cpu.registers.gpr.fill(0);
                let word = (0x11 << 26) | (selector << 21) | (target << 16);
                cpu::decode_opcode(&machine, word)(&mut machine, word);
                let expected = if !enabled {
                    (11 << 2) | (1 << 28)
                } else if matches!(selector, 0..=2 | 4..=6) || (selector == 8 && target < 4) {
                    0
                } else {
                    15 << 2
                };
                assert_eq!(
                    machine.cp0_register(MmuRegister::Cause) & 0x3000_007c,
                    expected,
                    "{word:#x}"
                );
            }
        }
    }
}

#[test]
fn doubleword_fpu_moves_check_cu1_before_width_and_preserve_faulted_destinations() {
    let mut machine = N64::new();
    for mode in [0_u64, 1, 2] {
        for extended in [false, true] {
            for enabled in [false, true] {
                for selector in [1_u32, 5] {
                    let status = (mode << 3)
                        | (u64::from(extended) << (7 - mode))
                        | (1 << 26)
                        | (u64::from(enabled) << 29);
                    machine.set_cp0_register(MmuRegister::Status, status);
                    machine.set_cp0_register(MmuRegister::Cause, 0);
                    machine.cpu.execution.pc = 0x1000;
                    machine.cpu.branch_state.state = cpu::State::Step;
                    machine.cpu_mut().set_gpr(2, 0x1234_5678_9abc_def0);
                    machine.fpu_mut().registers.fgr64[4] = 0xdead_beef_cafe_babe_u64.to_le_bytes();
                    let word = (0x11 << 26) | (selector << 21) | (2 << 16) | (4 << 11);
                    cpu::decode_opcode(&machine, word)(&mut machine, word);
                    let expected = if !enabled {
                        (11 << 2) | (1 << 28)
                    } else if mode != 0 && !extended {
                        10 << 2
                    } else {
                        0
                    };
                    assert_eq!(
                        machine.cp0_register(MmuRegister::Cause) & 0x3000_007c,
                        expected
                    );
                    if expected != 0 {
                        assert_eq!(machine.cpu().gpr(2), 0x1234_5678_9abc_def0);
                        assert_eq!(
                            u64::from_le_bytes(machine.fpu().registers.fgr64[4]),
                            0xdead_beef_cafe_babe
                        );
                    } else if selector == 1 {
                        assert_eq!(machine.cpu().gpr(2), 0xdead_beef_cafe_babe);
                    } else {
                        assert_eq!(
                            u64::from_le_bytes(machine.fpu().registers.fgr64[4]),
                            0x1234_5678_9abc_def0
                        );
                    }
                }
            }
        }
    }
}
