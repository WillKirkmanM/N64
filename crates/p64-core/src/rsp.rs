mod dispatch;
mod division;
use division::DivisionKind;
mod dma;
use dma::{DmaLayout, DMA_LENGTH_READBACK, DMA_START_CYCLES, IMEM_SELECT};
mod instruction;
mod memory;
mod operand;
use operand::{de, imm, rd, rs, sa, vd, ve, velement, voffset, vs, vt};
mod processor;
mod scalar;
use scalar::*;
mod registers;
mod state;
mod types;
mod vector;
mod vector_transfer;
use vector_transfer::*;
mod vector_register;
use crate::builder::registers::register_index;
use types::ElementPattern;
pub use types::*;
use vector_register::{get_vpr_byte, get_vpr_element, modify_vpr_byte, modify_vpr_element};

use super::cpu::rt;
use super::mips::set_rcp_interrupt;
use super::rdram::rdram_calculate_cycles;
use crate::events::remove_event;
use crate::mips;
use crate::mips::{clear_rcp_interrupt, schedule_rcp_interrupt, MI_INTR_SP};
use crate::rdp;
use crate::rsp;
use crate::rsp_vector::*;

use super::events::{create_event, EventType};
use super::mmu::MmuRegister;
use super::{memory as bus, N64};

pub const RSP_MEM_MASK: usize = 0x1FFF;

const VPR_SIZE: u8 = 16;

const ELEMENT_MASK: u32 = 15;

const VECTOR_SIZE: usize = 8;

const GROUP_ALIGNMENT: u32 = 8;
const VECTOR_ALIGNMENT: u32 = 16;

const BASE_MASK: u32 = 15;

const BYTE_SHIFT_LEFT: u32 = 1;

const BYTE_SHIFT_RIGHT: u32 = 7;

const ELEMENTS_PER_GROUP: usize = 4;

const VPR_SHIFT: u32 = 7;

const ELEMENT_PATTERNS: &[(u8, ElementPattern)] = &[
    (0, ElementPattern::new([0, 1, 2, 3])),
    (15, ElementPattern::new([0, 1, 2, 3])),
    (1, ElementPattern::new([6, 7, 4, 5])),
    (4, ElementPattern::new([1, 2, 3, 0])),
    (5, ElementPattern::new([7, 4, 5, 6])),
    (8, ElementPattern::new([4, 5, 6, 7])),
    (11, ElementPattern::new([3, 0, 1, 2])),
    (12, ElementPattern::new([5, 6, 7, 4])),
];

const ADDR_MASK: u32 = 0xFFF;
const COP0_REGISTER_SELECTOR_MASK: u32 = 0x0f;
const EXECUTION_SLICE_CYCLES: u64 = 4096;

const ELEMENT_COUNT: u8 = 16;

const VPR_ELEMENTS: usize = 16;

const STATUS_COMMANDS: &[(u32, u32, u32)] = &[
    (
        SpStatusWrite::ClrSstep as u32,
        SpStatusWrite::SetSstep as u32,
        SpStatus::Sstep as u32,
    ),
    (
        SpStatusWrite::ClrIntrBreak as u32,
        SpStatusWrite::SetIntrBreak as u32,
        SpStatus::IntrBreak as u32,
    ),
    (
        SpStatusWrite::ClrSig0 as u32,
        SpStatusWrite::SetSig0 as u32,
        SpStatus::Sig0 as u32,
    ),
    (
        SpStatusWrite::ClrSig1 as u32,
        SpStatusWrite::SetSig1 as u32,
        SpStatus::Sig1 as u32,
    ),
    (
        SpStatusWrite::ClrSig2 as u32,
        SpStatusWrite::SetSig2 as u32,
        SpStatus::Sig2 as u32,
    ),
    (
        SpStatusWrite::ClrSig3 as u32,
        SpStatusWrite::SetSig3 as u32,
        SpStatus::Sig3 as u32,
    ),
    (
        SpStatusWrite::ClrSig4 as u32,
        SpStatusWrite::SetSig4 as u32,
        SpStatus::Sig4 as u32,
    ),
    (
        SpStatusWrite::ClrSig5 as u32,
        SpStatusWrite::SetSig5 as u32,
        SpStatus::Sig5 as u32,
    ),
    (
        SpStatusWrite::ClrSig6 as u32,
        SpStatusWrite::SetSig6 as u32,
        SpStatus::Sig6 as u32,
    ),
    (
        SpStatusWrite::ClrSig7 as u32,
        SpStatusWrite::SetSig7 as u32,
        SpStatus::Sig7 as u32,
    ),
];

fn in_delay_slot_taken(n64: &N64) -> bool {
    let rsp = n64.rsp();

    rsp.processor.branch_state.state == CpuState::DelaySlotTaken
}

#[cfg(test)]
fn run(n64: &mut N64) -> u64 {
    run_at(n64, n64.cp0_register(MmuRegister::Count))
}

fn run_at(n64: &mut N64, start: u64) -> u64 {
    n64.rsp_mut().processor.cpu_slice_start = start;
    n64.rsp_mut().begin_execution();
    while !n64.rsp().processor.sync_point
        && n64.rsp().processor.cycle_counter < EXECUTION_SLICE_CYCLES
    {
        let instruction = n64.rsp_mut().begin_instruction();
        (instruction.func)(n64, instruction.opcode);
        n64.rsp_mut().retire_instruction();

        if n64.rsp().processor.broken {
            break;
        }
    }

    n64.rsp_mut().finish_execution()
}

fn decode_opcode(n64: &N64, opcode: u32) -> fn(&mut N64, u32) {
    n64.rsp().instruction_tables.decode(opcode)
}

