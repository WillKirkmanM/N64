use p64_core::N64;

#[derive(Debug, PartialEq)]
pub(super) struct Snapshot {
    gpr: [u64; 32],
    cp0: Vec<u64>,
    hi: u64,
    lo: u64,
    pc: u64,
    instruction_clock: u64,
    pending_load: u8,
    count_write_origin: Option<u64>,
    pending_wired_reset: Option<(u64, u64)>,
    fpu_writes: (Option<u8>, Option<u8>),
}

pub(super) fn snapshot(n64: &N64) -> Snapshot {
    Snapshot {
        gpr: n64.cpu.registers.gpr,
        cp0: n64.cpu.mmu.registers.regs.regs.clone(),
        hi: n64.cpu.registers.hi,
        lo: n64.cpu.registers.lo,
        pc: n64.cpu.execution.pc,
        instruction_clock: n64.cpu.mmu.clock.instruction_clock,
        pending_load: n64.cpu.execution.pending_gpr_load,
        count_write_origin: n64.cpu.mmu.clock.count_write_origin,
        pending_wired_reset: n64.cpu.mmu.clock.pending_wired_reset,
        fpu_writes: (
            n64.cpu.fpu.timing.previous_write,
            n64.cpu.fpu.timing.pending_write,
        ),
    }
}
