use super::*;

pub fn decode_opcode(n64: &N64, opcode: u32) -> fn(&mut N64, u32) {
    match opcode >> 26 {
        0 => n64.cpu.instructions.special_instrs[(opcode & 0x3F) as usize],
        1 => n64.cpu.instructions.regimm_instrs[((opcode >> 16) & 0x1F) as usize],
        16..=18 => [
            &n64.cpu.mmu.instrs,
            &n64.cpu.fpu.instructions.instrs,
            &n64.cpu.rcp.instrs,
        ][(opcode >> 26) as usize - 16][((opcode >> 21) & 0x1F) as usize],
        _ => n64.cpu.instructions.instrs[(opcode >> 26) as usize],
    }
}

pub fn start_cpu(n64: &mut N64) {
    n64.cpu.execution.clock_rate = 93750000;

    info!("Initializing CPU instructions...");
    n64.cpu.instructions.instrs = {
        let reserved_fn: InstrFn = mmu::reserved as fn(&mut N64, u32);
        let mut arr = [reserved_fn; 64];
        for &(i, f) in MAIN_OPS.iter() {
            arr[i] = f;
        }
        arr
    };
    info!("CPU instructions initialized");

    info!("Initializing CPU special instructions...");
    n64.cpu.instructions.special_instrs = {
        let reserved_fn: InstrFn = mmu::reserved as fn(&mut N64, u32);
        let mut arr = [reserved_fn; 64];
        for &(i, f) in SPECIAL_OPS.iter() {
            arr[i] = f;
        }
        arr
    };
    info!("CPU special instructions initialized");

    info!("Initializing CPU regimm instructions...");
    n64.cpu.instructions.regimm_instrs = {
        let reserved_fn: InstrFn = mmu::reserved as fn(&mut N64, u32);
        let mut arr = [reserved_fn; 32];
        for &(i, f) in REGIMM_OPS.iter() {
            arr[i] = f;
        }
        arr
    };
    info!("CPU regimm instructions initialized");

    write_mmu_registers(n64);
}