pub fn start_rsp(n64: &mut N64) {
    let rsp = n64.rsp_mut();
    rsp.vector.initialize_division_tables();

    rsp.vector.shuffle = {
        let mut shuffle =
            [from_bytes_reversed([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]); 16];

        shuffle[0] = from_bytes_reversed([15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
        shuffle[1] = from_bytes_reversed([15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);

        shuffle[2] = from_bytes_reversed([15, 14, 15, 14, 11, 10, 11, 10, 7, 6, 7, 6, 3, 2, 3, 2]);
        shuffle[3] = from_bytes_reversed([13, 12, 13, 12, 9, 8, 9, 8, 5, 4, 5, 4, 1, 0, 1, 0]);

        for i in 0..4 {
            shuffle[4 + i] = from_bytes_reversed([
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (7 - 2 * i) as i8,
                (6 - 2 * i) as i8,
                (7 - 2 * i) as i8,
                (6 - 2 * i) as i8,
                (7 - 2 * i) as i8,
                (6 - 2 * i) as i8,
                (7 - 2 * i) as i8,
                (6 - 2 * i) as i8,
            ]);
        }

        for i in 0..8 {
            shuffle[8 + i] = from_bytes_reversed([
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
                (15 - 2 * i) as i8,
                (14 - 2 * i) as i8,
            ]);
        }

        shuffle
    };

    rsp.instruction_tables = InstructionTables::initialized();
}

pub fn read_mem_fast(n64: &mut N64, address: u64, _access_size: bus::AccessSize) -> u32 {
    let rsp = n64.rsp_mut();
    let masked_address = address as usize & RSP_MEM_MASK;

    u32::from_be_bytes(
        rsp.memory.mem[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    )
}

pub fn read_mem(n64: &mut N64, address: u64, access_size: bus::AccessSize) -> u32 {
    read_mem_fast(n64, address, access_size)
}

pub fn write_mem(n64: &mut N64, address: u64, value: u32, _mask: u32) {
    let masked_address = address as usize & RSP_MEM_MASK;
    let data = value;
    // The CPU-facing SP bus writes a whole shifted word even for SB and SH.
    n64.rsp.memory.mem[masked_address..masked_address + 4].copy_from_slice(&data.to_be_bytes());

    if masked_address & IMEM_SELECT as usize != 0 {
        n64.rsp.instruction.instructions[(masked_address & ADDR_MASK as usize) / 4].func =
            rsp::decode_opcode(n64, data);
        n64.rsp.instruction.instructions[(masked_address & ADDR_MASK as usize) / 4].opcode = data;
    }
}

fn do_dma(n64: &mut N64, dma: RspDma) {
    let start = if n64.rsp().processor.running {
        n64.rsp().slice_cpu_time()
    } else {
        n64.cp0_register(MmuRegister::Count)
    };
    do_dma_at(n64, dma, start);
}

fn do_dma_at(n64: &mut N64, dma: RspDma, start: u64) {
    let DmaLayout {
        length,
        count,
        skip,
        memory_address: mut mem_addr,
        dram_address: mut dram_addr,
        bank: offset,
    } = DmaLayout::decode(&dma);

    let read_data = |n64: &N64, addr: usize, len: usize| -> u32 {
        u32::from_be_bytes(n64.rsp.memory.mem[addr..addr + len].try_into().unwrap())
    };

    // RDRAM uses canonical little-endian words; SP memory uses guest big-endian bytes.
    // DMA converts the representations regardless of the host endianness.
    let write_data = |n64: &mut N64, addr: usize, data: u32| {
        #[cfg(feature = "accuracy-diagnostics")]
        if crate::rdram::write_watched(addr as u64) {
            crate::rdram::trace_write(
            n64,
            &format!(
                "rsp-dma length_raw={:#x} length={length:#x} count={count:#x} skip={skip:#x} mem_start={:#x} dram_start={:#x} bank={offset:#x}",
                dma.length, dma.memaddr, dma.dramaddr
            ),
            addr as u64,
            data,
            u32::MAX,
            );
        }
        if let Some(destination) = n64.rdram.mem.data.get_mut(addr..addr + 4) {
            destination.copy_from_slice(&data.to_le_bytes());
            crate::rdram::record_write(n64, addr as u64);
        }
    };

    let mut process_dma = |n64: &mut N64, dir: DmaDir| {
        for _ in 0..count {
            for _ in (0..length).step_by(4) {
                let rsp_addr = (offset + (mem_addr & ADDR_MASK)) as usize;
                let bus_address = dram_addr as usize & crate::rdram::RDRAM_MASK;

                if dir == DmaDir::Read {
                    let data = read_data(n64, rsp_addr, 4);
                    write_data(n64, bus_address, data);
                } else {
                    let data = n64
                        .rdram
                        .mem
                        .data
                        .get(bus_address..bus_address + 4)
                        .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
                        .unwrap_or(0);

                    if offset != 0 {
                        let instr_idx = ((mem_addr & ADDR_MASK) / 4) as usize;
                        n64.rsp.instruction.instructions[instr_idx].func =
                            rsp::decode_opcode(n64, data);
                        n64.rsp.instruction.instructions[instr_idx].opcode = data;
                    }

                    n64.rsp.memory.mem[rsp_addr..rsp_addr + 4].copy_from_slice(&data.to_be_bytes());
                }

                mem_addr += 4;
                dram_addr = (dram_addr + 4) & crate::rdram::RDRAM_MASK as u32;
            }
            dram_addr = (dram_addr + skip) & crate::rdram::RDRAM_MASK as u32;
        }
    };

    process_dma(n64, dma.dir);

    n64.rsp.registers.regs[SpReg::MemAddr as usize] = (mem_addr & ADDR_MASK) + offset;
    n64.rsp.registers.regs[SpReg::DramAddr as usize] = dram_addr;
    n64.rsp.registers.regs[SpReg::RdLen as usize] = DMA_LENGTH_READBACK;
    n64.rsp.registers.regs[SpReg::WrLen as usize] = DMA_LENGTH_READBACK;

    create_event(
        n64,
        EventType::SPDma,
        start + rdram_calculate_cycles((count * length) as u64) + DMA_START_CYCLES,
        rsp::fifo_pop,
    );
}

fn fifo_push(n64: &mut N64, dir: DmaDir) {
    let rsp = n64.rsp_mut();
    let length_register = if dir == DmaDir::Read {
        SpReg::WrLen
    } else {
        SpReg::RdLen
    };
    let command = RspDma {
        dir,
        length: rsp.registers.regs[length_register as usize],
        memaddr: rsp.registers.regs[SpReg::MemAddr as usize],
        dramaddr: rsp.registers.regs[SpReg::DramAddr as usize],
    };

    if rsp.registers.regs[SpReg::DmaBusy as usize] != 0 {
        // A new submission replaces the pending command, even when DMAFull is set.
        rsp.memory.fifo[1] = command;
        rsp.registers.regs[SpReg::DmaFull as usize] = 1;
        rsp.registers.regs[SpReg::Status as usize] |= SpStatus::DmaFull as u32;
    } else {
        rsp.memory.fifo[0] = command;
        rsp.registers.regs[SpReg::DmaBusy as usize] = 1;
        rsp.registers.regs[SpReg::Status as usize] |= SpStatus::DmaBusy as u32;

        do_dma(n64, command)
    }
}

fn fifo_pop(n64: &mut N64) {
    if n64.rsp.registers.regs[SpReg::DmaFull as usize] != 0 {
        n64.rsp.memory.fifo[0] = n64.rsp.memory.fifo[1];
        n64.rsp.registers.regs[SpReg::DmaFull as usize] = 0;
        n64.rsp.registers.regs[SpReg::Status as usize] &= !(SpStatus::DmaFull as u32);

        let start = n64.cpu().event_deadline(EventType::SPDma);
        do_dma_at(n64, n64.rsp.memory.fifo[0], start)
    } else {
        n64.rsp.registers.regs[SpReg::DmaBusy as usize] = 0;
        n64.rsp.registers.regs[SpReg::Status as usize] &= !(SpStatus::DmaBusy as u32);
    }
}

pub fn read_regs(n64: &mut N64, address: u64, _access_size: bus::AccessSize) -> u32 {
    let rsp = n64.rsp_mut();
    let reg = register_index(address);

    match reg as u32 {
        x if x == SpReg::Semaphore as u32 => {
            let value = rsp.registers.regs[reg];
            rsp.registers.regs[reg] = 1;

            value
        }
        _ => rsp.registers.regs[reg],
    }
}

pub fn write_regs(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let reg = register_index(address);

    match reg as u32 {
        x if x == SpReg::Status as u32 => update_sp_status(n64, value),
        x if x == SpReg::RdLen as u32 => {
            bus::masked_write_32(&mut n64.rsp.registers.regs[reg], value, mask);

            fifo_push(n64, DmaDir::Write)
        }
        x if x == SpReg::WrLen as u32 => {
            bus::masked_write_32(&mut n64.rsp.registers.regs[reg], value, mask);

            fifo_push(n64, DmaDir::Read)
        }
        x if x == SpReg::Semaphore as u32 => {
            bus::masked_write_32(&mut n64.rsp.registers.regs[reg], 0, mask)
        }
        _ => {
            bus::masked_write_32(&mut n64.rsp.registers.regs[reg], value, mask);
        }
    }
}

pub fn read_regs2(n64: &mut N64, address: u64, _access_size: bus::AccessSize) -> u32 {
    let rsp = n64.rsp_mut();

    rsp.registers.regs2[register_index(address)]
}

pub fn write_regs2(n64: &mut N64, address: u64, value: u32, mask: u32) {
    let rsp = n64.rsp_mut();
    let reg = register_index(address);

    match reg as u32 {
        x if x == SpReg2::Pc as u32 => {
            bus::masked_write_32(&mut rsp.registers.regs2[reg], value & 0xFFC, mask);
        }
        _ => bus::masked_write_32(&mut rsp.registers.regs2[reg], value, mask),
    }
}

fn update_sp_status(n64: &mut N64, w: u32) {
    let was_halted = n64.rsp.registers.regs[SpReg::Status as usize] & SpStatus::Halt as u32 != 0;

    n64.rsp_mut().registers.apply_status_command(
        w,
        SpStatusWrite::ClrHalt as u32,
        SpStatusWrite::SetHalt as u32,
        SpStatus::Halt as u32,
    );
    if w & SpStatusWrite::SetHalt as u32 != 0 && w & SpStatusWrite::ClrHalt as u32 == 0 {
        remove_event(n64, EventType::SP)
    }

    if w & SpStatusWrite::ClrBroke as u32 != 0 {
        n64.rsp.registers.regs[SpReg::Status as usize] &= !(SpStatus::Broke as u32)
    }

    if w & SpStatusWrite::ClrIntr as u32 != 0 && w & SpStatusWrite::SetIntr as u32 == 0 {
        clear_rcp_interrupt(n64, MI_INTR_SP)
    }

    if w & SpStatusWrite::SetIntr as u32 != 0 && w & SpStatusWrite::ClrIntr as u32 == 0 {
        schedule_rcp_interrupt(n64, MI_INTR_SP)
    }

    let registers = &mut n64.rsp_mut().registers;

    for &(clear, set, flag) in STATUS_COMMANDS {
        registers.apply_status_command(w, clear, set, flag);
    }

    if n64.rsp.registers.regs[SpReg::Status as usize] & SpStatus::Halt as u32 == 0 && was_halted {
        n64.rsp.processor.broken = false;
        n64.rsp.processor.halted = false;
        n64.rsp.processor.sync_point = false;
        do_task(n64, n64.cp0_register(MmuRegister::Count));
    }
}

fn do_task(n64: &mut N64, start: u64) {
    let timer = run_at(n64, start);

    create_event(n64, EventType::SP, start + timer, rsp_event)
}

fn rsp_event(n64: &mut N64) {
    if n64.rsp.processor.broken {
        n64.rsp.registers.regs[SpReg::Status as usize] |=
            SpStatus::Halt as u32 | SpStatus::Broke as u32;

        if n64.rsp.registers.regs[SpReg::Status as usize] & SpStatus::IntrBreak as u32 != 0 {
            set_rcp_interrupt(n64, mips::MI_INTR_SP)
        }

        return;
    }

    if n64.rsp.processor.halted {
        n64.rsp.registers.regs[SpReg::Status as usize] |= SpStatus::Halt as u32;
        return;
    }

    n64.rsp.processor.sync_point = false;

    let start = n64.cpu().event_deadline(EventType::SP);
    do_task(n64, start)
}

#[inline(always)]
fn se16(value: i16) -> u32 {
    value as i32 as u32
}

fn sign_extend_7bit_offset(offset: u8, shift_amount: u32) -> u32 {
    let soffset = (((offset << 1) & 0x80) | offset) as i8;

    (((soffset) as i32) as u32) << shift_amount
}

fn mfc0(n64: &mut N64, opcode: u32) {
    let selector = rd(opcode) & COP0_REGISTER_SELECTOR_MASK;
    n64.rsp.processor.record_cop0_read(rt(opcode) as usize);
    if selector == SpReg::RegsCount as u32 + rdp::DpcReg::Clock as u32 {
        // RSP runs ahead of CPU event dispatch; include its elapsed RCP cycles.
        let start = if n64.rsp.processor.running {
            n64.rsp.processor.cpu_slice_start
        } else {
            n64.cp0_register(MmuRegister::Count)
        };
        n64.rsp.registers.gpr[rt(opcode) as usize] = rdp::clock_at(n64, start)
            .wrapping_add(n64.rsp.processor.cycle_counter as u32)
            & 0x00ff_ffff;
        return;
    }

    if selector < SpReg::RegsCount as u32 {
        n64.rsp.registers.gpr[rt(opcode) as usize] =
            rsp::read_regs(n64, (selector << 2) as u64, bus::AccessSize::Word)
    } else {
        n64.rsp.registers.gpr[rt(opcode) as usize] = rdp::read_regs_dpc(
            n64,
            ((selector - SpReg::RegsCount as u32) << 2) as u64,
            bus::AccessSize::Word,
        )
    }
    n64.rsp.processor.sync_point = true;
}

fn mtc0(n64: &mut N64, opcode: u32) {
    let selector = rd(opcode) & COP0_REGISTER_SELECTOR_MASK;
    let value = n64.rsp.registers.gpr[rt(opcode) as usize];
    if selector < SpReg::RegsCount as u32 {
        rsp::write_regs(n64, (selector << 2) as u64, value, u32::MAX)
    } else {
        rdp::write_regs_dpc(
            n64,
            ((selector - SpReg::RegsCount as u32) << 2) as u64,
            value,
            u32::MAX,
        )
    }

    if selector == SpReg::Status as u32
        && value & (SpStatusWrite::ClrHalt as u32 | SpStatusWrite::SetHalt as u32)
            == SpStatusWrite::SetHalt as u32
    {
        // Defer visible Halt until the scheduled RSP event completes.
        n64.rsp.registers.regs[SpReg::Status as usize] &= !(SpStatus::Halt as u32);
        n64.rsp.processor.halted = true;
    }
    n64.rsp.processor.sync_point = true;
}

fn mfc2(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let hi = get_vpr_byte(rsp.registers.vpr[rd(opcode) as usize], velement(opcode));
    let lo = get_vpr_byte(rsp.registers.vpr[rd(opcode) as usize], velement(opcode) + 1);

    rsp.registers.gpr[rt(opcode) as usize] = ((hi as u16) << 8 | (lo as u16)) as i16 as i32 as u32
}

fn cfc2(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let hi;
    let lo;
    let mut zero = { zero() };

    match rd(opcode) & 3 {
        0x00 => {
            hi = &mut rsp.vector_control.vcoh;
            lo = &mut rsp.vector_control.vcol;
        }
        0x01 => {
            hi = &mut rsp.vector_control.vcch;
            lo = &mut rsp.vector_control.vccl;
        }
        0x02 | 0x03 => {
            hi = &mut zero;
            lo = &mut rsp.vector_control.vce;
        }
        _ => {
            panic!("unknown ctc2")
        }
    }

    {
        let reverse = from_bytes_reversed([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        rsp.registers.gpr[rt(opcode) as usize] =
            (byte_sign_mask(shuffle_bytes(pack_signed_bytes(*hi, *lo), reverse))) as i16 as u32;
    }
}

fn mtc2(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    modify_vpr_byte(
        &mut rsp.registers.vpr[rd(opcode) as usize],
        (rsp.registers.gpr[rt(opcode) as usize] >> 8) as u8,
        velement(opcode),
    );

    if velement(opcode) != 15 {
        modify_vpr_byte(
            &mut rsp.registers.vpr[rd(opcode) as usize],
            rsp.registers.gpr[rt(opcode) as usize] as u8,
            velement(opcode) + 1,
        );
    }
}

fn ctc2(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let zero = zero();
    let rt_val = rsp.registers.gpr[rt(opcode) as usize];
    let mask = from_lanes_reversed([
        0x0101,
        0x0202,
        0x0404,
        0x0808,
        0x1010,
        0x2020,
        0x4040,
        0x8080u16 as i16,
    ]);

    let computed_value = equal_bytes(bit_and(splat_bytes(!rt_val as i8), mask), zero);

    let computed_high = equal_bytes(bit_and(splat_bytes(!(rt_val >> 8) as i8), mask), zero);

    match rd(opcode) & 3 {
        0x00 => {
            rsp.vector_control.vcoh = computed_high;
            rsp.vector_control.vcol = computed_value;
        }
        0x01 => {
            rsp.vector_control.vcch = computed_high;
            rsp.vector_control.vccl = computed_value;
        }
        0x02 | 0x03 => {
            rsp.vector_control.vce = computed_value;
        }
        _ => panic!("Invalid CTC2 opcode"),
    }
}

fn clamp_signed_32(value: i32) -> i16 {
    value.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

fn clamp_signed_64(value: i64) -> i16 {
    value.clamp(i16::MIN as i64, i16::MAX as i64) as i16
}

fn s_clip(x: i64, bits: u32) -> i64 {
    let b = 1_u64 << (bits - 1);
    let m = b * 2 - 1;

    ((((x as u64) & m) ^ b).wrapping_sub(b)) as i64
}

fn vte(n64: &N64, vt: u32, index: usize) -> Vector {
    let rsp = n64.rsp();

    shuffle_bytes(
        vector_from_bits(rsp.registers.vpr[vt as usize]),
        rsp.vector.shuffle[index],
    )
}

#[inline(always)]
fn vs_reg(n64: &N64, opcode: u32) -> Vector {
    let rsp = n64.rsp();

    vector_from_bits(rsp.registers.vpr[vs(opcode) as usize])
}

#[inline(always)]
fn set_vd_reg(n64: &mut N64, opcode: u32, value: Vector) {
    let rsp = n64.rsp_mut();
    rsp.registers.vpr[vd(opcode) as usize] = vector_to_bits(value);
}

fn rounded_multiply(
    n64: &mut N64,
    opcode: u32,
    finish: impl FnOnce(&AccumulatorState, Vector, Vector) -> Vector,
) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);
    let vs = vs_reg(n64, opcode);
    let lo = mul_low(vs, vte);
    let round = shift_left(equal_lanes(zero(), zero()), 15);
    let sign1 = shift_right_unsigned(lo, 15);
    let lo = add_wrapping(lo, lo);
    let sign2 = shift_right_unsigned(lo, 15);
    let hi = shift_left(mul_high_signed(vs, vte), 1);
    let equal = equal_lanes(vs, vte);
    let accumulator = &mut n64.rsp_mut().accumulator;
    accumulator.accl = add_wrapping(round, lo);
    accumulator.accm = add_wrapping(hi, add_wrapping(sign1, sign2));
    let negative = shift_right_signed(accumulator.accm, 15);
    accumulator.acch = and_not(equal, negative);
    let result = finish(accumulator, equal, negative);
    set_vd_reg(n64, opcode, result);
}

fn vmulf(n64: &mut N64, opcode: u32) {
    rounded_multiply(n64, opcode, |accumulator, equal, negative| {
        add_wrapping(accumulator.accm, bit_and(equal, negative))
    });
}

fn vmulu(n64: &mut N64, opcode: u32) {
    rounded_multiply(n64, opcode, |accumulator, _, negative| {
        and_not(accumulator.acch, bit_or(accumulator.accm, negative))
    });
}

#[inline(always)]
fn get_accumulator(acch: u128, accm: u128, accl: u128, n: usize) -> i64 {
    let mut acc = 0i64;
    acc |= get_vpr_element(acch, n as u8) as i64;
    acc <<= 16;
    acc |= get_vpr_element(accm, n as u8) as i64;
    acc <<= 16;
    acc |= get_vpr_element(accl, n as u8) as i64;

    acc << 16 >> 16
}

#[inline(always)]
fn set_accumulator(acch: &mut u128, accm: &mut u128, accl: &mut u128, acc: i64, n: usize) {
    modify_vpr_element(acch, (acc >> 32) as u16, n as u8);
    modify_vpr_element(accm, (acc >> 16) as u16, n as u8);
    modify_vpr_element(accl, acc as u16, n as u8);
}

fn round_accumulator(n64: &mut N64, opcode: u32, negative: bool) {
    let vte = { vector_to_bits(vte(n64, vt(opcode), ve(opcode) as usize)) };

    let mut acc_values = (
        vector_to_bits(n64.rsp.accumulator.acch),
        vector_to_bits(n64.rsp.accumulator.accm),
        vector_to_bits(n64.rsp.accumulator.accl),
    );
    let (acch, accm, accl) = (&mut acc_values.0, &mut acc_values.1, &mut acc_values.2);

    for n in 0..8 {
        let product = if vs(opcode) & 1 != 0 {
            (get_vpr_element(vte, n) as i16 as i32) << 16
        } else {
            get_vpr_element(vte, n) as i16 as i32
        };

        let mut acc = get_accumulator(*acch, *accm, *accl, n as usize);

        if (acc < 0) == negative {
            acc = s_clip(acc + product as i64, 48);
        }

        set_accumulator(acch, accm, accl, acc, n as usize);

        modify_vpr_element(
            &mut n64.rsp.registers.vpr[vd(opcode) as usize],
            clamp_signed_64(acc >> 16) as u16,
            n,
        );
    }
    n64.rsp.accumulator.acch = vector_from_bits(acc_values.0);
    n64.rsp.accumulator.accm = vector_from_bits(acc_values.1);
    n64.rsp.accumulator.accl = vector_from_bits(acc_values.2);
}

#[inline(always)]
fn vrndp(n64: &mut N64, opcode: u32) {
    round_accumulator(n64, opcode, false);
}

fn vector_mul_element(vs: u128, vte: u128, n: usize) -> i32 {
    (get_vpr_element(vs, n as u8) as i16 as i32)
        .wrapping_mul(get_vpr_element(vte, n as u8) as i16 as i32)
}

#[inline(always)]
fn set_acc_elements(acch: &mut u128, accm: &mut u128, accl: &mut u128, product: i32, n: usize) {
    modify_vpr_element(acch, (product >> 16) as u16, n as u8);
    modify_vpr_element(accm, product as u16, n as u8);
    modify_vpr_element(accl, 0, n as u8);
}

fn vmulq(n64: &mut N64, opcode: u32) {
    let vte = { vector_to_bits(vte(n64, vt(opcode), ve(opcode) as usize)) };

    let vs = n64.rsp.registers.vpr[vs(opcode) as usize];
    let mut acc_values = (
        vector_to_bits(n64.rsp.accumulator.acch),
        vector_to_bits(n64.rsp.accumulator.accm),
        vector_to_bits(n64.rsp.accumulator.accl),
    );
    let (acch, accm, accl) = (&mut acc_values.0, &mut acc_values.1, &mut acc_values.2);

    for n in 0..8 {
        let mut product = vector_mul_element(vs, vte, n);

        if product < 0 {
            product += 31
        }

        set_acc_elements(acch, accm, accl, product, n);

        modify_vpr_element(
            &mut n64.rsp.registers.vpr[vd(opcode) as usize],
            (crate::bits::align_down(clamp_signed_32(product >> 1), 16)) as u16,
            n as u8,
        );
    }
    n64.rsp.accumulator.acch = vector_from_bits(acc_values.0);
    n64.rsp.accumulator.accm = vector_from_bits(acc_values.1);
    n64.rsp.accumulator.accl = vector_from_bits(acc_values.2);
}

fn vmudl(n64: &mut N64, opcode: u32) {
    let vs = vs_reg(n64, opcode);
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    n64.rsp.accumulator.accl = mul_high_unsigned(vs, vte);

    n64.rsp.accumulator.accm = zero();
    n64.rsp.accumulator.acch = zero();

    set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);
}

fn multiply_accumulator(
    n64: &mut N64,
    opcode: u32,
    operation: impl FnOnce(&mut AccumulatorState, Vector, Vector) -> Vector,
) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);
    let vs = vs_reg(n64, opcode);
    let accumulator = &mut n64.rsp_mut().accumulator;
    accumulator.accl = mul_low(vs, vte);
    accumulator.accm = mul_high_unsigned(vs, vte);
    let result = operation(accumulator, vs, vte);
    set_vd_reg(n64, opcode, result);
}

fn vmudm(n64: &mut N64, opcode: u32) {
    multiply_accumulator(n64, opcode, |accumulator, vs, vte| {
        let sign = shift_right_signed(vs, 15);
        let vta = bit_and(vte, sign);
        accumulator.accm = sub_wrapping(accumulator.accm, vta);
        accumulator.acch = shift_right_signed(accumulator.accm, 15);

        accumulator.accm
    });
}

fn vmudn(n64: &mut N64, opcode: u32) {
    multiply_accumulator(n64, opcode, |accumulator, vs, vte| {
        let sign = shift_right_signed(vte, 15);
        let vsa = bit_and(vs, sign);
        accumulator.accm = sub_wrapping(accumulator.accm, vsa);
        accumulator.acch = shift_right_signed(accumulator.accm, 15);

        accumulator.accl
    });
}

fn vmudh(n64: &mut N64, opcode: u32) {
    multiply_accumulator(n64, opcode, |accumulator, vs, vte| {
        accumulator.accl = zero();
        accumulator.accm = mul_low(vs, vte);
        accumulator.acch = mul_high_signed(vs, vte);
        let lo = interleave_low(accumulator.accm, accumulator.acch);
        let hi = interleave_high(accumulator.accm, accumulator.acch);

        pack_signed_lanes(lo, hi)
    });
}

fn accumulate_fractional_product(accumulator: &mut AccumulatorState, left: Vector, right: Vector) {
    let low = mul_low(left, right);
    let high = mul_high_signed(left, right);
    let middle = bit_or(shift_left(high, 1), shift_right_unsigned(low, 15));
    let high = shift_right_signed(high, 15);
    let low = shift_left(low, 1);
    let (sum, overflow) = handle_accumulator(accumulator.accl, low);
    accumulator.accl = sum;
    let middle = sub_wrapping(middle, overflow);
    let carry = bit_and(equal_lanes(middle, zero()), overflow);
    let high = sub_wrapping(high, carry);
    let (sum, overflow) = handle_accumulator(accumulator.accm, middle);
    accumulator.accm = sum;
    accumulator.acch = sub_wrapping(add_wrapping(accumulator.acch, high), overflow);
}

fn vmacf(n64: &mut N64, opcode: u32) {
    let left = vs_reg(n64, opcode);
    let right = vte(n64, vt(opcode), ve(opcode) as usize);
    let accumulator = &mut n64.rsp_mut().accumulator;
    accumulate_fractional_product(accumulator, left, right);
    let low = interleave_low(accumulator.accm, accumulator.acch);
    let high = interleave_high(accumulator.accm, accumulator.acch);
    let result = pack_signed_lanes(low, high);
    set_vd_reg(n64, opcode, result);
}

fn vmacu(n64: &mut N64, opcode: u32) {
    let right = vte(n64, vt(opcode), ve(opcode) as usize);
    let left = vs_reg(n64, opcode);
    let accumulator = &mut n64.rsp_mut().accumulator;
    accumulate_fractional_product(accumulator, left, right);
    let middle_sign = shift_right_signed(accumulator.accm, 15);
    let high_sign = shift_right_signed(accumulator.acch, 15);
    let middle = and_not(high_sign, bit_or(middle_sign, accumulator.accm));
    let positive = greater_signed(accumulator.acch, zero());
    let result = bit_or(positive, middle);
    set_vd_reg(n64, opcode, result);
}

fn vrndn(n64: &mut N64, opcode: u32) {
    round_accumulator(n64, opcode, true);
}

fn vmacq(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let mut high = vector_to_bits(rsp.accumulator.acch);
    let acch = &mut high;
    let mut mid = vector_to_bits(rsp.accumulator.accm);
    let accm = &mut mid;

    let mut n = 0;

    while n < 8 {
        let mut product =
            (get_vpr_element(*acch, n) as i32) << 16 | (get_vpr_element(*accm, n) as i32);

        if product < 0 && (product & (1 << 5)) == 0 {
            product += 32
        } else if product >= 32 && (product & (1 << 5)) == 0 {
            product -= 32
        }

        modify_vpr_element(acch, (product >> 16) as u16, n);
        modify_vpr_element(accm, (product) as u16, n);
        modify_vpr_element(
            &mut rsp.registers.vpr[vd(opcode) as usize],
            (crate::bits::align_down(clamp_signed_32(product >> 1), 16)) as u16,
            n,
        );
        n += 1;
    }
    rsp.accumulator.acch = vector_from_bits(high);
    rsp.accumulator.accm = vector_from_bits(mid);
}

fn vmadl(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);
    let vs = vs_reg(n64, opcode);
    let hi = mul_high_unsigned(vs, vte);

    let omask = add_saturating_unsigned(n64.rsp.accumulator.accl, hi);
    n64.rsp.accumulator.accl = add_wrapping(n64.rsp.accumulator.accl, hi);
    let mut omask = equal_lanes(n64.rsp.accumulator.accl, omask);
    omask = equal_lanes(omask, zero());

    let hi = sub_wrapping(zero(), omask);
    let omask = add_saturating_unsigned(n64.rsp.accumulator.accm, hi);
    n64.rsp.accumulator.accm = add_wrapping(n64.rsp.accumulator.accm, hi);
    let omask = equal_lanes(n64.rsp.accumulator.accm, omask);
    let omask = equal_lanes(omask, zero());

    n64.rsp.accumulator.acch = sub_wrapping(n64.rsp.accumulator.acch, omask);
    let nhi = shift_right_signed(n64.rsp.accumulator.acch, 15);
    let nmd = shift_right_signed(n64.rsp.accumulator.accm, 15);

    let shi = equal_lanes(nhi, n64.rsp.accumulator.acch);
    let smd = equal_lanes(nhi, nmd);
    let cmask = bit_and(smd, shi);
    let cval = equal_lanes(nhi, zero());

    let result = blend_bytes(cval, n64.rsp.accumulator.accl, cmask);
    set_vd_reg(n64, opcode, result);
}

fn vmadm(n64: &mut N64, opcode: u32) {
    let vs = vs_reg(n64, opcode);
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    let lo = mul_low(vs, vte);
    let hi = mul_high_unsigned(vs, vte);
    let sign = shift_right_signed(vs, 15);
    let vta = bit_and(vte, sign);
    let hi = sub_wrapping(hi, vta);

    let mut omask = add_saturating_unsigned(n64.rsp.accumulator.accl, lo);
    n64.rsp.accumulator.accl = add_wrapping(n64.rsp.accumulator.accl, lo);
    omask = equal_lanes(n64.rsp.accumulator.accl, omask);
    omask = equal_lanes(omask, zero());

    let hi = sub_wrapping(hi, omask);
    let omask = add_saturating_unsigned(n64.rsp.accumulator.accm, hi);
    n64.rsp.accumulator.accm = add_wrapping(n64.rsp.accumulator.accm, hi);
    let omask = equal_lanes(n64.rsp.accumulator.accm, omask);
    let omask = equal_lanes(omask, zero());

    let hi = shift_right_signed(hi, 15);
    n64.rsp.accumulator.acch = add_wrapping(n64.rsp.accumulator.acch, hi);
    n64.rsp.accumulator.acch = sub_wrapping(n64.rsp.accumulator.acch, omask);

    let lo = interleave_low(n64.rsp.accumulator.accm, n64.rsp.accumulator.acch);
    let hi = interleave_high(n64.rsp.accumulator.accm, n64.rsp.accumulator.acch);
    let result = pack_signed_lanes(lo, hi);

    set_vd_reg(n64, opcode, result);
}

#[inline(always)]
fn vector_at(n64: &N64, index: u32) -> Vector {
    vector_from_bits(n64.rsp().registers.vpr[index as usize])
}

fn handle_accumulator(acc: Vector, value: Vector) -> (Vector, Vector) {
    let sum = add_saturating_unsigned(acc, value);
    let result = add_wrapping(acc, value);
    let omask = equal_lanes(equal_lanes(result, sum), zero());

    (result, omask)
}

fn vmadn(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);

        let lo = mul_low(vs, vte);
        let sign = shift_right_signed(vte, 15);
        let mut hi = sub_wrapping(mul_high_unsigned(vs, vte), bit_and(vs, sign));

        let (accl, omask1) = handle_accumulator(n64.rsp.accumulator.accl, lo);
        hi = sub_wrapping(hi, omask1);
        let (accm, omask2) = handle_accumulator(n64.rsp.accumulator.accm, hi);

        n64.rsp.accumulator.accl = accl;
        n64.rsp.accumulator.accm = accm;
        n64.rsp.accumulator.acch = sub_wrapping(
            add_wrapping(n64.rsp.accumulator.acch, shift_right_signed(hi, 15)),
            omask2,
        );

        let nhi = shift_right_signed(n64.rsp.accumulator.acch, 15);
        let cmask = bit_and(
            equal_lanes(nhi, shift_right_signed(accm, 15)),
            equal_lanes(nhi, n64.rsp.accumulator.acch),
        );

        set_vd_reg(
            n64,
            opcode,
            blend_bytes(equal_lanes(nhi, zero()), accl, cmask),
        );
    }
}

