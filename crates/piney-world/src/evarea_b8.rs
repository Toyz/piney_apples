//! The disc: `EVENTAREAB8` (gcmn areabk0.cpp; MUT 0x0041d200 constructor,
//! 0x0041ed30 `Draw`), the story map `WORLD_MAN::GO(1)` makes for fields
//! 9-12, Kyvia's stages. Fields 9-11 are `se1_6`: a sky of four clumps, 40
//! `FLOATROCK`s, and a disc (`ANM_se1_6ob1a`) that carries the party along
//! a path of the field's animation with `WORLD_MAN::SetTransMode` on.
//! Field 12 adds `se4_9`'s three pieces. docs/engine/evarea.md.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use piney_battle::rand::Rng as CcRand;
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::statics::{DrawPass, Position};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::cloud::{self, Cloud};
use crate::draw::{self, Draw};
use crate::ee::{self, F, ONE, V4};
use crate::evarea::{BOUNDS, StaticModel, StaticObject, StorySprite, clump_models, layer};
use crate::hit::{HitModel, Hits};
use crate::pose::Play;
use crate::story_map::StoryMap;
use crate::town::{FogParams, Light, TownLights, TownView};

/// `game.field` 9-12.
pub fn is_disc(field: i32) -> bool {
    (9..=12).contains(&field)
}

/// The scene file (+0x1a4), field 12's (+0x1d4, `bodyccs`) and `effccs`.
pub const FILE: &str = "se1_6";
pub const BODY_FILE: &str = "se4_9";
pub const EFF_FILE: &str = "town_z";

/// Fields 9-11: the disc's animation (`discAnm`, +0xcb0), the camera and
/// the dummy under it (`cam` +0x1cc, the path the disc follows) and the
/// constructor's `discCnt` (+0x1c4).
pub const DISCS: [(&str, &str, &str, i32); 3] = [
    ("ANM_se2_4_1_c", "CAM_camera2_4", "OBJ_dummy2_4", 2),
    ("ANM_se3_3_1_c", "CAM_camera3_3", "OBJ_dummy3_3", 0),
    ("ANM_se4_4_1_c", "CAM_camera4_4", "OBJ_dummy4_4", 0),
];

/// `bg[0..3]`, each on `bgLayer[k]` (+0x494..+0x4a0), fog off.
const BG: [(&str, i16); 4] = [
    ("CMP_se1_6bac1", layer::BG0),
    ("CMP_se1_6clo1_1", crate::evarea_b0::BG1),
    ("CMP_se1_6clo1_2", layer::BG2),
    ("CMP_se1_6moo1", layer::BG3),
];
/// The cloud layer's material, scrolled in V.
const SCROLL_MAT: &str = "MAT_se1_6clo1";
/// `DrawBG`'s scroll: its static starts at 1.0 and falls by 0.003
/// (0x3b449ba6) a frame, back to 1.0 below 0.
const SCROLL: F = 0x3b44_9ba6;

/// `rockname` (MUT 0x00600af0): a rock's clump, one by `fieldrand(6)`.
pub const ROCKS: [&str; 6] =
    ["CMP_se1_6ro1", "CMP_se1_6ro2", "CMP_se1_6ro3", "CMP_se1_6ro4", "CMP_se1_6ro5", "CMP_se1_6ro6"];
/// The rocks the constructor makes (`rock[40]`, +0x200).
pub const FLOAT_ROCKS: usize = 40;

/// `eventareaB8Light` (MUT 0x0038afc0): one distant light.
const LIGHTS: [(i32, &str); 1] = [(0, "LGT_se1_6lig1")];
const LIGHT_ANIM: &str = "ANM_se1_6bg1a";

/// `SetFog(6000, 20000, 0, 90, 0)`, and `ccSys.bgColor` 0.
pub const FOG: FogParams = FogParams { near: 6000.0, far: 20000.0, near_rate: 0.0, far_rate: 90.0, colour: [0; 3] };
pub const BG_COLOR: [u8; 3] = [0; 3];

/// The disc's marker, where its path is rooted (`discPos`, +0xc90).
const MARKER: &str = "DMY_marker01";
/// Field 12's start (the constructor's own, facing pi) and its three
/// pieces (`body[0..2]`, +0x1d8) at their places.
const START_12: V4 = [0x42a4_0000, 0xc62d_c800, 0x4463_8000, ee::PI];
const BODIES: [(&str, [F; 3]); 3] = [
    ("CMP_hit01", [0xc4c6_6000, 0xc60e_5c00, 0]),
    ("CMP_hit02", [0xc536_7ccd, 0xc5c2_84cd, 0]),
    ("CMP_hit03", [0xc3c3_599a, 0xc56e_4666, 0]),
];
const BODY_ANIM: &str = "ANM_se4_7_1a";

