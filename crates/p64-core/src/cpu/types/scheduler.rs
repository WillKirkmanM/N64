use crate::events::{self, Event, EventType};

pub struct CpuScheduler {
    pub events: [Event; EventType::Count as usize],
    pub next_event_count: u64,
    pub next_event: usize,
}

impl Default for CpuScheduler {
    fn default() -> Self {
        Self {
            events: [Event {
                enabled: false,
                count: u64::MAX,
                handler: events::dummy_event,
            }; EventType::Count as usize],
            next_event_count: u64::MAX,
            next_event: 0,
        }
    }
}
