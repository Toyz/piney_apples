//! The third form's eyes and killer eyes (`ccBoss08Eye`, QUA gcmn
//! 0x003be920-0x003bf350; `ccBoss08KillerEye`, 0x003bf360-0x003c1640),
//! each a boss on a character of its own (`bossTbl` row 9; a killer eye
//! takes row 45, 46 or 47 when it is born). The sixteen eyes ride the
//! body at its `DMY_dummy1`, each on an offset of its own, and follow its
//! acts; the three killer eyes are thrown out of it by the eye entry and
//! fight on their own tables until the body's pauses hold them.

use piney_data::field::ee;

use super::third::Third;
use super::{Pic, word};
use crate::boss::Part;
use crate::boss::kyvia::{as_me, dp, fabs, fp, head, plain_boss, set_base_param};
use crate::boss::{
    AfterImageParams, BaseActs, Boss, BossBase, BossData, Class, Cx, EffKind, Out, Patterns, Tbl, VF0,
    base_exec_pattern_index,
};
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, M4, V4};
use crate::item;
use crate::param::cond;
use crate::world::CharHit;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const NEG_PI: F = 0xc049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// `SelectTarget`'s radius: 999999.
const FAR: F = 0x4974_23f0;

/// The eyes' and killer eyes' `bossTbl` row; a killer eye's own from 45.
pub const ROW: usize = 9;
pub const KILLER_ROW: usize = 45;
/// The keys the killer eyes' drains are kept under.
pub const KILLER_GEN_KEY: u32 = 0x4b00_0000;

/// An eye's acts (`Think` 0x003beea0): the body's, which it follows; any
/// other is `ChangeAction(0, 1, 4)`.
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const LASER: i16 = 3;
    pub const TERA_LASER: i16 = 5;
}

/// A killer eye's acts (`Think` 0x003bfc60; any other the neutral).
pub mod killer_act {
    pub const NEUTRAL: i16 = 0;
    pub const GENERATE: i16 = 3;
    pub const BODY_PRESS: i16 = 4;
    pub const WANDER: i16 = 8;
    pub const DEAD: i16 = 14;
}

fn w2p(cx: &Cx, v: V4) -> V4 {
    crate::boss::w2p(cx, v)
}

fn p2w(cx: &Cx, v: V4) -> V4 {
    crate::boss::p2w(cx, v)
}

fn dist(cx: &Cx, a: V4, b: V4) -> F {
    crate::boss::dist(cx, a, b)
}

/// `atan2f(a.z - b.z, ccGetDist(a, b))`.
fn pitch(cx: &Cx, a: V4, b: V4) -> F {
    let d = dist(cx, a, b);
    piney_data::libm::atan2f(ee::sub(a[2], b[2]), d)
}

/// `(len, 0, 0)` turned `Ry(y) Rz(z) Rz(-pi/2)`: a step from one place to
/// another (`UnitMatrix`, `RotMatrixY`, `RotMatrixZ` twice).
fn step_along(y: F, z: F, len: F) -> V4 {
    let m = geom::rot_matrix_y(&geom::unit_matrix(), y);
    let m = geom::rot_matrix_z(&m, z);
    let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
    geom::apply_matrix(&m, [len, 0, 0, ONE])
}

// --- the eyes --------------------------------------------------------------------------------------

/// `ccBoss08Eye`'s own members (Quarantine's layout, 0x293d0 bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Eye {
    /// `m_master` (the body's character) and the eye's slot.
    pub master: usize,
    pub slave_id: i32,
    /// `bodyHit` (+0x240).
    pub hit: CharHit,
    /// +0x29350 `m_posB`: its offset from the body's dummy (in the
    /// player's frame); +0x29360 `m_posBP`; +0x29370 `m_moveVectorB`;
    /// +0x29380 `m_rotDirc`: the turn its offset takes each neutral frame.
    pub pos_b: V4,
    pub pos_bp: V4,
    pub move_vector_b: V4,
    pub rot_dirc: V4,
    /// +0x29390 `m_moveDircB`, +0x29394 `m_moveSpdB`.
    pub move_dirc_b: F,
    pub move_spd_b: F,
    /// +0x293a0 where the laser's ring sends it from, +0x293c0 where to,
    /// +0x293b4 its speed and +0x293b8 the speed's step.
    pub from: V4,
    pub to: V4,
    pub speed: F,
    pub step: F,
    /// +0x293b0 `m_colorBlend`: the flash a reflected laser leaves.
    pub color_blend: F,
    /// `ccBoss::actPos` (+0x1b0): where it was as the laser began.
    pub act_pos: V4,
}

/// What an eye reads of its body this frame.
#[derive(Clone, Copy, Debug)]
pub struct MasterView {
    /// The body's place and target (`posP`, `targetChar`), its target's
    /// way and distance.
    pub pos_p: V4,
    pub target: Option<usize>,
    pub target_dirc: F,
    pub target_dist: F,
    /// `ApplyMatrix(anm, dmyEye->pos)`: the dummy in the world, by the
    /// body's last drawn pose.
    pub dummy: V4,
}

