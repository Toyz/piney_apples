//! The summons' levels 3 and 4: `ccSummonsElementGenerate` (gcmn 0x005006b0)
//! and what it makes - `ccSummonsSystemElement` (ctor 0x004ed180, Main
//! 0x004ed840), which holds the magic circle (`ccMagicCircleElement`, in the
//! element manager) and from count 30 runs the summoned element's own class
//! by element (fire, water, thunder, dark, soil, wind, none), and
//! `effEnergyGrow`'s rising sparks (`ccEnergyGrowElement`). The classes are
//! in docs/engine/effects.md ("SummonsSystem").

use piney_data::volume::Volume;

use crate::drawelm::{self, AnmObj, DrawElm, DrawKind, PI, TWO_PI};
use crate::ee::{self, F, ONE, V4, VF0};
use crate::eff::Eff;
use crate::effect::EffectCtrl;
use crate::element::{Animate, Base, Element, ElmPtr};
use crate::fall::{self, damage2_of, eff_skill_break_se};
use crate::files::ObjRef;
use crate::particle::{FfParam, GenParam, GenRef};
use crate::spell::{Spell, attr, bits, camera_shake, check_camera_shake_range, first, noise, strings};
use crate::{Cx, Event, debris, pfx, ring, space, thunder, vu};

const NEG_HALF_PI: F = 0xbfc9_0fdb;

/// The summoned elements' tables (gcmn), names read from the executable.
#[derive(Clone, Debug, PartialEq)]
pub struct SummonedTables {
    /// Explosions (0x005ed5f0), bubbles (0x005ed600), the thunder's rings
    /// (0x005ed620), rocks (0x005ed640), the tree's rings (0x005ed650), the
    /// stars' rings (0x005ed660), by level.
    pub fire: [i32; 4],
    pub water: [i32; 4],
    pub thunder_rings: [i32; 4],
    pub soil: [i32; 4],
    pub tree_rings: [i32; 4],
    pub goblin_rings: [i32; 4],
    /// The light balls' first bearings (0x005ed630).
    pub spline_deg: [F; 3],
    /// `ccMagicCircleElement`'s palettes (0x005ed670: c1-c6, then CLT_x031),
    /// `ccNeedleElement`'s clumps (0x005ed6a0), `ccEnergyGrowElement`'s
    /// palettes (0x005ed740, by energy 134-140).
    pub circle_clut: [String; 7],
    pub needles: [String; 4],
    pub energy_clut: [String; 7],
    /// The dark ball's stars' texture (0x005ed690).
    pub dark_star_tex: i16,
    /// `effElementGeneratorTbl` rows and their force fields
    /// (`effElementFFTbl`): the dark ball's gathering (0x005ed050, its field
    /// 0x005ed3d0) and stars (0x005ed088), a light ball's (0x005ed210,
    /// 0x005ed410), a star's tail (0x005ed130, 0x005ed450), the stars'
    /// trails (0x005ed168, 0x005ed470), the leaves (0x005ed0f8,
    /// 0x005ed430).
    pub conv_gen: GenParam,
    pub conv_ff: FfParam,
    pub dark_star_gen: GenParam,
    pub ball_gen: GenParam,
    pub ball_ff: FfParam,
    pub star_gen: GenParam,
    pub star_ff: FfParam,
    pub trail_gen: GenParam,
    pub trail_ff: FfParam,
    pub leaf_gen: GenParam,
    pub leaf_ff: FfParam,
}

impl SummonedTables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> SummonedTables {
        let t = piney_data::tables::effect::of(volume);
        let (g, f) = (t.element_generators(), t.element_ffs());
        SummonedTables {
            fire: first(t.summoned_fire()),
            water: first(t.summoned_water()),
            thunder_rings: first(t.summoned_thunder_rings()),
            soil: first(t.summoned_soil()),
            tree_rings: first(t.summoned_tree_rings()),
            goblin_rings: first(t.summoned_goblin_rings()),
            spline_deg: bits(t.summoned_spline_deg()),
            circle_clut: strings(t.summoned_circle_clut()),
            needles: strings(t.summoned_needles()),
            energy_clut: strings(t.summoned_energy_clut()),
            dark_star_tex: t.summoned_dark_star_tex() as i16,
            conv_gen: GenParam::other(g, t.element_generators_va(), 4),
            conv_ff: FfParam::row(f, t.element_ffs_va(), 12),
            dark_star_gen: GenParam::other(g, t.element_generators_va(), 5),
            ball_gen: GenParam::other(g, t.element_generators_va(), 12),
            ball_ff: FfParam::row(f, t.element_ffs_va(), 14),
            star_gen: GenParam::other(g, t.element_generators_va(), 8),
            star_ff: FfParam::row(f, t.element_ffs_va(), 16),
            trail_gen: GenParam::other(g, t.element_generators_va(), 9),
            trail_ff: FfParam::row(f, t.element_ffs_va(), 17),
            leaf_gen: GenParam::other(g, t.element_generators_va(), 7),
            leaf_ff: FfParam::row(f, t.element_ffs_va(), 15),
        }
    }
}

/// A table's entry for a level (1-4).
fn by_level(t: &[i32; 4], level: i32) -> i32 {
    t.get((level - 1) as usize).copied().unwrap_or(0)
}

/// A ring's heading: (0, ccRandF(pi), ccRandF(pi), 1).
fn random_turn(cx: &mut Cx) -> V4 {
    let mut r = VF0;
    r[1] = thunder::rand_f(cx, PI);
    r[2] = thunder::rand_f(cx, PI);
    r
}

/// `(x, 0, 0, 1)` turned about y by `ty`, then about z by `tz`.
fn out_by(x: F, ty: F, tz: F) -> V4 {
    let m = vu::rot_z(&vu::rot_y(&vu::UNIT, ty), tz);
    ee::apply(&m, [x, 0, 0, ONE])
}

/// The cubic Bernstein weights of `t` as the elements work them out.
fn bernstein(t: F) -> V4 {
    let f4 = ee::sub(ONE, t);
    let f3 = ee::mul(f4, f4);
    let f2 = ee::mul(t, t);
    [ee::mul(f3, f4), ee::mul(ee::mul(0x4040_0000, f3), t), ee::mul(ee::mul(0x4040_0000, f4), f2), ee::mul(t, f2)]
}

/// `SetScaleAnm(ss, es, time)` written out, no acceleration.
fn scale_anim(a: &mut Animate, ss: V4, es: V4, time: F) {
    a.start_scale = ss;
    a.end_scale = es;
    for k in 0..3 {
        a.scale_spd[k] = ee::div(ee::sub(es[k], ss[k]), time);
    }
    a.scale = ss;
    a.scale_flag = 1;
    a.scale_accel = VF0;
}

/// `SetFade(st, et)` written out with its step.
fn fade(a: &mut Animate, st: F, et: F, spd: F) {
    a.start_fade = st;
    a.end_fade = et;
    a.fade_spd = spd;
    a.transparency = st;
    a.fade_flag = 1;
}

/// An explosion in the manager grown from 1 to 3 over 15 frames.
fn explode_growing(cx: &mut Cx, pos: V4, scale: F, n: i32) {
    if let Some(k) = drawelm::eff_explode3(cx, pos, VF0, scale, n)
        && let Some(b) = cx.spells.elements.base_mut(k)
    {
        scale_anim(&mut b.anim, [ONE; 4], [0x4040_0000, 0x4040_0000, 0x4040_0000, ONE], 0x4170_0000);
    }
}

/// `checkCameraShakeRange(pos)`: noise 20 and `cameraShake(2, 2, 20, 2)`.
fn shake_noise(cx: &mut Cx, pos: V4) {
    if check_camera_shake_range(cx, pos) {
        noise(cx, 20);
        camera_shake(cx, 2, 2, 20, 2);
    }
}

/// A generator following an element's position: what its `syncPos`
/// gives it each particle frame (the port has the owner move it).
fn follow(cx: &mut Cx, g: Option<GenRef>, pos: V4) {
    if let Some(g) = g.and_then(|g| cx.particles.get_mut(g))
        && g.g_sync
    {
        g.pos = ee::vadd(g.offset, pos);
        g.pos[3] = ONE;
    }
}

/// A generator's pause bit.
fn pause(cx: &mut Cx, g: Option<GenRef>, on: bool) {
    if let Some(g) = g.and_then(|g| cx.particles.get_mut(g)) {
        g.pause = on;
    }
}

/// `ccElementShock(pos, attr)` (gcmn 0x004e8a20): a ring of the element
/// (fire 191, water 190, thunder 192, soil 196, wind 193, dark 194)
/// 100-400 out from `pos` (added in the world, then taken out of the
/// player's frame, as the game has it).
pub fn element_shock(cx: &mut Cx, pos: V4, atr: i32) {
    let ty = match atr & 0xfc {
        attr::FIRE => 191,
        attr::WATER => 190,
        attr::THUNDER => 192,
        attr::SOIL => 196,
        attr::WIND => 193,
        attr::DARK => 194,
        _ => return,
    };
    let dirc = random_turn(cx);
    let x = ee::add(0x42c8_0000, thunder::rand_f(cx, 0x4396_0000));
    let ty_ = ee::fabsf(thunder::rand_f(cx, PI));
    let tz = thunder::rand_f(cx, PI);
    let v = out_by(x, ty_, tz);
    let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
    let p = space::p2w(ee::vadd(pos, v), player, bounds);
    ring::eff_summon_ring_element(cx, p, dirc, ty);
}

/// `effThunderShock(pos, n)` (gcmn 0x005002a0): a ring 200 at `pos`, the
/// fall's lightning generator there (texture 11), and an explosion of kind
/// 2 growing from 1 to 5 over 15 frames.
pub fn eff_thunder_shock(cx: &mut Cx, pos: V4) {
    let mut r = ring::RingElement::new(cx, pos, VF0, [ONE; 4], 200);
    r.eny_flg = 1;
    cx.spells.elements.add(Element::Ring(Box::new(r)));
    let (param, ff) = (cx.spells.data.fall.thunder_gen, cx.spells.data.fall.thunder_ff);
    pfx::start_param(cx, param, [Some(ff[0]), Some(ff[1]), None, None], |g| {
        g.pos = pos;
        g.p_tex_mod = 11;
    });
    if let Some(k) = drawelm::eff_explode3(cx, pos, VF0, ONE, 2)
        && let Some(b) = cx.spells.elements.base_mut(k)
    {
        scale_anim(&mut b.anim, [ONE; 4], [0x40a0_0000, 0x40a0_0000, 0x40a0_0000, ONE], 0x4170_0000);
    }
}

