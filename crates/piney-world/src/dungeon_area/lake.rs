//! The lakes (`DUNGEON` types 8 and 9): their sky and their fireflies
//! (`docs/engine/dungeon.md`, "The lakes' sky and fireflies").
//!
//! ```text
//! DUNGEON()     (0x005b8e84) type 8: by GetBG (WORLD_MAN.bgnum, 0-3) the
//!               clumps CMP_o_bac_l{n}_, bac_m, clo_l, clo_m of the
//!               dungeon's file; type 9: bac_l, bac_m, ero_l, ero_m,
//!               ero_s; each SetFogSw(0)
//! SetRoom       (0x005c3830) the lakes, by night (GetTime 2) or hacked
//!               (DUNGEON +0x40, WORLD_MAN.hackFlag, 3): five FIREFLYs
//!               up to 1000 on from the room's centre in x and y (the
//!               hacked kind when hacked), their sprites field_eff's
//! DrawBG(here)  (0x005ce3d0) the scroll's V into MAT_sfp7bac{n + 1}
//!               (type 8) or MAT_sfp9dat{n + 1}_3 (type 9), then the
//!               scroll on by 0.003 (back by 1 past 1); each clump at
//!               room[level][here]'s centre, scaled 0.5, 1 or 1.5 by the
//!               room's size, unturned, on the layers of priority -100,
//!               -90, -80, -70 and -60
//! DrawEff       (0x005cdee0) after the sparks and glows, by night or
//!               hacked: each firefly's Move and Draw
//! ```

use std::collections::HashMap;

use glam::{Mat4, Vec3};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use super::{DungeonArea, Slot};
use crate::draw::{self, Draw};
use crate::ee::{self, F, ONE, V4};
use crate::field_ambient::{Env, Op, get_time};
use crate::field_firefly::FieldFirefly;

/// `WORLD_MAN` +0x494 .. +0x4a4: the layers `DrawBG` draws on.
const LAYERS: [i16; 5] = [-100, -90, -80, -70, -60];
/// The scroll's step, 0.003.
const STEP: F = 0x3b44_9ba6;
/// `DUNGEON::SetRoom`'s fireflies.
const FIREFLIES: usize = 5;

/// A lake's sky and fireflies.
#[derive(Clone, Debug, Default)]
pub struct Lake {
    /// Each clump's nodes, with its layer, in `DrawBG`'s order.
    pub clumps: Vec<(i16, Vec<u32>)>,
    /// The scrolled material and its Material chunk's cropV.
    pub material: Option<(u32, u16)>,
    /// `DrawBG`'s static `v`.
    pub v: F,
    /// By night or hacked (`GetTime() == 2 || DUNGEON +0x40 == 3`).
    pub night: bool,
    pub hacked: bool,
    /// `WORLD_MAN`'s field type, which `FIREFLY::Move` reads.
    pub field_type: u32,
    pub fireflies: Vec<FieldFirefly>,
}

impl Lake {
    /// The constructor's clumps for type `dtype` (8 or 9) under `bg`
    /// (`WORLD_MAN::GetBG`); none past 3, which the constructor's switch
    /// leaves out.
    pub fn new(file: &SceneFile, dtype: u8, bg: u32, field_type: u32, hack: u32) -> Lake {
        let names: &[&str] = match dtype {
            8 => &["bac_l", "bac_m", "clo_l", "clo_m"],
            _ => &["bac_l", "bac_m", "ero_l", "ero_m", "ero_s"],
        };
        let mut lake = Lake {
            night: get_time(field_type, bg) == 2 || hack == 3,
            hacked: hack == 3,
            field_type,
            ..Lake::default()
        };
        if bg > 3 {
            return lake;
        }
        for (k, n) in names.iter().enumerate() {
            let name = format!("CMP_o_{n}{bg}_");
            let nodes = file
                .ccs
                .find_object(&name)
                .and_then(|c| file.scene.clumps.iter().find(|(o, _)| *o == c))
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            lake.clumps.push((LAYERS[k], nodes));
        }
        let mat = if dtype == 8 { format!("MAT_sfp7bac{}", bg + 1) } else { format!("MAT_sfp9dat{}_3", bg + 1) };
        lake.material = file.ccs.find_object(&mat).map(|o| (o, file.scene.materials.get(&o).map_or(0, |m| m.crop_v)));
        lake
    }
}

