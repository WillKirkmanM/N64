use super::Memory;

#[test]
fn memory_alignment_works_in_both_builder_orders() {
    for size in [0, 1, 65535, 65536, 65537, 8 * 1024 * 1024] {
        for mem in [
            Memory::new().with_size(size).with_alignment(65536).build(),
            Memory::new().with_alignment(65536).with_size(size).build(),
        ] {
            assert_eq!(mem.data.as_ptr() as usize % 65536, 0);
            assert_eq!(mem.len(), size);
            assert_eq!(mem.layout.size, size);
            assert!(mem.data.iter().all(|byte| *byte == 0));
        }
    }
}

#[test]
fn realignment_and_clone_preserve_contents_and_independent_storage() {
    let mut original = Memory::from(vec![1, 2, 3, 4]).with_alignment(65536);

    assert_eq!(&*original.data, &[1, 2, 3, 4]);
    let mut cloned = original.clone();

    assert_eq!(cloned.data.as_ptr() as usize % 65536, 0);
    assert_ne!(cloned.data.as_ptr(), original.data.as_ptr());
    cloned.data[0] = 9;

    assert_eq!(original.data[0], 1);
    original = original.with_alignment(128);

    assert_eq!(original.data.as_ptr() as usize % 128, 0);
    assert_eq!(&*original.data, &[1, 2, 3, 4]);
    original = original.with_alignment(0);

    assert_eq!(&*original.data, &[1, 2, 3, 4]);
    original = original.with_alignment(65536).with_size(3);

    assert_eq!(original.data.as_ptr() as usize % 65536, 0);
    assert_eq!(&*original.data, &[0, 0, 0]);
}

#[test]
#[should_panic(expected = "Invalid alignment")]
fn memory_rejects_non_power_of_two_alignment() {
    Memory::new().with_size(4).with_alignment(3);
}

#[test]
fn unmasked_memory_keeps_word_addresses_distinct() {
    let mut mem = Memory::new().with_size(8).build();
    mem.write_mem(0, 0x01234567);
    mem.write_mem(4, 0x89abcdef);

    assert_eq!(mem.read_mem_fast(0), 0x01234567);
    assert_eq!(mem.read_mem_fast(4), 0x89abcdef);
    mem.write_u32(4, 0x11223344, 0x00ff0000);

    assert_eq!(mem.read_u32(4), 0x8922cdef);
    assert_eq!(mem.read_u32(0), 0x01234567);
}

#[test]
fn masked_writes_check_bounds_after_address_masking() {
    let mut mem = Memory::new().with_size(8).with_mask(7).build();
    mem.write_u32(0x1004, 0x12345678, u32::MAX);

    assert_eq!(mem.read_u32(4), 0x12345678);
    mem.write_u32(7, u32::MAX, u32::MAX);

    assert_eq!(mem.read_u32(4), 0x12345678);
}
