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

    pub fn aot(opcodes: impl IntoIterator<Item = u32>) -> Self {
        let mut engine = Self {
            backend: Backend::Aot,
            ..Self::interpreter()
        };

        for opcode in opcodes {
            engine.precompile(opcode);
        }

        engine
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

    #[inline]
    pub fn record_native_reject(&mut self, reason: NativeReject) {
        self.stats.record_reject(reason);
    }

    /// Unique across engines, including replacing an engine at the same address.
    /// Also changes if an executable-memory error invalidates existing code.
    pub fn generation(&self) -> u64 {
        self.code.generation
    }

    /// Validate every record against the local emitter before allocating executable memory.
    pub fn from_aot_codes(codes: Vec<(u32, Vec<u8>)>) -> Result<Self, String> {
        if codes.len() > MAX_SCALAR_RECORDS {
            return Err("AOT instruction limit exceeded".into());
        }

        for (opcode, bytes) in &codes {
            if compile_opcode(*opcode).as_ref() != Some(bytes) {
                return Err(format!("Invalid or incompatible AOT code for {opcode:08x}"));
            }
        }

        Self::from_validated_aot_records(
            codes
                .into_iter()
                .map(|(opcode, bytes)| (vec![opcode], bytes))
                .collect(),
        )
    }

    fn from_validated_aot_records(codes: Vec<NativeCodeRecord>) -> Result<Self, String> {
        let engine = Self::aot(std::iter::empty());
        #[cfg(target_arch = "x86_64")]
        let mut engine = engine;
        #[cfg(target_arch = "x86_64")]
        {
            for (opcodes, bytes) in codes {
                let scalar = opcodes.len() == 1;
                let branch = opcodes
                    .last()
                    .is_some_and(|opcode| supported_branch_opcode(*opcode));
                let memory = opcodes
                    .iter()
                    .any(|opcode| supported_memory_opcode(*opcode));

                if (scalar && engine.code.cache.contains_key(&opcodes[0]))
                    || (!scalar
                        && !branch
                        && !memory
                        && engine.code.blocks.contains_key(opcodes.as_slice()))
                    || (branch && engine.code.branch_blocks.contains_key(opcodes.as_slice()))
                    || (memory && engine.code.memory_blocks.contains_key(opcodes.as_slice()))
                {
                    continue;
                }

                if bytes.is_empty() || bytes.len() > ExecutablePage::SIZE {
                    return Err("Invalid AOT native record size".into());
                }

                if engine
                    .code
                    .pages
                    .last()
                    .is_none_or(|page| !page.fits(bytes.len()))
                {
                    if let Some(page) = engine.code.pages.last_mut() {
                        if !page.seal() {
                            return Err("Native executable protection failed".into());
                        }
                    }

                    if engine.code.pages.len() >= MAX_NATIVE_BYTES / ExecutablePage::SIZE {
                        return Err("Native executable memory limit exceeded".into());
                    }

                    let page =
                        ExecutablePage::new().ok_or("Native executable allocation failed")?;
                    engine.code.pages.push(page);
                }

                let pointer = engine
                    .code
                    .pages
                    .last_mut()
                    .unwrap()
                    .insert_unsealed(&bytes)
                    .ok_or("Native AOT page packing failed")?;
                // SAFETY: Validated emitter bytes stay private until every page is sealed RX.
                if memory {
                    let function =
                        unsafe { std::mem::transmute::<*mut u8, NativeMemoryFunction>(pointer) };
                    engine.code.memory_blocks.insert(opcodes, function);
                } else if branch {
                    let function =
                        unsafe { std::mem::transmute::<*mut u8, NativeBranchFunction>(pointer) };
                    engine.code.branch_blocks.insert(opcodes, function);
                } else {
                    let function =
                        unsafe { std::mem::transmute::<*mut u8, NativeFunction>(pointer) };

                    if scalar {
                        engine.code.cache.insert(opcodes[0], function);
                    } else {
                        engine.code.blocks.insert(opcodes, function);
                    }
                }
            }

            if let Some(page) = engine.code.pages.last_mut() {
                if !page.seal() {
                    return Err("Native executable protection failed".into());
                }
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        if !codes.is_empty() {
            return Err("Native AOT is unavailable on this architecture".into());
        }

        Ok(engine)
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

    fn install_block(&mut self, opcodes: &[u32], code: &[u8]) -> bool {
        if self.code.blocks.contains_key(opcodes) {
            return true;
        }

        let Some(function) = self.install_integer_leaf(code) else {
            return false;
        };

        self.code.blocks.insert(opcodes.to_vec(), function);

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

    #[inline]
    pub fn execute_prepared_block(&mut self, block: NativeBlock, gpr: &mut [u64; 32]) -> bool {
        if !self.owns_generation(block.generation) {
            return false;
        }
        // SAFETY: The generation check confirms this engine still owns the RX
        // mapping; its trusted emitter accesses only the borrowed 32 GPRs.
        unsafe {
            (block.function)(gpr.as_mut_ptr());
        }
        self.stats.record_block(block.metadata.instruction_count);

        true
    }

    /// Updates GPRs and compares the branch; the caller handles retirement and delay state.
    #[inline]
    pub fn execute_prepared_branch_block(
        &mut self,
        block: NativeBranchBlock,
        gpr: &mut [u64; 32],
    ) -> Option<bool> {
        if !self.owns_generation(block.generation) {
            return None;
        }
        // SAFETY: The matching generation keeps the RX mapping alive, and the
        // trusted branch emitter uses only this live GPR array and returns 0/1.
        let taken = unsafe { (block.function)(gpr.as_mut_ptr()) } != 0;
        self.stats
            .record_branch_block(block.metadata.instruction_count);

        Some(taken)
    }

    /// Only JIT compiles missing entries; AOT misses return None for scalar fallback.
    pub fn prepare(&mut self, opcode: u32) -> Option<NativeInstruction> {
        if self.backend == Backend::Interpreter {
            return None;
        }

        let function = self.code.cache.get(&opcode).copied().or_else(|| {
            if self.backend == Backend::Jit && supported_opcode(opcode) && self.precompile(opcode) {
                self.code.cache.get(&opcode).copied()
            } else {
                None
            }
        });

        function.map(|function| NativeInstruction {
            function,
            generation: self.code.generation,
        })
    }

    /// Reject handles invalidated by engine replacement or executable-memory failure.
    #[inline]
    pub fn execute_prepared(
        &mut self,
        instruction: NativeInstruction,
        gpr: &mut [u64; 32],
    ) -> bool {
        if !self.owns_generation(instruction.generation) {
            self.stats.fallback_executions += 1;

            return false;
        }
        // SAFETY: The generation check keeps the sealed mapping live; the leaf uses only gpr.
        unsafe {
            (instruction.function)(gpr.as_mut_ptr());
        }
        self.stats.native_executions += 1;

        true
    }

    /// Returns false without changing registers for unsupported instructions.
    pub fn try_execute(&mut self, opcode: u32, gpr: &mut [u64; 32]) -> bool {
        if let Some(instruction) = self.prepare(opcode) {
            self.execute_prepared(instruction, gpr)
        } else {
            self.stats.fallback_executions += 1;

            false
        }
    }
}
