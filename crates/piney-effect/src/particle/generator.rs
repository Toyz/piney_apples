//! `ccParticleGenerator` (main particle.cpp, 0x100 bytes): a source of
//! particles - its constructor (0x001bd920), `Main` (0x001bed00) and
//! `Generate` (0x001bdda0).
//!
//! ```text
//! new ccParticleGenerator(param, f1..f4):
//!   gRate pGenRate / 30 (particles a frame), gRateCnt 1 - gRate below 1;
//!   distSW; the force fields f1..f4 or the row's ffNum; gSyncFlag and
//!   pSyncFlag from the row, gLife and pLife; pos, pos2, offset, offset2
//!   (0, 0, 0, 1); the next generatorSerialNum; pTexMod -1
//! Main, each frame:
//!   (the stream demo's strEffectStopFlag stops stream generators)
//!   with gSyncFlag: rot = *syncRot; pos = offset + *syncPos (w 1), pos2 =
//!     offset2 + *syncPos2
//!   mat = RotMatrix(rot)
//!   pauseFlag: nothing more
//!   alive (killFlag below 2, *syncSW not 0, gAge below gLife or gLife -1):
//!     gType 0: as many Generate as gRateCnt (+= gRate) passes whole
//!     numbers; 1: one; 2: pGenRate of them while pNum is 0
//!   else ParticleKill (killFlag 2: its particles fade out) and, once none
//!     are left (pCnt 0), endFlag (ccParticleCtrl::Main deletes it)
//!   gAge counts on while the row's gLife is not -1
//! Generate: a free particle (none: nothing); rand() >> 3 for where,
//!   Setup(pTexMod or pTex, pPatRnd, distSW), then by rType where it
//!   starts, rand() >> 3 again and by dType which way and how fast (the
//!   direction through mat), rand() >> 3 for a random turn (pRotRnd
//!   sprites), rand() >> 3 for its life: pLife less up to pLifeRR of it
//! ```

use super::tables::{FfParam, GenParam};
use super::{GenRef, IntRef, Particles, Step, normal2angle, one};
use crate::ee::{self, F, ONE, V4, VF0};
use crate::vu::{self, M4};
use crate::{Host, VecRef};

/// A `ccParticleGenerator`.
#[derive(Clone, Debug, PartialEq)]
pub struct Generator {
    /// The `particleGeneratorTbl` row `param` is, as far as it is one
    /// (`usize::MAX` for a row of another table).
    pub row: usize,
    /// +0x00 `param` (None for effect.cpp's static stand-ins, which are
    /// never run).
    pub param: Option<GenParam>,
    /// +0x04 bits: `distSW`, `gSyncFlag`, `pSyncFlag`, `pStopFlag`,
    /// `syncPosType` (the game reads `syncPos` + 0x10; here the
    /// [`VecRef`] names the vector read), `syncPosType2`, `strFlag` (signed
    /// 2 bits), +0x05 `pauseFlag`.
    pub dist_sw: bool,
    pub g_sync: bool,
    pub p_sync: bool,
    pub p_stop: bool,
    pub sync_pos_type: bool,
    pub sync_pos_type2: bool,
    pub str_flag: i8,
    pub pause: bool,
    /// +0x06 `pCnt` (live particles), +0x08 `gAge`, +0x0a `gLife`, +0x0c
    /// `pLife`, +0x0e `pNum` (made), +0x10 `regularlyOfst`.
    pub p_cnt: i16,
    pub g_age: i16,
    pub g_life: i16,
    pub p_life: i16,
    pub p_num: i16,
    pub regularly_ofst: i16,
    /// +0x14 `syncRot`, +0x18 `syncPos`, +0x1c `syncPos2`, +0x20 `syncSW`.
    pub sync_rot: Option<VecRef>,
    pub sync_pos: Option<VecRef>,
    pub sync_pos2: Option<VecRef>,
    pub sync_sw: Option<IntRef>,
    /// +0x24 `gRate`, +0x28 `gRateCnt`, +0x2c `pTexMod` (-1: the row's).
    pub g_rate: F,
    pub g_rate_cnt: F,
    pub p_tex_mod: i16,
    /// +0x34 `ff[4]`: the force fields' parameters.
    pub ff: [Option<FfParam>; 4],
    /// +0x44 `layer` its particles draw on (a priority; None the effect
    /// layer).
    pub layer: Option<i16>,
    /// +0x50 rot, +0x60 pos, +0x70 pos2, +0x80 offset, +0x90 offset2,
    /// +0xa0 velocity (not read), +0xb0 mat.
    pub rot: V4,
    pub pos: V4,
    pub pos2: V4,
    pub offset: V4,
    pub offset2: V4,
    pub velocity: V4,
    pub mat: M4,
    /// +0xf0 `killFlag` (1 its particles follow its end, 2 killed: they
    /// fade out, 3 deleted: they end), +0xf1 `endFlag`, +0xf4 `sn`.
    pub kill_flag: i8,
    pub end_flag: bool,
    pub sn: u32,
}

