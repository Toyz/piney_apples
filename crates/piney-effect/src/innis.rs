//! Innis's pictures (MUT gcmn bosseff.cpp, boss02.cpp). Its missiles
//! (`ccBossEffIceMissile` 0x00478910 and `Draw` 0x00478d50, Lightning
//! 0x004795b0 / 0x00479930, Blaze 0x00479ed0 / 0x0047a220): their flights
//! and `MagicDamage` are piney-battle's rules, and each `Draw`'s picture is
//! drawn here from what it left (`innis::Shown`), with its trails, and at
//! the first missile's landing its smoke, rocks and dust. And the particle
//! generators Innis and its images start (`innis::Gen`).

use piney_battle::boss::innis::{Call, Element, Gen, Missile as Flight, Shown};

use crate::boss::{Tables, WaveShock, ff_row, gen_row, va};
use crate::draw::DrawRec;
use crate::drawelm::AnmObj;
use crate::ee::{self, F, ONE, V4, VF0};
use crate::effect::{EFFECT_LAYER, EffectCtrl};
use crate::files::ObjRef;
use crate::particle::GenRef;
use crate::{Cx, VecRef, debris, dust, vu};

const THREE: [F; 4] = [0x4040_0000, 0x4040_0000, 0x4040_0000, ONE];
/// The lightning's and the fire's transparency in flight.
const HALF: F = 0x3f00_0000;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// The ice's burst: its CLUTs' suffix (@1798) on the explosion's objects.
const ICE_CLUT: &str = "c1";
/// The anchors a missile's trails follow: its place and its turn, by its
/// manager slot.
const ANCHOR: u32 = 0x8000_0000;

/// One of Innis's missiles in a manager slot, drawn for the rules' slot.
#[derive(Clone, Debug)]
pub struct Missile {
    pub rules_slot: i32,
    element: Element,
    no: i32,
    pics: Pics,
    /// The trails (`MissileSmokeGenerator` rows), killed at the landing.
    smoke: Vec<GenRef>,
    anchor: u32,
    /// The flight's `t` at the last `Draw` drawn: every `Draw` steps it.
    last_t: Option<F>,
    /// A missile the manager runs alone (no callback, no camera: the
    /// probe's): its flight and control points.
    own: Option<Box<(Flight, [V4; 4])>>,
}

/// What a missile draws with.
#[derive(Clone, Debug)]
enum Pics {
    /// +0xd8 the crystal (particle's `CMP_x202a`); the first's +0xe0
    /// `ANM_ex31lhit` (xeffect) and +0xe4 `ANM_x305`, its objects turned to
    /// the `c1` palettes.
    Ice { crystal: Option<ObjRef>, hit: Option<AnmObj>, burst: Option<AnmObj> },
    /// +0xe0 the bolt (particle's `CMP_x500`).
    Lightning { bolt: Option<ObjRef> },
    /// +0xe4 the fireball (`ANM_x300`), the first's `ANM_x305` once landed.
    Blaze { ball: Option<AnmObj> },
}

/// `new ccParticleGenerator(row, ff...)` of a [`va`] table, distSW off,
/// following the missile's place and turn (`syncPos`, `syncRot`).
fn trail(cx: &mut Cx, t: &Tables, row: u32, ff: u32, anchor: u32) -> Option<crate::particle::Generator> {
    let mut g = t.generator(cx, gen_row(va::MISSILE_SMOKE, row), [ff_row(va::MISSILE_SMOKE_FF, ff), 0, 0, 0])?;
    g.sync_pos_type = false;
    g.sync_pos = Some(VecRef::Anchor(anchor));
    g.sync_rot = Some(VecRef::Anchor(anchor + 1));
    g.dist_sw = false;
    Some(g)
}

