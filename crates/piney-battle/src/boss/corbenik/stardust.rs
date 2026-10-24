//! Corbenik's star dust (`ccBossEffStarDustFlash`, QUA gcmn
//! 0x00374bb0-0x00375c60), with the bolts it throws (`ccBossEffThunder3`,
//! 0x00375c60-0x00376400) and the laser rain it ends in
//! (`ccBossEffLaserRain`, 0x00379600-0x0037a4a0). Sixty-four stars gather
//! over the boss, rise, strike the centre with bolts and fall into it;
//! the core bursts, the camera drops from 3000 over the target, and
//! lasers rain round it. Every one of them draws `ccRandF`, so the rules
//! run them whole; what they draw is handed out as [`Pic`]s.

use piney_data::field::ee;
use piney_data::libm;

use super::Pic;
use crate::boss::{Boss, Cx, EffKind, Out, VF0};
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, V4};

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const STARS: usize = 64;

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

/// `atan2f(a.z - b.z, ccGetDist(a, b))`: the pitch from `b` up to `a`.
fn pitch(cx: &Cx, a: V4, b: V4) -> F {
    let d = dist(cx, a, b);
    libm::atan2f(ee::sub(a[2], b[2]), d)
}

/// An angle into -pi..pi: below -pi a turn on, past pi a turn back.
fn wrap(mut v: F) -> F {
    if ee::lt(v, 0xc049_0fdb) {
        v = ee::add(v, TWO_PI);
    }
    if !ee::le(v, PI) {
        v = ee::sub(v, TWO_PI);
    }
    v
}

/// `Unit, RotY(p), RotZ(y), RotZ(-pi/2)` applied to `v`.
fn turned(p: F, y: F, v: V4) -> V4 {
    let m = geom::rot_matrix_y(&geom::unit_matrix(), p);
    let m = geom::rot_matrix_z(&m, y);
    let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
    geom::apply_matrix(&m, v)
}

/// `STAR_T` (64 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Star {
    /// +0x00 its transparency, +0x10 its place, +0x20 where it began.
    pub scale: F,
    pub pos: V4,
    pub pos0: V4,
    /// +0x30 the step (0-7), +0x32 its count.
    pub state: i16,
    pub count: i16,
    /// +0x34 its distance from the core once risen, +0x38 its size.
    pub reach: F,
    pub size: F,
    pub f60: F,
}

/// `ccBossEffStarDustFlash` (0x180 bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StarDust {
    pub stars: Vec<Star>,
    /// +0x10: the camera drawn back each frame.
    pub follow: bool,
    /// +0x94 a count, +0x98 the phase (0 gather, 1 burst, 2 the camera
    /// down, 3 the rain), +0xb0 the core (the boss 200 up), +0xc0 the stars
    /// risen.
    pub count: i32,
    pub phase: i32,
    pub core: V4,
    pub risen: i32,
    /// +0xc4 the camera's fall a frame and +0xc8 its growth; +0xd0 the
    /// target, +0xe0 the eye over it (3000 up).
    pub cam_spd: F,
    pub cam_acc: F,
    pub target: V4,
    pub top: V4,
    /// +0x164 the core's size, +0x168 camera 3's roll.
    pub core_scale: F,
    pub roll: F,
    /// +0x170 the rain, once made.
    pub rain: Option<Box<LaserRain>>,
}

impl StarDust {
    /// The constructor (0x00374bb0): the stars 400-500 (`ccRandF(100)`)
    /// out from 200 over `pos` at a `ccRandF(pi)` pitch and heading, each
    /// waiting a fifth of its slot in fives; the camera to come down from
    /// 3000 over `target` at 5 a frame, growing by 4.9.
    pub fn new(cx: &mut Cx, pos: V4, target: V4) -> StarDust {
        let mut core = pos;
        core[2] = ee::add(core[2], 0x4348_0000);
        let mut stars = vec![Star::default(); STARS];
        for (k, s) in stars.iter_mut().enumerate() {
            let r = ee::add(0x43c8_0000, rand_f(cx.cc, 0x42c8_0000));
            let ry = rand_f(cx.cc, PI);
            let rz = rand_f(cx.cc, PI);
            let m = geom::rot_matrix_y(&geom::unit_matrix(), ry);
            let m = geom::rot_matrix_z(&m, rz);
            let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
            let v = geom::apply_matrix(&m, [r, 0, 0, ONE]);
            s.pos = geom::vadd(core, v);
            s.pos0 = s.pos;
            s.count = ((k / 5) * 5) as i16;
            s.state = 0;
            s.scale = ONE;
            s.size = 0x4220_0000;
            s.f60 = 0;
        }
        let mut top = target;
        top[2] = ee::add(top[2], 0x453b_8000);
        StarDust {
            stars,
            follow: false,
            count: 0,
            phase: 0,
            core,
            risen: 0,
            cam_spd: 0xc0a0_0000,
            cam_acc: 0xc09c_cccd,
            target,
            top,
            core_scale: 0,
            roll: 0,
            rain: None,
        }
    }

