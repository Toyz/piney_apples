//! Kyvia's thunderbolts, act 6 of its second fight (`ccBossKyvia02::
//! ThunderboltAtk`, OUT gcmn 0x004dc6b0; `ThunderDmg` 0x004dd3f0,
//! `ThunderSound` 0x004ddd40), and the bolt they make
//! (`ccBossEffThunderbolt`, OUT 0x00477410, `Draw` 0x004778b0, made by
//! `ccBossEffThunderboltCreate` 0x00489540). docs/engine/boss-kyvia-second.md.

use piney_data::field::ee;

use super::{Kyvia, Tree, as_me, core_fade_check, core_state, core_state_is, dp, fabs, fp};
use crate::boss::{Boss, Cx, EffKind, Out, VF0};
use crate::enemy_ai::rand_f;
use crate::geom::{self, M4, V4};

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;

/// `ThunderType`: which thunder the attack pattern chose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    /// 0, the disc's first stage: three bolts on the disc and one on each
    /// member, smoke under the members'.
    #[default]
    Strike,
    /// 1, the first stage's other: the members' bolts, then four wide on
    /// the disc.
    Wide,
    /// 2, the second stage's: fifteen over the disc for 120 frames, the
    /// camera overhead.
    Storm,
}

/// `ThunderData` (0x40 bytes, bosseff.h): how a bolt looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoltData {
    pub default_angle: F,
    pub rand_angle: F,
    pub default_scale: i32,
    pub rand_scale: i32,
    pub mode: i32,
    pub eff_sw: u8,
    pub rnd_point: V4,
    pub off_set: V4,
}

/// `ccBossKyvia02`'s thunder members.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Thunder {
    pub kind: Kind,
    /// `ThunderTime`: the frames the bolts strike.
    pub time: i32,
    /// `dat`: the disc's bolts; `Sdat`: the members'.
    pub dat: BoltData,
    pub sdat: BoltData,
    /// `T_NUM`, `T_Range`: the disc's bolts and how far they spread.
    pub num: i16,
    pub range: i32,
    /// `StageID`: the stage fader's element.
    pub stage_id: i32,
}

impl Thunder {
    /// What `ThunderboltAtk`'s first step sets for its kind.
    fn arm(&mut self) {
        self.sdat = BoltData {
            default_angle: 0x3f00_0000,
            rand_angle: 0x3f66_6666,
            default_scale: 2,
            rand_scale: 3,
            mode: 1,
            eff_sw: 0,
            rnd_point: [0x4120_0000, 0x4120_0000, 0x4120_0000, 0x3f00_0000],
            ..self.sdat
        };
        let fifty = 0x4248_0000;
        let (time, rnd_point, rand_angle, scale, num, range) = match self.kind {
            Kind::Strike => (90, [fifty, fifty, fifty, 0x3f00_0000], ONE, (1, 4), 3, 100),
            Kind::Wide => (90, [0x41a0_0000, 0x41f0_0000, fifty, ONE], PI, (2, 5), 4, 550),
            Kind::Storm => (120, [fifty, fifty, fifty, ONE], PI, (3, 10), 15, 450),
        };
        self.time = time;
        self.dat = BoltData {
            default_angle: 0,
            rand_angle,
            default_scale: scale.0,
            rand_scale: scale.1,
            mode: 1,
            eff_sw: 1,
            rnd_point,
            ..self.dat
        };
        self.num = num;
        self.range = range;
    }
}

// --- the bolt ----------------------------------------------------------------------------

/// `ccBossEffThunderbolt`: `Num` strokes from points round `CenterPos`,
/// each a chain of `BreakPoint` segments of `CMP_x012` (`particle`) turned
/// at random, drawn anew each frame; gone after `Time + 1` draws.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bolt {
    pub center: V4,
    pub spot: Vec<V4>,
    pub break_point: Vec<i16>,
    pub bottom_range: i32,
    pub mode: i32,
    /// `flg`: the smoke and the offset still to come on the first draw;
    /// `flg2`: the burst at the end.
    pub flg: u8,
    pub flg2: u8,
    pub hoge: i32,
    /// `DefaultData.OffSet` (+0x90), which the caller sets after `Create`.
    pub off_set: V4,
    pub default_angle: F,
    pub rand_angle: F,
    pub default_scale: i32,
    pub rand_scale: i32,
    pub rnd_point: V4,
    pub time: i32,
    pub end_flg: u8,
    /// The segments of the last draw, each the clump's matrix.
    pub segments: Vec<M4>,
}

