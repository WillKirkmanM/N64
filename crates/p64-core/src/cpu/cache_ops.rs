use super::{effective_address, instruction::Instruction, translate_aligned_address};
use crate::{cache, memory, mmu::MmuRegister, N64};

const DATA_LINE_SHIFT: u64 = 4;
const INSTRUCTION_LINE_SHIFT: u64 = 5;

mod types;
use types::{CacheAddress, CacheBank, CacheOperation, CacheSelector};

pub(super) fn cache(n64: &mut N64, opcode: u32) {
    if !n64.cp0().status().cp0_usable() {
        crate::exceptions::cop_unusable_exception(n64, 0);
        return;
    }

    let virtual_address = effective_address(n64, opcode);
    let (physical, _, error) =
        translate_aligned_address(n64, virtual_address, memory::AccessType::Read, 4);

    if error {
        return;
    }

    let encoding = Instruction::decode(opcode).rt;
    let Some(selector) = CacheSelector::decode(encoding) else {
        return;
    };

    match selector.bank {
        CacheBank::Instruction => instruction_cache(
            n64,
            CacheAddress::new(physical, virtual_address, INSTRUCTION_LINE_SHIFT),
            selector.operation,
        ),
        CacheBank::Data => data_cache(
            n64,
            CacheAddress::new(physical, virtual_address, DATA_LINE_SHIFT),
            selector.operation,
        ),
    }
}

fn instruction_cache(n64: &mut N64, address: CacheAddress, operation: CacheOperation) {
    let index = address.index;

    match operation {
        CacheOperation::IndexInvalidate => {
            let line = &mut n64.memory.caches.icache[index];
            line.valid = false;
            line.tag = address.page_tag();
        }
        CacheOperation::IndexLoadTag => {
            n64.set_cp0_register(
                MmuRegister::TagLo,
                n64.memory.caches.icache[index].cp0_tag(),
            );
        }
        CacheOperation::IndexStoreTag => {
            let tag = n64.cp0_register(MmuRegister::TagLo);
            n64.memory.caches.icache[index].set_cp0_tag(tag);
        }
        CacheOperation::HitInvalidate => {
            if cache::icache_hit(n64, index, address.physical) {
                n64.memory.caches.icache[index].valid = false;
            }
        }
        CacheOperation::FillOrWritebackInvalidate => {
            cache::icache_fill(n64, index, address.physical)
        }
        CacheOperation::HitWriteback => {
            if cache::icache_hit(n64, index, address.physical) {
                cache::icache_writeback(n64, index);
            }
        }
        CacheOperation::CreateDirtyExclusive => unreachable!("data-cache-only operation"),
    }
}

fn writeback_dirty_data(n64: &mut N64, index: usize) {
    let line = &n64.memory.caches.dcache[index];

    if line.valid && line.store_dirty {
        cache::dcache_writeback(n64, index);
    }
}

fn data_cache(n64: &mut N64, address: CacheAddress, operation: CacheOperation) {
    let index = address.index;

    match operation {
        CacheOperation::IndexInvalidate => {
            writeback_dirty_data(n64, index);
            let line = &mut n64.memory.caches.dcache[index];

            if line.valid {
                line.tag = address.page_tag();
            }
            line.valid = false;
        }
        CacheOperation::IndexLoadTag => {
            n64.set_cp0_register(
                MmuRegister::TagLo,
                n64.memory.caches.dcache[index].cp0_tag(),
            );
        }
        CacheOperation::IndexStoreTag => {
            let tag = n64.cp0_register(MmuRegister::TagLo);
            n64.memory.caches.dcache[index].set_cp0_tag(tag);
        }
        CacheOperation::CreateDirtyExclusive => {
            if !cache::dcache_hit(n64, index, address.physical) {
                writeback_dirty_data(n64, index);
                n64.memory.caches.dcache[index].store_dirty = false;
            }

            let line = &mut n64.memory.caches.dcache[index];
            line.tag = address.page_tag();
            line.valid = true;
            line.dirty = true;
        }
        CacheOperation::HitInvalidate => {
            if cache::dcache_hit(n64, index, address.physical) {
                n64.memory.caches.dcache[index].valid = false;
            }
        }
        CacheOperation::FillOrWritebackInvalidate => {
            if cache::dcache_hit(n64, index, address.physical) {
                writeback_dirty_data(n64, index);
                n64.memory.caches.dcache[index].valid = false;
            }
        }
        CacheOperation::HitWriteback => {
            if cache::dcache_hit(n64, index, address.physical)
                && n64.memory.caches.dcache[index].store_dirty
            {
                cache::dcache_writeback(n64, index);
                n64.memory.caches.dcache[index].store_dirty = false;
            }
        }
    }
}
