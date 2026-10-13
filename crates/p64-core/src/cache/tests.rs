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

fn run_cached_words(n64: &mut N64, words: usize) {
    for i in 0..words {
        n64.cpu.execution.pc = 0xFFFF_FFFF_8000_0000 + (i * 4) as u64;
        n64.cpu.registers.gpr[0] = 0;
        icache_fetch(n64, (i * 4) as u64);
    }
}

fn device() -> N64 {
    let mut n64 = N64::new();
    n64.cartridge.rom = crate::builder::memory::Memory::new().with_size(4).build();
    memory::initialise_memory_map(&mut n64);

    n64
}
