mod dispatch;
mod rounding;
pub(crate) use rounding::RoundingMode;
mod types;
pub use types::{Fpu, FpuControl, FpuInstructionTables, FpuRegisters, FpuTiming};

use crate::{exceptions, opcode::{Cop1Opcode, FpuFunction}};

use super::{cpu, memory::{self, AccessSize, AccessType}, mmu::{MmuCause, MmuRegister, MmuStatus}, rcp, N64};

pub const FCR31_CAUSE_UNIMP_BIT: u32 = 1 << 17;

pub const FCR31_CMP_BIT: u32 = 1 << 23;

pub const FCR31_FS_BIT: u32 = 1 << 24;

pub const FCR31_CAUSE_MASK: u32 = 0x0003_f000;

pub const FCR31_ENABLE_MASK: u32 = 0xf80;

pub const FCR31_WRITE_MASK: u32 = 0x0183_ffff;
pub(crate) const FCR31_ROUNDING_MASK: u32 = 0b11;
// FS flush behaviour requires both underflow and inexact exceptions disabled.
pub(crate) const FCR31_FLUSH_ENABLE_MASK: u32 = 0b11 << 7;

type ArithmeticInstructionTable = [fn(&mut N64, u32); 64];

// Called once by the shared interpreter/native dispatch wrapper. Only the
// immediately preceding COP1 arithmetic result creates the extra interlock.
pub fn begin_instruction_timing(n64: &mut N64, _opcode: u32) {
    n64.fpu_mut().begin_instruction();
}

#[inline(always)]
fn check_fpu_access(n64: &N64) -> bool {
    n64.cp0_register(MmuRegister::Status) & (MmuStatus::CU1 as u64) != 0
}

fn handle_fpu_op<F>(n64: &mut N64, opcode: u32, op: F)
where
    F: FnOnce(&mut N64, u32),
{
    if !check_fpu_access(n64) {
        return unusable(n64, opcode);
    }

    op(n64, opcode);
}

fn handle_wide_fpu_move(n64: &mut N64, opcode: u32, op: impl FnOnce(&mut N64, u32)) {
    handle_fpu_op(n64, opcode, |device, opcode| {
        if device.cp0().status().wide_instructions_allowed() {
            op(device, opcode);
        } else {
            crate::mmu::reserved(device, opcode);
        }
    });
}

fn handle_fpu_mem_op<F>(n64: &mut N64, opcode: u32, access_type: AccessType, alignment: u64, op: F)
where
    F: FnOnce(&mut N64, u64, bool),
{
    if !check_fpu_access(n64) {
        return unusable(n64, opcode);
    }

    let addr = cpu::effective_address(n64, opcode);

    if !crate::bits::is_aligned(addr, alignment) {
        exceptions::address_error_exception(n64, addr, access_type);
        return;
    }

    let (phys_addr, cached, err) = memory::translate_address(n64, addr, access_type);

    if err {
        return;
    }

    op(
        n64,
        memory::reverse_endian_address(n64, phys_addr, alignment),
        cached,
    );
}

#[inline(always)]
pub(crate) fn is_fr_mode(n64: &N64) -> bool {
    n64.cp0_register(MmuRegister::Status) & (MmuStatus::FR as u64) != 0
}

pub fn lwc1(n64: &mut N64, opcode: u32) {
    handle_fpu_mem_op(
        n64,
        opcode,
        AccessType::Read,
        AccessSize::Word as u64,
        |dev, addr, cached| {
            let value = memory::read_data(dev, addr, AccessSize::Word, cached);

            set_fpr_single(dev, rcp::ft(opcode) as usize, f32::from_bits(value), false)
        },
    );
}

pub fn ldc1(n64: &mut N64, opcode: u32) {
    handle_fpu_mem_op(
        n64,
        opcode,
        AccessType::Read,
        AccessSize::Dword as u64,
        |dev, addr, cached| {
            let value = memory::read_doubleword(dev, addr, cached);

            set_fpr_double(dev, rcp::ft(opcode) as usize, f64::from_bits(value))
        },
    );
}

pub fn swc1(n64: &mut N64, opcode: u32) {
    handle_fpu_mem_op(
        n64,
        opcode,
        AccessType::Write,
        AccessSize::Word as u64,
        |dev, addr, cached| {
            let value = get_fpr_single(dev, rcp::ft(opcode) as usize).to_bits();
            memory::write_data(dev, addr, value, u32::MAX, cached);
        },
    );
}

