//! The particle system (main particle.cpp): `ccThParticle` (0x001bc2f0,
//! priority 98), `ccParticleCtrl` (245 animation rows, 2000 particles),
//! `ccParticleGenerator`, `ccParticle` and `ccParticleForceField`, and the
//! starters the rest of the game calls. Each frame runs every generator in
//! the order started, then every live particle slot in order. The game keeps
//! pointers to generators to kill them later; here a generator is named by
//! its serial number ([`GenRef`]) and what it follows by [`VecRef`] /
//! [`IntRef`], read through the [`Host`]. See docs/engine/particles.md.

mod condition;
pub mod dmath;
pub mod force;
mod generator;
pub mod one;
mod starters;
pub mod tables;

pub use condition::{Ability, ConditionEffect, delete_condition_effect, kill_condition_effect, set_condition_effect};
pub use generator::Generator;
pub use one::{Obj, Particle};
pub use starters::*;
pub use tables::{EffectParam, FfParam, GenParam, PartParam, Tables};

use crate::draw::DrawRec;
use crate::ee::{self, ONE, V4};
use crate::effect::EffectCtrl;
use crate::files::Assets;
use crate::{Host, IntRef, VecRef};

/// `particles[2000]` (main 0x00388060).
pub const PARTICLES: usize = 2000;
/// `particlesStr` (main 0x003ddf60): the stream demo's second system's.
pub const PARTICLES_STR: usize = 150;

/// A generator, by its serial number (`sn`, from `generatorSerialNum`).
pub type GenRef = u32;

/// `ccParticleCtrl` (0x18 bytes) and particle.cpp's statics.
#[derive(Clone, Debug, PartialEq)]
pub struct Particles {
    /// The list `gTop`..`gTail` (`next` +0x30), in the order started.
    pub gens: Vec<Generator>,
    /// `pWork`: `particles[2000]`.
    pub slots: Vec<Particle>,
    /// +0x0c `gNum`, +0x0e `pNum`, +0x14 `particleSearch`.
    pub g_num: i16,
    pub p_num: i16,
    pub search: u32,
    /// `generatorSerialNum` (0x00378aa8), `particleSerialNum`
    /// (0x00378aac), `p2Num` (0x00378ab4): not reset by
    /// `InitParticleCtrl`.
    pub generator_serial: u32,
    pub particle_serial: u32,
    pub p2_num: u16,
    /// `strEffectStopFlag` (0x00378ab8), the stream demo's.
    pub str_effect_stop: bool,
    /// Generators made but never started, which particles may still name:
    /// effect.cpp's static stand-ins ([`Particles::install_statics`]).
    pub detached: Vec<Generator>,
    /// Generators deleted while particles still name them: the game's
    /// particles go on reading the freed generator (its force fields, kill
    /// flag, position), which is kept here as it was until none do.
    pub dead: Vec<Generator>,
    /// `ccpgSmoke`, `ccpgPawSmoke`, `hitPhotonDummyG`, `hitPhotonDummyStrG`
    /// (main 0x003fefd0, 0x003ff0e0, 0x003ff1f0, 0x003ff300) once
    /// installed.
    pub statics: [Option<GenRef>; 4],
    /// The generators started since the last [`Particles::main`], as
    /// started (a record for tests and probes).
    pub started: Vec<Generator>,
    /// The stream demo's second system (`particleSystemStr`, its particles
    /// run `MainStr`).
    pub stream: bool,
}

impl Particles {
    /// `ccThParticle`'s second system for the stream demo
    /// (`particleSystemStr`, `particleThreadStrFlag`): 150 particles, each
    /// run by `ccParticle::MainStr`.
    pub fn stream() -> Self {
        Particles { slots: vec![Particle::default(); PARTICLES_STR], stream: true, ..Particles::default() }
    }
}

impl Default for Particles {
    /// `new ccParticleCtrl(0)` over a zeroed `particles[]`.
    fn default() -> Self {
        Particles {
            gens: Vec::new(),
            slots: vec![Particle::default(); PARTICLES],
            g_num: 0,
            p_num: 0,
            search: 0,
            generator_serial: 0,
            particle_serial: 0,
            p2_num: 0,
            str_effect_stop: false,
            detached: Vec::new(),
            dead: Vec::new(),
            statics: [None; 4],
            started: Vec::new(),
            stream: false,
        }
    }
}

