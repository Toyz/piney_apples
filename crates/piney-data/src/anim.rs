//! Anime chunks (0x0700) evaluated at any time, the way `ccAnm` plays them:
//! controllers absent, single or keyed, keyed values linear in EE single
//! precision (`ccAnmCtrlFVec3_Set` 0x00146890), keyed rotations turned about
//! one axis between quantised keys (`ccAnmCtrlRot_SetCtrl` 0x001470c0), the
//! local matrix `T * R * S` (`ccAnm::SetAnmCtrlWork` 0x00150670), playback
//! ([`Animation::forward`]), morphs ([`morph`]) and notes
//! ([`Animation::passed_notes`]). The model is `docs/engine/animation.md`;
//! `tests/anim.rs` holds the port to `tools/anim.py`, checked against the game.

use std::collections::HashMap;

use glam::{DMat3, DVec3, Mat3, Mat4, Vec3, Vec4};

use crate::ccs::{self, Ccs};
use crate::scene::{Anime, Scene, Transform};
use crate::{Bytes, Result, format_err};

/// Time in 1/256 frames, as `ccAnm` counts it (frameNow * 256 + frameCnt).
pub type Ticks = u32;
/// `ccAnm::frameSpd`'s default: one frame per `_AnimateForward`.
pub const TICKS_PER_FRAME: Ticks = 256;

/// A frame number as [`Ticks`].
pub fn ticks(frame: f32) -> Ticks {
    (frame.max(0.0) * TICKS_PER_FRAME as f32) as Ticks
}

const OBJECT: u16 = 0x0102;
const MATERIAL: u16 = 0x0202;
const F_MORPHER: u16 = 0x1901;
const F_NOTE: u16 = 0x0108;
const F_OBJ: u16 = 0x0101;
const MORPHER: u16 = 0x1900;
const END_LOOP: u32 = 0xffff_fffe;

/// The EE's single-precision arithmetic on raw bit patterns, as tools/eemu.py
/// models it: no denormals (exponent 0 is zero), no infinities (exponent 255
/// is a number), overflow to +-0x7fffffff, and results truncated towards zero.
pub mod ee {
    const FMAX: u32 = 0x7fff_ffff;

    fn unpack(v: u32) -> (u32, i32, u64) {
        let e = (v >> 23 & 0xff) as i32;
        (v >> 31, e, if e != 0 { (v & 0x7f_ffff | 0x80_0000) as u64 } else { 0 })
    }

    /// The exact value n * 2^e2 (n >= 0), truncated to a float.
    fn round(sign: u32, n: u128, e2: i32) -> u32 {
        if n == 0 {
            return sign << 31;
        }
        let len = 128 - n.leading_zeros() as i32;
        let m = if len >= 24 { n >> (len - 24) } else { n << (24 - len) } as u32;
        let exp = e2 + len - 24 + 150;
        if exp > 255 {
            sign << 31 | FMAX
        } else if exp < 1 {
            sign << 31
        } else {
            sign << 31 | (exp as u32) << 23 | (m & 0x7f_ffff)
        }
    }

    fn value(v: u32) -> (i128, i32) {
        let (s, e, m) = unpack(v);
        (if s != 0 { -(m as i128) } else { m as i128 }, e - 150)
    }

    pub fn add(a: u32, b: u32) -> u32 {
        let (mut na, mut ea) = value(a);
        let (mut nb, mut eb) = value(b);
        if na == 0 && nb == 0 {
            return a & b & 0x8000_0000;
        }
        if na == 0 {
            return round((nb < 0) as u32, nb.unsigned_abs(), eb);
        }
        if nb == 0 {
            return round((na < 0) as u32, na.unsigned_abs(), ea);
        }
        // More than 60 bits apart, the smaller operand can only step the
        // truncated sum down by its sign; a unit in its place does the same.
        if ea - eb > 60 {
            nb = nb.signum();
            eb = ea - 60;
        } else if eb - ea > 60 {
            na = na.signum();
            ea = eb - 60;
        }
        let e = ea.min(eb);
        let n = (na << (ea - e)) + (nb << (eb - e));
        round((n < 0) as u32, n.unsigned_abs(), e)
    }

    pub fn sub(a: u32, b: u32) -> u32 {
        add(a, b ^ 0x8000_0000)
    }

    pub fn mul(a: u32, b: u32) -> u32 {
        let (sa, ea, ma) = unpack(a);
        let (sb, eb, mb) = unpack(b);
        round(sa ^ sb, (ma * mb) as u128, ea + eb - 300)
    }

    pub fn div(a: u32, b: u32) -> u32 {
        let (sa, ea, ma) = unpack(a);
        let (sb, eb, mb) = unpack(b);
        if mb == 0 {
            return (sa ^ sb) << 31 | FMAX;
        }
        round(sa ^ sb, ((ma as u128) << 60) / mb as u128, ea - eb - 60)
    }

    /// sqrt of the magnitude, as the EE's sqrt.s and VU0's vsqrt take it.
    pub fn sqrt(v: u32) -> u32 {
        let (_, e, m) = unpack(v);
        let t = 60 + ((e - 150 - 60) & 1);
        let n = (m as u128) << t;
        let mut r = (n as f64).sqrt() as u128;
        while r * r > n {
            r -= 1;
        }
        while (r + 1) * (r + 1) <= n {
            r += 1;
        }
        round(0, r, (e - 150 - t) / 2)
    }

    /// cvt.s.w
    pub fn from_int(i: i32) -> u32 {
        round((i < 0) as u32, i.unsigned_abs() as u128, 0)
    }

    /// cvt.w.s (and fptosi): truncate, saturating.
    pub fn to_int(v: u32) -> i32 {
        let (n, e) = value(v);
        let a = n.unsigned_abs();
        let mag = if a == 0 {
            0
        } else if e >= 0 {
            if e > 40 { u128::MAX >> 1 } else { a << e }
        } else if -e >= 100 {
            0
        } else {
            a >> -e
        };
        if n < 0 {
            if mag > 0x8000_0000 { i32::MIN } else { (mag as i64).wrapping_neg() as i32 }
        } else if mag > 0x7fff_ffff {
            i32::MAX
        } else {
            mag as i32
        }
    }