/// `ccBoss08Eye::ccBoss08Eye` (0x003be920): row 9, its clips, on and
/// drawn, its body hit by the row, act 0 (its clip set), at the origin,
/// its turn three `ccRandF(1 degree)`.
pub(crate) fn new_eye(cx: &mut Cx, me: usize) -> Part<Eye> {
    let mut b = plain_boss();
    set_base_param(cx, me, ROW);
    b.anm_tbl = cx.data.corbenik.eye_anims.clone();
    b.exit = 0;
    b.draw_sw = 1;
    let pos = cx.scene.chars[me].pos;
    let base = cx.scene.chars[me].base();
    let hit = CharHit { pos, radius: base.width, height: base.height, kind: base.ty as u32, ..CharHit::default() };
    as_me(cx, me, |cx| b.change_action(cx, act::NEUTRAL, 0, true));
    cx.scene.chars[me].pos = VF0;
    cx.scene.chars[me].pos_p = w2p(cx, VF0);
    let mut rot_dirc = VF0;
    for r in rot_dirc.iter_mut().take(3) {
        *r = rand_f(cx.cc, 0x3c8e_fa35);
    }
    let x = Eye { hit, pos_b: VF0, pos_bp: VF0, move_vector_b: VF0, rot_dirc, ..Eye::default() };
    Part { b: Box::new(Boss { base: b, class: Class::Plain }), x: Box::new(x), me }
}

/// `SetupSlave`'s part of an eye (0x003bd960): `SetMaster`, on, act 0 held
/// (`Order(0)`), and its offset: `|ccRandF(1000)| + 4500` out, turned down
/// by `ccRandF(pi)` (held to 30-150 degrees) and round by `ccRandF(pi)`.
pub(crate) fn setup_eye(p: &mut Part<Eye>, cx: &mut Cx, master: usize, k: i32) {
    p.x.master = master;
    p.x.slave_id = k;
    p.b.base.exit = 0;
    as_me(cx, p.me, |cx| order(&mut p.b.base, &mut p.x, cx, 0));
    let r = fp(dp(fabs(rand_f(cx.cc, 0x447a_0000))) + 4500.0);
    let mut a = rand_f(cx.cc, PI);
    if ee::lt(a, 0x3f06_0a92) {
        a = 0x3f06_0a92;
    } else if !ee::le(a, 0x4027_8d37) {
        a = 0x4027_8d37;
    }
    let m = geom::rot_matrix_y(&geom::unit_matrix(), a ^ 0x8000_0000);
    let m = geom::rot_matrix_z(&m, rand_f(cx.cc, PI));
    p.x.pos_b = geom::apply_matrix(&m, [r, 0, 0, ONE]);
    p.x.pos_bp = w2p(cx, p.x.pos_b);
}

/// `ccBoss08Eye::Order(n)` (0x003bf1a0): 23 the flash (100); else
/// `ChangeAction(n, 3, 0)`.
pub(crate) fn order(b: &mut BossBase, x: &mut Eye, cx: &mut Cx, n: i16) {
    if n == 23 {
        x.color_blend = 0x42c8_0000;
    } else {
        b.change_action(cx, n, 3, false);
    }
}

/// `ccBoss08Eye::Main` (0x003bebe0): the body's target its own, then
/// `ccBoss::Main` with its `Think`, `Move` and `PreDrawAnm`.
pub(crate) fn eye_main(p: &mut Part<Eye>, m: &MasterView, cx: &mut Cx) {
    as_me(cx, p.me, |cx| {
        let me = cx.me;
        p.b.base.target_dirc = m.target_dirc;
        p.b.base.target_dist = m.target_dist;
        cx.scene.chars[me].target_char = m.target;
        let b = &mut p.b;
        if b.base.exit != 0 {
            return;
        }
        head(&mut b.base, cx);
        eye_think(&mut b.base, &mut p.x, m, cx);
        eye_move(&mut b.base, &mut p.x, m, cx);
        if p.b.base.draw_sw != 0 {
            eye_pre_draw(&mut p.b.base, &mut p.x, cx);
            p.b.base.draw(cx);
        }
        cx.scene.chars[me].cond[cond::HOLD] = 0;
    });
}

fn eye_think(b: &mut BossBase, x: &mut Eye, m: &MasterView, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => eye_neutral(b, x, m, cx),
        act::LASER => eye_laser(b, x, m, cx),
        act::TERA_LASER => eye_tera(b, cx),
        _ => b.change_action(cx, act::NEUTRAL, 1, true),
    }
}

/// The heading toward `at` (in the player's frame): its pitch in x, its
/// way in z.
fn face(b: &mut BossBase, cx: &Cx, at: V4) {
    let pp = cx.scene.chars[cx.me].pos_p;
    b.dirc = VF0;
    b.dirc[0] = pitch(cx, pp, at);
    b.dirc[2] = get_dirc(pp, at);
}

/// `OnThinkNeutral` (0x003bef40): facing the body; its offset turned by
/// `m_rotDirc` (`RotMatrixX`, `Y`, `Z` over its translation).
fn eye_neutral(b: &mut BossBase, x: &mut Eye, m: &MasterView, cx: &mut Cx) {
    face(b, cx, m.pos_p);
    let t = geom::trans_matrix(&geom::unit_matrix(), x.pos_b);
    let t = geom::rot_matrix_x(&t, x.rot_dirc[0]);
    let t = geom::rot_matrix_y(&t, x.rot_dirc[1]);
    let t = geom::rot_matrix_z(&t, x.rot_dirc[2]);
    x.pos_b = geom::apply_matrix(&t, VF0);
}

/// `OnThinkTeraLaser` (0x003bf070): facing its target.
fn eye_tera(b: &mut BossBase, cx: &mut Cx) {
    let tp = w2p(cx, b.target_pos);
    face(b, cx, tp);
}