/// One `FLOATROCK` (0x40 bytes, areabk0.cpp): a rock rising slowly through
/// its cell of a 10 x 10 grid, fading in and out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FloatRock {
    /// +0x00, +0x04: its cell of `floatRockPos`.
    pub x: u32,
    pub y: u32,
    /// +0x08: frames since it started (fading in to 40).
    pub cnt: i32,
    /// +0x0c: frames left.
    pub life: i32,
    /// +0x10: its turn a frame, +0x14 the turn so far, +0x18 its rise.
    pub dirc: F,
    pub rot: F,
    pub speed: F,
    /// +0x20.
    pub pos: V4,
    /// +0x30: which of [`ROCKS`] it is.
    pub rock: usize,
}

/// One piece `EVENTAREAB8::Draw` draws, in its order, on its layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// `DrawBG`: `bg[k]` at the identity.
    Bg {
        k: usize,
        layer: i16,
    },
    /// `STATICMODEL::Draw` of `modelArray[k]`.
    Model {
        k: usize,
        layer: i16,
    },
    /// `STATICOBJECT::Draw` of `anmArray[k]`, stepped to `time`.
    Object {
        k: usize,
        layer: i16,
        time: u32,
    },
    /// `DrawRock`: rock `k` at its place and turn, `transparency`.
    Rock {
        k: usize,
        transparency: F,
    },
    /// Field 12: `bodyanm`'s `Draw`, then `body[k]`.
    BodyAnm {
        time: u32,
    },
    Body {
        k: usize,
    },
}

/// `EVENTAREAB8` as `WORLD_MAN.eventmap` holds it.
pub struct DiscArea {
    /// `game.field` (9-12).
    pub field: i32,
    pub file: Rc<SceneFile>,
    pub body_file: Option<Rc<SceneFile>>,
    pub eff: Rc<SceneFile>,
    morphers: HashMap<u32, u32>,
    /// `modelArray` (+0x78), `anmArray` (+0x140): `EA_MODELTABLEB8`,
    /// `EA_OBJTABLEB8` (one row each).
    pub models: Vec<StaticModel>,
    pub objects: Vec<StaticObject>,
    /// `bg[0..3]`: each clump's models.
    pub bg: [Option<(u32, Vec<u32>)>; 4],
    pub lights: TownLights,
    pub light_objects: Vec<u32>,
    /// `DrawBG`'s static scroll and the value the material last took.
    pub scroll_v: F,
    /// `discCnt` (+0x1c4): the stage the disc runs to; `move` (+0x1c8):
    /// 0 once it stopped there.
    pub disc_cnt: i32,
    pub moving: bool,
    /// `discAnm` (+0xcb0), the dummy it moves (`OBJ_dummy2_4` ...) and the
    /// camera (`cam` +0x1cc).
    pub disc: Option<Play>,
    pub dummy: Option<u32>,
    pub cam: Option<u32>,
    /// `discPos` (+0xc90): `DMY_marker01`; `discPrevPos` (+0xca0): where
    /// the disc was put last.
    pub disc_pos: V4,
    pub disc_prev: V4,
    /// `WORLD_MAN.eventStartPos` as the constructor left it: w the facing.
    pub start: V4,
    /// The 40 rocks (+0x200) and `floatRockPos` (MUT 0x0073eb00), the
    /// cells taken.
    pub rocks: Vec<FloatRock>,
    pub grid: [[u8; 10]; 10],
    /// The rocks' clumps' models, by [`ROCKS`].
    rock_models: Vec<Vec<u32>>,
    /// The 25 `CLOUD`s (+0xc00) of type 1: made, never moved or drawn.
    pub clouds: Vec<Cloud>,
    /// Field 12: `bodyanm` (+0x1f0) and `body[0..2]`.
    pub body_anm: Option<Play>,
    pub bodies: Vec<(u32, Vec<u32>, V4)>,
    /// `fieldrand` (the map's own; `GO(1)` sets no seed for a story map).
    pub rng: Rng,
    hit_models: Vec<HitModel>,
    pub hits: Hits,
}

/// `(float)(n)` of a `fieldrand` or `ccRand` result as the constructor
/// converts it: unsigned.
fn fu(n: u32) -> F {
    (n as f32).to_bits()
}

/// A glam matrix as the hits keep it: columns of bits.
fn cols(m: Mat4) -> [V4; 4] {
    let c = m.to_cols_array();
    std::array::from_fn(|k| std::array::from_fn(|j| c[4 * k + j].to_bits()))
}

