//! Corbenik's generators and one-shot effects (QUA gcmn boss08.cpp,
//! boss08_b.cpp, boss08_c.cpp, bosseff.cpp): each generator its code makes
//! (the row, the force fields, `distSW`, `pTexMod`), those it keeps by
//! their key ([`Kept`]: what `ccCheckParticleGenerator` answers for the
//! rules), the seeds' charges, the bursts, shields, smoke and radiates.
//! The rules own every step and `ccRand` (`piney_battle::boss::corbenik`).

use std::collections::{HashMap, HashSet};

use piney_battle::boss::corbenik::{Gen, Pic};

use crate::boss::{Tables, ff_row, gen_row, va};
use crate::draw::DrawRec;
use crate::ee::{self, F, ONE, V4};
use crate::eff::Eff;
use crate::effect::{EFFECT_LAYER, EffectCtrl};
use crate::particle::{GenRef, Generator};
use crate::{CharRef, Cx, debris, drawelm, dust, shield, vu};

const QUARTER: F = 0x3e80_0000;
const TENTH: F = 0x3dcc_cccd;
const HUNDRED: F = 0x42c8_0000;
/// `effSmoke`'s and `effRadiateSomething`'s sizes (8, 45).
const EIGHT: F = 0x4100_0000;
const FORTY_FIVE: F = 0x4234_0000;

/// `ccEff` +0x2c: the sprites' colour.
const SPRITE_COLOUR: u32 = 0x80e0_ffff;
const TEN: F = 0x4120_0000;

/// The generators the fight keeps, by the key the rules gave each, and
/// each seed's charge by its character; the sprites its effects made
/// (`Init(chunk, 1)` and their own settings), drawn as the rules say.
#[derive(Clone, Debug, Default)]
pub struct Kept {
    gens: HashMap<u32, GenRef>,
    charges: HashMap<CharRef, GenRef>,
    sprites: HashMap<Sprite, Box<Eff>>,
}

/// The effects' sprites, each `particle`'s `EFF_x000` or `EFF_x011`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Sprite {
    Flush,
    Star,
    Core,
    Spark,
}

/// `ccEff::Init(chunk, 1)` of the sprite, as its maker sets it up: the
/// flush on `CLT_x000c5`; the stars on `CLT_x000c7`, blend type 1, scale
/// 10 (`ccBossEffStarDustFlash` 0x00374b90) and its core on the same
/// CLUT; the sparks `EFF_x011` with `CLT_x011c2` for `CLT_x011`, PRIM's
/// fog off (`ccBossEffEnergyGrow2` 0x00377d40).
fn sprite(cx: &Cx, kind: Sprite) -> Option<Box<Eff>> {
    let (eff, clut) = match kind {
        Sprite::Flush => ("EFF_x000", "CLT_x000c5"),
        Sprite::Star | Sprite::Core => ("EFF_x000", "CLT_x000c7"),
        Sprite::Spark => ("EFF_x011", "CLT_x011c2"),
    };
    let mut e = crate::boss::particle_eff(cx, eff)?;
    e.clut = cx.assets.find("particle", clut);
    e.color = SPRITE_COLOUR;
    match kind {
        Sprite::Star => {
            (e.scale_x, e.scale_y) = (TEN, TEN);
            e.set_blend_type(1, &cx.assets.alpha_blend);
        }
        Sprite::Core => (e.scale_x, e.scale_y) = (0, 0),
        Sprite::Spark => e.prim &= !0x20,
        Sprite::Flush => {}
    }
    Some(e)
}

/// `ccEff::Draw(pos, pat)` of the kept sprite as `set` leaves it, on the
/// effect layer.
fn draw_sprite(cx: &mut Cx, kept: &mut Kept, kind: Sprite, pos: V4, pat: u16, set: impl FnOnce(&mut Eff)) {
    let e = match kept.sprites.entry(kind) {
        std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
        std::collections::hash_map::Entry::Vacant(v) => {
            let Some(e) = sprite(cx, kind) else { return };
            v.insert(e)
        }
    };
    set(e);
    e.pos[..3].copy_from_slice(&pos[..3]);
    cx.draws.push(DrawRec::Eff { eff: e.clone(), pat, layer: EFFECT_LAYER });
}

