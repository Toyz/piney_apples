//! `kyviaGomora` (kyviagomora.cpp, MUT gcmn 0x004fdf10-0x00503980): the
//! core's five gomoras. Each leaves the core on a spline, circles the disc
//! and, as its attack time runs out, acts by its attribute from the core's
//! lists: 0 heals the core (skill 295), 1 and 2 cast one of Kyvia's skills,
//! 3 dashes at a member (boss skill 35). The EX mode is left out.
//! docs/engine/boss-kyvia.md.

use piney_data::field::ee;

use super::{Gen, KyviaData, Part, as_me, catmull, dp, fp, plain_boss, rand_abs, set_base_param, wrap};
use crate::boss::{Anm, Boss, Class, Cx, Out, VF0};
use crate::enemy_ai::rand_f;
use crate::geom::{self, V4};
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;

/// The gomora's acts (`actNum`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DASH: i16 = 3;
    /// Casting its skill (`CheckGomoraSkillAct` looks for it).
    pub const SKILL: i16 = 4;
    pub const DEAD: i16 = 14;
}

/// `kyviaGomora`'s own members.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gomora {
    /// `master` (the core's character), `slaveID`, `orderNum`.
    pub master: usize,
    pub slave_id: i32,
    pub order_num: i32,
    pub ripus_num: i32,
    pub st_flg: u8,
    pub alpha: F,
    pub end_flg: i32,
    pub no: i32,
    pub time: F,
    pub time_count: F,
    pub time_flg: i32,
    pub time_mode: i32,
    /// 2 waiting, 1 coming out, 0 out, 3 dead, 6 fading out, 4 faded, 5
    /// fading in.
    pub state: i32,
    pub tmp_state: i32,
    pub eny_int: i32,
    pub eny_float: F,
    pub af_cou: i32,
    pub af_timing: i32,
    pub atk_wait: i32,
    pub atk_time: i32,
    pub tmp_atk_flg: i32,
    pub atk_flg: i32,
    pub atk_tmp_flg: i32,
    pub dp_cou: i32,
    /// `MasterList` (which of `AllGomoraList_lv`), `NowListNum`.
    pub master_list: usize,
    pub now_list_num: usize,
    pub my_attribute: i32,
    /// `MyAuraGp` made, `AuraLv`.
    pub my_aura: bool,
    pub aura_lv: i32,
    pub dead_voice_flg: u8,
    pub stop_move_count: i32,
    pub kyvia_lv: i16,
    pub kyvia_step: i16,
    pub life_flg: u8,
    pub fade_flg: u8,
    pub move_pattern: i32,
    pub quake_vector: V4,
    pub sp_vec: [V4; 4],
    pub atk_vec: [V4; 8],
    pub dp_vec: [V4; 4],
    pub departure: V4,
    pub off_set: V4,
    pub disc_pos: V4,
    pub aura_pos: V4,
    pub atk_tar_pos: V4,
    /// `AtkTarBase->height`.
    pub atk_tar_height: F,
    /// `bodyHit` enabled (`HitEnable`/`HitDisable`).
    pub hit_on: bool,
    pub anm_w: Anm,
}

/// `kyviaGomora::kyviaGomora` (MUT gcmn 0x004ff220, `InitData` 0x004fdf10),
/// `GomoraInit(lv)` (0x004fe460) and `SetMaster(core, k)`.
pub(crate) fn new(cx: &mut Cx, me: usize, master: usize, k: i32, lv: i16) -> Boss {
    let mut b = plain_boss();
    let x = Gomora {
        master,
        slave_id: k,
        st_flg: 1,
        end_flg: 1,
        time_count: 0x3ba3_d70a,
        time_mode: 1,
        state: 2,
        af_timing: 2,
        dead_voice_flg: 1,
        kyvia_lv: lv,
        kyvia_step: 1,
        quake_vector: [0x4120_0000, 0x4120_0000, 0x41a0_0000, ONE],
        sp_vec: [VF0; 4],
        atk_vec: [VF0; 8],
        dp_vec: [VF0; 4],
        departure: VF0,
        off_set: VF0,
        anm_w: Anm::new(),
        ..Gomora::default()
    };
    set_base_param(cx, me, 16);
    b.anm_tbl = cx.data.kyvia.gomora_anims.clone();
    cx.scene.chars[me].pos = VF0;
    b.dirc = VF0;
    b.dirc[0] = 0x3fc9_0fdb;
    b.anm.set("ANM_ex0gnut0", cx.clips);
    b.change_action(cx, act::NEUTRAL, 0, true);
    let mut x = x;
    x.anm_w.set("ANM_xx11wave", cx.clips);
    b.class = Class::Gomora(Box::new(x));
    b
}

