//! Kyvia's first fight (`ccBossKyvia01`, kyvia01.cpp; MUT gcmn 0x004d2960-
//! 0x004d93e8), code 12 of `bossFunc`, which Mutation's event 108 enters
//! on the disc of field 9 (`EVENTAREAB8`). The body stands off the disc
//! and strikes; its core (`kyviaCore`, [`core`]) is the fight's target,
//! with five gomoras (`kyviaGomora`, [`gomora`]) about it. Each part is a
//! `ccBoss` of its own character, taken out of it for the body's frame (a
//! [`Tree`]). Names and layouts are Infection's DWARF; the rules are
//! Mutation's code. docs/engine/boss-kyvia.md.

pub mod core;
pub mod gomora;

use piney_data::field::ee;

use self::core::Core;
use self::gomora::Gomora;
use super::{Anm, Boss, Class, Cx, EffKind, Out, VF0};
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, M4, V4};
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_PI: F = 0xc049_0fdb;

/// `bossFunc`'s code for this fight.
pub const CODE: i32 = 12;
/// `bossTbl` rows: the body, the core by level and attribute.
pub const ROW: usize = 12;

/// The body's acts (`actNum`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    /// Struck while the core is: its smoke and the camera (`CheckDmgSmokeEff`).
    pub const DAMAGE: i16 = 1;
    pub const DAMAGE2: i16 = 2;
    pub const HAND: i16 = 3;
    pub const LIGHT: i16 = 4;
    pub const MEGID: i16 = 5;
    pub const DEAD: i16 = 14;
}

/// Kyvia's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KyviaData {
    /// `Kyvia01AnmTbl`, `kyviaCoreAnmTbl`, `kyviaGomoraAnmTbl`.
    pub anims: Vec<Option<String>>,
    pub core_anims: Vec<Option<String>>,
    pub gomora_anims: Vec<Option<String>>,
    /// `AllGomoraList_1[0]`: each list the gomoras' attributes by slave.
    pub gomora_lists: Vec<[i16; 5]>,
    /// `Skill_VARIOUS`, `Skill_DOWNER`.
    pub various: Vec<i16>,
    pub downer: Vec<i16>,
}

impl KyviaData {
    /// The volume's.
    pub fn of(volume: piney_data::volume::Volume) -> KyviaData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        KyviaData {
            anims: s(t.kyvia01_anims()),
            core_anims: s(t.kyvia_core_anims()),
            gomora_anims: s(t.kyvia_gomora_anims()),
            gomora_lists: t
                .kyvia_gomora_lists()
                .iter()
                .map(|l| std::array::from_fn(|k| l.get(k).copied().unwrap_or(4)))
                .collect(),
            various: t.kyvia_various_skills().to_vec(),
            downer: t.kyvia_downer_skills().to_vec(),
        }
    }
}

/// The particle generators of the fight, by table and row (0x38 bytes a
/// row): `KyviaGenerator` (MUT gcmn 0x0061f650), `KyviaDmgGenerator`
/// (0x0061fce0), `CoreGenerator`, `GomoraGenerator` (0x0061eec0),
/// `GomoraAuraGenerator` (0x0061f1b0), `MissileSmokeGenerator`,
/// `BurstSmokeGenerator`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gen {
    Kyvia(u8),
    KyviaDmg(u8),
    Core(u8),
    Gomora(u8),
    /// By row, and the force field (the aura's level) for rows 0-3.
    GomoraAura(u8, u8),
    MissileSmoke(u8),
    BurstSmoke(u8),
}

/// `bossBlur` (the effect manager's +4) as the fight sets it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Blur {
    /// +0x24 `m_enabled`, +0x28 `m_exit`, +0x0c `m_scale`, +0x1c `m_abgr`.
    pub enabled: bool,
    pub exit: bool,
    pub scale: F,
    pub abgr: u32,
}

/// `ccBossKyvia01`'s own members (+0x29350 on).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kyvia {
    /// `anmw`: `ANM_ex0batc0`, the arm `HandAtk` swings.
    pub anm_w: Anm,
    /// `DiscPos`: the disc (`discPrevPos`) as the last frame read it.
    pub disc_pos: V4,
    pub disc_lv: i16,
    pub disc_max_lv: i16,
    /// `dummypos`: `DMY_marker01`, where the body stands.
    pub dummy_pos: V4,
    /// `tesPOS`, `tesDIRC`: where the arm is drawn.
    pub tes_pos: V4,
    pub tes_dirc: V4,
    pub snd_pos_l: V4,
    pub snd_pos_r: V4,
    pub atk_pat_mode: i32,
    pub l_offset: V4,
    pub l_point: [V4; 6],
    pub l_char: [V4; 3],
    pub l_bpos: [V4; 3],
    pub l_bpos_sub: [V4; 3],
    pub btime: F,
    pub add_time: F,
    pub sub_btime: F,
    pub max_hp: i32,
    pub switch_hp: i32,
    pub hp_proccess: i32,
    pub dmg_anm_flg: i32,
    pub ded_dmg_vec: V4,
    /// `DmgGp[k]` and `DeadGp` made.
    pub dmg_gp: [bool; 4],
    pub dead_gp: bool,
    pub cam_rot_x: F,
    pub move_transfer: V4,
    /// `Z`: the camera's turn about the body.
    pub z: F,
    pub atk_mat: M4,
    pub atk_vec: V4,
    pub quake_vector: V4,
    pub time_mode: i32,
    pub s_point: V4,
    /// `SlaveCore`: the core's character.
    pub core: usize,
    pub st_flg: u8,
    pub af_cou: i32,
    pub af_timing: i32,
    pub default_argb: u32,
    pub ex_argb: u32,
    pub wait_disc_count: i16,
    pub live_flg: u8,
    pub now_point: i16,
    pub max_point: i16,
    pub disc_ex_flg: u8,
    /// +0x295f8: the cinema is on (turned off with the player unlocked).
    pub cinema: u8,
    pub blur: Blur,
    /// `bossCam` +0xe0, the pitch, as the body last set it.
    pub cam_pitch: F,
}

/// One of the fight's parts as the body's frame holds it: its `ccBoss`, its
/// own members and its character.
#[derive(Debug)]
pub struct Part<T> {
    pub b: Box<Boss>,
    pub x: Box<T>,
    pub me: usize,
}

/// The core and its gomoras, out of their characters for the frame.
#[derive(Debug, Default)]
pub struct Tree {
    pub core: Option<Part<Core>>,
    pub gomoras: Vec<Part<Gomora>>,
}

/// `cx.me` set to `me` for `f`.
pub(crate) fn as_me<R>(cx: &mut Cx, me: usize, f: impl FnOnce(&mut Cx) -> R) -> R {
    let old = cx.me;
    cx.me = me;
    let r = f(cx);
    cx.me = old;
    r
}

pub(crate) fn dp(x: F) -> f64 {
    f64::from(f32::from_bits(x))
}

/// `dptofp`: a double rounded to the nearest float.
pub(crate) fn fp(d: f64) -> F {
    (d as f32).to_bits()
}

/// `fabs` of a float through `fptodp` and back: the float without its sign.
pub(crate) fn fabs(x: F) -> F {
    x & 0x7fff_ffff
}

/// `ccRandF(x)` without its sign.
pub(crate) fn rand_abs(cx: &mut Cx, x: F) -> F {
    fabs(rand_f(cx.cc, x))
}

/// An angle into -pi..pi as the game does it, once each way.
pub(crate) fn wrap(mut v: F) -> F {
    if !ee::le(v, PI) {
        v = ee::sub(v, TWO_PI);
    }
    if ee::lt(v, NEG_PI) {
        v = ee::add(v, TWO_PI);
    }
    v
}

