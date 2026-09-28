//! The boss's effects (gcmn bosseff.cpp): `ccBossEffManager`'s effects as
//! Skeith makes them - their `ccBossEff*Create`, their constructors and
//! the `Draw` the manager's task (`ccThBossEffect`, priority 66) runs over
//! them each frame, before the boss's own task.
//!
//! ```text
//! ccBossEffManager::Draw (0x00461750), each frame:
//!   SetActiveLayer(3) (effLayer, priority 20)
//!   each of the 1024 slots in order: an effect whose m_bEnabled (byte 0)
//!     is 0 is deleted (vtable +0xc, 1) and the slot freed; else its Draw
//!     (vtable +0x8)
//!   SetActiveLayer(5); ccBossBlur::Main; the cinema's Draw (not ported)
//! a Create: new the effect, the first free slot takes it; the slot (or -1)
//! ```
//!
//! Skeith's six (`docs/engine/boss.md`, "The effects' pictures"):
//!
//! ```text
//! WaveShock(pos, dirc, s)  0x00478820 SE 36, 68, 56 (note 60) at pos; an
//!                          Eruption: g_eruptionGenerator at pos, pTexMod 158,
//!                          into a slot disabled (the next pass deletes it);
//!                          then ANM_ex31lhit of xeffect: each Draw a step,
//!                          PosRotZYXScale(pos, 0, 5 s); at its end disabled
//!                          (the dirc is not used)
//! MagicSquare(pos, 0)      0x00478b10 SE 228; MagicSquareGenerator rows 0, 1
//!                          and 7 at pos, distSW off; ccBossEffLight(pos, 0,
//!                          80, 10) (returns -1)
//! Light(pos, 0, 80, 10)    0x00466a30 an omni light at pos plus (0, 200,
//!                          300) turned about x by -pi/2 and about z by the
//!                          camera's heading, into the light group after 10
//!                          Draws, blue rising by 10 a Draw to 250, 15 at
//!                          255, then falling; 80 Draws
//! ForceGenerator(p, r, speed, r0, r1, num, life, clt)
//!                          0x004797e0 a ccBossEffBrightMagicSquare: num
//!                          photons (EFF_x000 of particle, CLT_x000c(clt-3))
//!                          spiralling down from p, pi/16 a Draw, each with
//!                          g_energyOutGenerator following it; photon k
//!                          waits (k / 2) 10 Draws, then lives life
//! AutoSamonRing(pos, rot, (s, ds, _), n)
//!                          0x00479900 a ccEffSamonRing (SetModel(n): n 35 is
//!                          particle's CMP_x033) bursting: scale from s by ds
//!                          a Draw, transparency from 1 by 0.05; below 0 gone
//! IceBreak(pos, s)         0x00479240 particle's CMP_x201d at pos, scale s;
//!                          g_IceSmokeGenerator at (pos.x, pos.y, 180); ccRand()
//!                          drawn once; flash 60, after 60 Draws the smoke
//!                          killed, after 60 more twelve effIceRock thrown out
//!                          four ways, flash 30, gone
//! Dead(pos, pos, 0)        0x004795d0 BossDeadGenerator's rows over four
//!                          stages (at once, 35, 60 and 20 Draws apart), SE 56
//!                          and 104; gone once the last generator has ended
//! ```
//!
//! The generators go to the particle system ([`crate::particle`]); a
//! photon's position is published as an anchor ([`VecRef::Anchor`]) for
//! its generator to follow.

use std::collections::HashMap;

use piney_data::volume::Volume;

use crate::draw::DrawRec;
use crate::ee::{self, F, ONE, V4, VF0};
use crate::eff::Eff;
use crate::effect::{EFFECT_LAYER, EffectCtrl};
use crate::files::ObjRef;
use crate::particle::{FfParam, GenParam, GenRef, Generator};
use crate::{Cx, Event, VecRef, debris, drawelm, vu};

/// The manager's slots.
pub const SLOTS: usize = 1024;

const PI: F = ee::PI;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_PI: F = 0xc049_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;

