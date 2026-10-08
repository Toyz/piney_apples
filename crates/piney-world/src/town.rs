//! The Root Towns as `ccThFieldDisp` (priority 96) draws them each frame, and
//! what else their constructors set up: the collision mesh, the lights and
//! the fog. Each town is a class of its own in the game (`ROOTTOWN01` .. `05`);
//! here [`Base`] holds what every constructor builds the same way, each class
//! is a module implementing [`RootTown`] (town01.rs .. town05.rs), and
//! [`Town`] is the base and its class. A class's `select` is its `Draw`'s
//! choice of pieces for the frame; [`RootTown::draw`] draws them
//! (docs/engine/statics.md).

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::scene::Scene;
use piney_data::statics::{self, DrawPass, ModelTable, ObjTable, PlacedModel, Position};
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::draw::{self, Draw, OBJ_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::evarea::FlareCamera;
use crate::hit::{HitModel, Hits};
use crate::pose::Play;

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
    /// The draw environment's fog (`ccDrawEnv::SetFog`), which VU1 puts on
    /// every model drawn with PRIM.FGE (`SetFogSw` on: the characters, a
    /// town's buildings) by each vertex's depth; None, none.
    pub fog: Option<piney_draw::DepthFog>,
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

/// One `STATICOBJECT`.
#[derive(Clone, Debug)]
pub(crate) struct Object {
    /// Its `RT_OBJTABLE` row.
    pub(crate) row: usize,
    pub(crate) pass: DrawPass,
    pub(crate) play: Play,
    /// `ccCoord::SetMatrix_PosRotZYX(pos, rot)` of its dummy, as stored.
    pub(crate) root: [V4; 4],
    /// +0x10: the dummy's position (zero without one).
    pub(crate) pos: V4,
    pub(crate) clip: F,
}

/// What a town's `Draw` starts besides its drawing, for the host: effects
/// and sounds, in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TownEvent {
    /// `effSmokeN(pos, v, scale, life, kind, fade in, fade out)`: a puff of
    /// smoke not faded by distance (the particle's `distSW` cleared).
    SmokeN { pos: V4, v: V4, scale: F, life: i32, kind: i32, fade: (i16, i16) },
    /// `ccSeOn3D(n, pos)`.
    Se3d { n: i32, pos: V4 },
}

/// What a town's `Draw` reads of the rest of the field.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TownView {
    /// `cameraGetPos(camID)`.
    pub eye: V4,
    /// The player's `pos` (`plw`): the clouds gather about it and
    /// `ccCheckCameraDeg` and `W2P` measure from it.
    pub player: V4,
    /// `activeCamPtr`, which `ccCheckCameraDeg` reads.
    pub cam: crate::camera::Cam,
    /// What `LENSFLARE::Draw` reads of the camera.
    pub flare: FlareCamera,
    /// `eventMng.puppetShow` (+0x78c).
    pub puppet_show: bool,
    /// The field view's `world_screen` as the camera left it, which
    /// `waterUVModifi2` projects the water through (sysLayer's view +0xd0).
    pub world_screen: [V4; 4],
}

impl TownView {
    /// A view with only the camera eye (all Mac Anu's `Draw` reads).
    pub fn at(eye: V4) -> TownView {
        TownView { eye, ..TownView::default() }
    }
}

/// A `ccEff` a town's `Draw` draws (the lens flare, the clouds, Lia Fail's
/// glows), for the host's effects: `ccEff::Draw(pos, pattern)` of
/// `file`'s `eff` on `layer` as `Init(chunk, 1)` left it, with the scale,
/// turn and transparency given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TownSprite {
    pub file: &'static str,
    pub eff: &'static str,
    pub pos: V4,
    pub pattern: u16,
    pub layer: i16,
    pub scale: [F; 2],
    pub rotate: F,
    pub transparency: F,
    /// Whether it keeps `Init`'s depth test and fog. `LENSFLARE`'s and
    /// `CLOUD`'s constructors turn both off (`SetRenderState(CCRS_ZENABLE,
    /// 0)` and PRIM's fog bit); `ROOTTOWN05`'s glows keep them, so the
    /// buildings hide them.
    pub depth: bool,
}

