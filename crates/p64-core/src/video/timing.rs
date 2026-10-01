pub struct VideoTiming {
    pub clock: u64,
    pub delay: u64,
    pub field: u32,
}

impl Default for VideoTiming {
    fn default() -> Self {
        Self {
            clock: super::super::NTSC_CLOCK,
            delay: 0,
            field: 0,
        }
    }
}
