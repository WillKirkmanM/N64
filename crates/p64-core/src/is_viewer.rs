use crate::{
    memory::{masked_write_32, AccessSize},
    N64,
};
use std::io::Write;

const BUFFER_BYTES: usize = 0x10000;
const ADDRESS_MASK: u64 = (BUFFER_BYTES - 1) as u64;
const WORD_BYTES: usize = std::mem::size_of::<u32>();
const COMMIT_OFFSET: usize = 0x14;
const TEXT_OFFSET: usize = 0x20;

mod types;
pub use types::IsViewer;

pub fn read_mem(n64: &mut N64, address: u64, _: AccessSize) -> u32 {
    let offset = (address & ADDRESS_MASK) as usize;

    n64.is_viewer
        .buffer
        .get(offset..offset + WORD_BYTES)
        .map(|bytes| u32::from_be_bytes(bytes.try_into().unwrap()))
        .unwrap_or(0)
}

pub fn write_mem(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let offset = (address & ADDRESS_MASK) as usize;

    if offset == COMMIT_OFFSET {
        let length = (value & mask) as usize;

        if let Some(bytes) = n64
            .is_viewer
            .buffer
            .get(TEXT_OFFSET..TEXT_OFFSET.saturating_add(length))
        {
            let output = String::from_utf8_lossy(bytes);
            print!("{output}");
            let _ = std::io::stdout().flush();
            n64.is_viewer.transcript.push_str(&output);
            n64.is_viewer.completed = (n64.is_viewer.transcript.contains("Finished in")
                && n64.is_viewer.transcript.contains("success rate)"))
                || n64.is_viewer.transcript.contains("Done! Tests:");

            if n64.is_viewer.stop_on_completion && n64.is_viewer.completed {
                n64.cpu_mut().execution.running = 0;
            }
        }

        return;
    }

    if let Some(bytes) = n64.is_viewer.buffer.get_mut(offset..offset + WORD_BYTES) {
        let mut word = u32::from_be_bytes(bytes.try_into().unwrap());
        masked_write_32(&mut word, value, mask);
        bytes.copy_from_slice(&word.to_be_bytes());
    }
}

#[cfg(test)]
mod tests;