    /// c.eq.s: exact values, both zeros equal.
    pub fn eq(a: u32, b: u32) -> bool {
        let ((na, ea), (nb, eb)) = (value(a), value(b));
        if na == 0 || nb == 0 {
            return na == nb;
        }
        let e = ea.min(eb);
        (ea - e) < 100 && (eb - e) < 100 && (na << (ea - e)) == (nb << (eb - e))
    }

    /// c.lt.s against zero.
    pub fn negative(a: u32) -> bool {
        value(a).0 < 0
    }

    /// The value as a double (exponent 255 stays a finite number).
    pub fn to_f64(v: u32) -> f64 {
        let (n, e) = value(v);
        n as f64 * (e as f64).exp2()
    }

    /// A u32 to float the way the compiled code does it: cvt.s.w, halving and
    /// doubling when the top bit is set.
    pub fn from_u32(x: u32) -> u32 {
        if x < 0x8000_0000 {
            from_int(x as i32)
        } else {
            let h = from_int((x >> 1 | x & 1) as i32);
            add(h, h)
        }
    }
}

const ONE: u32 = 0x3f80_0000;
const PI: u32 = 0x4049_0fdb;
const HALF_PI: u32 = 0x3fc9_0fdb;
const DEG180: u32 = 0x4334_0000;
/// 182.04444 = 32768 / 180
const TO_S16: u32 = 0x4336_0b61;
/// pi / 32768
const FROM_S16: u32 = 0x38c9_0fdb;
/// EigenVector (0x00105be0) gives up when no cofactor reaches ~1e-11.
const EIGEN_MIN: u32 = 0x2d2f_ebff;
/// The sine polynomial's coefficients, S5432 (0x002f7520).
const SIN: [u32; 4] = [0x362e_9c14, 0xb94f_b21f, 0x3c08_873e, 0xbe2a_aaa4];

// Controllers ----------------------------------------------------------------

/// A keyed float or float[3], compiled as `ccAnmCtrlFVec3_SetCtrl` /
/// `ccAnmCtrlFloat_SetCtrl` do: the first key's value and (duration, delta)
/// segments, a hold before the first key when it is not at frame 0 and a
/// hold after the last.
#[derive(Clone, Debug)]
struct Keyed<const N: usize> {
    base: [u32; N],
    segs: Vec<(u32, [u32; N])>,
}

impl<const N: usize> Keyed<N> {
    fn new(keys: &[(u32, [u32; N])], frames: u32) -> Self {
        let (f0, v0) = keys[0];
        let mut t = f0 << 8;
        let mut segs = Vec::with_capacity(keys.len() + 1);
        if t != 0 {
            segs.push((t, [0; N]));
        }
        let mut prev = v0;
        for &(frame, v) in &keys[1..] {
            let dur = (frame << 8).wrapping_sub(t);
            t = t.wrapping_add(dur);
            let delta: [u32; N] = std::array::from_fn(|k| ee::sub(v[k], prev[k]));
            prev = std::array::from_fn(|k| ee::add(prev[k], delta[k]));
            segs.push((dur, delta));
        }
        segs.push(((frames << 8).wrapping_sub(t), [0; N]));
        Keyed { base: v0, segs }
    }

    fn at(&self, time: Ticks) -> [u32; N] {
        let mut acc = self.base;
        let mut t = time;
        let mut i = 0;
        while i + 1 < self.segs.len() && self.segs[i].0 < t {
            let (dur, delta) = self.segs[i];
            acc = std::array::from_fn(|k| ee::add(acc[k], delta[k]));
            t = t.wrapping_sub(dur);
            i += 1;
        }
        let (dur, delta) = self.segs[i];
        // 0/0 only at time 0 before a zero-length first segment, which the
        // game never evaluates.
        let frac = if dur != 0 { ee::div(ee::from_u32(t), ee::from_u32(dur)) } else { 0 };
        std::array::from_fn(|k| ee::add(acc[k], ee::mul(delta[k], frac)))
    }
}

#[derive(Clone, Debug)]
enum Ctrl<const N: usize> {
    None,
    Const([u32; N]),
    Keyed(Keyed<N>),
}

impl<const N: usize> Ctrl<N> {
    fn at(&self, time: Ticks, default: [u32; N]) -> [u32; N] {
        match self {
            Ctrl::None => default,
            Ctrl::Const(v) => *v,
            Ctrl::Keyed(k) => k.at(time),
        }
    }

    fn keyed(&self) -> bool {
        matches!(self, Ctrl::Keyed(_))
    }
}

/// `_sceVu0ecossin` (0x001109b8) as `sceVu0RotMatrixX/Y/Z` use it: cos is an
/// odd polynomial in pi/2 - |angle|, sin is +-sqrt(1 - cos^2). Returns
/// (sin, cos) as bits.
fn vu_cossin(angle: u32) -> (u32, u32) {
    let neg = ee::negative(angle);
    let x = if neg { ee::add(HALF_PI, angle) } else { ee::sub(HALF_PI, angle) };
    let x2 = ee::mul(x, x);
    let mut p = SIN.map(|k| ee::mul(ee::mul(k, x), x2));
    for v in &mut p[..3] {
        *v = ee::mul(*v, x2);
    }
    let mut r = ee::add(x, p[3]);
    for v in &mut p[..2] {
        *v = ee::mul(*v, x2);
    }
    r = ee::add(r, p[2]);
    p[0] = ee::mul(p[0], x2);
    r = ee::add(ee::add(r, p[1]), p[0]);
    let q = ee::sqrt(ee::sub(ONE, ee::mul(r, r)));
    (if neg { ee::sub(0, q) } else { ee::add(0, q) }, r)
}

/// `sceVu0MulMatrix`'s product of `cols` with each stored column of `m`:
/// ((a*x + b*y) + c*z) + d*w, as eemu models VU0's multiply-adds. Also
/// `ccCoord::_SetLWMatrix`'s parent-times-local product.
pub fn vu_mul(cols: &[[u32; 4]; 4], m: &[[u32; 4]; 4]) -> [[u32; 4]; 4] {
    m.map(|col| {
        std::array::from_fn(|k| {
            let acc = ee::mul(cols[0][k], col[0]);
            let acc = ee::add(acc, ee::mul(cols[1][k], col[1]));
            let acc = ee::add(acc, ee::mul(cols[2][k], col[2]));
            ee::add(acc, ee::mul(cols[3][k], col[3]))
        })
    })
}

/// `sceVu0RotMatrix(unit, rad)` as stored columns of float bits: Rz, then Ry,
/// then Rx on the left, so Rx * Ry * Rz.
pub fn rot_bits(rad: [u32; 3]) -> [[u32; 4]; 4] {
    rot_bits_of([[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]], rad)
}

