use super::*;

fn machine() -> N64 {
    let mut machine = N64::new();
    machine.rdram_interface.regs[RiReg::Refresh as usize] = 0;
    machine.rdram_interface.timing.banks[0].row = Some(0);
    machine
}

#[test]
fn sysad_edges_follow_physical_time_across_count_writes() {
    for (count, expected) in [(300, 31), (301, 32), (302, 32)] {
        let mut n64 = machine();
        n64.set_cp0_register(MmuRegister::Count, count);
        read(&mut n64, 0, AccessSize::Word);
        assert_eq!(n64.cp0_register(MmuRegister::Count) - count, expected);
        let mut rebased = machine();
        rebased.rdram_interface.timing.rebase(count, 0);
        read(&mut rebased, 0, AccessSize::Word);
        assert_eq!(rebased.cp0_register(MmuRegister::Count), expected);
    }
}

#[test]
fn row_retry_tracks_clean_dirty_and_independent_banks() {
    let mut bank = Bank::default();
    assert_eq!(bank.open(0), 66);
    assert_eq!(bank.open(0), 0);
    assert_eq!(bank.open(1), 66);
    bank.dirty = true;
    assert_eq!(bank.open(2), 90);
    assert!(!bank.dirty);
    let mut timing = RdramTiming::default();
    timing.write(0);
    timing.write(1 << 20);
    assert!(timing.banks[0].dirty);
    assert!(timing.banks[1].dirty);
    assert!(timing.banks[2].row.is_none());
}

#[test]
fn refresh_wait_uses_programmed_delays_and_preserves_open_rows() {
    let mut timing = RdramTiming::default();
    timing.banks[0].row = Some(17);
    assert_eq!(timing.refresh_wait(0, 48_000, 0x63634), 624);
    assert_eq!(timing.banks[0].row, Some(17));
    assert_eq!(timing.refresh_wait(624, 48_000, 0x63634), 0);
    timing.banks[0].dirty = true;
    assert_eq!(timing.refresh_wait(48_000, 48_000, 0x63634), 648);
    assert!(!timing.banks[0].dirty);
    assert_eq!(timing.banks[0].row, Some(17));
    assert_eq!(timing.refresh_wait(96_000, 48_000, 0), 0);
}
