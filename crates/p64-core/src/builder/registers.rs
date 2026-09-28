mod command;
pub use command::FlagCommand;

use crate::{memory::AccessSize, N64};
use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

const REGISTER_WINDOW_MASK: u64 = 0xffff;
const REGISTER_WORD_SHIFT: u32 = 2;

/// Decode a word register within an RCP device's 64 KiB register window.
#[inline]
pub fn register_index(address: u64) -> usize {
    ((address & REGISTER_WINDOW_MASK) >> REGISTER_WORD_SHIFT) as usize
}

mod types;
pub use types::Registers;

impl<T: Default + Copy> Registers<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self) -> Registers<T> {
        Registers {
            size: self.size,
            regs: vec![T::default(); self.size],
            read_mask: self.read_mask,
            write_mask: self.write_mask,
            read_callback: self.read_callback,
            write_callback: self.write_callback,
        }
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.regs = vec![T::default(); size];
        self.size = size;
        self
    }

    pub fn with_write_mask(mut self, mask: T) -> Self {
        self.write_mask = Some(mask);
        self
    }
}

impl Registers<u32> {
    pub fn try_as_array_mut<const LEN: usize>(&mut self) -> Option<&mut [u32; LEN]> {
        self.regs.as_mut_slice().try_into().ok()
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for Registers<T> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.regs[index]
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for Registers<T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.regs[index]
    }
}

impl<T> From<Vec<T>> for Registers<T> {
    fn from(regs: Vec<T>) -> Self {
        Self {
            size: regs.len(),
            regs,
            read_mask: None,
            write_mask: None,
            read_callback: None,
            write_callback: None,
        }
    }
}

impl<T> AsRef<[T]> for Registers<T> {
    fn as_ref(&self) -> &[T] {
        &self.regs
    }
}

impl<T> AsMut<[T]> for Registers<T> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut self.regs
    }
}

impl<T> Registers<T>
where
    T: Default
        + Copy
        + std::ops::BitAnd<Output = T>
        + std::ops::BitOr<Output = T>
        + std::ops::Not<Output = T>
        + From<u32>
        + Into<u32>,
{
    pub fn read(&self, n64: &N64, address: u64, access_size: AccessSize) -> T {
        let reg = register_index(address);

        if reg >= self.size {
            return T::default();
        }

        let value = self.regs[reg];

        if let Some(callback) = self.read_callback {
            T::from(callback(n64, address, access_size))
        } else if let Some(mask) = self.read_mask {
            value & mask
        } else {
            value
        }
    }
}