/// `sceVu0RotMatrix(out, m, rad)`: `m` turned about z, then y, then x, each
/// a product on the left (so Rx * Ry * Rz * m, rounded step by step).
pub fn rot_bits_of(m: [[u32; 4]; 4], rad: [u32; 3]) -> [[u32; 4]; 4] {
    let z = |v: u32| ee::add(0, v);
    let n = |v: u32| ee::sub(0, v);
    let mut m = rot_z_bits_of(m, rad[2]);
    let (s, c) = vu_cossin(rad[1]);
    m = vu_mul(&[[z(c), 0, n(s), 0], [0, ONE, 0, 0], [z(s), 0, z(c), 0], [0, 0, 0, ONE]], &m);
    let (s, c) = vu_cossin(rad[0]);
    vu_mul(&[[ONE, 0, 0, 0], [0, z(c), z(s), 0], [0, n(s), z(c), 0], [0, 0, 0, ONE]], &m)
}

/// `sceVu0RotMatrixX(out, m, rx)`: `m` turned about x, `Rx * m`.
pub fn rot_x_bits_of(m: [[u32; 4]; 4], rx: u32) -> [[u32; 4]; 4] {
    let z = |v: u32| ee::add(0, v);
    let n = |v: u32| ee::sub(0, v);
    let (s, c) = vu_cossin(rx);
    vu_mul(&[[ONE, 0, 0, 0], [0, z(c), z(s), 0], [0, n(s), z(c), 0], [0, 0, 0, ONE]], &m)
}

/// `sceVu0RotMatrixY(out, m, ry)`: `m` turned about y, `Ry * m`.
pub fn rot_y_bits_of(m: [[u32; 4]; 4], ry: u32) -> [[u32; 4]; 4] {
    let z = |v: u32| ee::add(0, v);
    let n = |v: u32| ee::sub(0, v);
    let (s, c) = vu_cossin(ry);
    vu_mul(&[[z(c), 0, n(s), 0], [0, ONE, 0, 0], [z(s), 0, z(c), 0], [0, 0, 0, ONE]], &m)
}

/// `sceVu0RotMatrixZ(out, m, rz)` (0x00110a30): `m` turned about z, `Rz *
/// m`.
pub fn rot_z_bits_of(m: [[u32; 4]; 4], rz: u32) -> [[u32; 4]; 4] {
    let z = |v: u32| ee::add(0, v);
    let n = |v: u32| ee::sub(0, v);
    let (s, c) = vu_cossin(rz);
    vu_mul(&[[z(c), z(s), 0, 0], [n(s), z(c), 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]], &m)
}

fn bits_mat3(m: &[[u32; 4]; 4]) -> Mat3 {
    let col = |c: usize| Vec3::new(f32::from_bits(m[c][0]), f32::from_bits(m[c][1]), f32::from_bits(m[c][2]));
    Mat3::from_cols(col(0), col(1), col(2))
}

fn bits_dmat3(m: &[[u32; 4]; 4]) -> DMat3 {
    let col = |c: usize| DVec3::new(ee::to_f64(m[c][0]), ee::to_f64(m[c][1]), ee::to_f64(m[c][2]));
    DMat3::from_cols(col(0), col(1), col(2))
}

/// A key's degrees quantised to s16 turns, in radians (bits).
fn key_radians(deg: [u32; 3]) -> [u32; 3] {
    deg.map(|d| ee::mul(FROM_S16, ee::from_int(ee::to_int(ee::mul(TO_S16, d)) as i16 as i32)))
}

/// Degrees to radians as the game's record decoders do it (`DecodeF_Obj`,
/// `DecodeF_Camera`): `(pi * deg) / 180` in EE single precision.
pub fn const_radians(deg: [u32; 3]) -> [u32; 3] {
    deg.map(|d| ee::div(ee::mul(PI, d), DEG180))
}

fn normalize(v: DVec3) -> DVec3 {
    let n = v.length();
    if n != 0.0 { v / n } else { DVec3::ZERO }
}

/// `m2a` (0x00105ec0): the axis `EigenVector` finds and the angle about it;
/// None where `EigenVector` gives up (the game stores zeros: no turn).
fn axis_angle(m: DMat3) -> Option<(DVec3, f64)> {
    // EigenVector works on the stored rows, which are the columns here.
    let a = m - DMat3::IDENTITY;
    let col = [a.x_axis, a.y_axis, a.z_axis];
    let (mut best, mut row) = (-1.0f64, 0usize);
    for i in 0..3 {
        for j in 0..3 {
            let (r1, r2, c1, c2) = ((i + 1) % 3, (i + 2) % 3, (j + 1) % 3, (j + 2) % 3);
            let cof = (col[r1][c1] * col[r2][c2] - col[r1][c2] * col[r2][c1]).abs();
            if cof > best {
                best = cof;
                row = i;
            }
        }
    }
    let u = normalize(col[(row + 1) % 3].cross(col[(row + 2) % 3]));
    if best < ee::to_f64(EIGEN_MIN) {
        return None;
    }
    let w = normalize(col[(row + 1) % 3]);
    let x = normalize(u.cross(w));
    let f = DMat3::from_cols(u, w, x).transpose();
    let local = f * (m * f.transpose());
    Some((u, local.y_axis.z.atan2(local.y_axis.y)))
}

/// `a2m` (0x00105f80): c I + s [u]x + (1 - c) u u^T.
fn axis_matrix(axis: Option<DVec3>, angle: f64) -> DMat3 {
    let Some(DVec3 { x, y, z }) = axis else { return DMat3::IDENTITY };
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    DMat3::from_cols(
        DVec3::new(c + t * x * x, t * x * y + s * z, t * x * z - s * y),
        DVec3::new(t * x * y - s * z, c + t * y * y, t * y * z + s * x),
        DVec3::new(t * x * z + s * y, t * y * z - s * x, c + t * z * z),
    )
}

/// A keyed rotation: the first key's matrix and, per segment, a duration and
/// the turn from one key's orientation to the next.
#[derive(Clone, Debug)]
struct RotKeyed {
    base: DMat3,
    segs: Vec<(u32, Option<DVec3>, f64)>,
}

