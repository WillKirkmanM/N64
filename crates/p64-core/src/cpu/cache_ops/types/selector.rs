use super::CacheOperation;

const BANK_MASK: usize = 0b11;

#[derive(Clone, Copy)]
pub(in crate::cpu::cache_ops) enum CacheBank {
    Instruction,
    Data,
}

pub(in crate::cpu::cache_ops) struct CacheSelector {
    pub bank: CacheBank,
    pub operation: CacheOperation,
}

impl CacheSelector {
    pub fn decode(encoding: usize) -> Option<Self> {
        let bank = match encoding & BANK_MASK {
            0 => CacheBank::Instruction,
            1 => CacheBank::Data,
            _ => return None,
        };

        Some(Self {
            bank,
            operation: CacheOperation::decode(encoding)?,
        })
    }
}

#[cfg(test)]
mod tests;
