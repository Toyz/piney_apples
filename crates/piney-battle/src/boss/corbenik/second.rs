//! Corbenik's second form (`ccBoss08_b`, QUA gcmn 0x003b1c50-0x003b9030):
//! the body (`bossTbl` row 8, `x81`) the first form leaves in the same
//! task, with five roots ([`Root`], bosses on characters of their own) it
//! drives up round its target, sixteen elements it shoots, and a barrier
//! it raises at a fifth of its HP. Its HP never kills it: it dies when the
//! barrier's own count runs out, or 3630 frames after the barrier first
//! falls under 4000 (the message's wait).

use piney_data::field::ee;

use super::eff::{self, Fx};
use super::{Pic, make_fx, manager_pass, select, target_ok, word, wrap_hi, wrap_lo};
use crate::boss::kyvia::{Part, as_me, plain_boss, set_base_param};
use crate::boss::{AfterImageParams, Anm, Boss, BossBase, Class, Cx, EffKind, Out, Tbl, VF0, blur};
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, V4};
use crate::item;
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// The stage fader's colour.
const DARK: u32 = 0x6400_0000;
/// `bossTbl` row 8: the body's and the roots'; row 48 the barrier's.
pub const ROW: usize = 8;
pub const BARRIER_ROW: usize = 48;
pub const ROOTS: usize = 5;
pub const ELEMENTS: usize = 16;
/// The clips `OnThinkRootDrain` copies (`@2104`) and the root's.
const ANM_ROOT_ATC: &str = "ANM_ex8natc0";
const ANM_ROOT_NUT: &str = "ANM_ex8nnut0";
const ANM_ROOT_END: &str = "ANM_ex8nend0";
const ANM_ROOT_L: &str = "ANM_ex8natc4";
const ANM_WAVE_ACT: &str = "ANM_ex8act3";
const ANM_WAVE: &str = "ANM_xx11wave";

/// The second form's acts (`Think` 0x003b3020; any other is
/// `ChangeAction(0, 1, 4)`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DAMAGE0: i16 = 1;
    pub const DAMAGE1: i16 = 2;
    pub const WAVE: i16 = 3;
    pub const BODY_PRESS: i16 = 4;
    pub const STAR_DUST: i16 = 5;
    pub const MAGIC: i16 = 6;
    /// The base's dash to the centre (word 6): back to `dashPos`.
    pub const RETURN: i16 = 7;
    /// The drain's act: no `Think` of its own.
    pub const DRAINED: i16 = 11;
    pub const BASE_EPITAPH: i16 = 12;
    pub const DIE: i16 = 14;
    pub const ESCAPE: i16 = 16;
    pub const BASE_RETURN: i16 = 17;
    pub const BASE_CHASE: i16 = 18;
    /// The base's wander (word 7).
    pub const RANDOM_DRIVE: i16 = 20;
    pub const SYBLY: i16 = 21;
    pub const ROOT_DRAIN: i16 = 22;
    pub const SETUP_BARRIER: i16 = 23;
    pub const BARRIER_MSG: i16 = 24;
}

/// `BODYELEMENT_T` (0x180 bytes): an element thrown at the target
/// (`m_elementTbl`) or a body press's image (`m_aiElements`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Element {
    pub pos: V4,
    pub sp: V4,
    pub ep: V4,
    pub vec: V4,
    /// +0x40: 0 free, 1 out, 2 back.
    pub state: i32,
    pub alpha: F,
    /// +0x50: its `ccAnm` (the body's clip at its first frame).
    pub anm: Anm,
    /// +0x160 the count; +0x164 a press image's angle, +0x168 its turn.
    pub count: i32,
    pub angle: F,
    pub turn: F,
    /// +0x170 a press image's heading.
    pub dirc: V4,
}

/// `RDINFO_T` (+0xfa5c0): the random drive's point, way, speed, after-image
/// count and step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RdInfo {
    pub point: V4,
    pub dir: F,
    pub spd: F,
    pub after: i32,
    pub step: i32,
}

/// `ccBoss08Root` (0x29360): a root, `bossTbl` row 8 on its own character.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Root {
    /// `m_quakeTime`: frames of the master's quake left.
    pub quake_time: i32,
}

/// `ccBoss08_b`'s own members (offsets as Quarantine's code has them).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Second {
    pub root_me: [usize; ROOTS],
    /// Scratch for [`main`]'s span: the roots out of their characters.
    pub roots: Vec<Part<Root>>,
    /// +0x29350 `anmw` (the wave), +0x29354 the barrier's, +0xf75a0 the
    /// root drain's (`m_anmRootL`).
    pub anm_w: Anm,
    pub anm_barrier: Anm,
    pub anm_root_l: Anm,
    /// +0x2935c `m_stageEff`, +0x29360 `m_effStar`, +0x29364 `m_bossEff`
    /// (the transform's, then the root drain's).
    pub stage_eff: i32,
    pub eff_star: Option<i32>,
    pub boss_eff: Option<i32>,
    /// +0x29410 `m_bBarriar`, +0x29414 its clip's end.
    pub barrier: bool,
    pub barrier_anm_status: bool,
    /// +0x29418.
    pub pat_mode: i32,
    /// +0x29420 where the root drain started.
    pub old_pos: V4,
    /// +0x29430 the root drain's picture's.
    pub root_transparency: F,
    /// +0x29434 `m_reserveSetupBarrier`, +0x29438 `m_flashBarrierFlag`
    /// (the barrier's hit clip plays).
    pub reserve_setup_barrier: bool,
    pub flash_barrier: bool,
    /// +0x2943c `m_lifeCounter`: the barrier's own HP.
    pub life_counter: i32,
    /// +0x29440, +0x29450: the camera the root drain keeps.
    pub cam_pos: V4,
    pub cam_view: V4,
    /// +0x294b0 the root drain's clip's end.
    pub root_anm_status: bool,
    /// +0x294b4 the message's wait, +0x294b8 the message shown, +0x294bc
    /// the wait run out (Quarantine's: Infection's class has none).
    pub msg_count: i32,
    pub msg_shown: bool,
    pub time_up: bool,
    /// +0xf75a4 `m_rootIndex` (never read).
    pub root_index: i32,
    pub elements: Vec<Element>,
    pub ai_elements: Vec<Element>,
    pub rd: RdInfo,
    /// `ccBoss::actPos` (+0x1b0): the press's and the root drain's places
    /// (the root drain's circling radius and angle are `m_actDirc`'s x and
    /// z).
    pub act_pos: V4,
    /// The key of the next particle generator the form keeps.
    pub gen_serial: u32,
}

/// `m_eaiRD`: the random drive's after-image.
pub const EAI_RD: AfterImageParams = AfterImageParams {
    color: 0x0050_5000,
    color_alpha: 0x42c8_0000,
    color_spd: 0,
    max_frame: 0x41a0_0000,
    transparency: ONE,
    trans_spd: 0xbd4c_cccd,
    ss: VF0,
    es: VF0,
    scale_time: 0,
};

/// A root out of its character (`Class::CorbenikRoot`), and whether one
/// was there.
fn take_root(cx: &mut Cx, me: usize) -> (Part<Root>, bool) {
    let taken = cx.scene.chars.get_mut(me).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take());
    let Some(mut rb) = taken else { return (Part { me, ..Part::default() }, false) };
    match std::mem::replace(&mut rb.class, Class::Plain) {
        Class::CorbenikRoot(x) => (Part { b: rb, x, me }, true),
        other => {
            rb.class = other;
            if let Some(f) = cx.scene.chars[me].foe_state_mut() {
                f.boss = Some(rb);
            }
            (Part { me, ..Part::default() }, false)
        }
    }
}

fn put_root(cx: &mut Cx, mut part: Part<Root>, found: bool) {
    if !found {
        return;
    }
    part.b.class = Class::CorbenikRoot(part.x);
    if let Some(f) = cx.scene.chars[part.me].foe_state_mut() {
        f.boss = Some(part.b);
    }
}

fn take_roots(cx: &mut Cx, x: &mut Second) -> [bool; ROOTS] {
    x.roots.clear();
    std::array::from_fn(|k| {
        let (part, found) = take_root(cx, x.root_me[k]);
        x.roots.push(part);
        found
    })
}

fn put_roots(cx: &mut Cx, x: &mut Second, found: [bool; ROOTS]) {
    for (k, part) in std::mem::take(&mut x.roots).into_iter().enumerate() {
        put_root(cx, part, found[k]);
    }
}

