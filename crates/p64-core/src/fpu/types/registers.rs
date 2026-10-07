const REGISTER_COUNT: usize = 32;

#[derive(Default)]
pub struct FpuRegisters {
    // Bytes remain little-endian on every host. FR=0 pairs low then high words.
    pub fgr32: [[u8; 4]; REGISTER_COUNT],
    pub fgr64: [[u8; 8]; REGISTER_COUNT],
}

impl FpuRegisters {
    pub(crate) fn operand_bits(&self, index: usize, double: bool, wide: bool) -> u64 {
        if double {
            self.double(index, wide).to_bits()
        } else {
            u64::from(self.single(index, wide).to_bits())
        }
    }

    pub(crate) fn target_bits(&self, index: usize, double: bool, wide: bool) -> u64 {
        if !wide && index & 1 != 0 {
            // Arithmetic ft selects the physical register in FR=0.
            let bits = u64::from_le_bytes(self.fgr64[index]);

            if double {
                bits
            } else {
                u64::from(bits as u32)
            }
        } else {
            self.operand_bits(index, double, wide)
        }
    }

    pub(crate) fn set_arithmetic_result(
        &mut self,
        index: usize,
        bits: u64,
        double: bool,
        wide: bool,
    ) {
        if !wide && index & 1 != 0 {
            // Odd physical fd is hidden until FR=1; loads/moves use paired mapping.
            self.fgr64[index] = bits.to_le_bytes();
        } else if double {
            self.set_double(index, f64::from_bits(bits), wide);
        } else {
            self.set_single(index, f32::from_bits(bits as u32), true, wide);
        }
    }

    pub fn set_single(&mut self, index: usize, value: f32, clear_high: bool, wide: bool) {
        let low = value.to_le_bytes();

        if wide {
            let high = if clear_high {
                [0; 4]
            } else {
                high_word(self.fgr64[index])
            };

            self.fgr64[index] = join_register_words(low, high);
        } else if clear_high {
            let (even, odd) = register_pair(index);
            self.fgr32[even] = low;
            self.fgr32[odd] = [0; 4];
        } else {
            self.fgr32[index] = low;
        }
    }

    pub fn single(&self, index: usize, wide: bool) -> f32 {
        f32::from_le_bytes(if wide {
            low_word(self.fgr64[index])
        } else {
            self.fgr32[index]
        })
    }

    pub fn set_double(&mut self, index: usize, value: f64, wide: bool) {
        let bytes = value.to_le_bytes();

        if wide {
            self.fgr64[index] = bytes;
        } else {
            let (even, odd) = register_pair(index);
            self.fgr32[even] = low_word(bytes);
            self.fgr32[odd] = high_word(bytes);
        }
    }

    pub fn double(&self, index: usize, wide: bool) -> f64 {
        f64::from_le_bytes(if wide {
            self.fgr64[index]
        } else {
            let (even, odd) = register_pair(index);

            join_register_words(self.fgr32[even], self.fgr32[odd])
        })
    }

    pub fn set_register_mode(&mut self, wide: bool) {
        for even in (0..self.fgr32.len()).step_by(2) {
            if wide {
                self.fgr64[even] = join_register_words(self.fgr32[even], self.fgr32[even + 1]);
            } else {
                self.fgr32[even] = low_word(self.fgr64[even]);
                self.fgr32[even + 1] = high_word(self.fgr64[even]);
            }
        }
    }
}

fn low_word(bytes: [u8; 8]) -> [u8; 4] {
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

fn high_word(bytes: [u8; 8]) -> [u8; 4] {
    [bytes[4], bytes[5], bytes[6], bytes[7]]
}

fn join_register_words(low: [u8; 4], high: [u8; 4]) -> [u8; 8] {
    [
        low[0], low[1], low[2], low[3], high[0], high[1], high[2], high[3],
    ]
}

fn register_pair(index: usize) -> (usize, usize) {
    (crate::bits::align_down(index, 2), index | 1)
}
