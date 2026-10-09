use super::*;

#[test]
fn malformed_packets_disable_stale_channels_before_device_dispatch() {
    let mut n64 = N64::default();
    n64.pif.memory.ram[..7].copy_from_slice(&[1, 4, 1, 0, 0, 0, 0]);
    n64.pif.memory.ram[7] = RAM_TERMINATOR;
    setup_channels_format(&mut n64);
    assert!(n64.pif.channels[0].buffers.tx.is_some());
    n64.pif.memory.ram[0] = 63;
    n64.pif.memory.ram[1] = 4;
    setup_channels_format(&mut n64);
    assert!(n64
        .pif
        .channels
        .iter()
        .all(|channel| channel.buffers.tx.is_none()));
    assert_eq!(update_pif_ram(&mut n64), TRANSFER_SETUP_CYCLES as u64);
}

#[test]
fn short_controller_and_cart_commands_report_protocol_errors() {
    let mut n64 = N64::default();
    for (channel, command, tx, rx) in [(0, 2, 1, 33), (4, 5, 2, 1), (4, 7, 1, 9), (4, 0xee, 1, 1)] {
        n64.pif.memory.ram.fill(0);
        n64.pif.memory.ram[..3].copy_from_slice(&[tx, rx, command]);
        n64.pif.channels[channel].configure(0, tx);
        process_channel(&mut n64, channel);
        assert_ne!(n64.pif.memory.ram[1] & CHANNEL_NO_DEVICE, 0);
    }
}

#[test]
fn pif_window_mirrors_the_two_kibibyte_device_aperture() {
    let mut n64 = N64::default();
    n64.pif.memory.rom[..4].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
    assert_eq!(
        read_mem(&mut n64, 0x1fc00800, memory::AccessSize::Word),
        0x12345678
    );
    write_mem(&mut n64, 0x1fc00fc0, 0xaabbccdd, u32::MAX);
    assert_eq!(
        read_mem(&mut n64, 0x1fc007c0, memory::AccessSize::Word),
        0xaabbccdd
    );
}
