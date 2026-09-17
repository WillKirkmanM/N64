use crate::recompiler::{Backend, NativeReject};
#[cfg(feature = "accuracy-diagnostics")]
#[path = "diagnostics.rs"]
mod diagnostics;
#[cfg(feature = "accuracy-diagnostics")]
pub use diagnostics::ExecutionDiagnostics;

#[derive(Default)]
pub struct ExecutionStats {
    pub native_executions: u64,
    pub native_block_executions: u64,
    pub native_branch_executions: u64,
    pub native_memory_executions: u64,
    pub fallback_executions: u64,
    #[cfg(feature = "accuracy-diagnostics")]
    pub diagnostics: ExecutionDiagnostics,
}

impl ExecutionStats {}
