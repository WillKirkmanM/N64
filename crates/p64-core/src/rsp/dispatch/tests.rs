use super::*;
use crate::{mmu::MmuRegister, N64};

fn cpu_snapshot(n64: &N64) -> [u64; 5] {
    [
        n64.cpu().execution.pc,
        n64.cp0_register(MmuRegister::Cause),
        n64.cp0_register(MmuRegister::EPC),
        n64.cp0_register(MmuRegister::Status),
        n64.cp0_register(MmuRegister::Count),
    ]
}

#[test]
fn every_undefined_scalar_family_stays_isolated_from_main_cpu() {
    let mut n64 = N64::default();
    let tables = InstructionTables::initialized();
    n64.cpu_mut().execution.pc = 0xffff_ffff_8000_1234;
    n64.set_cp0_register(MmuRegister::Cause, 0x8000_0400);
    n64.set_cp0_register(MmuRegister::EPC, 0xffff_ffff_8000_5678);
    n64.set_cp0_register(MmuRegister::Status, 0x2000_0000);
    n64.set_cp0_register(MmuRegister::Count, 100);
    let snapshot = cpu_snapshot(&n64);
    let families: [(u32, u32, u32, &[u32]); 7] = [
        (
            0,
            26,
            64,
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 32, 33, 35, 36, 37,
                39, 40, 41, 43, 50, 58,
            ],
        ),
        (
            0,
            0,
            64,
            &[
                0, 2, 3, 4, 6, 7, 8, 9, 13, 32, 33, 34, 35, 36, 37, 38, 39, 42, 43,
            ],
        ),
        (1 << 26, 16, 32, &[0, 1, 16, 17]),
        (16 << 26, 21, 32, &[0, 4]),
        (
            18 << 26,
            21,
            32,
            &[
                0, 2, 4, 6, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
            ],
        ),
        (50 << 26, 11, 32, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
        (58 << 26, 11, 32, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
    ];
    for (base, shift, count, implemented) in families {
        for encoding in 0..count {
            if implemented.contains(&encoding) {
                continue;
            }
            let opcode = base | (encoding << shift);
            tables.decode(opcode)(&mut n64, opcode);
            assert_eq!(cpu_snapshot(&n64), snapshot, "opcode={opcode:08x}");
        }
    }
}

#[test]
fn undefined_special_encodings_use_rsp_local_source_self_shift() {
    let mut n64 = N64::default();
    let tables = InstructionTables::initialized();
    for source in [0, 1, 31, 32, 0x8000_0001, u32::MAX] {
        n64.rsp_mut().registers.gpr[3] = source;
        n64.rsp_mut().registers.gpr[4] = 0x5555_5555;
        let opcode = (3 << 21) | (4 << 11) | 0x0c;
        tables.decode(opcode)(&mut n64, opcode);
        assert_eq!(n64.rsp().registers.gpr[4], source / (1u32 << (source % 32)));
        assert_eq!(n64.rsp().registers.gpr[3], source);
    }
}

mod operands;

mod transfers;

mod vector_ops;
