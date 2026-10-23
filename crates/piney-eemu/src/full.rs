//! The rest of the EE's integer instruction set, for whole-program runs
//! ([`crate::sys`], [`crate::cpu::Features::full`]): every MMI instruction,
//! the second pipeline's multiply and divide, the funnel shift's SA
//! register. The harness machines leave these off, as eemu's Python does.

use crate::cpu::{Cpu, Fault, Flow};

const M64: u128 = u64::MAX as u128;

fn words(v: u128) -> [u32; 4] {
    std::array::from_fn(|k| (v >> (32 * k)) as u32)
}

fn from_words(w: [u32; 4]) -> u128 {
    w.iter().rev().fold(0, |v, &x| v << 32 | u128::from(x))
}

fn halves(v: u128) -> [u16; 8] {
    std::array::from_fn(|k| (v >> (16 * k)) as u16)
}

fn from_halves(h: [u16; 8]) -> u128 {
    h.iter().rev().fold(0, |v, &x| v << 16 | u128::from(x))
}

fn bytes(v: u128) -> [u8; 16] {
    v.to_le_bytes()
}

fn from_bytes(b: [u8; 16]) -> u128 {
    u128::from_le_bytes(b)
}

fn sx32(v: u32) -> u128 {
    u128::from(v as i32 as i64 as u64)
}

fn satw(v: i64) -> u32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as u32
}

fn sath(v: i32) -> u16 {
    v.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as u16
}

fn satb(v: i32) -> u8 {
    v.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as u8
}

impl Cpu {
    fn hi128(&self) -> u128 {
        (self.hi & M64) | (self.hi1 << 64)
    }

    fn lo128(&self) -> u128 {
        (self.lo & M64) | (self.lo1 << 64)
    }

    fn set_hilo128(&mut self, hi: u128, lo: u128) {
        (self.hi, self.hi1, self.lo, self.lo1) = (hi & M64, hi >> 64, lo & M64, lo >> 64);
    }

    fn put(&mut self, rd: usize, v: u128) {
        if rd != 0 {
            self.r[rd] = v;
        }
    }

    /// MFSA, MTSA (special 0x28, 0x29), MTSAB, MTSAH (regimm 0x18, 0x19):
    /// None when `w` is none of them.
    pub(crate) fn sa_ops(&mut self, w: u32) -> Option<Flow> {
        let (op, rs, rt, rd) = (w >> 26, ((w >> 21) & 31) as usize, (w >> 16) & 31, ((w >> 11) & 31) as usize);
        let imm = w & 0xffff;
        match (op, w & 63, rt) {
            (0, 0x28, _) => self.put(rd, u128::from(self.sa)),
            (0, 0x29, _) => self.sa = self.r[rs] as u32 & 0xf,
            (1, _, 0x18) => self.sa = (self.r[rs] as u32 ^ imm) & 15,
            (1, _, 0x19) => self.sa = ((self.r[rs] as u32 ^ imm) & 7) * 2,
            _ => return None,
        }
        Some(Flow::Next)
    }