/// `OnThinkLaser` (0x003bf0f0): facing the body's target, its own way
/// stopped.
fn eye_laser(b: &mut BossBase, x: &mut Eye, m: &MasterView, cx: &mut Cx) {
    let at = m.target.map_or(VF0, |t| cx.scene.chars[t].pos_p);
    face(b, cx, at);
    x.move_vector_b = VF0;
    x.move_spd_b = 0;
}

/// `CalcMasterCoord` (0x003bf210): its offset stepped, then its place the
/// body's dummy plus the offset.
fn calc_master_coord(x: &mut Eye, m: &MasterView, cx: &mut Cx) {
    let s = x.move_spd_b;
    if !geom::eq(0, s) {
        x.pos_b[0] = ee::add(x.pos_b[0], ee::mul(s, piney_data::libm::sinf(x.move_dirc_b)));
        x.pos_b[1] = ee::sub(x.pos_b[1], ee::mul(s, piney_data::libm::cosf(x.move_dirc_b)));
    }
    x.pos_b = geom::vadd(x.pos_b, x.move_vector_b);
    let d = w2p(cx, m.dummy);
    let me = cx.me;
    let pp = geom::vadd(d, x.pos_b);
    cx.scene.chars[me].pos_p = pp;
    cx.scene.chars[me].pos = p2w(cx, pp);
}

/// `ccBoss08Eye::Move` (0x003bec10): on the body in its neutral, then the
/// base's step, its body hit pushed (all of the push kept).
fn eye_move(b: &mut BossBase, x: &mut Eye, m: &MasterView, cx: &mut Cx) {
    if b.act_num == act::NEUTRAL {
        calc_master_coord(x, m, cx);
    }
    let me = cx.me;
    let mut pp = w2p(cx, cx.scene.chars[me].pos);
    let s = b.move_spd;
    if !geom::eq(0, s) {
        pp[0] = ee::add(pp[0], ee::mul(s, piney_data::libm::sinf(b.move_dirc)));
        pp[1] = ee::sub(pp[1], ee::mul(s, piney_data::libm::cosf(b.move_dirc)));
    }
    pp = geom::vadd(pp, b.move_vector);
    let mut pos = p2w(cx, pp);
    let on = b.body_hit_sw != 0;
    cx.part_hits.switch(me, &mut x.hit, on);
    if on {
        x.hit.pos = pos;
        if cx.part_hits.collide(me, &mut x.hit) {
            let off = x.hit.offset;
            pos = geom::vadd(pos, off);
            pp = geom::vadd(pp, off);
            x.hit.pos = pos;
            cx.part_hits.sync(me, &x.hit);
        }
    }
    let ch = &mut cx.scene.chars[me];
    ch.pos_p = pp;
    ch.pos = pos;
}

/// `PreDrawAnm` (0x003bed70): the clip forward and drawn at its place,
/// turned XYZ; a flash fading 1/9.6 of its 100 a frame.
fn eye_pre_draw(b: &mut BossBase, x: &mut Eye, cx: &mut Cx) {
    b.anm_status = i8::from(b.anm.forward());
    let me = cx.me;
    let pos = cx.scene.chars[me].pos;
    let blend = x.color_blend;
    cx.out(Out::Corbenik(Pic::Eye { eye: me, time: b.anm.posed, pos, dirc: b.dirc, blend, alpha: b.transparency }));
    if !geom::eq(blend, 0) {
        let mut v = ee::sub(blend, 0x4055_5555);
        if ee::lt(v, 0) {
            v = 0;
        }
        x.color_blend = v;
    }
}

// --- the killer eyes -------------------------------------------------------------------------------

/// `ccBoss08KillerEye`'s own members (Quarantine's layout, 0x293e0 bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Killer {
    pub master: usize,
    pub slave_id: i32,
    pub hit: CharHit,
    /// +0x29350 `m_pauseFlag`: the body held it at its last pattern.
    pub pause: bool,
    /// +0x29354 `m_bossEff`: the drain it dies into.
    pub boss_eff: Option<i32>,
    /// +0x29360 `m_eai`: its press's after-image.
    pub eai: AfterImageParams,
    /// +0x293b0: its spin while thrown (x).
    pub spin: V4,
    /// +0x293c0 the press's reach, +0x293c4 its step, +0x293d0 where it
    /// pressed from.
    pub press_r: F,
    pub press_step: F,
    pub press_start: V4,
    /// `ccBoss::actPos` (+0x1b0): its throw, then the press's end.
    pub act_pos: V4,
}

/// `ccBoss08KillerEye::ccBoss08KillerEye` (0x003bf360): row 9, its clips,
/// out (`OnExit`), at Kite's place 500 behind on y with his heading, its
/// body hit 100 round and 200 high, its clip 0 set, after-images on.
pub(crate) fn new_killer(cx: &mut Cx, me: usize, kite_pos: V4, kite_dirc: V4) -> Part<Killer> {
    let mut b = plain_boss();
    set_base_param(cx, me, ROW);
    b.anm_tbl = cx.data.corbenik.killer_anims.clone();
    b.exit = 1;
    b.draw_sw = 0;
    b.body_hit_sw = 0;
    let mut pos = kite_pos;
    pos[1] = ee::sub(pos[1], 0x43fa_0000);
    cx.scene.chars[me].pos = pos;
    b.dirc = kite_dirc;
    let base = cx.scene.chars[me].base();
    let hit = CharHit { pos, radius: 0x42c8_0000, height: 0x4348_0000, kind: base.ty as u32, ..CharHit::default() };
    if let Some(Some(c)) = b.anm_tbl.first() {
        let c = c.clone();
        b.anm.set(&c, cx.clips);
    }
    let eai = AfterImageParams { color: 0xffff_ffff, ss: VF0, es: VF0, ..AfterImageParams::default() };
    let x = Killer { hit, eai, spin: VF0, press_start: VF0, ..Killer::default() };
    Part { b: Box::new(Boss { base: b, class: Class::Plain }), x: Box::new(x), me }
}