impl Bolt {
    /// The constructor (OUT gcmn 0x00477410) with `dat` (the
    /// constructor's defaults are for none, which Kyvia never passes).
    pub fn new(cx: &mut Cx, vec: V4, time: i32, ran: i32, num: i16, dat: &BoltData) -> Bolt {
        let n = usize::try_from(num).unwrap_or(0);
        let mut b = Bolt {
            center: vec,
            bottom_range: ran,
            time,
            default_scale: dat.default_scale,
            rand_scale: dat.rand_scale,
            default_angle: dat.default_angle,
            rand_angle: dat.rand_angle,
            mode: dat.mode,
            flg: dat.eff_sw,
            flg2: dat.eff_sw,
            rnd_point: dat.rnd_point,
            ..Bolt::default()
        };
        // SetBreakPoint (0x00477820): 10 to 17 segments a stroke.
        b.break_point = (0..n).map(|_| ((cx.cc.rand() % 8).abs() + 10) as i16).collect();
        // UnitPoint (0x004776e0): each stroke's top within the range.
        let r = ee::from_int(ran / 10);
        b.spot = (0..n)
            .map(|_| {
                let m = geom::rot_matrix_z(&geom::unit_matrix(), rand_f(cx.cc, PI));
                let mut s = VF0;
                s[1] = ee::sub(ee::from_int(ran), rand_f(cx.cc, r));
                geom::vadd(geom::apply_matrix(&m, s), vec)
            })
            .collect();
        b
    }

    /// A segment's turn: `ccRandF(pi)` about z, `DefaultAngle +
    /// |ccRandF(RandAngle)|` about x.
    fn turn(&self, cx: &mut Cx) -> (F, F) {
        let z = rand_f(cx.cc, PI);
        let x = fp(dp(self.default_angle) + dp(fabs(rand_f(cx.cc, self.rand_angle))));
        (z, x)
    }

    /// One `Draw` (OUT gcmn 0x004778b0): false once `m_bEnabled` clears.
    pub fn draw(&mut self, cx: &mut Cx) -> bool {
        if self.flg != 0 {
            // MissileSmokeGenerator row 8 at the bolt, moved by the offset.
            self.center = geom::vadd(self.center, self.off_set);
            cx.out(Out::KyviaParticles { which: super::Gen::MissileSmoke(8), pos: self.center });
            self.flg = 0;
        }
        const LENGTH: F = 0x4348_0000;
        let thin = ee::mul(0x3a91_a2b4, LENGTH);
        self.segments.clear();
        for i in 0..self.spot.len() {
            let r = cx.cc.rand();
            let w = self.default_scale + r.checked_rem(self.rand_scale).unwrap_or(0).abs();
            let w = ee::from_int(w);
            // Two draws the stroke never uses.
            rand_f(cx.cc, PI);
            rand_f(cx.cc, PI);
            let seg = |from: V4, (z, x): (F, F)| -> (M4, V4) {
                let mut m = geom::unit_matrix();
                m[0][0] = w;
                m[1][1] = thin;
                m[2][2] = w;
                let m = geom::rot_matrix_z(&geom::rot_matrix_x(&m, x), z);
                let turn = geom::rot_matrix_z(&geom::rot_matrix_x(&geom::unit_matrix(), x), z);
                let to = geom::vadd(geom::apply_matrix(&turn, [0, LENGTH, 0, ONE]), from);
                (geom::trans_matrix(&m, from), to)
            };
            let first = self.turn(cx);
            let mut at = self.spot[i];
            for (a, &r) in at.iter_mut().zip(&self.rnd_point).take(3) {
                *a = ee::add(*a, rand_f(cx.cc, r));
            }
            let (m, mut to) = seg(at, first);
            self.segments.push(m);
            for _ in 0..self.break_point[i] {
                let t = self.turn(cx);
                let (m, next) = seg(to, t);
                self.segments.push(m);
                to = next;
            }
        }
        let old = self.hoge;
        self.hoge += 1;
        if old == self.time {
            self.end_flg = 1;
            if self.flg2 != 0 {
                cx.out(Out::KyviaParticles { which: super::Gen::BurstSmoke(10), pos: self.center });
            }
            return false;
        }
        true
    }
}

/// `ccBossEffThunderboltCreate(pos, time, range, num, dat)` into the
/// manager, then `ccGetBossEffAdrs(id)`'s offset set when given.
fn bolt(b: &mut Boss, cx: &mut Cx, pos: V4, th: (i32, i32, i16), dat: &BoltData, off: Option<V4>) {
    let (time, range, num) = th;
    let mut e = Bolt::new(cx, pos, time, range, num, dat);
    if let Some(o) = off {
        e.off_set = o;
    }
    let kind = EffKind::Thunderbolt { num: i32::from(num) };
    let id = b.effects.create(kind);
    if let Some(s) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
        s.bolt = Some(Box::new(e));
    }
    cx.out(Out::Effect { id, kind, pos, dirc: VF0 });
}

