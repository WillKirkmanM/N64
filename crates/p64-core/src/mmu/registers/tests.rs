use super::MmuRegister;

const RAW_WRITE_MASKS: [u64; 32] = [
    0x8000_003f,
    0x0,
    0x3fff_ffff,
    0x3fff_ffff,
    0xffff_ffff_ff80_0000,
    0x01ff_e000,
    0x3f,
    0x0,
    0x0,
    0xffff_ffff,
    0xc000_00ff_ffff_e0ff,
    0xffff_ffff,
    0xff57_ffff,
    0x300,
    0xffff_ffff_ffff_ffff,
    0x0,
    0x0f00_800f,
    0xffff_ffff,
    0xffff_fffb,
    0xf,
    0xffff_fffe_0000_0000,
    0x0,
    0x0,
    0x0,
    0x0,
    0x0,
    0xff,
    0x0,
    0xffff_ffff,
    0x0,
    0xffff_ffff_ffff_ffff,
    0x0,
];

const RAW_RESET_VALUES: [Option<u64>; 32] = [
    None,
    Some(0x1f),
    None,
    None,
    Some(0x007f_fff0),
    None,
    None,
    None,
    Some(0xffff_ffff),
    None,
    None,
    None,
    Some(0x0040_0004),
    None,
    Some(0xffff_ffff_ffff_ffff),
    Some(0xb22),
    Some(0x7006_e460),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(0xffff_ffff_ffff_ffff),
    None,
];

#[test]
fn every_cp0_slot_matches_raw_hardware_write_mask_oracle() {
    for (index, expected) in RAW_WRITE_MASKS.into_iter().enumerate() {
        let actual = MmuRegister::from_index(index).map_or(0, MmuRegister::write_mask);

        assert_eq!(actual, expected, "CP0 slot {index}");
    }
}

#[test]
fn every_cp0_slot_matches_raw_reset_oracle() {
    for (index, expected) in RAW_RESET_VALUES.into_iter().enumerate() {
        let actual = MmuRegister::from_index(index).and_then(MmuRegister::reset_value);

        assert_eq!(actual, expected, "CP0 slot {index}");
    }
}

#[test]
fn initialization_installs_masks_and_preserves_nonreset_registers() {
    let mut registers = crate::mmu::MmuRegisters::default();

    for index in 0..32 {
        registers.regs[index] = 0xfeed_0000 + index as u64;
    }

    registers.initialize();

    assert_eq!(registers.reg_write_masks, RAW_WRITE_MASKS);

    for (index, reset) in RAW_RESET_VALUES.into_iter().enumerate() {
        assert_eq!(
            registers.regs[index],
            reset.unwrap_or(0xfeed_0000 + index as u64),
            "CP0 slot {index}"
        );
    }
}