/// `SetFloatRockParam(rock)` (MUT 0x0041cc80) in area `area`
/// (`WORLD_MAN.eventAreaNumber`, +0x120): a new life, rise and turn, and a
/// free cell. The centre of the grid is taken first (6 x 6 in fields
/// 10-12, else 4 x 4), so no rock rises through the disc's middle. The
/// place's own draws are made and then overwritten by the cell's.
pub fn set_float_rock_param(
    rock: &mut FloatRock,
    grid: &mut [[u8; 10]; 10],
    area: i32,
    rng: &mut Rng,
    cc: &mut dyn CcRand,
) {
    rock.cnt = 0;
    rock.rot = 0;
    rock.life = rng.below(150) as i32 + 90;
    rock.speed = fu(rng.below(20) + 5);
    let (lo, n, base, span) = if (10..=12).contains(&area) { (2, 6, 6000, 2000) } else { (3, 4, 2000, 6000) };
    for row in grid.iter_mut().skip(lo).take(n) {
        for c in row.iter_mut().skip(lo).take(n) {
            *c = 1;
        }
    }
    rock.pos[0] = fu(rng.below(span) + base);
    rock.pos[1] = fu(rng.below(span) + base);
    if rng.below(100) >= 51 {
        rock.pos[0] = ee::mul(rock.pos[0], 0xbf80_0000);
    }
    if rng.below(100) >= 51 {
        rock.pos[1] = ee::mul(rock.pos[1], 0xbf80_0000);
    }
    rock.pos[2] = fu(rng.below(7400) + 600);
    rock.pos[3] = ONE;
    loop {
        rock.x = (cc.rand() as u32) % 10;
        rock.y = (cc.rand() as u32) % 10;
        if grid[rock.x as usize][rock.y as usize] != 1 {
            break;
        }
    }
    grid[rock.x as usize][rock.y as usize] = 1;
    let cell = if area == 10 { 0x44bb_8000 } else { 0x4448_0000 };
    rock.pos[0] = ee::mul(cell, ee::sub(fu(rock.x), 0x40a0_0000));
    rock.pos[1] = ee::mul(cell, ee::sub(fu(rock.y), 0x40a0_0000));
    rock.dirc = ee::div(ee::from_int(rng.below(15) as i32), 0x447a_0000);
    if rng.below(100) >= 51 {
        rock.dirc = ee::mul(rock.dirc, 0xbf80_0000);
    }
}

