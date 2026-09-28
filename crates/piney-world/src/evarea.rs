//! Story area 15, Hidden Forbidden Holy Ground: `EVENTAREA02` (gcmn
//! area02.cpp), the hand-built story map `WORLD_MAN::GO(1)` (main 0x0019f8e0)
//! makes in place of a generated field when the story area's
//! `EVENTAREA_INFO.model` is not 0 (`docs/engine/evarea.md`). Its scene file
//! `se1_2` holds two blocks: 0 the holy ground outside, 1 the church.
//!
//! ```text
//! EVENTAREA02()          0x00401e00  se1_2; SetFog(1500, 12000, 0, 70,
//!                                    0x1e1e1e); ccSys.bgColor 0x1e1e1e;
//!                                    ChangeBlock(0); SetStartPos(DMY_marker01);
//!                                    town_z; ccEff EFF_sflenz_2..6, 1
//!                                    (Init(chunk, 1)), each SetRenderState(
//!                                    CCRS_ZENABLE, 0), PRIM's fog bit off
//! ChangeBlock(b)         0x00402300  blk = b; the pieces and lights of the
//!                                    block before deleted; for 0 (else 1):
//!                                    SetFog(250, 7800, 0, 90, 0x323232)
//!                                    ((10000, 15000, ..)); EA_MODELTABLE02's 4
//!                                    STATICMODELs (EA_MODELTABLE0202's 7 and
//!                                    EA_OBJTABLE0202's STATICOBJECT); the
//!                                    background clumps bg1_1, wa1_1, cl1_1,
//!                                    cl2_1 (bg1_2); ANM_se1_2bac1a's light
//!                                    (bac2a's five); SetStartPos(DMY_marker02)
//!                                    (DMY_marker01_2); block 1's revAnm
//!                                    ANM_se1_2la1a
//! Draw()                 0x004033e0  DrawBG; obj2Layer: DrawObj2, DrawObj,
//!                                    revAnm; objLayer: DrawFloor, the type-0
//!                                    rows; effLayer: DrawLensFlare in block 1
//!                                    with no event scene (puppetShow)
//! ```
//!
//! A `STATICMODEL`'s model registers its Hit chunk on the `ccModelHit` list
//! as it is built (`HitEnable(0)`, at the identity) and its destructor takes
//! it off: each block's floor model carries the block's whole collision
//! (`HIT_se1_2fl1_1hit`, `HIT_se1_2fl1_2hit`). `ccLandHitCheck` never falls
//! back on a height map in an event area (`WORLD_MAN::CheckEventArea`), and
//! `WORLD_MAN::GetHeight` answers 0 there. The map is not a torus: `GO` sets
//! the bounds -48000..48000, which nothing reaches, so the wraps are the
//! identity but for the rounding of `(p - player) + player`.
//!
//! `WORLD_MAN::Enter` (main 0x0019dda0) on the door (ground attribute
//! 0x80000) of area 15: in block 0 or -1 `ChangeBlock(1)` and
//! `ChangeScene(-2, -2, -2, -2, -2, 1)`, else `ChangeBlock(0)` and block 0.
//! The set-up that follows keeps the `EVENTAREA02` (`WORLD_MAN::Quit` spares
//! `eventmap` for area 15 when the next area is a field): it stands the
//! party where the block's `SetStartPos` left `eventStartPos`.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::statics::{self, DrawPass, Position};
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::draw::{self, Draw};
use crate::ee::{self, F, ONE, V4};
use crate::hit::{HitModel, Hits};
use crate::pose::Play;
use crate::town::{FogParams, Light, TownLights};

/// The story area `EVENTAREA02` is, and its scene files.
pub const AREA: i32 = 15;
pub const FILE: &str = "se1_2";
pub const EFF_FILE: &str = "town_z";

/// `WORLD_MAN::GO(1)`'s bounds for a story map: -48000..48000 both ways.
pub const BOUNDS: [F; 4] = [0xc73b_8000, 0xc73b_8000, 0x473b_8000, 0x473b_8000];

