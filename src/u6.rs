/// An unsigned 6-bit integer. Guaranteed to only represent values from 0..64 (0-63 inclusive).
#[derive(Debug, Clone, Copy)]
pub struct U6(u8);

impl U6 {
    #[inline]
    pub const fn from_low_six_bits(x: u8) -> U6 {
        U6(x & 0x3f)
    }

    #[inline]
    pub fn new(x: u8) -> U6 {
        assert!(x < 64);
        unsafe { Self::new_unchecked(x) }
    }

    #[inline]
    pub const unsafe fn new_unchecked(x: u8) -> U6 {
        U6(x)
    }
}

impl Into<usize> for U6 {
    #[inline]
    fn into(self) -> usize {
        <Self as Into<u8>>::into(self).into()
    }
}

impl Into<u8> for U6 {
    #[inline]
    fn into(self) -> u8 {
        self.0
    }
}
