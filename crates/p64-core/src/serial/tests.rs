use super::*;

#[cfg(test)]
mod accuracy_tests {
    use super::*;

    #[test]
    fn si_dma_converts_canonical_rdram_words_to_pif_wire_bytes_both_directions() {
        let mut n64 = N64::default();
        n64.serial.regs[SerialRegister::DramAddr as usize] = 0x80;
        for offset in (0..pif::PIF_RAM_SIZE).step_by(4) {
            n64.rdram
                .mem
                .write_u32((0x80 + offset) as u64, 0x11223344 ^ offset as u32, u32::MAX);
        }
        n64.serial.dma_dir = DmaDir::Write;
        copy_pif_rdram(&mut n64);
        for offset in (0..pif::PIF_RAM_SIZE).step_by(4) {
            assert_eq!(
                &n64.pif.memory.ram[offset..offset + 4],
                &(0x11223344 ^ offset as u32).to_be_bytes()
            );
            n64.pif.memory.ram[offset..offset + 4]
                .copy_from_slice(&(0xa1b2c3d4 ^ offset as u32).to_be_bytes());
        }
        n64.serial.dma_dir = DmaDir::Read;
        copy_pif_rdram(&mut n64);
        for offset in (0..pif::PIF_RAM_SIZE).step_by(4) {
            let value = 0xa1b2c3d4 ^ offset as u32;

            assert_eq!(
                &n64.rdram.mem[0x80 + offset..0x84 + offset],
                &value.to_le_bytes()
            );
            assert_eq!(n64.rdram.mem.read_u32((0x80 + offset) as u64), value);
        }

        assert_eq!(n64.byte_swap, 3);
    }
}

#[test]
fn si_dma_crossing_installed_ram_reads_zero_and_discards_writes() {
    let mut n64 = N64::default();
    n64.serial.regs[SerialRegister::DramAddr as usize] = 0x7ffff8;
    n64.rdram.mem.write_u32(0x7ffff8, 0x11223344, u32::MAX);
    n64.rdram.mem.write_u32(0x7ffffc, 0x55667788, u32::MAX);
    n64.serial.dma_dir = DmaDir::Write;
    copy_pif_rdram(&mut n64);
    assert_eq!(
        &n64.pif.memory.ram[..8],
        &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]
    );
    assert!(n64.pif.memory.ram[8..].iter().all(|byte| *byte == 0));
    n64.pif.memory.ram.fill(0xaa);
    n64.serial.dma_dir = DmaDir::Read;
    copy_pif_rdram(&mut n64);
    assert_eq!(n64.rdram.mem.read_u32(0x7ffffc), 0xaaaaaaaa);
    assert_eq!(n64.rdram.mem.read_u32(0), 0);
}

#[test]
fn reserved_si_registers_read_zero_and_ignore_writes_through_guest_bus() {
    let mut n64 = N64::default();
    memory::initialise_memory_map(&mut n64);
    for address in [0x0480001c, 0x0480fffc] {
        memory::write_data(&mut n64, address, u32::MAX, u32::MAX, false);
        assert_eq!(
            memory::read_data(&mut n64, address, memory::AccessSize::Word, false),
            0
        );
    }
    assert!(n64
        .serial
        .regs
        .as_ref()
        .iter()
        .all(|register| *register == 0));
    assert!(n64.serial.dma_dir == DmaDir::None);
    assert!(crate::events::get_event(&mut n64, EventType::SI).is_none());
}
