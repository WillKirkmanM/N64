const BITS_PER_BYTE: u64 = 8;

#[derive(Clone, Copy)]
pub(super) enum MergeSide {
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub(super) struct MergeLane {
    pub shift: u64,
    pub load_mask: u64,
    pub store_mask: u64,
    side: MergeSide,
}

impl MergeLane {
    pub fn new<const WIDTH: u64>(offset: u64, side: MergeSide) -> Self {
        match side {
            MergeSide::Left => {
                let shift = BITS_PER_BYTE * offset;

                Self {
                    shift,
                    load_mask: low_bits(shift),
                    store_mask: low_bits(BITS_PER_BYTE * (WIDTH - offset)),
                    side,
                }
            }
            MergeSide::Right => {
                let shift = BITS_PER_BYTE * (WIDTH - 1 - offset);

                Self {
                    shift,
                    load_mask: if offset == WIDTH - 1 {
                        0
                    } else {
                        !low_bits(BITS_PER_BYTE * (offset + 1))
                    },
                    store_mask: !low_bits(shift),
                    side,
                }
            }
        }
    }

    pub fn load(self, previous: u64, value: u64) -> u64 {
        let shifted = match self.side {
            MergeSide::Left => value << self.shift,
            MergeSide::Right => value >> self.shift,
        };

        (previous & self.load_mask) | shifted
    }

    pub fn store(self, value: u64) -> u64 {
        match self.side {
            MergeSide::Left => value >> self.shift,
            MergeSide::Right => value << self.shift,
        }
    }
}

fn low_bits(count: u64) -> u64 {
    if count == u64::BITS as u64 {
        u64::MAX
    } else {
        (1u64 << count) - 1
    }
}

#[cfg(test)]
mod tests;
