mod entry;
mod entry_hi;
mod entry_lo;
mod page;
mod registers;

pub use entry::TlbEntry;
pub use entry_hi::EntryHi;
pub use page::TlbPage;

use crate::{memory::AccessType, mmu::MmuRegister, N64};
use registers::RegisterSnapshot;

pub const ENTRY_COUNT: usize = 32;
const PROBE_MISS: u64 = 1 << 31;

fn entry_index(index: u64) -> Option<usize> {
    (index < ENTRY_COUNT as u64).then_some(index as usize)
}

pub fn read(n64: &mut N64, index: u64) {
    let Some(index) = entry_index(index) else {
        return;
    };

    let entry = n64.cp0().translations.tlb_entries[index];
    entry.registers().write(n64.cp0_mut());
}

pub fn write(n64: &mut N64, index: u64) {
    let Some(index) = entry_index(index) else {
        return;
    };

    let cp0 = n64.cp0_mut();
    cp0.translations.tlb_entries[index] = TlbEntry::from_registers(RegisterSnapshot::read(cp0));
}

pub fn probe(n64: &mut N64) {
    let cp0 = n64.cp0_mut();
    let query = EntryHi::decode(cp0.register(MmuRegister::EntryHi));
    let index = cp0
        .translations
        .tlb_entries
        .iter()
        .position(|entry| entry.matches(query, true))
        .map_or(PROBE_MISS, |index| index as u64);
    cp0.set_register(MmuRegister::Index, index);
}

pub fn lookup_entry(n64: &N64, address: u64) -> Option<(TlbEntry, bool)> {
    let cp0 = n64.cp0();
    let extended = cp0.status().extended_addressing();
    let query = EntryHi::for_address(address, cp0.register(MmuRegister::EntryHi) as u8);

    cp0.translations.tlb_entries.iter().find_map(|entry| {
        entry
            .matches(query, extended)
            .then_some((*entry, address & entry.page_size() != 0))
    })
}

pub fn get_physical_address(
    n64: &mut N64,
    address: u64,
    access_type: AccessType,
) -> (u64, bool, bool) {
    if let Some((entry, odd)) = lookup_entry(n64, address) {
        let page = entry.page(odd);

        if page.permits(access_type) {
            return (
                page.translate(address, entry.page_size()),
                page.cacheable(),
                false,
            );
        }
    }

    crate::exceptions::tlb_miss_exception(n64, address, access_type);

    (0, false, true)
}

#[cfg(test)]
mod tests;
