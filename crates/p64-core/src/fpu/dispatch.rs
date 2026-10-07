use super::{self as fpu, ArithmeticInstructionTable};
use crate::{
    opcode::{Cop1Opcode, FpuBranchOpcode, FpuFunction},
    rcp, N64,
};

type Handler = fn(&mut N64, u32);

pub(super) fn main(encoding: usize) -> Handler {
    match Cop1Opcode::from_encoding(encoding) {
        Some(Cop1Opcode::Mfc1) => fpu::mfc1,
        Some(Cop1Opcode::Dmfc1) => fpu::dmfc1,
        Some(Cop1Opcode::Cfc1) => fpu::cfc1,
        Some(Cop1Opcode::Dcfc1 | Cop1Opcode::Dctc1) => fpu::unsupported_control_move,
        Some(Cop1Opcode::Mtc1) => fpu::mtc1,
        Some(Cop1Opcode::Dmtc1) => fpu::dmtc1,
        Some(Cop1Opcode::Ctc1) => fpu::ctc1,
        Some(Cop1Opcode::Branch) => fpu::execute_fpu_b,
        Some(Cop1Opcode::Single) => fpu::execute_fpu_s,
        Some(Cop1Opcode::Double) => fpu::execute_fpu_d,
        Some(Cop1Opcode::Word) => fpu::execute_fpu_w,
        Some(Cop1Opcode::Long) => fpu::execute_fpu_l,
        _ => fpu::reserved,
    }
}

pub(super) fn branch(encoding: usize) -> Handler {
    match FpuBranchOpcode::from_encoding(encoding) {
        Some(FpuBranchOpcode::Bc1f) => rcp::bc1f,
        Some(FpuBranchOpcode::Bc1t) => rcp::bc1t,
        Some(FpuBranchOpcode::Bc1fl) => rcp::bc1fl,
        Some(FpuBranchOpcode::Bc1tl) => rcp::bc1tl,
        _ => fpu::reserved,
    }
}

pub(super) fn single(encoding: usize) -> Handler {
    match FpuFunction::from_encoding(encoding) {
        Some(FpuFunction::Add) => rcp::add_s,
        Some(FpuFunction::Sub) => rcp::sub_s,
        Some(FpuFunction::Mul) => rcp::mul_s,
        Some(FpuFunction::Div) => rcp::div_s,
        Some(FpuFunction::Sqrt) => rcp::sqrt_s,
        Some(FpuFunction::Abs) => rcp::abs_s,
        Some(FpuFunction::Mov) => rcp::mov_s,
        Some(FpuFunction::Neg) => rcp::neg_s,
        Some(FpuFunction::RoundL) => rcp::round_l_s,
        Some(FpuFunction::TruncL) => rcp::trunc_l_s,
        Some(FpuFunction::CeilL) => rcp::ceil_l_s,
        Some(FpuFunction::FloorL) => rcp::floor_l_s,
        Some(FpuFunction::RoundW) => rcp::round_w_s,
        Some(FpuFunction::TruncW) => rcp::trunc_w_s,
        Some(FpuFunction::CeilW) => rcp::ceil_w_s,
        Some(FpuFunction::FloorW) => rcp::floor_w_s,
        Some(FpuFunction::CvtD) => rcp::cvt_d_s,
        Some(FpuFunction::CvtW) => rcp::cvt_w_s,
        Some(FpuFunction::CvtL) => rcp::cvt_l_s,
        Some(FpuFunction::Cf) => rcp::c_f_s,
        Some(FpuFunction::CUn) => rcp::c_un_s,
        Some(FpuFunction::CEq) => rcp::c_eq_s,
        Some(FpuFunction::CUeq) => rcp::c_ueq_s,
        Some(FpuFunction::COlt) => rcp::c_olt_s,
        Some(FpuFunction::CUlt) => rcp::c_ult_s,
        Some(FpuFunction::COle) => rcp::c_ole_s,
        Some(FpuFunction::CUle) => rcp::c_ule_s,
        Some(FpuFunction::CSf) => rcp::c_sf_s,
        Some(FpuFunction::CNgle) => rcp::c_ngle_s,
        Some(FpuFunction::CSeq) => rcp::c_seq_s,
        Some(FpuFunction::CNgl) => rcp::c_ngl_s,
        Some(FpuFunction::CLt) => rcp::c_lt_s,
        Some(FpuFunction::CNge) => rcp::c_nge_s,
        Some(FpuFunction::CLe) => rcp::c_le_s,
        Some(FpuFunction::CNgt) => rcp::c_ngt_s,
        _ => fpu::reserved,
    }
}

