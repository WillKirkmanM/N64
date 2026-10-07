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
