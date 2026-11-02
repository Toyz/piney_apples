//! The disc: `EVENTAREAB8` (gcmn areabk0.cpp; MUT 0x0041d200 constructor,
//! 0x0041ed30 `Draw`), the story map `WORLD_MAN::GO(1)` makes for fields
//! 9-12, Kyvia's stages. Fields 9-11 are `se1_6`: a sky of four clumps, 40
//! `FLOATROCK`s, and a disc (`ANM_se1_6ob1a`) that carries the party along
//! a path of the field's animation with `WORLD_MAN::SetTransMode` on.
//! Field 12 adds `se4_9`'s three pieces. docs/engine/evarea.md.

use std::borrow::Cow;
use std::rc::Rc;

use glam::{Mat4, Vec3};
use piney_battle::rand::Rng as CcRand;
use piney_data::classes::StoryMapGameClass;
use piney_data::dungeon::Rng;
use piney_data::statics::DrawPass;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use crate::chara::pose::Play;
use crate::draw::{self, Fogging};
use crate::ee::{self, F, ONE, V4};
use crate::hit::HitModel;
use crate::sky::cloud;
use crate::story_map::{BASE_BLOCK, BASE_STORY_MAP, BlockDef, LightsDef, Spread, Start, StoryMapDef, layer};
use crate::systems::clouds::{CloudsDef, Made};
use crate::systems::scrolls::{MatScroll, ScrollsDef, Shown, Wrap};
use crate::systems::sky::{SkyClump, SkyDef};
use crate::systems::statics::{ModelsDef, ObjectsDef, PassLayers, StaticObjects};
use crate::systems::{
    BuildCx, DrawCx, Fog, Hook, Kind, MapSystem, Others, Piece, PlaceCx, Report, Request, Signal, Step, StepCx,
    SystemDef,
};
use crate::town::{FogParams, clump_models};

/// The scene file (+0x1a4) and field 12's (+0x1d4, `bodyccs`).
pub const FILE: &str = "se1_6";
pub const BODY_FILE: &str = "se4_9";

/// `modelArray` and `anmArray`: the whole of `EA_MODELTABLEB8` and
/// `EA_OBJTABLEB8`, at their dummies; `DrawObj` on effLayer, `DrawFloor`
/// on objLayer. The models have no fog.
const LAYERS: PassLayers = PassLayers { obj: layer::EFF, ..PassLayers::all(layer::OBJ) };
static MODELS: ModelsDef = ModelsDef { layers: LAYERS, fog: Fog::None, ..ModelsDef::on("EA_MODELTABLEB8", layer::OBJ) };
static OBJECTS: ObjectsDef = ObjectsDef { layers: LAYERS, ..ObjectsDef::on("EA_OBJTABLEB8", layer::OBJ) };

/// `bg[0..3]`, each on `bgLayer[k]` (+0x494..+0x4a0), fog off.
static SKY: SkyDef = SkyDef {
    clumps: &[
        SkyClump::on("CMP_se1_6bac1", layer::BG0),
        SkyClump::on("CMP_se1_6clo1_1", layer::BG1),
        SkyClump::on("CMP_se1_6clo1_2", layer::BG2),
        SkyClump::on("CMP_se1_6moo1", layer::BG3),
    ],
};

/// `DrawBG`'s scroll: its static starts at 1.0 and falls by 0.003
/// (0x3b449ba6) a frame, back to 1.0 below 0; the cloud layer's V.
static SCROLLS: ScrollsDef = ScrollsDef {
    steps: &[0x3b44_9ba6],
    wrap: Wrap::Under,
    from: ONE,
    shown: Shown::After,
    carry: None,
    mats: &[MatScroll::v("MAT_se1_6clo1", 0)],
};

/// `rockname` (MUT 0x00600af0): a rock's clump, one by `fieldrand(6)`.
pub const ROCKS: [&str; 6] =
    ["CMP_se1_6ro1", "CMP_se1_6ro2", "CMP_se1_6ro3", "CMP_se1_6ro4", "CMP_se1_6ro5", "CMP_se1_6ro6"];
