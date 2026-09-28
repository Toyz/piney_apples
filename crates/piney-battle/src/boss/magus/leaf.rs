//! `ccBoss03Leaf` (boss03.cpp, MUT gcmn 0x004a46a0-0x004a6f00) and its
//! burst (`ccBoss03LeafExplode`, 0x004a6f00-0x004a744c): Magus's twelve
//! leaves. A leaf falls from where it hangs on the body (order 24), lands
//! on the command lists, counts 450 frames down and bursts; killed first,
//! it fades and goes. docs/engine/boss-magus.md.

use piney_data::field::ee;
use piney_data::libm;

use super::{LEAF_ROW, LeafPart, Leaves, Magus, Pic};
use crate::boss::kyvia::{as_me, new_char, plain_boss, set_base_param};
use crate::boss::{Anm, Boss, Class, Cx, EffKind, Out};
use crate::geom::{self, V4};
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;

/// A leaf's acts.
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DIE: i16 = 14;
    pub const DROP: i16 = 19;
    pub const FIRE: i16 = 21;
    pub const COUNT_DOWN: i16 = 22;
    pub const DESTROY: i16 = 23;
}

/// `DRAWSTATUS`: one of the burst's two pictures' fade and growth.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawStatus {
    pub alpha: F,
    pub base_alpha: F,
    pub alpha_spd: F,
    pub scale_max: F,
    pub scale_spd: F,
    pub scale: F,
}

impl DrawStatus {
    /// A step of `ccBoss03LeafExplode::Draw` (0x004a7100): grown to its
    /// most, then fading; the alpha held to 0..1.
    fn step(&mut self) {
        self.scale = ee::add(self.scale, self.scale_spd);
        if !ee::lt(self.scale, self.scale_max) {
            self.alpha = ee::add(self.alpha, self.alpha_spd);
        }
        if !ee::le(self.alpha, ONE) {
            self.alpha = ONE;
        } else if ee::lt(self.alpha, 0) {
            self.alpha = 0;
        }
    }
}

/// `ccBoss03LeafExplode` (0x70 bytes): `EFF_ex31exp2` and `OBJ_ex31exp1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Explode {
    pub pos: V4,
    pub dirc: V4,
    pub eff: DrawStatus,
    pub exp: DrawStatus,
}

impl Explode {
    /// `Init` (0x004a7090).
    fn init(&mut self) {
        self.eff = DrawStatus {
            alpha: 0x3f4c_cccd,
            base_alpha: 0x3f4c_cccd,
            alpha_spd: 0xbd4c_cccd,
            scale_max: 0x4270_0000,
            scale_spd: 0x40a0_0000,
            scale: 0,
        };
        self.exp = DrawStatus { alpha: ONE, ..self.eff };
    }

    /// `Draw` (0x004a7100): true once the flash has faded out.
    fn draw(&mut self) -> bool {
        self.eff.step();
        self.exp.step();
        geom::eq(self.exp.alpha, 0)
    }
}

/// `ccBoss03Leaf`'s own members (+0x29350 on) and its `ccBoss` links.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Leaf {
    /// `master` (Magus's character), `slaveID` (`SetMaster`), `orderNum`
    /// (`Order`).
    pub master: usize,
    pub slave_id: i32,
    pub order_num: i32,
    /// `m_leafMarker`: `m_bInitAtOnce` (its smoke made) and `m_forbid`.
    pub marker_init: bool,
    pub marker_forbid: i32,
    pub marker_alpha: F,
    pub exp: Explode,
    /// `m_charge`: the grow's particles run.
    pub charge: bool,
    pub act_cnt: i32,
    pub anm_end: i32,
    pub exp_cntr: i32,
    pub exp_alpha: F,
    pub exp_alpha_spd: F,
    pub exp_base_alpha: F,
    pub exp_scale: F,
    pub exp_scale_spd: F,
    pub vec_rot: V4,
    pub prepare_explode: i32,
    pub drop: i32,
    pub blink_interval: i32,
    pub blink_base_time: i32,
    pub blink: i32,
    pub explode: i32,
    pub shock_scale: F,
    pub transparency: F,
    pub trans_spd: F,
    pub exit_alpha: F,
}

