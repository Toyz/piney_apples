//! The third form's lasers, as the manager's pass runs them: the tera
//! laser (`ccBossEffTeraLaser`, `Draw` QUA gcmn 0x00378d80) with its
//! charge (0x003781e0, `Draw` 0x00378490) and that charge's sparks
//! (`ccBossEffEnergyGrow2`, `Draw` 0x00377f60), and a laser reflected off
//! the eyes (`ccBossEffRefrectionLaser`, `Draw` 0x003773d0). The sparks
//! and the reflections' bursts draw `ccRand`; the rest is their pictures.

use piney_data::field::ee;

use super::third::Third;
use super::{Gen, Pic};
use crate::boss::kyvia::{dp, fabs, fp};
use crate::boss::{Anm, Boss, Cx, EffKind, Out, VF0};
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, V4};

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// The sparks a charge keeps.
pub const SPARKS: usize = 128;
/// A reflected laser's trail.
pub const SEGS: usize = 16;

fn w2p(cx: &Cx, v: V4) -> V4 {
    crate::boss::w2p(cx, v)
}

fn p2w(cx: &Cx, v: V4) -> V4 {
    crate::boss::p2w(cx, v)
}

fn dist(cx: &Cx, a: V4, b: V4) -> F {
    crate::boss::dist(cx, a, b)
}

/// `ccGetDist3D(a, b)`.
fn dist3d(cx: &Cx, a: V4, b: V4) -> F {
    geom::length_on(cx.actx.volume, geom::vsub(b, a))
}

/// `effExplode3(pos, 0, scale, kind)` grown by `grow`.
fn burst(cx: &mut Cx, pos: V4, scale: F, kind: i32, grow: Option<[F; 3]>) {
    cx.out(Out::Corbenik(Pic::Burst { pos, spd: VF0, scale, kind, grow }));
}

// --- the tera laser --------------------------------------------------------------------------------

/// A spark of the charge (48 bytes of `ccBossEffEnergyGrow2`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spark {
    pub pos: V4,
    pub rot: V4,
    pub frame: i32,
    pub alpha: F,
    pub turn: F,
}

/// `ccBossEffEnergyGrow2` (6224 bytes): sparks born on a sphere round
/// `center` (11 every fourth draw), each fading over 30.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grow {
    pub sparks: Vec<Spark>,
    /// +0x1820 the sphere's radius, +0x1830 its centre, +0x1840 the count.
    pub radius: F,
    pub center: V4,
    pub count: i32,
    pub enabled: bool,
}

impl Grow {
    /// `ccBossEffEnergyGrow2(pos)` (0x00377d40).
    pub fn new(pos: V4) -> Grow {
        Grow { sparks: vec![Spark::default(); SPARKS], radius: 0, center: pos, count: 0, enabled: true }
    }

    /// The spawn (0x00378090): up to `n` + 1 free sparks lit, each at
    /// `(radius, 0, 0)` turned by `RotMatrix(ccRandF(pi), 0, ccRandF(pi))`
    /// about the centre, its turn `ccRandF(pi)`.
    fn spawn(&mut self, cx: &mut Cx, mut n: i32) {
        for s in &mut self.sparks {
            if !geom::eq(0, s.alpha) {
                continue;
            }
            s.alpha = ONE;
            s.rot = VF0;
            s.rot[0] = rand_f(cx.cc, PI);
            s.rot[2] = rand_f(cx.cc, PI);
            let m = geom::rot_matrix(&geom::unit_matrix(), s.rot);
            let v = geom::apply_matrix(&m, [self.radius, 0, 0, ONE]);
            s.pos = geom::vadd(self.center, v);
            s.frame = 0;
            s.turn = rand_f(cx.cc, PI);
            let k = n;
            n -= 1;
            if k == 0 {
                break;
            }
        }
    }

    /// `Draw` (0x00377f60).
    fn draw(&mut self, cx: &mut Cx) {
        let c = self.count;
        self.count += 1;
        if c & 3 == 3 {
            self.spawn(cx, 10);
        }
        for s in &mut self.sparks {
            if ee::le(s.alpha, 0) {
                continue;
            }
            cx.out(Out::Corbenik(Pic::Spark { pos: s.pos, frame: s.frame, alpha: s.alpha }));
            s.frame += 1;
            let mut a = ee::sub(s.alpha, 0x3d08_8889);
            if ee::lt(a, 0) {
                a = 0;
            }
            s.alpha = a;
        }
    }
}

