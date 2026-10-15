use super::*;
use crate::fpu::FCR31_CMP_BIT;
const OP: u32 = (2 << 16) | (4 << 6); // fs=0, ft=2, fd=4

fn device(a: f32, b: f32, fcsr: u32) -> N64 {
    let mut n64 = N64::new();
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] =
        MmuStatus::FR as u64 | MmuStatus::CU1 as u64;
    fpu::set_fpr_single(&mut n64, 0, a, false);
    fpu::set_fpr_single(&mut n64, 2, b, false);
    n64.cpu.fpu.control.fcr31 = fcsr;

    n64
}

#[test]
fn arithmetic_latency_shortcuts_and_exception_flush_follow_hardware() {
    for (a, b, expected) in [(1.3f32, 0.66f32, 3), (0.0, 3.0, 2), (f32::INFINITY, 3.0, 2)] {
        let mut n64 = device(a, b, 0);
        add_s(&mut n64, OP);

        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] + 1,
            expected
        );
    }

    for (a, b, expected) in [
        (1.3f32, 0.66f32, 5),
        (2.0, 0.66, 2),
        (f32::MIN_POSITIVE, 0.222, 2),
    ] {
        let mut n64 = device(a, b, fpu::FCR31_FS_BIT);
        mul_s(&mut n64, OP);

        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] + 1,
            expected
        );
    }

    let mut n64 = device(1.0, 0.0, 8 << 7);
    div_s(&mut n64, OP);
    // General exception entry=2, operation2 + FP flush2, retire1 => 6.
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] + 1,
        6
    );
}

#[test]
fn fp_dependency_is_one_cycle_and_expires_after_integer_instruction() {
    let mut n64 = device(1.3, 0.66, 0);
    fpu::begin_instruction_timing(&mut n64, OP);
    add_s(&mut n64, OP);
    let before = n64.cpu.mmu.registers.regs[MmuRegister::Count as usize];
    fpu::begin_instruction_timing(&mut n64, OP);
    add_s(&mut n64, (4 << 11) | (2 << 16) | (6 << 6));

    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] - before,
        3
    );
    fpu::begin_instruction_timing(&mut n64, 0); // an intervening integer NOP
    fpu::begin_instruction_timing(&mut n64, OP);
    let before = n64.cpu.mmu.registers.regs[MmuRegister::Count as usize];
    add_s(&mut n64, (6 << 11) | (2 << 16) | (8 << 6));

    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] - before,
        2
    );
}

#[test]
fn unary_unused_ft_still_creates_dependency_and_conversion_threshold_is_exact() {
    let mut n64 = device(2.0, 3.0, 0);
    n64.cpu.fpu.timing.previous_write = Some(0);
    abs_s(&mut n64, (2 << 11) | (4 << 6));

    assert_eq!(n64.cpu.mmu.registers.regs[MmuRegister::Count as usize], 1);
    assert_eq!(fp_latency(16, 12, 4294967040.0f32.to_bits() as u64, 0), 5);
    assert_eq!(fp_latency(16, 12, 4294967296.0f32.to_bits() as u64, 0), 2);
    assert_eq!(fp_latency(17, 8, 9007199254740992.0f64.to_bits(), 0), 2);
    assert_eq!(fp_latency(16, 33, 1.3f32.to_bits() as u64, 0), 1);
}

#[test]
fn cop2_move_slots_and_reserved_slots_decode_correctly() {
    let mut n64 = N64::new();
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = MmuStatus::CU2 as u64;
    n64.cpu.registers.gpr[1] = 0x0123456789ABCDEF;
    n64.cpu.rcp.instrs[6](&mut n64, 1 << 16);

    assert_eq!(n64.cpu.rcp.reg_latch, 0x0123456789ABCDEF);
    n64.cpu.rcp.instrs[0](&mut n64, 2 << 16);

    assert_eq!(n64.cpu.registers.gpr[2], 0xFFFFFFFF89ABCDEF);

    for slot in [3, 7] {
        for usable in [false, true] {
            let mut n64 = N64::new();

            if usable {
                n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = MmuStatus::CU2 as u64;
            }

            n64.cpu.rcp.instrs[slot](&mut n64, 0);

            assert_eq!(
                n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
                MmuCause::CE2 as u64
                    | if usable {
                        MmuCause::ExccodeRI as u64
                    } else {
                        MmuCause::ExccodeCPU as u64
                    }
            );
        }
    }
}

