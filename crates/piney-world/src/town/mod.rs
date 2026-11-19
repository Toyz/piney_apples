//! The Root Towns as `ccThFieldDisp` (priority 96) draws them each frame, and
//! what else their constructors set up: the collision mesh, the lights and
//! the fog. Each town is a class of its own in the game (`ROOTTOWN01` ..
//! `06`); here each is a [`TownDef`] (town/mac_anu.rs .. town/relic_city.rs):
//! [`BASE_TOWN`] with its files, lights, fog, its systems
//! (`crate::systems`) and its `Draw`'s steps. A [`Town`] is the def, the
//! core every area holds and its systems; [`Town::step`] runs the steps,
//! [`Town::requests`] hands back what they drew (docs/engine/root-towns.md).

#[doc(alias = "ROOTTOWN03")]
pub mod carmina_gadelica;
pub mod cast;
#[doc(alias = "ccDog")]
pub mod dog;
#[doc(alias = "ROOTTOWN02")]
pub mod dun_loireag;
#[doc(alias = "ROOTTOWN04")]
pub mod fort_ouph;
#[doc(alias = "ROOTTOWN05")]
pub mod lia_fail;
#[doc(alias = "ROOTTOWN01")]
pub mod mac_anu;
#[doc(alias = "ccMerchan")]
pub mod merchant;
pub mod party;
#[doc(alias = "ccRtownPC")]
#[doc(alias = "rtownpc")]
pub mod pc;
#[doc(alias = "ROOTTOWN06")]
pub mod relic_city;

use std::collections::HashMap;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::classes::TownGameClass;
use piney_data::dungeon::Rng;
use piney_data::light::{AnimLight, Light};
use piney_data::scene::Scene;
use piney_data::statics::Position;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::area::{AreaCore, AreaMap, Disp, MapFrame};
use crate::chara::pose::Play;
use crate::draw::{self, Draw, OBJ_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::map::town::MinimapDef;
use crate::systems::{BuildCx, Carried, CarriedMut, DrawCx, Drawn, Request, Step, SystemDef, Systems};
use crate::systems::{MapEvent, MapView};

/// `vftoi12`'s 4096.
const K4096: F = 0x4580_0000;

/// `cc3d->SetFog(near, far, near rate, far rate, colour)` as a town's
/// constructor calls it: no fog at `near`, `far_rate` percent at `far`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogParams {
    pub near: f32,
    pub far: f32,
    pub near_rate: f32,
    pub far_rate: f32,
    /// R, G, B.
    pub colour: [u8; 3],
}

impl FogParams {
    /// VU1's fog by depth as this `SetFog` leaves `ccDrawEnv`.
    pub fn depth(&self) -> piney_draw::DepthFog {
        piney_draw::DepthFog::set_fog(self.near, self.far, self.near_rate, self.far_rate, self.colour)
    }
}

/// The light group a draw environment holds (`ccDrawEnv`'s group at +0x80,
/// its ambient at +0x30) and the fog it draws with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TownLights {
    /// The ambient (`SetAmbient`; w unused).
    pub ambient: V4,
    /// In the order added (`town00Light`'s for a town); `AddGrp` keeps the
    /// group in descending priority.
    pub lights: Vec<Light>,
    /// The lights the area's `STATICOBJECT`s' anms put in the group, added
    /// before [`TownLights::lights`] (`SetLightEnv(1)` then `SetAnm`), as
    /// their last step left them.
    pub objects: Vec<Light>,
    /// The draw environment's fog (`cc3d->SetFog`'s arguments), which VU1
    /// puts on every model drawn with PRIM.FGE (`SetFogSw` on: the
    /// characters, a town's buildings) by each vertex's depth, and the
    /// effects' sprites take at their centre; None, none.
    pub fog: Option<FogParams>,
    /// The disc whose code lights the group (its `CheckRange`s' `sqrt`).
    pub volume: Volume,
}

impl TownLights {
    /// [`TownLights::fog`] as VU1 fogs a model by depth.
    pub fn depth_fog(&self) -> Option<piney_draw::DepthFog> {
        self.fog.map(|f| f.depth())
    }
}

/// An Anime chunk's ambient (`ccAnm::GetAmbient` of the last 0x0601
/// record at or before frame 1) and its light records
/// ([`piney_data::light::anim_lights`]).
pub fn anim_lights(file: &SceneFile, anim: usize) -> Result<(Option<V4>, Vec<AnimLight>)> {
    let (packed, lights) = piney_data::light::anim_lights(&file.ccs.data, file.anims[anim].offset)?;
    Ok((packed.map(piney_data::light::ambient), lights))
}

/// An Anime chunk's lights at frame 1 (`SetAnm` then one `_AnimateForward`,
/// under the anm's unit matrix) in the order of the class's light table
/// (`town00Light`, `town02Light`: the distant light, then the omnis), and
/// its ambient, lit by `volume`'s code.
pub(crate) fn read_lights(file: &SceneFile, anim: usize, table: &[(i32, &str)], volume: Volume) -> Result<TownLights> {
    let (ambient, lights) = anim_lights(file, anim)?;
    let mut out = Vec::new();
    for (_, name) in table {
        let obj = file.ccs.find_object(name).ok_or_else(|| Error::NotFound((*name).into()))?;
        if let Some(l) = lights.iter().find(|l| l.object == obj) {
            out.push(l.at(256, &piney_data::vu0::UNIT));
        }
    }
    Ok(TownLights { ambient: ambient.unwrap_or([0; 4]), lights: out, fog: None, volume, objects: Vec::new() })
}