/// The tera laser's charge (0x60 bytes, made at 0x003781e0): two
/// generators where it is, and its sparks' sphere growing for 150 draws,
/// then shrinking to nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Charge {
    pub pos: V4,
    pub count: i32,
    pub state: i32,
    pub radius: F,
    pub grow: Grow,
    /// The glow's generator (+0x10), killed with the laser.
    pub glow: u32,
}

impl Charge {
    fn new(cx: &mut Cx, pos: V4, glow: u32) -> Charge {
        cx.out(Out::Corbenik(Pic::Particles { which: Gen::TeraCharge, pos }));
        cx.out(Out::Corbenik(Pic::Gen { key: glow, which: Gen::TeraChargeGlow, pos }));
        Charge { pos, count: 0, state: 0, radius: 0, grow: Grow::new(pos), glow }
    }

    /// `Draw` (0x00378490).
    fn draw(&mut self, cx: &mut Cx) {
        let c = self.count;
        self.count += 1;
        if !ee::le(self.radius, 0) {
            self.grow.radius = self.radius;
            self.grow.draw(cx);
        }
        match self.state {
            0 => {
                self.radius = ee::add(self.radius, 0x4055_5555);
                if c >= 150 {
                    self.radius = 0x43c8_0000;
                    self.count = 0;
                    self.state += 1;
                }
            }
            1 => {
                let r = ee::sub(self.radius, 0x4205_5555);
                self.radius = r;
                if ee::le(r, 0) {
                    self.radius = 0;
                    self.grow.enabled = false;
                    self.count = 0;
                    self.state += 1;
                }
            }
            _ => {}
        }
    }
}

/// `ccBossEffTeraLaser` (0xb0 bytes): the charge 200 before it, a flash
/// after 170 draws, the camera high over the target, the beam falling 200
/// a draw from 3000 onto it; a ring and bursts, the callback (boss skill
/// 56) at 25, three more bursts; the beam thinning out from 40.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tera {
    pub count: i32,
    pub state: i32,
    pub pos: V4,
    pub target: V4,
    pub rot: V4,
    pub scale: V4,
    /// The fall's generator (+0x10), following the beam, and its key.
    pub fall: Option<u32>,
    pub fall_key: u32,
    pub charge: Charge,
    pub cb: bool,
    pub enabled: bool,
}

