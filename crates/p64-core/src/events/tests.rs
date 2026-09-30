use super::*;
use crate::rsp::{self, SpReg, SpStatus, SpStatusWrite};

fn canceled_handler(_: &mut N64) {
    panic!("Canceled event was invoked");
}

fn following_handler(n64: &mut N64) {
    n64.rsp_mut().registers.gpr[1] += 1;
}

#[test]
fn rebasing_preserves_due_events_and_future_time_offsets() {
    let mut n64 = N64::new();
    create_event(&mut n64, EventType::SP, 95, following_handler);
    create_event(&mut n64, EventType::VI, 105, following_handler);

    translate_events(&mut n64, 100, 0);

    assert_eq!(n64.cpu().event_deadline(EventType::SP), 0);
    assert_eq!(n64.cpu().event_deadline(EventType::VI), 5);
    assert_eq!(n64.cpu().scheduler.next_event_count, 0);
    trigger_event(&mut n64);
    assert_eq!(n64.rsp().registers.gpr[1], 1);
    assert_eq!(n64.cpu().scheduler.next_event_count, 5);
}
