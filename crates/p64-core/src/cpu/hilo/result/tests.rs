use super::HiLo;

const SIGNED_CASES: [i64; 9] = [
    i64::MIN,
    i64::MIN + 1,
    i32::MIN as i64,
    -1,
    0,
    1,
    i32::MAX as i64,
    i64::MAX - 1,
    i64::MAX,
];
const UNSIGNED_CASES: [u64; 7] = [
    0,
    1,
    u32::MAX as u64,
    1 << 32,
    1 << 63,
    u64::MAX - 1,
    u64::MAX,
];

fn expected_signed(dividend: i64, divisor: i64) -> (u64, u64) {
    if divisor == 0 {
        (dividend as u64, if dividend < 0 { 1 } else { u64::MAX })
    } else {
        (
            (i128::from(dividend) % i128::from(divisor)) as u64,
            (i128::from(dividend) / i128::from(divisor)) as u64,
        )
    }
}

#[test]
fn signed_division_matches_wide_oracle_at_both_widths() {
    for dividend in SIGNED_CASES {
        for divisor in SIGNED_CASES {
            let result = HiLo::signed_division(dividend, divisor);

            assert_eq!((result.hi, result.lo), expected_signed(dividend, divisor));

            let result = HiLo::signed_division(dividend as i32 as i64, divisor as i32 as i64)
                .sign_extend_words();
            let (remainder, quotient) =
                expected_signed(dividend as i32 as i64, divisor as i32 as i64);

            assert_eq!(
                (result.hi, result.lo),
                (
                    remainder as i32 as i64 as u64,
                    quotient as i32 as i64 as u64
                )
            );
        }
    }
}

#[test]
fn unsigned_division_matches_wide_oracle_at_both_widths() {
    for dividend in UNSIGNED_CASES {
        for divisor in UNSIGNED_CASES {
            for word in [false, true] {
                let dividend = if word {
                    u64::from(dividend as u32)
                } else {
                    dividend
                };
                let divisor = if word {
                    u64::from(divisor as u32)
                } else {
                    divisor
                };

                let result = HiLo::unsigned_division(dividend, divisor);
                let (hi, lo) = if divisor == 0 {
                    (dividend, u64::MAX)
                } else {
                    (
                        (u128::from(dividend) % u128::from(divisor)) as u64,
                        (u128::from(dividend) / u128::from(divisor)) as u64,
                    )
                };

                assert_eq!((result.hi, result.lo), (hi, lo));

                if word {
                    let result = result.sign_extend_words();

                    assert_eq!(
                        (result.hi, result.lo),
                        (hi as i32 as i64 as u64, lo as i32 as i64 as u64)
                    );
                }
            }
        }
    }
}
