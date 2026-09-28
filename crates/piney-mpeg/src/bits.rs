//! A big-endian bit reader over one start-code unit of the elementary
//! stream. Reads past the end see zero bits; the caller checks
//! [`Bits::overrun`] where that matters.

pub(crate) struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bits<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Bits { data, pos: 0 }
    }

    /// The 64 bits from byte `at` on, zero past the end.
    #[inline]
    fn word(&self, at: usize) -> u64 {
        match self.data.get(at..at + 8) {
            Some(w) => u64::from_be_bytes(w.try_into().unwrap()),
            None => {
                let mut w = [0u8; 8];
                if at < self.data.len() {
                    let n = self.data.len() - at;
                    w[..n].copy_from_slice(&self.data[at..]);
                }
                u64::from_be_bytes(w)
            }
        }
    }

    /// The next `n` bits (1..=32) without consuming them.
    #[inline]
    pub fn peek(&self, n: u32) -> u32 {
        debug_assert!((1..=32).contains(&n));
        let w = self.word(self.pos >> 3) << (self.pos & 7);
        (w >> (64 - n)) as u32
    }

    #[inline]
    pub fn skip(&mut self, n: u32) {
        self.pos += n as usize;
    }

    #[inline]
    pub fn read(&mut self, n: u32) -> u32 {
        let v = self.peek(n);
        self.skip(n);
        v
    }

    #[inline]
    pub fn bit(&mut self) -> bool {
        self.read(1) != 0
    }

    /// Whether more bits were consumed than the unit holds.
    pub fn overrun(&self) -> bool {
        self.pos > self.data.len() * 8
    }
}

#[cfg(test)]
mod tests {
    use super::Bits;

    #[test]
    fn reads_across_bytes_and_past_the_end() {
        let mut b = Bits::new(&[0b1010_1100, 0b0101_0011]);
        assert_eq!(b.read(3), 0b101);
        assert_eq!(b.peek(8), 0b0110_0010);
        assert_eq!(b.read(10), 0b01100_01010);
        assert_eq!(b.read(8), 0b0110_0000);
        assert!(b.overrun());
    }
}