fn vmadh(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);
    let (mut lo, mut hi, mut omask);

    {
        lo = mul_low(vs_reg(n64, opcode), vte);
        hi = mul_high_signed(vs_reg(n64, opcode), vte);
        omask = add_saturating_unsigned(n64.rsp.accumulator.accm, lo);
        n64.rsp.accumulator.accm = add_wrapping(n64.rsp.accumulator.accm, lo);
        omask = equal_lanes(n64.rsp.accumulator.accm, omask);
        omask = equal_lanes(omask, zero());
        hi = sub_wrapping(hi, omask);
        n64.rsp.accumulator.acch = add_wrapping(n64.rsp.accumulator.acch, hi);
        lo = interleave_low(n64.rsp.accumulator.accm, n64.rsp.accumulator.acch);
        hi = interleave_high(n64.rsp.accumulator.accm, n64.rsp.accumulator.acch);
        n64.rsp.registers.vpr[vd(opcode) as usize] = vector_to_bits(pack_signed_lanes(lo, hi));
    }
}

fn vadd(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);
    let (sum, mut min, max);

    {
        sum = add_wrapping(vs_reg(n64, opcode), vte);
        n64.rsp.accumulator.accl = sub_wrapping(sum, n64.rsp.vector_control.vcol);
        min = min_signed(vs_reg(n64, opcode), vte);
        max = max_signed(vs_reg(n64, opcode), vte);
        min = sub_saturating_signed(min, n64.rsp.vector_control.vcol);
        n64.rsp.registers.vpr[vd(opcode) as usize] =
            vector_to_bits(add_saturating_signed(min, max));
        n64.rsp.vector_control.vcol = zero();
        n64.rsp.vector_control.vcoh = zero();
    }
}

