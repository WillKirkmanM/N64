use super::*;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum EventType {
    AI,
    SI,
    VI,
    PI,
    DP,
    SP,
    InterruptCheck,
    SPDma,
    Compare,
    Count,
}

#[derive(PartialEq, Copy, Clone)]
pub struct Event {
    pub enabled: bool,
    pub count: u64,
    pub handler: fn(&mut N64),
}

impl Default for Event {
    fn default() -> Self {
        fn default_handler(_n64: &mut N64) {}

        Event {
            enabled: false,
            count: 0,
            handler: default_handler,
        }
    }
}

impl Event {
    pub(super) fn new(count: u64, handler: fn(&mut N64)) -> Self {
        Self {
            enabled: true,
            count,
            handler,
        }
    }
}
