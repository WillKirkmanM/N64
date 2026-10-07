use super::{handle_move, vector_reserved};
use crate::{opcode::Cop2Opcode, N64};

pub(super) fn main(encoding: usize) -> fn(&mut N64, u32) {
    match Cop2Opcode::from_encoding(encoding) {
        Some(Cop2Opcode::Mfc2 | Cop2Opcode::Cfc2) => mfc2,
        Some(Cop2Opcode::Dmfc2) => dmfc2,
        Some(Cop2Opcode::Mtc2 | Cop2Opcode::Dmtc2 | Cop2Opcode::Ctc2) => mtc2,
        _ => vector_reserved,
    }
}

fn mfc2(d: &mut N64, op: u32) {
    handle_move(d, op, false, false);
}

fn dmfc2(d: &mut N64, op: u32) {
    handle_move(d, op, true, false);
}

fn mtc2(d: &mut N64, op: u32) {
    handle_move(d, op, false, true);
}