    /// `Draw` (0x00374fd0): its phase; past the fourth, off.
    pub(super) fn draw(&mut self, cx: &mut Cx, b: &mut Boss, slot: usize) -> bool {
        match self.phase {
            0 => self.generate(cx, b, slot),
            1 => self.birst(cx),
            2 => self.camera_down(cx),
            3 => self.fall(cx),
            _ => return false,
        }
        true
    }

    /// The core drawn at its size.
    fn draw_core(&self, cx: &mut Cx) {
        if !geom::eq(0, self.core_scale) {
            cx.out(Out::Corbenik(Pic::StarCore { pos: self.core, scale: self.core_scale }));
        }
    }

    /// `_Generate` (0x00375070).
    fn generate(&mut self, cx: &mut Cx, b: &mut Boss, slot: usize) {
        let mut all_done = true;
        let mut all_four = true;
        // The view the follow reads: the core as written here while the
        // camera is free, else the camera's own.
        let view = if cx.cam.reset == 5 { self.core } else { cx.cam.boss_view };
        if cx.cam.reset == 5 {
            cx.out(Out::CamLook(self.core));
        }
        if self.risen >= STARS as i32 / 3 {
            let c = self.count;
            self.count += 1;
            if c < 60 {
                let z = self.core[2];
                self.core[2] = ee::add(z, ee::div(ee::sub(0x453b_8000, z), 0x4270_0000));
            }
        }
        for k in 0..STARS {
            let s = &mut self.stars[k];
            match s.state {
                0 => {
                    let c = s.count;
                    s.count -= 1;
                    if c < 0 {
                        s.state += 1;
                        s.count = 0;
                    }
                }
                1 => {
                    let mut size = ee::sub(s.size, 0x4100_0000);
                    if ee::lt(size, 0x4120_0000) {
                        size = 0x4120_0000;
                        self.risen += 1;
                        s.state = 3;
                        s.count = 60;
                        s.f60 = 0xc234_0000;
                    }
                    s.size = size;
                }
                3 => {
                    let z = s.pos[2];
                    let gap = ee::sub(0x453b_8000, ee::sub(z, s.pos0[2]));
                    s.pos[2] = ee::add(z, ee::div(gap, 0x4270_0000));
                    let c = s.count;
                    s.count -= 1;
                    if c < 0 {
                        s.count = ((k >> 2) * 5) as i16;
                        s.state += 1;
                        let (p, q) = (s.pos, self.core);
                        let d = dist3d(cx, w2p(cx, p), w2p(cx, q));
                        self.stars[k].reach = d;
                    }
                }
                5 => {
                    let c = s.count;
                    s.count -= 1;
                    if c < 0 {
                        s.count = 45;
                        s.state += 1;
                        let t = Thunder::new(Some((slot, k)), s.pos, self.core, ONE, 15);
                        let id = b.effects.create(EffKind::Thunder3);
                        if let Some(e) =
                            usize::try_from(id).ok().and_then(|i| b.effects.slots.get_mut(i)).and_then(|e| e.as_mut())
                        {
                            e.corbenik = Some(Box::new(super::eff::Fx::Thunder(Box::new(t))));
                        }
                        cx.out(Out::Effect { id, kind: EffKind::Thunder3, pos: self.core, dirc: VF0 });
                    }
                }
                6 => {
                    let c = s.count;
                    s.count -= 1;
                    if c < 0 {
                        self.concentrate(cx, b, k);
                    }
                }
                _ => {}
            }
            let s = &self.stars[k];
            if s.state != 7 {
                all_done = false;
            }
            if s.state != 4 {
                all_four = false;
            }
            if s.state > 0 {
                cx.out(Out::Corbenik(Pic::Star { pos: s.pos, scale: s.scale, size: s.size }));
            }
        }
        self.draw_core(cx);
        if all_four {
            self.follow = true;
            for (k, s) in self.stars.iter_mut().enumerate() {
                s.count = ((k >> 2) * 5) as i16;
                s.state = 5;
            }
        }
        if all_done {
            self.phase += 1;
            self.count = 0;
        }
        if self.follow {
            // The camera drawn back 10 along its eye's line.
            let eye = cx.cam.eye;
            let (ep, vp) = (w2p(cx, eye), w2p(cx, view));
            let d = dist(cx, ep, vp);
            let pt = libm::atan2f(ee::sub(ep[2], vp[2]), d);
            let yaw = get_dirc(ep, vp);
            let m = geom::rot_matrix_x(&geom::unit_matrix(), pt);
            let m = geom::rot_matrix_z(&m, yaw);
            let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
            let v = geom::apply_matrix(&m, [0x4120_0000, 0, 0, ONE]);
            let eye = p2w(cx, geom::vadd(ep, v));
            if cx.cam.reset == 5 {
                cx.out(Out::CamEye(eye));
            }
        }
    }

