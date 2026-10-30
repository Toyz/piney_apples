//! Kyvia's magical ball, act 20 of its third fight (`ccBossKyvia03::
//! MagicBallAtk`, QUA gcmn 0x003d7d40; its callback
//! `KyviaMagicalBallDamage` 0x003d3c80), and the ball it makes
//! (`ccBossEffMagicalBall`, bosseff.cpp: `InitData` 0x0036b4e0, `Draw`
//! 0x0036b880, `DrawPlasma` 0x0036c170, `SetSpPoint` 0x0036c630, made by
//! `ccBossEffMagicalBallCreate` 0x0037ca70). docs/engine/boss-kyvia-third.md.

use piney_data::field::ee;

use super::{
    Follow, Gen, Kyvia, Place, Tree, as_me, catmull, core_fade_check, core_ready, core_state, core_state_is, dp, fabs,
    fp,
};
use crate::boss::blur::Set;
use crate::boss::{BossBase, Cx, EffKind, Out, VF0};
use crate::enemy_ai::rand_f;
use crate::geom::{self, V4};
use crate::rand::Rng;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_PI: F = 0xc049_0fdb;

/// The ball's particle generators (`MissileSmokeGenerator` rows, their
/// pictures piney-effect's), all with `distSW` off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BallGen {
    /// Full grown: row 12 with force fields 13 and 22, and again with 13
    /// and 23 (`gLife` 150, written into the row first).
    Grown(u8),
    /// A plasma's two ends (row 10, ff 11; `gLife` 120), following
    /// `PlaPos[k]` and `PlaPos2[k]`.
    Plasma,
    /// The gathering at the 120th frame (row 14, ff 14 22).
    Gather,
    /// The blast (rows 15 with ff 15 and 22, 15 with 15 and 23, 16 with
    /// 15).
    Blast(u8),
}

impl BallGen {
    /// Its row and force fields.
    pub fn rows(self) -> (u8, &'static [u8]) {
        match self {
            BallGen::Grown(0) => (12, &[13, 22]),
            BallGen::Grown(_) => (12, &[13, 23]),
            BallGen::Plasma => (10, &[11]),
            BallGen::Gather => (14, &[14, 22]),
            BallGen::Blast(0) => (15, &[15, 22]),
            BallGen::Blast(1) => (15, &[15, 23]),
            BallGen::Blast(_) => (16, &[15]),
        }
    }

    /// The `gLife` the ball writes into its row before making it.
    pub fn life(self) -> Option<i16> {
        match self {
            BallGen::Grown(_) => Some(150),
            BallGen::Plasma => Some(120),
            _ => None,
        }
    }
}

/// `ccBossEffMagicalBall`'s members (Infection's DWARF names).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MagicalBall {
    /// `EffMode`: 1 growing, 2 the plasmas, 3 the blast.
    pub eff_mode: i32,
    /// `Delay`: the frames of the mode.
    pub delay: i32,
    pub pos: V4,
    pub rot: V4,
    pub scale: V4,
    /// `ABGR`: the blur's colour as the blast fades it.
    pub abgr: u32,
    pub de_point: i32,
    /// `SBpoint1`-`3`: each plasma's spline points, and `RndSBpoint1`-`3`
    /// (drawn, never read).
    pub sb: [Vec<V4>; 3],
    pub rnd_sb: [Vec<V4>; 3],
    /// `time` and `Mode`: each plasma's place on its spline and its leg.
    pub time: Vec<F>,
    pub mode: Vec<i32>,
    pub scale_point: F,
    pub max_scale: F,
    /// `SetT`, `TempT`.
    pub set_t: Vec<F>,
    pub temp_t: Vec<F>,
    pub color: u32,
    pub transparency: F,
    pub count: i32,
    pub cflg: i32,
    pub blur_delay: i32,
    /// `PlaPos`, `PlaPos2`: where each plasma's two generators are.
    pub pla_pos: Vec<V4>,
    pub pla_pos2: Vec<V4>,
    /// `EffAnm`'s transparency (+0x88 of the `ccAnm`), which the blast
    /// fades.
    pub anm_transparency: F,
    /// `DrawPlasma`'s spline point (a local the legs without a spline,
    /// 0 and 4, leave as the last one wrote it).
    pub spline: V4,
    /// The scale `EffAnm` is drawn at: set while growing, before the
    /// frame's growth.
    pub anm_scale: V4,
}

