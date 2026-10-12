use crate::cpu;

use super::{
    cpu::{in_delay_slot, State},
    memory::{self, masked_write_64, AccessType},
    mmu::{
        add_cycles, MmuCause, MmuRegister, MmuStatus,
        MmuXContext::{BadVPN2Mask, RegionMask},
    },
    N64,
};

const GENERAL_VECTOR_OFFSET: u32 = 0x180;
const EXTENDED_REFILL_VECTOR_OFFSET: u32 = 0x80;
const CONTEXT_VPN_MASK: u64 = 0x007F_FFF0;
const ENTRY_HI_ASID_MASK: u64 = 0xFF;
const COPROCESSOR_CAUSE_MASK: u64 = MmuCause::CE1 as u64 | MmuCause::CE2 as u64;
const CONTEXT_VPN_SHIFT: u32 = 9;
const XCONTEXT_REGION_SHIFT: u32 = 31;
const EXCEPTION_STAGE_CYCLES: u64 = 2;
const NORMAL_VECTOR_BASE: u32 = 0x8000_0000;
const BOOT_VECTOR_BASE: u32 = 0xBFC0_0200;
const INSTRUCTION_BYTES: u64 = std::mem::size_of::<u32>() as u64;

#[cfg(feature = "accuracy-diagnostics")]
mod diagnostics;

fn interrupt_pending(n64: &N64) -> bool {
    n64.cp0()
        .status()
        .interrupt_pending(n64.cp0_register(MmuRegister::Cause))
}

pub fn check_pending_interrupts(n64: &mut N64) {
    if interrupt_pending(n64) {
        set_cause_and_exception(n64, 0);
    }
}

pub fn schedule_pending_interrupts(n64: &mut N64) {
    use crate::events::{create_event, remove_event, set_next_event, EventType};

    if interrupt_pending(n64) {
        create_event(
            n64,
            EventType::InterruptCheck,
            n64.cp0_register(MmuRegister::Count) + 1,
            check_pending_interrupts,
        );
    } else {
        remove_event(n64, EventType::InterruptCheck);
        set_next_event(n64);
    }
}

pub fn floating_point_exception(n64: &mut N64) {
    set_cause_and_exception(n64, MmuCause::ExccodeFPE as u64);
}

pub fn overflow_exception(n64: &mut N64) {
    set_cause_and_exception(n64, MmuCause::ExccodeOv as u64)
}

pub fn trap_exception(n64: &mut N64) {
    set_cause_and_exception(n64, MmuCause::ExccodeTR as u64)
}

pub fn syscall_exception(n64: &mut N64) {
    set_cause_and_exception(n64, MmuCause::ExccodeSys as u64)
}

pub fn break_exception(n64: &mut N64) {
    set_cause_and_exception(n64, MmuCause::ExccodeBP as u64)
}

pub fn reserved_exception(n64: &mut N64, cop: u64) {
    set_cause_and_exception(n64, (MmuCause::ExccodeRI as u64) | cop)
}

#[cfg(feature = "accuracy-diagnostics")]
pub(crate) fn trace_reserved(n64: &N64, opcode: u32) {
    diagnostics::trace_reserved(n64, opcode);
}

pub fn cop_unusable_exception(n64: &mut N64, cop: u64) {
    set_cause_and_exception(n64, (MmuCause::ExccodeCPU as u64) | cop);
}

pub fn address_error_exception(n64: &mut N64, address: u64, access_type: AccessType) {
    add_cycles(n64, EXCEPTION_STAGE_CYCLES);
    record_fault_address(n64, address);
    let cause = if access_type == AccessType::Read {
        MmuCause::ExccodeAdEL as u64
    } else {
        MmuCause::ExccodeAdES as u64
    };

    set_cause_and_exception(n64, cause);
}