/// A clump's models as `ccClump::Draw` draws them: each node's own
/// (morph targets, mtype bit 3, left out), in object order.
pub(crate) fn clump_models(file: &SceneFile, name: &str) -> Vec<u32> {
    let (c, sc) = (&file.ccs, &file.scene);
    c.find_object(name)
        .and_then(|o| sc.clumps.iter().find(|(cl, _)| *cl == o))
        .map(|(_, nodes)| {
            let mut m: Vec<u32> =
                sc.model_owner.iter().filter(|(_, owner)| nodes.contains(owner)).map(|(m, _)| *m).collect();
            m.retain(|m| file.models.get(m).is_some_and(|i| i.mtype & 8 == 0));
            m.sort_unstable();
            m
        })
        .unwrap_or_default()
}

/// `waterUVModifi2`'s screen: the frame buffer's corner in GS pixels and
/// its size; 256 the texture's width and height.
const SCREEN_X: F = 0x44e0_0000; // 1792
const SCREEN_Y: F = 0x44e4_0000; // 1824
const SCREEN_W: F = 0x4400_0000; // 512
const SCREEN_H: F = 0x43e0_0000; // 448
const K256: F = 0x4380_0000;

/// `waterUVModifi2(obj, eye, reach)` (gcmn 0x005025d0), which the towns'
/// `Draw` run on water 0 just before it steps: nothing when the object is out
/// of view or farther than `reach` from the eye on the ground; else each
/// vertex of the model's first mmat, through the object's world matrix and
/// `sceVu0RotTransPers`, becomes its screen point as a fraction of the frame
/// buffer, `fptoui(256 x)`: the water samples the picture behind it where its
/// unbent vertex would be (docs/engine/town02.md). From Outbreak on the
/// point is first held to the frame buffer, 0-512 across and 0-448 down
/// (OUT gcmn 0x005183cc; `waterUVModifi` 0x00517fe4 the same).
pub fn water_st(
    volume: piney_data::volume::Volume,
    model: &piney_data::model::Model,
    lw: &[V4; 4],
    at: V4,
    eye: V4,
    reach: F,
    world_screen: &[V4; 4],
) -> Option<Vec<[u16; 2]>> {
    let (dx, dy) = (ee::sub(eye[0], at[0]), ee::sub(eye[1], at[1]));
    // OUT gcmn 0x00518170.
    let d = ee::ground_dist(volume, dx, dy);
    if !ee::le(d, reach) {
        return None;
    }
    let scale = ee::div(model.scale.to_bits(), K4096);
    let mm = model.mmats.first()?;
    let held = matches!(volume, piney_data::volume::Volume::Out | piney_data::volume::Volume::Qua);
    // `x < 0` to 0, else anything not `<= top` (a NaN too) to `top`.
    let hold = |x: F, top: F| {
        if ee::lt(x, 0) {
            0
        } else if ee::le(x, top) {
            x
        } else {
            top
        }
    };
    Some(
        mm.positions
            .iter()
            .map(|p| {
                let v = p.map(|c| ee::mul(ee::from_int(i32::from(c)), scale));
                let q = ee::apply(lw, [v[0], v[1], v[2], ONE]);
                let r = ee::rot_trans_pers(world_screen, q);
                let (sx, sy) = (ee::from_int(r[0] / 16), ee::from_int(r[1] / 16));
                let (mut u, mut v) = (ee::sub(sx, SCREEN_X), ee::sub(sy, SCREEN_Y));
                if held {
                    u = hold(u, SCREEN_W);
                    v = hold(v, SCREEN_H);
                }
                let (u, v) = (ee::div(u, SCREEN_W), ee::div(v, SCREEN_H));
                let st = |x: F| soft_fptoui(ee::mul(K256, x)) as u16;
                [st(u), st(v)]
            })
            .collect(),
    )
}

/// libgcc's soft-float `fptoui` (main 0x00129d08, through `__unpack_f`),
/// as `waterUVModifi2` calls it: truncation toward zero, but 0 for any
/// negative value and all ones from 2^32 up.
fn soft_fptoui(v: F) -> u32 {
    let e = ((v >> 23) & 0xff) as i32 - 127;
    if v & 0x8000_0000 != 0 || v & 0x7f80_0000 == 0 || e < 0 {
        return 0;
    }
    if e >= 32 {
        return u32::MAX;
    }
    let frac = u64::from((v & 0x7f_ffff) | 0x80_0000);
    (if e >= 23 { frac << (e - 23) } else { frac >> (23 - e) }) as u32
}

