#[derive(Clone, Copy)]
pub(super) enum TrapCondition {
    SignedGreaterOrEqual,
    UnsignedGreaterOrEqual,
    SignedLess,
    UnsignedLess,
    Equal,
    NotEqual,
}

impl TrapCondition {
    pub(super) fn matches(self, source: u64, target: u64) -> bool {
        match self {
            Self::SignedGreaterOrEqual => source as i64 >= target as i64,
            Self::UnsignedGreaterOrEqual => source >= target,
            Self::SignedLess => (source as i64) < target as i64,
            Self::UnsignedLess => source < target,
            Self::Equal => source == target,
            Self::NotEqual => source != target,
        }
    }
}
