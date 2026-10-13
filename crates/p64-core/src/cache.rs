use super::memory::AccessSize;
use super::{cpu, memory, N64};

const WORD_SIZE: u64 = 4;
const DCACHE_FILL_CYCLES: u64 = 4;
const ICACHE_FILL_CYCLES: u64 = 8;
const ICACHE_INDEX_MASK: u16 = 0xFE0;
const DCACHE_INDEX_MASK: u16 = 0xFF0;
const CACHE_LINE_SIZE_ICACHE: usize = 8;
const ICACHE_LINE_BYTES: u64 = CACHE_LINE_SIZE_ICACHE as u64 * WORD_SIZE;
const CACHE_LINE_SIZE_DCACHE: usize = 4;
pub(crate) const TAG_PAGE_BYTES: u64 = 4096;
const ADDR_MASK: u32 = 0x1ffffffc;
const LINE_INDEX_SHIFT_ICACHE: u64 = 5;
const LINE_INDEX_MASK_ICACHE: u64 = 0x1FF;
const WORD_INDEX_SHIFT_ICACHE: u64 = 2;
const WORD_INDEX_MASK_ICACHE: u64 = 7;
const LINE_INDEX_SHIFT_DCACHE: u64 = 4;
const LINE_INDEX_MASK_DCACHE: u64 = 0x1FF;
const WORD_INDEX_SHIFT_DCACHE: u64 = 2;
const WORD_INDEX_MASK_DCACHE: u64 = 3;

mod types;
use types::FetchLatch;
pub use types::{DCache, FetchPipeline, ICache};

fn speculative_cached_address(n64: &N64, address: u64) -> Option<u64> {
    let (physical, cached) = memory::peek_translation(n64, address)?;
    // Speculative fetches must never read an MMIO device or a cartridge bus.
    (cached && physical < crate::rdram::RDRAM_SIZE as u64)
        .then_some(memory::reverse_endian_address(n64, physical, 4))
}

fn queue_fetch(n64: &mut N64, virtual_address: u64) {
    if n64.memory.fetch_pipeline.pending_refill.is_some() {
        return;
    }

    let Some(physical) = speculative_cached_address(n64, virtual_address) else {
        return;
    };

    let line_index = ICache::line_index(virtual_address);

    if !icache_hit(n64, line_index, physical) {
        n64.memory.fetch_pipeline.pending_refill = Some((virtual_address, physical));
    }
}

pub fn service_pending_fetch(n64: &mut N64) {
    if !n64.memory.fetch_pipeline.enabled || !n64.memory.fetch_pipeline.active {
        return;
    }

    let Some((virtual_address, physical)) = n64.memory.fetch_pipeline.pending_refill.take() else {
        return;
    };
    // Recheck after DC/CP0 operations; speculation never overrides permission or
    // mapping changes made by the instruction currently executing.
    if speculative_cached_address(n64, virtual_address) != Some(physical) {
        return;
    }

    let line_index = ICache::line_index(virtual_address);

    if !icache_hit(n64, line_index, physical) {
        icache_fill(n64, line_index, physical);
    }
}

fn begin_fetch_pipeline(n64: &mut N64) {
    n64.memory.fetch_pipeline.active = true;
    let next = if n64.cpu.branch_state.state == cpu::State::DelaySlotTaken {
        n64.cpu.branch_state.pc
    } else {
        n64.cpu.execution.pc.wrapping_add(4)
    };

    n64.memory.fetch_pipeline.next = speculative_cached_address(n64, next)
        .and_then(|physical| native::cached_pipeline_latch(n64, next, physical));
    // IC fetch is two instructions ahead of EX; miss service starts at RF.
    queue_fetch(n64, next);
    queue_fetch(n64, next.wrapping_add(4));
}

fn finish_fetch_pipeline(n64: &mut N64) {
    if n64.cpu.branch_state.state == cpu::State::Exception {
        n64.memory.fetch_pipeline.next = None;
        n64.memory.fetch_pipeline.pending_refill = None;
    } else {
        if n64.cpu.branch_state.state == cpu::State::Take {
            n64.memory.fetch_pipeline.pending_refill = None;
            queue_fetch(n64, n64.cpu.branch_state.pc);
        } else if n64.cpu.branch_state.state == cpu::State::Discard {
            n64.memory.fetch_pipeline.next = None;
        }

        service_pending_fetch(n64);
    }
    n64.memory.fetch_pipeline.active = false;
}

