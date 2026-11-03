//! What a field's weather and ambient systems share: `WORLD_MAN`'s
//! weather and time ([`Weather`]), the frame they read ([`Env`]), the
//! sprites they make ([`Eff`]) and the drawing's statics as the executable
//! starts them ([`BOOT_STATICS`]). Each picture is its own system under
//! [`crate::systems`] (snow, smoke, steam, haze, lake, rain, thunder, fires,
//! fireflies, TOBJ, BIRD), made by `WORLD::Init` (gcmn 0x005a4cd0) and
//! `WORLD::Generate` (0x005a6da0) and drawn by `WORLD::Draw` (0x005a97b0) and
//! `DrawEffect` (0x005a9070) in the order the field type's def gives
//! ([`crate::field::def::FieldDef`]; docs/engine/field.md).

use piney_data::dungeon::Rng;

use crate::ee::{self, F, ONE, V4};
use crate::field::area::{p2w, w2p};
use crate::sprite::{EffSprite, EffState};
use crate::systems::{FieldStatics, StepCx};

/// `GetWeather()` of `WORLD_MAN` (main 0x0019f190): the weather of field
/// type `ty` with background row `b`, as the type's def gives it
/// ([`crate::field::def::Skies`]).
pub fn get_weather(ty: u32, b: u32) -> u32 {
    crate::field::def::FieldDef::skies_of(ty).weather(b)
}

/// `GetTime()` (main 0x0019ef50): 0 day, 1 evening, 2 night.
pub fn get_time(ty: u32, b: u32) -> u32 {
    crate::field::def::FieldDef::skies_of(ty).time(b)
}

/// The field's kind, weather and hack state as the drawing reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weather {
    /// `WORLD.fieldType` (+0x10) and the background row (+0x14).
    pub field_type: u32,
    pub b: u32,
    /// `isHacked` (+0x2c, `WORLD_MAN` +0xf0): 3 when hacked.
    pub hack: i32,
}

impl Weather {
    /// The field's own CCS file (`WORLD` +0x6148): `fieldccs[type]`, a
    /// hacked field's `fieldccs2[type]`.
    pub fn file(&self, volume: piney_data::volume::Volume) -> &'static str {
        piney_data::field::tables_of(volume).field_ccs(self.field_type, self.hack).unwrap_or(EFF_FILE)
    }

    pub fn weather(&self) -> u32 {
        get_weather(self.field_type, self.b)
    }

    pub fn time(&self) -> u32 {
        get_time(self.field_type, self.b)
    }

    pub fn hacked(&self) -> bool {
        self.hack == 3
    }

    /// Rain or a storm (weather 2 or 3).
    pub fn rains(&self) -> bool {
        matches!(self.weather(), 2 | 3)
    }

    /// `WORLD_MAN::CheckLensFlare` (main 0x0019f0d0): not hacked, fine,
    /// cloudy or light snow, day or evening.
    pub fn lens_flare(&self) -> bool {
        !self.hacked() && matches!(self.weather(), 0 | 1 | 4) && matches!(self.time(), 0 | 1)
    }
}

/// What `WORLD_MAN::GetHeight` answers: before `Generate` makes the height
/// map (`Init`'s, over a blank field), a generated field's, or a dungeon's
/// (0 everywhere).
#[derive(Clone, Copy)]
pub enum Ground<'a> {
    Blank,
    Field(&'a piney_data::field::Field),
    Heights(&'a dyn crate::hit::Heights),
    Flat,
}

impl Ground<'_> {
    /// The height at (x, y).
    pub fn height(&self, x: F, y: F) -> F {
        match *self {
            Ground::Blank => piney_data::field::blank_height(x, y),
            Ground::Field(f) => f.get_height(x, y),
            Ground::Heights(h) => h.height(x, y),
            Ground::Flat => 0,
        }
    }
}

