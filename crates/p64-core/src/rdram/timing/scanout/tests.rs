use super::Scanout;

fn registers() -> [u32; 14] {
    let mut regs = [0; 14];
    regs[0] = 2;
    regs[1] = 0x100000;
    regs[2] = 320;
    regs[6] = 9;
    regs[7] = 399;
    regs[9] = (10 << 16) | 90;
    regs[10] = (2 << 16) | 6;
    regs[12] = 0x200;
    regs[13] = 0x400;
    regs
}

fn scanout(regs: &[u32]) -> Scanout {
    Scanout::from_registers(regs, 1, 8)
}

#[test]
fn quarter_rate_pixel_clock_and_scaled_source_span() {
    let output = scanout(&registers());
    assert_eq!(output.period, 400);
    assert_eq!(output.active_start, 40);
    assert_eq!(output.active_end, 360);
    assert_eq!(output.source_bytes, 80);
    assert_eq!(output.burst(440), Some((440, 0x100000, 96)));
    assert_eq!(output.burst(600), Some((600, 0x100040, 24)));
}

#[test]
fn row_stride_is_independent_of_visible_width() {
    let output = scanout(&registers());
    assert_eq!(output.burst(840), Some((840, 0x100280, 96)));
    let mut regs = registers();
    regs[2] = 640;
    let output = scanout(&regs);
    assert_eq!(output.source_bytes, 80);
    assert_eq!(output.burst(840), Some((840, 0x100500, 96)));
}

#[test]
fn horizontal_and_vertical_blanking_retain_latest_fetch() {
    let output = scanout(&registers());
    assert_eq!(output.burst(800), Some((600, 0x100040, 24)));
    assert_eq!(output.burst(1200), Some((1000, 0x1002c0, 24)));
    assert_eq!(output.burst(0), None);
    assert_eq!(output.burst(439), None);
}

#[test]
fn uneven_burst_spacing_never_fetches_beyond_source_span() {
    let mut regs = registers();
    regs[9] = (10 << 16) | 91;
    regs[12] = 0x400;
    let output = scanout(&regs);
    assert_eq!(output.source_bytes, 162);
    for clock in 440..800 {
        let (start, address, _) = output.burst(clock).expect("active scanline");
        assert!(start <= clock);
        assert!(matches!(address, 0x100000 | 0x100040 | 0x100080));
    }
    assert_eq!(output.burst(799).map(|burst| burst.1), Some(0x100080));
}