/// The rocks the constructor makes (`rock[40]`, +0x200).
pub const FLOAT_ROCKS: usize = 40;

/// The 25 `CLOUD`s (+0xc00) of type 1 the constructor makes after the
/// rocks: never moved or drawn.
static CLOUDS: CloudsDef = CloudsDef { count: 25, kind: 1, made: Made::Build };

/// `eventareaB8Light` (MUT 0x0038afc0): one distant light.
const LIGHTS: LightsDef = LightsDef { anim: "ANM_se1_6bg1a", table: &["LGT_se1_6lig1"], skip_missing: false };

/// `SetFog(6000, 20000, 0, 90, 0)`, and `ccSys.bgColor` 0.
pub const FOG: FogParams = FogParams { near: 6000.0, far: 20000.0, near_rate: 0.0, far_rate: 90.0, colour: [0; 3] };
pub const BG_COLOR: [u8; 3] = [0; 3];

/// The disc's marker, where its path is rooted (`discPos`, +0xc90).
const MARKER: &str = "DMY_marker01";

/// Field 12's start (the constructor's own, facing pi), where its two
/// others stand at height 900, and its three pieces (`body[0..2]`,
/// +0x1d8) at their places.
const START_12: V4 = [0x42a4_0000, 0xc62d_c800, 0x4463_8000, ee::PI];
const BODIES: [(&str, [F; 3]); 3] = [
    ("CMP_hit01", [0xc4c6_6000, 0xc60e_5c00, 0]),
    ("CMP_hit02", [0xc536_7ccd, 0xc5c2_84cd, 0]),
    ("CMP_hit03", [0xc3c3_599a, 0xc56e_4666, 0]),
];
const BODY_ANIM: &str = "ANM_se4_7_1a";

/// The disc's path in a field: its animation (`discAnm`, +0xcb0), the
/// dummy it moves (`OBJ_dummy2_4` ...; the camera `cam` +0x1cc follows the
/// same path), the constructor's `discCnt` (+0x1c4), the frame the disc
/// stops at for each `discCnt` (past them, the animation's end), and where
/// it starts back in the field it left (`fieldPrev`): its run's end.
#[derive(Clone, Copy, Debug)]
pub struct Path {
    pub anim: &'static str,
    pub dummy: &'static str,
    pub cnt: i32,
    pub stops: &'static [u32],
    pub back_at: Option<u32>,
}

/// A field's disc: its path, none in field 12.
#[derive(Clone, Copy, Debug)]
pub struct DiscDef {
    pub path: Option<Path>,
}

static PATH_9: DiscDef = DiscDef {
    path: Some(Path { anim: "ANM_se2_4_1_c", dummy: "OBJ_dummy2_4", cnt: 2, stops: &[], back_at: Some(0x1_a300) }),
};
static PATH_10: DiscDef = DiscDef {
    path: Some(Path { anim: "ANM_se3_3_1_c", dummy: "OBJ_dummy3_3", cnt: 0, stops: &[420, 800], back_at: None }),
};
static PATH_11: DiscDef = DiscDef {
    path: Some(Path {
        anim: "ANM_se4_4_1_c",
        dummy: "OBJ_dummy4_4",
        cnt: 0,
        stops: &[400, 1270, 2340],
        back_at: Some(0x9_2300),
    }),
};
static NO_PATH: DiscDef = DiscDef { path: None };

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

/// `EVENTAREAB8`'s own: the disc (`anmArray[0]`) and the path it runs,
/// `discCnt` (+0x1c4), `move` (+0x1c8, 0 once it stopped), `discPos`
/// (+0xc90) and `discPrevPos` (+0xca0).
#[derive(Clone)]
pub struct Disc {
    def: &'static DiscDef,
    file: Rc<SceneFile>,
    pub cnt: i32,
    pub moving: bool,
    pub play: Option<Play>,
    dummy: Option<u32>,
    pub disc_pos: V4,
    pub disc_prev: V4,
    hit_models: Vec<HitModel>,
}

