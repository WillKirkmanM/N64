use super::super::encoding::{
    modrm, wide_rex, ArithmeticOperation, DISPLACEMENT_MODE, GPR_BASE_REGISTER, REGISTER_MODE,
};
use super::{set_condition, Condition, GUEST_REGISTER_BYTES, GUEST_REGISTER_COUNT};

pub(super) fn mov_register(code: &mut Vec<u8>, source: u8, destination: u8) {
    if source == destination {
        return;
    }

    code.extend_from_slice(&[
        wide_rex(source, destination),
        0x89,
        modrm(REGISTER_MODE, source, destination),
    ]);
}

pub(super) fn load_memory(code: &mut Vec<u8>, guest: usize, host: u8) {
    code.extend_from_slice(&[
        wide_rex(host, GPR_BASE_REGISTER),
        0x8b,
        modrm(DISPLACEMENT_MODE, host, GPR_BASE_REGISTER),
    ]);
    code.extend_from_slice(&((guest * GUEST_REGISTER_BYTES) as u32).to_le_bytes());
}

pub(super) fn store_memory(code: &mut Vec<u8>, guest: usize, host: u8) {
    code.extend_from_slice(&[
        wide_rex(host, GPR_BASE_REGISTER),
        0x89,
        modrm(DISPLACEMENT_MODE, host, GPR_BASE_REGISTER),
    ]);
    code.extend_from_slice(&((guest * GUEST_REGISTER_BYTES) as u32).to_le_bytes());
}

pub(super) fn load_guest(
    code: &mut Vec<u8>,
    guest: usize,
    host: u8,
    mapping: &[Option<u8>; GUEST_REGISTER_COUNT],
) {
    if guest == 0 {
        code.extend_from_slice(&[0x31, modrm(REGISTER_MODE, host, host)]);
    } else if let Some(source) = mapping[guest] {
        mov_register(code, source, host);
    } else {
        load_memory(code, guest, host);
    }
}

pub(super) fn compare_result(code: &mut Vec<u8>, unsigned: bool) {
    set_condition(
        code,
        if unsigned {
            Condition::Below
        } else {
            Condition::Less
        },
    );
}

pub(super) fn arithmetic_immediate(
    code: &mut Vec<u8>,
    operation: ArithmeticOperation,
    immediate: i32,
    wide: bool,
) {
    if wide {
        code.push(0x48);
    }

    if let Ok(byte) = i8::try_from(immediate) {
        code.extend_from_slice(&[0x83, modrm(REGISTER_MODE, operation as u8, 0), byte as u8]);
    } else {
        code.extend_from_slice(&[0x81, modrm(REGISTER_MODE, operation as u8, 0)]);
        code.extend_from_slice(&immediate.to_le_bytes());
    }
}
