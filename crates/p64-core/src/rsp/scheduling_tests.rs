use super::*;

#[test]
fn delayed_dispatch_preserves_rsp_clock_deadlines() {
    let mut n64 = N64::new();
    for instruction in &mut n64.rsp_mut().instruction.instructions[..3] {
        *instruction = Instructions {
            func: mtc0,
            opcode: 0x4080_3800,
        };
    }
    create_event(&mut n64, EventType::SP, 100, rsp_event);
    n64.set_cp0_register(MmuRegister::Count, 150);

    for expected in [101, 103, 104] {
        crate::events::trigger_event(&mut n64);
        assert_eq!(n64.cpu().event_deadline(EventType::SP), expected);
        assert_eq!(n64.cp0_register(MmuRegister::Count), 150);
    }
    assert_eq!(n64.rsp().registers.pc(), 12);
}
