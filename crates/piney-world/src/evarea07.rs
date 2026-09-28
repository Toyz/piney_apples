//! Area 16's story map, Hideous Someone's Giant: `EVENTAREA07` (gcmn
//! 0x00405a00-0x00406cdc), which `WORLD_MAN::GO(1)` makes for
//! `eventAreaNumber` 16 (`docs/engine/evarea.md`). Infection reaches it
//! after the ending: side event 62, "SERVER1", gives its words and waits
//! in its dungeon.
//!
//! ```text
//! EVENTAREA07()     0x00405a00  ccs se1_7_1, effccs town_z, a LENSFLARE of
//!                               it; modelArray[50], anmArray[25], bg[0..5],
//!                               the 25 CLOUDs, lgtAnm cleared; the bob (+0x1d0
//!                               .. +0x1d8) and its direction (+0x1c4) 0;
//!                               ChangeBlock(0)
//! ChangeBlock(b)    0x00405c80  blk (+0x1e4) = b; the block before's pieces
//!                               and clouds deleted; then by block (below)
//! Draw()            0x00406b80  block 0: DrawBG; objLayer: DrawObj2,
//!                               DrawObj, DrawFloor; effLayer: each cloud's
//!                               Move then Draw; LENSFLARE::Draw(se1_7_1,
//!                               DMY_se1_7lig1point, 0). Block 1: objLayer:
//!                               DrawObj; floorLayer: DrawFloor
//! DrawBG()          0x00406730  the bob: fieldrand(25) up to 350, then
//!                               down to 0 (+0x1d8, turning at the ends);
//!                               MoveTexture of EA_moveTex07's two rows;
//!                               bg[k] on bgLayer[k] at (0, 0, bob)
//! DrawObj, DrawObj2, DrawFloor  the type-2, -3, -1 models, then objects
//! ```
//!
//! By block:
//!
//! ```text
//!                 block 0                         block 1
//! SetFog          (15000, 35000, 0, 80, 0xdcfae6) (500, 5000, 0, 80, 0)
//! ccSys.bgColor   0xdcfae6                        (kept)
//! models          EA_MODELTABLE07 (8)             EA_MODELTABLE0702 (2)
//! objects         EA_OBJTABLE07 (2)               none
//! bg              EA_BGNAME07: bg1, cl3, cl1, cl2  none
//! lights          ANM_se1_7bg1a: LGT_se1_7lig1     ANM_se1_7bg2a: LGT_se1_7lig2,
//!                                                 LGT_omn01, LGT_omn02
//! SetStartPos     DMY_marker02 back from the      DMY_marker03
//!                 dungeon (game.areaPrev 2),
//!                 else DMY_marker01
//! then            25 CLOUDs of type 1 (town_z);
//!                 objects[1], models[0] and [4]
//!                 SetPos at the bob
//! ```
//!
//! Block 1's names are `se1_7_2`'s, though the constructor looks them up
//! through `se1_7_1`'s handle; the port reads them from `se1_7_2`. Its
//! start, `DMY_marker03`, is in neither file. No Infection script brings
//! the map to block 1.

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

