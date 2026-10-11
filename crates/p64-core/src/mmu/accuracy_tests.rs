use super::*;

fn step(n64: &mut N64, opcode: u32) {
    cpu::execute_instruction(n64, opcode, cpu::decode_opcode(n64, opcode));
    add_cycles(n64, 1);
}

#[test]
fn count_write_hazard_and_half_cycle_calibration_match_hardware() {
    let mut n64 = N64::default();
    cpu::start_cpu(&mut n64);

    for value in [0, 100, 0xFFFF_FFFF] {
        n64.cpu.registers.gpr[1] = value;
        step(&mut n64, (16 << 26) | (4 << 21) | (1 << 16) | (9 << 11));
        for register in 2..6 {
            step(&mut n64, (16 << 26) | (register << 16) | (9 << 11));
        }

        for register in 2..5 {
            assert_eq!(
                n64.cpu.registers.gpr[register],
                cpu::se32(value as u32 as i32)
            );
        }

        assert_eq!(
            n64.cpu.registers.gpr[5],
            cpu::se32((value as u32).wrapping_add(1) as i32)
        );

        step(&mut n64, (16 << 26) | (4 << 21) | (1 << 16) | (9 << 11));
        step(&mut n64, 0);
        step(&mut n64, 0);
        for register in 2..6 {
            step(&mut n64, (16 << 26) | (register << 16) | (9 << 11));
        }

        assert_eq!(
            (n64.cpu.registers.gpr[3] as u32).wrapping_sub(n64.cpu.registers.gpr[2] as u32),
            1
        );
        assert_eq!(
            (n64.cpu.registers.gpr[4] as u32).wrapping_sub(n64.cpu.registers.gpr[3] as u32),
            0
        );
        assert_eq!(
            (n64.cpu.registers.gpr[5] as u32).wrapping_sub(n64.cpu.registers.gpr[4] as u32),
            1
        );
    }
}

#[test]
fn wired_reset_has_two_instruction_hazard_and_random_ignores_stall_cycles() {
    let mut n64 = N64::default();
    cpu::start_cpu(&mut n64);

    for wired in 0..64 {
        n64.cpu.registers.gpr[1] = wired;
        step(&mut n64, (16 << 26) | (4 << 21) | (1 << 16) | (6 << 11));
        step(&mut n64, 0);
        step(&mut n64, 0);

        assert_eq!(n64.cp0_register(MmuRegister::Random), 31);
        add_cycles(&mut n64, 1000);

        assert_eq!(n64.cp0_register(MmuRegister::Random), 31);
        let mut expected = 31u64;

        for _ in 0..100 {
            step(&mut n64, 0);
            expected = if expected == wired {
                31
            } else {
                expected.wrapping_sub(1) & 63
            };

            assert_eq!(n64.cp0_register(MmuRegister::Random), expected);
        }
    }
}

#[test]
fn mtc0_entry_hi_keeps_upper_vpn_and_region_bits() {
    let mut n64 = N64::default();
    write_mmu_registers(&mut n64);
    n64.set_cp0_register(MmuRegister::Status, 0);
    n64.cpu.registers.gpr[1] = 0x1234_5678_FFFF_FFFF;
    mtc0(&mut n64, (1 << 16) | (10 << 11));

    assert_eq!(n64.cp0_register(MmuRegister::EntryHi), 0x78_FFFF_E0FF);
    n64.cpu.registers.gpr[1] = u64::MAX;
    mtc0(&mut n64, (1 << 16) | (10 << 11));

    assert_eq!(
        n64.cp0_register(MmuRegister::EntryHi),
        0xC000_00FF_FFFF_E0FF
    );
}
