//! The SPU2's reverb in the mode the game sets at start-up: `spuInit`
//! (`INF SLUS_202.67:0x00181ee0`) gives both cores `sceSdSetEffectAttr`
//! mode 5 (hall, with the work area cleared), effect volume 0x3fff and
//! effects enabled; the voices whose sample asks for it (`spu_attr` bits 2
//! and 3, set per voice as VMIXEL / VMIXER) feed it.
//!
//! The algorithm and the resampling filter are the PlayStation SPU's as
//! psx-spx documents them ("SPU Reverb Formula", "Reverb Buffer
//! Resampling"): the unit runs at half the output rate (24 kHz here) over a
//! ring buffer, with same-side and cross reflections, four combs and two
//! all-pass stages. The hall preset is LIBSD.IRX's (module offset 0x42d8),
//! the same 32 register values as the PlayStation's; addresses count 8-byte
//! units, 4 samples each.

/// dAPF1 ... vRIN, as the SPU's registers 0x1dc0-0x1dfe are ordered.
#[rustfmt::skip]
pub const HALL: [u16; 32] = [
    0x01a5, 0x0139, 0x6000, 0x5000, 0x4c00, 0xb800, 0xbc00, 0xc000,
    0x6000, 0x5c00, 0x15ba, 0x11bb, 0x14c2, 0x10bd, 0x11bc, 0x0dc1,
    0x11c0, 0x0dc3, 0x0dc0, 0x09c1, 0x0bc4, 0x07c1, 0x0a00, 0x06cd,
    0x09c2, 0x05c1, 0x05c0, 0x041a, 0x0274, 0x013a, 0x8000, 0x8000,
];
/// The hall work area: 0xade0 bytes.
pub const HALL_SIZE: usize = 0xade0 / 2;

/// The 39-tap half-band filter the unit resamples through.
#[rustfmt::skip]
const FIR: [i32; 39] = [
    -1, 0, 2, 0, -10, 0, 35, 0, -103, 0, 266, 0, -616, 0, 1332, 0, -2960, 0, 10246, 16384,
    10246, 0, -2960, 0, 1332, 0, -616, 0, 266, 0, -103, 0, 35, 0, -10, 0, 2, 0, -1,
];

pub struct Reverb {
    r: [u16; 32],
    buf: Vec<i16>,
    cur: usize,
    /// Input history at the output rate, newest last.
    inp: [[i32; 39]; 2],
    /// Output history at the output rate (the wet samples with zeros
    /// between), newest last.
    out: [[i32; 39]; 2],
    phase: bool,
    /// EVOLL / EVOLR.
    pub volume: (i16, i16),
}

fn sat(v: i32) -> i32 {
    v.clamp(-0x8000, 0x7fff)
}

fn mul(a: i32, v: u16) -> i32 {
    (a * v as i16 as i32) >> 15
}

impl Reverb {
    pub fn hall() -> Reverb {
        Reverb {
            r: HALL,
            buf: vec![0; HALL_SIZE],
            cur: 0,
            inp: [[0; 39]; 2],
            out: [[0; 39]; 2],
            phase: false,
            volume: (0x3fff, 0x3fff),
        }
    }

    fn at(&self, reg: usize, back: isize) -> usize {
        let n = self.buf.len() as isize;
        ((self.cur as isize + self.r[reg] as isize * 4 + back).rem_euclid(n)) as usize
    }

    fn rd(&self, reg: usize, back: isize) -> i32 {
        self.buf[self.at(reg, back)] as i32
    }

    fn wr(&mut self, reg: usize, v: i32) {
        let a = self.at(reg, 0);
        self.buf[a] = sat(v) as i16;
    }

