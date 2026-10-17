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
