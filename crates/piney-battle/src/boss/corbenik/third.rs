//! Corbenik's third form (`ccBoss08_c`, QUA gcmn 0x003b9070-0x003be910):
//! the body (`bossTbl` row 9, `x81`) event 313's second fight plays in a
//! task of its own (`ccThBoss08c_Main`), with sixteen eyes riding its
//! `DMY_dummy1` and three killer eyes ([`super::eye`]). Its patterns
//! change at half its HP; its lasers bounce off the eyes or fall from
//! above (the tera laser), and its eye entry throws out the killer eyes.

use piney_data::field::ee;

use super::eff::Fx;
use super::eye::{self, Eye, Killer};
use super::laser::{Refrect, Tera};
use super::{Gen, Host, Pic, make_fx, manager_pass_x, select, target_ok, word};
use crate::boss::kyvia::{Part, as_me, new_char, plain_boss, set_base_param};
use crate::boss::{Anm, Boss, BossBase, Class, Cx, EffKind, Out, Tbl, VF0, blur};
use crate::enemy_ai::get_dirc;
use crate::geom::{self, M4, V4};
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const NEG_PI: F = 0xc049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// The stage fader's colours: in, and back out.
const DARK_IN: u32 = 0x8000_0000;
const DARK_OUT: u32 = 0x6400_0000;
/// `bossTbl` row 9: the body's (and the eyes').
pub const ROW: usize = 9;
pub const EYES: usize = 16;
pub const KILLERS: usize = 3;
/// `InitBossCamera(980, 1500)`, then its pitch `atan2f(500, 1000)`, its
/// range 1000 and `SetRotXLimit(1.0)`.
pub const CAM_Z: F = 0x4475_0000;
pub const CAM_Y: F = 0x44bb_8000;
/// The blur: on, `0x48808080`, scale 1.01.
pub const BLUR_ABGR: u32 = 0x4880_8080;
pub const BLUR_SCALE: F = 0x3f81_47ae;
const ANM_WAVE: &str = "ANM_xx11wave";

/// The third form's acts (`Think` 0x003b9a50; any other does nothing).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DMG0: i16 = 1;
    pub const DMG1: i16 = 2;
    pub const LASER: i16 = 3;
    pub const DATA_DRAIN: i16 = 4;
    pub const TERA_LASER: i16 = 5;
    pub const WAVE: i16 = 6;
    pub const WANDER: i16 = 8;
    pub const DIE: i16 = 14;
    pub const ENTRY_EYE: i16 = 20;
    pub const SLAVE_ATTACK: i16 = 21;
}

/// `ccBoss08_c`'s own members (Quarantine's layout, 0xa4fb0 bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Third {
    /// The eyes' and killer eyes' characters, and (for the frame's span)
    /// the parts out of them.
    pub eye_me: [usize; EYES],
    pub killer_me: [usize; KILLERS],
    pub eyes: Vec<Part<Eye>>,
    pub killers: Vec<Part<Killer>>,
    /// +0x29350 `anmw` (the wave).
    pub anm_w: Anm,
    /// +0x29354 `dmyEye`: `DMY_dummy1`'s place in the body's file (w 1).
    pub dummy: V4,
    /// The body clip's world matrix (`ccAnm` +0) as its last draw left it.
    pub body_world: M4,
    /// +0x29358 `m_effLaser`: the reflected lasers' slots.
    pub eff_laser: [Option<i32>; 16],
    /// +0x29398 `m_genEnergy`: the laser's charge.
    pub gen_energy: Option<u32>,
    /// +0xa4f44 `m_stageEff`, +0xa4f48 `m_patMode`, +0xa4f4c `m_bossEff`
    /// (the tera laser).
    pub stage_eff: i32,
    pub pat_mode: i32,
    pub boss_eff: Option<i32>,
    /// +0xa4f60 `m_eyeCenterOffset`, +0xa4f70 `m_oldPos`.
    pub eye_center_offset: V4,
    pub old_pos: V4,
    /// +0xa4f80 `m_pauseSlaveFlag`, +0xa4f88 `m_attackSlave` (pattern 18's
    /// row), +0xa4f8c `m_slvAtkMode` (always 1).
    pub pause: bool,
    pub attack_slave: i32,
    pub slv_atk_mode: i32,
    /// +0xa4f90: the half-HP turn taken (Quarantine's).
    pub half: bool,
    /// +0xa4fa0: where it dies (Quarantine's).
    pub dead_pos: V4,
    /// `ccBoss::actPos` (+0x1b0): where the laser started.
    pub act_pos: V4,
    pub gen_serial: u32,
}

fn w2p(cx: &Cx, v: V4) -> V4 {
    crate::boss::w2p(cx, v)
}

fn p2w(cx: &Cx, v: V4) -> V4 {
    crate::boss::p2w(cx, v)
}

/// `P2W(W2P(at) + Rz(-pi/2) Rz(d) v)`.
fn out_from(cx: &Cx, at: V4, d: F, v: V4) -> V4 {
    let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI), d);
    let o = geom::apply_matrix(&m, v);
    p2w(cx, geom::vadd(w2p(cx, at), o))
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

fn arena_clump(cx: &mut Cx, on: bool) {
    cx.out(Out::Corbenik(Pic::ArenaClump { on }));
}

/// `ccCheckTarget(t)` with `t` not dead (`lh 8`).
fn alive(cx: &Cx, t: Option<usize>) -> bool {
    target_ok(cx, t)
}

impl Third {
    /// `Order(23)` on eye `k`: its flash.
    pub(crate) fn flash_eye(&mut self, k: usize) {
        if let Some(p) = self.eyes.get_mut(k) {
            p.x.color_blend = 0x42c8_0000;
        }
    }

    fn next_key(&mut self) -> u32 {
        self.gen_serial += 1;
        self.gen_serial
    }
}

/// The parts out of their characters for the frame.
fn take_parts(cx: &mut Cx, x: &mut Third) -> ([bool; EYES], [bool; KILLERS]) {
    x.eyes.clear();
    x.killers.clear();
    let e = std::array::from_fn(|k| {
        let (p, f) = eye::take_eye(cx, x.eye_me[k]);
        x.eyes.push(p);
        f
    });
    let k = std::array::from_fn(|k| {
        let (p, f) = eye::take_killer(cx, x.killer_me[k]);
        x.killers.push(p);
        f
    });
    (e, k)
}

