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

#[inline(always)]
pub fn add_saturating_signed(a: Vector, b: Vector) -> Vector {
    Vector(a.0.saturating_add(b.0))
}

#[inline(always)]
pub fn sub_saturating_signed(a: Vector, b: Vector) -> Vector {
    Vector(a.0.saturating_sub(b.0))
}

#[inline(always)]
pub fn add_saturating_unsigned(a: Vector, b: Vector) -> Vector {
    Vector(
        a.0.cast_unsigned()
            .saturating_add(b.0.cast_unsigned())
            .cast_signed(),
    )
}

#[inline(always)]
pub fn sub_saturating_unsigned(a: Vector, b: Vector) -> Vector {
    Vector(
        a.0.cast_unsigned()
            .saturating_sub(b.0.cast_unsigned())
            .cast_signed(),
    )
}

#[inline(always)]
pub fn mul_low(a: Vector, b: Vector) -> Vector {
    Vector(a.0 * b.0)
}

#[inline(always)]
pub fn mul_high_signed(a: Vector, b: Vector) -> Vector {
    Vector(a.0.mul_keep_high(b.0))
}

#[inline(always)]
pub fn mul_high_unsigned(a: Vector, b: Vector) -> Vector {
    Vector(
        a.0.cast_unsigned()
            .mul_keep_high(b.0.cast_unsigned())
            .cast_signed(),
    )
}

#[inline(always)]
pub fn equal_lanes(a: Vector, b: Vector) -> Vector {
    Vector(a.0.simd_eq(b.0))
}

#[inline(always)]
pub fn greater_signed(a: Vector, b: Vector) -> Vector {
    Vector(a.0.simd_gt(b.0))
}

#[inline(always)]
pub fn less_signed(a: Vector, b: Vector) -> Vector {
    Vector(a.0.simd_lt(b.0))
}

#[inline(always)]
pub fn min_signed(a: Vector, b: Vector) -> Vector {
    Vector(a.0.min(b.0))
}

#[inline(always)]
pub fn max_signed(a: Vector, b: Vector) -> Vector {
    Vector(a.0.max(b.0))
}

#[inline(always)]
pub fn shift_left(a: Vector, n: i32) -> Vector {
    pack(lanes(a).map(|a| if n >= LANE_BITS as i32 { 0 } else { a << n }))
}

#[inline(always)]
pub fn shift_right_unsigned(a: Vector, n: i32) -> Vector {
    pack(lanes(a).map(|a| if n >= LANE_BITS as i32 { 0 } else { a >> n }))
}

#[inline(always)]
pub fn shift_right_signed(a: Vector, n: i32) -> Vector {
    pack(lanes(a).map(|a| ((a as i16) >> n.min(LANE_BITS as i32 - 1)) as u16))
}

#[inline(always)]
pub fn equal_bytes(a: Vector, b: Vector) -> Vector {
    let a = a.to_le_bytes();
    let b = b.to_le_bytes();

    Vector::from_le_bytes(core::array::from_fn(
        |i| if a[i] == b[i] { u8::MAX } else { 0 },
    ))
}

#[inline(always)]
pub fn shuffle_bytes(a: Vector, b: Vector) -> Vector {
    let a = a.to_le_bytes();
    let b = b.to_le_bytes();

    Vector::from_le_bytes(core::array::from_fn(|i| {
        if b[i] & BYTE_SIGN_MASK != 0 {
            0
        } else {
            a[(b[i] & SHUFFLE_INDEX_MASK) as usize]
        }
    }))
}

#[inline(always)]
pub fn blend_bytes(a: Vector, b: Vector, m: Vector) -> Vector {
    let a = a.to_le_bytes();
    let b = b.to_le_bytes();
    let m = m.to_le_bytes();

    Vector::from_le_bytes(core::array::from_fn(|i| {
        if m[i] & BYTE_SIGN_MASK != 0 {
            b[i]
        } else {
            a[i]
        }
    }))
}

#[inline(always)]
pub fn byte_sign_mask(a: Vector) -> i32 {
    a.to_le_bytes()
        .into_iter()
        .enumerate()
        .fold(0, |v, (i, x)| v | (((x >> BYTE_SIGN_SHIFT) as i32) << i))
}