impl Tera {
    /// `ccBossEffTeraLaser(pos, rot, scale, target, cb)` (0x00378900).
    /// `keys` names its two generators.
    pub fn new(cx: &mut Cx, pos: V4, rot: V4, scale: V4, target: V4, keys: [u32; 2]) -> Tera {
        let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI), rot[2]);
        let o = geom::apply_matrix(&m, [0x4348_0000, 0, 0, ONE]);
        let at = p2w(cx, geom::vadd(w2p(cx, pos), o));
        let charge = Charge::new(cx, at, keys[0]);
        Tera {
            count: 0,
            state: 0,
            pos,
            target,
            rot,
            scale,
            fall: None,
            fall_key: keys[1],
            charge,
            cb: true,
            enabled: true,
        }
    }

    /// `Draw` (0x00378d80): whether it is still on. `b` is the body
    /// (`cmn_thisPtr`, the callback's).
    pub fn draw(&mut self, cx: &mut Cx, b: &mut Boss) -> bool {
        let fall_key = self.fall_key;
        let c = self.count;
        self.count += 1;
        self.charge.draw(cx);
        match self.state {
            0 => {
                if c >= 170 {
                    cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                    self.count = 0;
                    self.state += 1;
                }
            }
            1 => {
                if c == 30 {
                    let t = w2p(cx, self.target);
                    let mut e = t;
                    e[1] = ee::add(e[1], 0x4554_8000);
                    e[2] = ee::add(e[2], 0x44a2_8000);
                    let mut v = t;
                    v[1] = ee::add(v[1], 0x452f_0000);
                    v[2] = ee::add(v[2], 0x448f_c000);
                    let (e, v) = (p2w(cx, e), p2w(cx, v));
                    cx.out(Out::FreeCam { pos: e, view: v });
                } else if c >= 31 {
                    if c == 31 {
                        self.rot[0] = NEG_HALF_PI;
                        self.pos = self.target;
                        self.pos[2] = 0x453b_8000;
                        self.fall = Some(fall_key);
                        cx.out(Out::Corbenik(Pic::Gen { key: fall_key, which: Gen::TeraFall, pos: self.pos }));
                    }
                    let z = ee::sub(self.pos[2], 0x4348_0000);
                    self.pos[2] = z;
                    if ee::le(z, self.target[2]) {
                        self.pos[2] = self.target[2];
                        self.count = 0;
                        self.state += 1;
                    }
                    if let Some(key) = self.fall {
                        cx.out(Out::Corbenik(Pic::GenMove { key, pos: self.pos }));
                    }
                }
            }
            2 => {
                if c < 41 {
                    cx.quake_vec([0x4248_0000, 0x4248_0000, 0x4270_0000]);
                }
                let t = self.target;
                if c == 5 {
                    let param = [0, 0x40d5_5555, 0x447a_0000, ONE];
                    b.effect_at(cx, EffKind::AutoSamonRingOf { n: 195, param }, t, VF0);
                    burst(cx, t, 0, 0, Some([0, 0x4080_0000, 0x4170_0000]));
                } else if c == 25 {
                    if self.cb {
                        b.skill_damage(cx, None, Some(t), 56);
                    }
                    let mut a: F = 0;
                    for _ in 0..3 {
                        a = ee::add(a, 0x4006_0a92);
                        if !ee::le(a, PI) {
                            a = ee::sub(a, TWO_PI);
                        }
                        let mut p = w2p(cx, t);
                        p[0] = ee::add(p[0], ee::mul(0x43fa_0000, piney_data::libm::sinf(a)));
                        p[1] = ee::add(p[1], ee::mul(0x43fa_0000, piney_data::libm::cosf(a)));
                        let p = p2w(cx, p);
                        let s = fp(dp(fabs(rand_f(cx.cc, 0x40a0_0000))) * 15.0);
                        burst(cx, p, ONE, 0, Some([0x4080_0000, 0x4120_0000, s]));
                    }
                } else if c >= 40 {
                    if c == 55 {
                        cx.out(Out::Corbenik(Pic::Explode2 { pos: t }));
                        burst(cx, t, ONE, 0, Some([0x4080_0000, 0x4120_0000, 0x4170_0000]));
                    }
                    let mut s = ee::sub(self.scale[0], 0x3daa_aaab);
                    self.scale[0] = s;
                    if ee::lt(s, 0) {
                        s = 0;
                        self.scale[0] = 0;
                        self.count = 0;
                        self.state += 1;
                    }
                    self.scale[1] = s;
                }
            }
            3 if c == 60 => self.enabled = false,
            _ => {}
        }
        if self.state != 0 {
            cx.out(Out::Corbenik(Pic::TeraBeam { pos: self.pos, rot: self.rot, scale: self.scale }));
        }
        if !self.enabled {
            // The destructor: the charge's glow killed if it still runs.
            if (cx.gen_alive)(self.charge.glow) {
                cx.out(Out::Corbenik(Pic::GenKill { key: self.charge.glow }));
            }
        }
        self.enabled
    }
}

// --- the reflected laser ---------------------------------------------------------------------------

/// One of a reflected laser's sixteen trail points (32 bytes at +0x20).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seg {
    pub pos: V4,
    pub alpha: F,
    pub done: bool,
}

/// `ccBossEffRefrectionLaser` (0x260 bytes): a head running 200 a draw
/// from the eye dummy through each eye in turn (the callback at each, the
/// eye's place taken as it was made) to the member, its trail of sixteen
/// behind it; over once every point has reached the member (the head's
/// arrival a burst, SE 35 and 30 draws of quake).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refrect {
    /// +0x250 the points (start, the eyes, the end), +0x224 the eyes (by
    /// slot), +0x258 the point it runs to.
    pub pts: Vec<V4>,
    pub eyes: Vec<usize>,
    pub idx: usize,
    pub segs: [Seg; SEGS],
    /// +0x25c its speed, +0x10 the quake's draws left.
    pub speed: F,
    pub shake: i32,
    /// +0x230 the head's `ANM_ex31les3`.
    pub head: Anm,
    pub enabled: bool,
}

impl Refrect {
    /// `ccBossEffRefrectionLaser(sp, ep, eyes, n, cb)` (0x00376f60): the
    /// trail all at `sp`, fading by sixteenths along it.
    pub fn new(cx: &Cx, sp: V4, ep: V4, eyes: Vec<usize>, eye_pos: &[V4]) -> Refrect {
        let mut pts = Vec::with_capacity(eyes.len() + 2);
        pts.push(sp);
        pts.extend(eye_pos.iter().copied());
        pts.push(ep);
        let mut segs = [Seg::default(); SEGS];
        let mut a = ONE;
        for s in &mut segs {
            s.pos = sp;
            s.alpha = a;
            a = ee::sub(a, 0x3d80_0000);
            if ee::lt(a, 0) {
                a = 0;
            }
            s.done = false;
        }
        let mut head = Anm::new();
        head.set("ANM_ex31les3", cx.clips);
        Refrect { pts, eyes, idx: 1, segs, speed: 0x4348_0000, shake: 0, head, enabled: true }
    }