/// Water 0 as `waterUVModifi2` and the town's frame-buffer copy leave it:
/// the model's first mmat's texture coordinates (its own until a call
/// within reach writes them; the game writes them into the model, so they
/// stay until the next), drawn with the picture so far as its texture
/// (`ccModel::ChangeTex` of the texture `ccLayer::MakePacketDrawBuffTrans`
/// fills each frame).
#[derive(Clone, Debug)]
pub struct WaterZero {
    /// The water's model: `OBJ_wat00`'s (`wat1`) or `OBJ_sr2wat00`'s.
    pub model: piney_data::model::Model,
    /// What the last call within reach wrote.
    pub st: Option<Vec<[u16; 2]>>,
    /// 9000 in Mac Anu, 32000 in Dun Loireag.
    pub reach: F,
}

impl WaterZero {
    pub(crate) fn new(file: &SceneFile, model: &str, reach: F) -> Result<WaterZero> {
        let o = file.ccs.find_object(model).ok_or_else(|| Error::NotFound(model.into()))?;
        let model = piney_data::model::models(&file.ccs)?
            .into_iter()
            .find(|m| m.object == o)
            .ok_or_else(|| Error::NotFound(model.into()))?;
        Ok(WaterZero { model, st: None, reach })
    }

    /// `waterUVModifi2(obj, eye, reach)` with the object's world matrix and
    /// place.
    pub fn modify(
        &mut self,
        volume: piney_data::volume::Volume,
        lw: &[V4; 4],
        at: V4,
        eye: V4,
        world_screen: &[V4; 4],
    ) {
        if let Some(st) = water_st(volume, &self.model, lw, at, eye, self.reach, world_screen) {
            self.st = Some(st);
        }
    }

    /// The model's texture coordinates and texture as the draw sends them.
    pub(crate) fn edits(&self) -> std::sync::Arc<piney_draw::VertexEdits> {
        let st = self.st.iter().flatten().enumerate().map(|(i, &st)| (0, i as u32, st)).collect();
        // MakePacketDrawBuffTrans: the picture point-sampled into the
        // 128 x 128 texture the water samples (WORLD +0x53bc, a town's
        // +0x1d4).
        let tex = piney_draw::TexRef::SampledFrame { width: 128, height: 128 };
        std::sync::Arc::new(piney_draw::VertexEdits { st, tex: Some(tex), ..Default::default() })
    }
}

/// Where each draw's scroll statics sit in `SaveState::scroll_statics`:
/// statics of the executable's own small data (INF names; 0 at power-on),
/// which no load sets again (docs/engine/root-towns.md).
pub mod scroll_statics {
    use piney_desktop::SCROLL_STATICS;

    /// `ROOTTOWN01::DrawBG`'s `v$1334` (two), `Draw`'s `u$1841`.
    pub const TOWN01_BG: usize = 0;
    pub const TOWN01_WATER: usize = 2;
    /// `ROOTTOWN02`'s `v$1346` (two), `v2$1347`, `u$1777`.
    pub const TOWN02_BG: usize = 3;
    pub const TOWN02_BG2: usize = 5;
    pub const TOWN02_WATER: usize = 6;
    /// `ROOTTOWN03`'s `v$1474` (two), `u$1875`.
    pub const TOWN03_BG: usize = 7;
    pub const TOWN03_WATER: usize = 9;
    /// `ROOTTOWN04`'s `v$1269` (two); `ROOTTOWN05`'s `v$1244` (two) and
    /// `v2$1245`; Quarantine's `ROOTTOWN06`'s three.
    pub const TOWN04_BG: usize = 10;
    pub const TOWN05_BG: usize = 12;
    pub const TOWN06_BG: usize = 15;
    /// `EVENTAREAB0::DrawBG`'s `v$1597`.
    pub const ARENA_BG: usize = 18;
    /// `DUNGEON::DrawWater`'s `u$8862`, `DrawBG`'s `v$8917`.
    pub const DUNGEON_WATER: usize = 19;
    pub const DUNGEON_LAKE: usize = 20;
    /// `WORLD::DrawBG`'s sky `v$4519` (0 on its first use, `init$4520`).
    pub const WORLD_BG: usize = 21;
    /// `WORLD::Draw`'s lake scrolls, `u$4930` and `v$4933`.
    pub const WORLD_LAKE: usize = 22;
    /// `WORLD::DrawBG`'s turns, `rot2$4526.z` and `rot3$4527.z`.
    pub const WORLD_BG_TURN: usize = 24;
    /// `EVENTAREAB8::DrawBG`'s `v` (MUT gp 0x0038ba78) and its first-use
    /// flag (0x0038ba7c): the first call sets it to 1.0.
    pub const DISC_BG: usize = 26;
    pub const DISC_BG_FIRST: usize = 27;
    /// The minimaps' arrow pulses, `ccRotate`'s angles: `ROOTTOWN01`-`05`'s
    /// `DrawMap` `r$1560`, `r$1488`, `r$1569`, `r$1407`, `r$1313` (INF
    /// 0x00378b4c-0x00378ba0), `WORLD::DrawMiniMap`'s `r$3072` (0x00378d04)
    /// and `DUNGEON::DrawMap`'s `r$8238` (0x00378d78). Infection's and
    /// Mutation's first call sets each to 0 (`init$1561` ..), as it loads.
    pub const TOWN01_PULSE: usize = 28;
    pub const TOWN02_PULSE: usize = 29;
    pub const TOWN03_PULSE: usize = 30;
    pub const TOWN04_PULSE: usize = 31;
    pub const TOWN05_PULSE: usize = 32;
    pub const WORLD_PULSE: usize = 33;
    pub const DUNGEON_PULSE: usize = 34;

