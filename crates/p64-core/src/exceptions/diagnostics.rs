use crate::{
    cache::ICache,
    mmu::{MmuCause, MmuRegister},
    rsp::SpReg2,
    N64,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    OnceLock,
};

const TRACE_LIMIT: usize = 1024;

static TRACE_ENABLED: OnceLock<bool> = OnceLock::new();
static TRACES: AtomicUsize = AtomicUsize::new(0);

fn tracing() -> bool {
    *TRACE_ENABLED.get_or_init(|| std::env::var_os("N64_TRACE_EXCEPTIONS").is_some())
        && TRACES.fetch_add(1, Ordering::Relaxed) < TRACE_LIMIT
}

pub(super) fn trace_reserved(n64: &N64, opcode: u32) {
    if tracing() {
        eprintln!(
            "[RESERVED] pc={:#018x} opcode={opcode:#010x}",
            n64.cpu().execution.pc
        );
    }
}

pub(super) fn trace(n64: &N64, cause: u64) {
    if cause & MmuCause::ExccodeMask as u64 == 0 || !tracing() {
        return;
    }
    let pc = n64.cpu().execution.pc;
    let physical = n64.cpu().execution.pc_phys;
    let cache_word =
        n64.memory.caches.icache[ICache::line_index(pc)].words[ICache::word_index(physical)];
    let rsp = n64.rsp();
    let rsp_pc = rsp.registers.regs2[SpReg2::Pc as usize];
    let rsp_word = rsp
        .instruction
        .instructions
        .get((rsp_pc / 4) as usize)
        .map(|instruction| instruction.opcode);
    eprintln!("[EXCEPTION] cause={cause:#x} pc={pc:#018x} physical={physical:#x} cached_word={cache_word:#010x} status={:#x} bad_vaddr={:#018x} rsp_running={} rsp_pc={rsp_pc:#x} rsp_word={rsp_word:x?}", n64.cp0_register(MmuRegister::Status), n64.cp0_register(MmuRegister::BadVAddr), rsp.processor.running);
}
