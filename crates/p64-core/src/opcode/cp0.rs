//! CO operand bits do not select the operation.

const COPROCESSOR_ENCODING_COUNT: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Cp0Opcode {
    Mfc0 = 0x00,
    Dmfc0 = 0x01,
    Cfc0 = 0x02,
    Mtc0 = 0x04,
    Dmtc0 = 0x05,
    Ctc0 = 0x06,
    Bc0 = 0x08,
    Co = 0x10,
}

impl Cp0Opcode {
    pub(crate) const fn from_encoding(encoding: usize) -> Option<Self> {
        if encoding >= Self::Co as usize && encoding < COPROCESSOR_ENCODING_COUNT {
            return Some(Self::Co);
        }

        match encoding {
            value if value == Self::Mfc0 as usize => Some(Self::Mfc0),
            value if value == Self::Dmfc0 as usize => Some(Self::Dmfc0),
            value if value == Self::Cfc0 as usize => Some(Self::Cfc0),
            value if value == Self::Mtc0 as usize => Some(Self::Mtc0),
            value if value == Self::Dmtc0 as usize => Some(Self::Dmtc0),
            value if value == Self::Ctc0 as usize => Some(Self::Ctc0),
            value if value == Self::Bc0 as usize => Some(Self::Bc0),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Cp0Function {
    Tlbr = 0x01,
    Tlbwi = 0x02,
    Tlbwr = 0x06,
    Tlbp = 0x08,
    Rfe = 0x10,
    Eret = 0x18,
}

impl Cp0Function {
    pub(crate) const fn from_encoding(encoding: usize) -> Option<Self> {
        match encoding {
            value if value == Self::Tlbr as usize => Some(Self::Tlbr),
            value if value == Self::Tlbwi as usize => Some(Self::Tlbwi),
            value if value == Self::Tlbwr as usize => Some(Self::Tlbwr),
            value if value == Self::Tlbp as usize => Some(Self::Tlbp),
            value if value == Self::Rfe as usize => Some(Self::Rfe),
            value if value == Self::Eret as usize => Some(Self::Eret),
            _ => None,
        }
    }
}