/// One frame's inputs to `ccParticleCtrl::Main`.
pub struct Step<'a> {
    pub host: &'a mut dyn Host,
    pub assets: &'a Assets,
    /// The effects, for generators following one.
    pub effects: &'a EffectCtrl,
    /// Where the draws go, in the order sent.
    pub draws: &'a mut Vec<DrawRec>,
}

/// The vector `r` names, now.
pub fn resolve(host: &dyn Host, effects: &EffectCtrl, r: VecRef) -> V4 {
    match r {
        VecRef::CharPos(c) => host.char_pos(c),
        VecRef::CharDirc(c) => host.char_dirc(c),
        VecRef::CharAt(c, off) => host.char_vec(c, off),
        VecRef::EffectPos(k) => effects.effects[k].pos,
        VecRef::EffectRot(k) => effects.effects[k].rot,
        VecRef::EffectPosT(k) => effects.effects[k].pos_t,
        VecRef::Anchor(k) => effects.anchors.get(&k).copied().unwrap_or([0, 0, 0, ONE]),
    }
}

impl Step<'_> {
    fn resolve(&self, r: VecRef) -> V4 {
        resolve(&*self.host, self.effects, r)
    }
}

/// `normal2Angle(out, v)` (main 0x00155210): the turns that point +y along
/// `v`: x -atan2(v.z, v.x^2 + v.y^2) (the sum not rooted, as the game has
/// it), y 0, z atan2(v.x, -v.y), w 1.
pub fn normal2angle(v: V4) -> V4 {
    let d = ee::dot([v[0], v[1], 0, ONE], [v[0], v[1], 0, ONE]);
    let a = ee::atan2f(v[2], d);
    let b = ee::atan2f(v[0], ee::mul(0xbf80_0000, v[1]));
    [ee::neg(a), 0, b, ONE]
}

impl Particles {
    /// `ccParticleCtrl::InitParticleCtrl(0)` (main 0x001c0e10) again: no
    /// generators (they are dropped, as the game leaks them), every slot
    /// free (the rest of each slot kept), the counts 0.
    pub fn init_ctrl(&mut self) {
        self.gens.clear();
        for p in &mut self.slots {
            p.tex_id = -1;
        }
        self.g_num = 0;
        self.p_num = 0;
        self.search = 0;
    }

    /// effect.cpp's static generators, never started, that its particles
    /// name for their force fields and fades: `__sinit_effect.cpp`
    /// (0x00375830) makes `ccpgSmoke`, `ccpgPawSmoke`, `hitPhotonDummyG` and
    /// `hitPhotonDummyStrG` with no row (the first four serial numbers of
    /// the game), and `ccEffectCtrl`'s constructor gives them their force
    /// fields: `ccpffpSmoke1-3`, `ccpffpSmoke1, 2, 4`, `hitPhotonDummyF1/F2`
    /// (`ccEffectCtrl(0)`), and `hitPhotonDummyF1/F2` again while the
    /// streams' `ccEffectCtrl(1)` exists (the stream system).
    pub fn install_statics(&mut self, assets: &Assets) {
        let t = &assets.particle;
        let ffs: [[Option<FfParam>; 4]; 4] = [
            [Some(t.smoke_ff[0]), Some(t.smoke_ff[1]), Some(t.smoke_ff[2]), None],
            [Some(t.smoke_ff[0]), Some(t.smoke_ff[1]), Some(t.smoke_ff[3]), None],
            [Some(t.hit_photon_ff[0]), Some(t.hit_photon_ff[1]), None, None],
            // hitPhotonDummyStrG: ccEffectCtrl(1)'s constructor gives it
            // the same two (0x001c3514-0x001c355c) and its destructor takes
            // them off, so it has them only beside the streams' control.
            if self.stream { [Some(t.hit_photon_ff[0]), Some(t.hit_photon_ff[1]), None, None] } else { [None; 4] },
        ];
        for (k, ff) in ffs.into_iter().enumerate() {
            let mut g = Generator::new(None, [None; 4], &t.force_fields, &mut self.generator_serial);
            g.ff = ff;
            self.statics[k] = Some(self.detach(g));
        }
    }

    /// Keeps `g` without starting it, for particles to name.
    pub fn detach(&mut self, g: Generator) -> GenRef {
        let sn = g.sn;
        self.detached.push(g);
        sn
    }

    /// Where the generator a particle names is: on the list (0), detached
    /// (1) or deleted (2).
    pub(crate) fn gene_at(&self, r: GenRef) -> Option<(u8, usize)> {
        let find = |v: &[Generator]| v.iter().position(|g| g.sn == r);
        find(&self.gens)
            .map(|k| (0, k))
            .or_else(|| find(&self.detached).map(|k| (1, k)))
            .or_else(|| find(&self.dead).map(|k| (2, k)))
    }