impl Disc {
    /// The constructor's: the disc's animation stepped once (at its run's
    /// end back in the field it left), the marker.
    pub fn new(def: &'static DiscDef, cx: &mut BuildCx) -> Result<Disc> {
        let file = cx.file.clone();
        let (c, sc) = (&file.ccs, &file.scene);
        let back = cx.map.is_some_and(|m| m.field == m.field_prev);
        let (play, dummy, cnt) = match def.path {
            Some(p) => {
                let mut play = Play::new(&file, p.anim).ok_or_else(|| Error::NotFound(p.anim.into()))?;
                play.forward(&file);
                if let Some(t) = p.back_at.filter(|_| back) {
                    play.time = t;
                    play.posed = t;
                    play.forward(&file);
                }
                (Some(play), c.find_object(p.dummy), p.cnt)
            }
            None => (None, None, 0),
        };
        let marker = c.find_object(MARKER).ok_or_else(|| Error::NotFound(format!("{FILE}: {MARKER}")))?;
        let disc_pos = sc
            .dummies
            .get(&marker)
            .map_or([0, 0, 0, ONE], |d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE]);
        let hit_models = HitModel::read(cx.volume, &file.ccs)?;
        Ok(Disc { def, file, cnt, moving: true, play, dummy, disc_pos, disc_prev: [0; 4], hit_models })
    }

    /// The path's dummy as the disc's animation poses it: its world
    /// translation (w 1).
    fn path_point(&self) -> V4 {
        let (Some(p), Some(d)) = (&self.play, self.dummy) else { return [0, 0, 0, ONE] };
        let w = p.worlds(&self.file, Mat4::IDENTITY, &[d]);
        let t = w.get(&d).map_or(Vec3::ZERO, |m| m.w_axis.truncate());
        [t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), ONE]
    }

    /// `ccAnm::SetHitMatrix` of the disc (`anmArray[0]`): each Hit chunk
    /// under an object the animation poses takes that object's world
    /// matrix under the object's root, and its inverse.
    fn disc_hits(&self, objects: &StaticObjects) -> Vec<HitModel> {
        let Some(o) = objects.get(0) else { return Vec::new() };
        let worlds = o.play.worlds(&self.file, draw::mat(&o.root), &[]);
        let mut list = Vec::new();
        for (&obj, &m) in &worlds {
            let target = self.file.scene.ext.get(&obj).copied().unwrap_or(obj);
            let Some(&model) = self.file.obj_model.get(&target) else { continue };
            for h in self.hit_models.iter().filter(|h| h.parent == model) {
                let mut h = h.clone();
                h.rm = cols(m);
                h.im = crate::dungeon::invers(&h.rm);
                list.push(h);
            }
        }
        list.sort_by_key(|h| h.object);
        list
    }

    /// The disc's step by `discCnt` (in `Draw` and in the part it calls
    /// while the party rides): on to its stop, or to its animation's end
    /// past them. `move` clears where it stops.
    fn disc_step(&mut self) {
        self.moving = true;
        let Some(p) = self.def.path else { return };
        let Some(play) = self.play.as_mut() else { return };
        match p.stops.get(self.cnt as usize) {
            Some(&s) if play.time >> 8 >= s => self.moving = false,
            Some(_) => {
                play.forward(&self.file);
            }
            None => {
                if play.forward(&self.file) {
                    self.moving = false;
                }
            }
        }
    }
}

