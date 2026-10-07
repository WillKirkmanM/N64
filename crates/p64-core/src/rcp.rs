mod dispatch;
mod memory;
pub(crate) use memory::{ldc2, lwc2, sdc2, swc2};
mod encoding;
mod latency;
mod operation;
mod result;
use operation::FloatOperation;
use result::FloatResult;
mod types;
use encoding::FloatEncoding;
pub use types::Rcp;

mod float_backend;

use super::{
    cpu::{self, check_relative_idle_loop, rt, se32, State},
    exceptions::{cop_unusable_exception, reserved_exception},
    fpu,
    mmu::{add_cycles, MmuCause, MmuRegister, MmuStatus},
};
use crate::{
    opcode::{Cop1Opcode, FpuFunction},
    N64,
};

const FP_INVALID: u32 = softfloat_pure::softfloat::softfloat_flag_invalid as u32;
const FP_UNDERFLOW: u32 = softfloat_pure::softfloat::softfloat_flag_underflow as u32;
const FP_INEXACT: u32 = softfloat_pure::softfloat::softfloat_flag_inexact as u32;
const COMPARE_UNORDERED: u32 = 1 << 0;
const COMPARE_EQUAL: u32 = 1 << 1;
const COMPARE_LESS: u32 = 1 << 2;
const COMPARE_SIGNALING: u32 = 1 << 3;
const FLOAT_TO_LONG_LIMIT: i64 = 1_i64 << 53;

const BASE_INSTRUCTION_CYCLES: u64 = 1;
const EXCEPTION_FLUSH_CYCLES: u64 = 2;
const INTEGER_CONVERSION_LIMIT: i64 = 1_i64 << 55;

#[inline(always)]
fn check_rcp_access(n64: &N64) -> bool {
    n64.cp0_register(MmuRegister::Status) & (MmuStatus::CU2 as u64) != 0
}

fn handle_move(n64: &mut N64, opcode: u32, double: bool, to_rcp: bool) {
    if !check_rcp_access(n64) {
        add_cycles(n64, 1);

        return cop_unusable_exception(n64, MmuCause::CE2 as u64);
    }

    let register = rt(opcode) as usize;

    if to_rcp {
        n64.rcp_mut().reg_latch = n64.cpu().gpr(register);
    } else {
        let latch = n64.rcp().reg_latch;
        let value = if double {
            latch
        } else {
            se32(latch as u32 as i32)
        };

        n64.cpu_mut().set_gpr(register, value);
    }
}

pub fn vector_reserved(n64: &mut N64, _opcode: u32) {
    if !check_rcp_access(n64) {
        add_cycles(n64, 1);
        cop_unusable_exception(n64, MmuCause::CE2 as u64);
    } else {
        reserved_exception(n64, MmuCause::CE2 as u64);
    }
}

pub fn fs(opcode: u32) -> u32 {
    cpu::rd(opcode)
}

pub fn ft(opcode: u32) -> u32 {
    cpu::rt(opcode)
}

pub fn fd(opcode: u32) -> u32 {
    cpu::Instruction::decode(opcode).shift_amount()
}

fn handle_branch(n64: &mut N64, opcode: u32, condition: bool, discard: bool) {
    if condition {
        check_relative_idle_loop(n64, opcode);
        let cpu = n64.cpu_mut();
        cpu.branch_state.state = State::Take;
        cpu.branch_state.pc = cpu
            .execution
            .pc
            .wrapping_add(cpu::se16(cpu::imm(opcode) as i16) << 2)
            + 4;
    } else {
        n64.cpu_mut().branch_state.state = if discard {
            State::Discard
        } else {
            State::NotTaken
        };
    }
}

pub fn bc1f(n64: &mut N64, opcode: u32) {
    handle_branch(n64, opcode, !n64.fpu().control.comparison(), false);
}

pub fn bc1t(n64: &mut N64, opcode: u32) {
    handle_branch(n64, opcode, n64.fpu().control.comparison(), false);
}

pub fn bc1fl(n64: &mut N64, opcode: u32) {
    handle_branch(n64, opcode, !n64.fpu().control.comparison(), true);
}

