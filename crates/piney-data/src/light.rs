//! The light group, every step in the game's own arithmetic: the lights as
//! `ccAnm::SetAnmCtrlWork` (main 0x00150670) and `ccSetColor` (0x00138b60)
//! leave them, each kind's `CheckRange`, `ccDrawEnv::SetLightMatrix`
//! (0x00105900) and `sceVu0NormalLightMatrix` (0x00110d50) on the EE,
//! `ccSetMatrixPacket`'s product with the model's `lwMatrix` (0x0013e4e8),
//! and VU1's `mc_SetMatrix`, which renormalises the directions and doubles
//! the colours. Only `sqrt` differs by disc: newlib's `sqrtf` on Infection
//! and Mutation, `sqrt.s` from Outbreak on (docs/engine/field-game.md).

use crate::anim::{Ctrl, RotKeyed, Ticks, const_radians, read_ctrl, read_keys};
use crate::field::ee::{add, div, eq, from_int, from_u32, le, lt, mul, rsqrt, sub};
use crate::libm::{neg, sqrtf_on, tanf};
use crate::volume::Volume;
use crate::vu0::{
    F, M4, ONE, UNIT, V4, apply, invers, mul_matrix, normalize, rot_matrix, trans_matrix, transpose, vmul, vscale, vsub,
};
use crate::{Bytes, Result, format_err};

/// `lightVector` (INF main 0x002fb100; every disc's holds the same).
pub const LIGHT_VECTOR: V4 = [0, 0, 0xbf80_0000, 0];
/// 1/128: `CheckRange` lights nothing at or below it.
pub const DIM: F = 0x3c00_0000;
/// `mc_SetMatrix`'s `loi 2.0` on the light colours.
pub const VU_LIGHT_SCALE: F = 0x4000_0000;
/// `WORLD_MAN::SleepDistantLight`'s intensity (0.3), on shaded ground.
pub const SHADE: F = 0x3e99_999a;
const MINUS_ONE: F = 0xbf80_0000;
const TWO: F = 0x4000_0000;
const PI: F = 0x4049_0fdb;
const DEG180: F = 0x4334_0000;
/// `ccSetColor`'s 1/255 and 255.
const INV255: F = 0x3b80_8081;
const K255: F = 0x437f_0000;

/// A fall-off: full to `start`, nothing past `end` (0: no end); `end2` the
/// square the light keeps (`farDownEnd2`, a direct light's second radius
/// squared), as the code that set it worked it out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Falloff {
    pub start: F,
    pub end: F,
    pub end2: F,
}

impl Falloff {
    /// As `SetAnmCtrlWork` sets one: the square by `mul.s`.
    pub fn new(start: F, end: F) -> Falloff {
        Falloff { start, end, end2: mul(end, end) }
    }

    /// `v` past `start`: `(v (end - x)) / (end - start)`.
    fn fade(&self, v: F, x: F) -> F {
        div(mul(v, sub(self.end, x)), sub(self.end, self.start))
    }

    /// Along a beam: full with no end or to `start`, then [`Falloff::fade`].
    fn along(&self, v: F, z: F) -> F {
        if eq(self.end, 0) || le(z, self.start) { v } else { self.fade(v, z) }
    }
}

/// What a light is (`ccLight.type`) and what its `CheckRange` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `ccDistantLight` (type 1): its direction (+0xd0), `lightVector`
    /// through its matrix.
    Distant { dir: V4 },
    /// `ccDirectLight` (2): a round beam down its matrix's -z, fading
    /// along it (+0x144, +0x148) and out from it (+0x14c, +0x150, +0x154).
    Direct { matrix: M4, far: Falloff, radius: Falloff },
    /// `ccSpotLight` (3): a cone down -z, fading along it and between the
    /// tangents of its two half angles (+0x154, +0x158).
    Spot { matrix: M4, far: Falloff, cone: [F; 2] },
    /// `ccOmniLight` (4): at its matrix's place (+0x30), fading with the
    /// distance (+0xc4, +0xc8, +0xcc).
    Omni { pos: V4, far: Falloff },
}

