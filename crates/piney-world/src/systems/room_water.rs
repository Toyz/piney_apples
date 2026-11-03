//! A dungeon room's water (`DUNGEON::SetWater` gcmn 0x005c3f30, `DrawWater`
//! 0x005ce1c0): `water[0..3]` (+0x364), three anms of the type's
//! `ANM_sdNaf0_a` at the room's first `OBJ_0paf0_` (`SetFogSw(0)`, each one
//! step on), its matrix moved by the room's centre (not turned), no hits.
//! Each frame the three step, `ccAnm::SetUV` writes `vftoi12(u)` into water
//! 1's U and water 2's V, the scroll steps 0.005 (back to 0 at 1), water 0's
//! `OBJ_f_8s40_` samples the picture behind it (`waterUVModifi`); waters 1
//! and 2 on objLayer, water 0 on refLayer, none fogged. `u$8862` (gp
//! 0x00378d88) is the executable's: it carries from dungeon to dungeon.

use std::borrow::Cow;
use std::rc::Rc;

use piney_data::Result;

use super::{
    BuildCx, Carried, CarriedMut, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, RoomCx, RoomPart, StepCx,
};
use crate::chara::pose::Play;
use crate::draw::{self, Fogging, OBJ_LAYER};
use crate::dungeon::Rooms;
use crate::ee::{self, F, ONE, V4};
use crate::field::area::REF_LAYER;
use crate::town::{WaterZero, scroll_statics, water_rows};

/// `SetWater`'s dummy (an exact name).
const DUMMY: &str = "OBJ_0paf0_";
/// The scroll's step, 0.005, and `vftoi12`'s 4096.
const STEP: F = 0x3ba3_d70a;
const K4096: F = 0x4580_0000;

/// The type's water animation (`waterAnmName[type]`).
#[derive(Clone, Copy, Debug)]
pub struct RoomWaterDef {
    pub anim: &'static str,
}

/// The water standing in the room built: the three anms, their root, water
/// 0's model and the texture coordinates `waterUVModifi` last wrote (None:
/// the file has no `OBJ_f_8s40_`), and what the last `DrawWater` passed
/// `ccAnm::SetUV`.
#[derive(Clone, Debug)]
pub struct Water {
    pub name: &'static str,
    pub plays: [Play; 3],
    pub root: [V4; 4],
    pub water0: Option<WaterZero>,
    pub uv: u16,
}

/// The room's water (none in a room without the dummy) and the scroll.
#[derive(Clone)]
pub struct RoomWater {
    def: &'static RoomWaterDef,
    rooms: Rc<Rooms>,
    pub water: Option<Water>,
    pub u: F,
}

impl RoomWater {
    pub fn new(def: &'static RoomWaterDef, cx: &mut BuildCx) -> Result<RoomWater> {
        let rooms = cx.rooms.expect("a dungeon's water").clone();
        Ok(RoomWater { def, rooms, water: None, u: 0 })
    }

    /// `waterUVModifi` of water 0's model through the picture.
    fn sample(&mut self, cx: &StepCx) {
        let Some(w) = &mut self.water else { return };
        let Some(w0) = &mut w.water0 else { return };
        let file = &*self.rooms.file;
        let worlds = w.plays[0].worlds(file, draw::mat(&w.root), &[]);
        let lw = worlds.iter().find(|(o, _)| {
            let target = file.scene.ext.get(o).copied().unwrap_or(**o);
            file.obj_model.get(&target) == Some(&w0.model.object)
        });
        if let Some((_, lw)) = lw {
            let lw: [V4; 4] = lw.to_cols_array_2d().map(|c| c.map(f32::to_bits));
            w0.modify(cx.core.hits.volume, &lw, lw[3], lw[3], &cx.view.world_screen);
        }
    }
}

impl MapSystem for RoomWater {
    /// `Draw`: the three stepped, the scroll written and stepped, water 0
    /// sampled; `Again`: the sampling only. Waters 1, 2, then 0.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        match hook {
            Hook::Draw => {
                let file = self.rooms.file.clone();
                if let Some(w) = &mut self.water {
                    w.plays.iter_mut().for_each(|p| {
                        p.forward(&file);
                    });
                    w.uv = (ee::to_int(ee::mul(self.u, K4096)) & 0xffff) as u16;
                    self.u = ee::add(self.u, STEP);
                    if !ee::lt(self.u, ONE) {
                        self.u = 0;
                    }
                }
            }
            Hook::Again => {}
            _ => return,
        }
        self.sample(cx);
        if self.water.is_some() {
            [1, 2, 0].into_iter().for_each(|k| cx.put(Kind::RoomWater, Piece::Anim(k)));
        }
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Anim(k) = piece else { return None };
        let w = self.water.as_ref()?;
        let file = &*self.rooms.file;
        let play = &w.plays[k];
        let rows = if k == 0 { play.uv_rows(file) } else { water_rows(file, k, w.uv) };
        let edit = (k == 0).then_some(()).and(w.water0.as_ref()).filter(|w0| w0.st.is_some());
        let pieces = play
            .worlds(file, draw::mat(&w.root), &[])
            .into_iter()
            .filter_map(|(obj, world)| {
                let target = file.scene.ext.get(&obj).copied().unwrap_or(obj);
                file.obj_model.get(&target).map(|&m| (m, world))
            })
            .collect();
        Some(Request::Pieces {
            file,
            pieces,
            layer: if k == 0 { REF_LAYER } else { OBJ_LAYER },
            rows: Cow::Owned(rows),
            fog: Fogging::None,
            clut_swaps: &self.rooms.clut_swaps,
            edit: edit.map(|w0| (w0.model.object, w0.edits())),
        })
    }

    /// `SetWater`'s water at the room's first dummy; cleared with the
    /// room.
    fn room(&mut self, part: RoomPart, cx: &mut RoomCx, _: &mut Others) {
        match part {
            RoomPart::Clear => self.water = None,
            RoomPart::Water => {
                self.water = None;
                let (Some(room), Some(slot)) = (cx.room, cx.slot) else { return };
                let Some(d) = self.rooms.subst(room, DUMMY).into_iter().next() else { return };
                let file = &self.rooms.file;
                let Some(mut play) = Play::new(file, self.def.anim) else { return };
                play.forward(file);
                let root = crate::dungeon::placed(&d.local, slot, false);
                let water0 =
                    self.rooms.water_model.clone().map(|model| WaterZero { model, st: None, reach: 0x7f7f_ffff });
                self.water =
                    Some(Water { name: self.def.anim, plays: [play.clone(), play.clone(), play], root, water0, uv: 0 });
            }
            _ => {}
        }
    }

    fn take(&mut self, c: &Carried) {
        self.u = c.statics[scroll_statics::DUNGEON_WATER];
    }

    fn put(&self, c: &mut CarriedMut) {
        c.statics[scroll_statics::DUNGEON_WATER] = self.u;
    }
}
