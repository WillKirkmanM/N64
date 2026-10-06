const REGISTER_BITS: u32 = u128::BITS;

#[inline(always)]
fn lane_shift<const WIDTH: u32>(element: u8) -> u32 {
    let mask = REGISTER_BITS / WIDTH - 1;

    (mask - (u32::from(element) & mask)) * WIDTH
}

#[inline(always)]
pub(super) fn modify_vpr_byte(register: &mut u128, value: u8, element: u8) {
    let shift = lane_shift::<{ u8::BITS }>(element);
    let mask = u128::from(u8::MAX) << shift;
    *register = crate::bits::replace_bits(*register, u128::from(value) << shift, mask);
}

#[inline(always)]
pub(super) fn get_vpr_byte(register: u128, element: u8) -> u8 {
    (register >> lane_shift::<{ u8::BITS }>(element)) as u8
}

#[inline(always)]
pub(super) fn modify_vpr_element(register: &mut u128, value: u16, element: u8) {
    let shift = lane_shift::<{ u16::BITS }>(element);
    let mask = u128::from(u16::MAX) << shift;
    *register = crate::bits::replace_bits(*register, u128::from(value) << shift, mask);
}

#[inline(always)]
pub(super) fn get_vpr_element(register: u128, element: u8) -> u16 {
    (register >> lane_shift::<{ u16::BITS }>(element)) as u16
}

#[cfg(test)]
#[path = "vector_register/tests.rs"]
mod tests;