impl Kind {
    /// `ccLight.type`.
    pub fn number(&self) -> i16 {
        match self {
            Kind::Distant { .. } => 1,
            Kind::Direct { .. } => 2,
            Kind::Spot { .. } => 3,
            Kind::Omni { .. } => 4,
        }
    }
}

/// A light of the group as the game holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Light {
    pub kind: Kind,
    /// Before the intensity (w unused).
    pub colour: V4,
    pub intensity: F,
    /// `ccLight.priority`: `ccCreateLight` gives a distant light -1, the
    /// others 0; a boss's `ccLight(4, 1)` 1.
    pub priority: i8,
}

impl Light {
    /// A distant light with world matrix `matrix` (`CheckRange`'s
    /// `ApplyMatrix(lwMatrix, lightVector)`), priority -1.
    pub fn distant(matrix: &M4, colour: V4, intensity: F) -> Light {
        Light { kind: Kind::Distant { dir: apply(matrix, LIGHT_VECTOR) }, colour, intensity, priority: -1 }
    }

    /// An omni light at `pos` (its matrix's place), priority 0.
    pub fn omni(pos: V4, colour: V4, intensity: F, far: Falloff) -> Light {
        Light { kind: Kind::Omni { pos, far }, colour, intensity, priority: 0 }
    }

    /// `ccOmniLight::Init` (main 0x001397e0): black, intensity 1, no
    /// fall-off, at the origin.
    pub const BLACK: Light = Light {
        kind: Kind::Omni { pos: [0, 0, 0, ONE], far: Falloff { start: 0, end: 0, end2: 0 } },
        colour: [0; 4],
        intensity: ONE,
        priority: 0,
    };

    /// The light's `CheckRange` at `at` (a model's place, w 1): the
    /// direction it travels and its colour times what reaches, or None.
    /// Omni (0x00139830), direct (0x00139330), distant (0x00139170) and
    /// spot (0x001395b0) as each disc's code has them.
    pub fn check_range(&self, volume: Volume, at: V4) -> Option<(V4, V4)> {
        if le(self.intensity, DIM) {
            return None;
        }
        let i = self.intensity;
        let (dir, f) = match &self.kind {
            Kind::Distant { dir } => (*dir, i),
            Kind::Omni { pos, far } => {
                let d = vsub(at, *pos);
                let sq = vmul(d, d);
                let d2 = add(sq[2], add(sq[0], sq[1]));
                let f = if eq(far.end, 0) {
                    i
                } else if !le(d2, far.end2) {
                    return None;
                } else {
                    let dist = sqrtf_on(volume, d2);
                    if le(dist, far.start) { i } else { far.fade(i, dist) }
                };
                (normalize(d), f)
            }
            Kind::Direct { matrix, far, radius } => {
                let (x, y, z) = beam_point(matrix, far, at)?;
                let r2 = add(mul(x, x), mul(y, y));
                if !le(r2, radius.end2) {
                    return None;
                }
                let f = far.along(i, z);
                let r = sqrtf_on(volume, r2);
                (apply(matrix, LIGHT_VECTOR), if le(r, radius.start) { f } else { radius.fade(f, r) })
            }
            Kind::Spot { matrix, far, cone } => {
                let (x, y, z) = beam_point(matrix, far, at)?;
                let r = sqrtf_on(volume, add(mul(x, x), mul(y, y)));
                let outer = mul(cone[1], z);
                if !le(r, outer) {
                    return None;
                }
                let f = far.along(i, z);
                let inner = mul(cone[0], z);
                let f = if le(r, inner) { f } else { div(mul(f, sub(outer, r)), sub(outer, inner)) };
                (apply(matrix, LIGHT_VECTOR), f)
            }
        };
        if le(f, DIM) {
            return None;
        }
        Some((dir, vscale(self.colour, f)))
    }
}

