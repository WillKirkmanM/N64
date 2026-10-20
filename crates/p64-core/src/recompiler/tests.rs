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