#[inline]
fn vsub(n64: &mut N64, opcode: u32) {
    let dest = vd(opcode) as usize;
    let vector_t = vte(n64, vt(opcode), ve(opcode) as usize);
    let vector_s = vs_reg(n64, opcode);

    {
        let udiff = sub_wrapping(vector_t, n64.rsp.vector_control.vcol);
        let sdiff = sub_saturating_signed(vector_t, n64.rsp.vector_control.vcol);

        n64.rsp.accumulator.accl = sub_wrapping(vector_s, udiff);
        let overflow = greater_signed(sdiff, udiff);

        n64.rsp.registers.vpr[dest] = vector_to_bits(sub_saturating_signed(vector_s, sdiff));
        n64.rsp.registers.vpr[dest] =
            vector_to_bits(add_saturating_signed(vector_at(n64, dest as u32), overflow));

        n64.rsp.vector_control.vcol = zero();
        n64.rsp.vector_control.vcoh = zero();
    }
}

fn vzero(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        n64.rsp.accumulator.accl = add_wrapping(vs_reg(n64, opcode), vte);
        n64.rsp.registers.vpr[vd(opcode) as usize] = vector_to_bits(bit_xor(
            vector_at(n64, vd(opcode)),
            vector_at(n64, vd(opcode)),
        ));
    }
}

