#[allow(deprecated)]
fn native_float_op(fmt: u32, function: u32, a_bits: u64, b_bits: u64, rounding: u32) -> (u64, u32) {
    use std::arch::{
        asm,
        x86_64::{_mm_getcsr, _mm_setcsr},
    };

    unsafe {
        let saved = _mm_getcsr();
        // Mask all host exceptions; disable DAZ/FTZ. MIPS rounding order differs
        // from MXCSR: nearest, toward zero, +infinity, -infinity.
        let mode = [0, 3, 2, 1][rounding as usize];
        _mm_setcsr(0x1F80 | (mode << 13));
        let bits;

        if function == 32 && fmt == 17 {
            let a = f64::from_bits(a_bits);
            let result: f32;
            asm!("cvtsd2ss {result}, {a}", result = out(xmm_reg) result, a = in(xmm_reg) a, options(nostack));
            bits = result.to_bits() as u64;
        } else if function == 33 && fmt == 16 {
            let a = f32::from_bits(a_bits as u32);
            let result: f64;
            asm!("cvtss2sd {result}, {a}", result = out(xmm_reg) result, a = in(xmm_reg) a, options(nostack));
            bits = result.to_bits();
        } else if fmt == 20 || fmt == 21 {
            let a = if fmt == 20 {
                a_bits as u32 as i32 as i64
            } else {
                a_bits as i64
            };

            if function == 32 {
                let result: f32;
                asm!("cvtsi2ss {result}, {a}", result = out(xmm_reg) result, a = in(reg) a, options(nostack));
                bits = result.to_bits() as u64;
            } else {
                let result: f64;
                asm!("cvtsi2sd {result}, {a}", result = out(xmm_reg) result, a = in(reg) a, options(nostack));
                bits = result.to_bits();
            }
        } else if fmt == 16 {
            let mut a = f32::from_bits(a_bits as u32);
            let b = f32::from_bits(b_bits as u32);

            match function {
                0 => {
                    asm!("addss {a}, {b}", a = inout(xmm_reg) a, b = in(xmm_reg) b, options(nostack))
                }
                1 => {
                    asm!("subss {a}, {b}", a = inout(xmm_reg) a, b = in(xmm_reg) b, options(nostack))
                }
                2 => {
                    asm!("mulss {a}, {b}", a = inout(xmm_reg) a, b = in(xmm_reg) b, options(nostack))
                }
                3 => {
                    asm!("divss {a}, {b}", a = inout(xmm_reg) a, b = in(xmm_reg) b, options(nostack))
                }
                4 => asm!("sqrtss {a}, {a}", a = inout(xmm_reg) a, options(nostack)),
                _ => unreachable!(),
            }
            bits = a.to_bits() as u64;
        } else {
            let mut a = f64::from_bits(a_bits);
            let b = f64::from_bits(b_bits);

            match function {
                0 => {
                    asm!("addsd {a}, {b}", a = inout(xmm_reg) a, b = in(xmm_reg) b, options(nostack))
                }
                1 => {
                    asm!("subsd {a}, {b}", a = inout(xmm_reg) a, b = in(xmm_reg) b, options(nostack))
                }
                2 => {
                    asm!("mulsd {a}, {b}", a = inout(xmm_reg) a, b = in(xmm_reg) b, options(nostack))
                }
                3 => {
                    asm!("divsd {a}, {b}", a = inout(xmm_reg) a, b = in(xmm_reg) b, options(nostack))
                }
                4 => asm!("sqrtsd {a}, {a}", a = inout(xmm_reg) a, options(nostack)),
                _ => unreachable!(),
            }
            bits = a.to_bits();
        }

        let status = _mm_getcsr();
        _mm_setcsr(saved);
        let flags = ((status >> 5) & 1)
            | ((status >> 3) & 2)
            | ((status >> 1) & 4)
            | ((status << 1) & 8)
            | ((status << 4) & 16);

        (bits, flags)
    }
}

use super::{float_backend::execute as portable_float_op, is_nan_bits};

fn random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;

    *state
}

fn check(fmt: u32, function: u32, a: u64, b: u64, mode: u32) {
    let native = native_float_op(fmt, function, a, b, mode);
    let portable = portable_float_op(fmt, function, a, b, mode);
    let double = function == 33 || (function != 32 && fmt == 17);
    // Guest results canonicalise NaN payloads; compare exception status for NaN pairs.
    let both_nan = is_nan_bits(native.0, double) && is_nan_bits(portable.0, double);
    assert!(
        native == portable || (both_nan && native.1 == portable.1),
        "fmt={fmt} fn={function} mode={mode} a={a:016x} b={b:016x}: native={native:x?} portable={portable:x?}"
    );
}

#[test]
fn portable_matches_native_results_and_flags_in_all_rounding_modes() {
    let mut state = 0x8a2348bbaacceeff;

    for mode in 0..4 {
        for fmt in [16, 17] {
            for function in 0..5 {
                for _ in 0..20000 {
                    let a = random(&mut state);
                    let b = random(&mut state);
                    check(fmt, function, a, b, mode);
                }
            }

            for a in [
                0,
                1,
                u64::MAX,
                0x00800000,
                0x7f800001,
                0x7fc00000,
                0x0010000000000000,
                0x7ff0000000000001,
                0x7ff8000000000000,
            ] {
                for b in [
                    0,
                    1,
                    u64::MAX,
                    0x00800000,
                    0x7f800001,
                    0x0010000000000000,
                    0x7ff0000000000001,
                ] {
                    for function in 0..5 {
                        check(fmt, function, a, b, mode);
                    }
                }
            }
        }

        for (fmt, function) in [(16, 33), (17, 32), (20, 32), (20, 33), (21, 32), (21, 33)] {
            for _ in 0..20000 {
                check(fmt, function, random(&mut state), 0, mode);
            }
        }
    }
}
