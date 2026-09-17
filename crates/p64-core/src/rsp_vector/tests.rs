use super::portable as p;
use std::arch::x86_64 as n;
#[test]
fn portable_lanes_match_native_sse() {
    if !is_x86_feature_detected!("ssse3") || !is_x86_feature_detected!("sse4.1") {
        return;
    }

    unsafe {
        compare();
    }
}

#[target_feature(enable = "ssse3,sse4.1")]
unsafe fn compare() {
    let mut state = 0x71d7a398af65931bu128;

    for i in 0..10000 {
        state = state.wrapping_mul(0xda942042e4dd58b5).wrapping_add(1);
        let a = if i < 4 {
            [
                0,
                u128::MAX,
                0x80008000800080008000800080008000,
                0x7fff7fff7fff7fff7fff7fff7fff7fff,
            ][i]
        } else {
            state
        };

        state = state.wrapping_mul(0xda942042e4dd58b5).wrapping_add(1);
        let b = state;
        let mask = a.rotate_left(13) ^ b;
        let na = native(a);
        let nb = native(b);
        macro_rules! check {
            ($portable:ident, $native:ident) => {
                assert_eq!(
                    pbits(p::$portable(pnative(a), pnative(b))),
                    bits(n::$native(na, nb)),
                    stringify!($portable)
                );
            };
        }
        check!(bit_and, _mm_and_si128);
        check!(bit_or, _mm_or_si128);
        check!(bit_xor, _mm_xor_si128);
        check!(and_not, _mm_andnot_si128);
        check!(add_wrapping, _mm_add_epi16);
        check!(sub_wrapping, _mm_sub_epi16);
        check!(add_saturating_signed, _mm_adds_epi16);
        check!(sub_saturating_signed, _mm_subs_epi16);
        check!(add_saturating_unsigned, _mm_adds_epu16);
        check!(sub_saturating_unsigned, _mm_subs_epu16);
        check!(mul_low, _mm_mullo_epi16);
        check!(mul_high_signed, _mm_mulhi_epi16);
        check!(mul_high_unsigned, _mm_mulhi_epu16);
        check!(equal_lanes, _mm_cmpeq_epi16);
        check!(greater_signed, _mm_cmpgt_epi16);
        check!(less_signed, _mm_cmplt_epi16);
        check!(min_signed, _mm_min_epi16);
        check!(max_signed, _mm_max_epi16);
        check!(equal_bytes, _mm_cmpeq_epi8);
        check!(shuffle_bytes, _mm_shuffle_epi8);
        check!(pack_signed_bytes, _mm_packs_epi16);
        check!(pack_signed_lanes, _mm_packs_epi32);
        check!(interleave_low, _mm_unpacklo_epi16);
        check!(interleave_high, _mm_unpackhi_epi16);
        assert_eq!(
            pbits(p::blend_bytes(pnative(a), pnative(b), pnative(mask))),
            bits(n::_mm_blendv_epi8(na, nb, native(mask)))
        );
        assert_eq!(p::byte_sign_mask(pnative(a)), n::_mm_movemask_epi8(na));
        assert_eq!(
            pbits(p::shift_left(pnative(a), 1)),
            bits(n::_mm_slli_epi16::<1>(na))
        );
        assert_eq!(
            pbits(p::shift_right_unsigned(pnative(a), 15)),
            bits(n::_mm_srli_epi16::<15>(na))
        );
        assert_eq!(
            pbits(p::shift_right_signed(pnative(a), 15)),
            bits(n::_mm_srai_epi16::<15>(na))
        );
    }
}

fn pnative(a: u128) -> p::Vector {
    p::vector_from_bits(a)
}

fn pbits(a: p::Vector) -> u128 {
    p::vector_to_bits(a)
}

unsafe fn native(a: u128) -> n::__m128i {
    std::mem::transmute(a)
}

unsafe fn bits(a: n::__m128i) -> u128 {
    std::mem::transmute(a)
}
