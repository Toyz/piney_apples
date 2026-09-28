//! The boss arenas: `EVENTAREAB0` (gcmn areab0.cpp), the story map
//! `WORLD_MAN::GO(1)` (main 0x0019f8e0) makes for fields 1-8. Each field is
//! a stage of its own scene file; Infection's Skeith is fought on field 1,
//! `se1_5`, which area 27's last door leads to (`docs/engine/boss.md`).
//!
//! ```text
//! EVENTAREAB0()          0x004079f0  sw = (0, 1, 0), layerSw 0; by
//!                                    game.field the stage's file, and
//!                                    EA_MODELTABLEB0's and eventareaB0Light's
//!                                    names patched to it; SetFog(1500, 12000,
//!                                    0, 70, 0x1e1e1e); ccSys.bgColor 0x1e1e1e;
//!                                    the 9 STATICMODELs; lgtAnm (SetLightEnv
//!                                    1) the stage's ANM_*bac1a, bg[0..2] its
//!                                    CMP_*bac1, clo1, clo2 (fog off); the
//!                                    lights (the distant one first) and the
//!                                    ambient at frame 1; BLT_bg, obj, floor;
//!                                    SetStartPos(DMY_center01); 54 FIREFLY2s
//!                                    at DMY_marker01..18 (EFF_*fir1,
//!                                    [`crate::firefly`])
//! Draw()                 0x004095e0  DrawBG; objLayer: DrawObj; floorLayer:
//!                                    DrawFloor; refLayer (objLayer with
//!                                    layerSw): modelArray[5]; effLayer: the
//!                                    fireflies (each Draw, then Move);
//!                                    modelArray[6..8] bob
//! DrawBG()               0x004091c0  MAT_*clo1's v scrolls 0.0015 a frame
//!                                    (a static, wrapping past 1); bg[k] on
//!                                    bgLayer[k] at the identity
//! SwitchLayer()          0x004095b0  layerSw ^= 1 (Skeith's magic)
//! NextStage()            0x004098e0  field 8 only: the stage deleted, the
//!                                    last stage's anm drawn instead
//! ```
//!
//! The collision is the floor model's Hit chunk (`HIT_se1_5fl0hit`), as for
//! `EVENTAREA02` ([`crate::evarea`]): `ccLandHitCheck` never falls back on a
//! height map in a story map, and the bounds are `GO`'s.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::statics::{self, DrawPass, Position};
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::draw::{self, Draw};
use crate::ee::{self, F, ONE, V4};
use crate::evarea::{BOUNDS, StaticModel, StorySprite, layer};
use crate::firefly::{Firefly, FireflyDraw};
use crate::hit::{HitModel, Hits};
use crate::town::{FogParams, Light, TownLights};

/// Each field's stage (`game.field` 1-8; the constructor's jump table at
/// 0x006a3d60). Field 8 also loads `se4_8`, `NextStage`'s last stage.
pub const STAGES: [&str; 9] = ["", "se1_5", "se2_3", "se2_4", "se3_2", "se3_4", "se4_3", "se4_5", "se4_7"];

/// `bossFireFlyEff` (gcmn 0x005d26e0): each field's firefly sprite, in
/// its stage's file.
pub const FIREFLY_EFF: [&str; 9] = [
    "",
    "EFF_se1_5fir1",
    "EFF_se2_3fir1",
    "EFF_se2_4fir1",
    "EFF_se3_2fir1",
    "EFF_se3_4fir1",
    "EFF_se4_3fir1",
    "EFF_se4_5fir1",
    "EFF_se4_7fir1",
];

/// The dummies the fireflies start at (`@1153`), the k-th firefly at
/// `k % 18`.
const MARKERS: [&str; 18] = [
    "DMY_marker01",
    "DMY_marker02",
    "DMY_marker03",
    "DMY_marker04",
    "DMY_marker05",
    "DMY_marker06",
    "DMY_marker07",
    "DMY_marker08",
    "DMY_marker09",
    "DMY_marker10",
    "DMY_marker11",
    "DMY_marker12",
    "DMY_marker13",
    "DMY_marker14",
    "DMY_marker15",
    "DMY_marker16",
    "DMY_marker17",
    "DMY_marker18",
];