fn put_parts(cx: &mut Cx, x: &mut Third, found: ([bool; EYES], [bool; KILLERS])) {
    for (k, p) in std::mem::take(&mut x.eyes).into_iter().enumerate() {
        eye::put_eye(cx, p, found.0[k]);
    }
    for (k, p) in std::mem::take(&mut x.killers).into_iter().enumerate() {
        eye::put_killer(cx, p, found.1[k]);
    }
}

// --- the constructor ------------------------------------------------------------------------------

/// `ccBoss08_c::ccBoss08_c` (0x003b9250): the three killer eyes made first
/// (out, at Kite), then the body: row 9, 500 behind Kite (his heading,
/// turned to face him), drawn and hit, its clips, the dummy (`dummy`,
/// `DMY_dummy1` in its file) and the wave clip, on the lists; the sixteen
/// eyes; the centre and the stage fader; act 0, `SetPatternMode(0)`; the
/// boss camera and the blur; the killer eyes' master.
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4, dummy: V4) -> Boss {
    let me = cx.me;
    let killer_me: [usize; KILLERS] = std::array::from_fn(|_| {
        let km = new_char(cx, eye::ROW);
        let part = eye::new_killer(cx, km, kite_pos, kite_dirc);
        eye::put_killer(cx, part, true);
        km
    });
    let mut b = plain_boss();
    set_base_param(cx, me, ROW);
    b.anm_tbl = cx.data.corbenik.c_anims.clone();
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    b.dirc = kite_dirc;
    let mut pos = kite_pos;
    pos[1] = ee::sub(pos[1], 0x43fa_0000);
    let pp = w2p(cx, pos);
    cx.scene.chars[me].pos = pos;
    cx.scene.chars[me].pos_p = pp;
    b.dirc[2] = get_dirc(pp, w2p(cx, kite_pos));
    if let Some(Some(c)) = b.anm_tbl.first() {
        let c = c.clone();
        b.anm.set(&c, cx.clips);
    }
    let mut x = Box::new(Third {
        killer_me,
        anm_w: Anm::new(),
        dummy,
        body_world: geom::unit_matrix(),
        stage_eff: 0,
        ..Third::default()
    });
    x.anm_w.set(ANM_WAVE, cx.clips);
    crate::fellow::entry_cmnd(cx.scene, me);
    setup_slave(&mut x, cx);
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = w2p(cx, b.center_pos);
    b.use_stage_eff = true;
    let found = take_parts(cx, &mut x);
    change_action(&mut b, &mut x, cx, act::NEUTRAL, 0, true);
    b.cheat_hp = 0;
    set_pattern_mode(&mut b, &mut x, cx, 0);
    x.boss_eff = None;
    x.eye_center_offset = VF0;
    x.pause = false;
    x.slv_atk_mode = 1;
    x.half = false;
    cx.out(Out::Blur(blur::Set::Enable));
    cx.out(Out::Blur(blur::Set::Abgr(BLUR_ABGR)));
    cx.out(Out::Blur(blur::Set::Scale(BLUR_SCALE)));
    for (k, p) in x.killers.iter_mut().enumerate() {
        eye::set_master(&mut p.x, me, k as i32);
    }
    put_parts(cx, &mut x, found);
    Boss { base: b, class: Class::CorbenikC(x) }
}

/// `SetupSlave` (0x003bd960): the sixteen eyes made (each its own
/// constructor's three draws first), then each set on the body.
fn setup_slave(x: &mut Third, cx: &mut Cx) {
    let me = cx.me;
    let pp = w2p(cx, cx.scene.chars[me].pos);
    cx.scene.chars[me].pos_p = pp;
    let mut parts: Vec<Part<Eye>> = (0..EYES)
        .map(|_| {
            let em = new_char(cx, eye::ROW);
            eye::new_eye(cx, em)
        })
        .collect();
    for (k, p) in parts.iter_mut().enumerate() {
        eye::setup_eye(p, cx, me, k as i32);
        x.eye_me[k] = p.me;
    }
    for p in parts {
        eye::put_eye(cx, p, true);
    }
}

// --- the frame ------------------------------------------------------------------------------------

/// The effect manager's pass, then `ccBoss08_c::Main` (0x003b99b0, the
/// base's).
pub(crate) fn main(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let found = take_parts(cx, x);
    manager_pass_x(b, cx, Some(x));
    frame(b, x, cx);
    put_parts(cx, x, found);
}

fn frame(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
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
        draw(b, x, cx);
    }
    slave(b, x, cx);
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// `PreDrawAnm` (0x003b99d0): the base's (the clip forward), then the
/// eyes' centre offset: the dummy by the clip's last drawn pose, less its
/// place.
fn pre_draw_anm(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    b.anm_status = i8::from(b.anm.forward());
    let d = w2p(cx, eye::dummy_at(&x.body_world, x.dummy));
    x.eye_center_offset = geom::vsub(d, cx.scene.chars[cx.me].pos_p);
}

/// `ccBoss::Draw` (QUA gcmn 0x00361040) as far as the rules see it: the
/// transparency the camera's fade leaves, and the clip drawn while it
/// shows, which leaves its world matrix `SetMatrix_PosRotZYX(pos, dirc)`.
fn draw(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let pos = cx.scene.chars[cx.me].pos;
    if b.draw(cx) {
        x.body_world = geom::trans_matrix(&geom::rot_matrix(&geom::unit_matrix(), b.dirc), pos);
    }
}

/// `Slave` (0x003bd870): each eye its `Main` (the body's target, its way
/// and distance handed down), then each killer eye's.
fn slave(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let m = x.master_view(b, cx);
    for p in &mut x.eyes {
        eye::eye_main(p, &m, cx);
    }
    let v = x.killer_view(b, cx);
    let mut killers = std::mem::take(&mut x.killers);
    for p in &mut killers {
        eye::killer_main(b, p, &v, cx);
    }
    x.killers = killers;
}

