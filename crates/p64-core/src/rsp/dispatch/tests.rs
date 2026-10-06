use super::*;
use crate::{mmu::MmuRegister, N64};

fn cpu_snapshot(n64: &N64) -> [u64; 5] {
    [
        n64.cpu().execution.pc,
        n64.cp0_register(MmuRegister::Cause),
        n64.cp0_register(MmuRegister::EPC),
        n64.cp0_register(MmuRegister::Status),
        n64.cp0_register(MmuRegister::Count),
    ]
}

mod operands;

mod vector_ops;