impl MapSystem for Disc {
    /// `Draw`'s step of the disc, then the part MUT 0x0041e8c0 does while
    /// the party rides (its trans mode on): the disc steps again, is put
    /// where the path was last frame (`STATICOBJECT::SetPos`, its hits
    /// after it), that place becomes the party's centre
    /// (`SetTransCenter`), and the path's point now (w 1) is kept for the
    /// next.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, others: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        self.disc_step();
        if self.def.path.is_none() {
            return;
        }
        self.disc_step();
        let t = self.path_point();
        let p = self.disc_pos;
        let pos = [ee::add(t[0], p[0]), ee::add(t[1], p[1]), ee::add(t[2], p[2]), ONE];
        let prev = self.disc_prev;
        let Some(objects) = others.get_mut::<StaticObjects>() else { return };
        if let Some(o) = objects.get_mut(0) {
            o.pos = prev;
            o.root = crate::town::pos_rot_zyx(prev, [0; 3]);
        }
        cx.core.hits.models = self.disc_hits(objects);
        cx.core.hits.trans = Some(prev);
        self.disc_prev = pos;
    }

    /// The start: the path's dummy (its world translation, w 1) plus the
    /// marker (w 1 too), so the leader faces 2.0; the disc's hits
    /// (`HitEnable`, `SetHitMatrix` at its root); the party's centre
    /// (`SetTransCenter(start)`, `SetTransDiff(0)`).
    fn place(&mut self, cx: &mut PlaceCx, others: &mut Others) {
        if self.def.path.is_none() {
            return;
        }
        let (t, p) = (self.path_point(), self.disc_pos);
        cx.core.start = [ee::add(t[0], p[0]), ee::add(t[1], p[1]), ee::add(t[2], p[2]), 0x4000_0000];
        if let Some(objects) = others.get::<StaticObjects>() {
            cx.core.hits.models = self.disc_hits(objects);
        }
        self.disc_prev = cx.core.start;
        cx.core.hits.trans = Some(cx.core.start);
    }

    /// `EVENTAREAB8::Move` (MUT 0x0041e750), from Kyvia's `CheckDiscMove`:
    /// the next stop, held at the last (field 9 has none).
    fn signal(&mut self, s: Signal, _: &mut PlaceCx) {
        let Some(p) = self.def.path.filter(|p| s == Signal::DiscMove && !p.stops.is_empty()) else { return };
        self.cnt = (self.cnt + 1).min(p.stops.len() as i32);
    }

    /// `IsMove()` (MUT 0x0041e7d0), `discPrevPos`, `DMY_marker01`'s place.
    fn report(&self, r: &mut Report) {
        r.disc = Some(piney_battle::boss::DiscView {
            moving: self.moving,
            prev_pos: self.disc_prev,
            marker: self.disc_pos,
            ..piney_battle::boss::DiscView::default()
        });
    }
}

/// `EVENTAREAB8`'s own: the 40 rocks (+0x200), `floatRockPos` (MUT
/// 0x0073eb00, the cells taken), the rocks' clumps' models by [`ROCKS`],
/// and what the last `DrawRock`s drew.
#[derive(Clone)]
pub struct Rocks {
    pub rocks: Vec<FloatRock>,
    pub grid: [[u8; 10]; 10],
    models: Vec<Vec<u32>>,
    file: Rc<SceneFile>,
    field: i32,
    drawn: Vec<(usize, F)>,
}

impl Rocks {
    /// The constructor's: each rock's `SetFloatRockParam`, then its clump
    /// by `fieldrand(6)`.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Rocks> {
        let file = cx.file.clone();
        let mut models = Vec::new();
        for name in ROCKS {
            if file.ccs.find_object(name).is_none() {
                return Err(Error::NotFound(name.into()));
            }
            models.push(clump_models(&file, name));
        }
        let field = cx.field();
        let mut grid = [[0; 10]; 10];
        let mut rocks = Vec::with_capacity(FLOAT_ROCKS);
        let mut no_cc = piney_battle::rand::Genrand::default();
        let cc: &mut dyn CcRand = match cx.cc.as_deref_mut() {
            Some(cc) => cc,
            None => &mut no_cc,
        };
        for _ in 0..FLOAT_ROCKS {
            let mut r = FloatRock::default();
            set_float_rock_param(&mut r, &mut grid, field, &mut cx.rng, cc);
            r.rock = cx.rng.below(ROCKS.len() as u32) as usize;
            rocks.push(r);
        }
        Ok(Rocks { rocks, grid, models, file, field, drawn: Vec::new() })
    }

    /// `DrawRock` (MUT 0x0041f130) for rock `k`: its fade (in over 40
    /// frames, out over its last 40), turn and rise, then drawn when
    /// `ccCheckCameraDeg(pos, 12288)`; at the end of its life its cell is
    /// freed and it starts again.
    fn rock_step(&mut self, k: usize, cx: &mut StepCx, cc: &mut dyn CcRand) {
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
        let v = cx.view;
        if crate::town::pc::check_camera_deg(r.pos, cloud::VIEW_DEG, &v.cam, v.player) {
            self.drawn.push((k, tr.unwrap_or(ONE)));
            cx.put(Kind::Rocks, Piece::Item(self.drawn.len() - 1));
        }
        if r.life == 0 {
            let (x, y) = (r.x as usize, r.y as usize);
            self.grid[x][y] = 0;
            let mut r = self.rocks[k];
            if let Some(rng) = &mut cx.core.field_rand {
                set_float_rock_param(&mut r, &mut self.grid, self.field, rng, cc);
            }
            self.rocks[k] = r;
        }
    }
}

