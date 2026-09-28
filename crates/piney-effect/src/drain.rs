//! Drains around a target: the orbs an HP or SP drain sends from its victim
//! to the drainer (`effDrainCtrl` main 0x001d79f0, `effDrain` 0x001d7c40),
//! and the drain file's wave over a character after a Data Drain
//! (`effAfterDrain` 0x001cd380). The same file's animations when a protect
//! breaks or comes back (`effProtect` 0x001cf250) are [`crate::hit`]'s.
//!
//! ```text
//! effDrainCtrl(ap, bp, type, time, num)     type 0 HP, 1 SP: num orbs from
//!                                           bp to ap over time frames
//!   effect -24 (a controller): param type, lifeTime time, flags num,
//!     target ap, posT its pos, temp[0] bp, temp[1] num / time
//!   and the word over bp: effect 154 (HP) or 153 (SP), crate::hit's words
//!   second chain of -24 (0x001cbcac), each frame: temp[2] += temp[1]; when
//!     its integer part grows by k and both ap and bp are on the lists,
//!     effDrain(ap, bp, type, k); flags -= k; ends when flags reach 0
//! effDrain(ap, bp, type, num)               num orbs:
//!   effect 132 (HP, EFF_x000 with CLT_x000c3) or 133 (SP, EFF_x002 with
//!   CLT_x002): lifeTime 240, target ap, posT its pos; at bp's middle
//!   (pos, z plus half its height); r = rand() >> 3 picks the direction
//!   (a turn about z by (r & 0x1f00), tilted down 45 degrees, turned about
//!   y by (r << 8 & 0xff00), then about z by the heading from ap to bp)
//!   into rot, and the speed velocity = 33 - 3.3 (r % 101) / 100; temp[0]
//!   2048, temp[1] 1; the sprite at scale 3, pattern sn % patNum; and a
//!   generator (particleGeneratorTbl[227 + type]) on the orb, switched by
//!   its temp[1]
//!   first switch (0x001c6280): the orb flies 3 frames straight, then
//!   homes on ap's middle turning rot toward it by at most temp[0] (a 16-bit
//!   angle, 2048 = 11.25 degrees, growing by 48 a frame from count 31 up to
//!   16384) as a quaternion rotation; it ends within ap's width, when ap
//!   leaves the lists or at its last frame (temp[1], the generator's
//!   syncSW, cleared)
//! effAfterDrain(target, size)    effect 12 + size (ANM_xdhdref0-2) at the
//!   target: the wave left after a Data Drain
//! ```
//!
//! The animations have no code of their own: `ccEffect::Main` steps and
//! draws them, and they end with their animation.

use crate::ee::{self, F, ONE, V4};
use crate::effect::{EffectCtrl, Next, Obj};
use crate::{CharRef, Cx, IntRef, VecRef, dmath, space, vu};

/// `effDrainCtrl`'s controller.
pub const DRAIN_CTRL: i16 = -24;
/// The words over the victim: 154 for HP, 153 for SP.
pub const WORD_HP: i16 = 154;
pub const WORD_SP: i16 = 153;
/// The orbs: 132 HP (`EFF_x000`), 133 SP (`EFF_x002`).
pub const ORB_HP: i16 = 132;
pub const ORB_SP: i16 = 133;
/// `particleCcsAnmTbl` rows (main 0x003739e0) whose palettes the orbs
/// take: 104 `CLT_x000c3` (HP), 118 `CLT_x002` (SP).
pub const ORB_CLUT: [&str; 2] = ["CLT_x000c3", "CLT_x002"];
/// `particleGeneratorTbl[227 + type]`: the orb's trail.
pub const ORB_GENERATOR: usize = 227;
/// `effAfterDrain`'s first id (12-14 by size).
pub const AFTER_DRAIN: i16 = 12;

/// The drain file's size argument from `ccCheckObjectSize` (gcmn
/// 0x0042e120, the entry's table's size: 1, 3 or 4 for every enemy), as
/// `effProtect` and `effAfterDrain` map a size of -1: 1 to 2 (large), 3 to
/// 1, 4 to 0 (small); any other leaves -1, where the game goes on with an
/// id it never set (None here).
pub fn size_of_object(object_size: i32) -> Option<i32> {
    match object_size {
        1 => Some(2),
        3 => Some(1),
        4 => Some(0),
        _ => None,
    }
}

