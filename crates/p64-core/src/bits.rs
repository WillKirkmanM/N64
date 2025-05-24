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