impl DiscArea {
    /// `EVENTAREAB8::EVENTAREAB8` for `field` (9-12), `field_prev`
    /// `game.fieldPrev`; `fieldrand` from 0, `ccRand` the game's.
    pub fn new(
        archive: &Arc<Archive>,
        volume: Volume,
        field: i32,
        field_prev: i32,
        def_se: u32,
        cc: &mut dyn CcRand,
    ) -> Result<DiscArea> {
        if !is_disc(field) {
            return Err(Error::NotFound(format!("EVENTAREAB8: field {field}")));
        }
        let file = Rc::new(SceneFile::read(archive, FILE)?);
        let body_file = if field == 12 { Some(Rc::new(SceneFile::read(archive, BODY_FILE)?)) } else { None };
        let eff = Rc::new(SceneFile::read(archive, EFF_FILE)?);
        let (c, sc) = (&file.ccs, &file.scene);
        let (models_of, objs_of, _) = piney_data::statics::of(volume);
        let table = models_of
            .iter()
            .find(|t| t.name == "EA_MODELTABLEB8")
            .ok_or_else(|| Error::NotFound("EA_MODELTABLEB8".into()))?;
        let mut models = Vec::new();
        for (row, r) in table.rows.iter().enumerate() {
            let model = c.find_object(r.model).ok_or_else(|| Error::NotFound(format!("{FILE}: {}", r.model)))?;
            let pos = match r.position {
                Position::None => [0; 4],
                _ => crate::town::dummy_bits(&r.position, c, sc).0,
            };
            models.push(StaticModel { row, pass: r.pass, model, pos, clip: r.clip.to_bits() });
        }
        let otable = objs_of
            .iter()
            .find(|t| t.name == "EA_OBJTABLEB8")
            .ok_or_else(|| Error::NotFound("EA_OBJTABLEB8".into()))?;
        let mut objects = Vec::new();
        for (row, r) in otable.rows.iter().enumerate() {
            let name = r.anime.ok_or_else(|| Error::NotFound(format!("EA_OBJTABLEB8: row {row}")))?;
            let play = Play::new(&file, name).ok_or_else(|| Error::NotFound(name.into()))?;
            let (pos, rot) = crate::town::dummy_bits(&r.position, c, sc);
            let root = crate::town::pos_rot_zyx(pos, rot);
            objects.push(StaticObject { row, pass: r.pass, play, root, pos, clip: r.clip.to_bits() });
        }
        let mut bg: [Option<(u32, Vec<u32>)>; 4] = Default::default();
        for (k, (name, _)) in BG.iter().enumerate() {
            bg[k] = Some(clump_models(&file, name).ok_or_else(|| Error::NotFound(format!("{FILE}: {name}")))?);
        }
        // The disc's animation, stepped once.
        let (disc, disc_cnt, cam, dummy) = match DISCS.get((field - 9) as usize) {
            Some(&(anim, cam, dummy, cnt)) => {
                let mut p = Play::new(&file, anim).ok_or_else(|| Error::NotFound(anim.into()))?;
                p.forward(&file);
                (Some(p), cnt, c.find_object(cam), c.find_object(dummy))
            }
            None => (None, 0, None, None),
        };
        // lgtAnm: its light at frame 1 and the ambient.
        let ai = file.anim(LIGHT_ANIM).ok_or_else(|| Error::NotFound(LIGHT_ANIM.into()))?;
        let (ambient, records) = crate::town::anim_lights(&file, ai)?;
        let mut group: Vec<Light> = Vec::new();
        let mut light_objects = Vec::new();
        for (_, name) in LIGHTS {
            let obj = c.find_object(name).ok_or_else(|| Error::NotFound(name.into()))?;
            light_objects.push(obj);
            if let Some(l) = records.iter().find(|l| l.object == obj) {
                group.push(l.at(256));
            }
        }
        let lights = TownLights { ambient: ambient.unwrap_or(Vec3::ZERO), lights: group, fog: Some(FOG.depth()) };
        let marker = c.find_object(MARKER).ok_or_else(|| Error::NotFound(format!("{FILE}: {MARKER}")))?;
        let disc_pos = sc
            .dummies
            .get(&marker)
            .map_or([0, 0, 0, ONE], |d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE]);
        let hit_models = HitModel::read(c)?;
        let hits = Hits { area: 1, bounds: Some(BOUNDS), heights: None, def_se, event_area: true, ..Hits::default() };
        let mut rock_models = Vec::new();
        for name in ROCKS {
            rock_models.push(clump_models(&file, name).ok_or_else(|| Error::NotFound(name.into()))?.1);
        }
        let pat_num = cloud::eff_pat_num(&eff.ccs, cloud::EFF_SMOKE)
            .ok_or_else(|| Error::NotFound(format!("{EFF_FILE}: {}", cloud::EFF_SMOKE)))?;
        let morphers = piney_data::anim::morphers(c).unwrap_or_default();
        let mut a = DiscArea {
            field,
            file: file.clone(),
            body_file,
            eff,
            morphers,
            models,
            objects,
            bg,
            lights,
            light_objects,
            scroll_v: ONE,
            disc_cnt,
            moving: true,
            disc,
            dummy,
            cam,
            disc_pos,
            disc_prev: [0; 4],
            start: [0; 4],
            rocks: Vec::new(),
            grid: [[0; 10]; 10],
            rock_models,
            clouds: Vec::new(),
            body_anm: None,
            bodies: Vec::new(),
            rng: Rng::new(0),
            hit_models,
            hits,
        };
        // Back in the field it left (fieldPrev), fields 9 and 11 start
        // with the disc at the end of its run.
        if field == field_prev
            && let Some(p) = a.disc.as_mut()
        {
            let end = match field {
                9 => Some(0x1_a300),
                11 => Some(0x9_2300),
                _ => None,
            };
            if let Some(t) = end {
                p.time = t;
                p.posed = t;
                p.forward(&file);
            }
        }
        // The start: the path's dummy (its world translation, w 1) plus
        // the marker (w 1 too), so the leader faces 2.0; field 12's own.
        if field == 12 {
            a.start = START_12;
            let bf = a.body_file.clone().expect("se4_9");
            let bhits = HitModel::read(&bf.ccs)?;
            for (name, at) in BODIES {
                let (clump, models) =
                    clump_models(&bf, name).ok_or_else(|| Error::NotFound(format!("{BODY_FILE}: {name}")))?;
                let pos = [at[0], at[1], at[2], ONE];
                for h in bhits.iter().filter(|h| models.contains(&h.parent)) {
                    let mut h = h.clone();
                    h.rm = crate::hit::UNIT;
                    h.rm[3] = pos;
                    h.im = crate::dungeon_area::invers(&h.rm);
                    a.hits.models.push(h);
                }
                a.bodies.push((clump, models, pos));
            }
            let play = Play::new(&bf, BODY_ANIM).ok_or_else(|| Error::NotFound(BODY_ANIM.into()))?;
            a.body_anm = Some(play);
        } else {
            let t = a.path_point();
            a.start = [ee::add(t[0], disc_pos[0]), ee::add(t[1], disc_pos[1]), ee::add(t[2], disc_pos[2]), 0x4000_0000];
            // anmArray[0]'s anm: HitEnable, SetHitMatrix at its root.
            a.disc_hits();
        }
        for _ in 0..FLOAT_ROCKS {
            let mut r = FloatRock::default();
            set_float_rock_param(&mut r, &mut a.grid, field, &mut a.rng, cc);
            r.rock = a.rng.below(ROCKS.len() as u32) as usize;
            a.rocks.push(r);
        }
        for _ in 0..25 {
            a.clouds.push(Cloud::init(1, pat_num, [0, 0, 0, ONE], &mut a.rng));
        }
        if field != 12 {
            // SetTransCenter(the start), SetTransDiff(0).
            a.disc_prev = a.start;
            a.hits.trans = Some(a.start);
        }
        Ok(a)
    }

    /// The path's dummy as the disc's animation poses it: its world
    /// translation (w 1).
    fn path_point(&self) -> V4 {
        let (Some(p), Some(d)) = (&self.disc, self.dummy) else { return [0, 0, 0, ONE] };
        let w = p.worlds(&self.file, Mat4::IDENTITY, &[d]);
        let t = w.get(&d).map_or(Vec3::ZERO, |m| m.w_axis.truncate());
        [t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), ONE]
    }

    /// `ccAnm::SetHitMatrix` of the disc (`anmArray[0]`): each Hit chunk
    /// under an object the animation poses takes that object's world
    /// matrix under the object's root, and its inverse.
    fn disc_hits(&mut self) {
        let Some(o) = self.objects.first() else { return };
        let worlds = o.play.worlds(&self.file, draw::mat(&o.root), &[]);
        let mut list = Vec::new();
        for (&obj, &m) in &worlds {
            let target = self.file.scene.ext.get(&obj).copied().unwrap_or(obj);
            let Some(&model) = self.file.obj_model.get(&target) else { continue };
            for h in self.hit_models.iter().filter(|h| h.parent == model) {
                let mut h = h.clone();
                h.rm = cols(m);
                h.im = crate::dungeon_area::invers(&h.rm);
                list.push(h);
            }
        }
        list.sort_by_key(|h| h.object);
        self.hits.models = list;
    }

    /// The disc's step by field and `discCnt` (in `Draw` and in the part
    /// it calls while the party rides): field 9 to its animation's end;
    /// field 10 to frame 420, 800, then the end; field 11 to 400, 1270,
    /// 2340, then the end. `move` clears where it stops.
    fn disc_step(&mut self) {
        self.moving = true;
        let Some(p) = self.disc.as_mut() else { return };
        let frame = p.time >> 8;
        let stop = match (self.field, self.disc_cnt) {
            (10, 0) => Some(420),
            (10, 1) => Some(800),
            (11, 0) => Some(400),
            (11, 1) => Some(1270),
            (11, 2) => Some(2340),
            (10, 2) | (11, 3) | (9, _) => None,
            _ => return,
        };
        match stop {
            Some(s) if frame >= s => self.moving = false,
            Some(_) => {
                p.forward(&self.file);
            }
            None => {
                if p.forward(&self.file) {
                    self.moving = false;
                }
            }
        }
    }

    /// `EVENTAREAB8::IsMove` (MUT 0x0041e7d0): `move`.
    pub fn is_move(&self) -> bool {
        self.moving
    }

    /// `EVENTAREAB8::Move` (MUT 0x0041e750), from Kyvia's `CheckDiscMove`:
    /// in fields 10 and 11 the next stage (`discCnt` + 1, held at 2 and 3).
    pub fn next_stage(&mut self) {
        if self.field == 9 {
            return;
        }
        self.disc_cnt += 1;
        if self.field == 10 && self.disc_cnt == 3 {
            self.disc_cnt = 2;
        }
        if self.field == 11 && self.disc_cnt == 4 {
            self.disc_cnt = 3;
        }
    }

    /// The part of `Draw` MUT 0x0041e8c0 does while the party rides (its
    /// trans mode on): the disc steps again, is put where the path was
    /// last frame (`STATICOBJECT::SetPos`, its hits after it), that place
    /// becomes the party's centre (`SetTransCenter`), and the path's point
    /// now (w 1) is kept for the next.
    fn move_disc(&mut self) {
        self.disc_step();
        if self.field == 12 {
            return;
        }
        let t = self.path_point();
        let p = self.disc_pos;
        let pos = [ee::add(t[0], p[0]), ee::add(t[1], p[1]), ee::add(t[2], p[2]), ONE];
        let prev = self.disc_prev;
        if let Some(o) = self.objects.first_mut() {
            o.pos = prev;
            o.root = crate::town::pos_rot_zyx(prev, [0; 3]);
        }
        self.disc_hits();
        self.hits.trans = Some(prev);
        self.disc_prev = pos;
    }

    /// `WORLD_MAN::GetTransCenter` while the party rides.
    pub fn trans_center(&self) -> Option<V4> {
        self.hits.trans
    }

    /// `DrawRock` (MUT 0x0041f130) for rock `k`: its fade (in over 40
    /// frames, out over its last 40), turn and rise, then drawn when
    /// `ccCheckCameraDeg(pos, 12288)`; at the end of its life its cell is
    /// freed and it starts again.
    fn rock_step(&mut self, k: usize, v: &TownView, cc: &mut dyn CcRand, out: &mut Vec<Piece>) {
        let r = &mut self.rocks[k];
        let mut tr = None;
        if r.cnt != 40 {
            r.cnt += 1;
            tr = Some(ee::mul(ee::from_int(r.cnt), 0x3ccc_cccd));
        }
        if r.life < 40 {
            tr = Some(ee::mul(ee::from_int(r.life), 0x3ccc_cccd));
        }
        r.rot = piney_battle::gimmick::cc_rotate(r.rot, r.dirc);
        r.pos[2] = ee::add(r.pos[2], r.speed);
        r.life -= 1;
        if crate::rtownpc::check_camera_deg(r.pos, cloud::VIEW_DEG, &v.cam, v.player) {
            out.push(Piece::Rock { k, transparency: tr.unwrap_or(ONE) });
        }
        if r.life == 0 {
            let (x, y) = (r.x as usize, r.y as usize);
            self.grid[x][y] = 0;
            let field = self.field;
            let mut r = self.rocks[k];
            set_float_rock_param(&mut r, &mut self.grid, field, &mut self.rng, cc);
            self.rocks[k] = r;
        }
    }

    /// `EVENTAREAB8::Draw` (MUT 0x0041ed30) for this frame: the pieces in
    /// its order, with what it steps on the way.
    pub fn select(&mut self, v: &TownView, cc: &mut dyn CcRand) -> Vec<Piece> {
        let mut out = Vec::new();
        if self.field != 12 {
            // DrawBG: the scroll steps, then the four clumps.
            self.scroll_v = ee::sub(self.scroll_v, SCROLL);
            if ee::lt(self.scroll_v, 0) {
                self.scroll_v = ONE;
            }
            for (k, &(_, layer)) in BG.iter().enumerate() {
                out.push(Piece::Bg { k, layer });
            }
            // effLayer: DrawObj, the type-2 models, then the objects.
            self.row_pieces(&mut out, DrawPass::Obj, layer::EFF, v.eye);
        }
        if self.field != 12 {
            if self.field != 11 {
                for k in 0..self.rocks.len() {
                    self.rock_step(k, v, cc, &mut out);
                }
            }
            self.row_pieces(&mut out, DrawPass::Floor, layer::OBJ, v.eye);
        }
        self.disc_step();
        if self.field == 12 {
            let bf = self.body_file.clone();
            if let (Some(p), Some(bf)) = (self.body_anm.as_mut(), bf) {
                p.forward(&bf);
                out.push(Piece::BodyAnm { time: p.time });
            }
            for k in 0..self.bodies.len() {
                out.push(Piece::Body { k });
            }
        } else {
            self.move_disc();
        }
        out
    }

    /// `DrawObj` or `DrawFloor`: the models of `pass`, then its static
    /// objects (each stepped and drawn within its clip of the eye).
    fn row_pieces(&mut self, out: &mut Vec<Piece>, pass: DrawPass, layer: i16, eye: V4) {
        for (k, m) in self.models.iter().enumerate() {
            if m.pass == pass {
                out.push(Piece::Model { k, layer });
            }
        }
        for k in 0..self.objects.len() {
            let o = &mut self.objects[k];
            if o.pass != pass {
                continue;
            }
            let (dx, dy) = (ee::sub(eye[0], o.pos[0]), ee::sub(eye[1], o.pos[1]));
            let d = ee::sqrtf(ee::add(ee::mul(dx, dx), ee::mul(dy, dy)));
            if o.clip == 0 || ee::lt(d, o.clip) {
                o.play.forward(&self.file);
                out.push(Piece::Object { k, layer, time: o.play.time });
            }
        }
    }

    /// `MAT_se1_6clo1`'s offset as `DrawBG` writes it (`vftoi12(v)` less
    /// the material's crop, in V).
    fn scroll_rows(&self) -> HashMap<u32, [u8; 2]> {
        let mut rows = HashMap::new();
        if let Some(mat) = self.file.ccs.find_object(SCROLL_MAT)
            && let Some(m) = self.file.scene.materials.get(&mat)
        {
            let v = (ee::to_int(ee::mul(self.scroll_v, 0x4580_0000)) & 0xffff) as u16;
            rows.insert(mat, [0, (v.wrapping_sub(m.crop_v) >> 4) as u8]);
        }
        rows
    }

    #[allow(clippy::too_many_arguments)]
    fn model_draw(&self, layers: &mut Layers, layer: i16, to_screen: Mat4, file: &SceneFile, model: u32, world: Mat4) {
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
        draw::model(layers, layer, to_screen, d);
    }

    /// The pieces into the field's layers.
    fn draw_pieces(&self, layers: &mut Layers, to_screen: Mat4, pieces: Vec<Piece>) {
        let file = &*self.file;
        for p in pieces {
            match p {
                Piece::Bg { k, layer } => {
                    let Some((_, models)) = &self.bg[k] else { continue };
                    for &m in models {
                        self.model_draw(layers, layer, to_screen, file, m, Mat4::IDENTITY);
                    }
                }
                Piece::Model { k, layer } => {
                    let m = &self.models[k];
                    let world = Mat4::from_translation(Vec3::new(ee::f(m.pos[0]), ee::f(m.pos[1]), ee::f(m.pos[2])));
                    self.model_draw(layers, layer, to_screen, file, m.model, world);
                }
                Piece::Object { k, layer, .. } => {
                    let o = &self.objects[k];
                    crate::town::anim_draw_fog(
                        layers,
                        layer,
                        to_screen,
                        (file, &self.morphers),
                        &o.play,
                        draw::mat(&o.root),
                        &[],
                        1.0,
                        None,
                        None,
                        draw::Fogging::Depth(FOG.depth()),
                    );
                }
                Piece::Rock { k, transparency } => {
                    let r = &self.rocks[k];
                    let world = Mat4::from_translation(Vec3::new(ee::f(r.pos[0]), ee::f(r.pos[1]), ee::f(r.pos[2])))
                        * Mat4::from_rotation_x(ee::f(r.rot));
                    let alpha = ee::f(transparency).clamp(0.0, 1.0);
                    for &m in &self.rock_models[r.rock] {
                        let rows = HashMap::new();
                        let d = Draw {
                            file,
                            model: m,
                            world,
                            alpha,
                            rows: &rows,
                            lights: None,
                            nodes: &[],
                            morph: Vec::new(),
                            clut_swaps: Vec::new(),
                        };
                        draw::model(layers, layer::OBJ, to_screen, d);
                    }
                }
                Piece::BodyAnm { .. } => {
                    let (Some(p), Some(bf)) = (&self.body_anm, &self.body_file) else { continue };
                    let morphers = piney_data::anim::morphers(&bf.ccs).unwrap_or_default();
                    crate::town::anim_draw_fog(
                        layers,
                        layer::OBJ,
                        to_screen,
                        (bf, &morphers),
                        p,
                        Mat4::IDENTITY,
                        &[],
                        1.0,
                        None,
                        None,
                        draw::Fogging::Depth(FOG.depth()),
                    );
                }
                Piece::Body { k } => {
                    let Some(bf) = &self.body_file else { continue };
                    let (_, models, pos) = &self.bodies[k];
                    let world = Mat4::from_translation(Vec3::new(ee::f(pos[0]), ee::f(pos[1]), ee::f(pos[2])));
                    for &m in models {
                        self.model_draw(layers, layer::OBJ, to_screen, bf, m, world);
                    }
                }
            }
        }
    }

    /// The Hit chunks of the stage, for a check.
    pub fn hit_model_count(&self) -> usize {
        self.hit_models.len()
    }
}