/// `effDrainCtrl(ap, bp, type, time, num)` (main 0x001d79f0): the
/// controller and the word; returns the word's slot as the game does.
/// A type other than 0 or 1 starts only the controller (the game's word
/// id is then a register it never set).
pub fn eff_drain_ctrl(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    ap: CharRef,
    bp: CharRef,
    kind: i32,
    time: i32,
    num: i32,
) -> Option<usize> {
    if let Some(i) = ctrl.new_effect(cx, DRAIN_CTRL) {
        let pos = cx.host.char_pos(ap);
        let e = &mut ctrl.effects[i];
        e.param = kind;
        e.life_time = ee::to_int(ee::from_int(time)) as i16;
        e.flags = num;
        e.target = Some(ap);
        e.pos_t = pos;
        e.temp[0] = bp;
        e.temp[1] = ee::div(ee::from_int(num), ee::from_int(time));
    }
    let id = match kind {
        1 => WORD_SP,
        0 => WORD_HP,
        _ => return None,
    };
    crate::hit::particle_word(ctrl, cx, bp, &[], id)
}

/// Effect -24's case of the second chain (main 0x001cbcac).
pub fn ctrl_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    let before = ee::to_int(e.temp[2]);
    e.temp[2] = ee::add(e.temp[2], e.temp[1]);
    let k = ee::to_int(e.temp[2]).wrapping_sub(before);
    if k > 0 {
        let (ap, bp, kind) = (e.target, e.temp[0], e.param);
        if let Some(ap) = ap.filter(|&a| cx.host.check_target(a) && cx.host.check_target(bp)) {
            eff_drain(ctrl, cx, ap, bp, kind, k);
        }
        let e = &mut ctrl.effects[i];
        e.flags = e.flags.wrapping_sub(k);
    }
    let e = &mut ctrl.effects[i];
    if e.flags <= 0 {
        e.end_flag = true;
    }
}

/// `normal2Angle(av, nv)` (main 0x00155210): the angles of a direction,
/// (-atan2f(z, x x + y y), 0, atan2f(x, -y), 1).
pub fn normal_to_angle(v: V4) -> V4 {
    let dd = ee::dot([v[0], v[1], 0, ONE], [v[0], v[1], 0, ONE]);
    let x = ee::neg(ee::atan2f(v[2], dd));
    let z = ee::atan2f(v[0], ee::mul(0xbf80_0000, v[1]));
    [x, 0, z, ONE]
}

/// A character's middle: its position, z plus half its height.
fn middle(cx: &Cx, c: CharRef) -> V4 {
    let mut p = cx.host.char_pos(c);
    p[2] = ee::add(p[2], ee::mul(0x3f00_0000, cx.host.char_height(c)));
    p
}

/// `effDrain(ap, bp, type, num)` (main 0x001d7c40): `num` orbs from `bp`
/// to `ap`; the last one's slot. A type other than 0 or 1 starts nothing
/// (the game's ids are then registers it never set).
pub fn eff_drain(ctrl: &mut EffectCtrl, cx: &mut Cx, ap: CharRef, bp: CharRef, kind: i32, num: i32) -> Option<usize> {
    let (id, clut) = match kind {
        0 => (ORB_HP, ORB_CLUT[0]),
        1 => (ORB_SP, ORB_CLUT[1]),
        _ => return None,
    };
    const SCALE: F = 0x4040_0000;
    let mut last = None;
    for _ in 0..num {
        let Some(i) = ctrl.new_effect(cx, id) else {
            last = None;
            continue;
        };
        last = Some(i);
        let ap_pos = cx.host.char_pos(ap);
        let a = middle(cx, ap);
        let b = middle(cx, bp);
        let ang = normal_to_angle(ee::vsub(b, a));
        let r = cx.host.rand() >> 3;
        let t = ee::deg2rad((r & 0x1f00) as i16);
        let mut d = [ee::sinf(t), ee::mul(0xbf80_0000, ee::cosf(t)), 0, 0];
        let t2 = ee::deg2rad(((r << 8) & 0xff00) as u16 as i16);
        d = ee::apply(&vu::rot_y(&vu::UNIT, t2), d);
        d[3] = ONE;
        let m = vu::rot_z(&vu::rot_x(&vu::UNIT, ee::deg2rad(-8192)), ang[2]);
        d = ee::apply(&m, d);
        d[3] = ONE;
        let rot = ee::normalize(d);
        let f = ee::div(ee::mul(0x4053_3333, ee::from_int(r % 101)), 0x42c8_0000);
        let velocity = ee::sub(0x4204_0000, f);
        let clut = cx.assets.find("particle", clut);
        let e = &mut ctrl.effects[i];
        e.life_time = 240;
        e.target = Some(ap);
        e.pos_t = ap_pos;
        e.pos = b;
        e.rot = rot;
        e.velocity = velocity;
        e.speed = ee::vscale(rot, velocity);
        e.temp[0] = (e.temp[0] & 0xffff_0000) | 2048;
        e.temp[1] = 1;
        let sn = e.sn;
        if let Obj::Eff(eff) = &mut e.obj {
            eff.clut = clut;
            eff.scale_y = SCALE;
            eff.scale_x = SCALE;
            e.tex_anm_pat = (sn % u32::from(eff.pat_num)) as u16;
        }
        let mut g = cx.particles.generator(cx.assets, ORB_GENERATOR + kind as usize);
        g.sync_sw = Some(IntRef::EffectTemp(i, 1));
        g.sync_pos_type = false;
        g.sync_pos = Some(VecRef::EffectPos(i));
        cx.particles.start(g);
    }
    last
}

