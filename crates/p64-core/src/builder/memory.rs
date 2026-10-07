mod data;
pub use data::MemoryData;

use crate::{bits::replace_bits, mempak::MEMPAK_SIZE};

const PAK_BANK_SHIFT: u32 = u16::BITS;
const PAK_OFFSET_MASK: u64 = u16::MAX as u64;

mod access;
mod layout;
mod timing;
pub use access::MemoryAccess;
pub use layout::MemoryLayout;
pub use timing::MemoryTiming;

mod types;
pub use types::{Memory, WriteCallback};

impl Memory {
    pub fn build(self) -> Self {
        self
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_write_callback(mut self, callback: WriteCallback) -> Self {
        self.access.write_callback = Some(callback);
        self
    }

    pub fn with_mask(mut self, mask: usize) -> Self {
        self.access.mask = Some(mask);
        self
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.data = MemoryData::zeroed(size, self.layout.alignment);
        self.layout.size = size;
        self
    }

    pub fn with_alignment(mut self, alignment: usize) -> Self {
        self.data.realign(alignment);
        self.layout.alignment = alignment;
        self
    }

    pub fn is_little_endian(mut self) -> Self {
        self.access.is_little_endian = true;
        self
    }

    pub fn is_rdram(mut self) -> Self {
        self.access.is_rdram = true;
        self
    }

    pub fn calculate_cycles_from_addr_size(mut self) -> Self {
        self.timing.calculate_cycles_from_addr_size = true;
        self
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn read_mem_fast(&self, address: u64) -> u32 {
        if self.access.is_mempak {
            return Self::pak_offset(address)
                .map(|offset| u32::from_be_bytes(self.data[offset..offset + 4].try_into().unwrap()))
                .unwrap_or(0);
        }

        let masked_address = address as usize & self.access.mask.unwrap_or(usize::MAX);
        let bytes = self.data[masked_address..masked_address + 4]
            .try_into()
            .unwrap();

        if self.access.is_little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        }
    }

    pub fn write_mem(&mut self, address: u64, value: u32) {
        if self.access.is_mempak {
            self.write_pak_word(address, value);
            return;
        }

        if let Some(callback) = self.access.write_callback {
            if callback(address, value) {
                return;
            }
        }

        let masked_address = match self.access.mask {
            Some(mask) => (address as usize & mask) as u64,
            None => address,
        };

        if masked_address + 4 > self.layout.size as u64 {
            panic!("Memory access out of bounds");
        }

        let bytes = self.word_bytes(value);

        self.data[masked_address as usize..masked_address as usize + 4].copy_from_slice(&bytes);
    }

    fn pak_offset(address: u64) -> Option<usize> {
        let bank = (address >> PAK_BANK_SHIFT) as usize;
        let offset = (address & PAK_OFFSET_MASK) as usize;

        (offset < MEMPAK_SIZE).then(|| bank * MEMPAK_SIZE + offset)
    }

    fn write_pak_word(&mut self, address: u64, value: u32) {
        if let Some(offset) = Self::pak_offset(address) {
            self.data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
    }

    pub fn read_u32(&self, address: u64) -> u32 {
        self.read_mem_fast(address)
    }

    #[inline]
    fn word_bytes(&self, value: u32) -> [u8; 4] {
        if self.access.is_little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        }
    }

    pub fn write_u32(&mut self, address: u64, value: u32, mask: u32) {
        if self.access.is_mempak {
            self.write_pak_word(address, value);
            return;
        }

        if let Some(callback) = self.access.write_callback {
            if callback(address, value) {
                return;
            }
        }

        let masked_address = address as usize & self.access.mask.unwrap_or(usize::MAX);

        if masked_address
            .checked_add(4)
            .is_none_or(|end| end > self.data.len())
        {
            return;
        }

        let value = if mask == u32::MAX {
            value
        } else {
            replace_bits(self.read_u32(address), value, mask)
        };

        let bytes = self.word_bytes(value);
        self.data[masked_address..masked_address + 4].copy_from_slice(&bytes);
    }
}

impl From<Vec<u8>> for Memory {
    fn from(data: Vec<u8>) -> Self {
        let size = data.len();

        Memory {
            data: data.into(),
            layout: MemoryLayout {
                size,
                ..MemoryLayout::default()
            },
            access: MemoryAccess {
                is_little_endian: true,
                ..MemoryAccess::default()
            },
            timing: MemoryTiming::default(),
        }
    }
}

impl<I: std::slice::SliceIndex<[u8]>> std::ops::Index<I> for Memory {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.data[index]
    }
}

impl<I: std::slice::SliceIndex<[u8]>> std::ops::IndexMut<I> for Memory {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.data[index]
    }
}