impl RotKeyed {
    fn new(keys: &[(u32, [u32; 3])], frames: u32) -> Self {
        let mats: Vec<DMat3> = keys.iter().map(|(_, v)| bits_dmat3(&rot_bits(key_radians(*v)))).collect();
        let mut t = keys[0].0 << 8;
        let mut segs = Vec::with_capacity(keys.len() + 1);
        if t != 0 {
            segs.push((t, None, 0.0));
        }
        let mut aa: (Option<DVec3>, f64) = (None, 0.0);
        for i in 1..keys.len() {
            let t_new = keys[i].0 << 8;
            aa = match axis_angle(mats[i] * mats[i - 1].transpose()) {
                Some((u, a)) => (Some(u), a),
                None => (None, 0.0),
            };
            segs.push((t_new.wrapping_sub(t), aa.0, aa.1));
            t = t_new;
        }
        // The final hold repeats the last turn over (last - N) * 256, wrapped:
        // a creep of angle * t / 2^32.
        let end = frames << 8;
        if t < end {
            segs.push((t.wrapping_sub(end), aa.0, aa.1));
        }
        RotKeyed { base: mats[0], segs }
    }

    fn at(&self, time: Ticks) -> DMat3 {
        let mut w = self.base;
        if self.segs.is_empty() {
            return w;
        }
        let mut t = time;
        let mut i = 0;
        while i + 1 < self.segs.len() && self.segs[i].0 < t {
            let (dur, axis, angle) = self.segs[i];
            w = axis_matrix(axis, angle) * w;
            t = t.wrapping_sub(dur);
            i += 1;
        }
        let (dur, axis, angle) = self.segs[i];
        let frac = if dur != 0 { ee::to_f64(ee::div(ee::from_u32(t), ee::from_u32(dur))) } else { 0.0 };
        axis_matrix(axis, angle * frac) * w
    }
}

#[derive(Clone, Debug)]
enum RotCtrl {
    None,
    Const(Mat3),
    Keyed(RotKeyed),
}

// Records --------------------------------------------------------------------

/// A pose: what `SetAnmCtrlWork` composes an object's local matrix from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub pos: Vec3,
    /// A matrix, not angles: keyed rotations turn about one axis between key
    /// orientations, which no linear blend of Euler angles reproduces.
    pub rot: Mat3,
    pub scale: Vec3,
    /// The object's local transparency (`ccCoord::localtp`), 1 when absent.
    pub alpha: f32,
}

impl Pose {
    /// T(pos) * R * S(scale).
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_cols(
            (self.rot.x_axis * self.scale.x).extend(0.0),
            (self.rot.y_axis * self.scale.y).extend(0.0),
            (self.rot.z_axis * self.scale.z).extend(0.0),
            Vec4::new(self.pos.x, self.pos.y, self.pos.z, 1.0),
        )
    }

    /// The same pose as position, X-Y-Z Euler degrees and scale, for
    /// [`Scene::world`] (exact up to float rounding).
    pub fn transform(&self) -> Transform {
        let r = |row: usize, col: usize| self.rot.col(col)[row];
        let sy = r(0, 2).clamp(-1.0, 1.0);
        let ry = sy.asin();
        let (rx, rz) = if ry.cos() > 1e-6 {
            ((-r(1, 2)).atan2(r(2, 2)), (-r(0, 1)).atan2(r(0, 0)))
        } else {
            (r(2, 1).atan2(r(1, 1)), 0.0)
        };
        Transform {
            pos: self.pos,
            rot: Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()),
            scale: self.scale,
        }
    }
}

/// One object record (0x0102).
#[derive(Clone, Debug)]
pub struct Track {
    /// The object the record names.
    pub object: u32,
    /// What it drives: `object` with ExtObj chunks followed to their target,
    /// as `ccGetExternalIndex` (0x00101a50) does. Several tracks can drive
    /// the same target: the game makes each its own instance of it.
    pub target: u32,
    pub flags: u32,
    pos: Ctrl<3>,
    rot: RotCtrl,
    scale: Ctrl<3>,
    alpha: Ctrl<1>,
}

impl Track {
    /// The pose at `time`, which should lie in 0..=[`Animation::last`] (the
    /// game clamps it there; [`Animation::poses_at`] does too).
    pub fn at(&self, time: Ticks) -> Pose {
        let pos = self.pos.at(time, [0; 3]);
        let scale = self.scale.at(time, [ONE; 3]);
        let rot = match &self.rot {
            RotCtrl::None => Mat3::IDENTITY,
            RotCtrl::Const(m) => *m,
            RotCtrl::Keyed(k) => k.at(time).as_mat3(),
        };
        let v = |b: [u32; 3]| Vec3::new(f32::from_bits(b[0]), f32::from_bits(b[1]), f32::from_bits(b[2]));
        Pose { pos: v(pos), rot, scale: v(scale), alpha: f32::from_bits(self.alpha.at(time, [ONE])[0]) }
    }

    /// Whether anything in the record changes over time.
    pub fn is_keyed(&self) -> bool {
        self.pos.keyed() || self.scale.keyed() || self.alpha.keyed() || matches!(self.rot, RotCtrl::Keyed(_))
    }
}

/// One F_Obj record (0x0101, 13 words: object, an unread word, position,
/// rotation in degrees, scale, transparency, flags): an object's whole
/// transform from its frame on, as `ccStream::DecodeF_Obj` (main 0x0014e950)
/// sets it: `T(pos) * sceVu0RotMatrix(pi * deg / 180) * S(scale)` and
/// `localtp`, the transparency clamped to 0..1.
#[derive(Clone, Copy, Debug)]
pub struct ObjRecord {
    pub frame: u32,
    /// The object the record names, and the one it drives (ExtObj chunks
    /// followed, as for [`Track::target`]).
    pub object: u32,
    pub target: u32,
    pos: [u32; 3],
    rot: [u32; 3],
    scale: [u32; 3],
    alpha: u32,
    /// The byte at +0x30: bit 0 goes to the object's flags (+0xa2).
    pub flags: u32,
}

impl ObjRecord {
    /// The pose it sets.
    pub fn pose(&self) -> Pose {
        let v = |b: [u32; 3]| Vec3::new(f32::from_bits(b[0]), f32::from_bits(b[1]), f32::from_bits(b[2]));
        let a = f32::from_bits(self.alpha);
        let alpha = if a < 0.0 {
            0.0
        } else if a <= 1.0 {
            a
        } else {
            1.0
        };
        Pose { pos: v(self.pos), rot: bits_mat3(&rot_bits(const_radians(self.rot))), scale: v(self.scale), alpha }
    }
}

