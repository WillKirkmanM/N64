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