fn calculate_cache_address(tag: u32, index: u16) -> u64 {
    ((tag | index as u32) & ADDR_MASK) as u64
}

fn read_cache_line<const N: usize>(
    n64: &mut N64,
    cache_address: u64,
    access_size: AccessSize,
) -> [u32; N] {
    let read_fn = n64.memory.bus.memory_map_read[memory::MemoryBus::region_index(cache_address)];

    std::array::from_fn(|i| {
        let addr = cache_address | (i as u64 * WORD_SIZE);

        read_fn(n64, addr, access_size)
    })
}

fn write_cache_line(n64: &mut N64, cache_address: u64, words: &[u32]) {
    let write_fn = n64.memory.bus.memory_map_write[memory::MemoryBus::region_index(cache_address)];

    for (i, &word) in words.iter().enumerate() {
        let addr = cache_address | (i as u64 * WORD_SIZE);
        write_fn(n64, addr, word, u32::MAX);
    }
}

fn dcache_fill(n64: &mut N64, line_index: usize, phys_address: u64) {
    crate::mmu::add_cycles(n64, DCACHE_FILL_CYCLES);
    let tag = crate::bits::align_down(phys_address, TAG_PAGE_BYTES) as u32;
    let cache_address = calculate_cache_address(tag, n64.memory.caches.dcache[line_index].index);
    let words = read_cache_line::<CACHE_LINE_SIZE_DCACHE>(n64, cache_address, AccessSize::Dcache);

    let dcache_line = &mut n64.memory.caches.dcache[line_index];
    dcache_line.valid = true;
    dcache_line.store_dirty = false;
    dcache_line.tag = tag;
    dcache_line.words.copy_from_slice(&words);
}

pub fn icache_writeback(n64: &mut N64, line_index: usize) {
    crate::mmu::add_cycles(
        n64,
        crate::rdram::rdram_calculate_cycles(AccessSize::Icache as u64),
    );
    let tag = n64.memory.caches.icache[line_index].tag;
    let index = n64.memory.caches.icache[line_index].index;
    let words = n64.memory.caches.icache[line_index].words;

    let cache_address = calculate_cache_address(tag, index);
    write_cache_line(n64, cache_address, &words);
}

/// Returns true when a native block has already retired and advanced the PC.
pub fn icache_fetch(n64: &mut N64, phys_address: u64) -> bool {
    let line_index = ICache::line_index(n64.cpu.execution.pc);

    if n64.execution_engine.backend != crate::recompiler::Backend::Interpreter {
        let word_index = ICache::word_index(phys_address);
        let line = &n64.memory.caches.icache[line_index];

        if (line.native.is_candidate(word_index)
            || line.native.generation != n64.execution_engine.generation())
            && try_execute_native_block(n64, phys_address)
        {
            return true;
        }
    }

    let pipelined = n64.memory.fetch_pipeline.enabled;
    let fetched = if pipelined {
        n64.memory.fetch_pipeline.next.take().filter(|latch| {
            latch.virtual_address == n64.cpu.execution.pc && latch.physical_address == phys_address
        })
    } else {
        None
    };

    if let Some(fetched) = fetched {
        begin_fetch_pipeline(n64);
        cpu::execute_decoded_instruction(n64, fetched.word, fetched.instruction);
        finish_fetch_pipeline(n64);

        return false;
    }

    if !icache_hit(n64, line_index, phys_address) {
        icache_fill(n64, line_index, phys_address);
    }

    let word_index = ICache::word_index(phys_address);
    let instruction = n64.memory.caches.icache[line_index].instruction[word_index];
    let word = n64.memory.caches.icache[line_index].words[word_index];

    if pipelined {
        begin_fetch_pipeline(n64);
    }

    cpu::execute_decoded_instruction(n64, word, instruction);
    if pipelined {
        finish_fetch_pipeline(n64);
    }

    false
}

mod native;
use native::prepare_icache_line;
pub use native::try_execute_native_block;