    /// The statics from `at` into `dst`.
    pub fn take(dst: &mut [u32], src: &[u32; SCROLL_STATICS], at: usize) {
        dst.copy_from_slice(&src[at..at + dst.len()]);
    }

    /// `src` into the statics from `at`.
    pub fn put(src: &[u32], dst: &mut [u32; SCROLL_STATICS], at: usize) {
        dst[at..at + src.len()].copy_from_slice(src);
    }
}

/// What `WORLD_MAN::GO(0)` makes a town from: the disc, the save's crisis
/// byte and `fieldrand`.
pub struct TownEnv<'a> {
    pub archive: &'a Arc<Archive>,
    pub volume: Volume,
    pub crisis: bool,
    pub rng: Rng,
}

/// What a Root Town is: its class, the discs that have it, its files
/// (normal and crisis), its lights, fog and clear colour, where Kite
/// starts, its systems as its constructor makes them and its `Draw`'s
/// steps in the game's order. One `static` a town, so a def is its address.
#[derive(Debug)]
pub struct TownDef {
    /// `game.town`: 0 Mac Anu .. 4 Lia Fail, 5 Quarantine's sixth.
    pub no: i32,
    pub class: TownGameClass,
    /// What the window's title calls it.
    pub name: &'static str,
    pub volumes: &'static [Volume],
    pub file: &'static str,
    /// `town0Nd`, loaded when the save's crisis byte is set.
    pub crisis_file: Option<&'static str>,
    /// Whose statics tables its rows are: Infection's for the first five
    /// on every disc, Quarantine's for the sixth.
    pub tables: Volume,
    /// The light animation and the class's light table (`town00Light`
    /// ..: the distant light, then the omnis).
    pub light_anim: &'static str,
    pub lights: &'static [(i32, &'static str)],
    pub fog: FogParams,
    /// `ccSys.bgColor` as the constructor sets it: None keeps
    /// `ccSetupGameCtrl`'s black.
    pub clear: Option<[u8; 3]>,
    /// `WORLD_MAN::SetCharPosition`'s place and heading for Kite (main
    /// 0x001a1190); None where it has no case (`StartPos` keeps what the
    /// scene before left, QUA main 0x001b3ac0).
    pub start: Option<(V4, V4)>,
    /// Its `DrawMap`'s minimap (`crate::map::town`), None where it draws
    /// none (the sixth's returns at once).
    pub minimap: Option<&'static MinimapDef>,
    pub systems: &'static [SystemDef],
    pub order: &'static [Step],
}

/// One def is one town.
impl PartialEq for TownDef {
    fn eq(&self, other: &TownDef) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for TownDef {}

/// What `ROOTTOWN` gives every town: every disc, no crisis file,
/// Infection's tables, no clear colour; no systems and an empty `Draw`.
pub static BASE_TOWN: TownDef = TownDef {
    no: -1,
    class: TownGameClass::Roottown,
    name: "a Root Town",
    volumes: &Volume::ALL,
    file: "",
    crisis_file: None,
    tables: Volume::Inf,
    light_anim: "",
    lights: &[],
    fog: FogParams { near: 0.0, far: 0.0, near_rate: 0.0, far_rate: 0.0, colour: [0; 3] },
    clear: None,
    start: None,
    minimap: None,
    systems: &[],
    order: &[],
};

/// `GO(0)`'s towns.
pub static TOWNS: [&TownDef; 6] = [
    &mac_anu::MAC_ANU,
    &dun_loireag::DUN_LOIREAG,
    &carmina_gadelica::CARMINA_GADE,
    &fort_ouph::FORT_OUPH,
    &lia_fail::LIA_FAIL,
    &relic_city::RELIC_CITY,
];

impl TownDef {
    /// `game.town` `no`'s def on `volume`.
    pub fn of(no: i32, volume: Volume) -> Option<&'static TownDef> {
        TOWNS.iter().copied().find(|d| d.no == no && d.volumes.contains(&volume))
    }
}

/// A Root Town as the field holds it: its def, the core every area holds,
/// its systems and what its last `Draw` made.
pub struct Town {
    pub def: &'static TownDef,
    /// The file (the crisis one in crisis), the lights, the collision, and
    /// `fieldrand` when a system draws from it.
    pub core: AreaCore,
    /// `WORLD_MAN::SetCharPosition`'s place and heading for Kite: the def's,
    /// else `StartPos` as the scene before left it ([`Town::keep_start`]).
    start: (V4, V4),
    /// Its `DrawMap`'s minimap ([`AreaMap::setup_map`]).
    map: Option<Box<crate::map::town::TownMap>>,
    systems: Systems,
    /// The last `Draw`'s pieces, the offsets its scrolls wrote into the
    /// file's materials, and the effects and sounds its steps started since
    /// the host last took them.
    frame: Vec<Drawn>,
    rows: HashMap<u32, [u8; 2]>,
    events: Vec<MapEvent>,
}