use crate::cloud::{self, Cloud};
use crate::draw::{self, Draw};
use crate::ee::{self, F, ONE, V4};
use crate::evarea::{BOUNDS, MoveTex, StaticModel, StaticObject, StorySprite, clump_models, layer};
use crate::hit::{HitModel, Hits, UNIT};
use crate::lensflare::{self, FLARES};
use crate::pose::Play;
use crate::town::{FogParams, Light, TownLights, TownView};

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
    ("CMP_se1_7cl3", crate::evarea_b0::BG1),
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
    /// `se1_7_1`, `se1_7_2` and `town_z`.
    pub file: Rc<SceneFile>,
    pub file_2: Rc<SceneFile>,
    pub eff: Rc<SceneFile>,
    morphers: HashMap<u32, u32>,
    /// `blk` (+0x1e4).
    pub block: i32,
    /// `modelArray` (+0x78), `anmArray` (+0x140).
    pub models: Vec<StaticModel>,
    pub objects: Vec<StaticObject>,
    /// `bg[0..3]` (+0x1a8): each clump's models.
    pub bg: [Option<(u32, Vec<u32>)>; 4],
    /// The light group from the block's light animation at frame 1, and
    /// the ambient.
    pub lights: TownLights,
    pub light_objects: Vec<u32>,
    pub fog: FogParams,
    /// `WORLD_MAN.eventStartPos` as the last `SetStartPos` left it: the
    /// dummy's position, w its turn about z.
    pub start: V4,
    pub move_tex: [MoveTex; 2],
    /// The bob (+0x1d8, the z of +0x1d0) and its direction (+0x1c4: 1
    /// rising).
    pub bob: F,
    pub rising: bool,
    /// The 25 `CLOUD`s (+0x1ec) and `EFF_srzsmo1`'s `patNum`.
    pub clouds: Vec<Cloud>,
    pat_num: u16,
    /// `fieldrand`, which the clouds and the bob draw from. `GO(1)` sets
    /// no seed for a story map; the port starts the map's own.
    pub rng: Rng,
    /// The sun's place, and where the flares were drawn last.
    pub sun: V4,
    pub flares: Option<[V4; 6]>,
    hit_models: [Vec<HitModel>; 2],
    pub hits: Hits,
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
    /// dungeon's 2 starts at its door), `fieldrand` from `seed`.
    pub fn new(archive: &Arc<Archive>, def_se: u32, area_prev: i32, seed: u32) -> Result<Giant> {
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
        let mut morphers = piney_data::anim::morphers(&file.ccs).unwrap_or_default();
        morphers.extend(piney_data::anim::morphers(&file_2.ccs).unwrap_or_default());
        let hit_models = [HitModel::read(&file.ccs)?, HitModel::read(&file_2.ccs)?];
        let hits = Hits { area: 1, bounds: Some(BOUNDS), heights: None, def_se, event_area: true, ..Hits::default() };
        let mut g = Giant {
            file,
            file_2,
            eff,
            morphers,
            block: 0,
            models: Vec::new(),
            objects: Vec::new(),
            bg: Default::default(),
            lights: TownLights { ambient: glam::Vec3::ZERO, lights: Vec::new(), fog: Some(FOG[0].depth()) },
            light_objects: Vec::new(),
            fog: FOG[0],
            start: [0, 0, 0, ONE],
            move_tex: MOVE_TEX,
            bob: 0,
            rising: false,
            clouds: Vec::new(),
            pat_num,
            rng: Rng::new(seed),
            sun,
            flares: None,
            hit_models,
            hits,
        };
        g.change_block(0, area_prev)?;
        Ok(g)
    }

    /// The scene file block `block`'s names are in.
    fn block_file(&self, block: i32) -> &Rc<SceneFile> {
        if block == 0 { &self.file } else { &self.file_2 }
    }

    /// `EVENTAREA07::ChangeBlock(block)` (gcmn 0x00405c80), `game.areaPrev`
    /// being `area_prev`.
    pub fn change_block(&mut self, block: i32, area_prev: i32) -> Result<()> {
        self.block = block;
        let b = usize::from(block != 0);
        let file = self.block_file(block).clone();
        let c = &file.ccs;
        let tname = ["EA_MODELTABLE07", "EA_MODELTABLE0702"][b];
        let table = statics::tables().iter().find(|t| t.name == tname).ok_or_else(|| Error::NotFound(tname.into()))?;
        self.models.clear();
        for (row, r) in table.rows.iter().enumerate() {
            let model = c.find_object(r.model).ok_or_else(|| Error::NotFound(format!("{}: {}", file.stem, r.model)))?;
            self.models.push(StaticModel { row, pass: r.pass, model, pos: [0; 4], clip: r.clip.to_bits() });
        }
        self.objects.clear();
        self.bg = Default::default();
        self.clouds.clear();
        if b == 0 {
            let otable = statics::obj_tables()
                .iter()
                .find(|t| t.name == "EA_OBJTABLE07")
                .ok_or_else(|| Error::NotFound("EA_OBJTABLE07".into()))?;
            for (row, r) in otable.rows.iter().enumerate() {
                let name = r.anime.ok_or_else(|| Error::NotFound(format!("EA_OBJTABLE07: row {row}")))?;
                let play = Play::new(&file, name).ok_or_else(|| Error::NotFound(name.into()))?;
                let root = crate::town::pos_rot_zyx([0; 4], [0; 3]);
                self.objects.push(StaticObject { row, pass: r.pass, play, root, pos: [0; 4], clip: r.clip.to_bits() });
            }
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
        let ai = file.anim(anim).ok_or_else(|| Error::NotFound(anim.into()))?;
        let (ambient, records) = crate::town::anim_lights(&file, ai)?;
        let mut group: Vec<Light> = Vec::new();
        self.light_objects.clear();
        for name in lights {
            let obj = c.find_object(name).ok_or_else(|| Error::NotFound((*name).into()))?;
            self.light_objects.push(obj);
            if let Some(l) = records.iter().find(|l| l.object == obj) {
                group.push(l.at(256));
            }
        }
        self.lights =
            TownLights { ambient: ambient.unwrap_or(glam::Vec3::ZERO), lights: group, fog: Some(FOG[b].depth()) };
        self.fog = FOG[b];
        match (b, area_prev) {
            (0, 2) => self.start = marker(&file, "DMY_marker02")?,
            (0, _) => self.start = marker(&file, "DMY_marker01")?,
            // DMY_marker03 is in neither of Infection's files: the game
            // would read through a null chunk. The start stays.
            _ => {
                if let Ok(m) = marker(&file, "DMY_marker03") {
                    self.start = m;
                }
            }
        }
        if b == 0 {
            for _ in 0..CLOUDS {
                let cl = Cloud::init(CLOUD_TYPE, self.pat_num, [0, 0, 0, ONE], &mut self.rng);
                self.clouds.push(cl);
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
        self.hits.models = self
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

    /// `WORLD_MAN::SetCharPosition` for fields 16 and 66 (main 0x001a1190):
    /// the leader at `eventStartPos` facing its w; the others at (+200,
    /// -150), (-200, -150) and (0, +300) from him, facing as he does.
    pub fn start_positions(&self) -> (V4, F, [V4; 3]) {
        let [x, y, z, dirc] = self.start;
        let starts = [
            [ee::add(0x4348_0000, x), ee::sub(y, 0x4316_0000), z, ONE],
            [ee::sub(x, 0x4348_0000), ee::sub(y, 0x4316_0000), z, ONE],
            [x, ee::add(0x4396_0000, y), z, ONE],
        ];
        ([x, y, z, ONE], dirc, starts)
    }

    /// `EVENTAREA::SetCenter(x, y)`.
    pub fn set_center(&mut self, x: F, y: F) {
        self.hits.center = [x, y];
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
        let r = ee::from_int(self.rng.below(25) as i32);
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
            self.clouds[k].step(v.player, &mut self.rng, &|_, _| 0);
            let c = &self.clouds[k];
            let in_view = crate::rtownpc::check_camera_deg(c.pos, cloud::VIEW_DEG, &v.cam, v.player);
            if c.drawn(v.player, in_view) {
                out.push(Piece::Cloud { k });
            }
        }
        // LENSFLARE::Draw(se1_7_1, DMY_se1_7lig1point, 0).
        let sun_in_view = crate::rtownpc::check_camera_deg(self.sun, lensflare::VIEW_DEG, &v.cam, v.player);
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
        for k in 0..self.objects.len() {
            let o = &mut self.objects[k];
            if o.pass != pass {
                continue;
            }
            let d = ground_dist(ee::sub(eye[0], o.pos[0]), ee::sub(eye[1], o.pos[1]));
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
            let Some(mat) = self.file.ccs.find_object(t.mat) else { continue };
            let Some(m) = self.file.scene.materials.get(&mat) else { continue };
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
        draw::model_edited(layers, layer, to_screen, d, None, fogging);
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
                self.rng,
                self.flares,
            );
            let p = self.select(v);
            (self.move_tex, self.objects, self.bob, self.rising, self.clouds, self.rng, self.flares) = saved;
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
                        (&file, &self.morphers),
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

/// `sqrt(x x + y y)` in double, as `STATICOBJECT::Draw` measures.
fn ground_dist(dx: F, dy: F) -> F {
    let s = ee::add(ee::mul(dx, dx), ee::mul(dy, dy));
    (f64::from(ee::f(s)).sqrt() as f32).to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both blocks: the tables' pieces, one Hit chunk each, the start
    /// markers, the lights, the clouds of block 0.
    #[test]
    fn the_giants_blocks() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut g = Giant::new(&archive, 0, 0, 0).unwrap();
        let name = |f: &SceneFile, o: u32| f.ccs.object_name(o).unwrap().to_string();
        assert_eq!((g.models.len(), g.objects.len()), (8, 2));
        assert!(g.bg.iter().all(|b| b.is_some()));
        assert_eq!(g.clouds.len(), CLOUDS);
        assert_eq!(g.hits.models.len(), 1);
        assert_eq!(name(&g.file, g.hits.models[0].object), "HIT_se1_7ob1hit");
        assert_eq!(name(&g.file, g.hits.models[0].parent), "MDL_se1_7ob1_1");
        assert_eq!(g.light_objects.len(), 1);
        let arrive = g.start;
        let back = Giant::new(&archive, 0, 2, 0).unwrap().start;
        assert_ne!(arrive, back, "DMY_marker01 and 02 are different places");
        g.change_block(1, 1).unwrap();
        assert_eq!((g.models.len(), g.objects.len(), g.clouds.len()), (2, 0, 0));
        assert_eq!(g.hits.models.len(), 1);
        assert_eq!(name(&g.file_2, g.hits.models[0].object), "HIT_se1_7ob1_5hit");
        assert_eq!(g.light_objects.len(), 3);
        assert_eq!(g.enter(1, 1).unwrap(), Some(0));
        assert_eq!(g.block, 0);
        assert_eq!(g.enter(0, 1).unwrap(), None);
    }

    /// The bob stays within 0..350 and turns at both ends.
    #[test]
    fn the_background_bobs() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut g = Giant::new(&archive, 0, 0, 7).unwrap();
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