/// How many fireflies the constructor makes.
pub const FIREFLIES: usize = 54;

/// The fields `EVENTAREAB0` is: `GO(1)`'s `eventAreaNumber` 1-8.
pub fn is_arena(field: i32) -> bool {
    (1..=8).contains(&field)
}

/// `refLayer`, where `modelArray[5]` draws unless `layerSw` is set.
pub const REF_LAYER: i16 = 30;
/// `bgLayer[1]`.
pub const BG1: i16 = -90;

/// The constructor's `ccSys.bgColor`.
pub const BG_COLOR: [u8; 3] = [0x1e, 0x1e, 0x1e];

/// `SetFog(1500, 12000, 0, 70, 0x1e1e1e)`.
pub const FOG: FogParams = FogParams { near: 1500.0, far: 12000.0, near_rate: 0.0, far_rate: 70.0, colour: [0x1e; 3] };

/// `DrawBG`'s scroll: 0.0015 (0x3ac49ba6) a frame.
const SCROLL: F = 0x3ac4_9ba6;
/// The bobbing models' turn, at +-50.
const BOB: F = 0x4248_0000;
const NEG_BOB: F = 0xc248_0000;

/// One piece `EVENTAREAB0::Draw` draws, in its order, on its layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// `DrawBG`: `bg[k]` at the identity.
    Bg { k: usize, layer: i16 },
    /// `STATICMODEL::Draw` of `modelArray[k]`.
    Model { k: usize, layer: i16 },
    /// A firefly's (or spark's) `ccEff::Draw`.
    Firefly { draw: FireflyDraw, layer: i16 },
}

/// `EVENTAREAB0` as `WORLD_MAN.eventmap` holds it.
pub struct Arena {
    /// `game.field` it was made for, and the stage's scene file (+0x1a4).
    pub field: i32,
    pub stage: &'static str,
    pub file: Rc<SceneFile>,
    /// `modelArray` (+0x78): EA_MODELTABLEB0's nine rows, the stage's.
    pub models: Vec<StaticModel>,
    /// `bg[0..2]` (+0x1a8): each clump's models.
    pub bg: [Option<(u32, Vec<u32>)>; 3],
    /// The light group from `lgtAnm` at frame 1, and the ambient.
    pub lights: TownLights,
    pub light_objects: Vec<u32>,
    /// `WORLD_MAN.eventStartPos` from `DMY_center01`, w its turn about z.
    pub start: V4,
    /// `DrawBG`'s `v` (a static of the function in the game), and the
    /// value the last `DrawBG` wrote to the material before stepping it.
    pub scroll_v: F,
    pub scroll_shown: F,
    /// `sw[0..2]` (+0x29c): each bobbing model rising (0) or sinking.
    pub sw: [i32; 3],
    /// `layerSw` (+0x2b4).
    pub layer_sw: bool,
    /// `fieldrand`, which the fireflies and the bobbing draw from. `GO(1)`
    /// sets no seed for a story map; the port starts the arena's own.
    pub rng: Rng,
    /// The 54 `FIREFLY2`s (+0x1c4).
    pub fireflies: Vec<Firefly>,
    hit_models: Vec<HitModel>,
    pub hits: Hits,
}

/// A clump's models as `ccClump::Draw` draws them.
fn clump_models(file: &SceneFile, name: &str) -> Option<(u32, Vec<u32>)> {
    let c = file.ccs.find_object(name)?;
    let nodes = file.scene.clumps.iter().find(|(cl, _)| *cl == c).map(|(_, n)| n.clone())?;
    let mut m: Vec<u32> = nodes.iter().filter_map(|n| file.obj_model.get(n).copied()).collect();
    m.sort_unstable();
    Some((c, m))
}