/// `WORLD_MAN`'s layers (`GO`, `ccLayer::Init` priorities) the map draws on.
pub mod layer {
    /// `bgLayer[0]`, `[2]`, `[3]` (+0x494, +0x49c, +0x4a0).
    pub const BG0: i16 = -100;
    pub const BG2: i16 = -80;
    pub const BG3: i16 = -70;
    /// `SetActiveLayer(1)` objLayer (+0x4bc), `(2)` obj2Layer (+0x4c0),
    /// `(3)` effLayer (+0x4c8: the jump table at main 0x00355790 sends 3
    /// there and 4 to floorLayer).
    pub const OBJ: i16 = crate::draw::OBJ_LAYER;
    pub const OBJ2: i16 = crate::draw::OBJ2_LAYER;
    pub const EFF: i16 = crate::draw::EFF_LAYER;
}

/// The constructor's `ccSys.bgColor` (+0x18): the frame's clear colour.
pub const BG_COLOR: [u8; 3] = [0x1e, 0x1e, 0x1e];

/// The lens flare's six `ccEff`s (`LensFlare[0..5]`, +0x1c4) in the order
/// the constructor makes them: `LENSFLARE`'s.
pub use crate::lensflare::FLARES;
/// The sun the flare looks at.
const SUN: &str = "DMY_se1_2lig1point";

/// `EA_moveTex02` (gcmn 0x005d1d50): block 0's two scrolling materials,
/// `MOVETEX {u, v, u_step, v_step, matname}`. The table is gcmn's own data:
/// its scroll outlives the map.
pub const MOVE_TEX: [MoveTex; 2] = [
    MoveTex { u: 0, v: 0, u_step: 0, v_step: 0x3c23_d70a, mat: "MAT_se1_2cl1" },
    MoveTex { u: 0, v: 0, u_step: 0, v_step: 0x3c23_d70a, mat: "MAT_se1_2wa1" },
];

/// `revAnm`'s place: `@1365` (0, 5400, 0, 0) turned by `@1364` (0, 3.14,
/// -3.14) - the church's lanterns again, upside down under its floor.
const REV_POS: V4 = [0, 0x45a8_c000, 0, 0];
const REV_ROT: [F; 3] = [0, 0x4048_f5c3, 0xc048_f5c3];

/// `revAnm`'s root: `SetMatrix_PosRotZYX(@1365, @1364)` each frame.
pub fn rev_root() -> [V4; 4] {
    crate::town::pos_rot_zyx(REV_POS, REV_ROT)
}

/// A `MOVETEX` (0x14 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveTex {
    pub u: F,
    pub v: F,
    pub u_step: F,
    pub v_step: F,
    pub mat: &'static str,
}

impl MoveTex {
    /// `MoveTexture` (main 0x0019c4b0) up to the material: the scroll steps,
    /// each lane back to 0 past 1. The material's offsets then are
    /// `vftoi12(u)` and `vftoi12(v)` less its crop ([`MoveTex::offsets`]).
    pub fn step(&mut self) {
        self.u = ee::add(self.u, self.u_step);
        self.v = ee::add(self.v, self.v_step);
        if !ee::le(self.u, ONE) {
            self.u = 0;
        }
        if !ee::le(self.v, ONE) {
            self.v = 0;
        }
    }

    /// `vftoi12(u)`, `vftoi12(v)` as 16-bit offsets.
    pub fn offsets(&self) -> [u16; 2] {
        let t = |x: F| (ee::to_int(ee::mul(x, 0x4580_0000)) & 0xffff) as u16;
        [t(self.u), t(self.v)]
    }
}

/// A `STATICMODEL` (0x40 bytes): `type` +0x00, its model +0x04, the
/// dummy's position +0x10 (zero without one), `clip` +0x30.
#[derive(Clone, Debug)]
pub struct StaticModel {
    pub row: usize,
    pub pass: DrawPass,
    /// The MDL_ object.
    pub model: u32,
    pub pos: V4,
    pub clip: F,
}