impl Town {
    /// The town `WORLD_MAN::GO(0)` builds for `game.town` `no`, `crisis` the
    /// save's crisis byte.
    pub fn open(archive: &Arc<Archive>, volume: Volume, no: i32, crisis: bool) -> Result<Town> {
        Town::open_with(archive, volume, no, crisis, Rng::POWER_ON)
    }

    /// [`Town::open`] with `fieldrand` as the game has it (`rng`), which
    /// the systems' draws take from.
    pub fn open_with(archive: &Arc<Archive>, volume: Volume, no: i32, crisis: bool, rng: Rng) -> Result<Town> {
        let def = TownDef::of(no, volume).ok_or_else(|| Error::NotFound(format!("town {no}: {volume:?} has none")))?;
        Town::build(def, &TownEnv { archive, volume, crisis, rng })
    }

    /// `def`'s constructor: its file, the light animation's lights at
    /// frame 1, the collision mesh (the disc's square root), the fog, and
    /// its systems.
    pub fn build(def: &'static TownDef, env: &TownEnv) -> Result<Town> {
        let stem = def.crisis_file.filter(|_| env.crisis).unwrap_or(def.file);
        let mut core = AreaCore::read(env.archive, def.tables, stem, def.light_anim, def.lights, def.fog)?;
        // The towns' tables are Infection's on every disc; the collision
        // takes the disc's square root.
        core.hits.volume = env.volume;
        core.lights.volume = env.volume;
        let file = core.file.clone();
        let mut cx = BuildCx {
            archive: env.archive,
            tables: def.tables,
            volume: env.volume,
            file: &file,
            stem: def.file,
            crisis: env.crisis,
            rng: env.rng,
            map: None,
            cc: None,
            field: None,
            made: Vec::new(),
            rooms: None,
        };
        let systems = Systems::build(def.systems, &mut cx)?;
        core.field_rand = def.systems.iter().any(SystemDef::field_rand).then_some(env.rng);
        let rows = systems.rows(&core.file);
        let start = def.start.unwrap_or(([0; 4], [0; 4]));
        Ok(Town { def, core, start, map: None, systems, frame: Vec::new(), rows, events: Vec::new() })
    }

    /// `StartPos` as the scene before left it: Kite's start where the def
    /// gives none (the sixth town's `SetCharPosition` has no case, QUA main
    /// 0x001b3ac0).
    pub fn keep_start(&mut self, before: (V4, V4)) {
        if self.def.start.is_none() {
            self.start = before;
        }
    }

    /// The systems, for tests and probes to read by kind.
    pub fn systems(&self) -> &Systems {
        &self.systems
    }

    /// `fieldrand` as the draws leave it; None for a town none of whose
    /// systems draws from it.
    pub fn field_rand(&self) -> Option<Rng> {
        self.core.field_rand
    }

    /// `fieldrand` set from outside (a gate menu's `SimGenerateCode`).
    pub fn set_field_rand(&mut self, rng: Rng) {
        if let Some(r) = &mut self.core.field_rand {
            *r = rng;
        }
    }

    /// The scroll statics as the game holds them, into each system.
    pub fn take_statics(&mut self, s: &[u32; piney_desktop::SCROLL_STATICS]) {
        self.systems.take(&Carried { statics: s, move_tex: None, field: None });
    }

    /// Each system's scroll statics written back.
    pub fn put_statics(&self, s: &mut [u32; piney_desktop::SCROLL_STATICS]) {
        self.systems.put(&mut CarriedMut { statics: s, move_tex: None, field: None });
    }

    /// The town's `Draw` for this frame: its def's steps in order.
    pub fn step(&mut self, v: &MapView) {
        self.frame.clear();
        // The group as this frame's characters see it: they draw (49, 50,
        // 64) before the town (96) steps its objects' anms.
        self.systems.object_lights(&mut self.core.lights.objects);
        self.systems.run(self.def.order, &mut self.core, v, &mut self.frame, &mut self.events);
        self.rows = self.systems.rows(&self.core.file);
    }

    /// What the last [`Town::step`] put in the frame, in its order.
    pub fn frame(&self) -> &[Drawn] {
        &self.frame
    }

    /// The last frame's pieces as generic draws.
    pub fn requests(&self) -> Vec<Request<'_>> {
        let cx = DrawCx { core: &self.core, fog: self.depth_fog(), rows: &self.rows };
        self.systems.requests(&self.frame, &cx)
    }

    /// What the draws started since the last call: effects and sounds.
    pub fn take_events(&mut self) -> Vec<MapEvent> {
        std::mem::take(&mut self.events)
    }

    /// The town's `SetFog` as VU1 fogs its buildings and objects (the sky,
    /// clouds and water have `SetFogSw(0)`).
    pub fn depth_fog(&self) -> draw::Fogging {
        draw::Fogging::Depth(self.def.fog.depth())
    }
}

