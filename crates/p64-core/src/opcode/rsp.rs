const COPROCESSOR_ENCODING_COUNT: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Cop2Opcode {
    Mfc2 = 0x00,
    Dmfc2 = 0x01,
    Cfc2 = 0x02,
    Mtc2 = 0x04,
    Dmtc2 = 0x05,
    Ctc2 = 0x06,
}

impl Cop2Opcode {
    pub(crate) const fn from_encoding(encoding: usize) -> Option<Self> {
        match encoding {
            value if value == Self::Mfc2 as usize => Some(Self::Mfc2),
            value if value == Self::Dmfc2 as usize => Some(Self::Dmfc2),
            value if value == Self::Cfc2 as usize => Some(Self::Cfc2),
            value if value == Self::Mtc2 as usize => Some(Self::Mtc2),
            value if value == Self::Dmtc2 as usize => Some(Self::Dmtc2),
            value if value == Self::Ctc2 as usize => Some(Self::Ctc2),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum RspCop2Opcode {
    Mfc2 = 0x00,
    Cfc2 = 0x02,
    Mtc2 = 0x04,
    Ctc2 = 0x06,
    Vector = 0x10,
}

impl RspCop2Opcode {
    pub(crate) const fn from_encoding(encoding: usize) -> Option<Self> {
        if encoding >= Self::Vector as usize && encoding < COPROCESSOR_ENCODING_COUNT {
            return Some(Self::Vector);
        }

        match encoding {
            value if value == Self::Mfc2 as usize => Some(Self::Mfc2),
            value if value == Self::Cfc2 as usize => Some(Self::Cfc2),
            value if value == Self::Mtc2 as usize => Some(Self::Mtc2),
            value if value == Self::Ctc2 as usize => Some(Self::Ctc2),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum VectorLoadOpcode {
    Lbv = 0x00,
    Lsv = 0x01,
    Llv = 0x02,
    Ldv = 0x03,
    Lqv = 0x04,
    Lrv = 0x05,
    Lpv = 0x06,
    Luv = 0x07,
    Lhv = 0x08,
    Lfv = 0x09,
    Lwv = 0x0a,
    Ltv = 0x0b,
}

impl VectorLoadOpcode {
    pub(crate) const fn from_encoding(encoding: usize) -> Option<Self> {
        match encoding {
            value if value == Self::Lbv as usize => Some(Self::Lbv),
            value if value == Self::Lsv as usize => Some(Self::Lsv),
            value if value == Self::Llv as usize => Some(Self::Llv),
            value if value == Self::Ldv as usize => Some(Self::Ldv),
            value if value == Self::Lqv as usize => Some(Self::Lqv),
            value if value == Self::Lrv as usize => Some(Self::Lrv),
            value if value == Self::Lpv as usize => Some(Self::Lpv),
            value if value == Self::Luv as usize => Some(Self::Luv),
            value if value == Self::Lhv as usize => Some(Self::Lhv),
            value if value == Self::Lfv as usize => Some(Self::Lfv),
            value if value == Self::Lwv as usize => Some(Self::Lwv),
            value if value == Self::Ltv as usize => Some(Self::Ltv),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum VectorStoreOpcode {
    Sbv = 0x00,
    Ssv = 0x01,
    Slv = 0x02,
    Sdv = 0x03,
    Sqv = 0x04,
    Srv = 0x05,
    Spv = 0x06,
    Suv = 0x07,
    Shv = 0x08,
    Sfv = 0x09,
    Swv = 0x0a,
    Stv = 0x0b,
}

impl VectorStoreOpcode {
    pub(crate) const fn from_encoding(encoding: usize) -> Option<Self> {
        match encoding {
            value if value == Self::Sbv as usize => Some(Self::Sbv),
            value if value == Self::Ssv as usize => Some(Self::Ssv),
            value if value == Self::Slv as usize => Some(Self::Slv),
            value if value == Self::Sdv as usize => Some(Self::Sdv),
            value if value == Self::Sqv as usize => Some(Self::Sqv),
            value if value == Self::Srv as usize => Some(Self::Srv),
            value if value == Self::Spv as usize => Some(Self::Spv),
            value if value == Self::Suv as usize => Some(Self::Suv),
            value if value == Self::Shv as usize => Some(Self::Shv),
            value if value == Self::Sfv as usize => Some(Self::Sfv),
            value if value == Self::Swv as usize => Some(Self::Swv),
            value if value == Self::Stv as usize => Some(Self::Stv),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum VectorOpcode {
    Vmulf = 0x00,
    Vmulu = 0x01,
    Vrndp = 0x02,
    Vmulq = 0x03,
    Vmudl = 0x04,
    Vmudm = 0x05,
    Vmudn = 0x06,
    Vmudh = 0x07,
    Vmacf = 0x08,
    Vmacu = 0x09,
    Vrndn = 0x0a,
    Vmacq = 0x0b,
    Vmadl = 0x0c,
    Vmadm = 0x0d,
    Vmadn = 0x0e,
    Vmadh = 0x0f,
    Vadd = 0x10,
    Vsub = 0x11,
    Vzero = 0x12,
    Vabs = 0x13,
    Vaddc = 0x14,
    Vsubc = 0x15,
    Vsar = 0x1d,
    Vlt = 0x20,
    Veq = 0x21,
    Vne = 0x22,
    Vge = 0x23,
    Vcl = 0x24,
    Vch = 0x25,
    Vcr = 0x26,
    Vmrg = 0x27,
    Vand = 0x28,
    Vnand = 0x29,
    Vor = 0x2a,
    Vnor = 0x2b,
    Vxor = 0x2c,
    Vnxor = 0x2d,
    Vrcp = 0x30,
    Vrcpl = 0x31,
    Vrcph = 0x32,
    Vmov = 0x33,
    Vrsq = 0x34,
    Vrsql = 0x35,
    Vrsqh = 0x36,
    Vnop = 0x37,
    Vnull = 0x3f,
}

impl VectorOpcode {
    pub(crate) const fn from_encoding(encoding: usize) -> Option<Self> {
        match encoding {
            value if value == Self::Vmulf as usize => Some(Self::Vmulf),
            value if value == Self::Vmulu as usize => Some(Self::Vmulu),
            value if value == Self::Vrndp as usize => Some(Self::Vrndp),
            value if value == Self::Vmulq as usize => Some(Self::Vmulq),
            value if value == Self::Vmudl as usize => Some(Self::Vmudl),
            value if value == Self::Vmudm as usize => Some(Self::Vmudm),
            value if value == Self::Vmudn as usize => Some(Self::Vmudn),
            value if value == Self::Vmudh as usize => Some(Self::Vmudh),
            value if value == Self::Vmacf as usize => Some(Self::Vmacf),
            value if value == Self::Vmacu as usize => Some(Self::Vmacu),
            value if value == Self::Vrndn as usize => Some(Self::Vrndn),
            value if value == Self::Vmacq as usize => Some(Self::Vmacq),
            value if value == Self::Vmadl as usize => Some(Self::Vmadl),
            value if value == Self::Vmadm as usize => Some(Self::Vmadm),
            value if value == Self::Vmadn as usize => Some(Self::Vmadn),
            value if value == Self::Vmadh as usize => Some(Self::Vmadh),
            value if value == Self::Vadd as usize => Some(Self::Vadd),
            value if value == Self::Vsub as usize => Some(Self::Vsub),
            value if value == Self::Vzero as usize => Some(Self::Vzero),
            value if value == Self::Vabs as usize => Some(Self::Vabs),
            value if value == Self::Vaddc as usize => Some(Self::Vaddc),
            value if value == Self::Vsubc as usize => Some(Self::Vsubc),
            value if value == Self::Vsar as usize => Some(Self::Vsar),
            value if value == Self::Vlt as usize => Some(Self::Vlt),
            value if value == Self::Veq as usize => Some(Self::Veq),
            value if value == Self::Vne as usize => Some(Self::Vne),
            value if value == Self::Vge as usize => Some(Self::Vge),
            value if value == Self::Vcl as usize => Some(Self::Vcl),
            value if value == Self::Vch as usize => Some(Self::Vch),
            value if value == Self::Vcr as usize => Some(Self::Vcr),
            value if value == Self::Vmrg as usize => Some(Self::Vmrg),
            value if value == Self::Vand as usize => Some(Self::Vand),
            value if value == Self::Vnand as usize => Some(Self::Vnand),
            value if value == Self::Vor as usize => Some(Self::Vor),
            value if value == Self::Vnor as usize => Some(Self::Vnor),
            value if value == Self::Vxor as usize => Some(Self::Vxor),
            value if value == Self::Vnxor as usize => Some(Self::Vnxor),
            value if value == Self::Vrcp as usize => Some(Self::Vrcp),
            value if value == Self::Vrcpl as usize => Some(Self::Vrcpl),
            value if value == Self::Vrcph as usize => Some(Self::Vrcph),
            value if value == Self::Vmov as usize => Some(Self::Vmov),
            value if value == Self::Vrsq as usize => Some(Self::Vrsq),
            value if value == Self::Vrsql as usize => Some(Self::Vrsql),
            value if value == Self::Vrsqh as usize => Some(Self::Vrsqh),
            value if value == Self::Vnop as usize => Some(Self::Vnop),
            value if value == Self::Vnull as usize => Some(Self::Vnull),
            _ => None,
        }
    }
}
