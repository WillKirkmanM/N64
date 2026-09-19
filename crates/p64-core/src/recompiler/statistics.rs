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

impl ExecutionStats {
    #[inline]
    pub(crate) fn record_decoded(&mut self, opcode: u32, backend: Backend) {
        #[cfg(feature = "accuracy-diagnostics")]
        {
            let primary = crate::cpu::Instruction::decode(opcode).primary() as usize;
            self.diagnostics.scalar_opcode_counts[primary] += 1;
        }
        #[cfg(not(feature = "accuracy-diagnostics"))]
        let _ = opcode;

        if backend != Backend::Interpreter {
            self.fallback_executions += 1;
        }
    }

    #[inline]
    pub(super) fn record_reject(&mut self, reason: NativeReject) {
        #[cfg(feature = "accuracy-diagnostics")]
        {
            self.diagnostics.native_reject_counts[reason as usize] += 1;
        }
        #[cfg(not(feature = "accuracy-diagnostics"))]
        let _ = reason;
    }

    #[inline]
    pub(super) fn record_block(&mut self, retired: usize) {
        self.native_executions += retired as u64;
        self.native_block_executions += 1;
        #[cfg(feature = "accuracy-diagnostics")]
        {
            self.diagnostics.native_block_length_counts[retired] += 1;
        }
    }
}