/// `ResetData` (MUT gcmn 0x004fe260).
fn reset_data(x: &mut Gomora) {
    x.st_flg = 1;
    x.alpha = 0;
    x.no = 0;
    x.time = 0;
    x.time_count = 0x3ba3_d70a;
    x.time_flg = 0;
    x.time_mode = 1;
    x.state = 2;
    x.eny_int = 0;
    x.eny_float = 0;
    x.end_flg = 1;
    x.af_cou = 0;
    x.af_timing = 3;
    x.atk_wait = 0;
    x.tmp_atk_flg = 0;
    x.atk_flg = 0;
    x.dp_cou = 0;
    x.my_aura = false;
    x.aura_lv = 0;
    x.stop_move_count = 0;
    x.dead_voice_flg = 1;
    x.sp_vec = [VF0; 4];
}

/// `MasterList[n]`: the list `n` of the gomora's `AllGomoraList_lv` step,
/// or none past them (the null).
fn list_at(x: &Gomora, data: &KyviaData, n: usize) -> Option<[i16; 5]> {
    data.gomora_lists_of(x.kyvia_lv, x.master_list)?.get(n).copied()
}

/// `GetAttribute` (MUT gcmn 0x004fe6c0): its attribute in the list now.
pub(crate) fn get_attribute(g: &mut Part<Gomora>, data: &KyviaData) -> i32 {
    if list_at(&g.x, data, g.x.now_list_num).is_none() {
        g.x.now_list_num = 0;
    }
    attribute_of(&g.x, data)
}

/// Its slave's attribute in `MasterList[NowListNum]`, 4 (none) past them.
fn attribute_of(x: &Gomora, data: &KyviaData) -> i32 {
    list_at(x, data, x.now_list_num).and_then(|l| l.get(x.slave_id as usize).copied()).map_or(4, i32::from)
}

/// `NextGList` (MUT gcmn 0x00500be0): the next list, or the first again.
pub(crate) fn next_g_list(g: &mut Part<Gomora>, cx: &Cx) {
    if list_at(&g.x, &cx.data.kyvia, g.x.now_list_num + 1).is_some() {
        g.x.now_list_num += 1;
    } else {
        g.x.now_list_num = 0;
    }
}

/// `ListStepUp(n)` (MUT gcmn 0x00500c50, OUT 0x004fd810):
/// `AllGomoraList_lv[n]` when it is not null (level 1 has only the first,
/// level 2 two).
pub(crate) fn list_step_up(g: &mut Part<Gomora>, data: &KyviaData, n: i32) {
    if let Ok(n) = usize::try_from(n)
        && data.gomora_lists_of(g.x.kyvia_lv, n).is_some()
    {
        g.x.master_list = n;
    }
}

/// `SetGomoraState(n)` (MUT gcmn 0x004ff3d0): 6 fades it out, off the
/// party's targets; 5 fades it back.
pub(crate) fn set_gomora_state(g: &mut Part<Gomora>, cx: &mut Cx, n: i32) {
    match n {
        6 => {
            g.x.time_flg = 0;
            g.x.tmp_state = g.x.state;
            g.x.fade_flg = 0;
            g.b.erase_cmnd_target(cx);
        }
        5 => g.x.fade_flg = 0,
        _ => {}
    }
    g.x.state = n;
}

// --- the frame ---------------------------------------------------------------------------

/// `kyviaGomora::Main` (MUT gcmn 0x005004d0). `acts` are the gomoras' acts
/// now (`CheckGomoraSkillAct`).
pub(crate) fn main(g: &mut Part<Gomora>, core: usize, acts: &[i16], cx: &mut Cx) {
    let _ = core;
    let me = g.me;
    as_me(cx, me, |cx| {
        if g.b.exit != 0 {
            return;
        }
        super::head(&mut g.b, cx);
        fade_gomora(g, cx);
        think(g, acts, cx);
        action(g, cx);
        move_(g, cx);
        if g.b.draw_sw != 0 {
            g.b.anm_status = i8::from(g.b.anm.forward());
            g.b.transparency = g.x.alpha;
            g.b.set_transparency = g.x.alpha;
        }
    });
}

/// `kyviaGomora::Move` (MUT gcmn 0x00500850): `ccBoss::Move`'s step, then
/// its body hit 100 below it pushed out of the others. The push (gomoras
/// on each other) is not ported.
fn move_(g: &mut Part<Gomora>, cx: &mut Cx) {
    let me = g.me;
    let mut pp = cx.world.frame.w2p(cx.scene.chars[me].pos);
    let s = g.b.move_spd;
    if !geom::eq(0, s) {
        pp[0] = ee::add(pp[0], ee::mul(s, piney_data::libm::sinf(g.b.move_dirc)));
        pp[1] = ee::sub(pp[1], ee::mul(s, piney_data::libm::cosf(g.b.move_dirc)));
    }
    pp = geom::vadd(pp, g.b.move_vector);
    let ch = &mut cx.scene.chars[me];
    ch.pos_p = pp;
    ch.pos = cx.world.frame.p2w(pp);
}