pub fn bc1tl(n64: &mut N64, opcode: u32) {
    handle_branch(n64, opcode, n64.fpu().control.comparison(), true);
}

fn update_fp_status(n64: &mut N64, flags: u32) -> bool {
    if n64.fpu_mut().record_exception_flags(flags) {
        true
    } else {
        fpu::raise_exception(n64);

        false
    }
}

fn fp_source_index(index: usize, wide: bool) -> usize {
    if wide {
        index
    } else {
        crate::bits::align_down(index, 2)
    }
}

fn is_nan_bits(bits: u64, double: bool) -> bool {
    FloatEncoding::for_precision(double).is_nan(bits)
}

fn is_quiet_nan(bits: u64, double: bool) -> bool {
    FloatEncoding::for_precision(double).is_quiet_nan(bits)
}

fn write_fp_result(n64: &mut N64, index: usize, bits: u64, double: bool) {
    let wide = fpu::is_fr_mode(n64);
    let fpu = n64.fpu_mut();
    fpu.timing.pending_write = Some(index as u8);
    fpu.registers
        .set_arithmetic_result(index, bits, double, wide);
}

fn execute_float(n64: &mut N64, opcode: u32, fmt: Cop1Opcode, function: FpuFunction) {
    let operation = FloatOperation {
        format: fmt,
        function,
    };
    let instruction = cpu::Instruction::decode(opcode);
    let dependency = n64.fpu().timing.depends_on(instruction.rd, instruction.rt);
    let mut latency = BASE_INSTRUCTION_CYCLES;
    execute_operation(n64, instruction, operation, &mut latency);
    let exception = function != FpuFunction::Mov && n64.fpu().exception_pending();

    if exception {
        n64.fpu_mut().timing.pending_write = None;
    }
    // The CPU retires one cycle separately. An FPE adds two COP1 pipeline
    // flush cycles in addition to the CPU's general exception-entry cost.
    add_cycles(
        n64,
        latency - BASE_INSTRUCTION_CYCLES
            + u64::from(dependency)
            + if exception { EXCEPTION_FLUSH_CYCLES } else { 0 },
    );
}

#[cfg(test)]
fn fp_latency(fmt: u32, function: u32, a: u64, b: u64) -> u64 {
    latency::cycles(test_operation(fmt, function), a, b)
}

#[cfg(test)]
fn test_operation(fmt: u32, function: u32) -> FloatOperation {
    FloatOperation {
        format: Cop1Opcode::from_encoding(fmt as usize).expect("test format"),
        function: FpuFunction::from_encoding(function as usize).expect("test function"),
    }
}

#[cfg(all(test, target_arch = "x86_64"))]
fn execute_float_operation(n64: &mut N64, opcode: u32, fmt: u32, function: u32, latency: &mut u64) {
    execute_operation(
        n64,
        cpu::Instruction::decode(opcode),
        test_operation(fmt, function),
        latency,
    );
}

// Compare encodings so UI-owned DAZ/FTZ cannot alter subnormal comparisons.
fn ordered_float_compare(a: u64, b: u64, double: bool) -> (bool, bool) {
    let sign = FloatEncoding::for_precision(double).sign;
    let a = if double { a } else { a & u64::from(u32::MAX) };
    let b = if double { b } else { b & u64::from(u32::MAX) };

    let both_zero = a & !sign == 0 && b & !sign == 0;
    let equal = a == b || both_zero;
    let less = !equal
        && if (a ^ b) & sign != 0 {
            a & sign != 0
        } else if a & sign != 0 {
            a > b
        } else {
            a < b
        };

    (equal, less)
}

fn execute_comparison(n64: &mut N64, operation: FloatOperation, a_bits: u64, b_bits: u64) {
    let double = operation.double();
    let a_nan = is_nan_bits(a_bits, double);
    let b_nan = is_nan_bits(b_bits, double);
    // Unlike arithmetic, comparisons support subnormals and signalling NaNs.
    let unordered = a_nan || b_nan;
    let invalid = is_quiet_nan(a_bits, double)
        || is_quiet_nan(b_bits, double)
        || ((operation.function as u32) & COMPARE_SIGNALING != 0 && unordered);

    if !update_fp_status(n64, if invalid { FP_INVALID } else { 0 }) {
        return;
    }

    let (equal, less) = ordered_float_compare(a_bits, b_bits, double);
    let condition = ((operation.function as u32) & COMPARE_UNORDERED != 0 && unordered)
        || ((operation.function as u32) & COMPARE_EQUAL != 0 && !unordered && equal)
        || ((operation.function as u32) & COMPARE_LESS != 0 && !unordered && less);
    n64.fpu_mut().control.set_comparison(condition);
}

