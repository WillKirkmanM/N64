use super::{Vector, ELEMENTS_PER_GROUP};

type SimdRegister = Vector;

const DIVISION_TABLE_SIZE: usize = 512;
const RECIPROCAL_SCALE: u64 = 1 << 34;
const RECIPROCAL_TABLE_SHIFT: u32 = 8;
const SQUARE_ROOT_SEED: u64 = 1 << 17;
const SQUARE_ROOT_SCALE: u64 = 1 << 44;

pub struct VectorState {
    pub shuffle: [SimdRegister; 16],
    pub reciprocals: [u16; DIVISION_TABLE_SIZE],
    pub inverse_square_roots: [u16; DIVISION_TABLE_SIZE],
}

pub struct VectorControlState {
    pub vcol: SimdRegister,
    pub vcoh: SimdRegister,
    pub vccl: SimdRegister,
    pub vcch: SimdRegister,
    pub vce: SimdRegister,
}

pub struct AccumulatorState {
    pub accl: SimdRegister,
    pub accm: SimdRegister,
    pub acch: SimdRegister,
}

#[derive(Default)]
pub struct DivisionState {
    pub divdp: bool,
    pub divin: i16,
    pub divout: i16,
}

impl Default for VectorState {
    fn default() -> Self {
        Self {
            shuffle: [Vector::ZERO; 16],
            reciprocals: [0; DIVISION_TABLE_SIZE],
            inverse_square_roots: [0; DIVISION_TABLE_SIZE],
        }
    }
}

impl Default for VectorControlState {
    fn default() -> Self {
        Self {
            vcol: Vector::ZERO,
            vcoh: Vector::ZERO,
            vccl: Vector::ZERO,
            vcch: Vector::ZERO,
            vce: Vector::ZERO,
        }
    }
}

impl Default for AccumulatorState {
    fn default() -> Self {
        Self {
            accl: Vector::ZERO,
            accm: Vector::ZERO,
            acch: Vector::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ElementPattern(pub(super) [usize; ELEMENTS_PER_GROUP]);

impl ElementPattern {
    pub(super) const fn new(pattern: [usize; ELEMENTS_PER_GROUP]) -> Self {
        Self(pattern)
    }
}

impl VectorState {
    pub(super) fn initialize_division_tables(&mut self) {
        self.reciprocals[0] = u16::MAX;
        for index in 1..DIVISION_TABLE_SIZE {
            let denominator = (index + DIVISION_TABLE_SIZE) as u64;
            let quotient = RECIPROCAL_SCALE / denominator;
            self.reciprocals[index] = ((quotient + 1) >> RECIPROCAL_TABLE_SHIFT) as u16;
        }

        for index in 0..DIVISION_TABLE_SIZE {
            let parity = if index % 2 == 1 { 1 } else { 0 };

            let denominator = ((index + DIVISION_TABLE_SIZE) >> parity) as u64;
            let mut root = SQUARE_ROOT_SEED;
            // Greatest fixed-point reciprocal root below the exact value.
            while denominator * (root + 1) * (root + 1) < SQUARE_ROOT_SCALE {
                root += 1;
            }
            self.inverse_square_roots[index] = (root >> 1) as u16;
        }
    }
}
