use super::*;

#[path = "code_cache.rs"]
mod code_cache;
#[path = "statistics.rs"]
mod statistics;
use code_cache::NativeCodeCache;
pub use statistics::ExecutionStats;

pub struct Engine {
    pub backend: Backend,
    pub(super) code: NativeCodeCache,
    pub stats: ExecutionStats,
}

impl Default for Engine {
    fn default() -> Self {
        Self::interpreter()
    }
}

impl Engine {
    fn accepts_block(&self, opcodes: &[u32]) -> bool {
        self.backend != Backend::Interpreter
            && (2..=MAX_BLOCK_INSTRUCTIONS).contains(&opcodes.len())
    }

    fn can_compile_block(&self) -> bool {
        self.backend == Backend::Jit
            && self.code.compilation_available
            && self.compiled_blocks() < MAX_BLOCK_RECORDS
    }

    #[inline]
    fn owns_generation(&self, generation: u64) -> bool {
        generation == self.code.generation && self.backend != Backend::Interpreter
    }

    pub fn interpreter() -> Self {
        Self {
            backend: Backend::Interpreter,
            code: NativeCodeCache::default(),
            stats: ExecutionStats::default(),
        }
    }

    pub fn jit() -> Self {
        Self {
            backend: Backend::Jit,
            ..Self::interpreter()
        }
    }

    pub fn compiled_instructions(&self) -> usize {
        self.code.cache.len()
    }

    pub fn compiled_blocks(&self) -> usize {
        self.code.blocks.len() + self.code.branch_blocks.len() + self.code.memory_blocks.len()
    }

    pub fn compiled_branch_blocks(&self) -> usize {
        self.code.branch_blocks.len()
    }

    pub fn compiled_memory_blocks(&self) -> usize {
        self.code.memory_blocks.len()
    }

    /// Unique across engines, including replacing an engine at the same address.
    /// Also changes if an executable-memory error invalidates existing code.
    pub fn generation(&self) -> u64 {
        self.code.generation
    }
}
