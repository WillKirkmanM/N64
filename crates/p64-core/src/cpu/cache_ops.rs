use super::*;

pub(super) fn cache(n64: &mut N64, opcode: u32) {
    let (phys_address, _, err) = memory::translate_address(
        n64,
        n64.cpu.registers.gpr[rs(opcode) as usize].wrapping_add(se16(imm(opcode) as i16)),
        memory::AccessType::Read,
    );
    if err {
        return;
    }

    let dcache_line = ((phys_address >> 4) & 0x1FF) as usize;
    let icache_line = ((phys_address >> 5) & 0x1FF) as usize;
    match rt(opcode) {
        0x00 => {
            //icache index invalidate
            n64.memory.caches.icache[icache_line].valid = false
        }
        0x04 => {
            //icache load tag
            let mut valid = 0;
            if n64.memory.caches.icache[icache_line].valid {
                valid = 1
            }
            memory::masked_write_64(
                &mut n64.cpu.mmu.regs[MmuRegister::TagLo as usize],
                valid << 7,
                0x80,
            );
            memory::masked_write_64(&mut n64.cpu.mmu.regs[MmuRegister::TagLo as usize], 0, 0x40);
            memory::masked_write_64(
                &mut n64.cpu.mmu.regs[MmuRegister::TagLo as usize],
                (n64.memory.caches.icache[icache_line].tag >> 4) as u64,
                0xFFFFF00,
            )
        }
        0x08 => {
            //icache store tag
            n64.memory.caches.icache[icache_line].valid =
                (n64.cpu.mmu.regs[MmuRegister::TagLo as usize] & 0x80) >> 7 != 0;
            n64.memory.caches.icache[icache_line].tag =
                ((n64.cpu.mmu.regs[MmuRegister::TagLo as usize] & 0xFFFFF00) << 4) as u32
        }
        0x10 => {
            //icache hit invalidate
            if cache::icache_hit(n64, icache_line, phys_address) {
                n64.memory.caches.icache[icache_line].valid = false
            }
        }
        0x14 => {
            //icache fill
            cache::icache_fill(n64, icache_line, phys_address)
        }
        0x18 => {
            //icache hit write back
            if cache::icache_hit(n64, icache_line, phys_address) {
                cache::icache_writeback(n64, icache_line)
            }
        }
        0x01 => {
            //dcache index write back invalidate
            if n64.memory.caches.dcache[dcache_line].dirty
                && n64.memory.caches.dcache[dcache_line].valid
            {
                cache::dcache_writeback(n64, dcache_line)
            }
            n64.memory.caches.dcache[dcache_line].valid = false
        }
        0x05 => {
            //dcache index load tag
            let mut valid = 0;
            let mut dirty = 0;
            if n64.memory.caches.dcache[dcache_line].valid {
                valid = 1
            }
            if n64.memory.caches.dcache[dcache_line].dirty {
                dirty = 1
            }
            memory::masked_write_64(
                &mut n64.cpu.mmu.regs[MmuRegister::TagLo as usize],
                valid << 7,
                0x80,
            );
            memory::masked_write_64(
                &mut n64.cpu.mmu.regs[MmuRegister::TagLo as usize],
                dirty << 6,
                0x40,
            );
            memory::masked_write_64(
                &mut n64.cpu.mmu.regs[MmuRegister::TagLo as usize],
                (n64.memory.caches.dcache[dcache_line].tag >> 4) as u64,
                0xFFFFF00,
            )
        }
        0x09 => {
            //dcache index store tag
            n64.memory.caches.dcache[dcache_line].valid =
                (n64.cpu.mmu.regs[MmuRegister::TagLo as usize] & 0x80) >> 7 != 0;
            n64.memory.caches.dcache[dcache_line].dirty =
                (n64.cpu.mmu.regs[MmuRegister::TagLo as usize] & 0x40) >> 6 != 0;
            n64.memory.caches.dcache[dcache_line].tag =
                ((n64.cpu.mmu.regs[MmuRegister::TagLo as usize] & 0xFFFFF00) << 4) as u32
        }
        0x0D => {
            //dcache create dirty exclusive
            if !cache::dcache_hit(n64, dcache_line, phys_address)
                && n64.memory.caches.dcache[dcache_line].dirty
            {
                cache::dcache_writeback(n64, dcache_line)
            }
            n64.memory.caches.dcache[dcache_line].tag = (phys_address & !0xFFF) as u32;
            n64.memory.caches.dcache[dcache_line].valid = true;
            n64.memory.caches.dcache[dcache_line].dirty = true
        }
        0x11 => {
            //dcache hit invalidate
            if cache::dcache_hit(n64, dcache_line, phys_address) {
                n64.memory.caches.dcache[dcache_line].valid = false;
                n64.memory.caches.dcache[dcache_line].dirty = false
            }
        }
        0x15 => {
            //dcache hit write back invalidate
            if cache::dcache_hit(n64, dcache_line, phys_address) {
                if n64.memory.caches.dcache[dcache_line].dirty {
                    cache::dcache_writeback(n64, dcache_line)
                }
                n64.memory.caches.dcache[dcache_line].valid = false
            }
        }
        0x19 => {
            //dcache hit write back
            if cache::dcache_hit(n64, dcache_line, phys_address)
                && n64.memory.caches.dcache[dcache_line].dirty
            {
                cache::dcache_writeback(n64, dcache_line)
            }
        }
        _ => {
            panic!("unknown cache code {:#01x}", rt(opcode))
        }
    }
}
