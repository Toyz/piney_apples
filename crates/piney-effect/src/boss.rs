//! The boss's effects (gcmn bosseff.cpp): `ccBossEffManager`'s effects as
//! Skeith and Fidchell make them. `ccBossEffManager::Draw` (0x00461750, the
//! task `ccThBossEffect`, priority 66) runs its 1024 slots in order on the
//! effect layer, deleting a disabled one. Skeith's six: WaveShock 0x00478820,
//! MagicSquare 0x00478b10 (with Light 0x00466a30), ForceGenerator 0x004797e0,
//! AutoSamonRing 0x00479900, IceBreak 0x00479240 and Dead 0x004795d0, in
//! docs/engine/boss.md ("The effects' pictures"); Fidchell's spells are
//! [`crate::fidchell`]. A photon's position is published as an anchor for
//! the generator that follows it.

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

/// The rows of [`va`], and the volume (its square root and its float to
/// unsigned conversion differ).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tables {
    gens: HashMap<u32, GenParam>,
    ffs: HashMap<u32, FfParam>,
    pub volume: Volume,
}

impl Tables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> Tables {
        let t = piney_data::tables::effect::of(volume);
        let mut out = Tables { volume, ..Tables::default() };
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
    /// One of Fidchell's spells ([`crate::fidchell`]).
    Spell(Box<crate::fidchell::Spell>),
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
    /// +0x10 its place, +0x50 its offset (turned), +0x70 the base; and
    /// where its `ccOmniLight` stands (+0x9c's matrix), which mode 1 moves
    /// one `Draw` in three.
    pub pos: V4,
    pub offset: V4,
    pub base: V4,
    pub light_pos: V4,
    /// +0x80 the camera's rotation as read, +0x98 the heading the offset
    /// is turned by.
    pub cam_rot: V4,
    pub heading: F,
    /// +0x90 falling, +0x94 the Draws held at 255, +0xa8 life, +0xac the
    /// intensity (mode 1's count), +0xb0 a count, +0xb4 the wait, +0xb8
    /// the type (`Mode`).
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
    WaveShock {
        pos: V4,
        dirc: V4,
        scale: F,
    },
    MagicSquare {
        pos: V4,
        n: i32,
    },
    ForceGenerator {
        p: V4,
        rot: V4,
        speed: F,
        r0: F,
        r1: F,
        num: i32,
        life: i32,
        clt: i32,
    },
    AutoSamonRing {
        pos: V4,
        rot: V4,
        param: V4,
        n: i32,
    },
    IceBreak {
        pos: V4,
        scale: F,
    },
    Dead {
        pos: V4,
    },
    /// Fidchell's spells run here by their rules ([`crate::fidchell`]):
    /// `ccBossEffMeteoSwormCreate(sp, ep, n, radius, v, &camView)`,
    /// `ccBossEffThunderStormCreate(pos, radius, n)`,
    /// `ccBossEffRockTowerCreate(pos, n)`.
    MeteoSworm {
        sp: V4,
        ep: V4,
        n: i32,
        radius: F,
        v: F,
    },
    ThunderStorm {
        pos: V4,
        radius: F,
        n: i32,
    },
    RockTower {
        pos: V4,
        n: i32,
    },
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
            Make::MeteoSworm { .. } | Make::ThunderStorm { .. } | Make::RockTower { .. } => {
                let s = crate::fidchell::Spell::create(cx, t, make);
                s.map_or(-1, |s| self.put(true, BossEff::Spell(Box::new(s))))
            }
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

    /// The omni lights in the light group: (place, colour, fall-off's
    /// end): the magic squares' (`ccOmniLight::Init`'s: none) and a
    /// meteor swarm's (500).
    pub fn lights(&self) -> Vec<(V4, [F; 4], F)> {
        self.slots
            .iter()
            .flatten()
            .filter_map(|s| match &s.eff {
                BossEff::Light(l) if l.in_group => Some((l.light_pos, l.colour, 0)),
                BossEff::Spell(x) => x.light(),
                _ => None,
            })
            .collect()
    }