pub fn tlb_miss_exception(n64: &mut N64, address: u64, access_type: memory::AccessType) {
    add_cycles(n64, EXCEPTION_STAGE_CYCLES);
    let mut cause = if access_type == memory::AccessType::Read {
        MmuCause::ExccodeTLBL as u64
    } else {
        MmuCause::ExccodeTLBS as u64
    };

    record_fault_address(n64, address);

    let matched = crate::tlb::lookup_entry(n64, address);
    let mut vector_offset = GENERAL_VECTOR_OFFSET;

    if let Some((entry, odd)) = matched {
        let page = entry.page(odd);

        if page.valid && !page.dirty && access_type == AccessType::Write {
            cause = MmuCause::ExccodeMod as u64;
        }
    } else if !n64.cp0().status().exception_level() {
        vector_offset = if n64.cp0().status().extended_addressing() {
            EXTENDED_REFILL_VECTOR_OFFSET
        } else {
            0
        };
    }

    set_cause(n64, cause);
    exception_general(n64, vector_offset);
}

fn record_fault_address(n64: &mut N64, address: u64) {
    n64.set_cp0_register(MmuRegister::BadVAddr, address);
    update_context_registers(n64, address);
    masked_write_64(
        n64.cp0_mut().register_mut(MmuRegister::EntryHi),
        address,
        crate::bits::replace_bits(crate::mmu::MmuMask::EntryHi as u64, 0, ENTRY_HI_ASID_MASK),
    );
}

fn set_cause(n64: &mut N64, cause: u64) {
    #[cfg(feature = "accuracy-diagnostics")]
    diagnostics::trace(n64, cause);
    // Exception entry updates ExcCode and CE without dropping pending interrupts
    // or the BD bit saved by an earlier exception while EXL is set.
    let mask = MmuCause::ExccodeMask as u64 | COPROCESSOR_CAUSE_MASK;
    masked_write_64(n64.cp0_mut().register_mut(MmuRegister::Cause), cause, mask);
}

fn set_cause_and_exception(n64: &mut N64, cause: u64) {
    set_cause(n64, cause);
    exception_general(n64, GENERAL_VECTOR_OFFSET);
}

fn update_context_registers(n64: &mut N64, address: u64) {
    masked_write_64(
        &mut n64.cpu.mmu.registers.regs[MmuRegister::Context as usize],
        address >> 9,
        BadVPN2Mask as u64,
    );
    masked_write_64(
        &mut n64.cpu.mmu.registers.regs[MmuRegister::XContext as usize],
        address >> 9,
        BadVPN2Mask as u64,
    );
    masked_write_64(
        &mut n64.cpu.mmu.registers.regs[MmuRegister::XContext as usize],
        address >> 31,
        RegionMask as u64,
    );
    masked_write_64(
        &mut n64.cpu.mmu.registers.regs[MmuRegister::EntryHi as usize],
        address,
        0xFFFFE000,
    );
}

fn exception_general(n64: &mut N64, vector_offset: u32) {
    let status = n64.cpu.mmu.registers.regs[MmuRegister::Status as usize];

    if status & (MmuStatus::EXL as u64) == 0 {
        n64.cpu.mmu.registers.regs[MmuRegister::EPC as usize] = n64.cpu.execution.pc;
        if in_delay_slot(n64) {
            n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] |= MmuCause::BD as u64;
            n64.cpu.mmu.registers.regs[MmuRegister::EPC as usize] -= 4;
        } else {
            n64.cpu.mmu.registers.regs[MmuRegister::Cause as usize] &= !(MmuCause::BD as u64);
        }
    }

    n64.cpu.mmu.registers.regs[MmuRegister::Status as usize] |= MmuStatus::EXL as u64;

    n64.cpu.execution.pc = if status & (MmuStatus::BEV as u64) == 0 {
        cpu::se32((0x80000000 + vector_offset) as i32)
    } else {
        cpu::se32((0xBFC00200 + vector_offset) as i32)
    };

    n64.cpu.branch_state.state = State::Exception;

    add_cycles(n64, 2);
}
