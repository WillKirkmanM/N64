pub(super) const LANE_COUNT: usize = 8;
pub(super) const VECTOR_BYTES: usize = std::mem::size_of::<u128>();
pub(super) const LANE_BYTES: usize = std::mem::size_of::<u16>();
pub(super) const WORD_BYTES: usize = std::mem::size_of::<u32>();
pub(super) const BYTE_BITS: usize = u8::BITS as usize;
pub(super) const LANE_BITS: usize = u16::BITS as usize;

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Vector(pub(super) wide::i16x8);
impl Vector {
    pub const ZERO: Self = Self(wide::i16x8::ZERO);
    #[inline(always)]
    pub(super) fn to_le_bytes(self) -> [u8; VECTOR_BYTES] {
        let lanes = self.0.to_array();

        core::array::from_fn(|i| lanes[i / LANE_BYTES].to_le_bytes()[i % LANE_BYTES])
    }

    #[inline(always)]
    pub(super) fn from_le_bytes(b: [u8; VECTOR_BYTES]) -> Self {
        Self(wide::i16x8::new(core::array::from_fn(|i| {
            i16::from_le_bytes([b[i * LANE_BYTES], b[i * LANE_BYTES + 1]])
        })))
    }
}

impl core::ops::BitAnd for Vector {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, b: Self) -> Self {
        Vector(self.0 & b.0)
    }
}

impl core::ops::BitOr for Vector {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, b: Self) -> Self {
        Vector(self.0 | b.0)
    }
}

impl core::ops::BitXor for Vector {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, b: Self) -> Self {
        Vector(self.0 ^ b.0)
    }
}

impl core::ops::Not for Vector {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self {
        Vector(!self.0)
    }
}
