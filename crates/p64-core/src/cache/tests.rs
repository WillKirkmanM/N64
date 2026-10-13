use super::*;

#[test]
fn minimum_native_length_prunes_short_blocks_before_dispatch() {
    let mut n64 = device();
    cpu::start_cpu(&mut n64);
    n64.execution_engine = crate::recompiler::Engine::jit();
    n64.cpu.execution.pc = 0xffff_ffff_8000_0000;
    n64.memory.fetch_pipeline.enabled = false;
    for address in (0..32).step_by(4) {
        crate::rdram::write_mem(&mut n64, address, 0x00000018, u32::MAX);
    }

    crate::rdram::write_mem(&mut n64, 0, 0x24210001, u32::MAX);
    crate::rdram::write_mem(&mut n64, 4, 0x24420002, u32::MAX);
    n64.memory.fetch_pipeline.native_min_block_length = 4;
    icache_fill(&mut n64, 0, 0);

    assert_eq!(n64.memory.caches.icache[0].native.candidates, 0);
    assert_eq!(n64.execution_engine.compiled_blocks(), 0);
    assert!(!try_execute_native_block(&mut n64, 0));
    assert_eq!(n64.cpu.registers.gpr[1], 0);
    n64.memory.fetch_pipeline.native_min_block_length = 2;
    icache_fill(&mut n64, 0, 0);

    assert_eq!(n64.memory.caches.icache[0].native.candidates, 1);
    assert!(try_execute_native_block(&mut n64, 0));
    assert_eq!(n64.cpu.registers.gpr[1], 1);
    assert_eq!(n64.cpu.registers.gpr[2], 2);
}

#[test]
fn memory_prefix_keeps_cold_next_line_refill_at_word_six() {
    fn setup(native: bool) -> N64 {
        let mut n64 = pipeline_device();
        init(&mut n64);
        n64.memory.fetch_pipeline.native_min_block_length = 2;
        if native {
            n64.execution_engine = crate::recompiler::Engine::jit();
        }

        let words = [
            0xde010000, 0x64210001, 0xfe010008, 0xde020008, 0x64420001, 0xfe020000, 0xde030000,
            0x64630001,
        ];

        for (index, opcode) in words.into_iter().enumerate() {
            crate::rdram::write_mem(&mut n64, index as u64 * 4, opcode, u32::MAX);
        }
        n64.cpu.execution.pc = 0xffff_ffff_8000_0000;
        icache_fill(&mut n64, 0, 0);
        n64.cpu.execution.data_address = 0xffff_ffff_8000_4000;
        dcache_read(&mut n64, 0x4000);
        n64.cpu.registers.gpr[16] = 0xffff_ffff_8000_4000;
        n64.cpu.mmu.registers.regs[crate::mmu::MmuRegister::Count as usize] = 0;
        n64.cpu.mmu.clock.instruction_clock = 0;
        n64.cpu.execution.data_address = n64.cpu.execution.pc;

        n64
    }
    fn scalar_step(n64: &mut N64) {
        let (physical, cached, error) =
            memory::translate_address(n64, n64.cpu.execution.pc, memory::AccessType::Read);
        assert!(cached && !error);
        n64.cpu.execution.pc_phys = physical;
        assert!(!icache_fetch(n64, physical));
        n64.cpu.execution.pc += 4;
        crate::mmu::add_cycles(n64, 1);
    }
    fn equal_architecture(a: &N64, b: &N64) {
        assert_eq!(a.cpu.registers.gpr, b.cpu.registers.gpr);
        assert_eq!(a.cpu.mmu.registers.regs.regs, b.cpu.mmu.registers.regs.regs);
        assert_eq!(
            (
                a.cpu.execution.pc,
                a.cpu.execution.pc_phys,
                a.cpu.execution.data_address,
                a.cpu.execution.pending_gpr_load,
                a.cpu.mmu.clock.instruction_clock
            ),
            (
                b.cpu.execution.pc,
                b.cpu.execution.pc_phys,
                b.cpu.execution.data_address,
                b.cpu.execution.pending_gpr_load,
                b.cpu.mmu.clock.instruction_clock
            )
        );
        for (a, b) in a
            .memory
            .caches
            .dcache
            .iter()
            .zip(b.memory.caches.dcache.iter())
        {
            assert_eq!(
                (a.valid, a.dirty, a.store_dirty, a.tag, a.index, a.words),
                (b.valid, b.dirty, b.store_dirty, b.tag, b.index, b.words)
            );
        }

        for (a, b) in a
            .memory
            .caches
            .icache
            .iter()
            .zip(b.memory.caches.icache.iter())
        {
            assert_eq!(
                (a.valid, a.tag, a.index, a.words),
                (b.valid, b.tag, b.index, b.words)
            );
        }

        let latch = |n64: &N64| {
            n64.memory
                .fetch_pipeline
                .next
                .map(|l| (l.virtual_address, l.physical_address, l.word))
        };
        assert_eq!(latch(a), latch(b));
    }
    let mut reference = setup(false);
    let mut native = setup(true);

    assert!(!native.memory.caches.icache[1].valid);
    assert!(try_execute_native_block(&mut native, 0));
    assert_eq!(native.cpu.execution.pc, 0xffff_ffff_8000_0018);
    assert_eq!(native.execution_engine.stats.native_memory_executions, 1);
    assert!(
        !native.memory.caches.icache[1].valid,
        "prefix must not prefill the next line"
    );
    for _ in 0..6 {
        scalar_step(&mut reference);
    }

    equal_architecture(&reference, &native);
    scalar_step(&mut reference);
    scalar_step(&mut native);

    assert!(
        native.memory.caches.icache[1].valid,
        "word six services the ordinary RF miss"
    );
    equal_architecture(&reference, &native);
}

