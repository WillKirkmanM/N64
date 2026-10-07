use super::{encoding::FloatEncoding, FP_INEXACT, FP_UNDERFLOW};
use crate::fpu::RoundingMode;

pub(super) struct FloatResult {
    pub bits: u64,
    pub flags: u32,
    encoding: FloatEncoding,
}

impl FloatResult {
    pub fn new(bits: u64, flags: u32, encoding: FloatEncoding) -> Self {
        Self {
            bits: if encoding.is_nan(bits) {
                encoding.canonical_nan
            } else {
                bits
            },
            flags,
            encoding,
        }
    }

    pub fn needs_flush(&self) -> bool {
        self.encoding.is_subnormal(self.bits) || self.flags & FP_UNDERFLOW != 0
    }

    pub fn flush(&mut self, rounding: RoundingMode) {
        let outward = rounding.flushes_outward(self.bits & self.encoding.sign != 0);
        self.bits = (self.bits & self.encoding.sign)
            | if outward {
                self.encoding.minimum_normal
            } else {
                0
            };
        self.flags = FP_UNDERFLOW | FP_INEXACT;
    }
}