/// What a killer eye reads of the body.
#[derive(Clone, Copy, Debug)]
pub struct KillerView {
    /// The body's character, its `m_pauseSlaveFlag`, its heading and the
    /// dummy's place (as [`MasterView::dummy`]).
    pub master: usize,
    pub paused: bool,
    pub dirc: V4,
    pub dummy: V4,
}

/// `ccBoss08KillerEye::Main` (0x003bf630): `ccBoss::Main` with its
/// `Think`, the base's `Move` and its own `PreDrawAnm`. `mb` is the body
/// (its manager's effects).
pub(crate) fn killer_main(mb: &mut BossBase, p: &mut Part<Killer>, v: &KillerView, cx: &mut Cx) {
    as_me(cx, p.me, |cx| {
        let me = cx.me;
        if p.b.base.exit != 0 {
            return;
        }
        head(&mut p.b.base, cx);
        killer_think(mb, &mut p.b.base, &mut p.x, v, cx);
        killer_move(&mut p.b.base, &mut p.x, cx);
        if p.b.base.draw_sw != 0 {
            killer_pre_draw(&mut p.b.base, cx);
            p.b.base.draw(cx);
        }
        cx.scene.chars[me].cond[cond::HOLD] = 0;
    });
}

fn killer_think(mb: &mut BossBase, b: &mut BossBase, x: &mut Killer, v: &KillerView, cx: &mut Cx) {
    match b.act_num {
        killer_act::DEAD => on_dead(mb, b, x, cx),
        killer_act::GENERATE => on_generate(b, x, v, cx),
        killer_act::BODY_PRESS => on_body_press(b, x, v, cx),
        killer_act::WANDER => on_wander(b, x, v, cx),
        _ => on_neutral(b, x, v, cx),
    }
}

/// `ccBoss::Move` (QUA gcmn 0x00360120) on a killer eye: the step and its
/// throw, then its body hit pushed out of the others, the push's height
/// dropped.
fn killer_move(b: &mut BossBase, x: &mut Killer, cx: &mut Cx) {
    let me = cx.me;
    let mut pp = w2p(cx, cx.scene.chars[me].pos);
    if !geom::eq(0, b.move_spd) {
        pp[0] = ee::add(pp[0], ee::mul(b.move_spd, piney_data::libm::sinf(b.move_dirc)));
        pp[1] = ee::sub(pp[1], ee::mul(b.move_spd, piney_data::libm::cosf(b.move_dirc)));
    }
    pp = geom::vadd(pp, b.move_vector);
    let mut pos = p2w(cx, pp);
    let on = b.body_hit_sw != 0;
    cx.part_hits.switch(me, &mut x.hit, on);
    if on {
        x.hit.pos = pos;
        if cx.part_hits.collide(me, &mut x.hit) {
            let mut off = x.hit.offset;
            off[2] = 0;
            pos = geom::vadd(pos, off);
            pp = geom::vadd(pp, off);
            x.hit.pos = pos;
            cx.part_hits.sync(me, &x.hit);
        }
    }
    let ch = &mut cx.scene.chars[me];
    ch.pos_p = pp;
    ch.pos = pos;
}

/// `PreDrawAnm` (0x003bf990): thrown, posed still; pressing, the clip
/// forward twice in its first step (its own and the base's); else the
/// base's. Drawn at its place, turned ZYX.
fn killer_pre_draw(b: &mut BossBase, cx: &mut Cx) {
    match b.act_num {
        killer_act::GENERATE => {}
        killer_act::BODY_PRESS => {
            if b.act_proccess == 0 {
                b.anm.forward();
            }
            b.anm_status = i8::from(b.anm.forward());
        }
        _ => b.anm_status = i8::from(b.anm.forward()),
    }
    let me = cx.me;
    let pos = cx.scene.chars[me].pos;
    cx.out(Out::Corbenik(Pic::Killer { eye: me, time: b.anm.posed, pos, dirc: b.dirc, alpha: b.transparency }));
}

/// `SetMaster(body, k)`.
pub(crate) fn set_master(x: &mut Killer, master: usize, k: i32) {
    x.master = master;
    x.slave_id = k;
}

