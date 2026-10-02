use super::*;

#[test]
fn tiny_vi_periods_and_elapsed_deadlines_do_not_divide_or_subtract_underflow() {
    let mut n64 = N64::default();
    write_regs(&mut n64, ViRegister::VSync as u64 * 4, 1, u32::MAX);
    write_regs(&mut n64, ViRegister::HSync as u64 * 4, 0, u32::MAX);
    let count = n64.cp0_register(MmuRegister::Count);
    n64.video.update_current_line(Some(count + 1), count);
    n64.video.update_current_line(Some(count), count + 10);
    assert!(n64.video.regs[ViRegister::Current as usize] <= 2);
    write_regs(&mut n64, ViRegister::VSync as u64 * 4, u32::MAX, u32::MAX);
    assert_eq!(n64.video.regs[ViRegister::VSync as usize], 0x3ff);
}