/// A `STATICOBJECT` (0x40 bytes) of an animation: its `ccAnm`, the root
/// `SetMatrix_PosRotZYX(pos, rot)` gave it, the dummy's position (+0x10)
/// and `clip` (+0x30).
#[derive(Clone, Debug)]
pub struct StaticObject {
    pub row: usize,
    pub pass: DrawPass,
    pub play: Play,
    pub root: [V4; 4],
    pub pos: V4,
    pub clip: F,
}

/// One piece `EVENTAREA02::Draw` draws, in its order, on its layer.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    /// `DrawBG`: `ccClump::Draw` of `bg[k]` at the identity.
    Bg { k: usize, layer: i16 },
    /// `STATICMODEL::Draw` of `modelArray[k]` (`DrawWithOutFog` when `fog`
    /// is false: `DrawObj2`'s third).
    Model { k: usize, layer: i16, fog: bool },
    /// `STATICOBJECT::Draw` of `anmArray[k]` within its clip: its anm
    /// stepped to `time`, then drawn.
    Object { k: usize, layer: i16, time: u32 },
    /// `revAnm` stepped to `time`, placed and drawn.
    Rev { layer: i16, time: u32 },
    /// `DrawLensFlare`: `LensFlare[k]->Draw(pos, 0)`.
    Flare { k: usize, layer: i16, pos: V4 },
}

/// A `ccEff` sprite a story map draws ([`EventArea::draw`] and
/// [`crate::evarea_b0::Arena::draw`] hand them to the effects, which draw
/// sprites): `eff` of scene file `file`, drawn with `ccEff::Draw(pos, pat)`
/// on `layer` at its scale and transparency, as `Init(chunk, 1)` left it
/// (fog on, the depth test) - and for a lens flare `flare`, as its
/// constructor's `SetRenderState(0, 0)` left it too (no depth test, no
/// fog).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorySprite {
    pub file: &'static str,
    pub eff: &'static str,
    pub pos: V4,
    pub layer: i16,
    pub pat: u16,
    pub scale: F,
    /// `ccEff` +0x28 (a `CLOUD`'s turn); 0 for the others.
    pub rotate: F,
    pub transparency: F,
    pub flare: bool,
}

/// What the camera gives `DrawLensFlare`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlareCamera {
    /// `cameraGetPos(camID)` and `cameraGetView(camID)`.
    pub eye: V4,
    pub view: V4,
    /// `cameraGetRot(1)`, or with the eye view (`checkCameraType()` 1)
    /// `cameraGetRot2(1)`: `tcam`'s +0x20 or +0x40, all four lanes.
    pub rot: V4,
}

impl FlareCamera {
    /// The field camera's as `DrawLensFlare` reads them.
    pub fn of(c: &crate::camera::Camera) -> FlareCamera {
        let a = c.active();
        let rot = if a.kind == crate::camera::kind::EYE { c.tcam.rot3 } else { c.tcam.rot };
        FlareCamera { eye: a.pos, view: a.view, rot }
    }
}

/// What `WORLD_MAN` keeps of an area between its scenes: the dungeons made
/// on the way in (`WORLD_MAN.dungeon[n]`, with `lastRoom`), or area 15's
/// story map (`eventmap`, which `WORLD_MAN::Quit` spares for a field).
pub enum Kept {
    Dungeon(Box<crate::dungeon_area::Dungeons>),
    Event(Box<EventArea>),
}

