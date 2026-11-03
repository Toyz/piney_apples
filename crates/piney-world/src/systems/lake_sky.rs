//! A lake's sky (`DUNGEON` types 8 and 9): the constructor (0x005b8e84,
//! 0x005b9500) makes its clumps by `WORLD_MAN::GetBG` (none past 3);
//! `DrawBG` (0x005ce3d0) writes the material's V from its scroll, steps it
//! 0.003 (past 1, back by 1) and draws each clump about the centre of the
//! room under the player, scaled by the room's size, on the layers -100 ..
//! -60, unfogged. `v$8917` is the executable's: it carries from dungeon to
//! dungeon (docs/engine/dungeon.md, "The lakes' sky and fireflies").

use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use glam::{Mat4, Vec3};
use piney_data::Result;

use super::{BuildCx, Carried, CarriedMut, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::draw::Fogging;
use crate::dungeon::Rooms;
use crate::ee::{self, F, ONE};
use crate::town::scroll_statics;

/// The scroll's step, 0.003.
const STEP: F = 0x3b44_9ba6;

/// The type's clumps (`CMP_o_<name><bg>_`), its scrolled material
/// (`<prefix><bg + 1><suffix>`) and the layers the clumps draw on.
#[derive(Clone, Copy, Debug)]
pub struct LakeSkyDef {
    pub clumps: &'static [&'static str],
    pub material: (&'static str, &'static str),
    pub layers: [i16; 5],
}

/// The clumps' models (each its nodes' models in the file), the scrolled
/// material and its Material chunk's cropV, the scroll, and what the last
/// frame drew: the V it wrote, the room's centre and size.
#[derive(Clone)]
pub struct LakeSky {
    def: &'static LakeSkyDef,
    rooms: Rc<Rooms>,
    pub clumps: Vec<Vec<u32>>,
    pub material: Option<(u32, u16)>,
    pub v: F,
    pub shown: u16,
    pub world: Option<Mat4>,
}

impl LakeSky {
    pub fn new(def: &'static LakeSkyDef, cx: &mut BuildCx) -> Result<LakeSky> {
        let rooms = cx.rooms.expect("a lake's sky").clone();
        let bg = cx.field.expect("a lake's sky").weather.b;
        let file = &rooms.file;
        let (mut clumps, mut material) = (Vec::new(), None);
        if bg <= 3 {
            for n in def.clumps {
                let nodes = rooms.clump_nodes(&format!("CMP_o_{n}{bg}_"));
                let mut models: Vec<u32> =
                    nodes.iter().filter_map(|n| rooms.obj_models.get(n)).flatten().copied().collect();
                models.dedup();
                models.retain(|m| file.models.get(m).is_some_and(|i| i.mtype & 0x600 != 0x600));
                clumps.push(models);
            }
            let (prefix, suffix) = def.material;
            let mat = format!("{prefix}{}{suffix}", bg + 1);
            material = file.ccs.find_object(&mat).map(|o| (o, file.scene.materials.get(&o).map_or(0, |m| m.crop_v)));
        }
        Ok(LakeSky { def, rooms, clumps, material, v: 0, shown: 0, world: None })
    }
}

impl MapSystem for LakeSky {
    /// `Draw`: the V written from the scroll, which steps on; `Again`: the
    /// V of the scroll as it stands. Then each clump at the room under the
    /// player, `SetMatrix_PosRotZYXScale` at its centre (z 0) unturned,
    /// scaled 0.5, 1 or 1.5 by its size (none outside a room).
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if !matches!(hook, Hook::Draw | Hook::Again) {
            return;
        }
        if let Some((_, crop_v)) = self.material {
            self.shown = (ee::to_int(ee::mul(self.v, 0x4580_0000)) as i16).wrapping_sub(crop_v as i16) as u16;
            if hook == Hook::Draw {
                self.v = ee::add(self.v, STEP);
                if !ee::le(self.v, ONE) {
                    self.v = ee::sub(self.v, ONE);
                }
            }
        }
        self.world = cx.view.room.map(|r| {
            let s = match r.size {
                0 => 0.5,
                2 => 1.5,
                _ => 1.0,
            };
            Mat4::from_translation(Vec3::new(ee::f(r.pos[0]), ee::f(r.pos[1]), 0.0)) * Mat4::from_scale(Vec3::splat(s))
        });
        if self.world.is_some() {
            (0..self.clumps.len()).for_each(|k| cx.put(Kind::LakeSky, Piece::Clump(k)));
        }
    }

    /// The texture's rows: V in 1/4096 of 256.
    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Clump(k) = piece else { return None };
        let mut rows = HashMap::new();
        if let Some((mat, _)) = self.material {
            rows.insert(mat, [0u8, (self.shown >> 4) as u8]);
        }
        Some(Request::Models {
            file: &self.rooms.file,
            models: Cow::Borrowed(self.clumps.get(k)?),
            world: self.world?,
            layer: self.def.layers[k],
            rows,
            fog: Fogging::None,
            alpha: 1.0,
        })
    }

    fn take(&mut self, c: &Carried) {
        self.v = c.statics[scroll_statics::DUNGEON_LAKE];
    }

    fn put(&self, c: &mut CarriedMut) {
        c.statics[scroll_statics::DUNGEON_LAKE] = self.v;
    }
}
