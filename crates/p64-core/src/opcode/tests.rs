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