// --- the constructor ------------------------------------------------------------------------

/// `ccBoss03Leaf::ccBoss03Leaf` (0x004a46a0) and `SetMaster(boss, k)`: a
/// character of `bossTbl` row 11 on no list, exited, its clip
/// `Boss03SlaveAnmTbl[0]`. The character's index.
pub(super) fn new(cx: &mut Cx, k: i32) -> usize {
    let me = new_char(cx, LEAF_ROW);
    set_base_param(cx, me, LEAF_ROW);
    let mut b = plain_boss();
    b.cheat_hp = 0;
    b.anm_tbl = cx.data.magus.leaf_anims.clone();
    b.anm = Anm::new();
    b.change_action(cx, act::NEUTRAL, 3, true);
    b.anm_status = 0;
    // The marker (a ccAnimateObject) starts opaque.
    let x = Leaf { master: cx.me, slave_id: k, marker_alpha: ONE, ..Leaf::default() };
    b.class = Class::MagusLeaf(Box::new(x));
    if let Some(f) = cx.scene.chars[me].foe_state_mut() {
        f.boss = Some(Box::new(b));
    }
    me
}

// --- the frame --------------------------------------------------------------------------------

/// `ccBoss03Leaf::Main` (0x004a4f30), run from Magus's `Slave()`.
pub(super) fn main(ls: &mut Leaves, k: usize, mb: &mut Boss, mx: &mut Magus, cx: &mut Cx) {
    let me = ls.0[k].me;
    as_me(cx, me, |cx| {
        if ls.0[k].b.exit != 0 {
            return;
        }
        crate::chara::calc_real(cx.t, &mut cx.scene.chars[me], 0, cx.env, &mut cx.ev);
        ls.0[k].b.calc_target_info(cx);
        let b = &mut ls.0[k].b;
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
        think(ls, k, mb, mx, cx);
        mov(&mut ls.0[k].b, cx);
        if ls.0[k].b.draw_sw != 0 {
            pre_draw_anm(&mut ls.0[k], cx);
        }
    });
}

/// `ccBoss03Leaf::Move` (0x004a4e00): `ccBoss::Move`.
fn mov(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let mut pp = crate::boss::w2p(cx, cx.scene.chars[me].pos);
    let spd = b.move_spd;
    if !geom::eq(0, spd) {
        pp[0] = ee::add(pp[0], ee::mul(spd, libm::sinf(b.move_dirc)));
        pp[1] = ee::sub(pp[1], ee::mul(spd, libm::cosf(b.move_dirc)));
    }
    pp = geom::vadd(pp, b.move_vector);
    let mut pos = crate::boss::p2w(cx, pp);
    if b.body_hit_sw != 0
        && let Some(mut off) = (cx.collide)(pos)
    {
        off[2] = 0;
        pos = geom::vadd(pos, off);
        pp = geom::vadd(pp, off);
    }
    let ch = &mut cx.scene.chars[me];
    ch.pos_p = pp;
    ch.pos = pos;
}

/// `ccBoss03Leaf::PreDrawAnm` (0x004a5080): the clip forward; bursting,
/// the burst's picture instead; fading, the leaf's fade.
fn pre_draw_anm(p: &mut LeafPart, _cx: &mut Cx) {
    let (b, x) = (&mut p.b, &mut p.x);
    match b.act_num {
        act::FIRE => {
            b.anm_status = i8::from(x.exp.draw());
        }
        act::DESTROY => {
            x.transparency = ee::sub(x.transparency, x.trans_spd);
            if ee::lt(x.transparency, 0) {
                x.transparency = 0;
            }
            b.transparency = x.transparency;
            b.set_transparency = x.transparency;
            b.anm_status = i8::from(b.anm.forward());
        }
        _ => b.anm_status = i8::from(b.anm.forward()),
    }
}