// --- the patterns ---------------------------------------------------------------------------------

/// `ccBoss08_c::ChangeAction(n, forbid, af)` (0x003bd6c0): the base's, and
/// each eye told the act (`Order(n)`).
pub(crate) fn change_action(b: &mut BossBase, x: &mut Third, cx: &mut Cx, n: i16, forbid: i16, af: bool) {
    let ok = match forbid {
        0 => b.act_forbid == 0,
        1 => {
            b.act_forbid = 0;
            true
        }
        2 | 3 => {
            b.act_forbid = forbid as i8;
            true
        }
        _ => false,
    };
    if !ok {
        return;
    }
    b.act_num = n;
    b.act_proccess = 0;
    b.act_count = 0;
    for p in &mut x.eyes {
        as_me(cx, p.me, |cx| eye::order(&mut p.b.base, &mut p.x, cx, n));
    }
    if af && let Some(Some(clip)) = usize::try_from(n).ok().and_then(|k| b.anm_tbl.get(k)) {
        let clip = clip.clone();
        b.anm.set(&clip, cx.clips);
        if b.epitaph == 0 && b.act_num == 12 {
            b.epitaph = 1;
        }
    }
}

struct ThirdHost<'a> {
    x: &'a mut Third,
}

impl Host for ThirdHost<'_> {
    fn change_action(&mut self, b: &mut BossBase, cx: &mut Cx, n: i16, forbid: i16, af: bool) {
        change_action(b, self.x, cx, n, forbid, af);
    }

    fn exec_pattern(&mut self, b: &mut BossBase, cx: &mut Cx, pat: i32, p0: i32) {
        exec_pattern(b, self.x, cx, pat, p0);
    }
}

fn words(cx: &Cx, b: &BossBase) -> Vec<i32> {
    cx.data.corbenik.words_c(b.pat_tbl).to_vec()
}

/// `SetPatternMode(m)` (0x003b9b90): 0 the first table, 1 the normal, any
/// other none; run from its start.
fn set_pattern_mode(b: &mut BossBase, x: &mut Third, cx: &mut Cx, m: i32) {
    b.pat_tbl = match m {
        0 => Tbl::Normal,
        1 => Tbl::Super,
        _ => Tbl::None,
    };
    b.pat_index = 0;
    x.pat_mode = m;
    let tbl = words(cx, b);
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, 0);
}

fn exec_pattern(b: &mut BossBase, x: &mut Third, cx: &mut Cx, pat: i32, p0: i32) {
    let tbl = [pat, p0, 0, 0, -1];
    exec_pattern_index(b, x, cx, &tbl, 0);
}

/// `ccBoss::ChangeNextPattern` (QUA gcmn 0x00360da0) through this class.
fn change_next_pattern(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let tbl = words(cx, b);
    if word(&tbl, b.pat_index) == -1 {
        b.pat_index = 0;
    }
    b.move_spd = 0;
    b.move_vector = VF0;
    if b.epitaph != 0 {
        change_action(b, x, cx, 12, 3, true);
    } else {
        change_action(b, x, cx, act::NEUTRAL, 1, true);
    }
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, b.pat_index);
}

/// `ExecPatternIndex` (0x003b9c50), the slaves let go first each word: 1
/// the wave (6), 14 the laser (3), 15 the drain (4), 16 the tera laser (5)
/// (those three holding the slaves), 17 the eye entry (20) once every
/// killer eye is out (else the next word), 18 a slave attack (21) by the
/// next word's row; the rest the base's, from the word it was called at.
fn exec_pattern_index(b: &mut BossBase, x: &mut Third, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
    let mut s = i;
    loop {
        let mut pat = word(tbl, s);
        s += 1;
        b.move_spd = 0;
        b.move_vector = VF0;
        if pat == -1 {
            pat = word(tbl, 0);
            s = 1;
        }
        b.pat_num = pat;
        x.pause = false;
        match pat {
            1 => change_action(b, x, cx, act::WAVE, 3, true),
            17 => {
                if x.killers.iter().any(|p| p.b.base.exit == 0) {
                    continue;
                }
                change_action(b, x, cx, act::ENTRY_EYE, 3, true);
            }
            15 => {
                x.pause = true;
                change_action(b, x, cx, act::DATA_DRAIN, 3, true);
            }
            14 => {
                x.pause = true;
                change_action(b, x, cx, act::LASER, 3, true);
            }
            16 => {
                x.pause = true;
                change_action(b, x, cx, act::TERA_LASER, 3, true);
            }
            18 => {
                x.attack_slave = word(tbl, s);
                s += 1;
                change_action(b, x, cx, act::SLAVE_ATTACK, 3, true);
            }
            _ => return super::base_exec_pattern_index(b, cx, &mut ThirdHost { x }, tbl, i),
        }
        return s;
    }
}

/// `ccBoss08_c::Think` (0x003b9a50).
fn think(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => on_neutral(b, x, cx),
        act::DMG0 | act::DMG1 => change_action(b, x, cx, act::NEUTRAL, 1, false),
        act::DIE => on_die(b, x, cx),
        act::LASER => on_laser(b, x, cx),
        act::DATA_DRAIN => on_data_drain(b, x, cx),
        act::TERA_LASER => on_tera_laser(b, x, cx),
        act::WAVE => on_wave(b, x, cx),
        act::ENTRY_EYE => on_entry_eye(b, x, cx),
        act::SLAVE_ATTACK => on_slave_attack(b, x, cx),
        act::WANDER => on_wander(b, x, cx),
        _ => {}
    }
}

/// `SelectTarget(0)`; none alive, pattern 2 -1.
fn pick(b: &mut BossBase, x: &mut Third, cx: &mut Cx) -> bool {
    let t = select(b, cx, 0);
    if !alive(cx, t) {
        exec_pattern(b, x, cx, 2, -1);
        return false;
    }
    b.calc_target_info(cx);
    true
}

/// Each killer eye not out: off (`EraseCmndTarget`) or on the targets.
fn killers_targets(x: &mut Third, cx: &mut Cx, on: bool) {
    for p in &mut x.killers {
        if p.b.base.exit != 0 {
            continue;
        }
        as_me(cx, p.me, |cx| if on { p.b.base.entry_cmnd_target(cx) } else { p.b.base.erase_cmnd_target(cx) });
    }
}