/// `cx.me` set to root `k`'s character for `f`.
fn with_root<R>(x: &mut Second, k: usize, cx: &mut Cx, f: impl FnOnce(&mut BossBase, &mut Root, &mut Cx) -> R) -> R {
    let p = &mut x.roots[k];
    as_me(cx, p.me, |cx| f(&mut p.b.base, &mut p.x, cx))
}

fn w2p(cx: &Cx, v: V4) -> V4 {
    crate::boss::w2p(cx, v)
}

fn p2w(cx: &Cx, v: V4) -> V4 {
    crate::boss::p2w(cx, v)
}

/// `P2W(W2P(at) + Rz(-pi/2) Rz(d) v)`: a place `v` out from `at` along
/// `d` (the matrix turned by `d`, then by -pi/2).
fn out_from(cx: &Cx, at: V4, d: F, v: V4) -> V4 {
    let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI), d);
    let o = geom::apply_matrix(&m, v);
    p2w(cx, geom::vadd(w2p(cx, at), o))
}

/// The same with the matrix `Rz(d)` first and `Rz(-pi/2)` after (the
/// sybly's and star dust's cameras: `RotMatrixX/Y/Z(d)` then `Z(-pi/2)`).
fn out_from_after(cx: &Cx, at: V4, d: F, v: V4) -> V4 {
    let m = geom::rot_matrix_y(&geom::rot_matrix_x(&geom::unit_matrix(), 0), 0);
    let m = geom::rot_matrix_z(&geom::rot_matrix_z(&m, d), NEG_HALF_PI);
    let o = geom::apply_matrix(&m, v);
    p2w(cx, geom::vadd(o, w2p(cx, at)))
}

fn cursor(cx: &mut Cx, on: bool) {
    cx.out(Out::CursorOff(on));
}

fn free_cam(cx: &mut Cx, pos: V4, view: V4) {
    cx.out(Out::CamMode { mode: 5 });
    cx.out(Out::FreeCam { pos, view });
}

fn cam_back(cx: &mut Cx) {
    cx.out(Out::CamMode { mode: 6 });
}

/// `bossCam->QuakeCam(q)` (the form always has its camera).
fn quake(cx: &mut Cx, q: [F; 3]) {
    if cx.boss_cam {
        cx.quake_vec(q);
    }
}

// --- the constructor ------------------------------------------------------------------------------

/// `ccBoss08_b::ccBoss08_b` (0x003b1c50): five roots made first (row 8,
/// each `ccRand() & 1` picking its clips, exited, hidden), then the body:
/// row 8, 500 behind Kite and 10 up, his heading, `OnCheatHP`; its wave,
/// barrier and root drain clips; on the lists; the boss camera (200,
/// 1000) and the blur on; no barrier; `SetPatternMode(0)`. `effects` are
/// the manager's as the first form left them.
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4, effects: crate::boss::Effects) -> Boss {
    let me = cx.me;
    let root_me = std::array::from_fn(|k| {
        let rm = crate::boss::kyvia::new_char(cx, ROW);
        let part = new_root(cx, rm, k);
        put_root(cx, part, true);
        rm
    });
    let mut b = plain_boss();
    b.effects = effects;
    set_base_param(cx, me, ROW);
    b.anm_tbl = cx.data.corbenik.b_anims.clone();
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    b.cheat_hp = 1;
    let mut pos = kite_pos;
    pos[1] = ee::sub(pos[1], 0x43fa_0000);
    pos[2] = ee::add(pos[2], 0x4120_0000);
    cx.scene.chars[me].pos = pos;
    b.dirc = kite_dirc;
    let mut x = Box::new(Second {
        root_me,
        anm_w: Anm::new(),
        anm_barrier: Anm::new(),
        anm_root_l: Anm::new(),
        // The constructor leaves `m_stageEff` as the object has it.
        stage_eff: 0,
        elements: vec![Element::default(); ELEMENTS],
        ai_elements: vec![Element::default(); ELEMENTS],
        ..Second::default()
    });
    x.anm_w.set(ANM_WAVE, cx.clips);
    x.anm_root_l.set(ANM_ROOT_L, cx.clips);
    if let Some(Some(c)) = cx.data.corbenik.dmg_barrier.first() {
        x.anm_barrier.set(c, cx.clips);
    }
    b.change_action(cx, act::NEUTRAL, 0, true);
    crate::fellow::entry_cmnd(cx.scene, me);
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = w2p(cx, b.center_pos);
    b.use_stage_eff = true;
    cx.out(Out::Blur(blur::Set::Enable));
    init_elements(&b, &mut x, cx);
    set_pattern_mode(&mut b, &mut x, cx, 0);
    Boss { base: b, class: Class::CorbenikB(x) }
}

/// `ccBoss08Root::ccBoss08Root` (0x003b8c10): row 8, its clips by
/// `ccRand() & 1`, at 512, its body off, drawn, exited.
fn new_root(cx: &mut Cx, me: usize, k: usize) -> Part<Root> {
    let mut b = plain_boss();
    set_base_param(cx, me, ROW);
    b.anm_tbl = if cx.cc.rand() & 1 != 0 {
        cx.data.corbenik.root_anims_a.clone()
    } else {
        cx.data.corbenik.root_anims_b.clone()
    };
    b.anm.speed = 512;
    b.body_hit_sw = 0;
    b.draw_sw = 1;
    b.exit = 1;
    let _ = k;
    Part { b: Box::new(Boss { base: b, class: Class::Plain }), x: Box::new(Root::default()), me }
}

/// `_InitElement` (0x003b74b0): each element's clip the body's own, a
/// frame on, free.
fn init_elements(b: &BossBase, x: &mut Second, cx: &Cx) {
    for e in &mut x.elements {
        e.anm = Anm::new();
        if let Some(c) = &b.anm.clip {
            e.anm.set(c, cx.clips);
            e.anm.forward();
        }
        e.state = 0;
    }
}

/// `SetPatternMode(m)` (0x003b32f0): 0 the normal table, 1 the super, any
/// other none; run from its start.
fn set_pattern_mode(b: &mut BossBase, x: &mut Second, cx: &mut Cx, m: i32) {
    let t = match m {
        0 => Tbl::Normal,
        1 => Tbl::Super,
        _ => Tbl::None,
    };
    b.pat_tbl = t;
    b.pat_index = 0;
    x.pat_mode = m;
    let tbl = cx.data.corbenik.words_b(t).to_vec();
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, 0);
}

// --- the frame ------------------------------------------------------------------------------------

/// The effect manager's pass, then `ccBoss08_b::Main` (0x003b2610).
pub(crate) fn main(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let found = take_roots(cx, x);
    manager_pass(b, cx);
    frame(b, x, cx);
    put_roots(cx, x, found);
}

