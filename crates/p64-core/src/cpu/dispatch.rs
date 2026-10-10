use super::{
    alu, cache_ops, control_flow, hilo, instruction::Instruction, memory_ops, traps, types::Cpu,
    InstrFn,
};
use crate::opcode::{PrimaryOpcode, RegImmOpcode, SpecialOpcode};
use crate::{fpu, mmu, rcp, N64};
use tracing::info;

const CPU_CLOCK_HZ: u64 = 93_750_000;

fn primary(encoding: usize) -> InstrFn {
    match PrimaryOpcode::from_encoding(encoding) {
        Some(PrimaryOpcode::J) => control_flow::j,
        Some(PrimaryOpcode::Jal) => control_flow::jal,
        Some(PrimaryOpcode::Beq) => control_flow::beq,
        Some(PrimaryOpcode::Bne) => control_flow::bne,
        Some(PrimaryOpcode::Blez) => control_flow::blez,
        Some(PrimaryOpcode::Bgtz) => control_flow::bgtz,
        Some(PrimaryOpcode::Addi) => alu::addi,
        Some(PrimaryOpcode::Addiu) => alu::addiu,
        Some(PrimaryOpcode::Slti) => alu::slti,
        Some(PrimaryOpcode::Sltiu) => alu::sltiu,
        Some(PrimaryOpcode::Andi) => alu::andi,
        Some(PrimaryOpcode::Ori) => alu::ori,
        Some(PrimaryOpcode::Xori) => alu::xori,
        Some(PrimaryOpcode::Lui) => alu::lui,
        Some(PrimaryOpcode::Beql) => control_flow::beql,
        Some(PrimaryOpcode::Bnel) => control_flow::bnel,
        Some(PrimaryOpcode::Blezl) => control_flow::blezl,
        Some(PrimaryOpcode::Bgtzl) => control_flow::bgtzl,
        Some(PrimaryOpcode::Daddi) => alu::daddi,
        Some(PrimaryOpcode::Daddiu) => alu::daddiu,
        Some(PrimaryOpcode::Ldl) => memory_ops::ldl,
        Some(PrimaryOpcode::Ldr) => memory_ops::ldr,
        Some(PrimaryOpcode::Lb) => memory_ops::lb,
        Some(PrimaryOpcode::Lh) => memory_ops::lh,
        Some(PrimaryOpcode::Lwl) => memory_ops::lwl,
        Some(PrimaryOpcode::Lw) => memory_ops::lw,
        Some(PrimaryOpcode::Lbu) => memory_ops::lbu,
        Some(PrimaryOpcode::Lhu) => memory_ops::lhu,
        Some(PrimaryOpcode::Lwr) => memory_ops::lwr,
        Some(PrimaryOpcode::Lwu) => memory_ops::lwu,
        Some(PrimaryOpcode::Sb) => memory_ops::sb,
        Some(PrimaryOpcode::Sh) => memory_ops::sh,
        Some(PrimaryOpcode::Swl) => memory_ops::swl,
        Some(PrimaryOpcode::Sw) => memory_ops::sw,
        Some(PrimaryOpcode::Sdl) => memory_ops::sdl,
        Some(PrimaryOpcode::Sdr) => memory_ops::sdr,
        Some(PrimaryOpcode::Swr) => memory_ops::swr,
        Some(PrimaryOpcode::Cache) => cache_ops::cache,
        Some(PrimaryOpcode::Ll) => memory_ops::ll,
        Some(PrimaryOpcode::Lwc1) => fpu::lwc1,
        Some(PrimaryOpcode::Lwc2) => rcp::lwc2,
        Some(PrimaryOpcode::Lld) => memory_ops::lld,
        Some(PrimaryOpcode::Ldc1) => fpu::ldc1,
        Some(PrimaryOpcode::Ldc2) => rcp::ldc2,
        Some(PrimaryOpcode::Ld) => memory_ops::ld,
        Some(PrimaryOpcode::Sc) => memory_ops::sc,
        Some(PrimaryOpcode::Swc1) => fpu::swc1,
        Some(PrimaryOpcode::Swc2) => rcp::swc2,
        Some(PrimaryOpcode::Scd) => memory_ops::scd,
        Some(PrimaryOpcode::Sdc1) => fpu::sdc1,
        Some(PrimaryOpcode::Sdc2) => rcp::sdc2,
        Some(PrimaryOpcode::Sd) => memory_ops::sd,
        _ => mmu::reserved,
    }
}

