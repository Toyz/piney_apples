//! The spells' particle calls: every `ccParticleGenerator` the spell
//! effects start (and `startParticleEffect`) goes through here.

use crate::particle::{self, FfParam, GenParam, GenRef, Generator};
use crate::{CharRef, Cx};

/// `new ccParticleGenerator(&particleGeneratorTbl[row], ff1, ff2, ff3,
/// ff4)` (a `None` force field: the row's own), the fields `set` changes,
/// then `startParticleGenerator`.
pub fn start_ff(cx: &mut Cx, row: usize, ff: [Option<FfParam>; 4], set: impl FnOnce(&mut Generator)) {
    let param = cx.assets.particle.generators[row];
    let mut g = Generator::new(Some(param), ff, &cx.assets.particle.force_fields, &mut cx.particles.generator_serial);
    set(&mut g);
    cx.particles.start(g);
}

/// `new ccParticleGenerator(param, ff1, ff2, ff3, ff4)` of another table's
/// row (GCMN.PRG's `effElementGeneratorTbl`), set, started.
pub fn start_param(cx: &mut Cx, param: GenParam, ff: [Option<FfParam>; 4], set: impl FnOnce(&mut Generator)) -> GenRef {
    let mut g = Generator::new(Some(param), ff, &cx.assets.particle.force_fields, &mut cx.particles.generator_serial);
    set(&mut g);
    cx.particles.start(g)
}

/// `new ccParticleGenerator(&particleGeneratorTbl[row], 0, 0, 0, 0)`, the
/// fields `set` changes, then `startParticleGenerator`.
pub fn start(cx: &mut Cx, row: usize, set: impl FnOnce(&mut Generator)) {
    let mut g = cx.particles.generator(cx.assets, row);
    set(&mut g);
    cx.particles.start(g);
}

/// `startParticleEffect(ch, n)` (main 0x001c2560).
pub fn start_particle_effect(cx: &mut Cx, ch: CharRef, n: i32) {
    particle::start_particle_effect(cx, ch, n as usize);
}