/// `WORLD_MAN`'s calls on a Root Town (`ROOTTOWN0n`, area 0).
impl AreaMap for Town {
    fn core(&self) -> &AreaCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut AreaCore {
        &mut self.core
    }

    /// `SetCharPosition`'s town case (INF main 0x001a120c-0x001a13d4): Kite
    /// at the town's place, the others (+200, +100), (-200, +100) and
    /// (0, +300) from him facing as he does, as in a field.
    fn start(&self, scene: &crate::area::Scene) -> (V4, F, [V4; 3]) {
        let (pos, dirc) = self.start;
        (pos, dirc[2], crate::area::map::party_starts(scene.area, pos, dirc[2]))
    }

    fn clear(&self) -> Option<[u8; 3]> {
        self.def.clear
    }

    /// `fieldrand` as the draws leave it, and the scroll statics.
    fn put(&self, out: &mut piney_desktop::SaveState) {
        if let Some(r) = self.field_rand() {
            out.field_rand = r;
        }
        self.put_statics(&mut out.scroll_statics);
    }

    /// `ROOTTOWN0n::Draw`: its def's steps, its pieces painted, its
    /// `ccEff`s and what its steps started handed back.
    fn disp(&mut self, cx: &mut Disp) {
        self.step(&MapView::of(cx.camera, cx.player, cx.odd));
        for r in self.requests() {
            crate::systems::paint(cx.layers, cx.to_screen, &r, &mut cx.sprites);
        }
        cx.events.append(&mut self.events);
    }

    /// `ccEvMarker2Pos` reads the town's own file.
    fn marker_file(&self) -> Option<&SceneFile> {
        Some(&self.core.file)
    }

    fn setup_map(&mut self, archive: &Arc<Archive>, volume: Volume, _scene: &crate::area::Scene) -> Result<()> {
        if let Some(m) = self.def.minimap {
            self.map = Some(Box::new(crate::map::town::TownMap::open(archive, volume, m, &self.core.file.stem)?));
        }
        Ok(())
    }

    /// `ROOTTOWN0n::DrawMap`.
    fn minimap(&mut self, f: &mut MapFrame) -> bool {
        if let Some(map) = self.map.as_mut() {
            crate::map::town_minimap(map, f);
        }
        false
    }
}

/// `ccModel::SetUV` (0x0013abe0) of every material of `file`: the offset
/// less each material's crop, in U for water 1 and V for water 2.
pub(crate) fn water_rows(file: &SceneFile, k: usize, u: u16) -> HashMap<u32, [u8; 2]> {
    let mut rows = HashMap::new();
    for (&mat, m) in &file.scene.materials {
        let (du, dv) = if k == 1 { (u.wrapping_sub(m.crop_u), 0) } else { (0, u.wrapping_sub(m.crop_v)) };
        rows.insert(mat, [(du >> 4) as u8, (dv >> 4) as u8]);
    }
    rows
}

/// A dummy's position (w 1) and rotation in radians as `Decode_DummyPos` /
/// `Decode_DummyPosRot` (0x0014d570: `pi * deg / 180`) store them, and what
/// a constructor copies of them: the position for postype 1, both for 2,
/// zeros without a dummy.
pub fn dummy_bits(position: &Position, c: &Ccs, sc: &Scene) -> (V4, V4) {
    match position.find(c, sc) {
        Some(d) => {
            let pos = [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE];
            let rot = match (position, d.rot) {
                (Position::DummyRot(_), Some(r)) => {
                    let [x, y, z] = piney_data::anim::const_radians([r.x.to_bits(), r.y.to_bits(), r.z.to_bits()]);
                    [x, y, z, 0]
                }
                _ => [0; 4],
            };
            (pos, rot)
        }
        None => ([0; 4], [0; 4]),
    }
}

/// `ccModel::Draw`'s morph of an anm's `entry`: the targets and weights
/// `morph` (this frame's, by morpher) holds for the entry's own modifier,
/// the morpher its Obj2 chunk names (`Decode_ExtObj` keeps it for a copy,
/// INF 0x0014cb80). Copies of one model morph each by their own: the
/// ending's book draws `MDL_xdhdpag_a0` as three pages, each its own flip.
pub fn object_morph(file: &SceneFile, morph: &[(u32, Vec<(u32, f32)>)], entry: u32) -> Vec<(u32, f32)> {
    let Some(m) = file.scene.modifier.get(&entry) else { return Vec::new() };
    morph.iter().find(|(mph, _)| mph == m).map(|(_, t)| t.clone()).unwrap_or_default()
}

/// What `ccAnm::Draw` draws of `play` under `root`, as (entry, piece,
/// world): each index entry at its own place (an ExtObj copy draws its
/// piece again, [`Play::instances`]), then each object of `extra` no entry
/// names, by piece and in the index's order within one.
pub(crate) fn anm_pieces(file: &SceneFile, play: &Play, root: Mat4, extra: &[u32]) -> Vec<(u32, u32, Mat4)> {
    let mut out: Vec<(u32, u32, Mat4)> =
        play.instances(file, root).into_iter().map(|i| (i.entry, i.object, i.world)).collect();
    let named: std::collections::HashSet<u32> = out.iter().map(|p| p.1).collect();
    let rest = play.worlds(file, root, extra).into_iter().filter(|(o, _)| !named.contains(o));
    out.extend(rest.map(|(o, w)| (o, o, w)));
    out.sort_by_key(|p| p.1);
    out
}