    /// `_Concentrate(star)` (0x003755a0): the star 25 a frame toward the
    /// core, fading with its way left; there (within 25), a ring (190 +
    /// `ccRand() & 7`, turned by two `ccRandF(pi)`) and the core grown.
    fn concentrate(&mut self, cx: &mut Cx, b: &mut Boss, k: usize) {
        let s = self.stars[k];
        let sp = w2p(cx, s.pos);
        let cp = w2p(cx, self.core);
        let yaw = get_dirc(sp, cp);
        let d3 = dist3d(cx, sp, cp);
        let pt = wrap(pitch(cx, sp, cp));
        if ee::le(d3, 0x41c8_0000) {
            self.stars[k].state += 1;
            let rx = rand_f(cx.cc, PI);
            let rz = rand_f(cx.cc, PI);
            let n = 190 + (cx.cc.rand() & 7);
            let param = [0x3f00_0000, ONE, 0, 0x4220_0000];
            b.effect_at(cx, EffKind::AutoSamonRingOf { n, param }, s.pos, [rx, 0, rz, 0]);
            self.stars[k].scale = 0;
            let grow = ee::div(0x4348_0000, ee::from_int(STARS as i32));
            self.core_scale = ee::add(self.core_scale, grow);
            return;
        }
        let v = turned(pt, yaw, [0x41c8_0000, 0, 0, ONE]);
        self.stars[k].pos = p2w(cx, geom::vadd(sp, v));
        let reach = s.reach;
        let mut f = if !ee::le(reach, 0) { ee::div(d3, reach) } else { 0 };
        if ee::lt(f, 0) {
            f = 0;
        }
        self.stars[k].scale = f;
    }

    /// `_Birst` (0x00375910): 15 frames on a flash, then the camera's fall.
    fn birst(&mut self, cx: &mut Cx) {
        let c = self.count;
        self.count += 1;
        if c == 15 {
            cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
            self.phase += 1;
        }
        self.draw_core(cx);
    }

    /// `_CameraDown` (0x00375990): the eye falls from 3000 over the target,
    /// faster each frame, camera 3 rolling a quarter turn a frame; under
    /// 200, the rain begins and the eye goes 2000 out and 110 up.
    fn camera_down(&mut self, cx: &mut Cx) {
        let z = ee::add(self.top[2], self.cam_spd);
        if ee::lt(z, 0x4348_0000) {
            self.phase += 1;
            self.rain = Some(Box::new(LaserRain::new(cx, self.target)));
            let tp = w2p(cx, self.target);
            let yaw = get_dirc(tp, w2p(cx, VF0));
            let m = geom::rot_matrix_z(&geom::unit_matrix(), yaw);
            let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
            let v = geom::apply_matrix(&m, [0x44fa_0000, 0, 0x42dc_0000, ONE]);
            let eye = p2w(cx, geom::vadd(tp, v));
            let mut view = self.target;
            view[2] = ee::add(view[2], 0x4396_0000);
            cx.out(Out::FreeCam { pos: eye, view });
            self.top[2] = z;
            return;
        }
        self.top[2] = z;
        self.cam_spd = ee::add(self.cam_spd, self.cam_acc);
        cx.out(Out::FreeCam { pos: self.top, view: self.target });
        let r = self.roll;
        cx.out(Out::CameraRot { cam: 3, rot: [r, 0, 0, ONE] });
        let mut r = ee::add(r, HALF_PI);
        if !ee::le(r, PI) {
            r = ee::sub(r, TWO_PI);
        }
        self.roll = r;
    }

    /// `_Fall` (0x00375be0): the rain's `Draw`; over, the end. The camera
    /// quakes (50, 50, 50) every frame.
    fn fall(&mut self, cx: &mut Cx) {
        if let Some(r) = self.rain.as_mut() {
            r.draw(cx);
            if !r.enabled {
                self.phase += 1;
            }
        }
        if cx.boss_cam {
            cx.quake_vec([0x4248_0000; 3]);
        }
    }
}

