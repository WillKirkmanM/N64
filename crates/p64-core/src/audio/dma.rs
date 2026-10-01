#[derive(Default)]
pub struct AudioDma {
    pub fifo: [AiDma; 2],
    pub last_read: u64,
    pub delayed_carry: bool,
}

#[derive(Copy, Clone, Default)]
pub struct AiDma {
    pub address: u64,
    pub length: u64,
    pub duration: u64,
}
