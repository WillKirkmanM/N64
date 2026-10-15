use super::*;

#[test]
fn undocumented_control_register_transfers_preserve_defined_registers() {
    let mut n64 = N64::new();
    n64.set_cp0_register(MmuRegister::Status, MmuStatus::CU1 as u64);
    n64.fpu_mut().control.fcr31 = FCR31_CMP_BIT | 3;
    let identification = n64.fpu().control.fcr0;
    let control = n64.fpu().control.fcr31;

    for index in 1..31 {
        n64.cpu_mut().set_gpr(2, u64::MAX);
        ctc1(&mut n64, (2 << 16) | (index << 11));
        cfc1(&mut n64, (2 << 16) | (index << 11));
        assert_eq!(n64.cpu().gpr(2), 0);
        assert_eq!(n64.fpu().control.fcr0, identification);
        assert_eq!(n64.fpu().control.fcr31, control);
        assert!(!matches!(
            n64.cpu().branch_state.state,
            cpu::State::Exception
        ));
    }

    n64.cpu_mut().set_gpr(2, u64::MAX);
    ctc1(&mut n64, 2 << 16);
    cfc1(&mut n64, 2 << 16);
    assert_eq!(n64.cpu().gpr(2), u64::from(identification));
    n64.cpu_mut().set_gpr(2, u64::from(FCR31_CMP_BIT));
    ctc1(&mut n64, (2 << 16) | (31 << 11));
    cfc1(&mut n64, (2 << 16) | (31 << 11));
    assert_eq!(n64.cpu().gpr(2), u64::from(FCR31_CMP_BIT));
}

#[test]
fn undocumented_control_register_transfers_still_check_coprocessor_access() {
    for handler in [cfc1, ctc1] {
        let mut n64 = N64::new();
        n64.set_cp0_register(MmuRegister::Status, 0);
        n64.cpu_mut().set_gpr(2, 0x1234);
        handler(&mut n64, (2 << 16) | (1 << 11));
        assert_eq!(n64.cpu().gpr(2), 0x1234);
        assert_eq!(
            n64.cp0_register(MmuRegister::Cause),
            MmuCause::ExccodeCPU as u64 | MmuCause::CE1 as u64
        );
    }
}

#[test]
fn register_word_joins_preserve_raw_bits_in_both_fr_modes() {
    let mut n64 = N64::new();
    let mut bits = 0x8f01_2345_7f80_abcd_u64;

    for iteration in 0..1024 {
        bits ^= bits << 13;
        bits ^= bits >> 7;
        bits ^= bits << 17;
        let bytes = bits.to_le_bytes();
        let low: [u8; 4] = bytes[..4].try_into().unwrap();
        let high: [u8; 4] = bytes[4..].try_into().unwrap();
        let index = (iteration % 16) * 2;

        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = 0;
        n64.cpu.fpu.registers.fgr32[index] = low;
        n64.cpu.fpu.registers.fgr32[index + 1] = high;
        // FR=0 aliases odd double register indices to the even pair.
        assert_eq!(get_fpr_double(&n64, index).to_bits(), bits);
        assert_eq!(get_fpr_double(&n64, index + 1).to_bits(), bits);
        n64.fpu_mut().set_register_mode(true);

        assert_eq!(n64.cpu.fpu.registers.fgr64[index], bytes);
        n64.cpu.fpu.registers.fgr32[index] = [0; 4];
        n64.cpu.fpu.registers.fgr32[index + 1] = [0; 4];
        n64.fpu_mut().set_register_mode(false);

        assert_eq!(n64.cpu.fpu.registers.fgr32[index], low);
        assert_eq!(n64.cpu.fpu.registers.fgr32[index + 1], high);

        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = MmuStatus::FR as u64;

        for clear_high in [false, true] {
            n64.cpu.fpu.registers.fgr64[index + 1] = bytes;
            let single_bits = (bits >> 17) as u32;
            set_fpr_single(&mut n64, index + 1, f32::from_bits(single_bits), clear_high);
            let result = n64.cpu.fpu.registers.fgr64[index + 1];

            assert_eq!(&result[..4], &single_bits.to_le_bytes());
            assert_eq!(&result[4..], &if clear_high { [0; 4] } else { high });
            assert_eq!(n64.cpu.fpu.registers.fgr64[index], bytes);
        }
    }
}

