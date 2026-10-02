use crate::N64;
use std::sync::OnceLock;

static WATCH_ADDRESS: OnceLock<Option<u64>> = OnceLock::new();

pub(crate) fn write_watched(address: u64) -> bool {
    let Some(watched) = *WATCH_ADDRESS.get_or_init(|| {
        std::env::var("N64_WATCH_WRITE")
            .ok()
            .and_then(|value| u64::from_str_radix(value.trim_start_matches("0x"), 16).ok())
    }) else {
        return false;
    };

    (watched..watched.saturating_add(8)).contains(&address)
}

pub(crate) fn trace_write(n64: &N64, source: &str, address: u64, value: u32, mask: u32) {
    if !write_watched(address) {
        return;
    }
    let cpu = n64.cpu();
    let instruction = n64.memory.caches.icache[crate::cache::ICache::line_index(cpu.execution.pc)]
        .words[crate::cache::ICache::word_index(cpu.execution.pc_phys)];
    let rsp = n64.rsp();
    let rsp_pc = rsp.registers.regs2[crate::rsp::SpReg2::Pc as usize];
    let rsp_word = rsp
        .instruction
        .instructions
        .get((rsp_pc / 4) as usize)
        .map(|instruction| instruction.opcode);
    eprintln!(
        "[WRITE] source={source} address={address:#x} value={value:#010x} mask={mask:#010x} pc={:#018x} opcode={instruction:#010x} ea={:#018x} gpr={:x?}",
        cpu.execution.pc, cpu.execution.data_address, std::array::from_fn::<_, 32, _>(|index| cpu.gpr(index))
    );
    if source.starts_with("rsp") {
        eprintln!(
            "[WRITE-SP] pc={rsp_pc:#x} opcode={rsp_word:x?} regs={:x?} gpr={:x?}",
            rsp.registers.regs.regs, rsp.registers.gpr
        );
    }
}
