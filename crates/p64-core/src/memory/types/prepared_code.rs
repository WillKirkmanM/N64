use crate::recompiler::{NativeBlock, NativeBranchBlock, NativeMemoryBlock};

const CACHE_LINE_COUNT: usize = 512;
const INSTRUCTIONS_PER_LINE: usize = 8;

type EntryHandles<T> = Vec<[Option<T>; INSTRUCTIONS_PER_LINE]>;

pub struct PreparedCode {
    pub native_blocks: EntryHandles<NativeBlock>,
    pub native_prefix_blocks: EntryHandles<NativeBlock>,
    pub native_branch_blocks: EntryHandles<NativeBranchBlock>,
    pub native_memory_blocks: EntryHandles<NativeMemoryBlock>,
    pub native_memory_prefix_blocks: EntryHandles<NativeMemoryBlock>,
}

impl Default for PreparedCode {
    fn default() -> Self {
        Self {
            native_blocks: vec![[None; INSTRUCTIONS_PER_LINE]; CACHE_LINE_COUNT],
            native_prefix_blocks: vec![[None; INSTRUCTIONS_PER_LINE]; CACHE_LINE_COUNT],
            native_branch_blocks: vec![[None; INSTRUCTIONS_PER_LINE]; CACHE_LINE_COUNT],
            native_memory_blocks: vec![[None; INSTRUCTIONS_PER_LINE]; CACHE_LINE_COUNT],
            native_memory_prefix_blocks: vec![[None; INSTRUCTIONS_PER_LINE]; CACHE_LINE_COUNT],
        }
    }
}

impl PreparedCode {
    pub fn clear_line(&mut self, line_index: usize) {
        self.native_blocks[line_index] = [None; INSTRUCTIONS_PER_LINE];
        self.native_prefix_blocks[line_index] = [None; INSTRUCTIONS_PER_LINE];
        self.native_branch_blocks[line_index] = [None; INSTRUCTIONS_PER_LINE];
        self.native_memory_blocks[line_index] = [None; INSTRUCTIONS_PER_LINE];
        self.native_memory_prefix_blocks[line_index] = [None; INSTRUCTIONS_PER_LINE];
    }
}