/// One material record (0x0202): texture offset controllers.
#[derive(Clone, Debug)]
pub struct MaterialTrack {
    /// The MAT_ object.
    pub material: u32,
    u: Ctrl<1>,
    v: Ctrl<1>,
}

impl MaterialTrack {
    /// U and V, 0 when absent. `SetAnmCtrlWork` stores int(value * 4096)
    /// minus the Material chunk's cropU/cropV in `ccMaterial::u/v`; how the
    /// draw applies those is not traced.
    pub fn at(&self, time: Ticks) -> [f32; 2] {
        [f32::from_bits(self.u.at(time, [0])[0]), f32::from_bits(self.v.at(time, [0])[0])]
    }

    /// What `SetAnmCtrlWork` stores in `ccMaterial::u` and `::v`: each offset
    /// as s16(int(x * 4096)) minus the Material chunk's cropU / cropV, in 16
    /// bits.
    pub fn offsets(&self, time: Ticks, crop_u: u16, crop_v: u16) -> [u16; 2] {
        let fixed = |x: u32, crop: u16| (ee::to_int(ee::mul(x, 0x4580_0000)) as i16).wrapping_sub(crop as i16) as u16;
        [fixed(self.u.at(time, [0])[0], crop_u), fixed(self.v.at(time, [0])[0], crop_v)]
    }
}

/// A morpher's F_Morpher records.
#[derive(Clone, Debug)]
pub struct MorphTrack {
    /// The MPH_ object.
    pub morpher: u32,
    /// (frame, [(target MDL_ object, weight)]) in frame order.
    pub keys: Vec<(u32, Vec<(u32, f32)>)>,
}

/// One F_Note record (0x0108): u32 object, u32 event, u32 param, after its
/// frame's Top. `ccStream::DecodeF_Note` (0x0014f830) makes a `ccAnmNote`
/// of it: +0x4 `event`, +0x8 `param`, +0xc `time`, +0x10 `coord` (the
/// object's instance, `ccAnmIndex.subst`), +0x14 `ccstag`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoteRecord {
    /// The frame whose Top it follows.
    pub frame: u32,
    /// The object it names (`OBJ_xnote` in all 2,697 records in DATA.BIN);
    /// `ccAnmNote.coord` is that object's instance.
    pub object: u32,
    /// What the note functions switch on: 0x8005 a normal attack's hit,
    /// 0x8003 an enemy's skill start, 0x8002 a shock wave, 1 and 2 sounds
    /// (DATA.BIN's records use 2, 3, 0x8002, 0x8003 and 0x8005).
    pub event: u32,
    pub param: u32,
}

/// [`Animation::forward`] for a clip of `frames` frames, looping or not:
/// the same step without the animation's records (a caller that keeps only
/// a clip's length, as a boss's rules do).
pub fn forward_clip(frames: u32, looping: bool, time: Ticks, mut step: Ticks) -> Forward {
    let last = frames.saturating_sub(1) << 8;
    let mut new = time.wrapping_add(step);
    let mut ended = false;
    if last < new {
        step = step.wrapping_sub(new - last);
        new = last;
        ended = true;
    }
    let pose_at = (step != 0).then_some(new);
    if new >> 8 != time >> 8 {
        ended = false;
        if new >> 8 >= frames.saturating_sub(1) {
            if looping {
                new = 0;
            } else {
                ended = true;
            }
        }
    }
    Forward { time: new, pose_at, ended }
}

/// What one `_AnimateForward` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Forward {
    /// The time after the step.
    pub time: Ticks,
    /// The time the pose was evaluated at; None for a zero step.
    pub pose_at: Option<Ticks>,
    /// `_AnimateForward`'s result: a play-once animation has reached its end.
    pub ended: bool,
}

#[derive(Clone, Debug)]
pub struct Animation {
    /// The ANM_ object.
    pub object: u32,
    /// Where its chunk starts in the file.
    pub offset: usize,
    pub frames: u32,
    /// Ends with Top -2: restarts; otherwise (-1) holds its last frame.
    pub looping: bool,
    pub tracks: Vec<Track>,
    pub materials: Vec<MaterialTrack>,
    pub morphs: Vec<MorphTrack>,
    /// The F_Note records in file order (so frame order).
    pub notes: Vec<NoteRecord>,
    /// The F_Obj records in file order (so frame order).
    pub objs: Vec<ObjRecord>,
    /// Sub-chunk kinds read but not evaluated here, with counts: F_Camera
    /// 0x0502, ambient and lights 0x0601, 0x0603, 0x0605, 0x0607, 0x0609
    /// (the streams' scenes evaluate these themselves).
    pub other: Vec<(u16, u32)>,
}

fn read_ctrl<const N: usize>(d: &[u8], q: &mut usize, kind: u32, frames: u32) -> Result<Ctrl<N>> {
    let vals = |at: usize| -> Result<[u32; N]> {
        let mut v = [0; N];
        for (k, x) in v.iter_mut().enumerate() {
            *x = d.u32_at(at + 4 * k)?;
        }
        Ok(v)
    };
    match kind {
        0 => Ok(Ctrl::None),
        1 => {
            let v = vals(*q)?;
            *q += 4 * N;
            Ok(Ctrl::Const(v))
        }
        2 => {
            let keys = read_keys::<N>(d, q)?;
            Ok(Ctrl::Keyed(Keyed::new(&keys, frames)))
        }
        k => format_err(format!("controller kind {k} at 0x{:x}", *q)),
    }
}

fn read_keys<const N: usize>(d: &[u8], q: &mut usize) -> Result<Vec<(u32, [u32; N])>> {
    let n = d.u32_at(*q)? as usize;
    if n == 0 {
        return format_err(format!("keyed controller without keys at 0x{:x}", *q));
    }
    let mut keys = Vec::with_capacity(n);
    for i in 0..n {
        let p = *q + 4 + (4 + 4 * N) * i;
        let mut v = [0; N];
        for (k, x) in v.iter_mut().enumerate() {
            *x = d.u32_at(p + 4 + 4 * k)?;
        }
        keys.push((d.u32_at(p)?, v));
    }
    *q += 4 + (4 + 4 * N) * n;
    Ok(keys)
}

