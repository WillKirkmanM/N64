use super::*;

fn finish_scheduled_slice(n64: &mut N64) {
    let event = n64.cpu.scheduler.events[EventType::SP as usize];
    assert!(event.enabled);
    n64.set_cp0_register(MmuRegister::Count, event.count);
    n64.cpu_mut().disable_event(EventType::SP);
    (event.handler)(n64);
    n64.cpu_mut().refresh_next_event();
}

#[test]
fn single_step_halts_at_the_scheduled_instruction_boundary() {
    let mut n64 = N64::new();
    start_rsp(&mut n64);
    n64.set_cp0_register(MmuRegister::Count, 100);
    for (address, opcode) in [(0, 0x2401_0001), (4, 0x2421_0001), (8, 13)] {
        write_mem(&mut n64, 0x1000 + address, opcode, u32::MAX);
    }
    write_regs(&mut n64, 16, SpStatusWrite::SetSstep as u32, u32::MAX);
    write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
    assert_eq!(n64.rsp.registers.gpr[1], 1);
    assert_eq!(n64.rsp.registers.pc(), 4);
    assert_eq!(
        n64.rsp.registers.regs[SpReg::Status as usize],
        SpStatus::Sstep as u32
    );
    assert!(!n64.rsp.processor.broken);
    assert_eq!(n64.cpu.scheduler.events[EventType::SP as usize].count, 101);

    finish_scheduled_slice(&mut n64);
    assert_eq!(
        n64.rsp.registers.regs[SpReg::Status as usize],
        SpStatus::Sstep as u32 | SpStatus::Halt as u32
    );
    assert!(!n64.cpu.scheduler.events[EventType::SP as usize].enabled);

    write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
    assert_eq!(n64.rsp.registers.gpr[1], 2);
    assert_eq!(n64.rsp.registers.pc(), 8);
    finish_scheduled_slice(&mut n64);
    write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
    finish_scheduled_slice(&mut n64);
    assert!(n64.rsp.processor.broken);
    assert_eq!(n64.rsp.registers.regs[SpReg::Status as usize] & 3, 3);
}

#[test]
fn single_step_preserves_taken_and_untaken_branch_delay_slots() {
    for taken in [false, true] {
        let mut n64 = N64::new();
        start_rsp(&mut n64);
        n64.rsp.registers.gpr[1] = u32::from(!taken);
        for (address, opcode) in [
            (0, (4 << 26) | (1 << 21) | 3),
            (4, 0x2402_0005),
            (8, 0x2403_0007),
            (16, 0x2403_0009),
            (20, 13),
        ] {
            write_mem(&mut n64, 0x1000 + address, opcode, u32::MAX);
        }
        write_regs(
            &mut n64,
            16,
            SpStatusWrite::SetSstep as u32 | SpStatusWrite::ClrHalt as u32,
            u32::MAX,
        );
        assert_eq!(n64.rsp.registers.pc(), 4);
        assert_eq!(n64.rsp.registers.gpr[2], 0);
        assert_eq!(
            n64.rsp.processor.branch_state.state,
            if taken {
                CpuState::DelaySlotTaken
            } else {
                CpuState::DelaySlotNotTaken
            }
        );
        finish_scheduled_slice(&mut n64);

        write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
        assert_eq!(n64.rsp.registers.gpr[2], 5);
        assert_eq!(n64.rsp.registers.pc(), if taken { 16 } else { 8 });
        assert_eq!(n64.rsp.processor.branch_state.state, CpuState::Step);
        finish_scheduled_slice(&mut n64);
        write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
        assert_eq!(n64.rsp.registers.gpr[3], if taken { 9 } else { 7 });
    }
}

#[test]
fn single_step_vector_issue_has_a_nonzero_deadline_without_break_interrupt() {
    let mut n64 = N64::new();
    start_rsp(&mut n64);
    write_mem(
        &mut n64,
        0x1000,
        (0x12 << 26) | (0x10 << 21) | 0x37,
        u32::MAX,
    );
    write_regs(
        &mut n64,
        16,
        SpStatusWrite::SetSstep as u32
            | SpStatusWrite::SetIntrBreak as u32
            | SpStatusWrite::ClrHalt as u32,
        u32::MAX,
    );
    assert_eq!(n64.rsp.registers.pc(), 4);
    assert_eq!(n64.rsp.processor.cycle_counter, 1);
    assert!(!n64.rsp.processor.pipeline_full);
    finish_scheduled_slice(&mut n64);
    assert!(!n64.rsp.processor.broken);
    assert_eq!(n64.mips.regs[2] & MI_INTR_SP, 0);
}