    /// One 24 kHz step of the reverb formula; returns (Lout, Rout).
    fn step(&mut self, lin: i32, rin: i32) -> (i32, i32) {
        const DAPF1: usize = 0;
        const DAPF2: usize = 1;
        const VIIR: usize = 2;
        const VCOMB: [usize; 4] = [3, 4, 5, 6];
        const VWALL: usize = 7;
        const VAPF1: usize = 8;
        const VAPF2: usize = 9;
        const MLSAME: usize = 10;
        const MRSAME: usize = 11;
        const MLCOMB: [usize; 4] = [12, 14, 20, 22];
        const MRCOMB: [usize; 4] = [13, 15, 21, 23];
        const DLSAME: usize = 16;
        const DRSAME: usize = 17;
        const MLDIFF: usize = 18;
        const MRDIFF: usize = 19;
        const DLDIFF: usize = 24;
        const DRDIFF: usize = 25;
        const MLAPF1: usize = 26;
        const MRAPF1: usize = 27;
        const MLAPF2: usize = 28;
        const MRAPF2: usize = 29;
        const VLIN: usize = 30;
        const VRIN: usize = 31;
        let r = self.r;
        let lin = sat(mul(lin, r[VLIN]));
        let rin = sat(mul(rin, r[VRIN]));
        let same = |s: &Self, input: i32, m: usize, d: usize| {
            let prev = s.rd(m, -1);
            sat(mul(sat(input + mul(s.rd(d, 0), r[VWALL]) - prev), r[VIIR]) + prev)
        };
        let v = same(self, lin, MLSAME, DLSAME);
        self.wr(MLSAME, v);
        let v = same(self, rin, MRSAME, DRSAME);
        self.wr(MRSAME, v);
        let v = same(self, lin, MLDIFF, DRDIFF);
        self.wr(MLDIFF, v);
        let v = same(self, rin, MRDIFF, DLDIFF);
        self.wr(MRDIFF, v);
        let comb = |s: &Self, m: [usize; 4]| sat((0..4).map(|k| mul(s.rd(m[k], 0), r[VCOMB[k]])).sum());
        let mut l = comb(self, MLCOMB);
        let mut rr = comb(self, MRCOMB);
        for (m_l, m_r, d, vol) in [(MLAPF1, MRAPF1, DAPF1, VAPF1), (MLAPF2, MRAPF2, DAPF2, VAPF2)] {
            let dl = self.rd(m_l, -(r[d] as isize) * 4);
            let dr = self.rd(m_r, -(r[d] as isize) * 4);
            l = sat(l - mul(dl, r[vol]));
            rr = sat(rr - mul(dr, r[vol]));
            self.wr(m_l, l);
            self.wr(m_r, rr);
            l = sat(mul(l, r[vol]) + dl);
            rr = sat(mul(rr, r[vol]) + dr);
        }
        self.cur = (self.cur + 1) % self.buf.len();
        (l, rr)
    }

    /// One output-rate sample: the wet input in, the reverb's output out
    /// (after the effect volume).
    pub fn sample(&mut self, input: (i32, i32)) -> (i32, i32) {
        for (c, x) in [input.0, input.1].into_iter().enumerate() {
            self.inp[c].copy_within(1.., 0);
            self.inp[c][38] = x;
        }
        self.phase = !self.phase;
        let wet = if self.phase {
            let fir = |h: &[i32; 39]| sat(h.iter().zip(FIR).map(|(&x, c)| x * c).sum::<i32>() >> 15);
            let (lin, rin) = (fir(&self.inp[0]), fir(&self.inp[1]));
            let (l, r) = self.step(lin, rin);
            [l, r]
        } else {
            [0, 0]
        };
        let mut o = [0i32; 2];
        for c in 0..2 {
            self.out[c].copy_within(1.., 0);
            self.out[c][38] = wet[c];
            // Zeros between the 24 kHz samples halve the gain: taps count double.
            o[c] = sat(self.out[c].iter().zip(FIR).map(|(&x, k)| x * k).sum::<i32>() >> 14);
        }
        (mul(o[0], self.volume.0 as u16), mul(o[1], self.volume.1 as u16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_impulse_rings_and_decays() {
        let mut r = Reverb::hall();
        let mut energy = Vec::new();
        for n in 0..48_000 * 4 {
            let x = if n < 64 { 12_000 } else { 0 };
            let (l, rr) = r.sample((x, x));
            if n % 48_000 == 0 {
                energy.push(0i64);
            }
            *energy.last_mut().unwrap() += (l as i64).pow(2) + (rr as i64).pow(2);
        }
        assert!(energy[0] > 0, "{energy:?}");
        assert!(energy[3] < energy[0], "{energy:?}");
    }
}