/// `FadeGomora` (MUT gcmn 0x00502220).
fn fade_gomora(g: &mut Part<Gomora>, cx: &mut Cx) {
    let x = &mut g.x;
    if x.time_flg != 0 {
        if g.b.act_num == act::DEAD {
            x.alpha = ee::sub(x.alpha, 0x3d4c_cccd);
            if ee::lt(x.alpha, 0) {
                x.alpha = 0;
            }
        }
        return;
    }
    match x.state {
        1 => {
            x.alpha = ee::add(x.alpha, 0x3d4c_cccd);
            if !ee::lt(x.alpha, ONE) {
                x.alpha = ONE;
            }
        }
        5 => {
            x.alpha = ee::add(x.alpha, 0x3d4c_cccd);
            if !ee::le(x.alpha, ONE) {
                if x.tmp_state == 0 {
                    x.time_flg = 1;
                    x.state = 0;
                    x.fade_flg = 1;
                    g.b.entry_cmnd_target(cx);
                } else {
                    x.state = x.tmp_state;
                    x.fade_flg = 1;
                }
                x.alpha = ONE;
            }
        }
        6 => {
            x.alpha = ee::sub(x.alpha, 0x3d4c_cccd);
            if ee::lt(x.alpha, 0) {
                x.alpha = 0;
                x.state = 4;
                x.fade_flg = 1;
            }
        }
        _ => {}
    }
}

/// `kyviaGomora::Think` (MUT gcmn 0x00500d50).
fn think(g: &mut Part<Gomora>, acts: &[i16], cx: &mut Cx) {
    let me = g.me;
    g.x.af_cou += 1;
    if g.x.af_timing < g.x.af_cou {
        g.x.af_cou = 0;
    }
    if g.x.st_flg == 1 {
        g.b.dirc[1] = wrap(ee::sub(g.b.dirc[1], 0x3e99_999a));
        match g.x.dp_cou {
            0 => {
                let ok = start_init(g, cx);
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 193, pos, note: 56 });
                if ok {
                    g.x.dp_cou = 1;
                }
            }
            1 => {
                g.x.alpha = ee::add(g.x.alpha, 0x3ca3_d70a);
                if !ee::lt(g.x.alpha, ONE) {
                    g.x.alpha = ONE;
                }
                if departure_move(g, cx) {
                    g.x.dp_cou = 2;
                }
            }
            2 => {
                g.x.st_flg = 0;
                set_sp_point(g, cx);
                g.x.state = 0;
                g.x.alpha = ONE;
                g.x.life_flg = 1;
                hit(g, cx, true);
                crate::fellow::entry_cmnd(cx.scene, me);
                g.x.dp_cou = 3;
                let r = rand_abs(cx, 0x3ba3_d70a);
                g.x.time_count = fp(dp(0x3ba3_d70a) + dp(r));
                g.x.tmp_atk_flg = 0;
            }
            _ => {}
        }
        return;
    }
    check_count_atk_time(g, acts, cx);
    g.b.dirc[1] = wrap(ee::sub(g.b.dirc[1], 0x3da3_d70a));
    match g.b.act_num {
        act::NEUTRAL => {
            let t = cx.scene.chars[me].target_char;
            if cx.valid(t) && cx.scene.chars[me].cond[cond::HOLD] == 0 && cx.scene.chars[me].skill_status <= 0 {
                if g.x.end_flg == 0 {
                    g.x.end_flg = 1;
                    g.x.no += 1;
                } else if g.x.end_flg == 1 {
                    let t = rnd_target(g, cx);
                    cx.scene.chars[me].target_char = t;
                    g.x.end_flg = 2;
                }
                move_gomora(g, cx);
                // SwitchActionPattern (MUT gcmn 0x005024a0).
                if g.x.state == 0 {
                    if g.x.tmp_atk_flg != 0 {
                        g.x.atk_flg = g.x.tmp_atk_flg;
                    }
                    if g.x.atk_flg != 0 {
                        g.b.change_action(cx, act::DASH, 0, true);
                    }
                }
            } else {
                g.x.stop_move_count += 1;
                // TargetSetting (MUT gcmn 0x00501280).
                if g.x.stop_move_count == 150 {
                    let t = rnd_target(g, cx);
                    cx.scene.chars[me].target_char = t;
                }
                if g.x.time_flg == 0 {
                    g.x.time_flg = 1;
                }
            }
        }
        act::DASH => {
            if g.x.af_cou == g.x.af_timing {
                cx.out(Out::AfterImage);
            }
        }
        1 | 2 | 4 | 5 | 11..=14 => {}
        _ => g.b.change_action(cx, act::NEUTRAL, 0, true),
    }
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// `HitEnable` / `HitDisable` of the gomora's `bodyHit`.
fn hit(g: &mut Part<Gomora>, cx: &mut Cx, on: bool) {
    g.x.hit_on = on;
    cx.out(Out::PartHit { who: g.me, on });
}