/// `ccBoss03Leaf::Think` (0x004a54b0).
fn think(ls: &mut Leaves, k: usize, mb: &mut Boss, mx: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    match ls.0[k].b.act_num {
        act::NEUTRAL => {
            if ls.0[k].x.order_num == 0 {
                return;
            }
            let mp = cx.scene.chars[ls.0[k].x.master].pos;
            cx.scene.chars[me].pos = mp;
            let p = &mut ls.0[k];
            p.b.dirc = mb.dirc;
            p.b.draw_sw = 1;
            p.x.act_cnt = 0;
            change_action(p, cx, act::DROP, 3, true);
        }
        7 => {
            let c = ls.0[k].b.act_count;
            ls.0[k].b.act_count += 1;
            if c >= 121 {
                on_exit(ls, k, mb, mx, cx);
                let p = &mut ls.0[k];
                p.b.draw_sw = 0;
                change_action(p, cx, act::NEUTRAL, 3, true);
            }
        }
        act::DIE => on_die(ls, k, mb, mx, cx),
        act::DROP => on_drop(&mut ls.0[k], cx),
        act::FIRE => on_fire(ls, k, mb, mx, cx),
        act::COUNT_DOWN => on_count_down(ls, k, mb, mx, cx),
        act::DESTROY if ls.0[k].b.anm_status != 0 => on_exit(ls, k, mb, mx, cx),
        _ => {}
    }
}

/// `ccBoss03Leaf::ChangeAction` (0x004a6e00): the base's; bursting, the
/// burst's clip (`xeffect`'s `ANM_ex31exp1`); `anmStatus` 0.
fn change_action(p: &mut LeafPart, cx: &mut Cx, n: i16, forbid: i16, af: bool) {
    p.b.change_action(cx, n, forbid, af);
    if af && p.b.act_num == act::FIRE {
        p.b.anm.set("ANM_ex31exp1", cx.clips);
    }
    p.b.anm_status = 0;
}

/// `CheckBlink` (0x004a6e80): the blink's time out, it flips.
fn check_blink(x: &mut Leaf) -> bool {
    let c = x.blink_interval;
    x.blink_interval = c - 1;
    if c > 0 {
        return false;
    }
    x.blink_interval = 0;
    x.blink ^= 1;
    true
}

/// `SetBlinkTime(t)` (0x004a6ee0).
fn set_blink_time(x: &mut Leaf, t: i32) {
    x.blink_interval = t;
    x.blink_base_time = t;
}

/// The countdown's start (orders 25, and the landing): 450 frames.
fn count_start(x: &mut Leaf) {
    set_blink_time(x, 30);
    x.exp_cntr = 450;
    x.exp_alpha = 0;
    x.exp_base_alpha = 0;
    x.exp_alpha_spd = ee::div(0x42c8_0000, ee::from_int(x.exp_cntr));
    x.act_cnt = 0;
    x.prepare_explode = 0;
    x.exp_scale = ONE;
    x.exp_scale_spd = ee::div(ONE, ee::from_int(x.exp_cntr));
    set_blink_time(x, 30);
}

/// `Init` (0x004a5920): at the body's leaf `slaveID` on the ground, the
/// body's heading, its fall clip, full HP, fallen.
fn init(p: &mut LeafPart, mb: &Boss, mx: &Magus, cx: &mut Cx) {
    let me = p.me;
    let k = usize::try_from(p.x.slave_id).unwrap_or(0).min(super::LEAVES - 1);
    let mut pos = mx.leaf_pos[k];
    pos[2] = 0;
    cx.scene.chars[me].pos = pos;
    p.b.dirc = mb.dirc;
    p.x.vec_rot = p.b.dirc;
    let clip = if p.x.slave_id & 1 != 0 { "ANM_ex31atc1" } else { "ANM_ex31atc0" };
    p.b.anm.set(clip, cx.clips);
    p.x.anm_end = 0;
    let ch = &mut cx.scene.chars[me];
    ch.hp = ch.max_hp;
    p.x.drop = 1;
    p.x.explode = 0;
}

/// `PrepareExplode` (0x004a6620): off the lists, the burst readied.
fn prepare_explode(p: &mut LeafPart, cx: &mut Cx) {
    p.x.prepare_explode = 1;
    crate::fellow::delete_cmnd(cx.scene, p.me);
    p.x.shock_scale = ONE;
    p.x.exp.init();
    marker_off(&mut p.x);
    crate::affect::clear_conditions(&mut cx.scene.chars[p.me]);
}