/// The parameter rows the effects make their generators from (gcmn data),
/// named by Infection's addresses (the rows are the volume's own,
/// `tables::effect`).
pub mod va {
    /// `g_eruptionGenerator`, `g_eruptionForceField`.
    pub const ERUPTION: u32 = 0x005e_ea30;
    pub const ERUPTION_FF: u32 = 0x005e_ea70;
    /// `MagicSquareGenerator`, `MagicSquareForceField`.
    pub const MAGIC_SQUARE: u32 = 0x005e_ff60;
    pub const MAGIC_SQUARE_FF: u32 = 0x005f_0120;
    /// `g_energyOutGenerator`, `g_energyOutForceField`.
    pub const ENERGY_OUT: u32 = 0x005f_0580;
    pub const ENERGY_OUT_FF: u32 = 0x005f_05c0;
    /// `g_IceSmokeGenerator`, `g_IceSmokeForceField`.
    pub const ICE_SMOKE: u32 = 0x005f_02c0;
    pub const ICE_SMOKE_FF: u32 = 0x005f_0300;
    /// `BossDeadGenerator`, `BossDeadForceField`.
    pub const DEAD: u32 = 0x005e_eed0;
    pub const DEAD_FF: u32 = 0x005e_ef80;
}

/// The rows of [`va`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tables {
    gens: HashMap<u32, GenParam>,
    ffs: HashMap<u32, FfParam>,
}

impl Tables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> Tables {
        let t = piney_data::tables::effect::of(volume);
        let mut out = Tables::default();
        // Keyed by Infection's addresses (the rows' names in [`va`]).
        let mut gens = |key: u32, rows: &[piney_data::tables::types::ParticleGeneratorParam], at: u32| {
            for k in 0..rows.len() {
                out.gens.insert(key + 0x38 * k as u32, GenParam::other(rows, at, k));
            }
        };
        gens(va::ERUPTION, std::slice::from_ref(&t.boss_eruption()), t.boss_eruption_va());
        gens(va::MAGIC_SQUARE, t.boss_magic_square(), t.boss_magic_square_va());
        gens(va::ENERGY_OUT, std::slice::from_ref(&t.boss_energy_out()), t.boss_energy_out_va());
        gens(va::ICE_SMOKE, std::slice::from_ref(&t.boss_ice_smoke()), t.boss_ice_smoke_va());
        gens(va::DEAD, t.boss_dead(), t.boss_dead_va());
        let mut ffs = |key: u32, rows: &[piney_data::tables::types::ParticleForceFieldParam], at: u32| {
            for k in 0..rows.len() {
                out.ffs.insert(key + 0x20 * k as u32, FfParam::row(rows, at, k));
            }
        };
        ffs(va::ERUPTION_FF, t.boss_eruption_ff(), t.boss_eruption_ff_va());
        ffs(va::MAGIC_SQUARE_FF, t.boss_magic_square_ff(), t.boss_magic_square_ff_va());
        ffs(va::ENERGY_OUT_FF, t.boss_energy_out_ff(), t.boss_energy_out_ff_va());
        ffs(va::ICE_SMOKE_FF, t.boss_ice_smoke_ff(), t.boss_ice_smoke_ff_va());
        ffs(va::DEAD_FF, t.boss_dead_ff(), t.boss_dead_ff_va());
        out
    }

    /// `new ccParticleGenerator(param, f1, f2, f3, f4)` of the rows at
    /// these addresses (0: none).
    fn generator(&self, cx: &mut Cx, param: u32, ff: [u32; 4]) -> Option<Generator> {
        let p = *self.gens.get(&param)?;
        let ff = ff.map(|a| self.ffs.get(&a).copied());
        Some(Generator::new(Some(p), ff, &cx.assets.particle.force_fields, &mut cx.particles.generator_serial))
    }
}

/// The boss effect manager's effects (`_g_bossEffManager`).
#[derive(Clone, Debug, Default)]
pub struct BossEffects {
    pub slots: Vec<Option<Slot>>,
}

/// One slot: `m_bEnabled` and the effect.
#[derive(Clone, Debug)]
pub struct Slot {
    pub enabled: bool,
    pub eff: BossEff,
}

#[derive(Clone, Debug)]
pub enum BossEff {
    /// `ccBossEffEruption`: its generator started, nothing to draw; made
    /// disabled.
    Eruption,
    WaveShock(WaveShock),
    Light(Light),
    Bright(Bright),
    Ring(SamonRing),
    IceBreak(IceBreak),
    Dead(Dead),
}

/// `ccBossEffWaveShock` (0x40 bytes).
#[derive(Clone, Debug)]
pub struct WaveShock {
    /// +0x10 pos, +0x20 rot (vf0), +0x34 scale.
    pub pos: V4,
    pub rot: V4,
    pub scale: F,
    /// +0x30: `ANM_ex31lhit` of xeffect.
    pub anm: Option<drawelm::AnmObj>,
}