/// A beam's `at` in the light's own space (`InversMatrix`, `ApplyMatrix`):
/// x, y and the depth `-z`, or None behind the light or past the beam's
/// end.
fn beam_point(matrix: &M4, far: &Falloff, at: V4) -> Option<(F, F, F)> {
    let p = apply(&invers(matrix), at);
    let z = neg(p[2]);
    if lt(z, 0) || (!eq(far.end, 0) && !le(z, far.end)) {
        return None;
    }
    Some((p[0], p[1], z))
}

/// VU1's light registers after `mc_SetMatrix`: per slot the direction
/// toward the light in model space (lane k of vf05-07) and its colour
/// doubled (vf09-11), and the ambient (vf12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Regs {
    pub dirs: [[F; 3]; 3],
    pub colours: [[F; 3]; 3],
    pub ambient: [F; 3],
}

/// The registers a lit model with `lwMatrix` `world` is drawn with: the
/// group's lights (`lights`, in the order added) at its place, the
/// ambient `ambient` (`ccDrawEnv` +0x30).
pub fn light_regs<I>(volume: Volume, lights: I, ambient: V4, world: &M4) -> Regs
where
    I: IntoIterator<Item = Light>,
    I::IntoIter: Clone,
{
    let (normals, colours) = set_light_matrix(volume, lights, ambient, world[3]);
    let product = mul_matrix(&normal_light_matrix(&normals), world);
    mc_set_matrix(&product, &colours)
}

/// The group as `AddGrp` keeps it: descending priority, equal ones in the
/// order added.
fn by_priority<I>(lights: I) -> impl Iterator<Item = Light>
where
    I: Iterator<Item = Light> + Clone,
{
    let below = lights.clone();
    let top = lights.clone().map(|l| l.priority).max();
    std::iter::successors(top, move |&p| below.clone().map(|l| l.priority).filter(|&q| q < p).max())
        .flat_map(move |p| lights.clone().filter(move |l| l.priority == p))
}

/// One of `SetLightMatrix`'s three records: a priority and the row its
/// light takes.
#[derive(Clone, Copy)]
struct Slot {
    priority: i8,
    row: usize,
}

/// `SetLightMatrix`: the unit matrix and black, then three records, all
/// priority -128; a light above the lowest record's priority that answers
/// `CheckRange` takes its place among them by priority (strictly greater
/// goes higher) and the row of the record it pushes out: its direction
/// there (w 1), its colour (w 0). The ambient is row 3.
pub fn set_light_matrix<I>(volume: Volume, lights: I, ambient: V4, at: V4) -> (M4, M4)
where
    I: IntoIterator<Item = Light>,
    I::IntoIter: Clone,
{
    let (mut normals, mut colours) = (UNIT, [[0; 4]; 4]);
    colours[3] = ambient;
    let mut slots = [0, 1, 2].map(|row| Slot { priority: -128, row });
    for l in by_priority(lights.into_iter()) {
        if l.priority <= slots[2].priority {
            continue;
        }
        let Some((dir, colour)) = l.check_range(volume, at) else { continue };
        let row = slots[2].row;
        let new = Slot { priority: l.priority, row };
        if l.priority <= slots[1].priority {
            slots[2] = new;
        } else if l.priority <= slots[0].priority {
            slots[2] = slots[1];
            slots[1] = new;
        } else {
            slots = [new, slots[0], slots[1]];
        }
        normals[row] = [dir[0], dir[1], dir[2], ONE];
        colours[row] = [colour[0], colour[1], colour[2], 0];
    }
    (normals, colours)
}

/// `sceVu0NormalLightMatrix(m, m[0], m[1], m[2])`: each row negated
/// (`ScaleVector` by -1) and normalised, row 3 (0, 0, 0, 1), transposed.
pub fn normal_light_matrix(normals: &M4) -> M4 {
    let mut m = [[0; 4]; 4];
    for (out, row) in m.iter_mut().zip(normals) {
        *out = normalize(vscale(*row, MINUS_ONE));
    }
    m[3] = [0, 0, 0, ONE];
    transpose(&m)
}

