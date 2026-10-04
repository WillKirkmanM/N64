use super::*;

fn finish_scheduled_slice(n64: &mut N64) {
    let event = n64.cpu.scheduler.events[EventType::SP as usize];
    assert!(event.enabled);
    n64.set_cp0_register(MmuRegister::Count, event.count);
    n64.cpu_mut().disable_event(EventType::SP);
    (event.handler)(n64);
    n64.cpu_mut().refresh_next_event();
}

#[test]
fn single_step_halts_at_the_scheduled_instruction_boundary() {
    let mut n64 = N64::new();
    start_rsp(&mut n64);
    n64.set_cp0_register(MmuRegister::Count, 100);
    for (address, opcode) in [(0, 0x2401_0001), (4, 0x2421_0001), (8, 13)] {
        write_mem(&mut n64, 0x1000 + address, opcode, u32::MAX);
    }
    write_regs(&mut n64, 16, SpStatusWrite::SetSstep as u32, u32::MAX);
    write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
    assert_eq!(n64.rsp.registers.gpr[1], 1);
    assert_eq!(n64.rsp.registers.pc(), 4);
    assert_eq!(
        n64.rsp.registers.regs[SpReg::Status as usize],
        SpStatus::Sstep as u32
    );
    assert!(!n64.rsp.processor.broken);
    assert_eq!(n64.cpu.scheduler.events[EventType::SP as usize].count, 101);

    finish_scheduled_slice(&mut n64);
    assert_eq!(
        n64.rsp.registers.regs[SpReg::Status as usize],
        SpStatus::Sstep as u32 | SpStatus::Halt as u32
    );
    assert!(!n64.cpu.scheduler.events[EventType::SP as usize].enabled);

    write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
    assert_eq!(n64.rsp.registers.gpr[1], 2);
    assert_eq!(n64.rsp.registers.pc(), 8);
    finish_scheduled_slice(&mut n64);
    write_regs(&mut n64, 16, SpStatusWrite::ClrHalt as u32, u32::MAX);
    finish_scheduled_slice(&mut n64);
    assert!(n64.rsp.processor.broken);
    assert_eq!(n64.rsp.registers.regs[SpReg::Status as usize] & 3, 3);
}

fn vector_opcode(element: u8) -> u32 {
    (1 << 21) | (2 << 16) | ((element as u32) << 7)
}