fn kyvia_of(b: &mut Boss) -> Box<Kyvia> {
    match std::mem::replace(&mut b.class, Class::Plain) {
        Class::Kyvia(k) => k,
        other => {
            b.class = other;
            Box::default()
        }
    }
}

impl Tree {
    /// The core and its gomoras taken out of their characters.
    pub fn take(cx: &mut Cx, core_me: usize) -> Tree {
        let mut t = Tree::default();
        let Some(mut cb) = cx.scene.chars.get_mut(core_me).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take())
        else {
            return t;
        };
        let Class::KyviaCore(cxs) = std::mem::replace(&mut cb.class, Class::Plain) else {
            if let Some(f) = cx.scene.chars[core_me].foe_state_mut() {
                f.boss = Some(cb);
            }
            return t;
        };
        for &g in &cxs.gomoras {
            let Some(mut gb) = cx.scene.chars.get_mut(g).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take())
            else {
                continue;
            };
            match std::mem::replace(&mut gb.class, Class::Plain) {
                Class::Gomora(gx) => t.gomoras.push(Part { b: gb, x: gx, me: g }),
                other => {
                    gb.class = other;
                    if let Some(f) = cx.scene.chars[g].foe_state_mut() {
                        f.boss = Some(gb);
                    }
                }
            }
        }
        t.core = Some(Part { b: cb, x: cxs, me: core_me });
        t
    }

    /// Everything put back.
    pub fn put(self, cx: &mut Cx) {
        for mut g in self.gomoras {
            g.b.class = Class::Gomora(g.x);
            if let Some(f) = cx.scene.chars[g.me].foe_state_mut() {
                f.boss = Some(g.b);
            }
        }
        if let Some(mut c) = self.core {
            c.b.class = Class::KyviaCore(c.x);
            if let Some(f) = cx.scene.chars[c.me].foe_state_mut() {
                f.boss = Some(c.b);
            }
        }
    }
}

/// `ccChar::SetBaseParam(ccGetBossParam(row), &param)` (MUT gcmn
/// 0x00591090): the row's base and stats, HP and SP full, the gauge
/// empty, the conditions cleared and alive.
pub(crate) fn set_base_param(cx: &mut Cx, who: usize, row: usize) {
    let Some(r) = cx.t.bosses.get(row).cloned() else { return };
    let ch = &mut cx.scene.chars[who];
    let (hp, sp) = (r.max_hp, r.max_sp);
    if let Some(f) = ch.foe_state_mut() {
        f.real = r.elm;
        f.temp = Default::default();
        f.time = Default::default();
        f.pp = 0;
        f.pp_count = 0;
        f.pp_restore = 0;
        f.row = r;
    }
    ch.max_hp = hp;
    ch.max_sp = sp;
    ch.hp = hp;
    ch.sp = sp;
    crate::affect::clear_conditions(ch);
    ch.cond[cond::DEAD] = 0;
}

/// A part's character: a boss of `row`, no condition, its affects its
/// class's, on no command list.
pub(crate) fn new_char(cx: &mut Cx, row: usize) -> usize {
    let r = cx.t.bosses.get(row).cloned().unwrap_or_default();
    let mut ch = crate::chara::Char::foe(r);
    ch.condition_num = -1;
    ch.affect.func = crate::chara::AffectFunc::Boss;
    cx.scene.add(ch, 3)
}

/// A `ccBoss` as its constructor (MUT gcmn 0x00471690) leaves it: exited,
/// not drawn, no body hit or cheat HP.
pub(crate) fn plain_boss() -> Boss {
    Boss {
        class: Class::Plain,
        wait_pat_num: -2,
        pat_num: 13,
        stage_eff_id: -1,
        exit: 1,
        transparency: ONE,
        set_transparency: ONE,
        anm: Anm::new(),
        anm_wave: Anm::new(),
        ..Boss::default()
    }
}

/// The head of `ccBoss::Main` as the core's and the gomoras' `Main`s have
/// it: `CalcReal`, the target and the centre, the party held while
/// `lockPlayer` (and the menu's forbid), then `stop`'s count.
pub(crate) fn head(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    crate::chara::calc_real(cx.t, &mut cx.scene.chars[me], 0, cx.env, &mut cx.ev);
    b.calc_target_info(cx);
    b.center_pos_p = super::w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = crate::enemy_ai::get_dist(pp, b.center_pos_p);
    b.center_dirc = get_dirc(pp, b.center_pos_p);
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
}

/// The affect the core and the gomoras share (MUT gcmn 0x004f99c0,
/// 0x004ff480): `ccBoss::Affect`'s kinds without its checks on the party.
/// True for a hit (kinds 1 and 3), whose resistance the core then shows.
pub(crate) fn part_affect(b: &mut Boss, cx: &mut Cx) -> bool {
    let me = cx.me;
    let (t, p0, person) = {
        let a = &cx.scene.chars[me].affect;
        (a.ty, a.param[0], a.person)
    };
    let hp = cx.scene.chars[me].hp;
    let mhp = cx.scene.chars[me].max_hp;
    match t {
        6 => b.stop = 0,
        5 => b.stop = 1,
        21 => {
            let v = mhp / 10;
            let ch = &mut cx.scene.chars[me];
            ch.hp = v;
            ch.max_hp = v;
            if let Some(f) = ch.foe_state_mut() {
                f.pp = -1;
                f.pp_count = 0;
            }
        }
        13 => {
            b.change_action(cx, 11, 2, true);
            cx.scene.chars[me].affect.ty = 0;
        }
        7 | 9 => {
            cx.out(Out::FlyFont { kind: 20, n: i32::from(p0) });
            cx.scene.chars[me].hp = hp.wrapping_add(p0).min(mhp);
        }
        1 | 3 => {
            cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
            if p0 >= 0 {
                cx.out(Out::HitMark { by: person });
                let mut h = if b.cheat_hp != 0 { hp.wrapping_sub(p0 / 10).max(mhp / 2) } else { hp.wrapping_sub(p0) };
                if h <= 0 {
                    h = 0;
                    crate::affect::clear_conditions(&mut cx.scene.chars[me]);
                    b.change_action(cx, 14, 2, true);
                } else if t == 1 {
                    let n = if cx.env.count & 1 != 0 { 1 } else { 2 };
                    b.change_action(cx, n, 0, true);
                }
                cx.scene.chars[me].hp = h;
            }
            return true;
        }
        _ => {}
    }
    false
}

// --- the constructor ---------------------------------------------------------------------