#[test]
fn exact_arithmetic_clears_causes_and_preserves_sticky_flags() {
    let mut n64 = device(1.0, 5.0, fpu::FCR31_CAUSE_MASK | (16 << 2));
    add_s(&mut n64, OP);

    assert_eq!(fpu::get_fpr_single(&n64, 4), 6.0);
    assert_eq!(n64.cpu.fpu.control.fcr31, 16 << 2);
}

#[test]
fn directed_rounding_reports_inexact_and_restores_host_mxcsr() {
    let mut n64 = device(1.0, f32::from_bits(0x33000000), 2);
    #[cfg(target_arch = "x86_64")]
    #[allow(deprecated)]
    let saved = unsafe { std::arch::x86_64::_mm_getcsr() };
    add_s(&mut n64, OP);

    assert_eq!(fpu::get_fpr_single(&n64, 4).to_bits(), 1.0f32.to_bits() + 1);
    assert_eq!(n64.cpu.fpu.control.fcr31, 2 | (1 << 12) | (1 << 2));
    #[cfg(target_arch = "x86_64")]
    #[allow(deprecated)]
    let after = unsafe { std::arch::x86_64::_mm_getcsr() };
    #[cfg(target_arch = "x86_64")]
    assert_eq!(after, saved);
}

#[test]
fn enabled_exception_preserves_destination_and_sets_cause_only() {
    let mut n64 = device(1.0, 0.0, 8 << 7);
    fpu::set_fpr_single(&mut n64, 4, 42.0, false);
    div_s(&mut n64, OP);

    assert_eq!(fpu::get_fpr_single(&n64, 4), 42.0);
    assert_eq!(n64.cpu.fpu.control.fcr31, (8 << 7) | (8 << 12));
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] & MmuCause::ExccodeMask as u64,
        MmuCause::ExccodeFPE as u64
    );
}

#[test]
fn nan_arithmetic_uses_r4300_canonical_result_and_signaling_input_traps() {
    let mut n64 = device(f32::from_bits(0x7FC00000), 2.0, 0);
    add_s(&mut n64, OP);

    assert_eq!(fpu::get_fpr_single(&n64, 4).to_bits(), 0x7FBFFFFF);
    assert_eq!(n64.cpu.fpu.control.fcr31, (16 << 12) | (16 << 2));
    let mut n64 = device(f32::from_bits(0x7F800001), 2.0, 0);
    add_s(&mut n64, OP);

    assert_eq!(n64.cpu.fpu.control.fcr31, fpu::FCR31_CAUSE_UNIMP_BIT);
}

#[test]
fn underflow_obeys_flush_mode_and_directed_rounding() {
    for (mode, expected) in [(0, 0), (1, 0), (2, 0x00800000), (3, 0)] {
        let mut n64 = device(f32::MIN_POSITIVE, 0.5, fpu::FCR31_FS_BIT | mode);
        mul_s(&mut n64, OP);

        assert_eq!(fpu::get_fpr_single(&n64, 4).to_bits(), expected);
        assert_eq!(
            n64.cpu.fpu.control.fcr31,
            fpu::FCR31_FS_BIT | mode | (3 << 12) | (3 << 2)
        );
    }

    let mut n64 = device(f32::MIN_POSITIVE, 0.5, 0);
    mul_s(&mut n64, OP);

    assert_eq!(n64.cpu.fpu.control.fcr31, fpu::FCR31_CAUSE_UNIMP_BIT);
}
