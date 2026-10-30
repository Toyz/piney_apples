//! The boss arenas: `EVENTAREAB0` (gcmn areab0.cpp, constructor 0x004079f0,
//! `Draw` 0x004095e0), the story map `WORLD_MAN::GO(1)` makes for fields 1-8,
//! each a stage of its own scene file (Infection's Skeith on field 1,
//! `se1_5`). Nine static models, three sky clumps with a scrolling cloud
//! layer, 54 `FIREFLY2`s ([`crate::sky::firefly`]), a reflection model on the
//! layer `SwitchLayer` flips (Skeith's magic) and three bobbing models. The
//! collision is the floor model's Hit chunk, with no height map
//! (docs/engine/evarea.md).

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::statics::{self, DrawPass, Position};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::draw::{self, Draw};
use crate::ee::{self, F, ONE, V4};
use crate::hit::{HitModel, Hits};
use crate::sky::firefly::{Firefly, FireflyDraw};
use crate::story_map::holy_ground::Placing;
use crate::story_map::holy_ground::{StaticModel, StorySprite, clump_models, layer};
use crate::story_map::{Made, MapDef, StoryMap};
use crate::story_map::{event_hits, light_group, model_hits};
use crate::town::TownView;
use crate::town::{FogParams, TownLights};
use piney_desktop::SCROLL_STATICS;

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
    /// The last stage's `ccAnm::Draw` (`ANM_se4_8bac1a`) at the origin.
    LastAnm { layer: i16 },
    /// Its clump (`CMP_se4_8f01`), while `+0x2bc` holds.
    LastClump { layer: i16 },
}

/// Field 8's last stage, which Quarantine's constructor loads beside
/// `se4_7` (QUA gcmn 0x00308b90): `se4_8` (+0x2a8), its `ANM_se4_8bac1a`
/// (+0x2ac) and its floor `CMP_se4_8f01` (+0x2b0), with their hits.
pub struct LastStage {
    pub file: Rc<SceneFile>,
    pub anm: piney_desktop::anm::Anm,
    pub clump: (u32, Vec<u32>),
    pub hit_models: Vec<HitModel>,
}

/// The stage `NextStage` leaves, and the field it does it on.
pub const LAST_STAGE: &str = "se4_8";
pub const LAST_FIELD: i32 = 8;
/// `saveData` byte the constructor reads: set, the arena starts at its
/// last stage (Corbenik's third form, `ccThBoss08`'s test likewise).
pub const LAST_STAGE_BYTE: usize = 0x652f;

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
    /// Field 8's last stage, `stageSW` (+0x2b4 in Quarantine's class:
    /// `NextStage` ran) and its clump drawn (+0x2bc, which Corbenik's
    /// lasers switch: QUA gcmn 0x0030aa40).
    pub last: Option<LastStage>,
    pub stage_sw: bool,
    pub clump_on: bool,
    /// `fieldrand`, which the fireflies and the bobbing draw from: the
    /// game's one stream, which `GO(1)` does not seed for a story map.
    pub rng: Rng,
    /// The 54 `FIREFLY2`s (+0x1c4).
    pub fireflies: Vec<Firefly>,
    hit_models: Vec<HitModel>,
    pub hits: Hits,
}

/// A name of `se1_5`'s with the stage's digits: the constructor writes the
/// field's two digits over characters 6 and 8 (`MDL_se1_5fl0`, `LGT_se1_5lig1`).
fn staged(name: &str, stage: &str) -> String {
    name.replacen("se1_5", stage, 1)
}

impl Arena {
    /// `EVENTAREAB0::EVENTAREAB0` for `field` (1-8), `fieldrand` from 0
    /// (for the checks).
    pub fn new(archive: &Arc<Archive>, volume: Volume, field: i32, def_se: u32) -> Result<Arena> {
        Arena::new_seeded(archive, volume, field, def_se, Rng::new(0))
    }