/// What every Root Town class's constructor builds the same way from its
/// [`Spec`].
pub struct Base {
    /// `ChangeScene`'s town number (`game.town`): 0 Mac Anu .. 4 Lia Fail.
    pub no: i32,
    /// `town01` .. `town05`, or a `d` variant in crisis.
    pub file: Rc<SceneFile>,
    /// The file's morphers (morpher object to the model it bends).
    pub(crate) morphers: HashMap<u32, u32>,
    pub(crate) table: &'static ModelTable,
    /// By row: where each static model draws.
    pub(crate) models: Vec<Option<PlacedModel>>,
    pub(crate) objects: Vec<Object>,
    pub lights: TownLights,
    pub fog: FogParams,
    pub hits: Hits,
    /// `ccSys.bgColor` (+0x18) as the constructor set it: None keeps
    /// `ccSetupGameCtrl`'s black.
    pub clear: Option<[u8; 3]>,
    /// What the draws started for the host ([`Base::take_events`]).
    pub(crate) events: Vec<TownEvent>,
    /// Dummies whose rotation the game wrote (from Mutation on,
    /// `setMerchant`'s Event NPC's), by name: the markers read them so for
    /// as long as the town's file stays loaded, across a scene change to
    /// the same town too (the session carries them).
    pub written: Vec<(String, V4)>,
}

/// A Root Town class: what `ROOTTOWN01` .. `05` hold besides the [`Base`],
/// and their `Draw` (virtual in the game, through each class's vtable).
pub trait RootTown: Any {
    /// The class's `Draw` for this frame into the field's layers through
    /// `to_screen`, stepping what it steps; the `ccEff`s it draws (the lens
    /// flare, the clouds, Lia Fail's glows) are handed back for the host's
    /// effects to draw.
    fn draw(&mut self, base: &mut Base, layers: &mut Layers, to_screen: Mat4, v: &TownView) -> Vec<TownSprite>;

    /// `waterAnm[k]`'s animation time (0 in a town without water).
    fn water_time(&self, _k: usize) -> u32 {
        0
    }

    /// Water 0 as the last draw left it (None in a town without water).
    fn water0(&self) -> Option<&WaterZero> {
        None
    }
}