/// `ccBossEffLight` (0xc0 bytes): an omni light.
#[derive(Clone, Debug)]
pub struct Light {
    /// +0x10 the light's place, +0x50 its offset (turned), +0x70 the base.
    pub pos: V4,
    pub offset: V4,
    pub base: V4,
    /// +0x80 the camera's rotation as read, +0x98 the heading the offset
    /// is turned by.
    pub cam_rot: V4,
    pub heading: F,
    /// +0x90 falling, +0x94 the Draws held at 255, +0xa8 life, +0xac the
    /// blue, +0xb0 a count, +0xb4 the wait, +0xb8 the type.
    pub falling: bool,
    pub hold: i32,
    pub life: i32,
    pub intensity: i32,
    pub count: i32,
    pub wait: i32,
    pub ty: i32,
    /// The light's colour (`ccSetColor`, r g b / 255) and whether it is in
    /// the light group (`AddGrp` once the wait is over).
    pub colour: [F; 4],
    pub in_group: bool,
}

/// `ccBossEffBrightMagicSquare` (0x60 bytes): its photons.
#[derive(Clone, Debug)]
pub struct Bright {
    /// +0x40 the fall a Draw, +0x44 / +0x48 the radius from / to, +0x58
    /// life.
    pub speed: F,
    pub r0: F,
    pub r1: F,
    pub life: i32,
    pub photons: Vec<Photon>,
}

/// One photon (0x60 bytes).
#[derive(Clone, Debug)]
pub struct Photon {
    /// +0x00 its generator: made with the photon, started when its wait is
    /// over (None once killed).
    pub pending: Option<Box<Generator>>,
    pub generator: Option<GenRef>,
    /// +0x04 its ccEff.
    pub eff: Option<Box<Eff>>,
    /// +0x10 where it is, +0x20 the centre it circles, +0x38 the angle.
    pub pos: V4,
    pub centre: V4,
    pub angle: F,
    /// +0x40 the wait, +0x44 the state (0 waiting, 1 alive, -1 gone),
    /// +0x50 the radius, +0x54 life.
    pub wait: i32,
    pub state: i8,
    pub radius: F,
    pub life: i32,
    /// The anchor its generator follows.
    pub anchor: u32,
}

/// `ccEffSamonRing` (0x80 bytes) as `ccBossEffAutoSamonRing` keeps it.
#[derive(Clone, Debug)]
pub struct SamonRing {
    /// +0x10 pos, +0x20 rot, +0x30 scale, +0x40 the burst's scale step.
    pub pos: V4,
    pub rot: V4,
    pub scale: V4,
    pub burst: V4,
    /// +0x60 bursting, +0x6c transparency, +0x70 its step.
    pub bursting: bool,
    pub transparency: F,
    pub step: F,
    /// +0x78 the clump and its CLUT swap (from, to).
    pub clump: Option<ObjRef>,
    pub clut: Option<(u32, u32)>,
    /// `ccBossEffAutoSamonRing` +0x1c: the step its Draw sets.
    pub ds: F,
}

/// `ccBossEffIceBreak` (0x40 bytes).
#[derive(Clone, Debug)]
pub struct IceBreak {
    /// +0x10 the clump (particle's `CMP_x201d`), +0x14 the count, +0x18 the
    /// state, +0x1c the scale, +0x20 pos, +0x30 transparency, +0x34 the
    /// smoke.
    pub clump: Option<ObjRef>,
    pub count: i32,
    pub state: i32,
    pub scale: F,
    pub pos: V4,
    pub transparency: F,
    pub smoke: Option<GenRef>,
}

/// `ccBossEffDead` (0x60 bytes).
#[derive(Clone, Debug)]
pub struct Dead {
    /// +0x10 the last generator, +0x30 pos (its z raised stage by stage),
    /// +0x54 the stage, +0x58 the count.
    pub generator: Option<GenRef>,
    pub pos: V4,
    pub stage: i32,
    pub count: i32,
}

/// `ccEffSamonRing::SetModel(n)` (0x00461bd0) for the rings Skeith makes:
/// the clump, the CLUT it becomes and the CLUT it replaces, and the file
/// (particle below 161, xeffect for some of the numbered sets).
fn samon_model(n: i32) -> Option<(&'static str, &'static str, &'static str)> {
    match n {
        35 => Some(("CMP_x033", "CLT_x033", "CLT_x033")),
        _ => None,
    }
}

