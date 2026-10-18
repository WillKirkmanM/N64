use super::*;
use crate::{
    cpu::{decode_opcode, execute_decoded_instruction, start_cpu},
    mmu,
};

#[test]
fn aot_miss_retains_production_decoded_wide_privilege_fault() {
    let mut n64 = N64::new();
    start_cpu(&mut n64);
    n64.execution_engine = crate::recompiler::Engine::aot([]);
    n64.cpu.execution.pc = 0x1000;
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 16; // User, UX=0.
    n64.cpu.registers.gpr[2] = 0xdeadbeef;
    let opcode = 0x64220001;
    let fallback = decode_opcode(&n64, opcode);
    execute_decoded_instruction(&mut n64, opcode, fallback);

    assert_eq!(n64.cpu.registers.gpr[2], 0xdeadbeef);
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] & 0x7c,
        10 << 2
    );
    assert_eq!(n64.execution_engine.stats.fallback_executions, 1);
    assert_eq!(n64.execution_engine.stats.native_executions, 0);
}

#[test]
fn random_batch_matches_all_six_bit_wired_states() {
    for wired in 0u64..64 {
        for random in 0u64..64 {
            let mut scalar = random;

            for length in 1..=8 {
                scalar = if scalar == wired {
                    31
                } else {
                    scalar.wrapping_sub(1) & 63
                };

                assert_eq!(
                    retired_random(random, wired, length),
                    scalar,
                    "Random={random} Wired={wired} length={length}"
                );
            }
        }
    }
}

#[test]
fn random_batch_matches_scalar_across_each_wired_zero_wrap() {
    let mut n64 = N64::new();
    start_cpu(&mut n64);
    n64.execution_engine = crate::recompiler::Engine::jit();

    for length in 2..=8 {
        let block = n64
            .execution_engine
            .prepare_block(&vec![0x24210001; length])
            .unwrap();

        for random in 0..=31 {
            n64.cpu.execution.pc = 0xffff_ffff_8000_0000;
            n64.cpu.mmu.registers.regs[MmuRegister::Random as usize] = random;
            n64.cpu.mmu.registers.regs[MmuRegister::Wired as usize] = 0;
            n64.cpu.mmu.clock.instruction_clock = 0;
            for _ in 0..length {
                mmu::begin_instruction(&mut n64);
            }

            let expected = n64.cpu.mmu.registers.regs[MmuRegister::Random as usize];
            n64.cpu.mmu.registers.regs[MmuRegister::Random as usize] = random;
            n64.cpu.mmu.clock.instruction_clock = 0;

            assert!(execute_native_block(&mut n64, block));
            assert_eq!(
                n64.cpu.mmu.registers.regs[MmuRegister::Random as usize],
                expected,
                "Random={random}, length={length}"
            );
            assert_eq!(n64.cpu.mmu.clock.instruction_clock, length as u64);
        }
    }
}