impl Missile {
    /// The constructor: its pictures, and its trails started (they
    /// follow the anchors [`Missile::draw`] sets).
    pub fn new(cx: &mut Cx, t: &Tables, slot: usize, rules_slot: i32, element: Element, no: i32) -> Missile {
        let anchor = ANCHOR | (slot as u32) << 1;
        let first = no == 1;
        let (pics, rows): (Pics, &[(u32, u32)]) = match element {
            Element::Ice => {
                let burst = first.then(|| AnmObj::new(cx, "particle", "ANM_x305")).flatten().map(|mut a| {
                    let d = &cx.spells.data.draw;
                    let subst: Vec<(String, String)> =
                        (0..4).map(|k| (d.subst[k].clone(), format!("{}{ICE_CLUT}", d.clut[k]))).collect();
                    a.change_clut(cx, &subst);
                    a
                });
                let hit = first.then(|| AnmObj::new(cx, "xeffect", "ANM_ex31lhit")).flatten();
                (Pics::Ice { crystal: cx.assets.find("particle", "CMP_x202a"), hit, burst }, &[(0, 0)])
            }
            Element::Lightning => {
                (Pics::Lightning { bolt: cx.assets.find("particle", "CMP_x500") }, &[(1, 12), (2, 1)])
            }
            Element::Blaze => (Pics::Blaze { ball: AnmObj::new(cx, "particle", "ANM_x300") }, &[(3, 2), (4, 3)]),
        };
        // Made in order, then started in order.
        let gens: Vec<_> = rows.iter().filter_map(|&(row, ff)| trail(cx, t, row, ff, anchor)).collect();
        let smoke = gens.into_iter().map(|g| cx.particles.start(g)).collect();
        Missile { rules_slot, element, no, pics, smoke, anchor, last_t: None, own: None }
    }

    /// `Create(ctrl, no, 0, 0)`: a missile the manager runs itself, its
    /// flight the rules' ([`Flight::step`]) with no callback.
    pub fn alone(cx: &mut Cx, t: &Tables, slot: usize, element: Element, no: i32, ctrl: [V4; 4]) -> Missile {
        let mut m = Missile::new(cx, t, slot, -1, element, no);
        m.own = Some(Box::new((Flight::alone(&ctrl), ctrl)));
        m
    }

    /// Whether the manager runs this one itself.
    pub fn runs_alone(&self) -> bool {
        self.own.is_some()
    }

    /// The manager's `Draw` of a missile it runs itself: the flight's
    /// step; its picture and, landing, its trails killed; its calls in
    /// order (its sounds, a lightning's WaveShock begun, returned for the
    /// manager's slots); then the first's burst. False once it ends.
    pub fn run(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables) -> (bool, Option<WaveShock>) {
        let Some(mut own) = self.own.take() else { return (false, None) };
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let mut calls = Vec::new();
        let (el, no) = (self.element, self.no);
        own.0.step(el, no, &own.1, |v| crate::space::w2p(v, player, bounds), &mut calls);
        let landed = self.picture(ctrl, cx, &own.0);
        let (mut alive, mut wave) = (true, None);
        for c in calls {
            match c {
                Call::Se3dNote { se, pos, note } => cx.raise(crate::Event::Sound3dNote { se, pos, note: note.into() }),
                Call::WaveShock { pos } => wave = Some(WaveShock::begin(cx, t, pos, ONE)),
                Call::Quake => {}
                Call::Due { .. } => alive = false,
            }
        }
        if landed {
            self.burst_calls(ctrl, cx, t, own.0.pos);
        }
        self.own = Some(own);
        (alive, wave)
    }

    /// The anchors the trails follow, for a slot let go.
    pub fn anchors(&self) -> [u32; 2] {
        [self.anchor, self.anchor + 1]
    }

    /// The picture of the `Draw` that left `f`: in flight, landing (its
    /// trails ended and, for the first, the burst's calls), or the burst.
    pub fn draw(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables, f: &Flight) {
        if self.picture(ctrl, cx, f) {
            self.burst_calls(ctrl, cx, t, f.pos);
        }
    }

