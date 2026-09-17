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
    pub fn interpreter() -> Self {
        Self {
            backend: Backend::Interpreter,
            code: NativeCodeCache::default(),
            stats: ExecutionStats::default(),
        }
    }
}
