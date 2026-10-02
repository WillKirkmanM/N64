use super::AccessSize;
use crate::{
    memory::{unmapped_read_mem, unmapped_write_mem},
    N64,
};

const BUS_REGION_COUNT: usize = 0x2000;
const BUS_REGION_SHIFT: u32 = 16;

type ReadHandler = fn(&mut N64, u64, AccessSize) -> u32;
type WriteHandler = fn(&mut N64, u64, u32, u32);

pub struct MemoryBus {
    pub fast_read: [ReadHandler; BUS_REGION_COUNT],
    pub memory_map_read: [ReadHandler; BUS_REGION_COUNT],
    pub memory_map_write: [WriteHandler; BUS_REGION_COUNT],
}

impl Default for MemoryBus {
    fn default() -> Self {
        Self {
            fast_read: [unmapped_read_mem; BUS_REGION_COUNT],
            memory_map_read: [unmapped_read_mem; BUS_REGION_COUNT],
            memory_map_write: [unmapped_write_mem; BUS_REGION_COUNT],
        }
    }
}

impl MemoryBus {
    #[inline]
    pub(crate) fn region_index(address: u64) -> usize {
        (address >> BUS_REGION_SHIFT) as usize
    }

    pub(crate) fn map_range(
        &mut self,
        start: usize,
        end: usize,
        read: ReadHandler,
        write: WriteHandler,
        fast_read: Option<ReadHandler>,
    ) {
        let regions = Self::region_index(start as u64)..=Self::region_index(end as u64);

        for index in regions {
            self.memory_map_read[index] = read;
            self.memory_map_write[index] = write;

            if let Some(fast_read) = fast_read {
                self.fast_read[index] = fast_read;
            }
        }
    }
}
