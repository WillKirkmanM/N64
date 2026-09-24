use super::*;

pub struct Mmu {
    pub reg_latch: u64,
    pub regs: Registers<u64>,
    pub reg_write_masks: [u64; MmuRegister::RegsCount as usize],
    pub instrs: [fn(&mut N64, u32); 32],
    pub instrs2: [fn(&mut N64, u32); 32],
    pub tlb_lut_r: Vec<TlbLookupTable>,
    pub tlb_lut_w: Vec<TlbLookupTable>,
    pub tlb_entries: [TlbEntry; 32],
}

impl Default for Mmu {
    fn default() -> Self {
        fn default_instruction(_device: &mut N64, _value: u32) {}

        Mmu {
            reg_latch: 0,
            regs: Registers::new().with_size(MmuRegister::RegsCount as usize),
            reg_write_masks: [0; MmuRegister::RegsCount as usize],
            instrs: [default_instruction; 32],
            instrs2: [default_instruction; 32],
            tlb_lut_r: vec![
                TlbLookupTable {
                    address: 0,
                    cached: false,
                };
                0x100000
            ],
            tlb_lut_w: vec![
                TlbLookupTable {
                    address: 0,
                    cached: false,
                };
                0x100000
            ],
            tlb_entries: [TlbEntry::default(); 32],
        }
    }
}
