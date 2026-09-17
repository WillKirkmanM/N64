#[cfg(target_arch = "x86_64")]
use super::super::{ExecutablePage, MAX_NATIVE_BYTES};
use super::super::{next_generation, NativeBranchFunction, NativeFunction, NativeMemoryFunction};
use std::collections::HashMap;

pub(in crate::recompiler) struct NativeCodeCache {
    pub(in crate::recompiler) cache: HashMap<u32, NativeFunction>,
    pub(in crate::recompiler) blocks: HashMap<Vec<u32>, NativeFunction>,
    pub(in crate::recompiler) branch_blocks: HashMap<Vec<u32>, NativeBranchFunction>,
    pub(in crate::recompiler) memory_blocks: HashMap<Vec<u32>, NativeMemoryFunction>,
    pub(in crate::recompiler) generation: u64,
    pub(in crate::recompiler) compilation_available: bool,
    #[cfg(target_arch = "x86_64")]
    pub(in crate::recompiler) page_limit: usize,
    #[cfg(target_arch = "x86_64")]
    pub(in crate::recompiler) pages: Vec<ExecutablePage>,
}

impl Default for NativeCodeCache {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            blocks: HashMap::new(),
            branch_blocks: HashMap::new(),
            memory_blocks: HashMap::new(),
            generation: next_generation(),
            compilation_available: true,
            #[cfg(target_arch = "x86_64")]
            page_limit: MAX_NATIVE_BYTES / ExecutablePage::SIZE,
            #[cfg(target_arch = "x86_64")]
            pages: Vec::new(),
        }
    }
}
