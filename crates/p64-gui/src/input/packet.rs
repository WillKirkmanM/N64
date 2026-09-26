use super::Input;

pub(super) fn press_button(packet: &mut u32, index: usize) {
    *packet |= 1 << index;
}

pub(super) fn encode_stick(x: f64, y: f64) -> u32 {
    let axis = |value: f64, field: Input| (value.round() as i8 as u8 as u32) << field.as_usize();

    axis(x, Input::XAxis) | axis(y, Input::YAxis)
}

#[cfg(test)]
mod tests;
