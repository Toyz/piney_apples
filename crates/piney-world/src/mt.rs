//! The game's Mersenne Twister, `ccRand` (INF main 0x001d9a10): MT19937 with
//! the 1998 `sgenrand` seeding as `genrand` (0x001d9620) runs it, 624 64-bit
//! words (`mt`, 0x003ff400) holding 32 bits, `mti` at 0x00377fd0. It seeds
//! with 4352 (not the reference 4357); `ccInitRand` (0x001d9900), on every
//! scene change, seeds so and then draws `ccSys+0x358` times (frames since
//! the machine started), the only thing that differs between two arrivals,
//! so [`Mt::init`] takes that count. One generator serves the walking PCs,
//! the enemy picks and a few effects (docs/engine/battle.md).

use piney_battle::rand::Genrand;
use piney_desktop::staffroll::CcRand;

/// `mag01`: 0 and the twist matrix `0x9908b0df`.
const MAG01: [u32; 2] = [0, 0x9908_b0df];
const N: usize = 624;
const M: usize = 397;

/// `mt[]` and `mti`.
#[derive(Clone, PartialEq, Eq)]
pub struct Mt {
    pub mt: [u32; N],
    pub mti: usize,
}

impl std::fmt::Debug for Mt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mt {{ mti: {}, mt[mti]: {:#x} }}", self.mti, self.mt[self.mti.min(N - 1)])
    }
}

impl Default for Mt {
    /// Never seeded: the first draw seeds with 4352.
    fn default() -> Self {
        Mt { mt: [0; N], mti: N + 1 }
    }
}

impl Mt {
    /// The seed `genrand` and `ccInitRand` use.
    pub const SEED: u32 = 0x1100;

    /// `sgenrand(seed)`: every word from two steps of the LCG; `mti` 624.
    pub fn seeded(seed: u32) -> Mt {
        let mut mt = [0u32; N];
        let mut s = seed;
        for w in mt.iter_mut() {
            *w = s & 0xffff_0000;
            s = s.wrapping_mul(69069).wrapping_add(1);
            *w |= (s & 0xffff_0000) >> 16;
            s = s.wrapping_mul(69069).wrapping_add(1);
        }
        Mt { mt, mti: N }
    }

    /// `ccInitRand` with `ccSys+0x358` at `count`: seeded, then `count`
    /// draws (`ccRandS`'s, a separate generator, are not kept).
    pub fn init(count: u32) -> Mt {
        let mut m = Mt::seeded(Mt::SEED);
        for _ in 0..count {
            m.genrand();
        }
        m
    }

    /// `genrand`: the next tempered word.
    pub fn genrand(&mut self) -> u32 {
        if self.mti >= N {
            if self.mti == N + 1 {
                *self = Mt::seeded(Mt::SEED);
            }
            let mt = &mut self.mt;
            let twist = |a: u32, b: u32| (a & 0x8000_0000) | (b & 0x7fff_ffff);
            for kk in 0..N - M {
                let y = twist(mt[kk], mt[kk + 1]);
                mt[kk] = mt[kk + M] ^ (y >> 1) ^ MAG01[(y & 1) as usize];
            }
            for kk in N - M..N - 1 {
                let y = twist(mt[kk], mt[kk + 1]);
                mt[kk] = mt[kk + M - N] ^ (y >> 1) ^ MAG01[(y & 1) as usize];
            }
            let y = twist(mt[N - 1], mt[0]);
            mt[N - 1] = mt[M - 1] ^ (y >> 1) ^ MAG01[(y & 1) as usize];
            self.mti = 0;
        }
        let mut y = self.mt[self.mti];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `ccRand()`: the word as a signed int.
    pub fn rand(&mut self) -> i32 {
        self.genrand() as i32
    }

    /// The same generator as the battle's side holds it.
    pub fn of(g: &Genrand) -> Mt {
        Mt { mt: g.mt, mti: g.mti as usize }
    }

    /// Back into the battle's side's form.
    pub fn store(&self, g: &mut Genrand) {
        g.mt = self.mt;
        g.mti = self.mti as i32;
    }

    /// Into the save's form `c` (`SaveState::cc`), in place.
    pub fn put(&self, c: &mut CcRand) {
        *c.mt = self.mt;
        c.mti = self.mti;
    }

    /// Back from the save's form `c`.
    pub fn take(&mut self, c: &CcRand) {
        self.mt = *c.mt;
        self.mti = c.mti;
    }
}

/// The battle's `ccRand` into the save's form `c`, in place (a host lends
/// it to the menus through `SaveState::cc`).
pub fn put(g: &Genrand, c: &mut CcRand) {
    *c.mt = g.mt;
    c.mti = g.mti as usize;
}

/// The battle's `ccRand` back from the save's form `c`.
pub fn take(g: &mut Genrand, c: &CcRand) {
    g.mt = *c.mt;
    g.mti = c.mti as i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_games_draws() {
        // ccInitRand then ccRand, run in eemu with ccSys+0x358 at 0 and 1000.
        let mut m = Mt::init(0);
        let first: Vec<u32> = (0..5).map(|_| m.genrand()).collect();
        assert_eq!(first, [0x889c_77e9, 0xc8c1_0059, 0x4f01_4569, 0x3b11_bcc1, 0x0e34_b264]);
        let mut m = Mt::init(1000);
        let first: Vec<u32> = (0..3).map(|_| m.genrand()).collect();
        assert_eq!(first, [0x3bc0_7e1f, 0xa399_9e24, 0x2620_455c]);
        // Unseeded draws seed with 4352, as ccInitRand does.
        let (mut a, mut b) = (Mt::default(), Mt::init(0));
        for _ in 0..700 {
            assert_eq!(a.rand(), b.rand());
        }
    }
}
