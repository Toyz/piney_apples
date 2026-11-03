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
pub mod ride;

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::classes::TownGameClass;
use piney_data::dungeon::Rng;
use piney_data::scene::Scene;
use piney_data::statics::Position;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::area::AreaCore;
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

/// A light of the town's group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    /// 1 distant, 2 direct, 4 omni (`ccLight.type`).
    pub kind: i16,
    /// Omni and direct: position. Distant and direct: the direction it
    /// travels (`lwMatrix` times (0, 0, -1)).
    pub pos: Vec3,
    pub dir: Vec3,
    /// Colour 0..1 before the intensity.
    pub colour: Vec3,
    pub intensity: f32,
    /// Omni fall-off: full to `far_start`, nothing past `far_end`. Direct:
    /// the same along its beam (`+0x144`, `+0x148`; 0 for no end).
    pub far_start: f32,
    pub far_end: f32,
    /// Direct: the beam's radius, full to the first (`+0x14c`), nothing
    /// past the second (`+0x150`).
    pub radius: [f32; 2],
    /// `ccLight.priority`: distant -1, omni and direct 0 (`ccCreateLight`).
    pub priority: i8,
}

/// The lights and ambient `ROOTTOWN01` sets up from `ANM_sr1town1a`: its
/// controllers at frame 1 (`SetAnm` then one `_AnimateForward`, never
/// again).
#[derive(Clone, Debug, PartialEq)]
pub struct TownLights {
    pub ambient: Vec3,
    /// In `town00Light`'s order (the group keeps descending priority).
    pub lights: Vec<Light>,
    /// The draw environment's fog (`cc3d->SetFog`'s arguments), which VU1
    /// puts on every model drawn with PRIM.FGE (`SetFogSw` on: the
    /// characters, a town's buildings) by each vertex's depth, and the
    /// effects' sprites take at their centre; None, none.
    pub fog: Option<FogParams>,
}

impl TownLights {
    /// [`TownLights::fog`] as VU1 fogs a model by depth.
    pub fn depth_fog(&self) -> Option<piney_draw::DepthFog> {
        self.fog.map(|f| f.depth())
    }
}

/// Little-endian reads that fail on short data.
struct Words<'a>(&'a [u8]);

impl Words<'_> {
    fn u32_at(&self, at: usize) -> Result<u32> {
        self.0
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
            .ok_or_else(|| Error::Format(format!("short read at 0x{at:x}")))
    }
}

/// One controller of an Anime chunk's light record, as
/// `ccAnmChunk::ConvLCNum2ALCNum` (0x00144e90) reads it: kind 0 absent (the
/// default), 1 one value, 2 keyed (`u32 n`, then n x (u32 frame, value)).
#[derive(Clone, Debug, PartialEq)]
enum LightCtrl<const N: usize> {
    None,
    Const([u32; N]),
    Keyed(Vec<(u32, [u32; N])>),
}

impl<const N: usize> LightCtrl<N> {
    fn read(d: &Words, q: &mut usize, kind: u32) -> Result<Self> {
        let vals = |at: usize| -> Result<[u32; N]> {
            let mut v = [0; N];
            for (k, x) in v.iter_mut().enumerate() {
                *x = d.u32_at(at + 4 * k)?;
            }
            Ok(v)
        };
        Ok(match kind {
            0 => LightCtrl::None,
            1 => {
                let v = vals(*q)?;
                *q += 4 * N;
                LightCtrl::Const(v)
            }
            2 => {
                let n = d.u32_at(*q)? as usize;
                *q += 4;
                let mut keys = Vec::with_capacity(n);
                for _ in 0..n {
                    keys.push((d.u32_at(*q)?, vals(*q + 4)?));
                    *q += 4 + 4 * N;
                }
                LightCtrl::Keyed(keys)
            }
            k => return Err(Error::Format(format!("light controller kind {k} at 0x{:x}", *q))),
        })
    }

