use super::*;

#[test]
fn byte_writes_form_big_endian_output_words() {
    let mut n64 = N64::new();
    write_mem(&mut n64, 0x13ff0020, 0x41000000, 0xff000000);
    write_mem(&mut n64, 0x13ff0020, 0x00420000, 0x00ff0000);

    assert_eq!(read_mem(&mut n64, 0x13ff0020, AccessSize::Word), 0x41420000);
    write_mem(&mut n64, 0x13ff0014, 2, u32::MAX);

    assert_eq!(n64.is_viewer.transcript, "AB");
}
