use crate::{
    cpu, memory,
    mmu::{MmuCause, MmuRegister, MmuStatus},
    tlb, N64,
};

const SOURCE: u64 = 0xfedc_ba98_7654_3210;
const UNCACHED_BASE: u64 = 0xffff_ffff_a000_0000;

fn machine() -> N64 {
    let mut n64 = N64::new();
    memory::initialise_memory_map(&mut n64);
    cpu::start_cpu(&mut n64);
    crate::cache::init(&mut n64);
    n64.set_cp0_register(MmuRegister::Status, MmuStatus::CU2 as u64);
    n64.cpu_mut().set_gpr(2, UNCACHED_BASE);
    n64.rdram.mem.data[..4].copy_from_slice(&0xfedc_ba98u32.to_le_bytes());
    n64.rdram.mem.data[4..8].copy_from_slice(&0x7654_3210u32.to_le_bytes());
    n64
}

fn execute(n64: &mut N64, primary: u32, register: u32, offset: u16) {
    let opcode = (primary << 26) | (2 << 21) | (register << 16) | u32::from(offset);
    cpu::decode_opcode(n64, opcode)(n64, opcode);
}

fn moved_latch(n64: &mut N64) -> u64 {
    let opcode = (0x12 << 26) | (1 << 21) | (4 << 16) | (9 << 11);
    cpu::decode_opcode(n64, opcode)(n64, opcode);
    n64.cpu().gpr(4)
}

fn word(n64: &N64, offset: usize) -> u32 {
    u32::from_le_bytes(n64.rdram.mem.data[offset..offset + 4].try_into().unwrap())
}

#[test]
fn hardware_cop2_memory_sequence_matches_seven_observed_results() {
    for cached in [false, true] {
        let mut n64 = machine();
        if cached {
            n64.cpu_mut().set_gpr(2, 0xffff_ffff_8000_0000);
        }
        execute(&mut n64, 0x32, 9, 0);
        assert_eq!(moved_latch(&mut n64), SOURCE);
        execute(&mut n64, 0x32, 9, 4);
        assert_eq!(moved_latch(&mut n64), SOURCE);
        execute(&mut n64, 0x3e, 9, 16);
        execute(&mut n64, 0x3a, 9, 28);
        execute(&mut n64, 0x36, 9, 0);
        assert_eq!(moved_latch(&mut n64), SOURCE);
        execute(&mut n64, 0x3e, 9, 40);
        execute(&mut n64, 0x3a, 9, 52);
        for (offset, expected) in [
            (16, 0xfedc_ba98),
            (20, 0x7654_3210),
            (28, 0x7654_3210),
            (40, 0xfedc_ba98),
            (44, 0x7654_3210),
            (52, 0x7654_3210),
        ] {
            n64.cpu_mut().execution.data_address = offset;
            let actual = memory::read_data(&mut n64, offset, memory::AccessSize::Word, cached);
            assert_eq!(actual, expected);
        }
        assert!(!matches!(
            n64.cpu().branch_state.state,
            cpu::State::Exception
        ));
    }
}

#[test]
fn cop2_unusable_precedes_alignment_translation_and_side_effects() {
    for primary in [0x32, 0x36, 0x3a, 0x3e] {
        for address in [UNCACHED_BASE + 1, 0x0040_0001, 0x0040_0000] {
            let mut n64 = machine();
            n64.set_cp0_register(MmuRegister::Status, 0);
            n64.set_cp0_register(MmuRegister::BadVAddr, 0x1234);
            n64.cpu_mut().set_gpr(2, address);
            n64.rcp_mut().reg_latch = 0xabcd;
            let before = n64.rdram.mem.data[..64].to_vec();
            execute(&mut n64, primary, 9, 0);
            assert_eq!(
                n64.cp0_register(MmuRegister::Cause),
                MmuCause::ExccodeCPU as u64 | MmuCause::CE2 as u64
            );
            assert_eq!(n64.cp0_register(MmuRegister::BadVAddr), 0x1234);
            assert_eq!(n64.rcp().reg_latch, 0xabcd);
            assert_eq!(&n64.rdram.mem.data[..64], before);
        }
    }
}

#[test]
fn cop2_alignment_and_tlb_faults_preserve_transfer_state() {
    for (primary, width, writing) in [
        (0x32, 4, false),
        (0x36, 8, false),
        (0x3a, 4, true),
        (0x3e, 8, true),
    ] {
        for offset in 1..width {
            let mut n64 = machine();
            n64.rcp_mut().reg_latch = 0xabcd;
            execute(&mut n64, primary, 9, offset);
            let cause = if writing {
                MmuCause::ExccodeAdES
            } else {
                MmuCause::ExccodeAdEL
            };
            assert_eq!(n64.cp0_register(MmuRegister::Cause), cause as u64);
            assert_eq!(
                n64.cp0_register(MmuRegister::BadVAddr),
                UNCACHED_BASE + u64::from(offset)
            );
            assert_eq!(n64.rcp().reg_latch, 0xabcd);
            assert_eq!(word(&n64, 0), 0xfedc_ba98);
        }
        let mut n64 = machine();
        n64.cpu_mut().set_gpr(2, 0x0040_0000);
        n64.rcp_mut().reg_latch = 0xabcd;
        execute(&mut n64, primary, 9, 0);
        let cause = if writing {
            MmuCause::ExccodeTLBS
        } else {
            MmuCause::ExccodeTLBL
        };
        assert_eq!(n64.cp0_register(MmuRegister::Cause), cause as u64);
        assert_eq!(n64.cp0_register(MmuRegister::BadVAddr), 0x0040_0000);
        assert_eq!(n64.rcp().reg_latch, 0xabcd);
    }
}

#[test]
fn cop2_doubleword_transfers_work_in_32_bit_user_mode_and_reverse_endian_word_stores() {
    for reverse in [false, true] {
        let mut n64 = machine();
        n64.set_cp0_register(MmuRegister::EntryHi, 0);
        n64.set_cp0_register(MmuRegister::EntryLo0, 0x17);
        n64.set_cp0_register(MmuRegister::EntryLo1, 0x57);
        tlb::write(&mut n64, 0);
        n64.set_cp0_register(
            MmuRegister::Status,
            MmuStatus::CU2 as u64 | (2 << 3) | if reverse { 1 << 25 } else { 0 },
        );
        n64.cpu_mut().set_gpr(2, 0);
        execute(&mut n64, 0x32, 9, 4);
        assert_eq!(moved_latch(&mut n64), SOURCE);
        execute(&mut n64, 0x36, 9, 0);
        assert_eq!(moved_latch(&mut n64), SOURCE);
        execute(&mut n64, 0x3e, 9, 16);
        assert_eq!(word(&n64, 16), 0xfedc_ba98);
        assert_eq!(word(&n64, 20), 0x7654_3210);
        execute(&mut n64, 0x3a, 9, 32);
        assert_eq!(word(&n64, if reverse { 36 } else { 32 }), 0x7654_3210);
        assert!(!matches!(
            n64.cpu().branch_state.state,
            cpu::State::Exception
        ));
    }
}
