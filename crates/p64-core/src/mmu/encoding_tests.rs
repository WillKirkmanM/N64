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