/// `ccBossKyvia01::ccBossKyvia01(1)` (MUT gcmn 0x004d2a80) with its core and
/// the core's five gomoras (`kyviaCore::CoreInit(1)`, `GomoraInit(1)`),
/// whose characters it adds to the scene with their bosses. The body is
/// `scene.chars[cx.me]` (`bossTbl` row 12); it stands at `DMY_marker01`
/// of the disc's file (`cx.disc.marker`).
pub fn new(cx: &mut Cx) -> Boss {
    let me = cx.me;
    let mut b = plain_boss();
    let mut x = Kyvia {
        st_flg: 1,
        time_mode: 1,
        now_point: 1,
        max_point: 1,
        live_flg: 1,
        atk_pat_mode: 1,
        disc_lv: 1,
        disc_max_lv: 1,
        disc_ex_flg: 1,
        af_timing: 2,
        anm_w: Anm::new(),
        atk_mat: geom::unit_matrix(),
        ..Kyvia::default()
    };
    set_base_param(cx, me, ROW);
    b.anm_tbl = cx.data.kyvia.anims.clone();
    // OffExit, OnDraw, OnBodyHit, OnCheatHP.
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    b.cheat_hp = 1;
    b.dirc = VF0;
    b.anm.set("ANM_ex01nut0", cx.clips);
    b.change_action(cx, act::NEUTRAL, 0, true);
    x.dummy_pos = cx.disc.marker;
    cx.scene.chars[me].pos = x.dummy_pos;
    // InitBossCamera(450, 1050); the pitch 0.26, its limit 0.6, the sway.
    x.cam_rot_x = 0x3e85_1eb8;
    x.cam_pitch = x.cam_rot_x;
    cx.out(Out::CamPitch { add: false, v: x.cam_pitch });
    cx.out(Out::CamRotXLimit(0x3f19_999a));
    cx.out(Out::CamSway(true));
    x.anm_w.set("ANM_ex0batc0", cx.clips);
    // InitStageEffect: the stage fader.
    b.use_stage_eff = true;
    x.default_argb = 0x3080_8080;
    x.ex_argb = 0x6080_8080;
    x.blur = Blur { enabled: true, exit: false, scale: ONE, abgr: x.default_argb };
    // The core, level 1.
    let core_me = new_char(cx, 32);
    x.core = core_me;
    let cb = self::core::new(cx, core_me, me, 1);
    if let Some(f) = cx.scene.chars[core_me].foe_state_mut() {
        f.boss = Some(Box::new(cb));
    }
    x.max_hp = i32::from(cx.scene.chars[core_me].max_hp);
    x.switch_hp = x.max_hp / 4;
    x.hp_proccess = 0;
    x.quake_vector = [0x4120_0000, 0x4120_0000, 0x4120_0000, 0];
    b.class = Class::Kyvia(Box::new(x));
    b
}

// --- the frame ---------------------------------------------------------------------------

/// `ccThBossEffect`'s pass and `ccBossKyvia01::Main` (MUT gcmn 0x004d4410).
pub(super) fn main(b: &mut Boss, cx: &mut Cx) {
    let mut x = kyvia_of(b);
    let mut t = Tree::take(cx, x.core);
    manager_pass(b, &mut x, &mut t, cx);
    if b.exit == 0 {
        frame(b, &mut x, &mut t, cx);
    }
    t.put(cx);
    b.class = Class::Kyvia(x);
}

fn frame(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    crate::chara::calc_real(cx.t, &mut cx.scene.chars[me], 0, cx.env, &mut cx.ev);
    b.calc_target_info(cx);
    b.center_pos_p = super::w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = crate::enemy_ai::get_dist(pp, b.center_pos_p);
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
    think(b, x, t, cx);
    action(b, x, t, cx);
    b.move_(cx);
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
        if x.cinema != 0 {
            cx.out(Out::Cinema(None));
            x.cinema = 0;
        }
    }
    if b.draw_sw != 0 {
        b.anm_status = i8::from(b.anm.forward());
    }
    // Slave(): the core's Main.
    if let Some(c) = t.core.as_mut() {
        self::core::main(c, &mut t.gomoras, cx);
    }
}

/// `ccBossKyvia01::Think` (MUT gcmn 0x004d47c0).
fn think(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    x.af_cou += 1;
    if x.af_timing < x.af_cou {
        x.af_cou = 0;
    }
    check_disc_move(b, x, t, cx);
    match b.act_num {
        act::NEUTRAL => switch_action_pattern(b, x, t, cx),
        1..=5 | 11..=14 => {}
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// `SwitchActionPattern` (MUT gcmn 0x004d48a0): the core's attack turn
/// (`kAtkFlg` 2, in its state 0) begins the body's next attack in turn.
fn switch_action_pattern(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    switch_dead_check(b, x, t, cx);
    let Some(c) = t.core.as_mut() else { return };
    if c.x.k_atk_flg != 2 || c.x.core_state != 0 {
        return;
    }
    x.cinema = 1;
    match x.atk_pat_mode {
        1 => {
            cx.out(Out::Cinema(Some(33)));
            b.change_action(cx, act::HAND, 0, true);
            x.atk_pat_mode += 1;
        }
        2 => {
            cx.out(Out::Cinema(Some(32)));
            b.change_action(cx, act::LIGHT, 0, true);
            x.atk_pat_mode += 1;
        }
        3 => {
            cx.out(Out::Cinema(Some(34)));
            b.change_action(cx, act::MEGID, 0, true);
            x.atk_pat_mode = 1;
        }
        _ => {}
    }
    c.x.k_atk_flg = 0;
}

/// `SwitchDeadCheck` (MUT gcmn 0x004d4a80): the core's flags read into the
/// body's acts, and the body's death once the core is gone.
fn switch_dead_check(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let flg = t.core.as_ref().map_or(0, |c| c.x.k_atk_flg);
    match flg {
        2 => {
            check_dmg_smoke_eff(x, t, cx);
        }
        1 => {
            b.change_action(cx, act::DAMAGE, 0, true);
            if let Some(c) = t.core.as_mut() {
                c.x.k_atk_flg = 3;
            }
        }
        3 => {
            if let Some(c) = t.core.as_mut() {
                c.x.k_atk_flg = 2;
            }
        }
        _ => {
            if t.core.as_ref().is_some_and(|c| c.x.k_dmg_flg == 4) {
                // `actNum != 2 || actNum != 1`: always.
                b.change_action(cx, act::DAMAGE2, 0, true);
                if let Some(c) = t.core.as_mut() {
                    c.x.k_dmg_flg = 0;
                }
            }
        }
    }
    if x.live_flg == 0 && !cx.annihilated() && !cx.game_over {
        cx.out(Out::ClearSpcCondition);
        b.change_action(cx, act::DEAD, 2, true);
    }
}

/// `CheckDiscMove` (MUT gcmn 0x004d4c10): the disc's ride (the camera on
/// the body), its stop and the core's entry; after the core's death the
/// next stage, or the body's end.
fn check_disc_move(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let _ = b;
    x.disc_pos = cx.disc.prev_pos;
    let me = cx.me;
    if x.st_flg == 1 {
        if !cx.disc.moving {
            if x.wait_disc_count == 0 {
                x.blur = Blur { enabled: true, exit: false, scale: 0x3f86_6666, abgr: x.ex_argb };
                x.quake_vector = [0x41a0_0000, 0x41a0_0000, 0x41a0_0000, x.quake_vector[3]];
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 56, pos, note: 60 });
                cx.out(Out::CamMode { mode: 6 });
                cx.out(Out::CamInitLock);
                x.disc_ex_flg = 1;
            }
            x.wait_disc_count += 1;
            if x.wait_disc_count < 21 {
                let q = x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
                x.blur.scale = 0x3f83_d70a;
            }
            if x.wait_disc_count == 21 {
                x.blur.abgr = x.default_argb;
            }
            if x.wait_disc_count == 35 {
                x.blur.scale = 0x3f81_47ae;
            }
            if x.wait_disc_count == 50 {
                x.st_flg = 0;
                x.wait_disc_count = 0;
                entry_slave(x, t, cx, 1);
                x.blur.scale = ONE;
            }
            return;
        }
        if x.disc_ex_flg != 0 {
            cx.out(Out::CamMode { mode: 5 });
            x.disc_ex_flg = 0;
        }
        // The camera 1500 off and 500 up, turned to the body from the
        // disc, looking 150 over the disc.
        let d = get_dirc(x.disc_pos, cx.scene.chars[me].pos);
        let m = geom::rot_matrix_z(&geom::unit_matrix(), d);
        let eye = geom::vadd(geom::apply_matrix(&m, [0, 0x44bb_8000, 0x43fa_0000, ONE]), x.disc_pos);
        let view = geom::vadd([0, 0, 0x4316_0000, ONE], x.disc_pos);
        cx.out(Out::FreeCam { pos: eye, view });
        if b.act_num == act::NEUTRAL && matches!(b.anm.frame(), 20 | 60) {
            cx.se3d(232);
        }
        return;
    }
    if !t.core.as_ref().is_some_and(|c| c.x.dead_flg != 0) {
        return;
    }
    if x.disc_lv < x.disc_max_lv {
        x.wait_disc_count += 1;
        match x.wait_disc_count {
            60 => {
                x.st_flg = 1;
                x.disc_lv += 1;
                x.wait_disc_count = 0;
                cx.out(Out::DiscNextStage);
                // m_enabled 0: the blur ends at its present values.
                x.blur.enabled = false;
            }
            41 => {
                x.blur.enabled = true;
                x.blur.exit = false;
            }
            1 => {
                x.quake_vector = [0x41a0_0000, 0x41a0_0000, 0x41a0_0000, x.quake_vector[3]];
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 56, pos, note: 60 });
            }
            n if n >= 41 => {
                let q = x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
            }
            _ => {}
        }
    } else if x.live_flg != 0 {
        x.live_flg = 0;
    }
}