/// `EVENTAREA02` as `WORLD_MAN.eventmap` holds it.
pub struct EventArea {
    /// `se1_2` (+0x1a4) and `town_z` (`effccs`, +0x1dc).
    pub file: Rc<SceneFile>,
    pub eff: Rc<SceneFile>,
    morphers: HashMap<u32, u32>,
    /// `blk` (+0x1e0).
    pub block: i32,
    /// `modelArray` (+0x78), `anmArray` (+0x140).
    pub models: Vec<StaticModel>,
    pub objects: Vec<StaticObject>,
    /// `bg[0..5]` (+0x1a8): each clump's models.
    pub bg: [Option<(u32, Vec<u32>)>; 5],
    /// `revAnm` (+0x1e4).
    pub rev: Option<Play>,
    /// The light group as `ChangeBlock` filled it from the light animation
    /// (`lgtAnm`, +0x74, after one `_AnimateForward`), and the ambient.
    pub lights: TownLights,
    /// `ccDrawEnv::SetFog` as the block set it.
    pub fog: FogParams,
    /// `WORLD_MAN.eventStartPos` (+0x70) as the last `SetStartPos` left it:
    /// the dummy's position, w its turn about z (radians).
    pub start: V4,
    /// `EA_moveTex02`.
    pub move_tex: [MoveTex; 2],
    /// The light group's LGT_ objects, in `lightList`'s order.
    pub light_objects: Vec<u32>,
    /// The block's `ccModelHit`s by model (the Hit chunks of the file), and
    /// the collision world with those of the block's models on its list.
    hit_models: Vec<HitModel>,
    pub hits: Hits,
}

/// A clump's models as `ccClump::Draw` draws them: each node's own model.
fn clump_models(file: &SceneFile, name: &str) -> Option<(u32, Vec<u32>)> {
    let c = file.ccs.find_object(name)?;
    let nodes = file.scene.clumps.iter().find(|(cl, _)| *cl == c).map(|(_, n)| n.clone())?;
    let mut m: Vec<u32> = nodes.iter().filter_map(|n| file.obj_model.get(n).copied()).collect();
    m.sort_unstable();
    Some((c, m))
}

/// `sqrt` of x x + y y in double (`mula`, `madd` in singles, `fptodp`,
/// `sqrt`, `dptofp`), as `STATICMODEL::DrawWithOutFog` and
/// `STATICOBJECT::Draw` measure.
fn ground_dist(dx: F, dy: F) -> F {
    let s = ee::add(ee::mul(dx, dx), ee::mul(dy, dy));
    (f64::from(ee::f(s)).sqrt() as f32).to_bits()
}

/// `d < clip` or `clip == 0`.
fn within(d: F, clip: F) -> bool {
    ee::lt(d, clip) || ee::eq(0, clip)
}

impl EventArea {
    /// `EVENTAREA02::EVENTAREA02`: the scene files, block 0 and
    /// `SetStartPos(DMY_marker01)`. `def_se` is `WORLD_MAN.defSE`, which a
    /// story map's ground never answers with.
    pub fn new(archive: &Arc<Archive>, def_se: u32) -> Result<EventArea> {
        let file = Rc::new(SceneFile::read(archive, FILE)?);
        let eff = Rc::new(SceneFile::read(archive, EFF_FILE)?);
        for name in FLARES {
            eff.ccs.find_object(name).ok_or_else(|| Error::NotFound(format!("{EFF_FILE}: {name}")))?;
        }
        let morphers = piney_data::anim::morphers(&file.ccs).unwrap_or_default();
        let hit_models = HitModel::read(&file.ccs)?;
        let hits = Hits { area: 1, bounds: Some(BOUNDS), heights: None, def_se, event_area: true, ..Hits::default() };
        let mut a = EventArea {
            file,
            eff,
            morphers,
            block: 0,
            models: Vec::new(),
            objects: Vec::new(),
            bg: Default::default(),
            rev: None,
            lights: TownLights { ambient: glam::Vec3::ZERO, lights: Vec::new(), fog: Some(FOG[0].depth()) },
            fog: FOG[0],
            start: [0, 0, 0, ONE],
            move_tex: MOVE_TEX,
            light_objects: Vec::new(),
            hit_models,
            hits,
        };
        a.change_block(0)?;
        a.start = a.marker("DMY_marker01")?;
        Ok(a)
    }

    /// The map as `GO(1)` finds it: `WORLD_MAN.eventmap` kept from the
    /// scene before (through the church's door, `Enter` having changed its
    /// block), else a new one, in block 0 whatever `game.block` says.
    pub fn for_scene(archive: &Arc<Archive>, kept: Option<Box<EventArea>>, def_se: u32) -> Result<Box<EventArea>> {
        match kept {
            Some(k) => Ok(k),
            None => Ok(Box::new(EventArea::new(archive, def_se)?)),
        }
    }

