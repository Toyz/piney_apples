//! Corbenik's seeds (`ccBoss08Seed`, QUA gcmn 0x003b00b0-0x003b1a40):
//! eight bosses (`bossTbl` row 44) the first form carries, each on a
//! character of its own. Thrown (`Order(3)`) they fly out and bounce; once
//! still they count 150 frames down (`Order(4)`, the number over them,
//! held while the body casts or dies), flush and burst (`Order(5)`): boss
//! skill 31 there, and every other seed in its area set off too.

use piney_data::field::ee;

use super::first::{self, First};
use super::{Pic, SEED_ROW};
use crate::boss::kyvia::{Part, as_me, plain_boss, set_base_param};
use crate::boss::{Boss, Class, Cx, EffKind, Out, VF0};
use crate::enemy_ai::rand_f;
use crate::geom::{self, V4};
use crate::param::cond;
use crate::world::CharHit;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;

/// A seed's acts (`Think` 0x003b0ac0; any other does nothing).
pub mod act {
    pub const WAIT: i16 = 0;
    pub const FLY: i16 = 3;
    pub const COUNT_DOWN: i16 = 4;
    pub const FIRE: i16 = 5;
    pub const DEAD: i16 = 14;
}

/// `ccBoss08Seed`'s own members (+0x29350 on).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seed {
    /// `m_master` (the first form's character) and the seed's slot.
    pub master: usize,
    pub slave_id: i32,
    /// `bodyHit` (+0x240): its body in the field's list.
    pub hit: CharHit,
    /// `m_effFlush` (+0x293b0): the flush before the burst.
    pub eff_flush: Option<i32>,
    /// `m_expCount`: frames to the burst; `m_blinkTime`: frames to the next
    /// after-image.
    pub exp_count: i32,
    pub blink_time: i32,
    /// `m_rotSpeed[0]` and `[2]`: the spin about x and z, in `RAD2DEG`'s
    /// units.
    pub rot_speed: [u16; 2],
    /// `m_rangeOut`: it flew off the land (fading out);
    /// `m_explodeFlag`: set off.
    pub range_out: bool,
    pub explode: bool,
    /// `m_chargeGen` (+0x29350): the charge's generator is running.
    pub charge: bool,
}

/// A seed's `ENTRYAFTERIMAGE_T` (+0x29360, its constructor 0x003b0288):
/// colour 0x005050ff at 100, transparency 1 by -1/30, its scale 3 to 9
/// over 30 draws.
pub const IMAGE: crate::boss::AfterImageParams = crate::boss::AfterImageParams {
    color: 0x0050_50ff,
    color_alpha: 0x42c8_0000,
    color_spd: 0,
    max_frame: 0,
    transparency: 0x3f80_0000,
    trans_spd: 0xbd08_8889,
    ss: [0x4040_0000, 0x4040_0000, 0x4040_0000, 0x3f80_0000],
    es: [0x4110_0000, 0x4110_0000, 0x4110_0000, 0x3f80_0000],
    scale_time: 0x41f0_0000,
};

/// A seed out of its character (`Class::CorbenikSeed`), and whether one
/// was there.
pub(crate) fn take(cx: &mut Cx, me: usize) -> (Part<Seed>, bool) {
    let taken = cx.scene.chars.get_mut(me).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take());
    let Some(mut sb) = taken else { return (Part { me, ..Part::default() }, false) };
    match std::mem::replace(&mut sb.class, Class::Plain) {
        Class::CorbenikSeed(x) => (Part { b: sb, x, me }, true),
        other => {
            sb.class = other;
            if let Some(f) = cx.scene.chars[me].foe_state_mut() {
                f.boss = Some(sb);
            }
            (Part { me, ..Part::default() }, false)
        }
    }
}

/// A seed put back into its character, unless [`take`] found none.
pub(crate) fn put(cx: &mut Cx, mut part: Part<Seed>, found: bool) {
    if !found {
        return;
    }
    part.b.class = Class::CorbenikSeed(part.x);
    if let Some(f) = cx.scene.chars[part.me].foe_state_mut() {
        f.boss = Some(part.b);
    }
}