pub fn sdc1(n64: &mut N64, opcode: u32) {
    handle_fpu_mem_op(
        n64,
        opcode,
        AccessType::Write,
        AccessSize::Dword as u64,
        |dev, addr, cached| {
            let value = get_fpr_double(dev, rcp::ft(opcode) as usize).to_bits();
            memory::write_data(dev, addr, (value >> u32::BITS) as u32, u32::MAX, cached);
            memory::write_data(
                dev,
                addr + AccessSize::Word as u64,
                value as u32,
                u32::MAX,
                cached,
            );
        },
    );
}

fn mfc1(n64: &mut N64, opcode: u32) {
    handle_fpu_op(n64, opcode, |dev, op| {
        let value = get_fpr_single(dev, rcp::fs(op) as usize);
        dev.cpu_mut()
            .set_gpr(cpu::rt(op) as usize, cpu::se32(value.to_bits() as i32));
    });
}

fn dmfc1(n64: &mut N64, opcode: u32) {
    handle_wide_fpu_move(n64, opcode, |dev, op| {
        let value = get_fpr_double(dev, rcp::fs(op) as usize);
        dev.cpu_mut().set_gpr(cpu::rt(op) as usize, value.to_bits());
    });
}

fn cfc1(n64: &mut N64, opcode: u32) {
    handle_fpu_op(n64, opcode, |dev, op| {
        let value = cpu::se32(get_control_registers_fpu(dev, rcp::fs(op)) as i32);
        dev.cpu_mut().set_gpr(cpu::rt(op) as usize, value);
    });
}

fn mtc1(n64: &mut N64, opcode: u32) {
    handle_fpu_op(n64, opcode, |dev, op| {
        let value = f32::from_bits(dev.cpu().gpr(cpu::rt(op) as usize) as u32);

        set_fpr_single(dev, rcp::fs(op) as usize, value, false)
    });
}

fn dmtc1(n64: &mut N64, opcode: u32) {
    handle_wide_fpu_move(n64, opcode, |dev, op| {
        let value = f64::from_bits(dev.cpu().gpr(cpu::rt(op) as usize));

        set_fpr_double(dev, rcp::fs(op) as usize, value)
    });
}

fn ctc1(n64: &mut N64, opcode: u32) {
    handle_fpu_op(n64, opcode, |dev, op| {
        set_control_registers_fpu(dev, rcp::fs(op), dev.cpu().gpr(cpu::rt(op) as usize) as u32)
    });
}

fn execute_fpu_b(n64: &mut N64, opcode: u32) {
    handle_fpu_op(n64, opcode, |dev, op| {
        match dev
            .fpu()
            .instructions
            .b_instrs
            .get(cpu::rt(op) as usize)
            .copied()
        {
            Some(handler) => handler(dev, op),
            None => unimplemented(dev),
        }
    });
}

fn execute_fpu_format(n64: &mut N64, opcode: u32, format: dispatch::Format) {
    handle_fpu_op(n64, opcode, |device, opcode| {
        let handler =
            format.instructions(device.fpu())[cpu::Instruction::decode(opcode).function()];
        handler(device, opcode);
    });
}

fn execute_fpu_s(n64: &mut N64, opcode: u32) {
    execute_fpu_format(n64, opcode, dispatch::Format::Single);
}

fn execute_fpu_d(n64: &mut N64, opcode: u32) {
    execute_fpu_format(n64, opcode, dispatch::Format::Double);
}

fn execute_fpu_l(n64: &mut N64, opcode: u32) {
    execute_fpu_format(n64, opcode, dispatch::Format::Long);
}

fn execute_fpu_w(n64: &mut N64, opcode: u32) {
    execute_fpu_format(n64, opcode, dispatch::Format::Word);
}

fn unsupported_control_move(n64: &mut N64, opcode: u32) {
    handle_fpu_op(n64, opcode, |dev, _op| unimplemented(dev));
}

fn unusable(n64: &mut N64, _opcode: u32) {
    crate::mmu::add_cycles(n64, 1);

    exceptions::cop_unusable_exception(n64, MmuCause::CE1 as u64)
}

pub fn reserved(n64: &mut N64, opcode: u32) {
    handle_fpu_op(n64, opcode, |dev, op| {
        // Reserved functions share the FP decode pipeline. Defined arithmetic
        // operations in integer-source formats get as far as format validation.
        let format = Cop1Opcode::from_encoding(cpu::rs(op) as usize);
        let function = FpuFunction::from_encoding(cpu::Instruction::decode(op).function());
        let integer_source = matches!(format, Some(Cop1Opcode::Word | Cop1Opcode::Long));
        let arithmetic = function.is_some_and(|function| {
            function as u8 <= FpuFunction::FloorW as u8
                || matches!(function, FpuFunction::CvtW | FpuFunction::CvtL)
        });
        let latency = if integer_source && arithmetic { 2 } else { 1 };

        crate::mmu::add_cycles(dev, latency + 1);

        unimplemented(dev)
    });
}