    /// The keys around `time` (1/256 frames) and how far between them.
    fn span(&self, time: u32) -> Option<([u32; N], [u32; N], f32)> {
        match self {
            LightCtrl::None => None,
            LightCtrl::Const(v) => Some((*v, *v, 0.0)),
            LightCtrl::Keyed(k) => {
                let t = time as f32 / 256.0;
                let i = k.iter().rposition(|(f, _)| (*f as f32) <= t).unwrap_or(0);
                let (f0, v0) = k[i];
                match k.get(i + 1) {
                    Some(&(f1, v1)) if f1 > f0 => Some((v0, v1, ((t - f0 as f32) / (f1 - f0) as f32).clamp(0.0, 1.0))),
                    _ => Some((v0, v0, 0.0)),
                }
            }
        }
    }

    /// Floats: linear between keys (`ccAnmCtrlFloat_Set`, `FVec3_Set`).
    fn floats(&self, time: u32, default: [f32; N]) -> [f32; N] {
        match self.span(time) {
            None => default,
            Some((a, b, t)) => std::array::from_fn(|k| {
                let (a, b) = (f32::from_bits(a[k]), f32::from_bits(b[k]));
                a + (b - a) * t
            }),
        }
    }
}

impl LightCtrl<1> {
    /// A colour (`ccAnmCtrlColor_Set`: `ccSetBlendColor` of the two keys,
    /// each channel over 255).
    fn colour(&self, time: u32) -> Vec3 {
        let col = |c: u32| Vec3::new((c & 0xff) as f32, ((c >> 8) & 0xff) as f32, ((c >> 16) & 0xff) as f32) / 255.0;
        match self.span(time) {
            None => Vec3::ONE,
            Some((a, b, t)) => col(a[0]) * (1.0 - t) + col(b[0]) * t,
        }
    }
}

/// A light record of an Anime chunk: 0x0603 a distant light (flags: bits
/// 3-5 rotation in degrees, 6-8 colour, 9-11 intensity), 0x0605 a direct
/// light (bits 0-2 position, 3-5 rotation, 6-8 colour, 9-11 intensity,
/// 12-14 and 15-17 the beam's start and end, 18-20 and 21-23 its radii) or
/// 0x0609 an omni (bits 0-2 position, 6-8 colour, 9-11 intensity, 12-14
/// far start, 15-17 far end); `SetAnmCtrlWork` (0x00150670) defaults an
/// absent intensity to 1 and the other floats to 0.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimLight {
    /// The LGT_ object.
    pub object: u32,
    /// 1 distant, 2 direct, 4 omni.
    kind: i16,
    pos: LightCtrl<3>,
    rot: LightCtrl<3>,
    colour: LightCtrl<1>,
    intensity: LightCtrl<1>,
    far_start: LightCtrl<1>,
    far_end: LightCtrl<1>,
    radius: [LightCtrl<1>; 2],
}

impl AnimLight {
    /// The light at `time` in the animation's own space.
    pub fn at(&self, time: u32) -> Light {
        let [intensity] = self.intensity.floats(time, [1.0]);
        let colour = self.colour.colour(time);
        let rot = match &self.rot {
            LightCtrl::None => [0; 3],
            LightCtrl::Const(v) => piney_data::anim::const_radians(*v),
            LightCtrl::Keyed(_) => {
                let r = self.rot.floats(time, [0.0; 3]);
                piney_data::anim::const_radians(r.map(f32::to_bits))
            }
        };
        // lightVector (main 0x002fb100) turned by the light's matrix.
        let dir = draw::mat(&piney_data::anim::rot_bits(rot)).transform_vector3(Vec3::new(0.0, 0.0, -1.0));
        let [far_start] = self.far_start.floats(time, [0.0]);
        let [far_end] = self.far_end.floats(time, [0.0]);
        let radius = self.radius.each_ref().map(|r| r.floats(time, [0.0])[0]);
        let pos = Vec3::from(self.pos.floats(time, [0.0; 3]));
        match self.kind {
            1 => Light {
                kind: 1,
                pos: Vec3::ZERO,
                dir,
                colour,
                intensity,
                far_start: 0.0,
                far_end: 0.0,
                radius: [0.0; 2],
                priority: -1,
            },
            2 => Light { kind: 2, pos, dir, colour, intensity, far_start, far_end, radius, priority: 0 },
            _ => Light {
                kind: 4,
                pos,
                dir: Vec3::ZERO,
                colour,
                intensity,
                far_start,
                far_end,
                radius: [0.0; 2],
                priority: 0,
            },
        }
    }
}

