//! `ccScFade` (fade.cpp) with its four elements, as the title uses it: the
//! flash at the end of the opening stream, the fade to black before the
//! attract loop and before a load (`EntryFlash` 0x00160240, `EntryFlash3`
//! 0x00160360, `EntryFade` 0x00160400, `CheckFade` 0x001604d0, `SendPacket`
//! 0x0015fb80), and the field menu's gate hack (`EntryFlash2` 0x001602d0).
//!
//! An element's `status` is a set of bits: 1 draws it, 2 ends it after its
//! current ramp, 4 and 8 chain further ramps. Each frame `SendPacket` draws
//! every element with bit 1, colour per channel
//! `c0 + trunc((c1 - c0) * cnt / tcnt)`, then counts `cnt` up; when it passes
//! `tcnt`:
//!
//! ```text
//! status & 2   the element ends (status 0)
//! status & 4   status = status & !4 | 2; cnt 0; tcnt = tcnt1; c0 = c1; c1 = c1 with alpha 0
//! status & 8   status = status & !8 | 4; cnt 0; tcnt = tcnt2; c0 = c1
//! otherwise    cnt = tcnt: the element holds at c1
//! ```
//!
//! So `EntryFlash(t, c)` (status 3) fades from `c` to clear over `t + 1`
//! frames; `EntryFlash3(t0, t1, t2, c)` (status 9) fades in over `t0 + 1`
//! frames, holds `c` for `t2 + 1` and fades out over `t1 + 1`; `EntryFade`
//! (status 1) ramps once and holds. Every element is drawn by
//! `piney_desktop::fade::draw_colours` on the font layer: the title only
//! ever fades the whole 512 x 384 screen.

use piney_desktop::anm::Ctx;

/// Elements in a `ccScFade` (`elm[4]`).
pub const ELEMENTS: usize = 4;

/// `ccScFadeElement.status` bits.
pub const DRAW: i16 = 1;
pub const LAST: i16 = 2;
pub const THEN_OUT: i16 = 4;
pub const THEN_HOLD: i16 = 8;

/// One `ccScFadeElement` (0x24 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Element {
    pub status: i16,
    pub cnt: i16,
    pub tcnt: i16,
    pub tcnt1: i16,
    pub tcnt2: i16,
    /// RGBA with R in the low byte, alpha 0x80 = opaque.
    pub col0: u32,
    pub col1: u32,
}

/// `ccScFade`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScFade {
    pub elm: [Element; ELEMENTS],
}

impl ScFade {
    /// The first free element, as every `Entry*` takes it; -1 when all four
    /// are busy.
    fn entry(&mut self, e: Element) -> i32 {
        for (i, slot) in self.elm.iter_mut().enumerate() {
            if slot.status == 0 {
                *slot = e;
                return i as i32;
            }
        }
        -1
    }

    /// `EntryFlash(t, c0, ...)`: from `c0` to `c0` with alpha 0 over `t`.
    pub fn entry_flash(&mut self, t: i16, c0: u32) -> i32 {
        self.entry(Element { status: DRAW | LAST, tcnt: t, col0: c0, col1: c0 & 0x00ff_ffff, ..Element::default() })
    }

    /// `EntryFlash2(t0, t1, c0, ...)` (0x001602d0, status 5): in over `t0`,
    /// then out over `t1`.
    pub fn entry_flash2(&mut self, t0: i16, t1: i16, c0: u32) -> i32 {
        self.entry(Element {
            status: DRAW | THEN_OUT,
            tcnt: t0,
            tcnt1: t1,
            col0: c0 & 0x00ff_ffff,
            col1: c0,
            ..Element::default()
        })
    }

    /// `EntryFlash3(t0, t1, t2, c0, ...)`: in over `t0`, hold for `t2`, out
    /// over `t1`.
    pub fn entry_flash3(&mut self, t0: i16, t1: i16, t2: i16, c0: u32) -> i32 {
        self.entry(Element {
            status: DRAW | THEN_HOLD,
            tcnt: t0,
            tcnt1: t1,
            tcnt2: t2,
            cnt: 0,
            col0: c0 & 0x00ff_ffff,
            col1: c0,
        })
    }

    /// `EntryFade(t, c0, c1, ...)`: from `c0` to `c1` over `t`, then held.
    pub fn entry_fade(&mut self, t: i16, c0: u32, c1: u32) -> i32 {
        self.entry(Element { status: DRAW, tcnt: t, col0: c0, col1: c1, ..Element::default() })
    }