/// A `ccBossEff*Create` call, with its arguments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Make {
    WaveShock { pos: V4, dirc: V4, scale: F },
    MagicSquare { pos: V4, n: i32 },
    ForceGenerator { p: V4, rot: V4, speed: F, r0: F, r1: F, num: i32, life: i32, clt: i32 },
    AutoSamonRing { pos: V4, rot: V4, param: V4, n: i32 },
    IceBreak { pos: V4, scale: F },
    Dead { pos: V4 },
}

impl BossEffects {
    /// The Create `make` names: the slot (or -1).
    pub fn create(&mut self, cx: &mut Cx, t: &Tables, make: Make) -> i32 {
        match make {
            Make::WaveShock { pos, dirc, scale } => self.wave_shock(cx, t, pos, dirc, scale),
            Make::MagicSquare { pos, n } => self.magic_square(cx, t, pos, n),
            Make::ForceGenerator { p, rot, speed, r0, r1, num, life, clt } => {
                self.force_generator(cx, t, p, rot, speed, r0, r1, num, life, clt)
            }
            Make::AutoSamonRing { pos, rot, param, n } => self.auto_samon_ring(cx, pos, rot, param, n),
            Make::IceBreak { pos, scale } => self.ice_break(cx, t, pos, scale),
            Make::Dead { pos } => self.dead(pos),
        }
    }

    fn free(&mut self) -> Option<usize> {
        if self.slots.is_empty() {
            self.slots = vec![None; SLOTS];
        }
        self.slots.iter().position(Option::is_none)
    }

    /// Into the first free slot: the slot, or -1 (the effect is lost).
    fn put(&mut self, enabled: bool, eff: BossEff) -> i32 {
        match self.free() {
            Some(k) => {
                self.slots[k] = Some(Slot { enabled, eff });
                k as i32
            }
            None => -1,
        }
    }

    /// `m_bEnabled` of slot `id`.
    pub fn enabled(&self, id: i32) -> bool {
        usize::try_from(id).ok().and_then(|k| self.slots.get(k)).and_then(|s| s.as_ref()).is_some_and(|s| s.enabled)
    }

    /// The omni lights in the light group: (place, colour).
    pub fn lights(&self) -> Vec<(V4, [F; 4])> {
        self.slots
            .iter()
            .flatten()
            .filter_map(|s| match &s.eff {
                BossEff::Light(l) if l.in_group => Some((l.pos, l.colour)),
                _ => None,
            })
            .collect()
    }

    /// [`BossEffects::lights`] as the characters are lit by them:
    /// `ccLight(4, 1)` after `ccOmniLight::Init` (intensity 1, no
    /// fall-off), priority 1.
    pub fn omni_lights(&self) -> Vec<piney_world::town::Light> {
        let f = |v: u32| f32::from_bits(v);
        self.lights()
            .into_iter()
            .map(|(p, c)| piney_world::town::Light {
                kind: 4,
                pos: glam::Vec3::new(f(p[0]), f(p[1]), f(p[2])),
                dir: glam::Vec3::ZERO,
                colour: glam::Vec3::new(f(c[0]), f(c[1]), f(c[2])),
                intensity: 1.0,
                far_start: 0.0,
                far_end: 0.0,
                radius: [0.0; 2],
                priority: 1,
            })
            .collect()
    }

    /// `ccBossEffWaveShockCreate(pos, dirc, scale)` (0x00478820).
    pub fn wave_shock(&mut self, cx: &mut Cx, t: &Tables, pos: V4, _dirc: V4, scale: F) -> i32 {
        for se in [36, 68, 56] {
            cx.raise(Event::Sound3dNote { se, pos, note: 60 });
        }
        let w = WaveShock { pos, rot: VF0, scale, anm: drawelm::AnmObj::new(cx, "xeffect", "ANM_ex31lhit") };
        if let Some(mut g) = t.generator(cx, va::ERUPTION, [va::ERUPTION_FF, va::ERUPTION_FF + 0x20, 0, 0]) {
            g.pos = pos;
            g.offset = VF0;
            g.p_tex_mod = 158;
            cx.particles.start(g);
        }
        self.put(false, BossEff::Eruption);
        self.put(true, BossEff::WaveShock(w))
    }

