//! Area 16's story map, Hideous Someone's Giant: `EVENTAREA07` (gcmn
//! 0x00405a00-0x00406cdc), which `WORLD_MAN::GO(1)` makes for
//! `eventAreaNumber` 16. Infection reaches it after the ending (side event
//! 62). Block 0 is the giant's plain under a bobbing sky with 25 `CLOUD`s and
//! a lens flare; block 1 (`se1_7_2`'s names, read through `se1_7_1`'s handle
//! in the game) is one no Infection script reaches. The tables by block are
//! in docs/engine/evarea.md.

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

use crate::area::AreaCore;
use crate::draw::{self, Draw};
use crate::ee::{self, F, ONE, V4};
use crate::hit::{HitModel, UNIT};
use crate::sky::cloud::{self, Cloud};
use crate::sky::lensflare::{self, FLARES};
use crate::story_map::holy_ground::Placing;
use crate::story_map::holy_ground::{MoveTex, StaticModel, StaticObject, StorySprite, clump_models, layer};
use crate::story_map::{Enter, StartSpread, StoryMap};
use crate::story_map::{Made, MapDef};
use crate::story_map::{event_hits, light_group};
use crate::town::{FogParams, TownLights, TownView};
use piney_desktop::MOVE_TEX_ROWS;

/// `eventAreaNumber` 16.
pub const AREA: i32 = 16;
/// Block 0's scene file (+0x1a4), block 1's, and `effccs` (+0x250).
pub const FILE: &str = "se1_7_1";
pub const FILE_2: &str = "se1_7_2";
pub const EFF_FILE: &str = "town_z";
/// `LENSFLARE::Draw`'s sun.
const SUN: &str = "DMY_se1_7lig1point";

/// Block 0's `ccSys.bgColor`, 0xdcfae6.
pub const BG_COLOR: [u8; 3] = [0xe6, 0xfa, 0xdc];

/// The blocks' `SetFog`.
pub const FOG: [FogParams; 2] = [
    FogParams { near: 15000.0, far: 35000.0, near_rate: 0.0, far_rate: 80.0, colour: BG_COLOR },
    FogParams { near: 500.0, far: 5000.0, near_rate: 0.0, far_rate: 80.0, colour: [0, 0, 0] },
];

/// `EA_BGNAME07` (gcmn 0x005d2230): `bg[k]`, each drawn on `bgLayer[k]`.
const BG: [(&str, i16); 4] = [
    ("CMP_se1_7bg1", layer::BG0),
    ("CMP_se1_7cl3", crate::story_map::boss_arena::BG1),
    ("CMP_se1_7cl1", layer::BG2),
    ("CMP_se1_7cl2", layer::BG3),
];

/// `EA_moveTex07` (gcmn 0x005d2240): the clouds' materials, V by 0.1 a
/// frame.
pub const MOVE_TEX: [MoveTex; 2] = [
    MoveTex { u: 0, v: 0, u_step: 0, v_step: 0x3dcc_cccd, mat: "MAT_se1_7clo1" },
    MoveTex { u: 0, v: 0, u_step: 0, v_step: 0x3dcc_cccd, mat: "MAT_se1_7cl3" },
];

/// The bob's top, 350.
const BOB_TOP: F = 0x43af_0000;
/// `CLOUD`s, of type 1.
pub const CLOUDS: usize = 25;
const CLOUD_TYPE: i32 = 1;

/// One piece `EVENTAREA07::Draw` draws, in its order, on its layer.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    /// `DrawBG`: `bg[k]` at (0, 0, bob).
    Bg { k: usize, layer: i16 },
    /// `STATICMODEL::Draw` of `modelArray[k]`.
    Model { k: usize, layer: i16 },
    /// `STATICOBJECT::Draw` of `anmArray[k]`, stepped to `time`.
    Object { k: usize, layer: i16, time: u32 },
    /// A cloud's `Draw`.
    Cloud { k: usize },
    /// `LENSFLARE::Draw`'s flare `k`.
    Flare { k: usize, pos: V4 },
}

