use crate::builder::registers::Registers;

mod metadata;

#[repr(u32)]
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum MmuRegister {
    Index = 0,
    Random = 1,
    EntryLo0 = 2,
    EntryLo1 = 3,
    Context = 4,
    PageMask = 5,
    Wired = 6,
    BadVAddr = 8,
    Count = 9,
    EntryHi = 10,
    Compare = 11,
    Status = 12,
    Cause = 13,
    EPC = 14,
    PrevID = 15,
    Config = 16,
    LLAddr = 17,
    WatchLo = 18,
    WatchHi = 19,
    XContext = 20,
    ParityErr = 26,
    CacheErr = 27,
    TagLo = 28,
    TagHi = 29,
    ErrorEPC = 30,
    RegsCount = 32,
}

impl<T> std::ops::Index<MmuRegister> for Registers<T> {
    type Output = T;
    fn index(&self, reg: MmuRegister) -> &Self::Output {
        &self.regs[reg as usize]
    }
}

impl<T> std::ops::IndexMut<MmuRegister> for Registers<T> {
    fn index_mut(&mut self, reg: MmuRegister) -> &mut Self::Output {
        &mut self.regs[reg as usize]
    }
}

#[repr(u64)]
pub enum MmuStatus {
    IE = 1 << 0,
    EXL = 1 << 1,
    ERL = 1 << 2,
    BEV = 1 << 22,
    FR = 1 << 26,
    CU1 = 1 << 29,
    CU2 = 1 << 30,
}

#[repr(u64)]
pub enum MmuCause {
    ExccodeMod = 1 << 2,
    ExccodeTLBL = 2 << 2,
    ExccodeTLBS = 3 << 2,
    ExccodeAdEL = 4 << 2,
    ExccodeAdES = 5 << 2,
    ExccodeSys = 8 << 2,
    ExccodeBP = 9 << 2,
    ExccodeRI = 10 << 2,
    ExccodeCPU = 11 << 2,
    ExccodeOv = 12 << 2,
    ExccodeTR = 13 << 2,
    ExccodeFPE = 15 << 2,
    IP2 = 1 << 10,
    IP7 = 1 << 15,
    BD = 1 << 31,
    CE1 = 1 << 28,
    CE2 = 1 << 29,
    IPMask = 0xff00,
    ExccodeMask = 0x1f << 2,
}

#[repr(u64)]
pub enum MmuXContext {
    BadVPN2Mask = 0x7fff_fff0,
    RegionMask = 0x0000_0001_8000_0000,
}

#[repr(u64)]
pub enum MmuMask {
    Index = 0x8000_003f,
    EntryLo = 0x3fff_ffff,
    Context = 0xffff_ffff_ff80_0000,
    PageMask = 0x01ff_e000,
    Wired = 0x3f,
    EntryHi = 0xc000_00ff_ffff_e0ff,
    Status = 0xff57_ffff,
    Cause = 0x300,
    Config = 0x0f00_800f,
    LLAddr = 0xffff_ffff,
    WatchLo = 0xffff_fffb,
    WatchHi = 0xf,
    XContext = 0xffff_fffe_0000_0000,
    ParityErr = 0xff,
}