/// A name of `se1_5`'s with the stage's digits: the constructor writes the
/// field's two digits over characters 6 and 8 (`MDL_se1_5fl0`, `LGT_se1_5lig1`).
fn staged(name: &str, stage: &str) -> String {
    name.replacen("se1_5", stage, 1)
}

impl Arena {
    /// `EVENTAREAB0::EVENTAREAB0` for `field` (1-8), `fieldrand` from 0.
    pub fn new(archive: &Arc<Archive>, field: i32, def_se: u32) -> Result<Arena> {
        Arena::new_seeded(archive, field, def_se, 0)
    }

    /// `EVENTAREAB0::EVENTAREAB0` for `field` (1-8), `fieldrand` from
    /// `seed` (the fireflies draw from it as they are made).
    pub fn new_seeded(archive: &Arc<Archive>, field: i32, def_se: u32, seed: u32) -> Result<Arena> {
        let stage = *usize::try_from(field)
            .ok()
            .filter(|_| is_arena(field))
            .and_then(|f| STAGES.get(f))
            .ok_or_else(|| Error::NotFound(format!("EVENTAREAB0: field {field}")))?;
        let file = Rc::new(SceneFile::read(archive, stage)?);
        let (c, sc) = (&file.ccs, &file.scene);
        let table = statics::tables()
            .iter()
            .find(|t| t.name == "EA_MODELTABLEB0")
            .ok_or_else(|| Error::NotFound("EA_MODELTABLEB0".into()))?;
        let mut models = Vec::new();
        for (row, r) in table.rows.iter().enumerate() {
            let name = staged(r.model, stage);
            let model = c.find_object(&name).ok_or_else(|| Error::NotFound(format!("{stage}: {name}")))?;
            let pos = match r.position {
                Position::None => [0; 4],
                _ => crate::town::dummy_bits(&r.position, c, sc).0,
            };
            models.push(StaticModel { row, pass: r.pass, model, pos, clip: r.clip.to_bits() });
        }
        let mut bg: [Option<(u32, Vec<u32>)>; 3] = Default::default();
        for (k, part) in ["bac1", "clo1", "clo2"].into_iter().enumerate() {
            let name = format!("CMP_{stage}{part}");
            bg[k] = Some(clump_models(&file, &name).ok_or_else(|| Error::NotFound(format!("{stage}: {name}")))?);
        }
        // eventareaB0Light: the distant light, then the 19 omni lights, at
        // the light animation's frame 1 (one _AnimateForward).
        let anim = format!("ANM_{stage}bac1a");
        let ai = file.anim(&anim).ok_or_else(|| Error::NotFound(anim.clone()))?;
        let (ambient, records) = crate::town::anim_lights(&file, ai)?;
        let mut group: Vec<Light> = Vec::new();
        let mut light_objects = Vec::new();
        for k in 0..20 {
            let name = if k == 0 { format!("LGT_{stage}lig1") } else { format!("LGT_{stage}omn{k:02}") };
            let Some(obj) = c.find_object(&name) else { continue };
            light_objects.push(obj);
            if let Some(l) = records.iter().find(|l| l.object == obj) {
                group.push(l.at(256));
            }
        }
        let lights = TownLights { ambient: ambient.unwrap_or(glam::Vec3::ZERO), lights: group, fog: Some(FOG.depth()) };
        let (pos, rot) = crate::town::dummy_bits(&Position::DummyRot("DMY_center01"), c, sc);
        if c.find_object("DMY_center01").is_none() {
            return Err(Error::NotFound(format!("{stage}: DMY_center01")));
        }
        let hit_models = HitModel::read(c)?;
        let mut hits =
            Hits { area: 1, bounds: Some(BOUNDS), heights: None, def_se, event_area: true, ..Hits::default() };
        hits.models = models.iter().filter_map(|m| hit_models.iter().find(|h| h.parent == m.model).cloned()).collect();
        // The fireflies, each over its marker.
        let mut rng = Rng::new(seed);
        let mut fireflies = Vec::with_capacity(FIREFLIES);
        for k in 0..FIREFLIES {
            let marker = MARKERS[k % MARKERS.len()];
            if c.find_object(marker).is_none() {
                return Err(Error::NotFound(format!("{stage}: {marker}")));
            }
            let (at, _) = crate::town::dummy_bits(&Position::Dummy(marker), c, sc);
            fireflies.push(Firefly::arena(&mut rng, at));
        }
        Ok(Arena {
            field,
            stage,
            file,
            models,
            bg,
            lights,
            light_objects,
            start: [pos[0], pos[1], pos[2], rot[2]],
            scroll_v: 0,
            scroll_shown: 0,
            sw: [0, 1, 0],
            layer_sw: false,
            rng,
            fireflies,
            hit_models,
            hits,
        })
    }

