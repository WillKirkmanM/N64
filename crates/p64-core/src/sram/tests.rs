use super::*;
use crate::{memory::AccessSize, rdram::RDRAM_SIZE};

#[test]
fn flashram_program_erase_and_silicon_id_are_reachable_through_guest_bus() {
    let mut n64 = N64::default();
    memory::initialise_memory_map(&mut n64);
    let base = memory::MemoryMap::Dom2Addr2 as u64;
    memory::write_data(&mut n64, base + 0x10000, 0xb4000000, u32::MAX, false);
    for offset in 0..128 {
        memory::write_rdram_byte(&mut n64, offset, offset as u8);
    }
    dma_read(&mut n64, base as u32, 0, 128);
    memory::write_data(&mut n64, base + 0x10000, 0xa5000081, u32::MAX, false);
    assert_eq!(
        &n64.save_data().flash.data[0x4080..0x4100],
        &(0u8..128).collect::<Vec<_>>()
    );
    assert!(n64.save_data().flash.dirty);
    assert_eq!(
        memory::read_data(&mut n64, base, AccessSize::Word, false),
        STATUS_PROGRAM_COMPLETE
    );
    memory::write_data(&mut n64, base + 0x10000, 0xf0000000, u32::MAX, false);
    dma_write(&mut n64, base as u32 + 0x2040, 0x100, 128);
    for offset in 0..128 {
        assert_eq!(memory::read_rdram_byte(&n64, 0x100 + offset), offset as u8);
    }
    memory::write_data(&mut n64, base + 0x10000, 0x4b000081, u32::MAX, false);
    memory::write_data(&mut n64, base + 0x10000, 0x78000000, u32::MAX, false);
    assert!(n64.save_data().flash.data[0x4000..0x8000]
        .iter()
        .all(|byte| *byte == 0xff));
    memory::write_data(&mut n64, base + 0x10000, 0xe1000000, u32::MAX, false);
    dma_write(&mut n64, base as u32, 0x200, 8);
    assert_eq!(n64.rdram.mem.read_u32(0x200), FLASHRAM_TYPE_ID);
    assert_eq!(n64.rdram.mem.read_u32(0x204), MX29L1100_ID);
    n64.save_data_mut().flash.data.fill(0);
    memory::write_data(&mut n64, base + 0x10000, 0x3c000000, u32::MAX, false);
    memory::write_data(&mut n64, base + 0x10000, 0x78000000, u32::MAX, false);
    assert!(n64.save_data().flash.data.iter().all(|byte| *byte == 0xff));
}

#[test]
fn dma_crossing_installed_rdram_and_sram_boundaries_does_not_mirror_ram() {
    let mut n64 = N64::default();
    n64.ui.save_data.save_type.push(SaveTypes::Sram);
    memory::write_rdram_byte(&mut n64, RDRAM_SIZE as u32 - 2, 0x12);
    memory::write_rdram_byte(&mut n64, RDRAM_SIZE as u32 - 1, 0x34);
    dma_read(&mut n64, 0, RDRAM_SIZE as u32 - 2, 4);
    assert_eq!(&n64.save_data().sram.data[..4], &[0x12, 0x34, 0, 0]);
    dma_write(&mut n64, SRAM_SIZE as u32, 0, 4);
    assert_eq!(n64.rdram.mem.read_u32(0), 0);
    dma_read(&mut n64, SRAM_SIZE as u32, 0, 4);
    assert_eq!(n64.save_data().sram.data.len(), SRAM_SIZE);
}