/// `Generate()` (0x003bfa60): its member (`k % 3`, or the nearest if that
/// one is gone), out of the body's dummy with its heading, row 45 + `k %
/// 3`, drawn and on but no body hit, unseen, its clip from the start, act
/// 3 held.
pub(crate) fn generate(p: &mut Part<Killer>, v: &KillerView, cx: &mut Cx) {
    let k = p.x.slave_id.rem_euclid(3) as usize;
    as_me(cx, p.me, |cx| {
        let me = cx.me;
        let b = &mut p.b;
        let mut t = cx.party.members[k];
        cx.scene.chars[me].target_char = t;
        if !cx.valid(t) || t.is_some_and(|t| cx.scene.chars[t].cond[cond::DEAD] != 0) {
            t = b.base.select_target(cx, 0, FAR);
            cx.scene.chars[me].target_char = t;
        }
        b.base.calc_target_info(cx);
        cx.scene.chars[me].pos = v.dummy;
        b.base.dirc = v.dirc;
        set_base_param(cx, me, KILLER_ROW + k);
        let b = &mut p.b;
        b.base.draw_sw = 1;
        b.base.exit = 0;
        b.base.body_hit_sw = 0;
        b.base.transparency = 0;
        b.base.set_transparency = 0;
        super::restart(&mut b.base.anm, cx);
        b.base.anm.forward();
        b.base.move_spd = 0;
        b.base.move_vector = VF0;
        b.base.change_action(cx, killer_act::GENERATE, 3, true);
    });
}

/// `Attack()` (0x003bfd20): by its row, pattern 14 (45), 16 (47) or 15
/// (46).
pub(crate) fn attack(p: &mut Part<Killer>, v: &KillerView, cx: &mut Cx) {
    let row = cx.scene.chars[p.me].id();
    let pat = match row {
        46 => 15,
        47 => 16,
        45 => 14,
        _ => return,
    };
    as_me(cx, p.me, |cx| KillerRules::new(&p.x, v).exec_pattern(&mut p.b.base, cx, pat, 0));
}

/// `ChangeNextPattern` (0x003bfe00): held while the body pauses (pattern
/// 2 -1, `m_pauseFlag` set); else the base's.
fn change_next_pattern(b: &mut BossBase, x: &mut Killer, v: &KillerView, cx: &mut Cx) {
    let mut r = KillerRules::new(x, v);
    if v.paused {
        x.pause = true;
        r.exec_pattern(b, cx, 2, -1);
        return;
    }
    x.pause = false;
    r.change_next_pattern(b, cx);
}

/// A killer eye's patterns: what the body tells it, and its table.
struct KillerRules<'a> {
    v: &'a KillerView,
    /// Its pattern table's (`m_slaveId % 3`).
    slot: usize,
}

impl<'a> KillerRules<'a> {
    fn new(x: &Killer, v: &'a KillerView) -> KillerRules<'a> {
        KillerRules { v, slot: x.slave_id.rem_euclid(3) as usize }
    }
}

impl Patterns for KillerRules<'_> {
    const ACTS: BaseActs = super::BASE_ACTS;

    fn words<'d>(&self, _b: &BossBase, data: &'d BossData) -> &'d [i32] {
        &data.corbenik.killer_acts[self.slot]
    }

    /// `ExecPatternIndex` (0x003bfe90): 7 the wander (act 8, its length the
    /// next word in `actProccess`), 14 a skill of `@2388` by `ccRand() % 13` on
    /// its target, 16 skill 154 on the body, 15 a compelled one of `@2391`, 17
    /// the press (act 4); the rest the base's.
    fn exec_pattern_index(&mut self, b: &mut BossBase, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
        let v = self.v;
        let mut s = i + 1;
        let mut pat = word(tbl, i);
        b.move_spd = 0;
        b.move_vector = VF0;
        if pat == -1 {
            pat = word(tbl, 0);
            s = 1;
        }
        b.pat_num = pat;
        let me = cx.me;
        let target = cx.scene.chars[me].target_char;
        match pat {
            7 => {
                b.change_action(cx, killer_act::WANDER, 3, true);
                b.act_proccess = word(tbl, s) as i16;
                s += 1;
            }
            14 => {
                let n = cx.cc.rand() % 13;
                let sid = word(&cx.data.corbenik.killer_skills, n.abs());
                if let Some(tp) = target
                    && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, tp, sid, 0, false, cx.rand)
                {
                    cx.out(Out::Skill(tp, k));
                }
            }
            16 => {
                let m = v.master;
                if let Some(k) = item::item_skill_request(cx.t, cx.scene, me, m, 154, 0, false, cx.rand) {
                    cx.out(Out::Skill(m, k));
                }
            }
            15 => {
                let n = cx.cc.rand() % 3;
                let sid = word(&cx.data.corbenik.killer_compels, n.abs());
                if let Some(tp) = target
                    && let Some(k) = item::item_skill_compel(cx.t, cx.scene, me, tp, sid, 0, false, cx.rand)
                {
                    cx.out(Out::Skill(tp, k));
                }
            }
            17 => b.change_action(cx, killer_act::BODY_PRESS, 3, true),
            _ => return base_exec_pattern_index(self, b, cx, tbl, i),
        }
        s
    }
}

/// The neutral (0x003c00e0): stopped, turning to its target (128); still
/// while the body pauses; then its wait, and the next pattern.
fn on_neutral(b: &mut BossBase, x: &mut Killer, v: &KillerView, cx: &mut Cx) {
    let me = cx.me;
    b.move_spd = 0;
    b.move_vector = VF0;
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 128);
    if v.paused {
        return;
    }
    if b.stop_time > 0 {
        let c = b.stop_counter;
        b.stop_counter += 1;
        if b.stop_time >= c {
            return;
        }
        b.stop_time = 0;
    } else if b.stop_time == -1 {
        return;
    } else if b.stop_time == -2 {
        if cx.scene.chars[me].skill_id != 0 {
            return;
        }
        b.stop_time = 0;
    }
    change_next_pattern(b, x, v, cx);
}

