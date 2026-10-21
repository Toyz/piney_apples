//! Magus's pictures (MUT gcmn boss03.cpp, bosseff.cpp): the leaves'
//! generators as the fight starts, kills and pauses them ([`start`],
//! [`gen_op`]); the needles (`ccBossEffNeedle` 0x00481180, `Draw`
//! 0x004814d0), drawn from the rules' state; a leaf's burst
//! (`ccBoss03LeafExplode::Draw` 0x004a7100); a laser shock where its beam
//! meets the ground (`ccBoss03LaserShock::Draw` 0x004a7630) and its burst.
//! docs/engine/boss-magus.md, "The pictures".

use piney_battle::boss::GenOp;
use piney_battle::boss::magus::leaf::Explode;
use piney_battle::boss::magus::{Gen, Held, Needle as Life, Place};
use piney_battle::geom;
use piney_battle::rand::Rng;

use crate::draw::DrawRec;
use crate::ee::{self, F, V4, VF0};
use crate::eff::Eff;
use crate::effect::{EFFECT_LAYER, EffectCtrl};
use crate::files::ObjRef;
use crate::particle::{GenRef, Generator};
use crate::{CharRef, Cx, VecRef, debris};

const HALF: F = 0x3f00_0000;
/// The leaves' bursts and the shocks are drawn in the boss's task before
/// `ccBoss::Draw`, on the layer the effect manager's pass left active,
/// `charLayer` (5, priority 10).
const CHAR_LAYER: i16 = 10;
const NEG_HALF_PI: F = 0xbfc9_0fdb;

/// The anchors a leaf's marker publishes (`ccLeafMark` +0xb0), by the
/// leaf's character.
const MARKER: u32 = 0x5000_0000;

/// The anchor of leaf `c`'s marker.
pub fn marker_anchor(c: usize) -> u32 {
    MARKER | c as u32
}

/// The generators the leaves keep.
pub type Kept = std::collections::HashMap<Held, GenRef>;

/// `new ccParticleGenerator(row, ff...)` of `which` as its maker leaves
/// it: distSW, a forced `gSync`, `pTexMod`.
fn make(cx: &mut Cx, t: &crate::boss::Tables, which: Gen) -> Option<Generator> {
    use crate::boss::va;
    let r = which.rows();
    let (gens, ffs) = match which {
        Gen::Marker(_) => (va::LEAF_MARKER, va::LEAF_MARKER_FF),
        Gen::Charge => (va::CHARGE, va::CHARGE_FF),
        Gen::LeafDead => (va::LEAF_DEAD, va::LEAF_DEAD_FF),
        Gen::Thunder => (va::THUNDER, va::THUNDER_FF),
    };
    let ff = std::array::from_fn(|k| r.ff.get(k).map_or(0, |&f| crate::boss::ff_row(ffs, u32::from(f))));
    let mut g = t.generator(cx, crate::boss::gen_row(gens, u32::from(r.row)), ff)?;
    g.dist_sw = r.dist_sw;
    if let Some(s) = r.g_sync {
        g.g_sync = s;
    }
    g.p_tex_mod = r.tex;
    Some(g)
}

/// One of Magus's starts (`Out::Magus(Pic::Particles)`).
pub fn start(cx: &mut Cx, t: &crate::boss::Tables, kept: &mut Kept, which: Gen, at: Place, held: Option<Held>) {
    let Some(mut g) = make(cx, t, which) else { return };
    match at {
        Place::At(p) => g.pos = p,
        Place::Marker(c) => {
            g.sync_pos_type = false;
            g.sync_pos = Some(VecRef::Anchor(marker_anchor(c)));
        }
    }
    let r = cx.particles.start(g);
    if let Some(h) = held {
        kept.insert(h, r);
    }
}

/// What a leaf does to a generator it keeps.
pub fn gen_op(cx: &mut Cx, kept: &mut Kept, held: Held, op: GenOp) {
    match op {
        GenOp::Kill => {
            if let Some(&r) = kept.get(&held) {
                cx.particles.particle_kill(r);
            }
        }
        GenOp::Pause(on) => {
            if let Some(g) = kept.get(&held).and_then(|&r| cx.particles.get_mut(r)) {
                g.pause = on;
            }
        }
        GenOp::Forget => {
            kept.remove(&held);
        }
    }
}

/// `ccRandF`'s source for needles a manager makes alone.
struct Gen32<'a>(&'a mut dyn crate::Host);

impl Rng for Gen32<'_> {
    fn rand(&mut self) -> i32 {
        self.0.genrand() as i32
    }
}

/// `ccBossEffNeedle` in a manager slot: each needle xeffect's
/// `OBJ_ex71body2`.
#[derive(Clone, Debug)]
pub struct Needles {
    pub rules_slot: i32,
    obj: Option<ObjRef>,
    /// The rules' `Draw`s drawn.
    drawn: u32,
    own: Option<Box<Life>>,
}

impl Needles {
    pub fn new(cx: &Cx, rules_slot: i32) -> Needles {
        Needles { rules_slot, obj: cx.assets.find("xeffect", "OBJ_ex71body2"), drawn: 0, own: None }
    }

    /// `Create(pos, scale, speed, alpha, n, t0, t1, t2)` run by this
    /// manager (the probe's).
    #[allow(clippy::too_many_arguments)]
    pub fn alone(cx: &mut Cx, pos: V4, scale: F, speed: F, alpha: F, n: i32, t0: i32, t1: i32) -> Needles {
        let life = Life::new(&mut Gen32(&mut *cx.host), pos, scale, speed, alpha, n, t0, t1);
        Needles { own: Some(Box::new(life)), ..Needles::new(cx, -1) }
    }

