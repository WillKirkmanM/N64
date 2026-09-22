use super::{
    run::{insert_run, RunKind},
    MAX_BLOCK_ENTRIES, MAX_SCALAR_ENTRIES,
};
use crate::recompiler::{supported_branch_opcode, supported_memory_opcode, supported_opcode};
use std::collections::BTreeSet;

const INSTRUCTION_BYTES: usize = 4;
const CACHE_LINE_BYTES: usize = 32;