impl Kept {
    /// The keys whose generator the particle system still runs
    /// (`ccCheckParticleGenerator`); those it ended are let go.
    pub fn alive(&mut self, particles: &crate::particle::Particles) -> HashSet<u32> {
        self.gens.retain(|_, r| particles.check(*r));
        self.gens.keys().copied().collect()
    }
}

/// How the code makes one generator: its table and row, the force fields'
/// rows, whether it clears `distSW`, its `pTexMod`.
struct Spec {
    gens: u32,
    row: u32,
    ffs: u32,
    ff: &'static [u32],
    dist_off: bool,
    tex: Option<i16>,
}

const fn spec(gens: u32, row: u32, ffs: u32, ff: &'static [u32], dist_off: bool, tex: Option<i16>) -> Spec {
    Spec { gens, row, ffs, ff, dist_off, tex }
}

/// The rows each start takes (docs/engine/boss-corbenik.md "In the
/// field"): the wave's two `MagicSquareGenerator`s, the thrown seeds'
/// smoke, the root drain's four, the transform, the eye's birth, the
/// death's three, the laser's charge, the tera laser's charge, glow and
/// fall.
fn spec_of(which: Gen) -> Spec {
    match which {
        Gen::WaveRing => spec(va::MAGIC_SQUARE, 7, va::MAGIC_SQUARE_FF, &[10, 1], true, None),
        Gen::WaveSquare => spec(va::MAGIC_SQUARE, 0, va::MAGIC_SQUARE_FF, &[0, 12], true, None),
        Gen::SeedShot => spec(va::SEED_SHOT, 0, va::SEED_SHOT_FF, &[0, 1], true, None),
        Gen::RootDrain0 => spec(va::ROOT_DRAIN, 0, va::ROOT_DRAIN_FF, &[0, 1], true, None),
        Gen::RootDrain1a => spec(va::ROOT_DRAIN, 1, va::ROOT_DRAIN_FF, &[6], true, None),
        Gen::RootDrain1b => spec(va::ROOT_DRAIN, 1, va::ROOT_DRAIN_FF, &[3, 2], true, None),
        Gen::RootDrain2 => spec(va::ROOT_DRAIN, 2, va::ROOT_DRAIN_FF, &[4, 2, 5], true, None),
        Gen::Transform => spec(va::TRANSFORM, 0, va::TRANSFORM_FF, &[0], false, None),
        Gen::GenerateEye => spec(va::GENERATE_EYE, 0, va::GENERATE_EYE_FF, &[0], false, None),
        Gen::BossDead0 => spec(va::DEAD, 0, va::DEAD_FF, &[0, 1], true, Some(105)),
        Gen::BossDead1a => spec(va::DEAD, 1, va::DEAD_FF, &[6], true, Some(31)),
        Gen::BossDead1b => spec(va::DEAD, 1, va::DEAD_FF, &[3, 2], true, Some(31)),
        Gen::LaserCharge => spec(va::CHARGE, 2, va::CHARGE_FF, &[2, 5], true, None),
        Gen::TeraCharge => spec(va::CHARGE, 4, va::CHARGE_FF, &[5, 7], false, Some(108)),
        Gen::TeraChargeGlow => spec(va::TERA_CHARGE, 0, va::TERA_CHARGE_FF, &[0], false, None),
        Gen::TeraFall => spec(va::TERA_FALL, 0, va::TERA_FALL_FF, &[0], false, None),
    }
}

