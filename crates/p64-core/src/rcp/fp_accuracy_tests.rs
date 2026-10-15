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
