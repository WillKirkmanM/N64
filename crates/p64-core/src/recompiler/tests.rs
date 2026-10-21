use super::*;
mod oracle;
use oracle::{assert_cache_equal, memory_oracle, next_random, reference};

#[test]
fn native_integer_instructions_match_reference_with_aliases_and_edge_bits() {
    let mut engine = Engine::jit();
    let mut state = 0x9133_5678_ffee_abcd_u64;
    let functions = [
        0, 2, 3, 4, 6, 7, 20, 22, 23, 33, 35, 36, 37, 38, 39, 42, 43, 45, 47, 56, 58, 59, 60, 62,
        63,
    ];
    let edges = [
        0,
        1,
        31,
        32,
        63,
        64,
        u64::MAX,
        0x80000000,
        0x8000000000000000,
        0x7fffffff,
    ];

    for iteration in 0..1500_u32 {
        let rs = iteration % 4;
        let rt = (iteration / 4) % 4;
        let rd = (iteration / 16) % 4;
        let sa = (iteration / 64) % 32;
        let mut initial = [0; 32];

        for register in initial.iter_mut().skip(1) {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *register = if iteration < 100 {
                edges[iteration as usize % edges.len()]
            } else {
                state
            };
        }

        let special = functions
            .iter()
            .map(|f| (rs << 21) | (rt << 16) | (rd << 11) | (sa << 6) | f);
        let immediate = [9, 10, 11, 12, 13, 14, 15, 25]
            .into_iter()
            .map(|p| (p << 26) | (rs << 21) | (rt << 16) | ((state as u32) & 65535));

        for opcode in special.chain(immediate) {
            let mut expected = initial;
            let mut actual = initial;
            reference(opcode, &mut expected);

            assert!(engine.try_execute(opcode, &mut actual), "{opcode:08x}");
            assert_eq!(
                actual, expected,
                "opcode {opcode:08x}, iteration {iteration}"
            );
        }
    }

    assert_eq!(engine.stats.native_executions, 49_500);
}

#[test]
fn unsupported_and_aot_misses_do_not_modify_registers() {
    let mut engine = Engine::aot([0x24220001]);
    let mut registers = [0xdeadbeef; 32];
    let before = registers;

    for opcode in [0x8c220000, 0x20220001, 0x08000000, 0x24220002] {
        assert!(!engine.try_execute(opcode, &mut registers));
        assert_eq!(registers, before);
    }

    assert!(engine.try_execute(0x24220001, &mut registers));
    assert_eq!(engine.compiled_instructions(), 1);
}

#[test]
fn aot_rejects_altered_native_bytes() {
    let opcode = 0x24220001;
    let mut code = compile_opcode(opcode).unwrap();

    assert!(Engine::from_aot_codes(vec![(opcode, code.clone())]).is_ok());
    code[0] ^= 1;

    assert!(Engine::from_aot_codes(vec![(opcode, code)]).is_err());
}

#[test]
fn prepared_handles_are_safe_after_engine_replacement() {
    let mut engine = Engine::jit();
    let old_generation = engine.generation();
    let instruction = engine.prepare(0x24220001).unwrap();
    let mut registers = [0; 32];
    registers[1] = 17;

    assert!(engine.execute_prepared(instruction, &mut registers));
    assert_eq!(registers[2], 18);
    assert_eq!(engine.stats.native_executions, 1);
    engine = Engine::jit();
    assert_ne!(engine.generation(), old_generation);
    let before = registers;

    assert!(!engine.execute_prepared(instruction, &mut registers));
    assert_eq!(registers, before);
    let fresh = engine.prepare(0x24220002).unwrap();

    assert!(engine.execute_prepared(fresh, &mut registers));
    assert_eq!(registers[2], 19);
    assert_eq!(engine.stats.native_executions, 1);
}

#[test]
fn preparing_aot_miss_never_generates_code() {
    let mut engine = Engine::aot([0x24220001]);

    assert!(engine.prepare(0x24220001).is_some());
    assert!(engine.prepare(0x24220002).is_none());
    assert_eq!(engine.compiled_instructions(), 1);
    assert_eq!(engine.stats.native_executions, 0);
}