/// `CheckDmgSmokeEff` (MUT gcmn 0x004d5140): each quarter of the core's HP
/// lost puts a smoke on the body, 500 up and up to 300 off; -1 when none is
/// due, else the next quarter.
fn check_dmg_smoke_eff(x: &mut Kyvia, t: &Tree, cx: &mut Cx) -> i32 {
    let core_hp = t.core.as_ref().map_or(0, |c| i32::from(cx.scene.chars[c.me].hp));
    let k = x.hp_proccess;
    if core_hp > x.max_hp - x.switch_hp * (k + 1) {
        return -1;
    }
    if x.dmg_gp.get(k as usize).copied().unwrap_or(true) {
        return -1;
    }
    let mut v = VF0;
    let r = rand_f(cx.cc, 0x4396_0000);
    v[1] = fabs(r) ^ 0x8000_0000;
    let turn = rand_abs(cx, PI);
    let m = geom::rot_matrix_z(&geom::unit_matrix(), turn);
    v = geom::apply_matrix(&m, v);
    v[2] = ee::add(v[2], 0x43fa_0000);
    let pos = geom::vadd(v, cx.scene.chars[cx.me].pos);
    x.dmg_gp[k as usize] = true;
    cx.out(Out::KyviaParticles { which: Gen::KyviaDmg(0), pos });
    x.hp_proccess += 1;
    x.hp_proccess
}

/// `EntrySlave(0, on)` (MUT gcmn 0x004d92b0): the first exited core, out,
/// ordered, at the disc: its slave number, or -1.
fn entry_slave(x: &mut Kyvia, t: &mut Tree, cx: &mut Cx, on: i32) -> i32 {
    let Some(c) = t.core.as_mut() else { return -1 };
    if c.b.exit == 0 {
        return -1;
    }
    c.b.exit = 0;
    c.x.order_num = on;
    c.x.disc_pos = x.disc_pos;
    let _ = cx;
    c.x.slave_id
}

// --- Action ------------------------------------------------------------------------------

/// `ccBossKyvia01::Action` (MUT gcmn 0x004d3330).
fn action(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    match b.act_num {
        act::NEUTRAL | 11..=13 => {}
        act::DAMAGE => {
            match b.act_proccess {
                0 => {
                    b.lock_player(cx, false);
                    if check_dmg_smoke_eff(x, t, cx) == -1 {
                        b.act_proccess = 50;
                    } else {
                        b.act_proccess = 1;
                        x.dmg_anm_flg = 1;
                        x.quake_vector = [0x4120_0000, 0x4120_0000, 0x4120_0000, x.quake_vector[3]];
                        x.blur.abgr = x.ex_argb;
                        damage_camera(x, cx, 0x43af_0000, 0x4422_8000, None);
                        set_pitch(x, cx, 0);
                    }
                    let pos = cx.scene.chars[me].pos;
                    cx.out(Out::Se3dNote { se: 191, pos, note: 72 });
                }
                1 => {
                    b.act_proccess = 2;
                    b.act_count = 0;
                }
                2 => {
                    if b.anm_status != 0 {
                        b.act_proccess = 50;
                        x.blur.abgr = x.default_argb;
                        cx.out(Out::CamMode { mode: 6 });
                        set_pitch(x, cx, x.cam_rot_x);
                    }
                }
                50 if b.anm_status != 0 => {
                    set_pitch(x, cx, x.cam_rot_x);
                    b.change_action(cx, act::NEUTRAL, 0, true);
                    x.dmg_anm_flg = 0;
                    b.unlock_player();
                }
                _ => {}
            }
            if x.dmg_anm_flg != 0 {
                let q = x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
            }
            b.spd_down(0, 0x40a0_0000);
        }
        act::DAMAGE2 => {
            if b.act_proccess == 0 {
                let n = 65 + cx.cc.rand() % 5;
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 191, pos, note: n as u8 });
                b.act_proccess += 1;
                let ch = &mut cx.scene.chars[me];
                ch.affect.person = Some(me);
                ch.affect.ty = 1;
                ch.affect.color_cnt = -10;
                ch.affect.color_rate = 70;
                ch.affect.color = 255;
            }
            cx.scene.chars[me].affect.color_cnt = -10;
            if b.anm_status != 0 {
                let ch = &mut cx.scene.chars[me];
                ch.affect.color_fix = 0;
                ch.affect.color = 0;
                ch.affect.color_rate = 0;
                b.change_action(cx, act::NEUTRAL, 0, true);
                if let Some(c) = t.core.as_mut()
                    && c.x.k_dmg_flg == 4
                {
                    c.x.k_dmg_flg = 0;
                }
            }
            b.spd_down(0, 0x40a0_0000);
        }
        act::HAND => hand_atk(b, x, t, cx),
        act::LIGHT => {
            light_atk(b, x, t, cx);
            b.spd_down(0, 0x40a0_0000);
        }
        act::MEGID => {
            megid_flame(b, x, t, cx);
            b.spd_down(0, 0x40a0_0000);
        }
        act::DEAD => dead(b, x, cx),
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// The pitch (`bossCam` +0xe0) set.
fn set_pitch(x: &mut Kyvia, cx: &mut Cx, v: F) {
    x.cam_pitch = v;
    cx.out(Out::CamPitch { add: false, v });
}

/// The free camera round the body the hit and death use: mode 5, the eye
/// `back` behind and `up` above the body's place (`LOffset`), `Atk_vec`
/// (0, -800, 0) turned about z by `Z` (a fresh `ccRandF(1)`, or `z` given)
/// and added to it.
fn damage_camera(x: &mut Kyvia, cx: &mut Cx, back: F, up: F, z: Option<F>) {
    let me = cx.me;
    x.l_offset = VF0;
    x.move_transfer = VF0;
    x.l_offset = cx.scene.chars[me].pos;
    x.atk_vec = VF0;
    x.atk_mat = geom::unit_matrix();
    x.l_offset[1] = ee::sub(x.l_offset[1], back);
    x.l_offset[2] = ee::add(x.l_offset[2], up);
    x.atk_vec[1] = 0xc448_0000;
    cx.out(Out::CamMode { mode: 5 });
    x.z = match z {
        Some(z) => z,
        None => rand_f(cx.cc, ONE),
    };
    x.atk_mat = geom::rot_matrix_z(&x.atk_mat, x.z);
    x.atk_vec = geom::apply_matrix(&x.atk_mat, x.atk_vec);
    x.atk_vec = geom::vadd(x.l_offset, x.atk_vec);
    cx.out(Out::FreeCam { pos: x.atk_vec, view: x.l_offset });
}