fn vabs(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);
        let vs0 = equal_lanes(vs, zero());
        let slt = shift_right_signed(vs, 15);

        let mut result = and_not(vs0, vte);
        result = bit_xor(result, slt);

        n64.rsp.accumulator.accl = sub_wrapping(result, slt);
        set_vd_reg(n64, opcode, sub_saturating_signed(result, slt));
    }
}

fn vaddc(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);
        let sum = add_saturating_unsigned(vs, vte);
        n64.rsp.accumulator.accl = add_wrapping(vs, vte);

        n64.rsp.vector_control.vcol = equal_lanes(sum, n64.rsp.accumulator.accl);
        n64.rsp.vector_control.vcol = equal_lanes(n64.rsp.vector_control.vcol, zero());
        n64.rsp.vector_control.vcoh = zero();

        set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);
    }
}

fn vsubc(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);
        let udiff = sub_saturating_unsigned(vs, vte);
        let equal = equal_lanes(vs, vte);
        let diff0 = equal_lanes(udiff, zero());

        n64.rsp.vector_control.vcoh = equal_lanes(equal, zero());
        n64.rsp.vector_control.vcol = and_not(equal, diff0);
        n64.rsp.accumulator.accl = sub_wrapping(vs, vte);

        set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);
    }
}

fn accumulator_bits(acc: Vector) -> u128 {
    vector_to_bits(acc)
}