    /// The picture of the `Draw` that left `f` (none twice for one `Draw`)
    /// and the anchors its trails follow; landing, the trails killed. True
    /// on the landing.
    fn picture(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, f: &Flight) -> bool {
        ctrl.anchors.insert(self.anchor, f.pos);
        ctrl.anchors.insert(self.anchor + 1, f.rot);
        if self.last_t == Some(f.t) {
            return false;
        }
        self.last_t = Some(f.t);
        match f.shown {
            Shown::Flight { rot } => self.flight(cx, f.pos, rot),
            Shown::Landed { rot } => {
                self.flight(cx, f.pos, rot);
                for g in self.smoke.drain(..) {
                    cx.particles.particle_kill(g);
                }
                return true;
            }
            Shown::Burst => self.burst(cx, f.pos),
            Shown::Nothing => {}
        }
        false
    }

    /// In flight: the crystal, bolt or fireball at `pos`, turned by `rot`
    /// (`SetMatrix_PosRotXYZScale`, scale 3).
    fn flight(&mut self, cx: &mut Cx, pos: V4, rot: V4) {
        let m = vu::pos_rot_xyz_scale(pos, rot, THREE);
        match &mut self.pics {
            Pics::Ice { crystal: Some(obj), .. } => {
                cx.draws.push(DrawRec::Clump { obj: *obj, matrix: m, alpha: ONE, layer: EFFECT_LAYER, clut: None });
            }
            Pics::Lightning { bolt: Some(obj) } => {
                cx.draws.push(DrawRec::Clump { obj: *obj, matrix: m, alpha: HALF, layer: EFFECT_LAYER, clut: None });
            }
            Pics::Blaze { ball: Some(a) } => {
                a.forward(cx);
                a.draw(cx, m, HALF);
            }
            _ => {}
        }
    }

    /// The first ice or fire missile after its landing: the ice's hit
    /// (scale 5) and explosion (2.5) turned by nothing
    /// (`SetMatrix_PosRotZYXScale`); the fire's explosion (scale 4,
    /// `SetMatrix_PosRotXYZScale`), set again whenever it ends.
    fn burst(&mut self, cx: &mut Cx, pos: V4) {
        let s = |v: F| [v, v, v, ONE];
        match &mut self.pics {
            Pics::Ice { hit, burst, .. } => {
                let mh = vu::pos_rot_zyx_scale(pos, VF0, s(0x40a0_0000));
                let mb = vu::pos_rot_zyx_scale(pos, VF0, s(0x4020_0000));
                for a in [hit.as_mut(), burst.as_mut()].into_iter().flatten() {
                    a.forward(cx);
                }
                if let Some(a) = hit {
                    a.draw(cx, mh, ONE);
                }
                if let Some(a) = burst {
                    a.draw(cx, mb, ONE);
                }
            }
            Pics::Blaze { ball: Some(a) } => {
                let ended = a.forward(cx);
                a.draw(cx, vu::pos_rot_xyz_scale(pos, VF0, s(0x4080_0000)), HALF);
                if ended {
                    a.restart();
                }
            }
            _ => {}
        }
    }