/// `StartInit` (MUT gcmn 0x004fe740): its attribute from the list (4: none,
/// back to waiting), its row and attack time by level, then out of the
/// core: a spline up and away from it, its particles. False for none.
fn start_init(g: &mut Part<Gomora>, cx: &mut Cx) -> bool {
    let me = g.me;
    let attr = attribute_of(&g.x, &cx.data.kyvia);
    g.x.my_attribute = attr;
    let lv = g.x.kyvia_lv;
    if attr == 4 {
        reset_data(&mut g.x);
        g.b.exit = 1;
        return false;
    }
    if (0..4).contains(&attr) {
        if (1..=4).contains(&lv) {
            set_base_param(cx, me, 16 + attr as usize + 4 * (lv as usize - 1));
            if attr == 0 {
                (g.x.atk_wait, g.x.ripus_num) = match lv {
                    1 => (270, 200),
                    2 => (390, 400),
                    _ => (360, 600),
                };
            }
        }
        g.x.atk_wait = match attr {
            1 => 270,
            2 => 600,
            3 => 150,
            _ => g.x.atk_wait,
        };
        g.x.atk_time = g.x.atk_wait;
    }
    g.b.draw_sw = 1;
    g.x.state = 1;
    let t = rnd_target(g, cx);
    cx.scene.chars[me].target_char = t;
    g.b.change_action(cx, act::NEUTRAL, 1, true);
    let core = cx.scene.chars[g.x.master].pos;
    cx.scene.chars[me].pos = core;
    let at = geom::vadd(core, [0, 0, 0x4348_0000, ONE]);
    let sign: i32 = if (cx.cc.rand() % 2).abs() != 0 { -1 } else { 1 };
    let turn = rand_f(cx.cc, 0x3fc0_0000);
    let m = geom::rot_matrix_z(&geom::unit_matrix(), turn);
    g.x.dp_vec[0] = core;
    g.x.departure = VF0;
    let z = fp(50.0 + dp(rand_abs(cx, 0x42c8_0000)));
    g.x.departure[2] = z;
    g.x.departure[0] = 0;
    g.x.dp_vec[1] = geom::vadd(geom::apply_matrix(&m, g.x.departure), core);
    let z = fp(100.0 + dp(rand_abs(cx, 0x42c8_0000)));
    g.x.departure[2] = z;
    g.x.departure[0] = 0;
    g.x.dp_vec[2] = geom::vadd(geom::apply_matrix(&m, g.x.departure), core);
    g.x.departure[2] = rand_abs(cx, 0x4248_0000);
    g.x.departure[0] = ee::from_int(sign * 200);
    g.x.dp_vec[3] = geom::vadd(geom::apply_matrix(&m, g.x.departure), core);
    cx.out(Out::KyviaParticles { which: Gen::Gomora(5), pos: at });
    cx.out(Out::KyviaParticles { which: Gen::Gomora(6), pos: at });
    g.x.time_mode = 1;
    true
}

/// `DepartureMove` (MUT gcmn 0x005009a0): along the departure spline, 30
/// steps a frame; true at its end.
fn departure_move(g: &mut Part<Gomora>, cx: &mut Cx) -> bool {
    if !ee::le(g.x.time, ONE) {
        g.x.time = 0;
        g.x.time_mode += 1;
    }
    let d = g.x.dp_vec;
    match g.x.time_mode {
        1 => move_spline(g, cx, [d[0], d[0], d[1], d[2]]),
        2 => move_spline(g, cx, [d[0], d[1], d[2], d[3]]),
        3 => move_spline(g, cx, [d[1], d[2], d[3], d[3]]),
        4 => {
            g.x.time_flg = 1;
            return true;
        }
        _ => {}
    }
    if g.x.time_mode < 4 {
        g.x.time = ee::add(g.x.time, ee::mul(0x41f0_0000, g.x.time_count));
    }
    false
}

/// `MoveSpline(p1, p2, p3, p4, 0)` (MUT gcmn 0x00502530): the Catmull-Rom
/// point at `time`.
fn move_spline(g: &mut Part<Gomora>, cx: &mut Cx, p: [V4; 4]) {
    let t = g.x.time;
    let mut v = [0, 0, 0, ONE];
    for (k, c) in v.iter_mut().take(3).enumerate() {
        *c = catmull(t, [p[0][k], p[1][k], p[2][k], p[3][k]]);
    }
    cx.scene.chars[g.me].pos = v;
}