#[test]
fn fpe_lookahead_uses_virtual_cache_index_for_tlb_aliases() {
    let mut n64 = N64::new();
    n64.cpu.execution.pc = 0x1000;
    n64.cpu.execution.pc_phys = 0;
    n64.cpu.mmu.translations.tlb_entries[0].pages[1].valid = true;
    n64.cpu.mmu.translations.tlb_entries[0].global = true;
    n64.memory.caches.icache[0].valid = true;
    n64.memory.caches.icache[0].tag = 0;
    n64.memory.caches.icache[0].words[1] = 0;
    n64.memory.caches.icache[128].valid = true;
    n64.memory.caches.icache[128].tag = 0;
    n64.memory.caches.icache[128].words[1] = 0x12 << 26;
    handle_fcr31(&mut n64, (4 << 7) | (4 << 12));

    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
        MmuCause::CE2 as u64 | MmuCause::ExccodeFPE as u64
    );
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::EPC as usize],
        0x1000
    );
}

#[test]
fn fpe_lookahead_never_invokes_mmio_bus_handlers() {
    fn side_effectful_read(n64: &mut N64, _address: u64, _size: AccessSize) -> u32 {
        crate::mmu::add_cycles(n64, 20);
        n64.mips.regs[0] = 0;

        0x12 << 26
    }
    let mut n64 = N64::new();
    n64.cpu.execution.pc = 0xFFFFFFFFA3EFFFFC;
    n64.cpu.execution.pc_phys = 0x03EFFFFC;
    n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] = 200;
    n64.mips.regs[0] = 0x12345678;
    n64.memory.bus.fast_read[0x03F0] = side_effectful_read;
    handle_fcr31(&mut n64, (4 << 7) | (4 << 12));

    assert_eq!(n64.cpu.mmu.registers.regs[MmuRegister::Count as usize], 202);
    assert_eq!(n64.mips.regs[0], 0x12345678);
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
        MmuCause::ExccodeFPE as u64
    );
}

#[test]
fn fpe_delay_slot_looks_at_branch_target_decode() {
    let mut n64 = N64::new();
    n64.cpu.execution.pc = 0xFFFFFFFF80000004;
    n64.cpu.execution.pc_phys = 4;
    n64.cpu.branch_state.state = cpu::State::DelaySlotTaken;
    n64.cpu.branch_state.pc = 0xFFFFFFFF80000010;
    n64.memory.caches.icache[0].valid = true;
    n64.memory.caches.icache[0].tag = 0;
    n64.memory.caches.icache[0].words[2] = 0x11 << 26; // skipped MFC1
    n64.memory.caches.icache[0].words[4] = 0; // branch target NOP
    handle_fcr31(&mut n64, (4 << 7) | (4 << 12));

    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
        MmuCause::BD as u64 | MmuCause::ExccodeFPE as u64
    );
    assert_eq!(
        n64.cpu.mmu.registers.regs[MmuRegister::EPC as usize],
        0xFFFFFFFF80000000
    );
}

#[test]
fn reserved_fp_formats_charge_their_decode_and_flush_latency() {
    for (format, function, expected) in [
        (16, 16, 4),
        (17, 33, 4),
        (20, 0, 5),
        (21, 36, 5),
        (20, 16, 4),
    ] {
        let mut n64 = N64::new();
        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = MmuStatus::CU1 as u64;
        reserved(&mut n64, (format << 21) | function);

        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::Count as usize],
            expected
        );
        assert_eq!(n64.cpu.fpu.control.fcr31, FCR31_CAUSE_UNIMP_BIT);
    }
}

#[test]
fn fcsr_software_exception_reports_following_pipeline_coprocessor() {
    for (coprocessor, pending) in [
        (1u64, 0),
        (2, 0),
        (1, MmuCause::IP7 as u64),
        (2, MmuCause::IP7 as u64),
    ] {
        let mut n64 = N64::new();
        n64.cpu.execution.pc = 0xFFFFFFFF80000000;
        n64.cpu.execution.pc_phys = 0;
        n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] = pending;
        n64.memory.caches.icache[0].valid = true;
        n64.memory.caches.icache[0].tag = 0;
        n64.memory.caches.icache[0].words[1] = ((0x10 + coprocessor) << 26) as u32;
        handle_fcr31(&mut n64, (4 << 7) | (4 << 12));

        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
            MmuCause::ExccodeFPE as u64 | (coprocessor << 28) | pending
        );
        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::EPC as usize],
            0xFFFFFFFF80000000
        );
    }
}

