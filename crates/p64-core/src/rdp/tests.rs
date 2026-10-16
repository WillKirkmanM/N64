use super::*;

fn write_status(n64: &mut N64, command: u32) {
    write_regs_dpc(n64, DpcReg::Status as u64 * 4, command, u32::MAX);
}

#[test]
fn rdp_clock_is_readonly_free_running_and_24_bit() {
    let mut n64 = N64::new();
    let clock_address = (DpcReg::Clock as u64) * 4;
    n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] = 300;

    assert_eq!(
        read_regs_dpc(&mut n64, clock_address, memory::AccessSize::Word),
        213
    );
    write_regs_dpc(&mut n64, clock_address, 0xabcdef, u32::MAX);
    write_status(&mut n64, DpcStatusWrite::SET_FREEZE.bits());
    n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] += 200_000;

    assert_eq!(clock(&n64), 213 + 133_333);
    n64.cpu.mmu.registers.regs[MmuRegister::Count as usize] = (1 << 24) * 3 / 2 + 3;

    assert_eq!(clock(&n64), 2);
}

#[test]
fn clearing_rdp_counters_clears_their_busy_flags() {
    let mut n64 = N64::new();
    n64.rdp.registers.regs_dpc[DpcReg::Status as usize] |=
        (DpcStatus::TMEM_BUSY | DpcStatus::PIPE_BUSY | DpcStatus::CMD_BUSY).bits();
    for reg in [
        DpcReg::Tmem,
        DpcReg::PipeBusy,
        DpcReg::BufBusy,
        DpcReg::Clock,
    ] {
        n64.rdp.registers.regs_dpc[reg as usize] = 123;
    }

    write_status(
        &mut n64,
        (DpcStatusWrite::CLR_TMEM_CTR
            | DpcStatusWrite::CLR_PIPE_CTR
            | DpcStatusWrite::CLR_CMD_CTR
            | DpcStatusWrite::CLR_CLOCK_CTR)
            .bits(),
    );

    assert_eq!(
        n64.rdp.registers.regs_dpc[DpcReg::Status as usize]
            & (DpcStatus::TMEM_BUSY | DpcStatus::PIPE_BUSY | DpcStatus::CMD_BUSY).bits(),
        0
    );
    for reg in [
        DpcReg::Tmem,
        DpcReg::PipeBusy,
        DpcReg::BufBusy,
        DpcReg::Clock,
    ] {
        assert_eq!(n64.rdp.registers.regs_dpc[reg as usize], 0);
    }

    assert_ne!(
        n64.rdp.registers.regs_dpc[DpcReg::Status as usize] & DpcStatus::CBUF_READY.bits(),
        0
    );
}

#[test]
fn full_sync_event_clears_busy_before_asserting_dp_interrupt() {
    let mut n64 = N64::new();
    n64.rdp.registers.regs_dpc[DpcReg::Status as usize] = (DpcStatus::START_GCLK
        | DpcStatus::PIPE_BUSY
        | DpcStatus::CMD_BUSY
        | DpcStatus::CBUF_READY
        | DpcStatus::XBUS_DMEM_DMA)
        .bits();
    rdp_interrupt_event(&mut n64);

    assert_eq!(
        n64.rdp.registers.regs_dpc[DpcReg::Status as usize],
        (DpcStatus::CBUF_READY | DpcStatus::XBUS_DMEM_DMA).bits()
    );
    assert_ne!(
        mips::read_regs(&mut n64, 8, memory::AccessSize::Word) & mips::MI_INTR_DP,
        0
    );
}