    pub(crate) fn gene(&self, at: (u8, usize)) -> &Generator {
        match at.0 {
            0 => &self.gens[at.1],
            1 => &self.detached[at.1],
            _ => &self.dead[at.1],
        }
    }

    fn gene_mut(&mut self, at: (u8, usize)) -> &mut Generator {
        match at.0 {
            0 => &mut self.gens[at.1],
            1 => &mut self.detached[at.1],
            _ => &mut self.dead[at.1],
        }
    }

    /// `new ccParticleGenerator(&particleGeneratorTbl[row], 0, 0, 0, 0)`:
    /// set it up, then [`Particles::start`] it.
    pub fn generator(&mut self, assets: &Assets, row: usize) -> Generator {
        self.generator_from(assets, assets.particle.generators[row])
    }

    /// `new ccParticleGenerator(param, 0, 0, 0, 0)` of any row (another
    /// table's, read with [`GenParam::read`]).
    pub fn generator_from(&mut self, assets: &Assets, param: GenParam) -> Generator {
        Generator::new(Some(param), [None; 4], &assets.particle.force_fields, &mut self.generator_serial)
    }

    /// `startParticleGenerator(g)` (main 0x001c2920) =
    /// `ccParticleCtrlAddGenerator` (0x001c2a20): onto the list's tail.
    pub fn start(&mut self, g: Generator) -> GenRef {
        let sn = g.sn;
        self.started.push(g.clone());
        self.gens.push(g);
        self.g_num = self.g_num.wrapping_add(1);
        sn
    }

    /// The generator named `r`, while it is on the list.
    pub fn get(&self, r: GenRef) -> Option<&Generator> {
        self.gens.iter().find(|g| g.sn == r)
    }

    pub fn get_mut(&mut self, r: GenRef) -> Option<&mut Generator> {
        self.gens.iter_mut().find(|g| g.sn == r)
    }

    /// `ccCheckParticleGenerator(g)` (main 0x001c2b40): is it on the list?
    pub fn check(&self, r: GenRef) -> bool {
        self.get(r).is_some()
    }

    /// `ccParticleGenerator::ParticleKill()` (main 0x001bf070): killFlag 2
    /// (its particles fade out, it ends when they have) while it is on the
    /// list.
    pub fn particle_kill(&mut self, r: GenRef) {
        if let Some(g) = self.get_mut(r) {
            g.kill_flag = 2;
        }
    }

    /// `ccParticleGenerator::ParticleDelete()` (0x001bf0b0): killFlag 3
    /// (its particles end at once).
    pub fn particle_delete(&mut self, r: GenRef) {
        if let Some(g) = self.get_mut(r) {
            g.kill_flag = 3;
        }
    }

    /// `ccParticleCtrl::GenerateParticle()` (main 0x001c10d0): the first
    /// free slot, from the front and from the back by turns.
    pub fn generate_particle(&mut self) -> Option<usize> {
        self.search = u32::from(self.search == 0);
        if self.search != 0 {
            self.slots.iter().position(|p| p.tex_id == -1)
        } else {
            self.slots.iter().rposition(|p| p.tex_id == -1)
        }
    }

    /// `ccParticleCtrlParticleCountUp()` (0x001c2ab0).
    pub fn count_up(&mut self) {
        self.p_num = self.p_num.wrapping_add(1);
    }

    /// `ccParticleCtrl::Main()` (main 0x001c1230): one frame of
    /// `ccThParticle`.
    pub fn main(&mut self, step: &mut Step) {
        self.started.clear();
        let mut i = 0;
        while i < self.gens.len() {
            self.generator_main(i, step);
            if self.gens[i].end_flag {
                // DelGenerator (0x001c11b0), then the force fields and the
                // generator deleted.
                let g = self.gens.remove(i);
                self.g_num = self.g_num.wrapping_sub(1);
                if self.slots.iter().any(|p| p.tex_id != -1 && p.gene == Some(g.sn)) {
                    self.dead.push(g);
                }
            } else {
                i += 1;
            }
        }
        for s in 0..self.slots.len() {
            if self.slots[s].tex_id == -1 {
                continue;
            }
            let gene = self.slots[s].gene.and_then(|r| self.gene_at(r));
            let layer = self.slots[s].layer;
            let mut view = one::View { host: &mut *step.host, assets: step.assets, draws: &mut *step.draws };
            let mut p = std::mem::take(&mut self.slots[s]);
            if self.stream {
                one::main_str(&mut p, gene.map(|at| self.gene(at)), &mut view, layer);
            } else {
                one::main(&mut p, gene.map(|at| self.gene(at)), &mut view, layer);
            }
            self.slots[s] = p;
            if self.slots[s].tex_id == -1 {
                if let Some(at) = gene {
                    let g = self.gene_mut(at);
                    g.p_cnt = g.p_cnt.wrapping_sub(1);
                }
                self.p_num = self.p_num.wrapping_sub(1);
            }
        }
        let slots = &self.slots;
        self.dead.retain(|g| slots.iter().any(|p| p.tex_id != -1 && p.gene == Some(g.sn)));
    }