/// `MoveGomora` (MUT gcmn 0x00501fe0): its ride round the disc, a new path
/// each fourth part.
fn move_gomora(g: &mut Part<Gomora>, cx: &mut Cx) {
    if g.x.state == 0 && g.x.time_flg == 0 {
        g.x.time_flg = 1;
    }
    if g.x.time_flg == 0 {
        return;
    }
    if !ee::le(g.x.time, ONE) {
        g.x.time = 0;
        g.x.time_mode += 1;
    }
    let s = g.x.sp_vec;
    match g.x.time_mode {
        1 => move_spline(g, cx, [s[0], s[0], s[1], s[2]]),
        2 => move_spline(g, cx, [s[0], s[1], s[2], s[3]]),
        3 => move_spline(g, cx, [s[1], s[2], s[3], s[3]]),
        4 => {
            set_sp_point(g, cx);
            g.x.time_mode = 1;
        }
        _ => {
            g.x.time_mode = 1;
            set_sp_point(g, cx);
        }
    }
    g.x.time = ee::add(g.x.time, g.x.time_count);
}

/// `SetSpPoint`'s ranges (`v0`, `s0`-`s4`) by pattern: level 4's first
/// step, its later ones, the other levels.
const SP_RANGES: [[[i32; 6]; 4]; 3] = [
    [
        [650, 100, 650, 200, 350, 100],
        [250, 400, 450, 200, 650, 0],
        [550, 400, 350, 200, 750, 500],
        [550, 400, 350, 400, 450, 400],
    ],
    [
        [450, 500, 450, 650, 300, 350],
        [350, 600, 450, 450, 300, 0],
        [350, 600, 250, 450, 300, 550],
        [450, 600, 250, 450, 100, 550],
    ],
    [
        [650, 100, 650, 500, 250, 300],
        [650, 500, 650, 100, 650, 0],
        [300, 700, 350, 230, 350, 300],
        [300, 200, 250, 400, 250, 200],
    ],
];

/// `SetSpPoint` (MUT gcmn 0x00502670): the next path from where it is to
/// three points round the disc, by one of four patterns.
fn set_sp_point(g: &mut Part<Gomora>, cx: &mut Cx) {
    let a = dp(rand_abs(cx, 0x43af_0000));
    let b = dp(rand_abs(cx, 0x4316_0000));
    let v = geom::apply_matrix(&geom::unit_matrix(), [0, fp(a + b), 0, ONE]);
    let base = geom::vadd(v, g.x.disc_pos);
    g.x.sp_vec = [cx.scene.chars[g.me].pos, base, base, base];
    g.x.move_pattern = (cx.cc.rand() % 4).abs();
    let set = if g.x.kyvia_lv == 4 { usize::from(g.x.kyvia_step != 1) } else { 2 };
    let pat = g.x.move_pattern as usize;
    let [v0, s0, s1, s2, s3, s4] = SP_RANGES[set][pat.min(3)];
    let r = |cx: &mut Cx, n: i32| rand_abs(cx, ee::from_int(n));
    let off = |cx: &mut Cx, base: f64, n: i32| fp(base + dp(r(cx, n)));
    let neg = |cx: &mut Cx, base: f64, n: i32| fp(base + -dp(r(cx, n)));
    let negz = |cx: &mut Cx, n: i32| fp(-dp(r(cx, n)));
    let add = |g: &mut Part<Gomora>, k: usize, v: [F; 3]| {
        g.x.sp_vec[k] = geom::vadd(g.x.sp_vec[k], [v[0], v[1], v[2], ONE]);
    };
    let (z10, z20, z30, z40) = (10, 20, 30, 40);
    match pat {
        0 => {
            let v = [off(cx, 100.0, v0), r(cx, s0), r(cx, z10)];
            add(g, 1, v);
            let v = [neg(cx, -100.0, s1), neg(cx, -100.0, s2), r(cx, z20)];
            add(g, 2, v);
            let v = [r(cx, s3), r(cx, s4), r(cx, z10)];
            add(g, 3, v);
        }
        1 => {
            let v = [neg(cx, -100.0, v0), neg(cx, -100.0, s0), r(cx, z30)];
            add(g, 1, v);
            let v = [off(cx, 100.0, s1), off(cx, 50.0, s2), r(cx, z20)];
            add(g, 2, v);
            let v = [neg(cx, -100.0, s3), neg(cx, -150.0, s2), r(cx, z10)];
            add(g, 3, v);
        }
        2 => {
            let v = [negz(cx, v0), negz(cx, s0), r(cx, z20)];
            add(g, 1, v);
            let v = [r(cx, s1), r(cx, s2), r(cx, z10)];
            add(g, 2, v);
            let v = [neg(cx, -100.0, s3), neg(cx, -250.0, s4), r(cx, z30)];
            add(g, 3, v);
        }
        _ => {
            let v = [off(cx, 100.0, v0), r(cx, s0), r(cx, z30)];
            add(g, 1, v);
            let v = [neg(cx, -100.0, s1), neg(cx, -250.0, s2), r(cx, z40)];
            add(g, 2, v);
            let v = [r(cx, s3), r(cx, s4), r(cx, z30)];
            add(g, 3, v);
        }
    }
}

