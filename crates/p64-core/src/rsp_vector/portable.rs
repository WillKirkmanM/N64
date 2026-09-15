use super::types::{BYTE_BITS, LANE_BITS, LANE_COUNT, VECTOR_BYTES, WORD_BYTES};
pub use super::Vector;

const BYTE_SIGN_SHIFT: usize = BYTE_BITS - 1;
const BYTE_SIGN_MASK: u8 = 1 << BYTE_SIGN_SHIFT;
const SHUFFLE_INDEX_MASK: u8 = VECTOR_BYTES as u8 - 1;
const HALF_LANE_COUNT: usize = LANE_COUNT / 2;

#[inline(always)]
pub fn vector_from_bits(bits: u128) -> Vector {
    Vector::from_le_bytes(bits.to_le_bytes())
}

#[inline(always)]
pub fn vector_to_bits(value: Vector) -> u128 {
    u128::from_le_bytes(value.to_le_bytes())
}