/// Act 14 (MUT gcmn 0x004d3898): the body's death, the voices and the
/// camera round it, then the task's exit.
fn dead(b: &mut Boss, x: &mut Kyvia, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            cx.out(Out::Cinema(Some(-1)));
            b.lock_player(cx, false);
            b.act_proccess += 1;
            x.ded_dmg_vec = cx.scene.chars[me].pos;
            x.ded_dmg_vec[2] = ee::add(x.ded_dmg_vec[2], 0x443b_8000);
            x.ded_dmg_vec[1] = ee::sub(x.ded_dmg_vec[1], 0x4391_0000);
            x.ded_dmg_vec[0] = 0x4120_0000;
            x.dead_gp = true;
            cx.out(Out::KyviaParticles { which: Gen::KyviaDmg(1), pos: x.ded_dmg_vec });
            b.act_count = 0;
            let pos = cx.scene.chars[me].pos;
            cx.out(Out::Se3dNote { se: 192, pos, note: 48 });
            cx.out(Out::MusicFade { t: 30 });
        }
        1 => {
            // `actCount++ == 60`: the smokes killed (their pointers kept).
            b.act_count += 1;
            if b.act_count >= 135 {
                x.ded_dmg_vec[0] = ee::add(x.ded_dmg_vec[0], ONE);
                x.ded_dmg_vec[2] = ee::sub(x.ded_dmg_vec[2], 0x3e19_999a);
            }
            if ee::lt(ONE, ee::mul(0x42c8_0000, x.cam_pitch)) {
                let v = ee::sub(x.cam_pitch, ee::div(x.cam_pitch, 0x41a0_0000));
                set_pitch(x, cx, v);
            }
            dead_voice(b, cx);
            dead_cam(b, x, cx);
            x.ded_dmg_vec[2] = ee::sub(x.ded_dmg_vec[2], ONE);
            x.ded_dmg_vec[1] = ee::add(x.ded_dmg_vec[1], 0x4000_0000);
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            if b.anm_status != 0 {
                b.act_proccess += 1;
                b.act_count = 0;
            }
        }
        2 => {
            x.dead_gp = false;
            b.act_proccess += 1;
        }
        3 => {
            b.exit = 1;
            b.unlock_player();
            cx.out(Out::Cinema(None));
        }
        _ => {}
    }
}

/// `DeadKyviaVoice` (MUT gcmn 0x004d3cf0): the body's cries by `actCount`.
fn dead_voice(b: &Boss, cx: &mut Cx) {
    const CRIES: [(i16, i32, u8); 19] = [
        (30, 192, 60),
        (50, 192, 48),
        (70, 191, 72),
        (90, 191, 69),
        (100, 192, 69),
        (120, 191, 69),
        (130, 192, 58),
        (150, 192, 48),
        (170, 191, 72),
        (190, 191, 69),
        (200, 192, 65),
        (220, 191, 72),
        (230, 192, 61),
        (250, 192, 48),
        (270, 191, 72),
        (290, 191, 69),
        (300, 192, 72),
        (320, 191, 69),
        (330, 192, 72),
    ];
    let pos = cx.scene.chars[cx.me].pos;
    for (at, se, note) in CRIES {
        if b.act_count == at {
            cx.out(Out::Se3dNote { se, pos, note });
        }
    }
}

/// `DeadKyviaCam` (MUT gcmn 0x004d3fc0): the death's cameras at 70 and 150,
/// then the eye rising 4 a frame.
fn dead_cam(b: &Boss, x: &mut Kyvia, cx: &mut Cx) {
    let me = cx.me;
    if b.act_count == 70 {
        x.blur.abgr = x.ex_argb;
        let z = fabs(rand_f(cx.cc, ONE)) ^ 0x8000_0000;
        damage_camera(x, cx, 0x43af_0000, 0x4409_8000, Some(z));
    }
    if b.act_count == 150 {
        x.l_offset = VF0;
        x.move_transfer = VF0;
        x.l_offset = cx.scene.chars[me].pos;
        x.atk_vec = VF0;
        x.atk_mat = geom::unit_matrix();
        x.l_offset[1] = ee::sub(x.l_offset[1], 0x42c8_0000);
        x.l_offset[2] = ee::add(x.l_offset[2], 0x4316_0000);
        x.atk_vec[1] = ee::sub(x.atk_vec[1], 0x44bb_8000);
        x.atk_vec[2] = ee::add(x.atk_vec[2], 0x4348_0000);
        let r = rand_abs(cx, HALF);
        x.z = fp(0.2 + dp(r));
        x.atk_mat = geom::rot_matrix_z(&x.atk_mat, x.z);
        x.atk_vec = geom::apply_matrix(&x.atk_mat, x.atk_vec);
        x.atk_vec = geom::vadd(x.l_offset, x.atk_vec);
        cx.out(Out::FreeCam { pos: x.atk_vec, view: x.l_offset });
    }
    if b.act_count >= 161 {
        x.atk_vec[2] = ee::add(x.atk_vec[2], 0x4080_0000);
        cx.out(Out::FreeCam { pos: x.atk_vec, view: x.l_offset });
    }
}

const HALF: F = 0x3f00_0000;

// --- the attacks -------------------------------------------------------------------------

/// The core's `SetCoreState(n)` from the body.
fn core_state(t: &mut Tree, cx: &mut Cx, n: i32) {
    if let Some(c) = t.core.as_mut() {
        self::core::set_core_state(c, &mut t.gomoras, cx, n);
    }
}

/// The core ready for the body's attack: in its state 0 with it and its
/// gomoras at rest (`GetActNum`).
fn core_ready(t: &Tree) -> bool {
    t.core.as_ref().is_some_and(|c| c.x.core_state == 0 && self::core::get_act_num(c, &t.gomoras))
}

fn core_state_is(t: &Tree, n: i32) -> bool {
    t.core.as_ref().is_some_and(|c| c.x.core_state == n)
}

fn core_fade_check(t: &Tree) -> bool {
    t.core.as_ref().is_some_and(|c| self::core::fade_check(c, &t.gomoras))
}

/// `ccBossSkillDamage(CORE_THIS, m, i)` on each member alive.
fn core_damage_all(t: &mut Tree, cx: &mut Cx, i: usize) {
    let Some(c) = t.core.as_mut() else { return };
    for m in cx.party.members.into_iter().flatten() {
        if cx.valid(Some(m)) && cx.scene.chars[m].hp > 0 {
            let cm = c.me;
            as_me(cx, cm, |cx| c.b.skill_damage(cx, Some(m), None, i));
        }
    }
}