impl DungeonArea {
    /// `SetRoom`'s end for a lake: by night or hacked, five new fireflies
    /// by the room's centre.
    pub(super) fn set_lake_fireflies(&mut self, slot: &Slot) {
        let Some(lake) = self.lake.as_mut() else { return };
        if !lake.night {
            return;
        }
        let hacked = lake.hacked;
        lake.fireflies = (0..FIREFLIES).map(|_| FieldFirefly::lake(hacked, slot.pos, &mut self.rng)).collect();
    }

    /// `DrawBG`'s scroll: the material and the halfword it writes to its
    /// substitute's V (+0x16), `int(v x 4096)` (VU0's `vftoi12`) less the
    /// Material chunk's; with `step`, the scroll on by 0.003, back by 1
    /// past 1.
    pub fn bg_scroll(&mut self, step: bool) -> Option<(u32, u16)> {
        let lake = self.lake.as_mut()?;
        let (mat, crop_v) = lake.material?;
        let v = (ee::to_int(ee::mul(lake.v, 0x4580_0000)) as i16).wrapping_sub(crop_v as i16) as u16;
        if step {
            lake.v = ee::add(lake.v, STEP);
            if !ee::le(lake.v, ONE) {
                lake.v = ee::sub(lake.v, ONE);
            }
        }
        Some((mat, v))
    }

    /// `DrawBG`'s matrix for room `here` of the floor:
    /// `SetMatrix_PosRotZYXScale` at the room's centre (z 0), unturned (the
    /// rotations it passes are all zero), scaled 0.5, 1 or 1.5 by the
    /// room's size.
    pub fn bg_world(&self, here: Option<usize>) -> Option<Mat4> {
        self.lake.as_ref()?;
        let slot = here.and_then(|h| self.slot(self.level, h))?;
        let s = match slot.size {
            0 => 0.5,
            2 => 1.5,
            _ => 1.0,
        };
        Some(
            Mat4::from_translation(Vec3::new(ee::f(slot.pos[0]), ee::f(slot.pos[1]), 0.0))
                * Mat4::from_scale(Vec3::splat(s)),
        )
    }

    /// `DrawBG(here)`: the sky's clumps about room `here`'s centre, the
    /// material's V from the scroll (and, with `step`, the scroll on).
    pub(super) fn draw_bg(&mut self, layers: &mut Layers, to_screen: Mat4, here: Option<usize>, step: bool) {
        let mut rows = HashMap::new();
        if let Some((mat, v)) = self.bg_scroll(step) {
            // The texture's rows: V in 1/4096 of 256.
            rows.insert(mat, [0u8, (v >> 4) as u8]);
        }
        let Some(world) = self.bg_world(here) else { return };
        let clumps = self.lake.as_ref().map(|l| l.clumps.clone()).unwrap_or_default();
        for (layer, nodes) in clumps {
            let mut models: Vec<u32> = nodes.iter().filter_map(|n| self.obj_models.get(n)).flatten().copied().collect();
            models.dedup();
            for model in models {
                if self.file.models.get(&model).is_none_or(|i| i.mtype & 0x600 == 0x600) {
                    continue;
                }
                let d = Draw {
                    file: &self.file,
                    model,
                    world,
                    alpha: 1.0,
                    rows: &rows,
                    lights: None,
                    nodes: &[],
                    morph: Vec::new(),
                    clut_swaps: Vec::new(),
                };
                draw::model_edited(layers, layer, to_screen, d, None, draw::Fogging::None);
            }
        }
    }

    /// `DrawEff`'s fireflies: by night or hacked, each one's `Move` (with
    /// `step`) and `Draw`, about the player, their sprites `field_eff`'s
    /// (`DUNGEON` +0x34c).
    pub fn draw_fireflies(&mut self, player: V4, step: bool, out: &mut Vec<Op>) {
        let Some(lake) = self.lake.as_mut() else { return };
        if !lake.night {
            return;
        }
        let flat = |_: F, _: F| 0;
        let env = Env {
            player,
            centre: player,
            ofs: [player[0], player[1]],
            height: &flat,
            eye: player,
            eye1: player,
            rot: [0; 4],
            rot2: [0; 4],
            eye_view: false,
            odd: false,
            bounds: super::BOUNDS,
        };
        // FIREFLY::Move's sparks: EFF_sfzfir1 in field type 9.
        let spark = if lake.field_type == 9 { "EFF_sfzfir1" } else { "EFF_sfpfir_1" };
        for f in &mut lake.fireflies {
            if step {
                f.step(&env, lake.field_type, &mut self.rng);
            }
            f.draw(&env, spark, &mut self.rng, out);
        }
    }
}
