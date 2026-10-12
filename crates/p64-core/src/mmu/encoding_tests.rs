use super::*;

type Handler = fn(&mut N64, u32);

const TRANSFERS: [Handler; 16] = [
    mfc0, dmfc0, cp0_noop, reserved, mtc0, dmtc0, cp0_noop, reserved, cp0_noop, reserved, reserved,
    reserved, reserved, reserved, reserved, reserved,
];

#[test]
fn every_cp0_dispatch_slot_matches_hardware_decoding_without_enum_oracles() {
    let machine = N64::new();
    for raw in 0..32 {
        let expected = TRANSFERS.get(raw).copied().unwrap_or(execute_cp0);
        assert!(
            std::ptr::fn_addr_eq(machine.cp0().instructions.instrs[raw], expected),
            "rs={raw:#x}"
        );
        let expected = match raw {
            1 => tlbr as Handler,
            2 => tlbwi,
            6 => tlbwr,
            8 => tlbp,
            16 => reserved,
            24 => eret,
            _ => cp0_noop,
        };
        assert!(
            std::ptr::fn_addr_eq(machine.cp0().instructions.instrs2[raw], expected),
            "function={raw:#x}"
        );
    }
}

#[test]
fn every_cp0_selector_checks_cu0_before_decoding_reserved_or_operand_fields() {
    let mut machine = N64::new();
    cpu::start_cpu(&mut machine);
    for mode in [1_u64, 2] {
        for selector in 0..32 {
            for operands in [0_u32, 0x001f_ffc0, 0x001f_ffff] {
                machine.set_cp0_register(MmuRegister::Status, mode << 3);
                machine.set_cp0_register(MmuRegister::Cause, 0);
                machine.set_cp0_register(MmuRegister::EntryHi, 0x1234_00ab);
                machine.cpu.branch_state.state = State::Step;
                machine.cpu.execution.pc = 0x2000;
                machine.cpu.registers.gpr.fill(0x5678);
                let opcode = (0x10 << 26) | (selector << 21) | operands;
                cpu::decode_opcode(&machine, opcode)(&mut machine, opcode);
                assert_eq!(
                    machine.cp0_register(MmuRegister::Cause) & 0x3000_007c,
                    11 << 2,
                    "{opcode:#x}"
                );
                assert_eq!(machine.cp0_register(MmuRegister::EntryHi), 0x1234_00ab);
                assert_eq!(machine.cpu.registers.gpr, [0x5678; 32]);
                assert_eq!(machine.cp0_register(MmuRegister::EPC), 0x2000);
            }
        }
    }
}
