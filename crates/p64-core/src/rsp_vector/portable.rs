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

#[inline(always)]
fn lanes(x: Vector) -> [u16; LANE_COUNT] {
    x.0.to_array().map(|x| x as u16)
}

#[inline(always)]
fn pack(x: [u16; LANE_COUNT]) -> Vector {
    Vector(wide::i16x8::new(x.map(|x| x as i16)))
}

#[inline(always)]
pub fn zero() -> Vector {
    Vector(wide::i16x8::ZERO)
}

#[inline(always)]
pub fn splat_bytes(a: i8) -> Vector {
    Vector::from_le_bytes([a as u8; VECTOR_BYTES])
}

#[inline(always)]
pub fn splat_words(a: i32) -> Vector {
    let bytes = a.to_le_bytes();

    Vector::from_le_bytes(core::array::from_fn(|i| bytes[i % WORD_BYTES]))
}

#[inline(always)]
pub fn bit_and(a: Vector, b: Vector) -> Vector {
    a & b
}

#[inline(always)]
pub fn bit_or(a: Vector, b: Vector) -> Vector {
    a | b
}

#[inline(always)]
pub fn bit_xor(a: Vector, b: Vector) -> Vector {
    a ^ b
}

#[inline(always)]
pub fn and_not(a: Vector, b: Vector) -> Vector {
    !a & b
}

#[inline(always)]
pub fn add_wrapping(a: Vector, b: Vector) -> Vector {
    Vector(a.0 + b.0)
}

#[inline(always)]
pub fn sub_wrapping(a: Vector, b: Vector) -> Vector {
    Vector(a.0 - b.0)
}
