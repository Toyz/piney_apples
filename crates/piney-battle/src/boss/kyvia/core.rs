//! `kyviaCore` (kyviacore.cpp, MUT gcmn 0x004f7e80-0x004fdf00): Kyvia's
//! core, the fight's target. It rises on the disc, circles it on splines,
//! sends out its five gomoras ([`super::gomora`]) and counts the damage and
//! the time it takes; enough of either calls the body's attack
//! (`kAtkFlg`). The EX mode (`KyviaExFlg`, Kyvia's last fights) is left
//! out. docs/engine/boss-kyvia.md.

use piney_data::field::ee;

use super::gomora::{self, Gomora};
use super::{Gen, Part, as_me, dp, fabs, fp, new_char, plain_boss, rand_abs, set_base_param, wrap};
use crate::boss::{Anm, Boss, Class, Cx, Out, VF0};
use crate::enemy_ai::rand_f;
use crate::geom::{self, V4};
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
/// `slaveNum[0]`: the gomoras.
pub const GOMORAS: usize = 5;

/// The core's acts (`actNum`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DAMAGE: i16 = 1;
    pub const DAMAGE2: i16 = 2;
    /// The hold and the gomoras' charge (`AtkGomora`).
    pub const HOLD: i16 = 3;
    pub const DRAINED: i16 = 11;
    pub const DRAINED_WAIT: i16 = 12;
    pub const WAVE: i16 = 13;
    pub const DEAD: i16 = 14;
    pub const START_GOMORA: i16 = 20;
    pub const ENTRY_GOMORA: i16 = 21;
}

/// `kyviaCore`'s own members (Infection's DWARF names).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Core {
    /// `slaveCtrl[0]`: the gomoras' characters.
    pub gomoras: Vec<usize>,
    /// `master`, `slaveID`, `orderNum` (`SetMaster`, `Order`).
    pub master: usize,
    pub slave_id: i32,
    pub order_num: i32,
    pub ex: u8,
    pub st_flg: u8,
    pub alpha: F,
    pub end_flg: i32,
    pub no: i32,
    pub time: F,
    pub time_count: F,
    /// A float here, 0 or 1.
    pub time_flg: F,
    pub time_mode: i32,
    pub now_gomora_num: i32,
    pub gomora_hold: i32,
    pub core_state: i32,
    pub temp_state: i32,
    pub gomora_meter: i32,
    pub gomora_lock: i32,
    pub think_proccess: i32,
    pub af_cou: i32,
    pub af_timing: i32,
    pub now_dmg_time: i32,
    pub dmg_count: i32,
    pub dmg_wait: i32,
    pub dmg_time: i32,
    pub atk_meter: i32,
    pub think_lock: i32,
    pub fade_flg: u8,
    pub dead_flg: u8,
    pub gs_count: i32,
    pub gs_rimit: i32,
    pub gomora_count: i32,
    pub gcunt_rimit: i32,
    /// `Aura`, `Aura2` made, and whether each has stopped emitting.
    pub aura: bool,
    pub aura2: bool,
    pub aura_stop: bool,
    pub aura2_stop: bool,
    pub dead_count: i32,
    pub attr: i32,
    pub k_atk_flg: i32,
    pub k_dmg_flg: i32,
    pub cone_voice_cou: i32,
    pub gomora_flg: [u8; GOMORAS],
    pub sp_vec: [V4; 4],
    /// `DiscPos`: where the body put it (`SetEnterPos`).
    pub disc_pos: V4,
    pub move_pattern: i32,
    pub escape_cou: i32,
    pub temp_time_mode: i32,
    pub temp_time: F,
    pub dead_pos: V4,
    pub dead_rot: V4,
    pub off_set: V4,
    pub kyvia_lv: i16,
    /// What the core's deaths have added to its two rows' `maxHP` (the
    /// game writes it into `bossTbl` for good).
    pub row_hp: i16,
    /// `anmw`: the wave (`ANM_xx11wave`) of act 13.
    pub anm_w: Anm,
}

impl Core {
    /// `InitData` (MUT gcmn 0x004f7e80).
    fn init() -> Core {
        Core {
            st_flg: 1,
            end_flg: 1,
            time_count: 0x3ba3_d70a,
            time_flg: ONE,
            time_mode: 1,
            now_gomora_num: -1,
            gomora_lock: 1,
            af_timing: 10,
            dmg_wait: 1000,
            dmg_time: 3000,
            atk_meter: 30,
            fade_flg: 1,
            gs_rimit: 30,
            gcunt_rimit: 60,
            sp_vec: [VF0; 4],
            anm_w: Anm::new(),
            ..Core::default()
        }
    }
}

// --- the constructor ---------------------------------------------------------------------

