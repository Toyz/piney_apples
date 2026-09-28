//! `ccThDfComp` (gcmn 0x004008c0, priority 33, stack 4096): the banner an
//! area shows when its last magic portal is opened. The entry control
//! starts it (`ccEntryObj::routine`'s circle, act 4, when `g_entCtrl`
//! +0x1c is 1: `piney_battle::entry::Out::AreaCleared`); the task runs
//! `ccDfComp::Main` (0x00400980) once a frame until it answers 0. `Init`
//! (0x00400da0) picks the banner's rows from the area's kind; the 90
//! frames' alpha, slide and scale are in docs/engine/field-ui.md (all
//! portals open).

use piney_desktop::eef;

use crate::spr::{Obj, Packet, Spr};

/// `ccDfComp`'s state (0x2c bytes).
#[derive(Clone, Debug)]
pub struct DfComp {
    /// +0x04 the frames left, +0x0c the total (90).
    count: i32,
    total: i32,
    /// +0x08 the alpha, +0x10 the scale, +0x1c the halves' spread.
    alpha: i32,
    scale: f32,
    spread: f32,
    /// +0x14 the texture's row, +0x18 the top half's x.
    row: i32,
    x: i32,
    spr: Spr,
}

impl DfComp {
    /// `new ccDfComp` + `Clear` + `Init`: `area` is `game+0x14` (1 a
    /// field, 2 a dungeon), with the field type and `game.dungeon`.
    pub fn new(area: i32, field_type: i32, dungeon: i32) -> DfComp {
        let (row, x) = match area {
            1 => (33, 0),
            2 if field_type == 4 && dungeon == 0 => (33, 0),
            2 => (0, 5),
            _ => (0, 0),
        };
        DfComp { count: 90, total: 90, alpha: 0, scale: 2.0, spread: 0.0, row, x, spr: Spr::new(Obj::DfComp, 20) }
    }

    /// `ccDfComp::Main`: false once it is done. The frame's packets are in
    /// [`DfComp::take`].
    pub fn main(&mut self) -> bool {
        let fade_in = self.total - 15;
        if self.count < 6 {
            if self.count != 0 {
                self.count -= 1;
                return true;
            }
            self.count = self.total;
            self.scale = 2.0;
            self.spread = 0.0;
            return false;
        }
        self.alpha = if fade_in < self.count {
            (self.total - self.count) * 96 / 15
        } else if self.count >= 16 {
            96
        } else {
            (self.count - 5) * 96 / 10
        };
        let mut slide = 0.0f32;
        if fade_in < self.count {
            let mut step = 0.5f32;
            for _ in 0..self.count - fade_in {
                step = eef::add(step, eef::div(step, 2.75));
                slide = eef::add(slide, step);
            }
        }
        let s = &mut self.spr;
        s.set_colour(6);
        s.set_alpha(self.alpha);
        if self.count < 15 {
            self.scale = eef::add(self.scale, f32::from_bits(0x3d75_c28f));
            self.spread = eef::add(self.spread, 0.5);
        }
        let k = self.scale;
        for (half, sv, dx, dy) in [
            (0, 16, eef::add(eef::sub(252.0, slide), eef::from_int(self.x)), eef::sub(110.0, self.spread)),
            (16, 15, eef::add(260.0, slide), eef::add(147.0, self.spread)),
        ] {
            s.set_grid(128, sv, eef::mul(128.0, k), eef::mul(sv as f32, k), 0, (self.row + half) << 4, 1);
            s.cx = eef::mul(-64.0, k);
            s.cy = eef::mul(-9.0, k);
            s.dx = dx;
            s.dy = dy;
            s.make_packet(0);
        }
        self.count -= 1;
        true
    }

    /// `SendPacket`: the frame's packets.
    pub fn take(&mut self) -> Vec<Packet> {
        self.spr.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 85 frames drawn (90 down to 6), five more with nothing, then the
    /// end; the alpha rising over the first 15 to 96 and falling over the
    /// last 10; the halves sliding in and parting as the scale grows.
    #[test]
    fn its_frames() {
        let mut d = DfComp::new(1, 7, -1);
        let mut drawn = Vec::new();
        let mut frames = 0;
        while d.main() {
            frames += 1;
            let p = d.take();
            if !p.is_empty() {
                assert_eq!(p.len(), 2);
                drawn.push((p[0].rgba[3], p[0].dx, p[1].dx, p[0].dy, p[0].sx, p[0].wv, p[1].wv));
            }
            assert!(frames < 200);
        }
        assert_eq!((frames, drawn.len()), (90, 85));
        let alphas: Vec<u8> = drawn.iter().map(|d| d.0).collect();
        assert_eq!(alphas[..3], [0, 6, 12]);
        assert!(alphas[15..75].iter().all(|&a| a == 96));
        assert_eq!(alphas[84], 9);
        // Row 33 in a field: the halves at 33 and 49 (in 1/16 texels).
        assert_eq!((drawn[0].5, drawn[0].6), (33 << 4, 49 << 4));
        // Slid in by frame 15, the halves then at 252 and 260.
        assert!(drawn[0].1 < 150.0 && drawn[0].2 > 360.0);
        assert_eq!((drawn[20].1, drawn[20].2), (252.0, 260.0));
        // The scale 2.0 until the last 14, then growing, the top rising.
        assert_eq!(drawn[20].4, 256.0);
        assert!(drawn[84].4 > 256.0 && drawn[84].3 < 110.0);
        // A dungeon: row 0, the top half 5 to the right.
        let mut d = DfComp::new(2, 7, 0);
        for _ in 0..20 {
            d.main();
            d.take();
        }
        d.main();
        let p = d.take();
        assert_eq!((p[0].wv, p[0].dx, p[1].dx), (0, 257.0, 260.0));
    }
}