/// A Root Town as the field holds it: the base and its class.
pub struct Town {
    pub base: Base,
    class: Box<dyn RootTown>,
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
/// unbent vertex would be (docs/engine/town02.md).
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
    // fptodp, sqrt, dptofp; sqrt.s from Outbreak on (OUT gcmn 0x00518170).
    let d = ee::dsqrt_on(volume, ee::add(ee::mul(dx, dx), ee::mul(dy, dy)));
    if !ee::le(d, reach) {
        return None;
    }
    let scale = ee::div(model.scale.to_bits(), K4096);
    let mm = model.mmats.first()?;
    Some(
        mm.positions
            .iter()
            .map(|p| {
                let v = p.map(|c| ee::mul(ee::from_int(i32::from(c)), scale));
                let q = ee::apply(lw, [v[0], v[1], v[2], ONE]);
                let r = ee::rot_trans_pers(world_screen, q);
                let (sx, sy) = (ee::from_int(r[0] / 16), ee::from_int(r[1] / 16));
                let u = ee::div(ee::sub(sx, SCREEN_X), SCREEN_W);
                let v = ee::div(ee::sub(sy, SCREEN_Y), SCREEN_H);
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
        let tex = piney_draw::TexRef::FrameBuffer {
            x: 0,
            y: 0,
            width: piney_draw::SCREEN_WIDTH,
            height: piney_draw::SCREEN_HEIGHT,
        };
        std::sync::Arc::new(piney_draw::VertexEdits { st, tex: Some(tex), ..Default::default() })
    }
}

/// Water 0's `ccAnm::Draw` on `layer`: every object its animation poses,
/// the water's model with its edits.
#[allow(clippy::too_many_arguments)]
pub(crate) fn water0_draw(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    (file, morphers): (&SceneFile, &HashMap<u32, u32>),
    play: &Play,
    root: Mat4,
    water: &WaterZero,
) {
    let worlds = play.worlds(file, root, &[]);
    let rows = play.uv_rows(file);
    let morph = play.morph(file);
    let edits = water.edits();
    let mut objs: Vec<(&u32, &Mat4)> = worlds.iter().collect();
    objs.sort_by_key(|(o, _)| **o);
    for (&obj, &world) in objs {
        let target = file.scene.ext.get(&obj).copied().unwrap_or(obj);
        let Some(&model) = file.obj_model.get(&target) else { continue };
        let m: Vec<(u32, f32)> = morph
            .iter()
            .filter(|(mph, _)| morphers.get(mph) == Some(&model))
            .flat_map(|(_, t)| t.iter().copied())
            .collect();
        let e = (model == water.model.object).then_some(&edits);
        let d = Draw {
            file,
            model,
            world,
            alpha: 1.0,
            rows: &rows,
            lights: None,
            nodes: &[],
            morph: m,
            clut_swaps: Vec::new(),
        };
        draw::model_edited(layers, layer, to_screen, d, e, draw::Fogging::None);
    }
}

/// A class's number, tables, light animation, fog and clear colour, as its
/// constructor names them.
pub(crate) struct Spec {
    pub(crate) no: i32,
    pub(crate) models: &'static str,
    /// The class's `RT_OBJTABLE`, None for a class with no static objects.
    pub(crate) objects: Option<&'static str>,
    pub(crate) light_anim: &'static str,
    pub(crate) lights: &'static [(i32, &'static str)],
    pub(crate) fog: FogParams,
    pub(crate) clear: Option<[u8; 3]>,
}

impl Base {
    /// The part of a constructor every class shares: the scene file
    /// `stem`, the static models and objects of its tables
    /// (`STATICMODEL`, `STATICOBJECT`), the light animation's lights at
    /// frame 1, the collision mesh, the fog and the clear colour.
    pub(crate) fn read(archive: &Arc<Archive>, stem: &str, spec: &Spec) -> Result<Base> {
        Base::read_volume(archive, piney_data::volume::Volume::Inf, stem, spec)
    }

    /// [`Base::read`] with `volume`'s tables (the story maps').
    pub(crate) fn read_volume(
        archive: &Arc<Archive>,
        volume: piney_data::volume::Volume,
        stem: &str,
        spec: &Spec,
    ) -> Result<Base> {
        let file = Rc::new(SceneFile::read(archive, stem)?);
        let (models_of, objs_of, _) = statics::of(volume);
        let table =
            models_of.iter().find(|t| t.name == spec.models).ok_or_else(|| Error::NotFound(spec.models.into()))?;
        let obj_table: Option<&ObjTable> = spec
            .objects
            .map(|name| objs_of.iter().find(|t| t.name == name).ok_or_else(|| Error::NotFound(name.into())))
            .transpose()?;
        let (c, sc) = (&file.ccs, &file.scene);
        let mut models = vec![None; table.rows.len()];
        for p in table.place(c, sc) {
            models[p.row] = Some(p);
        }
        let mut objects = Vec::new();
        for (row, r) in obj_table.map_or(&[][..], |t| t.rows).iter().enumerate() {
            // STATICOBJECT makes only the ccAnm when the row names one.
            let Some(play) = r.anime.and_then(|n| Play::new(&file, n)) else { continue };
            let (pos, rot) = dummy_bits(&r.position, c, sc);
            let root = pos_rot_zyx(pos, rot);
            objects.push(Object { row, pass: r.pass, play, root, pos, clip: r.clip.to_bits() });
        }
        let light_anim = file.anim(spec.light_anim).ok_or_else(|| Error::NotFound(spec.light_anim.into()))?;
        let lights = read_lights(&file, light_anim, spec.lights)?;
        let hits = Hits::new(volume, HitModel::read(volume, c)?);
        let morphers = piney_data::anim::morphers(c).unwrap_or_default();
        Ok(Base {
            no: spec.no,
            file,
            morphers,
            table,
            models,
            objects,
            lights: TownLights { fog: Some(spec.fog.depth()), ..lights },
            fog: spec.fog,
            hits,
            clear: spec.clear,
            events: Vec::new(),
            written: Vec::new(),
        })
    }