/// `ccSummonsElementGenerate(skill, attr, level)` (gcmn 0x005006b0): the
/// system element in the first empty slot (none: deleted, None).
pub fn summons_element_generate(cx: &mut Cx, k: usize, atr: i32, level: i32) -> Option<usize> {
    let s = cx.spells.runs[k].clone();
    let e = SummonsSystem::new(cx, &s, atr, level);
    cx.spells.elements.add(Element::Summons(Box::new(e)))
}

/// One of `ccEnergyGrowElement`'s `ENERGY_T`s (64 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Energy {
    /// +0x00 `transparency`, +0x10 `pos`, +0x20 `status` (0 free, 1 in, 2
    /// held, 3 out), +0x22 `anmIndex` (never set), +0x24 `velocity`, +0x28
    /// `fadeIn`, +0x2c `fadeOut`, +0x30 `noFade`.
    pub transparency: F,
    pub pos: V4,
    pub status: i16,
    pub anm_index: i16,
    pub velocity: F,
    pub fade_in: F,
    pub fade_out: F,
    pub no_fade: F,
}

/// `ccEnergyGrowElement` (gcmn effect2.cpp, 0x250 bytes): sparks rising
/// round a point, `m_genV` more a frame while it lives.
#[derive(Clone, Debug, PartialEq)]
pub struct EnergyGrow {
    pub base: Base,
    /// +0x190 `m_energy` (EFF_x005, its palette by energy), +0x200
    /// `m_energyTbl` / +0x204 `m_tblNum`, +0x208 `m_genV`, +0x20c `m_genA`,
    /// +0x210 `m_genRnd`, +0x214 `m_movV`, `m_movA`, `m_movMax`,
    /// `m_movRnd`, +0x224 `m_radius`, `m_radV`, `m_radMax`, `m_radRnd`,
    /// +0x234 `m_fadeIn`, `m_noFade`, `m_fadeOut`, `m_fadeRnd`.
    pub energy: Option<Box<Eff>>,
    pub tbl: Vec<Energy>,
    pub gen_v: i32,
    pub gen_a: i32,
    pub gen_rnd: F,
    pub mov: [F; 4],
    pub rad: [F; 4],
    pub fades: [F; 4],
}

impl EnergyGrow {
    /// `new ccEnergyGrowElement(gen, mov, rad, fade, n)` (gcmn 0x004fde60).
    fn new(cx: &Cx, gen_: V4, mov: V4, rad: V4, fades: V4, n: i32) -> EnergyGrow {
        let num = ee::to_int(gen_[2]).max(0) as usize;
        let mut energy = drawelm::particle_eff(cx, "EFF_x005");
        if let Some(e) = energy.as_mut() {
            if (134..141).contains(&n) {
                let name = &cx.spells.data.summoned.energy_clut[(n - 134) as usize];
                e.clut = cx.assets.find("particle", name);
            }
            e.scale_y = 0x40a0_0000;
            e.scale_x = 0x40a0_0000;
        }
        let blank = Energy {
            transparency: 0,
            pos: [0; 4],
            status: 0,
            anm_index: 0,
            velocity: 0,
            fade_in: 0,
            fade_out: 0,
            no_fade: 0,
        };
        EnergyGrow {
            base: Base::default(),
            energy,
            tbl: vec![blank; num],
            gen_v: ee::to_int(gen_[0]),
            gen_a: ee::to_int(gen_[1]),
            gen_rnd: gen_[3],
            mov,
            rad,
            fades,
        }
    }

    /// `ccEnergyGrowElement::Main` (gcmn 0x00502240): its `Draw`, which
    /// keeps its own life.
    pub fn main(&mut self, cx: &mut Cx) {
        let (mov_a, mov_max, mov_v) = (self.mov[1], self.mov[2], self.mov[0]);
        let mut idle = true;
        for i in 0..self.tbl.len() {
            if self.tbl[i].status == 0 {
                continue;
            }
            idle = false;
            if let Some(e) = self.energy.as_mut() {
                e.transparency = self.tbl[i].transparency;
                let (pos, pat) = (self.tbl[i].pos, self.tbl[i].anm_index as u16);
                drawelm::draw_eff_at(cx, e, pos, pat);
            }
            let p = &mut self.tbl[i];
            p.pos[2] = ee::add(p.pos[2], p.velocity);
            let mut v = ee::add(p.velocity, mov_a);
            if ee::lt(mov_v, mov_max) && !ee::le(v, mov_max) {
                v = mov_max;
            }
            p.velocity = v;
            match p.status {
                1 => {
                    let mut t = ee::add(p.transparency, ee::div(ONE, p.fade_in));
                    if !ee::le(t, ONE) {
                        t = ONE;
                        p.status += 1;
                    }
                    p.transparency = t;
                }
                2 => {
                    let mut t = ee::sub(p.no_fade, ONE);
                    if ee::lt(t, 0) {
                        t = 0;
                        p.status += 1;
                    }
                    p.no_fade = t;
                }
                3 => {
                    let mut t = ee::sub(p.transparency, ee::div(ONE, p.fade_out));
                    if ee::lt(t, 0) {
                        t = 0;
                        p.status = 0;
                    }
                    p.transparency = t;
                }
                _ => {}
            }
        }
        if self.base.life != 0 {
            self.entry(cx, self.gen_v);
        }
        if self.gen_a != 0 {
            self.gen_v = self.gen_v.wrapping_add(self.gen_a);
        }
        // m_genRnd scales m_genV at random (ccRandF(1, rnd)); its one
        // caller gives 0.
        if self.gen_v < 0 {
            self.gen_v = self.gen_v.wrapping_neg();
        }
        if ee::lt(self.rad[0], self.rad[2]) {
            self.rad[0] = ee::add(self.rad[0], self.rad[1]);
        }
        let life = self.base.life;
        if life > 0 {
            self.base.life = life - 1;
        } else if life == 0 && idle {
            self.base.del_flag = 1;
        }
    }

    /// 1 when `rnd` is 0, |ccRandF(1)| when 1, else rnd + |ccRandF(1 -
    /// rnd)| in double precision.
    fn spread(cx: &mut Cx, rnd: F) -> F {
        if ee::eq(rnd, 0) {
            ONE
        } else if ee::eq(ONE, rnd) {
            ee::fabsf(thunder::rand_f(cx, ONE))
        } else {
            thunder::add_abs(f64::from(f32::from_bits(rnd)), thunder::rand_f(cx, ee::sub(ONE, rnd)))
        }
    }

    /// `_Entry(n)` (gcmn 0x004fe570): n more sparks in the free entries
    /// from the first, each at a random bearing within the radius, rising
    /// at a random share of its speed, its fades stretched at random.
    fn entry(&mut self, cx: &mut Cx, n: i32) {
        let radius = self.rad[0];
        let (mov_v, mov_rnd) = (self.mov[0], self.mov[3]);
        let rad_rnd = self.rad[3];
        let fin = ee::to_int(self.fades[0]);
        let fout = ee::to_int(self.fades[2]);
        let hold = ee::to_int(self.fades[1]);
        let fade_rnd = self.fades[3];
        let pos = self.base.anim.pos;
        let mut i = 0;
        for _ in 0..n.max(0) {
            while i < self.tbl.len() && self.tbl[i].status != 0 {
                i += 1;
            }
            if i >= self.tbl.len() {
                continue;
            }
            let r = thunder::add_abs(f64::from(f32::from_bits(rad_rnd)), thunder::rand_f(cx, ee::sub(ONE, rad_rnd)));
            let x = ee::mul(radius, r);
            let a = thunder::rand_f(cx, PI);
            let v = ee::apply(&vu::rot_z(&vu::UNIT, a), [x, 0, 0, ONE]);
            let sp = Self::spread(cx, mov_rnd);
            let tm = Self::spread(cx, fade_rnd);
            let p = &mut self.tbl[i];
            p.pos = ee::vadd(pos, v);
            p.velocity = ee::mul(mov_v, sp);
            p.fade_in = ee::mul(ee::from_int(fin), tm);
            p.no_fade = ee::mul(ee::from_int(hold), tm);
            p.fade_out = ee::mul(ee::from_int(fout), tm);
            p.status = 1;
            p.transparency = ONE;
            i += 1;
        }
    }
}

/// `effEnergyGrow(pos, gen, mov, rad, fade, life, n)` (gcmn 0x004fff50).
#[allow(clippy::too_many_arguments)]
pub fn eff_energy_grow(
    cx: &mut Cx,
    pos: V4,
    gen_: V4,
    mov: V4,
    rad: V4,
    fades: V4,
    life: i32,
    n: i32,
) -> Option<usize> {
    let mut e = EnergyGrow::new(cx, gen_, mov, rad, fades, n);
    e.base.anim.pos = pos;
    e.base.life = life;
    cx.spells.elements.add(Element::EnergyGrow(Box::new(e)))
}

/// `ccSummonsSystemElement` (gcmn effect2.cpp, 0x5b0 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct SummonsSystem {
    pub base: Base,
    /// +0x5a0 `m_effCircle` (in the manager), +0x5a4 `m_effSummon` (its
    /// own), +0x5a8 `m_skillPtr`.
    pub circle: Option<ElmPtr>,
    pub summon: Box<Summoned>,
    pub skill: u32,
}