/// `ccAnm::Draw`: every object the animation poses that has a model, under
/// `root`, each morphed by its own modifier ([`object_morph`]); `rows`
/// overrides the animation's own texture offsets.
#[allow(clippy::too_many_arguments)]
pub fn anim_draw(
    layers: &mut Layers,
    to_screen: Mat4,
    file: &SceneFile,
    play: &Play,
    root: Mat4,
    extra: &[u32],
    alpha: f32,
    rows: Option<&HashMap<u32, [u8; 2]>>,
) {
    anim_draw_on(layers, OBJ_LAYER, to_screen, file, play, root, extra, alpha, rows, None);
}

/// [`anim_draw`] on `layer`, with `lights` for the lit models (mtype bit
/// 0) at their world matrices.
#[allow(clippy::too_many_arguments)]
pub fn anim_draw_on(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    file: &SceneFile,
    play: &Play,
    root: Mat4,
    extra: &[u32],
    alpha: f32,
    rows: Option<&HashMap<u32, [u8; 2]>>,
    lights: Option<&dyn Fn(Mat4) -> piney_draw::Lights>,
) {
    anim_draw_fog(layers, layer, to_screen, file, play, root, extra, alpha, rows, lights, draw::Fogging::None);
}

/// [`anim_draw_on`] with the models' fog.
#[allow(clippy::too_many_arguments)]
pub fn anim_draw_fog(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    file: &SceneFile,
    play: &Play,
    root: Mat4,
    extra: &[u32],
    alpha: f32,
    rows: Option<&HashMap<u32, [u8; 2]>>,
    lights: Option<&dyn Fn(Mat4) -> piney_draw::Lights>,
    fog: draw::Fogging,
) {
    anim_draw_blended(layers, layer, to_screen, file, play, root, extra, alpha, rows, lights, fog, None);
}

