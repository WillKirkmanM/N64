mod clock;
mod instructions;
mod registers;
mod translations;

pub use clock::MmuClock;
pub use instructions::MmuInstructionTables;
pub use registers::MmuRegisters;
pub use translations::MmuTranslations;

use super::{status::Status, MmuRegister};
use crate::tlb;

const RANDOM_RESET: u64 = (tlb::ENTRY_COUNT - 1) as u64;
const RANDOM_MASK: u64 = 0x3F;

#[derive(Default)]
pub struct Mmu {
    pub clock: MmuClock,
    pub registers: MmuRegisters,
    pub instructions: MmuInstructionTables,
    pub translations: MmuTranslations,
}

impl Mmu {
    pub fn initialized() -> Self {
        Self {
            instructions: MmuInstructionTables::initialized(),
            ..Self::default()
        }
    }

    #[inline]
    pub fn register(&self, register: MmuRegister) -> u64 {
        self.registers.register(register)
    }

    /// Raw internal access. Guest MTC0 writes use the masked instruction path.
    #[inline]
    pub fn register_mut(&mut self, register: MmuRegister) -> &mut u64 {
        self.registers.register_mut(register)
    }

    #[inline]
    pub fn set_register(&mut self, register: MmuRegister, value: u64) {
        self.registers.set_register(register, value);
    }

    #[inline]
    pub fn update_register(&mut self, register: MmuRegister, update: impl FnOnce(u64) -> u64) {
        self.set_register(register, update(self.register(register)));
    }

    #[inline]
    pub fn status(&self) -> Status {
        Status(self.register(MmuRegister::Status))
    }

    #[inline]
    pub fn add_cycles(&mut self, cycles: u64) {
        self.update_register(MmuRegister::Count, |count| count + cycles);
    }

    #[inline]
    pub fn begin_instruction(&mut self) {
        if self.clock.begin_instruction() {
            self.set_register(MmuRegister::Random, RANDOM_RESET);
            return;
        }

        let wired = self.effective_wired();
        self.update_register(MmuRegister::Random, |random| {
            if random == wired {
                RANDOM_RESET
            } else {
                random.wrapping_sub(1) & RANDOM_MASK
            }
        });
    }

    fn effective_wired(&self) -> u64 {
        self.clock
            .effective_wired(self.register(MmuRegister::Wired))
    }

    pub(super) fn defer_wired_reset(&mut self) {
        let wired = self.effective_wired();
        self.clock.defer_wired_reset(wired);
    }

    pub(super) fn count_cycles(&self) -> u64 {
        self.clock.count_cycles(self.register(MmuRegister::Count))
    }

    pub(super) fn count_pause_remaining(&self) -> u64 {
        self.clock
            .count_pause_remaining(self.register(MmuRegister::Count))
    }

    pub(super) fn write_masked(&mut self, index: usize, data: u64) {
        self.registers.write_masked(index, data);
    }

    /// Five-bit guest selector includes undocumented software-visible slots.
    pub(super) fn control_register(&self, index: u32) -> u64 {
        self.registers.control_register(index)
    }
}
