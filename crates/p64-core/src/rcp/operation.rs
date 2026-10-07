use crate::{fpu::RoundingMode, opcode::{Cop1Opcode, FpuFunction}};

#[derive(Clone, Copy)]
pub(super) struct FloatOperation {
    pub format: Cop1Opcode,
    pub function: FpuFunction,
}

impl FloatOperation {
    pub fn double(self) -> bool {
        self.format == Cop1Opcode::Double
    }

    pub fn floating_source(self) -> bool {
        matches!(self.format, Cop1Opcode::Single | Cop1Opcode::Double)
    }

    pub fn binary(self) -> bool {
        matches!(
            self.function,
            FpuFunction::Add | FpuFunction::Sub | FpuFunction::Mul | FpuFunction::Div
        )
    }

    pub fn comparison(self) -> bool {
        self.function as u32 >= FpuFunction::Cf as u32
    }

    pub fn integer_result(self) -> bool {
        use FpuFunction::*;

        matches!(
            self.function,
            RoundL | TruncL | CeilL | FloorL | RoundW | TruncW | CeilW | FloorW | CvtW | CvtL
        )
    }

    pub fn long_result(self) -> bool {
        use FpuFunction::*;

        matches!(self.function, RoundL | TruncL | CeilL | FloorL | CvtL)
    }

    pub fn result_double(self) -> bool {
        self.function == FpuFunction::CvtD || (self.double() && self.function != FpuFunction::CvtS)
    }

    pub fn rounding(self, current: RoundingMode) -> RoundingMode {
        use FpuFunction::*;

        match self.function {
            RoundL | RoundW => RoundingMode::Nearest,
            TruncL | TruncW => RoundingMode::TowardZero,
            CeilL | CeilW => RoundingMode::TowardPositive,
            FloorL | FloorW => RoundingMode::TowardNegative,
            _ => current,
        }
    }
}