pub fn icache_hit(n64: &mut N64, line_index: usize, phys_address: u64) -> bool {
    let cache_line = &n64.memory.caches.icache[line_index];

    cache_line.valid
        && (cache_line.tag & ADDR_MASK)
            == crate::bits::align_down(phys_address, TAG_PAGE_BYTES) as u32
}

pub fn dcache_hit(n64: &N64, line_index: usize, phys_address: u64) -> bool {
    let cache_line = &n64.memory.caches.dcache[line_index];
    let masked_tag = cache_line.tag & ADDR_MASK;
    let masked_addr = crate::bits::align_down(phys_address, TAG_PAGE_BYTES) as u32;

    cache_line.valid && masked_tag == masked_addr
}

pub fn dcache_writeback(n64: &mut N64, line_index: usize) {
    crate::mmu::add_cycles(
        n64,
        crate::rdram::rdram_calculate_cycles(AccessSize::Dcache as u64),
    );
    let cache_address = calculate_cache_address(
        n64.memory.caches.dcache[line_index].tag,
        n64.memory.caches.dcache[line_index].index,
    );
    let cached_words = n64.memory.caches.dcache[line_index].words;
    write_cache_line(n64, cache_address, &cached_words);
}

pub fn icache_fill(n64: &mut N64, line_index: usize, phys_address: u64) {
    crate::mmu::add_cycles(n64, ICACHE_FILL_CYCLES);
    let tag = crate::bits::align_down(phys_address, TAG_PAGE_BYTES) as u32;
    let cache_address = calculate_cache_address(tag, n64.memory.caches.icache[line_index].index);
    let words =
        read_cache_line::<CACHE_LINE_SIZE_ICACHE>(n64, cache_address, memory::AccessSize::Icache);

    let instructions = words.map(|opcode| cpu::decode_opcode(n64, opcode));

    n64.memory.clear_native_line(line_index);
    let icache_line = &mut n64.memory.caches.icache[line_index];
    icache_line.valid = true;
    icache_line.tag = tag;
    icache_line.words.copy_from_slice(&words);
    icache_line.native.reset(0, 0);
    icache_line.instruction = instructions;

    if n64.execution_engine.backend != crate::recompiler::Backend::Interpreter {
        prepare_icache_line(n64, line_index);
    }
}

pub fn dcache_read(n64: &mut N64, phys_address: u64) -> u32 {
    let line_index = DCache::line_index(n64.cpu.execution.data_address);
    let word_index = DCache::word_index(phys_address);

    let cache_line = &n64.memory.caches.dcache[line_index];

    if !dcache_hit(n64, line_index, phys_address) {
        if cache_line.valid && cache_line.store_dirty {
            dcache_writeback(n64, line_index);
        }

        dcache_fill(n64, line_index, phys_address);
    }

    n64.memory.caches.dcache[line_index].words[word_index]
}

pub fn dcache_write(n64: &mut N64, phys_address: u64, value: u32, mask: u32) {
    #[cfg(feature = "accuracy-diagnostics")]
    crate::rdram::trace_write(n64, "dcache", phys_address, value, mask);
    let line_index = DCache::line_index(n64.cpu.execution.data_address);
    let word_index = DCache::word_index(phys_address);

    let needs_fill = !dcache_hit(n64, line_index, phys_address);
    let needs_writeback = {
        let cache_line = &n64.memory.caches.dcache[line_index];

        needs_fill && cache_line.valid && cache_line.store_dirty
    };

    if needs_writeback {
        dcache_writeback(n64, line_index);
    }

    if needs_fill {
        dcache_fill(n64, line_index, phys_address);
    }

    let cache_line = &mut n64.memory.caches.dcache[line_index];
    memory::masked_write_32(&mut cache_line.words[word_index], value, mask);
    cache_line.dirty = true;
    cache_line.store_dirty = true;
}

pub fn init(n64: &mut N64) {
    for (pos, i) in n64.memory.caches.icache.iter_mut().enumerate() {
        i.index = (pos << LINE_INDEX_SHIFT_ICACHE) as u16 & ICACHE_INDEX_MASK
    }

    for (pos, i) in n64.memory.caches.dcache.iter_mut().enumerate() {
        i.index = (pos << LINE_INDEX_SHIFT_DCACHE) as u16 & DCACHE_INDEX_MASK
    }
}
