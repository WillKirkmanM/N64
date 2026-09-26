use super::Status;

#[test]
fn privilege_and_width_match_raw_status_bits() {
    // Include reserved KSU=3, exception overrides and each UX/SX/KX bit.
    for low in 0..256u64 {
        for high in [0, 1 << 25, 1 << 28, (1 << 25) | (1 << 28)] {
            let raw = low | high;
            let status = Status(raw);
            let mode = if raw & 6 != 0 { 0 } else { (raw >> 3) & 3 };

            let extended = raw & (1 << (7 - mode.min(2))) != 0;

            assert_eq!(status.mode(), mode);
            assert_eq!(status.extended_addressing(), extended);
            assert_eq!(status.wide_instructions_allowed(), mode == 0 || extended);
            assert_eq!(status.reverse_endian(), raw & (1 << 25) != 0);
            assert_eq!(status.exception_level(), raw & 2 != 0);
            assert_eq!(status.error_level(), raw & 4 != 0);
            assert_eq!(
                status.cp0_usable(),
                raw & ((1 << 28) | 6) != 0 || raw & 0x18 == 0
            );
        }
    }
}
