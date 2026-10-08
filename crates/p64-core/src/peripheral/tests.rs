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

    #[test]
    fn pi_timing_uses_page_size_and_release_registers_in_hardware_order() {
        let mut n64 = N64::new();

        for (domain, base) in [
            (1, PiRegister::BsdDom1Lat as usize),
            (2, PiRegister::BsdDom2Lat as usize),
        ] {
            // SDK rcp.h places PGS at LAT+8 and RLS at LAT+12.
            write_regs(&mut n64, ((base + 2) * 4) as u64, 2, u32::MAX);
            write_regs(&mut n64, ((base + 3) * 4) as u64, 3, u32::MAX);
            // Two 16-byte pages: ((15+5)*2 + (1+4)*16) * 1.5.
            assert_eq!(calculate_cycles(&n64, domain, 32), 180);
        }
    }
}

fn no_pi_interrupt(n64: &mut N64) {
    assert_eq!(n64.mips.regs[2] & (1 << 4), 0);
}