/// `EVENTAREA07` as `WORLD_MAN.eventmap` holds it.
pub struct Giant {
    /// `se1_7_1` (the core's file), its lights and start as the block set
    /// them, the collision, `fieldrand`; then `se1_7_2` and `town_z`.
    pub core: AreaCore,
    pub file_2: Rc<SceneFile>,
    pub eff: Rc<SceneFile>,
    /// `blk` (+0x1e4).
    pub block: i32,
    /// `modelArray` (+0x78), `anmArray` (+0x140).
    pub models: Vec<StaticModel>,
    pub objects: Vec<StaticObject>,
    /// `bg[0..3]` (+0x1a8): each clump's models.
    pub bg: [Option<(u32, Vec<u32>)>; 4],
    pub light_objects: Vec<u32>,
    pub fog: FogParams,
    pub move_tex: [MoveTex; 2],
    /// The bob (+0x1d8, the z of +0x1d0) and its direction (+0x1c4: 1
    /// rising).
    pub bob: F,
    pub rising: bool,
    /// The 25 `CLOUD`s (+0x1ec) and `EFF_srzsmo1`'s `patNum`.
    pub clouds: Vec<Cloud>,
    pat_num: u16,
    /// The sun's place, and where the flares were drawn last.
    pub sun: V4,
    pub flares: Option<[V4; 6]>,
    hit_models: [Vec<HitModel>; 2],
}

/// A dummy's position with its turn about z in w (the chunk's +0x28
/// copied into +0x1c before `SetStartPos`).
fn marker(file: &SceneFile, name: &'static str) -> Result<V4> {
    if file.ccs.find_object(name).is_none() {
        return Err(Error::NotFound(format!("{}: {name}", file.stem)));
    }
    let (pos, rot) = crate::town::dummy_bits(&Position::DummyRot(name), &file.ccs, &file.scene);
    Ok([pos[0], pos[1], pos[2], rot[2]])
}

