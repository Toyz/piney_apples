//! A character on the field: `ccChar` (gcmn chara.cpp, 0xe0 bytes) as every
//! entry-controlled character extends it (`ccEntryObj`, entctrl.cpp, 0x1d0
//! bytes; `ccGimmick`; `ccMerchan`, `ccRtownPC`): a body posed by one
//! animation, a place and heading, a transparency, a ground attribute, a
//! collision body. [`Char::fade`] and [`Char::draw`] are `ccChar::Draw`
//! (0x0056b1c0) in a town, with `ccGetCameraTransparency` (INF main
//! 0x001da8b0). The layouts are in docs/engine/field-game.md.

use std::rc::Rc;

use glam::Mat4;
use piney_desktop::layers::Layers;

use crate::body::{Body, root};
use crate::ee::{self, F, ONE, V4};
use crate::hit;
use crate::pose::Play;
use crate::town::TownLights;

/// `ccChar::Draw` in a town: `ccGetCameraTransparency(.., 4000, 400)`, and
/// nothing drawn under 0.05.
pub const FADE_FAR: F = 0x457a_0000;
pub const FADE_LEN: F = 0x43c8_0000;
pub const MIN_DRAWN: F = 0x3d4c_cccd;
/// `WORLD_MAN::GO`'s bounds in a town, which `ccPlayer::W2PPos` wraps by.
const BOUND: F = 0x46bb_8000; // 24000
/// `hitAttribute` bit: shaded ground (`WORLD_MAN::SleepDistantLight`).
pub const SHADED: u32 = 0x4_0000;

/// `ccCalcTagPosChar(c, out, offset, mode)` (gcmn 0x0051a500) for a
/// character at `pos`: `ccCalcTagPos` (0x0051a640) puts `pos + offset`
/// (w 1) through `sceVu0RotTransPers(world_screen)`, and with the depth in
/// (0, 0x0fffffff) the point less the GS offset in pixels,
/// `fptosi(x - 28672.0) / 16` and `(y - 29184) / 16` (rounding toward
/// zero); None when behind the camera. The answer: with `mode` 0, 1 when
/// the point is in (-127..640, -31..480), else 0; otherwise 1 inside
/// (-19..532, -15..464), else 2.
pub fn calc_tag_pos(world_screen: &[V4; 4], pos: V4, offset: V4, mode: i32) -> Option<(i32, i32, i32)> {
    let mut p = ee::vadd(pos, offset);
    p[3] = ONE;
    let v = ee::rot_trans_pers(world_screen, p);
    if !(v[2] > 0 && v[2] < 0x0fff_ffff) {
        return None;
    }
    let x = ee::to_int(ee::sub(ee::from_int(v[0]), 0x46e0_0000)) / 16;
    let y = v[1].wrapping_sub(0x8000).wrapping_add(0xe00) / 16;
    let r = if mode == 0 {
        i32::from((-127..640).contains(&x) && (-31..480).contains(&y))
    } else if (-19..532).contains(&x) && (-15..464).contains(&y) {
        1
    } else {
        2
    };
    Some((x, y, r))
}

/// `ccPlayer::W2PPos` (gcmn 0x0059b5a0) in a town: x and y less the
/// player's, wrapped into the map's bounds; z kept; w 1.
pub fn w2p(pos: V4, player: V4) -> V4 {
    let (lo, hi) = (ee::neg(BOUND), BOUND);
    let span = ee::sub(hi, lo);
    let half = ee::div(ee::add(lo, hi), 0x4000_0000);
    let wrap = |v: F| -> F {
        if ee::lt(v, ee::sub(lo, half)) {
            ee::add(v, span)
        } else if !ee::le(v, ee::sub(hi, half)) {
            ee::add(v, ee::sub(lo, hi))
        } else {
            v
        }
    };
    [wrap(ee::sub(pos[0], player[0])), wrap(ee::sub(pos[1], player[1])), pos[2], ONE]
}

/// `ccPlayer::P2WPos` (gcmn 0x0059b710) in a town: `pp` plus the
/// player's position (every lane, w then 1), x and y wrapped back into the
/// map's bounds on the player's side of its centre; z `pp`'s own.
pub fn p2w(pp: V4, player: V4) -> V4 {
    let (lo, hi) = (ee::neg(BOUND), BOUND);
    let half = ee::div(ee::add(lo, hi), 0x4000_0000);
    let wrap = |v: F, p: F, pl: F| -> F {
        if ee::lt(pl, half) {
            if ee::le(p, ee::sub(hi, half)) { v } else { ee::add(v, ee::sub(lo, hi)) }
        } else if ee::lt(p, ee::sub(lo, half)) {
            ee::add(v, ee::sub(hi, lo))
        } else {
            v
        }
    };
    let s = ee::vadd(pp, player);
    [wrap(s[0], pp[0], player[0]), wrap(s[1], pp[1], player[1]), pp[2], ONE]
}

