//! The pictures of Fidchell's spells (OUT gcmn bosseff.cpp): the meteors
//! (`ccBossEffMeteo2::Draw` 0x0047dd80) and their light, the thunders
//! (`ccBossEffThunder2::Draw` 0x0047e9a0), the rock towers
//! (`ccBossEffRockTower::Draw` 0x004800f0). The rules
//! (`piney_battle::boss::fidchell::eff`) own each `Draw`'s steps and every
//! `ccRand`; a picture is drawn from what the last one left, after the
//! boss's task in the fight ([`crate::boss::BossEffects::sync_spells`]),
//! or in the manager's own run of the rules for the probe ([`Spell::run`]).

use piney_battle::boss::Out;
use piney_battle::boss::fidchell::Pic;
use piney_battle::boss::fidchell::eff::{self as rules, Fx};
use piney_battle::rand::Rng;
use piney_data::volume::Volume;

use crate::boss::{Make, Tables};
use crate::draw::DrawRec;
use crate::drawelm::AnmObj;
use crate::ee::{self, F, ONE, V4};
use crate::eff::Eff;
use crate::effect::{EFFECT_LAYER, EffectCtrl};
use crate::files::ObjRef;
use crate::{Cx, Event, debris, dust, fall, particle, space, vu};

const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
const THREE: F = 0x4040_0000;
/// `m_rotLight`'s step, pi/30.
const GLOW_TURN: F = 0x3dd6_7750;
/// The meteors' light: `ccSetColor(0x80ffffff, 1.0)`, full to its end.
const METEOR_LIGHT: u32 = 0x80ff_ffff;
const METEOR_LIGHT_FAR: F = 0x43fa_0000;

/// One of Fidchell's spells in a manager slot: the rules' state, the
/// rules' slot it is drawn for (None: the manager runs it), its pictures.
#[derive(Clone, Debug)]
pub struct Spell {
    pub fx: Fx,
    pub rules_slot: Option<i32>,
    pics: Pics,
}

/// What a spell draws with.
#[derive(Clone, Debug)]
enum Pics {
    /// Each meteor's fire (`m_anmFire`, particle's `ANM_x300`), its glow
    /// (`m_cmpLight`, `CMP_x100`) and the glow's turn (`m_rotLight`).
    Meteo(Vec<(Option<AnmObj>, Option<ObjRef>, F)>),
    /// Each bolt's segment (`CMP_x012`, `ChangeClut(@4154[0] "CLT_x012",
    /// 0)`: its own CLUT again) and sprite (`EFF_x011`, `CLT_x011c2`).
    Storm(Vec<(Option<ObjRef>, Option<Box<Eff>>)>),
    /// The towers' clump: `CMP_x101%c` of "abcd"[1].
    Tower(Option<ObjRef>),
}

impl Spell {
    /// A spell of `fx`, its pictures as its constructor makes them.
    pub fn of(cx: &Cx, fx: Fx, rules_slot: Option<i32>) -> Spell {
        let pics = match &fx {
            Fx::Meteo(m) => Pics::Meteo(
                m.meteors
                    .iter()
                    .map(|_| (AnmObj::new(cx, "particle", "ANM_x300"), cx.assets.find("particle", "CMP_x100"), 0))
                    .collect(),
            ),
            Fx::Storm(s) => Pics::Storm(
                s.bolts
                    .iter()
                    .map(|_| {
                        let mut e = particle_eff(cx, "EFF_x011");
                        if let Some(e) = e.as_mut() {
                            e.clut = cx.assets.find("particle", "CLT_x011c2");
                        }
                        (cx.assets.find("particle", "CMP_x012"), e)
                    })
                    .collect(),
            ),
            Fx::Tower(_) => Pics::Tower(cx.assets.find("particle", "CMP_x101b")),
        };
        Spell { fx, rules_slot, pics }
    }

    /// The Create `make` names, run here: its rules' constructor on this
    /// manager's generators.
    pub fn create(cx: &mut Cx, t: &Tables, make: Make) -> Option<Spell> {
        let mut run = Run::new(cx, t.volume);
        let fx = match make {
            Make::MeteoSworm { sp, ep, n, radius, v } => {
                Fx::Meteo(rules::MeteoSworm::new(&mut run, sp, ep, n.max(0) as usize, radius, v))
            }
            Make::ThunderStorm { pos, radius, n } => {
                Fx::Storm(rules::ThunderStorm::new(&mut run, pos, radius, n.max(0) as usize))
            }
            Make::RockTower { pos, n } => Fx::Tower(rules::RockTower::new(&mut run, pos, n.max(0) as usize)),
            _ => return None,
        };
        let outs = run.outs;
        let s = Spell::of(cx, fx, None);
        for o in &outs {
            out_here(None, cx, o);
        }
        Some(s)
    }