fn pipeline_device() -> N64 {
    let mut n64 = device();
    cpu::start_cpu(&mut n64);
    n64.memory.fetch_pipeline.enabled = true;

    n64
}

#[derive(Debug, PartialEq, Eq)]
struct BlockRefillTrace {
    gpr1: u64,
    pc: u64,
    count: u64,
    random: u64,
    instruction_clock: u64,
    next_fetch: Option<(u64, u64, u32)>,
}

fn run_block_refill_trace(native: bool) -> BlockRefillTrace {
    let mut n64 = pipeline_device();

    if native {
        n64.execution_engine = crate::recompiler::Engine::jit();
    }

    for address in (0..64).step_by(4) {
        crate::rdram::write_mem(&mut n64, address, 0x24210001, u32::MAX);
    }
    n64.cpu.execution.pc = 0xffff_ffff_8000_0000;
    icache_fill(&mut n64, 0, 0);
    n64.cpu.mmu.registers.regs[crate::mmu::MmuRegister::Count as usize] = 0;
    let mut batches = 0;

    while n64.cpu.execution.pc < 0xffff_ffff_8000_0020 {
        let pc = n64.cpu.execution.pc;
        let (physical, cached, error) =
            memory::translate_address(&mut n64, pc, memory::AccessType::Read);
        assert!(cached && !error);
        n64.cpu.execution.pc_phys = physical;

        if native && try_execute_native_block(&mut n64, physical) {
            batches += 1;
        } else {
            icache_fetch(&mut n64, physical);
            n64.cpu.execution.pc += 4;
            crate::mmu::add_cycles(&mut n64, 1);
        }
    }

    assert_eq!(batches, if native { 1 } else { 0 });
    assert!(
        n64.memory.caches.icache[1].valid,
        "last two scalar words must service RF refill"
    );
    assert!(n64.memory.fetch_pipeline.pending_refill.is_none());

    BlockRefillTrace {
        gpr1: n64.cpu.registers.gpr[1],
        pc: n64.cpu.execution.pc,
        count: n64.cpu.mmu.registers.regs[crate::mmu::MmuRegister::Count as usize],
        random: n64.cpu.mmu.registers.regs[crate::mmu::MmuRegister::Random as usize],
        instruction_clock: n64.cpu.mmu.clock.instruction_clock,
        next_fetch: n64
            .memory
            .fetch_pipeline
            .next
            .map(|latch| (latch.virtual_address, latch.physical_address, latch.word)),
    }
}