impl StoryMap for DiscArea {
    fn hits(&self) -> &Hits {
        &self.hits
    }
    fn hits_mut(&mut self) -> &mut Hits {
        &mut self.hits
    }
    fn lights(&self) -> &TownLights {
        &self.lights
    }
    /// `WORLD_MAN::SetCharPosition` for fields 9-11 (the default): the
    /// leader at the start facing its w, the others at (+300, +150),
    /// (-300, +150) and (0, +300); field 12's own places.
    fn start_positions(&self) -> (V4, F, [V4; 3]) {
        let [x, y, z, dirc] = self.start;
        if self.field == 12 {
            let z = 0x4463_8000;
            let starts = [
                [0xc258_0000, 0xc634_1400, z, ONE],
                [0x4380_0000, 0xc633_fc00, z, ONE],
                [x, ee::add(0x4396_0000, y), z, ONE],
            ];
            return ([x, y, z, ONE], dirc, starts);
        }
        let starts = [
            [ee::add(0x4396_0000, x), ee::add(0x4316_0000, y), z, ONE],
            [ee::sub(x, 0x4396_0000), ee::add(0x4316_0000, y), z, ONE],
            [x, ee::add(0x4396_0000, y), z, ONE],
        ];
        ([x, y, z, ONE], dirc, starts)
    }
    fn clear(&self) -> [u8; 3] {
        BG_COLOR
    }
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite> {
        let mut cc = piney_battle::rand::Genrand::default();
        self.draw_rand(layers, to_screen, v, step, &mut cc)
    }
    fn draw_rand(
        &mut self,
        layers: &mut Layers,
        to_screen: Mat4,
        v: &TownView,
        step: bool,
        cc: &mut dyn CcRand,
    ) -> Vec<StorySprite> {
        let pieces = if step {
            self.select(v, cc)
        } else {
            // A frame the tasks sleep through: drawn as it stands.
            let mut out: Vec<Piece> = BG.iter().enumerate().map(|(k, &(_, layer))| Piece::Bg { k, layer }).collect();
            if self.field != 12 {
                out.extend((0..self.objects.len()).map(|k| Piece::Object { k, layer: layer::EFF, time: 0 }));
                out.extend((0..self.models.len()).map(|k| Piece::Model { k, layer: layer::OBJ }));
            }
            out
        };
        self.draw_pieces(layers, to_screen, pieces);
        Vec::new()
    }
    fn trans_center(&self) -> Option<V4> {
        self.hits.trans
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piney_battle::rand::Genrand;

    fn archive() -> Option<Arc<Archive>> {
        let p = std::path::Path::new("../../work/mutation/mutation.iso");
        if !p.exists() {
            eprintln!("skipped: no {}", p.display());
            return None;
        }
        let mut iso = piney_data::iso::Iso::open(p).ok()?;
        Some(Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").ok()?).ok()?))
    }

    /// Field 9, Kyvia's first stage: the floor and the disc, the four sky
    /// clumps, the light, the disc's collision under it, the 40 rocks on
    /// cells of their own outside the centre, the 25 clouds, the start on
    /// the path facing 2.0, and the party's centre on.
    #[test]
    fn the_disc_of_field_9() {
        let Some(archive) = archive() else { return };
        let mut cc = Genrand::default();
        let a = DiscArea::new(&archive, Volume::Mut, 9, 0, 0, &mut cc).unwrap();
        let name = |o: u32| a.file.ccs.object_name(o).unwrap().to_string();
        assert_eq!(a.models.len(), 1);
        assert_eq!(name(a.models[0].model), "MDL_se1_6flo1");
        assert_eq!(a.models[0].pass, DrawPass::Floor);
        assert_eq!((a.objects.len(), a.objects[0].pass), (1, DrawPass::Obj));
        assert!(a.bg.iter().all(|b| b.is_some()));
        assert_eq!(a.light_objects.len(), 1);
        assert_eq!(a.disc_cnt, 2);
        assert!(a.disc.is_some() && a.dummy.is_some() && a.cam.is_some());
        assert!(!a.hits.models.is_empty(), "the disc's Hit chunk");
        assert!(a.hits.models.iter().all(|h| name(h.object) == "HIT_se1_6ob1hit"));
        assert_eq!(a.rocks.len(), FLOAT_ROCKS);
        let cells: std::collections::BTreeSet<(u32, u32)> = a.rocks.iter().map(|r| (r.x, r.y)).collect();
        assert_eq!(cells.len(), FLOAT_ROCKS, "a cell each");
        assert!(cells.iter().all(|&(x, y)| !((3..7).contains(&x) && (3..7).contains(&y))));
        assert!(a.rocks.iter().all(|r| (90..240).contains(&r.life) && r.cnt == 0));
        assert_eq!(a.clouds.len(), 25);
        assert_eq!(a.start[3], 0x4000_0000);
        assert_eq!(a.trans_center(), Some(a.start));
        let (leader, facing, others) = a.start_positions();
        assert_eq!(facing, 0x4000_0000);
        assert_eq!(others[2][1], ee::add(0x4396_0000, leader[1]));
    }

    /// The ride: each frame the disc steps (twice: `Draw`'s and the
    /// ride's), lands where the path was the frame before, its hits and
    /// the party's centre with it; the rocks fade in, rise and start again
    /// on a free cell; the run ends at the animation's end (`move` 0).
    #[test]
    fn the_disc_rides_to_its_end() {
        let Some(archive) = archive() else { return };
        let mut cc = Genrand::default();
        let mut a = DiscArea::new(&archive, Volume::Mut, 9, 0, 0, &mut cc).unwrap();
        let v = TownView::default();
        let first = a.trans_center().unwrap();
        let mut prev = a.disc_prev;
        let mut frames = 0;
        let mut restarted = 0;
        while a.is_move() && frames < 5000 {
            let lives: Vec<i32> = a.rocks.iter().map(|r| r.life).collect();
            let pieces = a.select(&v, &mut cc);
            frames += 1;
            assert_eq!(a.trans_center(), Some(prev), "the centre lags the path by a frame");
            assert_eq!(a.objects[0].pos, prev);
            prev = a.disc_prev;
            restarted += lives.iter().filter(|&&l| l == 1).count();
            assert!(pieces.iter().any(|p| matches!(p, Piece::Object { .. })));
            assert!(pieces.iter().filter(|p| matches!(p, Piece::Bg { .. })).count() == 4);
        }
        assert!(!a.is_move(), "the run ends");
        assert!(frames > 10, "{frames}");
        assert_ne!(a.trans_center().unwrap(), first, "the disc moved");
        assert!(restarted > 0, "a rock lived out its life");
        let cells: std::collections::BTreeSet<(u32, u32)> = a.rocks.iter().map(|r| (r.x, r.y)).collect();
        assert_eq!(cells.len(), FLOAT_ROCKS);
        assert!(a.rocks.iter().all(|r| a.grid[r.x as usize][r.y as usize] == 1));
    }

    /// Fields 10 and 11 stop at their stages until Kyvia's `Move`.
    #[test]
    fn the_later_discs_stop_at_their_stages() {
        let Some(archive) = archive() else { return };
        let mut cc = Genrand::default();
        let mut a = DiscArea::new(&archive, Volume::Mut, 10, 0, 0, &mut cc).unwrap();
        let v = TownView::default();
        let mut n = 0;
        while a.is_move() && n < 2000 {
            a.select(&v, &mut cc);
            n += 1;
        }
        assert!(!a.is_move());
        assert!((a.disc.as_ref().unwrap().time >> 8) >= 420);
        a.next_stage();
        assert_eq!(a.disc_cnt, 1);
        a.select(&v, &mut cc);
        assert!(a.is_move(), "on to 800");
        a.next_stage();
        a.next_stage();
        assert_eq!(a.disc_cnt, 2, "held at the last stage");
    }
}