#[test]
fn third_dma_submission_replaces_only_the_pending_command() {
    let mut n64 = N64::new();
    n64.rdram.mem[0..32].fill(0x5a);
    write_regs(&mut n64, 0, 0, u32::MAX);
    write_regs(&mut n64, 4, 0, u32::MAX);
    write_regs(&mut n64, 8, 7, u32::MAX);
    let current = n64.rsp.memory.fifo[0];
    let deadline = n64.cpu.scheduler.events[EventType::SPDma as usize].count;

    write_regs(&mut n64, 0, 0x20, u32::MAX);
    write_regs(&mut n64, 4, 0x20, u32::MAX);
    write_regs(&mut n64, 8, 15, u32::MAX);
    write_regs(&mut n64, 0, 0x40, u32::MAX);
    write_regs(&mut n64, 4, 0x80, u32::MAX);
    write_regs(&mut n64, 12, 23, u32::MAX);

    assert_eq!(n64.rsp.memory.fifo[0].memaddr, current.memaddr);
    assert_eq!(n64.rsp.memory.fifo[0].dramaddr, current.dramaddr);
    assert_eq!(n64.rsp.memory.fifo[0].length, current.length);
    assert!(n64.rsp.memory.fifo[0].dir == current.dir);
    let pending = n64.rsp.memory.fifo[1];
    assert!(pending.dir == DmaDir::Read);
    assert_eq!(
        (pending.memaddr, pending.dramaddr, pending.length),
        (0x40, 0x80, 23)
    );
    assert_eq!(n64.rsp.registers.regs[SpReg::DmaFull as usize], 1);
    assert_eq!(
        n64.cpu.scheduler.events[EventType::SPDma as usize].count,
        deadline
    );
    n64.rsp.memory.mem[0x40..0x58].fill(0xa5);
    fifo_pop(&mut n64);
    assert_eq!(&n64.rdram.mem[0x80..0x98], &[0xa5; 24]);
    assert_eq!(n64.rsp.registers.regs[SpReg::DmaFull as usize], 0);
    assert_eq!(n64.rsp.registers.regs[SpReg::DmaBusy as usize], 1);
    fifo_pop(&mut n64);
    assert_eq!(n64.rsp.registers.regs[SpReg::DmaBusy as usize], 0);
}

#[test]
fn nonyielding_rsp_loop_returns_to_the_cpu_scheduler() {
    let mut n64 = N64::new();
    start_rsp(&mut n64);
    write_mem(&mut n64, 0x1000, 2 << 26, u32::MAX);
    write_mem(&mut n64, 0x1004, 0, u32::MAX);
    write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
    assert_eq!(n64.rsp.processor.cycle_counter, EXECUTION_SLICE_CYCLES);
    assert_eq!(
        n64.cpu.scheduler.events[EventType::SP as usize].count,
        EXECUTION_SLICE_CYCLES * 3 / 2
    );
    assert!(!n64.rsp.processor.running);
    assert!(!n64.rsp.processor.halted);
    assert_eq!(n64.rsp.registers.pc(), 4);
    finish_scheduled_slice(&mut n64);
    assert_eq!(n64.rsp.processor.cycle_counter, EXECUTION_SLICE_CYCLES + 1);
    assert_eq!(
        n64.cpu.scheduler.events[EventType::SP as usize].count,
        (EXECUTION_SLICE_CYCLES * 2 + 1) * 3 / 2
    );
    write_regs(&mut n64, 16, SpStatusWrite::SetHalt as u32, u32::MAX);
    assert!(!n64.cpu.scheduler.events[EventType::SP as usize].enabled);
}

#[test]
fn cpu_sp_subword_stores_replace_the_whole_shifted_word() {
    let mut n64 = N64::new();
    for (value, mask) in [(0x7800_0000, 0xff00_0000), (0x1234_5678, 0xffff)] {
        n64.rsp.memory.mem[0..4].fill(0xaa);
        write_mem(&mut n64, 0, value, mask);
        assert_eq!(&n64.rsp.memory.mem[0..4], &value.to_be_bytes());
    }
}

