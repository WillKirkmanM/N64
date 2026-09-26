use super::{MmuCause, MmuStatus};

const INTERRUPT_STATE_MASK: u64 =
    MmuStatus::IE as u64 | MmuStatus::EXL as u64 | MmuStatus::ERL as u64;

const PRIVILEGE_MASK: u64 = 3;
const PRIVILEGE_SHIFT: u32 = 3;
const EXTENDED_KERNEL_SHIFT: u32 = 7;
const REVERSE_ENDIAN: u64 = 1 << 25;
const COPROCESSOR_ZERO_ENABLED: u64 = 1 << 28;

#[derive(Copy, Clone, Debug)]
pub struct Status(pub(super) u64);

impl Status {
    #[inline]
    pub fn interrupt_pending(self, cause: u64) -> bool {
        self.0 & INTERRUPT_STATE_MASK == MmuStatus::IE as u64
            && self.0 & cause & MmuCause::IPMask as u64 != 0
    }

    #[inline]
    pub fn boot_exception_vectors(self) -> bool {
        self.0 & MmuStatus::BEV as u64 != 0
    }

    #[inline]
    pub fn exception_level(self) -> bool {
        self.0 & MmuStatus::EXL as u64 != 0
    }

    #[inline]
    pub fn error_level(self) -> bool {
        self.0 & MmuStatus::ERL as u64 != 0
    }

    /// Exception and error handlers always execute with kernel privileges.
    #[inline]
    pub fn mode(self) -> u64 {
        if self.exception_level() || self.error_level() {
            0
        } else {
            (self.0 >> PRIVILEGE_SHIFT) & PRIVILEGE_MASK
        }
    }

    #[inline]
    pub fn extended_addressing(self) -> bool {
        self.0 & (1 << (EXTENDED_KERNEL_SHIFT - self.mode().min(2) as u32)) != 0
    }

    #[inline]
    pub fn reverse_endian(self) -> bool {
        self.0 & REVERSE_ENDIAN != 0
    }

    #[inline]
    pub fn wide_instructions_allowed(self) -> bool {
        self.mode() == 0 || self.extended_addressing()
    }

    #[inline]
    pub fn cp0_usable(self) -> bool {
        self.0 & (COPROCESSOR_ZERO_ENABLED | MmuStatus::EXL as u64 | MmuStatus::ERL as u64) != 0
            || self.0 & (PRIVILEGE_MASK << PRIVILEGE_SHIFT) == 0
    }
}
