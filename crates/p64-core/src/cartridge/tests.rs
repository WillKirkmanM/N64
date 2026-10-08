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

    #[test]
    fn pi_dma_matches_nemu_small_misaligned_and_page_end_vectors() {
        let mut n64 = N64::new();
        n64.cartridge.rom.data = (0..512).map(|byte| byte as u8).collect::<Vec<_>>().into();
        // Expected vectors: nemu64-test PIDMASmallSize/PIDMAMisaligned/Page.
        for misalignment in [0u32, 2, 4, 6] {
            for page_offset in [16u32, 1984, 2040] {
                for requested in 1u32..=300 {
                    let mut expected = vec![0xaa; 4096];
                    let start = page_offset + misalignment;
                    let boundary = if page_offset == 2040 {
                        8 - misalignment
                    } else if page_offset == 1984 {
                        64 - misalignment
                    } else {
                        128 - misalignment
                    };
                    let threshold = if page_offset == 2040 && misalignment != 0 {
                        boundary
                    } else {
                        boundary - 2
                    };
                    let rom_length = if requested >= threshold {
                        (requested + 1) & !1
                    } else {
                        requested
                    };
                    let mut written_extent = if rom_length < boundary {
                        rom_length.saturating_sub(misalignment)
                    } else {
                        rom_length
                    };

                    let second_hole = boundary + 128 - misalignment;

                    if page_offset == 2040 && written_extent > second_hole {
                        written_extent += misalignment;
                    }

                    for offset in 0..written_extent {
                        let first_hole = boundary.saturating_sub(misalignment);

                        if offset >= first_hole && offset < boundary {
                            continue;
                        }

                        let source = if page_offset == 2040 && offset >= second_hole {
                            if offset < second_hole + misalignment {
                                continue;
                            }

                            offset - misalignment
                        } else {
                            offset
                        };

                        expected[(start + offset) as usize] = source as u8;
                    }

                    n64.rdram.mem.data[..4096].fill(0xaa);
                    n64.peripheral.regs[PiRegister::CartAddr as usize] = 0x1000_0000;
                    n64.peripheral.regs[PiRegister::DramAddr as usize] = start;
                    n64.peripheral.regs[PiRegister::WrLen as usize] = requested - 1;
                    peripheral::dma_write(&mut n64);
                    for (address, expected_byte) in expected.iter().enumerate() {
                        assert_eq!(
                            n64.rdram.mem.data[address ^ n64.byte_swap],
                            *expected_byte,
                            "PI DMA misalignment={misalignment} page_offset={page_offset} size={requested} byte={address}"
                        );
                    }

                    assert_eq!(
                        n64.peripheral.regs[PiRegister::CartAddr as usize],
                        0x1000_0000 + ((rom_length + 1) & !1)
                    );
                    assert_eq!(
                        n64.peripheral.regs[PiRegister::DramAddr as usize],
                        (start + written_extent + 7) & !7
                    );
                }
            }
        }
    }
}

#[test]
fn cart_dma_reads_zero_beyond_installed_rdram_without_overwriting_ram_start() {
    let mut n64 = N64::default();
    n64.cartridge.rom = Memory::from(vec![0xff; 8]);
    memory::write_rdram_byte(&mut n64, 0x7ffffe, 0x12);
    memory::write_rdram_byte(&mut n64, 0x7fffff, 0x34);
    dma_read(&mut n64, 0, 0x7ffffe, 4);
    assert_eq!(&n64.cartridge.rom.data[..4], &[0x12, 0x34, 0, 0]);
    assert_eq!(n64.rdram.mem.read_u32(0), 0);
}