fn frame(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    if b.exit != 0 {
        return;
    }
    let me = cx.me;
    b.calc_real(cx);
    b.calc_target_info(cx);
    b.center_pos_p = w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = crate::boss::dist(cx, pp, b.center_pos_p);
    b.center_dirc = get_dirc(pp, b.center_pos_p);
    if b.stop != 0 {
        b.stop_count += 1;
        if b.stop_count >= 61 {
            b.stop_count = 0;
            b.stop = 0;
        }
        b.move_spd = 0;
    } else {
        b.stop_count = 0;
    }
    // The barrier's message's wait.
    if b.act_num != act::DIE && x.barrier && !x.time_up && x.msg_shown {
        let c = x.msg_count;
        x.msg_count = c - 1;
        if c <= 0 {
            x.time_up = true;
        }
    }
    think(b, x, cx);
    if b.lock_player != 0 {
        for m in cx.party.members.into_iter().flatten() {
            if cx.valid(Some(m)) {
                cx.affect(m, 5, 32767);
            }
        }
        if b.reserve_forbid_menu == 1 && cx.env.menu_forbid == 0 && cx.env.menu_type == -1 {
            cx.out(Out::MenuForbid { on: true, chat_except: b.reserve_forbid_chat_except != 0 });
            b.reserve_forbid_menu = 0;
            b.reserve_forbid_chat_except = 0;
        }
    } else if b.reserve_forbid_menu == -1 && cx.env.menu_type == -1 {
        cx.out(Out::MenuForbid { on: false, chat_except: false });
        b.reserve_forbid_menu = 0;
        b.reserve_forbid_chat_except = 0;
    }
    b.move_(cx);
    if b.draw_sw != 0 {
        pre_draw_anm(b, x, cx);
        b.draw(cx);
    }
    slave(b, x, cx);
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// `PreDrawAnm` (0x003b3600): the clip forward (not in the press past its
/// start); with the barrier, its hit clip while it flashes (ended, the
/// barrier shown again).
fn pre_draw_anm(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    if !(b.act_num == act::BODY_PRESS && b.act_proccess != 0) {
        b.anm_status = i8::from(b.anm.forward());
    }
    if !x.barrier {
        return;
    }
    if x.flash_barrier {
        x.barrier_anm_status = x.anm_barrier.forward();
        if !x.barrier_anm_status {
            cx.out(Out::Corbenik(Pic::BarrierHit { time: x.anm_barrier.posed }));
            return;
        }
        x.flash_barrier = false;
    }
    cx.out(Out::Corbenik(Pic::Barrier));
}

/// `Slave` (0x003b2a00): each root's `Main`; a root's quake time shakes
/// the camera (30, 30, 0) a frame.
fn slave(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let _ = b;
    for k in 0..ROOTS {
        with_root(x, k, cx, root_main);
        if x.roots[k].x.quake_time > 0 {
            x.roots[k].x.quake_time -= 1;
            quake(cx, [0x41f0_0000, 0x41f0_0000, 0]);
        }
    }
}

// --- the patterns ---------------------------------------------------------------------------------

/// `ccBoss08_b::ChangeNextPattern` (0x003b3200): a barrier asked for is
/// raised (pattern 19); with the barrier up and the wait out, the death;
/// else the base's.
fn change_next_pattern(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    if x.reserve_setup_barrier && !x.barrier {
        x.reserve_setup_barrier = false;
        exec_pattern(b, x, cx, 19, 0);
        return;
    }
    if x.barrier && x.time_up {
        x.time_up = false;
        b.change_action(cx, act::DIE, 2, true);
        return;
    }
    let tbl = cx.data.corbenik.words_b(b.pat_tbl).to_vec();
    if word(&tbl, b.pat_index) == -1 {
        b.pat_index = 0;
    }
    b.move_spd = 0;
    b.move_vector = VF0;
    if b.epitaph != 0 {
        b.change_action(cx, act::BASE_EPITAPH, 3, true);
    } else {
        b.change_action(cx, act::NEUTRAL, 1, true);
    }
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, b.pat_index);
}

fn exec_pattern(b: &mut BossBase, x: &mut Second, cx: &mut Cx, pat: i32, p0: i32) {
    let tbl = [pat, p0, 0, 0, -1];
    exec_pattern_index(b, x, cx, &tbl, 0);
}

/// `ccBoss08_b::ExecPatternIndex` (0x003b33c0): 1 the wave (3), 14 the
/// magic (6), 15 the star dust (5), 16 the press (4), 17 the sybly (21),
/// 18 the root drain (22), 19 the barrier (23); the rest the base's.
fn exec_pattern_index(b: &mut BossBase, x: &mut Second, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
    let mut s = i + 1;
    let mut pat = word(tbl, i);
    b.move_spd = 0;
    b.move_vector = VF0;
    if pat == -1 {
        pat = word(tbl, 0);
        s = 1;
    }
    b.pat_num = pat;
    let n = match pat {
        1 => act::WAVE,
        14 => act::MAGIC,
        15 => act::STAR_DUST,
        16 => act::BODY_PRESS,
        17 => act::SYBLY,
        18 => act::ROOT_DRAIN,
        19 => act::SETUP_BARRIER,
        _ => return base_exec_pattern_index(b, x, cx, tbl, i),
    };
    b.change_action(cx, n, 3, true);
    s
}

/// `ccBoss::ExecPatternIndex` through this class's `ChangeAction` and
/// `ExecPattern` ([`super::base_exec_pattern_index`]).
fn base_exec_pattern_index(b: &mut BossBase, x: &mut Second, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
    super::base_exec_pattern_index(b, cx, &mut BaseHost { x }, tbl, i)
}

struct BaseHost<'a> {
    x: &'a mut Second,
}

impl super::Host for BaseHost<'_> {
    fn change_action(&mut self, b: &mut BossBase, cx: &mut Cx, n: i16, forbid: i16, af: bool) {
        b.change_action(cx, n, forbid, af);
    }

    fn exec_pattern(&mut self, b: &mut BossBase, cx: &mut Cx, pat: i32, p0: i32) {
        exec_pattern(b, self.x, cx, pat, p0);
    }
}

/// `ccBoss08_b::Think` (0x003b3020).
fn think(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => on_neutral(b, x, cx),
        act::DAMAGE0 | act::DAMAGE1 => b.change_action(cx, act::NEUTRAL, 1, false),
        act::RETURN => on_return(b, x, cx),
        act::BARRIER_MSG => on_barrier_msg(b, x, cx),
        act::WAVE => on_wave(b, x, cx),
        act::BODY_PRESS => on_body_press(b, x, cx),
        act::STAR_DUST => on_star_dust(b, x, cx),
        act::DIE => on_die(b, x, cx),
        act::RANDOM_DRIVE => on_random_drive(b, x, cx),
        act::MAGIC => on_magic(b, x, cx),
        act::SYBLY => on_sybly(b, x, cx),
        act::ESCAPE => on_escape(b, x, cx),
        act::ROOT_DRAIN => on_root_drain(b, x, cx),
        act::SETUP_BARRIER => on_setup_barrier(b, x, cx),
        _ => b.change_action(cx, act::NEUTRAL, 1, true),
    }
}

/// The act's target (`SelectTarget(ty)`), or the 2-1 wait when it is gone
/// or down.
fn pick(b: &mut BossBase, x: &mut Second, cx: &mut Cx, ty: i32) -> bool {
    let t = select(b, cx, ty);
    if !target_ok(cx, t) {
        exec_pattern(b, x, cx, 2, -1);
        return false;
    }
    b.calc_target_info(cx);
    true
}

/// `OnThinkNeutral` (0x003b38a0): still, turning to its target; the wait,
/// then the next pattern at `SelectTarget(0)`.
fn on_neutral(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    b.move_spd = 0;
    b.move_vector = VF0;
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 256);
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
        if cx.scene.chars[me].skill_status != 0 {
            return;
        }
        b.stop_time = 0;
    }
    let t = select(b, cx, 0);
    if !cx.valid(t) {
        return;
    }
    b.calc_target_info(cx);
    change_next_pattern(b, x, cx);
}

/// `ccBoss08_b::IsValidArea(p)` (0x003b3820): within 2000 of the centre
/// each way.
fn valid_area(b: &BossBase, p: V4) -> bool {
    let (cx_, cy) = (b.center_pos_p[0], b.center_pos_p[1]);
    let r = 0x44fa_0000;
    ee::le(ee::sub(cx_, r), p[0])
        && !ee::lt(ee::add(r, cx_), p[0])
        && ee::le(ee::sub(cy, r), p[1])
        && !ee::lt(ee::add(r, cy), p[1])
}

/// `OnThinkEscape` (0x003b3a40): away from the target at 30 for 300
/// frames, turned a quarter off it every 32, toward the centre outside its
/// box.
fn on_escape(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    b.move_spd = 0x41f0_0000;
    if c & 31 == 0 {
        let off = (f64::from(f32::from_bits(ee::sub(b.target_dirc, b.center_dirc))).abs() as f32).to_bits();
        let half = 0x3fc9_0fdb;
        let mut r = if ee::lt(off, half) { ee::sub(b.target_dirc, half) } else { ee::add(half, b.target_dirc) };
        if ee::le(r, PI) && ee::lt(r, 0xc049_0fdb) {
            r = ee::add(r, 0x40c9_0fdb);
        }
        b.m_act_dirc[2] = r;
    }
    b.move_dirc = geom::set_dirc(b.move_dirc, b.m_act_dirc[2], 256);
    if !valid_area(b, cx.scene.chars[me].pos_p) {
        b.m_act_dirc[2] = b.center_dirc;
    }
    match b.act_proccess {
        0 => {
            if !pick(b, x, cx, 0) {
                return;
            }
            if !valid_area(b, cx.scene.chars[me].pos_p) {
                b.move_dirc = b.center_dirc;
            }
            b.act_proccess += 1;
        }
        1 => {
            if c == 300 {
                change_next_pattern(b, x, cx);
                return;
            }
            if b.stop != 0 || cx.scene.chars[me].cond[cond::HOLD] != 0 {
                b.move_spd = 0;
            }
        }
        _ => {}
    }
    b.dirc[2] = b.move_dirc;
}