/// `ccBoss08Seed::ccBoss08Seed` (0x003b00b0) and `SetMaster(body, k)`:
/// row 44's stats, its body hit by the row (off), at the origin, exited,
/// in act 0 held off (`ChangeAction(0, 3, 4)`).
pub(crate) fn new(cx: &mut Cx, me: usize, master: usize, k: i32) -> Part<Seed> {
    let mut b = plain_boss();
    set_base_param(cx, me, SEED_ROW);
    b.anm_tbl = cx.data.corbenik.seed_anims.clone();
    cx.scene.chars[me].pos = VF0;
    b.dirc = VF0;
    let base = cx.scene.chars[me].base();
    let hit = CharHit { radius: base.width, height: base.height, kind: base.ty as u32, ..CharHit::default() };
    as_me(cx, me, |cx| b.change_action(cx, act::WAIT, 3, true));
    let x = Seed { master, slave_id: k, hit, exp_count: 150, blink_time: 30, ..Seed::default() };
    Part { b: Box::new(b), x: Box::new(x), me }
}

/// `ccBoss08Seed::Main` (0x003b06a0), `x.seeds[k]` its own frame: the
/// base's head, `Think`, its own `Move`, the clip forward. `mb` is the
/// first form's `ccBoss` (its effects the manager's).
pub(crate) fn main(mb: &mut Boss, x: &mut First, k: usize, cx: &mut Cx) {
    let mut p = std::mem::take(&mut x.seeds[k]);
    as_me(cx, p.me, |cx| frame(mb, x, &mut p, cx));
    x.seeds[k] = p;
}

