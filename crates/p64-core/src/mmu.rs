use crate::{cpu, events, N64};
use crate::tlb::TlbEntry;

use super::cpu::State;
use super::events::{create_event, EventType};
use super::exceptions::{
    break_exception, check_pending_interrupts, reserved_exception, syscall_exception,
};
use super::{memory, tlb};
use super::{fpu, tlb::TlbLookupTable};
use super::builder::registers::Registers;
use crate::mmu;

impl<T> std::ops::Index<MmuRegister> for Registers<T> {
    type Output = T;
    fn index(&self, reg: MmuRegister) -> &Self::Output {
        &self.regs[reg as usize]
    }
}

impl<T> std::ops::IndexMut<MmuRegister> for Registers<T> {
    fn index_mut(&mut self, reg: MmuRegister) -> &mut Self::Output {
        &mut self.regs[reg as usize]
    }
}

pub fn add_cycles(n64: &mut N64, cycles: u64) {
    n64.cpu.mmu.regs[MmuRegister::Count as usize] += cycles
}

pub fn syscall(n64: &mut N64, _opcode: u32) {
    syscall_exception(n64)
}
pub fn break_(n64: &mut N64, _opcode: u32) {
    break_exception(n64)
}
pub fn reserved(n64: &mut N64, _opcode: u32) {
    reserved_exception(n64, 0)
}

fn mfc0(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[cpu::rt(opcode) as usize] =
        cpu::se32((get_control_registers(n64, cpu::rd(opcode))) as u32 as i32)
}

fn dmfc0(n64: &mut N64, opcode: u32) {
    n64.cpu.gpr[cpu::rt(opcode) as usize] = get_control_registers(n64, cpu::rd(opcode))
}

fn mtc0(n64: &mut N64, opcode: u32) {
    mmu::set_control_registers(
        n64,
        cpu::rd(opcode),
        cpu::se32(n64.cpu.gpr[cpu::rt(opcode) as usize] as u32 as i32),
    )
}

fn dmtc0(n64: &mut N64, opcode: u32) {
    mmu::set_control_registers(n64, cpu::rd(opcode), n64.cpu.gpr[cpu::rt(opcode) as usize])
}

fn tlbr(n64: &mut N64, _opcode: u32) {
    tlb::read(n64, n64.cpu.mmu.regs[MmuRegister::Index as usize])
}
fn tlbwi(n64: &mut N64, _opcode: u32) {
    tlb::write(n64, n64.cpu.mmu.regs[MmuRegister::Index as usize])
}
fn tlbwr(n64: &mut N64, _opcode: u32) {
    let random = set_random_register(n64);
    tlb::write(n64, random)
}

fn tlbp(n64: &mut N64, _opcode: u32) {
    tlb::probe(n64)
}

fn eret(n64: &mut N64, _opcode: u32) {
    if n64.cpu.mmu.regs[MmuRegister::Status as usize] & MmuStatus::ERL as u64 != 0 {
        n64.cpu.pc = n64.cpu.mmu.regs[MmuRegister::ErrorEPC as usize];
        n64.cpu.mmu.regs[MmuRegister::Status as usize] &= !(MmuStatus::ERL as u64)
    } else {
        n64.cpu.pc = n64.cpu.mmu.regs[MmuRegister::EPC as usize];
        n64.cpu.mmu.regs[MmuRegister::Status as usize] &= !(MmuStatus::EXL as u64)
    }
    n64.cpu.branch_state.state = State::Exception;
    n64.cpu.llbit = false;
    check_pending_interrupts(n64)
}

fn execute_cp0(n64: &mut N64, opcode: u32) {
    n64.cpu.mmu.instrs2[(opcode & 0x3F) as usize](n64, opcode)
}

fn get_control_registers(n64: &N64, index: u32) -> u64 {
    match index {
        x if x == MmuRegister::Count as u32 => n64.cpu.mmu.regs[index as usize] >> 1,
        x if x == MmuRegister::Random as u32 => set_random_register(n64),
        7 | 21 | 22 | 23 | 24 | 25 | 31 => n64.cpu.mmu.reg_latch,
        _ => n64.cpu.mmu.regs[index as usize],
    }
}