    /// `SwitchLayer` (gcmn 0x004095b0).
    pub fn switch_layer(&mut self) {
        self.layer_sw = !self.layer_sw;
    }

    /// `WORLD_MAN::SetCharPosition` in a story map (main 0x001a1414): the
    /// leader at `eventStartPos` facing its w, the others at (+300, +150),
    /// (-300, +150) and (0, +300) from him.
    pub fn start_positions(&self) -> (V4, F, [V4; 3]) {
        let [x, y, z, dirc] = self.start;
        let starts = [
            [ee::add(0x4396_0000, x), ee::add(0x4316_0000, y), z, ONE],
            [ee::sub(x, 0x4396_0000), ee::add(0x4316_0000, y), z, ONE],
            [x, ee::add(0x4396_0000, y), z, ONE],
        ];
        ([x, y, z, ONE], dirc, starts)
    }

    /// `EVENTAREA::SetCenter(x, y)`.
    pub fn set_center(&mut self, x: F, y: F) {
        self.hits.center = [x, y];
    }

    /// `EVENTAREAB0::Draw` for this frame, the player at `player`: the
    /// pieces in its order, with what it steps (the scroll, the fireflies,
    /// the bobbing models, their draws from `fieldrand`).
    pub fn select(&mut self, player: V4) -> Vec<Piece> {
        let mut out = Vec::new();
        // DrawBG: the material takes v, then v steps.
        self.scroll_shown = self.scroll_v;
        self.scroll_v = ee::add(self.scroll_v, SCROLL);
        if !ee::le(self.scroll_v, ONE) {
            self.scroll_v = ee::sub(self.scroll_v, ONE);
        }
        out.push(Piece::Bg { k: 0, layer: layer::BG0 });
        out.push(Piece::Bg { k: 1, layer: BG1 });
        out.push(Piece::Bg { k: 2, layer: layer::BG2 });
        for (k, m) in self.models.iter().enumerate() {
            if m.pass == DrawPass::Obj {
                out.push(Piece::Model { k, layer: layer::OBJ });
            }
        }
        for (k, m) in self.models.iter().enumerate() {
            if m.pass == DrawPass::Floor {
                out.push(Piece::Model { k, layer: draw::FLOOR_LAYER });
            }
        }
        out.push(Piece::Model { k: 5, layer: if self.layer_sw { layer::OBJ } else { REF_LAYER } });
        // effLayer: each firefly's Draw, then its Move.
        let mut drawn = Vec::new();
        for f in &mut self.fireflies {
            f.draw(player, &mut drawn);
            f.step(player, &mut self.rng);
        }
        out.extend(drawn.into_iter().map(|draw| Piece::Firefly { draw, layer: layer::EFF }));
        // Models 6-8 bob: up by fieldrand(5) until past 50, down until
        // under -50.
        for k in 0..3 {
            let r = ee::from_int(self.rng.below(5) as i32);
            let z = &mut self.models[6 + k].pos[2];
            if self.sw[k] == 0 {
                *z = ee::add(*z, r);
                if !ee::le(*z, BOB) {
                    self.sw[k] = 1;
                }
            } else {
                *z = ee::sub(*z, r);
                if ee::lt(*z, NEG_BOB) {
                    self.sw[k] = 0;
                }
            }
        }
        out
    }