    /// What the draws started since the last call: effects and sounds.
    pub fn take_events(&mut self) -> Vec<TownEvent> {
        std::mem::take(&mut self.events)
    }

    /// A static model row's placement.
    pub fn model(&self, row: usize) -> Option<PlacedModel> {
        self.models.get(row).copied().flatten()
    }

    /// A static object's animation time and root matrix, by its row.
    pub fn object(&self, row: usize) -> Option<(u32, [V4; 4])> {
        self.objects.iter().find(|o| o.row == row).map(|o| (o.play.time, o.root))
    }

    /// The model table's rows of `pass`, in order.
    pub(crate) fn rows(&self, pass: DrawPass) -> impl Iterator<Item = usize> + use<> {
        let rows = self.table.rows;
        (0..rows.len()).filter(move |&row| rows[row].pass == pass)
    }

    /// A pass's static objects: the rows of those of `pass` that
    /// [`Base::object_step`] draws, stepped, in order.
    pub(crate) fn step_objects(&mut self, pass: DrawPass, eye: V4) -> Vec<usize> {
        let mut out = Vec::new();
        for i in 0..self.objects.len() {
            if self.objects[i].pass == pass && self.object_step(i, eye) {
                out.push(self.objects[i].row);
            }
        }
        out
    }

    /// `STATICOBJECT::Draw` (0x005cfe70) up to the draw: within its clip of
    /// the eye (`sceVu0SubVector`, x x + y y, `sqrt` in double; `sqrt.s`
    /// from Outbreak on) or with a clip of 0, the animation steps. True
    /// when it is to be drawn.
    pub(crate) fn object_step(&mut self, i: usize, eye: V4) -> bool {
        let volume = self.hits.volume;
        let o = &mut self.objects[i];
        let (dx, dy) = (ee::sub(eye[0], o.pos[0]), ee::sub(eye[1], o.pos[1]));
        let d = ee::dsqrt_on(volume, ee::add(ee::mul(dx, dx), ee::mul(dy, dy)));
        if !(ee::lt(d, o.clip) || ee::eq(o.clip, 0)) {
            return false;
        }
        o.play.forward(&self.file);
        true
    }

    /// The static model of a row, as drawn, on `layer`.
    pub(crate) fn row_draw(&self, layers: &mut Layers, layer: i16, to_screen: Mat4, row: usize) {
        self.row_draw_with(layers, layer, to_screen, row, self.depth_fog());
    }

    /// The same with its fog: `STATICMODEL::DrawWithOutFog` draws with
    /// none.
    pub(crate) fn row_draw_with(
        &self,
        layers: &mut Layers,
        layer: i16,
        to_screen: Mat4,
        row: usize,
        fog: draw::Fogging,
    ) {
        let Some(p) = self.model(row) else { return };
        let rows = Default::default();
        let d = Draw {
            file: &self.file,
            model: p.model,
            world: Mat4::from_translation(p.pos),
            alpha: 1.0,
            rows: &rows,
            lights: None,
            nodes: &[],
            morph: Vec::new(),
            clut_swaps: Vec::new(),
        };
        draw::model_edited(layers, layer, to_screen, d, None, fog);
    }