/// `ccGetCameraTransparency(pos, width, height, far, fadeLen, &hide)`
/// (`INF SLUS_202.67:0x001da8b0`) for anyone at `pos`, the player at
/// `player` and the camera at `cam` with pitch `deg1`: toward 0 as the
/// camera comes within 1.25 r of it (r the pitch's blend of width and
/// height, at least 180; `hide` set), and beyond `far`.
#[allow(clippy::too_many_arguments)]
pub fn camera_transparency(
    pos: V4,
    player: V4,
    cam: V4,
    deg1: i16,
    width: F,
    height: F,
    far: F,
    fade: F,
    hide: &mut bool,
) -> F {
    let p = w2p(pos, player);
    let c = w2p(cam, player);
    let d = ee::vsub(c, p);
    let dist = ee::sqrtf(ee::dot(d, d));
    let t = ee::div(ee::from_int(8192 - i32::from(deg1)), 0x4600_0000);
    let u = ee::sub(ONE, t);
    let mut r = ee::add(ee::mul(width, t), ee::mul(height, u));
    if ee::lt(r, 0x4334_0000) {
        r = 0x4334_0000;
    }
    let r2 = ee::mul(0x3fa0_0000, r);
    if !*hide && ee::lt(dist, r2) {
        *hide = true;
        let near = ee::mul(0x3f4c_cccd, r);
        let f = ee::sub(dist, near);
        return if ee::le(f, 0) { 0 } else { ee::div(f, ee::sub(r2, near)) };
    }
    *hide = false;
    if ee::le(far, 0) || ee::le(dist, far) {
        return ONE;
    }
    let over = ee::sub(dist, far);
    if ee::le(over, fade) { ee::div(ee::sub(fade, over), fade) } else { 0 }
}

/// Where the camera is, for `ccChar::Draw`'s fade.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// The player's position (`plw->pos`).
    pub player: V4,
    /// The active camera's eye and pitch (`activeCamPtr->pos`, `->deg[1]`:
    /// `tcam`'s, or `ecam`'s while an event runs its camera).
    pub cam: V4,
    pub deg1: i16,
    /// The eye view (`checkCameraType() == 1`).
    pub eye: bool,
}

/// A character's drawable state.
#[derive(Clone)]
pub struct Char {
    pub body: Rc<Body>,
    /// `ccEntryChangeCLUT`: palette swaps of the body's own models.
    pub clut_swaps: Vec<(u32, u32)>,
    /// The base parameters' +0x18 height and +0x1c width.
    pub height: F,
    pub width: F,
    /// +0x40, +0x50, +0x60.
    pub pos: V4,
    pub pos_p: V4,
    pub dirc: V4,
    /// +0xd4: what the anm plays.
    pub play: Play,
    /// +0x80.
    pub hit_attribute: u32,
    /// +0x88 and +0x8c.
    pub transparency: F,
    pub set_transparency: F,
    /// +0x90 1: `ccChar::Draw`'s near fade on (`ccChar::ccChar` sets 1).
    pub trans_dist: bool,
    /// `ccEntryObj::dispSW`.
    pub disp: bool,
    /// What the last [`Char::fade`] decided.
    pub drawn: bool,
    /// And the transparency its shadow's alpha comes from:
    /// `setTransparency` while the camera fade is off (`hide`), else the
    /// transparency.
    pub shadow_t: F,
    /// +0x160 `bodyHit`.
    pub hit: hit::Body,
    /// +0xa6-+0xac: the affect's tint.
    pub affect: AffectColour,
    /// The blend the last [`Char::fade`] set (`ccDrawEnv::SetFogBlend`).
    pub blend: Option<piney_draw::Fog>,
}

/// `affectColorFix` (+0xa6), `affectColorCnt` (+0xa8), `affectColorRate`
/// (+0xaa) and `affectColor` (+0xac).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AffectColour {
    pub fix: i16,
    pub cnt: i16,
    pub rate: i16,
    pub colour: u32,
}

