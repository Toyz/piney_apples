//! Corbenik's own boss effects, as the manager's pass runs them: the wave
//! (`ccBossEffWave`, `Draw` 0x00377c80, made by `ccBossEffWaveCreate`
//! 0x0037dd60), whose callback hurts, and a seed's flush
//! (`ccBossEffFlush`, `Draw` 0x00376ea0, `ccBossEffFlushCreate`
//! 0x0037dec0). What they draw is handed out as [`Pic`]s. The energy grow
//! is the flash's alone, which nothing reaches.

use piney_data::field::ee;

use super::Pic;
use crate::boss::{Anm, BossBase, Cx, Out};
use crate::geom::V4;

type F = u32;

/// A Corbenik effect with a life of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fx {
    Wave(Box<Wave>),
    Flush(Flush),
    Transform(Box<Transform>),
    Drain(Box<Drain>),
    StarDust(Box<super::stardust::StarDust>),
    Thunder(Box<super::stardust::Thunder>),
    Tera(Box<super::laser::Tera>),
    Refrect(Box<super::laser::Refrect>),
}

impl Fx {
    /// The effect's `Draw` (out of its slot of `b`'s manager while it
    /// runs): whether it is still enabled. `third` is the third form when
    /// it runs the pass (a reflected laser's callback reaches its eyes).
    pub fn draw(
        &mut self,
        cx: &mut Cx,
        b: &mut BossBase,
        slot: usize,
        third: Option<&mut super::third::Third>,
    ) -> bool {
        match self {
            Fx::Wave(w) => w.draw(cx, b),
            Fx::Flush(f) => f.draw(cx),
            Fx::Transform(t) => t.draw(cx),
            Fx::Drain(d) => d.draw(cx),
            Fx::StarDust(s) => s.draw(cx, b, slot),
            Fx::Thunder(t) => t.draw(cx, b),
            Fx::Tera(t) => t.draw(cx, b),
            Fx::Refrect(r) => match third {
                Some(x) => r.draw(cx, b, x),
                None => r.enabled,
            },
        }
    }
}

/// `ccBossEffWave` (0x40 bytes): `xeffect`'s `ANM_xx11wave` at 512, then
/// the callback (`ccBoss08::WaveFuncCB`) every `Draw`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wave {
    pub anm: Anm,
    pub pos: V4,
}

impl Wave {
    pub fn new(cx: &Cx, pos: V4) -> Wave {
        let mut anm = Anm::new();
        anm.set("ANM_xx11wave", cx.clips);
        anm.speed = 512;
        Wave { anm, pos }
    }

    /// `Draw` (0x00377c80): the clip on (drawn at the place), off once it
    /// ends; `WaveFuncCB(anm, pos)` (0x003adf80) after, every time: at the
    /// clip's frame 8 boss skill 27 from the place and a quake (35, 25,
    /// 50).
    fn draw(&mut self, cx: &mut Cx, b: &mut BossBase) -> bool {
        let ended = self.anm.forward();
        cx.out(Out::Corbenik(Pic::Wave { time: self.anm.posed, pos: self.pos }));
        if self.anm.frame() == 8 {
            b.skill_damage(cx, None, Some(self.pos), 27);
            if cx.boss_cam {
                cx.out(Out::Quake([0x420c_0000, 0x41c8_0000, 0x4248_0000]));
            }
        }
        !ended
    }
}

/// `ccBossEffFlush` (0x30 bytes): particle's `EFF_x000` on `CLT_x000c5`
/// shrinking from `start` to `end` in ten steps, off after the step past.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flush {
    pub pos: V4,
    pub start: F,
    pub end: F,
    pub size: F,
}

impl Flush {
    pub fn new(pos: V4, start: F, end: F) -> Flush {
        Flush { pos, start, end, size: start }
    }

    pub(crate) fn draw(&mut self, cx: &mut Cx) -> bool {
        cx.out(Out::Corbenik(Pic::Flush { pos: self.pos, size: self.size }));
        let v = ee::add(self.size, ee::div(ee::sub(self.end, self.start), 0x4120_0000));
        self.size = v;
        if ee::lt(v, self.end) {
            self.size = self.start;
            return false;
        }
        true
    }
}

/// One of the transform's eight lays (32 bytes): its turn (y and z a
/// `ccRandF(pi)` each), transparency and state (0 off, 1 in, 2 out).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lay {
    pub rot: V4,
    pub alpha: F,
    pub state: i32,
}

/// `ccBossEffTransform` (0x140 bytes, QUA gcmn 0x00372e10-0x003733f0): the
/// second form's barrier rising. `CMP_ex8efc2` (its CLUT `CLT_ex8efc2c1`
/// for `CLT_ex8efc2`) drawn as eight lays fading in and out at its place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Transform {
    pub pos: V4,
    pub count: i32,
    pub state: i32,
    pub lays: [Lay; 8],
}