    /// `EVENTAREAB0::EVENTAREAB0` for `field` (1-8), `fieldrand` the
    /// game's (`rng`: the fireflies draw from it as they are made).
    pub fn new_seeded(archive: &Arc<Archive>, volume: Volume, field: i32, def_se: u32, rng: Rng) -> Result<Arena> {
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
        let models = StaticModel::load(&file, table, Placing::Dummy, |n| staged(n, stage))?;
        let mut bg: [Option<(u32, Vec<u32>)>; 3] = Default::default();
        for (k, part) in ["bac1", "clo1", "clo2"].into_iter().enumerate() {
            let name = format!("CMP_{stage}{part}");
            bg[k] = Some(clump_models(&file, &name).ok_or_else(|| Error::NotFound(format!("{stage}: {name}")))?);
        }
        // eventareaB0Light: the distant light, then the 19 omni lights, at
        // the light animation's frame 1 (one _AnimateForward).
        let names = (0..20).map(|k| if k == 0 { format!("LGT_{stage}lig1") } else { format!("LGT_{stage}omn{k:02}") });
        let (ambient, group, light_objects) = light_group(&file, &format!("ANM_{stage}bac1a"), names, true)?;
        let lights = TownLights { ambient, lights: group, fog: Some(FOG) };
        let (pos, rot) = crate::town::dummy_bits(&Position::DummyRot("DMY_center01"), c, sc);
        if c.find_object("DMY_center01").is_none() {
            return Err(Error::NotFound(format!("{stage}: DMY_center01")));
        }
        let hit_models = HitModel::read(volume, c)?;
        let hits = event_hits(volume, def_se, model_hits(models.iter().map(|m| m.model), &hit_models));
        // The fireflies, each over its marker.
        let mut rng = rng;
        let mut fireflies = Vec::with_capacity(FIREFLIES);
        for k in 0..FIREFLIES {
            let marker = MARKERS[k % MARKERS.len()];
            if c.find_object(marker).is_none() {
                return Err(Error::NotFound(format!("{stage}: {marker}")));
            }
            let (at, _) = crate::town::dummy_bits(&Position::Dummy(marker), c, sc);
            fireflies.push(Firefly::arena(volume, &mut rng, at));
        }
        let last = if field == LAST_FIELD {
            let f = Rc::new(SceneFile::read(archive, LAST_STAGE)?);
            let mut anm = piney_desktop::anm::Anm::new();
            anm.set(&f, "ANM_se4_8bac1a");
            let clump = clump_models(&f, "CMP_se4_8f01")
                .ok_or_else(|| Error::NotFound(format!("{LAST_STAGE}: CMP_se4_8f01")))?;
            let hit_models = HitModel::read(volume, &f.ccs)?;
            Some(LastStage { file: f, anm, clump, hit_models })
        } else {
            None
        };
        Ok(Arena {
            last,
            stage_sw: false,
            clump_on: false,
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

    /// `NextStage` (QUA gcmn 0x0030aa70), field 8 only and once: the
    /// stage's models deleted, its omni lights out of the group (the
    /// distant light kept), `stageSW` and the clump on, the last stage's
    /// clump and clip hit instead.
    pub fn next_stage(&mut self) {
        if self.field != LAST_FIELD || self.stage_sw {
            return;
        }
        let Some(last) = self.last.as_ref() else { return };
        self.models.clear();
        self.lights.lights.truncate(1);
        self.stage_sw = true;
        self.clump_on = true;
        self.hits.models = last.hit_models.clone();
    }

    /// `+0x2bc` set (QUA gcmn 0x0030aa40, `on` the call's 1).
    pub fn set_clump(&mut self, on: bool) {
        self.clump_on = on;
    }

    /// `SwitchLayer` (gcmn 0x004095b0).
    pub fn switch_layer(&mut self) {
        self.layer_sw = !self.layer_sw;
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
        // The last stage (QUA 0x0030a968): its clip forward and drawn on
        // objLayer, the clump on floorLayer while +0x2bc holds; no more.
        if self.stage_sw {
            if let Some(l) = self.last.as_mut() {
                l.anm.forward();
            }
            out.push(Piece::LastAnm { layer: layer::OBJ });
            if self.clump_on {
                out.push(Piece::LastClump { layer: draw::FLOOR_LAYER });
            }
            return out;
        }
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
        let volume = self.hits.volume;
        for f in &mut self.fireflies {
            f.draw(volume, player, &mut drawn);
            f.step(volume, player, &mut self.rng);
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
        self.file_draw(&self.file, layers, layer, to_screen, model, world);
    }

    fn file_draw(&self, file: &SceneFile, layers: &mut Layers, layer: i16, to_screen: Mat4, model: u32, world: Mat4) {
        let rows = self.scroll_rows();
        let d = Draw {
            file,
            model,
            world,
            alpha: 1.0,
            rows: &rows,
            lights: None,
            nodes: &[],
            morph: Vec::new(),
            clut_swaps: Vec::new(),
        };
        draw::model(layers, layer, to_screen, &d);
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
            let anm = self.last.as_ref().map(|l| l.anm.clone());
            let p = self.select(player);
            (self.scroll_v, self.scroll_shown, self.sw, self.rng, self.models, self.fireflies) = saved;
            if let (Some(l), Some(a)) = (self.last.as_mut(), anm) {
                l.anm = a;
            }
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
                Piece::LastAnm { layer } => {
                    let Some(l) = self.last.as_ref() else { continue };
                    for inst in l.anm.instances() {
                        self.file_draw(&l.file, layers, layer, to_screen, inst.model, inst.world);
                    }
                }
                Piece::LastClump { layer } => {
                    let Some(l) = self.last.as_ref() else { continue };
                    for &m in &l.clump.1 {
                        self.file_draw(&l.file, layers, layer, to_screen, m, Mat4::IDENTITY);
                    }
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
                    fog: true,
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

/// `GO(1)`'s boss arenas. Quarantine's constructor puts field 8 at its last
/// stage once the save says so (QUA gcmn 0x0030a170).
pub const DEF: MapDef = MapDef {
    class: "EVENTAREAB0",
    claims: |e| is_arena(e.field),
    make: |e| {
        let mut a = Arena::new_seeded(e.archive, e.volume, e.field, e.def_se, e.field_rand)?;
        if e.volume == Volume::Qua && e.save.record()[LAST_STAGE_BYTE] != 0 {
            a.next_stage();
        }
        Ok(Made::New(Box::new(a)))
    },
};

impl StoryMap for Arena {
    /// `marker_pos`'s stream (+0x1a4).
    fn file(&self) -> Option<&piney_desktop::assets::SceneFile> {
        Some(&self.file)
    }
    fn take_statics(&mut self, s: &[u32; SCROLL_STATICS]) {
        self.scroll_v = s[crate::town::scroll_statics::ARENA_BG];
    }
    fn put_statics(&self, s: &mut [u32; SCROLL_STATICS]) {
        s[crate::town::scroll_statics::ARENA_BG] = self.scroll_v;
    }
    fn field_rand(&self) -> Option<&Rng> {
        Some(&self.rng)
    }
    fn field_rand_mut(&mut self) -> Option<&mut Rng> {
        Some(&mut self.rng)
    }
    fn hits(&self) -> &Hits {
        &self.hits
    }
    fn hits_mut(&mut self) -> &mut Hits {
        &mut self.hits
    }
    fn lights(&self) -> &TownLights {
        &self.lights
    }
    fn start(&self) -> V4 {
        self.start
    }
    fn clear(&self) -> [u8; 3] {
        crate::story_map::boss_arena::BG_COLOR
    }
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite> {
        Arena::draw(self, layers, to_screen, v.player, step)
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
        let a = Arena::new(&archive, Volume::Inf, 1, 0).unwrap();
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

    /// Corbenik's arena's last stage: `se4_8`'s clip and floor drawn, its
    /// floor gone while the lasers switch it off.
    #[test]
    fn corbenik_last_stage() {
        let Some(archive) = crate::town::tests::archive_of("quarantine") else { return };
        let mut a = Arena::new(&archive, Volume::Qua, LAST_FIELD, 0).unwrap();
        assert!(a.select([0, 0, 0, ONE]).iter().any(|p| matches!(p, Piece::Firefly { .. } | Piece::Model { .. })));
        a.next_stage();
        assert!(a.stage_sw && a.clump_on);
        assert_eq!(a.lights.lights.len(), 1, "the distant light kept");
        assert!(!a.hits.models.is_empty(), "the last stage's hit");
        let p = a.select([0, 0, 0, ONE]);
        assert_eq!(p, vec![Piece::LastAnm { layer: layer::OBJ }, Piece::LastClump { layer: draw::FLOOR_LAYER }]);
        a.set_clump(false);
        assert_eq!(a.select([0, 0, 0, ONE]), vec![Piece::LastAnm { layer: layer::OBJ }]);
    }

    /// `DrawBG`'s `v$1597` is the executable's: an arena takes it as the
    /// last one left it and hands it back.
    #[test]
    fn the_arenas_scroll_is_the_executables() {
        use crate::story_map::StoryMap;
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut a = Arena::new(&archive, Volume::Inf, 1, 0).unwrap();
        let mut s = [0; piney_desktop::SCROLL_STATICS];
        s[crate::town::scroll_statics::ARENA_BG] = 0x3e80_0000;
        a.take_statics(&s);
        assert_eq!(a.scroll_v, 0x3e80_0000);
        a.scroll_v = 0x3f00_0000;
        a.put_statics(&mut s);
        assert_eq!(s[crate::town::scroll_statics::ARENA_BG], 0x3f00_0000);
    }
}