impl MapSystem for Rocks {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        self.drawn.clear();
        let mut no_cc = piney_battle::rand::Genrand::default();
        let mut cc = cx.cc.take();
        for k in 0..self.rocks.len() {
            let cc: &mut dyn CcRand = match cc.as_deref_mut() {
                Some(cc) => cc,
                None => &mut no_cc,
            };
            self.rock_step(k, cx, cc);
        }
        cx.cc = cc;
    }

    /// A rock at its place and turn, `transparency` opaque.
    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Item(i) = piece else { return None };
        let &(k, transparency) = self.drawn.get(i)?;
        let r = &self.rocks[k];
        let world = Mat4::from_translation(Vec3::new(ee::f(r.pos[0]), ee::f(r.pos[1]), ee::f(r.pos[2])))
            * Mat4::from_rotation_x(ee::f(r.rot));
        Some(Request::Models {
            file: &self.file,
            models: Cow::Borrowed(&self.models[r.rock]),
            world,
            layer: layer::OBJ,
            rows: Default::default(),
            fog: Fogging::None,
            alpha: ee::f(transparency).clamp(0.0, 1.0),
        })
    }
}

/// `EVENTAREAB8`'s own in field 12: `se4_9` (`bodyccs`), its `bodyanm`
/// (+0x1f0) and `body[0..2]` (+0x1d8) at their places, each with its hit
/// models (on the list while its `ccClump::HitEnable` holds).
#[derive(Clone)]
pub struct Bodies {
    file: Rc<SceneFile>,
    pub anm: Play,
    pub bodies: Vec<(Vec<u32>, V4)>,
    hits: Vec<Vec<HitModel>>,
}

impl Bodies {
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Bodies> {
        let file = Rc::new(SceneFile::read(cx.archive, BODY_FILE)?);
        let all = HitModel::read(cx.volume, &file.ccs)?;
        let (mut bodies, mut hits) = (Vec::new(), Vec::new());
        for (name, at) in BODIES {
            if file.ccs.find_object(name).is_none() {
                return Err(Error::NotFound(format!("{BODY_FILE}: {name}")));
            }
            let models = clump_models(&file, name);
            let pos = [at[0], at[1], at[2], ONE];
            let own = all
                .iter()
                .filter(|h| models.contains(&h.parent))
                .map(|h| {
                    let mut h = h.clone();
                    h.rm = crate::hit::UNIT;
                    h.rm[3] = pos;
                    h.im = crate::dungeon::invers(&h.rm);
                    h
                })
                .collect();
            hits.push(own);
            bodies.push((models, pos));
        }
        let anm = Play::new(&file, BODY_ANIM).ok_or_else(|| Error::NotFound(BODY_ANIM.into()))?;
        Ok(Bodies { file, anm, bodies, hits })
    }
}