/// `HandAtk` (MUT gcmn 0x004d5930): the arm's sweep, skill 33 on every
/// member at its frame 48.
fn hand_atk(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            if core_ready(t) {
                core_state(t, cx, 8);
                b.act_proccess += 1;
                x.blur.enabled = true;
                x.blur.exit = false;
                x.blur.abgr = x.default_argb;
                b.lock_player(cx, false);
            }
        }
        1 => {
            if core_state_is(t, 6) {
                b.act_proccess += 1;
            }
        }
        2 => {
            cx.out(Out::CamModeRange { mode: 3, range: 0x4541_c000 });
            b.act_proccess += 1;
        }
        3 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
            }
        }
        4 => {
            let m = geom::unit_matrix();
            x.tes_pos = cx.cam.pos;
            x.tes_dirc = cx.cam.rot;
            let v = geom::apply_matrix(&geom::rot_matrix_z(&m, x.tes_dirc[2]), [0xc2dc_0000, 0xc509_8000, 0, ONE]);
            let rm = geom::rot_matrix_z(&m, x.tes_dirc[2]);
            x.tes_dirc[0] = 0;
            x.tes_dirc[1] = 0;
            x.tes_dirc[2] = wrap(ee::sub(x.tes_dirc[2], PI));
            x.tes_pos = geom::vadd(x.tes_pos, v);
            b.act_proccess += 1;
            x.snd_pos_l = [0xc3fa_0000, 0, 0, ONE];
            x.snd_pos_r = [0x43fa_0000, 0, 0, ONE];
            x.snd_pos_l = geom::vadd(geom::apply_matrix(&rm, x.snd_pos_l), x.tes_pos);
            x.snd_pos_r = geom::vadd(geom::apply_matrix(&rm, x.snd_pos_r), x.tes_pos);
            let pos = cx.scene.chars[me].pos;
            cx.out(Out::Se3dNote { se: 40, pos, note: 60 });
        }
        5 => {
            if x.af_cou == x.af_timing {
                cx.out(Out::AfterImage);
            }
            x.quake_vector = [0x41a0_0000, 0x41f0_0000, 0x4220_0000, x.quake_vector[3]];
            if x.anm_w.frame() == 20 {
                cx.out(Out::Se3dNote { se: 206, pos: x.snd_pos_r, note: 68 });
            }
            if x.anm_w.frame() == 12 {
                cx.out(Out::Se3dNote { se: 206, pos: x.snd_pos_l, note: 68 });
            }
            let ended = x.anm_w.forward();
            cx.out(Out::DrawArm { pos: x.tes_pos, dirc: x.tes_dirc });
            if x.anm_w.frame() == 48 {
                core_damage_all(t, cx, 33);
                cx.out(Out::Se3dNote { se: 56, pos: x.disc_pos, note: 72 });
                cx.out(Out::Se3d { se: 40, pos: x.disc_pos });
            }
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            if ended {
                cx.out(Out::CamMode { mode: 4 });
                core_state(t, cx, 7);
                x.anm_w.set("ANM_ex0batc0", cx.clips);
                b.act_proccess += 1;
            }
        }
        6 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
                x.blur.abgr = x.default_argb;
            }
        }
        7 if b.anm_status != 0 && core_fade_check(t) => {
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        _ => {}
    }
}

/// `KyviaMagicDamage(pos)` (MUT gcmn 0x004d53b0), the meteors' callback:
/// skill 34 by the core on each member alive; 1.
fn magic_damage(t: &mut Tree, cx: &mut Cx) -> i32 {
    core_damage_all(t, cx, 34);
    1
}

/// `MegidFlame` (MUT gcmn 0x004d8bb0): the meteors (`ccBossEffMeteoriteMissile`
/// of seven over the disc, 2500 up and 500 ahead), stream 48 shown first.
fn megid_flame(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            if core_ready(t) {
                core_state(t, cx, 8);
                b.act_proccess += 1;
                b.lock_player(cx, false);
            }
        }
        1 => {
            if core_state_is(t, 6) {
                b.act_proccess += 1;
                cx.out(Out::CamModeRange { mode: 3, range: 0x451c_4000 });
                x.blur.enabled = true;
                x.blur.exit = false;
                x.blur.abgr = x.ex_argb;
            }
        }
        2 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
            }
        }
        3 => {
            if cx.env.menu_type != -1 {
                return;
            }
            cx.out(Out::StreamMenu { stream: 48, mask: 0 });
            cx.out(Out::CamMode { mode: 5 });
            x.l_offset = cx.scene.chars[me].pos;
            x.l_offset[1] = ee::sub(x.l_offset[1], 0x43af_0000);
            x.l_offset[2] = ee::add(x.l_offset[2], 0x4422_8000);
            x.atk_vec = VF0;
            x.atk_mat = geom::unit_matrix();
            x.atk_vec[1] = 0xc522_8000;
            let r = rand_f(cx.cc, 0x3ecc_cccd);
            x.atk_mat = geom::rot_matrix_z(&x.atk_mat, r);
            x.atk_mat = geom::rot_matrix_x(&x.atk_mat, 0x3e4c_cccd);
            let mut eye = geom::vadd(geom::apply_matrix(&x.atk_mat, x.atk_vec), x.l_offset);
            eye[3] = ONE;
            cx.out(Out::FreeCam { pos: eye, view: x.l_offset });
            b.act_proccess += 1;
        }
        4 => {
            b.act_proccess += 1;
            x.s_point = x.disc_pos;
            x.s_point[1] = ee::add(x.s_point[1], 0x43fa_0000);
            x.s_point[2] = ee::add(x.s_point[2], 0x451c_4000);
        }
        5 => {
            b.act_proccess += 1;
            meteorite(b, x, cx);
            cx.out(Out::Se3dNote { se: 57, pos: x.disc_pos, note: 70 });
            b.act_count = 0;
        }
        6 => {
            let at = x.disc_pos;
            match b.act_count {
                5 => cx.out(Out::Se3dNote { se: 57, pos: at, note: 60 }),
                20 => {
                    cx.out(Out::Se3dNote { se: 40, pos: at, note: 60 });
                    cx.out(Out::Se3dNote { se: 56, pos: at, note: 50 });
                }
                40 | 60 => cx.out(Out::Se3dNote { se: 40, pos: at, note: 60 }),
                _ => {}
            }
            let c = b.act_count;
            b.act_count += 1;
            if c == 100 {
                b.act_proccess += 1;
                cx.out(Out::CamMode { mode: 6 });
            }
        }
        7 => {
            core_state(t, cx, 7);
            cx.out(Out::CamMode { mode: 4 });
            b.act_proccess += 1;
        }
        8 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
                x.blur.abgr = x.default_argb;
            }
        }
        9 if core_fade_check(t) => {
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        _ => {}
    }
}

// --- LightAtk ----------------------------------------------------------------------------

/// A Catmull-Rom point of `p` at `t`, with the game's `mula`/`madd` chain.
pub(crate) fn catmull(t: F, p: [F; 4]) -> F {
    let t2 = ee::mul(t, t);
    let t3 = ee::mul(t2, t);
    let acc = ee::add(ee::mul(0xbf00_0000, t3), t2);
    let b0 = ee::sub(acc, ee::mul(HALF, t));
    let half_t = ee::mul(HALF, t);
    let b1 = ee::add(ONE, ee::add(ee::mul(0x3fc0_0000, t3), ee::mul(0xc020_0000, t2)));
    let b2 = ee::add(half_t, ee::add(ee::mul(0xbfc0_0000, t3), ee::mul(0x4000_0000, t2)));
    let b3 = ee::sub(ee::mul(HALF, t3), ee::mul(HALF, t2));
    let f1 = ee::add(ee::mul(p[0], b0), ee::mul(p[1], b1));
    let acc = ee::add(ee::mul(p[2], b2), f1);
    ee::add(acc, ee::mul(p[3], b3))
}

/// A beam's point: the spline through `p` at `t` in each of x, y, z, w 1.
fn beam(t: F, p: [V4; 4]) -> V4 {
    let mut v = [0, 0, 0, ONE];
    for (k, c) in v.iter_mut().take(3).enumerate() {
        *c = catmull(t, [p[0][k], p[1][k], p[2][k], p[3][k]]);
    }
    v
}

