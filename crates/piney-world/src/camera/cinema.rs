//! `ccBossEffCinemaFade` (gcmn 0x0046a7e0-0x0046b35c): the boss fight's
//! "cinema", black bars sliding in over the top and bottom of the picture
//! with the skill's name in the top one. `OnCinemaMode(n)` starts it for a
//! skill with a name in `_g_cinemaSkillName`; `CineMode` steps it: 2-3 the
//! bars slide in (each gap divided by 1.5 a frame) and the name fades in by
//! 1/15, 1 at rest, 4-5 out again. Each bar is a one-frame
//! `ccScFade::EntryFlash` black at alpha 0x80 (docs/engine/boss.md, "The
//! cinema").

use piney_data::anim::ee;
use piney_desktop::anm::Ctx;
use piney_desktop::fade;
use piney_desktop::sprite::Sprite;
use piney_desktop::view::{LayerView, View};
use piney_draw::{Cmd, TexRef};

/// The cinema's layer: `ccLayer::Init(241, sysLayer's view)`.
pub const CINEMA_LAYER: i16 = 241;
/// `CinemaHight`: the bars' height in logical lines.
pub const HEIGHT: i32 = 30;
/// `CinemaData.BotY`: the bottom bar's place at rest.
const BOT_Y: u32 = 0x43b1_0000;
/// The name's transparency step, 1/15 (0x3d888889).
const NAME_STEP: u32 = 0x3d88_8889;
/// The slide: `NowCinemaData` divided by 1.5 a frame.
const SLIDE: u32 = 0x3fc0_0000;
/// A bar: a one-frame flash from black at alpha 0x80.
const BAR_COLOUR: u32 = 0x8000_0000;
/// `_g_cinemaSkillName` (gcmn 0x005eb040): 72 rows of (file, texture, row);
/// `CinemaOn` takes 0 to 60.
pub const SKILL_NAMES: u32 = 0x005e_b040;
pub const SKILL_NAMES_MAX: i32 = 61;

/// `c.lt.s`, `c.le.s` on EE floats.
fn lt(a: u32, b: u32) -> bool {
    ee::to_f64(a) < ee::to_f64(b)
}

fn le(a: u32, b: u32) -> bool {
    ee::to_f64(a) <= ee::to_f64(b)
}

/// A skill's name: its texture (`SetTex(file, tex, 1)`), the texture's
/// height and the name's row in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Name {
    pub tex: TexRef,
    pub tex_h: i32,
    pub row: i32,
}

/// What one `Draw` sends: the name (its transparency and `wv`) and the two
/// bars' logical y, as the game's calls give them (f32 bits).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sent {
    pub name: Option<(u32, i32)>,
    pub bars: Option<[u32; 2]>,
}

/// `ccBossEffCinemaFade`.
#[derive(Clone, Debug, PartialEq)]
pub struct Cinema {
    /// `CineMode`.
    pub mode: u8,
    /// `NowCinemaData.TopY`, `BotY` (f32 bits).
    pub now_top: u32,
    pub now_bot: u32,
    /// `tempDat.TopY`, `BotY`: where the bars were last drawn. `Init` leaves
    /// them as the heap had them; the port takes a fresh block's zeros, so
    /// the first cinema's first frame draws both bars at the top.
    pub temp_top: u32,
    pub temp_bot: u32,
    /// `m_texTransparency`, `m_drawNameFlag`, `m_cinemaName`.
    pub name_tp: u32,
    pub draw_name: bool,
    pub name: Option<Name>,
}

impl Default for Cinema {
    fn default() -> Self {
        Cinema::new()
    }
}

impl Cinema {
    /// The constructor's `Clear` and `Init`: at rest, no name.
    pub fn new() -> Cinema {
        let h = ee::from_int(HEIGHT);
        Cinema {
            mode: 0,
            now_top: h,
            now_bot: ee::from_int(-HEIGHT),
            temp_top: 0,
            temp_bot: 0,
            name_tp: 0,
            draw_name: false,
            name: None,
        }
    }

    /// `ccBossEffManager::OnCinemaMode(n)` once its own checks pass (the
    /// party not wiped out, no forced game over): `CinemaOn` with `name`,
    /// `_g_cinemaSkillName[n]`'s when `n` is in range and has one.
    pub fn on(&mut self, name: Option<Name>) {
        if (1..=3).contains(&self.mode) {
            return;
        }
        if let Some(n) = name {
            self.name = Some(n);
            self.name_tp = 0;
            self.draw_name = true;
        }
        self.mode = 2;
    }

