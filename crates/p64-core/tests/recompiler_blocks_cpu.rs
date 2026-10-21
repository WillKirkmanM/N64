#[cfg(target_arch = "x86_64")]
#[path = "support/retirement.rs"]
mod retirement;
#[cfg(target_arch = "x86_64")]
mod support;
#[cfg(target_arch = "x86_64")]
#[test]
fn native_blocks_match_scalar_cpu_retirement_with_aliases_and_hazards() {
    use p64_core::{
        cpu,
        mmu::{self, MmuRegister},
        recompiler::{compile_block, Engine},
        N64,
    };
    use retirement::snapshot;
    use support::random;
    let functions = support::ALU_FUNCTIONS;
    let primaries = support::ALU_PRIMARIES;
    let mut seed = 0x9153_aafa_dead_beef;
    let mut programs = Vec::new();

    for index in 0..512 {
        let mut words = Vec::new();

        for instruction in 0..2 + index % 7 {
            let value = random(&mut seed);
            let mask = if index % 2 == 0 { 3 } else { 31 };

            let rs = value as u32 & mask;
            let rt = (value >> 8) as u32 & mask;
            let rd = (value >> 16) as u32 & mask;
            let selector = (index + instruction) % 33;
            words.push(if selector < functions.len() {
                (rs << 21)
                    | (rt << 16)
                    | (rd << 11)
                    | (((value >> 24) as u32 & 31) << 6)
                    | functions[selector]
            } else {
                (primaries[selector - functions.len()] << 26)
                    | (rs << 21)
                    | (rt << 16)
                    | (value as u32 & 65535)
            });
        }

        programs.push(words);
    }

    let codes = programs
        .iter()
        .map(|words| (words.clone(), compile_block(words).unwrap()))
        .collect();
    let mut aot = Engine::from_aot_blocks(codes).unwrap();
    let mut jit = Engine::jit();
    let mut device = N64::default();
    cpu::start_cpu(&mut device);

    for (index, words) in programs.iter().enumerate() {
        let initial =
            std::array::from_fn(|register| if register == 0 { 0 } else { random(&mut seed) });
        let wired = [0, 1, 24, 30, 31][index % 5];
        let initial_random = wired + random(&mut seed) % (32 - wired);
        let initial_count = [0, 1, 0x0001_ffff_fffc][index % 3];
        let setup = |device: &mut N64| {
            device.cpu.registers.gpr = initial;
            device.cpu.registers.hi = 0x12345678;
            device.cpu.registers.lo = u64::MAX;
            device.cpu.execution.pc = 0xffffffff80001000;
            device.cpu.branch_state.state = cpu::State::Step;
            device.cpu.scheduler.next_event_count = u64::MAX;
            device.cpu.mmu.registers.regs[MmuRegister::Count as usize] = initial_count;
            device.cpu.mmu.registers.regs[MmuRegister::Wired as usize] = wired;
            device.cpu.mmu.registers.regs[MmuRegister::Random as usize] = initial_random;
            device.cpu.mmu.clock.instruction_clock = u64::MAX - 4;
            device.cpu.mmu.clock.pending_wired_reset = None;
            device.cpu.mmu.clock.count_write_origin = if index % 2 == 0 {
                None
            } else {
                Some(initial_count.saturating_sub(3))
            };
            // Counts below 3 still lie in the post-write pause.
            if initial_count < 3 {
                device.cpu.mmu.clock.count_write_origin = None;
            }
            device.cpu.execution.pending_gpr_load = (index % 32) as u8;
            device.cpu.fpu.timing.previous_write = Some(7);
            device.cpu.fpu.timing.pending_write = Some(3);
        };
        setup(&mut device);
        device.execution_engine = Engine::interpreter();
        for &word in words {
            let handler = cpu::decode_opcode(&device, word);
            cpu::execute_instruction(&mut device, word, handler);
            device.cpu.registers.gpr[0] = 0;
            mmu::add_cycles(&mut device, 1);
            device.cpu.execution.pc += 4;
        }

        let expected = snapshot(&device);

        for engine in [&mut jit, &mut aot] {
            setup(&mut device);
            std::mem::swap(&mut device.execution_engine, engine);
            let block = device.execution_engine.prepare_block(words).unwrap();

            assert!(
                cpu::execute_native_block(&mut device, block),
                "block {index} unexpectedly refused"
            );
            assert_eq!(
                snapshot(&device),
                expected,
                "CPU block {index}: {words:08x?}"
            );
            std::mem::swap(&mut device.execution_engine, engine);
        }
    }
}