    /// Whether `fx` is this spell as a later `Draw` left it: of its kind,
    /// with as many meteors, bolts or towers.
    pub fn fits(&self, fx: &Fx) -> bool {
        match (&self.fx, fx) {
            (Fx::Meteo(a), Fx::Meteo(b)) => a.meteors.len() == b.meteors.len(),
            (Fx::Storm(a), Fx::Storm(b)) => a.bolts.len() == b.bolts.len(),
            (Fx::Tower(a), Fx::Tower(b)) => a.towers.len() == b.towers.len(),
            _ => false,
        }
    }

    /// The swarm's omni light while it is in the light group: at the last
    /// meteor, its colour and its fall-off's end.
    pub fn light(&self) -> Option<(V4, [F; 4], F)> {
        let Fx::Meteo(m) = &self.fx else { return None };
        let last = m.meteors.last().filter(|_| m.light)?;
        let mut c = [0; 4];
        crate::boss::set_color(&mut c, METEOR_LIGHT, ONE);
        Some((last.pos, c, METEOR_LIGHT_FAR))
    }

    /// The manager's `Draw` of a spell it runs itself: the rules' step on
    /// its generators and frame, their calls made, then the picture.
    /// (Still enabled, a storm's magic square.)
    pub fn run(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables) -> (bool, Option<V4>) {
        let mut run = Run::new(cx, t.volume);
        let (on, square) = match &mut self.fx {
            Fx::Meteo(m) => (m.draw(&mut run).0, None),
            Fx::Storm(s) => s.draw(&mut run),
            Fx::Tower(r) => (r.draw(&mut run), None),
        };
        let outs = run.outs;
        for o in &outs {
            out_here(Some(ctrl), cx, o);
        }
        self.draw(cx);
        (on, square)
    }