    /// `ccBossEffMagicSquareCreate(pos, n)` (0x00478b10) for n 0: SE 228,
    /// the square's generators (rows 0, 1 and 7) at pos with distSW off,
    /// then `ccBossEffLightCreate(pos, 0, 80, 10)`; -1. (Other n are not
    /// Skeith's and not ported.)
    pub fn magic_square(&mut self, cx: &mut Cx, t: &Tables, pos: V4, n: i32) -> i32 {
        if n != 0 {
            return -1;
        }
        let ms = |row: u32| va::MAGIC_SQUARE + 0x38 * row;
        let ff = |k: u32| va::MAGIC_SQUARE_FF + 0x20 * k;
        cx.raise(Event::Sound3d { se: 228, pos });
        let s2 = t.generator(cx, ms(1), [ff(2), 0, 0, 0]);
        let s1 = t.generator(cx, ms(7), [ff(10), ff(1), 0, 0]);
        let s3 = t.generator(cx, ms(0), [ff(0), ff(12), 0, 0]);
        if let Some(mut g) = s3 {
            g.dist_sw = false;
            g.pos = pos;
            cx.particles.start(g);
        }
        for mut g in [s2, s1].into_iter().flatten() {
            g.dist_sw = false;
            g.pos = pos;
            cx.particles.start(g);
        }
        self.light(pos, n, 80, 10);
        -1
    }

    /// `ccBossEffLightCreate(pos, n, life, wait)` (0x004793d0).
    pub fn light(&mut self, pos: V4, ty: i32, life: i32, wait: i32) -> i32 {
        let mut offset = [0, 0x4348_0000, 0x4348_0000, ONE];
        if ty == 0 {
            offset[2] = ee::add(offset[2], 0x42c8_0000);
        }
        // ApplyMatrix(unit, offset) + base, four lanes.
        let p = ee::vadd(ee::apply(&vu::UNIT, offset), pos);
        let l = Light {
            pos: p,
            offset,
            base: pos,
            cam_rot: VF0,
            heading: 0,
            falling: false,
            hold: 0,
            life,
            intensity: 0,
            count: 0,
            wait,
            ty,
            colour: set_color(1),
            in_group: false,
        };
        self.put(true, BossEff::Light(l))
    }

    /// `ccBossEffForceGeneratorCreate(p, rot, speed, r0, r1, num, life,
    /// clt)` (0x004797e0): a `ccBossEffBrightMagicSquare` (0x0046dd90).
    #[allow(clippy::too_many_arguments)]
    pub fn force_generator(
        &mut self,
        cx: &mut Cx,
        t: &Tables,
        p: V4,
        rot: V4,
        speed: F,
        r0: F,
        r1: F,
        num: i32,
        life: i32,
        clt: i32,
    ) -> i32 {
        let clt_name = if clt == 3 { "CLT_x000".to_string() } else { format!("CLT_x000c{}", clt - 3) };
        let clut = cx.assets.find("particle", &clt_name);
        let key = self.free().unwrap_or(SLOTS) as u32;
        let mut photons = Vec::new();
        for k in 0..num.max(0) {
            let mut eff = particle_eff(cx, "EFF_x000");
            if let Some(e) = eff.as_mut() {
                e.clut = clut;
            }
            let mut off = VF0;
            off[0] = r0;
            let a = if k & 1 != 0 { PI } else { 0 };
            let m = vu::rot_z(&vu::UNIT, a);
            let off = ee::apply(&m, off);
            let pos = ee::vadd(p, off);
            let anchor = (key << 8) | k as u32;
            // The generator by rot.w: 1 (energyOutFF +0x20, +0x40), 0 (+0x20),
            // else (+0, +0x20).
            let ff = if ee::eq(rot[3], ONE) {
                [va::ENERGY_OUT_FF + 0x20, va::ENERGY_OUT_FF + 0x40, 0, 0]
            } else if ee::eq(rot[3], 0) {
                [va::ENERGY_OUT_FF + 0x20, 0, 0, 0]
            } else {
                [va::ENERGY_OUT_FF, va::ENERGY_OUT_FF + 0x20, 0, 0]
            };
            let generator = t.generator(cx, va::ENERGY_OUT, ff).map(|mut g| {
                g.sync_pos_type = false;
                g.sync_pos = Some(VecRef::Anchor(anchor));
                g
            });
            photons.push(Photon {
                pending: generator.map(Box::new),
                generator: None,
                eff,
                pos,
                centre: p,
                angle: a,
                wait: (k / 2) * 10,
                state: 0,
                radius: r0,
                life,
                anchor,
            });
        }
        let b = Bright { speed, r0, r1, life, photons };
        self.put(true, BossEff::Bright(b))
    }

