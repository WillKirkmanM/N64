use super::*;
use crate::mmu::MmuCause;
use crate::mmu::MmuRegister;

#[test]
fn speculative_instruction_translation_never_raises_or_mutates_a_fault() {
    let mut n64 = N64::default();
    n64.cpu.execution.data_address = 0x12345678;
    n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] = 0xABCD;
    n64.cpu.mmu.registers.regs[MmuRegister::BadVAddr as usize] = 0x98765432;
    for address in [
        0xFFFF_FFFF_8000_0001,
        0x8000_0000,
        0x0000_0000_1000_0000,
        0x9000_0001_0000_0000,
    ] {
        assert_eq!(peek_translation(&n64, address), None);
    }

    assert_eq!(
        peek_translation(&n64, 0xFFFF_FFFF_8000_0040),
        Some((64, true))
    );
    assert_eq!(
        peek_translation(&n64, 0xFFFF_FFFF_A000_0040),
        Some((64, false))
    );
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0x10;

    assert_eq!(peek_translation(&n64, 0xFFFF_FFFF_8000_0040), None);
    assert_eq!(n64.cpu.execution.data_address, 0x12345678);
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
        0xABCD
    );
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::BadVAddr as usize],
        0x98765432
    );
    assert!(matches!(
        n64.cpu.branch_state.state,
        crate::cpu::State::Step
    ));
}

#[test]
fn xkphys_decodes_all_coherency_attributes_and_preserves_physical_bits() {
    let mut n64 = N64::default();
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0x80;
    for coherency in 0..8 {
        let address = 0x8000_0000_1234_5678 | (coherency << 59);

        assert_eq!(
            translate_address(&mut n64, address, AccessType::Read),
            (0x1234_5678, coherency != 2, false)
        );
        assert_eq!(n64.cpu.execution.data_address, address);
    }

    assert_eq!(
        translate_address(&mut n64, 0x9000_0000_7FFF_FFF8, AccessType::Read),
        (0x7FFF_FFF8, false, false)
    );
    assert_eq!(read_data(&mut n64, 0x7FFF_FFF8, AccessSize::Word, false), 3);
}

#[test]
fn invalid_extended_and_noncanonical_32_bit_addresses_fault() {
    let mut n64 = N64::default();

    for (status, address) in [
        (0, 0x1234_5678_8000_1000),
        (0x80, 0x9000_0001_0000_0000),
        (0x80, 0x0000_0100_0000_0000),
        (0x80, 0xC000_00FF_8000_0000),
        (0x10, 0xFFFF_FFFF_8000_1000),
    ] {
        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = status;

        assert!(translate_address(&mut n64, address, AccessType::Read).2);
        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
            MmuCause::ExccodeAdEL as u64
        );
        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::BadVAddr as usize],
            address
        );
    }
}

#[test]
fn extended_tlb_lookup_uses_region_asid_and_full_vpn() {
    let mut n64 = N64::default();
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0x80;
    let address = 0x4000_0012_3456_0000;
    n64.cpu.mmu.registers.regs[MmuRegister::EntryHi as usize] = address | 0xAB;
    n64.cpu.mmu.registers.regs[MmuRegister::EntryLo0 as usize] = (0x200 << 6) | (2 << 3) | 6;
    n64.cpu.mmu.registers.regs[MmuRegister::EntryLo1 as usize] = (0x300 << 6) | (3 << 3) | 6;
    tlb::write(&mut n64, 1);

    assert_eq!(
        translate_address(&mut n64, address + 0x234, AccessType::Read),
        (0x20_0234, false, false)
    );
    assert_eq!(
        translate_address(&mut n64, address + 0x1234, AccessType::Write),
        (0x30_0234, true, false)
    );
    n64.cpu.mmu.registers.regs[MmuRegister::EntryHi as usize] = 0xAA;

    assert!(translate_address(&mut n64, address, AccessType::Read).2);
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::BadVAddr as usize],
        address
    );
    assert_eq!(n64.cpu.execution.pc, 0xFFFF_FFFF_8000_0080);
}
