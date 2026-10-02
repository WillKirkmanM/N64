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