/// The marker's smoke stopped (`m_forbid` 1) once made.
fn marker_off(x: &mut Leaf) {
    if x.marker_init {
        x.marker_forbid = 1;
    }
}

/// `Order(n)` (0x004a6780): 24 fall, 25 the countdown again, 26 burst, 27
/// go, 28 fade off the body, 29 and 30 the grow's charge on and off.
pub(super) fn order(ls: &mut Leaves, k: usize, n: i32, mb: &mut Boss, mx: &mut Magus, cx: &mut Cx) {
    let me = ls.0[k].me;
    as_me(cx, me, |cx| {
        let p = &mut ls.0[k];
        p.x.order_num = n;
        match n {
            24 => {
                p.x.act_cnt = 0;
                change_action(p, cx, act::DROP, 3, true);
                init(p, mb, mx, cx);
                let pos = cx.pos();
                cx.out(Out::Se3dNote { se: 169, pos, note: 50 });
                p.x.marker_init = true;
                p.b.transparency = ONE;
                p.b.set_transparency = ONE;
            }
            25 => {
                count_start(&mut p.x);
                change_action(p, cx, act::COUNT_DOWN, 3, true);
            }
            26 => {
                p.x.act_cnt = 0;
                p.x.explode = 1;
                prepare_explode(p, cx);
                change_action(p, cx, act::FIRE, 3, true);
                cx.out(Out::SeNote { se: 35, note: 53 });
                cx.out(Out::Se { se: 56 });
            }
            27 => {
                if p.x.charge {
                    cx.out(Out::Magus(Pic::Charge { leaf: k, on: false }));
                    p.x.charge = false;
                }
                marker_off(&mut p.x);
                on_exit(ls, k, mb, mx, cx);
            }
            28 => {
                init(p, mb, mx, cx);
                p.x.transparency = ONE;
                p.x.trans_spd = ee::div(ONE, ee::from_int(p.b.anm.frames as i32));
                change_action(p, cx, act::DESTROY, 3, true);
            }
            29 => {
                if p.x.charge {
                    cx.out(Out::Magus(Pic::Charge { leaf: k, on: false }));
                }
                p.x.charge = true;
                cx.out(Out::Magus(Pic::Charge { leaf: k, on: true }));
            }
            30 if p.x.charge => {
                cx.out(Out::Magus(Pic::Charge { leaf: k, on: false }));
                p.x.charge = false;
            }
            _ => {}
        }
    });
}

/// `OnExit` (0x004a65a0): exited; unless Magus is drained its
/// `OnExitLeaf`; off the lists.
fn on_exit(ls: &mut Leaves, k: usize, mb: &mut Boss, mx: &mut Magus, cx: &mut Cx) {
    ls.0[k].b.exit = 1;
    if mx.epitaph == 0 {
        let mme = ls.0[k].x.master;
        as_me(cx, mme, |cx| super::on_exit_leaf(mb, mx, ls, cx));
    }
    crate::fellow::delete_cmnd(cx.scene, ls.0[k].me);
    ls.0[k].x.prepare_explode = 0;
}

/// `OnThinkDrop` (0x004a5ab0): once the fall's clip ends, the countdown
/// begins and the leaf lands 137.6 to its side, on the lists.
fn on_drop(p: &mut LeafPart, cx: &mut Cx) {
    let me = p.me;
    if p.b.anm_status == 0 {
        return;
    }
    count_start(&mut p.x);
    change_action(p, cx, act::COUNT_DOWN, 3, true);
    let s = if p.x.slave_id & 1 != 0 { 0x4309_999a } else { 0xc309_999a };
    let d = p.b.dirc[2];
    let v = [ee::mul(s, libm::cosf(d)), ee::mul(s, libm::sinf(d)), 0, ONE];
    let pp = geom::vadd(crate::boss::w2p(cx, cx.scene.chars[me].pos), v);
    let pos = crate::boss::p2w(cx, pp);
    let ch = &mut cx.scene.chars[me];
    ch.pos = pos;
    ch.pos_p = pp;
    let clip = if p.x.slave_id & 1 != 0 { "ANM_ex31lea1" } else { "ANM_ex31lea0" };
    p.b.anm.set(clip, cx.clips);
    cx.out(Out::Magus(Pic::Dust { pos }));
    cx.out(Out::Se3dNote { se: 56, pos, note: 72 });
    if p.x.marker_init {
        p.x.marker_forbid = 0;
    }
    crate::fellow::entry_cmnd(cx.scene, me);
}

