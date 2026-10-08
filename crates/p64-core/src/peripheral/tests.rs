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

    #[test]
    fn pi_timing_registers_ignore_unimplemented_bits() {
        let mut n64 = N64::new();

        for base in [
            PiRegister::BsdDom1Lat as usize,
            PiRegister::BsdDom2Lat as usize,
        ] {
            for (offset, expected) in [(0, 0xff), (1, 0xff), (2, 0xf), (3, 3)] {
                let address = ((base + offset) * 4) as u64;
                write_regs(&mut n64, address, u32::MAX, u32::MAX);

                assert_eq!(
                    read_regs(&mut n64, address, memory::AccessSize::Word),
                    expected
                );
                write_regs(&mut n64, address, 0, 1);

                assert_eq!(
                    read_regs(&mut n64, address, memory::AccessSize::Word),
                    expected & !1
                );
            }
        }
    }
}

#[cfg(test)]
mod command_tests {
    use super::*;

    #[test]
    fn unsupported_controller_command_does_not_modify_pif_response() {
        let mut n64 = N64::new();
        n64.pif.channels[0].buffers.tx_buf = Some(0);
        n64.pif.channels[0].buffers.rx_buf = Some(4);

        for command in 4..255 {
            n64.pif.memory.ram.fill(0x5a);
            n64.pif.memory.ram[0] = command;
            let before = n64.pif.memory.ram;
            process(&mut n64, 0);

            assert_eq!(n64.pif.memory.ram, before, "command={command}");
        }
    }
}

#[test]
fn pak_write_uses_transmit_payload_and_returns_augmented_crc() {
    let mut n64 = N64::default();
    n64.pif.memory.ram[..5].copy_from_slice(&[35, 1, 3, 6, 0]);
    n64.pif.memory.ram[5..37].fill(0x5a);
    n64.pif.memory.ram[37] = 0xff;
    n64.pif.channels[0].buffers.tx = Some(0);
    n64.pif.channels[0].buffers.rx = Some(1);
    n64.pif.channels[0].buffers.tx_buf = Some(2);
    n64.pif.channels[0].buffers.rx_buf = Some(37);
    process(&mut n64, 0);
    assert_eq!(&n64.save_data().mempak.data[0x600..0x620], &[0x5a; 32]);
    assert_eq!(n64.pif.memory.ram[37], 0xd1);
    n64.pif.memory.ram[..5].copy_from_slice(&[3, 33, 2, 6, 0]);
    n64.pif.memory.ram[5..38].fill(0);
    n64.pif.channels[0].buffers.rx_buf = Some(5);
    process(&mut n64, 0);
    assert_eq!(&n64.pif.memory.ram[5..37], &[0x5a; 32]);
    assert_eq!(n64.pif.memory.ram[37], 0xd1);
}

fn no_pi_interrupt(n64: &mut N64) {
    assert_eq!(n64.mips.regs[2] & (1 << 4), 0);
}
