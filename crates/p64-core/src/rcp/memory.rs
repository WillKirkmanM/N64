use super::check_rcp_access;
use crate::{cpu, exceptions, memory::{self, AccessSize, AccessType}, mmu::{self, MmuCause}, N64};

const WORD_BYTES: u64 = 4;
const DOUBLEWORD_BYTES: u64 = 8;

