#[cfg(target_arch = "x86_64")]
mod support;
#[cfg(target_arch = "x86_64")]
mod control {
    use p64_core::{aot, cache, cpu, memory, mmu::MmuRegister, recompiler::Engine, N64};

    use crate::support::run_until;

    #[derive(Debug, PartialEq)]
    struct Snapshot {
        gpr: [u64; 32],
        cp0: Vec<u64>,
        pc: u64,
        physical: u64,
        data: u64,
        branch: (u8, u64),
        clock: u64,
        pending: u8,
        fpu: (Option<u8>, Option<u8>),
        icache: Vec<(bool, u32, u16, [u32; 8])>,
    }

    fn snapshot(n64: &N64) -> Snapshot {
        let branch = match n64.cpu.branch_state.state {
            cpu::State::Step => 0,
            cpu::State::Take => 1,
            cpu::State::NotTaken => 2,
            cpu::State::DelaySlotTaken => 3,
            cpu::State::DelaySlotNotTaken => 4,
            cpu::State::Discard => 5,
            cpu::State::Exception => 6,
        };

        Snapshot {
            gpr: n64.cpu.registers.gpr,
            cp0: n64.cpu.mmu.registers.regs.regs.clone(),
            pc: n64.cpu.execution.pc,
            physical: n64.cpu.execution.pc_phys,
            data: n64.cpu.execution.data_address,
            branch: (branch, n64.cpu.branch_state.pc),
            clock: n64.cpu.mmu.clock.instruction_clock,
            pending: n64.cpu.execution.pending_gpr_load,
            fpu: (
                n64.cpu.fpu.timing.previous_write,
                n64.cpu.fpu.timing.pending_write,
            ),
            icache: n64
                .memory
                .caches
                .icache
                .iter()
                .map(|line| (line.valid, line.tag, line.index, line.words))
                .collect(),
        }
    }

    fn program(primary: u32, offset: usize, prefix: usize, nop_slot: bool) -> Vec<u32> {
        let mut words = vec![0; 64];

        for index in 0..prefix {
            words[offset + index] = if index + 1 == prefix {
                if prefix == 1 {
                    0x24420001
                } else {
                    0x30220001
                } // r2 increment / r1 parity
            } else {
                0x24210001
            };
        }

        let branch = offset + prefix;
        let target = 48;
        words[branch] = if primary == 2 {
            0x08000000 | target as u32
        } else {
            (primary << 26) | (2 << 21) | ((target - branch - 1) as u32 & 65535)
        };
        words[branch + 1] = if nop_slot { 0 } else { 0x24630001 };
        words[branch + 2] = 0x26940003; // untaken path
        words[branch + 3] = 0x08000000 | offset as u32;
        words[branch + 4] = 0x24a50001;
        words[target] = 0x26940007; // taken path
        words[target + 1] = 0x08000000 | offset as u32;
        words[target + 2] = 0x24c60001;

        words
    }

    fn device(
        words: &[u32],
        offset: usize,
        engine: Engine,
        wired: u64,
        pending: u8,
        warm: bool,
    ) -> N64 {
        let mut n64 = N64::new();
        n64.cartridge.rom = p64_core::builder::memory::Memory::new()
            .with_size(4)
            .build();
        memory::initialise_memory_map(&mut n64);
        cpu::start_cpu(&mut n64);
        cache::init(&mut n64);
        for (index, &word) in words.iter().enumerate() {
            p64_core::rdram::write_mem(&mut n64, index as u64 * 4, word, u32::MAX);
        }
        n64.memory.fetch_pipeline.enabled = true;
        n64.cpu.execution.pc = 0xffff_ffff_8000_0000 + offset as u64 * 4;
        n64.cpu.branch_state.state = cpu::State::Step;
        n64.cpu.branch_state.pc = 0x12345678;
        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0;
        if warm {
            // Warm both sequential lookahead lines and the potential branch target.
            for index in [offset / 8, offset / 8 + 1, 6] {
                cache::icache_fill(&mut n64, index, index as u64 * 32);
            }
        }
        n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] = 0;
        n64.cpu.mmu.registers.regs[MmuRegister::Wired as usize] = wired;
        n64.cpu.mmu.registers.regs[MmuRegister::Random as usize] = 31;
        n64.cpu.mmu.clock.instruction_clock = 0;
        n64.cpu.execution.pending_gpr_load = pending;
        n64.cpu.registers.gpr[2] = u64::MAX - 1; // prefix1 visits negative, zero, then positive
        n64.cpu.fpu.timing.previous_write = Some(7);
        n64.cpu.fpu.timing.pending_write = Some(3);
        n64.execution_engine = engine;

        n64
    }

    #[test]
    fn control_blocks_match_fetch_delay_slots_and_event_resume() {
        let mut native_calls = 0;

        for primary in [2, 4, 5, 6, 7] {
            for offset in [0, 3, 6, 7] {
                for prefix in [1, 2, 5, 7] {
                    for nop_slot in [false, true] {
                        let words = program(primary, offset, prefix, nop_slot);
                        let (rom, artifact) = crate::support::artifact_for(&words);

                        for deadline in [43, 51, 97, 160, 600] {
                            let wired = match deadline {
                                43 | 600 => 33,
                                51 => 63,
                                160 => 31,
                                _ => 0,
                            };

                            let pending = (offset % 3) as u8;
                            let warm = deadline >= 160;
                            let mut interpreter =
                                device(&words, offset, Engine::interpreter(), wired, pending, warm);
                            run_until(&mut interpreter, deadline);
                            let first = snapshot(&interpreter);
                            let resumed_deadline = interpreter.cpu.mmu.registers.regs
                                [MmuRegister::Count as usize]
                                + 19;
                            run_until(&mut interpreter, resumed_deadline);
                            let resumed = snapshot(&interpreter);

                            for engine in
                                [Engine::jit(), aot::load_artifact(&artifact, &rom).unwrap()]
                            {
                                let mut actual =
                                    device(&words, offset, engine, wired, pending, warm);
                                run_until(&mut actual, deadline);

                                assert_eq!(
                                    snapshot(&actual),
                                    first,
                                    "primary={primary} offset={offset} prefix={prefix} nop={nop_slot} wired={wired} deadline={deadline}"
                                );
                                run_until(&mut actual, resumed_deadline);

                                assert_eq!(
                                    snapshot(&actual),
                                    resumed,
                                    "resumed primary={primary} offset={offset} prefix={prefix} nop={nop_slot} wired={wired} deadline={deadline}"
                                );
                                native_calls +=
                                    actual.execution_engine.stats.native_branch_executions;
                            }
                        }
                    }
                }
            }
        }

        assert!(
            native_calls > 0,
            "production path never exercised native branch blocks"
        );
    }
}