/// `mc_SetMatrix` on the packet's light rows (VU memory 16-18, the light
/// matrix times `lwMatrix`) and colours (12-15): each light's lane squared
/// and summed (`mula`, `madda`, `madd`), times `ERSQRT` of it; the colours'
/// x, y, z times 2; the ambient as it is.
fn mc_set_matrix(rows: &M4, colours: &M4) -> Regs {
    let dirs = std::array::from_fn(|k| {
        let len2 = add(add(mul(rows[0][k], rows[0][k]), mul(rows[1][k], rows[1][k])), mul(rows[2][k], rows[2][k]));
        let p = rsqrt(ONE, len2);
        std::array::from_fn(|j| mul(rows[j][k], p))
    });
    Regs {
        dirs,
        colours: std::array::from_fn(|k| std::array::from_fn(|j| mul(colours[k][j], VU_LIGHT_SCALE))),
        ambient: [colours[3][0], colours[3][1], colours[3][2]],
    }
}

/// `ccSetColor(out, color, s)` (main 0x00138b60, every disc's alike). With
/// no top byte its r, g, b over 255 (`ITOF0`, times 1/255 at an `s` of 1,
/// else times s/255), w 0. With one, an HSV colour: hue the low byte
/// (sixths of 256), saturation and value the next two over 255 (value
/// times s), turned to r, g, b by the hue's sector; w kept.
pub fn set_color(out: &mut V4, color: u32, s: F) {
    let byte = |k: u32| from_int(((color >> (8 * k)) & 0xff) as i32);
    if color >> 24 == 0 {
        let k = if eq(ONE, s) { INV255 } else { div(s, K255) };
        *out = [mul(byte(0), k), mul(byte(1), k), mul(byte(2), k), 0];
        return;
    }
    let h6 = (color & 0xff) as i32 * 6;
    let scale = div(s, K255);
    let frac = mul(from_int(h6 & 0xff), 0x3b80_0000);
    let v = mul(byte(2), scale);
    let sat = mul(byte(1), INV255);
    let sf = mul(sat, frac);
    let p = mul(v, sub(ONE, sat));
    let q = mul(v, sub(ONE, sf));
    let t = mul(v, sub(add(ONE, sf), sat));
    let rgb = match h6 >> 8 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        5 => [v, p, q],
        _ => return,
    };
    out[..3].copy_from_slice(&rgb);
}

/// [`set_color`] into black.
pub fn colour_of(color: u32, s: F) -> V4 {
    let mut out = [0; 4];
    set_color(&mut out, color, s);
    out
}

/// `ccSetBlendColor(out, a, b, t)` (main 0x00138db0): `ccSetColor(a, 1 -
/// t)` plus `ccSetColor(b, t)`, x, y, z; w left as `out` had it.
pub fn blend_color(a: u32, b: u32, t: F) -> V4 {
    let (x, y) = (colour_of(a, sub(ONE, t)), colour_of(b, t));
    [add(x[0], y[0]), add(x[1], y[1]), add(x[2], y[2]), 0]
}

/// `ccAnm::GetAmbient` (main 0x001528e0): each byte of the packed ambient
/// over 255 (`cvt.s.w`, `div.s`).
pub fn ambient(packed: u32) -> V4 {
    let c = |k: u32| div(from_int(((packed >> (8 * k)) & 0xff) as i32), K255);
    [c(0), c(1), c(2), 0]
}

// The lights of an Anime chunk -------------------------------------------------

/// A colour controller as `ccAnmCtrlColor_SetCtrl` (main 0x00146310)
/// compiles one: one packed colour, or the first key's and (duration,
/// colour) segments, a hold before a first key past frame 0 and one after
/// the last.
#[derive(Clone, Debug, PartialEq)]
enum Colour {
    Const(u32),
    Keyed { first: u32, segs: Vec<(u32, u32)> },
}

