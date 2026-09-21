#[path = "block_metadata.rs"]
mod block_metadata;
pub(super) use block_metadata::BlockMetadata;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Backend {
    #[default]
    Interpreter,
    Jit,
    Aot,
}

pub(crate) type NativeCodeRecord = (Vec<u32>, Vec<u8>);

pub(super) type NativeFunction = unsafe extern "C" fn(*mut u64);
pub(super) type NativeBranchFunction = unsafe extern "C" fn(*mut u64) -> u64;
pub(super) type NativeMemoryFunction = unsafe extern "C" fn(*mut NativeMemoryContext<'static>);

/// Only the originating engine can execute this handle; replacement invalidates it.
#[derive(Clone, Copy, Debug)]
pub struct NativeInstruction {
    pub(super) function: NativeFunction,
    pub(super) generation: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct NativeBlock {
    pub(super) function: NativeFunction,
    pub(super) generation: u64,
    pub(super) metadata: BlockMetadata,
}

impl NativeBlock {
    pub fn len(&self) -> usize {
        self.metadata.instruction_count
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn first_opcode(&self) -> u32 {
        self.metadata.first_opcode
    }

    pub fn requires_wide(&self) -> bool {
        self.metadata.wide
    }
}

/// Integer prefix and branch comparison; PC and delay state stay with the dispatcher.
#[derive(Clone, Copy, Debug)]
pub struct NativeBranchBlock {
    pub(super) function: NativeBranchFunction,
    pub(super) generation: u64,
    pub(super) metadata: BlockMetadata,
    pub(super) branch_opcode: u32,
}

impl NativeBranchBlock {
    pub fn len(&self) -> usize {
        self.metadata.instruction_count
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn first_opcode(&self) -> u32 {
        self.metadata.first_opcode
    }

    pub fn branch_opcode(&self) -> u32 {
        self.branch_opcode
    }

    pub fn branch_index(&self) -> usize {
        self.metadata.instruction_count - 1
    }

    pub fn branch_offset(&self) -> u64 {
        self.branch_index() as u64 * 4
    }

    pub fn requires_wide(&self) -> bool {
        self.metadata.wide
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum NativeMemoryExit {
    Complete = 0,
    Address = 1,
    CacheMiss = 2,
}

/// Borrows both arrays exclusively for a native call; generated code retains no pointers.
#[repr(C)]
pub struct NativeMemoryContext<'a> {
    pub(super) gpr: *mut u64,
    pub(super) dcache: *mut crate::cache::DCache,
    pub start_pc: u64,
    pub retired_count: u64,
    pub extra_cycles: u64,
    pub last_data_address: u64,
    pub pending_load: u8,
    pub side_exit: NativeMemoryExit,
    pub(super) borrow:
        std::marker::PhantomData<(&'a mut [u64; 32], &'a mut [crate::cache::DCache; 512])>,
}

impl<'a> NativeMemoryContext<'a> {
    pub fn new(
        gpr: &'a mut [u64; 32],
        dcache: &'a mut [crate::cache::DCache; 512],
        start_pc: u64,
        pending_load: u8,
        last_data_address: u64,
    ) -> Self {
        Self {
            gpr: gpr.as_mut_ptr(),
            dcache: dcache.as_mut_ptr(),
            start_pc,
            retired_count: 0,
            extra_cycles: 0,
            last_data_address,
            pending_load,
            side_exit: NativeMemoryExit::Complete,
            borrow: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NativeMemoryBlock {
    pub(super) function: NativeMemoryFunction,
    pub(super) generation: u64,
    pub(super) metadata: BlockMetadata,
}

impl NativeMemoryBlock {
    pub fn len(&self) -> usize {
        self.metadata.instruction_count
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn requires_wide(&self) -> bool {
        self.metadata.wide
    }
}

/// Diagnostic-only dispatch refusals. A single attempt may refuse more than one
/// candidate (memory, branch, integer), so counts are not instruction totals.
#[derive(Clone, Copy, Debug)]
#[repr(usize)]
pub enum NativeReject {
    MissingCandidate,
    BranchState,
    InvalidFetch,
    ReversedFetch,
    PipelineActive,
    PipelineRefill,
    PipelineStale,
    Lookahead,
    BranchTarget,
    PendingWired,
    CountWrite,
    RandomRing,
    WidePrivilege,
    EventBudget,
    Overflow,
    InvalidLength,
    MemoryPrivilege,
    StaleCode,
    SelfBranch,
}

#[cfg(feature = "accuracy-diagnostics")]
impl NativeReject {
    pub const ALL: [Self; 19] = [
        Self::MissingCandidate,
        Self::BranchState,
        Self::InvalidFetch,
        Self::ReversedFetch,
        Self::PipelineActive,
        Self::PipelineRefill,
        Self::PipelineStale,
        Self::Lookahead,
        Self::BranchTarget,
        Self::PendingWired,
        Self::CountWrite,
        Self::RandomRing,
        Self::WidePrivilege,
        Self::EventBudget,
        Self::Overflow,
        Self::InvalidLength,
        Self::MemoryPrivilege,
        Self::StaleCode,
        Self::SelfBranch,
    ];
}