/// What the frame's drawing reads of the world around it.
#[derive(Clone, Copy)]
pub struct Env<'a> {
    /// The disc's volume: its code's square root (`sqrt.s` from Outbreak
    /// on, [`ee::sqrtf_on`]).
    pub volume: piney_data::volume::Volume,
    /// The player's place (`plw` +0x40), which `ccTransPosW2P` / `P2W`
    /// wrap about.
    pub player: V4,
    /// `WORLD_MAN::GetCenter` (+0x80).
    pub centre: V4,
    /// `WORLD.ofs_x`, `ofs_y` (+0x6140, +0x6144).
    pub ofs: [F; 2],
    pub ground: Ground<'a>,
    /// `cameraGetPos(camID)` and `cameraGetPos(1)`.
    pub eye: V4,
    pub eye1: V4,
    /// `cameraGetRot(1)` and `cameraGetRot2(1)`, and `checkCameraType()`
    /// 1 (the eye view), which picks the second.
    pub rot: V4,
    pub rot2: V4,
    pub eye_view: bool,
    /// `ccSys` +0x358 & 1: the frame counter's low bit.
    pub odd: bool,
    /// `WORLD_MAN` +0x420 .. +0x42c: the area's bounds, which the wrap
    /// goes by (a field's 48000, a dungeon's 60000).
    pub bounds: [F; 4],
}

impl<'a> Env<'a> {
    /// What `WORLD::Init` and `Generate` see: everything at `centre`, a
    /// field's bounds.
    pub fn at(volume: piney_data::volume::Volume, centre: V4, ground: Ground<'a>) -> Env<'a> {
        Env {
            volume,
            player: centre,
            centre,
            ofs: [centre[0], centre[1]],
            ground,
            eye: centre,
            eye1: centre,
            rot: [0; 4],
            rot2: [0; 4],
            eye_view: false,
            odd: false,
            bounds: crate::field::area::BOUNDS,
        }
    }

    /// A frame's: the view, and of the area its centre (`WORLD.ofs`), its
    /// heights and its bounds.
    pub fn of(cx: &'a StepCx) -> Env<'a> {
        let (hits, v) = (&cx.core.hits, cx.view);
        let [x, y] = hits.center;
        Env {
            volume: hits.volume,
            player: v.player,
            centre: [x, y, 0, ONE],
            ofs: [x, y],
            ground: hits.heights.as_deref().map_or(Ground::Flat, Ground::Heights),
            eye: v.eye,
            eye1: v.eye1,
            rot: v.rot,
            rot2: v.rot2,
            eye_view: v.eye_view(),
            odd: v.odd,
            bounds: hits.bounds.unwrap_or(crate::field::area::BOUNDS),
        }
    }
}

impl Env<'_> {
    pub fn w2p(&self, p: V4) -> V4 {
        w2p(p, self.player, self.bounds)
    }

    pub fn p2w(&self, d: V4) -> V4 {
        p2w(d, self.player, self.bounds)
    }

    pub fn height(&self, x: F, y: F) -> F {
        self.ground.height(x, y)
    }

    /// The camera's turn as the eye view picks it.
    pub fn cam_rot(&self) -> V4 {
        if self.eye_view { self.rot2 } else { self.rot }
    }

    /// What a `FIREFLY2` reads of it.
    pub fn fly(&self) -> crate::sky::firefly::Fly<'_> {
        crate::sky::firefly::Fly {
            volume: self.volume,
            player: self.player,
            bounds: self.bounds,
            eye: self.eye,
            rot: self.cam_rot(),
            ground: self.ground,
        }
    }
}

/// The WORLD_MAN layer a picture goes on (`SetActiveLayer(k)`): 3 the
/// effects, 5 TOBJ and BIRD, 6 the heat haze.
pub type Layer = u8;

/// `field_eff`, `WORLD`'s effects file (+0x6154).
pub const EFF_FILE: &str = "field_eff";

/// A float of a `fieldrand` result, which the game converts as unsigned
/// (the results are below 2^31, so as signed).
pub fn fr(rng: &mut Rng, n: u32) -> F {
    ee::from_int(rng.below(n) as i32)
}

/// An unsigned int as a float, as the EE converts one (`cvt.s.w` of the
/// value halved, rounding bit kept, doubled when the top bit is set).
pub fn uint_float(u: u32) -> F {
    if (u as i32) >= 0 {
        ee::from_int(u as i32)
    } else {
        let h = (u >> 1) | (u & 1);
        let f = ee::from_int(h as i32);
        ee::add(f, f)
    }
}