fn execute_integer_conversion(
    n64: &mut N64,
    operation: FloatOperation,
    a_bits: u64,
    destination: usize,
    rounding: fpu::RoundingMode,
) {
    let double = operation.double();
    let (value, flags) = float_backend::to_integer(a_bits, double, operation.rounding(rounding));
    let long = operation.long_result();
    // VR4300 float -> long supports magnitude strictly below 2^53.
    let valid = if long {
        value > -FLOAT_TO_LONG_LIMIT && value < FLOAT_TO_LONG_LIMIT
    } else {
        i32::try_from(value).is_ok()
    };

    if !valid || flags & FP_INVALID != 0 {
        fpu::unimplemented(n64);
        return;
    }

    if !update_fp_status(n64, flags & FP_INEXACT) {
        return;
    }

    write_fp_result(
        n64,
        destination,
        if long {
            value as u64
        } else {
            value as i32 as u32 as u64
        },
        long,
    );
}

fn execute_operation(
    n64: &mut N64,
    instruction: cpu::Instruction,
    operation: FloatOperation,
    latency: &mut u64,
) {
    let function = operation.function;
    let double = operation.double();
    let wide = fpu::is_fr_mode(n64);
    let source = fp_source_index(instruction.rd, wide);
    let destination = instruction.shift_amount() as usize;
    let registers = &n64.fpu().registers;
    let a_bits =
        registers.operand_bits(source, double || operation.format == Cop1Opcode::Long, wide);
    let b_bits = registers.target_bits(instruction.rt, double, wide);
    *latency = latency::cycles(operation, a_bits, b_bits);
    if function == FpuFunction::Mov {
        // MOV.S, like MOV.D, copies the full physical register and leaves FCSR.
        let value = fpu::get_fpr_double(n64, source);
        write_fp_result(n64, destination, value.to_bits(), true);
        return;
    }

    n64.fpu_mut().control.clear_causes();
    let rounding = n64.fpu().control.rounding_mode();

    if operation.comparison() {
        execute_comparison(n64, operation, a_bits, b_bits);
        return;
    }

    let a_nan = is_nan_bits(a_bits, double);
    let b_nan = is_nan_bits(b_bits, double);
    let binary = operation.binary();

    if operation.floating_source() {
        let encoding = FloatEncoding::for_precision(double);

        if encoding.unsupported_arithmetic_operand(a_bits)
            || (binary && encoding.unsupported_arithmetic_operand(b_bits))
        {
            fpu::unimplemented(n64);
            return;
        }
    } else if operation.format == Cop1Opcode::Long
        && !(-INTEGER_CONVERSION_LIMIT..INTEGER_CONVERSION_LIMIT).contains(&(a_bits as i64))
    {
        fpu::unimplemented(n64);
        return;
    }

    if operation.integer_result() {
        execute_integer_conversion(n64, operation, a_bits, destination, rounding);
        return;
    }

    let result_double = operation.result_double();
    let result_encoding = FloatEncoding::for_precision(result_double);
    let sign = result_encoding.sign;
    let canonical_nan = result_encoding.canonical_nan;
    let (bits, flags) = if operation.floating_source() && (a_nan || (binary && b_nan)) {
        (canonical_nan, FP_INVALID)
    } else if function == FpuFunction::Abs {
        (a_bits & !sign, 0)
    } else if function == FpuFunction::Neg {
        (a_bits ^ sign, 0)
    } else {
        float_backend::execute_operation(operation, a_bits, b_bits, rounding)
    };

    let mut result = FloatResult::new(bits, flags, result_encoding);

    if result.needs_flush() {
        if function == FpuFunction::Mul && !n64.fpu().control.flush_exceptions_enabled() {
            *latency = 2;
        }
        // Underflow can only be handled with FS=1 and both U/I masked. Directed
        // rounding flushes outward to the minimum normal, otherwise signed zero.
        if !n64.fpu().control.can_flush_underflow() {
            fpu::unimplemented(n64);
            return;
        }

        result.flush(rounding);
    }

    if !update_fp_status(n64, result.flags) {
        return;
    }

    write_fp_result(n64, destination, result.bits, result_double);
}