    /// `MAT_*clo1`'s texture offset as `DrawBG` writes it (v * 4096 less
    /// the chunk's own; u is left alone), for the draw.
    fn scroll_rows(&self) -> HashMap<u32, [u8; 2]> {
        let mut rows = HashMap::new();
        let name = format!("MAT_{}clo1", self.stage);
        if let Some(mat) = self.file.ccs.find_object(&name)
            && let Some(m) = self.file.scene.materials.get(&mat)
        {
            let v = (ee::to_int(ee::mul(self.scroll_shown, 0x4580_0000)) & 0xffff) as u16;
            rows.insert(mat, [0, (v.wrapping_sub(m.crop_v) >> 4) as u8]);
        }
        rows
    }

    fn model_draw(&self, layers: &mut Layers, layer: i16, to_screen: Mat4, model: u32, world: Mat4) {
        let rows = self.scroll_rows();
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
        draw::model(layers, layer, to_screen, d);
    }

    /// `EVENTAREAB0::Draw` into the field's layers, stepping only when
    /// `step` (the tasks awake); the fireflies' sprites for the effects to
    /// draw.
    pub fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, player: V4, step: bool) -> Vec<StorySprite> {
        let pieces = if step {
            self.select(player)
        } else {
            let saved =
                (self.scroll_v, self.scroll_shown, self.sw, self.rng, self.models.clone(), self.fireflies.clone());
            let p = self.select(player);
            (self.scroll_v, self.scroll_shown, self.sw, self.rng, self.models, self.fireflies) = saved;
            p
        };
        let eff = usize::try_from(self.field).ok().and_then(|f| FIREFLY_EFF.get(f)).copied().unwrap_or("");
        let mut sprites = Vec::new();
        for p in pieces {
            match p {
                Piece::Bg { k, layer } => {
                    let Some((_, models)) = &self.bg[k] else { continue };
                    for &m in models {
                        self.model_draw(layers, layer, to_screen, m, Mat4::IDENTITY);
                    }
                }
                Piece::Model { k, layer } => {
                    let m = &self.models[k];
                    let world =
                        Mat4::from_translation(glam::Vec3::new(ee::f(m.pos[0]), ee::f(m.pos[1]), ee::f(m.pos[2])));
                    self.model_draw(layers, layer, to_screen, m.model, world);
                }
                Piece::Firefly { draw, layer } => sprites.push(StorySprite {
                    file: self.stage,
                    eff,
                    pos: draw.pos,
                    layer,
                    pat: draw.pattern,
                    scale: draw.scale,
                    rotate: 0,
                    transparency: draw.transparency,
                    flare: false,
                }),
            }
        }
        sprites
    }

    /// The Hit chunks of the stage, for a check.
    pub fn hit_model_count(&self) -> usize {
        self.hit_models.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Skeith's arena: the stage's models, the floor's collision, the
    /// start at the centre, the lights.
    #[test]
    fn skeith_arena() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let a = Arena::new(&archive, 1, 0).unwrap();
        let name = |o: u32| a.file.ccs.object_name(o).unwrap().to_string();
        assert_eq!(a.stage, "se1_5");
        assert_eq!(a.models.len(), 9);
        assert_eq!(name(a.models[5].model), "MDL_se1_5ob0");
        assert_eq!(a.hits.models.len(), 1);
        assert_eq!(name(a.hits.models[0].object), "HIT_se1_5fl0hit");
        assert_eq!(a.light_objects.len(), 20);
        assert!(a.bg.iter().all(|b| b.is_some()));
        assert_eq!(a.fireflies.len(), FIREFLIES);
    }
}