/// The wander (0x003c01f0): still while the body pauses; off the field on
/// annihilation; with no target the next pattern; held while stopped; for
/// `actProccess` frames at 15 toward the target beyond 300, else round it
/// a quarter turn off at 10.
fn on_wander(b: &mut BossBase, x: &mut Killer, v: &KillerView, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    b.move_spd = 0;
    b.move_vector = VF0;
    if v.paused {
        b.move_spd = 0;
        return;
    }
    if cx.annihilated() || cx.game_over {
        KillerRules::new(x, v).exec_pattern(b, cx, 2, -1);
        return;
    }
    let t = cx.scene.chars[me].target_char;
    if !cx.valid(t) || t.is_some_and(|t| cx.scene.chars[t].cond[cond::DEAD] != 0) {
        change_next_pattern(b, x, v, cx);
        return;
    }
    if b.stop != 0 || cx.scene.chars[me].cond[cond::HOLD] != 0 {
        return;
    }
    if i32::from(c) >= i32::from(b.act_proccess) {
        change_next_pattern(b, x, v, cx);
        return;
    }
    if !ee::le(b.target_dist, 0x4396_0000) {
        b.move_spd = 0x4170_0000;
        b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 256);
        b.move_dirc = b.dirc[2];
    } else {
        let mut d = ee::add(HALF_PI, b.target_dirc);
        if !ee::le(d, PI) {
            d = ee::sub(d, TWO_PI);
        }
        b.move_spd = 0x4120_0000;
        b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 256);
        b.move_dirc = d;
    }
}

/// `OnThinkGenerate` (0x003c0420): fading in over 90 frames, drifting out
/// along its heading; at 120 thrown (30 a frame, turned by `@2448` sixths
/// of pi off a quarter turn from its heading) spinning by `ccRandF(pi/2)`;
/// bouncing on the ground (damped 0.8, the fall turned at 0.7) to a stop;
/// 30 frames on facing its target and rising to 200; at 90 on its table,
/// drawn, out, hit, on the lists, its HP full.
fn on_generate(b: &mut BossBase, x: &mut Killer, v: &KillerView, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            b.move_vector = VF0;
            if c < 91 {
                let mut t = ee::add(b.transparency, 0x3c36_0b61);
                if !ee::lt(t, ONE) {
                    t = ONE;
                }
                b.move_vector = VF0;
                let d = b.dirc[2];
                let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI), d);
                let o = geom::apply_matrix(&m, [0x400e_38e4, 0, 0, ONE]);
                b.move_vector = p2w(cx, geom::vadd(w2p(cx, b.move_vector), o));
                b.transparency = t;
                b.set_transparency = t;
            } else if c == 120 {
                let turn = word_f(&cx.data.corbenik.killer_turns, x.slave_id.rem_euclid(3));
                let mut a = ee::add(ee::sub(b.dirc[2], HALF_PI), ee::mul(0x3f06_0a92, turn));
                if !ee::le(a, PI) {
                    a = ee::sub(a, TWO_PI);
                } else if ee::lt(a, NEG_PI) {
                    a = ee::add(a, TWO_PI);
                }
                let _ = cx.rand.rand() >> 3;
                let m = geom::rot_matrix_x(&geom::unit_matrix(), 0);
                let m = geom::rot_matrix_y(&m, 0x8000_0000);
                let m = geom::rot_matrix_z(&m, a);
                let v = geom::apply_matrix(&m, [ONE, 0, 0, ONE]);
                x.act_pos = v.map(|c| ee::mul(c, 0x41f0_0000));
                x.spin = VF0;
                let r = rand_f(cx.cc, HALF_PI);
                x.spin[0] = fp((dp(fabs(r)) + dp(HALF_PI)) / -10.0);
                b.act_proccess += 1;
                b.act_count = 0;
            }
        }
        1 => {
            let pp = geom::vadd(cx.scene.chars[me].pos_p, x.act_pos);
            cx.scene.chars[me].pos_p = pp;
            cx.scene.chars[me].pos = p2w(cx, pp);
            if dp(x.act_pos[2]) < -3.0 && ee::lt(cx.scene.chars[me].pos[2], 0x428c_0000) {
                cx.scene.chars[me].pos[2] = 0x428c_0000;
                x.spin = x.spin.map(|c| ee::mul(c, 0x3f4c_cccd));
                if x.slave_id == 0 {
                    cx.out(Out::SeNote { se: 56, note: 85 });
                }
                let v = &mut x.act_pos;
                v[0] = ee::mul(v[0], 0x3f4c_cccd);
                v[1] = ee::mul(v[1], 0x3f4c_cccd);
                v[2] = ee::mul(v[2], 0xbf33_3333);
                if dp(fabs(v[0])) < 4.0 {
                    v[0] = 0;
                }
                if dp(fabs(v[1])) < 4.0 {
                    v[1] = 0;
                }
                if dp(fabs(v[2])) < 16.0 {
                    b.move_vector = VF0;
                    b.act_count = 0;
                    b.act_proccess += 1;
                    return;
                }
            }
            x.act_pos[2] = ee::sub(x.act_pos[2], 0x4080_0000);
            b.dirc = geom::vadd(b.dirc, x.spin);
            for d in b.dirc.iter_mut().take(3) {
                if ee::lt(*d, NEG_PI) {
                    *d = ee::add(*d, TWO_PI);
                } else if !ee::le(*d, PI) {
                    *d = ee::sub(*d, TWO_PI);
                }
            }
        }
        2 => {
            if c < 30 {
                return;
            }
            b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 256);
            let pp = cx.scene.chars[me].pos_p;
            let a = pitch(cx, pp, b.target_pos_p);
            b.dirc[0] = geom::set_dirc(b.dirc[0], a, 256);
            geom::set_dist(&mut cx.scene.chars[me].pos[2], 0x4348_0000, 0x3d4c_cccd);
            if c < 90 {
                return;
            }
            b.pat_tbl = Tbl::Normal;
            b.pat_index = 0;
            let mut r = KillerRules::new(x, v);
            let tbl = r.words(b, cx.data);
            b.pat_index = r.exec_pattern_index(b, cx, tbl, 0);
            b.draw_sw = 1;
            b.exit = 0;
            b.body_hit_sw = 1;
            crate::fellow::entry_cmnd(cx.scene, me);
            cx.scene.chars[me].hp = cx.scene.chars[me].max_hp;
        }
        _ => {}
    }
}

