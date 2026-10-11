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

#[test]
fn cop0_usability_guard_precedes_register_access() {
    let mut n64 = N64::default();
    n64.set_cp0_register(MmuRegister::Status, 0x10);
    n64.cpu.registers.gpr[1] = 0xBADDECAF;
    execute_checked_cp0(&mut n64, (16 << 26) | (1 << 16) | (12 << 11));

    assert_eq!(n64.cpu.registers.gpr[1], 0xBADDECAF);
    assert_eq!(
        n64.cp0_register(MmuRegister::Cause),
        MmuCause::ExccodeCPU as u64
    );
}

#[test]
fn mtc0_transfers_all_source_bits_and_tag_registers_follow_hardware_masks() {
    let mut n64 = N64::default();
    write_mmu_registers(&mut n64);
    n64.cpu.registers.gpr[1] = 0xF123_4567_89AB_CDEF;
    for register in [
        MmuRegister::Context,
        MmuRegister::XContext,
        MmuRegister::EPC,
    ] {
        n64.set_cp0_register(register, 0);
        mtc0(&mut n64, (1 << 16) | ((register as u32) << 11));

        assert_eq!(
            n64.cp0_register(register),
            n64.cpu.registers.gpr[1] & n64.cp0().registers.reg_write_masks[register as usize]
        );
    }

    mtc0(&mut n64, (1 << 16) | (7 << 11));

    assert_eq!(get_control_registers(&n64, 7), n64.cpu.registers.gpr[1]);
    dmtc0(&mut n64, (1 << 16) | (28 << 11));

    assert_eq!(get_control_registers(&n64, 28), 0x89AB_CDEF);
    dmtc0(&mut n64, (1 << 16) | (29 << 11));

    assert_eq!(get_control_registers(&n64, 29), 0);
    assert_eq!(get_control_registers(&n64, 28), 0x89AB_CDEF);
}

#[test]
fn writing_count_recomputes_compare_without_acknowledging_interrupt() {
    let mut n64 = N64::default();
    n64.set_cp0_register(MmuRegister::Count, 20);
    n64.set_cp0_register(MmuRegister::Compare, 100);
    schedule_compare(&mut n64, 100);
    create_event(&mut n64, EventType::PI, 60, events::dummy_event);
    n64.set_cp0_register(MmuRegister::Cause, MmuCause::IP7 as u64);
    set_control_registers(&mut n64, MmuRegister::Count as u32, 90);

    assert_eq!(
        n64.cpu.scheduler.events[EventType::Compare as usize].count,
        202
    );
    assert_eq!(n64.cpu.scheduler.events[EventType::PI as usize].count, 220);
    assert_eq!(n64.cp0_register(MmuRegister::Cause), MmuCause::IP7 as u64);
    set_control_registers(&mut n64, MmuRegister::Count as u32, 101);

    assert_eq!(
        n64.cpu.scheduler.events[EventType::Compare as usize].count,
        204 + ((u32::MAX as u64) << 1)
    );
}

#[test]
fn compare_match_uses_the_next_count_tick_from_an_odd_cpu_cycle() {
    let mut n64 = N64::default();
    n64.set_cp0_register(MmuRegister::Count, 101);
    schedule_compare(&mut n64, 51);

    assert_eq!(
        n64.cpu.scheduler.events[EventType::Compare as usize].count,
        102
    );
}

#[test]
fn reserved_cp0_co_functions_are_noops_for_every_rs_encoding() {
    let mut n64 = N64::default();
    n64.cpu.execution.pc = 0xFFFF_FFFF_8000_1000;
    n64.cpu.registers.gpr[8] = 0x1234;
    n64.set_cp0_register(MmuRegister::EntryHi, 0x1234_00AB);

    for rs in 16..32 {
        for function in 0..64 {
            if [1, 2, 6, 8, 16, 24].contains(&function) {
                continue;
            }

            let opcode = (16 << 26) | (rs << 21) | function;
            let instruction = cpu::decode_opcode(&n64, opcode);
            instruction(&mut n64, opcode);

            assert_eq!(n64.cpu.execution.pc, 0xFFFF_FFFF_8000_1000);
            assert_eq!(n64.cpu.registers.gpr[8], 0x1234);
            assert_eq!(n64.cp0_register(MmuRegister::EntryHi), 0x1234_00AB);
            assert_eq!(n64.cp0_register(MmuRegister::Cause), 0);
            assert_eq!(n64.cp0_register(MmuRegister::Count), 0);
            assert!(matches!(n64.cpu.branch_state.state, State::Step));
        }
    }
}

#[test]
fn cp0_rfe_raises_reserved_instruction_for_every_co_encoding() {
    let mut n64 = N64::default();

    for rs in 16..32 {
        n64.cpu.execution.pc = 0xFFFF_FFFF_8000_1000;
        n64.cpu.branch_state.state = State::Step;
        n64.set_cp0_register(MmuRegister::Status, 0);
        let opcode = (16 << 26) | (rs << 21) | 0x10;
        cpu::decode_opcode(&n64, opcode)(&mut n64, opcode);

        assert_eq!(
            n64.cp0_register(MmuRegister::Cause),
            MmuCause::ExccodeRI as u64
        );
        assert_eq!(n64.cp0_register(MmuRegister::EPC), 0xFFFF_FFFF_8000_1000);
        assert!(matches!(n64.cpu.branch_state.state, State::Exception));
    }
}
