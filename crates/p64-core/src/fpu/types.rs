mod control;
mod instructions;
mod registers;
mod timing;

pub use control::FpuControl;
pub use instructions::FpuInstructionTables;
pub use registers::FpuRegisters;
pub use timing::FpuTiming;

#[derive(Default)]
pub struct Fpu {
    pub registers: FpuRegisters,
    pub control: FpuControl,
    pub timing: FpuTiming,
    pub instructions: FpuInstructionTables,
}

impl Fpu {
    pub fn set_single(&mut self, index: usize, value: f32, clear_high: bool, wide: bool) {
        self.registers.set_single(index, value, clear_high, wide);
    }

    pub fn single(&self, index: usize, wide: bool) -> f32 {
        self.registers.single(index, wide)
    }

    pub fn set_double(&mut self, index: usize, value: f64, wide: bool) {
        self.registers.set_double(index, value, wide);
    }

    pub fn double(&self, index: usize, wide: bool) -> f64 {
        self.registers.double(index, wide)
    }

    pub fn set_register_mode(&mut self, wide: bool) {
        self.registers.set_register_mode(wide);
    }

    pub fn begin_instruction(&mut self) {
        self.timing.begin_instruction();
    }

    pub fn exception_pending(&self) -> bool {
        self.control.exception_pending()
    }

    pub fn record_exception_flags(&mut self, flags: u32) -> bool {
        self.control.record_exception_flags(flags)
    }
}