/// `kyviaCore::kyviaCore` (MUT gcmn 0x004f8a30), `CoreInit(lv)` (0x004f82c0)
/// and the master's `SetMaster(master, 0)`: the core at Kite's place, not
/// drawn, exited, with its five gomoras made (`GomoraInit(lv)`), each a
/// character and boss of its own. `me` is the core's character.
pub(crate) fn new(cx: &mut Cx, me: usize, master: usize, lv: i16) -> Boss {
    let mut b = plain_boss();
    let mut x = Core::init();
    x.master = master;
    x.slave_id = 0;
    b.draw_sw = 0;
    let kite = cx.world.player.map_or(VF0, |k| cx.scene.chars[k].pos);
    cx.scene.chars[me].pos = kite;
    b.dirc = VF0;
    let attr = x.attr;
    set_core_base_param(cx, &mut x, me, lv, attr);
    b.anm_tbl = cx.data.kyvia.core_anims.clone();
    x.dmg_count = i32::from(cx.scene.chars[me].max_hp);
    b.anm.set("ANM_ex0cnut0", cx.clips);
    b.change_action(cx, act::NEUTRAL, 0, true);
    for k in 0..GOMORAS {
        let g = new_char(cx, 16);
        let mut gb = gomora::new(cx, g, me, k as i32, lv);
        if let Class::Gomora(gx) = &mut gb.class {
            gx.state = 2;
            // `LifeFlg = &GomoraFlg[i] != 0`: the address, so always 1.
            gx.life_flg = 1;
        }
        if let Some(f) = cx.scene.chars[g].foe_state_mut() {
            f.boss = Some(Box::new(gb));
        }
        x.gomoras.push(g);
    }
    x.anm_w.set("ANM_xx11wave", cx.clips);
    b.class = Class::KyviaCore(Box::new(x));
    b
}

/// `SetCoreBaseParam(lv, attr)` (MUT gcmn 0x004f8770, OUT 0x004f5150): the
/// core's row by level and attribute (32-39; 15 for level 5 with attribute
/// -1). From level 2, each call while the core is dead (`DeadFlg`) adds
/// 3000, 1500 or 1000 to both rows' `maxHP` for good ([`Core::row_hp`]).
fn set_core_base_param(cx: &mut Cx, x: &mut Core, me: usize, lv: i16, attr: i32) {
    let (a, extra) = match lv {
        1 => (32, 0),
        2 => (34, 3000),
        3 => (36, 1500),
        4 => (38, 1000),
        5 if attr == -1 => (15, 0),
        _ => {
            x.kyvia_lv = lv;
            return;
        }
    };
    let row = if lv == 5 { a } else { a + usize::from(attr != 0) };
    if x.dead_flg != 0 && extra != 0 {
        x.row_hp = x.row_hp.wrapping_add(extra);
    }
    set_base_param(cx, me, row);
    if lv != 5 && x.row_hp != 0 {
        let ch = &mut cx.scene.chars[me];
        ch.max_hp = ch.max_hp.wrapping_add(x.row_hp);
        ch.hp = ch.max_hp;
    }
    x.kyvia_lv = lv;
}

/// `ResetData` (MUT gcmn 0x004f80f0): the core as its death leaves it for
/// the next rise.
fn reset_data(x: &mut Core, cx: &mut Cx, me: usize) {
    x.st_flg = 1;
    x.alpha = 0;
    x.end_flg = 1;
    x.no = 0;
    x.time = 0;
    x.time_count = 0x3ba3_d70a;
    x.time_flg = ONE;
    x.time_mode = 1;
    x.k_atk_flg = 0;
    x.k_dmg_flg = 0;
    x.core_state = 0;
    x.gomora_meter = 0;
    x.gomora_lock = 0;
    x.attr = 0;
    let lv = x.kyvia_lv;
    set_core_base_param(cx, x, me, lv, 0);
    x.dmg_count = i32::from(cx.scene.chars[me].max_hp);
    x.af_cou = 0;
    x.atk_meter = 30;
    x.think_lock = 0;
    x.fade_flg = 1;
    x.k_dmg_flg = 0;
    x.gomora_flg = [0; GOMORAS];
    x.sp_vec = [VF0; 4];
}

// --- the frame ---------------------------------------------------------------------------

/// `kyviaCore::Main` (MUT gcmn 0x004f9630) with its `Slave()`: the gomoras'
/// `Main`s after its own.
pub(crate) fn main(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    let me = c.me;
    if c.b.exit != 0 {
        return;
    }
    as_me(cx, me, |cx| {
        super::head(&mut c.b, cx);
        fade_core(c, cx);
        think(c, gs, cx);
        action(c, gs, cx);
        c.b.move_(cx);
        if c.b.draw_sw != 0 {
            c.b.anm_status = i8::from(c.b.anm.forward());
            c.b.transparency = c.x.alpha;
            c.b.set_transparency = c.x.alpha;
        }
    });
    for k in 0..gs.len() {
        let acts: Vec<i16> = gs.iter().map(|g| g.b.act_num).collect();
        gomora::main(&mut gs[k], me, &acts, cx);
    }
}