impl MagicalBall {
    /// `InitData(ran, pos, blur, cb)` (QUA gcmn 0x0036b4e0): seven
    /// plasmas, their splines (`SetSpPoint`) and their times
    /// `-|ccRandF(2)|`.
    pub fn new(cc: &mut dyn Rng, ran: F, pos: V4) -> MagicalBall {
        let n = 7;
        let mut b = MagicalBall {
            eff_mode: 1,
            pos,
            rot: VF0,
            scale: VF0,
            max_scale: ran,
            color: 0x000f_0f28,
            transparency: 0x3f00_0000,
            cflg: 1,
            // ccCoord's constructor's.
            anm_transparency: ONE,
            ..MagicalBall::default()
        };
        b.set_sp_point(cc, n);
        b.time = (0..n).map(|_| fp(-dp(fabs(rand_f(cc, 0x4000_0000))))).collect();
        b.mode = vec![0; n];
        b.pla_pos = vec![VF0; n];
        b.pla_pos2 = vec![VF0; n];
        b
    }

    /// `SetSpPoint` (QUA gcmn 0x0036c630): each plasma's three points
    /// about the ball turned at random (`|ccRandF(100)|`s out, 350 and 750
    /// down, the last `MaxScale` times -17.5), and its `RndSBpoint`s.
    fn set_sp_point(&mut self, cc: &mut dyn Rng, n: usize) {
        let out = |cc: &mut dyn Rng| fp(150.0 + dp(fabs(rand_f(cc, 0x42c8_0000))));
        let down = |cc: &mut dyn Rng, d: f64| fp(-dp(fabs(rand_f(cc, 0x42c8_0000))) - d);
        for s in self.sb.iter_mut().chain(self.rnd_sb.iter_mut()) {
            *s = vec![VF0; n];
        }
        self.set_t = vec![0; n];
        self.temp_t = vec![0; n];
        for k in 0..n {
            for s in self.sb.iter_mut().chain(self.rnd_sb.iter_mut()) {
                s[k] = VF0;
            }
            let m = geom::rot_matrix_x(&geom::unit_matrix(), rand_f(cc, PI));
            let m = geom::rot_matrix_y(&m, rand_f(cc, PI));
            let mut p = VF0;
            p[0] = out(cc);
            p[1] = down(cc, 350.0);
            p[2] = out(cc);
            self.sb[0][k] = geom::apply_matrix(&m, p);
            p[0] = out(cc);
            p[1] = down(cc, 750.0);
            p[2] = out(cc);
            self.sb[1][k] = geom::apply_matrix(&m, p);
            p[0] = rand_f(cc, 0x42c8_0000);
            p[1] = fp(dp(self.max_scale) * -17.5);
            p[2] = rand_f(cc, 0x42c8_0000);
            self.sb[2][k] = geom::apply_matrix(&m, p);
            let r = |cc: &mut dyn Rng, x: F, y: F, z: F| [rand_f(cc, x), rand_f(cc, y), rand_f(cc, z), ONE];
            self.rnd_sb[0][k] = r(cc, 0x4120_0000, 0x4120_0000, 0x4170_0000);
            self.rnd_sb[1][k] = r(cc, 0x4120_0000, 0x40a0_0000, 0x40a0_0000);
            self.rnd_sb[2][k] = r(cc, 0x40a0_0000, 0x4120_0000, 0x4100_0000);
            self.set_t[k] = ONE;
            self.temp_t[k] = fabs(rand_f(cc, 0x3dcc_cccd));
        }
    }

