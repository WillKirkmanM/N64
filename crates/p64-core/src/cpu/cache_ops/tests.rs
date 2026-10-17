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

#[test]
fn cache_coprocessor_access_precedes_address_and_selector_checks() {
    for mode in [1_u64, 2] {
        for address in [0x1000_u64, 0x1001, 0xffff_ffff_a000_0000] {
            for selector in [0_u32, 2] {
                let mut n64 = N64::new();
                crate::cpu::start_cpu(&mut n64);
                n64.set_cp0_register(MmuRegister::Status, mode << 3);
                n64.set_cp0_register(MmuRegister::BadVAddr, 0x5678);
                n64.cpu.execution.pc = 0x2000;
                n64.cpu_mut().set_gpr(1, address);
                n64.memory.caches.icache[0].valid = true;
                let opcode = (0x2f << 26) | (1 << 21) | (selector << 16);
                crate::cpu::decode_opcode(&n64, opcode)(&mut n64, opcode);
                assert_eq!(n64.cp0_register(MmuRegister::Cause) & 0x3000_007c, 11 << 2);
                assert_eq!(n64.cp0_register(MmuRegister::BadVAddr), 0x5678);
                assert_eq!(n64.cp0_register(MmuRegister::EPC), 0x2000);
                assert!(n64.memory.caches.icache[0].valid);
            }
        }
    }
}

#[test]
fn cache_kernel_overrides_do_not_require_cu0() {
    for status in [0_u64, (2 << 3) | (1 << 1), (2 << 3) | (1 << 2)] {
        let mut n64 = N64::new();
        n64.set_cp0_register(MmuRegister::Status, status);
        n64.cpu_mut().set_gpr(1, 0xffff_ffff_a000_0000);
        n64.memory.caches.icache[0].valid = true;
        cache(&mut n64, 1 << 21);
        assert!(!n64.memory.caches.icache[0].valid);
        assert_eq!(n64.cp0_register(MmuRegister::Cause), 0);
    }
}

#[test]
fn cache_enabled_in_user_and_supervisor_modes_keeps_data_address_faults() {
    for mode in [1_u64, 2] {
        for (address, cause) in [(0x1000_u64, 2_u64 << 2), (0x1001, 4 << 2)] {
            let mut n64 = N64::new();
            n64.set_cp0_register(MmuRegister::Status, (mode << 3) | (1 << 28));
            n64.cpu_mut().set_gpr(1, address);
            cache(&mut n64, (1 << 21) | (2 << 16));
            assert_eq!(n64.cp0_register(MmuRegister::Cause) & 0x3000_007c, cause);
            assert_eq!(n64.cp0_register(MmuRegister::BadVAddr), address);
        }
    }
}