/// `FadeCore` (MUT gcmn 0x004fb360): the fades `SetCoreState` 7 and 8 begin.
fn fade_core(c: &mut Part<Core>, cx: &mut Cx) {
    let x = &mut c.x;
    if !geom::eq(0, x.time_flg) {
        return;
    }
    match x.core_state {
        7 => {
            x.alpha = ee::add(x.alpha, 0x3d4c_cccd);
            if !ee::le(x.alpha, ONE) {
                x.time_flg = ONE;
                x.core_state = x.temp_state;
                x.alpha = ONE;
                x.fade_flg = 1;
                c.b.entry_cmnd_target(cx);
            }
        }
        8 => {
            x.alpha = ee::sub(x.alpha, 0x3d4c_cccd);
            if ee::lt(x.alpha, 0) {
                x.alpha = 0;
                x.fade_flg = 1;
                x.core_state = 6;
            }
        }
        _ => {}
    }
}

/// `kyviaCore::Think` (MUT gcmn 0x004faac0).
fn think(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    let me = c.me;
    if c.x.ex == 0 {
        if c.x.st_flg == 1 {
            core_voice(c, cx, 0);
            if geom::eq(0, c.x.alpha) {
                start_init(c, gs, cx);
            }
            c.x.alpha = ee::add(c.x.alpha, 0x3ca3_d70a);
            if !ee::lt(c.x.alpha, ONE) {
                c.x.st_flg = 0;
                c.x.alpha = ONE;
                action_start(c, cx);
            }
        }
        let d = &mut c.b.dirc[2];
        *d = wrap(ee::add(*d, 0x3cf5_c28f));
        judgment_dmg_anm(c, cx);
        check_count_gomora(c, gs, cx);
        check_count_atk_time(c, gs, cx);
    }
    match c.b.act_num {
        1..=5 | 11..=14 | 20 | 21 => {}
        act::NEUTRAL => {
            if c.x.end_flg == 0 {
                c.x.end_flg = 1;
                c.x.no += 1;
            }
            if c.x.end_flg == 1 {
                // ActionDecision: ccRand() % 15 with its sign.
                let ty = cx.cc.rand() % 15;
                let t = c.b.select_target(cx, ty, crate::boss::FAR);
                cx.scene.chars[me].target_char = t;
            }
            if cx.scene.chars[me].cond[cond::HOLD] == 0 {
                switch_action_pattern(c, gs, cx);
            }
        }
        _ => c.b.change_action(cx, act::NEUTRAL, 0, true),
    }
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// `CoreVoice(mode)` (MUT gcmn 0x004fa960): the core's cries as it rises
/// (0) and dies (1).
fn core_voice(c: &mut Part<Core>, cx: &mut Cx, mode: i32) {
    c.x.cone_voice_cou += 1;
    let pos = cx.scene.chars[c.me].pos;
    let notes: [(i32, i32, u8); 3] = if mode != 0 {
        [(1, 193, 58), (5, 194, 64), (12, 193, 57)]
    } else {
        [(1, 193, 56), (5, 194, 60), (12, 193, 58)]
    };
    for (at, se, note) in notes {
        if c.x.cone_voice_cou == at {
            cx.out(Out::Se3dNote { se, pos, note });
        }
    }
}

/// `StartInit` (MUT gcmn 0x004f9dc0): the core shown near the disc (5 off
/// it, 500 at level 5), its particles, its first path.
fn start_init(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    let _ = gs;
    let r = if c.x.kyvia_lv < 5 { 0x40a0_0000 } else { 0x43fa_0000 };
    let turn = rand_f(cx.cc, PI);
    let m = geom::rot_matrix_z(&geom::unit_matrix(), turn);
    let v = geom::apply_matrix(&m, [0, r, 0, ONE]);
    let pos = geom::vadd(v, c.x.disc_pos);
    cx.scene.chars[c.me].pos = pos;
    c.b.draw_sw = 1;
    cx.out(Out::KyviaParticles { which: Gen::Core(0), pos });
    c.x.dead_flg = 0;
    c.x.time_mode = 1;
    set_sp_point(c, cx);
    c.x.cone_voice_cou = 0;
}

/// `ActionStart` (MUT gcmn 0x004f9f40): on the command lists, its two auras
/// (the one of the other attribute not emitting), its damage limit.
fn action_start(c: &mut Part<Core>, cx: &mut Cx) {
    let me = c.me;
    crate::fellow::entry_cmnd(cx.scene, me);
    if !c.x.aura {
        let mut pos = cx.scene.chars[me].pos;
        pos[2] = ee::add(pos[2], 0x437a_0000);
        c.x.aura = true;
        cx.out(Out::KyviaParticles { which: Gen::Core(7), pos });
        c.x.aura2 = true;
        cx.out(Out::KyviaParticles { which: Gen::Core(8), pos });
    }
    if c.x.attr == 0 {
        c.x.aura2_stop = c.x.aura2 || c.x.aura2_stop;
    } else {
        c.x.aura_stop = c.x.aura || c.x.aura_stop;
    }
    let max = ee::from_int(i32::from(cx.scene.chars[me].max_hp));
    let part = |k: F| ee::to_int(ee::mul(k, max));
    c.x.dmg_wait = match (c.x.kyvia_lv, c.x.dead_count) {
        (1, _) => 800,
        (2, 0) => part(0x3e80_0000),
        (2, _) => part(0x3e4c_cccd),
        (3, 0) | (4, 0) => part(0x3e99_999a),
        (3, 1) => part(0x3e4c_cccd),
        (3, _) => part(0x3e38_51ec),
        (4, 1) => part(0x3e80_0000),
        (4, _) => part(0x3e61_47ae),
        _ => c.x.dmg_wait,
    };
}

/// `JudgmentDmgAnm` (MUT gcmn 0x004fb860): hurt (act 2), one time in five
/// the body flinches too (`kDmgFlg` 4).
fn judgment_dmg_anm(c: &mut Part<Core>, cx: &mut Cx) {
    if c.b.act_num == act::DAMAGE2 && (cx.cc.rand() % 10).abs() < 2 {
        c.x.k_dmg_flg = 4;
    }
}

/// `CheckCountAtkTime` (MUT gcmn 0x004fb8e0): the core's time out
/// (`Dmgtime` frames: the body attacks, `kAtkFlg` 2) or its damage limit
/// (`DmgWait` HP: the body is hurt first, 1); either kills the gomoras and
/// sends the core away (state 4) once they are gone.
fn check_count_atk_time(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    let me = c.me;
    if c.x.gomora_hold == 1 || c.x.core_state == 4 || ee::lt(c.x.alpha, ONE) {
        return;
    }
    let t = c.x.now_dmg_time;
    c.x.now_dmg_time += 1;
    let hp = i32::from(cx.scene.chars[me].hp);
    let (due, flg) = if t >= c.x.dmg_time { (true, 2) } else { (c.x.dmg_count - hp >= c.x.dmg_wait, 1) };
    if due {
        c.x.time_flg = 0;
        let n = kill_gomora(c, gs, cx);
        c.b.erase_cmnd_target(cx);
        if n == 0 && cx.scene.chars[me].cond[cond::HOLD] == 0 && get_gomora_state(gs, 3) {
            c.x.time_flg = ONE;
            c.x.dmg_count = i32::from(cx.scene.chars[me].hp);
            c.x.now_dmg_time = 0;
            c.x.k_dmg_flg = 0;
            c.x.temp_state = c.x.core_state;
            c.x.core_state = 4;
            if flg == 2 {
                c.b.erase_cmnd_target(cx);
            }
            c.x.k_atk_flg = flg;
            step_gomora_list(gs, cx);
        }
    }
    let hp = i32::from(cx.scene.chars[me].hp);
    if c.x.dmg_count < hp {
        c.x.dmg_count = hp;
    }
}

/// `CheckCountGomora` (MUT gcmn 0x004fbba0): after 60 frames a gomora every
/// 30 until five are out; once all are gone again, the next lists and 1800
/// frames more on the core's time.
fn check_count_gomora(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    if c.x.core_state == 4 || ee::lt(c.x.alpha, ONE) {
        return;
    }
    if c.x.gomora_hold < 2 && c.x.gomora_count >= c.x.gcunt_rimit {
        let mut spawned = false;
        if c.x.gomora_meter < 5 {
            let n = c.x.gs_count;
            c.x.gs_count += 1;
            if n == c.x.gs_rimit {
                entry_gomora(c, gs, cx);
                start_gomora(c, gs, cx);
                c.x.gs_count = 0;
                c.x.gomora_meter += 1;
                c.x.gomora_hold = 1;
                spawned = true;
            }
        }
        if !spawned && c.x.gomora_meter >= 5 {
            c.x.gomora_hold = 2;
        }
    } else {
        c.x.gomora_count += 1;
    }
    if c.x.gomora_hold == 2 && c.x.gomora_meter != 0 && get_gomora_state(gs, 1) {
        c.x.gomora_meter = 0;
        c.x.gomora_count = 0;
        c.x.now_dmg_time += 1800;
        for g in gs.iter_mut() {
            gomora::next_g_list(g, cx);
        }
    }
}

/// `SwitchActionPattern` (MUT gcmn 0x004fc5e0): the escape (state 4), and
/// the core's ride along its spline, a new one each fourth part.
fn switch_action_pattern(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    let _ = gs;
    match c.x.core_state {
        6..=8 => return,
        4 => think_escape(c, cx),
        _ => {}
    }
    if c.x.ex != 0 {
        c.x.escape_cou = 2;
        return;
    }
    if !ee::le(c.x.time, ONE) {
        c.x.time = 0;
        c.x.time_mode += 1;
        if c.x.core_state == 4 {
            c.x.escape_cou += 1;
        }
        c.x.think_lock = 1;
    }
    let s = c.x.sp_vec;
    match c.x.time_mode {
        1 => move_spline(c, cx, [s[0], s[0], s[1], s[2]]),
        2 => move_spline(c, cx, [s[0], s[1], s[2], s[3]]),
        3 => move_spline(c, cx, [s[1], s[2], s[3], s[3]]),
        4 => {
            set_sp_point(c, cx);
            c.x.time_mode = 1;
        }
        _ => {}
    }
    if !geom::eq(0, c.x.time_flg) {
        c.x.time = ee::add(c.x.time, c.x.time_count);
    }
}

/// `ThinkESCAPE` (MUT gcmn 0x004fae40): the core fades away five times as
/// fast along its path, after-images behind it, and comes back with the
/// other attribute once it has ridden two parts.
fn think_escape(c: &mut Part<Core>, cx: &mut Cx) {
    match c.x.think_proccess {
        0 => {
            c.x.temp_time_mode = c.x.time_mode;
            c.x.think_proccess += 1;
            c.x.temp_time = c.x.time_count;
            c.x.time_count = ee::mul(0x40a0_0000, c.x.time_count);
            c.b.erase_cmnd_target(cx);
            c.x.escape_cou = 0;
            if c.x.attr == 0 {
                c.x.aura_stop = c.x.aura || c.x.aura_stop;
            } else {
                c.x.aura2_stop = c.x.aura2 || c.x.aura2_stop;
            }
        }
        1 => {
            c.x.af_cou += 1;
            c.x.alpha = ee::sub(c.x.alpha, 0x3c75_c28f);
            if ee::le(c.x.alpha, 0) {
                c.x.alpha = 0;
            }
            if c.x.af_timing < c.x.af_cou {
                c.x.af_cou = 0;
            }
            if c.x.af_cou == 1 {
                cx.out(Out::AfterImage);
            }
            if geom::eq(0, c.x.alpha) && c.x.escape_cou == 2 {
                c.x.core_state = 0;
                c.x.time_count = c.x.temp_time;
                c.x.think_proccess = 0;
                change_attribute(c, cx);
                c.b.change_action(cx, act::NEUTRAL, 0, true);
            }
        }
        _ => {}
    }
}

/// `ChangeAttribute` (MUT gcmn 0x004fb110): the other attribute's row, the
/// HP kept, still dead to the party's targeting if it was.
fn change_attribute(c: &mut Part<Core>, cx: &mut Cx) {
    if c.x.ex != 0 {
        return;
    }
    let me = c.me;
    let dead = cx.scene.chars[me].cond[cond::DEAD] == 1;
    c.x.attr = i32::from(c.x.attr == 0);
    let (lv, attr) = (c.x.kyvia_lv, c.x.attr);
    set_core_base_param(cx, &mut c.x, me, lv, attr);
    cx.scene.chars[me].hp = c.x.dmg_count as i16;
    if dead {
        cx.scene.chars[me].cond[cond::DEAD] = 1;
    }
}

/// `MoveSpline(p1, p2, p3, p4)` (MUT gcmn 0x004fc8b0): the Catmull-Rom point
/// at `time`, the four points a matrix's rows (`sceVu0ApplyMatrix`).
fn move_spline(c: &mut Part<Core>, cx: &mut Cx, p: [V4; 4]) {
    let t = c.x.time;
    let t2 = ee::mul(t, t);
    let t3 = ee::mul(t2, t);
    let acc = ee::add(ee::mul(0xbf00_0000, t3), t2);
    let b0 = ee::sub(acc, ee::mul(0x3f00_0000, t));
    let b1 = ee::add(ONE, ee::add(ee::mul(0x3fc0_0000, t3), ee::mul(0xc020_0000, t2)));
    let b2 = ee::add(ee::mul(0x3f00_0000, t), ee::add(ee::mul(0xbfc0_0000, t3), ee::mul(0x4000_0000, t2)));
    let b3 = ee::sub(ee::mul(0x3f00_0000, t3), ee::mul(0x3f00_0000, t2));
    let mut pos = geom::apply_matrix(&p, [b0, b1, b2, b3]);
    pos[3] = ONE;
    cx.scene.chars[c.me].pos = pos;
}

/// `SetSpPoint` (MUT gcmn 0x004fca70): the next path from where the core is
/// to three points over the disc, by one of three patterns.
fn set_sp_point(c: &mut Part<Core>, cx: &mut Cx) {
    let a = dp(rand_abs(cx, 0x43af_0000));
    let b = dp(rand_abs(cx, 0x4316_0000));
    let v = geom::apply_matrix(&geom::unit_matrix(), [0, fp(a + b), 0, ONE]);
    let base = geom::vadd(v, c.x.disc_pos);
    c.x.sp_vec = [cx.scene.chars[c.me].pos, base, base, base];
    c.x.move_pattern = (cx.cc.rand() % 3).abs();
    // Each point's offset: x, y, z drawn in turn.
    let r = |cx: &mut Cx, k: F| rand_abs(cx, k);
    let raw = |cx: &mut Cx, k: F| rand_f(cx.cc, k);
    let off = |cx: &mut Cx, base: f64, k: F| fp(base + dp(rand_abs(cx, k)));
    let neg = |cx: &mut Cx, base: f64, k: F| fp(base + -dp(rand_abs(cx, k)));
    let negz = |cx: &mut Cx, k: F| fp(-dp(rand_abs(cx, k)));
    let (h100, h150, h300, h450, h500, h600, h650, h700, h800) = (
        0x42c8_0000,
        0x4316_0000,
        0x4396_0000,
        0x43e1_0000,
        0x43fa_0000,
        0x4416_0000,
        0x4422_8000,
        0x442f_0000,
        0x4448_0000,
    );
    let _ = h150;
    let add = |c: &mut Part<Core>, k: usize, v: [F; 3]| {
        c.x.sp_vec[k] = geom::vadd(c.x.sp_vec[k], [v[0], v[1], v[2], ONE]);
    };
    match c.x.move_pattern {
        0 => {
            let v = [r(cx, h650), r(cx, h100), r(cx, h100)];
            add(c, 1, v);
            let v = [off(cx, -100.0, h650), neg(cx, -50.0, h300), r(cx, h100)];
            add(c, 2, v);
            let v = [raw(cx, h450), r(cx, h100), r(cx, h100)];
            add(c, 3, v);
        }
        1 => {
            let v = [raw(cx, h650), negz(cx, h700), r(cx, h100)];
            add(c, 1, v);
            let v = [off(cx, 100.0, h450), r(cx, h100), r(cx, h100)];
            add(c, 2, v);
            let v = [off(cx, -100.0, h450), neg(cx, -50.0, h500), r(cx, h100)];
            add(c, 3, v);
        }
        _ => {
            let v = [r(cx, h600), negz(cx, h800), r(cx, h100)];
            add(c, 1, v);
            let v = [raw(cx, h650), r(cx, h100), r(cx, h100)];
            add(c, 2, v);
            let v = [off(cx, -100.0, h650), neg(cx, -50.0, h700), r(cx, h100)];
            add(c, 3, v);
        }
    }
}

// --- Action ------------------------------------------------------------------------------

/// `kyviaCore::Action` (MUT gcmn 0x004f8cd0).
fn action(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    match c.b.act_num {
        act::NEUTRAL | act::DRAINED_WAIT | 4 | 5 => {}
        act::DAMAGE | act::DAMAGE2 => {
            if c.b.anm_status != 0 {
                c.b.change_action(cx, act::NEUTRAL, 0, true);
            }
            c.b.spd_down(0, 0x40a0_0000);
        }
        act::HOLD => {
            char_hold_on(c, cx, 5000, 5000);
            match c.b.act_proccess {
                0 => {
                    atk_gomora(gs);
                    c.b.act_proccess += 1;
                }
                1 => {
                    if get_gomora_state(gs, 0) {
                        c.b.change_action(cx, act::NEUTRAL, 1, true);
                    }
                    c.b.spd_down(0, 0x40a0_0000);
                }
                _ => {}
            }
        }
        act::ENTRY_GOMORA => {
            entry_gomora(c, gs, cx);
            c.b.change_action(cx, act::NEUTRAL, 1, true);
        }
        act::START_GOMORA => {
            start_gomora(c, gs, cx);
            c.b.change_action(cx, act::ENTRY_GOMORA, 3, true);
        }
        act::DRAINED => {
            c.b.cheat_hp = 0;
            c.b.change_action(cx, act::DRAINED_WAIT, 3, true);
            c.b.move_spd = 0;
        }
        act::WAVE => {
            match c.b.act_proccess {
                0 => {
                    c.x.anm_w.set("ANM_xx11wave", cx.clips);
                    c.b.act_proccess += 1;
                }
                1 => {
                    let n = c.b.act_count;
                    c.b.act_count += 1;
                    if n == 25 {
                        for m in cx.party.members.into_iter().flatten() {
                            cx.affect(m, 1, 10);
                        }
                    }
                    let done = c.x.anm_w.forward();
                    cx.out(Out::DrawWave);
                    if done {
                        c.b.change_action(cx, act::DRAINED_WAIT, 3, true);
                    }
                }
                _ => {}
            }
            c.b.spd_down(0, 0x40a0_0000);
        }
        act::DEAD => dead(c, gs, cx),
        _ => c.b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// Act 14 (MUT gcmn 0x004f9184): the gomoras killed, the core's particles
/// and fade, its cries; at nothing it has died (`DeadFlg`), exits and waits
/// to rise again.
fn dead(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    let me = c.me;
    match c.b.act_proccess {
        0 => {
            crate::fellow::delete_cmnd(cx.scene, me);
            c.x.aura = false;
            c.x.aura2 = false;
            c.x.aura_stop = false;
            c.x.aura2_stop = false;
            c.x.dead_pos = cx.scene.chars[me].pos;
            c.x.dead_rot = VF0;
            c.x.cone_voice_cou = 0;
            c.b.act_proccess += 1;
        }
        1 => {
            kill_gomora(c, gs, cx);
            if get_gomora_state(gs, 3) {
                c.b.act_proccess = if c.x.kyvia_lv == 5 && c.x.dead_count == 3 { 10 } else { c.b.act_proccess + 1 };
            }
        }
        2 => {
            c.x.off_set = [0, 0, 0x4348_0000, ONE];
            c.x.dead_pos = geom::vadd(c.x.dead_pos, c.x.off_set);
            cx.out(Out::KyviaParticles { which: Gen::Core(1), pos: c.x.dead_pos });
            c.b.act_proccess += 1;
            c.x.off_set[2] = 0;
            dead_fade(c, gs, cx);
        }
        3 => dead_fade(c, gs, cx),
        10 => {
            c.x.alpha = 0;
            c.x.dead_count += 1;
            c.x.dead_flg = 1;
            c.b.exit = 1;
            reset_data(&mut c.x, cx, me);
            c.x.dead_pos = VF0;
            c.b.change_action(cx, act::NEUTRAL, 1, true);
        }
        _ => {}
    }
}

/// Act 14's third step, its fade (falling through from the second).
fn dead_fade(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    let me = c.me;
    c.x.alpha = ee::sub(c.x.alpha, 0x3ca3_d70a);
    if c.x.ex == 0 {
        core_voice(c, cx, 1);
    }
    c.x.dead_rot = cx.cam.rot;
    let n = c.b.act_count;
    c.b.act_count += 1;
    if n == 30 {
        cx.out(Out::KyviaParticles { which: Gen::Core(2), pos: c.x.dead_pos });
    }
    if ee::le(c.x.alpha, 0) {
        c.x.alpha = 0;
        c.x.dead_count += 1;
        c.x.dead_flg = 1;
        c.b.exit = 1;
        reset_data(&mut c.x, cx, me);
        c.x.dead_pos = VF0;
        step_all_gomora_list(c, gs, cx);
        c.b.change_action(cx, act::NEUTRAL, 1, true);
    }
}

/// `CharHoldON(DiscPos, x, y)` (MUT gcmn 0x004fb1f0): each member within
/// `x` and `y` of the disc held (affect 5, -1).
fn char_hold_on(c: &mut Part<Core>, cx: &mut Cx, rx: i32, ry: i32) {
    for m in cx.party.members.into_iter().flatten() {
        if !cx.valid(Some(m)) {
            continue;
        }
        let p = cx.scene.chars[m].pos;
        let dx = fabs(ee::sub(p[0], c.x.disc_pos[0]));
        let dy = fabs(ee::sub(p[1], c.x.disc_pos[1]));
        if ee::lt(dx, ee::from_int(rx)) && ee::lt(dy, ee::from_int(ry)) {
            cx.affect(m, 5, -1);
        }
    }
}

// --- Affect ------------------------------------------------------------------------------

/// `kyviaCore::Affect` (MUT gcmn 0x004f99c0): [`super::part_affect`], then
/// a hit of the kind its attribute resists shows the shield (physical on
/// attribute 0, magic on 1).
pub(crate) fn affect(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let t = cx.scene.chars[me].affect.ty;
    if t != 21 && (b.act_forbid == 2 || b.lock_player != 0) {
        return;
    }
    let hit = super::part_affect(b, cx);
    let Class::KyviaCore(x) = &b.class else { return };
    if !hit || x.ex != 0 {
        return;
    }
    let sid = i32::from(cx.scene.chars[me].affect.param[1]);
    let k = crate::skill::check_type_of(cx.t, sid);
    let magic = match (k, x.attr) {
        (0, 0) => 0,
        (1, 1) => 1,
        _ => return,
    };
    cx.ev.push(crate::event::Event::ResistantShield { on: crate::event::Who::Char(me), magic });
}

// --- the gomoras -------------------------------------------------------------------------

/// `SetCoreState(n)` (MUT gcmn 0x004fc300): 7 fades the core and its
/// gomoras back in, its aura on; 8 fades them out, the core off the party's
/// targets.
pub(crate) fn set_core_state(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx, n: i32) {
    let me = c.me;
    match n {
        7 => {
            if c.x.core_state != 7 {
                fade_gomoras(gs, cx, 5);
                c.x.core_state = 7;
                if c.x.attr == 0 {
                    c.x.aura_stop = false;
                } else {
                    c.x.aura2_stop = false;
                }
            }
            c.x.fade_flg = 0;
        }
        8 => {
            // `CoreState != 8 || CoreState != 6`: always.
            c.x.temp_state = c.x.core_state;
            c.x.time_flg = 0;
            as_me(cx, me, |cx| c.b.erase_cmnd_target(cx));
            c.x.core_state = 8;
            fade_gomoras(gs, cx, 6);
            c.x.fade_flg = 0;
        }
        _ => {}
    }
    c.x.core_state = n;
}

/// `FadeOutGomora` (6) and `FadeInGomora` (5): each gomora out (not waiting
/// or dead) set so.
fn fade_gomoras(gs: &mut [Part<Gomora>], cx: &mut Cx, n: i32) {
    for g in gs.iter_mut() {
        if !matches!(g.x.state, 2 | 3) {
            let me = g.me;
            as_me(cx, me, |cx| gomora::set_gomora_state(g, cx, n));
        }
    }
}

/// `GetActNum` (MUT gcmn 0x004fc4c0): the core at rest (act 0 or 14) and
/// its gomoras too.
pub(crate) fn get_act_num(c: &Part<Core>, gs: &[Part<Gomora>]) -> bool {
    let rest = matches!(c.b.act_num, act::NEUTRAL | act::DEAD);
    let g = get_gomora_state(gs, 0);
    rest && g
}

/// `FadeCheck` (MUT gcmn 0x004fb550): every gomora waiting, dead or done
/// fading, and the core done.
pub(crate) fn fade_check(c: &Part<Core>, gs: &[Part<Gomora>]) -> bool {
    gs.iter().all(|g| matches!(g.x.state, 2 | 3) || g.x.fade_flg != 0) && c.x.fade_flg != 0
}

/// `GetGomoraState(mode)` (MUT gcmn 0x004fb5e0): 0 all at rest (act 0 or
/// 14); 1 and 3 all exited; 2 all exited or alive.
pub(crate) fn get_gomora_state(gs: &[Part<Gomora>], mode: i32) -> bool {
    if gs.is_empty() {
        return false;
    }
    match mode {
        0 => gs.iter().all(|g| matches!(g.b.act_num, 0 | 14)),
        1 | 3 => gs.iter().all(|g| g.b.exit != 0),
        2 => gs.iter().all(|g| g.b.exit != 0 || g.x.life_flg != 0),
        _ => false,
    }
}

/// `KillGomora` (MUT gcmn 0x004fd980): every gomora out and alive to its
/// death (act 14, quietly but for the first's cry); how many.
fn kill_gomora(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) -> i32 {
    c.x.gomora_count = 0;
    c.x.gomora_hold = 0;
    c.x.gomora_meter = 0;
    let mut n = 0;
    let mut first = true;
    let pos = cx.scene.chars[c.me].pos;
    for g in gs.iter_mut() {
        if g.b.exit != 0 || matches!(g.x.state, 2 | 3) {
            continue;
        }
        let me = g.me;
        cx.scene.chars[me].hp = 0;
        as_me(cx, me, |cx| g.b.change_action(cx, gomora::act::DEAD, 2, true));
        g.x.dead_voice_flg = 0;
        if first {
            cx.out(Out::Se3dNote { se: 194, pos, note: 48 });
            first = false;
        }
        n += 1;
    }
    n
}

/// `AtkGomora` (MUT gcmn 0x004fdad0): every gomora out charges.
fn atk_gomora(gs: &mut [Part<Gomora>]) {
    for g in gs.iter_mut() {
        if !matches!(g.x.state, 2 | 3) {
            g.x.tmp_atk_flg = 1;
        }
    }
}

/// `StepGomoraList` (MUT gcmn 0x004fd870): each gomora's next list.
fn step_gomora_list(gs: &mut [Part<Gomora>], cx: &mut Cx) {
    for g in gs.iter_mut() {
        gomora::next_g_list(g, cx);
    }
}

/// `StepAllGomoraList` (MUT gcmn 0x004fd8e0): the lists of the core's next
/// life.
fn step_all_gomora_list(c: &Part<Core>, gs: &mut [Part<Gomora>], cx: &Cx) {
    for g in gs.iter_mut() {
        gomora::list_step_up(g, &cx.data.kyvia, c.x.dead_count);
        g.x.kyvia_step += 1;
    }
}

/// `EntryGomora` (MUT gcmn 0x004fdc90): the first gomora with an attribute
/// that is waiting or dead gets the disc; the first exited one comes out
/// (`EntrySlave(0, 1)`), which is `NowGomoraNum`.
fn entry_gomora(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    let data = &cx.data.kyvia;
    let Some(k) = gs.iter_mut().position(|g| gomora::get_attribute(g, data) != 4 && matches!(g.x.state, 2 | 3)) else {
        return;
    };
    gs[k].x.disc_pos = c.x.disc_pos;
    c.x.now_gomora_num = entry_slave(c, gs, cx, 1);
}

/// `kyviaCore::EntrySlave(0, on)` (MUT gcmn 0x004fd700): the first exited
/// gomora out at the core, ordered `on`: its slave number, or -1.
fn entry_slave(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx, on: i32) -> i32 {
    let core_pos = cx.scene.chars[c.me].pos;
    for g in gs.iter_mut() {
        if g.b.exit == 0 {
            continue;
        }
        // SetMasterPos: the core's place (EX's GomoraKpos is not ported).
        let pos = geom::vadd(core_pos, g.x.off_set);
        cx.scene.chars[g.me].pos = pos;
        g.b.exit = 0;
        g.x.order_num = on;
        return g.x.slave_id;
    }
    -1
}

/// `StartGomora` (MUT gcmn 0x004fdda0): the gomora `NowGomoraNum` on its way
/// out (state 1), if it has an attribute.
fn start_gomora(c: &mut Part<Core>, gs: &mut [Part<Gomora>], cx: &mut Cx) {
    let data = &cx.data.kyvia;
    let Some(g) = usize::try_from(c.x.now_gomora_num).ok().and_then(|k| gs.get_mut(k)) else { return };
    if gomora::get_attribute(g, data) != 4 {
        g.x.time_flg = 1;
        g.x.state = 1;
    }
}

/// `CheckGomoraSkillAct(id)` (MUT gcmn 0x004fde50): another gomora is in its
/// skill (act 4).
pub(crate) fn check_gomora_skill_act(acts: &[i16], id: i32) -> bool {
    acts.iter().enumerate().any(|(k, &a)| k as i32 != id && a == gomora::act::SKILL)
}