    /// One MMI-opcode instruction (0x1c), every function.
    pub(crate) fn mmi_full(&mut self, pc: u64, w: u32) -> Result<Flow, Fault> {
        let (rs, rt, rd, sa) =
            (((w >> 21) & 31) as usize, ((w >> 16) & 31) as usize, ((w >> 11) & 31) as usize, (w >> 6) & 31);
        let (a, b) = (self.r[rs], self.r[rt]);
        let bad = Err(Fault::Bad { pc, w });
        let v: u128 = match w & 63 {
            0x00 | 0x01 | 0x20 | 0x21 => {
                // madd, maddu, madd1, maddu1
                let one = w & 0x20 != 0;
                let (hi, lo) = if one { (self.hi1, self.lo1) } else { (self.hi, self.lo) };
                let acc = ((hi as u64 as u128) << 32 | (lo as u32 as u128)) as u64;
                let p = if w & 1 == 0 {
                    (i64::from(a as u32 as i32) * i64::from(b as u32 as i32)) as u64
                } else {
                    u64::from(a as u32) * u64::from(b as u32)
                };
                let s = acc.wrapping_add(p);
                let (l, h) = (sx32(s as u32), sx32((s >> 32) as u32));
                if one {
                    (self.lo1, self.hi1) = (l, h);
                } else {
                    (self.lo, self.hi) = (l, h);
                }
                l
            }
            0x04 => {
                // plzcw: leading bits equal to the sign, less one, per word
                let f = |x: u32| {
                    let y = if x as i32 >= 0 { x } else { !x };
                    y.leading_zeros().saturating_sub(1)
                };
                let wa = words(a);
                (u128::from(f(wa[1])) << 32 | u128::from(f(wa[0]))) | (a & !M64)
            }
            0x10 => self.hi1,
            0x12 => self.lo1,
            0x11 => {
                self.hi1 = a & M64;
                return Ok(Flow::Next);
            }
            0x13 => {
                self.lo1 = a & M64;
                return Ok(Flow::Next);
            }
            0x18 | 0x19 => {
                // mult1, multu1
                let p = if w & 1 == 0 {
                    (i64::from(a as u32 as i32) * i64::from(b as u32 as i32)) as u64
                } else {
                    u64::from(a as u32) * u64::from(b as u32)
                };
                (self.lo1, self.hi1) = (sx32(p as u32), sx32((p >> 32) as u32));
                self.lo1
            }
            0x1a | 0x1b => {
                // div1, divu1
                let (n, d) = if w & 1 == 0 {
                    (i64::from(a as u32 as i32), i64::from(b as u32 as i32))
                } else {
                    (i64::from(a as u32), i64::from(b as u32))
                };
                if d != 0 {
                    let q = n / d;
                    (self.lo1, self.hi1) = (sx32(q as u32), sx32((n - q * d) as u32));
                }
                return Ok(Flow::Next);
            }
            0x30 => {
                // pmfhl
                let (hi, lo) = (words(self.hi128()), words(self.lo128()));
                match sa {
                    0 => from_words([lo[0], hi[0], lo[2], hi[2]]),
                    1 => from_words([lo[1], hi[1], lo[3], hi[3]]),
                    2 => {
                        let s = |l: u32, h: u32| satw(((u64::from(h) << 32) | u64::from(l)) as i64);
                        let x = s(lo[0], hi[0]);
                        let y = s(lo[2], hi[2]);
                        from_words([x, sx32(x).wrapping_shr(32) as u32, y, sx32(y).wrapping_shr(32) as u32])
                    }
                    3 => {
                        let (lh, hh) = (halves(self.lo128()), halves(self.hi128()));
                        from_halves([lh[0], lh[2], hh[0], hh[2], lh[4], lh[6], hh[4], hh[6]])
                    }
                    4 => {
                        let s = |x: u32| sath(x as i32);
                        from_halves([s(lo[0]), s(lo[1]), s(hi[0]), s(hi[1]), s(lo[2]), s(lo[3]), s(hi[2]), s(hi[3])])
                    }
                    _ => return bad,
                }
            }
            0x31 => {
                // pmthl.lw
                if sa != 0 {
                    return bad;
                }
                let x = words(a);
                let (mut hi, mut lo) = (words(self.hi128()), words(self.lo128()));
                (lo[0], hi[0], lo[2], hi[2]) = (x[0], x[1], x[2], x[3]);
                self.set_hilo128(from_words(hi), from_words(lo));
                return Ok(Flow::Next);
            }
            0x34 => from_halves(halves(b).map(|x| x << (sa & 15))),
            0x36 => from_halves(halves(b).map(|x| x >> (sa & 15))),
            0x37 => from_halves(halves(b).map(|x| ((x as i16) >> (sa & 15)) as u16)),
            0x3c => from_words(words(b).map(|x| x << sa)),
            0x3e => from_words(words(b).map(|x| x >> sa)),
            0x3f => from_words(words(b).map(|x| ((x as i32) >> sa) as u32)),
            0x08 => self.mmi0(a, b, sa).ok_or(Fault::Bad { pc, w })?,
            0x28 => self.mmi1(a, b, sa).ok_or(Fault::Bad { pc, w })?,
            0x09 => match self.mmi2(rs, a, b, sa) {
                Some(Some(v)) => v,
                Some(None) => return Ok(Flow::Next),
                None => return bad,
            },
            0x29 => match self.mmi3(rs, a, b, sa) {
                Some(Some(v)) => v,
                Some(None) => return Ok(Flow::Next),
                None => return bad,
            },
            _ => return bad,
        };
        self.put(rd, v);
        Ok(Flow::Next)
    }