impl Transform {
    /// The constructor (0x00372e10): eight lays, each drawing two
    /// `ccRandF(pi)`.
    pub fn new(cx: &mut Cx, pos: V4) -> Transform {
        let mut lays = [Lay::default(); 8];
        for l in &mut lays {
            l.rot = [0, 0, 0, 0x3f80_0000];
            l.rot[1] = crate::enemy_ai::rand_f(cx.cc, 0x4049_0fdb);
            l.rot[2] = crate::enemy_ai::rand_f(cx.cc, 0x4049_0fdb);
        }
        Transform { pos, count: 0, state: 0, lays }
    }

    /// `EntryLay(n)` (0x003733a0): up to `n` free lays fading in.
    fn entry_lay(&mut self, mut n: i32) {
        for l in &mut self.lays {
            if n <= 0 {
                break;
            }
            if l.state == 0 {
                l.state = 1;
                l.alpha = 0;
                n -= 1;
            }
        }
    }

    /// `Draw` (0x00373050): the generator and SE 246 (note 52) at once; from
    /// 30 frames two lays every 16, a flash at 60 (`EntryFlash3(120, 30, 30,
    /// 0x80ffffff)`), at 150 the second step: SE 56 at 30, off once its count
    /// reaches 120 (272 draws in all). Each lay in over 30 draws, then out.
    fn draw(&mut self, cx: &mut Cx) -> bool {
        let mut on = true;
        match self.state {
            0 => {
                cx.out(Out::Corbenik(Pic::Particles { which: super::Gen::Transform, pos: self.pos }));
                cx.out(Out::SeNote { se: 246, note: 52 });
                self.state += 1;
            }
            1 => {
                let c = self.count;
                self.count += 1;
                if c >= 30 {
                    if c & 15 == 0 {
                        self.entry_lay(2);
                    }
                    if c == 60 {
                        cx.out(Out::FlashFade { t_in: 120, t_out: 30, hold: Some(30), colour: 0x80ff_ffff });
                    } else if c >= 150 {
                        self.count = 0;
                        self.state += 1;
                    }
                }
            }
            2 => {
                let c = self.count;
                self.count += 1;
                if c == 30 {
                    cx.out(Out::Se { se: 56 });
                }
                if self.count >= 120 {
                    on = false;
                }
            }
            _ => {}
        }
        for l in &mut self.lays {
            if l.state <= 0 {
                continue;
            }
            cx.out(Out::Corbenik(Pic::TransformLay { pos: self.pos, rot: l.rot, alpha: l.alpha }));
            match l.state {
                1 => {
                    let a = ee::add(l.alpha, 0x3d08_8889);
                    l.alpha = a;
                    if !ee::lt(a, 0x3f80_0000) {
                        l.alpha = 0x3f80_0000;
                        l.state = 2;
                    }
                }
                2 => {
                    let a = ee::sub(l.alpha, 0x3d08_8889);
                    l.alpha = a;
                    if ee::le(a, 0) {
                        l.alpha = 0;
                        l.state = 0;
                    }
                }
                _ => {}
            }
        }
        on
    }
}

/// `ccBossEffDrain` (0x50 bytes, `Draw` QUA gcmn 0x00378600), as
/// `ccBossEffRootDrainCreate` makes it at the target: `g_rootDrainGen`'s
/// rows in turn (its place 50 up), the last kept; off once that generator
/// is gone from the particle system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drain {
    pub pos: V4,
    pub state: i32,
    pub count: i32,
    /// The key the last generator is kept under, once made.
    pub key: u32,
    pub kept: bool,
}

impl Drain {
    pub fn new(pos: V4, key: u32) -> Drain {
        Drain { pos, state: 1, count: 0, key, kept: false }
    }

    fn draw(&mut self, cx: &mut Cx) -> bool {
        use super::Gen;
        match self.state {
            1 => {
                self.state = 2;
                self.pos[2] = ee::add(self.pos[2], 0x4248_0000);
                cx.out(Out::Corbenik(Pic::Particles { which: Gen::RootDrain0, pos: self.pos }));
            }
            2 => {
                let c = self.count;
                self.count += 1;
                if c == 35 {
                    self.state = 3;
                    cx.out(Out::Corbenik(Pic::Particles { which: Gen::RootDrain1a, pos: self.pos }));
                    self.count = 0;
                }
            }
            3 => {
                let c = self.count;
                self.count += 1;
                if c == 60 {
                    self.state = 4;
                    cx.out(Out::Corbenik(Pic::Particles { which: Gen::RootDrain1b, pos: self.pos }));
                    self.count = 0;
                }
            }
            4 => {
                let c = self.count;
                self.count += 1;
                if c == 20 {
                    self.state = 5;
                    self.kept = true;
                    cx.out(Out::Corbenik(Pic::Gen { key: self.key, which: Gen::RootDrain2, pos: self.pos }));
                }
            }
            5 if !(cx.gen_alive)(self.key) => {
                self.kept = false;
                return false;
            }
            _ => {}
        }
        true
    }
}