    /// A dummy's position with its turn about z in w, as the constructor
    /// and `ChangeBlock` hand it to `WORLD_MAN::SetStartPos` (they copy the
    /// DummyPosRot chunk's +0x28 into +0x1c).
    fn marker(&self, name: &'static str) -> Result<V4> {
        let (c, sc) = (&self.file.ccs, &self.file.scene);
        if c.find_object(name).is_none() {
            return Err(Error::NotFound(format!("{FILE}: {name}")));
        }
        let (pos, rot) = crate::town::dummy_bits(&Position::DummyRot(name), c, sc);
        Ok([pos[0], pos[1], pos[2], rot[2]])
    }

    /// `EVENTAREA02::ChangeBlock(block)` (gcmn 0x00402300): 0 the holy
    /// ground, any other the church.
    pub fn change_block(&mut self, block: i32) -> Result<()> {
        self.block = block;
        let b = usize::from(block != 0);
        let (c, sc) = (&self.file.ccs, &self.file.scene);
        let names = [("EA_MODELTABLE02", "EA_OBJTABLE02"), ("EA_MODELTABLE0202", "EA_OBJTABLE0202")][b];
        let table =
            statics::tables().iter().find(|t| t.name == names.0).ok_or_else(|| Error::NotFound(names.0.into()))?;
        let obj_table =
            statics::obj_tables().iter().find(|t| t.name == names.1).ok_or_else(|| Error::NotFound(names.1.into()))?;
        // modelNum 4 and anmNum 0, or 7 and 1: the whole tables but for the
        // first's one placeholder object row.
        let anm_num = [0, 1][b];
        self.models.clear();
        for (row, r) in table.rows.iter().enumerate() {
            let model = c.find_object(r.model).ok_or_else(|| Error::NotFound(format!("{FILE}: {}", r.model)))?;
            let (pos, _) = crate::town::dummy_bits(&r.position, c, sc);
            let pos = match r.position {
                Position::None => [0; 4],
                _ => pos,
            };
            self.models.push(StaticModel { row, pass: r.pass, model, pos, clip: r.clip.to_bits() });
        }
        self.objects.clear();
        for (row, r) in obj_table.rows.iter().enumerate().take(anm_num) {
            let name = r.anime.ok_or_else(|| Error::NotFound(format!("{}: row {row} has no animation", names.1)))?;
            let play = Play::new(&self.file, name).ok_or_else(|| Error::NotFound(name.into()))?;
            let (pos, rot) = crate::town::dummy_bits(&r.position, c, sc);
            let pos = match r.position {
                Position::None => [0; 4],
                _ => pos,
            };
            let root = crate::town::pos_rot_zyx(pos, rot);
            self.objects.push(StaticObject { row, pass: r.pass, play, root, pos, clip: r.clip.to_bits() });
        }
        // The collision: the block's models' Hit chunks, in the order the
        // STATICMODELs were built.
        self.hits.models =
            self.models.iter().filter_map(|m| self.hit_models.iter().find(|h| h.parent == m.model).cloned()).collect();
        let bg: &[(usize, &str)] = if b == 0 {
            &[(3, "CMP_se1_2bg1_1"), (2, "CMP_se1_2wa1_1"), (1, "CMP_se1_2cl1_1"), (0, "CMP_se1_2cl2_1")]
        } else {
            &[(0, "CMP_se1_2bg1_2")]
        };
        self.bg = Default::default();
        for &(k, name) in bg {
            self.bg[k] = Some(clump_models(&self.file, name).ok_or_else(|| Error::NotFound(name.into()))?);
        }
        // The light animation's lights (eventarea02Light, eventarea0202Light)
        // at frame 1, and its ambient.
        let (anim, lights): (&str, &[&str]) = if b == 0 {
            ("ANM_se1_2bac1a", &["LGT_se1_2lig1"])
        } else {
            ("ANM_se1_2bac2a", &["LGT_se1_2lig2", "LGT_omni01", "LGT_omni02", "LGT_omni03", "LGT_omni13"])
        };
        let ai = self.file.anim(anim).ok_or_else(|| Error::NotFound(anim.into()))?;
        let (ambient, records) = crate::town::anim_lights(&self.file, ai)?;
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
        self.start = self.marker(["DMY_marker02", "DMY_marker01_2"][b])?;
        self.rev = if b == 1 {
            Some(Play::new(&self.file, "ANM_se1_2la1a").ok_or_else(|| Error::NotFound("ANM_se1_2la1a".into()))?)
        } else {
            None
        };
        Ok(())
    }