/// `SetAtkPoint` (MUT gcmn 0x00503640): the dash's eight points, from where
/// it is to its target (half its height down) and back.
fn set_atk_point(g: &mut Part<Gomora>, cx: &mut Cx) {
    let pos = cx.scene.chars[g.me].pos;
    let tar = g.x.atk_tar_pos;
    let a = &mut g.x.atk_vec;
    a[0] = pos;
    a[1] = pos;
    a[2] = pos;
    a[6] = pos;
    a[7] = pos;
    a[3] = tar;
    a[4] = tar;
    a[5] = tar;
    let r = cx.cc.rand() % 4;
    g.x.atk_vec[2][0] = ee::add(g.x.atk_vec[2][0], ee::from_int(15 * r));
    let r = cx.cc.rand() % 4;
    g.x.atk_vec[2][1] = ee::add(g.x.atk_vec[2][1], ee::from_int(15 * r));
    let r = cx.cc.rand() % 6;
    g.x.atk_vec[2][2] = ee::add(g.x.atk_vec[2][2], fp(f64::from(r << 3).abs()));
    let half = ee::div(g.x.atk_tar_height, 0x4000_0000);
    g.x.atk_vec[3][2] = ee::sub(g.x.atk_vec[3][2], half);
    g.x.atk_vec[3] = geom::vadd(g.x.atk_vec[3], VF0);
    g.x.atk_vec[4] = g.x.atk_vec[3];
    g.x.atk_vec[5] = g.x.atk_vec[3];
}

/// `RndTarget` (MUT gcmn 0x00501de0): `SelectTarget(|ccRand() % 15|)`, else
/// the first member standing.
fn rnd_target(g: &mut Part<Gomora>, cx: &mut Cx) -> Option<usize> {
    let ty = (cx.cc.rand() % 15).abs();
    if let Some(t) = g.b.select_target(cx, ty, crate::boss::FAR) {
        return Some(t);
    }
    cx.scene.pc_list.iter().copied().find(|&c| cx.scene.chars[c].hp > 0)
}

/// `TimeAuraCount` (MUT gcmn 0x00501370): its aura grows as its attack
/// time runs down (at 0.8, 0.5 and 0.3 of it).
fn time_aura_count(g: &mut Part<Gomora>, cx: &mut Cx) {
    let (time, wait) = (f64::from(g.x.atk_time), f64::from(g.x.atk_wait));
    let row = g.x.my_attribute as u8;
    let pos = g.x.aura_pos;
    if 0.3 * time > wait && g.x.aura_lv == 2 {
        cx.out(Out::KyviaParticles { which: Gen::GomoraAura(row, 2), pos });
        g.x.my_aura = true;
        g.x.aura_lv += 1;
    } else if 0.5 * time > wait && g.x.aura_lv == 1 {
        cx.out(Out::KyviaParticles { which: Gen::GomoraAura(row, 1), pos });
        g.x.my_aura = true;
        g.x.aura_lv += 1;
    } else if 0.8 * time > wait && g.x.aura_lv == 0 {
        cx.out(Out::KyviaParticles { which: Gen::GomoraAura(row, 0), pos });
        g.x.my_aura = true;
        g.x.aura_lv += 1;
    }
}

