#[cfg(target_arch = "x86_64")]
mod support;
#[cfg(target_arch = "x86_64")]
mod memory_blocks {
    use p64_core::{aot, cache, cpu, memory, mmu::MmuRegister, recompiler::Engine, N64};

    use crate::support::run_until;

    #[derive(Debug, PartialEq)]
    struct Snapshot {
        gpr: [u64; 32],
        cp0: Vec<u64>,
        pc: (u64, u64, u64),
        clock: u64,
        pending: u8,
        llbit: bool,
        fpu: (Option<u8>, Option<u8>),
        branch: (u8, u64),
        dcache: Vec<(bool, bool, bool, u32, u16, [u32; 4])>,
        icache: Vec<(bool, u32, [u32; 8])>,
        ram: Vec<u8>,
    }

    fn snapshot(n64: &N64) -> Snapshot {
        let state = match n64.cpu.branch_state.state {
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
            pc: (
                n64.cpu.execution.pc,
                n64.cpu.execution.pc_phys,
                n64.cpu.execution.data_address,
            ),
            clock: n64.cpu.mmu.clock.instruction_clock,
            pending: n64.cpu.execution.pending_gpr_load,
            llbit: n64.cpu.registers.llbit,
            fpu: (
                n64.cpu.fpu.timing.previous_write,
                n64.cpu.fpu.timing.pending_write,
            ),
            branch: (state, n64.cpu.branch_state.pc),
            dcache: n64
                .memory
                .caches
                .dcache
                .iter()
                .map(|line| {
                    (
                        line.valid,
                        line.dirty,
                        line.store_dirty,
                        line.tag,
                        line.index,
                        line.words,
                    )
                })
                .collect(),
            icache: n64
                .memory
                .caches
                .icache
                .iter()
                .map(|line| (line.valid, line.tag, line.words))
                .collect(),
            ram: n64.rdram.mem.data[0x4000..0xa000].to_vec(),
        }
    }

    fn program(second_load_offset: u32, wide: bool) -> Vec<u32> {
        let mut words = vec![
            0x8e110000,                      // LW r17,0(r16)
            0x26310001,                      // ADDIU r17,r17,1: dependent load interlock
            0xae110004,                      // SW r17,4(r16)
            0x8e120000 | second_load_offset, // hit, cold next line, or dirty conflicting tag
            0x02519821,                      // ADDU r19,r18,r17: another dependent load
            0xae130000,                      // SW r19,0(r16)
            0x66100010,                      // DADDIU r16,r16,16
            0x8e00fff0, // LW r0,-16(r16): discard value, retain memory semantics
            0x32101ffc,
            0x36104000,
            0x3c148000,
            0x02148025,
            0x08000000,
            0x26b50001, // J entry; observable delay slot
            0,
            0,
        ];

        if wide {
            for index in [0, 3, 7] {
                words[index] = (words[index] & 0x03ff_ffff) | (55 << 26);
            }

            for index in [2, 5] {
                words[index] = (words[index] & 0x03ff_ffff) | (63 << 26);
            }
            words[1] = 0x66310001; // DADDIU r17,r17,1
            words[2] = (words[2] & !65535) | 8; // aligned 64-bit store in same line
            words[4] = 0x0251982d; // DADDU r19,r18,r17
        }

        words
    }

    fn device(words: &[u32], engine: Engine, warm: bool, uncached: bool, pending: u8) -> N64 {
        let mut n64 = N64::new();
        n64.cartridge.rom = p64_core::builder::memory::Memory::new()
            .with_size(4)
            .build();
        memory::initialise_memory_map(&mut n64);
        // Use a NOP handler for the midblock AdEL, avoiding boot ROM execution.
        fn exception_nop(_: &mut N64, _: u64, _: memory::AccessSize) -> u32 {
            0
        }
        n64.memory.bus.memory_map_read[0x1fc0] = exception_nop;
        cpu::start_cpu(&mut n64);
        cache::init(&mut n64);
        n64.memory.fetch_pipeline.enabled = true;
        n64.memory.fetch_pipeline.native_min_block_length = 2;
        for (index, &word) in words.iter().enumerate() {
            p64_core::rdram::write_mem(&mut n64, index as u64 * 4, word, u32::MAX);
        }

        for address in (0x4000..0xa000).step_by(4) {
            p64_core::rdram::write_mem(&mut n64, address, 0x8bad0000 ^ address as u32, u32::MAX);
        }
        n64.cpu.execution.pc = 0xffff_ffff_8000_0000;
        n64.cpu.branch_state.state = cpu::State::Step;
        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0;
        n64.cpu.mmu.registers.regs[MmuRegister::Wired as usize] = if warm { 29 } else { 0 };
        n64.cpu.mmu.registers.regs[MmuRegister::Random as usize] = 31;
        if warm {
            for index in 0..3 {
                cache::icache_fill(&mut n64, index, index as u64 * 32);
            }
            n64.cpu.execution.data_address = 0xffff_ffff_8000_4000;
            cache::dcache_read(&mut n64, 0x4000);
        }
        n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] = 0;
        n64.cpu.mmu.clock.instruction_clock = 0;
        n64.cpu.execution.pending_gpr_load = pending;
        n64.cpu.registers.gpr[16] = if uncached {
            0xffff_ffff_a000_4000
        } else {
            0xffff_ffff_8000_4000
        };
        n64.cpu.fpu.timing.previous_write = Some(7);
        n64.cpu.fpu.timing.pending_write = Some(3);
        n64.cpu.registers.llbit = true;
        n64.execution_engine = engine;

        n64
    }

    #[test]
    fn one_instruction_side_exit_preserves_fpu_and_load_timing() {
        let words = [0x24210007, 0x8e110000];
        let mut n64 = device(&words, Engine::jit(), false, false, 1);
        n64.cpu.scheduler.next_event_count = u64::MAX;
        let block = n64.execution_engine.prepare_memory_block(&words).unwrap();

        assert_eq!(cpu::execute_native_memory_block(&mut n64, block), 1);
        assert_eq!(n64.cpu.registers.gpr[1], 7);
        assert_eq!(n64.cpu.mmu.registers.regs[MmuRegister::Count as usize], 2);
        assert_eq!(n64.cpu.mmu.registers.regs[MmuRegister::Random as usize], 30);
        assert_eq!(n64.cpu.mmu.clock.instruction_clock, 1);
        assert_eq!(n64.cpu.execution.pc, 0xffff_ffff_8000_0004);
        assert_eq!(n64.cpu.execution.data_address, 0xffff_ffff_8000_0000);
        assert_eq!(n64.cpu.execution.pending_gpr_load, 0);
        assert_eq!(n64.cpu.fpu.timing.previous_write, Some(3));
        assert_eq!(n64.cpu.fpu.timing.pending_write, None);
        assert!(!n64.memory.caches.dcache[0].valid);
        assert_eq!(n64.execution_engine.stats.native_memory_executions, 1);
    }
}
