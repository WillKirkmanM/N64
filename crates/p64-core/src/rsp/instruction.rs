use crate::{opcode::PrimaryOpcode, N64};

pub(super) type InstrFn = fn(&mut N64, u32);

pub struct InstructionState {
    pub instructions: [Instructions; 0x1000 / 4],
    pub last_instruction_type: InstructionType,
    pub instruction_type: InstructionType,
}

pub struct InstructionTables {
    pub special_table: [InstrFn; 64],
    pub register_immediate_table: [InstrFn; 32],
    pub memory_management_table: [InstrFn; 32],
    pub rsp_table: [InstrFn; 32],
    pub load_word_rsp_table: [InstrFn; 32],
    pub store_word_rsp_table: [InstrFn; 32],
    pub primary_table: [InstrFn; 64],
    pub vector_table: [InstrFn; 64],
}

impl Default for InstructionTables {
    fn default() -> Self {
        let default_instruction: InstrFn = |_n64, _insn| {};

        Self {
            special_table: [default_instruction; 64],
            register_immediate_table: [default_instruction; 32],
            memory_management_table: [default_instruction; 32],
            rsp_table: [default_instruction; 32],
            load_word_rsp_table: [default_instruction; 32],
            store_word_rsp_table: [default_instruction; 32],
            primary_table: [default_instruction; 64],
            vector_table: [default_instruction; 64],
        }
    }
}

impl Default for InstructionState {
    fn default() -> Self {
        Self {
            instructions: [Instructions::default(); 0x1000 / 4],
            last_instruction_type: InstructionType::default(),
            instruction_type: InstructionType::default(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Instructions {
    pub func: fn(&mut N64, u32),
    pub opcode: u32,
}

impl Default for Instructions {
    fn default() -> Self {
        fn default_func(_device: &mut N64, _value: u32) {}

        Instructions {
            func: default_func,
            opcode: 0,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Default)]
pub enum InstructionType {
    #[default]
    Su,
    Vu,
}

#[derive(Debug)]
pub enum OpcodeType {
    Special,
    RegImm,
    MMU,
    Cop2,
    Lwc2,
    Swc2,
    Other(u8),
}

impl From<u32> for OpcodeType {
    fn from(opcode: u32) -> Self {
        match PrimaryOpcode::from_encoding((opcode >> 26) as usize) {
            Some(PrimaryOpcode::Special) => OpcodeType::Special,
            Some(PrimaryOpcode::RegImm) => OpcodeType::RegImm,
            Some(PrimaryOpcode::Cop0) => OpcodeType::MMU,
            Some(PrimaryOpcode::Cop2) => OpcodeType::Cop2,
            Some(PrimaryOpcode::Lwc2) => OpcodeType::Lwc2,
            Some(PrimaryOpcode::Swc2) => OpcodeType::Swc2,
            _ => OpcodeType::Other((opcode >> 26) as u8),
        }
    }
}

impl InstructionTables {
    pub(super) fn decode(&self, opcode: u32) -> InstrFn {
        let rs = crate::cpu::rs(opcode) as usize;
        let rt = crate::cpu::rt(opcode) as usize;
        let rd = crate::cpu::rd(opcode) as usize;

        match OpcodeType::from(opcode) {
            OpcodeType::Special => self.special_table[(opcode & 0x3F) as usize],
            OpcodeType::RegImm => self.register_immediate_table[rt],
            OpcodeType::MMU => self.memory_management_table[rs],
            OpcodeType::Cop2 => self.rsp_table[rs],
            OpcodeType::Lwc2 => self.load_word_rsp_table[rd],
            OpcodeType::Swc2 => self.store_word_rsp_table[rd],
            OpcodeType::Other(op) => self.primary_table[op as usize],
        }
    }
}