fn frame(mb: &mut Boss, x: &mut First, p: &mut Part<Seed>, cx: &mut Cx) {
    let b = &mut p.b;
    if b.exit != 0 {
        return;
    }
    let me = cx.me;
    b.calc_real(cx);
    b.calc_target_info(cx);
    b.center_pos_p = crate::boss::w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = crate::boss::dist(cx, pp, b.center_pos_p);
    b.center_dirc = crate::enemy_ai::get_dirc(pp, b.center_pos_p);
    // `lockPlayer` and the menu's forbid: a seed never locks.
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
    think(mb, x, p, cx);
    move_(p, cx);
    if p.b.draw_sw != 0 {
        // `PreDrawAnm` (0x003b09f0): the clip forward, drawn 50 up at 3x.
        p.b.anm_status = i8::from(p.b.anm.forward());
        p.b.draw(cx);
    }
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// `ccBoss::Move` (QUA gcmn 0x00360120) on a seed: the step and its
/// throw, then its body hit (`SetHitSW(bodyHitSW)`) pushed out of the
/// others, the push's height dropped.
fn move_(p: &mut Part<Seed>, cx: &mut Cx) {
    let me = p.me;
    let b = &mut p.b;
    let mut pp = crate::boss::w2p(cx, cx.scene.chars[me].pos);
    if !geom::eq(0, b.move_spd) {
        pp[0] = ee::add(pp[0], ee::mul(b.move_spd, piney_data::libm::sinf(b.move_dirc)));
        pp[1] = ee::sub(pp[1], ee::mul(b.move_spd, piney_data::libm::cosf(b.move_dirc)));
    }
    pp = geom::vadd(pp, b.move_vector);
    let mut pos = crate::boss::p2w(cx, pp);
    let on = b.body_hit_sw != 0;
    cx.part_hits.switch(me, &mut p.x.hit, on);
    if on {
        p.x.hit.pos = pos;
        if cx.part_hits.collide(me, &mut p.x.hit) {
            let mut off = p.x.hit.offset;
            off[2] = 0;
            pos = geom::vadd(pos, off);
            pp = geom::vadd(pp, off);
            p.x.hit.pos = pos;
            cx.part_hits.sync(me, &p.x.hit);
        }
    }
    let ch = &mut cx.scene.chars[me];
    ch.pos_p = pp;
    ch.pos = pos;
}

fn think(mb: &mut Boss, x: &mut First, p: &mut Part<Seed>, cx: &mut Cx) {
    match p.b.act_num {
        act::FLY => on_fly(&mut p.b, &mut p.x, cx),
        act::COUNT_DOWN => on_count_down(&mut p.b, &mut p.x, x.pause_cnt_down, cx),
        act::FIRE => on_fire(mb, x, p, cx),
        act::DEAD => on_dead(&mut p.b, &mut p.x, cx),
        _ => {}
    }
}

/// `Order(n)` (0x003b0b50): 3 thrown from 500 over `master_pos` (out at
/// 60 a frame, between 54 and 72 degrees down from level, any way round,
/// spinning by `rand()`), back on the field off the targets; 4 still: on
/// the lists, its count and the charge; 5 the charge off, set off once.
pub(crate) fn order(b: &mut Boss, x: &mut Seed, cx: &mut Cx, n: i32, master_pos: V4) {
    let me = cx.me;
    match n {
        3 => {
            let mut pos = master_pos;
            pos[2] = ee::add(pos[2], 0x43fa_0000);
            cx.scene.chars[me].pos = pos;
            let ry = ee::sub(0x3fa0_d97c, rand_f(cx.cc, 0x3ea0_d97c));
            let rz = rand_f(cx.cc, PI);
            let r = cx.rand.rand() >> 3;
            let m = geom::rot_matrix_x(&geom::unit_matrix(), 0);
            let m = geom::rot_matrix_y(&m, ry ^ 0x8000_0000);
            let m = geom::rot_matrix_z(&m, rz);
            let v = geom::apply_matrix(&m, [ONE, 0, 0, ONE]);
            b.move_vector = v.map(|c| ee::mul(c, 0x4270_0000));
            x.rot_speed = [((r >> 4) & 0xf000) as u16, ((r >> 8) & 0xf000) as u16];
            b.exit = 0;
            b.erase_cmnd_target(cx);
            b.draw_sw = 1;
            b.body_hit_sw = 1;
            let max = cx.scene.chars[me].max_hp;
            cx.scene.chars[me].hp = max;
            b.transparency = ONE;
            b.set_transparency = ONE;
            x.explode = false;
            b.change_action(cx, act::FLY, 3, true);
        }
        4 => {
            crate::fellow::entry_cmnd(cx.scene, me);
            x.exp_count = 150;
            x.blink_time = 30;
            x.charge = true;
            let pos = cx.pos();
            cx.out(Out::Corbenik(Pic::Charge { seed: me, pos, on: true }));
            b.entry_cmnd_target(cx);
            b.change_action(cx, act::COUNT_DOWN, 3, true);
        }
        5 => {
            charge_off(x, cx);
            if !x.explode {
                x.explode = true;
                b.change_action(cx, act::FIRE, 3, true);
            }
        }
        _ => {}
    }
}

/// The charge's generator deleted (`ParticleDelete`), if it runs.
fn charge_off(x: &mut Seed, cx: &mut Cx) {
    if x.charge {
        x.charge = false;
        let me = cx.me;
        cx.out(Out::Corbenik(Pic::Charge { seed: me, pos: VF0, on: false }));
    }
}

/// `OnExit` (0x003b1440): exited, hidden, its body off, off the lists, the
/// charge off.
fn on_exit(b: &mut Boss, x: &mut Seed, cx: &mut Cx) {
    b.exit = 1;
    b.draw_sw = 0;
    b.body_hit_sw = 0;
    x.range_out = false;
    crate::fellow::delete_cmnd(cx.scene, cx.me);
    cx.out(Out::DeleteCmnd);
    charge_off(x, cx);
}

/// `RAD2DEG(a)` plus `d` in its units, back through `DEG2RAD`.
fn spin(a: F, d: u16) -> F {
    geom::deg2rad(geom::rad2deg(a).wrapping_add(d as i16))
}

/// `OnThinkFly` (0x003b0f40): falling 4 a frame, spinning. On the way
/// down at 75 or under: put at 50 and a bounce (SE 65), its spin and
/// throw damped (0.8, the fall turned at half); still enough, it lies
/// down and counts (`Order(4)`). No land under it: off the field, fading
/// out over 15 frames.
fn on_fly(b: &mut Boss, x: &mut Seed, cx: &mut Cx) {
    let me = cx.me;
    b.act_count += 1;
    if !x.range_out && ee::lt(b.move_vector[2], 0xc0e0_0000) {
        let pos = cx.scene.chars[me].pos;
        if ee::le(pos[2], 0x4296_0000) {
            let mut pos = pos;
            pos[2] = 0x4248_0000;
            cx.scene.chars[me].pos = pos;
            cx.out(Out::Se3dNote { se: 65, pos, note: 65 });
            let mut sp = pos;
            sp[2] = ee::add(sp[2], 0x42c8_0000);
            let mut ep = pos;
            ep[2] = ee::sub(ep[2], 0x42c8_0000);
            if cx.part_hits.line(sp, ep, 0x2000_0000).is_none() {
                x.range_out = true;
                return;
            }
            for s in &mut x.rot_speed {
                let v = ee::mul(0x3f4c_cccd, ee::from_int(i32::from(*s)));
                *s = ee::to_int(v) as u16;
                if *s < 16 {
                    *s = 0;
                }
            }
            let mv = &mut b.move_vector;
            mv[0] = ee::mul(mv[0], 0x3f4c_cccd);
            mv[1] = ee::mul(mv[1], 0x3f4c_cccd);
            mv[2] = ee::mul(mv[2], 0xbf00_0000);
            let small = |v: F, lim: f64| f64::from(f32::from_bits(v)).abs() < lim;
            if small(mv[0], 4.0) {
                mv[0] = 0;
            }
            if small(mv[1], 4.0) {
                mv[1] = 0;
            }
            if small(mv[2], 16.0) {
                b.move_vector = VF0;
                b.dirc[0] = 0;
                b.dirc[1] = HALF_PI;
                order(b, x, cx, 4, VF0);
                return;
            }
        }
    } else if x.range_out {
        let mut t = ee::sub(b.transparency, 0x3d88_8889);
        if ee::le(t, 0) {
            t = 0;
            on_exit(b, x, cx);
        }
        b.transparency = t;
        b.set_transparency = t;
    }
    b.move_vector[2] = ee::sub(b.move_vector[2], 0x4080_0000);
    b.dirc[0] = spin(b.dirc[0], x.rot_speed[0]);
    b.dirc[2] = spin(b.dirc[2], x.rot_speed[1]);
}

/// `OnThinkCountDown` (0x003b1500): its seconds left over it; unless the
/// body holds the count, a frame off, and past 0 set off (`Order(5)`);
/// an after-image each time its blink runs out, sooner as the count
/// falls (never under 10 frames apart).
fn on_count_down(b: &mut Boss, x: &mut Seed, paused: bool, cx: &mut Cx) {
    let mut tag = cx.pos();
    tag[2] = ee::add(tag[2], 0x42c8_0000);
    let n = ee::to_int(ee::div(ee::from_int(x.exp_count + 29), 0x41f0_0000));
    cx.out(Out::Corbenik(Pic::CountDown { pos: tag, n }));
    if paused {
        return;
    }
    let old = x.exp_count;
    x.exp_count = old - 1;
    if old <= 0 {
        order(b, x, cx, 5, VF0);
        return;
    }
    if x.blink_time == 0 {
        return;
    }
    x.blink_time -= 1;
    if x.blink_time > 0 {
        return;
    }
    let mut pos = cx.pos();
    pos[2] = ee::add(pos[2], 0x4248_0000);
    let me = cx.me;
    cx.out(Out::Corbenik(Pic::SeedImage { seed: me, frame: b.anm.frame(), pos, dirc: b.dirc }));
    x.blink_time = (x.exp_count * 30 / 150).max(10);
}

/// `OnThinkFire` (0x003b17b0): the flush (1000 to 0); once it is over the
/// burst (SE 35, note 53), every other seed in its area set off, off the
/// targets and the body off; 15 frames on boss skill 31 there, and out.
fn on_fire(mb: &mut Boss, x: &mut First, p: &mut Part<Seed>, cx: &mut Cx) {
    let b = &mut p.b;
    let pos = cx.pos();
    match b.act_proccess {
        0 => {
            let f = super::eff::Flush::new(pos, 0x447a_0000, 0);
            let id = mb.effects.create(EffKind::Flush);
            if let Some(e) = usize::try_from(id).ok().and_then(|k| mb.effects.slots.get_mut(k)).and_then(|s| s.as_mut())
            {
                e.corbenik = Some(Box::new(super::eff::Fx::Flush(f)));
            }
            cx.out(Out::Effect { id, kind: EffKind::Flush, pos, dirc: VF0 });
            p.x.eff_flush = Some(id);
            b.act_proccess += 1;
        }
        1 => {
            if mb.effects.enabled(p.x.eff_flush) {
                return;
            }
            p.x.eff_flush = None;
            cx.out(Out::Corbenik(Pic::Burst { pos, spd: b.move_vector, scale: ONE, kind: 0, grow: Some(super::GROW) }));
            cx.out(Out::Se3dNote { se: 35, pos, note: 53 });
            let at = cx.scene.chars[p.me].pos_p;
            let from = usize::try_from(p.x.slave_id).unwrap_or(usize::MAX);
            first::send_message(x, from, at, cx);
            b.act_proccess += 1;
            b.erase_cmnd_target(cx);
            b.body_hit_sw = 0;
        }
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 15 {
                b.skill_damage(cx, None, Some(pos), 31);
                on_exit(b, &mut p.x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkDead` (0x003b1970): off the lists and the body off, faded out
/// over 30 frames.
fn on_dead(b: &mut Boss, x: &mut Seed, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            crate::fellow::delete_cmnd(cx.scene, cx.me);
            cx.out(Out::DeleteCmnd);
            b.body_hit_sw = 0;
            b.act_proccess += 1;
        }
        1 => {
            let mut t = ee::sub(b.transparency, 0x3d08_8889);
            if ee::lt(t, 0) {
                t = 0;
                on_exit(b, x, cx);
            }
            b.transparency = t;
            b.set_transparency = t;
        }
        _ => {}
    }
}

/// `ccBoss08Seed::Affect` (0x003b03b0): nothing while held off or locked
/// (but a drain's 21); a blow (1, 3): its number, the mark and the HP
/// down (with cheat HP a fifth, never under half), at 0 its conditions
/// cleared and dead; then a spell on it shows the shield. Any other
/// type the base's.
pub(crate) fn affect(b: &mut Boss, cx: &mut Cx) {
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
    cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
    if p0 >= 0 {
        cx.out(Out::HitMark { by: person });
        let hp = cx.scene.chars[me].hp;
        let mhp = cx.scene.chars[me].max_hp;
        let mut h = crate::boss::hit_hp(hp, p0, (b.cheat_hp != 0).then_some(5), mhp);
        if h <= 0 {
            h = 0;
            crate::affect::clear_conditions(&mut cx.scene.chars[me]);
            b.change_action(cx, act::DEAD, 2, true);
        } else if t == 1 {
            let n = if cx.env.count & 1 != 0 { 1 } else { 2 };
            b.change_action(cx, n, 0, true);
        }
        cx.scene.chars[me].hp = h;
    }
    if crate::skill::check_type_of(cx.t, i32::from(p1)) == 1 {
        cx.out(Out::Corbenik(Pic::SeedShield { seed: me }));
    }
}