    fn mmi0(&mut self, a: u128, b: u128, sa: u32) -> Option<u128> {
        let (wa, wb, ha, hb, ba, bb) = (words(a), words(b), halves(a), halves(b), bytes(a), bytes(b));
        let w2 = |f: &dyn Fn(u32, u32) -> u32| from_words(std::array::from_fn(|i| f(wa[i], wb[i])));
        let h2 = |f: &dyn Fn(u16, u16) -> u16| from_halves(std::array::from_fn(|i| f(ha[i], hb[i])));
        let b2 = |f: &dyn Fn(u8, u8) -> u8| from_bytes(std::array::from_fn(|i| f(ba[i], bb[i])));
        Some(match sa {
            0x00 => w2(&|x, y| x.wrapping_add(y)),
            0x01 => w2(&|x, y| x.wrapping_sub(y)),
            0x02 => w2(&|x, y| if (x as i32) > (y as i32) { u32::MAX } else { 0 }),
            0x03 => w2(&|x, y| (x as i32).max(y as i32) as u32),
            0x04 => h2(&|x, y| x.wrapping_add(y)),
            0x05 => h2(&|x, y| x.wrapping_sub(y)),
            0x06 => h2(&|x, y| if (x as i16) > (y as i16) { u16::MAX } else { 0 }),
            0x07 => h2(&|x, y| (x as i16).max(y as i16) as u16),
            0x08 => b2(&|x, y| x.wrapping_add(y)),
            0x09 => b2(&|x, y| x.wrapping_sub(y)),
            0x0a => b2(&|x, y| if (x as i8) > (y as i8) { u8::MAX } else { 0 }),
            0x10 => w2(&|x, y| satw(i64::from(x as i32) + i64::from(y as i32))),
            0x11 => w2(&|x, y| satw(i64::from(x as i32) - i64::from(y as i32))),
            0x12 => from_words([wb[0], wa[0], wb[1], wa[1]]),
            0x13 => from_words([wb[0], wb[2], wa[0], wa[2]]),
            0x14 => h2(&|x, y| sath(i32::from(x as i16) + i32::from(y as i16))),
            0x15 => h2(&|x, y| sath(i32::from(x as i16) - i32::from(y as i16))),
            0x16 => from_halves([hb[0], ha[0], hb[1], ha[1], hb[2], ha[2], hb[3], ha[3]]),
            0x17 => from_halves([hb[0], hb[2], hb[4], hb[6], ha[0], ha[2], ha[4], ha[6]]),
            0x18 => b2(&|x, y| satb(i32::from(x as i8) + i32::from(y as i8))),
            0x19 => b2(&|x, y| satb(i32::from(x as i8) - i32::from(y as i8))),
            0x1a => from_bytes(std::array::from_fn(|i| if i % 2 == 0 { bb[i / 2] } else { ba[i / 2] })),
            0x1b => from_bytes(std::array::from_fn(|i| if i < 8 { bb[2 * i] } else { ba[2 * (i - 8)] })),
            0x1e => {
                // pext5: 1-5-5-5 to 8-8-8-8
                from_words(wb.map(|x| {
                    let (r, g, bl, al) = (x & 31, (x >> 5) & 31, (x >> 10) & 31, (x >> 15) & 1);
                    (r << 3) | (g << 11) | (bl << 19) | (al << 31)
                }))
            }
            0x1f => {
                // ppac5
                from_words(
                    wb.map(|x| {
                        ((x >> 3) & 31) | (((x >> 11) & 31) << 5) | (((x >> 19) & 31) << 10) | ((x >> 31) << 15)
                    }),
                )
            }
            _ => return None,
        })
    }

