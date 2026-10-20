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