impl MapSystem for Bodies {
    /// `bodyanm`'s `Draw`, then `body[k]`.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        self.anm.forward(&self.file);
        cx.put(Kind::Bodies, Piece::Anim(0));
        (0..self.bodies.len()).for_each(|k| cx.put(Kind::Bodies, Piece::Item(k)));
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        let file = &*self.file;
        match piece {
            Piece::Anim(_) => Some(Request::Anim {
                file,
                play: &self.anm,
                root: Mat4::IDENTITY,
                layer: layer::OBJ,
                rows: None,
                fog: cx.fog,
                edit: None,
            }),
            Piece::Item(k) => {
                let (models, pos) = self.bodies.get(k)?;
                let world = Mat4::from_translation(Vec3::new(ee::f(pos[0]), ee::f(pos[1]), ee::f(pos[2])));
                let models = Cow::Borrowed(&models[..]);
                let (rows, fog) = (cx.rows.clone(), Fogging::None);
                Some(Request::Models { file, models, world, layer: layer::OBJ, rows, fog, alpha: 1.0 })
            }
            _ => None,
        }
    }

    /// Each body's hits onto the list.
    fn place(&mut self, cx: &mut PlaceCx, _: &mut Others) {
        cx.core.hits.models.extend(self.hits.iter().flatten().cloned());
    }

    /// `body[n]`'s `ccClump::HitEnable` (its models onto the list's tail,
    /// those not on it) or `HitDisable` (off it): Kyvia's fourth fight's
    /// gates (QUA gcmn 0x0030c620, 0x0030c690).
    fn signal(&mut self, s: Signal, cx: &mut PlaceCx) {
        let Signal::BodyHit(n, on) = s else { return };
        let Some(own) = self.hits.get(n) else { return };
        for h in own {
            let at = cx.core.hits.models.iter().position(|m| m == h);
            match (on, at) {
                (true, None) => cx.core.hits.models.push(h.clone()),
                (false, Some(k)) => {
                    cx.core.hits.models.remove(k);
                }
                _ => {}
            }
        }
    }
}

/// `Draw` in fields 9-11: `DrawBG` (the scroll, the clumps), `DrawObj` on
/// effLayer (its models, then its objects), the rocks (not in field 11),
/// `DrawFloor` on objLayer, then the disc and the ride. Through a frame
/// the tasks sleep: the clumps, the objects and the models as they stand.
const fn ride(rocks: bool) -> &'static [Step] {
    if rocks { RIDE } else { RIDE_NO_ROCKS }
}
const RIDE: &[Step] = &[
    Step::Run(Kind::Scrolls, Hook::Draw),
    Step::Run(Kind::Sky, Hook::Draw),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Rocks, Hook::Draw),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Floor)),
    Step::Run(Kind::Disc, Hook::Draw),
];
const RIDE_NO_ROCKS: &[Step] = &[
    Step::Run(Kind::Scrolls, Hook::Draw),
    Step::Run(Kind::Sky, Hook::Draw),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Floor)),
    Step::Run(Kind::Disc, Hook::Draw),
];
const STILL: &[Step] = &[
    Step::Run(Kind::Sky, Hook::Draw),
    Step::Run(Kind::Objects, Hook::Still(layer::EFF)),
    Step::Run(Kind::Models, Hook::Still(layer::OBJ)),
];

/// A disc field's block: `lgtAnm`'s light, the fog, its systems in the
/// constructor's order (the rocks draw from `fieldrand` and `ccRand`,
/// then the clouds from `fieldrand`).
const fn stage(systems: &'static [SystemDef], rocks: bool) -> BlockDef {
    BlockDef { lights: Some(LIGHTS), fog: Some(FOG), systems, order: ride(rocks), asleep: Some(STILL), ..BASE_BLOCK }
}

/// A disc field's systems with its path.
macro_rules! systems {
    ($disc:expr) => {
        &[
            SystemDef::Models(&MODELS),
            SystemDef::Objects(&OBJECTS),
            SystemDef::Sky(&SKY),
            SystemDef::Scrolls(&SCROLLS),
            SystemDef::Disc($disc),
            SystemDef::Rocks(&()),
            SystemDef::Clouds(&CLOUDS),
        ]
    };
}

