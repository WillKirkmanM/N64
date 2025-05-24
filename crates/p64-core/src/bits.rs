//! Alignment helpers require a nonzero power-of-two alignment.
use std::{
    num::Wrapping,
    ops::{Add, BitAnd, BitOr, Not, Sub},
};

#[inline]
pub(crate) fn align_down<T>(value: T, alignment: T) -> T
where
    T: Copy + From<u8> + Sub<Output = T> + BitAnd<Output = T> + Not<Output = T>,
{
    value & !(alignment - T::from(1))
}

#[inline]
pub(crate) fn alignment_offset<T>(value: T, alignment: T) -> T
where
    T: Copy + From<u8> + Sub<Output = T> + BitAnd<Output = T>,
{
    value & (alignment - T::from(1))
}

#[inline]
pub(crate) fn is_aligned<T>(value: T, alignment: T) -> bool
where
    T: Copy + From<u8> + Sub<Output = T> + BitAnd<Output = T> + PartialEq,
{
    alignment_offset(value, alignment) == T::from(0)
}

#[inline]
pub(crate) fn align_up<T>(value: T, alignment: T) -> T
where
    T: Copy + From<u8> + Add<Output = T> + Sub<Output = T> + BitAnd<Output = T> + Not<Output = T>,
{
    align_down(value + (alignment - T::from(1)), alignment)
}

#[inline]
pub(crate) fn align_up_wrapping<T>(value: T, alignment: T) -> T
where
    T: Copy + From<u8> + Sub<Output = T> + BitAnd<Output = T> + Not<Output = T>,
    Wrapping<T>: Add<Output = Wrapping<T>>,
{
    align_down(
        (Wrapping(value) + Wrapping(alignment - T::from(1))).0,
        alignment,
    )
}

#[inline]
pub(crate) fn replace_bits<T>(value: T, replacement: T, mask: T) -> T
where
    T: Copy + BitAnd<Output = T> + BitOr<Output = T> + Not<Output = T>,
{
    (value & !mask) | (replacement & mask)
}

#[cfg(test)]
mod tests {
    use super::{align_down, align_up, align_up_wrapping, alignment_offset, is_aligned, replace_bits};

    #[test]
    fn power_of_two_alignment_matches_arithmetic_rounding() {
        for alignment in [1_u64, 2, 4, 8, 16, 64, 4096] {
            for value in 0..8193 {
                assert_eq!(align_down(value, alignment), value / alignment * alignment);
                assert_eq!(
                    align_up(value, alignment),
                    value.div_ceil(alignment) * alignment
                );
                assert_eq!(
                    align_up_wrapping(value, alignment),
                    align_up(value, alignment)
                );
            }
        }
    }

    #[test]
    fn signed_quantization_rounds_toward_the_alignment_boundary() {
        for alignment in [1_i32, 2, 4, 16, 256] {
            for value in -1025_i32..=1025 {
                let expected_down = value.div_euclid(alignment) * alignment;
                let expected_up = if value.rem_euclid(alignment) == 0 {
                    expected_down
                } else {
                    expected_down + alignment
                };

                assert_eq!(align_down(value, alignment), expected_down);
                assert_eq!(align_up(value, alignment), expected_up);
            }
        }
    }

    #[test]
    fn wrapping_alignment_crosses_integer_width_boundaries() {
        assert_eq!(align_up_wrapping(u32::MAX, 8), 0);
        assert_eq!(align_up_wrapping(u64::MAX, 4096), 0);
        assert_eq!(align_up_wrapping(u128::MAX, 16), 0);
        assert_eq!(align_up_wrapping(usize::MAX, 8), 0);
        assert_eq!(align_up_wrapping(u8::MAX, 16), 0);
        assert_eq!(align_up_wrapping(u16::MAX, 256), 0);
        assert_eq!(align_up_wrapping(u64::MAX - 7, 8), u64::MAX - 7);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "attempt to add with overflow")]
    fn ordinary_alignment_preserves_checked_overflow() {
        let _ = align_up(u32::MAX, 8);
    }

    #[test]
    fn field_replacement_matches_independent_byte_operations() {
        let value = 0xfedc_ba98_7654_3210_u64;
        let replacement = 0x0123_4567_89ab_cdef_u64;

        for bytes in 0..=8 {
            let mask = if bytes == 8 {
                u64::MAX
            } else {
                (1_u64 << (bytes * 8)) - 1
            };

            let mut expected = value.to_le_bytes();
            expected[..bytes].copy_from_slice(&replacement.to_le_bytes()[..bytes]);

            assert_eq!(
                replace_bits(value, replacement, mask),
                u64::from_le_bytes(expected)
            );
        }
    }

    #[test]
    fn alignment_predicates_and_offsets_match_division_remainders() {
        for alignment in [1_u64, 2, 4, 8, 16, 4096] {
            for value in 0..8193 {
                assert_eq!(alignment_offset(value, alignment), value % alignment);
                assert_eq!(is_aligned(value, alignment), value % alignment == 0);
            }
        }

        for value in -65_i32..=65 {
            assert_eq!(alignment_offset(value, 16), value.rem_euclid(16));
            assert_eq!(is_aligned(value, 16), value.rem_euclid(16) == 0);
        }
    }
}