impl Colour {
    fn read(d: &[u8], q: &mut usize, kind: u32, frames: u32) -> Result<Colour> {
        match kind {
            1 => {
                let c = d.u32_at(*q)?;
                *q += 4;
                Ok(Colour::Const(c))
            }
            2 => {
                let keys = read_keys::<1>(d, q)?;
                let (f0, [first]) = keys[0];
                let mut t = f0 << 8;
                let mut segs = Vec::with_capacity(keys.len() + 1);
                if t != 0 {
                    segs.push((t, first));
                }
                for &(frame, [c]) in &keys[1..] {
                    let dur = (frame << 8).wrapping_sub(t);
                    t = t.wrapping_add(dur);
                    segs.push((dur, c));
                }
                let last = keys[keys.len() - 1].1[0];
                segs.push(((frames << 8).wrapping_sub(t), last));
                Ok(Colour::Keyed { first, segs })
            }
            // `ccAnmCtrlColor_Set` leaves the colour as its scratch had it;
            // no light of any disc goes without one.
            k => format_err(format!("light colour controller kind {k} at 0x{:x}", *q)),
        }
    }

    /// `ccAnmCtrlColor_Set` (main 0x001461e0) at `time`: a constant through
    /// `ccSetColor(c, 1)`; keyed, the segments passed (strictly shorter than
    /// the time left) then `ccSetBlendColor` of the colour held and the
    /// segment's by the time into it over its length (`cvt.s.w` of each).
    fn at(&self, time: Ticks) -> V4 {
        match self {
            Colour::Const(c) => colour_of(*c, ONE),
            Colour::Keyed { first, segs } => {
                let (mut held, mut t, mut i) = (*first, time, 0);
                while i + 1 < segs.len() && segs[i].0 < t {
                    held = segs[i].1;
                    t = t.wrapping_sub(segs[i].0);
                    i += 1;
                }
                let (dur, next) = segs[i];
                // 0/0 only past a last key at the chunk's end (as the floats).
                let frac = if dur != 0 { div(from_u32(t), from_u32(dur)) } else { 0 };
                blend_color(held, next, frac)
            }
        }
    }
}

/// A rotation controller (`ccAnmCtrlRot_Set`, main 0x00146f30): the unit
/// matrix when absent, the matrix `ccAnmCtrlRot_SetCtrl` stored, or keyed.
#[derive(Clone, Debug)]
enum Rot {
    Unit,
    Const(M4),
    Keyed(RotKeyed),
}

impl Rot {
    fn read(d: &[u8], q: &mut usize, kind: u32, frames: u32) -> Result<Rot> {
        match kind {
            0 => Ok(Rot::Unit),
            1 => {
                let deg = [d.u32_at(*q)?, d.u32_at(*q + 4)?, d.u32_at(*q + 8)?];
                *q += 12;
                let [x, y, z] = const_radians(deg);
                Ok(Rot::Const(rot_matrix(&UNIT, [x, y, z, 0])))
            }
            2 => Ok(Rot::Keyed(RotKeyed::new(&read_keys::<3>(d, q)?, frames))),
            k => format_err(format!("light rotation controller kind {k} at 0x{:x}", *q)),
        }
    }

    fn at(&self, time: Ticks) -> M4 {
        match self {
            Rot::Unit => UNIT,
            Rot::Const(m) => *m,
            // No light of any disc turns by keys; the animations' own way.
            Rot::Keyed(k) => k.at(time),
        }
    }
}

/// Which light record (`ccLightChunk.type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Record {
    Distant,
    Direct,
    Spot,
    Omni,
}

/// A light record of an Anime chunk: 0x0603 distant (flags bits 3-5
/// rotation, 6-8 colour, 9-11 intensity), 0x0605 direct and 0x0607 spot
/// (0-2 position, 3-5 rotation, 6-8 colour, 9-11 intensity, 12-14 and 15-17
/// the beam's start and end, 18-20 and 21-23 its two radii or half angles
/// in degrees), 0x0609 omni (0-2 position, 6-8 colour, 9-11 intensity,
/// 12-14 and 15-17 the fall-off's start and end).
#[derive(Clone, Debug)]
pub struct AnimLight {
    /// The LGT_ object.
    pub object: u32,
    record: Record,
    pos: Ctrl<3>,
    rot: Rot,
    colour: Colour,
    intensity: Ctrl<1>,
    far: [Ctrl<1>; 2],
    radius: [Ctrl<1>; 2],
}