    /// One `Draw` (QUA gcmn 0x0036b880) for `host`: its callback
    /// (`m_cbFunc`), the blur (`blur`) and what else it makes; false once
    /// `m_bEnabled` clears.
    pub fn draw(&mut self, host: &mut dyn BallHost) -> bool {
        let mut alive = true;
        let blur = host.has_blur();
        match self.eff_mode {
            1 => {
                self.anm_scale = self.scale;
                self.anm_transparency = self.transparency;
                if self.scale_point == 0 {
                    host.out(BallOut::SeNote(61, 36));
                    host.out(BallOut::SeNote(246, 65));
                }
                self.scale_point = ee::add(self.scale_point, 0x3fc0_0000);
                self.scale = [self.scale_point, self.scale_point, self.scale_point, self.scale[3]];
                if !ee::le(self.scale_point, self.max_scale) {
                    self.eff_mode += 1;
                    self.delay = 0;
                    for k in 0..2 {
                        host.out(BallOut::Gen(BallGen::Grown(k), Place::At(self.pos)));
                    }
                    for k in 0..self.time.len() as u8 {
                        host.out(BallOut::Gen(BallGen::Plasma, Place::Follows(Follow::Plasma(k), VF0)));
                        host.out(BallOut::Gen(BallGen::Plasma, Place::Follows(Follow::Plasma2(k), VF0)));
                    }
                }
            }
            2 => {
                self.delay += 1;
                if self.delay < 120 {
                    if self.delay % 6 == 0 {
                        host.out(BallOut::SeNote(229, 60));
                    }
                    self.draw_plasma(host);
                }
                if self.delay == 120 {
                    host.out(BallOut::Gen(BallGen::Gather, Place::At(self.pos)));
                }
                if self.delay == 150 {
                    self.eff_mode += 1;
                    self.delay = 0;
                    self.blur_delay = 80;
                }
            }
            3 => {
                let c = self.delay;
                self.delay += 1;
                if c == self.blur_delay {
                    if blur {
                        host.out(BallOut::Blur(Set::Abgr(0x3080_8080)));
                        host.out(BallOut::Blur(Set::Scale(ONE)));
                    }
                    host.callback(self.pos, 2);
                    alive = false;
                }
                if self.delay >= 21 {
                    self.anm_transparency = ee::sub(self.anm_transparency, 0x3d4c_cccd);
                    if ee::lt(self.anm_transparency, 0) {
                        self.anm_transparency = 0;
                    }
                    if blur {
                        self.de_point = self.blur_delay - self.delay;
                        if self.de_point >= 21 {
                            let s = ee::add(ONE, ee::mul(0x3a83_126f, ee::from_int(self.de_point)));
                            host.out(BallOut::Blur(Set::Scale(s)));
                        } else {
                            self.abgr = self.abgr.wrapping_add(0xfe00_0000);
                            host.out(BallOut::Blur(Set::Abgr(self.abgr)));
                        }
                    }
                }
                if self.delay == 30 {
                    host.callback(self.pos, 1);
                    host.out(BallOut::SeNote(56, 55));
                }
                if self.delay == 30 {
                    host.out(BallOut::SeNote(35, 53));
                }
                if self.delay == 20 {
                    host.out(BallOut::SeNote(56, 60));
                    host.callback(self.pos, 3);
                    if blur {
                        host.out(BallOut::Blur(Set::Scale(0x3f87_ae14)));
                        self.abgr = 0x6080_8080;
                        host.out(BallOut::Blur(Set::Abgr(self.abgr)));
                    }
                    host.out(BallOut::Flash(5, 0x50ff_ffff));
                    for k in 0..3 {
                        host.out(BallOut::Gen(BallGen::Blast(k), Place::At(self.pos)));
                    }
                }
            }
            _ => {}
        }
        alive
    }