    /// `ccBossEffAutoSamonRingCreate(pos, rot, param, n)` (0x00479900): the
    /// ring bursting from scale `param.x` (all three) by `param.y` a Draw.
    pub fn auto_samon_ring(&mut self, cx: &mut Cx, pos: V4, rot: V4, param: V4, n: i32) -> i32 {
        let s = param[0];
        let (clump, clut) = match samon_model(n) {
            Some((cmp, to, from)) => {
                let find = |name: &str| cx.assets.find("particle", name);
                (find(cmp), find(from).zip(find(to)).map(|(f, t)| (f.object, t.object)))
            }
            None => (None, None),
        };
        let r = SamonRing {
            pos,
            rot,
            scale: [s, s, s, ONE],
            burst: [0x3f00_0000; 4],
            bursting: true,
            transparency: ONE,
            step: 0x3d4c_cccd,
            clump,
            clut,
            ds: param[1],
        };
        self.put(true, BossEff::Ring(r))
    }

    /// `ccBossEffIceBreakCreate(pos, scale)` (0x00479240): its constructor
    /// (0x0046c950) draws `ccRand()` once and starts the smoke.
    pub fn ice_break(&mut self, cx: &mut Cx, t: &Tables, pos: V4, scale: F) -> i32 {
        let _ = cx.host.genrand();
        let clump = cx.assets.find("particle", "CMP_x201d");
        let mut smoke = None;
        if let Some(mut g) =
            t.generator(cx, va::ICE_SMOKE, [va::ICE_SMOKE_FF, va::ICE_SMOKE_FF + 0x20, va::ICE_SMOKE_FF + 0x40, 0])
        {
            let mut p = pos;
            p[2] = 0x4334_0000;
            g.pos = p;
            p[2] = ee::add(p[2], 0x42c8_0000);
            g.offset = p;
            smoke = Some(cx.particles.start(g));
        }
        let b = IceBreak { clump, count: 0, state: 0, scale, pos, transparency: 0, smoke };
        self.put(true, BossEff::IceBreak(b))
    }

    /// `ccBossEffDeadCreate(pos, pos2, n)` (0x004795d0).
    pub fn dead(&mut self, pos: V4) -> i32 {
        self.put(true, BossEff::Dead(Dead { generator: None, pos, stage: 1, count: 0 }))
    }

    /// `ccBossEffManager::Draw` (0x00461750): each slot's Draw in order, a
    /// disabled one deleted.
    pub fn draw(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables) {
        for k in 0..self.slots.len() {
            let Some(s) = self.slots[k].as_mut() else { continue };
            if !s.enabled {
                if let BossEff::Bright(b) = &s.eff {
                    for p in &b.photons {
                        ctrl.anchors.remove(&p.anchor);
                    }
                }
                self.slots[k] = None;
                continue;
            }
            let on = match &mut s.eff {
                BossEff::Eruption => false,
                BossEff::WaveShock(w) => w.draw(cx),
                BossEff::Light(l) => l.draw(cx),
                BossEff::Bright(b) => b.draw(ctrl, cx),
                BossEff::Ring(r) => r.draw(cx),
                BossEff::IceBreak(i) => i.draw(ctrl, cx),
                BossEff::Dead(d) => d.draw(cx, t),
            };
            if !on {
                s.enabled = false;
            }
        }
    }
}

/// `new ccEff`, `Init(GetChunkAdrsF("particle", name), 1)`: fog on.
fn particle_eff(cx: &Cx, name: &str) -> Option<Box<Eff>> {
    let o = cx.assets.find("particle", name)?;
    let (j, c) = cx.assets.eff_chunk(o)?;
    Some(Box::new(Eff::init(o.file, j, c, true, &cx.assets.alpha_blend)))
}

/// `ccSetColor(out, color, 1.0)` (main 0x00138b60) for a colour with no
/// alpha byte: its r, g, b over 255 (`ITOF0`, times 1/255), alpha 0.
fn set_color(color: u32) -> [F; 4] {
    const INV255: F = 0x3b80_8081;
    let b = |k: u32| ee::mul(ee::from_int(((color >> (8 * k)) & 0xff) as i32), INV255);
    [b(0), b(1), b(2), 0]
}

/// `fptoui` (0x00129d08) of a float: 0 below 1 (a negative too).
fn fptoui(v: F) -> u32 {
    let f = ee::f(v);
    if f.is_nan() || f < 1.0 { 0 } else { f as u32 }
}

/// An angle kept in (-pi, pi] as the effects wrap it: above pi less 2 pi,
/// below -pi plus 2 pi.
fn wrap(a: F) -> F {
    let a = if ee::le(a, PI) { a } else { ee::sub(a, TWO_PI) };
    if ee::lt(a, NEG_PI) { ee::add(a, TWO_PI) } else { a }
}

