#[cfg(target_arch = "x86_64")]
mod support;
#[cfg(target_arch = "x86_64")]
#[test]
fn native_backend_matches_cpu_interpreter() {
    use p64_core::{cpu, recompiler::Engine, N64};
    let mut device = N64::default();
    cpu::start_cpu(&mut device);
    let mut jit = Engine::jit();
    let mut random = 0x8f12_789a_dead_beef_u64;
    let special = support::ALU_FUNCTIONS;

    for iteration in 0..512_u32 {
        let mut initial = [0; 32];

        for value in initial.iter_mut().skip(1) {
            *value = support::random(&mut random);
        }

        let rs = iteration % 4;
        let rt = (iteration / 4) % 4;
        let rd = (iteration / 16) % 4;
        let sa = (iteration / 16) % 32;
        let register_ops = special
            .iter()
            .map(|function| (rs << 21) | (rt << 16) | (rd << 11) | (sa << 6) | function);
        let immediate_ops = support::ALU_PRIMARIES
            .into_iter()
            .map(|primary| (primary << 26) | (rs << 21) | (rt << 16) | ((random as u32) & 65535));

        for opcode in register_ops.chain(immediate_ops) {
            device.cpu.registers.gpr = initial;
            let interpreter = cpu::decode_opcode(&device, opcode);
            interpreter(&mut device, opcode);
            device.cpu.registers.gpr[0] = 0;
            let mut actual = initial;

            assert!(jit.try_execute(opcode, &mut actual));
            assert_eq!(
                actual, device.cpu.registers.gpr,
                "opcode {opcode:08x}, seed {iteration}"
            );
        }
    }

    assert_eq!(jit.stats.native_executions, 16_896);
}
