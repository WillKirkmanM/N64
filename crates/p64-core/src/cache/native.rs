use super::types::NativeEntryPlan;
use super::{icache_hit, speculative_cached_address, FetchLatch, ICache, ADDR_MASK, ICACHE_LINE_BYTES, TAG_PAGE_BYTES};
use crate::{cpu, N64};

fn native_entry_plan(words: &[u32; 8], start: usize, minimum: usize) -> NativeEntryPlan {
    if start >= 7 {
        return NativeEntryPlan::default();
    }

    let length = words[start..]
        .iter()
        .take_while(|&&opcode| crate::recompiler::supported_opcode(opcode))
        .count();
    let branch_length = length + 1;
    let fused_length =
        branch_length + usize::from(start + branch_length < 8 && words[start + branch_length] == 0);
    let branch = branch_length >= 2
        && fused_length >= minimum
        && start + length < 8
        && crate::recompiler::supported_branch_opcode(words[start + length]);
    let mixed_length = words[start..]
        .iter()
        .take_while(|&&opcode| {
            crate::recompiler::supported_opcode(opcode)
                || crate::recompiler::supported_memory_opcode(opcode)
        })
        .count();
    let mixed_length = if mixed_length >= minimum
        && words[start..start + mixed_length]
            .iter()
            .any(|opcode| crate::recompiler::supported_memory_opcode(*opcode))
    {
        mixed_length
    } else {
        0
    };

    NativeEntryPlan {
        integer_length: if length >= minimum { length } else { 0 },
        memory_length: mixed_length,
        branch_length: branch.then_some(branch_length),
    }
}

pub(super) fn prepare_icache_line(n64: &mut N64, line_index: usize) {
    let words = n64.memory.caches.icache[line_index].words;
    let minimum = n64.memory.fetch_pipeline.native_min_block_length;
    let mut candidates = 0u8;

    if n64.execution_engine.backend != crate::recompiler::Backend::Interpreter {
        for start in 0..7 {
            let plan = native_entry_plan(&words, start, minimum);

            if plan.is_candidate() {
                candidates |= 1 << start;
            }
        }
    }

    let line = &mut n64.memory.caches.icache[line_index];
    line.native
        .reset(n64.execution_engine.generation(), candidates);
    n64.memory.clear_native_line(line_index);
}

fn prepare_icache_entry(n64: &mut N64, line_index: usize, start: usize) {
    if n64.memory.caches.icache[line_index]
        .native
        .was_prepared(start)
    {
        return;
    }

    let words = n64.memory.caches.icache[line_index].words;
    let minimum = n64.memory.fetch_pipeline.native_min_block_length;
    let plan = native_entry_plan(&words, start, minimum);
    let length = plan.integer_length;
    let mixed_length = plan.memory_length;

    if length != 0 {
        n64.memory.prepared_code.native_blocks[line_index][start] = n64
            .execution_engine
            .prepare_block(&words[start..start + length]);
        // Preserve the prefix whose two speculative fetches stay in this line.
        if start + minimum <= 6 && start + length > 6 {
            n64.memory.prepared_code.native_prefix_blocks[line_index][start] =
                n64.execution_engine.prepare_block(&words[start..6]);
        }
    }

    if let Some(branch_length) = plan.branch_length {
        n64.memory.prepared_code.native_branch_blocks[line_index][start] = n64
            .execution_engine
            .prepare_branch_block(&words[start..start + branch_length]);
    }

    if mixed_length != 0 {
        n64.memory.prepared_code.native_memory_blocks[line_index][start] = n64
            .execution_engine
            .prepare_memory_block(&words[start..start + mixed_length]);

        if start + minimum <= 6
            && start + mixed_length > 6
            && words[start..6]
                .iter()
                .any(|opcode| crate::recompiler::supported_memory_opcode(*opcode))
        {
            n64.memory.prepared_code.native_memory_prefix_blocks[line_index][start] =
                n64.execution_engine.prepare_memory_block(&words[start..6]);
        }
    }

    if n64.memory.caches.icache[line_index].native.generation != n64.execution_engine.generation() {
        // A W^X protection failure invalidates the engine's earlier handles.
        prepare_icache_line(n64, line_index);
        return;
    }

    n64.memory.caches.icache[line_index]
        .native
        .mark_prepared(start);

    if n64.memory.prepared_code.native_blocks[line_index][start].is_none()
        && n64.memory.prepared_code.native_branch_blocks[line_index][start].is_none()
        && n64.memory.prepared_code.native_memory_blocks[line_index][start].is_none()
        && n64.memory.prepared_code.native_memory_prefix_blocks[line_index][start].is_none()
    {
        n64.memory.caches.icache[line_index]
            .native
            .discard_candidate(start);
    }
}

fn cached_pipeline_word(n64: &N64, virtual_address: u64, physical: u64) -> Option<FetchLatch> {
    if speculative_cached_address(n64, virtual_address) != Some(physical) {
        return None;
    }

    cached_pipeline_latch(n64, virtual_address, physical)
}

#[inline]
pub(super) fn cached_pipeline_latch(
    n64: &N64,
    virtual_address: u64,
    physical: u64,
) -> Option<FetchLatch> {
    let index = ICache::line_index(virtual_address);
    let line = &n64.memory.caches.icache[index];

    if !line.valid
        || line.tag & ADDR_MASK != crate::bits::align_down(physical, TAG_PAGE_BYTES) as u32
    {
        return None;
    }

    let word = ICache::word_index(physical);

    Some(FetchLatch {
        virtual_address,
        physical_address: physical,
        word: line.words[word],
        instruction: line.instruction[word],
    })
}

#[inline]
fn block_lookahead_is_cached(n64: &N64, physical: u64, length: usize) -> bool {
    if !n64.memory.fetch_pipeline.enabled {
        return true;
    }

    let highest = (length as u64 + 1) * 4;
    // Integer blocks cannot change mapping or privilege within this checked cache line.
    if crate::bits::alignment_offset(physical, ICACHE_LINE_BYTES) + highest < ICACHE_LINE_BYTES {
        return true;
    }

    let Some(virtual_address) = n64.cpu.execution.pc.checked_add(highest) else {
        return false;
    };

    let Some(physical_address) = physical.checked_add(highest) else {
        return false;
    };
    // Both lookahead words share a line and page; checking the last covers both.
    cached_pipeline_word(n64, virtual_address, physical_address).is_some()
}

#[inline]
fn reject_cached_native(n64: &mut N64, reason: crate::recompiler::NativeReject) -> bool {
    n64.execution_engine.record_native_reject(reason);

    false
}

fn finish_native_fetch(n64: &mut N64, physical: u64, length: usize) {
    if n64.memory.fetch_pipeline.enabled {
        n64.memory.fetch_pipeline.next =
            cached_pipeline_latch(n64, n64.cpu.execution.pc, physical + length as u64 * 4);
    }
    n64.cpu.execution.pc_phys = physical + (length as u64 - 1) * 4;
}
