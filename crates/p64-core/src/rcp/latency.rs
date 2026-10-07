use super::{encoding::FloatEncoding, operation::FloatOperation, INTEGER_CONVERSION_LIMIT};
use crate::opcode::{Cop1Opcode, FpuFunction};

pub(super) fn cycles(operation: FloatOperation, a: u64, b: u64) -> u64 {
    use FpuFunction::*;
    let double = operation.double();
    let encoding = FloatEncoding::for_precision(double);
    let special =
        |bits: u64| bits & encoding.exponent == 0 || bits & encoding.exponent == encoding.exponent;

    if operation.comparison() || matches!(operation.function, Abs | Mov | Neg) {
        return 1;
    }

    if matches!(operation.format, Cop1Opcode::Word | Cop1Opcode::Long) {
        let integer = if operation.format == Cop1Opcode::Word {
            a as u32 as i32 as i64
        } else {
            a as i64
        };

        return if integer == 0
            || (operation.format == Cop1Opcode::Long
                && !(-INTEGER_CONVERSION_LIMIT..INTEGER_CONVERSION_LIMIT).contains(&integer))
        {
            2
        } else {
            5
        };
    }

    match operation.function {
        Add | Sub => {
            if special(a) || special(b) {
                2
            } else {
                3
            }
        }
        Mul => {
            if special(a) || special(b) || a & encoding.fraction == 0 || b & encoding.fraction == 0
            {
                2
            } else if double {
                8
            } else {
                5
            }
        }
        Div => {
            if special(a) || special(b) {
                2
            } else if double {
                58
            } else {
                29
            }
        }
        Sqrt => {
            if special(a) || a & encoding.sign != 0 {
                2
            } else if double {
                58
            } else {
                29
            }
        }
        CvtD => 1,
        CvtS => 2,
        _ if operation.integer_result() => {
            let limit = encoding.integer_limit(if operation.long_result() { 53 } else { 32 });

            if encoding.is_subnormal(a) || a & !encoding.sign >= limit {
                2
            } else {
                5
            }
        }
        _ => 1,
    }
}