/// `OnThinkSybly` (0x003b3d00): at the target, off the targets, the party
/// locked, the cinema of skill 157, the dark stage, the camera 1500 out and
/// 300 up from the target along the centre's way; 15 frames on skill 157
/// compelled on every member up; the stage back at 45, all back at 60.
fn on_sybly(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            if !pick(b, x, cx, 0) {
                return;
            }
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.lock_player(cx, true);
            cx.out(Out::CinemaSkill(157));
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 15, 3000]);
            let t = cx.scene.chars[me].target_char.unwrap_or(me);
            let mut vp = cx.scene.chars[t].pos;
            vp[2] = ee::add(vp[2], 0x42c8_0000);
            let eye = out_from_after(cx, vp, b.center_dirc, [0x44bb_8000, 0, 0x4396_0000, ONE]);
            free_cam(cx, eye, vp);
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c != 15 {
                return;
            }
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m))
                    && !cx.dead(m)
                    && let Some(k) = item::item_skill_compel(cx.t, cx.scene, me, m, 157, 0, false, cx.rand)
                {
                    cx.out(Out::Skill(m, k));
                }
            }
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 => {
            b.act_count += 1;
            let c = b.act_count;
            if c == 45 {
                b.end_stage_effect(cx, x.stage_eff, DARK, 15);
                x.stage_eff = -1;
            } else if c == 60 {
                cx.out(Out::SwitchLayer);
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                b.unlock_player();
                cx.out(Out::Cinema(None));
                cam_back(cx);
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkWave` (0x003b4160): at the clip's frame 1, at the target, the
/// wave's clip from its start at 512; its notes, SE 223 at 59 and 224 at
/// 101; at 105 a flash and the WaveShock, at 110 boss skill 28 there; from
/// 105 the quake and the wave drawn; both clips over, the next pattern.
fn on_wave(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let pos = cx.pos();
    match b.act_proccess {
        0 => {
            if b.anm.frame() != 1 {
                return;
            }
            if !pick(b, x, cx, 0) {
                return;
            }
            b.anm.set(ANM_WAVE_ACT, cx.clips);
            x.anm_w.set(ANM_WAVE, cx.clips);
            x.anm_w.speed = 512;
            b.act_proccess += 1;
        }
        1 => {
            let f = b.anm.frame();
            if matches!(f, 7 | 21 | 31 | 37 | 42 | 46) {
                cx.out(Out::Se3dNote { se: 172, pos, note: 63 });
            }
            if f == 59 {
                cx.out(Out::Se3d { se: 223, pos });
            } else if f == 101 {
                cx.out(Out::Se3d { se: 224, pos });
            }
            if (f as u32) < 105 {
                return;
            }
            if f == 105 {
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                let d = b.dirc;
                b.effect_at(cx, EffKind::WaveShock, pos, d);
            } else if f == 110 {
                b.skill_damage(cx, None, Some(pos), 28);
            }
            quake(cx, [0x4248_0000; 3]);
            let ended = x.anm_w.forward();
            cx.out(Out::Corbenik(Pic::Wave { time: x.anm_w.posed, pos }));
            if ended && b.anm_status != 0 {
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkStarDust` (0x003b4500): at the member with the least HP, the camera 2000 out and
/// 200 up from its place at height 200, the dark stage, the party locked,
/// cinema 30, off the targets, the blur eased; 15 frames on the star dust
/// (`ccBossEffStarDustFlash`) at the target; its notes while it runs;
/// over, the stage back and boss skill 30 at the target; 30 frames on all
/// back.
fn on_star_dust(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let pos = cx.pos();
    match b.act_proccess {
        0 => {
            // `SelectTarget`'s type is the 3 left in `$a1` by the step's
            // switch: the member with the least HP.
            if !pick(b, x, cx, 3) {
                return;
            }
            let mut vp = pos;
            vp[2] = 0x4348_0000;
            let eye = out_from_after(cx, vp, b.center_dirc, [0x44fa_0000, 0, 0x4348_0000, ONE]);
            free_cam(cx, eye, vp);
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 15, 3000]);
            b.lock_player(cx, true);
            cx.out(Out::Cinema(Some(30)));
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.act_proccess += 1;
            cx.out(Out::Blur(blur::Set::Ease));
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c != 15 {
                return;
            }
            let tp = b.target_pos;
            let s = super::stardust::StarDust::new(cx, pos, tp);
            x.eff_star = Some(make_fx(b, cx, EffKind::StarDustFlash, Fx::StarDust(Box::new(s)), pos));
            b.act_proccess += 1;
        }
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            match c {
                468 | 452 | 441 | 430 | 416 | 396 | 386 | 369 | 358 | 351 => cx.out(Out::Se { se: 35 }),
                467 | 456 | 447 | 440 | 432 | 420 | 410 | 405 | 399 | 393 | 388 | 382 | 372 | 365 | 359 | 350 | 340 => {
                    cx.out(Out::SeNote { se: 61, note: 72 })
                }
                319 => cx.out(Out::Se { se: 1 }),
                205 => cx.out(Out::SeNote { se: 246, note: 55 }),
                256 | 249 | 240 | 229 | 223 | 210 | 203 | 197 | 190 | 185 | 178 | 168 | 163 | 156 => {
                    cx.out(Out::Se { se: 68 })
                }
                63 | 58 | 52 | 47 | 38 | 33 | 27 => cx.out(Out::SeNote { se: 59, note: 65 }),
                _ => {}
            }
            if b.effects.enabled(x.eff_star) {
                return;
            }
            b.end_stage_effect(cx, x.stage_eff, DARK, 15);
            x.stage_eff = -1;
            let tp = b.target_pos;
            b.skill_damage(cx, None, Some(tp), 30);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        3 => {
            let c = b.act_count;
            b.act_count += 1;
            if c != 30 {
                return;
            }
            cx.out(Out::SwitchLayer);
            cam_back(cx);
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            b.unlock_player();
            cx.out(Out::Cinema(None));
            cx.out(Out::Blur(blur::Set::Enable));
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// The press's way round the target (`GetDirc(W2P(targetPos), centre)`).
fn round_target(b: &BossBase, cx: &Cx) -> F {
    get_dirc(w2p(cx, b.target_pos), b.center_pos_p)
}

/// `OnThinkBodyPress` (0x003b4cd0), seven steps: to a place 500 past the
/// target from the centre (after-images); vanished into sixteen images
/// fanned round the target, the camera 700 behind; the images circling in
/// and fading (the camera drawn back at 120); the elements shot from round
/// the target every 8 frames to 90 (boss skill 29 each one that arrives);
/// the stage back; shown again.
fn on_body_press(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            if c != 0 {
                return;
            }
            if !pick(b, x, cx, 0) {
                return;
            }
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::Cinema(Some(29)));
            let d = round_target(b, cx);
            x.act_pos = out_from(cx, b.target_pos, d, [0x43fa_0000, 0, 0, ONE]);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            let dest = w2p(cx, x.act_pos);
            let pp = cx.scene.chars[me].pos_p;
            b.move_dirc = get_dirc(pp, dest);
            b.move_spd = 0x42c8_0000;
            if ee::le(crate::boss::dist(cx, pp, dest), 0x4302_0000) {
                let d = round_target(b, cx);
                let pos = cx.pos();
                for (k, e) in x.ai_elements.iter_mut().enumerate() {
                    let a = if k < 8 {
                        let f = ee::div(ee::mul(PI, ee::from_int(k as i32)), ee::from_int(8));
                        wrap_hi(ee::add(d, f))
                    } else {
                        let f = ee::div(ee::mul(PI, ee::from_int(k as i32 - 8 + 1)), ee::from_int(8));
                        wrap_lo(ee::sub(d, f))
                    };
                    e.pos = pos;
                    e.turn = a;
                    e.angle = d;
                    e.alpha = ONE;
                    e.state = 0;
                }
                x.ai_elements[0].alpha = ONE;
                x.ai_elements[0].state = 1;
                b.draw_sw = 0;
                b.move_spd = 0;
                b.act_count = 0;
                b.act_proccess += 1;
            }
            if c & 1 != 0 {
                cx.out(Out::AfterImage);
                if c == 1 {
                    cx.out(Out::Se3dNote { se: 57, pos: cx.pos(), note: 55 });
                }
            }
            b.dirc[2] = b.move_dirc;
        }
        2 => {
            if c == 30 {
                let d = round_target(b, cx);
                cx.out(Out::CamMode { mode: 5 });
                let eye = out_from(cx, b.target_pos, d, [0xc42f_0000, 0, 0x428c_0000, ONE]);
                let mut view = b.target_pos;
                view[2] = 0x4348_0000;
                cx.out(Out::FreeCam { pos: eye, view });
                cx.out(Out::SwitchLayer);
                x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 3000, 3000]);
            } else if c == 45 {
                cx.out(Out::Blur(blur::Set::Func(blur::BlurFunc::Boss08B)));
                cx.out(Out::Se { se: 229 });
                cx.out(Out::Se { se: 1 });
                b.act_count = 0;
                b.act_proccess += 1;
            }
            b.dirc[2] = b.target_dirc;
            let d = round_target(b, cx);
            let e = &mut x.ai_elements[0];
            e.dirc = VF0;
            e.dirc[2] = get_dirc(b.center_pos_p, w2p(cx, b.target_pos));
            e.pos = out_from(cx, b.target_pos, d, [0x43fa_0000, 0, 0, ONE]);
            b.anm.forward();
            // The body's clip at the image, at the anm's transparency as the
            // last ccBoss::Draw left it (0x003b5460: none is set here).
            let (pos, dirc, alpha) = (e.pos, e.dirc, b.transparency);
            cx.out(Out::Corbenik(Pic::PressImage { time: b.anm.posed, pos, dirc, alpha }));
        }
        3 => {
            let tp = w2p(cx, b.target_pos);
            if c < 61 {
                for e in &mut x.ai_elements {
                    let a = ee::add(e.angle, ee::div(e.turn, 0x4270_0000));
                    e.angle = wrap_lo(wrap_hi(a));
                    let al = ee::add(e.alpha, 0x3c88_8889);
                    e.alpha = if ee::le(al, ONE) { al } else { ONE };
                    e.dirc = VF0;
                    e.dirc[2] = get_dirc(w2p(cx, e.pos), tp);
                }
                if c == 30 {
                    let vp = b.target_pos;
                    let eye = out_from(cx, vp, b.center_dirc, [0x451c_4000, 0, 0x451c_4000, ONE]);
                    cx.out(Out::FreeCam { pos: eye, view: vp });
                }
            } else if c >= 90 {
                if c == 90 {
                    cx.out(Out::Se { se: 59 });
                }
                for e in &mut x.ai_elements {
                    let al = ee::sub(e.alpha, 0x3d08_8889);
                    e.alpha = if ee::lt(al, 0) { 0 } else { al };
                }
                if c == 120 {
                    let d = round_target(b, cx);
                    let eye = out_from(cx, b.target_pos, d, [0x44fa_0000, 0, 0x447a_0000, ONE]);
                    if cx.cam.reset == 5 {
                        cx.out(Out::CamEye(eye));
                    }
                    cx.out(Out::Blur(blur::Set::NoFunc));
                    b.act_count = 0;
                    b.act_proccess += 1;
                }
            }
            b.anm.forward();
            for e in &mut x.ai_elements {
                e.pos = out_from(cx, b.target_pos, e.angle, [0x447a_0000, 0, 0, ONE]);
                e.dirc[2] = get_dirc(w2p(cx, e.pos), tp);
                let (pos, dirc, alpha) = (e.pos, e.dirc, e.alpha);
                cx.out(Out::Corbenik(Pic::PressImage { time: b.anm.posed, pos, dirc, alpha }));
            }
        }
        4 => {
            let live = draw_elements(b, x, cx);
            if c < 90 {
                if c & 7 == 0 {
                    entry_element(b, x, cx);
                    cx.out(Out::Se { se: 253 });
                }
            } else if !live {
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        5 => {
            b.end_stage_effect(cx, x.stage_eff, DARK, 15);
            x.stage_eff = -1;
            b.act_count = 0;
            b.act_proccess += 1;
        }
        6 => {
            if c != 15 {
                return;
            }
            b.draw_sw = 1;
            b.transparency = ONE;
            b.set_transparency = ONE;
            cx.out(Out::SwitchLayer);
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::Cinema(None));
            cam_back(cx);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `_EntryElement` (0x003b6f50): the first free element thrown across the
/// target (a `ccRandF(pi)` way, from 1000 out to 1000 beyond, 133 a frame
/// from 100 up); the bearing is drawn even with none free.
fn entry_element(b: &BossBase, x: &mut Second, cx: &mut Cx) {
    let r = rand_f(cx.cc, PI);
    let tp = w2p(cx, b.target_pos);
    let Some(e) = x.elements.iter_mut().find(|e| e.state == 0) else { return };
    e.state = 1;
    let m = geom::rot_matrix_z(&geom::unit_matrix(), r);
    let v = geom::apply_matrix(&m, [0x447a_0000, 0, 0, ONE]);
    let sp = geom::vadd(tp, v);
    let ep = geom::vsub(tp, v);
    let m = geom::rot_matrix_z(&geom::unit_matrix(), get_dirc(sp, ep));
    let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
    let vec = geom::apply_matrix(&m, [0x4305_5555, 0, 0, ONE]);
    e.sp = p2w(cx, sp);
    e.ep = p2w(cx, ep);
    e.pos = e.sp;
    e.vec = vec;
    e.pos[2] = ee::add(e.pos[2], 0x42c8_0000);
    e.alpha = 0;
    e.count = 0;
}

/// `_DrawElement` (0x003b7140): each element out drawn and moved; one
/// arriving (its way from the start 1000 or more) hits the target with
/// boss skill 29 and rings (195, a `ccRandF(pi)` turn each way); going on
/// it fades with its way left. Whether any is out.
fn draw_elements(b: &mut BossBase, x: &mut Second, cx: &mut Cx) -> bool {
    let mut any = false;
    let tp = w2p(cx, b.target_pos);
    for k in 0..ELEMENTS {
        if x.elements[k].state == 0 {
            continue;
        }
        any = true;
        let e = &mut x.elements[k];
        let d = [NEG_HALF_PI, 0, get_dirc(w2p(cx, e.sp), tp), 0];
        let (pos, alpha) = (e.pos, e.alpha);
        cx.out(Out::Corbenik(Pic::Element { pos, dirc: d, alpha }));
        let cur = w2p(cx, e.pos);
        match e.state {
            1 => {
                let mut f = ee::div(crate::boss::dist(cx, w2p(cx, e.sp), cur), 0x447a_0000);
                if !ee::lt(f, ONE) {
                    f = ONE;
                    e.state += 1;
                    let t = cx.scene.chars[cx.me].target_char;
                    b.skill_damage(cx, t, None, 29);
                    let rx = rand_f(cx.cc, PI);
                    let rz = rand_f(cx.cc, PI);
                    let tpos = b.target_pos;
                    let param = [0x3f00_0000, ONE, 0, 0x4220_0000];
                    b.effect_at(cx, EffKind::AutoSamonRingOf { n: 195, param }, tpos, [rx, 0, rz, 0]);
                    cx.out(Out::SeNote { se: 171, note: 60 });
                }
                x.elements[k].alpha = f;
            }
            2 => {
                let mut f = ee::sub(ONE, ee::div(crate::boss::dist(cx, tp, cur), 0x447a_0000));
                if ee::le(f, 0) {
                    f = 0;
                    e.state = 0;
                }
                e.alpha = f;
            }
            _ => {}
        }
        let e = &mut x.elements[k];
        let n = e.count;
        e.count += 1;
        if n & 3 == 0 {
            let pos = e.pos;
            cx.out(Out::Corbenik(Pic::ElementSmoke { pos, vec: e.vec }));
        }
        let e = &mut x.elements[k];
        e.pos = p2w(cx, geom::vadd(cur, e.vec));
    }
    any
}

/// `OnThinkMagic` (0x003b5f60): at the target, off the targets, locked,
/// the cinema of skill 244, the dark stage, the camera 1500 out and 500 up
/// from the target along its way; 15 frames on skill 244 at it; the stage
/// back 150 after; 15 more, all back.
fn on_magic(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            if !pick(b, x, cx, 0) {
                return;
            }
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.lock_player(cx, true);
            cx.out(Out::CinemaSkill(244));
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 15, 3000]);
            let vp = b.target_pos;
            let eye = out_from(cx, vp, b.target_dirc, [0x44bb_8000, 0, 0x43fa_0000, ONE]);
            free_cam(cx, eye, vp);
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c != 15 {
                return;
            }
            if let Some(tp) = cx.scene.chars[me].target_char
                && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, tp, 244, 0, false, cx.rand)
            {
                cx.out(Out::Skill(tp, k));
            }
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            if c != 150 {
                return;
            }
            b.end_stage_effect(cx, x.stage_eff, DARK, 15);
            x.stage_eff = -1;
            b.act_count = 0;
            b.act_proccess += 1;
        }
        3 => {
            let c = b.act_count;
            b.act_count += 1;
            if c != 15 {
                return;
            }
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::SwitchLayer);
            cx.out(Out::Cinema(None));
            cam_back(cx);
            change_next_pattern(b, x, cx);
            b.act_proccess += 1;
        }
        _ => {}
    }
}