    /// A static object as drawn, on `layer`.
    pub(crate) fn object_draw(&self, layers: &mut Layers, layer: i16, to_screen: Mat4, row: usize) {
        let Some(o) = self.objects.iter().find(|o| o.row == row) else { return };
        let root = draw::mat(&o.root);
        let fm = (&*self.file, &self.morphers);
        anim_draw_fog(layers, layer, to_screen, fm, &o.play, root, &[], 1.0, None, None, self.depth_fog());
    }

    /// The town's `SetFog` as VU1 fogs its buildings and objects (the sky,
    /// clouds and water have `SetFogSw(0)`).
    pub fn depth_fog(&self) -> draw::Fogging {
        draw::Fogging::Depth(self.fog.depth())
    }

    /// A clump's models at `world` on `layer`, with the scrolled materials'
    /// offsets `rows`.
    pub(crate) fn clump_draw(
        &self,
        layers: &mut Layers,
        layer: i16,
        to_screen: Mat4,
        models: &[u32],
        world: Mat4,
        rows: &HashMap<u32, [u8; 2]>,
    ) {
        for &m in models {
            let d = Draw {
                file: &self.file,
                model: m,
                world,
                alpha: 1.0,
                rows,
                lights: None,
                nodes: &[],
                morph: Vec::new(),
                clut_swaps: Vec::new(),
            };
            draw::model(layers, layer, to_screen, d);
        }
    }
}

impl Town {
    pub(crate) fn new(base: Base, class: impl RootTown) -> Town {
        Town { base, class: Box::new(class) }
    }

    /// The town `WORLD_MAN::GO(0)` builds for `game.town` `no`
    /// (`ROOTTOWN01` .. `05`), `crisis` the save's crisis byte.
    pub fn open(archive: &Arc<Archive>, volume: piney_data::volume::Volume, no: i32, crisis: bool) -> Result<Town> {
        let mut town = match no {
            0 => crate::town01::new(archive, crisis),
            1 => crate::town02::new(archive, crisis),
            2 => crate::town03::new(archive, crisis),
            3 => crate::town04::new(archive, crisis),
            4 => crate::town05::new(archive, crisis),
            n => Err(Error::NotFound(format!("town {n}: there are five, 0-4"))),
        }?;
        // The towns' tables are Infection's on every disc; the collision
        // takes the disc's square root.
        town.base.hits.volume = volume;
        Ok(town)
    }

    /// The class, when it is a `T`.
    pub fn class<T: RootTown>(&self) -> Option<&T> {
        (&*self.class as &dyn Any).downcast_ref()
    }

    /// The base and the class, when it is a `T` (what the class's own
    /// `select` steps).
    pub fn parts_mut<T: RootTown>(&mut self) -> Option<(&mut Base, &mut T)> {
        let class = (&mut *self.class as &mut dyn Any).downcast_mut()?;
        Some((&mut self.base, class))
    }

    /// The class's `Draw` for this frame ([`RootTown::draw`]).
    pub fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView) -> Vec<TownSprite> {
        self.class.draw(&mut self.base, layers, to_screen, v)
    }

    /// `waterAnm[k]`'s animation time (0 in a town without water).
    pub fn water_time(&self, k: usize) -> u32 {
        self.class.water_time(k)
    }

    /// Water 0 as the last draw left it (None in a town without water).
    pub fn water0(&self) -> Option<&WaterZero> {
        self.class.water0()
    }
}

