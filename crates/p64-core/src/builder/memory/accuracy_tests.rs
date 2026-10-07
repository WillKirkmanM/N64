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
