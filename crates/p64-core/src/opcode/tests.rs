use super::*;

#[test]
fn opcode_decoders_round_trip_known_encodings_and_reject_truncation() {
    for raw in (0..=255).chain([256, 512, usize::MAX]) {
        let decoded = [
            PrimaryOpcode::from_encoding(raw).map(|op| op as usize),
            SpecialOpcode::from_encoding(raw).map(|op| op as usize),
            RegImmOpcode::from_encoding(raw).map(|op| op as usize),
            Cp0Function::from_encoding(raw).map(|op| op as usize),
            Cop1Opcode::from_encoding(raw).map(|op| op as usize),
            FpuBranchOpcode::from_encoding(raw).map(|op| op as usize),
            FpuFunction::from_encoding(raw).map(|op| op as usize),
            Cop2Opcode::from_encoding(raw).map(|op| op as usize),
            VectorLoadOpcode::from_encoding(raw).map(|op| op as usize),
            VectorStoreOpcode::from_encoding(raw).map(|op| op as usize),
            VectorOpcode::from_encoding(raw).map(|op| op as usize),
        ];

        for encoding in decoded.into_iter().flatten() {
            assert_eq!(encoding, raw, "decoder truncated or changed encoding {raw}");
        }

        if raw > u8::MAX as usize {
            assert!(decoded.into_iter().all(|op| op.is_none()));
        }
    }
}

#[test]
fn coprocessor_operation_bits_preserve_aliases_without_accepting_invalid_fields() {
    for raw in 16..32 {
        assert_eq!(Cp0Opcode::from_encoding(raw), Some(Cp0Opcode::Co));
        assert_eq!(
            RspCop2Opcode::from_encoding(raw),
            Some(RspCop2Opcode::Vector)
        );
    }

    for raw in [32, 48, 256, usize::MAX] {
        assert_eq!(Cp0Opcode::from_encoding(raw), None);
        assert_eq!(RspCop2Opcode::from_encoding(raw), None);
    }

    assert_eq!(Cp0Opcode::from_encoding(0), Some(Cp0Opcode::Mfc0));
    assert_eq!(RspCop2Opcode::from_encoding(6), Some(RspCop2Opcode::Ctc2));
    assert_eq!(Cp0Opcode::from_encoding(3), None);
    assert_eq!(RspCop2Opcode::from_encoding(1), None);
}

#[test]
fn hardware_fields_distinguish_reserved_slots_formats_and_vector_aliases() {
    assert_eq!(
        PrimaryOpcode::from_encoding(0x09),
        Some(PrimaryOpcode::Addiu)
    );
    assert_eq!(
        PrimaryOpcode::from_encoding(0x10),
        Some(PrimaryOpcode::Cop0)
    );
    assert_eq!(
        PrimaryOpcode::from_encoding(0x32),
        Some(PrimaryOpcode::Lwc2)
    );
    assert_eq!(SpecialOpcode::from_encoding(1), None);
    assert_eq!(RegImmOpcode::from_encoding(4), None);
    assert_eq!(Cp0Function::from_encoding(0x10), Some(Cp0Function::Rfe));
    assert_eq!(Cp0Function::from_encoding(0x18), Some(Cp0Function::Eret));
    assert_eq!(Cop1Opcode::from_encoding(0x11), Some(Cop1Opcode::Double));
    assert_eq!(FpuFunction::from_encoding(0x3f), Some(FpuFunction::CNgt));
    assert_eq!(VectorOpcode::from_encoding(0x37), Some(VectorOpcode::Vnop));
    assert_eq!(VectorOpcode::from_encoding(0x3f), Some(VectorOpcode::Vnull));
}
