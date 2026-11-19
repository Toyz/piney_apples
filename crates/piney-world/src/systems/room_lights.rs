//! `DUNGEON::SetLight` (gcmn 0x005c4560): the room lights (`ROOMLIGHT`,
//! 0x40 bytes, `roomlight[k]` +0x374), each a glow at a dummy with an omni
//! light in the room's light group; `DrawEff` (0x005cdee0) draws each glow
//! at its pattern and flickers the light by the pattern's transparency.

use std::rc::Rc;

use piney_data::Result;
use piney_data::light::{Falloff, Light};

use super::sparks::fieldrand;
use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, RoomCx, RoomPart, StepCx};
use crate::dungeon::Rooms;
use crate::ee::{self, F, ONE, V4};
use crate::sprite::{EffSprite, EffState};

/// `roomlight[32]`: `SetLight` checks the count before each dummy, and
/// `DrawEff` walks the 32.
const ROOM_LIGHTS: usize = 32;
/// The searches' dummies, in order.
const LIGHT_M0: &str = "OBJ_o_light_m0*";
const LIGHT_S0: &str = "OBJ_o_light_s0*";
const ALTAR: &str = "OBJ_o_altar_l0*";
/// Every omni light's fall-off (`farDownStart` 100, `farDownEnd` 250 and
/// its square 62500, stored as constants), and a pattern's 4096.
const FAR: Falloff = Falloff { start: 0x42c8_0000, end: 0x437a_0000, end2: 0x4774_2400 };
const K4096: F = 0x4580_0000;

/// One `ROOMLIGHT` as its case makes it: the glow at `at` (a vector the
/// dummy's world matrix turns, then its place added), the light's colour
/// (`ccSetColor` of the packed bytes), and the second glow at the same
/// place.
#[derive(Clone, Copy, Debug)]
pub struct Glow {
    pub eff: &'static str,
    pub at: [F; 3],
    pub colour: u32,
    pub eff2: Option<&'static str>,
}

pub const fn glow(eff: &'static str, at: [F; 3], colour: u32, eff2: Option<&'static str>) -> Glow {
    Glow { eff, at, colour, eff2 }
}

/// The type's glows for each dummy of the three searches.
#[derive(Clone, Copy, Debug)]
pub struct RoomLightsDef {
    pub m0: &'static [Glow],
    pub s0: &'static [Glow],
    pub altar: &'static [Glow],
}

/// One room light: its glow (`Init(chunk, 1)`, the fog bit cleared) at its
/// place and pattern, the second glow (fogged) and its pattern, its omni
/// light's place in the group, and the patterns the last `DrawEff` drew.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomLight {
    pub eff: &'static str,
    pub pos: V4,
    pub pat: u16,
    pub eff2: Option<(&'static str, V4)>,
    pub pat2: u16,
    pub light: usize,
    pub shown: (u16, u16),
}

/// The room's lights.
#[derive(Clone)]
pub struct RoomLights {
    def: &'static RoomLightsDef,
    rooms: Rc<Rooms>,
    pub lights: Vec<RoomLight>,
}

impl RoomLights {
    pub fn new(def: &'static RoomLightsDef, cx: &mut BuildCx) -> Result<RoomLights> {
        let rooms = cx.rooms.expect("a dungeon's room lights").clone();
        Ok(RoomLights { def, rooms, lights: Vec::new() })
    }

    /// One `ROOMLIGHT` at a dummy whose world matrix is `lw`: the glow at
    /// `lw` applied to the case's vector (w 0) plus its place, its pattern
    /// `fieldrand(patNum)`; the omni light (type 4, priority 1, the case's
    /// colour, falling off from 100 to 250) at the dummy's place, into the
    /// group (`AddGrp`); the second glow at the same place, its pattern
    /// drawn after.
    fn room_light(&mut self, g: &Glow, lw: &[V4; 4], cx: &mut RoomCx) {
        let t = lw[3];
        let pos = ee::vadd(ee::apply(lw, [g.at[0], g.at[1], g.at[2], 0]), t);
        let pat = fieldrand(cx.rng, self.rooms.pat_num(g.eff)) as u16;
        // `ccLight(4, 1)`, `ccSetColor(colour, 1)`, the fall-off and its
        // square as constants, `UnitMatrix` then `TransMatrix` to the place.
        let colour = piney_data::light::colour_of(g.colour, ONE);
        let place = piney_data::vu0::trans_matrix(&piney_data::vu0::UNIT, t)[3];
        cx.lights.lights.push(Light { priority: 1, ..Light::omni(place, colour, ONE, FAR) });
        let light = cx.lights.lights.len() - 1;
        let (eff2, pat2) = match g.eff2 {
            Some(e) => (Some((e, pos)), fieldrand(cx.rng, self.rooms.pat_num(e)) as u16),
            None => (None, 0),
        };
        self.lights.push(RoomLight { eff: g.eff, pos, pat, eff2, pat2, light, shown: (pat, pat2) });
    }
}