// Decode-side lookahead only: no guest fault, timing, or address-register writes.
pub(crate) fn translate_instruction_speculatively(n64: &N64, address: u64) -> Option<(u64, bool)> {
    memory::peek_translation(n64, address)
        .map(|(physical, cached)| (memory::reverse_endian_address(n64, physical, 4), cached))
}

fn peek_instruction_word(n64: &N64, address: u64) -> Option<u32> {
    // Direct backing-buffer reads only. A normal "fast" bus handler may still
    // acknowledge devices, alter registers or charge cycles.
    let (buffer, offset) = match address {
        0..=0x03ef_ffff => {
            if address >= crate::rdram::RDRAM_SIZE as u64 {
                return Some(0);
            }

            (&n64.rdram.mem, address as usize)
        }
        0x0400_0000..=0x0403_ffff => (
            &n64.rsp.memory.mem,
            (address as usize) & crate::rsp::RSP_MEM_MASK,
        ),
        0x1000_0000..=0x1fbf_ffff => (
            &n64.cartridge.rom,
            (address as usize) & crate::cartridge::CART_MASK,
        ),
        _ => return None,
    };

    let bytes: [u8; 4] = buffer
        .data
        .get(offset..offset.checked_add(4)?)?
        .try_into()
        .ok()?;

    Some(if buffer.access.is_little_endian {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    })
}

pub fn raise_exception(n64: &mut N64) {
    // Cause.CE reflects the coprocessor decode already in the following
    // pipeline stage, as exercised by CTC1 followed immediately by MFC1/MFC2.
    let next_virtual = if n64.cpu.branch_state.state == cpu::State::DelaySlotTaken {
        n64.cpu.branch_state.pc
    } else {
        n64.cpu.execution.pc.wrapping_add(4)
    };

    let translation = translate_instruction_speculatively(n64, next_virtual);
    let next_opcode = translation.map_or(0, |(next_address, cached)| {
        let line_index = ((next_virtual >> 5) & 0x1ff) as usize;
        let line = &n64.memory.caches.icache[line_index];

        if cached
            && line.valid
            && (line.tag & 0x1fff_fffc) as u64
                == crate::bits::align_down(next_address, crate::cache::TAG_PAGE_BYTES)
        {
            line.words[((next_address >> 2) & 7) as usize]
        } else {
            peek_instruction_word(n64, next_address).unwrap_or(0)
        }
    });
    let coprocessor = match next_opcode >> 26 {
        0x10..=0x13 => ((next_opcode >> 26) - 0x10) as u64,
        _ => 0,
    };

    exceptions::floating_point_exception(n64);
    n64.update_cp0_register(MmuRegister::Cause, |cause| {
        (cause & !(3 << 28)) | (coprocessor << 28)
    });
}

pub fn unimplemented(n64: &mut N64) {
    n64.fpu_mut().control.mark_unimplemented();
    raise_exception(n64);
}

fn handle_fcr31(n64: &mut N64, data: u32) {
    n64.fpu_mut().control.fcr31 = data & FCR31_WRITE_MASK;

    if n64.fpu().exception_pending() {
        raise_exception(n64)
    }
}

fn set_control_registers_fpu(n64: &mut N64, index: u32, data: u32) {
    match index {
        0 => (),
        31 => {
            handle_fcr31(n64, data);
        }
        _ => (),
    }
}

fn get_control_registers_fpu(n64: &N64, index: u32) -> u32 {
    let fpu = n64.fpu();

    match index {
        0 => fpu.control.fcr0,
        31 => fpu.control.fcr31,
        // Undocumented control registers have no stable hardware readback.
        _ => 0,
    }
}

pub fn set_fpr_single(n64: &mut N64, index: usize, value: f32, clear_high: bool) {
    let wide = is_fr_mode(n64);
    n64.fpu_mut().set_single(index, value, clear_high, wide);
}

pub fn get_fpr_single(n64: &N64, index: usize) -> f32 {
    n64.fpu().single(index, is_fr_mode(n64))
}

pub fn set_fpr_double(n64: &mut N64, index: usize, value: f64) {
    let wide = is_fr_mode(n64);
    n64.fpu_mut().set_double(index, value, wide);
}

pub fn get_fpr_double(n64: &N64, index: usize) -> f64 {
    n64.fpu().double(index, is_fr_mode(n64))
}

pub fn set_fgr_registers(n64: &mut N64, status_reg: u64) {
    n64.fpu_mut()
        .set_register_mode(status_reg & MmuStatus::FR as u64 != 0);
}