    /// `WORLD_MAN::Enter` on the door of area 15 (main 0x0019dec4): into the
    /// church from block 0 (or -1, the constructor's), else out; the block
    /// the scene changes to (`ChangeScene(-2, -2, -2, -2, -2, block)`).
    pub fn enter(&mut self, scene_block: i32) -> Result<i32> {
        let to = if scene_block == 0 || scene_block == -1 { 1 } else { 0 };
        self.change_block(to)?;
        Ok(to)
    }

    /// `WORLD_MAN::SetCharPosition` in area 15's map (main 0x001a1414): the
    /// leader at `eventStartPos` facing its w; the three others at (+300,
    /// +150), (-300, +150) and (0, +300) from him, facing as he does.
    pub fn start_positions(&self) -> (V4, F, [V4; 3]) {
        let [x, y, z, dirc] = self.start;
        let starts = [
            [ee::add(0x4396_0000, x), ee::add(0x4316_0000, y), z, ONE],
            [ee::sub(x, 0x4396_0000), ee::add(0x4316_0000, y), z, ONE],
            [x, ee::add(0x4396_0000, y), z, ONE],
        ];
        ([x, y, z, ONE], dirc, starts)
    }

    /// `EVENTAREA::SetCenter(x, y)`: `cx`, `cy` (+0x08), which
    /// `WORLD_MAN::AddCenter` moves with the player ([`Hits::center`]; the
    /// map's bounds are never crossed, so it never wraps).
    pub fn set_center(&mut self, x: F, y: F) {
        self.hits.center = [x, y];
    }

    /// `ccSys.bgColor`, the constructor's.
    pub fn clear(&self) -> [u8; 3] {
        BG_COLOR
    }