impl MapSystem for RoomLights {
    /// `Draw` (`DrawEff`): each light's glows at the patterns it shows,
    /// the light's intensity the glow's transparency times its pattern's
    /// over 4096, both patterns on by one (the second's first), each
    /// wrapping at its `patNum`. `Again`: the glows as they stand.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        let step = match hook {
            Hook::Draw => true,
            Hook::Again => false,
            _ => return,
        };
        let pats = |name: &str| self.rooms.file.ccs.find_object(name).and_then(|o| self.rooms.eff_pats.get(&o));
        for (k, rl) in self.lights.iter_mut().take(ROOM_LIGHTS).enumerate() {
            cx.put(Kind::RoomLights, Piece::Item(2 * k));
            if rl.eff2.is_some() {
                cx.put(Kind::RoomLights, Piece::Item(2 * k + 1));
            }
            if !step {
                continue;
            }
            rl.shown = (rl.pat, rl.pat2);
            if let Some((e2, _)) = rl.eff2 {
                let n = pats(e2).map_or(0, Vec::len);
                rl.pat2 += 1;
                if usize::from(rl.pat2) >= n {
                    rl.pat2 = 0;
                }
            }
            let p = pats(rl.eff);
            let t = p.and_then(|p| p.get(usize::from(rl.pat))).copied().unwrap_or(4096);
            let tr = ee::mul(ONE, ee::div(ee::from_int(i32::from(t)), K4096));
            if let Some(l) = cx.core.lights.lights.get_mut(rl.light) {
                l.intensity = tr;
            }
            rl.pat += 1;
            if usize::from(rl.pat) >= p.map_or(0, Vec::len) {
                rl.pat = 0;
            }
        }
    }

    /// Item `2k` light k's glow, `2k + 1` its second.
    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Item(n) = piece else { return None };
        let rl = self.lights.get(n / 2)?;
        let (eff, pos, pattern, state) = match n % 2 {
            0 => (rl.eff, rl.pos, rl.shown.0, EffState::NO_FOG),
            _ => {
                let (e2, pos2) = rl.eff2?;
                (e2, pos2, rl.shown.1, EffState::INIT)
            }
        };
        let layer = crate::draw::EFF_LAYER;
        Some(Request::Sprite(EffSprite::new(self.rooms.name, eff, pos, pattern, layer, state)))
    }

    /// `SetLight(f, i)`: for each dummy of the three searches, while fewer
    /// than 32 are made, its case's lights. A case of several fills
    /// `roomlight[n..]` and adds its count after (so a room could run past
    /// the 32 into `door[]`; none of the files' rooms does). Gone with the
    /// room (`DelGrp` takes their omni lights).
    fn room(&mut self, part: RoomPart, cx: &mut RoomCx, _: &mut Others) {
        match part {
            RoomPart::Clear => self.lights.clear(),
            RoomPart::Light => {
                let Some(room) = cx.room else { return };
                let mut n = 0usize;
                let def = self.def;
                for (pattern, glows) in [(LIGHT_M0, def.m0), (LIGHT_S0, def.s0), (ALTAR, def.altar)] {
                    for d in self.rooms.subst(room, pattern) {
                        if n >= ROOM_LIGHTS {
                            return;
                        }
                        for g in glows {
                            self.room_light(g, &d.world, cx);
                        }
                        n += glows.len();
                    }
                }
            }
            _ => {}
        }
    }
}