impl SummonsSystem {
    /// `new ccSummonsSystemElement(skill, attr, level)` (gcmn 0x004ed180).
    fn new(cx: &mut Cx, s: &Spell, atr: i32, level: i32) -> SummonsSystem {
        let mut base = Base { target: s.target, attr: atr & 0xfc, level, ..Base::default() };
        base.anim.pos = s.t_pos;
        let (summon, n) = match base.attr {
            attr::FIRE => (Summoned::fire(cx, s, level), 135),
            attr::WATER => (Summoned::water(cx, s, level), 134),
            attr::THUNDER => (Summoned::thunder(cx, s, level), 136),
            attr::WIND => (Summoned::tree(cx, s, level), 137),
            attr::DARK => (Summoned::dark(cx, s, level), 138),
            attr::SOIL => (Summoned::soil(cx, s, level), 140),
            _ => (Summoned::goblin(cx, s, level), 139),
        };
        let mut p = s.t_pos;
        p[2] = ee::add(p[2], 0x4120_0000);
        let mut c = DrawElm::magic_circle(cx, atr);
        c.base.anim.pos = p;
        c.base.anim.transparency = 0;
        c.base.anim.scale[0] = 0;
        c.base.anim.scale[1] = 0;
        c.base.anim.scale[2] = 0;
        if let DrawKind::MagicCircle { scale_spd, scale_limit, fade_spd, fade_limit, .. } = &mut c.kind {
            *scale_spd = 0x3eaa_aaab;
            *scale_limit = 0x40a0_0000;
            *fade_limit = ONE;
            *fade_spd = 0x3d88_8889;
        }
        let circle = cx.spells.elements.add_ptr(Element::Draw(Box::new(c)));
        let lv = ee::from_int(level * 80);
        let pos = base.anim.pos;
        eff_energy_grow(
            cx,
            pos,
            [0x4040_0000, ONE, lv, 0],
            [0x4120_0000, 0x3f00_0000, 0x42c8_0000, ONE],
            [0x42c8_0000, 0x41a0_0000, 0x43fa_0000, 0x3f00_0000],
            [0x4170_0000, 0x41f0_0000, 0x4170_0000, ONE],
            60,
            n,
        );
        cx.raise(Event::Sound3dNote { se: 56, pos, note: 72 });
        SummonsSystem { base, circle, summon: Box::new(summon), skill: s.key }
    }

    /// `ccSummonsSystemElement::Main` (gcmn 0x004ed840).
    pub fn main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        if let Some(p) = self.circle {
            let mut done = false;
            let mut drop_it = false;
            let count = &mut self.base.count;
            if let Some(Element::Draw(c)) = cx.spells.elements.get_ptr_mut(p) {
                match c.base.status[0] {
                    3 => done = true,
                    1 => {
                        let n = *count;
                        *count = n + 1;
                        if n == 60 {
                            let t = c.base.anim.transparency;
                            if let DrawKind::MagicCircle { fade_spd, fade_limit, .. } = &mut c.kind {
                                *fade_limit = 0;
                                let d = ee::sub(t, 0);
                                *fade_spd = if ee::le(d, 0) { 0 } else { ee::div(d, 0xc1f0_0000) };
                            }
                            c.base.status[0] += 1;
                        }
                    }
                    _ => {}
                }
                if done {
                    c.base.del_flag = 1;
                    drop_it = true;
                }
            }
            if drop_it {
                self.circle = None;
                self.base.proccess += 1;
            }
        }
        if self.base.count < 30 {
            return;
        }
        if self.summon.base().del_flag == 0 {
            self.summon.main(ctrl, cx);
        } else if self.circle.is_none() {
            self.base.del_flag = 1;
        }
    }
}

/// One of `ccFireSummonElement`'s `ELEMENT_T`s (8 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct FireSlot {
    pub elm: DrawElm,
    pub interval: i32,
}

/// A `ccWaterSummonsElement` `ELEMENT_T` (0x1b0 bytes), a
/// `ccSoilSummonsElement` one (+0x1a0 `status`, +0x1a4 `interval`), a
/// `ccTreeSummonsElement` `NEEDLE_T` (+0x1a0 `interval`, +0x1a4 `status`,
/// +0x1b0 `cp[2]`, +0x1d0 `splineTime`): an embedded drawn element and its
/// counters.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldSlot {
    pub elm: DrawElm,
    pub status: i32,
    pub interval: i32,
    pub cp: [V4; 2],
    pub t: F,
}

/// A `ccThunderSummonsElement` `THUNDER_T` (0x440 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct ThunderSlot {
    pub elm: DrawElm,
    pub status: i32,
}

/// A `ccThunderSummonsElement` `S_THUNDER_T`: a small bolt in the manager.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmallThunder {
    pub thunder: Option<ElmPtr>,
    pub status: i32,
}

/// A `ccDarkSummonsElement` `ELEMENT_T` (0x200 bytes): a bat on a spline
/// (+0x1a0 `status`, +0x1b0 `spP`, +0x1c0 `epP`, +0x1d0 `cpP[2]`, +0x1f0
/// `splineTime`, +0x1f4 `timeMode`), or a `BALL_T` (0x1e0 bytes: +0x1a0
/// `spP`, +0x1b0 `epP`, +0x1c0 `cpP`, +0x1d0 `status`, +0x1d4
/// `splineTime`, +0x1d8 `addTime`, +0x1dc `timeMode`).
#[derive(Clone, Debug, PartialEq)]
pub struct SplineSlot {
    pub elm: DrawElm,
    pub status: i32,
    pub sp: V4,
    pub ep: V4,
    pub cp: [V4; 2],
    pub t: F,
    pub add_time: F,
    pub time_mode: i32,
}

/// A `ccGoblinSummonsElement` `ELEMENT_T` (0x2b0 bytes): +0x00 `gp`, +0x10
/// `star`, +0x220 `status`, +0x230 `pos`, +0x240 `splineTime`, +0x244
/// `splineSpd`, +0x250 `cp[2]`, +0x270 `cpP[2]`, +0x290 `ep`, +0x2a0 `epP`.
#[derive(Clone, Debug, PartialEq)]
pub struct StarTrail {
    pub gp: Option<GenRef>,
    pub star: DrawElm,
    pub status: i32,
    pub pos: V4,
    pub t: F,
    pub spd: F,
    pub cp: [V4; 2],
    pub cp_p: [V4; 2],
    pub ep: V4,
    pub ep_p: V4,
}

/// The summoned element's own class.
#[derive(Clone, Debug, PartialEq)]
pub enum SummonKind {
    /// +0x5a0 `m_elements`, +0x5b0 `m_tPos`.
    Fire { elements: Vec<FireSlot> },
    /// +0x5a0 `m_elements` (level 4's bubbles), +0x5a4 `m_gp` (never set),
    /// +0x5a8 `m_iceFall` (level 3's).
    Water { elements: Option<Vec<HeldSlot>>, ice_fall: Option<Box<fall::FallElement>> },
    /// +0x5a0 `m_thunderTbl[6]`, +0x1f20 `m_smallThunderTbl[16]`, +0x1fa0
    /// `m_basePos`, +0x1fb0 `m_shockSEOne`.
    Thunder { bolts: Vec<ThunderSlot>, small: Box<[SmallThunder; 16]>, base_pos: V4, shock_se_one: i32 },
    /// +0x5a0 `m_elements` (level 3's bats), +0x5a4 `m_batGenerateSE`,
    /// +0x5a8 `m_batShockSE`, +0x5ac `m_smokeGenerateSEOne`, +0x5b0
    /// `m_ballTbl` (level 4's), +0x5b4 `m_darkBall`.
    Dark {
        bats: Option<Vec<SplineSlot>>,
        bat_generate_se: i32,
        bat_shock_se: i32,
        smoke_generate_se_one: i32,
        balls: Option<Vec<SplineSlot>>,
        dark_ball: Box<DrawElm>,
    },
    /// +0x5a0 `m_elements`.
    Soil { elements: Vec<HeldSlot> },
    /// +0x5a0 `m_needleTbl` (level 4's), +0x5b0 `m_rootsTbl[8]`, +0x630
    /// `m_gpLeaf`, +0x634 `m_cmpTree`, +0x638 `m_anmRoots`, +0x63c
    /// `m_treeAlpha` (never set).
    Tree {
        needles: Option<Vec<HeldSlot>>,
        roots: [V4; 8],
        gp_leaf: Option<GenRef>,
        cmp_tree: Option<ObjRef>,
        anm_roots: Option<AnmObj>,
        tree_alpha: F,
    },
    /// +0x5a0 `m_genPos`, +0x5b0 `m_genPosP`, +0x5c0 `m_elements` (level
    /// 4's), +0x5c4 `m_starTbl` (level 3's).
    Goblin { gen_pos: V4, gen_pos_p: V4, elements: Option<Vec<StarTrail>>, stars: Option<Vec<HeldSlot>> },
}

/// A summoned element (`ccMoveElement`, its class's own after): the
/// shared part and `m_elmNum`, `m_tPos`.
#[derive(Clone, Debug, PartialEq)]
pub struct Summoned {
    pub base: Base,
    pub elm_num: i32,
    pub t_pos: V4,
    pub kind: SummonKind,
}

impl Summoned {
    fn base(&self) -> &Base {
        &self.base
    }

    fn base_of(s: &Spell, level: i32) -> Base {
        Base { skill: Some(s.key), target: s.target, level, ..Base::default() }
    }

