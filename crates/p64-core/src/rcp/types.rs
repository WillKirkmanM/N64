use super::*;

pub struct Rcp {
    pub instrs: [fn(&mut N64, u32); 32],
    pub reg_latch: u64,
}

impl Default for Rcp {
    fn default() -> Self {
        fn default_instruction(_device: &mut N64, _value: u32) {}

        fn mfc2(d: &mut N64, op: u32) {
            handle_move(d, op, false, false);
        }
        fn dmfc2(d: &mut N64, op: u32) {
            handle_move(d, op, true, false);
        }
        fn cfc2(d: &mut N64, op: u32) {
            handle_move(d, op, false, false);
        }
        fn mtc2(d: &mut N64, op: u32) {
            handle_move(d, op, false, true);
        }
        fn dmtc2(d: &mut N64, op: u32) {
            handle_move(d, op, true, true);
        }
        fn ctc2(d: &mut N64, op: u32) {
            handle_move(d, op, false, true);
        }

        let mut instrs: [fn(&mut N64, u32); 32] = [default_instruction; 32];
        let functions: [fn(&mut N64, u32); 6] = [mfc2, dmfc2, cfc2, mtc2, dmtc2, ctc2];

        for (i, &f) in functions.iter().enumerate() {
            instrs[i] = f;
        }

        Rcp {
            instrs,
            reg_latch: 0,
        }
    }
}
