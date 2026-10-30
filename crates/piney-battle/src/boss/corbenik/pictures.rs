//! What Corbenik's forms and effects draw beside the bodies, as pictures
//! ([`Picture`]): the second form's barrier at its body's `OBJ_ex8body2`,
//! its press's images, elements and drain's root; the star dust's lasers
//! and the refraction laser (`xeffect`'s `OBJ_ex31les*`).

use piney_data::field::ee;
use piney_data::libm;

use super::{Cx, F, V4};
use crate::boss::{AnmLayer, Blend, ONE, Out, PicFile, Picture, Root};
use crate::geom::{self, M4};

/// The second form's barrier object, and the node it rides.
const BARRIER: &str = "OBJ_ex8bria";
const BODY2: &str = "OBJ_ex8body2";
/// The clips its elements and its drain's root play (`ccBoss08_b`'s
/// constructor; `_InitElement` 0x003b74b0 copies the body's neutral clip
/// one step on; the drain's `ANM_ex8natc4`).
const ELEMENT_CLIP: &str = "ANM_ex8nut2";
const ELEMENT_TIME: u32 = 256;
const ROOT_L: &str = "ANM_ex8natc4";
/// The lasers' objects and head clip in xeffect, and their fog
/// (`SetFogBlend(50, 0xff0000)` over the second object).
const LES0: &str = "OBJ_ex31les0";
const LES1: &str = "OBJ_ex31les1";
const LES3: &str = "ANM_ex31les3";
const LASER_FOG: Blend = Blend { rate: 0x4248_0000, colour: 0x00ff_0000 };
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;

/// `SetMatrix_PosRotXYZ(pos, rot)`.
fn pos_rot_xyz(pos: V4, rot: V4) -> M4 {
    let m = geom::rot_matrix_x(&geom::unit_matrix(), rot[0]);
    let m = geom::rot_matrix_y(&m, rot[1]);
    let m = geom::rot_matrix_z(&m, rot[2]);
    geom::trans_matrix(&m, pos)
}

/// A laser segment's matrix (`ccBossEffLaserRain::_Init`'s +0x240 moved
/// to the segment, `ccBossEffRefrectionLaser::Draw`'s per segment): its
/// length scale on z, turned -pi/2 about x, then about x by `pitch` and
/// z by `yaw`, at `pos`.
fn segment(pos: V4, pitch: F, yaw: F, len: F) -> M4 {
    let mut m = geom::unit_matrix();
    m[2][2] = len;
    let m = geom::rot_matrix_x(&m, NEG_HALF_PI);
    let m = geom::rot_matrix_x(&m, pitch);
    let m = geom::rot_matrix_z(&m, yaw);
    geom::trans_matrix(&m, pos)
}

/// A laser head's clip matrix: turned +pi/2 about x, then by `pitch` and
/// `yaw`, at `pos`.
fn head(pos: V4, pitch: F, yaw: F) -> M4 {
    let r = geom::rot_matrix_x(&geom::unit_matrix(), pitch);
    let r = geom::rot_matrix_z(&r, yaw);
    let x = geom::rot_matrix_x(&geom::unit_matrix(), HALF_PI);
    geom::trans_matrix(&geom::mul_matrix(&r, &x), pos)
}

/// `ccGetDist3D` and the pitch and heading from `a` to `b`
/// (`atan2f(a.z - b.z, ccGetDist(a, b))`, `ccGetDirc(a, b)`).
fn way(a: V4, b: V4) -> (F, F, F) {
    let d = ee::sqrt(ee::add(
        ee::mul(ee::sub(b[0], a[0]), ee::sub(b[0], a[0])),
        ee::mul(ee::sub(b[1], a[1]), ee::sub(b[1], a[1])),
    ));
    let dz = ee::sub(b[2], a[2]);
    let d3 = ee::sqrt(ee::add(ee::mul(d, d), ee::mul(dz, dz)));
    let pitch = libm::atan2f(ee::sub(a[2], b[2]), d);
    (d3, pitch, crate::enemy_ai::get_dirc(a, b))
}