/// An Anime chunk's ambient (0x0601 records: the last at or before frame
/// 1) and its light records.
pub fn anim_lights(file: &SceneFile, anim: usize) -> Result<(Option<Vec3>, Vec<AnimLight>)> {
    let d = Words(&file.ccs.data);
    let off = file.anims[anim].offset;
    let words = d.u32_at(off + 16)? as usize;
    let (mut p, end) = (off + 20, off + 20 + 4 * words);
    let mut frame = 0u32;
    let mut ambient = None;
    let mut lights: Vec<AnimLight> = Vec::new();
    let col = |c: u32| Vec3::new((c & 0xff) as f32, ((c >> 8) & 0xff) as f32, ((c >> 16) & 0xff) as f32) / 255.0;
    while p < end {
        let kind = d.u32_at(p)? as u16;
        let n = d.u32_at(p + 4)? as usize;
        let mut q = p + 8;
        match kind {
            0xff01 => frame = d.u32_at(q)?,
            0x0601 if frame <= 1 => ambient = Some(col(d.u32_at(q)?)),
            0x0603 | 0x0605 | 0x0609 => {
                let object = d.u32_at(q)?;
                let flags = d.u32_at(q + 4)?;
                q += 8;
                let kind = match kind {
                    0x0603 => 1,
                    0x0605 => 2,
                    _ => 4,
                };
                let field = |shift: u32| (flags >> shift) & 7;
                let pos = if kind == 1 { LightCtrl::None } else { LightCtrl::read(&d, &mut q, field(0))? };
                let rot = if kind == 4 { LightCtrl::None } else { LightCtrl::read(&d, &mut q, field(3))? };
                let colour = LightCtrl::read(&d, &mut q, field(6))?;
                let intensity = LightCtrl::read(&d, &mut q, field(9))?;
                let (far_start, far_end) = if kind == 1 {
                    (LightCtrl::None, LightCtrl::None)
                } else {
                    (LightCtrl::read(&d, &mut q, field(12))?, LightCtrl::read(&d, &mut q, field(15))?)
                };
                let radius = if kind == 2 {
                    [LightCtrl::read(&d, &mut q, field(18))?, LightCtrl::read(&d, &mut q, field(21))?]
                } else {
                    [LightCtrl::None, LightCtrl::None]
                };
                if q > p + 8 + 4 * n {
                    return Err(Error::Format(format!("light record at 0x{p:x} overruns")));
                }
                lights.retain(|l| l.object != object);
                lights.push(AnimLight { object, kind, pos, rot, colour, intensity, far_start, far_end, radius });
            }
            _ => {}
        }
        p += 8 + 4 * n;
    }
    Ok((ambient, lights))
}

