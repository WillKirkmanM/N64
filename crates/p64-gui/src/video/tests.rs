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

#[test]
fn short_dmem_is_rejected_without_native_reads() {
    assert_eq!(process_rdp_list(&mut [0; 8], &mut [0; DMEM_SIZE - 1]), 0);
}

#[test]
fn native_invalid_init_and_repeated_close_are_safe() {
    let _lock = lock_renderer();
    unsafe {
        assert_ne!(vk_init(0, 8 * 1024 * 1024, 0), 0);
        vk_close();
        vk_close();
        assert_eq!(rdp_update_screen(), 0);
        full_sync();
        rdp_set_vi_register(u32::MAX, 0);
    }
}

extern "C" {
    fn rdp_command_range_valid(current: u32, end: u32, memory_size: u32) -> u8;
    fn rdp_read_rdram_word(memory: *const u8, memory_size: u32, offset: u32) -> u32;
}

#[test]
fn native_command_ranges_use_allocated_rdram_size() {
    const SIZE: u32 = 8 * 1024 * 1024;
    unsafe {
        assert_eq!(rdp_command_range_valid(SIZE - 8, SIZE, SIZE), 1);
        assert_eq!(rdp_command_range_valid(SIZE - 8, SIZE + 8, SIZE), 0);
        assert_eq!(rdp_command_range_valid(SIZE, SIZE + 8, SIZE), 0);
        assert_eq!(rdp_command_range_valid(16, 8, SIZE), 0);
        assert_eq!(rdp_command_range_valid(0, SIZE, 4 * 1024 * 1024), 0);
    }
}

#[test]
fn native_rdram_fetch_handles_unaligned_and_unpopulated_words() {
    let mut memory = [0u8; 9];
    memory[1..5].copy_from_slice(&0x12345678u32.to_le_bytes());
    memory[5..9].copy_from_slice(&0x89abcdefu32.to_le_bytes());
    let pointer = memory[1..].as_ptr();
    unsafe {
        assert_eq!(rdp_read_rdram_word(pointer, 8, 0), 0x12345678);
        assert_eq!(rdp_read_rdram_word(pointer, 8, 4), 0x89abcdef);
        assert_eq!(rdp_read_rdram_word(pointer, 8, 5), 0);
        assert_eq!(rdp_read_rdram_word(pointer, 8, 8), 0);
        assert_eq!(rdp_read_rdram_word(pointer, 8, u32::MAX), 0);
        assert_eq!(rdp_read_rdram_word(std::ptr::null(), 8, 0), 0);
    }
}