/// The two objects of a laser segment, the second in the red fog.
/// `OBJ_ex31les1`'s model is set to `alphaBlendTbl[1]` (and Z unwritten)
/// by both effects' constructors.
fn laser_pair(cx: &mut Cx, world: M4, alpha: F, first: &'static str, second: &'static str) {
    let blend = |n: &str| (n == LES1).then_some(1);
    let at = Root::Matrix(world);
    let obj = |name, fog| Picture::Obj { file: PicFile::Eff, name, at, alpha: Some(alpha), fog, blend: blend(name) };
    cx.out(obj(first, None));
    cx.out(obj(second, Some(LASER_FOG)));
}

/// A clip of x81 on charLayer.
fn x81(clip: Option<String>, time: u32, at: Root, alpha: Option<F>) -> Out {
    Out::Picture(Picture::Anm { file: PicFile::Boss, layer: AnmLayer::Char, clip, time, at, alpha, blend: None })
}

/// The second form's `PreDrawAnm` (0x003b3600): the barrier object, or
/// its hit clip posed at `time` while a blow flashes it, at the body's
/// node, at its `setTransparency`.
pub(super) fn barrier(cx: &mut Cx, hit: Option<(String, u32)>) {
    let at = Root::Node(BODY2);
    match hit {
        Some((clip, time)) => cx.out(x81(Some(clip), time, at, None)),
        None => cx.out(Picture::Obj { file: PicFile::Boss, name: BARRIER, at, alpha: None, fog: None, blend: None }),
    }
}

/// The body drawn at a press's image: its clip's pose, the place, the
/// heading, its transparency.
pub(super) fn press_image(cx: &mut Cx, time: u32, pos: V4, dirc: V4, alpha: F) {
    cx.out(x81(None, time, Root::At { pos, dirc }, Some(alpha)));
}

/// An element (`BODYELEMENT_T`'s clip) at `pos` turned XYZ by `dirc`.
pub(super) fn element(cx: &mut Cx, pos: V4, dirc: V4, alpha: F) {
    cx.out(x81(Some(ELEMENT_CLIP.into()), ELEMENT_TIME, Root::Matrix(pos_rot_xyz(pos, dirc)), Some(alpha)));
}

/// The root drain's root (`ANM_ex8natc4`) at `pos`.
pub(super) fn root_l(cx: &mut Cx, time: u32, pos: V4, alpha: F) {
    cx.out(x81(Some(ROOT_L.into()), time, Root::Matrix(geom::trans_matrix(&geom::unit_matrix(), pos)), Some(alpha)));
}

/// `ccBossEffLaserRain::Draw` (0x00379940): les1, then les0 in the fog,
/// a twentieth long.
pub(super) fn laser(cx: &mut Cx, pos: V4, pitch: F, yaw: F, alpha: F) {
    laser_pair(cx, segment(pos, pitch, yaw, 0x3d4c_cccd), alpha, LES1, LES0);
}

/// A laser's head: the shared clip at its first segment.
pub(super) fn laser_head(cx: &mut Cx, time: u32, pos: V4, pitch: F, yaw: F) {
    head_anm(cx, time, head(pos, pitch, yaw));
}

/// `ccBossEffRefrectionLaser::Draw` (0x003773d0): les0, then les1 in the
/// fog, `ccGetDist3D / 2000` long.
pub(super) fn refrect_seg(cx: &mut Cx, from: V4, to: V4, alpha: F) {
    let (d, pitch, yaw) = way(from, to);
    laser_pair(cx, segment(from, pitch, yaw, ee::div(d, 0x44fa_0000)), alpha, LES0, LES1);
}

/// The refraction laser's head.
pub(super) fn refrect_head(cx: &mut Cx, time: u32, from: V4, to: V4) {
    let (_, pitch, yaw) = way(from, to);
    head_anm(cx, time, head(from, pitch, yaw));
}

fn head_anm(cx: &mut Cx, time: u32, m: M4) {
    let at = Root::Matrix(m);
    let clip = Some(LES3.into());
    cx.out(Picture::Anm {
        file: PicFile::Eff,
        layer: AnmLayer::Char,
        clip,
        time,
        at,
        alpha: Some(ONE),
        blend: Some(1),
    });
}