#[test]
fn reserved_cop1_decodes_to_unimplemented_with_usable_priority() {
    let reserved_rs = [
        3, 7, 9, 10, 11, 12, 13, 14, 15, 18, 19, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    ];

    for rs in reserved_rs {
        for usable in [false, true] {
            let mut n64 = N64::new();

            if usable {
                n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = MmuStatus::CU1 as u64;
            }

            n64.cpu.fpu.instructions.instrs[rs as usize](&mut n64, rs << 21);
            let expected = if usable {
                MmuCause::ExccodeFPE as u64
            } else {
                MmuCause::ExccodeCPU as u64 | MmuCause::CE1 as u64
            };

            assert_eq!(
                n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize],
                expected
            );
            assert_eq!(
                n64.cpu.fpu.control.fcr31,
                if usable { FCR31_CAUSE_UNIMP_BIT } else { 0 }
            );
        }
    }

    for rt in 4..32 {
        let mut n64 = N64::new();
        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = MmuStatus::CU1 as u64;
        execute_fpu_b(&mut n64, (8 << 21) | (rt << 16));

        assert_eq!(n64.cpu.fpu.control.fcr31, FCR31_CAUSE_UNIMP_BIT);
    }
}

#[test]
fn unaligned_fpu_memory_operations_fault_before_memory_access() {
    for (instruction, cause, offset) in [
        (lwc1 as fn(&mut N64, u32), MmuCause::ExccodeAdEL as u64, 2),
        (ldc1, MmuCause::ExccodeAdEL as u64, 4),
        (swc1, MmuCause::ExccodeAdES as u64, 2),
        (sdc1, MmuCause::ExccodeAdES as u64, 4),
    ] {
        let mut n64 = N64::new();
        n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] = MmuStatus::CU1 as u64;
        n64.cpu.registers.gpr[1] = 0xFFFFFFFF80000000;
        n64.cpu.fpu.registers.fgr32[2] = 0x12345678u32.to_le_bytes();
        n64.rdram.mem.write_u32(0, 0x89ABCDEF, u32::MAX);
        instruction(&mut n64, (1 << 21) | (2 << 16) | offset);

        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] & MmuCause::ExccodeMask as u64,
            cause
        );
        assert_eq!(
            n64.cpu.mmu.registers.regs[MmuRegister::BadVAddr as usize],
            0xFFFFFFFF80000000 + offset as u64
        );
        assert_eq!(n64.cpu.fpu.registers.fgr32[2], 0x12345678u32.to_le_bytes());
        assert_eq!(n64.rdram.mem.read_u32(0), 0x89ABCDEF);
    }
}

#[test]
fn aligned_fpu_memory_operations_still_transfer_register_bits() {
    let mut n64 = N64::new();
    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] =
        MmuStatus::CU1 as u64 | MmuStatus::FR as u64;
    n64.cpu.registers.gpr[1] = 0xFFFFFFFFA0000000;
    n64.memory.bus.memory_map_read[0] = crate::rdram::read_mem_fast;
    n64.memory.bus.memory_map_write[0] = crate::rdram::write_mem;
    crate::rdram::write_mem(&mut n64, 4, 0x12345678, u32::MAX);
    crate::rdram::write_mem(&mut n64, 8, 0x89ABCDEF, u32::MAX);
    crate::rdram::write_mem(&mut n64, 12, 0x01234567, u32::MAX);
    lwc1(&mut n64, (1 << 21) | (2 << 16) | 4);
    ldc1(&mut n64, (1 << 21) | (3 << 16) | 8);

    assert_eq!(get_fpr_single(&n64, 2).to_bits(), 0x12345678);
    assert_eq!(get_fpr_double(&n64, 3).to_bits(), 0x89ABCDEF01234567);
    swc1(&mut n64, (1 << 21) | (2 << 16) | 20);
    sdc1(&mut n64, (1 << 21) | (3 << 16) | 24);

    assert_eq!(n64.rdram.mem.read_u32(20), 0x12345678);
    assert_eq!(n64.rdram.mem.read_u32(24), 0x89ABCDEF);
    assert_eq!(n64.rdram.mem.read_u32(28), 0x01234567);
    assert_eq!(n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize], 0);
}