/// `ccBossEffThunder3` (0x110 bytes): a bolt of eight points from `sp` to
/// `ep`, drawn out a point a frame, held `life` frames and faded over 30.
/// Its start follows a star of the star dust (`follow`: the effect's slot
/// and the star).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Thunder {
    pub sp: V4,
    pub ep: V4,
    pub follow: Option<(usize, usize)>,
    pub alpha: F,
    pub width: F,
    pub shown: i32,
    pub life: i32,
    pub pts: Vec<V4>,
}

impl Thunder {
    pub fn new(follow: Option<(usize, usize)>, sp: V4, ep: V4, width: F, life: i32) -> Thunder {
        Thunder { sp, ep, follow, alpha: ONE, width, shown: 0, life, pts: vec![VF0; 8] }
    }

    /// `Draw` (0x00375f00): the points jittered anew (`ccRandF(200)` aside,
    /// turned by `ccRandF(pi)`, and one more `ccRandF(pi)` unused, each);
    /// the drawn ones as segments.
    pub(super) fn draw(&mut self, cx: &mut Cx, b: &Boss) -> bool {
        if let Some((slot, k)) = self.follow
            && let Some(Some(e)) = b.effects.slots.get(slot)
            && let Some(fx) = &e.corbenik
            && let super::eff::Fx::StarDust(s) = fx.as_ref()
            && let Some(star) = s.stars.get(k)
        {
            self.sp = star.pos;
        }
        let (s, e) = (self.sp, self.ep);
        let n = self.pts.len() as i32;
        let (spp, epp) = (w2p(cx, s), w2p(cx, e));
        let seg = ee::div(dist3d(cx, spp, epp), ee::from_int(n));
        let yaw = get_dirc(spp, epp);
        let pt = pitch(cx, spp, epp);
        for k in 0..n as usize {
            let mut v = [ee::mul(ee::from_int(k as i32), seg), 0, 0, ONE];
            let j = rand_f(cx.cc, 0x4348_0000);
            let rx = rand_f(cx.cc, PI);
            let m = geom::rot_matrix_x(&geom::unit_matrix(), rx);
            let jv = geom::apply_matrix(&m, [0, j, 0, ONE]);
            v = geom::vadd(v, jv);
            let _ = rand_f(cx.cc, PI);
            let v = turned(pt, yaw, v);
            self.pts[k] = geom::vadd(s, v);
        }
        self.pts[0] = s;
        let last = self.pts.len() - 1;
        self.pts[last] = e;
        let shown = (self.shown - 1).max(0) as usize;
        cx.out(Out::Corbenik(Pic::Thunder {
            pts: self.pts[..=shown.min(last)].to_vec(),
            shown: self.shown,
            alpha: self.alpha,
            width: self.width,
        }));
        if self.shown != n {
            self.shown += 1;
            return true;
        }
        if self.life > 0 {
            self.life -= 1;
            return true;
        }
        let mut a = ee::sub(self.alpha, 0x3d08_8889);
        let on = !ee::lt(a, 0);
        if !on {
            a = 0;
        }
        self.alpha = a;
        on
    }
}

/// `LASER_T` (784 bytes): sixteen segments (place, transparency, landed)
/// falling from `start` toward `end` together.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Laser {
    pub segs: Vec<(V4, F, bool)>,
    pub start: V4,
    pub end: V4,
    /// +0x220 the fall a frame (200 along its way).
    pub vec: V4,
    /// +0x234, +0x238 its pitch and heading.
    pub pitch: F,
    pub yaw: F,
    /// +0x300: 0 falling, 1 landed this frame, 3 free.
    pub state: i32,
}

/// `ccBossEffLaserRain` (25152 bytes): 32 lasers over the target, four
/// more every fourth frame to 120 (a flash at 60), each from a random
/// place 1000 up (within 2000) to a random place on the ground within 1000; the first of
/// every fourth to land bursts there (with a radiate before 60). Off once
/// past 120 and none falls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaserRain {
    pub center: V4,
    pub count: i32,
    pub lasers: Vec<Laser>,
    pub enabled: bool,
}

impl LaserRain {
    /// The constructor (0x00379600): each laser set (`_Init`) and free, then
    /// four entered.
    pub fn new(cx: &mut Cx, center: V4) -> LaserRain {
        let mut r = LaserRain { center, count: 0, lasers: vec![Laser::default(); 32], enabled: true };
        for k in 0..32 {
            r.init(cx, k);
            r.lasers[k].state = 3;
        }
        r.entry(cx, 4);
        r.count = 0;
        r
    }