impl Generator {
    /// `new ccParticleGenerator(param, f1, f2, f3, f4)` (main 0x001bd920):
    /// `ff` the force fields given (None: the row's `ffNum`; none at all
    /// without a row), `serial` `generatorSerialNum`.
    pub fn new(param: Option<GenParam>, ff: [Option<FfParam>; 4], table: &[FfParam], serial: &mut u32) -> Generator {
        let g_rate = param.map_or(0, |p| ee::div(p.p_gen_rate, 0x41f0_0000));
        let g_rate_cnt = if ee::lt(g_rate, ONE) { ee::sub(ONE, g_rate) } else { 0 };
        let ff = std::array::from_fn(|k| {
            let p = param?;
            ff[k].or_else(|| usize::try_from(p.ff_num[k]).ok().and_then(|i| table.get(i)).copied())
        });
        let sn = *serial;
        *serial = serial.wrapping_add(1);
        let row = param.map_or(usize::MAX, |p| p.row);
        Generator {
            row,
            param,
            dist_sw: true,
            g_sync: param.is_some_and(|p| p.g_sync),
            p_sync: param.is_some_and(|p| p.p_sync),
            p_stop: false,
            sync_pos_type: false,
            sync_pos_type2: false,
            str_flag: 0,
            pause: false,
            p_cnt: 0,
            g_age: 0,
            g_life: param.map_or(-999, |p| p.g_life),
            p_life: param.map_or(-999, |p| p.p_life),
            p_num: 0,
            regularly_ofst: 0,
            sync_rot: None,
            sync_pos: None,
            sync_pos2: None,
            sync_sw: None,
            g_rate,
            g_rate_cnt,
            p_tex_mod: -1,
            ff,
            layer: None,
            rot: [0; 4],
            pos: VF0,
            pos2: VF0,
            offset: VF0,
            offset2: VF0,
            velocity: [0; 4],
            mat: [[0; 4]; 4],
            kill_flag: 0,
            end_flag: false,
            sn,
        }
    }

    /// Its handle.
    pub fn id(&self) -> GenRef {
        self.sn
    }
}

/// 100.0, 30.0, 16128.0.
const HUNDRED: F = 0x42c8_0000;
const SPREAD: F = 0x467c_0000;

/// `x - float(r % 101) * (x * rr) / 100`: a value less up to `rr` of it.
fn reduced(x: F, rr: F, r: i32) -> F {
    let t = ee::mul(x, rr);
    ee::sub(x, ee::div(ee::mul(ee::from_int(r % 101), t), HUNDRED))
}

/// The angle `s` (the game's 16 bits) as radians.
fn ang(s: i32) -> F {
    ee::deg2rad(s as i16)
}

/// `(a / 2)` as C truncates it.
fn half(a: i16) -> i32 {
    i32::from(a) / 2
}

impl Particles {
    /// `ccParticleGenerator::Main()` (main 0x001bed00) of generator `i`.
    pub(super) fn generator_main(&mut self, i: usize, step: &mut Step) {
        if self.str_effect_stop && self.gens[i].str_flag != 0 {
            return;
        }
        {
            let g = &self.gens[i];
            let rot = if g.g_sync { g.sync_rot.map(|r| step.resolve(r)) } else { None };
            let pos = if g.g_sync { g.sync_pos.map(|r| step.resolve(r)) } else { None };
            let pos2 = if g.g_sync { g.sync_pos2.map(|r| step.resolve(r)) } else { None };
            let g = &mut self.gens[i];
            if let Some(r) = rot {
                g.rot = r;
            }
            if let Some(p) = pos {
                g.pos = ee::vadd(g.offset, p);
                g.pos[3] = ONE;
            }
            if let Some(p) = pos2 {
                g.pos2 = ee::vadd(g.offset2, p);
                g.pos2[3] = ONE;
            }
            g.mat = vu::rot_zyx(&vu::UNIT, g.rot);
            if g.pause {
                return;
            }
        }
        let g = &self.gens[i];
        let sw_on = g.sync_sw.is_none_or(|s| match s {
            // An effect's temp[k] (effDrain's orbs clear temp[1] as they end).
            IntRef::EffectTemp(k, t) => step.effects.effects[k].temp[t] != 0,
            IntRef::EffectFlags(k) => step.effects.effects[k].flags != 0,
            other => step.host.int(other) != 0,
        });
        let alive = g.kill_flag < 2 && sw_on && (g.g_life == -1 || g.g_age < g.g_life);
        if alive {
            let g = &mut self.gens[i];
            let a = ee::to_int(g.g_rate_cnt);
            g.g_rate_cnt = ee::add(g.g_rate_cnt, g.g_rate);
            let b = ee::to_int(g.g_rate_cnt);
            let param = g.param.unwrap_or_default();
            match param.g_type {
                0 => {
                    for _ in 0..b.wrapping_sub(a).max(0) {
                        self.generate(i, step);
                    }
                }
                1 => {
                    self.generate(i, step);
                }
                2 if g.p_num == 0 => {
                    let mut n = 0;
                    while ee::lt(ee::from_int(n), param.p_gen_rate) {
                        self.generate(i, step);
                        n += 1;
                    }
                }
                _ => {}
            }
        } else {
            if g.kill_flag < 2 {
                self.particle_kill(self.gens[i].sn);
            }
            let g = &mut self.gens[i];
            if g.p_cnt <= 0 {
                g.end_flag = true;
            }
        }
        let g = &mut self.gens[i];
        if g.param.unwrap_or_default().g_life != -1 && g.g_age >= 0 {
            g.g_age = g.g_age.wrapping_add(1);
        }
    }