    pub fn runs_alone(&self) -> bool {
        self.own.is_some()
    }

    /// The manager's `Draw` of needles it runs itself: false at their end.
    pub fn run(&mut self, cx: &mut Cx) -> bool {
        let Some(mut n) = self.own.take() else { return false };
        if let Some(se) = n.draw() {
            cx.raise(crate::Event::Sound3d { se, pos: n.pos });
        }
        self.draw(cx, &n);
        let alive = !n.done;
        self.own = Some(n);
        alive
    }

    /// The pictures of the `Draw` that left `n` (once a `Draw`): each
    /// needle scaled (0.5, 0.5, scale / 200, at least 0), stood up by
    /// -pi/2 about x and turned by its own x, y, z, at the place, at the
    /// transparency the `Draw` began with. (Its fog blend, 75, is not in
    /// the draw records.)
    pub fn draw(&mut self, cx: &mut Cx, n: &Life) {
        if self.drawn == n.draws {
            return;
        }
        self.drawn = n.draws;
        let Some(obj) = self.obj else { return };
        let (alpha, scale) = n.shown;
        let mut s = ee::div(scale, 0x4348_0000);
        if ee::lt(s, 0) {
            s = 0;
        }
        for r in &n.rots {
            let mut m = geom::unit_matrix();
            m[0][0] = HALF;
            m[1][1] = HALF;
            m[2][2] = s;
            let m = geom::rot_matrix_x(&m, NEG_HALF_PI);
            let m = geom::rot_matrix_z(&geom::rot_matrix_y(&geom::rot_matrix_x(&m, r[0]), r[1]), r[2]);
            let matrix = geom::trans_matrix(&m, n.pos);
            cx.draws.push(DrawRec::Clump { obj, matrix, alpha, layer: EFFECT_LAYER, clut: None });
        }
    }
}

/// xeffect's `EFF_` chunk `name` as `ccEffObj::Init` makes it (fog on).
fn xeffect_eff(cx: &Cx, name: &str) -> Option<Box<Eff>> {
    let o = cx.assets.find("xeffect", name)?;
    let (j, c) = cx.assets.eff_chunk(o)?;
    Some(Box::new(Eff::init(o.file, j, c, true, &cx.assets.alpha_blend)))
}

/// A leaf's burst as `ccBoss03LeafExplode::Draw` left it (`ex`, after its
/// step): `EFF_ex31exp2` 100 over the place at its scale and
/// transparency, then `OBJ_ex31exp1` there, scaled by the second, at its
/// transparency.
pub fn leaf_explode(cx: &mut Cx, ex: &Explode) {
    if let Some(mut e) = xeffect_eff(cx, "EFF_ex31exp2") {
        let mut at = ex.pos;
        at[2] = ee::add(at[2], 0x42c8_0000);
        e.scale_x = ex.eff.scale;
        e.scale_y = ex.eff.scale;
        e.transparency = ex.eff.alpha;
        draw_eff(cx, &mut e, at);
    }
    if let Some(obj) = cx.assets.find("xeffect", "OBJ_ex31exp1") {
        let mut s = geom::unit_matrix();
        let k = ex.exp.scale;
        s[0][0] = k;
        s[1][1] = k;
        s[2][2] = k;
        // sceVu0MulMatrix(m, local, m): the place's matrix through the
        // scale.
        let matrix = geom::mul_matrix(&geom::trans_matrix(&geom::unit_matrix(), ex.pos), &s);
        cx.draws.push(DrawRec::Clump { obj, matrix, alpha: ex.exp.alpha, layer: CHAR_LAYER, clut: None });
    }
}

/// `ccEff::Draw(pos, 0)` on the boss's layer.
fn draw_eff(cx: &mut Cx, e: &mut Eff, at: V4) {
    e.pos[0] = at[0];
    e.pos[1] = at[1];
    e.pos[2] = at[2];
    cx.draws.push(DrawRec::Eff { eff: Box::new(e.clone()), pat: 0, layer: CHAR_LAYER });
}

/// A laser shock whose beam met the ground at `at`: `EFF_ex31les2` there,
/// scale 6, at its transparency.
pub fn laser_shock(cx: &mut Cx, at: V4, transparency: F) {
    let Some(mut e) = xeffect_eff(cx, "EFF_ex31les2") else { return };
    e.scale_x = 0x40c0_0000;
    e.scale_y = 0x40c0_0000;
    e.transparency = transparency;
    draw_eff(cx, &mut e, at);
}

/// A shock's burst at `at`: `ccParticleExplode(at, (0, 0, 10), 5, 1)` and
/// with 15, then `effSmokeRock(at, (-pi/2, 0, turn), 45, 2, 2, -1)`.
pub fn shock_burst(ctrl: &mut EffectCtrl, cx: &mut Cx, at: V4, turn: F) {
    for z in [0x4120_0000, 0x4170_0000] {
        let mut speed = VF0;
        speed[2] = z;
        crate::particle::cc_particle_explode(cx, at, speed, 0x40a0_0000, 1);
    }
    let mut rot = VF0;
    rot[0] = ee::deg2rad(-16384);
    rot[2] = turn;
    debris::eff_smoke_rock(ctrl, cx, at, rot, 0x4234_0000, 2, -1);
}

/// A leaf's character as the effects name it.
pub fn leaf_ref(c: usize) -> CharRef {
    c as CharRef
}