/// `CheckCountAtkTime` (MUT gcmn 0x005017c0): at the end of its attack time
/// (and no other gomora casting) it acts by its attribute.
fn check_count_atk_time(g: &mut Part<Gomora>, acts: &[i16], cx: &mut Cx) {
    let me = g.me;
    g.x.aura_pos = cx.scene.chars[me].pos;
    g.x.aura_pos[2] = ee::add(g.x.aura_pos[2], 0x4366_0000);
    if g.x.life_flg == 0 || !matches!(g.b.act_num, 0 | 1) {
        return;
    }
    time_aura_count(g, cx);
    let w = g.x.atk_wait;
    g.x.atk_wait -= 1;
    if w != 0 {
        return;
    }
    if super::core::check_gomora_skill_act(acts, g.x.slave_id) {
        g.x.atk_wait = 60;
        return;
    }
    if cx.scene.pc_list.is_empty() {
        g.x.atk_wait = 2;
        return;
    }
    let pos = g.x.aura_pos;
    let attr = g.x.my_attribute;
    if !(0..4).contains(&attr) {
        return;
    }
    if attr == 1 || attr == 2 {
        let t = rnd_target(g, cx);
        cx.scene.chars[me].target_char = t;
    }
    g.x.my_aura = false;
    if attr != 3 {
        g.x.atk_wait = g.x.atk_time;
        g.x.aura_lv = 0;
    }
    cx.out(Out::KyviaParticles { which: Gen::GomoraAura(4 + attr as u8, 0), pos });
    match attr {
        0 => {
            cx.out(Out::SkillStart { who: me, sid: 295 });
            let master = g.x.master;
            let request = crate::skill::request(cx.t, cx.scene, me, master, 295, 0, false, cx.rand);
            let skill = crate::item::ItemSkill { sid: 295, stype: 0, param: None, compel: false, request };
            let skill = crate::item::ItemSkill { param: skill.request.is_some().then_some(g.x.ripus_num), ..skill };
            cx.out(Out::SkillFrom { user: me, target: master, skill });
            g.b.change_action(cx, act::SKILL, 0, true);
        }
        1 | 2 => {
            let d = &cx.data.kyvia;
            let sid = if attr == 1 {
                let k = (cx.cc.rand() % 7).unsigned_abs() as usize;
                d.various.get(k).copied().unwrap_or(1)
            } else {
                let k = (cx.cc.rand() % 12).unsigned_abs() as usize;
                d.downer.get(k).copied().unwrap_or(1)
            };
            if let Some(t) = cx.scene.chars[me].target_char
                && let Some(skill) =
                    crate::item::item_skill_request(cx.t, cx.scene, me, t, i32::from(sid), 0, false, cx.rand)
            {
                cx.out(Out::SkillFrom { user: me, target: t, skill });
            }
            g.b.change_action(cx, act::SKILL, 0, true);
        }
        _ => {
            g.b.change_action(cx, act::DASH, 0, true);
            g.x.atk_wait = g.x.atk_time;
            g.x.aura_lv = 0;
        }
    }
    g.x.my_aura = false;
}

// --- Action ------------------------------------------------------------------------------