#[test]
fn register_resident_native_blocks_match_sequential_interpreter_semantics() {
    let mut engine = Engine::jit();
    let mut random = 0x784c_5466_f031_a992_u64;
    let functions = [
        0, 2, 3, 4, 6, 7, 20, 22, 23, 33, 35, 36, 37, 38, 39, 42, 43, 45, 47, 56, 58, 59, 60, 62,
        63,
    ];
    let immediate = [9, 10, 11, 12, 13, 14, 15, 25];

    for iteration in 0..4000 {
        let mut initial = [0; 32];

        for value in initial.iter_mut().skip(1) {
            next_random(&mut random);
            *value = random;
        }

        let length = 2 + iteration % 7;
        let mut opcodes = Vec::new();

        for _ in 0..length {
            next_random(&mut random);
            let register_mask = if iteration & 1 == 0 { 3 } else { 31 };

            let rs = random as u32 & register_mask;
            let rt = (random >> 8) as u32 & register_mask;
            let rd = (random >> 16) as u32 & register_mask;
            let sa = (random >> 24) as u32 & 31;
            let kind = (random >> 32) as usize % 33;
            let opcode = if kind < functions.len() {
                (rs << 21) | (rt << 16) | (rd << 11) | (sa << 6) | functions[kind]
            } else {
                (immediate[kind - functions.len()] << 26)
                    | (rs << 21)
                    | (rt << 16)
                    | ((random >> 40) as u32 & 65535)
            };

            opcodes.push(opcode);
        }

        let mut expected = initial;
        let mut actual = initial;

        for opcode in &opcodes {
            reference(*opcode, &mut expected);
        }

        let block = engine.prepare_block(&opcodes).unwrap();

        assert_eq!(block.len(), opcodes.len());
        assert_eq!(block.first_opcode(), opcodes[0]);
        assert_eq!(
            block.requires_wide(),
            opcodes.iter().any(|op| is_wide_opcode(*op))
        );
        assert!(engine.execute_prepared_block(block, &mut actual));
        assert_eq!(
            actual, expected,
            "block {opcodes:08x?}, iteration {iteration}"
        );
    }
}

#[test]
fn aot_native_blocks_are_validated_and_stale_handles_are_rejected() {
    let opcodes = vec![0x24210001, 0x00211025, 0x24420002];
    let code = compile_block(&opcodes).unwrap();
    let mut engine = Engine::from_aot_blocks(vec![(opcodes.clone(), code.clone())]).unwrap();

    assert_eq!(engine.compiled_blocks(), 1);
    let block = engine.prepare_block(&opcodes).unwrap();
    let mut registers = [0; 32];
    registers[1] = 10;

    assert!(engine.execute_prepared_block(block, &mut registers));
    assert_eq!(registers[1], 11);
    assert_eq!(registers[2], 13);
    assert_eq!(engine.stats.native_executions, 3);
    assert!(engine.prepare_block(&[0x24210001, 0x24210002]).is_none());
    engine = Engine::jit();
    let before = registers;

    assert!(!engine.execute_prepared_block(block, &mut registers));
    assert_eq!(before, registers);
    let mut altered = code;
    altered[0] ^= 1;

    assert!(Engine::from_aot_blocks(vec![(opcodes, altered)]).is_err());
    assert!(compile_block(&[0xD4220000, 0x24210001]).is_none());
}

#[test]
fn folded_self_updates_preserve_word_sign_extension_and_doubleword_wrap() {
    let cases: Vec<(u32, u64, Vec<i16>, u64)> = vec![
        (9, 0xABCD_1234_7FFF_FFFF, vec![1, 1, -1, -1], 0x7FFF_FFFF),
        (9, 0xABCD_1234_FFFF_FFFF, vec![1, 0, 0, 0], 0),
        (
            9,
            0xABCD_1234_8000_0000,
            vec![-1, 1, -32768, 32767],
            0x7FFF_FFFF,
        ),
        (
            9,
            0x1234_5678_7FFF_FFFF,
            vec![32767; 8],
            0xFFFF_FFFF_8003_FFF7,
        ),
        (
            9,
            0x1234_5678_0000_0000,
            vec![-32768; 8],
            0xFFFF_FFFF_FFFC_0000,
        ),
        (25, u64::MAX, vec![1, 1], 1),
        (25, 0, vec![-1, -1], u64::MAX - 1),
        (25, u64::MAX, vec![-32768; 8], 0xFFFF_FFFF_FFFB_FFFF),
        (
            25,
            0x8000_0000_0000_0000,
            vec![32767; 8],
            0x8000_0000_0003_FFF8,
        ),
    ];

    for (primary, initial, immediates, expected) in cases {
        for register in [1_u32, 31] {
            let opcodes: Vec<u32> = immediates
                .iter()
                .map(|immediate| {
                    (primary << 26)
                        | (register << 21)
                        | (register << 16)
                        | (*immediate as u16 as u32)
                })
                .collect();
            let mut engine = Engine::jit();
            let block = engine.prepare_block(&opcodes).unwrap();
            let mut gpr = [0; 32];
            gpr[register as usize] = initial;
            let mut expected_registers = gpr;
            expected_registers[register as usize] = expected;

            assert!(engine.execute_prepared_block(block, &mut gpr));
            assert_eq!(
                gpr, expected_registers,
                "primary {primary}, register {register}"
            );
            assert_eq!(block.len(), immediates.len());
            assert_eq!(engine.stats.native_executions, immediates.len() as u64);
            assert_eq!(engine.stats.native_block_executions, 1);
        }
    }
    assert_eq!(
        compile_block(&[0x24210001; 8]),
        compile_block(&[0x24210004; 2])
    );
}