#[test]
fn dma_wraps_sp_bank_but_does_not_mirror_unpopulated_rdram_tail() {
    let mut n64 = N64::new();
    let bytes: [u8; 16] = core::array::from_fn(|i| (i * 13 + 7) as u8);
    let end = n64.rdram.mem.len() - 8;

    for offset in [0usize, 0x1000] {
        n64.rdram.mem[0..8].fill(0x5a);
        n64.rsp.memory.mem[offset + 0xff8..offset + 0x1000].copy_from_slice(&bytes[..8]);
        n64.rsp.memory.mem[offset..offset + 8].copy_from_slice(&bytes[8..]);
        do_dma(
            &mut n64,
            RspDma {
                dir: DmaDir::Read,
                length: 15,
                memaddr: (offset + 0xff8) as u32,
                dramaddr: end as u32,
            },
        );
        let canonical: [u8; 16] = core::array::from_fn(|i| bytes[(i & !3) + (3 - (i & 3))]);

        assert_eq!(&n64.rdram.mem[end..end + 8], &canonical[..8]);
        assert_eq!(&n64.rdram.mem[0..8], &[0x5a; 8]);
        n64.rsp.memory.mem[offset + 0xff8..offset + 0x1000].fill(0);
        n64.rsp.memory.mem[offset..offset + 8].fill(0);
        do_dma(
            &mut n64,
            RspDma {
                dir: DmaDir::Write,
                length: 15,
                memaddr: (offset + 0xff8) as u32,
                dramaddr: end as u32,
            },
        );

        assert_eq!(
            &n64.rsp.memory.mem[offset + 0xff8..offset + 0x1000],
            &bytes[..8]
        );
        assert_eq!(&n64.rsp.memory.mem[offset..offset + 8], &[0; 8]);

        if offset != 0 {
            assert_eq!(
                n64.rsp.instruction.instructions[0xff8 / 4].opcode,
                u32::from_be_bytes(bytes[..4].try_into().unwrap())
            );
            assert_eq!(n64.rsp.instruction.instructions[0].opcode, 0);
        }
    }
}

fn vector_opcode(element: u8) -> u32 {
    (1 << 21) | (2 << 16) | ((element as u32) << 7)
}

#[test]
fn vector_loads_ignore_high_address_bits_and_wrap_dmem() {
    let mut n64 = N64::new();

    for address in 0..0x1000 {
        n64.rsp.memory.mem[address] = address as u8;
    }
    // LDV crosses the end of DMEM, but never reads from IMEM.
    n64.rsp.memory.mem[0x1000..0x2000].fill(0xaa);
    n64.rsp.registers.gpr[1] = u32::MAX;
    ldv(&mut n64, vector_opcode(0));

    assert_eq!(
        n64.rsp.registers.vpr[2].to_be_bytes(),
        [0xff, 0, 1, 2, 3, 4, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0,]
    );

    let loads: [InstrFn; 11] = [lbv, lsv, llv, ldv, lqv, lrv, lpv, luv, lhv, lfv, ltv];

    for instruction in loads {
        for element in 0..16 {
            for low_address in 0xff0..=0xfff {
                n64.rsp.registers.vpr.fill(u128::MAX);
                n64.rsp.registers.gpr[1] = low_address;
                instruction(&mut n64, vector_opcode(element));
                let expected = n64.rsp.registers.vpr;
                n64.rsp.registers.vpr.fill(u128::MAX);
                n64.rsp.registers.gpr[1] = low_address | 0xffff_f000;
                instruction(&mut n64, vector_opcode(element));

                assert_eq!(n64.rsp.registers.vpr, expected);
            }
        }
    }
}

