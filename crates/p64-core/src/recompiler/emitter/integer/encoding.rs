use super::super::encoding::{
    modrm, wide_rex, ArithmeticOperation, DISPLACEMENT_MODE, GPR_BASE_REGISTER, REGISTER_MODE,
};
use super::{set_condition, Condition, GUEST_REGISTER_BYTES, GUEST_REGISTER_COUNT};