#[test]
fn folded_logical_updates_and_observed_intermediate_results_are_exact() {
    let initial = 0x1234_5678_ABCD_EF01;

    for (primary, immediates, expected) in [
        (12, [0xFFFF_u32, 0xF00F, 0xFF0F, 0xFFFF], 0xE001),
        (
            13,
            [0xAAAA_u32, 0x5555, 0xF00F, 0x0FF0],
            0x1234_5678_ABCD_FFFF,
        ),
        (14, [0xAAAA_u32, 0x5555, 0xF00F, 0x0FF0], initial),
    ] {
        let opcodes: Vec<u32> = immediates
            .into_iter()
            .map(|immediate| (primary << 26) | (31 << 21) | (31 << 16) | immediate)
            .collect();
        let mut engine = Engine::jit();
        let block = engine.prepare_block(&opcodes).unwrap();
        let mut gpr = [0; 32];
        gpr[31] = initial;

        assert!(engine.execute_prepared_block(block, &mut gpr));
        assert_eq!(gpr[31], expected);
    }

    let opcodes = [0x27FF0001, 0x37E10000, 0x27FF0001];
    let mut engine = Engine::jit();
    let block = engine.prepare_block(&opcodes).unwrap();
    let mut gpr = [0; 32];
    gpr[31] = 0x7FFFFFFF;

    assert!(engine.execute_prepared_block(block, &mut gpr));
    assert_eq!(
        gpr[1], 0xFFFF_FFFF_8000_0000,
        "Copy observes the intermediate update"
    );
    assert_eq!(gpr[31], 0xFFFF_FFFF_8000_0001);
    let zero_ops = [0x24000001, 0x3400FFFF, 0x3800ABCD, 0x30005555];
    let zero_block = engine.prepare_block(&zero_ops).unwrap();
    let before = gpr;

    assert!(engine.execute_prepared_block(zero_block, &mut gpr));
    assert_eq!(before, gpr);
}

#[test]
fn overwritten_cached_registers_need_no_live_in_load() {
    let opcodes = [
        0x3C01FFFF, 0x3421FFFF, 0x24220001, 0x34030004, 0x00432021, 0x00642804, 0x2463FFFF,
        0x00A33021,
    ];
    let code = compile_block(&opcodes).unwrap();
    // After the ABI pointer move, LUI defines the first cached value without a load.
    assert_eq!(code[3], 0xB8);
    let mut engine = Engine::jit();
    let block = engine.prepare_block(&opcodes).unwrap();

    for poison in [0, u64::MAX, 0xABCD_5678_1234_9876] {
        let mut gpr = [poison; 32];
        gpr[0] = 0;
        let mut expected = gpr;
        expected[1] = u64::MAX;
        expected[2] = 0;
        expected[3] = 3;
        expected[4] = 4;
        expected[5] = 64;
        expected[6] = 67;

        assert!(engine.execute_prepared_block(block, &mut gpr));
        assert_eq!(gpr, expected);
    }
}

