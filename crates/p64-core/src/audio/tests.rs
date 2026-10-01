use super::*;

#[test]
fn hardware_masks_and_partial_writes_drive_audio_deadline() {
    let mut n64 = N64::default();
    handle_ai_register_write(&mut n64, 16, u32::MAX, u32::MAX);
    assert_eq!(
        n64.audio.read_register(AiRegister::DacRate as usize),
        0x3fff
    );
    handle_ai_register_write(&mut n64, 16, 0, 0xff);
    assert_eq!(
        n64.audio.read_register(AiRegister::DacRate as usize),
        0x3f00
    );
    n64.audio
        .write_register(AiRegister::Len as usize, 8, u32::MAX);
    assert_eq!(
        n64.audio.calculate_dma_duration(48_681_812, 93_750_000),
        8 * 93_750_000 * 0x3f01 / (4 * 48_681_812)
    );
    handle_ai_register_write(&mut n64, 16, 50_000_000, u32::MAX);
    handle_ai_register_write(&mut n64, 4, 8, u32::MAX);
    assert!(n64.audio.is_busy());
}

#[test]
fn silence_deficit_converts_bytes_to_whole_stereo_frames() {
    assert_eq!(silence_sample_count(32_000.0 * 0.02 * 4.0), 1280);
    assert_eq!(silence_sample_count(1.0), 2);
    assert_eq!(silence_sample_count(0.0), 0);
}
