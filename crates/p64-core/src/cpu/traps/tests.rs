use super::{teq, teqi, tge, tgei, tgeiu, tgeu, tlt, tlti, tltiu, tltu, tne, tnei};
use crate::{
    cpu::State,
    mmu::{MmuCause, MmuRegister},
    N64,
};

type Handler = fn(&mut N64, u32);
type Predicate = fn(u64, u64) -> bool;

fn reset(n64: &mut N64, source: u64, target: u64) {
    n64.cpu.execution.pc = 0xffff_ffff_8000_1000;
    n64.cpu.branch_state.state = State::Step;
    n64.cpu.registers.gpr[1] = source;
    n64.cpu.registers.gpr[2] = target;
    n64.set_cp0_register(MmuRegister::Status, 0);
    n64.set_cp0_register(MmuRegister::Cause, 0);
    n64.set_cp0_register(MmuRegister::Count, 100);
}

fn assert_trap(n64: &N64, expected: bool) {
    assert_eq!(
        matches!(n64.cpu.branch_state.state, State::Exception),
        expected
    );
    assert_eq!(
        n64.cp0_register(MmuRegister::Count),
        if expected { 104 } else { 100 }
    );
    assert_eq!(
        n64.cp0_register(MmuRegister::Cause),
        if expected {
            MmuCause::ExccodeTR as u64
        } else {
            0
        }
    );
}

#[test]
fn register_and_immediate_traps_preserve_signedness_and_exception_cycles() {
    let cases: [(Handler, Handler, Predicate); 6] = [
        (tge, tgei, |a, b| a as i64 >= b as i64),
        (tgeu, tgeiu, |a, b| a >= b),
        (tlt, tlti, |a, b| (a as i64) < b as i64),
        (tltu, tltiu, |a, b| a < b),
        (teq, teqi, |a, b| a == b),
        (tne, tnei, |a, b| a != b),
    ];
    let boundaries = [0, 1, i64::MAX as u64, i64::MIN as u64, u64::MAX];
    let mut n64 = N64::default();

    for (register, immediate, expected) in cases {
        for source in boundaries {
            for target in boundaries {
                reset(&mut n64, source, target);
                register(&mut n64, (1 << 21) | (2 << 16));
                assert_trap(&n64, expected(source, target));
            }

            for operand in [0u16, 1, 0x7fff, 0x8000, 0xffff] {
                let target = (operand as i16 as i64) as u64;
                reset(&mut n64, source, 0);
                immediate(&mut n64, (1 << 21) | u32::from(operand));
                assert_trap(&n64, expected(source, target));
            }
        }
    }
}
