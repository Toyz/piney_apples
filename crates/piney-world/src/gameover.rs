//! The game over's picture (`ccThGameOver`, gcmn 0x00516a80, and
//! `ccGameOverNoise`, 0x00516d30-0x005177f4): bursts of `ccNoiz` over the
//! frozen field, then the screen switched off like a television, the
//! picture squeezed to a line (sound 90, a white flash) and the line to a
//! dot (`docs/engine/field-walk.md`, "The game over").
//!
//! [`GameOverNoise::main`] is one frame of `ccGameOverNoise::Main`: what it
//! drew (`ccNoiz::Draw`), the sounds (`ccSeOn`), the flash
//! (`ccScFade::EntryFlash` on sysLayer) and sysLayer's view frame
//! (`ccView::SetFrame`) it left.

use piney_desktop::noiz::Noiz;
use piney_draw::Cmd;

/// `ccView::SetFrame(x, y, w, h, cx, cy, ax, ay)` as the noise leaves
/// sysLayer's view: in the view's 512 x 384 units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewFrame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub cx: f32,
    pub cy: f32,
    pub ax: f32,
    pub ay: f32,
}

/// What one frame of the noise did.
#[derive(Default)]
pub struct NoiseFrame {
    /// `ccNoiz::Draw`'s packets (its own layer, 241).
    pub cmds: Vec<Cmd>,
    /// `ccSeOn(n)`, in order.
    pub se: Vec<i32>,
    /// `EntryFlash(6, 0x80ffffff, 0, 0, 512, 384)` on the noise's own
    /// `ccScFade` (on sysLayer).
    pub flash: bool,
    /// That fader's `SendPacket` this frame: only `StretchTV_Y` sends it, so
    /// a flash shows only while the picture is squeezing to its line.
    pub send_fade: bool,
    /// The TV has started: `ccSys.bgColor` 0 (the frame cleared black), and
    /// in a dungeon `DUNGEON.fog` (+0x50) 0.
    pub black: bool,
}

/// `ccGameOverNoise` (0xa8 bytes).
#[derive(Clone, Debug)]
pub struct GameOverNoise {
    /// +0x00.
    pub noiz: Noiz,
    /// +0x04 frames of noise left, +0x08 set by every burst.
    pub cnt: i32,
    pub started: bool,
    /// +0x0c the television's phase: 0 not yet, 1 squeezing to a line,
    /// 2-4 a frame each, 5 the line to a dot, 6 done.
    pub tv: i32,
    /// +0x10 the noise's phase (1, 2, 3, then 0, 4 once the TV is done),
    /// +0x14 its frames.
    pub state: i32,
    pub timer: i32,
    /// +0x18, +0x1c: frames to the next pick of each.
    pub rn: [i32; 2],
    /// +0x20: the flash still to come.
    pub flash: bool,
    /// +0x30: `Main`'s answer; the task steps it while it is set.
    pub active: bool,
    /// +0x84 .. +0x90: the picture's width, height and place.
    pub w: i32,
    pub h: i32,
    pub x: i32,
    pub y: i32,
    /// +0x9c: the squeeze (f32).
    pub scale: f32,
    /// sysLayer's view as the noise last set it; None: untouched.
    pub view: Option<ViewFrame>,
}

/// `abs(v % m)` with C's remainder.
fn cmod(v: i32, m: i32) -> i32 {
    (v % m).abs()
}

impl GameOverNoise {
    /// `ccGameOverNoise::ccGameOverNoise` (`InitData`, 0x005175d0): the
    /// `ccNoiz` made (its bands from newlib's `rand`), the picture whole
    /// (512 x 384), the noise at phase 1.
    pub fn new(rand: &mut dyn FnMut() -> i32) -> GameOverNoise {
        GameOverNoise {
            noiz: Noiz::new(rand),
            cnt: 0,
            started: false,
            tv: 0,
            state: 1,
            timer: 0,
            rn: [0; 2],
            flash: true,
            active: true,
            w: 512,
            h: 384,
            x: 0,
            y: 0,
            scale: 0.0,
            view: None,
        }
    }

    /// `Main` (0x00516d30): `NOISE`, and `TV` while active; whether it is
    /// still active. `cc` is `ccRand`, `rand` newlib's.
    pub fn main(&mut self, cc: &mut dyn FnMut() -> i32, rand: &mut dyn FnMut() -> i32) -> (bool, NoiseFrame) {
        let mut out = NoiseFrame::default();
        let tv = self.active;
        self.noise(cc, rand, &mut out);
        if tv {
            self.television(&mut out);
        }
        (self.active, out)
    }

    /// `NOISE` (0x00516d90).
    fn noise(&mut self, cc: &mut dyn FnMut() -> i32, rand: &mut dyn FnMut() -> i32, out: &mut NoiseFrame) {
        match self.state {
            1 | 2 => {
                if self.cnt == 0 && cmod(cc(), 8) < 4 {
                    if self.state == 1 { self.noise2(cc) } else { self.noise3(cc) }
                }
                let t = self.timer;
                self.timer += 1;
                if t == if self.state == 1 { 50 } else { 40 } {
                    self.state += 1;
                    self.timer = 0;
                }
            }
            3 => {
                out.black = true;
                self.noise1(cc);
                self.state = 0;
                self.timer = 0;
                self.tv = 1;
            }
            4 if self.cnt == 0 && cmod(cc(), 8) < 2 => self.noise4(cc),
            _ => {}
        }
        if self.cnt <= 0 {
            return;
        }
        if self.cnt % 10 == 1 && self.tv == 0 {
            out.se.push(94);
        }
        if self.rn[0] <= 0 {
            self.rn[0] = cmod(cc(), 50);
        }
        if self.rn[1] <= 0 {
            self.rn[1] = cmod(cc(), 50);
        }
        if self.rn[0] >= 0 {
            self.noiz.set_rn(1, rand);
        }
        out.cmds.extend(self.noiz.draw());
        self.cnt -= 1;
        self.rn[0] -= 1;
        self.rn[1] -= 1;
    }

