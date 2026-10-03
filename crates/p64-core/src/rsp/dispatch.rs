use super::{InstrFn, InstructionTables};
use crate::{
    opcode::{
        Cp0Opcode, PrimaryOpcode, RegImmOpcode, RspCop2Opcode, SpecialOpcode, VectorLoadOpcode,
        VectorOpcode, VectorStoreOpcode,
    },
};

impl InstructionTables {
    pub(super) fn initialized() -> Self {
        Self {
            primary_table: std::array::from_fn(primary),
            special_table: std::array::from_fn(special),
            register_immediate_table: std::array::from_fn(register_immediate),
            memory_management_table: std::array::from_fn(memory_management),
            rsp_table: std::array::from_fn(coprocessor),
            load_word_rsp_table: std::array::from_fn(vector_load),
            store_word_rsp_table: std::array::from_fn(vector_store),
            vector_table: std::array::from_fn(vector),
        }
    }
}

fn primary(opcode: usize) -> InstrFn {
    match PrimaryOpcode::from_encoding(opcode) {
        Some(PrimaryOpcode::J) => super::j,
        Some(PrimaryOpcode::Jal) => super::jal,
        Some(PrimaryOpcode::Beq) => super::beq,
        Some(PrimaryOpcode::Bne) => super::bne,
        Some(PrimaryOpcode::Blez) => super::blez,
        Some(PrimaryOpcode::Bgtz) => super::bgtz,
        Some(PrimaryOpcode::Addi) => super::addi,
        Some(PrimaryOpcode::Addiu) => super::addiu,
        Some(PrimaryOpcode::Slti) => super::slti,
        Some(PrimaryOpcode::Sltiu) => super::sltiu,
        Some(PrimaryOpcode::Andi) => super::andi,
        Some(PrimaryOpcode::Ori) => super::ori,
        Some(PrimaryOpcode::Xori) => super::xori,
        Some(PrimaryOpcode::Lui) => super::lui,
        Some(PrimaryOpcode::Lb) => super::lb,
        Some(PrimaryOpcode::Lh) => super::lh,
        Some(PrimaryOpcode::Lw) => super::lw,
        Some(PrimaryOpcode::Lbu) => super::lbu,
        Some(PrimaryOpcode::Lhu) => super::lhu,
        Some(PrimaryOpcode::Lwu) => super::lwu,
        Some(PrimaryOpcode::Sb) => super::sb,
        Some(PrimaryOpcode::Sh) => super::sh,
        Some(PrimaryOpcode::Sw) => super::sw,
        _ => reserved,
    }
}

fn reserved(_n64: &mut crate::N64, _opcode: u32) {}

fn special_reserved(n64: &mut crate::N64, opcode: u32) {
    let source = n64.rsp().registers.gpr[crate::cpu::rs(opcode) as usize];
    n64.rsp_mut().registers.gpr[crate::cpu::rd(opcode) as usize] = source >> (source & 31);
}

fn special(opcode: usize) -> InstrFn {
    match SpecialOpcode::from_encoding(opcode) {
        Some(SpecialOpcode::Sll) => super::sll,
        Some(SpecialOpcode::Srl) => super::srl,
        Some(SpecialOpcode::Sra) => super::sra,
        Some(SpecialOpcode::Sllv) => super::sllv,
        Some(SpecialOpcode::Srlv) => super::srlv,
        Some(SpecialOpcode::Srav) => super::srav,
        Some(SpecialOpcode::Jr) => super::jr,
        Some(SpecialOpcode::Jalr) => super::jalr,
        Some(SpecialOpcode::Break) => super::break_,
        Some(SpecialOpcode::Add) => super::add,
        Some(SpecialOpcode::Addu) => super::addu,
        Some(SpecialOpcode::Sub) => super::sub,
        Some(SpecialOpcode::Subu) => super::subu,
        Some(SpecialOpcode::And) => super::and,
        Some(SpecialOpcode::Or) => super::or,
        Some(SpecialOpcode::Xor) => super::xor,
        Some(SpecialOpcode::Nor) => super::nor,
        Some(SpecialOpcode::Slt) => super::slt,
        Some(SpecialOpcode::Sltu) => super::sltu,
        _ => special_reserved,
    }
}

fn register_immediate(opcode: usize) -> InstrFn {
    match RegImmOpcode::from_encoding(opcode) {
        Some(RegImmOpcode::Bltz) => super::bltz,
        Some(RegImmOpcode::Bgez) => super::bgez,
        Some(RegImmOpcode::Bltzal) => super::bltzal,
        Some(RegImmOpcode::Bgezal) => super::bgezal,
        _ => reserved,
    }
}

fn memory_management(opcode: usize) -> InstrFn {
    match Cp0Opcode::from_encoding(opcode) {
        Some(Cp0Opcode::Mfc0) => super::mfc0,
        Some(Cp0Opcode::Mtc0) => super::mtc0,
        _ => reserved,
    }
}

fn coprocessor(opcode: usize) -> InstrFn {
    match RspCop2Opcode::from_encoding(opcode) {
        Some(RspCop2Opcode::Mfc2) => super::mfc2,
        Some(RspCop2Opcode::Cfc2) => super::cfc2,
        Some(RspCop2Opcode::Mtc2) => super::mtc2,
        Some(RspCop2Opcode::Ctc2) => super::ctc2,
        Some(RspCop2Opcode::Vector) => super::execute_vec,
        _ => reserved,
    }
}

