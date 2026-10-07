#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RoundingMode {
    Nearest,
    TowardZero,
    TowardPositive,
    TowardNegative,
}

impl RoundingMode {
    pub(crate) const fn from_encoding(encoding: u32) -> Option<Self> {
        match encoding {
            0 => Some(Self::Nearest),
            1 => Some(Self::TowardZero),
            2 => Some(Self::TowardPositive),
            3 => Some(Self::TowardNegative),
            _ => None,
        }
    }

    pub(crate) fn flushes_outward(self, negative: bool) -> bool {
        matches!(
            (self, negative),
            (Self::TowardPositive, false) | (Self::TowardNegative, true)
        )
    }
}