/// `new ccParticleGenerator(row, ff...)` of `which` at `pos` as its maker
/// sets it.
fn make(cx: &mut Cx, t: &Tables, which: Gen, pos: V4) -> Option<Generator> {
    let s = spec_of(which);
    let ff = std::array::from_fn(|k| s.ff.get(k).map_or(0, |&f| ff_row(s.ffs, f)));
    let mut g = t.generator(cx, gen_row(s.gens, s.row), ff)?;
    if s.dist_off {
        g.dist_sw = false;
    }
    if let Some(tex) = s.tex {
        g.p_tex_mod = tex;
    }
    g.pos = pos;
    Some(g)
}

const NEG_HALF_PI: F = 0xbfc9_0fdb;

/// `ccClump::SetTransparency(alpha)`, `ccClump::Draw` at `m` on the effect
/// layer.
fn clump(cx: &mut Cx, file: &str, name: &str, matrix: vu::M4, alpha: F, clut: Option<(u32, u32)>) {
    if let Some(obj) = cx.assets.find(file, name) {
        cx.draws.push(DrawRec::Clump { obj, matrix, alpha, layer: EFFECT_LAYER, clut, fog: true });
    }
}

/// The pitch and heading from `a` to `b` (`atan2f(a.z - b.z,
/// ccGetDist(a, b))`, `ccGetDirc(a, b)`).
fn way(a: V4, b: V4) -> (F, F) {
    let (dx, dy) = (ee::sub(b[0], a[0]), ee::sub(b[1], a[1]));
    let d = ee::sqrt(ee::add(ee::mul(dx, dx), ee::mul(dy, dy)));
    (ee::atan2f(ee::sub(a[2], b[2]), d), piney_battle::enemy_ai::get_dirc(a, b))
}

/// A laser head's clump matrix (`_Init`'s +0x2c0, moved to the head).
fn head_clump(pos: V4, pitch: F, yaw: F) -> vu::M4 {
    let mut s = vu::UNIT;
    (s[0][0], s[1][1], s[2][2]) = (0x4000_0000, 0x4000_0000, 0x4040_0000);
    let s = vu::rot_x(&s, NEG_HALF_PI);
    let r = vu::rot_x(&vu::UNIT, pitch);
    let r = vu::rot_z(&r, yaw);
    let mut m = piney_battle::geom::mul_matrix(&r, &s);
    m[3] = [pos[0], pos[1], pos[2], ONE];
    m
}

/// `ccBossEffThunder3::Draw`'s segments: `CMP_x012` `width` across and
/// `ccGetDist3D / 900` long, turned -pi/2 about z, then about y by the
/// pitch (held to (-pi, pi]), z by the heading and -pi/2 again, from each
/// point, at `alpha`.
fn thunder(cx: &mut Cx, pts: &[V4], alpha: F, width: F) {
    const PI: F = 0x4049_0fdb;
    const TWO_PI: F = 0x40c9_0fdb;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dy, dz) = (ee::sub(b[0], a[0]), ee::sub(b[1], a[1]), ee::sub(b[2], a[2]));
        let d = ee::sqrt(ee::add(ee::mul(dx, dx), ee::mul(dy, dy)));
        let d3 = ee::sqrt(ee::add(ee::mul(d, d), ee::mul(dz, dz)));
        let mut pt = ee::atan2f(ee::sub(a[2], b[2]), d);
        if ee::lt(pt, ee::sub(0, PI)) {
            pt = ee::add(pt, TWO_PI);
        }
        if !ee::le(pt, PI) {
            pt = ee::sub(pt, TWO_PI);
        }
        let mut m = vu::UNIT;
        (m[0][0], m[1][1], m[2][2]) = (width, ee::mul(0x3a91_a2b4, d3), width);
        let m = vu::rot_z(&m, NEG_HALF_PI);
        let m = vu::rot_y(&m, pt);
        let m = vu::rot_z(&m, piney_battle::enemy_ai::get_dirc(a, b));
        let mut m = vu::rot_z(&m, NEG_HALF_PI);
        m[3] = [a[0], a[1], a[2], ONE];
        clump(cx, "particle", "CMP_x012", m, alpha, None);
    }
}

