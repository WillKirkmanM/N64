use super::{de, get_vpr_element, modify_vpr_element, vd, ve, vt, Rsp};
use crate::rsp_vector::{shuffle_bytes, vector_from_bits};

const INPUT_SIGN_SHIFT: u32 = 31;
const HALFWORD_BITS: u32 = 16;
const TABLE_INDEX_MASK: u64 = 0x7fc0_0000;
const TABLE_INDEX_SHIFT: u32 = 22;
const SQUARE_ROOT_INDEX_MASK: u32 = 0x1fe;
const ESTIMATE_IMPLICIT_BIT: u32 = 0x1_0000;
const ESTIMATE_SHIFT: u32 = 14;
const ZERO_INPUT_RESULT: u32 = i32::MAX as u32;
const MIN_HALFWORD_RESULT: u32 = 0xffff_0000;

pub(super) enum DivisionKind {
    Reciprocal,
    SquareRoot,
}

pub(super) fn divide(rsp: &mut Rsp, opcode: u32, kind: DivisionKind, paired: bool) {
    let source = shuffle_bytes(
        vector_from_bits(rsp.registers.vpr[vt(opcode) as usize]),
        rsp.vector.shuffle[ve(opcode) as usize],
    );
    let low = get_vpr_element(rsp.registers.vpr[vt(opcode) as usize], ve(opcode) as u8);
    let input = if paired && rsp.division.divdp {
        (rsp.division.divin as i32) << HALFWORD_BITS | low as i32
    } else {
        low as i16 as i32
    };

    let result = estimate(rsp, input, kind);
    rsp.division.divdp = false;
    rsp.division.divout = (result >> HALFWORD_BITS) as i16;
    rsp.accumulator.accl = source;
    modify_vpr_element(
        &mut rsp.registers.vpr[vd(opcode) as usize],
        result as u16,
        de(opcode) as u8,
    );
}

fn estimate(rsp: &Rsp, input: i32, kind: DivisionKind) -> u32 {
    let sign = input >> INPUT_SIGN_SHIFT;
    let mut data = input ^ sign;

    if input > i16::MIN as i32 {
        data -= sign;
    }

    if data == 0 {
        return ZERO_INPUT_RESULT;
    }

    if input == i16::MIN as i32 {
        return MIN_HALFWORD_RESULT;
    }

    let shift = (data as u32).leading_zeros();
    let index = (((data as u64) << shift) & TABLE_INDEX_MASK) as u32 >> TABLE_INDEX_SHIFT;
    let (table_value, output_shift) = match kind {
        DivisionKind::Reciprocal => (
            rsp.vector.reciprocals[index as usize],
            INPUT_SIGN_SHIFT - shift,
        ),
        DivisionKind::SquareRoot => (
            rsp.vector.inverse_square_roots
                [((index & SQUARE_ROOT_INDEX_MASK) | (shift & 1)) as usize],
            (INPUT_SIGN_SHIFT - shift) >> 1,
        ),
    };

    let result = (ESTIMATE_IMPLICIT_BIT | table_value as u32) << ESTIMATE_SHIFT;

    (result >> output_shift) ^ sign as u32
}

pub(super) fn latch_high(rsp: &mut Rsp, opcode: u32) {
    rsp.accumulator.accl = shuffle_bytes(
        vector_from_bits(rsp.registers.vpr[vt(opcode) as usize]),
        rsp.vector.shuffle[ve(opcode) as usize],
    );
    rsp.division.divdp = true;
    rsp.division.divin =
        get_vpr_element(rsp.registers.vpr[vt(opcode) as usize], ve(opcode) as u8) as i16;
    modify_vpr_element(
        &mut rsp.registers.vpr[vd(opcode) as usize],
        rsp.division.divout as u16,
        de(opcode) as u8,
    );
}
