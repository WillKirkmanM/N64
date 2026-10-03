use super::{ADDRESS_MASK, CLOCK_DENOMINATOR, CLOCK_MASK, CLOCK_NUMERATOR};

#[path = "processing.rs"]
mod processing;
#[path = "registers.rs"]
mod registers;
pub use processing::RdpProcessing;
pub use registers::RdpRegisters;

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum DpcReg {
    Start = 0,
    End = 1,
    Current = 2,
    Status = 3,
    Clock = 4,
    BufBusy = 5,
    PipeBusy = 6,
    Tmem = 7,
    RegsCount = 8,
}

#[repr(u32)]
pub enum DpsReg {
    RegsCount = 4,
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub(super) struct DpcStatus: u32 {
        const XBUS_DMEM_DMA = 1 << 0;
        const FREEZE = 1 << 1;
        const FLUSH = 1 << 2;
        const START_GCLK = 1 << 3;
        const TMEM_BUSY = 1 << 4;
        const PIPE_BUSY = 1 << 5;
        const CMD_BUSY = 1 << 6;
        const CBUF_READY = 1 << 7;
        const START_VALID = 1 << 10;
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub(super) struct DpcStatusWrite: u32 {
        const CLR_XBUS_DMEM_DMA = 1 << 0;
        const SET_XBUS_DMEM_DMA = 1 << 1;
        const CLR_FREEZE = 1 << 2;
        const SET_FREEZE = 1 << 3;
        const CLR_FLUSH = 1 << 4;
        const SET_FLUSH = 1 << 5;
        const CLR_TMEM_CTR = 1 << 6;
        const CLR_PIPE_CTR = 1 << 7;
        const CLR_CMD_CTR = 1 << 8;
        const CLR_CLOCK_CTR = 1 << 9;
    }
}

#[derive(Default)]
pub struct Rdp {
    pub registers: RdpRegisters,
    pub processing: RdpProcessing,
}

#[derive(Clone, Copy)]
pub(super) enum CommandAction {
    None,
    Submit,
    ResumeFrozen,
}

impl DpcReg {
    fn decode(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Start),
            1 => Some(Self::End),
            2 => Some(Self::Current),
            3 => Some(Self::Status),
            4 => Some(Self::Clock),
            5 => Some(Self::BufBusy),
            6 => Some(Self::PipeBusy),
            7 => Some(Self::Tmem),
            _ => None,
        }
    }
}

impl Rdp {
    pub(super) fn register(&self, register: DpcReg) -> u32 {
        self.registers.regs_dpc[register as usize]
    }

    fn set_register(&mut self, register: DpcReg, value: u32) {
        self.registers.regs_dpc[register as usize] = value;
    }

    fn update_register(&mut self, register: DpcReg, update: impl FnOnce(u32) -> u32) {
        self.set_register(register, update(self.register(register)));
    }

    fn masked_register(&mut self, register: DpcReg, value: u32, mask: u32) {
        crate::memory::masked_write_32(
            &mut self.registers.regs_dpc[register as usize],
            value,
            mask,
        );
    }

    fn has_status(&self, flag: DpcStatus) -> bool {
        self.register(DpcReg::Status) & flag.bits() != 0
    }

    pub(super) fn clock(&self, cpu_count: u64) -> u32 {
        let elapsed = i128::from(cpu_count) + self.processing.clock_offset;

        ((elapsed * CLOCK_NUMERATOR / CLOCK_DENOMINATOR) as u32) & CLOCK_MASK
    }

    pub(super) fn rebase_clock(&mut self, old_count: u64, new_count: u64) {
        self.processing.clock_offset += i128::from(old_count) - i128::from(new_count);
    }

    pub(super) fn write_command(
        &mut self,
        index: usize,
        value: u32,
        mask: u32,
        cpu_count: u64,
    ) -> CommandAction {
        use DpcReg::*;

        match DpcReg::decode(index) {
            Some(Current | Clock | BufBusy | PipeBusy | Tmem) => CommandAction::None,
            Some(Status) => self.write_status(value & mask, cpu_count),
            Some(Start) => {
                if !self.has_status(DpcStatus::START_VALID) {
                    self.masked_register(Start, value & ADDRESS_MASK, mask);
                }

                self.update_register(Status, |status| status | DpcStatus::START_VALID.bits());

                CommandAction::None
            }
            Some(End) => {
                self.masked_register(End, value & ADDRESS_MASK, mask);
                if self.has_status(DpcStatus::START_VALID) {
                    self.set_register(Current, self.register(Start));
                    self.update_register(Status, |status| status & !DpcStatus::START_VALID.bits());
                }

                if self.has_status(DpcStatus::FREEZE) {
                    self.processing.wait_frozen = true;

                    CommandAction::None
                } else {
                    CommandAction::Submit
                }
            }
            None | Some(RegsCount) => {
                crate::memory::masked_write_32(&mut self.registers.regs_dpc[index], value, mask);

                CommandAction::None
            }
        }
    }

    fn clear_counter(
        &mut self,
        status: &mut DpcStatus,
        command: DpcStatusWrite,
        clear: DpcStatusWrite,
        busy: DpcStatus,
        register: DpcReg,
    ) {
        if command.contains(clear) {
            status.remove(busy);
            self.set_register(register, 0);
        }
    }

    fn write_status(&mut self, value: u32, cpu_count: u64) -> CommandAction {
        use DpcStatusWrite as Write;
        let command = Write::from_bits_truncate(value);
        let mut raw_status = DpcStatus::from_bits_truncate(self.register(DpcReg::Status)).bits();

        for control in super::STATUS_COMMANDS {
            control.apply(&mut raw_status, value);
        }

        let mut status = DpcStatus::from_bits_truncate(raw_status);
        self.clear_counter(
            &mut status,
            command,
            Write::CLR_TMEM_CTR,
            DpcStatus::TMEM_BUSY,
            DpcReg::Tmem,
        );
        self.clear_counter(
            &mut status,
            command,
            Write::CLR_PIPE_CTR,
            DpcStatus::PIPE_BUSY,
            DpcReg::PipeBusy,
        );
        self.clear_counter(
            &mut status,
            command,
            Write::CLR_CMD_CTR,
            DpcStatus::CMD_BUSY,
            DpcReg::BufBusy,
        );
        if command.contains(Write::CLR_CLOCK_CTR) {
            self.set_register(DpcReg::Clock, 0);
            self.processing.clock_offset = -i128::from(cpu_count);
        }

        self.set_register(DpcReg::Status, status.bits());

        if command.contains(Write::CLR_FREEZE)
            && !status.contains(DpcStatus::FREEZE)
            && self.processing.wait_frozen
        {
            CommandAction::ResumeFrozen
        } else {
            CommandAction::None
        }
    }

    pub(super) fn finish_commands(&mut self) {
        self.update_register(DpcReg::Status, |status| {
            status & !(DpcStatus::START_GCLK | DpcStatus::PIPE_BUSY | DpcStatus::CMD_BUSY).bits()
        });
    }
}
