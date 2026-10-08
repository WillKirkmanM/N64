use super::joybus::{controller_input_bytes, handle_status};
use super::*;

#[cfg(test)]
mod accuracy_tests {
    use super::*;

    #[test]
    fn controller_wire_bytes_preserve_button_and_signed_stick_positions() {
        let input = (1 << 7) | (1 << 11) | (0x81 << 16) | (0x7f << 24);

        assert_eq!(controller_input_bytes(input), [0x80, 0x08, 0x81, 0x7f]);
        let mut n64 = N64::default();
        n64.pif.channels[0].buffers.rx_buf = Some(4);
        n64.pif.channels[0].pak_handler = None;
        handle_status(&mut n64, 0);

        assert_eq!(&n64.pif.memory.ram[4..7], &[0x05, 0x00, 0x00]);
    }
}

fn no_pi_interrupt(n64: &mut N64) {
    assert_eq!(n64.mips.regs[2] & (1 << 4), 0);
}
