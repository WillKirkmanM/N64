use crate::{opcode::{Cp0Function, Cp0Opcode}, N64};

type Handler = fn(&mut N64, u32);
const TABLE_LEN: usize = 32;

pub const MMU_INSTRUCTIONS: [Handler; TABLE_LEN] = tables().0;
pub const MMU_SECONDARY_INSTRUCTIONS: [Handler; TABLE_LEN] = tables().1;

const fn primary(encoding: usize) -> Handler {
    match Cp0Opcode::from_encoding(encoding) {
        Some(Cp0Opcode::Mfc0) => super::mfc0,
        Some(Cp0Opcode::Dmfc0) => super::dmfc0,
        Some(Cp0Opcode::Mtc0) => super::mtc0,
        Some(Cp0Opcode::Dmtc0) => super::dmtc0,
        Some(Cp0Opcode::Cfc0 | Cp0Opcode::Ctc0 | Cp0Opcode::Bc0) => super::cp0_noop,
        // CO is bit25; the remaining rs bits are operands, not sub-opcodes.
        Some(Cp0Opcode::Co) => super::execute_cp0,
        _ => super::reserved,
    }
}

const fn secondary(encoding: usize) -> Handler {
    match Cp0Function::from_encoding(encoding) {
        Some(Cp0Function::Tlbr) => super::tlbr,
        Some(Cp0Function::Tlbwi) => super::tlbwi,
        Some(Cp0Function::Tlbwr) => super::tlbwr,
        Some(Cp0Function::Tlbp) => super::tlbp,
        Some(Cp0Function::Eret) => super::eret,
        // Unlike other unused CO functions, the former R3000 RFE slot traps.
        Some(Cp0Function::Rfe) => super::reserved,
        _ => super::cp0_noop,
    }
}

const fn tables() -> ([Handler; TABLE_LEN], [Handler; TABLE_LEN]) {
    let mut transfers = [super::reserved as Handler; TABLE_LEN];
    let mut functions = [super::cp0_noop as Handler; TABLE_LEN];
    let mut index = 0;

    while index < TABLE_LEN {
        transfers[index] = primary(index);
        functions[index] = secondary(index);
        index += 1;
    }

    (transfers, functions)
}