impl Giant {
    /// `EVENTAREA07::EVENTAREA07`, `game.areaPrev` being `area_prev` (a
    /// dungeon's 2 starts at its door), `fieldrand` the game's (`rng`).
    pub fn new(archive: &Arc<Archive>, volume: Volume, def_se: u32, area_prev: i32, rng: Rng) -> Result<Giant> {
        let file = Rc::new(SceneFile::read(archive, FILE)?);
        let file_2 = Rc::new(SceneFile::read(archive, FILE_2)?);
        let eff = Rc::new(SceneFile::read(archive, EFF_FILE)?);
        for name in FLARES {
            eff.ccs.find_object(name).ok_or_else(|| Error::NotFound(format!("{EFF_FILE}: {name}")))?;
        }
        let pat_num = cloud::eff_pat_num(&eff.ccs, cloud::EFF_SMOKE)
            .ok_or_else(|| Error::NotFound(format!("{EFF_FILE}: {}", cloud::EFF_SMOKE)))?;
        let sun = file
            .ccs
            .find_object(SUN)
            .and_then(|o| file.scene.dummies.get(&o))
            .map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE])
            .ok_or_else(|| Error::NotFound(format!("{FILE}: {SUN}")))?;
        let hit_models = [HitModel::read(volume, &file.ccs)?, HitModel::read(volume, &file_2.ccs)?];
        let hits = event_hits(volume, def_se, Vec::new());
        // The file is block 0's, `marker_pos`'s stream (+0x1a4); the lights
        // and the start are the block's, as `ChangeBlock` sets them;
        // fieldrand, which the clouds and the bob draw from, is the game's
        // one stream, which `GO(1)` does not seed for a story map.
        let lights = TownLights { ambient: glam::Vec3::ZERO, lights: Vec::new(), fog: Some(FOG[0]) };
        let core = AreaCore { file, hits, lights, field_rand: Some(rng), start: [0, 0, 0, ONE] };
        let mut g = Giant {
            core,
            file_2,
            eff,
            block: 0,
            models: Vec::new(),
            objects: Vec::new(),
            bg: Default::default(),
            light_objects: Vec::new(),
            fog: FOG[0],
            move_tex: MOVE_TEX,
            bob: 0,
            rising: false,
            clouds: Vec::new(),
            pat_num,
            sun,
            flares: None,
            hit_models,
        };
        g.change_block(0, area_prev)?;
        Ok(g)
    }

    /// The scene file block `block`'s names are in.
    fn block_file(&self, block: i32) -> &Rc<SceneFile> {
        if block == 0 { &self.core.file } else { &self.file_2 }
    }

    /// `EVENTAREA07::ChangeBlock(block)` (gcmn 0x00405c80), `game.areaPrev`
    /// being `area_prev`.
    pub fn change_block(&mut self, block: i32, area_prev: i32) -> Result<()> {
        self.block = block;
        let b = usize::from(block != 0);
        let file = self.block_file(block).clone();
        let _c = &file.ccs;
        let tname = ["EA_MODELTABLE07", "EA_MODELTABLE0702"][b];
        let table = statics::tables().iter().find(|t| t.name == tname).ok_or_else(|| Error::NotFound(tname.into()))?;
        self.models = StaticModel::load(&file, table, Placing::Origin, str::to_string)?;
        self.objects.clear();
        self.bg = Default::default();
        self.clouds.clear();
        if b == 0 {
            let otable = statics::obj_tables()
                .iter()
                .find(|t| t.name == "EA_OBJTABLE07")
                .ok_or_else(|| Error::NotFound("EA_OBJTABLE07".into()))?;
            self.objects = StaticObject::load(&file, otable, usize::MAX, Placing::Origin)?;
            for (k, (name, _)) in BG.iter().enumerate() {
                self.bg[k] = Some(clump_models(&file, name).ok_or_else(|| Error::NotFound((*name).into()))?);
            }
        }
        // The light animation's lights at frame 1 (eventarea07Light,
        // eventarea0702Light), and its ambient.
        let (anim, lights): (&str, &[&str]) = if b == 0 {
            ("ANM_se1_7bg1a", &["LGT_se1_7lig1"])
        } else {
            ("ANM_se1_7bg2a", &["LGT_se1_7lig2", "LGT_omn01", "LGT_omn02"])
        };
        let (ambient, group, objects) = light_group(&file, anim, lights, false)?;
        self.light_objects = objects;
        self.core.lights = TownLights { ambient, lights: group, fog: Some(FOG[b]) };
        self.fog = FOG[b];
        match (b, area_prev) {
            (0, 2) => self.core.start = marker(&file, "DMY_marker02")?,
            (0, _) => self.core.start = marker(&file, "DMY_marker01")?,
            // DMY_marker03 is in neither of Infection's files: the game
            // would read through a null chunk. The start stays.
            _ => {
                if let Ok(m) = marker(&file, "DMY_marker03") {
                    self.core.start = m;
                }
            }
        }
        if b == 0 {
            if let Some(rng) = &mut self.core.field_rand {
                self.clouds = (0..CLOUDS).map(|_| Cloud::init(CLOUD_TYPE, self.pat_num, [0, 0, 0, ONE], rng)).collect();
            }
            // objects[1], models[0] and [4] at the bob, as it stands.
            let at = [0, 0, self.bob, 0];
            if let Some(o) = self.objects.get_mut(1) {
                o.pos = at;
                o.root = crate::town::pos_rot_zyx(at, [0; 3]);
            }
            for k in [0, 4] {
                if let Some(m) = self.models.get_mut(k) {
                    m.pos = at;
                }
            }
        }
        // The collision: the block's models' Hit chunks, at their places.
        let hm = &self.hit_models[b];
        self.core.hits.models = self
            .models
            .iter()
            .filter_map(|m| {
                let mut h = hm.iter().find(|h| h.parent == m.model).cloned()?;
                // STATICMODEL::SetPos: the hit's matrix the translation,
                // and its inverse.
                if m.pos[..3] != [0; 3] {
                    h.rm = UNIT;
                    h.rm[3] = [m.pos[0], m.pos[1], m.pos[2], ONE];
                    h.im = UNIT;
                    h.im[3] = [m.pos[0] ^ 0x8000_0000, m.pos[1] ^ 0x8000_0000, m.pos[2] ^ 0x8000_0000, ONE];
                }
                Some(h)
            })
            .collect();
        Ok(())
    }

    /// `WORLD_MAN::Enter` on area 16's ground (main 0x0019dda0): from
    /// block 0 (or -1) the dungeon (`ChangeArea(2, 0)`: None); else
    /// `ChangeBlock(0)` and the block the scene changes to.
    pub fn enter(&mut self, scene_block: i32, area_prev: i32) -> Result<Option<i32>> {
        if scene_block == 0 || scene_block == -1 {
            return Ok(None);
        }
        self.change_block(0, area_prev)?;
        Ok(Some(0))
    }

    /// `EVENTAREA::SetCenter(x, y)`.
    pub fn set_center(&mut self, x: F, y: F) {
        self.core.hits.center = [x, y];
    }

    /// `ccSys.bgColor`: block 0's (block 1 keeps it).
    pub fn clear(&self) -> [u8; 3] {
        BG_COLOR
    }

    /// `EVENTAREA07::Draw` (gcmn 0x00406b80) for this frame: the pieces in
    /// its order, with what it steps on the way (the bob, the scrolls, the
    /// objects' animations, the clouds).
    pub fn select(&mut self, v: &TownView) -> Vec<Piece> {
        let mut out = Vec::new();
        if self.block != 0 {
            self.row_pieces(&mut out, DrawPass::Obj, layer::OBJ, v.eye);
            self.row_pieces(&mut out, DrawPass::Floor, draw::FLOOR_LAYER, v.eye);
            return out;
        }
        // DrawBG: the bob steps by fieldrand(25), turning at 350 and 0.
        let r = self.core.field_rand.as_mut().map_or(0, |rng| ee::from_int(rng.below(25) as i32));
        if self.rising {
            self.bob = ee::add(self.bob, r);
            if !ee::le(self.bob, BOB_TOP) {
                self.bob = BOB_TOP;
                self.rising = false;
            }
        } else {
            self.bob = ee::sub(self.bob, r);
            if ee::lt(self.bob, 0) {
                self.bob = 0;
                self.rising = true;
            }
        }
        for t in &mut self.move_tex {
            t.step();
        }
        for (k, &(_, layer)) in BG.iter().enumerate() {
            out.push(Piece::Bg { k, layer });
        }
        // objLayer: DrawObj2, DrawObj, DrawFloor.
        self.row_pieces(&mut out, DrawPass::Obj2, layer::OBJ, v.eye);
        self.row_pieces(&mut out, DrawPass::Obj, layer::OBJ, v.eye);
        self.row_pieces(&mut out, DrawPass::Floor, layer::OBJ, v.eye);
        // effLayer: each cloud's Move, then its Draw.
        for k in 0..self.clouds.len() {
            let volume = self.core.hits.volume;
            if let Some(rng) = &mut self.core.field_rand {
                self.clouds[k].step(volume, v.player, rng, &|_, _| 0);
            }
            let c = &self.clouds[k];
            let in_view = crate::town::pc::check_camera_deg(c.pos, cloud::VIEW_DEG, &v.cam, v.player);
            if c.drawn(volume, v.player, in_view) {
                out.push(Piece::Cloud { k });
            }
        }
        // LENSFLARE::Draw(se1_7_1, DMY_se1_7lig1point, 0).
        let sun_in_view = crate::town::pc::check_camera_deg(self.sun, lensflare::VIEW_DEG, &v.cam, v.player);
        self.flares = lensflare::draw(self.sun, &v.flare, v.puppet_show, sun_in_view);
        if let Some(f) = self.flares {
            out.extend(f.into_iter().enumerate().map(|(k, pos)| Piece::Flare { k, pos }));
        }
        out
    }

    /// One of `DrawObj`, `DrawObj2`, `DrawFloor`: the models of `pass`,
    /// then its static objects (each stepped and drawn within its clip of
    /// the eye).
    fn row_pieces(&mut self, out: &mut Vec<Piece>, pass: DrawPass, layer: i16, eye: V4) {
        for (k, m) in self.models.iter().enumerate() {
            if m.pass == pass {
                out.push(Piece::Model { k, layer });
            }
        }
        let file = self.block_file(self.block).clone();
        let volume = self.core.hits.volume;
        for k in 0..self.objects.len() {
            let o = &mut self.objects[k];
            if o.pass != pass {
                continue;
            }
            let d = ee::ground_dist(volume, ee::sub(eye[0], o.pos[0]), ee::sub(eye[1], o.pos[1]));
            if o.clip == 0 || ee::lt(d, o.clip) {
                o.play.forward(&file);
                out.push(Piece::Object { k, layer, time: o.play.time });
            }
        }
    }

    /// The clouds' materials' offsets as `MoveTexture` wrote them.
    fn scroll_rows(&self) -> HashMap<u32, [u8; 2]> {
        let mut rows = HashMap::new();
        for t in &self.move_tex {
            let Some(mat) = self.core.file.ccs.find_object(t.mat) else { continue };
            let Some(m) = self.core.file.scene.materials.get(&mat) else { continue };
            let [u, v] = t.offsets();
            rows.insert(mat, [(u.wrapping_sub(m.crop_u) >> 4) as u8, (v.wrapping_sub(m.crop_v) >> 4) as u8]);
        }
        rows
    }

    #[allow(clippy::too_many_arguments)]
    fn model_draw(&self, layers: &mut Layers, layer: i16, to_screen: Mat4, model: u32, world: Mat4, fog: bool) {
        let rows = self.scroll_rows();
        let d = Draw {
            file: self.block_file(self.block),
            model,
            world,
            alpha: 1.0,
            rows: &rows,
            lights: None,
            nodes: &[],
            morph: Vec::new(),
            clut_swaps: Vec::new(),
        };
        let fogging = if fog { draw::Fogging::Depth(self.fog.depth()) } else { draw::Fogging::None };
        draw::model_edited(layers, layer, to_screen, &d, None, fogging);
    }

    /// `EVENTAREA07::Draw` into the field's layers, stepping only when
    /// `step` (the tasks awake); the clouds' and the flares' sprites for
    /// the effects to draw. The background clumps have their fog off.
    pub fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite> {
        let pieces = if step {
            self.select(v)
        } else {
            let saved = (
                self.move_tex,
                self.objects.clone(),
                self.bob,
                self.rising,
                self.clouds.clone(),
                self.core.field_rand,
                self.flares,
            );
            let p = self.select(v);
            (self.move_tex, self.objects, self.bob, self.rising, self.clouds, self.core.field_rand, self.flares) =
                saved;
            p
        };
        let file = self.block_file(self.block).clone();
        let mut sprites = Vec::new();
        for p in pieces {
            match p {
                Piece::Bg { k, layer } => {
                    let Some((_, models)) = &self.bg[k] else { continue };
                    let world = Mat4::from_translation(glam::Vec3::new(0.0, 0.0, ee::f(self.bob)));
                    for &m in models {
                        self.model_draw(layers, layer, to_screen, m, world, false);
                    }
                }
                Piece::Model { k, layer } => {
                    let m = &self.models[k];
                    let world =
                        Mat4::from_translation(glam::Vec3::new(ee::f(m.pos[0]), ee::f(m.pos[1]), ee::f(m.pos[2])));
                    self.model_draw(layers, layer, to_screen, m.model, world, true);
                }
                Piece::Object { k, layer, .. } => {
                    let o = &self.objects[k];
                    crate::town::anim_draw_fog(
                        layers,
                        layer,
                        to_screen,
                        &file,
                        &o.play,
                        draw::mat(&o.root),
                        &[],
                        1.0,
                        None,
                        None,
                        draw::Fogging::Depth(self.fog.depth()),
                    );
                }
                Piece::Cloud { k } => {
                    let c = &self.clouds[k];
                    sprites.push(StorySprite {
                        file: EFF_FILE,
                        eff: cloud::EFF_SMOKE,
                        pos: c.pos,
                        layer: layer::EFF,
                        pat: c.pattern as u16,
                        scale: c.eff.scale_x,
                        rotate: c.eff.rotate,
                        transparency: c.eff.transparency,
                        flare: true,
                        fog: true,
                    });
                }
                Piece::Flare { k, pos } => sprites.push(StorySprite {
                    file: EFF_FILE,
                    eff: FLARES[k],
                    pos,
                    layer: layer::EFF,
                    pat: 0,
                    scale: ONE,
                    rotate: 0,
                    transparency: ONE,
                    flare: true,
                    fog: true,
                }),
            }
        }
        sprites
    }
}