/// `OnThinkSetupBarrier` (0x003b6340): locked, off the targets, the camera
/// 2000 out and 70 up along its heading, looking 400 over it, the dark
/// stage; 30 frames on the transform (`ccBossEffTransform`, 450 up) and the
/// barrier's stats (`bossTbl` row 48, its `Exdefense` written 256); the
/// transform over, the barrier up (SE 229), the stage back; 30 frames on
/// all back.
fn on_setup_barrier(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            if c == 0 {
                b.lock_player(cx, true);
                b.erase_cmnd_target(cx);
                cursor(cx, true);
                let pos = cx.pos();
                let eye = out_from(cx, pos, b.dirc[2], [0x44fa_0000, 0, 0x428c_0000, ONE]);
                let mut view = pos;
                view[2] = ee::add(view[2], 0x43c8_0000);
                free_cam(cx, eye, view);
                cx.out(Out::SwitchLayer);
                x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 15, 30000]);
            } else if c >= 30 {
                cx.out(Out::Cinema(Some(-1)));
                let mut p = cx.pos();
                p[2] = ee::add(p[2], 0x43e1_0000);
                let t = eff::Transform::new(cx, p);
                x.boss_eff = Some(make_fx(b, cx, EffKind::Transform, Fx::Transform(Box::new(t)), p));
                barrier_stats(cx, me);
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        1 => {
            if x.boss_eff.is_none() || b.effects.enabled(x.boss_eff) {
                return;
            }
            x.boss_eff = None;
            x.barrier = true;
            cx.out(Out::Se { se: 229 });
            b.end_stage_effect(cx, x.stage_eff, DARK, 30);
            x.stage_eff = -1;
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 => {
            if c < 30 {
                return;
            }
            cx.out(Out::SwitchLayer);
            cam_back(cx);
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::Cinema(None));
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// The barrier's stats: `base` row 48 (its `Exdefense` 256, written into
/// the table) and its elements copied into the boss's `param`.
fn barrier_stats(cx: &mut Cx, me: usize) {
    let Some(mut r) = cx.t.bosses.get(BARRIER_ROW).cloned() else { return };
    r.exdefense = 256;
    if let Some(p) = cx.scene.param_mut(me) {
        p.real = r.elm;
    }
    if let Some(f) = cx.scene.chars[me].foe_state_mut() {
        f.row = r;
    }
}

/// The base's dash to the centre on the second form (0x003b6750): up to 50
/// (by 50), off the targets, SE 229; an after-image every fourth frame
/// (SE 57 at the third); toward `dashPos` until within 200 or 121 frames;
/// back on the targets and the next pattern.
fn on_return(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    b.spd_up(0x4248_0000, 0x4248_0000);
    match b.act_proccess {
        0 => {
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.act_proccess += 1;
            cx.se3d(229);
        }
        1 => {
            let c = b.act_count;
            if c & 3 == 3 {
                cx.out(Out::AfterImage);
                if c == 3 {
                    let pos = cx.pos();
                    cx.out(Out::Se3dNote { se: 57, pos, note: 55 });
                    cx.out(Out::Se3d { se: 57, pos });
                }
            }
            if c >= 121 {
                b.act_proccess += 1;
                return;
            }
            let dest = w2p(cx, b.dash_pos);
            let pp = cx.scene.chars[me].pos_p;
            b.move_dirc = get_dirc(pp, dest);
            if ee::lt(crate::boss::dist(cx, pp, dest), 0x4348_0000) {
                b.act_proccess += 1;
            }
            b.dirc[2] = b.move_dirc;
            b.act_count += 1;
        }
        2 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// The barrier's message (0x003b5b80): unless the party is out, the party
/// held, off the targets and menu 74's stream 94; once it closes, all back
/// and the next pattern.
fn on_barrier_msg(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            if cx.annihilated() || cx.game_over {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.lock_player(cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::StreamMenu { stream: 94, mask: 0 });
            b.act_proccess += 1;
        }
        1 => {
            if cx.env.menu_type != -1 {
                return;
            }
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// The death (0x003b5cf0): off the targets, the party held, the camera
/// 1500 out and 200 up from 200 over it along its heading; once not held
/// and no menu, hidden, the camera back, out of the lists and exited.
fn on_die(b: &mut BossBase, _x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.lock_player(cx, false);
            let mut vp = cx.pos();
            vp[2] = ee::add(vp[2], 0x4348_0000);
            let d = b.dirc;
            let m = geom::rot_matrix_x(&geom::unit_matrix(), d[0]);
            let m = geom::rot_matrix_y(&m, d[1]);
            let m = geom::rot_matrix_z(&m, d[2]);
            let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
            let v = geom::apply_matrix(&m, [0x44bb_8000, 0, 0x4348_0000, ONE]);
            let eye = p2w(cx, geom::vadd(v, w2p(cx, vp)));
            free_cam(cx, eye, vp);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            b.act_count += 1;
            if cx.scene.chars[me].cond[cond::HOLD] != 0 || cx.env.menu_type != -1 {
                return;
            }
            b.unlock_player();
            b.draw_sw = 0;
            cam_back(cx);
            b.exit = 1;
            crate::fellow::delete_cmnd(cx.scene, me);
            cx.out(Out::DeleteCmnd);
        }
        _ => {}
    }
}

/// `OnThinkRandomDrive` (0x003b6960): still while stopped or held. The
/// camera locked, the party locked, a point 250 out from it; then five
/// runs (25 frames, then 30 each, turning 150 degrees, then 120), at 10
/// then 50, an after-image every third frame; then by `ccRand() & 0xff`
/// the neutral (below 128), the press (below 240) or the star dust.
fn on_random_drive(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    if b.stop != 0 || cx.scene.chars[me].cond[cond::HOLD] != 0 {
        b.move_spd = 0;
        b.move_vector = VF0;
        return;
    }
    match b.act_proccess {
        0 => {
            cx.out(Out::CamLock(true));
            b.lock_player(cx, true);
            b.move_spd = 0;
            b.act_proccess += 1;
            x.rd = RdInfo::default();
            let pp = cx.scene.chars[me].pos_p;
            let dir = get_dirc(b.center_pos_p, pp);
            x.rd.dir = dir;
            x.rd.spd = 0x4120_0000;
            let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI), dir);
            let v = geom::apply_matrix(&m, [0x437a_0000, 0, 0, ONE]);
            x.rd.point = p2w(cx, geom::vadd(pp, v));
        }
        1 => {
            match x.rd.step {
                0 => {
                    let c = b.act_count;
                    b.act_count += 1;
                    if c == 25 {
                        b.act_count = 0;
                        x.rd.dir = wrap_lo(wrap_hi(ee::add(x.rd.dir, 0x4027_8d37)));
                        x.rd.step += 1;
                        x.rd.spd = 0x4248_0000;
                    }
                }
                1..=3 => {
                    let c = b.act_count;
                    b.act_count += 1;
                    if c == 30 {
                        x.rd.dir = wrap_hi(ee::add(x.rd.dir, 0x4006_0a92));
                        b.act_count = 0;
                        x.rd.step += 1;
                    }
                }
                4 => {
                    b.move_vector = VF0;
                    b.act_proccess += 1;
                    return;
                }
                _ => {}
            }
            let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI), x.rd.dir);
            b.move_vector = geom::apply_matrix(&m, [x.rd.spd, 0, 0, ONE]);
            let a = x.rd.after;
            x.rd.after += 1;
            if a == 2 {
                x.rd.after = 0;
                cx.out(Out::AfterImageWith(EAI_RD));
            }
        }
        2 => {
            let r = cx.cc.rand() & 0xff;
            if r < 128 {
                b.change_action(cx, act::NEUTRAL, 1, false);
            } else if r < 240 {
                b.spd_down(0, 0x40a0_0000);
                b.change_action(cx, act::BODY_PRESS, 3, true);
                b.act_count = 0;
            } else {
                b.spd_down(0, 0x40a0_0000);
                b.change_action(cx, act::STAR_DUST, 3, true);
                b.act_count = 0;
            }
            b.unlock_player();
            cx.out(Out::CamLock(false));
        }
        _ => {}
    }
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.move_dirc, 256);
}

