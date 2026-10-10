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