fn vector_load(opcode: usize) -> InstrFn {
    match VectorLoadOpcode::from_encoding(opcode) {
        Some(VectorLoadOpcode::Lbv) => super::lbv,
        Some(VectorLoadOpcode::Lsv) => super::lsv,
        Some(VectorLoadOpcode::Llv) => super::llv,
        Some(VectorLoadOpcode::Ldv) => super::ldv,
        Some(VectorLoadOpcode::Lqv) => super::lqv,
        Some(VectorLoadOpcode::Lrv) => super::lrv,
        Some(VectorLoadOpcode::Lpv) => super::lpv,
        Some(VectorLoadOpcode::Luv) => super::luv,
        Some(VectorLoadOpcode::Lhv) => super::lhv,
        Some(VectorLoadOpcode::Lfv) => super::lfv,
        Some(VectorLoadOpcode::Lwv) => super::lwv,
        Some(VectorLoadOpcode::Ltv) => super::ltv,
        _ => reserved,
    }
}

fn vector_store(opcode: usize) -> InstrFn {
    match VectorStoreOpcode::from_encoding(opcode) {
        Some(VectorStoreOpcode::Sbv) => super::sbv,
        Some(VectorStoreOpcode::Ssv) => super::ssv,
        Some(VectorStoreOpcode::Slv) => super::slv,
        Some(VectorStoreOpcode::Sdv) => super::sdv,
        Some(VectorStoreOpcode::Sqv) => super::sqv,
        Some(VectorStoreOpcode::Srv) => super::srv,
        Some(VectorStoreOpcode::Spv) => super::spv,
        Some(VectorStoreOpcode::Suv) => super::suv,
        Some(VectorStoreOpcode::Shv) => super::shv,
        Some(VectorStoreOpcode::Sfv) => super::sfv,
        Some(VectorStoreOpcode::Swv) => super::swv,
        Some(VectorStoreOpcode::Stv) => super::stv,
        _ => reserved,
    }
}

fn vector(opcode: usize) -> InstrFn {
    match VectorOpcode::from_encoding(opcode) {
        Some(VectorOpcode::Vmulf) => super::vmulf,
        Some(VectorOpcode::Vmulu) => super::vmulu,
        Some(VectorOpcode::Vrndp) => super::vrndp,
        Some(VectorOpcode::Vmulq) => super::vmulq,
        Some(VectorOpcode::Vmudl) => super::vmudl,
        Some(VectorOpcode::Vmudm) => super::vmudm,
        Some(VectorOpcode::Vmudn) => super::vmudn,
        Some(VectorOpcode::Vmudh) => super::vmudh,
        Some(VectorOpcode::Vmacf) => super::vmacf,
        Some(VectorOpcode::Vmacu) => super::vmacu,
        Some(VectorOpcode::Vrndn) => super::vrndn,
        Some(VectorOpcode::Vmacq) => super::vmacq,
        Some(VectorOpcode::Vmadl) => super::vmadl,
        Some(VectorOpcode::Vmadm) => super::vmadm,
        Some(VectorOpcode::Vmadn) => super::vmadn,
        Some(VectorOpcode::Vmadh) => super::vmadh,
        Some(VectorOpcode::Vadd) => super::vadd,
        Some(VectorOpcode::Vsub) => super::vsub,
        Some(VectorOpcode::Vzero) => super::vzero,
        Some(VectorOpcode::Vabs) => super::vabs,
        Some(VectorOpcode::Vaddc) => super::vaddc,
        Some(VectorOpcode::Vsubc) => super::vsubc,
        Some(VectorOpcode::Vsar) => super::vsar,
        Some(VectorOpcode::Vlt) => super::vlt,
        Some(VectorOpcode::Veq) => super::veq,
        Some(VectorOpcode::Vne) => super::vne,
        Some(VectorOpcode::Vge) => super::vge,
        Some(VectorOpcode::Vcl) => super::vcl,
        Some(VectorOpcode::Vch) => super::vch,
        Some(VectorOpcode::Vcr) => super::vcr,
        Some(VectorOpcode::Vmrg) => super::vmrg,
        Some(VectorOpcode::Vand) => super::vand,
        Some(VectorOpcode::Vnand) => super::vnand,
        Some(VectorOpcode::Vor) => super::vor,
        Some(VectorOpcode::Vnor) => super::vnor,
        Some(VectorOpcode::Vxor) => super::vxor,
        Some(VectorOpcode::Vnxor) => super::vnxor,
        Some(VectorOpcode::Vrcp) => super::vrcp,
        Some(VectorOpcode::Vrcpl) => super::vrcpl,
        Some(VectorOpcode::Vrcph) => super::vrcph,
        Some(VectorOpcode::Vmov) => super::vmov,
        Some(VectorOpcode::Vrsq) => super::vrsq,
        Some(VectorOpcode::Vrsql) => super::vrsql,
        Some(VectorOpcode::Vrsqh) => super::vrsqh,
        Some(VectorOpcode::Vnop | VectorOpcode::Vnull) => super::vnop,
        _ => super::vzero,
    }
}