    /// `DrawLensFlare` (gcmn 0x00402dc0): where each of the six flares is
    /// drawn - on the line from the sun (`DMY_se1_2lig1point`) to a point
    /// 2500 before the eye (turned by the camera, raised 500), pulled a
    /// fifth of the way back toward the sun and put at z -200, each flare
    /// its `@1260` share of the way from the sun toward it: `LENSFLARE`'s
    /// drawing ([`crate::lensflare::points`]) without its tests.
    pub fn lens_flare(&self, cam: &FlareCamera) -> [V4; 6] {
        let (c, sc) = (&self.file.ccs, &self.file.scene);
        let sun = c
            .find_object(SUN)
            .and_then(|o| sc.dummies.get(&o))
            .map_or([0; 4], |d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE]);
        crate::lensflare::points(sun, cam)
    }

    /// `EVENTAREA02::Draw` (gcmn 0x004033e0) for this frame: the pieces in
    /// its order, with what it steps on the way (the scrolls, the objects'
    /// and `revAnm`'s animations). `eye` is `cameraGetPos(camID)`, `player`
    /// Kite's place (`DrawWithOutFog` measures from him), `puppet_show`
    /// `eventMng.puppetShow`.
    pub fn select(&mut self, eye: V4, player: V4, cam: &FlareCamera, puppet_show: bool) -> Vec<Piece> {
        let mut out = Vec::new();
        // DrawBG.
        if self.block == 0 {
            for t in &mut self.move_tex {
                t.step();
            }
            out.push(Piece::Bg { k: 3, layer: layer::BG3 });
            out.push(Piece::Bg { k: 2, layer: layer::BG2 });
            out.push(Piece::Bg { k: 1, layer: layer::EFF });
            out.push(Piece::Bg { k: 0, layer: layer::EFF });
        } else if self.block == 1 {
            out.push(Piece::Bg { k: 0, layer: layer::BG0 });
        }
        // DrawObj2: the type-3 rows, the third without fog and only within
        // its clip of the player.
        for (k, m) in self.models.iter().enumerate() {
            if m.pass != DrawPass::Obj2 {
                continue;
            }
            if k == 2 {
                let d = crate::field_area::w2p(m.pos, player, BOUNDS);
                if within(ground_dist(d[0], d[1]), m.clip) {
                    out.push(Piece::Model { k, layer: layer::OBJ2, fog: false });
                }
            } else {
                out.push(Piece::Model { k, layer: layer::OBJ2, fog: true });
            }
        }
        // DrawObj: the type-2 rows, then the type-2 static objects within
        // their clip of the eye.
        for (k, m) in self.models.iter().enumerate() {
            if m.pass == DrawPass::Obj {
                out.push(Piece::Model { k, layer: layer::OBJ2, fog: true });
            }
        }
        for k in 0..self.objects.len() {
            let o = &mut self.objects[k];
            if o.pass != DrawPass::Obj {
                continue;
            }
            let d = ground_dist(ee::sub(eye[0], o.pos[0]), ee::sub(eye[1], o.pos[1]));
            if within(d, o.clip) {
                o.play.forward(&self.file);
                out.push(Piece::Object { k, layer: layer::OBJ2, time: o.play.time });
            }
        }
        if let Some(r) = &mut self.rev {
            r.forward(&self.file);
            out.push(Piece::Rev { layer: layer::OBJ2, time: r.time });
        }
        // DrawFloor, then the type-0 rows, on objLayer.
        for (k, m) in self.models.iter().enumerate() {
            if m.pass == DrawPass::Floor {
                out.push(Piece::Model { k, layer: layer::OBJ, fog: true });
            }
        }
        for (k, m) in self.models.iter().enumerate() {
            if m.pass == DrawPass::Other {
                out.push(Piece::Model { k, layer: layer::OBJ, fog: true });
            }
        }
        if self.block == 1 && !puppet_show {
            for (k, pos) in self.lens_flare(cam).into_iter().enumerate() {
                out.push(Piece::Flare { k, layer: layer::EFF, pos });
            }
        }
        out
    }

    /// The scrolling materials' texture offsets as `MoveTexture` wrote them
    /// (the offsets less each material's crop), for the draw.
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

    /// One model at a world matrix.
    fn model_draw(&self, layers: &mut Layers, layer: i16, to_screen: Mat4, model: u32, world: Mat4, fog: bool) {
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
        draw::model_edited(layers, layer, to_screen, d, None, if fog { self.depth_fog() } else { draw::Fogging::None });
    }

    /// The blocks' `SetFog` as VU1 fogs the models drawn with it.
    fn depth_fog(&self) -> draw::Fogging {
        draw::Fogging::Depth(self.fog.depth())
    }

    /// `EVENTAREA02::Draw` into the field's layers: the pieces
    /// [`EventArea::select`] gives (stepping only when `step`: the tasks
    /// awake), and the lens flare's sprites for the effects to draw. With
    /// `step` false the pieces are drawn as the last frame left them. The
    /// models and objects take the blocks' fog by depth, but the
    /// background's and `DrawObj2`'s third (`DrawWithOutFog`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        layers: &mut Layers,
        to_screen: Mat4,
        eye: V4,
        player: V4,
        cam: &FlareCamera,
        puppet_show: bool,
        step: bool,
    ) -> Vec<StorySprite> {
        let pieces = if step {
            self.select(eye, player, cam, puppet_show)
        } else {
            let saved = (self.move_tex, self.objects.clone(), self.rev.clone());
            let p = self.select(eye, player, cam, puppet_show);
            (self.move_tex, self.objects, self.rev) = saved;
            p
        };
        let mut sprites = Vec::new();
        for p in pieces {
            match p {
                Piece::Bg { k, layer } => {
                    let Some((_, models)) = &self.bg[k] else { continue };
                    for &m in models {
                        self.model_draw(layers, layer, to_screen, m, Mat4::IDENTITY, false);
                    }
                }
                Piece::Model { k, layer, fog } => {
                    let m = &self.models[k];
                    let world =
                        Mat4::from_translation(glam::Vec3::new(ee::f(m.pos[0]), ee::f(m.pos[1]), ee::f(m.pos[2])));
                    self.model_draw(layers, layer, to_screen, m.model, world, fog);
                }
                Piece::Object { k, layer, .. } => {
                    let o = &self.objects[k];
                    let root = draw::mat(&o.root);
                    crate::town::anim_draw_fog(
                        layers,
                        layer,
                        to_screen,
                        (&self.file, &self.morphers),
                        &o.play,
                        root,
                        &[],
                        1.0,
                        None,
                        None,
                        self.depth_fog(),
                    );
                }
                Piece::Rev { layer, .. } => {
                    let Some(r) = &self.rev else { continue };
                    let root = draw::mat(&rev_root());
                    crate::town::anim_draw_fog(
                        layers,
                        layer,
                        to_screen,
                        (&self.file, &self.morphers),
                        r,
                        root,
                        &[],
                        1.0,
                        None,
                        None,
                        self.depth_fog(),
                    );
                }
                Piece::Flare { k, layer, pos } => sprites.push(StorySprite {
                    file: EFF_FILE,
                    eff: FLARES[k],
                    pos,
                    layer,
                    pat: 0,
                    scale: ONE,
                    rotate: 0,
                    transparency: ONE,
                    flare: true,
                }),
            }
        }
        sprites
    }
}

