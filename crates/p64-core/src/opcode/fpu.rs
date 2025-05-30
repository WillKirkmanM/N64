#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Cop1Opcode {
    Mfc1 = 0x00,
    Dmfc1 = 0x01,
    Cfc1 = 0x02,
    Dcfc1 = 0x03,
    Mtc1 = 0x04,
    Dmtc1 = 0x05,
    Ctc1 = 0x06,
    Dctc1 = 0x07,
    Branch = 0x08,
    Single = 0x10,
    Double = 0x11,
    Word = 0x14,
    Long = 0x15,
}

impl Cop1Opcode {
    pub(crate) const fn from_encoding(encoding: usize) -> Option<Self> {
        match encoding {
            value if value == Self::Mfc1 as usize => Some(Self::Mfc1),
            value if value == Self::Dmfc1 as usize => Some(Self::Dmfc1),
            value if value == Self::Cfc1 as usize => Some(Self::Cfc1),
            value if value == Self::Dcfc1 as usize => Some(Self::Dcfc1),
            value if value == Self::Mtc1 as usize => Some(Self::Mtc1),
            value if value == Self::Dmtc1 as usize => Some(Self::Dmtc1),
            value if value == Self::Ctc1 as usize => Some(Self::Ctc1),
            value if value == Self::Dctc1 as usize => Some(Self::Dctc1),
            value if value == Self::Branch as usize => Some(Self::Branch),
            value if value == Self::Single as usize => Some(Self::Single),
            value if value == Self::Double as usize => Some(Self::Double),
            value if value == Self::Word as usize => Some(Self::Word),
            value if value == Self::Long as usize => Some(Self::Long),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum FpuBranchOpcode {
    Bc1f = 0x00,
    Bc1t = 0x01,
    Bc1fl = 0x02,
    Bc1tl = 0x03,
}

impl FpuBranchOpcode {
    pub(crate) const fn from_encoding(encoding: usize) -> Option<Self> {
        match encoding {
            value if value == Self::Bc1f as usize => Some(Self::Bc1f),
            value if value == Self::Bc1t as usize => Some(Self::Bc1t),
            value if value == Self::Bc1fl as usize => Some(Self::Bc1fl),
            value if value == Self::Bc1tl as usize => Some(Self::Bc1tl),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum FpuFunction {
    Add = 0x00,
    Sub = 0x01,
    Mul = 0x02,
    Div = 0x03,
    Sqrt = 0x04,
    Abs = 0x05,
    Mov = 0x06,
    Neg = 0x07,
    RoundL = 0x08,
    TruncL = 0x09,
    CeilL = 0x0a,
    FloorL = 0x0b,
    RoundW = 0x0c,
    TruncW = 0x0d,
    CeilW = 0x0e,
    FloorW = 0x0f,
    CvtS = 0x20,
    CvtD = 0x21,
    CvtW = 0x24,
    CvtL = 0x25,
    Cf = 0x30,
    CUn = 0x31,
    CEq = 0x32,
    CUeq = 0x33,
    COlt = 0x34,
    CUlt = 0x35,
    COle = 0x36,
    CUle = 0x37,
    CSf = 0x38,
    CNgle = 0x39,
    CSeq = 0x3a,
    CNgl = 0x3b,
    CLt = 0x3c,
    CNge = 0x3d,
    CLe = 0x3e,
    CNgt = 0x3f,
}

impl FpuFunction {

}
