use super::*;

pub(super) type InstrFn = fn(&mut N64, u32);

/// Instruction state
pub struct InstructionState {
    pub instructions: [Instructions; 0x1000 / 4],
    pub last_instruction_type: InstructionType,
    pub instruction_type: InstructionType,
}

/// Instruction lookup tables for different RSP operation types
pub struct InstructionTables {
    /// Special instructions (SPECIAL opcode)
    pub special_table: [InstrFn; 64],

    /// Register immediate instructions (REGIMM opcode)
    pub register_immediate_table: [InstrFn; 32],

    /// Memory Management Unit instructions
    pub memory_management_table: [InstrFn; 32],

    /// Coprocessor 2 instructions
    pub rsp_table: [InstrFn; 32],

    /// Load Word Coprocessor 2 instructions
    pub load_word_rsp_table: [InstrFn; 32],

    /// Store Word Coprocessor 2 instructions
    pub store_word_rsp_table: [InstrFn; 32],

    /// Primary instruction table
    pub primary_table: [InstrFn; 64],

    /// Vector instruction table
    pub vector_table: [InstrFn; 64],
}

#[derive(Copy, Clone)]
pub struct Instructions {
    pub func: fn(&mut N64, u32),
    pub opcode: u32,
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

impl Default for Instructions {
    fn default() -> Self {
        fn default_func(_device: &mut N64, _value: u32) {}

        Instructions {
            func: default_func,
            opcode: 0,
        }
    }
}

impl From<u32> for OpcodeType {
    fn from(opcode: u32) -> Self {
        match opcode >> 26 {
            0 => OpcodeType::Special,
            1 => OpcodeType::RegImm,
            16 => OpcodeType::MMU,
            18 => OpcodeType::Cop2,
            50 => OpcodeType::Lwc2,
            58 => OpcodeType::Swc2,
            other => OpcodeType::Other(other as u8),
        }
    }
}