    /// `Draw` (0x003773d0). `x` is the third form (its eyes, the
    /// callback's `_RefrectLaserCB`).
    pub fn draw(&mut self, cx: &mut Cx, b: &mut Boss, x: &mut Third) -> bool {
        if !self.segs[0].done {
            self.head.forward();
            let (from, to) = (self.segs[0].pos, self.segs[1].pos);
            cx.out(Out::Corbenik(Pic::RefrectHead { time: self.head.posed, from, to }));
        }
        for k in 0..SEGS - 1 {
            if !self.segs[k].done {
                let (from, to, alpha) = (self.segs[k].pos, self.segs[k + 1].pos, self.segs[k].alpha);
                cx.out(Out::Corbenik(Pic::RefrectSeg { from, to, alpha }));
            }
        }
        let last = self.pts.len() - 1;
        if self.idx == last {
            let tgt = w2p(cx, self.pts[self.idx]);
            let mut all = true;
            for k in 0..SEGS {
                if self.segs[k].done {
                    continue;
                }
                let d = dist3d(cx, w2p(cx, self.segs[k].pos), tgt);
                if !ee::le(d, self.speed) {
                    all = false;
                    continue;
                }
                self.segs[k].done = true;
                if k != 0 {
                    continue;
                }
                let mut v = VF0;
                v[0] = rand_f(cx.cc, 0x42c8_0000);
                let m = geom::rot_matrix_y(&geom::unit_matrix(), rand_f(cx.cc, PI));
                let m = geom::rot_matrix_z(&m, rand_f(cx.cc, PI));
                let v = geom::apply_matrix(&m, v);
                let p = p2w(cx, geom::vadd(w2p(cx, self.segs[k].pos), v));
                burst(cx, p, 0x3fc0_0000, -1, None);
                cx.out(Out::Se { se: 35 });
                self.shake = 30;
            }
            if all {
                self.enabled = false;
            }
        } else if !self.segs[0].done {
            let at = self.pts[self.idx];
            let d = dist3d(cx, w2p(cx, self.segs[0].pos), w2p(cx, at));
            if ee::le(d, self.speed) {
                self.segs[0].pos = at;
                let k = self.eyes[self.idx - 1];
                refrect_cb(cx, b, x, k);
                self.idx += 1;
            }
        }
        for k in (1..SEGS).rev() {
            self.segs[k].pos = self.segs[k - 1].pos;
        }
        let head = w2p(cx, self.segs[0].pos);
        let tgt = w2p(cx, self.pts[self.idx]);
        let y = piney_data::libm::atan2f(ee::sub(head[2], tgt[2]), dist(cx, head, tgt));
        let m = geom::rot_matrix_y(&geom::unit_matrix(), y);
        let m = geom::rot_matrix_z(&m, get_dirc(head, tgt));
        let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
        let v = geom::apply_matrix(&m, [self.speed, 0, 0, ONE]);
        if !self.segs[0].done {
            self.segs[0].pos = p2w(cx, geom::vadd(head, v));
        }
        if self.shake > 0 {
            self.shake -= 1;
            if cx.boss_cam {
                cx.quake_vec([0x42a0_0000, 0x42a0_0000, 0x42c8_0000]);
            }
        }
        self.enabled
    }
}

/// `_RefrectLaserCB(eye)` (0x003b9140): three rings at the eye (each
/// turned by two `ccRandF(pi)`, `190 + abs(ccRand()) & 7`, params 0.5, 1,
/// 0, 40), then its flash (`Order(23)`).
fn refrect_cb(cx: &mut Cx, b: &mut Boss, x: &mut Third, k: usize) {
    let me = x.eye_me[k];
    let pos = cx.scene.chars[me].pos;
    for _ in 0..3 {
        let rx = rand_f(cx.cc, PI);
        let rz = rand_f(cx.cc, PI);
        let n = (cx.cc.rand().abs() & 7) + 190;
        let param = [0x3f00_0000, ONE, 0, 0x4220_0000];
        b.effect_at(cx, EffKind::AutoSamonRingOf { n, param }, pos, [rx, 0, rz, ONE]);
    }
    x.flash_eye(k);
}