/// `OnThinkFire` (0x004a5db0): the burst over, the leaf goes; the last of
/// the fallen to go brings Magus down (act 28) and its blur back.
fn on_fire(ls: &mut Leaves, k: usize, mb: &mut Boss, mx: &mut Magus, cx: &mut Cx) {
    if ls.0[k].b.anm_status == 0 {
        return;
    }
    on_exit(ls, k, mb, mx, cx);
    if ls.0.iter().any(|p| p.x.drop != 0 && p.b.exit == 0) {
        return;
    }
    mx.explosion = 0;
    let mme = ls.0[k].x.master;
    as_me(cx, mme, |cx| super::begin_fall(mb, mx, cx));
    mx.blur = super::BLUR;
    cx.out(Out::Blur { colour: mx.blur });
}

/// `OnThinkCountDown` (0x004a5ec0): the blink quickening to the burst.
/// At 0, the burst readied, and Magus rises unless it waves, is held or
/// already rises (then all burst at its next neutral). The count waits
/// while Magus is stuck or the party is down; the marker is drawn after.
fn on_count_down(ls: &mut Leaves, k: usize, mb: &mut Boss, mx: &mut Magus, cx: &mut Cx) {
    if mx.stuck != 0 || cx.annihilated() || cx.game_over {
        return;
    }
    count_down(ls, k, mb, mx, cx);
    let x = &mut ls.0[k].x;
    x.marker_alpha = ee::div(x.exp_alpha, 0x42c8_0000);
}

fn count_down(ls: &mut Leaves, k: usize, mb: &mut Boss, mx: &mut Magus, cx: &mut Cx) {
    let p = &mut ls.0[k];
    let x = &mut p.x;
    if x.prepare_explode != 0 {
        return;
    }
    let c = p.b.act_count;
    p.b.act_count += 1;
    if c < 0 {
        return;
    }
    if check_blink(x) {
        let t = ((u64::from((x.exp_cntr as u32).wrapping_mul(30)) * 0x91a2_b3c5) >> 32) as u32 >> 8;
        set_blink_time(x, t as i32);
        cx.out(Out::Magus(Pic::LeafAfterImage { leaf: k }));
    }
    if x.blink_base_time != 0 {
        let base = ee::from_int(x.blink_base_time);
        x.exp_alpha_spd = ee::div(ee::sub(0x42c8_0000, x.exp_base_alpha), base);
        x.exp_scale_spd = ee::div(0x3f00_0000, base);
    }
    if x.blink != 0 {
        x.exp_alpha = ee::sub(x.exp_alpha, x.exp_alpha_spd);
        x.exp_scale = ee::sub(x.exp_scale, x.exp_scale_spd);
    } else {
        x.exp_alpha = ee::add(x.exp_alpha, x.exp_alpha_spd);
        x.exp_scale = ee::add(x.exp_scale, x.exp_scale_spd);
    }
    if ee::lt(x.exp_scale, ONE) {
        x.exp_scale = ONE;
    }
    x.exp_base_alpha = ee::add(x.exp_base_alpha, 0x3de3_8e39);
    if !ee::le(x.exp_base_alpha, 0x4248_0000) {
        x.exp_base_alpha = 0x4248_0000;
    }
    if !ee::le(x.exp_alpha, 0x42c8_0000) {
        x.exp_alpha = 0x42c8_0000;
    } else if ee::lt(x.exp_alpha, x.exp_base_alpha) {
        x.exp_alpha = x.exp_base_alpha;
    }
    let n = x.exp_cntr;
    x.exp_cntr = n - 1;
    if n != 0 {
        return;
    }
    x.exp_cntr = 0;
    prepare_explode(p, cx);
    let mme = p.x.master;
    if mb.act_num != super::act::RISE {
        mx.prev_rise = cx.scene.chars[mme].pos[2];
        if mb.act_num == super::act::WAVE || mb.stop != 0 || cx.scene.chars[mme].cond[cond::HOLD] != 0 {
            mx.prepare_explode_all = 1;
        } else {
            mx.explosion = 1;
            mb.move_spd = 0;
            as_me(cx, mme, |cx| super::begin_rise(mb, mx, cx));
            mx.prepare_explode_all = 0;
        }
    }
    crate::fellow::delete_cmnd(cx.scene, ls.0[k].me);
}

