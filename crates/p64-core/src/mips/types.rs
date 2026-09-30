use super::{MI_INIT_LENGTH_MASK, MI_INTR_MASK_REG, MI_INTR_REG, MI_REGS_COUNT, MI_VERSION_REG};
use crate::builder::registers::{FlagCommand, Registers};

const HARDWARE_VERSION: u32 = 0x02020102;

pub struct Mips {
    pub regs: Registers,
}

impl Default for Mips {
    fn default() -> Self {
        let mut mips = Self {
            regs: Registers::new()
                .with_size(MI_REGS_COUNT as usize)
                .with_write_mask(MI_INIT_LENGTH_MASK)
                .build(),
        };

        mips.regs[MI_VERSION_REG as usize] = HARDWARE_VERSION;

        mips
    }
}

impl Mips {
    pub(super) fn register(&self, index: u32) -> u32 {
        self.regs[index as usize]
    }

    pub(super) fn interrupt_pending(&self) -> bool {
        self.register(MI_INTR_REG) & self.register(MI_INTR_MASK_REG) != 0
    }

    pub(super) fn apply_commands(&mut self, index: u32, command: u32, controls: &[FlagCommand]) {
        let state = &mut self.regs[index as usize];

        for control in controls {
            control.apply(state, command);
        }
    }
}