    /// `DrawPlasma` (QUA gcmn 0x0036c170): each plasma along its three
    /// legs (Catmull-Rom splines from its third point in to the ball's
    /// centre), a fifth of the way a frame; at each leg's end the next,
    /// and past the third the callback's damage (from every third plasma
    /// but the first). Its two ends are the point turned by `Rot` and the
    /// same turned over by pi about x. `Rot` turns 0.03 a frame.
    fn draw_plasma(&mut self, host: &mut dyn BallHost) {
        let centre = VF0;
        // The local's quadword is the top of a frame under the manager's
        // `Draw`: the cinema's `ccScFade::SendPacket` saved the task's s7
        // (0) there last (or `ccSeOnNote` its return address, as a float
        // nothing), so it starts each frame as zeros.
        self.spline = [0; 4];
        for k in 0..self.time.len() {
            if !ee::lt(self.time[k], 0) {
                if !ee::le(self.time[k], ONE) {
                    self.time[k] = 0;
                    self.mode[k] += 1;
                }
                let (s1, s2, s3) = (self.sb[0][k], self.sb[1][k], self.sb[2][k]);
                let spline = |tt: F, p: [V4; 4]| -> V4 {
                    let mut v = [0, 0, 0, ONE];
                    for (j, c) in v.iter_mut().take(3).enumerate() {
                        *c = catmull(tt, [p[0][j], p[1][j], p[2][j], p[3][j]]);
                    }
                    v
                };
                match self.mode[k] {
                    1 => self.spline = spline(self.time[k], [s3, s3, s2, s1]),
                    2 => self.spline = spline(self.time[k], [s3, s2, s1, centre]),
                    3 => self.spline = spline(self.time[k], [s2, s1, centre, centre]),
                    4 => {
                        if k != 0 && k % 3 == 0 {
                            host.callback(self.pos, 0);
                        }
                        self.mode[k] = 0;
                        for s in self.sb.iter_mut() {
                            s[k][3] = ONE;
                        }
                        self.set_t[k] = ee::sub(self.set_t[k], self.temp_t[k]);
                    }
                    _ => {}
                }
                if ee::lt(self.set_t[k], 0) {
                    self.set_t[k] = ONE;
                }
                let m = geom::rot_matrix_x(&geom::unit_matrix(), self.rot[0]);
                let m = geom::rot_matrix_y(&m, self.rot[1]);
                let m = geom::rot_matrix_z(&m, self.rot[2]);
                let over = geom::rot_matrix_x(&m, NEG_PI);
                let a = geom::apply_matrix(&m, self.spline);
                let b = geom::apply_matrix(&over, self.spline);
                self.spline = a;
                self.pla_pos[k] = geom::vadd(self.pos, a);
                self.pla_pos2[k] = geom::vadd(self.pos, b);
            }
            self.time[k] = ee::add(self.time[k], 0x3e4c_cccd);
        }
        self.count += 1;
        if self.count % 30 == 0 {
            self.cflg = -self.cflg;
        }
        for r in self.rot.iter_mut().take(3) {
            *r = ee::add(*r, 0x3cf5_c28f);
            if !ee::le(*r, PI) {
                *r = ee::sub(*r, TWO_PI);
            }
            if ee::lt(*r, NEG_PI) {
                *r = ee::add(*r, TWO_PI);
            }
        }
    }
}

/// What the ball's `Draw` makes besides its own members: its sounds
/// (`ccSeOnNote`), the blur's writes, the flash (`EntryFlash(5, c, 0, 0,
/// 512, 384)`) and its generators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BallOut {
    SeNote(i32, u8),
    Blur(Set),
    Flash(i32, u32),
    Gen(BallGen, Place),
}

/// Where a ball's `Draw` runs: its callback and blur (either may be
/// none), and what it makes.
pub trait BallHost {
    fn has_blur(&self) -> bool;
    /// `m_cbFunc(Pos, atkFlg)`, when the ball has one.
    fn callback(&mut self, pos: V4, flg: i32);
    fn out(&mut self, o: BallOut);
}

