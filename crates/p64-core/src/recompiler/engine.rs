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

    pub fn precompile(&mut self, opcode: u32) -> bool {
        if self.code.cache.contains_key(&opcode) {
            return true;
        }

        if !self.code.compilation_available || self.code.cache.len() >= MAX_SCALAR_RECORDS {
            return false;
        }
        #[cfg(target_arch = "x86_64")]
        if let Some(code) = compile_opcode(opcode) {
            return self.install_code(opcode, &code);
        }

        false
    }

    #[cfg(target_arch = "x86_64")]
    fn install_code(&mut self, opcode: u32, code: &[u8]) -> bool {
        if self.code.cache.contains_key(&opcode) {
            return true;
        }

        let Some(function) = self.install_integer_leaf(code) else {
            return false;
        };

        self.code.cache.insert(opcode, function);

        true
    }

    fn install_integer_leaf(&mut self, code: &[u8]) -> Option<NativeFunction> {
        let pointer = self.install_native(code)?;
        // SAFETY: Both private callers supply trusted GPR-ABI emitter results;
        // installation seals the owned mapping before exposing its entry.
        Some(unsafe { std::mem::transmute::<*mut u8, NativeFunction>(pointer) })
    }

    fn install_native(&mut self, code: &[u8]) -> Option<*mut u8> {
        #[cfg(target_arch = "x86_64")]
        {
            if code.is_empty() || code.len() > ExecutablePage::SIZE {
                return None;
            }

            if self
                .code
                .pages
                .last()
                .is_none_or(|page| !page.fits(code.len()))
            {
                if self.code.pages.len() >= self.code.page_limit {
                    self.code.compilation_available = false;
                    return None;
                }

                let Some(page) = ExecutablePage::new() else {
                    self.code.compilation_available = false;
                    return None;
                };
                self.code.pages.push(page);
            }

            let Some(pointer) = self.code.pages.last_mut().unwrap().insert(code) else {
                // A failed RW -> RX transition can invalidate earlier entries in this page.
                self.code.cache.clear();
                self.code.blocks.clear();
                self.code.branch_blocks.clear();
                self.code.memory_blocks.clear();
                self.code.generation = next_generation();
                self.backend = Backend::Interpreter;
                self.code.compilation_available = false;

                return None;
            };

            Some(pointer)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = code;

            None
        }
    }
}
