//! The game's `rand()`: newlib's (`INF SLUS_202.67:0x00133a38`), one
//! generator for the whole game. Its state is the 64-bit
//! `_impure_ptr->_new._reent._rand_next` (`*_impure_ptr + 168`):
//!
//! ```text
//! state = state * 6364136223846793005 + 1
//! return (state >> 32) & 0x7fffffff
//! ```
//!
//! Battle draws from the same generator as every other caller (the field,
//! the enemies' movement, the effects), so the functions here take it as a
//! [`Rng`] and the runtime owns the one instance: a [`Rand`], or any
//! `FnMut() -> i32` that forwards to the runtime's own.
//!
//! The enemies draw from a second generator, `ccRand()` (main 0x001d9a10):
//! [`Genrand`], a Mersenne Twister the field and the enemies' movement
//! share too (see [`crate::enemy_ai`]).

/// Something that answers `rand()`.
pub trait Rng {
    /// The next value, 0 to 0x7fffffff.
    fn rand(&mut self) -> i32;
}

/// newlib's generator. `srand(s)` sets the state to `s`; a fresh `_reent`
/// starts at 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rand(pub u64);

impl Default for Rand {
    fn default() -> Self {
        Rand(1)
    }
}

impl Rand {
    pub const MUL: u64 = 6_364_136_223_846_793_005;

    pub fn new(state: u64) -> Self {
        Rand(state)
    }
}

impl Rng for Rand {
    fn rand(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(Self::MUL).wrapping_add(1);
        ((self.0 >> 32) & 0x7fff_ffff) as i32
    }
}

impl<F: FnMut() -> i32> Rng for F {
    fn rand(&mut self) -> i32 {
        self()
    }
}

/// Counts the draws of another generator (for callers that must know how
/// many values a rule used).
pub struct Counted<'a> {
    pub inner: &'a mut dyn Rng,
    pub count: u32,
}

impl Rng for Counted<'_> {
    fn rand(&mut self) -> i32 {
        self.count += 1;
        self.inner.rand()
    }
}

/// `ccRand`'s source: `genrand()` (main 0x001d9620), MT19937 with the 1998
/// `sgenrand` seeding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Genrand {
    pub mt: [u32; 624],
    /// `mti`: the next word; 624 or more regenerates, 625 (the value in
    /// the executable) seeds with 4352 first.
    pub mti: i32,
}

impl Default for Genrand {
    fn default() -> Self {
        Genrand { mt: [0; 624], mti: 625 }
    }
}

impl Genrand {
    /// The state `sgenrand(seed)` leaves (the loop of `genrand` and
    /// `ccInitRand`), `mti` 624.
    pub fn seeded(seed: u32) -> Genrand {
        let mut g = Genrand::default();
        g.seed(seed);
        g
    }

    pub fn seed(&mut self, mut seed: u32) {
        for w in self.mt.iter_mut() {
            *w = seed & 0xffff_0000;
            seed = seed.wrapping_mul(69069).wrapping_add(1);
            *w |= (seed & 0xffff_0000) >> 16;
            seed = seed.wrapping_mul(69069).wrapping_add(1);
        }
        self.mti = 624;
    }

    /// `genrand()`: the next tempered word.
    pub fn next_u32(&mut self) -> u32 {
        const N: usize = 624;
        const M: usize = 397;
        let mix = |a: u32, b: u32, c: u32| {
            let y = (a & 0x8000_0000) | (b & 0x7fff_ffff);
            c ^ (y >> 1) ^ if y & 1 != 0 { 0x9908_b0df } else { 0 }
        };
        if self.mti >= N as i32 {
            if self.mti == 625 {
                self.seed(4352);
            }
            for k in 0..N - M {
                self.mt[k] = mix(self.mt[k], self.mt[k + 1], self.mt[k + M]);
            }
            for k in N - M..N - 1 {
                self.mt[k] = mix(self.mt[k], self.mt[k + 1], self.mt[k + M - N]);
            }
            self.mt[N - 1] = mix(self.mt[N - 1], self.mt[0], self.mt[M - 1]);
            self.mti = 0;
        }
        let mut y = self.mt[self.mti as usize];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }
}

impl Rng for Genrand {
    /// `ccRand()`: the word as a signed `int`.
    fn rand(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newlib_sequence() {
        // srand(1) then rand(): newlib's first values.
        let mut r = Rand::default();
        let v: Vec<i32> = (0..3).map(|_| r.rand()).collect();
        assert_eq!(v, [1_481_765_933, 1_085_377_743, 1_270_216_262]);
    }

    #[test]
    fn closures_forward() {
        let mut inner = Rand(5);
        let mut copy = inner;
        let mut f = || inner.rand();
        let a = Rng::rand(&mut f);
        assert_eq!(a, copy.rand());
    }

    #[test]
    fn genrand_seeds_on_first_use() {
        // An unseeded generator seeds with 4352 before its first word.
        let mut a = Genrand::default();
        let mut b = Genrand::seeded(4352);
        let x: Vec<u32> = (0..700).map(|_| a.next_u32()).collect();
        let y: Vec<u32> = (0..700).map(|_| b.next_u32()).collect();
        assert_eq!(x, y);
        assert_eq!(a.mti, 700 - 624);
    }

    #[test]
    fn genrand_first_words() {
        // The game's genrand from the executable's state (eemu).
        let mut g = Genrand::default();
        let v: Vec<u32> = (0..3).map(|_| g.next_u32()).collect();
        assert_eq!(v, [2_291_955_689, 3_368_091_737, 1_325_483_369]);
        assert_eq!(g.mti, 3);
    }
}
