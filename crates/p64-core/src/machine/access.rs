use super::N64;
use crate::{cpu::Cpu, fpu, mmu, rcp, rdp::Rdp, rsp::Rsp};
use p64_gui::storage::{SaveData, SaveTypes};

use super::buffers::SavePacketBuffers;

impl N64 {
    pub fn has_save_type(&self, save_type: SaveTypes) -> bool {
        self.ui.save_data.save_type.contains(&save_type)
    }

    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    #[inline]
    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    #[inline]
    pub fn cp0(&self) -> &mmu::Mmu {
        &self.cpu.mmu
    }

    #[inline]
    pub fn cp0_mut(&mut self) -> &mut mmu::Mmu {
        &mut self.cpu.mmu
    }

    /// Read internal CP0 state without applying guest read hazards.
    #[inline]
    pub fn cp0_register(&self, register: mmu::MmuRegister) -> u64 {
        self.cp0().regs[register as usize]
    }

    /// Update internal CP0 state; guest writes use MMU instruction handlers.
    #[inline]
    pub fn set_cp0_register(&mut self, register: mmu::MmuRegister, value: u64) {
        self.cp0_mut().regs[register as usize] = value;
    }

    #[inline]
    pub fn update_cp0_register(
        &mut self,
        register: mmu::MmuRegister,
        update: impl FnOnce(u64) -> u64,
    ) {
        let value = update(self.cp0_register(register));
        self.set_cp0_register(register, value);
    }

    #[inline]
    pub fn fpu(&self) -> &fpu::Fpu {
        &self.cpu.fpu
    }

    #[inline]
    pub fn fpu_mut(&mut self) -> &mut fpu::Fpu {
        &mut self.cpu.fpu
    }

    #[inline]
    pub fn rcp(&self) -> &rcp::Rcp {
        &self.cpu.rcp
    }

    #[inline]
    pub fn rcp_mut(&mut self) -> &mut rcp::Rcp {
        &mut self.cpu.rcp
    }

    #[inline]
    pub fn rdp(&self) -> &Rdp {
        &self.rdp
    }

    #[inline]
    pub fn rdp_mut(&mut self) -> &mut Rdp {
        &mut self.rdp
    }

    #[inline]
    pub fn rsp(&self) -> &Rsp {
        &self.rsp
    }

    #[inline]
    pub fn rsp_mut(&mut self) -> &mut Rsp {
        &mut self.rsp
    }

    #[inline]
    pub fn save_data(&self) -> &SaveData {
        &self.ui.save_data.data
    }

    #[inline]
    pub fn save_data_mut(&mut self) -> &mut SaveData {
        &mut self.ui.save_data.data
    }
}

impl N64 {
    pub(crate) fn rdp_buffers(&mut self) -> (&mut [u32; 8], &mut [u8]) {
        let registers = self
            .rdp
            .registers
            .regs_dpc
            .try_as_array_mut()
            .expect("RDP register bank has eight words");
        (registers, &mut self.rsp.memory.mem.data[..])
    }
}
