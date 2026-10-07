use super::operation::FloatOperation;
use crate::{
    fpu::RoundingMode,
    opcode::{Cop1Opcode, FpuFunction},
};
use softfloat_pure::softfloat::*;

/// FCSR positive/negative directions reverse SoftFloat's min/max encodings.
fn rounding_mode(rounding: RoundingMode) -> u8 {
    match rounding {
        RoundingMode::Nearest => softfloat_round_near_even,
        RoundingMode::TowardZero => softfloat_round_minMag,
        RoundingMode::TowardPositive => softfloat_round_max,
        RoundingMode::TowardNegative => softfloat_round_min,
    }
}

pub(super) fn to_integer(bits: u64, double: bool, rounding: RoundingMode) -> (i64, u32) {
    let mode = rounding_mode(rounding);
    let (value, flags) = if double {
        f64_to_i64(float64_t { v: bits }, mode, true)
    } else {
        f32_to_i64(float32_t { v: bits as u32 }, mode, true)
    };

    (value, u32::from(flags))
}

pub(super) fn execute_operation(
    operation: FloatOperation,
    a: u64,
    b: u64,
    rounding: RoundingMode,
) -> (u64, u32) {
    use Cop1Opcode::*;
    use FpuFunction::*;
    let mode = rounding_mode(rounding);
    let tiny = softfloat_tininess_afterRounding;

    match (operation.format, operation.function) {
        (Double, CvtS) => {
            let (value, flags) = f64_to_f32(float64_t { v: a }, mode, tiny);

            (u64::from(value.v), u32::from(flags))
        }
        (Single, CvtD) => {
            let (value, flags) = f32_to_f64(float32_t { v: a as u32 });

            (value.v, u32::from(flags))
        }
        (Word | Long, function) => {
            let integer = if operation.format == Word {
                a as u32 as i32 as i64
            } else {
                a as i64
            };

            if function == CvtS {
                let (value, flags) = i64_to_f32(integer, mode, tiny);

                (u64::from(value.v), u32::from(flags))
            } else {
                let (value, flags) = i64_to_f64(integer, mode, tiny);

                (value.v, u32::from(flags))
            }
        }
        (Single, function) => {
            let a = float32_t { v: a as u32 };
            let b = float32_t { v: b as u32 };
            let (value, flags) = match function {
                Add => f32_add(a, b, mode, tiny),
                Sub => f32_sub(a, b, mode, tiny),
                Mul => f32_mul(a, b, mode, tiny),
                Div => f32_div(a, b, mode, tiny),
                Sqrt => f32_sqrt(a, mode, tiny),
                _ => unreachable!("validated single arithmetic operation"),
            };

            (u64::from(value.v), u32::from(flags))
        }
        (_, function) => {
            let a = float64_t { v: a };
            let b = float64_t { v: b };
            let (value, flags) = match function {
                Add => f64_add(a, b, mode, tiny),
                Sub => f64_sub(a, b, mode, tiny),
                Mul => f64_mul(a, b, mode, tiny),
                Div => f64_div(a, b, mode, tiny),
                Sqrt => f64_sqrt(a, mode, tiny),
                _ => unreachable!("validated double arithmetic operation"),
            };

            (value.v, u32::from(flags))
        }
    }
}

#[cfg(all(test, target_arch = "x86_64"))]
pub(super) fn execute(fmt: u32, function: u32, a: u64, b: u64, rounding: u32) -> (u64, u32) {
    let operation = super::test_operation(fmt, function);
    let rounding = RoundingMode::from_encoding(rounding).expect("test rounding mode");

    execute_operation(operation, a, b, rounding)
}