/// An Anime chunk's lights at frame 1 in the order of the class's light
/// table (`town00Light`, `town02Light`: the distant light, then the
/// omnis), and its ambient.
pub(crate) fn read_lights(file: &SceneFile, anim: usize, table: &[(i32, &str)]) -> Result<TownLights> {
    let (ambient, lights) = anim_lights(file, anim)?;
    let mut out = Vec::new();
    for (_, name) in table {
        let obj = file.ccs.find_object(name).ok_or_else(|| Error::NotFound((*name).into()))?;
        if let Some(l) = lights.iter().find(|l| l.object == obj) {
            out.push(l.at(256));
        }
    }
    Ok(TownLights { ambient: ambient.unwrap_or(Vec3::ZERO), lights: out, fog: None })
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
    /// Dummies whose rotation the game wrote (from Mutation on,
    /// `setMerchant`'s Event NPC's), by name: the markers read them so for
    /// as long as the town's file stays loaded, across a scene change to
    /// the same town too (the session carries them).
    pub written: Vec<(String, V4)>,
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
        };
        let systems = Systems::build(def.systems, &mut cx)?;
        core.field_rand = def.systems.iter().any(SystemDef::field_rand).then_some(env.rng);
        let rows = systems.rows(&core.file);
        Ok(Town { def, core, written: Vec::new(), systems, frame: Vec::new(), rows, events: Vec::new() })
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
pub fn dummy_bits(position: &Position, c: &Ccs, sc: &Scene) -> (V4, [F; 3]) {
    match position.find(c, sc) {
        Some(d) => {
            let pos = [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE];
            let rot = match (position, d.rot) {
                (Position::DummyRot(_), Some(r)) => {
                    piney_data::anim::const_radians([r.x.to_bits(), r.y.to_bits(), r.z.to_bits()])
                }
                _ => [0; 3],
            };
            (pos, rot)
        }
        None => ([0; 4], [0; 3]),
    }
}

/// `ccCoord::SetMatrix_PosRotZYX(pos, rot)` (0x001382f0) as it stores the
/// coordinate's matrix: `sceVu0RotMatrix` of the unit matrix (z, then y,
/// then x), then `sceVu0TransMatrix` adds the position.
pub fn pos_rot_zyx(pos: V4, rot: [F; 3]) -> [V4; 4] {
    let mut m = piney_data::anim::rot_bits(rot);
    for (k, &p) in pos.iter().take(3).enumerate() {
        m[3][k] = ee::add(m[3][k], p);
    }
    m
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
        let m = pos_rot_zyx(pos, [0; 3]);
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
        assert_eq!(l.ambient, Vec3::new(113.0, 113.0, 120.0) / 255.0);
        assert_eq!(l.lights.len(), 6);
        assert_eq!((l.lights[0].kind, l.lights[0].intensity, l.lights[0].priority), (1, 1.0, -1));
        let omni: Vec<(Vec3, f32, f32, f32)> =
            l.lights[1..].iter().map(|o| (o.pos, o.intensity, o.far_start, o.far_end)).collect();
        assert_eq!(omni[0], (Vec3::new(2400.0, -2300.0, 0.0), 1.0, 600.0, 800.0));
        assert_eq!(omni[4], (Vec3::new(-800.0, 3600.0, 300.0), 1.0, 600.0, 800.0));
        assert_eq!(l.lights[1].colour, Vec3::new(250.0, 218.0, 148.0) / 255.0);
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
        let l = lights.iter().find(|l| l.object == obj).unwrap().at(256);
        assert_eq!((l.kind, l.priority, l.intensity, l.radius), (2, 0, 1.0, [498.0, 500.0]));
        assert_eq!((l.pos, l.dir, l.colour), (Vec3::new(0.0, 0.0, 2500.0), Vec3::NEG_Z, Vec3::ONE));
        let lit = |at: Vec3| {
            let group = TownLights { ambient: Vec3::ZERO, lights: vec![l], fog: None };
            crate::chara::kite::light_matrix(&group, glam::Mat4::from_translation(at), false).colours[2][0]
        };
        let full = lit(Vec3::ZERO);
        assert!(full > 0.0);
        assert!((lit(Vec3::new(0.0, 499.0, 0.0)) - full / 2.0).abs() < 1e-3);
        assert_eq!(lit(Vec3::new(501.0, 0.0, 0.0)), 0.0);
        assert_eq!(lit(Vec3::new(0.0, 0.0, 2600.0)), 0.0);
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