/// `SetBPos` (MUT gcmn 0x004d5450): the six beam points off the disc.
fn set_b_pos(x: &mut Kyvia, cx: &mut Cx) {
    x.l_point.fill(x.disc_pos);
    for (k, r) in [0x442f_0000, 0x4422_8000, 0x4402_0000, 0x43fa_0000, 0x443b_8000, 0x4422_8000].into_iter().enumerate()
    {
        let v = rand_f(cx.cc, r);
        x.l_point[k][0] = ee::add(x.l_point[k][0], v);
    }
    for (k, r) in [0x43c8_0000, 0x43e1_0000, 0x4413_8000, 0x4396_0000, 0x4409_8000, 0x43c8_0000].into_iter().enumerate()
    {
        let v = rand_f(cx.cc, r);
        x.l_point[k][1] = ee::add(x.l_point[k][1], v);
    }
    for (k, r) in [0x43aa_0000, 0x43af_0000, 0x43c3_0000].into_iter().enumerate() {
        let v = rand_f(cx.cc, r);
        x.l_point[k][2] = ee::add(x.l_point[k][2], ee::add(0x4396_0000, v));
    }
    for (k, r) in [0x4396_0000, 0x4409_8000, 0x43c8_0000].into_iter().enumerate() {
        let v = rand_abs(cx, r);
        x.l_point[3 + k][2] = ee::add(x.l_point[3 + k][2], fp(100.0 + dp(v)));
    }
}

/// `LightAtk` (MUT gcmn 0x004d60a0): three beams from the body over the
/// party, their paths splines through `SetBPos`'s points, landing as skill
/// 32 on every member.
fn light_atk(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            if !core_state_is(t, 0) {
                return;
            }
            x.l_offset = VF0;
            x.move_transfer = VF0;
            x.l_offset = cx.scene.chars[me].pos;
            x.l_offset[1] = ee::sub(x.l_offset[1], 0x43af_0000);
            x.l_offset[2] = ee::add(x.l_offset[2], 0x4422_8000);
            if core_ready(t) {
                core_state(t, cx, 8);
                b.act_proccess += 1;
                cx.out(Out::CamMode { mode: 5 });
                x.atk_vec = VF0;
                x.atk_mat = geom::unit_matrix();
                x.atk_vec[1] = 0xc3af_0000;
                x.z = rand_f(cx.cc, HALF);
                x.atk_mat = geom::rot_matrix_z(&x.atk_mat, x.z);
                x.atk_vec = geom::apply_matrix(&x.atk_mat, x.atk_vec);
                x.atk_vec = geom::vadd(x.l_offset, x.atk_vec);
                cx.out(Out::FreeCam { pos: x.atk_vec, view: x.l_offset });
                b.lock_player(cx, false);
            }
            x.blur.enabled = true;
            x.blur.exit = false;
            x.blur.abgr = x.ex_argb;
        }
        1 => {
            if core_state_is(t, 6) {
                b.act_proccess += 1;
                x.atk_vec = VF0;
                x.atk_vec[1] = 0xc3af_0000;
                x.quake_vector = [0x4120_0000, 0x4120_0000, 0x4120_0000, x.quake_vector[3]];
            }
        }
        2 => {
            b.act_proccess += 1;
            cx.out(Out::KyviaParticles { which: Gen::Kyvia(0), pos: x.l_offset });
            let mut n = 0;
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) && cx.scene.chars[m].hp > 0 {
                    x.l_char[n] = cx.scene.chars[m].pos;
                    n += 1;
                }
            }
            set_b_pos(x, cx);
            match 3 - n as i32 {
                2 => {
                    x.l_char[1] = x.l_char[0];
                    x.l_char[2] = x.l_char[0];
                }
                1 => {
                    let r = cx.cc.rand();
                    if r % 2 != 0 {
                        x.l_char[2] = x.l_char[0];
                    } else {
                        x.l_char[2] = x.l_char[1];
                    }
                }
                _ => {}
            }
            x.add_time = 0x3d23_d70a;
            x.sub_btime = fp(1.5 * dp(0xbd23_d70a));
            x.btime = 0;
            cx.out(Out::Se3dNote { se: 191, pos: cx.scene.chars[me].pos, note: 60 });
        }
        3 => {
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            let c = b.act_count;
            b.act_count += 1;
            if c == 40 {
                b.act_count = 0;
                b.act_proccess += 1;
                x.atk_mat = geom::rot_matrix_x(&x.atk_mat, 0x3e19_999a);
            }
        }
        4 => {
            let left = fp(2500.0 - dp(fabs(x.move_transfer[1])));
            if !ee::le(left, 0x4100_0000) {
                x.move_transfer[1] = ee::add(x.move_transfer[1], ee::mul(0xbf80_0000, ee::div(left, 0x4080_0000)));
            }
            let mut eye = geom::apply_matrix(&x.atk_mat, x.move_transfer);
            eye = geom::vadd(eye, x.atk_vec);
            eye = geom::vadd(x.l_offset, eye);
            cx.out(Out::FreeCam { pos: eye, view: x.l_offset });
            let c = b.act_count;
            b.act_count += 1;
            if c == 30 {
                b.act_proccess += 1;
                cx.out(Out::KyviaParticles { which: Gen::Kyvia(7), pos: x.l_offset });
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 192, pos, note: 60 });
                cx.out(Out::Se3dNote { se: 61, pos, note: 48 });
                for k in 0..3 {
                    x.l_bpos[k] = x.l_offset;
                    x.l_bpos_sub[k] = x.l_offset;
                    cx.out(Out::KyviaParticles { which: Gen::Kyvia(5), pos: x.l_bpos[k] });
                    cx.out(Out::KyviaParticles { which: Gen::Kyvia(5), pos: x.l_bpos_sub[k] });
                    b.act_count = 0;
                }
            }
        }
        5 => {
            let c = b.act_count;
            b.act_count += 1;
            if c < 10 {
                let q = x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
            }
            if !ee::le(x.btime, ONE) {
                x.time_mode += 1;
                if x.time_mode == 3 {
                    x.add_time = ee::mul(0x4000_0000, x.add_time);
                }
                x.btime = 0;
                x.sub_btime = fp(1.5 * dp(x.add_time ^ 0x8000_0000));
            }
            let (o, p, c) = (x.l_offset, x.l_point, x.l_char);
            let paths: Option<[[V4; 4]; 3]> = match x.time_mode {
                1 => Some(std::array::from_fn(|k| [o, o, p[k], p[k + 3]])),
                2 => Some(std::array::from_fn(|k| [o, p[k], p[k + 3], c[k]])),
                3 => Some(std::array::from_fn(|k| [p[k], p[k + 3], c[k], c[k]])),
                _ => None,
            };
            if let Some(paths) = paths {
                // Each beam's head, then (while SubBtime is past 0) its tail.
                for (b, p) in x.l_bpos.iter_mut().zip(&paths) {
                    *b = beam(x.btime, *p);
                }
                if !ee::le(x.sub_btime, 0) {
                    let tt = if geom::eq(0, x.sub_btime) { x.btime } else { x.sub_btime };
                    for (b, p) in x.l_bpos_sub.iter_mut().zip(&paths) {
                        *b = beam(tt, *p);
                    }
                }
            } else if x.time_mode == 4 {
                x.time_mode = 1;
                for k in 0..3 {
                    let m = cx.party.members[k];
                    if let Some(m) = m
                        && cx.valid(Some(m))
                        && cx.scene.chars[m].hp > 0
                    {
                        cx.out(Out::KyviaParticles { which: Gen::Kyvia(3), pos: x.l_bpos[k] });
                    }
                }
                b.act_proccess += 1;
            }
            x.btime = ee::add(x.btime, x.add_time);
            x.sub_btime = ee::add(x.sub_btime, x.add_time);
        }
        6 => {
            core_damage_all(t, cx, 32);
            b.act_proccess += 1;
            b.act_count = 0;
            let pos = cx.scene.chars[me].pos;
            cx.out(Out::Se3dNote { se: 35, pos, note: 60 });
            cx.out(Out::Se3dNote { se: 56, pos, note: 60 });
        }
        7 => {
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
            let c = b.act_count;
            b.act_count += 1;
            if c == 30 {
                b.act_proccess += 1;
            }
        }
        8 => {
            cx.out(Out::CamMode { mode: 6 });
            x.blur.abgr = x.default_argb;
            core_state(t, cx, 7);
            b.act_proccess += 1;
        }
        9 if core_fade_check(t) => {
            b.change_action(cx, act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        _ => {}
    }
}

