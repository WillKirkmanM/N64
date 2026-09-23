use super::*;

pub const MMU_INSTRUCTIONS: [MmuInstrFn; 32] = {
    let mut instrs: [MmuInstrFn; 32] = [mmu::reserved; 32];

    instrs[MmuInstruction::MFC0 as usize] = mmu::mfc0 as MmuInstrFn;
    instrs[MmuInstruction::DMFC0 as usize] = mmu::dmfc0 as MmuInstrFn;
    instrs[MmuInstruction::MTC0 as usize] = mmu::mtc0 as MmuInstrFn;
    instrs[MmuInstruction::DMTC0 as usize] = mmu::dmtc0 as MmuInstrFn;
    instrs[MmuInstruction::EXECUTECP0 as usize] = mmu::execute_cp0 as MmuInstrFn;

    instrs
};

pub const MMU_SECONDARY_INSTRUCTIONS: [MmuInstrFn; 32] = {
    let mut instrs: [MmuInstrFn; 32] = [mmu::reserved; 32];

    instrs[MmuSecondaryInstruction::TLBR as usize] = mmu::tlbr;
    instrs[MmuSecondaryInstruction::TLBWI as usize] = mmu::tlbwi;
    instrs[MmuSecondaryInstruction::TLBWR as usize] = mmu::tlbwr;
    instrs[MmuSecondaryInstruction::TLBP as usize] = mmu::tlbp;
    instrs[MmuSecondaryInstruction::ERET as usize] = mmu::eret;

    instrs
};