/// What a picture of Corbenik's asks of the effects. `char_of` names a
/// scene character as the effects know it.
pub fn calls(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    t: &Tables,
    kept: &mut Kept,
    char_of: &dyn Fn(usize) -> CharRef,
    p: &Pic,
) {
    match *p {
        Pic::Particles { which, pos } => {
            if let Some(g) = make(cx, t, which, pos) {
                cx.particles.start(g);
            }
        }
        Pic::Gen { key, which, pos } => {
            if let Some(g) = make(cx, t, which, pos) {
                let r = cx.particles.start(g);
                kept.gens.insert(key, r);
            }
        }
        Pic::GenMove { key, pos } => {
            if let Some(g) = kept.gens.get(&key).and_then(|&r| cx.particles.get_mut(r)) {
                g.pos = pos;
            }
        }
        Pic::GenKill { key } => {
            if let Some(&r) = kept.gens.get(&key) {
                cx.particles.particle_kill(r);
            }
        }
        // A seed's charge: `g_energyChargeGenerator` row 0 (ff 1, 4),
        // `distSW` off, at its place; deleted at `Order(5)`.
        Pic::Charge { seed, pos, on } => {
            let c = char_of(seed);
            if on {
                let ff = [ff_row(va::CHARGE_FF, 1), ff_row(va::CHARGE_FF, 4), 0, 0];
                if let Some(mut g) = t.generator(cx, gen_row(va::CHARGE, 0), ff) {
                    g.dist_sw = false;
                    g.pos = pos;
                    let r = cx.particles.start(g);
                    kept.charges.insert(c, r);
                }
            } else if let Some(r) = kept.charges.remove(&c) {
                cx.particles.particle_delete(r);
            }
        }
        // `effExplode3(pos, spd, scale, kind)`, then `SetScaleAnm(grow)`.
        Pic::Burst { pos, spd, scale, kind, grow } => {
            if let Some(k) = drawelm::eff_explode3(cx, pos, spd, scale, kind)
                && let Some(g) = grow
                && let Some(b) = cx.spells.elements.base_mut(k)
            {
                b.anim.set_scale_anm_f(g[0], g[1], g[2]);
            }
        }
        // `effResistantShield(seed, 1, -1)`, its scale's x a quarter, its
        // offset 100 down.
        Pic::SeedShield { seed } => {
            if let Some(k) = shield::eff_resistant_shield(cx, char_of(seed), 1, -1)
                && let Some(b) = cx.spells.elements.base_mut(k)
            {
                b.anim.scale[0] = ee::mul(b.anim.scale[0], QUARTER);
                b.anim.offset[2] = ee::sub(b.anim.offset[2], HUNDRED);
            }
        }
        // `effResistantShield(eye, magic, -1)`, its offset's height 0.
        Pic::KillerShield { eye, magic } => {
            if let Some(k) = shield::eff_resistant_shield(cx, char_of(eye), magic, -1)
                && let Some(b) = cx.spells.elements.base_mut(k)
            {
                b.anim.offset[2] = 0;
            }
        }
        // `effSmoke(pos, vec * 0.1, 8, 60, 114, 512, 32)`.
        Pic::ElementSmoke { pos, vec } => {
            let v = vec.map(|x| ee::mul(x, TENTH));
            dust::eff_smoke(cx, pos, v, EIGHT, 60, 114, 512, 32);
        }
        Pic::Smoke { pos, rot } => {
            dust::eff_smoke(cx, pos, rot, EIGHT, 60, 114, 512, 32);
        }
        // `effRadiateSomething(pos, (0, -pi/2, 0), 45, 4, 4)`.
        Pic::Radiate { pos } => {
            let r = [0, ee::deg2rad(-16384), 0, ONE];
            debris::eff_radiate_something(ctrl, cx, pos, r, FORTY_FIVE, 4, 4);
        }
        // `effRadiateSomething2(pos, (0, -pi/2, turn), 45, 4, 4)`.
        Pic::LaserRadiate { pos, turn } => {
            let r = [0, ee::deg2rad(-16384), turn, ONE];
            debris::eff_radiate_something2(ctrl, cx, pos, r, FORTY_FIVE, 4, 4);
        }
        Pic::Explode2 { pos } => crate::boss::explode2(cx, t, pos),
        // `ccBossEffThunder3::Draw` (0x00375f10): `CMP_x012` from each
        // drawn point to the next.
        Pic::Thunder { ref pts, alpha, width, .. } => thunder(cx, pts, alpha, width),
        // `ccBossEffTeraLaser::Draw` (0x00378d80): the beam scaled, turned
        // -pi/2 about x then x, y, z by its turn, at its place.
        Pic::TeraBeam { pos, rot, scale } => {
            let mut m = vu::UNIT;
            (m[0][0], m[1][1], m[2][2]) = (scale[0], scale[1], scale[2]);
            let m = vu::rot_x(&m, NEG_HALF_PI);
            let m = vu::rot_x(&m, rot[0]);
            let m = vu::rot_y(&m, rot[1]);
            let m = vu::rot_z(&m, rot[2]);
            let mut m = m;
            m[3] = [pos[0], pos[1], pos[2], pos[3]];
            clump(cx, "x81", "CMP_ex8efc3", m, ONE, None);
        }
        // `ccBossEffTransform::Draw` (0x00373060): a lay at (0.5, 2, 0.5),
        // turned, on `CLT_ex8efc2c1` for `CLT_ex8efc2`.
        Pic::TransformLay { pos, rot, alpha } => {
            let m = vu::pos_rot_zyx_scale(pos, rot, [0x3f00_0000, 0x4000_0000, 0x3f00_0000, ONE]);
            let clut = cx.assets.find("x81", "CLT_ex8efc2").zip(cx.assets.find("x81", "CLT_ex8efc2c1"));
            clump(cx, "x81", "CMP_ex8efc2", m, alpha, clut.map(|(f, t)| (f.object, t.object)));
        }
        // A laser's head's clump: (2, 2, 3) turned -pi/2 about x, then
        // the head's turn, at its place.
        Pic::LaserHead { pos, pitch, yaw, .. } => {
            clump(cx, "xeffect", "CMP_ex31les3", head_clump(pos, pitch, yaw), ONE, None)
        }
        Pic::RefrectHead { from, to, .. } => {
            let (pitch, yaw) = way(from, to);
            clump(cx, "xeffect", "CMP_ex31les3", head_clump(from, pitch, yaw), ONE, None);
        }
        // `ccBossEffFlush::Draw` (0x00376ea0): its size each way, no
        // depth test, the colour set again.
        Pic::Flush { pos, size } => draw_sprite(cx, kept, Sprite::Flush, pos, 0, |e| {
            (e.scale_x, e.scale_y) = (size, size);
            e.test &= !(1 << 16);
            e.color = SPRITE_COLOUR;
        }),
        // The star dust's `_Generate` (0x00375070): each star at its
        // transparency and size; its core at its size.
        Pic::Star { pos, scale, size } => draw_sprite(cx, kept, Sprite::Star, pos, 0, |e| {
            e.transparency = scale;
            (e.scale_x, e.scale_y) = (size, size);
        }),
        Pic::StarCore { pos, scale } => draw_sprite(cx, kept, Sprite::Core, pos, 0, |e| {
            (e.scale_x, e.scale_y) = (scale, scale);
        }),
        // `ccBossEffEnergyGrow2::Draw` (0x00377f60): a spark turned, at 10,
        // its transparency, pattern its frame.
        Pic::Spark { pos, frame, alpha, turn } => draw_sprite(cx, kept, Sprite::Spark, pos, frame as u16, |e| {
            e.rotate = turn;
            (e.scale_x, e.scale_y) = (TEN, TEN);
            e.transparency = alpha;
        }),
        // Drawn from the rules' state each frame (the field's or the
        // boss effects' pass).
        _ => {}
    }
}