fn read_rot(d: &[u8], q: &mut usize, kind: u32, frames: u32) -> Result<RotCtrl> {
    match kind {
        0 => Ok(RotCtrl::None),
        1 => {
            let deg = [d.u32_at(*q)?, d.u32_at(*q + 4)?, d.u32_at(*q + 8)?];
            *q += 12;
            Ok(RotCtrl::Const(bits_mat3(&rot_bits(const_radians(deg)))))
        }
        2 => {
            let keys = read_keys::<3>(d, q)?;
            Ok(RotCtrl::Keyed(RotKeyed::new(&keys, frames)))
        }
        k => format_err(format!("rotation controller kind {k} at 0x{:x}", *q)),
    }
}

/// Follow ExtObj targets to the object they stand for.
fn resolve(ext: &HashMap<u32, u32>, mut obj: u32) -> u32 {
    for _ in 0..64 {
        match ext.get(&obj) {
            Some(&t) if t != obj => obj = t,
            _ => break,
        }
    }
    obj
}

impl Animation {
    pub fn read(c: &Ccs, scene: &Scene, anime: &Anime) -> Result<Self> {
        let d = &c.data;
        let words = d.u32_at(anime.offset + 16)? as usize;
        let frames = d.u32_at(anime.offset + 12)?;
        let mut a = Animation {
            object: d.u32_at(anime.offset + 8)?,
            offset: anime.offset,
            frames,
            looping: false,
            tracks: Vec::new(),
            materials: Vec::new(),
            morphs: Vec::new(),
            notes: Vec::new(),
            objs: Vec::new(),
            other: Vec::new(),
        };
        let mut p = anime.offset + 20;
        let end = p + 4 * words;
        let mut frame = 0u32;
        while p < end {
            let kind = d.u32_at(p)? as u16;
            let n = d.u32_at(p + 4)? as usize;
            let mut q = p + 8;
            match kind {
                ccs::TOP => {
                    frame = d.u32_at(q)?;
                    if frame >= 0xffff_ff00 {
                        a.looping = frame == END_LOOP;
                    }
                }
                OBJECT => {
                    let object = d.u32_at(q)?;
                    let flags = d.u32_at(q + 4)?;
                    q += 8;
                    let pos = read_ctrl::<3>(d, &mut q, flags & 7, frames)?;
                    let rot = read_rot(d, &mut q, flags >> 3 & 7, frames)?;
                    let scale = read_ctrl::<3>(d, &mut q, flags >> 6 & 7, frames)?;
                    let alpha = read_ctrl::<1>(d, &mut q, flags >> 9 & 7, frames)?;
                    let target = resolve(&scene.ext, object);
                    a.tracks.push(Track { object, target, flags, pos, rot, scale, alpha });
                }
                MATERIAL => {
                    let material = d.u32_at(q)?;
                    let flags = d.u32_at(q + 4)?;
                    q += 8;
                    let u = read_ctrl::<1>(d, &mut q, flags & 7, frames)?;
                    let v = read_ctrl::<1>(d, &mut q, flags >> 3 & 7, frames)?;
                    a.materials.push(MaterialTrack { material, u, v });
                }
                F_MORPHER => {
                    let morpher = d.u32_at(q)?;
                    let count = d.u16_at(q + 4)? as usize;
                    let targets = (0..count)
                        .map(|i| Ok((d.u32_at(q + 8 + 8 * i)?, d.f32_at(q + 12 + 8 * i)?)))
                        .collect::<Result<Vec<_>>>()?;
                    match a.morphs.iter_mut().find(|m| m.morpher == morpher) {
                        Some(m) => m.keys.push((frame, targets)),
                        None => a.morphs.push(MorphTrack { morpher, keys: vec![(frame, targets)] }),
                    }
                }
                F_NOTE => a.notes.push(NoteRecord {
                    frame,
                    object: d.u32_at(q)?,
                    event: d.u32_at(q + 4)?,
                    param: d.u32_at(q + 8)?,
                }),
                F_OBJ => {
                    let object = d.u32_at(q)?;
                    let v = |k: usize| -> Result<[u32; 3]> {
                        Ok([d.u32_at(q + k)?, d.u32_at(q + k + 4)?, d.u32_at(q + k + 8)?])
                    };
                    a.objs.push(ObjRecord {
                        frame,
                        object,
                        target: resolve(&scene.ext, object),
                        pos: v(8)?,
                        rot: v(20)?,
                        scale: v(32)?,
                        alpha: d.u32_at(q + 44)?,
                        flags: d.u32_at(q + 48)?,
                    });
                }
                k => match a.other.iter_mut().find(|(kk, _)| *kk == k) {
                    Some((_, count)) => *count += 1,
                    None => a.other.push((k, 1)),
                },
            }
            p += 8 + 4 * n;
        }
        if p != end {
            return format_err(format!("anime sub-chunks overrun at 0x{p:x}"));
        }
        Ok(a)
    }

    /// Every Anime chunk of a file.
    pub fn all(c: &Ccs, scene: &Scene) -> Result<Vec<Self>> {
        scene.animes.iter().map(|anime| Animation::read(c, scene, anime)).collect()
    }

    pub fn frames(&self) -> u32 {
        self.frames
    }

    /// The last time the animation reaches: (frames - 1) * 256.
    pub fn last(&self) -> Ticks {
        self.frames.saturating_sub(1) << 8
    }

    pub fn clamp(&self, time: Ticks) -> Ticks {
        time.min(self.last())
    }

    /// One `ccAnm::_AnimateForward(step)` from `time`. The time clamps at the
    /// last frame; reaching the last frame's records ends a play-once
    /// animation and resets a looping one to 0 after posing it at the last
    /// frame, so a loop repeats frames 1..N-1 and never shows frame 0 again.
    pub fn forward(&self, time: Ticks, step: Ticks) -> Forward {
        forward_clip(self.frames, self.looping, time, step)
    }