    fn mmi1(&mut self, a: u128, b: u128, sa: u32) -> Option<u128> {
        let (wa, wb, ha, hb, ba, bb) = (words(a), words(b), halves(a), halves(b), bytes(a), bytes(b));
        let w2 = |f: &dyn Fn(u32, u32) -> u32| from_words(std::array::from_fn(|i| f(wa[i], wb[i])));
        let h2 = |f: &dyn Fn(u16, u16) -> u16| from_halves(std::array::from_fn(|i| f(ha[i], hb[i])));
        let b2 = |f: &dyn Fn(u8, u8) -> u8| from_bytes(std::array::from_fn(|i| f(ba[i], bb[i])));
        Some(match sa {
            0x01 => from_words(wb.map(|x| if x == 0x8000_0000 { 0x7fff_ffff } else { (x as i32).unsigned_abs() })),
            0x02 => w2(&|x, y| if x == y { u32::MAX } else { 0 }),
            0x03 => w2(&|x, y| (x as i32).min(y as i32) as u32),
            0x04 => {
                // padsbh
                from_halves(std::array::from_fn(|i| {
                    if i < 4 { ha[i].wrapping_sub(hb[i]) } else { ha[i].wrapping_add(hb[i]) }
                }))
            }
            0x05 => from_halves(hb.map(|x| if x == 0x8000 { 0x7fff } else { (x as i16).unsigned_abs() })),
            0x06 => h2(&|x, y| if x == y { u16::MAX } else { 0 }),
            0x07 => h2(&|x, y| (x as i16).min(y as i16) as u16),
            0x0a => b2(&|x, y| if x == y { u8::MAX } else { 0 }),
            0x10 => w2(&|x, y| x.saturating_add(y)),
            0x11 => w2(&|x, y| x.saturating_sub(y)),
            0x12 => from_words([wb[2], wa[2], wb[3], wa[3]]),
            0x14 => h2(&|x, y| x.saturating_add(y)),
            0x15 => h2(&|x, y| x.saturating_sub(y)),
            0x16 => from_halves([hb[4], ha[4], hb[5], ha[5], hb[6], ha[6], hb[7], ha[7]]),
            0x18 => b2(&|x, y| x.saturating_add(y)),
            0x19 => b2(&|x, y| x.saturating_sub(y)),
            0x1a => from_bytes(std::array::from_fn(|i| if i % 2 == 0 { bb[8 + i / 2] } else { ba[8 + i / 2] })),
            0x1b => {
                // qfsrv: rs:rt shifted right by SA bytes (SA in bits / 8)
                let wide = [bb, ba].concat();
                let s = self.sa as usize;
                from_bytes(std::array::from_fn(|i| *wide.get(i + s).unwrap_or(&0)))
            }
            _ => return None,
        })
    }

