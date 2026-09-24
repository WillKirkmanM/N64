use super::*;

#[derive(Clone, Copy)]
pub(super) enum PifCommand {
    SetupChannels = 0x01,
    ProcessChallenge = 0x02,
    Unknown = 0x08,
    ClearRom = 0x10,
    SetStatusBit = 0x20,
}

pub struct Pif {
    pub rom: [u8; 1984],
    pub ram: [u8; 64],
    pub channels: [PifChannel; 5],
}

#[derive(Copy, Clone, Default)]
pub struct PifChannel {
    pub tx: Option<usize>,
    pub tx_buf: Option<usize>,
    pub rx: Option<usize>,
    pub rx_buf: Option<usize>,
    pub process: Option<fn(&mut N64, usize)>,
    pub pak_handler: Option<peripheral::PakHandler>,
}

impl Default for Pif {
    fn default() -> Self {
        // Create base PIF with zeroed memory
        let mut pif = Self {
            rom: [0; 1984],
            ram: [0; 64],
            channels: [PifChannel::default(); 5],
        };

        // Configure controller channel
        pif.channels[CONTROLLER_CHANNEL] = PifChannel {
            pak_handler: Some(PakHandler {
                read: mempak::read,
                write: mempak::write,
            }),
            process: Some(peripheral::process),
            ..Default::default()
        };

        // Configure cartridge channel
        pif.channels[CART_CHANNEL] = PifChannel {
            process: Some(cartridge::process),
            ..Default::default()
        };

        pif
    }
}