fn set_control_registers(n64: &mut N64, index: u32, mut data: u64) {
    n64.cpu.mmu.reg_latch = data;
    match index {
        x if x == MmuRegister::Count as u32 => {
            data &= 0xFFFFFFFF;
            data <<= 1;
            events::translate_events(n64, n64.cpu.mmu.regs[MmuRegister::Count as usize], data);
            n64.cpu.mmu.regs[MmuRegister::Count as usize] = data;
            return;
        }
        x if x == MmuRegister::Wired as u32 => n64.cpu.mmu.regs[MmuRegister::Random as usize] = 31,
        x if x == MmuRegister::Compare as u32 => {
            data &= 0xFFFFFFFF;
            let current_count = (n64.cpu.mmu.regs[MmuRegister::Count as usize] >> 1) & 0xFFFFFFFF;
            let mut compare_event_diff = (data as u32).wrapping_sub(current_count as u32);

            if compare_event_diff == 0 {
                compare_event_diff += u32::MAX
            }

            create_event(
                n64,
                EventType::Compare,
                n64.cpu.mmu.regs[MmuRegister::Count as usize] + ((compare_event_diff as u64) << 1),
                compare_event,
            );
            n64.cpu.mmu.regs[MmuRegister::Cause as usize] &= !(MmuCause::IP7 as u64);
        }
        x if x == MmuRegister::Status as u32 => {
            if data & MmuStatus::FR as u64
                != n64.cpu.mmu.regs[index as usize] & MmuStatus::FR as u64
            {
                fpu::set_fgr_registers(n64, data)
            }
        }
        _ => {}
    }
    memory::masked_write_64(
        &mut n64.cpu.mmu.regs[index as usize],
        data,
        n64.cpu.mmu.reg_write_masks[index as usize],
    );
    check_pending_interrupts(n64);
}

fn compare_event(n64: &mut N64) {
    n64.cpu.mmu.regs[MmuRegister::Cause as usize] &= !(MmuCause::ExccodeMask as u64);
    n64.cpu.mmu.regs[MmuRegister::Cause as usize] |= MmuCause::IP7 as u64;

    create_event(
        n64,
        EventType::Compare,
        n64.cpu.next_event_count + (u32::MAX as u64),
        compare_event,
    );
    check_pending_interrupts(n64);
}

fn set_random_register(n64: &N64) -> u64 {
    if n64.cpu.mmu.regs[MmuRegister::Wired as usize] > 31 {
        (u64::MAX - n64.cpu.mmu.regs[MmuRegister::Count as usize]) & 0x3F
    } else {
        (u64::MAX - n64.cpu.mmu.regs[MmuRegister::Count as usize])
            % (32 - n64.cpu.mmu.regs[MmuRegister::Wired as usize])
            + n64.cpu.mmu.regs[MmuRegister::Wired as usize]
    }
}

#[derive(Copy, Clone)]
enum MmuInstruction {
    MFC0 = 0,
    DMFC0 = 1,
    MTC0 = 4,
    DMTC0 = 5,
    EXECUTECP0 = 16,
}

type MmuInstrFn = fn(&mut N64, u32);

#[derive(Copy, Clone)]
enum MmuSecondaryInstruction {
    TLBR = 1,
    TLBWI = 2,
    TLBWR = 6,
    TLBP = 8,
    ERET = 24,
}

const MMU_INIT_VALUES: [(MmuRegister, u64); 8] = [
    (MmuRegister::Random, 0b00000000000000000000000000011111),
    (MmuRegister::Config, 0b01110000000001101110010001100000),
    (MmuRegister::Status, 0b00000000010000000000000000000100),
    (MmuRegister::PrevID, 0b00000000000000000000101100100010),
    (MmuRegister::EPC, 0xFFFFFFFF_FFFFFFFF),
    (MmuRegister::ErrorEPC, 0xFFFFFFFF_FFFFFFFF),
    (MmuRegister::BadVAddr, 0xFFFFFFFF),
    (MmuRegister::Context, 0x7FFFF0),
];

pub fn write_mmu_registers(n64: &mut N64) {
    n64.cpu.mmu.reg_write_masks = [
        MmuMask::Index as u64,
        0, // Random, read only
        MmuMask::EntryLo as u64,
        MmuMask::EntryLo as u64,
        MmuMask::Context as u64,
        MmuMask::PageMask as u64,
        MmuMask::Wired as u64,
        0,               // 7
        0,               // BadVAddr, read only
        u32::MAX as u64, // count
        MmuMask::EntryHi as u64,
        u32::MAX as u64, // compare
        MmuMask::Status as u64,
        MmuMask::Cause as u64,
        u64::MAX, // EPC
        0,        // previd, read only
        MmuMask::Config as u64,
        MmuMask::LLAddr as u64,
        MmuMask::WatchLo as u64,
        MmuMask::WatchHi as u64,
        MmuMask::XContext as u64,
        0, // 21
        0, // 22
        0, // 23
        0, // 24
        0, // 25
        MmuMask::ParityErr as u64,
        0, // cache error
        MmuMask::TagLo as u64,
        0,        // taghi
        u64::MAX, // ErrorPC
        0,        // 31
    ];

    for &(reg, value) in MMU_INIT_VALUES.iter() {
        n64.cpu.mmu.regs[reg as usize] = value
    }

    create_event(n64, EventType::Compare, u32::MAX as u64, compare_event)
}

mod instruction;
pub use instruction::{MMU_INSTRUCTIONS, MMU_SECONDARY_INSTRUCTIONS};

mod registers;
pub use registers::{MmuRegister, MmuStatus, MmuCause, MmuXContext, MmuMask};

mod state;
pub use state::{Mmu};

mod status;
pub use status::Status;