// --- the meteors -------------------------------------------------------------------------

/// `ccBossEffMeteoriteMissile` (MUT gcmn 0x0047aa90, `Draw` 0x0047b330): each
/// meteor's fall, a Bezier from 500 back (turned by -2.5525 about x) down to
/// a random point round the missile's place; the first's landing calls the
/// body's `KyviaMagicDamage` at 20 and 60 frames on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meteorite {
    pub pos: V4,
    pub start: Vec<V4>,
    pub pos2: Vec<V4>,
    pub pos3: Vec<V4>,
    pub end: Vec<V4>,
    pub now: Vec<V4>,
    /// `t` (each meteor's step), `Time` (its place on the path).
    pub step: Vec<F>,
    pub time: Vec<F>,
    pub landed: Vec<u8>,
    pub count: Vec<i32>,
    pub quake: V4,
}

impl Meteorite {
    /// The constructor for `n` meteors round `pos` within `range`.
    fn new(cx: &mut Cx, pos: V4, range: F, n: usize) -> Meteorite {
        let mut m = Meteorite { pos, ..Meteorite::default() };
        for _ in 0..n {
            let turn = rand_f(cx.cc, 0x4048_f5c3);
            let rm = geom::rot_matrix_z(&geom::unit_matrix(), turn);
            let r = rand_f(cx.cc, ee::mul(range, HALF));
            let mut v = VF0;
            v[1] = fabs(ee::add(ee::mul(range, ee::sub(ONE, HALF)), r));
            m.end.push(geom::apply_matrix(&rm, v));
        }
        let rx = geom::rot_matrix_x(&geom::unit_matrix(), 0xc023_5ce2);
        for k in 0..n {
            let back = geom::apply_matrix(&rx, [0, 0xc3fa_0000, 0, ONE]);
            let p3 = geom::vadd(m.end[k], back);
            let p2 = geom::vadd(p3, geom::vadd(p3, back));
            let p1 = geom::vadd(p2, back);
            m.pos3.push(p3);
            m.pos2.push(p2);
            m.start.push(p1);
        }
        for _ in 0..n {
            let r = rand_abs(cx, 0x3cf5_c28f);
            m.step.push(fp(dp(0x3cf5_c28f) + dp(r)));
            m.time.push(0);
        }
        m.now = vec![VF0; n];
        m.landed = vec![0; n];
        m.count = vec![0; n];
        m.quake = [0x41f0_0000, 0x4220_0000, 0x4248_0000, 0];
        // Each meteor's two trails, following it.
        for k in 0..n {
            cx.out(Out::KyviaParticles { which: Gen::MissileSmoke(6), pos: m.now[k] });
            cx.out(Out::KyviaParticles { which: Gen::MissileSmoke(7), pos: m.now[k] });
        }
        m
    }

    /// `Spline` (MUT 0x0047b910): the cubic Bezier at `t`.
    fn spline(t: F, p: [V4; 4]) -> V4 {
        let u = ee::sub(ONE, t);
        let b0 = ee::mul(u, ee::mul(u, u));
        let b1 = ee::mul(t, ee::mul(ee::mul(0x4040_0000, u), u));
        let b2 = ee::mul(t, ee::mul(ee::mul(0x4040_0000, u), t));
        let b3 = ee::mul(t, ee::mul(t, t));
        let mut v = [0, 0, 0, ONE];
        for (i, c) in v.iter_mut().take(3).enumerate() {
            let acc = ee::add(ee::mul(p[0][i], b0), ee::mul(p[1][i], b1));
            let acc = ee::add(ee::mul(p[2][i], b2), acc);
            *c = ee::add(acc, ee::mul(p[3][i], b3));
        }
        v
    }

    /// One `Draw`: false once every meteor has lain 90 frames.
    fn draw(&mut self, t: &mut Tree, cx: &mut Cx) -> bool {
        for k in 0..self.end.len() {
            if self.landed[k] == 0 {
                let p = Self::spline(self.time[k], [self.start[k], self.pos2[k], self.pos3[k], self.end[k]]);
                self.now[k] = geom::vadd(p, self.pos);
            }
            if dp(self.time[k]) > 0.984 && self.landed[k] == 0 {
                self.landed[k] = 1;
                cx.out(Out::Se3dNote { se: 35, pos: self.pos, note: 60 });
                if k == 0 {
                    cx.out(Out::KyviaParticles { which: Gen::BurstSmoke(8), pos: self.pos });
                    cx.out(Out::KyviaParticles { which: Gen::BurstSmoke(9), pos: self.pos });
                    let mut up = VF0;
                    up[2] = 0x42f0_0000;
                    cx.out(Out::KyviaParticles { which: Gen::BurstSmoke(11), pos: geom::vadd(up, self.pos) });
                    let mut rot = cx.cam.rot;
                    rot[2] = wrap(ee::sub(rot[2], PI));
                    rot[0] = 0;
                    rot[1] = 0;
                    cx.out(Out::SmokeRock { pos: self.now[k], rot });
                }
            }
            if self.landed[k] != 0 {
                self.count[k] += 1;
                if self.count[k] < 60 {
                    let q = self.quake;
                    cx.quake_vec([q[0], q[1], q[2]]);
                }
            }
            if k == 0 && matches!(self.count[0], 20 | 60) {
                magic_damage(t, cx);
            }
            self.time[k] = ee::add(self.time[k], self.step[k]);
        }
        !self.landed.iter().zip(&self.count).all(|(&l, &c)| l == 1 && c >= 90)
    }
}

/// `ccBossEffMeteoriteMissileCreate(DiscPos, 500, 7, 0, KyviaMagicDamage,
/// bossCam)`.
fn meteorite(b: &mut Boss, x: &mut Kyvia, cx: &mut Cx) {
    let m = Meteorite::new(cx, x.disc_pos, 0x43fa_0000, 7);
    let kind = EffKind::Meteorite { count: 7 };
    let id = b.effects.create(kind);
    if let Some(e) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
        e.meteorite = Some(Box::new(m));
    }
    cx.out(Out::Effect { id, kind, pos: x.disc_pos, dirc: VF0 });
}

/// `ccBossEffManager::Draw` over the fight's effects: a meteorite's `Draw`
/// may call the body's `KyviaMagicDamage`.
fn manager_pass(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let _ = x;
    for k in 0..b.effects.slots.len() {
        let Some(e) = b.effects.slots[k].as_mut() else { continue };
        if !e.enabled {
            b.effects.slots[k] = None;
            continue;
        }
        let EffKind::Meteorite { .. } = e.kind else {
            e.draw();
            continue;
        };
        let Some(mut m) = e.meteorite.take() else { continue };
        let alive = m.draw(t, cx);
        if let Some(e) = b.effects.slots[k].as_mut() {
            e.meteorite = Some(m);
            if !alive {
                e.enabled = false;
            }
        }
    }
}

#[cfg(test)]
mod tests;