impl WaveShock {
    /// `ccBossEffWaveShock::Draw` (0x0046b3f0): a step of the animation,
    /// drawn at `PosRotZYXScale(pos, rot, 5 scale)`; false at its end.
    fn draw(&mut self, cx: &mut Cx) -> bool {
        let s = ee::mul(0x40a0_0000, self.scale);
        let Some(a) = self.anm.as_mut() else { return false };
        let ended = a.forward(cx);
        let m = vu::pos_rot_zyx_scale(self.pos, self.rot, [s, s, s, ONE]);
        a.draw(cx, m, ONE);
        !ended
    }
}

impl Light {
    /// `ccBossEffLight::Draw` (0x00466df0) for type 0; false once its life
    /// has run out.
    fn draw(&mut self, cx: &mut Cx) -> bool {
        if self.wait > 0 {
            self.wait -= 1;
            return self.life >= 0;
        }
        if self.wait == 0 {
            self.wait = -1;
            self.in_group = true;
        }
        self.life -= 1;
        if self.ty == 0 {
            self.cam_rot = cx.host.camera_rot(VF0);
            self.heading = self.cam_rot[2];
            self.count += 1;
            self.cam_rot[2] = wrap(self.cam_rot[2]);
            self.heading = wrap(self.heading);
            let m = vu::rot_z(&vu::rot_x(&vu::UNIT, NEG_HALF_PI), self.heading);
            self.pos = ee::vadd(ee::apply(&m, self.offset), self.base);
            if self.intensity >= 250 {
                self.falling = true;
            }
            self.intensity += if self.falling { -10 } else { 10 };
            if self.falling && self.hold < 15 {
                self.hold += 1;
                self.intensity = 255;
            }
            let c = u32::from(self.intensity >= 2);
            let v = ((self.intensity as u32) << 16) | (c << 8) | c;
            self.colour = set_color(fptoui(ee::from_int(v as i32)));
        }
        self.life >= 0
    }
}

impl Bright {
    /// `ccBossEffBrightMagicSquare::Draw` (0x0046e320): false once every
    /// photon is gone.
    fn draw(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) -> bool {
        let mut done = true;
        let span = ee::sub(self.r1, self.r0);
        for p in &mut self.photons {
            match p.state {
                0 => {
                    let w = p.wait;
                    p.wait -= 1;
                    if w <= 0 {
                        p.state = 1;
                        if let Some(g) = p.pending.take() {
                            p.generator = Some(cx.particles.start(*g));
                        }
                    }
                    done = false;
                }
                1 => {
                    done = false;
                    p.centre[2] = ee::sub(p.centre[2], self.speed);
                    let mut off = VF0;
                    off[0] = ee::mul(p.radius, ee::cosf(p.angle));
                    off[1] = ee::mul(p.radius, ee::sinf(p.angle));
                    p.pos = ee::vadd(p.centre, off);
                    let mut a = ee::add(p.angle, 0x3e49_0fdb);
                    if ee::lt(a, NEG_PI) {
                        a = ee::add(a, TWO_PI);
                    }
                    if !ee::le(a, PI) {
                        a = ee::sub(a, TWO_PI);
                    }
                    p.angle = a;
                    let mut r = ee::add(p.radius, ee::div(span, ee::from_int(self.life)));
                    if ee::lt(span, 0) {
                        if ee::lt(r, self.r1) {
                            r = self.r1;
                        }
                    } else if !ee::le(r, self.r1) {
                        r = self.r1;
                    }
                    p.radius = r;
                    let l = p.life;
                    p.life -= 1;
                    if l <= 0 {
                        if let Some(g) = p.generator.take()
                            && cx.particles.check(g)
                        {
                            cx.particles.particle_kill(g);
                        }
                        p.life = 0;
                        p.state = -1;
                    }
                    ctrl.anchors.insert(p.anchor, p.pos);
                    if let Some(e) = p.eff.as_mut() {
                        e.scale_x = 0x4120_0000;
                        e.scale_y = 0x4120_0000;
                        drawelm::draw_eff_at(cx, e, p.pos, 0);
                    }
                }
                _ => {}
            }
        }
        !done
    }
}