impl AnimLight {
    /// The light `SetAnmCtrlWork` leaves at `time` (1/256 frames since
    /// `SetAnm`) under its anm, whose `lwMatrix` is `root`: its place
    /// (`FVec3_Set`, 0 when absent) and rotation make the local matrix
    /// (`sceVu0TransMatrix`; an omni light's from the unit matrix), which
    /// `_SetLWMatrix` takes through `root` (`SetAnm` makes the anm the
    /// light's parent). Intensity defaults to 1, the rest to 0.
    pub fn at(&self, time: Ticks, root: &M4) -> Light {
        let [p0, p1, p2] = self.pos.at(time, [0; 3]);
        let pos = [p0, p1, p2, 0];
        let colour = self.colour.at(time);
        let [intensity] = self.intensity.at(time, [ONE]);
        let float = |c: &Ctrl<1>| c.at(time, [0])[0];
        let far = Falloff::new(float(&self.far[0]), float(&self.far[1]));
        let lw = |local: M4| mul_matrix(root, &local);
        let kind = match self.record {
            Record::Distant => return Light::distant(&lw(self.rot.at(time)), colour, intensity),
            Record::Omni => Kind::Omni { pos: lw(trans_matrix(&UNIT, pos))[3], far },
            Record::Direct => {
                let matrix = lw(trans_matrix(&self.rot.at(time), pos));
                Kind::Direct { matrix, far, radius: Falloff::new(float(&self.radius[0]), float(&self.radius[1])) }
            }
            Record::Spot => {
                // Each half angle (pi deg / 180) / 2, kept as its tangent.
                let tan = |c: &Ctrl<1>| tanf(div(div(mul(PI, float(c)), DEG180), TWO));
                let matrix = lw(trans_matrix(&self.rot.at(time), pos));
                Kind::Spot { matrix, far, cone: [tan(&self.radius[0]), tan(&self.radius[1])] }
            }
        };
        Light { kind, colour, intensity, priority: 0 }
    }
}