fn vsar(n64: &mut N64, opcode: u32) {
    let rsp = n64.rsp_mut();
    let value = match ve(opcode) {
        0x8 => accumulator_bits(rsp.accumulator.acch),
        0x9 => accumulator_bits(rsp.accumulator.accm),
        0xa => accumulator_bits(rsp.accumulator.accl),
        _ => 0,
    };

    rsp.registers.vpr[vd(opcode) as usize] = value;
}

fn vlt(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);
        let eq = bit_and(
            bit_and(n64.rsp.vector_control.vcoh, equal_lanes(vs, vte)),
            n64.rsp.vector_control.vcol,
        );
        n64.rsp.vector_control.vccl = bit_or(less_signed(vs, vte), eq);
        n64.rsp.accumulator.accl = blend_bytes(vte, vs, n64.rsp.vector_control.vccl);

        n64.rsp.vector_control.vcch = zero();
        n64.rsp.vector_control.vcoh = zero();
        n64.rsp.vector_control.vcol = zero();

        set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);
    }
}

fn veq(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);
        let eq = equal_lanes(vs, vte);
        n64.rsp.vector_control.vccl = and_not(n64.rsp.vector_control.vcoh, eq);
        n64.rsp.accumulator.accl = blend_bytes(vte, vs, n64.rsp.vector_control.vccl);

        n64.rsp.vector_control.vcch = zero();
        n64.rsp.vector_control.vcoh = zero();
        n64.rsp.vector_control.vcol = zero();

        set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);
    }
}

