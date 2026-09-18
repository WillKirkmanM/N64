mod condition;
mod fields;
pub(super) use condition::Condition;
pub(super) use fields::{
    modrm, wide_rex, ArithmeticOperation, DISPLACEMENT_MODE, GPR_BASE_REGISTER, REGISTER_MODE,
};

pub(super) const GUEST_REGISTER_BYTES: usize = std::mem::size_of::<u64>();
pub(super) const GUEST_REGISTER_COUNT: usize = 32;
pub(super) const GUEST_INSTRUCTION_BYTES: usize = std::mem::size_of::<u32>();

pub(super) fn displacement(code: &mut Vec<u8>, value: usize) {
    code.extend_from_slice(&(value as u32).to_le_bytes());
}

/// RDX holds the guest array. Load into RAX, or R8 for the right-hand operand.
pub(super) fn load_operand(code: &mut Vec<u8>, guest: usize, rhs: bool) {
    if guest == 0 {
        code.extend_from_slice(if rhs {
            &[0x45, 0x31, 0xc0]
        } else {
            &[0x31, 0xc0]
        });
    } else {
        code.extend_from_slice(if rhs {
            &[0x4c, 0x8b, 0x82]
        } else {
            &[0x48, 0x8b, 0x82]
        });
        displacement(code, guest * GUEST_REGISTER_BYTES);
    }
}

pub(super) fn sign_extend_word(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x48, 0x63, 0xc0]);
}

pub(super) fn set_condition(code: &mut Vec<u8>, condition: Condition) {
    code.extend_from_slice(&[0x0f, condition.set_opcode(), 0xc0, 0x0f, 0xb6, 0xc0]);
}
