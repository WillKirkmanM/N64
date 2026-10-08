#[derive(Clone, Copy, Default)]
pub struct ChannelBuffers {
    pub tx: Option<usize>,
    pub tx_buf: Option<usize>,
    pub rx: Option<usize>,
    pub rx_buf: Option<usize>,
}