/// Ends an orb: `endFlag`, and `temp[1]` 0 (its generator's switch).
fn orb_end(ctrl: &mut EffectCtrl, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    e.end_flag = true;
    e.temp[1] = 0;
    Next::Draw
}

/// `pos += speed`, w 1, then `ccTransPosFW2LW`.
fn orb_move(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
    let e = &mut ctrl.effects[i];
    e.pos = ee::vadd(e.pos, e.speed);
    e.pos[3] = ONE;
    e.pos = space::fw2lw(e.pos, player, bounds);
    Next::Draw
}

/// Effects 132 and 133's case of the first switch (main 0x001c6280).
pub fn orb_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let e = &ctrl.effects[i];
    let target = e.target;
    if !(i32::from(e.age) < i32::from(e.life_time) - 1) {
        return orb_end(ctrl, i);
    }
    let Some(t) = target.filter(|&t| cx.host.check_target(t)) else {
        return orb_end(ctrl, i);
    };
    let e = &mut ctrl.effects[i];
    if e.cnt >= 31 {
        let a = (e.temp[0] as i16).wrapping_add(48).min(16384);
        e.temp[0] = (e.temp[0] & 0xffff_0000) | u32::from(a as u16);
    }
    if e.cnt < 3 {
        return orb_move(ctrl, cx, i);
    }
    let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
    let tp = space::fw2lw(middle(cx, t), player, bounds);
    let e = &mut ctrl.effects[i];
    let me = space::fw2lw(e.pos, player, bounds);
    let d = ee::vsub(tp, me);
    if !ee::lt(cx.host.char_width(t), ee::sqrtf(ee::dot(d, d))) {
        return orb_end(ctrl, i);
    }
    let d = ee::normalize(d);
    let axis = ee::cross(d, e.rot);
    let c = ee::dot(d, e.rot);
    let s = ee::sqrtf(ee::mul(0x4000_0000, ee::add(ONE, c)));
    let q: V4 = if ee::eq(0, s) {
        let h = ee::mul(HALF, ee::deg2rad(32767));
        let mut q = ee::vscale([0, 0, ONE, ONE], dmath::sin_fd(h));
        q[3] = dmath::cos_fd(h);
        q
    } else {
        let r = ee::div(ONE, s);
        [ee::mul(axis[0], r), ee::mul(axis[1], r), ee::mul(axis[2], r), ee::mul(HALF, s)]
    };
    let ang = ee::mul(0x4000_0000, dmath::acosf(q[3]));
    let sc = ee::div(ONE, ee::sinf(dmath::acosf(q[3])));
    let mut axis = ee::vscale_xyz(q, sc);
    axis[3] = ONE;
    let mut a = ee::rad2deg(ang) as u16;
    let limit = e.temp[0] as i16;
    if i32::from(limit) < i32::from(a) {
        a = limit as u16;
    }
    let h = ee::mul(HALF, ee::deg2rad(a as i16));
    let mut q = ee::vscale(axis, dmath::sin_fd(h));
    q[3] = dmath::cos_fd(h);
    let m = quat_matrix(q);
    e.rot = ee::normalize(ee::apply(&m, e.rot));
    e.speed = ee::vscale(e.rot, e.velocity);
    orb_move(ctrl, cx, i)
}

const HALF: F = 0x3f00_0000;