    /// `Noise` (0x00517150): 30-149 frames of the sampling and the
    /// inversion.
    fn noise1(&mut self, cc: &mut dyn FnMut() -> i32) {
        self.cnt = cmod(cc(), 120) + 30;
        self.noiz.set_br(self.cnt);
        self.noiz.set_bs(self.cnt);
        self.started = true;
    }

    /// `Noise2` (0x005171d0): 0-9 frames of the sampling.
    fn noise2(&mut self, cc: &mut dyn FnMut() -> i32) {
        self.cnt = cmod(cc(), 10);
        self.noiz.set_bs(self.cnt);
        self.started = true;
    }

    /// `Noise3` (0x00517240): 0-9 frames of the sampling and the inversion.
    fn noise3(&mut self, cc: &mut dyn FnMut() -> i32) {
        self.cnt = cmod(cc(), 10);
        self.noiz.set_br(self.cnt);
        self.noiz.set_bs(self.cnt);
        self.started = true;
    }

    /// `Noise4` (0x005172c0): 20-29 frames of the sampling.
    fn noise4(&mut self, cc: &mut dyn FnMut() -> i32) {
        self.cnt = cmod(cc(), 10) + 20;
        self.noiz.set_bs(self.cnt);
        self.started = true;
    }

    /// `TV` (0x005170c0).
    fn television(&mut self, out: &mut NoiseFrame) {
        match self.tv {
            0 => {}
            1 => self.stretch_y(out),
            5 => self.stretch_x(),
            // StretchTV_Init: done; the noise's last phase.
            6 => {
                self.active = false;
                self.state = 4;
            }
            _ => self.tv += 1,
        }
    }

    /// `StretchTV_Y` (0x00517330): the height halved (by at least 1) about
    /// the middle; at 1, sound 90 and the flash; the view squeezed to it.
    fn stretch_y(&mut self, out: &mut NoiseFrame) {
        self.h -= (self.h >> 1).max(1);
        if self.h == 1 {
            self.tv += 1;
            self.flash = true;
            out.se.push(90);
        }
        self.y = (384 - self.h) / 2;
        self.scale = self.h as f32 / 384.0;
        // `_dpfgt(0.5, scale)`: the flash once the squeeze is under half.
        if self.flash && 0.5 > f64::from(self.scale) {
            out.flash = true;
            self.flash = false;
        }
        out.send_fade = true;
        let h = self.h as f32;
        self.view =
            Some(ViewFrame { x: 0.0, y: self.y as f32, w: 512.0, h, cx: 256.0, cy: h / 2.0, ax: 1.0, ay: self.scale });
    }

    /// `StretchTV_X` (0x005174d0): the width less a quarter (at least 1)
    /// about the middle; at 1, done.
    fn stretch_x(&mut self) {
        self.w -= (self.w >> 2).max(1);
        if self.w == 1 {
            self.tv = 6;
        }
        self.x = (512 - self.w) / 2;
        self.scale = self.w as f32 / 512.0;
        let (w, h) = (self.w as f32, self.h as f32);
        self.view = Some(ViewFrame {
            x: self.x as f32,
            y: self.y as f32,
            w,
            h,
            cx: w / 2.0,
            cy: h / 2.0,
            ax: 1.0,
            ay: self.scale,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A generator that answers from a list, then zeros.
    fn from(v: Vec<i32>) -> impl FnMut() -> i32 {
        let mut it = v.into_iter();
        move || it.next().unwrap_or(0)
    }

    /// With every `ccRand` 0: phase 1's 51 frames each pick a burst of 0
    /// frames (0 % 8 < 4), phase 2's 41 likewise, then phase 3 a burst of
    /// 30 and the TV: the height 384 halves to 1 in 9 frames, three frames
    /// pass, the width 512 loses a quarter at a time to 1, and the noise is
    /// done. The flash fires twice: `InitData` leaves it armed, so once the
    /// squeeze is under half (the second frame), and again with sound 90 at
    /// the line.
    #[test]
    fn the_television_switches_off() {
        let mut rand = from(vec![]);
        let mut n = GameOverNoise::new(&mut rand);
        let mut cc = from(vec![]);
        let mut frames = 0;
        let mut flashes = Vec::new();
        let mut se = Vec::new();
        loop {
            frames += 1;
            let (active, out) = n.main(&mut cc, &mut rand);
            if out.flash {
                flashes.push(frames);
            }
            se.extend(out.se);
            if !active {
                break;
            }
            assert!(frames < 400);
        }
        // 51 + 41 frames of phases 1 and 2, then the TV from phase 3's frame.
        assert_eq!(n.tv, 6);
        assert_eq!((n.w, n.h), (1, 1));
        let tv = 51 + 41 + 1;
        assert_eq!(flashes, [tv + 1, tv + 8]);
        assert!(se.contains(&90));
        assert_eq!(n.state, 4);
    }
}
