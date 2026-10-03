use super::*;

fn write_status(n64: &mut N64, command: u32) {
    write_regs_dpc(n64, DpcReg::Status as u64 * 4, command, u32::MAX);
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
