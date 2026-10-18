use super::{MergeLane, MergeSide};

#[test]
fn merge_lanes_match_byte_transfers() {
    for width in [4, 8] {
        for offset in 0..width {
            for side in [MergeSide::Left, MergeSide::Right] {
                for bit in 0..64 {
                    let previous: u64 = 0xfedc_ba98_7654_3210;
                    let source = 1u64 << bit;
                    let lane = if width == 4 {
                        MergeLane::new::<4>(offset, side)
                    } else {
                        MergeLane::new::<8>(offset, side)
                    };

                    let mut expected = previous.to_be_bytes();
                    let source_bytes = source.to_be_bytes();
                    let start = 8 - width as usize;
                    let offset = offset as usize;

                    match side {
                        MergeSide::Left => {
                            expected[start..8 - offset]
                                .copy_from_slice(&source_bytes[start + offset..]);
                        }
                        MergeSide::Right => {
                            expected[7 - offset..]
                                .copy_from_slice(&source_bytes[start..start + offset + 1]);
                        }
                    }

                    if width == 4 && offset == 3 && matches!(side, MergeSide::Right) {
                        expected[..4].fill(0);
                    }
                    // Word left truncates upper bits before its instruction sign-extension.
                    let actual = lane.load(
                        previous,
                        source
                            & if width == 4 {
                                u32::MAX as u64
                            } else {
                                u64::MAX
                            },
                    );
                    let expected = u64::from_be_bytes(expected);
                    let mask = if width == 4 && matches!(side, MergeSide::Left) {
                        u32::MAX as u64
                    } else {
                        u64::MAX
                    };

                    assert_eq!(actual & mask, expected & mask);

                    let mut expected_store = [0u8; 8];
                    let mut expected_mask = [0u8; 8];

                    match side {
                        MergeSide::Left => {
                            expected_store[start + offset..]
                                .copy_from_slice(&source_bytes[start..8 - offset]);
                            expected_mask[start + offset..].fill(0xff);
                        }
                        MergeSide::Right => {
                            expected_store[start..start + offset + 1]
                                .copy_from_slice(&source_bytes[7 - offset..]);
                            expected_mask[start..start + offset + 1].fill(0xff);
                        }
                    }

                    let width_mask = if width == 4 {
                        u32::MAX as u64
                    } else {
                        u64::MAX
                    };

                    assert_eq!(
                        lane.store_mask & width_mask,
                        u64::from_be_bytes(expected_mask)
                    );
                    assert_eq!(
                        lane.store(source) & lane.store_mask & width_mask,
                        u64::from_be_bytes(expected_store)
                    );
                }
            }
        }
    }
}
