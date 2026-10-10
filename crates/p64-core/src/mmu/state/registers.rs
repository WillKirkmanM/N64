use super::super::MmuRegister;
use crate::builder::registers::Registers;

pub struct MmuRegisters {
    pub reg_latch: u64,
    pub regs: Registers<u64>,
    pub reg_write_masks: [u64; MmuRegister::RegsCount as usize],
}

impl Default for MmuRegisters {
    fn default() -> Self {
        Self {
            reg_latch: 0,
            regs: Registers::new().with_size(MmuRegister::RegsCount as usize),
            reg_write_masks: [0; MmuRegister::RegsCount as usize],
        }
    }
}

impl MmuRegisters {
    pub(in crate::mmu) fn initialize(&mut self) {
        self.reg_write_masks = std::array::from_fn(|index| {
            MmuRegister::from_index(index).map_or(0, MmuRegister::write_mask)
        });

        for index in 0..MmuRegister::RegsCount as usize {
            if let Some(register) = MmuRegister::from_index(index) {
                if let Some(value) = register.reset_value() {
                    self.set_register(register, value);
                }
            }
        }
    }

    #[inline]
    pub fn register(&self, register: MmuRegister) -> u64 {
        self.regs[register]
    }

    /// Internal access; guest writes use the masked instruction path.
    #[inline]
    pub fn register_mut(&mut self, register: MmuRegister) -> &mut u64 {
        &mut self.regs[register]
    }

    #[inline]
    pub fn set_register(&mut self, register: MmuRegister, value: u64) {
        self.regs[register] = value;
    }

    pub(super) fn write_masked(&mut self, index: usize, data: u64) {
        let mask = self.reg_write_masks[index];
        crate::memory::masked_write_64(&mut self.regs[index], data, mask);
    }

    pub(super) fn control_register(&self, index: u32) -> u64 {
        self.regs[index as usize]
    }
}