    /// `ccParticleGenerator::Generate()` (main 0x001bdda0): a particle from
    /// generator `i`, its slot.
    pub(super) fn generate(&mut self, i: usize, step: &mut Step) -> Option<usize> {
        let s = self.generate_particle()?;
        let host: &mut dyn Host = step.host;
        let mut r = host.rand() >> 3;
        let g = &self.gens[i];
        // A generator without a row (the static stand-ins) is never run.
        let param = g.param.unwrap_or_default();
        let tex = if g.p_tex_mod == -1 { i32::from(param.p_tex) } else { i32::from(g.p_tex_mod) };
        one::setup(
            &mut self.slots[s],
            Some(g),
            tex,
            i32::from(param.p_pat_rnd),
            g.dist_sw,
            host,
            step.assets,
            &mut self.particle_serial,
        );
        self.count_up();
        self.p2_num = self.p2_num.wrapping_add(1);
        let g = &mut self.gens[i];
        g.p_cnt = g.p_cnt.wrapping_add(1);
        g.p_num = g.p_num.wrapping_add(1);
        let g = &self.gens[i];
        let p = &mut self.slots[s];
        let place = |p: &mut one::Particle, t: V4| {
            if param.p_sync {
                p.pos = t;
            } else {
                p.pos = ee::vadd(g.pos, t);
                p.pos[3] = ONE;
            }
        };
        // What rType 5 keeps for dType 1: the point on the line.
        let mut line_at: V4 = [0; 4];
        match param.r_type {
            0 => {
                if param.p_sync {
                    p.pos = VF0;
                } else {
                    p.pos = g.pos;
                    p.pos[3] = ONE;
                }
            }
            1 => {
                let radius = reduced(param.g_radius, param.g_radius_rr, r);
                let a = if param.d_type == 3 {
                    let n = ee::to_int(param.p_gen_rate);
                    let step_a = if n == 0 { 0 } else { (i32::from(g.p_num) << 16) / n };
                    i32::from(g.regularly_ofst) + i32::from(param.p_dirc) + step_a
                } else {
                    (r << 8) & 0xff00
                };
                let a = ang(a);
                let t = [ee::mul(radius, ee::sinf(a)), ee::mul(ee::neg(radius), ee::cosf(a)), 0, ONE];
                place(p, t);
            }
            2 => {
                let radius = reduced(param.g_radius, param.g_radius_rr, r);
                let a = ang((r % 28672) - 12288);
                let z = ee::mul(radius, ee::sinf(a));
                let rc = ee::mul(radius, ee::cosf(a));
                let b = ang((r << 8) & 0xff00);
                let t = [ee::mul(rc, ee::sinf(b)), ee::mul(ee::neg(rc), ee::cosf(b)), z, ONE];
                place(p, t);
            }
            3 => {
                let radius = reduced(param.g_radius, param.g_radius_rr, r);
                let k = ((r >> 5) % 42) as usize;
                let t = ee::vscale(step.assets.particle.polyhedron[k], radius);
                place(p, t);
            }
            4 => {
                let t = ee::vsub(g.pos2, g.pos);
                let d = ee::sqrtf(ee::dot(t, t));
                let t = ee::normalize(t);
                p.temp = if param.d_type == 3 {
                    ee::div(ee::mul(d, ee::from_int(i32::from(g.p_num))), ee::add(ONE, param.p_gen_rate))
                } else {
                    ee::div(ee::mul(d, ee::from_int(r & 0x3f00)), SPREAD)
                };
                let t = ee::vscale_xyz(t, p.temp);
                place(p, t);
            }
            5 => {
                let line = ee::vsub(g.pos2, g.pos);
                let an = normal2angle(ee::normalize(line));
                let m = vu::rot_z(&vu::rot_x(&vu::UNIT, an[0]), an[2]);
                let pct = ee::to_int(ee::mul(param.g_radius_rr, ee::from_int(r % 101)));
                let radius = ee::sub(param.g_radius, ee::div(ee::mul(param.g_radius, ee::from_int(pct)), HUNDRED));
                let b = ang((r << 8) & 0xff00);
                let t = [ee::mul(radius, ee::sinf(b)), 0, ee::mul(ee::neg(radius), ee::cosf(b)), ONE];
                let t = ee::apply(&m, t);
                let d = ee::sqrtf(ee::dot(line, line));
                let along = if param.d_type == 3 {
                    ee::div(ee::mul(d, ee::from_int(i32::from(g.p_num))), ee::add(ONE, param.p_gen_rate))
                } else {
                    ee::div(ee::mul(d, ee::from_int(r & 0x3f00)), SPREAD)
                };
                line_at = ee::vscale_xyz(ee::normalize(line), along);
                let mut t = ee::vadd(t, line_at);
                t[3] = ONE;
                if param.p_sync {
                    p.pos = t;
                    p.rotate[1] = (step.host.rand() & 0xfc00) as u16;
                    p.ofst_r = (100 - pct) as i16;
                    p.ofst_d = ee::to_int(ee::div(ee::mul(HUNDRED, along), d)) as i16;
                } else {
                    p.pos = ee::vadd(g.pos, t);
                    p.pos[3] = ONE;
                }
            }
            _ => {}
        }
        r = step.host.rand() >> 3;
        let mat = g.mat;
        let speed = |r: i32| reduced(param.p_iv, param.p_iv_rr, r);
        let fly = |p: &mut one::Particle, dirc: V4| {
            p.dirc = dirc;
            p.dirc[3] = ONE;
            p.dirc = ee::apply(&mat, p.dirc);
            p.dirc[3] = ONE;
            p.dirc
        };
        match param.d_type {
            0 | 5 => {
                let a = i32::from(param.p_range) - half(param.p_range_rr)
                    + (i32::from(param.p_range_rr) * (r & 0xff)) / 256;
                let a = ang(a);
                let z = ee::sinf(a);
                let c = ee::cosf(a);
                let b = i32::from(param.p_dirc) - half(param.p_dirc_rr)
                    + (i32::from(param.p_dirc_rr) * ((r >> 12) & 0xff)) / 256;
                let b = ang(b);
                let t = [ee::mul(c, ee::sinf(b)), ee::mul(ee::neg(c), ee::cosf(b)), z, 0];
                let d = fly(p, ee::normalize(t));
                if param.d_type == 5 {
                    p.rot = normal2angle(d);
                }
                p.speed = speed(r);
                p.velocity = ee::vscale(p.dirc, p.speed);
            }
            1 => {
                let d = if param.r_type == 5 {
                    let at = if param.p_sync { line_at } else { ee::vadd(line_at, g.pos) };
                    ee::normalize(ee::vsub(p.pos, at))
                } else if param.p_sync {
                    ee::normalize(p.pos)
                } else {
                    ee::normalize(ee::vsub(p.pos, g.pos))
                };
                fly(p, d);
                p.speed = speed(r >> 16);
                p.velocity = ee::vscale(p.dirc, p.speed);
            }
            2 => {
                let d = if param.p_sync {
                    ee::normalize(ee::vsub([0; 4], p.pos))
                } else {
                    ee::normalize(ee::vsub(g.pos, p.pos))
                };
                fly(p, d);
                p.speed = speed(r >> 16);
                p.velocity = ee::vscale(p.dirc, p.speed);
            }
            4 => {
                let a = ang((r & 0x7f00) - 16384);
                let z = ee::sinf(a);
                let c = ee::cosf(a);
                let b = ang((r >> 5) & 0xff00);
                let t = [ee::mul(c, ee::sinf(b)), ee::mul(ee::neg(c), ee::cosf(b)), z, 0];
                fly(p, ee::normalize(t));
                p.speed = speed(r);
                p.velocity = ee::vscale(p.dirc, p.speed);
            }
            _ => {}
        }
        r = step.host.rand() >> 3;
        if param.p_rot_rnd && p.style == 0 {
            p.rotate[3] = (r & 0xfff0) as u16;
        }
        r = step.host.rand() >> 3;
        let life = ee::from_int(i32::from(param.p_life));
        let cut = ee::div(ee::mul(ee::from_int(r % 101), ee::mul(life, param.p_life_rr)), HUNDRED);
        p.life_time = ee::to_int(ee::sub(life, cut)) as i16;
        p.layer = g.layer;
        p.str_flag = (g.str_flag as u8) & 15;
        Some(s)
    }
}
