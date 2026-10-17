use super::*;
use crate::mmu::MmuCause;

#[test]
fn undefined_cache_selectors_preserve_cache_lines() {
    let mut n64 = N64::new();
    n64.cpu_mut().set_gpr(1, 0xffff_ffff_a000_0000);
    n64.memory.caches.icache[0].valid = true;
    n64.memory.caches.icache[0].tag = 0x1234;
    n64.memory.caches.dcache[0].valid = true;
    n64.memory.caches.dcache[0].dirty = true;
    n64.memory.caches.dcache[0].store_dirty = true;
    n64.memory.caches.dcache[0].tag = 0x5678;
    n64.set_cp0_register(MmuRegister::TagLo, 0x9abc);

    for selector in 0..32 {
        if CacheSelector::decode(selector).is_some() {
            continue;
        }
        cache(&mut n64, (1 << 21) | ((selector as u32) << 16));
        assert!(n64.memory.caches.icache[0].valid);
        assert_eq!(n64.memory.caches.icache[0].tag, 0x1234);
        let data = &n64.memory.caches.dcache[0];
        assert!(data.valid && data.dirty && data.store_dirty);
        assert_eq!(data.tag, 0x5678);
        assert_eq!(n64.cp0_register(MmuRegister::TagLo), 0x9abc);
    }
}

#[test]
fn undefined_cache_selectors_preserve_address_fault_priority() {
    let mut n64 = N64::new();
    n64.cpu_mut().set_gpr(1, 0xffff_ffff_a000_0001);
    cache(&mut n64, (1 << 21) | (2 << 16));
    assert_eq!(
        n64.cp0_register(MmuRegister::BadVAddr),
        0xffff_ffff_a000_0001
    );
    assert_eq!(
        n64.cp0_register(MmuRegister::Cause),
        MmuCause::ExccodeAdEL as u64
    );

    n64.cpu_mut().set_gpr(1, 0x1000);
    cache(&mut n64, (1 << 21) | (2 << 16));
    assert_eq!(n64.cp0_register(MmuRegister::BadVAddr), 0x1000);
    assert_eq!(
        n64.cp0_register(MmuRegister::Cause) & (31 << 2),
        MmuCause::ExccodeTLBL as u64
    );
}