    /// The F_Note records one `_AnimateForward(step)` from `time` leaves on the
    /// anm's note list (`ccAnm.noteRoot`, +0xa0), in the order
    /// `ccAnm::NoteProcess` hands them on: those of frames old + 1 ..= new (the new
    /// clamped to the last; frame 0's are never read), last first. A step that
    /// does not change the frame leaves the list empty.
    pub fn passed_notes(&self, time: Ticks, step: Ticks) -> impl Iterator<Item = &NoteRecord> + '_ {
        let (old, new) = (time >> 8, time.wrapping_add(step).min(self.last()) >> 8);
        self.notes.iter().rev().filter(move |n| old < n.frame && n.frame <= new)
    }

    /// [`Animation::forward`] and the (event, param) of each note it
    /// passed, in the order `NoteProcess` hands them on
    /// ([`Animation::passed_notes`]).
    pub fn forward_notes(&self, time: Ticks, step: Ticks) -> (Forward, Vec<(u32, u32)>) {
        (self.forward(time, step), self.passed_notes(time, step).map(|n| (n.event, n.param)).collect())
    }

    /// The time shown after `elapsed` ticks of continuous play, for steps
    /// that divide the loop (the default one frame per step does).
    pub fn looped(&self, elapsed: u64) -> Ticks {
        let last = self.last() as u64;
        if !self.looping || last == 0 {
            return elapsed.min(last) as Ticks;
        }
        if elapsed == 0 { 0 } else { ((elapsed - 1) % last + 1) as Ticks }
    }

    /// Every track's pose, in track order.
    pub fn poses_at(&self, time: Ticks) -> Vec<Pose> {
        let t = self.clamp(time);
        self.tracks.iter().map(|tr| tr.at(t)).collect()
    }

    /// Every track's pose under the object its record names, in record order:
    /// ExtObj copies of one piece stay separate (each is drawn as its own
    /// instance of [`Track::target`]).
    pub fn controllers_at(&self, time: Ticks) -> Vec<(u32, Pose)> {
        self.tracks.iter().map(|tr| tr.object).zip(self.poses_at(time)).collect()
    }

    /// Local matrices by target object, the first track for a target winning
    /// (as `Scene::anime_frame0` keeps them), then the F_Obj poses of the
    /// objects no track drives; see [`Track::target`] for instances.
    pub fn locals_at(&self, time: Ticks) -> HashMap<u32, Mat4> {
        let mut out = HashMap::new();
        for (tr, pose) in self.tracks.iter().zip(self.poses_at(time)) {
            out.entry(tr.target).or_insert_with(|| pose.matrix());
        }
        for (target, pose) in self.obj_poses_at(time) {
            out.entry(target).or_insert_with(|| pose.matrix());
        }
        out
    }

    /// [`Animation::locals_at`] as Euler transforms, for [`Scene::world`].
    pub fn transforms_at(&self, time: Ticks) -> HashMap<u32, Transform> {
        let mut out = HashMap::new();
        for (tr, pose) in self.tracks.iter().zip(self.poses_at(time)) {
            out.entry(tr.target).or_insert_with(|| pose.transform());
        }
        for (target, pose) in self.obj_poses_at(time) {
            out.entry(target).or_insert_with(|| pose.transform());
        }
        out
    }

    /// The pose each F_Obj-driven target holds at `time`: its last record
    /// at a frame read by then. Records are read from frame 1 on, as a
    /// frame is reached (`DecodeFrameChunk`), so frame 0's never apply.
    pub fn obj_poses_at(&self, time: Ticks) -> Vec<(u32, Pose)> {
        let frame = self.clamp(time) >> 8;
        let mut out: Vec<(u32, Pose)> = Vec::new();
        for r in self.objs.iter().filter(|r| r.frame >= 1 && r.frame <= frame) {
            match out.iter_mut().find(|(t, _)| *t == r.target) {
                Some((_, p)) => *p = r.pose(),
                None => out.push((r.target, r.pose())),
            }
        }
        out
    }

    /// Each morpher's targets and weights at a frame: its last F_Morpher
    /// record at or before it. (The game starts reading records at frame 1,
    /// so frame 0's are only what a viewer would expect, not what it does.)
    pub fn morph_weights_at(&self, frame: u32) -> Vec<(u32, &[(u32, f32)])> {
        self.morphs
            .iter()
            .filter_map(|m| {
                m.keys.iter().take_while(|(f, _)| *f <= frame).last().map(|(_, t)| (m.morpher, t.as_slice()))
            })
            .collect()
    }

    /// Texture offsets by material.
    pub fn uv_at(&self, time: Ticks) -> Vec<(u32, [f32; 2])> {
        let t = self.clamp(time);
        self.materials.iter().map(|m| (m.material, m.at(t))).collect()
    }
}

/// World matrices from local matrices, through the Obj parents, as
/// [`Scene::world`] does for transforms.
pub fn world(scene: &Scene, locals: &HashMap<u32, Mat4>) -> HashMap<u32, Mat4> {
    fn w(s: &Scene, locals: &HashMap<u32, Mat4>, cache: &mut HashMap<u32, Mat4>, obj: u32, depth: u32) -> Mat4 {
        if let Some(m) = cache.get(&obj) {
            return *m;
        }
        let mut m = locals.get(&obj).copied().unwrap_or(Mat4::IDENTITY);
        let parent = s.parent.get(&obj).copied().unwrap_or(0);
        if parent != 0 && parent != obj && depth < 256 {
            m = w(s, locals, cache, parent, depth + 1) * m;
        }
        cache.insert(obj, m);
        m
    }
    let mut cache = HashMap::new();
    for &obj in scene.parent.keys().chain(locals.keys()) {
        w(scene, locals, &mut cache, obj, 0);
    }
    cache
}

/// Morpher chunks (0x1900: u32 morpher, u32 base model): MPH_ -> base MDL_.
pub fn morphers(c: &Ccs) -> Result<HashMap<u32, u32>> {
    let mut out = HashMap::new();
    for ch in c.walk().chunks {
        if ch.in_frames {
            break;
        }
        if ch.kind == MORPHER {
            out.insert(c.data.u32_at(ch.payload())?, c.data.u32_at(ch.payload() + 4)?);
        }
    }
    Ok(out)
}

/// One morph target's stored positions (the matching mmat of the target
/// model), its model's vertexScale and its weight.
#[derive(Clone, Copy, Debug)]
pub struct MorphTarget<'a> {
    pub positions: &'a [[i16; 3]],
    pub scale: f32,
    pub weight: f32,
}

/// The three per-axis weights `Modify` multiplies by: int(w * 4096) as an s16
/// for weights in [0, 8) (all of them on the disc); a negative weight leaves
/// y and z at -1, as the game's packing does.
fn morph_lanes(weight: f32) -> [i32; 3] {
    let w = weight.to_bits();
    let x = ee::to_int(ee::mul(w, 0x4580_0000)) as u32 as u64;
    let v = if w & 0x8000_0000 != 0 { 0xffff_ffff << 32 } else { 0 } | x;
    let t = v | v << 16 | v << 32;
    [0, 1, 2].map(|k| (t >> (16 * k)) as i16 as i32)
}

