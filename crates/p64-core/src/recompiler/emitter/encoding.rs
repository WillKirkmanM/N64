mod condition;
mod fields;
pub(super) use condition::Condition;
pub(super) use fields::{ArithmeticOperation, DISPLACEMENT_MODE, GPR_BASE_REGISTER, REGISTER_MODE};

pub(super) const GUEST_REGISTER_BYTES: usize = std::mem::size_of::<u64>();
pub(super) const GUEST_REGISTER_COUNT: usize = 32;
pub(super) const GUEST_INSTRUCTION_BYTES: usize = std::mem::size_of::<u32>();
