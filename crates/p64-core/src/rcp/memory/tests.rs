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
