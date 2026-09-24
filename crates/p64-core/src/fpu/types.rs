use super::*;

pub struct Fpu {
    pub fcr0: u32,
    pub fcr31: u32,
    pub flush_mode: u32,
    pub fgr32: [[u8; 4]; 32],
    pub fgr64: [[u8; 8]; 32],
    pub instrs: [fn(&mut N64, u32); 32],
    pub b_instrs: [fn(&mut N64, u32); 4],
    pub s_instrs: [fn(&mut N64, u32); 64],
    pub d_instrs: [fn(&mut N64, u32); 64],
    pub w_instrs: [fn(&mut N64, u32); 64],
    pub l_instrs: [fn(&mut N64, u32); 64],
}

impl Default for Fpu {
    fn default() -> Self {
        fn default_instruction(_device: &mut N64, _value: u32) {}

        let mut fpu = Self {
            fcr0: 0,
            fcr31: 0,
            flush_mode: 0,
            fgr32: [[0; 4]; 32],
            fgr64: [[0; 8]; 32],
            instrs: [default_instruction; 32],
            b_instrs: [default_instruction; 4],
            s_instrs: [default_instruction; 64],
            d_instrs: [default_instruction; 64],
            w_instrs: [default_instruction; 64],
            l_instrs: [default_instruction; 64],
        };

        // Set FGR registers
        let status_reg = 0;
        let is_32bit_mode = (status_reg & (MmuStatus::FR as u64)) == 0;

        (0..32).step_by(2).for_each(|i| {
            if is_32bit_mode {
                let bytes = fpu.fgr64[i];
                fpu.fgr32[i] = bytes[0..4].try_into().unwrap();
                fpu.fgr32[i + 1] = bytes[4..8].try_into().unwrap();
            } else {
                let bytes_lo = fpu.fgr32[i];
                let bytes_hi = fpu.fgr32[i + 1];
                fpu.fgr64[i] = [bytes_lo, bytes_hi].concat().try_into().unwrap();
            }
        });

        // Initialize FCR0
        fpu.fcr0 = 0b101000000000;

        // Initialize instruction tables
        fpu.b_instrs = B_INSTRUCTIONS;
        fpu.s_instrs = init_s_instructions();
        fpu.d_instrs = init_d_instructions();

        let (l_instrs, w_instrs) = init_conversion_instructions();
        fpu.l_instrs = l_instrs;
        fpu.w_instrs = w_instrs;

        fpu.instrs = MAIN_INSTRUCTIONS;

        fpu
    }
}
