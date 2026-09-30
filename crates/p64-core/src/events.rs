use crate::{cpu::Cpu, N64};

mod types;
pub use types::{Event, EventType};

pub type EventHandler = fn(&mut N64);

impl Cpu {
    pub(crate) fn event_deadline(&self, event: EventType) -> u64 {
        self.scheduler.events[event as usize].count
    }

    pub fn arm_event(&mut self, event: EventType, when: u64, handler: EventHandler) {
        self.scheduler.events[event as usize] = Event::new(when, handler);
        self.refresh_next_event();
    }

    /// Mutating a deadline requires `refresh_next_event` after the update.
    pub fn scheduled_event(&mut self, event: EventType) -> Option<&mut Event> {
        let event = &mut self.scheduler.events[event as usize];

        event.enabled.then_some(event)
    }

    /// Disable without refreshing; handlers may batch changes before refreshing.
    pub fn disable_event(&mut self, event: EventType) {
        self.scheduler.events[event as usize].enabled = false;
    }

    fn take_next_handler(&mut self) -> EventHandler {
        let event = &mut self.scheduler.events[self.scheduler.next_event];
        event.enabled = false;

        event.handler
    }

    pub fn refresh_next_event(&mut self) {
        (self.scheduler.next_event, self.scheduler.next_event_count) = self
            .scheduler
            .events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.enabled)
            .min_by_key(|(_, event)| event.count)
            .map(|(index, event)| (index, event.count))
            .unwrap_or((0, u64::MAX));
    }

    fn shift_event_counts(&mut self, old_count: u64, new_count: u64) {
        for event in self
            .scheduler
            .events
            .iter_mut()
            .filter(|event| event.enabled)
        {
            event.count = new_count.saturating_add(event.count.saturating_sub(old_count));
        }

        self.refresh_next_event();
    }
}

pub fn create_event(n64: &mut N64, event: EventType, when: u64, handler: EventHandler) {
    n64.cpu_mut().arm_event(event, when, handler);
}

pub fn get_event(n64: &mut N64, event: EventType) -> Option<&mut Event> {
    n64.cpu_mut().scheduled_event(event)
}

pub fn remove_event(n64: &mut N64, event: EventType) {
    let cpu = n64.cpu_mut();
    cpu.disable_event(event);
    cpu.refresh_next_event();
}

pub fn trigger_event(n64: &mut N64) {
    let handler = n64.cpu_mut().take_next_handler();
    handler(n64);
    set_next_event(n64);
}

pub fn set_next_event(n64: &mut N64) {
    n64.cpu_mut().refresh_next_event();
}

pub fn translate_events(n64: &mut N64, old_count: u64, new_count: u64) {
    n64.cpu_mut().shift_event_counts(old_count, new_count);
}

#[inline(never)]
pub fn dummy_event(_: &mut N64) {
    panic!("dummy event");
}