impl SamonRing {
    /// `ccBossEffAutoSamonRing::Draw` (0x0046ed70): the burst's step set,
    /// then `ccEffSamonRing::Draw` (0x00462660); false once the ring's
    /// transparency has fallen below 0.
    fn draw(&mut self, cx: &mut Cx) -> bool {
        self.burst = [self.ds, self.ds, self.ds, ONE];
        let mut alpha = ONE;
        if self.bursting {
            for k in 0..3 {
                self.scale[k] = ee::add(self.scale[k], self.burst[k]);
            }
            alpha = self.transparency;
            self.transparency = ee::sub(self.transparency, self.step);
        }
        let m = vu::pos_rot_xyz_scale(self.pos, self.rot, self.scale);
        let on = !ee::lt(self.transparency, 0);
        if let Some(obj) = self.clump {
            cx.draws.push(DrawRec::Clump { obj, matrix: m, alpha, layer: EFFECT_LAYER, clut: self.clut });
        }
        on
    }
}

impl IceBreak {
    /// `ccBossEffIceBreak::Draw` (0x0046cc10).
    fn draw(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) -> bool {
        const FULL: [F; 4] = [0, 0, 0x4400_0000, 0x43c0_0000];
        let mut on = true;
        match self.state {
            0 => {
                cx.raise(Event::Flash { time: 60, color: 0x80e0_ffff, rect: FULL });
                self.count = 0;
                self.state += 1;
            }
            1 => {
                self.transparency = ONE;
                let c = self.count;
                self.count += 1;
                if c == 60 {
                    if let Some(g) = self.smoke.take() {
                        cx.particles.particle_kill(g);
                    }
                    self.state += 1;
                    self.count = 0;
                }
            }
            2 => {
                let c = self.count;
                self.count += 1;
                if c >= 60 {
                    let mut p = self.pos;
                    self.state += 1;
                    cx.raise(Event::Flash { time: 30, color: 0x80e0_ffff, rect: FULL });
                    p[2] = ee::add(p[2], 0x42c8_0000);
                    for dir in [0, 0x3fc9_0fdb, NEG_PI, NEG_HALF_PI] {
                        let r = [0, 0, dir, 0];
                        debris::eff_ice_rock(ctrl, cx, p, r, 0x4248_0000, 0x3e4c_cccd, 10);
                        debris::eff_ice_rock(ctrl, cx, p, r, 0x41c8_0000, ONE, 5);
                        debris::eff_ice_rock(ctrl, cx, p, r, 0x4120_0000, 0x40a0_0000, 1);
                    }
                    on = false;
                }
            }
            _ => {}
        }
        if matches!(self.state, 1 | 2) {
            let s = self.scale;
            let m = vu::pos_rot_zyx_scale(self.pos, [0; 4], [s, s, s, ONE]);
            if let Some(obj) = self.clump {
                cx.draws.push(DrawRec::Clump {
                    obj,
                    matrix: m,
                    alpha: self.transparency,
                    layer: EFFECT_LAYER,
                    clut: None,
                });
            }
        }
        on
    }
}

impl Dead {
    /// `ccBossEffDead::Draw` (0x0046a440).
    fn draw(&mut self, cx: &mut Cx, t: &Tables) -> bool {
        let row = |k: u32| va::DEAD + 0x38 * k;
        let ff = |k: u32| va::DEAD_FF + 0x20 * k;
        let start = |cx: &mut Cx, param: u32, f: [u32; 4], dz: F, pos: &mut V4| -> Option<GenRef> {
            let g = t.generator(cx, param, f);
            pos[2] = ee::add(pos[2], dz);
            g.map(|mut g| {
                g.dist_sw = false;
                g.pos = *pos;
                cx.particles.start(g)
            })
        };
        match self.stage {
            1 => {
                self.stage += 1;
                start(cx, row(0), [ff(0), ff(1), 0, 0], 0x435c_0000, &mut self.pos);
                cx.raise(Event::SoundNote { se: 56, note: 56 });
            }
            2 => {
                let c = self.count;
                self.count += 1;
                if c == 35 {
                    self.stage += 1;
                    start(cx, row(1), [ff(6), 0, 0, 0], 0x41a0_0000, &mut self.pos);
                    self.count = 0;
                    cx.raise(Event::SoundNote { se: 104, note: 36 });
                }
            }
            3 => {
                let c = self.count;
                self.count += 1;
                if c == 60 {
                    self.stage += 1;
                    start(cx, row(1), [ff(3), ff(2), 0, 0], 0x41a0_0000, &mut self.pos);
                    self.count = 0;
                }
            }
            4 => {
                let c = self.count;
                self.count += 1;
                if c == 20 {
                    self.stage += 1;
                    self.generator = start(cx, row(2), [ff(4), ff(2), ff(5), 0], 0x41a0_0000, &mut self.pos);
                }
            }
            5 => {
                return self.generator.is_some_and(|g| cx.particles.check(g));
            }
            _ => {}
        }
        true
    }
}