// --- the attack --------------------------------------------------------------------------

/// `ThunderSound` (OUT gcmn 0x004ddd40): the build-up's flashes by the
/// raised `actCount`; true (and the count back to 0) at 80.
fn thunder_sound(b: &mut Boss, cx: &mut Cx) -> bool {
    b.act_count += 1;
    let flash = |t, colour| Out::Flash { t, colour };
    match b.act_count {
        10 => {
            cx.out(flash(10, 0x50ff_ffff));
            cx.out(Out::Se { se: 41 });
        }
        25 => cx.out(flash(2, 0x60ff_ffff)),
        30 => cx.out(flash(10, 0x30ff_ffff)),
        50 => cx.out(flash(6, 0x50ff_ffff)),
        80 => {
            cx.out(flash(5, 0x60ff_ffff));
            b.act_count = 0;
            return true;
        }
        _ => {}
    }
    false
}

/// A strike of `ThunderDmg`: `actCount`, the flash's time and colour and
/// the boss skill the core deals round the body.
type Strike = (i16, i32, u32, usize);

/// The strikes of a kind, the last's skill (at `ThunderTime`), and whether
/// a strike's sound (68) plays twice.
fn strikes(kind: Kind) -> ([Strike; 5], usize, bool) {
    const W5: u32 = 0x50ff_ffff;
    const W6: u32 = 0x60ff_ffff;
    const W3: u32 = 0x30ff_ffff;
    match kind {
        Kind::Strike => {
            ([(10, 10, W5, 45), (25, 10, W6, 45), (30, 15, W3, 45), (50, 10, W5, 45), (78, 5, W6, 45)], 45, true)
        }
        Kind::Wide => {
            ([(5, 10, W5, 45), (20, 10, W6, 45), (40, 15, W3, 45), (55, 10, W5, 45), (80, 5, W6, 46)], 46, true)
        }
        Kind::Storm => {
            ([(25, 10, W5, 45), (50, 10, W6, 45), (75, 15, W3, 46), (80, 10, W5, 46), (90, 5, W6, 46)], 46, false)
        }
    }
}

/// `ThunderDmg` (OUT gcmn 0x004dd3f0): the strikes by kind and count; the
/// last, at `ThunderTime`, is 10 frames of 0x08ffffff with the other sound.
fn thunder_dmg(b: &Boss, x: &Kyvia, t: &mut Tree, cx: &mut Cx) {
    let Some(th) = x.thunder else { return };
    let (list, last, twice) = strikes(th.kind);
    let pos = cx.scene.chars[cx.me].pos;
    let c = b.act_count;
    for &(_, ft, colour, skill) in list.iter().filter(|s| s.0 == c) {
        cx.out(Out::Flash { t: ft, colour });
        core_damage_at(t, cx, pos, skill);
        cx.out(Out::Se { se: 68 });
        if twice {
            cx.out(Out::Se { se: 68 });
        }
        cx.out(Out::SeNote { se: 39, note: 65 });
    }
    if i32::from(c) == th.time {
        cx.out(Out::Flash { t: 10, colour: 0x08ff_ffff });
        core_damage_at(t, cx, pos, last);
        cx.out(Out::Se { se: 68 });
        cx.out(Out::SeNote { se: 35, note: 45 });
    }
}

/// `ccBossSkillDamage(CORE_THIS, pos, i)`: the core's skill round `pos`.
fn core_damage_at(t: &mut Tree, cx: &mut Cx, pos: V4, i: usize) {
    let Some(c) = t.core.as_mut() else { return };
    let cm = c.me;
    as_me(cx, cm, |cx| c.b.skill_damage(cx, None, Some(pos), i));
}

/// The core and its gomoras at rest (`GetActNum`), whatever its state.
fn core_at_rest(t: &Tree) -> bool {
    t.core.as_ref().is_some_and(|c| super::core::get_act_num(c, &t.gomoras))
}