    /// `CheckFade(n)`: element `n` is still ramping (`cnt < tcnt`).
    pub fn check(&self, n: usize) -> bool {
        self.elm.get(n).is_some_and(|e| e.cnt < e.tcnt)
    }

    /// Whether any element draws.
    pub fn active(&self) -> bool {
        self.elm.iter().any(|e| e.status & DRAW != 0)
    }

    /// `SendPacket`'s bookkeeping for one frame, without drawing: each
    /// drawing element's colour this frame, then the count moved on.
    pub fn advance(&mut self) -> Vec<(u32, u32, i16, i16)> {
        let mut drawn = Vec::new();
        for e in &mut self.elm {
            if e.status & DRAW == 0 {
                continue;
            }
            drawn.push((e.col0, e.col1, e.cnt, e.tcnt));
            e.cnt += 1;
            if e.cnt > e.tcnt {
                if e.status & LAST != 0 {
                    e.status = 0;
                } else if e.status & THEN_OUT != 0 {
                    e.status = (e.status & !THEN_OUT) | LAST;
                    e.cnt = 0;
                    e.tcnt = e.tcnt1;
                    e.col0 = e.col1;
                    e.col1 &= 0x00ff_ffff;
                } else if e.status & THEN_HOLD != 0 {
                    e.status = (e.status & !THEN_HOLD) | THEN_OUT;
                    e.cnt = 0;
                    e.tcnt = e.tcnt2;
                    e.col0 = e.col1;
                } else {
                    e.cnt = e.tcnt;
                }
            }
        }
        drawn
    }

    /// `SendPacket`: every drawing element on the font layer, then counted.
    pub fn send(&mut self, ctx: &mut Ctx) {
        for (c0, c1, cnt, tcnt) in self.advance() {
            draw(ctx, c0, c1, cnt, tcnt);
        }
    }
}

/// The colour an element draws at `cnt` of `tcnt`: 64-bit signed per
/// channel, masked to a byte (`SendPacket` 0x0015fca8..0x0015fe48). The
/// title never enters a zero `tcnt`; this takes the step as 0 there, which
/// is not checked against `__divdi3`.
pub fn colour(c0: u32, c1: u32, cnt: i16, tcnt: i16) -> [u8; 4] {
    let mut out = [0u8; 4];
    for (i, o) in out.iter_mut().enumerate() {
        let a = i64::from((c0 >> (8 * i)) & 0xff);
        let b = i64::from((c1 >> (8 * i)) & 0xff);
        let t = i64::from(tcnt);
        let step = if t == 0 { 0 } else { (b - a) * i64::from(cnt) / t };
        *o = ((a + step) & 0xff) as u8;
    }
    out
}

fn draw(ctx: &mut Ctx, c0: u32, c1: u32, cnt: i16, tcnt: i16) {
    // draw_colours takes the same formula with unsigned counts; the counts
    // here are never negative.
    piney_desktop::fade::draw_colours(ctx, c0, c1, cnt.max(0) as u32, tcnt.max(0) as u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alphas(f: &mut ScFade, frames: usize) -> Vec<u8> {
        (0..frames).map(|_| f.advance().first().map_or(0xff, |&(c0, c1, cnt, t)| colour(c0, c1, cnt, t)[3])).collect()
    }

    #[test]
    fn flash3_goes_in_holds_and_out() {
        let mut f = ScFade::default();
        assert_eq!(f.entry_flash3(20, 1, 1, 0x8000_0000), 0);
        let a = alphas(&mut f, 26);
        // In over 21 frames (0 .. 128), hold 2, out 2, then gone.
        assert_eq!(&a[..3], &[0, 6, 12]);
        assert_eq!(a[20], 128);
        assert_eq!(&a[21..23], &[128, 128]);
        assert_eq!(&a[23..25], &[128, 0]);
        assert_eq!(a[25], 0xff);
        assert!(!f.active());
    }

    #[test]
    fn flash_fades_to_clear() {
        let mut f = ScFade::default();
        f.entry_flash(50, 0x80ff_ffff);
        let a = alphas(&mut f, 52);
        assert_eq!(a[0], 128);
        assert_eq!(a[50], 0);
        assert_eq!(a[51], 0xff);
    }

    #[test]
    fn fade_holds() {
        let mut f = ScFade::default();
        f.entry_fade(20, 0, 0x8000_0000);
        let a = alphas(&mut f, 30);
        assert_eq!(a[20], 128);
        assert_eq!(a[29], 128);
        assert!(!f.check(0));
    }
}
