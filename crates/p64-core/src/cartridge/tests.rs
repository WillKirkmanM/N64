use super::*;

#[test]
fn persisted_rom_patches_keep_valid_bytes_and_ignore_malformed_entries() {
    let mut cartridge = Cartridge {
        rom: Memory::from(vec![0x11; 4]),
        ..Cartridge::default()
    };
    let patches = serde_json::json!({
        "0": 0,
        "1": 255,
        "2": 256,
        "3": -1,
        "4": 42,
        "invalid": 77,
        "999999999999999999999999": 88
    });
    cartridge.restore_rom(patches.as_object().unwrap());

    assert_eq!(&*cartridge.rom.data, &[0, 255, 0x11, 0x11]);
}

#[cfg(test)]
mod accuracy_tests {
    use super::*;

    #[test]
    fn cart_subword_bus_skips_the_low_halfword() {
        let mut n64 = N64::new();
        n64.cartridge.rom.data = vec![
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x21, 0x43, 0x65, 0x87, 0x99, 0xba,
            0xdc, 0xfe, 0xa9, 0x88, 0x77, 0x66,
        ]
        .into();
        // Hardware vectors from nemu64-test cart:Read8 and cart:Read16.
        let expected = [
            0x01, 0x23, 0x89, 0xab, 0x89, 0xab, 0x21, 0x43, 0x21, 0x43, 0x99, 0xba, 0x99, 0xba,
            0xa9, 0x88,
        ];

        for address in 0..16 {
            let word = read_subword(&mut n64, 0x1000_0000 + (address & !3));

            assert_eq!(
                (word >> ((3 - (address & 3)) * 8)) as u8,
                expected[address as usize]
            );
        }

        assert_eq!(
            read_mem(&mut n64, 0x1000_0000, memory::AccessSize::Word),
            0x0123_4567
        );
    }

    #[test]
    fn cart_write_latch_keeps_first_write_and_all_driven_bits() {
        let mut n64 = N64::new();
        n64.cartridge.rom.data = vec![0x01, 0x23, 0x45, 0x67].into();
        write_mem(&mut n64, 0x1000_0000, 0xbadc_0ffe, u32::MAX);
        write_mem(&mut n64, 0x1000_1000, 0xdecaf, u32::MAX);

        assert_eq!(
            read_mem(&mut n64, 0x1000_0000, memory::AccessSize::Word),
            0xbadc_0ffe
        );
        assert_eq!(
            read_mem(&mut n64, 0x1000_0000, memory::AccessSize::Word),
            0x0123_4567
        );
        // SB of 0x123456BA at byte offset 1 drives 0x56BA0000 onto the bus.
        write_mem(&mut n64, 0x1000_0000, 0x56ba_0000, 0x00ff_0000);

        assert_eq!(
            read_mem(&mut n64, 0x1000_0000, memory::AccessSize::Word),
            0x56ba_0000
        );
    }
}