    /// The first missile's landing (the trails already killed). The ice
    /// throws its smoke (`BurstSmokeGenerator` rows 0, 0 50 up, 5 200 up),
    /// two sprays of rocks turned by -pi/2 and its dust; the lightning its
    /// smoke (rows 1, 6, 13; its WaveShock is the rules'); the fire its
    /// smoke (rows 2, 2 and 3 70 up, 4 at the floor, 7 90 up), its dust,
    /// and `ANM_x305` in place of its fireball.
    fn burst_calls(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables, pos: V4) {
        if self.no != 1 {
            return;
        }
        let up = |z: F| {
            let mut p = pos;
            p[2] = z;
            p
        };
        let z = pos[2];
        // (row, its force fields, where): `BurstSmokeGenerator` and
        // `BurstSmokeForceField` rows.
        let burst: Vec<(u32, &[u32], V4)> = match self.element {
            Element::Ice => vec![
                (0, &[0], pos),
                (0, &[0], up(ee::add(z, 0x4248_0000))),
                (5, &[4, 0, 9], up(ee::add(ee::add(z, 0x4248_0000), 0x4316_0000))),
            ],
            Element::Lightning => vec![(1, &[1], pos), (6, &[5], pos), (13, &[9], pos)],
            Element::Blaze => {
                let z70 = ee::add(z, 0x428c_0000);
                vec![
                    (2, &[10], pos),
                    (2, &[2], up(z70)),
                    (3, &[16, 2], up(z70)),
                    (4, &[3, 2], up(0)),
                    (7, &[6, 7, 8], up(0x42b4_0000)),
                ]
            }
        };
        for (k, (row, ff, at)) in burst.into_iter().enumerate() {
            let ffs = std::array::from_fn(|k| ff.get(k).map_or(0, |&r| ff_row(va::BURST_SMOKE_FF, r)));
            if let Some(mut g) = t.generator(cx, gen_row(va::BURST_SMOKE, row), ffs) {
                g.pos = at;
                // The lightning's second takes its offset (unused: it
                // follows nothing) from +0x40, its place by then.
                if self.element == Element::Lightning && k == 1 {
                    g.offset = pos;
                }
                g.dist_sw = false;
                cx.particles.start(g);
            }
        }
        match self.element {
            Element::Ice => {
                let r = [NEG_HALF_PI, 0, 0, ONE];
                debris::eff_ice_rock(ctrl, cx, pos, r, 0x4248_0000, ONE, 20);
                debris::eff_ice_rock(ctrl, cx, pos, r, 0x42c8_0000, HALF, 20);
                dust::enemy_eff_dust(cx, pos, 10, 0x40a0_0000, dust::DUST_LIFE, dust::DUST_TEX);
            }
            Element::Blaze => {
                dust::enemy_eff_dust(cx, pos, 10, 0x40a0_0000, dust::DUST_LIFE, dust::DUST_TEX);
                self.pics = Pics::Blaze { ball: AnmObj::new(cx, "particle", "ANM_x305") };
            }
            Element::Lightning => {}
        }
    }
}

/// A generator Innis or one of its images starts at `pos` (MUT gcmn):
/// `SwitchActionPattern`'s field (0x00496fc0: rows 2, 3, 0, 1 with
/// `InisFieldForceField` 0 and 1 or 2), `SkillAttack`'s glow (0x00499110:
/// row 4, force fields 3 and 1), `Pattern10`'s tornado (0x00498990, force
/// fields 0-2), `AllAttack`'s burst (0x0049aa10: `BurstGenerator` row 2,
/// force field 1) and `EnemyBurst`'s (0x0049d1b0: rows 3 and 4). Rows 4,
/// the tornado and burst row 2 have distSW off.
pub fn start(cx: &mut Cx, t: &Tables, which: Gen, pos: V4) {
    let field = |k: u32| ff_row(va::INIS_FIELD_FF, k);
    let burst = |k: u32| ff_row(va::BURST_FF, k);
    let (row, ff, dist_sw) = match which {
        Gen::InisField(k @ 0..=3) => {
            (gen_row(va::INIS_FIELD, u32::from(k)), [field(0), field(1 + u32::from(k & 1)), 0, 0], true)
        }
        Gen::InisField(_) => (gen_row(va::INIS_FIELD, 4), [field(3), field(1), 0, 0], false),
        Gen::Tornado => {
            let tf = |k: u32| ff_row(va::TORNADO_FF, k);
            (va::TORNADO, [tf(0), tf(1), tf(2), 0], false)
        }
        Gen::Burst(2) => (gen_row(va::BURST, 2), [burst(1), 0, 0, 0], false),
        Gen::Burst(3) => (gen_row(va::BURST, 3), [burst(4), burst(3), 0, 0], true),
        Gen::Burst(_) => (gen_row(va::BURST, 4), [burst(5), 0, 0, 0], true),
    };
    if let Some(mut g) = t.generator(cx, row, ff) {
        g.pos = pos;
        g.dist_sw = dist_sw;
        cx.particles.start(g);
    }
}