/// `OnThinkNeutral` (0x003b9ff0): still, turning to its target
/// (`SelectTarget(0)` each frame); the wait (-3: while a killer eye is
/// out and busy but in its neutral or wander), then the next pattern.
fn on_neutral(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let me = cx.me;
    b.move_spd = 0;
    b.move_vector = VF0;
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 256);
    select(b, cx, 0);
    b.calc_target_info(cx);
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
    } else if b.stop_time == -3
        && x.killers
            .iter()
            .any(|p| p.b.base.exit == 0 && p.b.base.act_num != 0 && p.b.base.act_num != eye::killer_act::WANDER)
    {
        return;
    }
    change_next_pattern(b, x, cx);
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 256);
}

/// The death (0x003ba210): the party freed, everything off the lists, the
/// camera 2000 out and 300 up along its heading; at a point 50 before it
/// and 520 up `BossDeadGenerator` row 0 (SE 56), 35 frames on row 1 20
/// higher (SE 104), 60 on row 1 again 20 higher; 20 more a fade to black,
/// at 140 out and the camera back.
fn on_die(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.lock_player(cx, false);
            killers_targets(x, cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            crate::fellow::delete_cmnd(cx.scene, me);
            cx.out(Out::DeleteCmnd);
            let pos = cx.pos();
            let mut view = pos;
            view[2] = ee::add(view[2], 0x4396_0000);
            let d = b.dirc[2];
            let eye = out_from(cx, pos, d, [0x44fa_0000, 0, 0x4396_0000, ONE]);
            free_cam(cx, eye, view);
            x.dead_pos = out_from(cx, pos, d, [0x4248_0000, 0, 0, ONE]);
            x.dead_pos[2] = ee::add(x.dead_pos[2], 0x4396_0000);
            for p in &x.killers {
                crate::fellow::delete_cmnd(cx.scene, p.me);
                cx.out(Out::DeleteCmnd);
            }
            b.act_proccess += 1;
        }
        1 => {
            b.act_proccess += 1;
            x.dead_pos[2] = ee::add(x.dead_pos[2], 0x435c_0000);
            cx.out(Out::Corbenik(Pic::Particles { which: Gen::BossDead0, pos: x.dead_pos }));
            cx.out(Out::SeNote { se: 56, note: 56 });
        }
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 35 {
                b.act_proccess += 1;
                x.dead_pos[2] = ee::add(x.dead_pos[2], 0x41a0_0000);
                cx.out(Out::Corbenik(Pic::Particles { which: Gen::BossDead1a, pos: x.dead_pos }));
                b.act_count = 0;
                cx.out(Out::SeNote { se: 104, note: 36 });
            }
        }
        3 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 60 {
                b.act_proccess += 1;
                x.dead_pos[2] = ee::add(x.dead_pos[2], 0x41a0_0000);
                cx.out(Out::Corbenik(Pic::Particles { which: Gen::BossDead1b, pos: x.dead_pos }));
                b.act_count = 0;
            }
        }
        4 => {
            b.act_count += 1;
            match b.act_count {
                20 => cx.out(Out::EntryFade { t: 45, from: 0, to: 0x8000_0000 }),
                140 => {
                    on_exit(b, x, cx);
                    cam_back(cx);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// `OnExit` (0x003ba8e0): each killer eye out and off the lists, then the
/// body out.
fn on_exit(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    for p in &mut x.killers {
        eye::on_exit(&mut p.b.base, cx);
        crate::fellow::delete_cmnd(cx.scene, p.me);
        cx.out(Out::DeleteCmnd);
    }
    b.exit = 1;
}

/// `OnThinkDataDrain` (0x003ba9c0): its target (none, pattern 2 -1), the
/// party held, off the lists, the slaves paused, the cinema; at 45 stream
/// 107 on the drained member's slot; once the menu is shut the drain (13)
/// on the target, all back and the next pattern.
fn on_data_drain(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            if !pick(b, x, cx) {
                return;
            }
            b.lock_player(cx, false);
            killers_targets(x, cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            x.pause = true;
            cx.out(Out::Cinema(Some(-1)));
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            if c != 45 {
                return;
            }
            let id = cx.scene.chars[me].target_char.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
            let slot = cx.party.slot_of(id);
            cx.out(Out::StreamMenu { stream: 107, mask: 1 << slot });
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 => {
            if cx.env.menu_type != -1 {
                return;
            }
            x.pause = false;
            let t = cx.scene.chars[me].target_char;
            if let Some(tp) = t
                && alive(cx, t)
            {
                cx.affect(tp, 13, 0);
            }
            b.unlock_player();
            killers_targets(x, cx, true);
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::Cinema(None));
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkEntryEye` (0x003bad50): the party held, off the lists, facing
/// the centre; the camera 1500 out from the dummy a quarter turn off, the
/// eye's birth there and the killer eyes thrown (SE 48); at 90 the camera
/// 3000 out and 300 up, 30 degrees off its heading, drawn out 6.67 a frame
/// to 220; all back and the next pattern.
fn on_entry_eye(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            b.lock_player(cx, true);
            killers_targets(x, cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.dirc[2] = b.center_dirc;
            let mut d = ee::sub(b.center_dirc, 0x3f49_0fdb);
            if ee::lt(d, NEG_PI) {
                d = ee::add(d, TWO_PI);
            } else if !ee::le(d, PI) {
                d = ee::sub(d, TWO_PI);
            }
            let at = eye::dummy_at(&x.body_world, x.dummy);
            let eye_pos = out_from(cx, at, d, [0x44bb_8000, 0, 0xc2c8_0000, ONE]);
            free_cam(cx, eye_pos, at);
            cx.out(Out::Corbenik(Pic::Particles { which: Gen::GenerateEye, pos: at }));
            entry_killer(b, x, cx);
            cx.out(Out::SeNote { se: 48, note: 48 });
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            if c == 120 {
                cx.out(Out::SeNote { se: 57, note: 63 });
            }
            if c < 90 {
                return;
            }
            let mut eye_at = cx.cam.eye;
            if c == 90 {
                let mut d = ee::sub(b.dirc[2], 0x3f06_0a92);
                if ee::lt(d, NEG_PI) {
                    d = ee::add(d, TWO_PI);
                }
                let pos = cx.pos();
                let view = out_from(cx, pos, b.center_dirc, [0x44a2_8000, 0, 0x4348_0000, ONE]);
                let e = out_from(cx, pos, d, [0x453b_8000, 0, 0x4396_0000, ONE]);
                cx.out(Out::FreeCam { pos: e, view });
                if cx.cam.reset == 5 {
                    eye_at = e;
                }
            } else if c == 220 {
                b.act_proccess += 1;
                b.act_count = 0;
                return;
            }
            let v = out_from(cx, VF0, b.dirc[2], [0x40d5_5555, 0, 0, ONE]);
            let e = geom::vadd(eye_at, v);
            if cx.cam.reset == 5 {
                cx.out(Out::CamEye(e));
            }
        }
        2 => {
            b.unlock_player();
            killers_targets(x, cx, true);
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cam_back(cx);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `EntryKiller(3)` (0x003bdbc0): each killer eye that is out born again.
fn entry_killer(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let v = x.killer_view(b, cx);
    for p in &mut x.killers {
        if p.b.base.exit != 0 {
            eye::generate(p, &v, cx);
        }
    }
}

/// `OnThinkWave` (0x003bb460): at its target (none, pattern 2 -1) the
/// wave clip from its start at 512 (SE 175 note 46); by the body's clip,
/// SE 255 at 61, from 70 the quake (50) and the wave drawn, at 73 a flash
/// and a WaveShock, at 85 boss skill 59; both clips over, the next
/// pattern.
fn on_wave(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let pos = cx.pos();
    match b.act_proccess {
        0 => {
            if !pick(b, x, cx) {
                return;
            }
            super::restart(&mut x.anm_w, cx);
            x.anm_w.speed = 512;
            b.act_proccess += 1;
            cx.out(Out::Se3dNote { se: 175, pos, note: 46 });
        }
        1 => {
            let f = b.anm.frame();
            if f == 61 {
                cx.out(Out::Se3d { se: 255, pos });
            }
            if (f as u32) < 70 {
                return;
            }
            if f == 73 {
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                let d = b.dirc;
                b.effect_at(cx, EffKind::WaveShock, pos, d);
            } else if f == 85 {
                b.skill_damage(cx, None, Some(pos), 59);
            }
            cx.quake_vec([0x4248_0000; 3]);
            let ended = x.anm_w.forward();
            cx.out(Out::Corbenik(Pic::Wave { time: x.anm_w.posed, pos }));
            if ended && b.anm_status != 0 {
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkTeraLaser` (0x003bb750), six steps: 1500 past its target,
/// unseen, the party held, off the lists, the slaves paused; it shows as
/// the eyes and killer eyes fade (the killer eyes out at 60) and rises out
/// of sight; the camera high over the arena, the dark stage, the clump
/// off, the tera laser (`ccBossEffTeraLaser`) from 3000 over the origin
/// onto the target; over, all back where it was and the next pattern.
fn on_tera_laser(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            if !pick(b, x, cx) {
                return;
            }
            x.old_pos = cx.pos();
            let d = get_dirc(b.target_pos_p, b.center_pos_p);
            let pos = out_from(cx, b.target_pos, d, [0x44bb_8000, 0, 0, ONE]);
            let pp = w2p(cx, pos);
            cx.scene.chars[me].pos = pos;
            cx.scene.chars[me].pos_p = pp;
            b.transparency = 0;
            b.set_transparency = 0;
            let d2 = get_dirc(pp, b.target_pos_p);
            let mut view = pos;
            view[2] = ee::add(view[2], 0x4396_0000);
            let eye_pos = out_from(cx, b.target_pos, d2, [0x44bb_8000, 0, 0x4248_0000, ONE]);
            free_cam(cx, eye_pos, view);
            b.lock_player(cx, true);
            killers_targets(x, cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.body_hit_sw = 0;
            cx.out(Out::Cinema(Some(56)));
            x.pause = true;
            x.act_pos = VF0;
            x.act_pos[2] = 0x42c8_0000;
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            if c < 30 {
                return;
            }
            b.dirc[2] = b.target_dirc;
            if c >= 60 {
                for p in &mut x.eyes {
                    p.b.base.transparency = 0;
                    p.b.base.set_transparency = 0;
                }
                for p in &mut x.killers {
                    crate::fellow::delete_cmnd(cx.scene, p.me);
                    cx.out(Out::DeleteCmnd);
                    eye::on_exit(&mut p.b.base, cx);
                }
                b.act_count = 0;
                b.act_proccess += 1;
                return;
            }
            let mut t = ee::add(b.transparency, 0x3d08_8889);
            if !ee::le(t, ONE) {
                t = ONE;
            }
            b.transparency = t;
            b.set_transparency = t;
            for p in &mut x.eyes {
                let mut t = ee::sub(p.b.base.transparency, 0x3d08_8889);
                if ee::le(t, 0) {
                    t = 0;
                }
                p.b.base.transparency = t;
                p.b.base.set_transparency = t;
            }
            for p in &mut x.killers {
                let mut t = ee::sub(p.b.base.transparency, 0x3d08_8889);
                if ee::lt(t, 0) {
                    t = 0;
                }
                p.b.base.transparency = t;
                p.b.base.set_transparency = t;
            }
        }
        2 => {
            if c < 30 {
                return;
            }
            if c == 30 {
                cx.out(Out::Se { se: 223 });
            }
            let pos = geom::vadd(cx.pos(), x.act_pos);
            cx.scene.chars[me].pos = pos;
            x.act_pos[2] = ee::sub(x.act_pos[2], 0x4000_0000);
            if ee::lt(x.act_pos[2], 0) {
                b.act_count = 0;
                b.act_proccess += 1;
                return;
            }
            let mut view = pos;
            view[2] = ee::add(view[2], 0x4396_0000);
            if cx.cam.reset == 5 {
                cx.out(Out::CamLook(view));
            }
        }
        3 => {
            if c == 29 {
                let e = [0x4411_0000, 0xc4a9_6000, 0x4590_d800, ONE];
                let v = [0x43c9_0000, 0xc484_4000, 0x4585_b800, ONE];
                cx.out(Out::FreeCam { pos: e, view: v });
                x.stage_eff = b.begin_stage_effect(cx, DARK_IN, [15, 3000, 3000]);
                arena_clump(cx, false);
            } else if c == 30 {
                let mut pos = VF0;
                pos[2] = 0x453b_8000;
                cx.scene.chars[me].pos = pos;
                let pp = w2p(cx, pos);
                cx.scene.chars[me].pos_p = pp;
                let mut e = pp;
                e[2] = ee::add(e[2], 0x43fa_0000);
                let e = p2w(cx, e);
                let keys = [x.next_key(), x.next_key()];
                let scale = [0x40a0_0000, 0x40a0_0000, 0x3fc0_0000, ONE];
                let t = Tera::new(cx, e, VF0, scale, b.target_pos, keys);
                x.boss_eff = Some(make_fx(b, cx, EffKind::TeraLaser, Fx::Tera(Box::new(t)), e));
                b.dirc = VF0;
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        4 => {
            match c {
                293 | 283 | 278 | 275 | 266 | 258 | 245 => {
                    cx.out(Out::SeNote { se: 35, note: 53 });
                    cx.out(Out::SeNote { se: 56, note: 55 });
                }
                220 => cx.out(Out::Se { se: 56 }),
                179 => cx.out(Out::Se { se: 254 }),
                167 | 155 | 148 | 142 | 137 | 131 | 120 | 110 | 104 | 89 | 82 | 58 | 42 | 27 => {
                    cx.out(Out::Se { se: 68 })
                }
                0 => {
                    cx.out(Out::SeNote { se: 61, note: 67 });
                    cx.out(Out::SeNote { se: 246, note: 50 });
                    cx.out(Out::Se { se: 68 });
                }
                _ => {}
            }
            if x.boss_eff.is_some() && !b.effects.enabled(x.boss_eff) {
                arena_clump(cx, true);
                x.boss_eff = None;
                b.end_stage_effect(cx, x.stage_eff, DARK_OUT, 15);
                x.stage_eff = -1;
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        5 => {
            for p in &mut x.eyes {
                p.b.base.draw_sw = 1;
                p.b.base.transparency = ONE;
                p.b.base.set_transparency = ONE;
            }
            cx.scene.chars[me].pos = x.old_pos;
            b.body_hit_sw = 1;
            b.dirc = VF0;
            cam_back(cx);
            b.unlock_player();
            killers_targets(x, cx, true);
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::Cinema(None));
            x.pause = false;
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkSlaveAttack` (0x003bc430): the slaves paused; with the party up
/// the killer eye of the pattern's row (-1: one of `@1714` by `ccRand() %
/// 3`) that is on the lists and alive attacks (`Attack()`), else the next
/// pattern; the frame after, the next pattern.
fn on_slave_attack(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let mut k = x.attack_slave;
    match b.act_proccess {
        0 => {
            x.pause = true;
            if x.slv_atk_mode != 1 {
                return;
            }
            if cx.annihilated() || cx.game_over {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.act_proccess += 1;
        }
        1 => {
            if k == -1 {
                let n = cx.cc.rand() % 3;
                k = word(&cx.data.corbenik.slave_ids, n.abs());
            }
            let v = x.killer_view(b, cx);
            let mut found = false;
            for p in &mut x.killers {
                let ch = &cx.scene.chars[p.me];
                if cx.valid(Some(p.me)) && ch.cond[cond::DEAD] == 0 && i32::from(ch.id()) == k {
                    eye::attack(p, &v, cx);
                    found = true;
                    break;
                }
            }
            if found {
                b.act_proccess += 1;
            } else {
                x.pause = false;
                change_next_pattern(b, x, cx);
            }
        }
        2 if x.slv_atk_mode == 1 => {
            x.pause = false;
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkWander` (0x003bc6c0): round the centre at 15 (stopped while
/// stopped or held) for 300 frames, the next pattern after; with the party
/// gone, pattern 2 -1.
fn on_wander(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    b.move_spd = 0x4170_0000;
    if b.stop != 0 || cx.scene.chars[me].cond[cond::HOLD] != 0 {
        b.move_spd = 0;
        b.move_vector = VF0;
    }
    if cx.annihilated() || cx.game_over {
        exec_pattern(b, x, cx, 2, -1);
        return;
    }
    let dist = b.center_dist;
    let d = if !ee::le(dist, 0x447a_0000) {
        ee::sub(ee::add(HALF_PI, b.center_dirc), 0x3e32_b8c3)
    } else if ee::lt(dist, 0x43fa_0000) {
        let f = ee::mul(HALF_PI, ee::sub(ONE, ee::div(dist, 0x447a_0000)));
        ee::add(ee::add(HALF_PI, b.center_dirc), f)
    } else {
        ee::add(HALF_PI, b.center_dirc)
    };
    if !ee::le(d, PI) {
        b.move_dirc = ee::sub(b.move_dirc, TWO_PI);
    } else if ee::lt(d, NEG_PI) {
        b.move_dirc = ee::add(b.move_dirc, TWO_PI);
    }
    b.dirc[2] = geom::set_dirc(b.dirc[2], d, 256);
    b.move_dirc = b.dirc[2];
    if c >= 300 {
        change_next_pattern(b, x, cx);
    }
}

/// `OnThinkLaser` (0x003bc900), seven steps: the dark stage, the party
/// held, off the lists, the slaves paused; unseen, 1000 past the target,
/// the eyes flying out to their rings (`SetupRefrectLaserPos(50, 90)`) as
/// they show; the charge at the dummy, a flash and the body hidden; `16 /
/// n` reflected lasers at each of the n members alive; once all are over
/// boss skill 58 at the target and the stage back; 15 frames on, all back.
fn on_laser(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            if cx.annihilated() || cx.game_over {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            select(b, cx, 0);
            b.calc_target_info(cx);
            x.stage_eff = b.begin_stage_effect(cx, DARK_IN, [15, 3000, 3000]);
            b.lock_player(cx, false);
            killers_targets(x, cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::Cinema(Some(58)));
            x.act_pos = cx.pos();
            x.pause = true;
            let d = get_dirc(b.target_pos_p, b.center_pos_p);
            let mut view = b.target_pos;
            view[2] = ee::add(view[2], 0x4348_0000);
            let eye_pos = out_from(cx, b.target_pos, d, [0x447a_0000, 0, 0x4248_0000, ONE]);
            free_cam(cx, eye_pos, view);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            if c == 0 {
                b.transparency = 0;
                b.set_transparency = 0;
                for p in &mut x.eyes {
                    p.b.base.transparency = 0;
                    p.b.base.set_transparency = 0;
                }
                for p in &mut x.killers {
                    p.b.base.draw_sw = 0;
                }
                let d = get_dirc(b.center_pos_p, b.target_pos_p);
                let pos = out_from(cx, b.target_pos, d, [0x447a_0000, 0, 0, ONE]);
                cx.scene.chars[me].pos = pos;
                cx.scene.chars[me].pos_p = w2p(cx, pos);
                b.calc_target_info(cx);
                setup_refrect_laser_pos(b, x, cx, 0x4248_0000, 90);
                return;
            }
            if c < 31 {
                if c == 15 {
                    arena_clump(cx, false);
                } else if c == 30 {
                    cx.out(Out::Se { se: 229 });
                }
                for p in &mut x.eyes {
                    let mut t = ee::add(p.b.base.transparency, 0x3d08_8889);
                    if !ee::le(t, ONE) {
                        t = ONE;
                    }
                    p.b.base.transparency = t;
                    p.b.base.set_transparency = t;
                }
                return;
            }
            if c < 121 {
                if c == 60 {
                    let mut view = b.target_pos;
                    view[2] = ee::add(view[2], 0x4396_0000);
                    let e = out_from(cx, b.target_pos, b.target_dirc, [0x453b_8000, 0, 0x447a_0000, ONE]);
                    cx.out(Out::FreeCam { pos: e, view });
                }
                calc_tera_laser_pos(x, cx);
                if c < 90 {
                    return;
                }
                let mut t = ee::add(b.transparency, 0x3d08_8889);
                if !ee::le(t, ONE) {
                    t = ONE;
                }
                b.transparency = t;
                b.set_transparency = t;
                b.dirc[2] = b.target_dirc;
                return;
            }
            if c == 121 {
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        2 => {
            let at = eye::dummy_at(&x.body_world, x.dummy);
            let key = x.next_key();
            x.gen_energy = Some(key);
            cx.out(Out::Corbenik(Pic::Gen { key, which: Gen::LaserCharge, pos: at }));
            cx.out(Out::SeNote { se: 246, note: 65 });
            b.act_count = 0;
            b.act_proccess += 1;
        }
        3 => {
            if c != 60 {
                return;
            }
            if let Some(key) = x.gen_energy {
                cx.out(Out::Corbenik(Pic::GenKill { key }));
            }
            cx.out(Out::Flash { t: 10, colour: 0x80e0_ffff });
            b.draw_sw = 0;
            b.act_proccess += 1;
        }
        4 => {
            let from = eye::dummy_at(&x.body_world, x.dummy);
            cx.out(Out::Se { se: 61 });
            let members = cx.party.members;
            let n = members.iter().flatten().filter(|&&m| cx.scene.chars[m].cond[cond::DEAD] == 0).count() as i32;
            // `16 / n`; with none alive the EE's quotient (-1) makes none.
            let each = if n > 0 { 16 / n } else { -1 };
            x.eff_laser = [None; 16];
            let mut i = 0;
            for m in members.into_iter().flatten() {
                if cx.scene.chars[m].cond[cond::DEAD] != 0 {
                    continue;
                }
                for _ in 0..each.max(0) {
                    let eyes = calc_laser_turning_points(cx);
                    let end = cx.scene.chars[m].pos;
                    let pts: Vec<V4> = eyes.iter().map(|&k| cx.scene.chars[x.eye_me[k]].pos).collect();
                    let r = Refrect::new(cx, from, end, eyes, &pts);
                    let id = make_fx(b, cx, EffKind::RefrectionLaser, Fx::Refrect(Box::new(r)), from);
                    if let Some(s) = x.eff_laser.get_mut(i) {
                        *s = (id >= 0).then_some(id);
                    }
                    i += 1;
                }
            }
            b.act_proccess += 1;
        }
        5 => {
            if matches!(c, 117 | 103 | 92 | 79 | 72) {
                cx.out(Out::SeNote { se: 61, note: 70 });
            }
            let mut all = true;
            for s in &mut x.eff_laser {
                if s.is_some() && b.effects.enabled(*s) {
                    all = false;
                } else {
                    *s = None;
                }
            }
            if !all {
                return;
            }
            let tp = b.target_pos;
            b.skill_damage(cx, None, Some(tp), 58);
            b.end_stage_effect(cx, x.stage_eff, DARK_OUT, 15);
            x.stage_eff = -1;
            arena_clump(cx, true);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        6 => {
            if c != 15 {
                return;
            }
            for p in &mut x.killers {
                p.b.base.draw_sw = 1;
            }
            reset_refrect_laser_pos(x, cx);
            cx.scene.chars[me].pos = x.act_pos;
            b.draw_sw = 1;
            b.unlock_player();
            cam_back(cx);
            killers_targets(x, cx, true);
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            x.pause = false;
            cx.out(Out::Cinema(None));
            change_action(b, x, cx, act::NEUTRAL, 1, false);
        }
        _ => {}
    }
}

/// `CalcTeraLaserPos` (0x003bdc50): each eye still under way steps its
/// speed toward its ring (the speed its step less each frame, never under
/// 0).
fn calc_tera_laser_pos(x: &mut Third, cx: &mut Cx) {
    for p in &mut x.eyes {
        let e = &mut p.x;
        if ee::le(e.speed, 0) {
            continue;
        }
        let mut s = ee::add(e.speed, e.step);
        e.speed = s;
        if ee::lt(s, 0) {
            s = 0;
            e.speed = 0;
        }
        let a = w2p(cx, e.from);
        let t = w2p(cx, e.to);
        let d = crate::boss::dist(cx, a, t);
        let rx = piney_data::libm::atan2f(ee::sub(a[2], t[2]), d);
        let rz = get_dirc(a, t);
        let m = geom::rot_matrix_y(&geom::unit_matrix(), rx);
        let m = geom::rot_matrix_z(&m, rz);
        let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
        let v = geom::apply_matrix(&m, [s, 0, 0, ONE]);
        let me = p.me;
        let pp = geom::vadd(cx.scene.chars[me].pos_p, v);
        cx.scene.chars[me].pos_p = pp;
        cx.scene.chars[me].pos = p2w(cx, pp);
    }
}

/// `SetupRefrectLaserPos(v0, n)` (0x003bdf00): each eye's place kept, its
/// ring point round the target (three at `@2016[0]` out and `@2017[1]`
/// up, five at `[1]`, `[2]`, eight at `[2]`, `[3]`, from a quarter turn
/// off the way back from the target), all set off from 1500 behind the
/// target and 200 up at `v0` slowing to arrive in `n` frames.
fn setup_refrect_laser_pos(b: &BossBase, x: &mut Third, cx: &mut Cx, v0: F, n: i32) {
    let radii = cx.data.corbenik.refrect_radii.clone();
    let heights = cx.data.corbenik.refrect_heights.clone();
    let r = |k: usize| radii.get(k).copied().unwrap_or(0);
    let h = |k: usize| heights.get(k).copied().unwrap_or(0);
    for p in &mut x.eyes {
        p.x.act_pos = cx.scene.chars[p.me].pos;
    }
    let tp = w2p(cx, b.target_pos);
    let mut back = ee::sub(b.target_dirc, PI);
    if ee::lt(back, NEG_PI) {
        back = ee::add(back, TWO_PI);
    }
    let start = |back: F| {
        let a = ee::sub(back, HALF_PI);
        if ee::lt(a, NEG_PI) { ee::add(a, TWO_PI) } else { a }
    };
    let rings: [(std::ops::Range<usize>, F, F, F); 3] =
        [(0..3, r(0), h(1), HALF_PI), (3..8, r(1), h(2), 0x3f49_0fdb), (8..16, r(2), h(3), 0x3ee5_c8fa)];
    for (ks, rad, z, step) in rings {
        let mut a = start(back);
        for k in ks {
            let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), a), NEG_HALF_PI);
            let o = geom::apply_matrix(&m, [rad, 0, z, ONE]);
            x.eyes[k].x.to = p2w(cx, geom::vadd(tp, o));
            a = ee::add(a, step);
            if !ee::le(a, PI) {
                a = ee::sub(a, TWO_PI);
            }
        }
    }
    let from = out_from(cx, b.target_pos, b.target_dirc, [0xc4bb_8000, 0, 0x4348_0000, ONE]);
    for p in &mut x.eyes {
        cx.scene.chars[p.me].pos = from;
        cx.scene.chars[p.me].pos_p = w2p(cx, from);
        p.x.from = from;
    }
    let nf = ee::from_int(n);
    for p in &mut x.eyes {
        let to = w2p(cx, p.x.to);
        let pp = cx.scene.chars[p.me].pos_p;
        let d = geom::length_on(cx.actx.volume, geom::vsub(pp, to));
        p.x.speed = v0;
        p.x.step = ee::mul(0x4000_0000, ee::div(ee::sub(d, ee::mul(v0, nf)), ee::mul(nf, nf)));
    }
}

/// `ResetRefrectLaserPos` (0x003be760): meant to put each eye back where
/// it was; its loop never steps on from the first eye, which it puts back
/// sixteen times.
fn reset_refrect_laser_pos(x: &mut Third, cx: &mut Cx) {
    if let Some(p) = x.eyes.first() {
        cx.scene.chars[p.me].pos = p.x.act_pos;
    }
}

/// `CalcLaserTurningPoints` (0x003be7c0): 3 to 5 eyes (`abs(ccRand()) % 3
/// + 3`), each by `ccRand() % 16`, the first free one from 0 when taken.
fn calc_laser_turning_points(cx: &mut Cx) -> Vec<usize> {
    let n = cx.cc.rand().abs() % 3 + 3;
    let mut used = [false; EYES];
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let r = cx.cc.rand() % 16;
        let mut k = r.unsigned_abs() as usize;
        if used[k] {
            k = (0..EYES).find(|&j| !used[j]).unwrap_or(EYES - 1);
        }
        used[k] = true;
        out.push(k);
    }
    out
}

// --- Affect ---------------------------------------------------------------------------------------

/// `ccBoss08_c::Affect` (0x003b9810): the base's (through its own
/// `ChangeAction`), then for a blow (1, 3): at half its HP the first time
/// stream 88 and the normal table; at 0 the death (14); a hit's note
/// (`ccSeOn3DNote(195, pos, abs(ccRand() % 5) + 60)`).
pub(crate) fn affect(b: &mut BossBase, x: &mut Third, cx: &mut Cx) {
    let found = take_parts(cx, x);
    b.base_affect_with(cx, &mut |b, cx, n, forbid, af| change_action(b, x, cx, n, forbid, af));
    let me = cx.me;
    let t = cx.scene.chars[me].affect.ty;
    if t == 1 || t == 3 {
        let hp = cx.scene.chars[me].hp;
        let mhp = cx.scene.chars[me].max_hp;
        if b.exit == 0 && !x.half && hp <= mhp / 2 {
            cx.out(Out::StreamMenu { stream: 88, mask: 0 });
            x.half = true;
            set_pattern_mode(b, x, cx, 1);
        }
        if b.exit == 0 && cx.scene.chars[me].hp <= 0 {
            change_action(b, x, cx, act::DIE, 2, true);
        }
        if cx.scene.chars[me].affect.param[0] > 0 {
            let n = (cx.cc.rand() % 5).abs() + 60;
            let pos = cx.pos();
            cx.out(Out::Se3dNote { se: 195, pos, note: n as u8 });
        }
    }
    put_parts(cx, x, found);
}