fn vne(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);
        let eq = equal_lanes(vs, vte);
        let ne = equal_lanes(eq, zero());

        n64.rsp.vector_control.vccl = bit_or(bit_and(n64.rsp.vector_control.vcoh, eq), ne);
        n64.rsp.accumulator.accl = blend_bytes(vte, vs, n64.rsp.vector_control.vccl);

        n64.rsp.vector_control.vcch = zero();
        n64.rsp.vector_control.vcoh = zero();
        n64.rsp.vector_control.vcol = zero();

        set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);
    }
}

fn vge(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);
        let eq = equal_lanes(vs, vte);
        let gt = greater_signed(vs, vte);
        let es = bit_and(n64.rsp.vector_control.vcoh, n64.rsp.vector_control.vcol);

        n64.rsp.vector_control.vccl = bit_or(gt, and_not(es, eq));
        n64.rsp.accumulator.accl = blend_bytes(vte, vs, n64.rsp.vector_control.vccl);

        n64.rsp.vector_control.vcch = zero();
        n64.rsp.vector_control.vcoh = zero();
        n64.rsp.vector_control.vcol = zero();

        set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);
    }
}

fn vcl(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);
        let nvt = sub_wrapping(
            bit_xor(vte, n64.rsp.vector_control.vcol),
            n64.rsp.vector_control.vcol,
        );

        let diff = sub_wrapping(vs, nvt);
        let ncarry = equal_lanes(add_saturating_unsigned(vs, vte), diff);
        let nvce = equal_lanes(n64.rsp.vector_control.vce, zero());
        let diff0 = equal_lanes(diff, zero());

        let lec1 = bit_and(nvce, bit_and(diff0, ncarry));
        let lec2 = bit_and(n64.rsp.vector_control.vce, bit_or(diff0, ncarry));
        let leeq = bit_or(lec1, lec2);

        let geeq = equal_lanes(sub_saturating_unsigned(vte, vs), zero());
        let le = blend_bytes(
            n64.rsp.vector_control.vccl,
            leeq,
            and_not(n64.rsp.vector_control.vcoh, n64.rsp.vector_control.vcol),
        );

        let ge = blend_bytes(
            geeq,
            n64.rsp.vector_control.vcch,
            bit_or(n64.rsp.vector_control.vcol, n64.rsp.vector_control.vcoh),
        );

        let mask = blend_bytes(ge, le, n64.rsp.vector_control.vcol);
        n64.rsp.accumulator.accl = blend_bytes(vs, nvt, mask);

        n64.rsp.vector_control.vcch = ge;
        n64.rsp.vector_control.vccl = le;
        n64.rsp.vector_control.vcoh = zero();
        n64.rsp.vector_control.vcol = zero();
        n64.rsp.vector_control.vce = zero();

        set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);
    }
}