    /// [`BossEffects::lights`] as the characters are lit by them:
    /// `ccLight(4, 1)` after `ccOmniLight::Init` (intensity 1), priority 1.
    pub fn omni_lights(&self) -> Vec<piney_world::town::Light> {
        let f = |v: u32| f32::from_bits(v);
        self.lights()
            .into_iter()
            .map(|(p, c, far)| piney_world::town::Light {
                kind: 4,
                pos: glam::Vec3::new(f(p[0]), f(p[1]), f(p[2])),
                dir: glam::Vec3::ZERO,
                colour: glam::Vec3::new(f(c[0]), f(c[1]), f(c[2])),
                intensity: 1.0,
                far_start: 0.0,
                far_end: f(far),
                radius: [0.0; 2],
                priority: 1,
            })
            .collect()
    }

    /// Fidchell's spells as the fight's rules left them this frame (their
    /// slot there and state), drawn: each one's picture made at its first
    /// sight and dropped once its slot is gone.
    pub fn sync_spells(&mut self, cx: &mut Cx, spells: &[(i32, &piney_battle::boss::fidchell::eff::Fx)]) {
        for s in self.slots.iter_mut() {
            if let Some(Slot { eff: BossEff::Spell(x), .. }) = s
                && x.rules_slot.is_some_and(|k| !spells.iter().any(|(id, f)| *id == k && x.fits(f)))
            {
                *s = None;
            }
        }
        for &(id, fx) in spells {
            let at = self
                .slots
                .iter()
                .position(|s| matches!(s, Some(Slot { eff: BossEff::Spell(x), .. }) if x.rules_slot == Some(id)));
            let k = match at {
                Some(k) => k,
                None => {
                    let s = crate::fidchell::Spell::of(cx, fx.clone(), Some(id));
                    match usize::try_from(self.put(true, BossEff::Spell(Box::new(s)))) {
                        Ok(k) => k,
                        Err(_) => continue,
                    }
                }
            };
            if let Some(Slot { eff: BossEff::Spell(x), .. }) = self.slots[k].as_mut() {
                x.fx = fx.clone();
                x.draw(cx);
            }
        }
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

    /// `ccBossEffMagicSquareCreate(pos, n)` (OUT gcmn 0x00489820): by `n`
    /// a sound, the square's generators at `pos` with distSW off, then
    /// `ccBossEffLightCreate(pos, n, life, 10)`; -1. Skeith's 0: SE 228,
    /// rows 0, 1 and 7, life 80. Fidchell's thunders' 1: SE 68 in fields
    /// 4-8 (else 41 at note 57), row 2 at 350 up, life 60; its meteors' 2:
    /// SE 35 at note 48, rows 6 and 5, life 80. Any other n: nothing.
    pub fn magic_square(&mut self, cx: &mut Cx, t: &Tables, pos: V4, n: i32) -> i32 {
        let ms = |row: u32| va::MAGIC_SQUARE + 0x38 * row;
        let ff = |k: u32| va::MAGIC_SQUARE_FF + 0x20 * k;
        // Made in the game's order, started in the listed one.
        let mut at = pos;
        let (gens, life) = match n {
            0 => {
                cx.raise(Event::Sound3d { se: 228, pos });
                let s1 = t.generator(cx, ms(1), [ff(2), 0, 0, 0]);
                let s2 = t.generator(cx, ms(7), [ff(10), ff(1), 0, 0]);
                let s0 = t.generator(cx, ms(0), [ff(0), ff(12), 0, 0]);
                ([s0, s1, s2], 80)
            }
            1 => {
                if (4..9).contains(&cx.host.game_field()) {
                    cx.raise(Event::Sound3d { se: 68, pos });
                } else {
                    cx.raise(Event::Sound3dNote { se: 41, pos, note: 57 });
                }
                let s2 = t.generator(cx, ms(2), [ff(3), ff(4), 0, 0]);
                at[2] = 0x43af_0000;
                ([None, None, s2], 60)
            }
            2 => {
                cx.raise(Event::Sound3dNote { se: 35, pos, note: 48 });
                let s1 = t.generator(cx, ms(6), [ff(8), ff(7), 0, 0]);
                let s2 = t.generator(cx, ms(5), [ff(6), ff(10), ff(9), 0]);
                ([None, s1, s2], 80)
            }
            _ => return -1,
        };
        for mut g in gens.into_iter().flatten() {
            g.dist_sw = false;
            g.pos = at;
            cx.particles.start(g);
        }
        self.light(pos, n, life, 10);
        -1
    }

    /// `ccBossEffLightCreate(pos, n, life, wait)` (0x004793d0): its
    /// `ccOmniLight` at its place, `ccSetColor(1, 1.0)`.
    pub fn light(&mut self, pos: V4, ty: i32, life: i32, wait: i32) -> i32 {
        let mut offset = [0, 0x4348_0000, 0x4348_0000, ONE];
        if ty == 0 {
            offset[2] = ee::add(offset[2], 0x42c8_0000);
        }
        // ApplyMatrix(unit, offset) + base, four lanes.
        let p = ee::vadd(ee::apply(&vu::UNIT, offset), pos);
        let mut colour = [0; 4];
        set_color(&mut colour, 1, ONE);
        let l = Light {
            pos: p,
            offset,
            base: pos,
            light_pos: p,
            cam_rot: VF0,
            heading: 0,
            falling: false,
            hold: 0,
            life,
            intensity: 0,
            count: 0,
            wait,
            ty,
            colour,
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
            let mut square = None;
            let on = match &mut s.eff {
                BossEff::Eruption => false,
                BossEff::WaveShock(w) => w.draw(cx),
                BossEff::Light(l) => l.draw(cx, t),
                BossEff::Bright(b) => b.draw(ctrl, cx),
                BossEff::Ring(r) => r.draw(cx),
                BossEff::IceBreak(i) => i.draw(ctrl, cx),
                BossEff::Dead(d) => d.draw(cx, t),
                // The fight's spells are drawn as its rules leave them
                // ([`BossEffects::sync_spells`]).
                BossEff::Spell(x) if x.rules_slot.is_some() => continue,
                BossEff::Spell(x) => {
                    let (on, sq) = x.run(ctrl, cx, t);
                    square = sq;
                    on
                }
            };
            if !on {
                s.enabled = false;
            }
            // A storm's first bolt gone: its magic square, which this pass
            // reaches when its slot comes later.
            if let Some(p) = square {
                self.magic_square(cx, t, p, 1);
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

/// `ccSetColor(out, color, s)` (OUT main 0x00136f90). With no alpha byte
/// its r, g, b over 255 (`ITOF0`, times 1/255, or s/255), w 0. With one,
/// an HSV colour: hue the low byte (sixths of 256), saturation and value
/// the next two over 255 (value times s), turned to r, g, b by the hue's
/// sector; w kept.
pub(crate) fn set_color(out: &mut [F; 4], color: u32, s: F) {
    const INV255: F = 0x3b80_8081;
    let byte = |k: u32| ee::from_int(((color >> (8 * k)) & 0xff) as i32);
    if color >> 24 == 0 {
        let k = if ee::eq(ONE, s) { INV255 } else { ee::div(s, 0x437f_0000) };
        *out = [ee::mul(byte(0), k), ee::mul(byte(1), k), ee::mul(byte(2), k), 0];
        return;
    }
    let h6 = (color & 0xff) as i32 * 6;
    let scale = ee::div(s, 0x437f_0000);
    let frac = ee::mul(ee::from_int(h6 & 0xff), 0x3b80_0000);
    let v = ee::mul(byte(2), scale);
    let sat = ee::mul(byte(1), INV255);
    let sf = ee::mul(sat, frac);
    let p = ee::mul(v, ee::sub(ONE, sat));
    let q = ee::mul(v, ee::sub(ONE, sf));
    let t = ee::mul(v, ee::sub(ee::add(ONE, sf), sat));
    let rgb = match h6 >> 8 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        5 => [v, p, q],
        _ => return,
    };
    out[..3].copy_from_slice(&rgb);
}

/// A float to the unsigned colour `ccBossEffLight::Draw` hands on:
/// Infection's and Mutation's `fptoui` (0x00129d08: 0 below 1, a negative
/// too); Outbreak's and Quarantine's inline `cvt.w.s`, which keeps a
/// negative as its two's complement.
fn to_uint(v: F, volume: Volume) -> u32 {
    match volume {
        Volume::Inf | Volume::Mut => fptoui(v),
        Volume::Out | Volume::Qua => {
            if ee::le(0x4f00_0000, v) {
                ee::to_int(ee::sub(v, 0x4f00_0000)) as u32 | 0x8000_0000
            } else {
                ee::to_int(v) as u32
            }
        }
    }
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
    /// `ccBossEffLight::Draw` (OUT gcmn 0x00478260) by mode: 0 (Skeith's
    /// square) swings over the base against the camera's heading and
    /// pulses blue; 1 (the thunders') faces the camera, flickers between
    /// grey and orange and moves its light one `Draw` in three (a
    /// `ccRandF(0.5)` drawn and lost); 2 (the meteors') swings as 0, its
    /// turn falling behind after 15, and pulses red. False once its life
    /// has run out.
    fn draw(&mut self, cx: &mut Cx, t: &Tables) -> bool {
        const NEG_TILT: F = 0xbfc5_0a6b;
        if self.wait > 0 {
            self.wait -= 1;
            return self.life >= 0;
        }
        if self.wait == 0 {
            self.wait = -1;
            self.in_group = true;
        }
        self.life -= 1;
        match self.ty {
            0 => {
                self.cam_rot = cx.host.camera_rot(VF0);
                self.heading = self.cam_rot[2];
                self.count += 1;
                self.cam_rot[2] = wrap(self.cam_rot[2]);
                self.heading = wrap(self.heading);
                let m = vu::rot_z(&vu::rot_x(&vu::UNIT, NEG_HALF_PI), self.heading);
                self.pos = ee::vadd(ee::apply(&m, self.offset), self.base);
                self.pulse();
                let c = u32::from(self.intensity >= 2);
                let v = ((self.intensity as u32) << 16) | (c << 8) | c;
                set_color(&mut self.colour, to_uint(ee::from_int(v as i32), t.volume), ONE);
                self.light_pos = self.pos;
            }
            1 => {
                self.cam_rot = cx.host.camera_rot(VF0);
                self.cam_rot[2] = wrap(self.cam_rot[2]);
                // RotMatrixX of the unit matrix, then RotMatrixZ of the unit
                // matrix again: the tilt is lost.
                let _ = crate::thunder::rand_f(cx, 0x3f00_0000);
                let m = vu::rot_z(&vu::UNIT, self.cam_rot[2]);
                self.intensity += 1;
                self.pos = ee::vadd(ee::apply(&m, self.offset), self.base);
                if self.intensity % 3 == 0 {
                    set_color(&mut self.colour, 0x0055_5555, ONE);
                    self.light_pos = self.pos;
                } else {
                    set_color(&mut self.colour, 0x00ff_ab44, ONE);
                }
            }
            2 => {
                self.cam_rot = cx.host.camera_rot(VF0);
                self.count += 1;
                if self.count < 15 {
                    self.heading = self.cam_rot[2];
                } else {
                    self.heading = ee::sub(self.heading, 0x3f19_999a);
                    self.count = 15;
                }
                self.cam_rot[2] = wrap(self.cam_rot[2]);
                self.heading = wrap(self.heading);
                let m = vu::rot_z(&vu::rot_x(&vu::UNIT, NEG_TILT), self.heading);
                self.pos = ee::vadd(ee::apply(&m, self.offset), self.base);
                self.pulse();
                let c = to_uint(ee::from_int(self.intensity), t.volume);
                set_color(&mut self.colour, c, ONE);
                self.light_pos = self.pos;
            }
            _ => {}
        }
        self.life >= 0
    }

    /// Modes 0 and 2: up 10 a `Draw` to 250, held at 255 for 15, then
    /// down 10 a `Draw`.
    fn pulse(&mut self) {
        if self.intensity >= 250 {
            self.falling = true;
        }
        self.intensity += if self.falling { -10 } else { 10 };
        if self.falling && self.hold < 15 {
            self.hold += 1;
            self.intensity = 255;
        }
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