/// The rotation of a quaternion as the orb's case builds it (main
/// 0x001c66a8-0x001c67c0): `s = 2 / |q|^2` (the norm through the FPU's
/// accumulator, `mula.s`/`madd.s`/`adda.s`), then the usual products into a
/// unit matrix's upper 3x3, stored as columns.
pub fn quat_matrix(q: V4) -> vu::M4 {
    let [x, y, z, w] = q;
    let acc = ee::mul(x, x);
    let f1 = ee::add(acc, ee::mul(y, y));
    let f0 = ee::mul(z, z);
    let acc = ee::add(f1, f0);
    let n = ee::add(acc, ee::mul(w, w));
    let s = if ee::le(n, 0) { 0 } else { ee::div(0x4000_0000, n) };
    let xx = ee::mul(s, ee::mul(x, x));
    let yy = ee::mul(s, ee::mul(y, y));
    let zz = ee::mul(s, ee::mul(z, z));
    let xy = ee::mul(s, ee::mul(x, y));
    let xz = ee::mul(s, ee::mul(x, z));
    let yz = ee::mul(s, ee::mul(y, z));
    let wx = ee::mul(s, ee::mul(x, w));
    let wy = ee::mul(s, ee::mul(y, w));
    let wz = ee::mul(s, ee::mul(w, z));
    let mut m = vu::UNIT;
    m[0][0] = ee::sub(ONE, ee::add(yy, zz));
    m[1][0] = ee::add(xy, wz);
    m[2][0] = ee::sub(xz, wy);
    m[0][1] = ee::sub(xy, wz);
    m[1][1] = ee::sub(ONE, ee::add(xx, zz));
    m[2][1] = ee::add(yz, wx);
    m[0][2] = ee::add(xz, wy);
    m[1][2] = ee::sub(yz, wx);
    m[2][2] = ee::sub(ONE, ee::add(xx, yy));
    m
}

/// `effAfterDrain(target, size)` (main 0x001cd380): the drain file's wave
/// at `target`, `size` 0-2 (small to large; [`size_of_object`]).
pub fn eff_after_drain(ctrl: &mut EffectCtrl, cx: &mut Cx, target: CharRef, size: i32) -> Option<usize> {
    if !(0..=2).contains(&size) {
        return None;
    }
    let i = ctrl.new_effect(cx, AFTER_DRAIN + size as i16)?;
    let pos = cx.host.char_pos(target);
    let e = &mut ctrl.effects[i];
    e.pos = pos;
    e.target = Some(target);
    e.pos_t = pos;
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Camera;
    use crate::ee::k;
    use crate::host::{CharState, Simple};

    #[test]
    fn five_orbs_fly_home() {
        let Some(mut fx) = crate::testing::effects() else { return };
        fx.ctrl.town = false;
        let mut seed = 1u64;
        let mut r = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 32) & 0x7fff_ffff) as i32
        };
        let eye = [0, k(-900.0), k(400.0), ONE];
        let cam = Camera { eye, cam_pos: eye, cam_view: [0, 0, k(100.0), ONE], ..Camera::default() };
        let mut host = Simple::new(&mut r, [0, 0, 0, ONE], cam);
        host.chars.push(CharState { id: 1, pos: [0, 0, 0, ONE], dirc: [0; 4], height: k(160.0), width: k(45.0) });
        host.chars.push(CharState {
            id: 2,
            pos: [k(400.0), k(300.0), 0, ONE],
            dirc: [0; 4],
            height: k(120.0),
            width: k(60.0),
        });
        let word = fx.drain_ctrl(&mut host, 1, 2, 0, 5, 5).unwrap();
        assert_eq!(fx.ctrl.effects[word].id, WORD_HP);
        let (mut orbs, mut frames) = (Vec::new(), 0);
        while fx.ctrl.effects.iter().any(|e| e.status != 0) && frames < 300 {
            fx.step(&mut host);
            orbs.extend(fx.particles.started.drain(..).map(|g| (g.row, g.sync_sw)));
            frames += 1;
        }
        assert_eq!(orbs.len(), 5);
        assert!(orbs.iter().all(|&(row, sw)| row == ORB_GENERATOR && matches!(sw, Some(IntRef::EffectTemp(_, 1)))));
        assert!(frames < 240, "the orbs reach the drainer: {frames}");
        assert_eq!(size_of_object(1), Some(2));
        assert_eq!(size_of_object(2), None);
    }
}