/// `ccEff::Init(chunk, 1)` then `+0x62 &= ~0x20` (the fog bit off), as
/// every ambient sprite is made; scale 1, no turn, transparency 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Eff {
    pub file: &'static str,
    pub name: &'static str,
    pub pos: V4,
    pub scale: [F; 2],
    pub transparency: F,
    pub fog: bool,
}

impl Eff {
    pub fn new(name: &'static str) -> Eff {
        Eff { file: EFF_FILE, name, pos: [0, 0, 0, ONE], scale: [ONE; 2], transparency: ONE, fog: false }
    }

    /// With the fog bit as `Init(chunk, 1)` leaves it.
    pub fn fogged(name: &'static str) -> Eff {
        Eff { fog: true, ..Eff::new(name) }
    }

    pub fn scaled(mut self, s: F) -> Eff {
        self.scale = [s; 2];
        self
    }

    /// `Draw(pattern)` at its own place.
    pub fn draw(&self, layer: Layer, pattern: u16) -> EffSprite {
        self.draw_at(layer, self.pos, pattern)
    }

    /// `Draw(pos, pattern)`.
    pub fn draw_at(&self, layer: Layer, pos: V4, pattern: u16) -> EffSprite {
        let layer = crate::field::area::ambient_layer(layer);
        EffSprite {
            scale: self.scale,
            transparency: self.transparency,
            ..EffSprite::new(self.file, self.name, pos, pattern, layer, EffState::fogged(self.fog))
        }
    }
}

/// `SetMatrix_PosRotXYZScale(pos, rot, scale)` (main 0x00138050): the
/// scale, turned about x, then y, then z, then moved to `pos`.
pub fn pos_rot_xyz_scale(pos: V4, rot: V4, scale: [F; 3]) -> [V4; 4] {
    let mut m = [[scale[0], 0, 0, 0], [0, scale[1], 0, 0], [0, 0, scale[2], 0], [0, 0, 0, ONE]];
    m = piney_data::anim::rot_x_bits_of(m, rot[0]);
    m = piney_data::anim::rot_y_bits_of(m, rot[1]);
    m = piney_data::anim::rot_z_bits_of(m, rot[2]);
    for k in 0..3 {
        m[3][k] = ee::add(m[3][k], pos[k]);
    }
    m
}

/// The drawing's function statics as the executable starts them, which
/// live as long as the game does (the first `DrawRain` of the whole game
/// seeds its thunder's count): `DrawRain`'s `tcnt` (unset), `tcnt2` 750,
/// `secnt`, `seflag`; `DrawSteam`'s `life`, `sleep`, `step` 0.5, `num`,
/// `pos`; TOBJ's cloth scrolls and what they last showed ([`statics`]).
pub const BOOT_STATICS: FieldStatics = [0, 0, 750, 0, 0, 0, 0, 0x3f00_0000, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// Where each system's statics sit in [`FieldStatics`].
pub mod statics {
    /// `DrawRain`'s (gcmn 0x00378d24-40): `tcnt` set, `tcnt`, `tcnt2`,
    /// `secnt`, `seflag`.
    pub const RAIN: usize = 0;
    /// `DrawSteam`'s (0x00378d5c-74, 0x00730580): `life`, `sleep`, `step`,
    /// `num`, `pos` (four).
    pub const STEAM: usize = 5;
    /// TOBJ's cloth scrolls `u`, `v` (0x00378da0, 0x00378da8) and the two
    /// it last showed.
    pub const CLOTH: usize = 13;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The boot statics: `tcnt2` 750 and `step` 0.5, the rest 0.
    #[test]
    fn boot_statics() {
        assert_eq!(BOOT_STATICS[statics::RAIN + 2], 750);
        assert_eq!(BOOT_STATICS[statics::STEAM + 2], 0x3f00_0000);
        assert_eq!(BOOT_STATICS.iter().filter(|&&w| w != 0).count(), 2);
    }
}