fn vch(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);

        n64.rsp.vector_control.vcol = less_signed(bit_xor(vs, vte), zero());
        let nvt = sub_wrapping(
            bit_xor(vte, n64.rsp.vector_control.vcol),
            n64.rsp.vector_control.vcol,
        );
        let diff = sub_wrapping(vs, nvt);

        let diff0 = equal_lanes(diff, zero());
        let vtn = less_signed(vte, zero());
        let dlez = equal_lanes(zero(), greater_signed(diff, zero()));
        let dgez = bit_or(greater_signed(diff, zero()), diff0);

        n64.rsp.vector_control.vcch = blend_bytes(dgez, vtn, n64.rsp.vector_control.vcol);
        n64.rsp.vector_control.vccl = blend_bytes(vtn, dlez, n64.rsp.vector_control.vcol);
        n64.rsp.vector_control.vce = bit_and(
            equal_lanes(diff, n64.rsp.vector_control.vcol),
            n64.rsp.vector_control.vcol,
        );
        n64.rsp.vector_control.vcoh =
            equal_lanes(bit_or(diff0, n64.rsp.vector_control.vce), zero());

        let mask = blend_bytes(
            n64.rsp.vector_control.vcch,
            n64.rsp.vector_control.vccl,
            n64.rsp.vector_control.vcol,
        );
        n64.rsp.accumulator.accl = blend_bytes(vs, nvt, mask);

        set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);
    }
}

fn vcr(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        let vs = vs_reg(n64, opcode);
        let sign = shift_right_signed(bit_xor(vs, vte), 15);

        let dlez = add_wrapping(bit_and(vs, sign), vte);
        n64.rsp.vector_control.vccl = shift_right_signed(dlez, 15);

        let dgez = min_signed(bit_or(vs, sign), vte);
        n64.rsp.vector_control.vcch = equal_lanes(dgez, vte);

        let nvt = bit_xor(vte, sign);
        let mask = blend_bytes(
            n64.rsp.vector_control.vcch,
            n64.rsp.vector_control.vccl,
            sign,
        );
        n64.rsp.accumulator.accl = blend_bytes(vs, nvt, mask);

        set_vd_reg(n64, opcode, n64.rsp.accumulator.accl);

        n64.rsp.vector_control.vcol = zero();
        n64.rsp.vector_control.vcoh = zero();
        n64.rsp.vector_control.vce = zero();
    }
}

#[inline]
fn vector_binary(n64: &mut N64, opcode: u32, operation: impl FnOnce(Vector, Vector) -> Vector) {
    let right = vte(n64, vt(opcode), ve(opcode) as usize);
    let left = vs_reg(n64, opcode);
    let result = operation(left, right);
    n64.rsp_mut().accumulator.accl = result;
    set_vd_reg(n64, opcode, result);
}

fn vand(n64: &mut N64, opcode: u32) {
    vector_binary(n64, opcode, bit_and);
}

fn vor(n64: &mut N64, opcode: u32) {
    vector_binary(n64, opcode, bit_or);
}

fn vxor(n64: &mut N64, opcode: u32) {
    vector_binary(n64, opcode, bit_xor);
}

fn vnand(n64: &mut N64, opcode: u32) {
    vector_binary(n64, opcode, |a, b| bit_xor(bit_and(a, b), splat_words(-1)));
}

fn vnor(n64: &mut N64, opcode: u32) {
    vector_binary(n64, opcode, |a, b| bit_xor(bit_or(a, b), splat_words(-1)));
}

fn vnxor(n64: &mut N64, opcode: u32) {
    vector_binary(n64, opcode, |a, b| bit_xor(bit_xor(a, b), splat_words(-1)));
}

fn vmrg(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);

    {
        n64.rsp.accumulator.accl = blend_bytes(
            vte,
            vector_from_bits(n64.rsp.registers.vpr[vs(opcode) as usize]),
            n64.rsp.vector_control.vccl,
        );
        n64.rsp.vector_control.vcoh = zero();
        n64.rsp.vector_control.vcol = zero();
        n64.rsp.registers.vpr[vd(opcode) as usize] = vector_to_bits(n64.rsp.accumulator.accl);
    }
}

fn vrcp(n64: &mut N64, opcode: u32) {
    division::divide(n64.rsp_mut(), opcode, DivisionKind::Reciprocal, false);
}

fn vrcpl(n64: &mut N64, opcode: u32) {
    division::divide(n64.rsp_mut(), opcode, DivisionKind::Reciprocal, true);
}

fn vrcph(n64: &mut N64, opcode: u32) {
    division::latch_high(n64.rsp_mut(), opcode);
}

fn vrsqh(n64: &mut N64, opcode: u32) {
    division::latch_high(n64.rsp_mut(), opcode);
}

fn vmov(n64: &mut N64, opcode: u32) {
    let vte = vte(n64, vt(opcode), ve(opcode) as usize);
    let value = get_vpr_element(vector_to_bits(vte), de(opcode) as u8);
    modify_vpr_element(
        &mut n64.rsp.registers.vpr[vd(opcode) as usize],
        value,
        de(opcode) as u8,
    );
    n64.rsp.accumulator.accl = vte;
}

fn vrsq(n64: &mut N64, opcode: u32) {
    division::divide(n64.rsp_mut(), opcode, DivisionKind::SquareRoot, false);
}

fn vrsql(n64: &mut N64, opcode: u32) {
    division::divide(n64.rsp_mut(), opcode, DivisionKind::SquareRoot, true);
}

fn vnop(_device: &mut N64, _opcode: u32) {}

fn execute_vec(n64: &mut N64, opcode: u32) {
    n64.rsp.instruction.instruction_type = rsp::InstructionType::Vu;

    n64.rsp.instruction_tables.vector_table[(opcode & 0x3F) as usize](n64, opcode)
}

#[cfg(test)]
mod accuracy_tests;