/// `GO(1)`'s area 16, started by `game.areaPrev`.
pub const DEF: MapDef = MapDef {
    class: "EVENTAREA07",
    claims: |e| e.field == AREA,
    make: |e| Ok(Made::New(Box::new(Giant::new(e.archive, e.volume, e.def_se, e.area_prev, e.field_rand)?))),
};

impl StoryMap for Giant {
    fn core(&self) -> &AreaCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut AreaCore {
        &mut self.core
    }
    fn put_scrolls(&self, gcmn: &mut [[u32; 2]; MOVE_TEX_ROWS]) {
        for (k, t) in self.move_tex.iter().enumerate() {
            gcmn[crate::story_map::holy_ground::scroll::EA07 + k] = t.scroll();
        }
    }
    fn take_scrolls(&mut self, gcmn: &[[u32; 2]; MOVE_TEX_ROWS]) {
        for (k, t) in self.move_tex.iter_mut().enumerate() {
            *t = t.at(gcmn, crate::story_map::holy_ground::scroll::EA07 + k);
        }
    }
    /// Main 0x001a1190's (+-200, -150).
    fn spread(&self) -> StartSpread {
        StartSpread::AHEAD
    }
    fn clear(&self) -> [u8; 3] {
        Giant::clear(self)
    }
    /// Area 16: from block 0 its dungeon, else back to block 0.
    fn enter(&mut self, block: i32, area_prev: i32) -> Result<Enter> {
        Ok(match Giant::enter(self, block, area_prev)? {
            None => Enter::Dungeon,
            Some(b) => Enter::Block(b),
        })
    }
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite> {
        Giant::draw(self, layers, to_screen, v, step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both blocks: the tables' pieces, one Hit chunk each, the start
    /// markers, the lights, the clouds of block 0.
    #[test]
    fn the_giants_blocks() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut g = Giant::new(&archive, Volume::Inf, 0, 0, Rng::new(0)).unwrap();
        let name = |f: &SceneFile, o: u32| f.ccs.object_name(o).unwrap().to_string();
        assert_eq!((g.models.len(), g.objects.len()), (8, 2));
        assert!(g.bg.iter().all(|b| b.is_some()));
        assert_eq!(g.clouds.len(), CLOUDS);
        assert_eq!(g.core.hits.models.len(), 1);
        assert_eq!(name(&g.core.file, g.core.hits.models[0].object), "HIT_se1_7ob1hit");
        assert_eq!(name(&g.core.file, g.core.hits.models[0].parent), "MDL_se1_7ob1_1");
        assert_eq!(g.light_objects.len(), 1);
        let arrive = g.core.start;
        let back = Giant::new(&archive, Volume::Inf, 0, 2, Rng::new(0)).unwrap().core.start;
        assert_ne!(arrive, back, "DMY_marker01 and 02 are different places");
        g.change_block(1, 1).unwrap();
        assert_eq!((g.models.len(), g.objects.len(), g.clouds.len()), (2, 0, 0));
        assert_eq!(g.core.hits.models.len(), 1);
        assert_eq!(name(&g.file_2, g.core.hits.models[0].object), "HIT_se1_7ob1_5hit");
        assert_eq!(g.light_objects.len(), 3);
        assert_eq!(g.enter(1, 1).unwrap(), Some(0));
        assert_eq!(g.block, 0);
        assert_eq!(g.enter(0, 1).unwrap(), None);
    }

    /// The bob stays within 0..350 and turns at both ends.
    #[test]
    fn the_background_bobs() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut g = Giant::new(&archive, Volume::Inf, 0, 0, Rng::new(7)).unwrap();
        let v = TownView::default();
        let (mut top, mut turns, mut last) = (0.0f32, 0, g.rising);
        for _ in 0..600 {
            g.select(&v);
            let z = ee::f(g.bob);
            assert!((0.0..=350.0).contains(&z), "{z}");
            top = top.max(z);
            if g.rising != last {
                turns += 1;
                last = g.rising;
            }
        }
        assert!(top == 350.0 && turns >= 2, "top {top}, turns {turns}");
    }
}