fn special(encoding: usize) -> InstrFn {
    match SpecialOpcode::from_encoding(encoding) {
        Some(SpecialOpcode::Sll) => alu::sll,
        Some(SpecialOpcode::Srl) => alu::srl,
        Some(SpecialOpcode::Sra) => alu::sra,
        Some(SpecialOpcode::Sllv) => alu::sllv,
        Some(SpecialOpcode::Srlv) => alu::srlv,
        Some(SpecialOpcode::Srav) => alu::srav,
        Some(SpecialOpcode::Jr) => control_flow::jr,
        Some(SpecialOpcode::Jalr) => control_flow::jalr,
        Some(SpecialOpcode::Syscall) => mmu::syscall,
        Some(SpecialOpcode::Break) => mmu::break_,
        Some(SpecialOpcode::Sync) => hilo::sync,
        Some(SpecialOpcode::Mfhi) => hilo::mfhi,
        Some(SpecialOpcode::Mthi) => hilo::mthi,
        Some(SpecialOpcode::Mflo) => hilo::mflo,
        Some(SpecialOpcode::Mtlo) => hilo::mtlo,
        Some(SpecialOpcode::Dsllv) => alu::dsllv,
        Some(SpecialOpcode::Dsrlv) => alu::dsrlv,
        Some(SpecialOpcode::Dsrav) => alu::dsrav,
        Some(SpecialOpcode::Mult) => hilo::mult,
        Some(SpecialOpcode::Multu) => hilo::multu,
        Some(SpecialOpcode::Div) => hilo::div,
        Some(SpecialOpcode::Divu) => hilo::divu,
        Some(SpecialOpcode::Dmult) => hilo::dmult,
        Some(SpecialOpcode::Dmultu) => hilo::dmultu,
        Some(SpecialOpcode::Ddiv) => hilo::ddiv,
        Some(SpecialOpcode::Ddivu) => hilo::ddivu,
        Some(SpecialOpcode::Add) => alu::add,
        Some(SpecialOpcode::Addu) => alu::addu,
        Some(SpecialOpcode::Sub) => alu::sub,
        Some(SpecialOpcode::Subu) => alu::subu,
        Some(SpecialOpcode::And) => alu::and,
        Some(SpecialOpcode::Or) => alu::or,
        Some(SpecialOpcode::Xor) => alu::xor,
        Some(SpecialOpcode::Nor) => alu::nor,
        Some(SpecialOpcode::Slt) => alu::slt,
        Some(SpecialOpcode::Sltu) => alu::sltu,
        Some(SpecialOpcode::Dadd) => alu::dadd,
        Some(SpecialOpcode::Daddu) => alu::daddu,
        Some(SpecialOpcode::Dsub) => alu::dsub,
        Some(SpecialOpcode::Dsubu) => alu::dsubu,
        Some(SpecialOpcode::Tge) => traps::tge,
        Some(SpecialOpcode::Tgeu) => traps::tgeu,
        Some(SpecialOpcode::Tlt) => traps::tlt,
        Some(SpecialOpcode::Tltu) => traps::tltu,
        Some(SpecialOpcode::Teq) => traps::teq,
        Some(SpecialOpcode::Tne) => traps::tne,
        Some(SpecialOpcode::Dsll) => alu::dsll,
        Some(SpecialOpcode::Dsrl) => alu::dsrl,
        Some(SpecialOpcode::Dsra) => alu::dsra,
        Some(SpecialOpcode::Dsll32) => alu::dsll32,
        Some(SpecialOpcode::Dsrl32) => alu::dsrl32,
        Some(SpecialOpcode::Dsra32) => alu::dsra32,
        _ => mmu::reserved,
    }
}

fn register_immediate(encoding: usize) -> InstrFn {
    match RegImmOpcode::from_encoding(encoding) {
        Some(RegImmOpcode::Bltz) => control_flow::bltz,
        Some(RegImmOpcode::Bgez) => control_flow::bgez,
        Some(RegImmOpcode::Bltzl) => control_flow::bltzl,
        Some(RegImmOpcode::Bgezl) => control_flow::bgezl,
        Some(RegImmOpcode::Tgei) => traps::tgei,
        Some(RegImmOpcode::Tgeiu) => traps::tgeiu,
        Some(RegImmOpcode::Tlti) => traps::tlti,
        Some(RegImmOpcode::Tltiu) => traps::tltiu,
        Some(RegImmOpcode::Teqi) => traps::teqi,
        Some(RegImmOpcode::Tnei) => traps::tnei,
        Some(RegImmOpcode::Bltzal) => control_flow::bltzal,
        Some(RegImmOpcode::Bgezal) => control_flow::bgezal,
        Some(RegImmOpcode::Bltzall) => control_flow::bltzall,
        Some(RegImmOpcode::Bgezall) => control_flow::bgezall,
        _ => mmu::reserved,
    }
}

impl Cpu {
    fn initialize_instructions(&mut self) {
        self.instructions.instrs = std::array::from_fn(primary);
        self.instructions.special_instrs = std::array::from_fn(special);
        self.instructions.regimm_instrs = std::array::from_fn(register_immediate);
    }

    fn integer_handler(&self, instruction: Instruction) -> InstrFn {
        match PrimaryOpcode::from_encoding(instruction.primary() as usize) {
            Some(PrimaryOpcode::Special) => {
                self.instructions.special_instrs[instruction.function()]
            }
            Some(PrimaryOpcode::RegImm) => self.instructions.regimm_instrs[instruction.rt],
            _ => self.instructions.instrs[instruction.primary() as usize],
        }
    }
}

pub fn decode_opcode(n64: &N64, opcode: u32) -> InstrFn {
    let instruction = Instruction::decode(opcode);

    if instruction.requires_wide() {
        return execute_wide_instruction;
    }

    match PrimaryOpcode::from_encoding(instruction.primary() as usize) {
        Some(PrimaryOpcode::Cop0) => mmu::execute_checked_cp0,
        Some(PrimaryOpcode::Cop1) => n64.fpu().instructions.instrs[instruction.rs],
        Some(PrimaryOpcode::Cop2) => n64.rcp().instrs[instruction.rs],
        _ => n64.cpu().integer_handler(instruction),
    }
}

fn execute_wide_instruction(n64: &mut N64, opcode: u32) {
    if !n64.cp0().status().wide_instructions_allowed() {
        mmu::reserved(n64, opcode);
        return;
    }

    let handler = n64.cpu().integer_handler(Instruction::decode(opcode));
    handler(n64, opcode);
}

pub fn start_cpu(n64: &mut N64) {
    info!("Initializing CPU instruction tables");
    let cpu = n64.cpu_mut();
    cpu.execution.clock_rate = CPU_CLOCK_HZ;
    cpu.initialize_instructions();
    mmu::write_mmu_registers(n64);
}
