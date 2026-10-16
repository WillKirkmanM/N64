use super::*;
use crate::builder::memory::Memory;

#[test]
fn mi_repeat_respects_bus_lane_start_and_programmed_length() {
    let mut n64 = N64::new();
    let value = 0x9abcdef1u32;

    for width in [1, 2, 4] {
        for start in (0..=16usize).step_by(width) {
            for length in 1..=128usize {
                n64.rdram.mem.data[0..256].fill(0xff);
                crate::mips::write_regs(&mut n64, 0, 0x100 | (length - 1) as u32, u32::MAX);
                let shift = ((4 - width - (start & 3)) * 8) as u32;
                let mask = match width {
                    1 => 0xff << shift,
                    2 => 0xffff << shift,
                    _ => u32::MAX,
                };

                let bus_value = value.wrapping_shl(shift);
                write_mem(&mut n64, (start & !3) as u64, bus_value, mask);
                let end = start + length.saturating_sub(start & 7);
                let pattern = bus_value.to_be_bytes();

                for index in 0..256 {
                    let expected = if index >= start && index < end {
                        pattern[index & 3]
                    } else {
                        0xff
                    };

                    assert_eq!(
                        n64.rdram.mem[index ^ n64.byte_swap],
                        expected,
                        "width={width} start={start} length={length} index={index}"
                    );
                }

                assert!(!crate::mips::repeat_enabled(&n64));
            }
        }
    }
}

#[test]
fn mi_repeat_doubleword_wraps_inside_two_kibibytes() {
    let mut n64 = N64::new();
    n64.rdram.mem.data[0..4096].fill(0xff);
    crate::mips::write_regs(&mut n64, 0, 0x100 | 127, u32::MAX);
    write_mem_double_repeat(&mut n64, 2040, 0);

    for index in 0..4096 {
        let expected = if index < 120 || (2040..2048).contains(&index) {
            0
        } else {
            0xff
        };

        assert_eq!(n64.rdram.mem[index ^ n64.byte_swap], expected);
    }
}

#[test]
fn partial_rdram_stores_preserve_other_bytes() {
    let mut n64 = N64::new();
    write_mem(&mut n64, 4, 0x01234567, u32::MAX);
    write_mem(&mut n64, 4, 0x89abcdef, 0xff000000);

    assert_eq!(read_mem_fast(&mut n64, 4, AccessSize::Word), 0x89234567);
    write_mem(&mut n64, 4, 0x12345678, 0x0000ffff);

    assert_eq!(read_mem_fast(&mut n64, 4, AccessSize::Word), 0x89235678);
}

#[test]
fn unpopulated_rdram_reads_zero_without_mirroring() {
    let mut n64 = N64::new();
    write_mem(&mut n64, 0, 0x12345678, u32::MAX);
    for address in [0x800000, 0x1000000, 0x2000000, 0x3effffc] {
        assert_eq!(read_mem_fast(&mut n64, address, AccessSize::Word), 0);
        write_mem(&mut n64, address, u32::MAX, u32::MAX);
    }

    assert_eq!(read_mem_fast(&mut n64, 0, AccessSize::Word), 0x12345678);
    write_mem(&mut n64, (RDRAM_SIZE - 4) as u64, 0x89abcdef, u32::MAX);

    assert_eq!(
        read_mem_fast(&mut n64, (RDRAM_SIZE - 4) as u64, AccessSize::Word),
        0x89abcdef
    );
}

#[test]
fn rdram_map_charges_cycles_only_for_timed_reads() {
    let mut n64 = N64::new();
    n64.cartridge.rom = Memory::new().with_size(4).build();
    crate::memory::initialise_memory_map(&mut n64);
    n64.rdram_interface.regs[RiReg::Refresh as usize] = 0;
    write_mem(&mut n64, 0, 0, u32::MAX);
    let before = n64.cpu.mmu.registers.regs[crate::mmu::MmuRegister::Count as usize];
    (n64.memory.bus.fast_read[0])(&mut n64, 0, AccessSize::Word);

    assert_eq!(
        n64.cpu.mmu.registers.regs[crate::mmu::MmuRegister::Count as usize],
        before
    );
    (n64.memory.bus.memory_map_read[0])(&mut n64, 0, AccessSize::Word);

    assert_eq!(
        n64.cpu.mmu.registers.regs[crate::mmu::MmuRegister::Count as usize],
        before + 31
    );
}

#[test]
fn ri_registers_are_independent_words_with_masked_writes() {
    let mut ri = RdramInterface::default();

    assert_eq!(ri.read(12, AccessSize::Word), 0x14);
    for index in 0..8 {
        ri.write(index * 4, 0x12340000 | index as u32, u32::MAX);
    }

    for index in 0..8 {
        assert_eq!(
            ri.read(index * 4, AccessSize::Word),
            0x12340000 | index as u32
        );
    }

    ri.write(28, 0xabcdef01, 0x0000ffff);

    assert_eq!(ri.read(28, AccessSize::Word), 0x1234ef01);
    ri.write(32, u32::MAX, u32::MAX);

    assert_eq!(ri.read(32, AccessSize::Word), 0);
}
