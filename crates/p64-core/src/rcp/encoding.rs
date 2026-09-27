#[derive(Clone, Copy)]
pub(super) struct FloatEncoding {
    pub(super) exponent: u64,
    pub(super) fraction: u64,
    pub(super) sign: u64,
    pub(super) quiet_nan: u64,
    pub(super) canonical_nan: u64,
    pub(super) minimum_normal: u64,
    exponent_bias: u64,
    fraction_shift: u32,
}

impl FloatEncoding {
    const SINGLE: Self = Self {
        exponent: 0x7f80_0000,
        fraction: 0x007f_ffff,
        sign: 1 << 31,
        quiet_nan: 0x0040_0000,
        canonical_nan: 0x7fbf_ffff,
        minimum_normal: 0x0080_0000,
        exponent_bias: 127,
        fraction_shift: 23,
    };
    const DOUBLE: Self = Self {
        exponent: 0x7ff0_0000_0000_0000,
        fraction: 0x000f_ffff_ffff_ffff,
        sign: 1 << 63,
        quiet_nan: 0x0008_0000_0000_0000,
        canonical_nan: 0x7ff7_ffff_ffff_ffff,
        minimum_normal: 0x0010_0000_0000_0000,
        exponent_bias: 1023,
        fraction_shift: 52,
    };

    pub(super) const fn for_precision(double: bool) -> Self {
        if double {
            Self::DOUBLE
        } else {
            Self::SINGLE
        }
    }

    pub(super) fn integer_limit(self, power: u64) -> u64 {
        (self.exponent_bias + power) << self.fraction_shift
    }

    pub(super) fn is_subnormal(self, bits: u64) -> bool {
        bits & self.exponent == 0 && bits & self.fraction != 0
    }

    pub(super) fn is_nan(self, bits: u64) -> bool {
        bits & self.exponent == self.exponent && bits & self.fraction != 0
    }

    pub(super) fn unsupported_arithmetic_operand(self, bits: u64) -> bool {
        self.is_subnormal(bits) || (self.is_nan(bits) && !self.is_quiet_nan(bits))
    }

    pub(super) fn is_quiet_nan(self, bits: u64) -> bool {
        self.is_nan(bits) && bits & self.quiet_nan != 0
    }
}
