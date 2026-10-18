use super::super::{instruction::se32, Cpu};

#[cfg(test)]
#[path = "result/tests.rs"]
mod tests;

pub(super) struct HiLo {
    hi: u64,
    lo: u64,
}

impl HiLo {
    pub(super) fn product(value: u128) -> Self {
        Self {
            hi: (value >> 64) as u64,
            lo: value as u64,
        }
    }

    pub(super) fn word_product(value: u64) -> Self {
        Self {
            hi: se32((value >> 32) as i32),
            lo: se32(value as i32),
        }
    }

    pub(super) fn signed_division(dividend: i64, divisor: i64) -> Self {
        if divisor == 0 {
            Self {
                hi: dividend as u64,
                lo: if dividend < 0 { 1 } else { u64::MAX },
            }
        } else {
            Self {
                hi: dividend.wrapping_rem(divisor) as u64,
                lo: dividend.wrapping_div(divisor) as u64,
            }
        }
    }

    pub(super) fn unsigned_division(dividend: u64, divisor: u64) -> Self {
        match dividend.checked_div(divisor) {
            Some(quotient) => Self {
                hi: dividend % divisor,
                lo: quotient,
            },
            None => Self {
                hi: dividend,
                lo: u64::MAX,
            },
        }
    }

    pub(super) fn sign_extend_words(self) -> Self {
        Self {
            hi: se32(self.hi as i32),
            lo: se32(self.lo as i32),
        }
    }

    pub(super) fn write(self, cpu: &mut Cpu) {
        cpu.registers.hi = self.hi;
        cpu.registers.lo = self.lo;
    }
}
