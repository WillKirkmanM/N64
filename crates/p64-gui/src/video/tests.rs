use super::*;

#[test]
fn rejects_invalid_rdram_before_native_initialization() {
    assert!(validate_rdram(std::ptr::null_mut(), 8 * 1024 * 1024).is_err());
    assert!(validate_rdram(std::ptr::without_provenance_mut(1), 8).is_err());
    assert!(validate_rdram(std::ptr::without_provenance_mut(RDRAM_ALIGNMENT), 0).is_err());
    if usize::BITS > 32 {
        assert!(validate_rdram(
            std::ptr::without_provenance_mut(RDRAM_ALIGNMENT),
            u32::MAX as usize + 1
        )
        .is_err());
    }
}

extern "C" {
    fn rdp_command_range_valid(current: u32, end: u32, memory_size: u32) -> u8;
    fn rdp_read_rdram_word(memory: *const u8, memory_size: u32, offset: u32) -> u32;
}