pub fn add_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::Add);
}

pub fn sub_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::Sub);
}

pub fn mul_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::Mul);
}

pub fn div_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::Div);
}

pub fn sqrt_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::Sqrt);
}

pub fn abs_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::Abs);
}

pub fn mov_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::Mov);
}

pub fn neg_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::Neg);
}

pub fn round_l_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::RoundL);
}

pub fn trunc_l_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::TruncL);
}

pub fn ceil_l_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CeilL);
}

pub fn floor_l_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::FloorL);
}

pub fn round_w_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::RoundW);
}

pub fn trunc_w_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::TruncW);
}

pub fn ceil_w_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CeilW);
}

pub fn floor_w_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::FloorW);
}

pub fn cvt_w_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CvtW);
}

pub fn cvt_l_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CvtL);
}

pub fn cvt_d_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CvtD);
}

pub fn c_f_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::Cf);
}

pub fn c_un_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CUn);
}

pub fn c_eq_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CEq);
}

pub fn c_ueq_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CUeq);
}

pub fn c_olt_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::COlt);
}

pub fn c_ult_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CUlt);
}

pub fn c_ole_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::COle);
}

pub fn c_ule_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CUle);
}

pub fn c_sf_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CSf);
}

pub fn c_ngle_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CNgle);
}

pub fn c_seq_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CSeq);
}

pub fn c_ngl_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CNgl);
}

pub fn c_lt_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CLt);
}

pub fn c_nge_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CNge);
}

pub fn c_le_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CLe);
}

pub fn c_ngt_s(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Single, FpuFunction::CNgt);
}

pub fn add_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::Add);
}

pub fn sub_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::Sub);
}

pub fn mul_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::Mul);
}

pub fn div_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::Div);
}

pub fn sqrt_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::Sqrt);
}

pub fn abs_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::Abs);
}

pub fn mov_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::Mov);
}

pub fn neg_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::Neg);
}

pub fn round_l_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::RoundL);
}

pub fn trunc_l_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::TruncL);
}

pub fn ceil_l_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CeilL);
}

pub fn floor_l_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::FloorL);
}

pub fn round_w_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::RoundW);
}

pub fn trunc_w_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::TruncW);
}

pub fn ceil_w_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CeilW);
}

pub fn floor_w_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::FloorW);
}

pub fn cvt_w_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CvtW);
}

pub fn cvt_l_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CvtL);
}

pub fn cvt_s_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CvtS);
}

pub fn c_f_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::Cf);
}

pub fn c_un_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CUn);
}

pub fn c_eq_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CEq);
}

pub fn c_ueq_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CUeq);
}

pub fn c_olt_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::COlt);
}

pub fn c_ult_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CUlt);
}

pub fn c_ole_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::COle);
}

pub fn c_ule_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CUle);
}

pub fn c_sf_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CSf);
}

pub fn c_ngle_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CNgle);
}

pub fn c_seq_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CSeq);
}

pub fn c_ngl_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CNgl);
}

pub fn c_lt_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CLt);
}

pub fn c_nge_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CNge);
}

pub fn c_le_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CLe);
}

pub fn c_ngt_d(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Double, FpuFunction::CNgt);
}

pub fn cvt_s_w(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Word, FpuFunction::CvtS);
}

pub fn cvt_d_w(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Word, FpuFunction::CvtD);
}

pub fn cvt_s_l(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Long, FpuFunction::CvtS);
}

pub fn cvt_d_l(n64: &mut N64, opcode: u32) {
    execute_float(n64, opcode, Cop1Opcode::Long, FpuFunction::CvtD);
}