    /// `ccBossEffManager::OffCinemaMode`: `CinemaOff` unless it is off or
    /// going.
    pub fn off(&mut self) {
        if !matches!(self.mode, 0 | 4 | 5) {
            self.mode = 4;
        }
    }

    /// One `NowCinemaData` value's slide: divided by 1.5, 0 under 1 in
    /// size; whether it was still moving.
    fn slide(v: &mut u32) -> bool {
        if ee::eq(*v, 0) {
            return false;
        }
        *v = ee::div(*v, SLIDE);
        if ee::to_f64(*v).abs() < 1.0 {
            *v = 0;
        }
        true
    }

    /// `Draw`: the state's step and what it sends.
    pub fn step(&mut self) -> Sent {
        let mut out = Sent::default();
        if self.draw_name
            && let Some(n) = &self.name
        {
            out.name = Some((self.name_tp, n.row * 5 * 64));
        }
        let h = ee::from_int(HEIGHT);
        match self.mode {
            0 => self.draw_name = false,
            1 => {
                out.bars = Some([0, BOT_Y]);
                let one = 0x3f80_0000;
                if lt(self.name_tp, one) {
                    let t = ee::add(self.name_tp, NAME_STEP);
                    self.name_tp = if lt(t, one) { t } else { one };
                }
            }
            2 | 4 => {
                self.mode += 1;
                out.bars = Some([self.temp_top, self.temp_bot]);
            }
            3 | 5 => {
                let out_going = self.mode == 5;
                if out_going && !le(self.name_tp, 0) {
                    let t = ee::sub(self.name_tp, NAME_STEP);
                    self.name_tp = if le(t, 0) { 0 } else { t };
                }
                let mut moving = Cinema::slide(&mut self.now_top);
                if out_going {
                    self.temp_top = ee::sub(0, ee::sub(h, self.now_top));
                } else {
                    self.temp_top = ee::sub(0, self.now_top);
                }
                moving |= Cinema::slide(&mut self.now_bot);
                if out_going {
                    self.temp_bot = ee::add(BOT_Y, ee::add(h, self.now_bot));
                } else {
                    self.temp_bot = ee::sub(BOT_Y, self.now_bot);
                }
                out.bars = Some([self.temp_top, self.temp_bot]);
                if !moving {
                    self.mode = if out_going { 0 } else { 1 };
                    self.now_top = h;
                    self.now_bot = ee::from_int(-HEIGHT);
                }
            }
            _ => {}
        }
        out
    }

    /// `Draw`'s packets for `sent`: the name's sprite, then each bar's
    /// element, on [`CINEMA_LAYER`] (each send to the front of the layer).
    pub fn packets(&self, sent: &Sent) -> Vec<Cmd> {
        let mut ctx = Ctx::new(View::default());
        if let (Some((tp, wv)), Some(n)) = (sent.name, &self.name) {
            let mut sp = Sprite::mask(CINEMA_LAYER, n.tex.clone(), n.tex_h, 128);
            sp.transp = f32::from_bits(tp);
            sp.wu = 0;
            sp.wv = wv;
            sp.wi = 1;
            sp.sx = 256.0;
            sp.sy = 20.0;
            sp.su = 256;
            sp.sv = 20;
            sp.dx = 128.0;
            sp.dy = 10.0;
            sp.make_packet(0, &LayerView::default_layer());
            sp.send(&mut ctx.layers);
        }
        if let Some(bars) = sent.bars {
            for y in bars {
                let rect = [0.0, f32::from_bits(y), 512.0, HEIGHT as f32];
                fade::draw_rect(&mut ctx, CINEMA_LAYER, rect, BAR_COLOUR, BAR_COLOUR & 0x00ff_ffff, 0, 1);
            }
        }
        ctx.layers.flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On, the bars slide in over a dozen frames and hold; off, they slide
    /// back out and the cinema rests.
    #[test]
    fn slides_in_and_out() {
        let mut c = Cinema::new();
        c.on(None);
        let mut frames = 0;
        while c.mode != 1 {
            c.step();
            frames += 1;
            assert!(frames < 40);
        }
        assert_eq!(c.step().bars, Some([0, BOT_Y]));
        c.off();
        while c.mode != 0 {
            c.step();
            frames += 1;
            assert!(frames < 80);
        }
        assert_eq!(c.step(), Sent::default());
    }
}