/// The blocks' `SetFog(near, far, 0, 90, 0x323232)`.
const FOG: [FogParams; 2] = [
    FogParams { near: 250.0, far: 7800.0, near_rate: 0.0, far_rate: 90.0, colour: [0x32, 0x32, 0x32] },
    FogParams { near: 10000.0, far: 15000.0, near_rate: 0.0, far_rate: 90.0, colour: [0x32, 0x32, 0x32] },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The constructor's block 0 and the church: their models, the floor's
    /// collision, the start places and the door's way back and forth.
    #[test]
    fn blocks_and_the_door() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut a = EventArea::new(&archive, 0).unwrap();
        let name = |a: &EventArea, o: u32| a.file.ccs.object_name(o).unwrap().to_string();
        assert_eq!(a.models.len(), 4);
        assert!(a.objects.is_empty() && a.rev.is_none());
        assert_eq!(a.hits.models.len(), 1);
        assert_eq!(name(&a, a.hits.models[0].object), "HIT_se1_2fl1_1hit");
        // DMY_marker01: (0, -3600, 0), turned -179.99998 degrees.
        assert_eq!(a.start[..3], [0, ee::k(-3600.0), 0]);
        assert!((ee::f(a.start[3]) + std::f32::consts::PI).abs() < 1e-5);
        assert_eq!(a.lights.lights.len(), 1);
        // Into the church and out again.
        assert_eq!(a.enter(-1).unwrap(), 1);
        assert_eq!((a.models.len(), a.objects.len(), a.rev.is_some()), (7, 1, true));
        assert_eq!(name(&a, a.hits.models[0].object), "HIT_se1_2fl1_2hit");
        assert_eq!(a.start[..3], [0, ee::k(900.0), 0]);
        assert_eq!(a.lights.lights.len(), 5);
        assert_eq!(a.enter(1).unwrap(), 0);
        assert_eq!(a.start[..3], [0, ee::k(2700.0), ee::k(200.0)]);
        // Standing on the door's floor: its ground has the Enter bit.
        let z = a.hits.land([ee::k(95.0), ee::k(3700.0), ee::k(300.0), ONE], crate::hit::LAND_MASK);
        assert!((ee::f(z) - 200.0).abs() < 1e-3);
        assert_ne!(a.hits.nearest.att & 0x8_0000, 0);
    }
}