pub(super) fn double(encoding: usize) -> Handler {
    match FpuFunction::from_encoding(encoding) {
        Some(FpuFunction::Add) => rcp::add_d,
        Some(FpuFunction::Sub) => rcp::sub_d,
        Some(FpuFunction::Mul) => rcp::mul_d,
        Some(FpuFunction::Div) => rcp::div_d,
        Some(FpuFunction::Sqrt) => rcp::sqrt_d,
        Some(FpuFunction::Abs) => rcp::abs_d,
        Some(FpuFunction::Mov) => rcp::mov_d,
        Some(FpuFunction::Neg) => rcp::neg_d,
        Some(FpuFunction::RoundL) => rcp::round_l_d,
        Some(FpuFunction::TruncL) => rcp::trunc_l_d,
        Some(FpuFunction::CeilL) => rcp::ceil_l_d,
        Some(FpuFunction::FloorL) => rcp::floor_l_d,
        Some(FpuFunction::RoundW) => rcp::round_w_d,
        Some(FpuFunction::TruncW) => rcp::trunc_w_d,
        Some(FpuFunction::CeilW) => rcp::ceil_w_d,
        Some(FpuFunction::FloorW) => rcp::floor_w_d,
        Some(FpuFunction::CvtS) => rcp::cvt_s_d,
        Some(FpuFunction::CvtW) => rcp::cvt_w_d,
        Some(FpuFunction::CvtL) => rcp::cvt_l_d,
        Some(FpuFunction::Cf) => rcp::c_f_d,
        Some(FpuFunction::CUn) => rcp::c_un_d,
        Some(FpuFunction::CEq) => rcp::c_eq_d,
        Some(FpuFunction::CUeq) => rcp::c_ueq_d,
        Some(FpuFunction::COlt) => rcp::c_olt_d,
        Some(FpuFunction::CUlt) => rcp::c_ult_d,
        Some(FpuFunction::COle) => rcp::c_ole_d,
        Some(FpuFunction::CUle) => rcp::c_ule_d,
        Some(FpuFunction::CSf) => rcp::c_sf_d,
        Some(FpuFunction::CNgle) => rcp::c_ngle_d,
        Some(FpuFunction::CSeq) => rcp::c_seq_d,
        Some(FpuFunction::CNgl) => rcp::c_ngl_d,
        Some(FpuFunction::CLt) => rcp::c_lt_d,
        Some(FpuFunction::CNge) => rcp::c_nge_d,
        Some(FpuFunction::CLe) => rcp::c_le_d,
        Some(FpuFunction::CNgt) => rcp::c_ngt_d,
        _ => fpu::reserved,
    }
}

pub(super) fn long(encoding: usize) -> Handler {
    match FpuFunction::from_encoding(encoding) {
        Some(FpuFunction::CvtS) => rcp::cvt_s_l,
        Some(FpuFunction::CvtD) => rcp::cvt_d_l,
        _ => fpu::reserved,
    }
}

pub(super) fn word(encoding: usize) -> Handler {
    match FpuFunction::from_encoding(encoding) {
        Some(FpuFunction::CvtS) => rcp::cvt_s_w,
        Some(FpuFunction::CvtD) => rcp::cvt_d_w,
        _ => fpu::reserved,
    }
}

pub(super) enum Format {
    Single,
    Double,
    Word,
    Long,
}

impl Format {
    pub(super) fn instructions(self, fpu: &fpu::Fpu) -> &ArithmeticInstructionTable {
        match self {
            Self::Single => &fpu.instructions.s_instrs,
            Self::Double => &fpu.instructions.d_instrs,
            Self::Word => &fpu.instructions.w_instrs,
            Self::Long => &fpu.instructions.l_instrs,
        }
    }
}