/// An Anime chunk's (at `offset` in `data`) packed ambient (the last 0x0601
/// record at or before frame 1, for [`ambient`]) and its light records,
/// each object's last.
pub fn anim_lights(data: &[u8], offset: usize) -> Result<(Option<u32>, Vec<AnimLight>)> {
    let frames = data.u32_at(offset + 12)?;
    let words = data.u32_at(offset + 16)? as usize;
    let (mut p, end) = (offset + 20, offset + 20 + 4 * words);
    let (mut frame, mut packed) = (0u32, None);
    let mut lights: Vec<AnimLight> = Vec::new();
    while p < end {
        let kind = data.u32_at(p)? as u16;
        let n = data.u32_at(p + 4)? as usize;
        let mut q = p + 8;
        let record = match kind {
            0x0603 => Some(Record::Distant),
            0x0605 => Some(Record::Direct),
            0x0607 => Some(Record::Spot),
            0x0609 => Some(Record::Omni),
            _ => None,
        };
        match (kind, record) {
            (0xff01, _) => frame = data.u32_at(q)?,
            (0x0601, _) if frame <= 1 => packed = Some(data.u32_at(q)?),
            (_, Some(record)) => {
                let object = data.u32_at(q)?;
                let flags = data.u32_at(q + 4)?;
                q += 8;
                let field = |shift: u32| (flags >> shift) & 7;
                let placed = record != Record::Distant;
                let pos = if placed { read_ctrl::<3>(data, &mut q, field(0), frames)? } else { Ctrl::None };
                let rot = if record == Record::Omni { Rot::Unit } else { Rot::read(data, &mut q, field(3), frames)? };
                let colour = Colour::read(data, &mut q, field(6), frames)?;
                let intensity = read_ctrl::<1>(data, &mut q, field(9), frames)?;
                let far = if placed {
                    [read_ctrl::<1>(data, &mut q, field(12), frames)?, read_ctrl::<1>(data, &mut q, field(15), frames)?]
                } else {
                    [Ctrl::None, Ctrl::None]
                };
                let radius = if matches!(record, Record::Direct | Record::Spot) {
                    [read_ctrl::<1>(data, &mut q, field(18), frames)?, read_ctrl::<1>(data, &mut q, field(21), frames)?]
                } else {
                    [Ctrl::None, Ctrl::None]
                };
                if q > p + 8 + 4 * n {
                    return format_err(format!("light record at 0x{p:x} overruns"));
                }
                lights.retain(|l| l.object != object);
                lights.push(AnimLight { object, record, pos, rot, colour, intensity, far, radius });
            }
            _ => {}
        }
        p += 8 + 4 * n;
    }
    Ok((packed, lights))
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn k(x: f32) -> F {
        x.to_bits()
    }

    fn f(x: F) -> f32 {
        f32::from_bits(x)
    }

    /// An omni light 500 below a model at the origin, full to 0 and gone at
    /// 1000: half its colour (doubled, as VU1 does) toward it; past its end,
    /// nothing in its slot.
    #[test]
    fn an_omni_light_fades_to_its_end() {
        let light = Light::omni([0, 0, k(-500.0), ONE], colour_of(0x00ff_ffff, ONE), ONE, Falloff::new(0, k(1000.0)));
        let regs = light_regs(Volume::Inf, [light], [0; 4], &UNIT);
        assert_eq!(regs.colours[2].map(f), [1.0; 3]);
        // Toward the light, one short of -1 (`Normalize` and `ERSQRT` truncate).
        assert_eq!(regs.dirs[2].map(f), [0.0, 0.0, -0.999_999_94]);
        let far = Light::omni([0, 0, k(-1000.0), ONE], light.colour, ONE, Falloff::new(0, k(1000.0)));
        assert_eq!(light_regs(Volume::Inf, [far], [0; 4], &UNIT).colours[2], [0; 3]);
    }

    /// Three slots by priority: the distant light (-1) loses its slot to
    /// later omni lights, a priority-1 light goes highest.
    #[test]
    fn slots_go_by_priority() {
        let white = colour_of(0x00ff_ffff, ONE);
        let near =
            |x: f32, p: i8| Light { priority: p, ..Light::omni([k(x), 0, 0, ONE], white, ONE, Falloff::default()) };
        let sun = Light::distant(&UNIT, white, ONE);
        let (normals, _) =
            set_light_matrix(Volume::Inf, [sun, near(1.0, 0), near(2.0, 0), near(3.0, 1)], [0; 4], [0, 0, 0, ONE]);
        // Slot 2 the first taken: the priority-1 light, then the omni
        // lights in order; the distant light (straight down) none.
        let towards = [0, 1, 2].map(|r| f(normals[r][0]));
        assert!(towards.iter().all(|&x| (x + 1.0).abs() < 1e-6), "{towards:?}");
        assert!(normals.iter().take(3).all(|r| r[3] == ONE && r[2] == 0));
    }

    /// `AddGrp`'s order: descending priority, equal ones as added.
    #[test]
    fn the_group_goes_by_priority_then_order() {
        let at =
            |x: f32, p: i8| Light { priority: p, ..Light::omni([k(x), 0, 0, ONE], [0; 4], ONE, Falloff::default()) };
        let group = [at(1.0, 0), at(2.0, -1), at(3.0, 1), at(4.0, 0), at(5.0, 1)];
        let order: Vec<f32> = by_priority(group.iter().copied())
            .map(|l| match l.kind {
                Kind::Omni { pos, .. } => f(pos[0]),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(order, [3.0, 5.0, 1.0, 4.0, 2.0]);
    }

    /// `ccSetColor` at 1 takes each byte times 1/255; `ccSetBlendColor` at
    /// 0 is the first colour alone.
    #[test]
    fn colours() {
        assert_eq!(colour_of(0x00ff_ffff, ONE)[0], mul(from_int(255), INV255));
        assert_eq!(colour_of(0x0000_0080, k(0.5))[0], mul(from_int(128), div(k(0.5), K255)));
        assert_eq!(blend_color(0x00ff_0000, 0x0000_00ff, 0), colour_of(0x00ff_0000, ONE));
    }
}