#[test]
fn cop0_read_stalls_its_immediate_consumer_until_the_third_cycle() {
    for (selector, expected) in [(1, 0x03d8), (4, 0), (7, 0), (11, 0x1234)] {
        let mut n64 = N64::new();
        start_rsp(&mut n64);
        n64.set_cp0_register(MmuRegister::Count, 100);
        n64.rsp.registers.regs[SpReg::DramAddr as usize] = 0x03d8;
        n64.rdp.registers.regs_dpc[rdp::DpcReg::Status as usize] = 0x1234;
        for (address, opcode) in [
            (0, 0x4001_0000 | (selector << 11)),
            (4, 0x2422_0001),
            (8, 13),
        ] {
            write_mem(&mut n64, 0x1000 + address, opcode, u32::MAX);
        }

        write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
        assert_eq!(n64.rsp.processor.cycle_counter, 1, "selector={selector}");
        assert_eq!(n64.rsp.registers.pc(), 4);
        assert_eq!(n64.rsp.registers.gpr[1], expected);
        assert_eq!(n64.rsp.registers.gpr[2], 0);
        assert_eq!(n64.cp0_register(MmuRegister::Count), 100);
        assert_eq!(n64.cpu.scheduler.events[EventType::SP as usize].count, 101);

        finish_scheduled_slice(&mut n64);
        assert_eq!(n64.rsp.registers.gpr[2], expected + 1);
        assert_eq!(n64.rsp.registers.pc(), 12);
        assert_eq!(n64.rsp.processor.cycle_counter, 4);
        assert!(n64.rsp.processor.broken);
    }
}

#[test]
fn cop0_yields_preserve_fractional_cpu_cycles_across_one_hundred_reads() {
    let mut n64 = N64::new();
    start_rsp(&mut n64);
    n64.set_cp0_register(MmuRegister::Count, 100);
    for instruction in 0..100 {
        for (offset, opcode) in [(0, 0x2402_0000), (4, 0), (8, 0x4001_2000)] {
            write_mem(
                &mut n64,
                0x1000 + instruction * 12 + offset,
                opcode,
                u32::MAX,
            );
        }
    }
    write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);

    let mut previous_deadline = 100;
    for retired in 1..=100 {
        let deadline = n64.cpu.scheduler.events[EventType::SP as usize].count;
        assert_eq!(n64.rsp.processor.cycle_counter, 3);
        assert_eq!(n64.rsp.registers.pc(), retired * 12);
        assert_eq!(deadline, 100 + u64::from(retired) * 9 / 2);
        assert_eq!(
            deadline - previous_deadline,
            if retired % 2 == 1 { 4 } else { 5 }
        );
        previous_deadline = deadline;
        if retired < 100 {
            finish_scheduled_slice(&mut n64);
        }
    }
    assert_eq!(previous_deadline - 100, 450);
}

#[test]
fn ipl3_semaphore_countdown_preserves_delay_slots_across_mmio_yields() {
    let mut n64 = N64::new();
    start_rsp(&mut n64);
    n64.set_cp0_register(MmuRegister::Count, 100);
    n64.rsp.registers.gpr[5] = 0x0020_0000;
    n64.rsp.registers.regs[SpReg::Semaphore as usize] = 1;
    for (address, opcode) in [
        (0, 0x04a0_0003),
        (4, 0x4003_3800),
        (8, 0x1460_fffd),
        (12, 0x20a5_ffff),
        (16, 13),
    ] {
        write_mem(&mut n64, 0x1000 + address, opcode, u32::MAX);
    }

    write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
    assert_eq!(n64.rsp.registers.pc(), 8);
    assert_eq!(n64.rsp.processor.cycle_counter, 2);
    assert_eq!(n64.cpu.scheduler.events[EventType::SP as usize].count, 103);

    for iteration in 1..=100 {
        finish_scheduled_slice(&mut n64);
        assert_eq!(n64.rsp.registers.pc(), 8);
        assert_eq!(n64.rsp.processor.cycle_counter, 7);
        assert_eq!(n64.rsp.registers.gpr[5], 0x0020_0000 - iteration);
        assert_eq!(
            n64.cpu.scheduler.events[EventType::SP as usize].count,
            103 + u64::from(iteration) * 21 / 2
        );
    }

    write_regs(&mut n64, 28, 0, u32::MAX);
    finish_scheduled_slice(&mut n64);
    assert_eq!(n64.rsp.registers.gpr[3], 0);
    finish_scheduled_slice(&mut n64);
    assert_eq!(n64.rsp.registers.gpr[5], 0x0020_0000 - 102);
    assert_eq!(n64.rsp.registers.pc(), 20);
    assert!(n64.rsp.processor.broken);
}
