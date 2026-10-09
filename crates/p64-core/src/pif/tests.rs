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