fn word_f(tbl: &[F], i: i32) -> F {
    usize::try_from(i).ok().and_then(|k| tbl.get(k)).copied().unwrap_or(0)
}

/// `OnThinkBodyPress` (0x003c0c30), six steps: its target (or the nearest;
/// none, the next pattern), off the targets and its body; flying at 200
/// to within 400; drawing back 16 frames (50 out, 4.44 less a frame);
/// diving 100 a frame to half the target's height, boss skill 60 there;
/// 15 frames on its way back worked out; flying back (an after-image
/// every other frame) to where it started, on the lists again and the
/// next pattern. The target held all the while.
fn on_body_press(b: &mut BossBase, x: &mut Killer, v: &KillerView, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    let target = cx.scene.chars[me].target_char;
    let hold = |cx: &mut Cx| {
        if let Some(t) = target {
            cx.affect(t, 5, 0);
        }
    };
    match b.act_proccess {
        0 => {
            if !cx.valid(target) || target.is_some_and(|t| cx.scene.chars[t].cond[cond::DEAD] != 0) {
                let t = b.select_target(cx, 0, FAR);
                cx.scene.chars[me].target_char = t;
                if t.is_none() {
                    change_next_pattern(b, x, v, cx);
                    return;
                }
                b.calc_target_info(cx);
            }
            b.erase_cmnd_target(cx);
            b.body_hit_sw = 0;
            let pos = cx.pos();
            cx.out(Out::Se3d { se: 63, pos });
            super::restart(&mut b.anm, cx);
            b.anm.forward();
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            hold(cx);
            if !ee::le(b.target_dist, 0x43c8_0000) {
                b.move_dirc = b.target_dirc;
                b.move_spd = 0x4348_0000;
                return;
            }
            x.press_start = cx.pos();
            b.move_spd = 0;
            x.press_r = 0x4248_0000;
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 => {
            hold(cx);
            if c >= 16 {
                let pos = cx.pos();
                cx.out(Out::Se3d { se: 63, pos });
                b.act_count = 0;
                b.act_proccess += 1;
                return;
            }
            let tp = w2p(cx, b.target_pos);
            let pp = cx.scene.chars[me].pos_p;
            let o = step_along(pitch(cx, tp, pp), get_dirc(tp, pp), x.press_r);
            let mut r = ee::add(x.press_r, 0xc08e_38e4);
            if ee::lt(r, 0) {
                r = 0;
            }
            x.press_r = r;
            let pp = geom::vadd(pp, o);
            cx.scene.chars[me].pos_p = pp;
            cx.scene.chars[me].pos = p2w(cx, pp);
            b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 512);
        }
        3 => {
            hold(cx);
            let mut at = b.target_pos;
            if let Some(t) = target {
                let h = cx.scene.chars[t].base().height;
                at[2] = ee::add(at[2], ee::mul(0x3f00_0000, h));
            }
            x.act_pos = at;
            let tp = w2p(cx, at);
            let pp = cx.scene.chars[me].pos_p;
            let o = step_along(pitch(cx, pp, tp), get_dirc(pp, tp), 0x42c8_0000);
            let pp = geom::vadd(pp, o);
            cx.scene.chars[me].pos_p = pp;
            cx.scene.chars[me].pos = p2w(cx, pp);
            if ee::le(cx.scene.chars[me].pos[2], at[2]) {
                cx.scene.chars[me].pos = at;
                cx.scene.chars[me].pos_p = tp;
                b.skill_damage(cx, target, None, 60);
                let pos = cx.pos();
                cx.out(Out::Se3d { se: 170, pos });
                b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 512);
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        4 => {
            if c < 15 {
                return;
            }
            let a = w2p(cx, x.press_start);
            let e = w2p(cx, x.act_pos);
            let d = geom::length_on(cx.actx.volume, geom::vsub(e, a));
            x.press_r = 0x4248_0000;
            x.press_step = ee::mul(0x4000_0000, ee::div(ee::sub(d, 0x443b_8000), 0x4361_0000));
            b.act_count = 0;
            b.act_proccess += 1;
        }
        5 => {
            hold(cx);
            if c & 1 != 0 {
                cx.out(Out::AfterImageWith(x.eai));
            }
            let s = w2p(cx, x.press_start);
            let pp = cx.scene.chars[me].pos_p;
            let o = step_along(pitch(cx, pp, s), get_dirc(pp, s), x.press_r);
            let pp = geom::vadd(pp, o);
            cx.scene.chars[me].pos_p = pp;
            cx.scene.chars[me].pos = p2w(cx, pp);
            x.press_r = ee::add(x.press_r, x.press_step);
            if dp(fabs(x.press_r)) < 10.0 {
                cx.scene.chars[me].pos = x.press_start;
                cx.scene.chars[me].pos_p = w2p(cx, x.press_start);
                b.entry_cmnd_target(cx);
                b.body_hit_sw = 1;
                change_next_pattern(b, x, v, cx);
            }
            b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 512);
        }
        _ => {}
    }
}