    /// The live particles, in slot order.
    pub fn live(&self) -> impl Iterator<Item = (usize, &Particle)> {
        self.slots.iter().enumerate().filter(|(_, p)| p.tex_id != -1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Camera;
    use crate::ee::k;
    use crate::host::{CharState, Simple};
    use crate::testing;

    /// The tables as the executable has them, `particleCcsAdrs` resolved
    /// in the effect files, and `Setup`'s texture switch decoded.
    #[test]
    fn the_particle_tables() {
        let Some(fx) = testing::effects() else { return };
        let t = &fx.assets.particle;
        assert_eq!((t.generators.len(), t.force_fields.len(), t.particles.len(), t.effects.len()), (244, 148, 245, 49));
        // The arrival's row: a ring of sprites rising about the character.
        let g = t.generators[82];
        assert_eq!((g.row, g.g_type, g.g_sync, g.p_sync), (82, 0, true, true));
        // Every object resolves: 101 objects and 144 CLUTs, two in DRAIN.CCS.
        assert!(t.adrs.iter().all(Option::is_some));
        assert_eq!(fx.assets.name(t.adrs(0).unwrap()), "EFF_x000");
        assert_eq!(fx.assets.files[t.adrs(48).unwrap().file].stem.to_ascii_lowercase(), "drain");
        // Ids 91-244: their own row below 101, then rows 0-84 with a CLUT.
        assert_eq!((t.tex_first, t.tex_map.len()), (91, 154));
        assert_eq!(t.tex_base(91), (91, 0));
        assert_eq!(t.tex_base(101), (0, 101));
        assert_eq!(t.tex_base(182), (38, 182));
        assert_eq!(t.tex_base(235), (57, 143));
        assert_eq!(t.tex_base(244), (84, 243));
        assert_eq!(t.tex_base(3), (3, 0));
        assert_eq!(t.polyhedron[0], [0, 0, 0xbfb5_04f7, ONE]);
    }

    /// The arrival's generator on a character: sprites come, rise and fade;
    /// killed, it and they end.
    #[test]
    fn a_generator_runs_and_ends() {
        let Some(mut fx) = testing::effects() else { return };
        let mut seed = 1u64;
        let mut rand = move || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 32) & 0x7fff_ffff) as i32
        };
        let pos = [0, k(5600.0), k(600.0), ONE];
        let cam = Camera {
            eye: [0, k(6100.0), k(780.0), ONE],
            cam_pos: [0, k(6100.0), k(780.0), ONE],
            cam_view: [0, k(5600.0), k(700.0), ONE],
            ..Default::default()
        };
        let mut host = Simple::new(&mut rand, pos, cam);
        host.chars.push(CharState { id: 7, pos, dirc: [0; 4], height: k(160.0), width: k(45.0) });
        let sn = {
            let (_, cx) = fx.split(&mut host);
            let mut g = cx.particles.generator(cx.assets, 82);
            g.sync_pos = Some(VecRef::CharPos(7));
            g.offset = [0, 0, k(180.0), 0];
            cx.particles.start(g)
        };
        let mut most = 0;
        for f in 0..400 {
            if f == 60 {
                fx.particles.particle_kill(sn);
            }
            fx.step_particles(&mut host);
            let n = fx.particles.live().count();
            most = most.max(n);
            assert_eq!(fx.particle_draws().len(), n, "every sprite in view is drawn");
            if fx.particles.gens.is_empty() && n == 0 {
                assert!(f > 60);
                break;
            }
        }
        assert!(most > 10, "{most}");
        assert!(fx.particles.gens.is_empty());
        assert_eq!(fx.particles.p_num, 0);
    }
}