/// The fight's ball: the manager's blur and `KyviaMagicalBallDamage`.
pub(crate) struct FightBall<'a, 'c> {
    pub b: &'a mut BossBase,
    pub x: &'a mut Kyvia,
    pub t: &'a mut Tree,
    pub cx: &'a mut Cx<'c>,
}

impl BallHost for FightBall<'_, '_> {
    fn has_blur(&self) -> bool {
        true
    }

    fn callback(&mut self, pos: V4, flg: i32) {
        ball_damage(self.b, self.x, self.t, self.cx, pos, flg);
    }

    fn out(&mut self, o: BallOut) {
        let cx = &mut *self.cx;
        match o {
            BallOut::SeNote(se, note) => cx.out(Out::SeNote { se, note }),
            BallOut::Blur(s) => cx.out(Out::Blur(s)),
            BallOut::Flash(t, colour) => cx.out(Out::Flash { t, colour }),
            BallOut::Gen(which, at) => super::particles(cx, Gen::Ball(which), at, None),
        }
    }
}

/// `KyviaMagicalBallDamage(pos, atkFlg)` (QUA gcmn 0x003d3c80): 0 and 1
/// the core's boss skills 72 and 73 round `pos`; 2 the attack on
/// (`actProccess`), the core's state 7 and the camera back (mode 4); 3
/// the quake 30 each way.
pub(crate) fn ball_damage(b: &mut BossBase, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx, pos: V4, flg: i32) {
    if x.fight == super::Fight::Fourth {
        super::fourth::ball_damage(b, t, cx, pos, flg);
        return;
    }
    match flg {
        0 | 1 => {
            let Some(c) = t.core.as_mut() else { return };
            let cm = c.me;
            let skill = if flg == 0 { 72 } else { 73 };
            as_me(cx, cm, |cx| c.b.base.skill_damage(cx, None, Some(pos), skill));
        }
        2 => {
            b.act_proccess += 1;
            core_state(t, cx, 7);
            cx.out(Out::CamMode { mode: 4 });
        }
        3 => x.quake_vector[..3].copy_from_slice(&[0x41f0_0000; 3]),
        _ => {}
    }
}

/// `ccBossEffMagicalBallCreate(40, DiscPos, blur, KyviaMagicalBallDamage)`
/// into the manager.
pub(super) fn create(b: &mut BossBase, x: &Kyvia, cx: &mut Cx) {
    let ball = MagicalBall::new(cx.cc, 0x4220_0000, x.disc_pos);
    let kind = EffKind::MagicalBall;
    b.make_fx(cx, kind, ball, x.disc_pos, VF0);
}

/// `MagicBallAtk` (QUA gcmn 0x003d7d40): the core away (state 8) and the
/// player held; at its state 6 the camera out to 5500 and the blur's
/// colour 0x60808080 with the ball's sound; once the camera is there, the
/// ball over the disc (its callback moves the attack on); the quake while
/// it lasts; the colour back; the core back and the act's end.
pub(super) fn magic_ball_atk(b: &mut BossBase, x: &mut Kyvia, t: &mut Tree, cx: &mut Cx) {
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
                cx.out(Out::CamModeRange { mode: 3, range: 0x45ab_e000 });
                cx.out(Out::Blur(Set::Abgr(0x6080_8080)));
                cx.out(Out::SeNote { se: 191, note: 60 });
            }
        }
        2 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
                create(b, x, cx);
                x.quake_vector = VF0;
            }
        }
        3 => {
            let q = x.quake_vector;
            cx.quake_vec([q[0], q[1], q[2]]);
        }
        4 => {
            if cx.cam.reset == 0 {
                b.act_proccess += 1;
                cx.out(Out::Blur(Set::Abgr(x.default_argb)));
            }
        }
        5 if core_fade_check(t) => {
            b.change_action(cx, super::act::NEUTRAL, 1, true);
            b.unlock_player();
        }
        _ => {}
    }
}