/// `SetupRoot` (0x003b8910): the five roots set out between it and the
/// target (a sixth of the way apart, `ccRandF(100)` aside), its heading.
fn setup_root(b: &BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    let pp = cx.scene.chars[me].pos_p;
    let dist = crate::boss::dist(cx, pp, w2p(cx, b.target_pos));
    let m1 = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI), b.target_dirc);
    let m2 = geom::rot_matrix_z(&m1, NEG_HALF_PI);
    for k in 0..ROOTS {
        let a = ee::div(ee::mul(dist, ee::from_int(k as i32 + 1)), 0x40c0_0000);
        let mut v = geom::apply_matrix(&m1, [a, 0, 0, ONE]);
        let s = rand_f(cx.cc, 0x42c8_0000);
        let w = geom::apply_matrix(&m2, [s, 0, 0, ONE]);
        v = geom::vadd(v, w);
        let pos = p2w(cx, geom::vadd(pp, v));
        let rm = x.roots[k].me;
        cx.scene.chars[rm].pos = pos;
        x.roots[k].b.base.dirc = b.dirc;
    }
}

/// `EntryRoot` (0x003b8b00): the first exited root up (`Order(3)`), its
/// slot; -1 for none.
fn entry_root(x: &mut Second, cx: &mut Cx) -> i32 {
    let Some(k) = (0..ROOTS).find(|&k| x.roots[k].b.base.exit != 0) else { return -1 };
    with_root(x, k, cx, |rb, _, cx| root_order(rb, cx, 3));
    k as i32
}