    /// The class the checks know it by.
    pub fn class(&self) -> &'static str {
        match self.kind {
            SummonKind::Fire { .. } => "ccFireSummonElement",
            SummonKind::Water { .. } => "ccWaterSummonsElement",
            SummonKind::Thunder { .. } => "ccThunderSummonsElement",
            SummonKind::Dark { .. } => "ccDarkSummonsElement",
            SummonKind::Soil { .. } => "ccSoilSummonsElement",
            SummonKind::Tree { .. } => "ccTreeSummonsElement",
            SummonKind::Goblin { .. } => "ccGoblinSummonsElement",
        }
    }

    /// Its `Main`.
    fn main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        match self.kind {
            SummonKind::Fire { .. } => self.fire_main(cx),
            SummonKind::Water { .. } => self.water_main(ctrl, cx),
            SummonKind::Thunder { .. } => self.thunder_main(cx),
            SummonKind::Dark { .. } => self.dark_main(cx),
            SummonKind::Soil { .. } => self.soil_main(ctrl, cx),
            SummonKind::Tree { .. } => self.tree_main(cx),
            SummonKind::Goblin { .. } => self.goblin_main(ctrl, cx),
        }
    }

    /// The spell's `tPos` while it is on `SkillEntryTop`.
    fn track(&mut self, cx: &Cx) {
        if let Some(k) = self.base.skill
            && cx.spells.entry_check(k)
            && let Some(s) = cx.spells.get(k)
        {
            self.t_pos = s.t_pos;
        }
    }

    fn damage(&self, cx: &mut Cx) {
        if let Some(k) = self.base.skill {
            damage2_of(cx, k);
        }
    }

    /// `(W2P(tPos) + (0, 0, up))` and it back in the world.
    fn above(cx: &Cx, tp: V4, up: F) -> (V4, V4) {
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let mut p = space::w2p(tp, player, bounds);
        p[2] = ee::add(p[2], up);
        (p, space::p2w(p, player, bounds))
    }

    // fire ------------------------------------------------------------------

    /// `new ccFireSummonElement(skill, level)` (gcmn 0x004f1570).
    fn fire(cx: &mut Cx, s: &Spell, level: i32) -> Summoned {
        let n = by_level(&cx.spells.data.summoned.fire, level);
        let mut base = Self::base_of(s, level);
        base.anim.pos = s.t_pos;
        let (top, _) = Self::above(cx, s.t_pos, 0x438c_0000);
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let mut elements = Vec::new();
        for i in 0..n.max(0) {
            let mut elm = DrawElm::explode(cx, attr::FIRE);
            let p = if level == 1 {
                s.t_pos
            } else {
                let x = thunder::add_abs(100.0, thunder::rand_f(cx, 0x43fa_0000));
                let ty = thunder::rand_f(cx, PI);
                let tz = thunder::rand_f(cx, PI);
                space::p2w(ee::vadd(top, out_by(x, ty, tz)), player, bounds)
            };
            elm.base.anim.pos = p;
            elm.base.anim.sp = p;
            let dirc = random_turn(cx);
            ring::eff_summon_ring_element(cx, p, dirc, 162);
            elements.push(FireSlot { elm, interval: i * 5 / 2 + 1 });
        }
        Summoned { base, elm_num: n, t_pos: s.t_pos, kind: SummonKind::Fire { elements } }
    }

    /// `ccFireSummonElement::Main` (gcmn 0x004f1b00).
    fn fire_main(&mut self, cx: &mut Cx) {
        let tp = self.t_pos;
        let key = self.base.skill;
        let SummonKind::Fire { elements } = &mut self.kind else { return };
        let mut all = true;
        for (i, slot) in elements.iter_mut().enumerate() {
            if slot.elm.base.del_flag != 0 {
                continue;
            }
            slot.interval -= 1;
            if slot.interval <= 0 {
                if slot.interval == 0 {
                    let pos = slot.elm.base.anim.pos;
                    element_shock(cx, pos, attr::FIRE);
                    cx.raise(Event::Sound3d { se: 35, pos });
                    shake_noise(cx, tp);
                    if i == 0
                        && let Some(k) = key
                    {
                        damage2_of(cx, k);
                    }
                }
                slot.elm.main(cx);
            }
            all = false;
        }
        if all {
            self.base.del_flag = 1;
        }
    }

    // water -----------------------------------------------------------------

    /// `new ccWaterSummonsElement(skill, level)` (gcmn 0x004f1c70).
    fn water(cx: &mut Cx, s: &Spell, level: i32) -> Summoned {
        let n = by_level(&cx.spells.data.summoned.water, level);
        let mut base = Self::base_of(s, level);
        let (top, p) = Self::above(cx, s.t_pos, 0x438c_0000);
        base.anim.pos = p;
        base.anim.sp = p;
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let kind = if level == 3 {
            let f = fall::FallElement::new(cx, s, attr::WATER, 4);
            for _ in 0..n.max(0) {
                let dirc = random_turn(cx);
                ring::eff_summon_ring_element(cx, p, dirc, 161);
            }
            SummonKind::Water { elements: None, ice_fall: Some(Box::new(f)) }
        } else {
            let mut els: Vec<HeldSlot> = (0..n.max(0))
                .map(|_| HeldSlot { elm: DrawElm::bubble(cx), status: 0, interval: 0, cp: [[0; 4]; 2], t: 0 })
                .collect();
            for (i, slot) in els.iter_mut().enumerate() {
                let x = ee::add(0x42c8_0000, thunder::rand_f(cx, 0x43fa_0000));
                let ty = thunder::rand_f(cx, PI);
                let tz = thunder::rand_f(cx, PI);
                let q = space::p2w(ee::vadd(out_by(x, ty, tz), top), player, bounds);
                let a = &mut slot.elm.base.anim;
                a.pos = q;
                a.sp = q;
                a.scale[0] = 0x4120_0000;
                a.scale[1] = 0x4120_0000;
                a.scale[2] = 0x4120_0000;
                slot.interval = i as i32 * 5 / 2 + 1;
                let dirc = random_turn(cx);
                ring::eff_summon_ring_element(cx, p, dirc, 161);
            }
            SummonKind::Water { elements: Some(els), ice_fall: None }
        };
        Summoned { base, elm_num: n, t_pos: s.t_pos, kind }
    }

    /// `ccWaterSummonsElement::Main` (gcmn 0x004f2320): level 3 the ice
    /// fall, then (it deleted) the hit and the end; level 4 the bubbles.
    fn water_main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        self.track(cx);
        let tp = self.t_pos;
        let key = self.base.skill;
        let level = self.base.level;
        let SummonKind::Water { elements, ice_fall } = &mut self.kind else { return };
        if level == 3 {
            if let Some(f) = ice_fall.as_mut() {
                f.main(ctrl, cx);
                if f.base.del_flag != 0 {
                    if let Some(k) = key {
                        damage2_of(cx, k);
                    }
                    self.base.del_flag = 1;
                }
            }
            return;
        }
        let Some(els) = elements.as_mut() else { return };
        let mut all = true;
        for (i, slot) in els.iter_mut().enumerate() {
            if slot.elm.base.del_flag != 0 {
                continue;
            }
            slot.interval -= 1;
            if slot.interval <= 0 {
                if slot.interval == 0 {
                    let mut r = VF0;
                    r[0] = ee::deg2rad(-16384);
                    r[2] = thunder::rand_f(cx, PI);
                    let pos = slot.elm.base.anim.pos;
                    debris::eff_radiate_something2(ctrl, cx, pos, r, 0x4234_0000, attr::WATER, 10);
                    ring::eff_summon_ring_element(cx, pos, r, 161);
                    if i & 3 == 0 {
                        cx.raise(Event::Sound3d { se: 35, pos: tp });
                        cx.raise(Event::Sound3dNote { se: 66, pos: tp, note: 52 });
                    }
                    if i & 1 != 0 {
                        element_shock(cx, pos, attr::WATER);
                    }
                    if i == 0
                        && let Some(k) = key
                    {
                        damage2_of(cx, k);
                    }
                }
                slot.elm.main(cx);
            }
            all = false;
        }
        if all {
            self.base.del_flag = 1;
        }
    }

    // thunder ---------------------------------------------------------------

    /// `new ccThunderSummonsElement(skill, level)` (gcmn 0x004f25f0) and
    /// its `_Init` (0x004f2d90).
    fn thunder(cx: &mut Cx, s: &Spell, level: i32) -> Summoned {
        let mut base = Self::base_of(s, level);
        let (_, p) = Self::above(cx, s.t_pos, 0x438c_0000);
        base.anim.pos = p;
        base.anim.sp = p;
        let bolts = (0..6).map(|_| ThunderSlot { elm: DrawElm::thunder(cx), status: 0 }).collect();
        let small = Box::new([SmallThunder { thunder: None, status: 0 }; 16]);
        let mut e = Summoned {
            base,
            elm_num: 0,
            t_pos: s.t_pos,
            kind: SummonKind::Thunder { bolts, small, base_pos: p, shock_se_one: 0 },
        };
        e.thunder_init(cx);
        e
    }

    /// `(|(int)ccRandF(100)|, 0, 0, 1)` turned about z at random.
    fn jitter(cx: &mut Cx, x: F) -> V4 {
        let r = ee::from_int(thunder::fptosi(thunder::rand_f(cx, x)).wrapping_abs());
        let a = thunder::rand_f(cx, PI);
        ee::apply(&vu::rot_z(&vu::UNIT, a), [r, 0, 0, ONE])
    }

    fn thunder_init(&mut self, cx: &mut Cx) {
        let tp = self.t_pos;
        let level = self.base.level;
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let SummonKind::Thunder { bolts, small, base_pos, .. } = &mut self.kind else { return };
        **small = [SmallThunder { thunder: None, status: 0 }; 16];
        let bp = *base_pos;
        for b in bolts.iter_mut() {
            let tpp = space::w2p(tp, player, bounds);
            let mut top = space::w2p(bp, player, bounds);
            top[2] = ee::add(top[2], 0x44fa_0000);
            let from = space::p2w(ee::vadd(top, Self::jitter(cx, 0x42c8_0000)), player, bounds);
            let to = space::p2w(ee::vadd(tpp, Self::jitter(cx, 0x42c8_0000)), player, bounds);
            let a = &mut b.elm.base.anim;
            a.pos = from;
            a.sp = from;
            a.ep = to;
            b.status = 0;
        }
        for _ in 0..by_level(&cx.spells.data.summoned.thunder_rings, level).max(0) {
            let dirc = random_turn(cx);
            ring::eff_summon_ring_element(cx, bp, dirc, 163);
        }
        let a = &mut bolts[0].elm.base.anim;
        a.scale[0] = 0x40a0_0000;
        a.scale[1] = 0x40a0_0000;
        a.scale[2] = 0x40a0_0000;
    }

    /// `_EntryThunder(n)` (gcmn 0x004f2a00): n small bolts (in the
    /// manager, one link from done) from 2000 above to within 500 of the
    /// target, while the table has room; then the break's sounds.
    fn entry_thunder(&mut self, cx: &mut Cx, n: i32) {
        let tp = self.t_pos;
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let SummonKind::Thunder { small, base_pos, .. } = &mut self.kind else { return };
        let bp = *base_pos;
        for _ in 0..n {
            let Some(k) = small.iter().position(|s| s.thunder.is_none()) else { break };
            let mut e = DrawElm::thunder(cx);
            let t = space::w2p(tp, player, bounds);
            let mut top = space::w2p(bp, player, bounds);
            top[2] = ee::add(top[2], 0x44fa_0000);
            let t = ee::vadd(t, Self::jitter(cx, 0x43fa_0000));
            let to = space::p2w(t, player, bounds);
            let mut q = t;
            q[2] = ee::add(q[2], 0x44fa_0000);
            let from = space::p2w(q, player, bounds);
            let _ = top;
            let a = &mut e.base.anim;
            a.pos = from;
            a.sp = from;
            a.ep = to;
            if let DrawKind::Thunder { anm_index, .. } = &mut e.kind {
                *anm_index = 18;
            }
            let Some(p) = cx.spells.elements.add_ptr(Element::Draw(Box::new(e))) else { break };
            small[k] = SmallThunder { thunder: Some(p), status: 0 };
        }
        eff_skill_break_se(cx, tp, attr::THUNDER);
        cx.raise(Event::Sound3d { se: 41, pos: tp });
    }

    /// The bolts' pass: each one's `Main`, and as each strikes the hit's
    /// sounds, shake and (the sixth) a ring; true when all are done.
    fn thunder_bolts(&mut self, cx: &mut Cx, four: bool) -> bool {
        let tp = self.t_pos;
        let key = self.base.skill;
        let SummonKind::Thunder { bolts, shock_se_one, .. } = &mut self.kind else { return true };
        let mut all = true;
        for (i, b) in bolts.iter_mut().enumerate() {
            if b.elm.base.del_flag != 0 {
                continue;
            }
            b.elm.main(cx);
            all = false;
            if b.status != 0 || b.elm.base.status[0] == 0 {
                continue;
            }
            b.status += 1;
            let ep = b.elm.base.anim.ep;
            if i == 5 {
                eff_thunder_shock(cx, ep);
                if four && let Some(k) = key {
                    damage2_of(cx, k);
                }
            }
            if check_camera_shake_range(cx, ep) {
                if *shock_se_one == 0 {
                    *shock_se_one = 1;
                    cx.raise(Event::Sound3dNote { se: 56, pos: tp, note: 72 });
                    cx.raise(Event::Sound3d { se: 35, pos: tp });
                    eff_skill_break_se(cx, tp, attr::THUNDER);
                    cx.raise(Event::Sound3d { se: 41, pos: tp });
                }
                camera_shake(cx, 2, 2, 30, 2);
            }
            if !four
                && i == 0
                && let Some(k) = key
            {
                damage2_of(cx, k);
            }
        }
        all
    }

    /// `ccThunderSummonsElement::Main` (gcmn 0x004f30a0).
    fn thunder_main(&mut self, cx: &mut Cx) {
        self.track(cx);
        match self.base.level {
            3 => {
                if self.thunder_bolts(cx, false) {
                    self.base.del_flag = 1;
                }
            }
            4 => {
                if self.base.proccess != 0 {
                    return;
                }
                let c = self.base.count;
                self.base.count = c + 1;
                if c >= 60 {
                    if self.thunder_bolts(cx, true) {
                        self.base.del_flag = 1;
                    }
                    return;
                }
                if c & 7 == 0 {
                    self.entry_thunder(cx, 2);
                }
                let dirc = self.base.anim.dirc;
                let SummonKind::Thunder { small, .. } = &mut self.kind else { return };
                for s in small.iter_mut().take(6) {
                    let Some(p) = s.thunder else { continue };
                    let (struck, ep, gone) = match cx.spells.elements.get_ptr(p) {
                        Some(Element::Draw(e)) => (e.base.status[0] != 0, e.base.anim.ep, e.base.del_flag != 0),
                        Some(Element::Busy) => (false, VF0, false),
                        // Deleted: the game reads its m_delFlag from the
                        // freed memory, still set.
                        _ => (false, VF0, true),
                    };
                    if s.status == 0 && struck {
                        drawelm::eff_explode3(cx, ep, VF0, 0x4000_0000, 2);
                        ring::eff_summon_ring_element(cx, ep, dirc, 195);
                        s.status += 1;
                    } else if gone {
                        s.thunder = None;
                    }
                }
            }
            _ => {}
        }
    }

    // dark ------------------------------------------------------------------

    /// `new ccDarkSummonsElement(skill, level)` (gcmn 0x004f3510).
    fn dark(cx: &mut Cx, s: &Spell, level: i32) -> Summoned {
        let mut base = Self::base_of(s, level);
        let (top, p) = Self::above(cx, s.t_pos, 0x4411_0000);
        base.anim.pos = p;
        base.anim.sp = p;
        base.anim.ep = s.t_pos;
        let mut ball = DrawElm::dark_ball(cx);
        ball.base.anim.scale[0] = 0x4248_0000;
        ball.base.anim.scale[1] = 0x4248_0000;
        ball.base.anim.scale[2] = 0x4248_0000;
        ball.base.anim.pos = p;
        let blank = |elm: DrawElm| SplineSlot {
            elm,
            status: 0,
            sp: [0; 4],
            ep: [0; 4],
            cp: [[0; 4]; 2],
            t: 0,
            add_time: 0,
            time_mode: 0,
        };
        let mut bats = None;
        let mut balls = None;
        if level == 3 {
            let mut v: Vec<SplineSlot> = (0..32).map(|_| blank(DrawElm::dark_bat(cx))).collect();
            for b in v.iter_mut() {
                for k in 0..3 {
                    b.elm.base.anim.scale[k] = 0x4000_0000;
                }
                b.status = 0;
            }
            bats = Some(v);
        } else if level == 4 {
            let mut v: Vec<SplineSlot> = (0..4).map(|_| blank(DrawElm::light_ball())).collect();
            for i in 0..4 {
                v[i].elm.light_ball_init(cx, p, 8);
                if let DrawKind::LightBall { gp_ball, .. } = v[i].elm.kind {
                    pause(cx, gp_ball, true);
                }
                v[i].status = 0;
                v[i].t = 0;
                v[i].add_time = ee::add(0x3d23_d70a, thunder::rand_f(cx, 0x3c23_d70a));
                v[i].time_mode = 0;
                // The game writes the first ball's spP and epP each time.
                v[0].sp = top;
                v[0].ep = top;
            }
            balls = Some(v);
        }
        for _ in 0..16 {
            let dirc = random_turn(cx);
            ring::eff_summon_ring_element(cx, p, dirc, 165);
        }
        let kind = SummonKind::Dark {
            bats,
            bat_generate_se: 0,
            bat_shock_se: 0,
            smoke_generate_se_one: 0,
            balls,
            dark_ball: Box::new(ball),
        };
        Summoned { base, elm_num: 0, t_pos: s.t_pos, kind }
    }

    /// `CalcSplinePoint(ball)` (gcmn 0x004f3d80): the ball's next leg - the
    /// first 300 out from the target at one of three bearings, the others
    /// 300-500 out from the summons' point at random.
    fn calc_spline_point(cx: &mut Cx, b: &mut SplineSlot, pos: V4, tp: V4) {
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        b.cp[0] = space::p2w(b.cp[0], player, bounds);
        b.sp = space::w2p(b.cp[0], player, bounds);
        let k = ((cx.host.genrand() as i32).wrapping_abs() % 3) as usize;
        if b.time_mode == 0 {
            b.cp[0] = space::w2p(tp, player, bounds);
            let a = cx.spells.data.summoned.spline_deg.get(k).copied().unwrap_or(0);
            b.cp[0][0] = ee::add(b.cp[0][0], ee::mul(0x4396_0000, ee::cosf(a)));
            b.cp[0][1] = ee::add(b.cp[0][1], ee::mul(0x4396_0000, ee::sinf(a)));
        } else {
            b.cp[0] = space::w2p(pos, player, bounds);
            for i in 0..2 {
                let r = thunder::rand_f(cx, 0x4348_0000);
                let d = if ee::lt(r, 0) { ee::mul(0x4396_0000, 0xbf80_0000) } else { 0x4396_0000 };
                b.cp[0][i] = ee::add(b.cp[0][i], ee::add(r, d));
            }
            let r = thunder::rand_f(cx, 0x4348_0000);
            b.cp[0][2] = ee::add(b.cp[0][2], r);
            b.cp[0][3] = ONE;
        }
    }

    /// `EntryBall(n)` (gcmn 0x004f4710): n free balls off from the
    /// summons' point towards up to 300 round its end.
    fn entry_ball(&mut self, cx: &mut Cx, n: i32) {
        let (pos, ep, tp) = (self.base.anim.pos, self.base.anim.ep, self.t_pos);
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let SummonKind::Dark { balls: Some(balls), smoke_generate_se_one, .. } = &mut self.kind else { return };
        for _ in 0..n {
            let Some(i) = balls.iter().position(|b| b.status == 0) else { break };
            let z = thunder::rand_f(cx, PI);
            let _ = thunder::fptosi(thunder::rand_f(cx, 0x43fa_0000));
            let b = &mut balls[i];
            b.elm.base.anim.pos = pos;
            b.elm.base.anim.sp = pos;
            b.sp = space::w2p(pos, player, bounds);
            b.cp[0] = b.sp;
            let d = ee::fabsf(thunder::rand_f(cx, 0x4396_0000));
            let mut q = space::w2p(ep, player, bounds);
            q[0] = ee::add(q[0], ee::mul(d, ee::cosf(z)));
            q[1] = ee::add(q[1], ee::mul(d, ee::sinf(z)));
            let b = &mut balls[i];
            b.elm.base.anim.ep = space::p2w(q, player, bounds);
            b.ep = q;
            b.status = 1;
            b.t = 0;
            if let DrawKind::LightBall { gp_ball, .. } = b.elm.kind {
                pause(cx, gp_ball, false);
            }
            let b = &mut balls[i];
            b.time_mode = 0;
            b.t = 0;
            Self::calc_spline_point(cx, b, pos, tp);
            if *smoke_generate_se_one == 0 {
                *smoke_generate_se_one = 1;
                cx.raise(Event::Sound3dNote { se: 174, pos: tp, note: 36 });
            }
        }
    }

    /// `EntryBat()` (gcmn 0x004f4a20): a free bat off along a spline out
    /// 500-1000 and back to the summons' end.
    fn entry_bat(&mut self, cx: &mut Cx) {
        let (pos, ep, tp) = (self.base.anim.pos, self.base.anim.ep, self.t_pos);
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let SummonKind::Dark { bats: Some(bats), bat_generate_se, bat_shock_se, .. } = &mut self.kind else {
            return;
        };
        let Some(i) = bats.iter().position(|b| b.status == 0) else {
            *bat_generate_se = 0;
            *bat_shock_se = 0;
            return;
        };
        let pp = space::w2p(pos, player, bounds);
        let mut r = VF0;
        r[1] = ee::from_int(thunder::fptosi(thunder::rand_f(cx, PI)).wrapping_abs());
        r[2] = thunder::rand_f(cx, PI);
        let d = ee::add(0x43fa_0000, ee::from_int(thunder::fptosi(thunder::rand_f(cx, 0x43fa_0000)).wrapping_abs()));
        let c0 = ee::vadd(pp, ee::apply(&vu::rot_zyx(&vu::UNIT, r), [d, 0, 0, ONE]));
        let c1 = ee::vadd(pp, ee::apply(&vu::rot_z(&vu::UNIT, r[2]), [d, 0, 0, ONE]));
        let epp = space::w2p(ep, player, bounds);
        let b = &mut bats[i];
        b.cp = [c0, c1];
        let a = &mut b.elm.base.anim;
        a.pos = pos;
        a.sp = pos;
        a.ep = ep;
        a.scale[0] = 0x4040_0000;
        a.scale[1] = 0x4040_0000;
        a.scale[2] = 0x4040_0000;
        b.sp = pp;
        b.ep = epp;
        b.status = 1;
        b.t = 0;
        if *bat_generate_se == 0 {
            *bat_generate_se = 1;
            cx.raise(Event::Sound3d { se: 63, pos: tp });
        }
    }

    /// `ccDarkSummonsElement::Main` (gcmn 0x004f3f80).
    fn dark_main(&mut self, cx: &mut Cx) {
        self.track(cx);
        let tp = self.t_pos;
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        match self.base.level {
            3 => {
                let mut all = true;
                let count = self.base.count;
                let SummonKind::Dark { bats: Some(bats), bat_shock_se, .. } = &mut self.kind else { return };
                let mut first_shock = None;
                for (i, b) in bats.iter_mut().enumerate() {
                    if b.status != 1 {
                        continue;
                    }
                    b.elm.main(cx);
                    let t = b.t;
                    let m = [b.sp, b.cp[0], b.cp[1], b.ep];
                    let mut p = ee::apply(&m, bernstein(t));
                    p[3] = ONE;
                    let w = space::p2w(p, player, bounds);
                    let cur = space::w2p(b.elm.base.anim.pos, player, bounds);
                    if !ee::lt(t, ONE) {
                        b.status = 2;
                        let (bp, bd) = (b.elm.base.anim.pos, b.elm.base.anim.dirc);
                        ring::eff_summon_ring_element(cx, bp, bd, 194);
                        if *bat_shock_se == 0 {
                            cx.raise(Event::Sound3dNote { se: 187, pos: tp, note: 72 });
                            *bat_shock_se = 1;
                            first_shock = Some(count);
                        }
                        if i & 3 == 0 {
                            cx.raise(Event::Sound3dNote { se: 170, pos: tp, note: 52 });
                        }
                    }
                    b.t = ee::add(t, 0x3cf5_c28f);
                    let up = ee::atan2f(ee::sub(cur[2], p[2]), drawelm::get_dist(cur, p));
                    let dirc = drawelm::get_dirc(cur, p);
                    b.elm.base.anim.dirc = [0, up, dirc, ONE];
                    b.elm.base.anim.pos = w;
                    all = false;
                }
                if let Some(c) = first_shock {
                    self.base.proccess = c;
                }
                if let SummonKind::Dark { dark_ball, .. } = &mut self.kind {
                    dark_ball.main(cx);
                    let (pos, gs) = (dark_ball.base.anim.pos, dark_gens(dark_ball));
                    follow(cx, gs.0, pos);
                    follow(cx, gs.1, pos);
                }
                let c = self.base.count;
                if c >= 60 {
                    if all {
                        self.base.del_flag = 1;
                        self.damage(cx);
                    }
                } else if c & 0x20 == 0 {
                    self.entry_bat(cx);
                }
                if let SummonKind::Dark { bat_shock_se, .. } = &mut self.kind
                    && *bat_shock_se != 0
                    && self.base.count - self.base.proccess == 60
                {
                    *bat_shock_se = 0;
                }
                self.base.count += 1;
            }
            4 => {
                let c = self.base.count;
                self.base.count = c + 1;
                if c < 150 && c & 15 == 0 {
                    self.entry_ball(cx, 1);
                } else if c >= 150 {
                    let SummonKind::Dark { balls: Some(balls), .. } = &self.kind else { return };
                    if balls.iter().all(|b| b.status == 0) {
                        self.base.del_flag = 1;
                        self.damage(cx);
                    }
                }
                let pos = self.base.anim.pos;
                let SummonKind::Dark { balls: Some(balls), .. } = &mut self.kind else { return };
                for b in balls.iter_mut() {
                    let mut t = b.t;
                    if b.elm.base.del_flag != 0 || b.status == 0 {
                        continue;
                    }
                    let m = [b.sp, b.cp[0], b.cp[0], b.cp[0]];
                    let mut p = ee::apply(&m, bernstein(t));
                    p[3] = ONE;
                    b.elm.base.anim.pos = space::p2w(p, player, bounds);
                    let gp = match b.elm.kind {
                        DrawKind::LightBall { gp_ball, .. } => gp_ball,
                        _ => None,
                    };
                    if b.time_mode >= 4 {
                        explode_growing(cx, b.elm.base.anim.ep, ONE, 4);
                        shake_noise(cx, tp);
                        cx.raise(Event::Sound3d { se: 57, pos: tp });
                        cx.raise(Event::Sound3dNote { se: 35, pos: tp, note: 55 });
                        pause(cx, gp, true);
                        b.status = 0;
                    }
                    t = ee::add(t, b.add_time);
                    if !ee::lt(t, ONE) {
                        b.time_mode += 1;
                        t = 0;
                        Self::calc_spline_point(cx, b, pos, tp);
                        if b.time_mode == 3 {
                            b.add_time = ee::mul(b.add_time, 0x4000_0000);
                            b.cp[0] = b.ep;
                        }
                    }
                    b.t = t;
                    follow(cx, gp, b.elm.base.anim.pos);
                }
                if let SummonKind::Dark { dark_ball, .. } = &mut self.kind {
                    dark_ball.main(cx);
                    let (pos, gs) = (dark_ball.base.anim.pos, dark_gens(dark_ball));
                    follow(cx, gs.0, pos);
                    follow(cx, gs.1, pos);
                }
            }
            _ => {}
        }
    }

    // soil ------------------------------------------------------------------

    /// `new ccSoilSummonsElement(skill, level)` (gcmn 0x004f4d10).
    fn soil(cx: &mut Cx, s: &Spell, level: i32) -> Summoned {
        let n = by_level(&cx.spells.data.summoned.soil, level);
        let mut base = Self::base_of(s, level);
        let (top, p) = Self::above(cx, s.t_pos, 0x4334_0000);
        base.anim.pos = p;
        base.anim.sp = p;
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let mut els: Vec<HeldSlot> = (0..n.max(0))
            .map(|_| HeldSlot { elm: DrawElm::rock(cx), status: 0, interval: 0, cp: [[0; 4]; 2], t: 0 })
            .collect();
        for (i, slot) in els.iter_mut().enumerate() {
            let x = thunder::add_abs(1000.0, thunder::rand_f(cx, 0x43fa_0000));
            let ty = ee::neg(ee::fabsf(thunder::rand_f(cx, PI)));
            let tz = thunder::rand_f(cx, PI);
            let q = space::p2w(ee::vadd(top, out_by(x, ty, tz)), player, bounds);
            let a = &mut slot.elm.base.anim;
            a.pos = q;
            a.sp = q;
            a.ep = p;
            fade(a, 0, ONE, 0x3d88_8889);
            for k in 0..3 {
                a.scale[k] = 0x4040_0000;
                a.speed[k] = 0x4248_0000;
            }
            slot.status = 0;
            slot.interval = (i as i32 / 2) * 5;
            let dirc = random_turn(cx);
            ring::eff_summon_ring_element(cx, p, dirc, 167);
        }
        Summoned { base, elm_num: n, t_pos: s.t_pos, kind: SummonKind::Soil { elements: els } }
    }

    /// `ccSoilSummonsElement::Main` (gcmn 0x004f5390).
    fn soil_main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        self.track(cx);
        let tp = self.t_pos;
        let key = self.base.skill;
        let level = self.base.level;
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let SummonKind::Soil { elements } = &mut self.kind else { return };
        let mut all = true;
        for (i, slot) in elements.iter_mut().enumerate() {
            if slot.elm.base.del_flag != 0 {
                continue;
            }
            all = false;
            match slot.status {
                0 => {
                    let iv = slot.interval;
                    slot.interval = iv - 1;
                    if iv <= 0 {
                        slot.status += 1;
                    }
                }
                1 => {
                    let a = &slot.elm.base.anim;
                    let mut p = space::w2p(a.pos, player, bounds);
                    let q = space::w2p(a.ep, player, bounds);
                    let up = ee::atan2f(ee::sub(p[2], q[2]), drawelm::get_dist(p, q));
                    let dirc = drawelm::get_dirc(p, q);
                    let mut spd = a.speed[0];
                    let m = vu::rot_z(&vu::rot_z(&vu::rot_y(&vu::UNIT, up), dirc), NEG_HALF_PI);
                    p = ee::vadd(p, ee::apply(&m, [spd, 0, 0, ONE]));
                    let a = &mut slot.elm.base.anim;
                    a.pos = space::p2w(p, player, bounds);
                    a.dirc[1] = up;
                    a.dirc[2] = dirc;
                    if ee::le(drawelm::get_dist3d(p, q), ee::add(0x4120_0000, spd)) {
                        let (pos, d) = (a.pos, a.dirc);
                        ring::eff_summon_ring_element(cx, pos, d, 196);
                        if level == 4 {
                            let mut e = pos;
                            for v in e.iter_mut().take(3) {
                                *v = ee::add(*v, thunder::rand_f(cx, 0x42c8_0000));
                            }
                            drawelm::eff_explode3(cx, e, VF0, ONE, 5);
                        }
                        shake_noise(cx, tp);
                        debris::eff_radiate_something(ctrl, cx, pos, d, 0x4234_0000, attr::SOIL, 4);
                        if i == 0
                            && let Some(k) = key
                        {
                            damage2_of(cx, k);
                        }
                        cx.raise(Event::Sound3d { se: 65, pos });
                        slot.elm.base.del_flag = 1;
                    }
                    spd = ee::add(spd, ONE);
                    if !ee::lt(spd, 0x42c8_0000) {
                        spd = 0x42c8_0000;
                    }
                    let a = &mut slot.elm.base.anim;
                    a.speed[2] = spd;
                    a.speed[1] = spd;
                    a.speed[0] = spd;
                    a.animate_fade();
                    slot.elm.main(cx);
                }
                _ => {}
            }
        }
        if all {
            self.base.del_flag = 1;
        }
    }

    // wind ------------------------------------------------------------------

    /// `new ccTreeSummonsElement(skill, level)` (gcmn 0x004f5850): the tree
    /// at the model 1000 above the target's point (ccHitCheckLM2; its x
    /// and y the stack's leftovers in the game, 0 here), else at it.
    fn tree(cx: &mut Cx, s: &Spell, level: i32) -> Summoned {
        let mut base = Self::base_of(s, level);
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let tpp = space::w2p(s.t_pos, player, bounds);
        let mut to = [0, 0, ee::add(tpp[2], 0x447a_0000), 0];
        let hit = cx.host.hit_check_lm2(tpp, &mut to, 0x2000_0000);
        let p = if ee::eq(0xbf80_0000, hit) { s.t_pos } else { space::p2w(to, player, bounds) };
        base.anim.pos = p;
        base.anim.sp = p;
        let anm_roots = AnmObj::new(cx, "particle", "ANM_x403");
        let cmp_tree = drawelm::particle_clump(cx, "CMP_x401b_1");
        fade(&mut base.anim, 0, ONE, 0x3d88_8889);
        scale_anim(&mut base.anim, VF0, [0x4040_0000, 0x4040_0000, 0x4040_0000, ONE], 0x4170_0000);
        for _ in 0..by_level(&cx.spells.data.summoned.tree_rings, level).max(0) {
            let dirc = random_turn(cx);
            ring::eff_summon_ring_element(cx, p, dirc, 164);
        }
        let mut at = p;
        at[2] = ee::add(at[2], 0x43c8_0000);
        let (param, ff) = (cx.spells.data.summoned.leaf_gen, cx.spells.data.summoned.leaf_ff);
        let leaf = pfx::start_param(cx, param, [Some(ff), None, None, None], |g| g.pos = at);
        let mut needles = None;
        if level == 4 {
            let mut top = tpp;
            top[2] = ee::add(top[2], 0x43fa_0000);
            let start = space::p2w(top, player, bounds);
            let mut low = tpp;
            low[2] = ee::add(low[2], 0x42b4_0000);
            let end = space::p2w(low, player, bounds);
            let mut v: Vec<HeldSlot> = (0..32)
                .map(|_| HeldSlot { elm: DrawElm::needle(cx), status: 0, interval: 0, cp: [[0; 4]; 2], t: 0 })
                .collect();
            for n in v.iter_mut() {
                let mut d = VF0;
                d[2] = thunder::add_abs(500.0, thunder::rand_f(cx, 0x43fa_0000));
                let q = ee::vadd(top, d);
                n.cp[0] = space::p2w(q, player, bounds);
                n.elm.base.anim.dirc[1] = ee::atan2f(ee::sub(top[2], q[2]), drawelm::get_dist(top, q));
                n.elm.base.anim.dirc[2] = drawelm::get_dirc(top, q);
                let r = thunder::rand_f(cx, 0x3f00_0000);
                let x = ((1.0 - f64::from(f32::from_bits(r)).abs()) * 1000.0) as f32;
                let a = thunder::rand_f(cx, PI);
                let v1 = ee::apply(&vu::rot_z(&vu::UNIT, a), [x.to_bits(), 0, 0, ONE]);
                n.cp[1] = space::p2w(ee::vadd(top, v1), player, bounds);
                n.status = 0;
                n.t = 0;
                n.interval = (cx.host.genrand() as i32) % 15 + 15;
                let an = &mut n.elm.base.anim;
                an.pos = start;
                an.sp = start;
                an.ep = end;
                fade(an, 0, ONE, 0x3d88_8889);
                scale_anim(an, VF0, [ONE; 4], 0x4170_0000);
            }
            needles = Some(v);
        }
        let mut roots = [[0; 4]; 8];
        let mut a: F = 0;
        for _ in 0..8 {
            roots[0] = p;
            roots[0][0] = ee::add(roots[0][0], ee::mul(0x42c8_0000, ee::cosf(a)));
            roots[0][1] = ee::add(roots[0][1], ee::mul(0x42c8_0000, ee::sinf(a)));
            a = ee::add(a, 0x3f49_0fdb);
            if !ee::le(a, PI) {
                a = ee::sub(a, TWO_PI);
            }
        }
        let kind = SummonKind::Tree { needles, roots, gp_leaf: Some(leaf), cmp_tree, anm_roots, tree_alpha: 0 };
        Summoned { base, elm_num: 0, t_pos: s.t_pos, kind }
    }

    /// The tree's clump and roots at the element's place.
    fn tree_draw(cx: &mut Cx, a: &Animate, alpha: F, cmp: Option<ObjRef>, anm: &mut Option<AnmObj>, forward: bool) {
        let m = vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale);
        drawelm::draw_clump(cx, cmp, m, alpha);
        if let Some(r) = anm.as_mut() {
            if forward {
                r.forward(cx);
            }
            r.draw(cx, m, alpha);
        }
    }

    /// `ccTreeSummonsElement::Main` (gcmn 0x004f6420).
    fn tree_main(&mut self, cx: &mut Cx) {
        let alpha = self.base.anim.transparency;
        self.track(cx);
        let tp = self.t_pos;
        let key = self.base.skill;
        let half = ee::vscale(self.base.anim.scale, 0x3f00_0000);
        let dirc = self.base.anim.dirc;
        let tr = self.base.anim.transparency;
        let SummonKind::Tree { roots, anm_roots, .. } = &mut self.kind else { return };
        if let Some(r) = anm_roots.as_ref() {
            for p in roots.iter() {
                r.draw(cx, vu::pos_rot_zyx_scale(*p, dirc, half), tr);
            }
        }
        let level = self.base.level;
        let pulse = |cx: &mut Cx| {
            cx.raise(Event::Sound3d { se: 56, pos: tp });
            shake_noise(cx, tp);
        };
        if level == 4 {
            self.base.count += 1;
            if self.base.count & 15 == 0 {
                pulse(cx);
            }
        }
        if level != 3 && level != 4 {
            return;
        }
        match self.base.proccess {
            0 => {
                let faded = self.base.anim.animate_fade();
                let scaled = self.base.anim.animate_scale();
                let a = self.base.anim.clone();
                let SummonKind::Tree { cmp_tree, anm_roots, gp_leaf, .. } = &mut self.kind else { return };
                Self::tree_draw(cx, &a, alpha, *cmp_tree, anm_roots, false);
                if faded && scaled {
                    if level == 3 {
                        fade(&mut self.base.anim, ONE, 0, 0xbd88_8889);
                    }
                    let g = *gp_leaf;
                    pause(cx, g, false);
                    self.base.proccess += 1;
                }
            }
            1 if level == 3 => {
                let a = self.base.anim.clone();
                let SummonKind::Tree { cmp_tree, anm_roots, gp_leaf, .. } = &mut self.kind else { return };
                Self::tree_draw(cx, &a, alpha, *cmp_tree, anm_roots, true);
                let g = *gp_leaf;
                let c = self.base.count;
                if c >= 120 {
                    if c == 120 {
                        pause(cx, g, true);
                        if let Some(k) = key {
                            damage2_of(cx, k);
                        }
                    }
                    if self.base.anim.animate_fade() {
                        self.base.del_flag = 1;
                    }
                }
                if self.base.count & 15 == 0 {
                    pulse(cx);
                }
                self.base.count += 1;
            }
            1 => {
                let a = self.base.anim.clone();
                let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
                let SummonKind::Tree { cmp_tree, anm_roots, needles, .. } = &mut self.kind else { return };
                Self::tree_draw(cx, &a, alpha, *cmp_tree, anm_roots, true);
                let mut all = true;
                let Some(needles) = needles.as_mut() else { return };
                for (i, n) in needles.iter_mut().enumerate() {
                    n.elm.main(cx);
                    match n.status {
                        0 => {
                            let iv = n.interval;
                            n.interval = iv - 1;
                            if iv < 0 {
                                n.status += 1;
                            }
                            all = false;
                        }
                        1 => {
                            let faded = n.elm.base.anim.animate_fade();
                            let scaled = n.elm.base.anim.animate_scale();
                            if faded && scaled {
                                n.status += 1;
                            }
                            all = false;
                        }
                        2 => {
                            let an = &n.elm.base.anim;
                            let m = [
                                space::w2p(an.sp, player, bounds),
                                space::w2p(n.cp[0], player, bounds),
                                space::w2p(n.cp[1], player, bounds),
                                space::w2p(an.ep, player, bounds),
                            ];
                            let mut p = space::p2w(ee::apply(&m, bernstein(n.t)), player, bounds);
                            p[3] = ONE;
                            let cur = space::w2p(an.pos, player, bounds);
                            let q = space::w2p(p, player, bounds);
                            let up = ee::atan2f(ee::sub(cur[2], q[2]), drawelm::get_dist(cur, q));
                            let d = drawelm::get_dirc(cur, q);
                            let an = &mut n.elm.base.anim;
                            an.dirc[1] = up;
                            an.dirc[2] = d;
                            an.pos = p;
                            n.t = ee::add(n.t, 0x3cf5_c28f);
                            if !ee::le(n.t, ONE) {
                                n.status += 1;
                                n.interval = 0;
                                fade(&mut n.elm.base.anim, ONE, 0, 0xbd88_8889);
                                let r = random_turn(cx);
                                ring::eff_summon_ring_element(cx, p, r, 196);
                                eff_skill_break_se(cx, p, attr::WIND);
                                if i == 0
                                    && let Some(k) = key
                                {
                                    damage2_of(cx, k);
                                }
                            }
                            all = false;
                        }
                        3 => {
                            let c = n.interval;
                            n.interval = c + 1;
                            if c >= 16 && n.elm.base.anim.animate_fade() {
                                n.status += 1;
                            }
                            all = false;
                        }
                        _ => {}
                    }
                }
                if all {
                    self.base.del_flag = 1;
                }
            }
            _ => {}
        }
    }

    // none ------------------------------------------------------------------

    /// `new ccGoblinSummonsElement(skill, level)` (gcmn 0x004f7020).
    fn goblin(cx: &mut Cx, s: &Spell, level: i32) -> Summoned {
        let mut base = Self::base_of(s, level);
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let tpp = space::w2p(s.t_pos, player, bounds);
        let (_, p) = Self::above(cx, s.t_pos, 0x4334_0000);
        let mut gen_pos_p = tpp;
        gen_pos_p[2] = ee::add(gen_pos_p[2], 0x442a_0000);
        let gen_pos = space::p2w(gen_pos_p, player, bounds);
        base.anim.pos = p;
        base.anim.sp = p;
        base.anim.ep = s.t_pos;
        for _ in 0..by_level(&cx.spells.data.summoned.goblin_rings, level).max(0) {
            let mut dirc = [0; 4];
            dirc[1] = thunder::rand_f(cx, PI);
            dirc[2] = thunder::rand_f(cx, PI);
            ring::eff_summon_ring_element(cx, p, dirc, 166);
        }
        let mut elements = None;
        let mut stars = None;
        if level == 3 {
            let mut v: Vec<HeldSlot> = (0..8)
                .map(|_| HeldSlot { elm: DrawElm::star(cx), status: 0, interval: 0, cp: [[0; 4]; 2], t: 0 })
                .collect();
            for (i, st) in v.iter_mut().enumerate() {
                let mut q = space::w2p(s.t_pos, player, bounds);
                let mut d = VF0;
                d[0] = thunder::add_abs(100.0, thunder::rand_f(cx, 0x43c8_0000));
                d[2] = 0x442a_0000;
                let a = thunder::rand_f(cx, PI);
                q = ee::vadd(q, ee::apply(&vu::rot_z(&vu::UNIT, a), d));
                let sp = space::p2w(q, player, bounds);
                let mut to = q;
                to[2] = ee::add(to[2], 0x447a_0000);
                if ee::eq(0xbf80_0000, cx.host.hit_check_lm2(q, &mut to, 0x2000_0000)) {
                    let _ = thunder::rand_f(cx, PI);
                    to = q;
                    to[2] = ee::add(tpp[2], thunder::rand_f(cx, 0x4348_0000));
                }
                let ep = space::p2w(to, player, bounds);
                let an = &mut st.elm.base.anim;
                an.sp = sp;
                an.ep = ep;
                an.pos = sp;
                if let DrawKind::Star { gp_tail, .. } = st.elm.kind {
                    pause(cx, gp_tail, true);
                }
                let an = &mut st.elm.base.anim;
                for k in 0..3 {
                    an.scale[k] = 0x4040_0000;
                }
                fade(an, 0, ONE, 0x3e4c_cccd);
                an.speed[2] = 0xc120_0000;
                st.elm.base.accel[2] = 0xc120_0000;
                st.interval = i as i32 * 5;
                st.status = 0;
            }
            stars = Some(v);
        } else if level == 4 {
            let mut v: Vec<StarTrail> = (0..8)
                .map(|_| StarTrail {
                    gp: None,
                    star: DrawElm::star(cx),
                    status: 0,
                    pos: [0; 4],
                    t: 0,
                    spd: 0,
                    cp: [[0; 4]; 2],
                    cp_p: [[0; 4]; 2],
                    ep: [0; 4],
                    ep_p: [0; 4],
                })
                .collect();
            let (param, ff) = (cx.spells.data.summoned.trail_gen, cx.spells.data.summoned.trail_ff);
            for e in v.iter_mut() {
                let g = pfx::start_param(cx, param, [Some(ff), None, None, None], |g| {
                    g.sync_pos_type = false;
                    g.p_tex_mod = 109;
                });
                pause(cx, Some(g), true);
                e.pos = gen_pos;
                e.status = 0;
                e.gp = Some(g);
            }
            elements = Some(v);
        }
        let kind = SummonKind::Goblin { gen_pos, gen_pos_p, elements, stars };
        Summoned { base, elm_num: 0, t_pos: s.t_pos, kind }
    }

    /// `_EntryElement(n)` (gcmn 0x004f8070): n free stars off towards up
    /// to 500 round the summons' end. The game keeps the element it last
    /// set up in the register it searches from, so its second and later
    /// searches count from there and use what they find as an index from
    /// the table's start (and write a star's control points into the one
    /// before); a search past the table's end reads beyond it - taken here
    /// as busy.
    fn goblin_entry(&mut self, cx: &mut Cx, n: i32) {
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let epp = space::w2p(self.base.anim.ep, player, bounds);
        let SummonKind::Goblin { gen_pos, gen_pos_p, elements: Some(els), .. } = &mut self.kind else { return };
        let (gpos, gpp) = (*gen_pos, *gen_pos_p);
        let mut from = 0;
        for _ in 0..n {
            let Some(r) = (0..8).find(|&a| els.get(from + a).is_some_and(|e| e.status == 0)) else { return };
            let x = thunder::add_abs(100.0, thunder::rand_f(cx, 0x447a_0000));
            let a = thunder::rand_f(cx, PI);
            let c = ee::vadd(gpp, ee::apply(&vu::rot_z(&vu::UNIT, a), [x, 0, 0, ONE]));
            let w = space::p2w(c, player, bounds);
            let e = &mut els[from];
            e.cp_p = [c, c];
            e.cp = [w, w];
            let d = ee::fabsf(thunder::rand_f(cx, 0x43fa_0000));
            let a = thunder::rand_f(cx, PI);
            let q = ee::vadd(epp, ee::apply(&vu::rot_z(&vu::UNIT, a), [d, 0, 0, ONE]));
            let e = &mut els[r];
            e.ep = space::p2w(q, player, bounds);
            e.ep_p = q;
            e.status = 1;
            let g = e.gp;
            pause(cx, g, false);
            let e = &mut els[r];
            e.t = 0;
            e.spd = thunder::add_abs(f64::from(f32::from_bits(0x3ca3_d70a)), thunder::rand_f(cx, 0x3ca3_d70a));
            e.pos = gpos;
            let ep = e.ep;
            let an = &mut e.star.base.anim;
            an.sp = gpos;
            an.ep = ep;
            an.pos = gpos;
            for k in 0..3 {
                an.scale[k] = 0x4040_0000;
            }
            if let DrawKind::Star { gp_tail, .. } = e.star.kind {
                pause(cx, gp_tail, false);
            }
            from = r;
        }
    }

    /// `ccGoblinSummonsElement::Main` (gcmn 0x004f79e0).
    fn goblin_main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let key = self.base.skill;
        let pos = self.base.anim.pos;
        match self.base.level {
            3 => {
                let SummonKind::Goblin { stars: Some(stars), .. } = &mut self.kind else { return };
                let mut all = true;
                for st in stars.iter_mut() {
                    if st.elm.base.del_flag != 0 {
                        continue;
                    }
                    all = false;
                    match st.status {
                        0 => {
                            let iv = st.interval;
                            st.interval = iv - 1;
                            if iv < 0 {
                                st.status += 1;
                                cx.raise(Event::Sound3d { se: 75, pos: st.elm.base.anim.pos });
                            }
                        }
                        1 => {
                            st.elm.main(cx);
                            let accel = st.elm.base.accel;
                            let an = &mut st.elm.base.anim;
                            an.animate_fade();
                            an.pos = ee::vadd(an.pos, an.speed);
                            an.speed = ee::vadd(an.speed, accel);
                            if ee::lt(an.speed[2], 0xc348_0000) {
                                an.speed[2] = 0xc348_0000;
                            }
                            let gp = match st.elm.kind {
                                DrawKind::Star { gp_tail, .. } => gp_tail,
                                _ => None,
                            };
                            if ee::le(st.elm.base.anim.pos[2], st.elm.base.anim.ep[2]) {
                                st.elm.base.del_flag = 1;
                                pause(cx, gp, true);
                                let ep = st.elm.base.anim.ep;
                                drawelm::eff_explode3(cx, ep, VF0, 0x3fc0_0000, -1);
                                eff_skill_break_se(cx, pos, attr::FIRE);
                                let mut r = VF0;
                                r[0] = ee::deg2rad(-16384);
                                r[2] = thunder::rand_f(cx, PI);
                                debris::eff_radiate_something2(ctrl, cx, ep, r, 0x4234_0000, -1, 16);
                                st.status += 1;
                            }
                            follow(cx, gp, st.elm.base.anim.pos);
                        }
                        _ => {}
                    }
                }
                if all {
                    if let Some(k) = key {
                        damage2_of(cx, k);
                    }
                    self.base.del_flag = 1;
                }
            }
            4 => {
                let tp = self.t_pos;
                let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
                let SummonKind::Goblin { gen_pos_p, elements: Some(els), .. } = &mut self.kind else { return };
                let gpp = *gen_pos_p;
                let mut all = true;
                for (i, e) in els.iter_mut().enumerate() {
                    if e.status == 0 {
                        continue;
                    }
                    let m = [
                        space::w2p(gpp, player, bounds),
                        space::w2p(e.cp_p[0], player, bounds),
                        space::w2p(e.cp_p[1], player, bounds),
                        space::w2p(e.ep_p, player, bounds),
                    ];
                    let mut p = space::p2w(ee::apply(&m, bernstein(e.t)), player, bounds);
                    p[3] = ONE;
                    let w = space::p2w(p, player, bounds);
                    let t = ee::add(e.t, e.spd);
                    if !ee::lt(t, ONE) {
                        e.status = 0;
                        if i & 1 != 0 {
                            explode_growing(cx, e.ep, 0x4000_0000, -1);
                        }
                        shake_noise(cx, tp);
                        let s = key.and_then(|k| cx.spells.get(k)).map_or(VF0, |s| s.t_pos);
                        eff_skill_break_se(cx, s, attr::FIRE);
                    }
                    e.t = t;
                    e.pos = w;
                    e.star.base.anim.pos = w;
                    e.star.main(cx);
                    follow(cx, e.gp, e.pos);
                    if let DrawKind::Star { gp_tail, .. } = e.star.kind {
                        follow(cx, gp_tail, e.star.base.anim.pos);
                    }
                    all = false;
                }
                let c = self.base.count;
                self.base.count = c + 1;
                if c < 60 {
                    if self.base.count & 0x1f != 0 {
                        self.goblin_entry(cx, 3);
                    }
                } else if all {
                    if let Some(k) = key {
                        damage2_of(cx, k);
                    }
                    self.base.del_flag = 1;
                }
            }
            _ => {}
        }
    }
}

/// The dark ball's two generators.
fn dark_gens(b: &DrawElm) -> (Option<GenRef>, Option<GenRef>) {
    match b.kind {
        DrawKind::DarkBall { gp_star, gp_conv, .. } => (gp_star, gp_conv),
        _ => (None, None),
    }
}

impl SummonsSystem {
    /// The summoned element.
    pub fn summoned(&self) -> &Summoned {
        &self.summon
    }
}
