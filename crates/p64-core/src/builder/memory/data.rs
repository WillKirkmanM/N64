use std::ops::{Deref, DerefMut};

/// Own the original byte allocation while exposing an independently aligned slice.
/// The Vec must retain its original allocation pointer and layout for deallocation.
#[derive(Default)]
pub struct MemoryData {
    allocation: Vec<u8>,
    offset: usize,
    len: usize,
    alignment: usize,
}

impl MemoryData {
    pub(super) fn zeroed(len: usize, alignment: usize) -> Self {
        assert!(
            alignment == 0 || alignment.is_power_of_two(),
            "Invalid alignment"
        );
        let effective_alignment = alignment.max(1);
        let allocation_len = len
            .checked_add(effective_alignment - 1)
            .expect("Aligned memory allocation size overflow");
        let allocation = vec![0; allocation_len];
        let address = allocation.as_ptr() as usize;
        let offset = address.wrapping_neg() & (effective_alignment - 1);

        Self {
            allocation,
            offset,
            len,
            alignment,
        }
    }

    pub(super) fn realign(&mut self, alignment: usize) {
        if self.alignment == alignment {
            return;
        }

        let mut aligned = Self::zeroed(self.len, alignment);
        aligned.copy_from_slice(self);
        *self = aligned;
    }
}

impl Clone for MemoryData {
    fn clone(&self) -> Self {
        let mut data = Self::zeroed(self.len, self.alignment);
        data.copy_from_slice(self);

        data
    }
}

impl From<Vec<u8>> for MemoryData {
    fn from(allocation: Vec<u8>) -> Self {
        let len = allocation.len();

        Self {
            allocation,
            offset: 0,
            len,
            alignment: 0,
        }
    }
}

impl Deref for MemoryData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.allocation[self.offset..self.offset + self.len]
    }
}

impl DerefMut for MemoryData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.allocation[self.offset..self.offset + self.len]
    }
}
