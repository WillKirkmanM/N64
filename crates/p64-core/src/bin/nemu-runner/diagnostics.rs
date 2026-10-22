use p64_core::{mmu::MmuRegister, recompiler::NativeReject, N64};

pub(super) fn report(device: &N64) {
    let diagnostics = &device.execution_engine.stats.diagnostics;
    eprintln!(
        "Native successful block calls by retired guest length (index=length): {:?}",
        diagnostics.native_block_length_counts
    );
    eprintln!(
        "Native successful branch block calls by retired guest length (index=length): {:?}",
        diagnostics.native_branch_length_counts
    );
    eprintln!(
        "Scalar opcode counts (index=primary): {:?}",
        diagnostics.scalar_opcode_counts
    );
    for reason in NativeReject::ALL {
        eprintln!(
            "Native reject {reason:?}={}",
            diagnostics.native_reject_counts[reason as usize]
        );
    }

    eprintln!(
        "Final CP0 Wired={} Random={}",
        device.cp0_register(MmuRegister::Wired),
        device.cp0_register(MmuRegister::Random)
    );
}
