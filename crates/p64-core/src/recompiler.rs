//! Native code is keyed by fetched opcodes. The CPU dispatcher handles timing,
//! privilege checks and exceptions.
mod emitter;

#[cfg(target_arch = "x86_64")]
mod executable;
#[cfg(target_arch = "x86_64")]
use executable::ExecutablePage;

use crate::opcode::{PrimaryOpcode, SpecialOpcode};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_BLOCK_INSTRUCTIONS: usize = 8;
#[cfg(target_arch = "x86_64")]
const MAX_NATIVE_BYTES: usize = 128 * 1024 * 1024;
pub(crate) const MAX_SCALAR_RECORDS: usize = 65_536;
pub(crate) const MAX_BLOCK_RECORDS: usize = 262_144;

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

fn next_generation() -> u64 {
    NEXT_GENERATION
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .expect("Native engine generation exhausted")
}

mod engine;
mod types;
pub use engine::{Engine, ExecutionStats};
pub(crate) use types::NativeCodeRecord;
pub use types::NativeReject;
pub use types::{
    Backend, NativeBlock, NativeBranchBlock, NativeInstruction, NativeMemoryBlock,
    NativeMemoryContext, NativeMemoryExit,
};
use types::{BlockMetadata, NativeBranchFunction, NativeFunction, NativeMemoryFunction};
