use super::{CART_CHANNEL, CONTROLLER_CHANNEL, PIF_CHANNELS_COUNT};
use crate::peripheral::PakHandler;
use crate::{cartridge, mempak, peripheral, N64};

#[path = "channel.rs"]
mod channel;
#[path = "memory.rs"]
mod memory;
pub use channel::ChannelBuffers;
pub use memory::PifMemory;

#[derive(Clone, Copy)]
pub(super) enum PifCommand {
    SetupChannels = 0x01,
    ProcessChallenge = 0x02,
    Unknown = 0x08,
    ClearRom = 0x10,
    SetStatusBit = 0x20,
}

impl PifCommand {
    pub(super) const ALL: [Self; 5] = [
        Self::SetupChannels,
        Self::ProcessChallenge,
        Self::Unknown,
        Self::ClearRom,
        Self::SetStatusBit,
    ];
}

pub struct Pif {
    pub memory: PifMemory,
    pub channels: [PifChannel; PIF_CHANNELS_COUNT],
}

impl Default for Pif {
    fn default() -> Self {
        let mut pif = Self {
            memory: PifMemory::default(),
            channels: [PifChannel::default(); PIF_CHANNELS_COUNT],
        };

        pif.channels[CONTROLLER_CHANNEL] = PifChannel {
            pak_handler: Some(PakHandler {
                read: mempak::read,
                write: mempak::write,
            }),
            process: Some(peripheral::process),
            ..Default::default()
        };

        pif.channels[CART_CHANNEL] = PifChannel {
            process: Some(cartridge::process),
            ..Default::default()
        };

        pif
    }
}

#[derive(Copy, Clone, Default)]
pub struct PifChannel {
    pub buffers: ChannelBuffers,
    pub process: Option<fn(&mut N64, usize)>,
    pub pak_handler: Option<peripheral::PakHandler>,
}

impl PifChannel {
    pub(crate) fn tx_buffer(&self) -> usize {
        self.buffers.tx_buf.expect("configured PIF transmit buffer")
    }

    pub(crate) fn rx_buffer(&self) -> usize {
        self.buffers.rx_buf.expect("configured PIF receive buffer")
    }

    pub(super) fn disable(&mut self) {
        self.buffers.tx = None;
        self.buffers.rx = None;
        self.buffers.tx_buf = None;
        self.buffers.rx_buf = None;
    }

    pub(super) fn configure(&mut self, offset: usize, tx_length: u8) {
        self.buffers.tx = Some(offset);
        self.buffers.rx = Some(offset + 1);
        self.buffers.tx_buf = Some(offset + 2);
        self.buffers.rx_buf = Some(offset + 2 + tx_length as usize);
    }
}
