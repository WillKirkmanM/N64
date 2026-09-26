use crate::cpu::{BranchState, State};

#[derive(Clone, Copy)]
pub(super) enum BranchCondition {
    Equal,
    NotEqual,
    LessOrEqualZero,
    GreaterZero,
    Negative,
    Nonnegative,
}

impl BranchCondition {
    pub(super) fn matches(self, source: u64, target: u64) -> bool {
        match self {
            Self::Equal => source == target,
            Self::NotEqual => source != target,
            Self::LessOrEqualZero => source as i64 <= 0,
            Self::GreaterZero => source as i64 > 0,
            Self::Negative => (source as i64) < 0,
            Self::Nonnegative => source as i64 >= 0,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum DelaySlotPolicy {
    Execute,
    AnnulIfUntaken,
}

impl BranchState {
    pub(super) fn take(&mut self, target: u64) {
        self.pc = target;
        self.state = State::Take;
    }

    pub(super) fn decline(&mut self, delay_slot: DelaySlotPolicy) {
        self.state = match delay_slot {
            DelaySlotPolicy::Execute => State::NotTaken,
            DelaySlotPolicy::AnnulIfUntaken => State::Discard,
        };
    }
}