/// `ThunderboltAtk` (OUT gcmn 0x004dc6b0).
pub(super) fn thunderbolt_atk(b: &mut Boss, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
    let mut th = x.thunder.unwrap_or_default();
    let me = cx.me;
    let off = [0, 0, 0x43fa_0000, ONE];
    match b.act_proccess {
        0 => {
            if !core_at_rest(t) {
                return;
            }
            core_state(t, cx, 8);
            th.stage_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 32277]);
            b.act_proccess += 1;
            b.act_count = 0;
            b.lock_player(cx, false);
            x.quake_vector = [0x41f0_0000, 0x41f0_0000, 0x41a0_0000, x.quake_vector[3]];
            th.arm();
        }
        1 => {
            if !core_state_is(t, 6) {
                return;
            }
            if th.kind == Kind::Storm {
                b.act_proccess = 50;
                cx.out(Out::CamMode { mode: 5 });
                let rx = geom::rot_matrix_x(&geom::unit_matrix(), 0x3f33_3333);
                let m = geom::rot_matrix_z(&rx, cx.cam.rot[2]);
                let eye = geom::vadd(geom::apply_matrix(&m, [0, 0x456d_8000, 0, ONE]), x.disc_pos);
                cx.out(Out::FreeCam { pos: eye, view: cx.scene.chars[me].pos });
                b.act_count = 0;
            } else {
                b.act_proccess = 2;
                let range = if x.disc_lv != x.disc_max_lv { 0x457a_0000 } else { 0x453b_8000 };
                cx.out(Out::CamModeRange { mode: 3, range });
            }
        }
        50 => {
            let m = geom::rot_matrix_z(&geom::unit_matrix(), cx.cam.rot[2]);
            let pos = geom::vadd(cx.cam.pos, geom::apply_matrix(&m, [0, 0xc020_0000, 0xc190_0000, ONE]));
            let mut view = cx.cam.view;
            view[2] = ee::add(view[2], 0x4120_0000);
            cx.out(Out::FreeCam { pos, view });
            if thunder_sound(b, cx) {
                b.act_proccess = 3;
                b.act_count = 0;
                bolt(b, cx, x.disc_pos, (th.time, th.range, th.num), &th.dat, Some(off));
                cx.out(Out::Se3d { se: 41, pos: x.disc_pos });
            }
        }
        2 => {
            if !(thunder_sound(b, cx) && cx.cam.reset == 0) {
                x.thunder = Some(th);
                return;
            }
            cx.out(Out::Se3d { se: 41, pos: x.disc_pos });
            for m in cx.party.members {
                let Some(m) = m.filter(|&m| cx.valid(Some(m)) && cx.scene.chars[m].hp > 0) else { continue };
                let p = cx.scene.chars[m].pos;
                bolt(b, cx, p, (th.time, 50, 3), &th.sdat, None);
                if th.kind == Kind::Strike {
                    // MissileSmokeGenerator row 17, set to live ThunderTime.
                    cx.out(Out::KyviaParticles { which: super::Gen::MissileSmoke(17), pos: p });
                }
            }
            if th.kind == Kind::Wide {
                bolt(b, cx, x.disc_pos, (th.time, th.range, th.num), &th.dat, Some(off));
            }
            b.act_proccess = 3;
        }
        3 => {
            let c = b.act_count;
            let shake: Option<[F; 3]> = match (th.kind, c) {
                (Kind::Storm, ..5) => Some([0x41f0_0000, 0x41f0_0000, 0x43c8_0000]),
                (Kind::Storm, 10) => Some([0x4120_0000, 0x4120_0000, 0x4348_0000]),
                (Kind::Storm, 20) => Some([0x4120_0000, 0x4120_0000, 0x42c8_0000]),
                _ => None,
            };
            if let Some(v) = shake {
                x.quake_vector[..3].copy_from_slice(&v);
            }
            if i32::from(c) == th.time / 2 {
                x.quake_vector[..3].copy_from_slice(&[0x41f0_0000, 0x41f0_0000, 0x4220_0000]);
            }
            x.thunder = Some(th);
            thunder_dmg(b, x, t, cx);
            if c != 0 && c % 5 == 0 {
                let note = 70 + cx.cc.rand() % 20;
                cx.out(Out::SeNote { se: 70, note: note as u8 });
                cx.out(Out::Se { se: 94 });
            }
            let v = x.quake_vector;
            cx.quake_vec([v[0], v[1], v[2]]);
            b.act_count += 1;
            if i32::from(c) == th.time {
                b.act_proccess = 4;
                b.act_count = 0;
            }
        }
        4 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 30 {
                b.act_proccess = 7;
                b.act_count = 0;
            }
        }
        7 => {
            core_state(t, cx, 7);
            cx.out(Out::CamMode { mode: if th.kind == Kind::Storm { 6 } else { 4 } });
            b.act_proccess += 1;
            let id = std::mem::replace(&mut th.stage_id, -1);
            b.end_stage_effect(cx, id, 0x6400_0000, 35);
        }
        8 if cx.cam.reset == 0 => b.act_proccess += 1,
        9 if core_fade_check(t) => {
            b.unlock_player();
            b.change_action(cx, super::act::NEUTRAL, 1, true);
        }
        _ => {}
    }
    x.thunder = Some(th);
}