    /// MMI2: Some(Some(v)) writes rd, Some(None) does not.
    fn mmi2(&mut self, _rs: usize, a: u128, b: u128, sa: u32) -> Option<Option<u128>> {
        let (wa, wb, ha, hb) = (words(a), words(b), halves(a), halves(b));
        Some(Some(match sa {
            0x00 | 0x04 => {
                // pmaddw, pmsubw
                let (hi, lo) = (words(self.hi128()), words(self.lo128()));
                let mut out = [0u64; 2];
                let (mut h, mut l) = (hi, lo);
                for (j, k) in [(0usize, 0usize), (1, 2)] {
                    let acc = (u64::from(hi[k]) << 32) | u64::from(lo[k]);
                    let p = (i64::from(wa[k] as i32) * i64::from(wb[k] as i32)) as u64;
                    let s = if sa == 0 { acc.wrapping_add(p) } else { acc.wrapping_sub(p) };
                    out[j] = s;
                    l[k] = s as u32;
                    h[k] = (s >> 32) as u32;
                    l[k + 1] = (sx32(s as u32) >> 32) as u32;
                    h[k + 1] = (sx32((s >> 32) as u32) >> 32) as u32;
                }
                self.set_hilo128(from_words(h), from_words(l));
                (u128::from(out[1]) << 64) | u128::from(out[0])
            }
            0x02 => from_words([wb[0] << (wa[0] & 31), 0, wb[2] << (wa[2] & 31), 0]).pipe_sx(),
            0x03 => from_words([wb[0] >> (wa[0] & 31), 0, wb[2] >> (wa[2] & 31), 0]).pipe_sx(),
            0x08 => self.hi128(),
            0x09 => self.lo128(),
            0x0a => from_halves([hb[0], ha[0], hb[1], ha[1], hb[2], ha[2], hb[3], ha[3]]),
            0x0c => {
                // pmultw
                let p0 = i64::from(wa[0] as i32) * i64::from(wb[0] as i32);
                let p1 = i64::from(wa[2] as i32) * i64::from(wb[2] as i32);
                let lo =
                    from_words([p0 as u32, (sx32(p0 as u32) >> 32) as u32, p1 as u32, (sx32(p1 as u32) >> 32) as u32]);
                let hi = from_words([
                    (p0 >> 32) as u32,
                    (sx32((p0 >> 32) as u32) >> 32) as u32,
                    (p1 >> 32) as u32,
                    (sx32((p1 >> 32) as u32) >> 32) as u32,
                ]);
                self.set_hilo128(hi, lo);
                (u128::from(p1 as u64) << 64) | u128::from(p0 as u64)
            }
            0x0d => {
                // pdivw
                let mut lo = [0u32; 4];
                let mut hi = [0u32; 4];
                for k in [0, 2] {
                    let (n, d) = (i64::from(wa[k] as i32), i64::from(wb[k] as i32));
                    if d != 0 {
                        let q = n / d;
                        lo[k] = q as u32;
                        hi[k] = (n - q * d) as u32;
                        lo[k + 1] = (sx32(lo[k]) >> 32) as u32;
                        hi[k + 1] = (sx32(hi[k]) >> 32) as u32;
                    }
                }
                self.set_hilo128(from_words(hi), from_words(lo));
                return Some(None);
            }
            0x0e => (b & M64) | ((a & M64) << 64),
            0x10 | 0x11 | 0x14 | 0x15 => {
                // pmaddh, phmadh, pmsubh, phmsbh
                let mut h = words(self.hi128()).map(|x| x as i32);
                let mut l = words(self.lo128()).map(|x| x as i32);
                let p: [i32; 8] = std::array::from_fn(|k| i32::from(ha[k] as i16) * i32::from(hb[k] as i16));
                match sa {
                    0x10 | 0x14 => {
                        let sg = if sa == 0x10 { 1 } else { -1 };
                        let idx = [
                            (false, 0),
                            (false, 1),
                            (true, 0),
                            (true, 1),
                            (false, 2),
                            (false, 3),
                            (true, 2),
                            (true, 3),
                        ];
                        for (k, (is_hi, j)) in idx.into_iter().enumerate() {
                            if is_hi {
                                h[j] = h[j].wrapping_add(sg * p[k]);
                            } else {
                                l[j] = l[j].wrapping_add(sg * p[k]);
                            }
                        }
                        self.set_hilo128(from_words(h.map(|x| x as u32)), from_words(l.map(|x| x as u32)));
                        from_words([l[0] as u32, h[0] as u32, l[2] as u32, h[2] as u32])
                    }
                    _ => {
                        let s = |x: i32, y: i32| if sa == 0x11 { x.wrapping_add(y) } else { x.wrapping_sub(y) };
                        let r = [s(p[1], p[0]), s(p[3], p[2]), s(p[5], p[4]), s(p[7], p[6])];
                        l = [r[0], 0, r[2], 0];
                        h = [r[1], 0, r[3], 0];
                        self.set_hilo128(from_words(h.map(|x| x as u32)), from_words(l.map(|x| x as u32)));
                        from_words(r.map(|x| x as u32))
                    }
                }
            }
            0x12 => a & b,
            0x13 => a ^ b,
            0x1a => from_halves([hb[2], hb[1], hb[0], hb[3], hb[6], hb[5], hb[4], hb[7]]),
            0x1b => from_halves([hb[3], hb[2], hb[1], hb[0], hb[7], hb[6], hb[5], hb[4]]),
            0x1c => {
                // pmulth
                let p: [i32; 8] = std::array::from_fn(|k| i32::from(ha[k] as i16) * i32::from(hb[k] as i16));
                let lo = [p[0], p[1], p[4], p[5]];
                let hi = [p[2], p[3], p[6], p[7]];
                self.set_hilo128(from_words(hi.map(|x| x as u32)), from_words(lo.map(|x| x as u32)));
                from_words([p[0] as u32, p[2] as u32, p[4] as u32, p[6] as u32])
            }
            0x1d => {
                // pdivbw
                let d = i32::from(hb[0] as i16);
                if d == 0 {
                    return Some(None);
                }
                let q: [u32; 4] = std::array::from_fn(|k| (wa[k] as i32).wrapping_div(d) as u32);
                let r: [u32; 4] = std::array::from_fn(|k| (wa[k] as i32).wrapping_rem(d) as u32);
                self.set_hilo128(from_words(r), from_words(q));
                return Some(None);
            }
            0x1e => from_words([wb[2], wb[1], wb[0], wb[3]]),
            0x1f => from_words([wb[1], wb[2], wb[0], wb[3]]),
            _ => return None,
        }))
    }