#[test]
fn aot_bulk_loading_seals_all_pages_before_publishing_handles() {
    let mut records = Vec::new();

    for immediate in 0..1024 {
        let scalar = 0x24220000 | immediate;
        records.push((vec![scalar], compile_opcode(scalar).unwrap()));
        let block = vec![0x24210000 | immediate, 0x00201025];
        records.push((block.clone(), compile_block(&block).unwrap()));
    }

    records.push(records[0].clone());
    records.push(records[1].clone());
    let mut engine = Engine::from_aot_blocks(records).unwrap();

    assert_eq!(engine.compiled_instructions(), 1024);
    assert_eq!(engine.compiled_blocks(), 1024);
    assert!(engine.code.pages.len() > 1);
    assert!(engine.code.pages.iter().all(|page| page.is_sealed()));
    for immediate in 0..1024 {
        let mut gpr = [0; 32];
        gpr[1] = 7;
        let scalar = engine.prepare(0x24220000 | immediate).unwrap();

        assert!(engine.execute_prepared(scalar, &mut gpr));
        assert_eq!(gpr[2], 7 + immediate as u64);
        let block = engine
            .prepare_block(&[0x24210000 | immediate, 0x00201025])
            .unwrap();

        assert!(engine.execute_prepared_block(block, &mut gpr));
        assert_eq!(gpr[1], 7 + immediate as u64);
        assert_eq!(gpr[2], gpr[1]);
    }

    assert_eq!(engine.stats.native_block_executions, 1024);
    assert_eq!(engine.stats.native_executions, 3072);
}

#[test]
fn branch_tail_native_predicates_match_alu_prefix_state() {
    let mut engine = Engine::jit();
    let mut random = 0x4923_B6DC_48F3_A37D_u64;
    let functions = [
        0, 2, 3, 4, 6, 7, 20, 22, 23, 33, 35, 36, 37, 38, 39, 42, 43, 45, 47, 56, 58, 59, 60, 62,
        63,
    ];
    let immediate = [9, 10, 11, 12, 13, 14, 15, 25];

    for iteration in 0..3000 {
        let mut initial = [0; 32];

        for value in initial.iter_mut().skip(1) {
            next_random(&mut random);
            *value = random;
        }
        initial[1] = match iteration % 7 {
            0 => 0,
            1 => 1,
            2 => u64::MAX,
            3 => i64::MIN as u64,
            4 => i64::MAX as u64,
            _ => random,
        };
        let mut opcodes = Vec::new();

        for _ in 0..1 + iteration % 7 {
            next_random(&mut random);
            let rs = random as u32 & 31;
            let rt = (random >> 8) as u32 & 31;
            let rd = (random >> 16) as u32 & 31;
            let sa = (random >> 24) as u32 & 31;
            let kind = (random >> 32) as usize % 33;
            opcodes.push(if kind < functions.len() {
                (rs << 21) | (rt << 16) | (rd << 11) | (sa << 6) | functions[kind]
            } else {
                (immediate[kind - functions.len()] << 26)
                    | (rs << 21)
                    | (rt << 16)
                    | ((random >> 40) as u32 & 65535)
            });
        }

        let primary = [2, 4, 5, 6, 7][iteration % 5];
        let rs = iteration as u32 % 4;
        let rt = (iteration / 4) as u32 % 4;
        let branch = (primary << 26) | (rs << 21) | (rt << 16) | (random as u32 & 65535);
        let mut expected = initial;

        for opcode in &opcodes {
            reference(*opcode, &mut expected);
        }

        let expected_taken = match primary {
            2 => true,
            4 => expected[rs as usize] == expected[rt as usize],
            5 => expected[rs as usize] != expected[rt as usize],
            6 => expected[rs as usize] as i64 <= 0,
            7 => expected[rs as usize] as i64 > 0,
            _ => unreachable!(),
        };

        opcodes.push(branch);

        assert!(
            engine.prepare_block(&opcodes).is_none(),
            "Control block cannot use the void-returning ABI"
        );
        let block = engine.prepare_branch_block(&opcodes).unwrap();

        assert_eq!(block.branch_opcode(), branch);
        assert_eq!(block.branch_index(), opcodes.len() - 1);
        assert_eq!(block.branch_offset(), (opcodes.len() as u64 - 1) * 4);
        let mut actual = initial;

        assert_eq!(
            engine.execute_prepared_branch_block(block, &mut actual),
            Some(expected_taken)
        );
        assert_eq!(actual, expected, "branch block {opcodes:08x?}");
    }

    assert_eq!(engine.stats.native_branch_executions, 3000);
}