/// [`anim_draw_fog`] with the anm's blend (`ccAnm::SetBlendType(t)`).
#[allow(clippy::too_many_arguments)]
pub fn anim_draw_blended(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    file: &SceneFile,
    play: &Play,
    root: Mat4,
    extra: &[u32],
    alpha: f32,
    rows: Option<&HashMap<u32, [u8; 2]>>,
    lights: Option<&dyn Fn(Mat4) -> piney_draw::Lights>,
    fog: draw::Fogging,
    blend: Option<u8>,
) {
    let own = play.uv_rows(file);
    let rows = rows.unwrap_or(&own);
    let morph = play.morph(file);
    for (entry, obj, world) in anm_pieces(file, play, root, extra) {
        let target = file.scene.ext.get(&obj).copied().unwrap_or(obj);
        let Some(&model) = file.obj_model.get(&target) else { continue };
        let m = object_morph(file, &morph, entry);
        let lit = file.models.get(&model).is_some_and(|i| i.mtype & 1 != 0);
        let lights = lights.filter(|_| lit).map(|l| l(world));
        let d = Draw { file, model, world, alpha, rows, lights, nodes: &[], morph: m, clut_swaps: Vec::new() };
        draw::model_blended(layers, layer, to_screen, &d, None, fog, &[], blend);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn root_is_translated_rotation() {
        let pos = [1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits(), ONE];
        let m = piney_data::vu0::pos_rot_zyx(pos, [0; 4]);
        assert_eq!(m[3], pos);
        assert_eq!(m[0], [ONE, 0, 0, 0]);
    }

    pub(crate) fn archive() -> Option<Arc<Archive>> {
        archive_of("infection")
    }

    /// The archive of `work/{disc}/{disc}.iso`, if it is there.
    pub(crate) fn archive_of(disc: &str) -> Option<Arc<Archive>> {
        let iso = format!("../../work/{disc}/{disc}.iso");
        if !std::path::Path::new(&iso).exists() {
            eprintln!("skipped: no {iso}");
            return None;
        }
        let mut iso = piney_data::iso::Iso::open(&iso).unwrap();
        Some(Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap()))
    }

    /// `ANM_sr1town1a`'s light records are controllers by flag: the omni
    /// lights (flags 0x9041) carry no intensity, which is then 1, and fall
    /// off from 600 to 800; the distant light (0x48) a rotation and a
    /// colour.
    #[test]
    fn town_lights() {
        let Some(archive) = archive() else { return };
        let t = Town::open(&archive, piney_data::volume::Volume::Inf, 0, false).unwrap();
        let l = &t.core.lights;
        assert_eq!(l.ambient, piney_data::light::ambient(0x0078_7171));
        assert_eq!(l.lights.len(), 6);
        let sun = &l.lights[0];
        assert!(
            matches!(sun.kind, piney_data::light::Kind::Distant { .. }) && sun.intensity == ONE && sun.priority == -1
        );
        let omni: Vec<([f32; 3], F, f32, f32)> = l.lights[1..]
            .iter()
            .map(|o| match o.kind {
                piney_data::light::Kind::Omni { pos, far } => {
                    ([0, 1, 2].map(|k| ee::f(pos[k])), o.intensity, ee::f(far.start), ee::f(far.end))
                }
                _ => panic!("{o:?}"),
            })
            .collect();
        assert_eq!(omni[0], ([2400.0, -2300.0, 0.0], ONE, 600.0, 800.0));
        assert_eq!(omni[4], ([-800.0, 3600.0, 300.0], ONE, 600.0, 800.0));
        assert_eq!(l.lights[1].colour, piney_data::light::colour_of(0x0094_dafa, ONE));
    }

    /// The Aura shrine's light (`LGT_se1_3lig1` of `ANM_se1_3_1a`, record
    /// 0x0605, flags 0x240041) is a direct light: its place (0, 0, 2500),
    /// white, no intensity (so 1), and the radii 498 and 500. At
    /// the floor under it the beam is full; halfway through its edge, half;
    /// outside it or above the light, nothing.
    #[test]
    fn the_shrine_has_a_direct_light() {
        let Some(archive) = archive() else { return };
        let file = SceneFile::read(&archive, "se1_3").unwrap();
        let ai = file.anim("ANM_se1_3_1a").unwrap();
        let obj = file.ccs.find_object("LGT_se1_3lig1").unwrap();
        let (_, lights) = anim_lights(&file, ai).unwrap();
        let l = lights.iter().find(|l| l.object == obj).unwrap().at(256, &piney_data::vu0::UNIT);
        let piney_data::light::Kind::Direct { matrix, radius, .. } = l.kind else { panic!("{l:?}") };
        assert_eq!((l.priority, l.intensity, ee::f(radius.start), ee::f(radius.end)), (0, ONE, 498.0, 500.0));
        assert_eq!(matrix[3].map(ee::f), [0.0, 0.0, 2500.0, 1.0]);
        assert_eq!(l.colour, piney_data::light::colour_of(0x00ff_ffff, ONE));
        let lit = |at: [f32; 3]| {
            let group = TownLights { lights: vec![l], ..TownLights::default() };
            let world = glam::Mat4::from_translation(at.into());
            crate::chara::kite::light_matrix(&group, world, false).colours[2][0]
        };
        let full = lit([0.0; 3]);
        assert!(full > 0.0);
        assert!((lit([0.0, 499.0, 0.0]) - full / 2.0).abs() < 1e-3);
        assert_eq!(lit([501.0, 0.0, 0.0]), 0.0);
        assert_eq!(lit([0.0, 0.0, 2600.0]), 0.0);
    }

    /// Issue #75: the ending's book (`ANM_book_anm2`, Quarantine's
    /// `ending`) draws `MDL_xdhdpag_a0` as three pages (a0 and the ExtObj
    /// copies d and e) and `MDL_xdhdpag_b0` as two (b0, f). Each page bends
    /// by its own Obj2 modifier: three pages turning apart, where pairing a
    /// morpher with its base model summed all three flips into each page.
    #[test]
    fn book_pages_morph_by_their_own_morphers() {
        let Some(archive) = archive_of("quarantine") else { return };
        let file = SceneFile::read(&archive, "ending").unwrap();
        let mut play = Play::new(&file, "ANM_book_anm2").unwrap();
        while play.frame() < 200 {
            play.forward(&file);
        }
        let morph = play.morph(&file);
        let name = |o: u32| file.ccs.object_name(o).unwrap_or_default().to_string();
        let draws_a0 = |o: u32| {
            let target = file.scene.ext.get(&o).copied().unwrap_or(o);
            file.obj_model.get(&target).is_some_and(|&m| name(m) == "MDL_xdhdpag_a0")
        };
        let objs: Vec<(u32, Vec<(u32, f32)>)> = anm_pieces(&file, &play, Mat4::IDENTITY, &[])
            .into_iter()
            .filter(|&(_, o, _)| draws_a0(o))
            .map(|(e, _, _)| (e, object_morph(&file, &morph, e)))
            .collect();
        let mut pages: Vec<(String, Vec<(u32, f32)>)> = objs.iter().map(|(o, m)| (name(*o), m.clone())).collect();
        pages.sort_by(|a, b| a.0.cmp(&b.0));
        let names: Vec<&str> = pages.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["OBJ_xdhdpag_a0", "OBJ_xdhdpag_d", "OBJ_xdhdpag_e"]);
        for (o, m) in &objs {
            let mph = file.scene.modifier[o];
            assert_eq!(name(mph), name(*o).replace("OBJ_", "MPH_"), "its Obj2's modifier");
            let own = morph.iter().find(|(k, _)| *k == mph).map(|(_, t)| t.clone()).unwrap_or_default();
            assert!(!own.is_empty() && m == &own, "{}: its own morpher's targets", name(*o));
        }
        assert!(pages[0].1 != pages[1].1 && pages[1].1 != pages[2].1, "the three pages turn apart");
    }
}