impl AffectColour {
    /// `ccChar::Draw`'s blend for a character alive (gcmn 0x0056b284): with
    /// the fix, the rate as it is; else a flash counting its rate down by
    /// its count, cleared at 0; none at rate 0 (the condition's tint, which
    /// a town's characters do not have, comes after).
    pub fn blend(&mut self) -> Option<piney_draw::Fog> {
        if self.fix == 0 {
            if self.rate == 0 {
                return None;
            }
            self.rate = self.rate.wrapping_add(self.cnt);
            if self.rate <= 0 {
                self.rate = 0;
                self.colour = 0;
            }
        }
        crate::foe::FogBlend { rate: ee::from_int(i32::from(self.rate)), colour: self.colour }.fog()
    }
}

impl Char {
    /// A character with `body` at `pos` facing `dirc`, playing `anim` from
    /// frame 0, of the given size; opaque, drawn.
    pub fn new(body: Rc<Body>, anim: &str, pos: V4, dirc: V4, height: F, width: F) -> Option<Char> {
        let play = body.play(anim)?;
        Some(Char {
            body,
            clut_swaps: Vec::new(),
            height,
            width,
            pos,
            pos_p: pos,
            dirc,
            play,
            hit_attribute: 0,
            transparency: ONE,
            set_transparency: ONE,
            trans_dist: true,
            disp: true,
            drawn: false,
            shadow_t: ONE,
            hit: hit::Body { pos, ..hit::Body::default() },
            affect: AffectColour::default(),
            blend: None,
        })
    }

    /// `ccAnm::SetAnm` of the named animation of the body's file: frame 0,
    /// the same frame speed. False when the file has none by that name.
    pub fn set_anim(&mut self, name: &str) -> bool {
        match self.body.anim(name) {
            Some(anim) => {
                self.play = Play { anim, time: 0, posed: 0, frame_spd: self.play.frame_spd };
                true
            }
            None => false,
        }
    }

    /// The name of the animation playing.
    pub fn anim_name(&self) -> Option<&str> {
        let a = self.body.file.anims.get(self.play.anim)?;
        self.body.file.ccs.object_name(a.object)
    }

    /// `ccAnm::_AnimateForward(frameSpd)`: true when a play-once animation
    /// ends.
    pub fn forward(&mut self) -> bool {
        self.play.forward(&self.body.file)
    }

    /// `ccCoord::SetMatrix_PosRotZYX(pos, dirc)`.
    pub fn root(&self) -> Mat4 {
        root(self.pos, self.dirc)
    }

    /// `ccChar::Draw` up to the draw in a town: the camera fade (`+0x90`
    /// on: hidden only in the eye view) into `transparency`; whether it is
    /// drawn.
    pub fn fade(&mut self, view: &View) -> bool {
        let mut hide = !self.trans_dist || view.eye;
        let t = camera_transparency(
            self.pos,
            view.player,
            view.cam,
            view.deg1,
            self.width,
            self.height,
            FADE_FAR,
            FADE_LEN,
            &mut hide,
        );
        self.transparency = ee::mul(self.set_transparency, t);
        self.shadow_t = if hide { self.set_transparency } else { self.transparency };
        self.drawn = self.disp && !(ee::lt(self.transparency, MIN_DRAWN) && !hide);
        self.blend = self.affect.blend();
        self.drawn
    }

    /// `ccAnm::Draw` of the body as the last [`Char::fade`] left it, on the
    /// character layer, lit by `lights`.
    pub fn draw(&self, layers: &mut Layers, to_screen: Mat4, lights: &TownLights) {
        if !self.drawn {
            return;
        }
        crate::draw::char_shadow(layers, self.shadow_t, self.height, |layers| {
            self.body.draw_char_fog(
                layers,
                to_screen,
                &self.play,
                self.pos,
                self.dirc,
                ee::f(self.transparency),
                lights,
                self.hit_attribute & SHADED != 0,
                &self.clut_swaps,
                self.blend.into(),
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w2p_keeps_z_and_wraps() {
        let p = w2p([0x4000_0000, 0, 0x4416_0000, 0], [0x3f80_0000, 0, 0, ONE]);
        assert_eq!(p, [ONE, 0, 0x4416_0000, ONE]);
        let far = w2p([0x46c3_5000, 0, 0, ONE], [0xc6c3_5000, 0, 0, ONE]); // 25000 - -25000
        assert_eq!(ee::f(far[0]), 2000.0);
    }
}
