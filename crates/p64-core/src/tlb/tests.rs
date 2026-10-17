use super::*;
use crate::mmu::Mmu;

#[test]
fn paired_register_decode_matches_raw_hardware_fields() {
    let mut cp0 = Mmu::default();
    let mut seed = 0xCAD0_1EAF_4300_0064u64;

    for raw_mask in 0..4096u64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let hi = seed;
        let lo = [seed.rotate_left(19), seed.rotate_right(11)];
        cp0.set_register(MmuRegister::PageMask, raw_mask << 13);
        cp0.set_register(MmuRegister::EntryHi, hi);
        cp0.set_register(MmuRegister::EntryLo0, lo[0]);
        cp0.set_register(MmuRegister::EntryLo1, lo[1]);
        let entry = TlbEntry::from_registers(RegisterSnapshot::read(&cp0));
        let mask = (raw_mask & 0xAAA) | ((raw_mask & 0xAAA) >> 1);

        assert_eq!(entry.mask, mask);
        assert_eq!(entry.hi.vpn2, ((hi >> 13) & 0x7FF_FFFF) & !mask);
        assert_eq!(entry.hi.region, (hi >> 62) as u8);
        assert_eq!(entry.hi.asid, hi as u8);
        assert_eq!(entry.global, lo[0] & lo[1] & 1 != 0);
        let start = (hi & 0xC000_0000_0000_0000) | (entry.hi.vpn2 << 13);
        let size = (mask + 1) << 12;

        for (index, raw) in lo.into_iter().enumerate() {
            let page = entry.pages[index];

            assert_eq!(page.pfn, (raw >> 6) & 0xF_FFFF);
            assert_eq!(page.coherency, ((raw >> 3) & 7) as u8);
            assert_eq!(page.dirty, raw & 4 != 0);
            assert_eq!(page.valid, raw & 2 != 0);
            let offset = seed & (size - 1);
            let address = start + index as u64 * size + offset;

            assert_eq!(
                page.translate(address, size),
                ((((raw >> 6) & 0xF_FFFF) << 12) & !(size - 1)) | (address & (size - 1))
            );
        }

        entry.registers().write(&mut cp0);

        assert_eq!(cp0.register(MmuRegister::PageMask), mask << 13);
        assert_eq!(
            cp0.register(MmuRegister::EntryHi),
            (hi & 0xC000_0000_0000_00FF) | (entry.hi.vpn2 << 13)
        );

        for (register, raw) in [MmuRegister::EntryLo0, MmuRegister::EntryLo1]
            .into_iter()
            .zip(lo)
        {
            assert_eq!(
                cp0.register(register),
                (raw & 0x3FF_FFFE) | u64::from(entry.global)
            );
        }
    }
}

#[test]
fn matching_distinguishes_probe_regions_from_compatible_addresses() {
    let entry = TlbEntry {
        mask: 3,
        hi: EntryHi {
            vpn2: 0x100123,
            region: 2,
            asid: 7,
        },
        ..TlbEntry::default()
    };
    let query = EntryHi {
        vpn2: 0x121,
        region: 0,
        asid: 7,
    };

    assert!(entry.matches(query, false));
    assert!(!entry.matches(query, true));
    assert!(!entry.matches(EntryHi { asid: 8, ..query }, false));
    assert!(TlbEntry {
        global: true,
        ..entry
    }
    .matches(EntryHi { asid: 8, ..query }, false));
}

#[test]
fn replacing_entries_preserves_asid_permissions_and_physical_translation() {
    let mut n64 = N64::new();
    n64.set_cp0_register(MmuRegister::PageMask, 0x6000);
    n64.set_cp0_register(MmuRegister::EntryHi, 0x0040_0007);
    n64.set_cp0_register(MmuRegister::EntryLo0, (0x123 << 6) | (3 << 3) | 6);
    n64.set_cp0_register(MmuRegister::EntryLo1, (0x456 << 6) | (2 << 3) | 2);
    write(&mut n64, 5);
    let even = get_physical_address(&mut n64, 0x0040_0123, AccessType::Write);
    assert_eq!(even, (0x0012_0123, true, false));
    let odd = get_physical_address(&mut n64, 0x0040_4123, AccessType::Read);
    assert_eq!(odd, (0x0045_4123, false, false));
    let page = lookup_entry(&n64, 0x0040_4123).unwrap().0.page(true);
    assert!(!page.permits(AccessType::Write));
    n64.set_cp0_register(MmuRegister::EntryHi, 0x0040_0008);
    assert!(lookup_entry(&n64, 0x0040_0123).is_none());
    n64.set_cp0_register(MmuRegister::EntryLo0, (0x789 << 6) | (3 << 3) | 7);
    n64.set_cp0_register(MmuRegister::EntryLo1, (0xABC << 6) | (3 << 3) | 7);
    write(&mut n64, 5);
    n64.set_cp0_register(MmuRegister::EntryHi, 9);
    assert_eq!(
        get_physical_address(&mut n64, 0x0040_0123, AccessType::Write),
        (0x0078_8123, true, false)
    );
    read(&mut n64, 5);
    assert_eq!(n64.cp0_register(MmuRegister::EntryHi), 0x0040_0008);
}