/// `OnThinkDie` (0x004a57a0): off the lists, its death's sound, smoke and
/// ring; it fades in 30 frames and goes.
fn on_die(ls: &mut Leaves, k: usize, mb: &mut Boss, mx: &mut Magus, cx: &mut Cx) {
    let me = ls.0[k].me;
    let p = &mut ls.0[k];
    match p.b.act_proccess {
        0 => {
            p.x.exit_alpha = ONE;
            p.b.act_proccess += 1;
            crate::fellow::delete_cmnd(cx.scene, me);
            marker_off(&mut p.x);
            crate::affect::clear_conditions(&mut cx.scene.chars[me]);
            let dirc = p.b.dirc;
            dead_effect(mb, dirc, cx);
        }
        1 => {
            p.x.exit_alpha = ee::sub(p.x.exit_alpha, 0x3d08_8889);
            let gone = ee::lt(p.x.exit_alpha, 0);
            if gone {
                p.x.exit_alpha = 0;
            }
            let a = p.x.exit_alpha;
            p.b.transparency = a;
            p.b.set_transparency = a;
            if gone {
                on_exit(ls, k, mb, mx, cx);
            }
        }
        _ => p.b.act_proccess = 1,
    }
}

/// `DeadEffect` (0x004a5680): SE 104, the smoke, an auto samon ring
/// (model 195) 50 up, in the one effect manager (Magus's).
fn dead_effect(mb: &mut Boss, dirc: V4, cx: &mut Cx) {
    let pos = cx.pos();
    cx.out(Out::Se3dNote { se: 104, pos, note: 36 });
    cx.out(Out::Magus(Pic::LeafDead { pos }));
    let mut at = pos;
    at[2] = ee::add(at[2], 0x4248_0000);
    mb.effect_at(cx, EffKind::LeafRing, at, dirc);
}

// --- Affect ------------------------------------------------------------------------------------

/// `ccBoss03Leaf::Affect` (0x004a52b0): a hit (with no party checks and
/// no mark) takes HP; at 0 the leaf dies; the rest the base's.
pub(crate) fn affect(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    if b.lock_player != 0 || b.act_forbid == 2 {
        return;
    }
    let (t, p0) = {
        let a = &cx.scene.chars[me].affect;
        (a.ty, a.param[0])
    };
    if t != 1 && t != 3 {
        b.base_affect(cx);
        return;
    }
    let hp = cx.scene.chars[me].hp;
    let mhp = cx.scene.chars[me].max_hp;
    cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
    if p0 < 0 {
        return;
    }
    let mut h = if b.cheat_hp != 0 { hp.wrapping_sub(p0 / 10).max(mhp / 2) } else { hp.wrapping_sub(p0) };
    if h <= 0 {
        h = 0;
        crate::affect::clear_conditions(&mut cx.scene.chars[me]);
        leaf_change_action(b, cx, act::DIE, 2);
    } else if t == 1 {
        let n = if cx.env.count & 1 != 0 { 1 } else { 2 };
        leaf_change_action(b, cx, n, 0);
    }
    cx.scene.chars[me].hp = h;
}

/// The leaf's `ChangeAction` on a bare `Boss` (from its affect).
fn leaf_change_action(b: &mut Boss, cx: &mut Cx, n: i16, forbid: i16) {
    b.change_action(cx, n, forbid, true);
    if b.act_num == act::FIRE {
        b.anm.set("ANM_ex31exp1", cx.clips);
    }
    b.anm_status = 0;
}
