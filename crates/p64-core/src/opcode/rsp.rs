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

}
