use crate::{
    audio::Audio,
    cartridge::Cartridge,
    cpu::Cpu,
    is_viewer,
    memory::Memory,
    mips::Mips,
    peripheral::Peripheral,
    pif::Pif,
    rdp::Rdp,
    rdram::{Rdram, RdramInterface},
    recompiler,
    rsp::Rsp,
    serial::Serial,
    sram::Flashram,
    video::Video,
};
use p64_gui::ui::Ui;

const RDRAM_BYTE_LANE_XOR: usize = if cfg!(target_endian = "little") { 3 } else { 0 };

pub struct N64 {
    pub ui: Ui,
    pub byte_swap: usize,
    pub cpu: Cpu,
    pub pif: Pif,
    pub cartridge: Cartridge,
    pub memory: Memory,
    pub rsp: Rsp,
    pub rdp: Rdp,
    pub rdram: Rdram,
    pub mips: Mips,
    pub peripheral: Peripheral,
    pub video: Video,
    pub audio: Audio,
    pub serial: Serial,
    pub rdram_interface: RdramInterface,
    pub flashram: Flashram,
    pub is_viewer: is_viewer::IsViewer,
    pub execution_engine: recompiler::Engine,
}

impl Default for N64 {
    fn default() -> Self {
        Self::with_ui(Ui::default())
    }
}

impl N64 {
    pub fn with_ui(ui: Ui) -> Self {
        Self {
            ui,
            // RDRAM words use canonical little-endian storage on every host.
            byte_swap: RDRAM_BYTE_LANE_XOR,
            cpu: Cpu::default(),
            pif: Pif::default(),
            cartridge: Cartridge::default(),
            memory: Memory::default(),
            rsp: Rsp::default(),
            rdp: Rdp::default(),
            rdram: Rdram::default(),
            mips: Mips::default(),
            peripheral: Peripheral::default(),
            video: Video::default(),
            audio: Audio::default(),
            serial: Serial::default(),
            rdram_interface: RdramInterface::default(),
            flashram: Flashram::default(),
            is_viewer: is_viewer::IsViewer::default(),
            execution_engine: recompiler::Engine::default(),
        }
    }
}
