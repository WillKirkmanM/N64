use super::super::{instruction::Instruction, Cpu};
use crate::{exceptions, mmu::add_cycles, N64};

const OVERFLOW_CYCLES: u64 = 2;

impl Cpu {
    pub(super) fn apply_registers(
        &mut self,
        instruction: Instruction,
        operation: impl FnOnce(u64, u64) -> u64,
    ) {
        let result = operation(self.gpr(instruction.rs), self.gpr(instruction.rt));
        self.set_gpr(instruction.rd, result);
    }

    pub(super) fn apply_immediate(
        &mut self,
        instruction: Instruction,
        operation: impl FnOnce(u64, i64) -> u64,
    ) {
        let result = operation(self.gpr(instruction.rs), instruction.immediate_signed());
        self.set_gpr(instruction.rt, result);
    }

    pub(super) fn apply_unsigned_immediate(
        &mut self,
        instruction: Instruction,
        operation: impl FnOnce(u64, u64) -> u64,
    ) {
        let result = operation(self.gpr(instruction.rs), instruction.immediate_unsigned());
        self.set_gpr(instruction.rt, result);
    }

    pub(super) fn apply_immediate_shift(
        &mut self,
        instruction: Instruction,
        offset: u32,
        operation: impl FnOnce(u64, u32) -> u64,
    ) {
        let result = operation(
            self.gpr(instruction.rt),
            instruction.shift_amount() + offset,
        );
        self.set_gpr(instruction.rd, result);
    }

    pub(super) fn apply_register_shift(
        &mut self,
        instruction: Instruction,
        mask: u32,
        operation: impl FnOnce(u64, u32) -> u64,
    ) {
        let amount = self.gpr(instruction.rs) as u32 & mask;
        let result = operation(self.gpr(instruction.rt), amount);
        self.set_gpr(instruction.rd, result);
    }
}

pub(super) fn write_checked(n64: &mut N64, destination: usize, result: Option<u64>) {
    match result {
        Some(value) => n64.cpu_mut().set_gpr(destination, value),
        None => {
            add_cycles(n64, OVERFLOW_CYCLES);
            exceptions::overflow_exception(n64);
        }
    }
}

pub(super) fn checked_registers(
    n64: &mut N64,
    opcode: u32,
    operation: impl FnOnce(u64, u64) -> Option<u64>,
) {
    let instruction = Instruction::decode(opcode);
    let cpu = n64.cpu();
    let result = operation(cpu.gpr(instruction.rs), cpu.gpr(instruction.rt));
    write_checked(n64, instruction.rd, result);
}

pub(super) fn checked_immediate(
    n64: &mut N64,
    opcode: u32,
    operation: impl FnOnce(u64, i64) -> Option<u64>,
) {
    let instruction = Instruction::decode(opcode);
    let result = operation(
        n64.cpu().gpr(instruction.rs),
        instruction.immediate_signed(),
    );
    write_checked(n64, instruction.rt, result);
}

pub(super) fn registers(n64: &mut N64, opcode: u32, operation: impl FnOnce(u64, u64) -> u64) {
    n64.cpu_mut()
        .apply_registers(Instruction::decode(opcode), operation);
}

pub(super) fn immediate(n64: &mut N64, opcode: u32, operation: impl FnOnce(u64, i64) -> u64) {
    n64.cpu_mut()
        .apply_immediate(Instruction::decode(opcode), operation);
}

pub(super) fn unsigned_immediate(
    n64: &mut N64,
    opcode: u32,
    operation: impl FnOnce(u64, u64) -> u64,
) {
    n64.cpu_mut()
        .apply_unsigned_immediate(Instruction::decode(opcode), operation);
}

pub(super) fn immediate_shift(
    n64: &mut N64,
    opcode: u32,
    offset: u32,
    operation: impl FnOnce(u64, u32) -> u64,
) {
    n64.cpu_mut()
        .apply_immediate_shift(Instruction::decode(opcode), offset, operation);
}

pub(super) fn register_shift(
    n64: &mut N64,
    opcode: u32,
    mask: u32,
    operation: impl FnOnce(u64, u32) -> u64,
) {
    n64.cpu_mut()
        .apply_register_shift(Instruction::decode(opcode), mask, operation);
}