    /// MMI3: Some(Some(v)) writes rd, Some(None) does not.
    fn mmi3(&mut self, _rs: usize, a: u128, b: u128, sa: u32) -> Option<Option<u128>> {
        let (wa, wb, ha, hb) = (words(a), words(b), halves(a), halves(b));
        Some(Some(match sa {
            0x00 => {
                // pmadduw
                let (hi, lo) = (words(self.hi128()), words(self.lo128()));
                let mut out = [0u64; 2];
                let (mut h, mut l) = (hi, lo);
                for (j, k) in [(0usize, 0usize), (1, 2)] {
                    let acc = (u64::from(hi[k]) << 32) | u64::from(lo[k]);
                    let s = acc.wrapping_add(u64::from(wa[k]) * u64::from(wb[k]));
                    out[j] = s;
                    l[k] = s as u32;
                    h[k] = (s >> 32) as u32;
                    l[k + 1] = (sx32(s as u32) >> 32) as u32;
                    h[k + 1] = (sx32((s >> 32) as u32) >> 32) as u32;
                }
                self.set_hilo128(from_words(h), from_words(l));
                (u128::from(out[1]) << 64) | u128::from(out[0])
            }
            0x03 => {
                from_words([((wb[0] as i32) >> (wa[0] & 31)) as u32, 0, ((wb[2] as i32) >> (wa[2] & 31)) as u32, 0])
                    .pipe_sx()
            }
            0x08 => {
                self.set_hilo128(a, self.lo128());
                return Some(None);
            }
            0x09 => {
                self.set_hilo128(self.hi128(), a);
                return Some(None);
            }
            0x0a => from_halves([hb[0], ha[0], hb[2], ha[2], hb[4], ha[4], hb[6], ha[6]]),
            0x0c => {
                // pmultuw
                let p0 = u64::from(wa[0]) * u64::from(wb[0]);
                let p1 = u64::from(wa[2]) * u64::from(wb[2]);
                let lo =
                    from_words([p0 as u32, (sx32(p0 as u32) >> 32) as u32, p1 as u32, (sx32(p1 as u32) >> 32) as u32]);
                let hi = from_words([
                    (p0 >> 32) as u32,
                    (sx32((p0 >> 32) as u32) >> 32) as u32,
                    (p1 >> 32) as u32,
                    (sx32((p1 >> 32) as u32) >> 32) as u32,
                ]);
                self.set_hilo128(hi, lo);
                (u128::from(p1) << 64) | u128::from(p0)
            }
            0x0d => {
                // pdivuw
                let mut lo = [0u32; 4];
                let mut hi = [0u32; 4];
                for k in [0, 2] {
                    if let (Some(q), Some(r)) = (wa[k].checked_div(wb[k]), wa[k].checked_rem(wb[k])) {
                        lo[k] = q;
                        hi[k] = r;
                        lo[k + 1] = (sx32(lo[k]) >> 32) as u32;
                        hi[k + 1] = (sx32(hi[k]) >> 32) as u32;
                    }
                }
                self.set_hilo128(from_words(hi), from_words(lo));
                return Some(None);
            }
            0x0e => (a >> 64) | ((b >> 64) << 64),
            0x12 => a | b,
            0x13 => !(a | b),
            0x1a => from_halves([hb[0], hb[2], hb[1], hb[3], hb[4], hb[6], hb[5], hb[7]]),
            0x1b => from_halves([hb[0], hb[0], hb[0], hb[0], hb[4], hb[4], hb[4], hb[4]]),
            0x1e => from_words([wb[0], wb[2], wb[1], wb[3]]),
            _ => return None,
        }))
    }
}

/// A 128-bit value whose words 0 and 2 are 32-bit results sign-extended
/// into words 1 and 3 (psllvw, psrlvw, psravw).
trait PipeSx {
    fn pipe_sx(self) -> u128;
}

impl PipeSx for u128 {
    fn pipe_sx(self) -> u128 {
        let w = words(self);
        from_words([w[0], (sx32(w[0]) >> 32) as u32, w[2], (sx32(w[2]) >> 32) as u32])
    }
}
