use super::*;

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
    XContext = 20,
    TagLo = 28,
    ErrorEPC = 30,
    RegsCount = 32,
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
    ExccodeSys = 8 << 2,
    ExccodeBP = 9 << 2,
    ExccodeRI = 10 << 2,
    ExccodeCPU = 11 << 2,
    ExccodeTR = 13 << 2,
    ExccodeFPE = 15 << 2,
    IP2 = 1 << 10,
    IP7 = 1 << 15,
    BD = 1 << 31,
    CE1 = 1 << 28,
    CE2 = 1 << 29,
    IPMask = 0b00000000000000001111111100000000,
    ExccodeMask = 0x1F << 2,
}

#[repr(u64)]
pub enum MmuXContext {
    BadVPN2Mask = 0b01111111111111111111111111110000,
    RegionMask = 0b110000000000000000000000000000000,
}

#[repr(u64)]
pub enum MmuMask {
    Index = 0b10000000000000000000000000111111,
    EntryLo = 0b0000000000000000000000000000000000111111111111111111111111111111,
    Context = 0b1111111111111111111111111111111111111111100000000000000000000000,
    PageMask = 0b00000001111111111110000000000000,
    Wired = 0b00000000000000000000000000111111,
    EntryHi = 0b1100000000000000000000001111111111111111111111111110000011111111,
    Status = 0b11111111010101111111111111111111,
    Cause = 0b00000000000000000000001100000000,
    Config = 0b00001111000000001000000000001111,
    LLAddr = 0b11111111111111111111111111111111,
    WatchLo = 0b11111111111111111111111111111011,
    WatchHi = 0b00000000000000000000000000001111,
    XContext = 0b1111111111111111111111111111111000000000000000000000000000000000,
    ParityErr = 0b00000000000000000000000011111111,
    TagLo = 0b00001111111111111111111111000000,
}