/// `OnThinkRootDrain` (0x003b7560), nine steps: at `SelectTarget(1)`,
/// locked, cinema 57; to 1500 past the target from the centre (after-
/// images); 60 frames on, a camera on it; the drain's clip; at its frame 29
/// the ground breaks (SE 36, 56, a quake), the roots set out; every eighth
/// frame a root up; the camera pushed in 50 a frame from 15 to 40 (the
/// blur's beat); the camera circling the target until the roots' clip is
/// over at 90; the drain (`ccBossEffRootDrain`) at the target, boss skill
/// 57 every fifth frame from 45 to 95 draining HP and SP, the camera
/// rising; over, the roots fade; then its clip, and all back.
fn on_root_drain(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            if !pick(b, x, cx, 1) {
                return;
            }
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::Cinema(Some(57)));
            x.anm_root_l.set(ANM_ROOT_L, cx.clips);
            x.root_transparency = 0;
            x.root_anm_status = false;
            x.old_pos = cx.pos();
            let d = round_target(b, cx);
            x.act_pos = out_from(cx, b.target_pos, d, [0x44bb_8000, 0, 0, ONE]);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            b.move_spd = 0x42c8_0000;
            let dest = w2p(cx, x.act_pos);
            let pp = cx.scene.chars[me].pos_p;
            b.move_dirc = get_dirc(pp, dest);
            b.dirc[2] = b.move_dirc;
            if ee::le(crate::boss::dist(cx, pp, dest), 0x42f0_0000) {
                b.move_spd = 0;
                b.act_count = 0;
                b.act_proccess += 1;
            }
            if c & 1 == 0 {
                cx.out(Out::AfterImage);
                if c == 0 {
                    cx.out(Out::Se3dNote { se: 57, pos: cx.pos(), note: 55 });
                }
            }
        }
        2 => {
            if c >= 60 {
                cx.scene.chars[me].pos = x.act_pos;
                cx.scene.chars[me].pos_p = w2p(cx, x.act_pos);
                let pp = cx.scene.chars[me].pos_p;
                let d = get_dirc(pp, w2p(cx, b.target_pos));
                let eye = out_from(cx, x.act_pos, d, [0x447a_0000, 0, 0x4248_0000, ONE]);
                let view = out_from(cx, eye, d, [0xc2c8_0000, 0, 0, ONE]);
                free_cam(cx, eye, view);
                b.move_spd = 0;
                b.dirc[2] = b.center_dirc;
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        3 => {
            b.dirc[2] = b.center_dirc;
            if b.anm.frame() == 1 {
                b.anm.set(ANM_ROOT_ATC, cx.clips);
                b.act_proccess += 1;
            }
        }
        4 => {
            let f = b.anm.frame();
            if f == 29 {
                cx.out(Out::Se { se: 36 });
                cx.out(Out::Se { se: 56 });
                let pos = cx.pos();
                cx.out(Out::Corbenik(Pic::Radiate { pos }));
                for k in 0..4 {
                    let z = ee::mul(0x40fb_53d2, ee::from_int(k));
                    cx.out(Out::Corbenik(Pic::Smoke { pos, rot: [0, 0, z, ONE] }));
                }
                quake(cx, [0x41f0_0000, 0x41f0_0000, 0x4248_0000]);
            } else if f == 35 {
                cx.out(Out::SeNote { se: 63, note: 52 });
            }
            if b.anm_status != 0 {
                b.anm.set(ANM_ROOT_NUT, cx.clips);
                setup_root(b, x, cx);
                x.cam_pos = cx.cam.active_pos;
                x.cam_view = cx.cam.active_view;
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        5 => {
            if c & 7 == 0 {
                let r = entry_root(x, cx);
                if r != -1 {
                    cx.out(Out::Se { se: 206 });
                } else {
                    x.root_transparency = ONE;
                }
            }
            if !(15..41).contains(&c) {
                // nothing
            } else if c == 40 {
                cx.out(Out::Blur(blur::Set::NoFunc));
                b.act_count = 0;
                b.act_proccess += 1;
            } else {
                if c == 15 {
                    cx.out(Out::Blur(blur::Set::Func(blur::BlurFunc::Boss08B)));
                }
                let off = out_from(cx, VF0, b.target_dirc, [0x4248_0000, 0, 0, ONE]);
                let view = p2w(cx, geom::vadd(w2p(cx, cx.cam.boss_view), off));
                let eye = p2w(cx, geom::vadd(w2p(cx, cx.cam.eye), off));
                b.m_act_dirc = VF0;
                cx.out(Out::FreeCam { pos: eye, view });
            }
        }
        6 => {
            if c >= 60 {
                if c == 60 {
                    let e = cx.cam.eye;
                    let pos = cx.pos();
                    x.act_pos = out_from(cx, pos, b.target_dirc, [0x4448_0000, 0, e[2], ONE]);
                    b.m_act_dirc[2] = b.target_dirc;
                    b.m_act_dirc[0] = crate::boss::dist(cx, w2p(cx, e), w2p(cx, x.act_pos));
                }
                let view = x.act_pos;
                let a = wrap_hi(ee::add(0x3c8e_fa35, b.m_act_dirc[2]));
                b.m_act_dirc[2] = a;
                b.m_act_dirc[0] = ee::add(b.m_act_dirc[0], 0x41ba_aaab);
                let eye = out_from(cx, x.act_pos, a, [b.m_act_dirc[0], 0, 0, ONE]);
                cx.out(Out::FreeCam { pos: eye, view });
            }
            if x.root_anm_status && c == 90 {
                let tp = b.target_pos;
                x.gen_serial += 1;
                let d = eff::Drain::new(tp, x.gen_serial);
                x.boss_eff = Some(make_fx(b, cx, EffKind::RootDrain, Fx::Drain(Box::new(d)), tp));
                x.cam_pos = cx.cam.eye;
                x.cam_view = cx.cam.boss_view;
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        7 => {
            if x.boss_eff.is_some() {
                if !b.effects.enabled(x.boss_eff) {
                    x.boss_eff = None;
                } else if (45..96).contains(&c) && c % 5 == 0 {
                    if c == 45 {
                        cx.out(Out::SeNote { se: 61, note: 36 });
                    }
                    cx.scene.chars[me].cond[cond::DRAIN_HP] = 100;
                    cx.scene.chars[me].cond[cond::DRAIN_SP] = 100;
                    let t = cx.scene.chars[me].target_char;
                    let from = cx.ev.len();
                    b.skill_damage(cx, t, None, 57);
                    self_affects(b, x, cx, from, t);
                    cx.scene.chars[me].cond[cond::DRAIN_HP] = 0;
                    cx.scene.chars[me].cond[cond::DRAIN_SP] = 0;
                    if cx.annihilated() || cx.game_over {
                        for k in 0..ROOTS {
                            x.roots[k].b.base.exit = 1;
                        }
                        root_drain_end(b, x, cx);
                        root_l(b, x, cx);
                        return;
                    }
                }
            }
            // The eye risen a little (its push along the view's way is
            // figured and never used).
            let mut e = w2p(cx, cx.cam.eye);
            e[2] = ee::add(e[2], 0x3fd5_5555);
            let eye = p2w(cx, e);
            if cx.cam.reset == 5 {
                cx.out(Out::CamEye(eye));
            }
            if x.boss_eff.is_none() {
                for k in 0..ROOTS {
                    let rb = &mut x.roots[k].b;
                    let mut t = ee::sub(rb.base.transparency, 0x3d08_8889);
                    if ee::le(t, 0) {
                        t = 0;
                        rb.base.exit = 1;
                    }
                    rb.base.transparency = t;
                    rb.base.set_transparency = t;
                }
                let mut t = ee::sub(x.root_transparency, 0x3d08_8889);
                if ee::le(t, 0) {
                    t = 0;
                }
                if !ee::le(t, 0) {
                    b.act_count = 0;
                }
                if geom::eq(0, t) && c == 45 {
                    b.act_count = 0;
                    b.act_proccess += 1;
                    let pos = cx.pos();
                    let eye = out_from(cx, pos, b.target_dirc, [0x4496_0000, 0, 0x428c_0000, ONE]);
                    let view = out_from(cx, pos, b.target_dirc, [0, 0, 0x4348_0000, ONE]);
                    cx.out(Out::FreeCam { pos: eye, view });
                }
                x.root_transparency = t;
            }
        }
        8 => {
            if c == 30 {
                b.anm.set(ANM_ROOT_END, cx.clips);
                cx.out(Out::SeNote { se: 63, note: 52 });
            }
            if b.anm_status != 0 {
                root_drain_end(b, x, cx);
            }
        }
        _ => {}
    }
    root_l(b, x, cx);
}

/// The drains' `EntryAffect`s on the boss itself, which the game makes at
/// once inside the damage (`calcBattleDamage`'s HP and SP drains, 9 and
/// 10, by the target): taken from the events since `from` and run through
/// the form's `Affect` as `ccChar::EntryAffect` would (on the lists, not
/// drained; 6 nothing).
fn self_affects(b: &mut BossBase, x: &mut Second, cx: &mut Cx, from: usize, target: Option<usize>) {
    use crate::event::{Event, Who};
    let me = cx.me;
    let evs: Vec<Event> = cx.ev.drain(from..).collect();
    for e in evs {
        let Event::Affect { on: Who::Me, by, kind, p } = e else {
            cx.ev.push(e);
            continue;
        };
        let by = match by {
            Who::Target => target,
            Who::Char(c) => Some(c),
            Who::Me => Some(me),
            Who::Nobody => None,
        };
        if !cx.scene.listed(me) || cx.scene.chars[me].affect.ty == 13 || kind == 6 {
            continue;
        }
        if kind == 5 {
            cx.scene.chars[me].cond[cond::HOLD] = 1;
            continue;
        }
        let a = &mut cx.scene.chars[me].affect;
        a.person = by;
        a.ty = kind;
        a.param = p;
        affect(b, x, cx);
    }
}

/// The root drain's end: all back and the next pattern.
fn root_drain_end(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    b.unlock_player();
    b.entry_cmnd_target(cx);
    cursor(cx, false);
    cam_back(cx);
    cx.out(Out::Cinema(None));
    change_next_pattern(b, x, cx);
}

/// The drain's own root (`m_anmRootL`) at the target while it shows; SE
/// 206 at its frame 2.
fn root_l(b: &BossBase, x: &mut Second, cx: &mut Cx) {
    if ee::le(x.root_transparency, 0) {
        return;
    }
    x.root_anm_status = x.anm_root_l.forward();
    let pos = b.target_pos;
    cx.out(Out::Corbenik(Pic::RootL { time: x.anm_root_l.posed, pos, alpha: x.root_transparency }));
    if x.anm_root_l.frame() == 2 {
        cx.out(Out::Se { se: 206 });
    }
}

// --- the roots ------------------------------------------------------------------------------------

/// `Order(n)` (0x003b8e00): opaque, back on, act `n`.
fn root_order(b: &mut BossBase, cx: &mut Cx, n: i16) {
    b.transparency = ONE;
    b.set_transparency = ONE;
    b.exit = 0;
    b.change_action(cx, n, 3, true);
}

/// A root's `ccBoss::Main`: its `Think` (0x003b8e80): up (3), its clip
/// over, a quake of 30 frames and the ground's radiate, then still; dying
/// (14), faded out over 15 frames. Drawn at 1.5.
fn root_main(b: &mut BossBase, x: &mut Root, cx: &mut Cx) {
    b.base_main(cx, &mut |b, cx| match b.act_num {
        3 => {
            if b.anm_status != 0 {
                x.quake_time = 30;
                let pos = cx.pos();
                cx.out(Out::Corbenik(Pic::Radiate { pos }));
                b.change_action(cx, 0, 3, true);
            }
        }
        14 => {
            let mut t = ee::sub(b.transparency, 0x3d88_8889);
            if ee::le(t, 0) {
                t = 0;
                b.exit = 1;
            }
            b.transparency = t;
            b.set_transparency = t;
        }
        _ => {}
    });
}

// --- Affect ---------------------------------------------------------------------------------------

/// `ccBoss08_b::Affect` (0x003b2ae0).
/// A heal (7, 9) passes the guards in the root drain. 6 and 5 the stop; 13
/// the drain: act 11 (no `Think`: the neutral), a tenth of its HP, the
/// gauge -1; 7 and 9 a heal (number 20). A blow (1, 3): on the barrier its
/// number 0, its own count down (out: the death; under 4000 the first
/// time, the message and its 3630-frame wait), its hit clip and SE 99;
/// else the mark, SE 195 at `abs(ccRand() % 5) + 60` and the HP down: at
/// half the super table, at a fifth the barrier asked for (its count the
/// HP before the blow). Its HP never kills it.
pub(crate) fn affect(b: &mut BossBase, x: &mut Second, cx: &mut Cx) {
    let me = cx.me;
    let (t, p0, person) = {
        let a = &cx.scene.chars[me].affect;
        (a.ty, a.param[0], a.person)
    };
    let healing = b.act_num == act::ROOT_DRAIN && (t == 7 || t == 9);
    if !healing && (b.act_forbid == 2 || b.lock_player != 0) {
        return;
    }
    let hp = cx.scene.chars[me].hp;
    let mhp = cx.scene.chars[me].max_hp;
    match t {
        6 => b.stop = 0,
        5 => b.stop = 1,
        13 => {
            b.change_action(cx, act::DRAINED, 2, true);
            cx.scene.chars[me].affect.ty = 0;
            let v = (i32::from(mhp) / 10) as i16;
            cx.scene.chars[me].hp = v;
            cx.scene.chars[me].max_hp = v;
            if let Some(p) = cx.scene.param_mut(me) {
                p.pp = -1;
                p.pp_count = 0;
            }
        }
        7 | 9 => {
            cx.out(Out::FlyFont { kind: 20, n: i32::from(p0) });
            cx.scene.chars[me].hp = crate::boss::heal_hp(hp, p0, mhp);
        }
        1 | 3 => {
            let shown = if x.barrier { if p0 == -1 { -1 } else { 0 } } else { p0 };
            cx.out(Out::FlyFont { kind: 2, n: i32::from(shown) });
            if p0 < 0 {
                return;
            }
            if x.barrier {
                x.life_counter -= i32::from(p0);
                if x.life_counter <= 0 {
                    x.msg_count = -1;
                    b.change_action(cx, act::DIE, 2, true);
                    return;
                }
                if x.life_counter < 4000 && !x.msg_shown {
                    b.change_action(cx, act::BARRIER_MSG, 3, true);
                    x.msg_shown = true;
                    x.msg_count = 3630;
                }
                if let Some(Some(c)) = cx.data.corbenik.dmg_barrier.first() {
                    x.anm_barrier.set(c, cx.clips);
                }
                x.flash_barrier = true;
                cx.se3d(99);
                return;
            }
            cx.out(Out::HitMark { by: person });
            let h = crate::boss::hit_hp(hp, p0, None, mhp);
            let note = (cx.cc.rand() % 5).abs() + 60;
            cx.out(Out::Se3dNote { se: 195, pos: cx.pos(), note: note as u8 });
            if h > 0 {
                if x.pat_mode == 0 && mhp / 2 >= h {
                    set_pattern_mode(b, x, cx, 1);
                }
                if mhp / 5 >= h && !x.barrier {
                    x.reserve_setup_barrier = true;
                    x.life_counter = i32::from(cx.scene.chars[me].hp);
                }
            }
            cx.scene.chars[me].hp = h;
        }
        _ => {}
    }
}