/// `ccMorpher::Modify` on one mmat's stored positions: a target at another
/// vertexScale is first rescaled to the base's, then every component is
/// base + ((sum of s16(target - base) * weight) >> 12), the sum wrapping in
/// 32 bits and the add saturating. Normals are not morphed. Vertices a
/// target does not have are left to the others.
pub fn morph(base: &[[i16; 3]], base_scale: f32, targets: &[MorphTarget]) -> Vec<[i16; 3]> {
    let bs = base_scale.to_bits();
    let prepared: Vec<(Vec<[i16; 3]>, [i32; 3])> = targets
        .iter()
        .map(|t| {
            let ts = t.scale.to_bits();
            let pos = if ee::eq(ts, bs) {
                t.positions.to_vec()
            } else {
                let ratio = ee::div(ts, bs);
                t.positions
                    .iter()
                    .map(|p| p.map(|c| ee::to_int(ee::mul(ratio, ee::from_int(c as i32))) as i16))
                    .collect()
            };
            (pos, morph_lanes(t.weight))
        })
        .collect();
    base.iter()
        .enumerate()
        .map(|(i, v)| {
            std::array::from_fn(|k| {
                let mut acc = 0i32;
                for (pos, w) in &prepared {
                    if let Some(p) = pos.get(i) {
                        acc = acc.wrapping_add((p[k].wrapping_sub(v[k]) as i32).wrapping_mul(w[k]));
                    }
                }
                ((acc >> 12) as i16 as i32 + v[k] as i32).clamp(-0x8000, 0x7fff) as i16
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ee_arithmetic_truncates() {
        // 1/3 truncates, IEEE would round up the last bit
        assert_eq!(ee::div(ONE, 0x4040_0000), 0x3eaa_aaaa);
        assert_eq!(ee::add(ONE, 0x3380_0000), ONE); // 1 + 2^-24
        assert_eq!(ee::sub(ONE, 0x2000_0000), 0x3f7f_ffff); // 1 - tiny steps down
        assert_eq!(ee::from_u32(0xffff_ffff), 0x4f7f_ffff);
        assert_eq!(ee::to_int(0xcf00_0001), i32::MIN);
        assert_eq!(ee::sqrt(0x4080_0000), 0x4000_0000);
        assert_eq!(ee::mul(0x0080_0000, 0x0080_0000), 0); // underflow to zero
    }

    #[test]
    fn keyed_holds_and_interpolates() {
        let one = 1.0f32.to_bits();
        let three = 3.0f32.to_bits();
        let k = Keyed::<1>::new(&[(2, [one]), (4, [three])], 6);
        assert_eq!(f32::from_bits(k.at(0)[0]), 1.0);
        assert_eq!(f32::from_bits(k.at(2 << 8)[0]), 1.0);
        assert_eq!(f32::from_bits(k.at(3 << 8)[0]), 2.0);
        assert_eq!(f32::from_bits(k.at(5 << 8)[0]), 3.0);
    }

    #[test]
    fn rotations_turn_about_one_axis() {
        // 0 to 90 degrees about z: halfway is 45 degrees about z
        let key = |deg: f32| [0, 0, deg.to_bits()];
        let r = RotKeyed::new(&[(0, key(0.0)), (2, key(90.0))], 3);
        let m = r.at(1 << 8);
        assert!((m.x_axis.x - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-4, "{m:?}");
        assert!((m.x_axis.y - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-4, "{m:?}");
    }

    #[test]
    fn looping_repeats_frames_one_to_last() {
        let a = Animation {
            object: 0,
            offset: 0,
            frames: 3,
            looping: true,
            tracks: vec![],
            materials: vec![],
            morphs: vec![],
            notes: vec![],
            objs: vec![],
            other: vec![],
        };
        let f = a.forward(256, 256);
        assert_eq!(f, Forward { time: 0, pose_at: Some(512), ended: false });
        assert_eq!(a.looped(512), 512);
        assert_eq!(a.looped(513), 1);
        let once = Animation { looping: false, ..a };
        assert_eq!(once.forward(256, 1000), Forward { time: 512, pose_at: Some(512), ended: true });
        assert_eq!(once.forward(512, 256), Forward { time: 512, pose_at: None, ended: true });
    }

    #[test]
    fn notes_go_out_last_first() {
        let note = |frame, event, param| NoteRecord { frame, object: 1, event, param };
        let a = Animation {
            object: 0,
            offset: 0,
            frames: 5,
            looping: true,
            tracks: vec![],
            materials: vec![],
            morphs: vec![],
            notes: vec![note(0, 9, 9), note(1, 2, 0), note(3, 2, 2), note(3, 0x8005, 1), note(4, 2, 7)],
            objs: vec![],
            other: vec![],
        };
        // one frame at a time: frame 0's is never handed on
        assert_eq!(a.forward_notes(0, 256).1, vec![(2, 0)]);
        assert_eq!(a.forward_notes(256, 256).1, vec![]);
        assert_eq!(a.forward_notes(512, 256).1, vec![(0x8005, 1), (2, 2)]);
        // within a frame, and a zero step: nothing
        assert_eq!(a.forward_notes(768, 100).1, vec![]);
        assert_eq!(a.forward_notes(768, 0).1, vec![]);
        // several frames at once, the last frame's first; the loop's end
        let (f, notes) = a.forward_notes(300, 5000);
        assert_eq!(f.time, 0);
        assert_eq!(notes, vec![(2, 7), (0x8005, 1), (2, 2)]);
        // held at a play-once animation's end
        let once = Animation { looping: false, ..a };
        let (f, notes) = once.forward_notes(1024, 256);
        assert!(f.ended && notes.is_empty());
    }

    #[test]
    fn morph_is_integer() {
        let base = [[0i16, 100, -100]];
        let tgt = [[4096i16, 200, 32767]];
        let out = morph(&base, 1.0, &[MorphTarget { positions: &tgt, scale: 1.0, weight: 0.5 }]);
        // (4096 * 2048) >> 12 = 2048; (100 * 2048) >> 12 = 50; 32867 wraps to -32669
        assert_eq!(out, vec![[2048, 150, -100 + ((-32669i32 * 2048) >> 12) as i16]]);
    }
}
