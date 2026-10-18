use super::*;
use crate::{events::EventType, mmu};

fn machine() -> N64 {
    let mut n64 = N64::new();
    n64.cpu_mut().execution.running = 1;
    n64.cpu_mut().execution.pc = 0xffff_ffff_8000_0000;
    n64
}

fn complete_twenty_cycle_instruction(n64: &mut N64) {
    mmu::begin_instruction(n64);
    add_cycles(n64, 20);
    retire_scalar(n64);
}

fn recurring_sp(n64: &mut N64) {
    assert_eq!(n64.cp0_register(MmuRegister::Count), 21);
    assert_eq!(n64.cp0().clock.instruction_clock, 1);
    let deadline = n64.cpu().event_deadline(EventType::SP);
    let cpu = n64.cpu_mut();
    cpu.registers.gpr[1] += 1;
    let period = cpu.registers.gpr[2];
    events::create_event(n64, EventType::SP, deadline + period, recurring_sp);
}

#[test]
fn short_rsp_slices_catch_up_after_a_long_cpu_instruction() {
    for period in [1, 2] {
        let mut n64 = machine();
        n64.cpu_mut().set_gpr(2, period);
        events::create_event(&mut n64, EventType::SP, period, recurring_sp);
        complete_twenty_cycle_instruction(&mut n64);
        assert_eq!(n64.cpu().gpr(1), 20 / period);
        assert_eq!(n64.cpu().event_deadline(EventType::SP), 20 + period);
        assert_eq!(n64.cp0_register(MmuRegister::Count), 21);
        assert_eq!(n64.cpu().execution.pc, 0xffff_ffff_8000_0004);
        assert_eq!(n64.cp0().clock.instruction_clock, 1);
    }
}

fn record(n64: &mut N64, digit: u64) {
    assert_eq!(n64.cp0_register(MmuRegister::Count), 21);
    n64.cpu_mut().registers.gpr[1] = n64.cpu().gpr(1) * 10 + digit;
}

fn record_vi(n64: &mut N64) {
    record(n64, 1);
}
fn record_pi(n64: &mut N64) {
    record(n64, 2);
}
fn record_sp(n64: &mut N64) {
    record(n64, 3);
}

#[test]
fn due_devices_drain_in_deadline_order_and_existing_same_date_priority() {
    let mut n64 = machine();
    events::create_event(&mut n64, EventType::SP, 10, record_sp);
    events::create_event(&mut n64, EventType::VI, 10, record_vi);
    events::create_event(&mut n64, EventType::PI, 5, record_pi);
    complete_twenty_cycle_instruction(&mut n64);
    assert_eq!(n64.cpu().gpr(1), 213);
    assert_eq!(n64.cpu().scheduler.next_event_count, u64::MAX);
    assert_eq!(n64.cp0_register(MmuRegister::Count), 21);
}

fn complete_viewer(n64: &mut N64) {
    n64.is_viewer.completed = true;
}
fn restart_from_vi(n64: &mut N64) {
    n64.cpu_mut().execution.running = 1;
    n64.cpu_mut().set_gpr(1, 99);
}

#[test]
fn viewer_completion_stops_before_a_later_due_vi_can_restart_execution() {
    let mut n64 = machine();
    n64.is_viewer.stop_on_completion = true;
    events::create_event(&mut n64, EventType::SP, 5, complete_viewer);
    events::create_event(&mut n64, EventType::VI, 6, restart_from_vi);
    complete_twenty_cycle_instruction(&mut n64);
    assert_eq!(n64.cpu().execution.running, 0);
    assert_eq!(n64.cpu().gpr(1), 0);
    assert!(n64.cpu().scheduler.events[EventType::VI as usize].enabled);
}

fn stop(n64: &mut N64) {
    n64.cpu_mut().execution.running = 0;
}

fn transient_zero_cycle_sp(n64: &mut N64) {
    let deadline = n64.cpu().event_deadline(EventType::SP);
    n64.cpu_mut().registers.gpr[1] += 1;
    if n64.cpu().gpr(1) == 1 {
        events::create_event(n64, EventType::SP, deadline, transient_zero_cycle_sp);
    }
}

fn raise_exception(n64: &mut N64) {
    n64.cpu_mut().branch_state.state = State::Exception;
}
fn verify_exception_cleanup(n64: &mut N64) {
    assert!(matches!(n64.cpu().branch_state.state, State::Step));
    n64.cpu_mut().set_gpr(1, 1);
}