/// `EVENTAREAB8` for a field: the disc's own tables, `fieldrand` the
/// game's one stream (`GO(1)` sets no seed for a story map).
const fn disc(fields: &'static [i32], blocks: &'static [BlockDef]) -> StoryMapDef {
    StoryMapDef { class: StoryMapGameClass::Eventareab8, fields, file: FILE, clear: BG_COLOR, blocks, ..BASE_STORY_MAP }
}

const SYSTEMS_9: &[SystemDef] = systems!(&PATH_9);
const SYSTEMS_10: &[SystemDef] = systems!(&PATH_10);
const SYSTEMS_11: &[SystemDef] = systems!(&PATH_11);
const BLOCK_9: &[BlockDef] = &[stage(SYSTEMS_9, true)];
const BLOCK_10: &[BlockDef] = &[stage(SYSTEMS_10, true)];
const BLOCK_11: &[BlockDef] = &[stage(SYSTEMS_11, false)];
pub static DISC_9: StoryMapDef = disc(&[9], BLOCK_9);
pub static DISC_10: StoryMapDef = disc(&[10], BLOCK_10);
pub static DISC_11: StoryMapDef = disc(&[11], BLOCK_11);

/// Field 12: the constructor's start, the others at their own places at
/// height 900; `Draw` steps the disc (none: `move` stays set), then
/// `bodyanm` and the three pieces. Through a frame the tasks sleep: the
/// clumps.
const BLOCK_12: &[BlockDef] = &[BlockDef {
    lights: Some(LIGHTS),
    fog: Some(FOG),
    start: Start::At(START_12),
    systems: &[
        SystemDef::Models(&MODELS),
        SystemDef::Objects(&OBJECTS),
        SystemDef::Sky(&SKY),
        SystemDef::Scrolls(&SCROLLS),
        SystemDef::Disc(&NO_PATH),
        SystemDef::Bodies(&()),
        SystemDef::Rocks(&()),
        SystemDef::Clouds(&CLOUDS),
    ],
    order: &[Step::Run(Kind::Disc, Hook::Draw), Step::Run(Kind::Bodies, Hook::Draw)],
    asleep: Some(&[Step::Run(Kind::Sky, Hook::Draw)]),
    ..BASE_BLOCK
}];
pub static DISC_12: StoryMapDef = StoryMapDef {
    spread: Spread::Placed { others: [[0xc258_0000, 0xc634_1400], [0x4380_0000, 0xc633_fc00]], z: 0x4463_8000 },
    ..disc(&[12], BLOCK_12)
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story_map::StoryArea;
    use crate::story_map::tests::made;
    use crate::systems::Drawn;
    use crate::systems::clouds::Clouds;
    use crate::systems::statics::StaticModels;
    use crate::town::TownView;
    use piney_battle::rand::Genrand;
    use piney_data::volume::Volume;

    fn disc(a: &StoryArea) -> &Disc {
        a.systems().get::<Disc>().unwrap()
    }

    fn rocks(a: &StoryArea) -> &Rocks {
        a.systems().get::<Rocks>().unwrap()
    }

    /// Field 9, Kyvia's first stage: the floor and the disc, the light, the
    /// disc's collision under it, the 40 rocks on cells of their own
    /// outside the centre, the 25 clouds, the start on the path facing
    /// 2.0, and the party's centre on.
    #[test]
    fn the_disc_of_field_9() {
        let Some(archive) = crate::town::tests::archive_of("mutation") else { return };
        let a = made(&archive, Volume::Mut, &DISC_9, 0, 0);
        let name = |o: u32| a.core.file.ccs.object_name(o).unwrap().to_string();
        let models = a.systems().get::<StaticModels>().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(name(models[0].model), "MDL_se1_6flo1");
        assert_eq!(models[0].pass, DrawPass::Floor);
        let objects = a.systems().get::<StaticObjects>().unwrap();
        assert_eq!((objects.len(), objects[0].pass), (1, DrawPass::Obj));
        assert_eq!(a.core.lights.lights.len(), 1);
        assert_eq!(disc(&a).cnt, 2);
        assert!(disc(&a).play.is_some() && disc(&a).dummy.is_some());
        assert!(!a.core.hits.models.is_empty(), "the disc's Hit chunk");
        assert!(a.core.hits.models.iter().all(|h| name(h.object) == "HIT_se1_6ob1hit"));
        let r = rocks(&a);
        assert_eq!(r.rocks.len(), FLOAT_ROCKS);
        let cells: std::collections::BTreeSet<(u32, u32)> = r.rocks.iter().map(|r| (r.x, r.y)).collect();
        assert_eq!(cells.len(), FLOAT_ROCKS, "a cell each");
        assert!(cells.iter().all(|&(x, y)| !((3..7).contains(&x) && (3..7).contains(&y))));
        assert!(r.rocks.iter().all(|r| (90..240).contains(&r.life) && r.cnt == 0));
        assert_eq!(a.systems().get::<Clouds>().unwrap().clouds.len(), 25);
        assert_eq!(a.core.start[3], 0x4000_0000);
        assert_eq!(a.trans_center(), Some(a.core.start));
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
        let Some(archive) = crate::town::tests::archive_of("mutation") else { return };
        let mut cc = Genrand::default();
        let mut a = made(&archive, Volume::Mut, &DISC_9, 0, 0);
        let v = TownView::default();
        let first = a.trans_center().unwrap();
        let mut prev = disc(&a).disc_prev;
        let mut frames = 0;
        let mut restarted = 0;
        while disc(&a).moving && frames < 5000 {
            let lives: Vec<i32> = rocks(&a).rocks.iter().map(|r| r.life).collect();
            a.step(&v, true, Some(&mut cc));
            frames += 1;
            assert_eq!(a.trans_center(), Some(prev), "the centre lags the path by a frame");
            assert_eq!(a.systems().get::<StaticObjects>().unwrap()[0].pos, prev);
            prev = disc(&a).disc_prev;
            restarted += lives.iter().filter(|&&l| l == 1).count();
            let kinds: Vec<Kind> =
                a.frame().iter().filter_map(|d| if let Drawn::Piece(k, _) = d { Some(*k) } else { None }).collect();
            assert!(kinds.contains(&Kind::Objects));
            assert_eq!(kinds.iter().filter(|&&k| k == Kind::Sky).count(), 4);
        }
        assert!(!disc(&a).moving, "the run ends");
        assert!(frames > 10, "{frames}");
        assert_ne!(a.trans_center().unwrap(), first, "the disc moved");
        assert!(restarted > 0, "a rock lived out its life");
        let r = rocks(&a);
        let cells: std::collections::BTreeSet<(u32, u32)> = r.rocks.iter().map(|r| (r.x, r.y)).collect();
        assert_eq!(cells.len(), FLOAT_ROCKS);
        assert!(r.rocks.iter().all(|x| r.grid[x.x as usize][x.y as usize] == 1));
    }

    /// Fields 10 and 11 stop at their stages until Kyvia's `Move`.
    #[test]
    fn the_later_discs_stop_at_their_stages() {
        let Some(archive) = crate::town::tests::archive_of("mutation") else { return };
        let mut cc = Genrand::default();
        let mut a = made(&archive, Volume::Mut, &DISC_10, 0, 0);
        let v = TownView::default();
        let mut n = 0;
        while disc(&a).moving && n < 2000 {
            a.step(&v, true, Some(&mut cc));
            n += 1;
        }
        assert!(!disc(&a).moving);
        assert!((disc(&a).play.as_ref().unwrap().time >> 8) >= 420);
        a.signal(Signal::DiscMove);
        assert_eq!(disc(&a).cnt, 1);
        a.step(&v, true, Some(&mut cc));
        assert!(disc(&a).moving, "on to 800");
        a.signal(Signal::DiscMove);
        a.signal(Signal::DiscMove);
        assert_eq!(disc(&a).cnt, 2, "held at the last stage");
    }
}