    /// `_Init(laser)` (0x00379f40): its start `ccRandF(2000)` out at a
    /// `ccRandF(pi)` heading, 1000 up; its end `ccRandF(1000)` out on the
    /// ground.
    fn init(&mut self, cx: &mut Cx, k: usize) {
        let c = self.center;
        let r = rand_f(cx.cc, 0x44fa_0000);
        let a = rand_f(cx.cc, PI);
        let start = [
            ee::add(c[0], ee::mul(r, libm::cosf(a))),
            ee::add(c[1], ee::mul(r, libm::sinf(a))),
            ee::add(c[2], 0x447a_0000),
            ee::add(c[3], ONE),
        ];
        let r = rand_f(cx.cc, 0x447a_0000);
        let a = rand_f(cx.cc, PI);
        let end = [
            ee::add(c[0], ee::mul(r, libm::cosf(a))),
            ee::add(c[1], ee::mul(r, libm::sinf(a))),
            ee::add(c[2], 0),
            ee::add(c[3], ONE),
        ];
        let (sp, epp) = (w2p(cx, start), w2p(cx, end));
        let pt = pitch(cx, sp, epp);
        let yaw = get_dirc(sp, epp);
        let vec = turned(pt, yaw, [0x4348_0000, 0, 0, ONE]);
        // The trail: each segment 100 back up the way from the one before.
        let back = turned(pitch(cx, epp, sp), get_dirc(epp, sp), [0x42c8_0000, 0, 0, ONE]);
        let l = &mut self.lasers[k];
        l.start = start;
        l.end = end;
        l.pitch = pt;
        l.yaw = yaw;
        l.vec = vec;
        l.segs = Vec::with_capacity(16);
        let mut at = start;
        for i in 0..16 {
            let alpha = if i == 0 { ONE } else { ee::sub(ONE, ee::div(ee::from_int(i), 0x4180_0000)) };
            if i > 0 {
                at = geom::vadd(at, back);
            }
            l.segs.push((at, alpha, false));
        }
        l.state = 0;
    }

    /// `_Entry(n)` (0x0037a3f0): up to `n` free lasers set falling, each
    /// search going on after the last found.
    fn entry(&mut self, cx: &mut Cx, n: i32) {
        let mut from = 0;
        for _ in 0..n {
            if let Some(k) = (from..32).find(|&k| self.lasers[k].state == 3) {
                self.init(cx, k);
                from = k + 1;
            }
        }
    }

    /// `_LaserHit(laser)` (0x00379e50): the burst at its end; before 60 a
    /// radiate, turned by `ccRandF(pi)`.
    fn hit(&mut self, cx: &mut Cx, k: usize) {
        let end = self.lasers[k].end;
        cx.out(Out::Corbenik(Pic::Burst { pos: end, spd: VF0, scale: ONE, kind: -1, grow: Some(super::GROW) }));
        if self.count < 60 {
            let z = rand_f(cx.cc, PI);
            cx.out(Out::Corbenik(Pic::LaserRadiate { pos: end, turn: z }));
        }
    }

    /// `Draw` (0x00379930).
    fn draw(&mut self, cx: &mut Cx) {
        let c = self.count;
        self.count += 1;
        let mut all_done = true;
        for k in 0..32 {
            if self.lasers[k].state >= 2 {
                continue;
            }
            let mut landed_all = true;
            for s in 0..15 {
                let (seg, next) = (self.lasers[k].segs[s], self.lasers[k].segs[s + 1]);
                if seg.2 || next.2 {
                    continue;
                }
                let vec = self.lasers[k].vec;
                let mut p = geom::vadd(seg.0, vec);
                if ee::le(p[2], 0) {
                    p[2] = 0;
                    self.lasers[k].segs[s].2 = true;
                    self.lasers[k].state = 1;
                    self.lasers[k].segs[s].0 = p;
                    if s == 0 && k & 3 == 0 {
                        self.hit(cx, k);
                    }
                } else {
                    landed_all = false;
                    self.lasers[k].segs[s].0 = p;
                }
                let l = &self.lasers[k];
                cx.out(Out::Corbenik(Pic::Laser { pos: p, pitch: l.pitch, yaw: l.yaw, alpha: l.segs[s].1 }));
            }
            if landed_all {
                self.lasers[k].state = 3;
            } else {
                all_done = false;
            }
        }
        if c < 120 {
            if c & 3 == 0 {
                self.entry(cx, 4);
            }
            if c == 60 {
                cx.out(Out::FlashFade { t_in: 60, t_out: 30, hold: None, colour: 0x80e0_ffff });
            }
        } else if all_done {
            self.enabled = false;
        }
    }
}