    /// The picture of the last `Draw`, on the effect layer.
    pub fn draw(&mut self, cx: &mut Cx) {
        let volume = cx.assets.boss.volume;
        match (&self.fx, &mut self.pics) {
            (Fx::Meteo(m), Pics::Meteo(p)) => {
                for (meteor, (fire, glow, turn)) in m.meteors.iter().zip(p.iter_mut()) {
                    if meteor.drawn {
                        draw_meteor(cx, meteor, fire, *glow, turn);
                    }
                }
            }
            (Fx::Storm(s), Pics::Storm(p)) => {
                for ((bolt, _, _), (clump, eff)) in s.bolts.iter().zip(p.iter_mut()) {
                    draw_bolt(cx, volume, bolt, *clump, eff);
                }
            }
            (Fx::Tower(r), Pics::Tower(clump)) => {
                for t in &r.towers {
                    let m = vu::pos_rot_zyx_scale(t.drawn_pos, t.dirc, [t.scale, t.scale, t.scale, ONE]);
                    if let Some(obj) = *clump {
                        cx.draws.push(DrawRec::Clump {
                            obj,
                            matrix: m,
                            alpha: t.drawn_alpha,
                            layer: EFFECT_LAYER,
                            clut: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// `ccBossEffMeteo2::Draw`: the fire and the glow at the meteor, the
/// glow turning about its own y (`m_rotLight`); the fire started again
/// once it ends.
fn draw_meteor(cx: &mut Cx, m: &rules::Meteor, fire: &mut Option<AnmObj>, glow: Option<ObjRef>, turn: &mut F) {
    let mut mat = vu::rot_y(&vu::UNIT, *turn);
    mat = vu::rot_z(&mat, HALF_PI);
    mat = vu::rot_y(&mat, m.dirc[1]);
    mat = vu::rot_z(&mat, m.dirc[2]);
    mat = vu::rot_z(&mat, NEG_HALF_PI);
    mat = vu::trans(&mat, m.pos);
    if let Some(a) = fire.as_mut() {
        let ended = a.forward(cx);
        a.draw(cx, mat, ONE);
        if ended {
            a.restart();
        }
    }
    if let Some(obj) = glow {
        cx.draws.push(DrawRec::Clump { obj, matrix: mat, alpha: ONE, layer: EFFECT_LAYER, clut: None });
    }
    let mut r = ee::add(*turn, GLOW_TURN);
    if ee::lt(r, ee::neg(PI)) {
        r = ee::add(r, TWO_PI);
    }
    if !ee::le(r, PI) {
        r = ee::sub(r, TWO_PI);
    }
    *turn = r;
}

/// An angle wrapped: under -pi plus 2 pi, then over pi less 2 pi.
fn wrap(mut a: F) -> F {
    if ee::lt(a, ee::neg(PI)) {
        a = ee::add(a, TWO_PI);
    }
    if !ee::le(a, PI) {
        a = ee::sub(a, TWO_PI);
    }
    a
}

/// `ccBossEffThunder2::Draw`'s segments: each from one point to the next
/// (through `ccTransPosW2P`), a clump stretched to its length (1/900) and
/// laid along it, then its sprite there (0.025 of the length tall, turned
/// by its pitch less pi/2), a pattern by the segment's `ccRand()`.
fn draw_bolt(cx: &mut Cx, volume: Volume, t: &rules::Thunder, clump: Option<ObjRef>, eff: &mut Option<Box<Eff>>) {
    let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
    for s in 0..t.shown.clamp(0, 9) as usize {
        let (p, q) = (t.points[s], t.points[s + 1]);
        let a = space::w2p(p, player, bounds);
        let b = space::w2p(q, player, bounds);
        let yaw = crate::drawelm::get_dirc(a, b);
        let d2 = dist(volume, a, b, false);
        let d3 = dist(volume, a, b, true);
        let pitch = wrap(ee::atan2f(ee::sub(p[2], q[2]), d2));
        let mut m = vu::UNIT;
        m[0][0] = THREE;
        m[1][1] = ee::mul(0x3a91_a2b4, d3);
        m[2][2] = THREE;
        m = vu::rot_z(&m, NEG_HALF_PI);
        m = vu::rot_y(&m, pitch);
        m = vu::rot_z(&m, yaw);
        m = vu::rot_z(&m, NEG_HALF_PI);
        m = vu::trans(&m, p);
        if let Some(obj) = clump {
            cx.draws.push(DrawRec::Clump { obj, matrix: m, alpha: t.alpha, layer: EFFECT_LAYER, clut: None });
        }
        if let Some(e) = eff.as_mut() {
            e.scale_y = ee::mul(0x3ccc_cccd, d3);
            e.scale_x = THREE;
            e.rotate = wrap(ee::sub(pitch, HALF_PI));
            e.transparency = t.alpha;
            let n = i32::from(e.pat_num.max(1));
            let pat = t.pats[s].wrapping_rem(n) as u16;
            crate::drawelm::draw_eff_at(cx, e, p, pat);
        }
    }
}

/// `ccGetDist(a, b)` or `ccGetDist3D(a, b)` with the volume's square root
/// (Outbreak and Quarantine inline `sqrt.s`).
fn dist(volume: Volume, a: V4, b: V4, three: bool) -> F {
    let mut d = ee::vsub(b, a);
    if !three {
        d[2] = 0;
    }
    d[3] = ONE;
    let dd = ee::add(ee::add(ee::mul(d[0], d[0]), ee::mul(d[1], d[1])), ee::mul(d[2], d[2]));
    match volume {
        Volume::Out | Volume::Qua => ee::sqrt(dd),
        Volume::Inf | Volume::Mut => ee::sqrtf(dd),
    }
}

/// `new ccEff`, `Init(GetChunkAdrsF("particle", name), 1)`: fog on.
fn particle_eff(cx: &Cx, name: &str) -> Option<Box<Eff>> {
    let o = cx.assets.find("particle", name)?;
    let (j, c) = cx.assets.eff_chunk(o)?;
    Some(Box::new(Eff::init(o.file, j, c, true, &cx.assets.alpha_blend)))
}

/// What a spell's rule asks beside its own picture (the rules' `Pic`s):
/// a landing's rocks and bursts, a strike's flare, rocks and bursts, a
/// tower's rocks, dust.
pub fn calls(ctrl: &mut EffectCtrl, cx: &mut Cx, p: &Pic) {
    const FORTY_FIVE: F = 0x4234_0000;
    const FIVE: F = 0x40a0_0000;
    let down = ee::deg2rad(-16384);
    match *p {
        // effSmoke(pos, vel, size, 60, 114, 512, 32).
        Pic::Smoke { pos, vel, size } => {
            dust::eff_smoke(cx, pos, vel, size, 60, 114, 512, 32);
        }
        // Meteo2::Move: effSmokeRock(pos, (-pi/2, 0, 0), 45, 2, 1, -1),
        // ccParticleExplode(pos, (0, 0, 10), 1, 5.0) and again at 15 up.
        Pic::MeteorLand { pos } => {
            debris::eff_smoke_rock(ctrl, cx, pos, [down, 0, 0, ONE], FORTY_FIVE, 1, -1);
            for z in [0x4120_0000, 0x4170_0000] {
                particle::cc_particle_explode(cx, pos, [0, 0, z, ONE], FIVE, 1);
            }
        }
        // effRadiateSomething2(pos, rot, 45, 4, 4).
        Pic::Radiate { pos, rot } => {
            debris::eff_radiate_something2(ctrl, cx, pos, rot, FORTY_FIVE, 4, 4);
        }
        // Thunder2::Shock: effFlareRing(pos, 2), four
        // effRadiateSomething2(pos, (-pi/2, 0, turn), 45, 4, 4), five
        // ccParticleExplode(pos, vel, 1, 5.0).
        Pic::Shock { pos, turn, vels } => {
            fall::eff_flare_ring(ctrl, cx, pos, 2);
            for _ in 0..4 {
                debris::eff_radiate_something2(ctrl, cx, pos, [down, 0, turn, ONE], FORTY_FIVE, 4, 4);
            }
            for v in vels {
                particle::cc_particle_explode(cx, pos, v, FIVE, 1);
            }
        }
        Pic::Text { .. } | Pic::Voice(_) | Pic::VoiceStop => {}
    }
}

/// A call the rules made while this manager ran them: sounds raised, the
/// calls beside made (with the effect slots at hand).
fn out_here(ctrl: Option<&mut EffectCtrl>, cx: &mut Cx, o: &Out) {
    match *o {
        Out::Se { se } => cx.raise(Event::Sound { se }),
        Out::SeNote { se, note } => cx.raise(Event::SoundNote { se, note: i32::from(note) }),
        Out::Se3d { se, pos } => cx.raise(Event::Sound3d { se, pos }),
        Out::Fidchell(ref p) => {
            if let Some(ctrl) = ctrl {
                calls(ctrl, cx, p);
            }
        }
        _ => {}
    }
}

/// The rules' [`rules::Host`] over this manager: `ccRand` its host's
/// `genrand`, the field's frame about the player, the volume's square
/// root; their calls kept to make after.
struct Run<'a> {
    rng: Gen<'a>,
    player: V4,
    bounds: [F; 4],
    volume: Volume,
    outs: Vec<Out>,
}

struct Gen<'a>(&'a mut dyn crate::Host);

impl Rng for Gen<'_> {
    fn rand(&mut self) -> i32 {
        self.0.genrand() as i32
    }
}

impl<'a> Run<'a> {
    fn new(cx: &'a mut Cx, volume: Volume) -> Run<'a> {
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        Run { rng: Gen(&mut *cx.host), player, bounds, volume, outs: Vec::new() }
    }
}

impl rules::Host for Run<'_> {
    fn cc(&mut self) -> &mut dyn Rng {
        &mut self.rng
    }
    fn w2p(&self, v: V4) -> V4 {
        space::w2p(v, self.player, self.bounds)
    }
    fn p2w(&self, v: V4) -> V4 {
        space::p2w(v, self.player, self.bounds)
    }
    fn dist(&self, a: V4, b: V4) -> F {
        dist(self.volume, a, b, false)
    }
    fn out(&mut self, o: Out) {
        self.outs.push(o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The swarm's light: `ccSetColor(0x80ffffff, 1.0)`'s HSV, hue 255 in
    /// the sixth sector, full saturation and value: red with a little blue.
    #[test]
    fn the_meteors_light_is_red() {
        let mut c = [0; 4];
        crate::boss::set_color(&mut c, METEOR_LIGHT, ONE);
        let f = |k: usize| ee::f(c[k]);
        assert!((f(0) - 1.0).abs() < 1e-6, "{c:x?}");
        assert_eq!(c[1], 0);
        assert!((f(2) - 6.0 / 256.0).abs() < 1e-3, "{c:x?}");
        assert_eq!(c[3], 0);
    }
}