/// `OnThinkDead` (0x003c1520): its body off, still, the root drain at its
/// place (`ccBossEffRootDrainCreate`), off the lists; fading over 60
/// frames until the drain is over, then out.
fn on_dead(mb: &mut BossBase, b: &mut BossBase, x: &mut Killer, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            b.body_hit_sw = 0;
            b.move_vector = VF0;
            b.move_spd = 0;
            let pos = cx.pos();
            // A killer eye's drain is kept under a key of its own (one at a
            // time: it is out before it can be born again).
            let key = KILLER_GEN_KEY + x.slave_id as u32;
            let d = super::eff::Drain::new(pos, key);
            x.boss_eff = Some(mb.make_fx(cx, EffKind::RootDrain, super::eff::Fx::Drain(Box::new(d)), pos, VF0));
            crate::fellow::delete_cmnd(cx.scene, cx.me);
            cx.out(Out::DeleteCmnd);
            b.act_proccess += 1;
        }
        1 => {
            let mut t = ee::sub(b.transparency, 0x3c88_8889);
            if ee::le(t, 0) {
                t = 0;
            }
            b.transparency = t;
            b.set_transparency = t;
            if !mb.effects.enabled(x.boss_eff) {
                on_exit(b, cx);
            }
        }
        _ => {}
    }
}

/// `ccBoss::OnExit` (0x0035fc30): exited (still drawn if it was).
pub(crate) fn on_exit(b: &mut BossBase, _cx: &mut Cx) {
    b.exit = 1;
}

impl crate::boss::BossClass for Eye {}

impl crate::boss::BossClass for Killer {
    fn affect(&mut self, b: &mut BossBase, cx: &mut Cx) {
        affect(b, cx);
    }
}

/// `ccBoss08KillerEye::Affect` (0x003bf650): nothing while held off or
/// locked (but a drain's 21); a blow (1, 3), unless the party is gone: its
/// number, the mark, its HP down (a fifth with cheat HP, never under
/// half); at 0 its conditions cleared and dead (SE 175 note 48), else SE
/// 175 note 52; then its row's guard (`Exdefense` 1 a physical blow, 2 a
/// spell) shows the shield, its height flattened. Any other type the
/// base's.
pub(crate) fn affect(b: &mut BossBase, cx: &mut Cx) {
    let me = cx.me;
    let (t, p0, p1, person) = {
        let a = &cx.scene.chars[me].affect;
        (a.ty, a.param[0], a.param[1], a.person)
    };
    if t != 21 && (b.act_forbid == 2 || b.lock_player != 0) {
        return;
    }
    if t != 1 && t != 3 {
        b.base_affect(cx);
        return;
    }
    if cx.annihilated() || cx.game_over {
        return;
    }
    cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
    if p0 >= 0 {
        cx.out(Out::HitMark { by: person });
        let hp = cx.scene.chars[me].hp;
        let mhp = cx.scene.chars[me].max_hp;
        let mut h = crate::boss::hit_hp(hp, p0, (b.cheat_hp != 0).then_some(5), mhp);
        let pos = cx.pos();
        if h <= 0 {
            h = 0;
            crate::affect::clear_conditions(&mut cx.scene.chars[me]);
            b.change_action(cx, killer_act::DEAD, 2, true);
            cx.out(Out::Se3dNote { se: 175, pos, note: 48 });
        } else {
            cx.out(Out::Se3dNote { se: 175, pos, note: 52 });
        }
        cx.scene.chars[me].hp = h;
    }
    let ex = cx.scene.chars[me].foe_state().map_or(0, |f| f.row.exdefense);
    let kind = crate::skill::check_type_of(cx.t, i32::from(p1));
    match ex {
        1 if kind == 0 => cx.out(Out::Corbenik(Pic::KillerShield { eye: me, magic: 0 })),
        2 if kind == 1 => cx.out(Out::Corbenik(Pic::KillerShield { eye: me, magic: 1 })),
        _ => {}
    }
}

/// The body's dummy in the world: `ApplyMatrix(anm, dmyEye->pos)`.
pub fn dummy_at(world: &M4, dummy: V4) -> V4 {
    geom::apply_matrix(world, dummy)
}

impl Third {
    /// The view an eye reads (`m` the body).
    pub(crate) fn master_view(&self, b: &BossBase, cx: &Cx) -> MasterView {
        let me = cx.me;
        MasterView {
            pos_p: cx.scene.chars[me].pos_p,
            target: cx.scene.chars[me].target_char,
            target_dirc: b.target_dirc,
            target_dist: b.target_dist,
            dummy: dummy_at(&self.body_world, self.dummy),
        }
    }

    pub(crate) fn killer_view(&self, b: &BossBase, cx: &Cx) -> KillerView {
        KillerView { master: cx.me, paused: self.pause, dirc: b.dirc, dummy: dummy_at(&self.body_world, self.dummy) }
    }
}