/// `kyviaGomora::Action` (MUT gcmn 0x004ff800).
fn action(g: &mut Part<Gomora>, cx: &mut Cx) {
    let me = g.me;
    match g.b.act_num {
        0 | 12 | 13 => {}
        1 | 2 => {
            if g.b.act_proccess == 0 {
                if g.b.act_num == 1 {
                    let pos = cx.scene.chars[me].pos;
                    cx.out(Out::Se3dNote { se: 194, pos, note: 60 });
                }
                g.b.act_proccess += 1;
            }
            if g.b.anm_status != 0 {
                g.b.change_action(cx, act::NEUTRAL, 0, true);
            }
            g.b.spd_down(0, 0x40a0_0000);
        }
        act::DASH => dash(g, cx),
        act::SKILL => {
            g.b.spd_down(0, 0x40a0_0000);
            g.b.dirc[1] = wrap(ee::sub(g.b.dirc[1], 0x3e23_d70a));
            if g.b.anm_status != 0 {
                g.b.change_action(cx, act::NEUTRAL, 0, true);
            }
        }
        5 => {
            g.b.change_action(cx, act::NEUTRAL, 1, true);
            g.b.spd_down(0, 0x40a0_0000);
        }
        11 => g.b.change_action(cx, act::NEUTRAL, 1, true),
        act::DEAD => dead(g, cx),
        _ => g.b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// Act 3, the dash (MUT gcmn 0x004ff970): a new target in turn (the fifth
/// frame times its slave number), held while it flies out along a spline
/// to it, strikes at 0.72 of the third part and flies back.
fn dash(g: &mut Part<Gomora>, cx: &mut Cx) {
    let me = g.me;
    let mut d = g.b.dirc[1];
    if g.b.act_proccess != 0 {
        d = ee::sub(d, 0x3e99_999a);
    }
    g.b.dirc[1] = wrap(d);
    let t = cx.scene.chars[me].target_char;
    if g.b.act_proccess >= 2
        && let Some(tp) = t.filter(|_| cx.valid(t))
    {
        cx.affect(tp, 5, -1);
    }
    match g.b.act_proccess {
        0 => {
            g.b.act_proccess = 1;
            g.x.eny_float = g.x.time;
            g.x.eny_int = 0;
        }
        1 => {
            let c = g.b.act_count;
            g.b.act_count += 1;
            if i32::from(c) == 5 * g.x.slave_id {
                let t = rnd_target(g, cx);
                cx.scene.chars[me].target_char = t;
                if let Some(tp) = t.filter(|_| cx.valid(t)) {
                    hit(g, cx, false);
                    g.b.act_proccess += 1;
                    g.x.atk_tmp_flg = 1;
                    g.b.act_count = 0;
                    g.x.atk_tar_pos = cx.scene.chars[tp].pos;
                    g.x.atk_tar_height = cx.scene.chars[tp].base().height;
                } else {
                    g.x.eny_int = 7;
                }
            }
        }
        2 => dash_flight(g, cx),
        _ => {}
    }
}

/// The dash's flight (act 3's third step).
fn dash_flight(g: &mut Part<Gomora>, cx: &mut Cx) {
    let me = g.me;
    if !ee::le(g.x.time, ONE) {
        g.x.time = 0;
        g.x.eny_int += 1;
    }
    let a = g.x.atk_vec;
    match g.x.eny_int {
        0 => {
            g.x.time = 0;
            g.x.eny_int += 1;
            g.b.erase_cmnd_target(cx);
            let t = cx.scene.chars[me].target_char;
            if cx.valid(t) {
                set_atk_point(g, cx);
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 64, pos, note: 55 });
            } else {
                g.x.eny_int = 7;
            }
        }
        1 => move_spline(g, cx, [a[0], a[0], a[1], a[2]]),
        2 => move_spline(g, cx, [a[0], a[1], a[2], a[3]]),
        3 => {
            move_spline(g, cx, [a[1], a[2], a[3], a[3]]);
            if g.x.atk_tmp_flg != 0 && !ee::le(g.x.time, 0x3f38_51eb) {
                let pos = cx.scene.chars[me].pos;
                g.x.atk_vec[4] = pos;
                g.x.atk_vec[5] = pos;
                let at = geom::vadd([0, 0, 0x4270_0000, ONE], g.x.atk_vec[3]);
                g.x.atk_tmp_flg = 0;
                cx.out(Out::KyviaParticles { which: Gen::Gomora(0), pos: at });
                let t = cx.scene.chars[me].target_char;
                if cx.valid(t) {
                    let i = match g.x.kyvia_lv {
                        1 | 4 => Some(35),
                        2 => Some(39),
                        3 => Some(54),
                        _ => None,
                    };
                    if let Some(i) = i {
                        g.b.skill_damage(cx, t, None, i);
                    }
                    cx.out(Out::Se3dNote { se: 31, pos, note: 57 });
                }
            }
        }
        4 => {
            let c = g.b.act_count;
            g.b.act_count += 1;
            if c < 21 {
                let q = g.x.quake_vector;
                cx.quake_vec([q[0], q[1], q[2]]);
            }
            move_spline(g, cx, [a[4], a[4], a[5], a[6]]);
        }
        5 => move_spline(g, cx, [a[4], a[5], a[6], a[7]]),
        6 => move_spline(g, cx, [a[5], a[6], a[7], a[7]]),
        7 => {
            g.b.change_action(cx, act::NEUTRAL, 0, true);
            g.x.eny_int = 0;
            g.x.time = g.x.eny_float;
            g.x.atk_flg = 0;
            g.x.tmp_atk_flg = 0;
            hit(g, cx, true);
            g.b.entry_cmnd_target(cx);
        }
        _ => {}
    }
    if g.x.eny_int != 0 {
        g.x.time = ee::add(g.x.time, 0x3df5_c28f);
    }
}

/// Act 14 (MUT gcmn 0x00500250): off the lists, its particles and cry, its
/// fade (`FadeGomora`), then back to waiting, exited.
fn dead(g: &mut Part<Gomora>, cx: &mut Cx) {
    let me = g.me;
    let mut at = cx.scene.chars[me].pos;
    at[2] = ee::add(at[2], 0x437a_0000);
    match g.b.act_proccess {
        0 => {
            crate::fellow::delete_cmnd(cx.scene, me);
            hit(g, cx, false);
            g.x.my_aura = false;
            g.x.life_flg = 0;
            g.x.st_flg = 0;
            g.b.act_proccess += 1;
            cx.out(Out::KyviaParticles { which: Gen::Gomora(1), pos: at });
            g.x.state = 3;
            if g.x.dead_voice_flg != 0 {
                let pos = cx.scene.chars[me].pos;
                cx.out(Out::Se3dNote { se: 194, pos, note: 48 });
            }
        }
        1 => {
            if ee::le(g.x.alpha, 0) {
                g.x.alpha = ONE;
                g.b.act_proccess += 1;
                cx.out(Out::KyviaParticles { which: Gen::Gomora(2), pos: at });
            }
        }
        2 => {
            reset_data(&mut g.x);
            g.b.exit = 1;
            cx.scene.chars[me].pos = VF0;
            g.x.state = 3;
        }
        _ => {}
    }
}

/// `kyviaGomora::Affect` (MUT gcmn 0x004ff480): [`super::part_affect`].
pub(crate) fn affect(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let t = cx.scene.chars[me].affect.ty;
    if t != 21 && (b.act_forbid == 2 || b.lock_player != 0) {
        return;
    }
    super::part_affect(b, cx);
}
