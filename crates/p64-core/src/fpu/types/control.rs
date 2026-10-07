use super::super::{
    RoundingMode, FCR31_CAUSE_MASK, FCR31_CAUSE_UNIMP_BIT, FCR31_CMP_BIT, FCR31_ENABLE_MASK,
    FCR31_FLUSH_ENABLE_MASK, FCR31_FS_BIT, FCR31_ROUNDING_MASK,
};

const FCR0_ID: u32 = 0x0a00;
const FLAG_SHIFT: u32 = 2;
const ENABLE_SHIFT: u32 = 7;
const CAUSE_SHIFT: u32 = 12;
const CAUSE_TO_ENABLE_SHIFT: u32 = CAUSE_SHIFT - ENABLE_SHIFT;

pub struct FpuControl {
    pub fcr0: u32,
    pub fcr31: u32,
}

impl Default for FpuControl {
    fn default() -> Self {
        Self {
            fcr0: FCR0_ID,
            fcr31: 0,
        }
    }
}

impl FpuControl {
    pub(crate) fn clear_causes(&mut self) {
        self.fcr31 &= !FCR31_CAUSE_MASK;
    }

    pub(crate) fn mark_unimplemented(&mut self) {
        self.clear_causes();
        self.fcr31 |= FCR31_CAUSE_UNIMP_BIT;
    }

    pub(crate) fn comparison(&self) -> bool {
        self.fcr31 & FCR31_CMP_BIT != 0
    }

    pub(crate) fn set_comparison(&mut self, condition: bool) {
        self.fcr31 = crate::bits::replace_bits(
            self.fcr31,
            if condition { FCR31_CMP_BIT } else { 0 },
            FCR31_CMP_BIT,
        );
    }

    pub(crate) fn rounding_mode(&self) -> RoundingMode {
        RoundingMode::from_encoding(self.fcr31 & FCR31_ROUNDING_MASK)
            .expect("masked FCSR rounding mode")
    }

    pub(crate) fn flush_exceptions_enabled(&self) -> bool {
        self.fcr31 & FCR31_FLUSH_ENABLE_MASK != 0
    }

    pub(crate) fn can_flush_underflow(&self) -> bool {
        self.fcr31 & FCR31_FS_BIT != 0 && !self.flush_exceptions_enabled()
    }

    pub fn exception_pending(&self) -> bool {
        self.fcr31 & FCR31_CAUSE_UNIMP_BIT != 0
            || ((self.fcr31 & FCR31_CAUSE_MASK) >> CAUSE_TO_ENABLE_SHIFT)
                & (self.fcr31 & FCR31_ENABLE_MASK)
                != 0
    }

    pub fn record_exception_flags(&mut self, flags: u32) -> bool {
        self.fcr31 |= flags << CAUSE_SHIFT;

        if (flags << ENABLE_SHIFT) & self.fcr31 != 0 {
            false
        } else {
            self.fcr31 |= flags << FLAG_SHIFT;

            true
        }
    }
}
