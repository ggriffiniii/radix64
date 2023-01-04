/// An unsigned 6-bit integer. Guaranteed to only represent values from 0..64 (0-63 inclusive).
#[derive(Debug, Clone, Copy)]
pub struct U6(u8);

impl U6 {
    #[inline]
    pub const fn from_low_six_bits(x: u8) -> U6 {
        U6(x & 0x3f)
    }
}

impl From<U6> for usize {
    #[inline]
    fn from(from: U6) -> usize {
        from.0 as usize
    }
}

impl From<U6> for u8 {
    #[inline]
    fn from(from: U6) -> u8 {
        from.0
    }
}
