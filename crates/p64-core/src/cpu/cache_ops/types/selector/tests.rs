use super::{CacheBank, CacheOperation, CacheSelector};

#[test]
fn cache_selector_preserves_all_valid_and_reserved_encodings() {
    for encoding in 0..256 {
        let expected = match encoding {
            0x00 | 0x01 | 0x04 | 0x05 | 0x08 | 0x09 | 0x0d | 0x10 | 0x11 | 0x14 | 0x15 | 0x18
            | 0x19 => Some((encoding & 1, encoding >> 2)),
            _ => None,
        };

        let actual = CacheSelector::decode(encoding).map(|selector| {
            let bank = match selector.bank {
                CacheBank::Instruction => 0,
                CacheBank::Data => 1,
            };
            let operation = match selector.operation {
                CacheOperation::IndexInvalidate => 0,
                CacheOperation::IndexLoadTag => 1,
                CacheOperation::IndexStoreTag => 2,
                CacheOperation::CreateDirtyExclusive => 3,
                CacheOperation::HitInvalidate => 4,
                CacheOperation::FillOrWritebackInvalidate => 5,
                CacheOperation::HitWriteback => 6,
            };

            (bank, operation)
        });

        assert_eq!(actual, expected, "CACHE selector {encoding:#x}");
    }
}