#[test]
fn native_batch_preserves_last_two_words_refill_and_fetch_latch() {
    assert_eq!(run_block_refill_trace(true), run_block_refill_trace(false));
}

#[test]
fn native_blocks_use_cached_words_after_store_and_engine_replacement() {
    let mut n64 = pipeline_device();
    n64.execution_engine = crate::recompiler::Engine::jit();
    for address in (0..32).step_by(4) {
        crate::rdram::write_mem(&mut n64, address, 0x24210001, u32::MAX);
    }
    n64.cpu.execution.pc = 0xffff_ffff_8000_0000;
    icache_fill(&mut n64, 0, 0);
    for address in (0..32).step_by(4) {
        crate::rdram::write_mem(&mut n64, address, 0x24210002, u32::MAX);
    }
    n64.execution_engine = crate::recompiler::Engine::jit();

    assert!(try_execute_native_block(&mut n64, 0));
    assert_eq!(
        n64.cpu.registers.gpr[1], 6,
        "compiled blocks must use cached +1 words"
    );
    assert_eq!(n64.execution_engine.stats.native_executions, 6);
    assert_eq!(n64.cpu.execution.pc, 0xffff_ffff_8000_0018);
    let next = n64.memory.fetch_pipeline.next.unwrap();

    assert_eq!(next.word, 0x24210001);
    assert_eq!(next.physical_address, 24);
}

fn run_cached_words(n64: &mut N64, words: usize) {
    for i in 0..words {
        n64.cpu.execution.pc = 0xFFFF_FFFF_8000_0000 + (i * 4) as u64;
        n64.cpu.registers.gpr[0] = 0;
        icache_fetch(n64, (i * 4) as u64);
    }
}

#[test]
fn instruction_refill_precedes_late_uncached_store_but_follows_dc_writeback() {
    for position in [5, 6, 7] {
        let mut n64 = pipeline_device();
        n64.cpu.registers.gpr[1] = 0xFFFF_FFFF_A000_0020;
        n64.cpu.registers.gpr[2] = 0x3403_2222;
        crate::rdram::write_mem(&mut n64, (position * 4) as u64, 0xAC22_0000, u32::MAX);
        crate::rdram::write_mem(&mut n64, 32, 0x3403_1111, u32::MAX);
        run_cached_words(&mut n64, 9);

        assert_eq!(
            n64.cpu.registers.gpr[3],
            if position < 6 { 0x2222 } else { 0x1111 },
            "store {position}"
        );
    }

    for position in [5, 6, 7] {
        let mut n64 = pipeline_device();
        n64.cpu.registers.gpr[1] = 0xFFFF_FFFF_8000_0020;
        crate::rdram::write_mem(&mut n64, (position * 4) as u64, 0xBC39_0000, u32::MAX);
        crate::rdram::write_mem(&mut n64, 32, 0x3403_1111, u32::MAX);
        n64.cpu.execution.data_address = n64.cpu.registers.gpr[1];
        dcache_write(&mut n64, 32, 0x3403_2222, u32::MAX);
        run_cached_words(&mut n64, 9);

        assert_eq!(
            n64.cpu.registers.gpr[3],
            if position < 7 { 0x2222 } else { 0x1111 },
            "writeback {position}"
        );
    }
}

fn device() -> N64 {
    let mut n64 = N64::new();
    n64.cartridge.rom = crate::builder::memory::Memory::new().with_size(4).build();
    memory::initialise_memory_map(&mut n64);

    n64
}
