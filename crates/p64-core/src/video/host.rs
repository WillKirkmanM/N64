//! Host frame pacing; these settings do not alter VI deadlines.
pub struct VideoHost {
    pub limiter: Option<governor::DefaultDirectRateLimiter>,
    pub host_throttle: bool,
}

impl Default for VideoHost {
    fn default() -> Self {
        Self {
            limiter: None,
            host_throttle: true,
        }
    }
}