/// `DrawBG`'s crisis offsets: the scroll's `vftoi12` less each material's
/// crop, the first material in U, the second in U and V (STROW values).
pub(crate) fn crisis_rows(file: &SceneFile, mats: &[&str; 2], uv: u16) -> HashMap<u32, [u8; 2]> {
    let mut rows = HashMap::new();
    for (i, name) in mats.iter().enumerate() {
        let Some(mat) = file.ccs.find_object(name) else { continue };
        let Some(m) = file.scene.materials.get(&mat) else { continue };
        let du = uv.wrapping_sub(m.crop_u);
        let dv = if i == 1 { uv.wrapping_sub(m.crop_v) } else { 0 };
        rows.insert(mat, [(du >> 4) as u8, (dv >> 4) as u8]);
    }
    rows
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

/// `ccAnm::Draw`: every object the animation poses that has a model, under
/// `root`; `file` with its morphers (`piney_data::anim::morphers`); `rows`
/// overrides the animation's own texture offsets.
#[allow(clippy::too_many_arguments)]
pub fn anim_draw(
    layers: &mut Layers,
    to_screen: Mat4,
    (file, morphers): (&SceneFile, &HashMap<u32, u32>),
    play: &Play,
    root: Mat4,
    extra: &[u32],
    alpha: f32,
    rows: Option<&HashMap<u32, [u8; 2]>>,
) {
    anim_draw_on(layers, OBJ_LAYER, to_screen, (file, morphers), play, root, extra, alpha, rows, None);
}

/// [`anim_draw`] on `layer`, with `lights` for the lit models (mtype bit
/// 0) at their world matrices.
#[allow(clippy::too_many_arguments)]
pub fn anim_draw_on(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    fm: (&SceneFile, &HashMap<u32, u32>),
    play: &Play,
    root: Mat4,
    extra: &[u32],
    alpha: f32,
    rows: Option<&HashMap<u32, [u8; 2]>>,
    lights: Option<&dyn Fn(Mat4) -> piney_draw::Lights>,
) {
    anim_draw_fog(layers, layer, to_screen, fm, play, root, extra, alpha, rows, lights, draw::Fogging::None);
}

/// [`anim_draw_on`] with the models' fog.
#[allow(clippy::too_many_arguments)]
pub fn anim_draw_fog(
    layers: &mut Layers,
    layer: i16,
    to_screen: Mat4,
    (file, morphers): (&SceneFile, &HashMap<u32, u32>),
    play: &Play,
    root: Mat4,
    extra: &[u32],
    alpha: f32,
    rows: Option<&HashMap<u32, [u8; 2]>>,
    lights: Option<&dyn Fn(Mat4) -> piney_draw::Lights>,
    fog: draw::Fogging,
) {
    let worlds = play.worlds(file, root, extra);
    let own = play.uv_rows(file);
    let rows = rows.unwrap_or(&own);
    let morph = play.morph(file);
    let mut objs: Vec<(&u32, &Mat4)> = worlds.iter().collect();
    objs.sort_by_key(|(o, _)| **o);
    for (&obj, &world) in objs {
        let target = file.scene.ext.get(&obj).copied().unwrap_or(obj);
        let Some(&model) = file.obj_model.get(&target) else { continue };
        let m: Vec<(u32, f32)> = morph
            .iter()
            .filter(|(mph, _)| morphers.get(mph) == Some(&model))
            .flat_map(|(_, t)| t.iter().copied())
            .collect();
        let lit = file.models.get(&model).is_some_and(|i| i.mtype & 1 != 0);
        let lights = lights.filter(|_| lit).map(|l| l(world));
        let d = Draw { file, model, world, alpha, rows, lights, nodes: &[], morph: m, clut_swaps: Vec::new() };
        draw::model_edited(layers, layer, to_screen, d, None, fog);
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
        const ISO: &str = "../../work/infection/infection.iso";
        if !std::path::Path::new(ISO).exists() {
            eprintln!("skipped: no {ISO}");
            return None;
        }
        let mut iso = piney_data::iso::Iso::open(ISO).unwrap();
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
        let l = &t.base.lights;
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
            crate::chara::light_matrix(&group, glam::Mat4::from_translation(at), false).colours[2][0]
        };
        let full = lit(Vec3::ZERO);
        assert!(full > 0.0);
        assert!((lit(Vec3::new(0.0, 499.0, 0.0)) - full / 2.0).abs() < 1e-3);
        assert_eq!(lit(Vec3::new(501.0, 0.0, 0.0)), 0.0);
        assert_eq!(lit(Vec3::new(0.0, 0.0, 2600.0)), 0.0);
    }
}
