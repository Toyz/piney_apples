//! Kyvia's pictures. The fight's particle generators as its rules start,
//! kill and pause them ([`start`], [`gen_op`]); the meteors
//! (`ccBossEffMeteoriteMissile`, MUT 0x0047aa90) and the second fight's
//! bolts (`ccBossEffThunderbolt`, OUT 0x00477410) drawn from the flights
//! their rules left, with trails, smoke and rocks; the third's magical ball
//! (`ccBossEffMagicalBall`, QUA 0x0036b4e0) and the fourth's hell's express
//! (`HellSpecialExpress`, QUA 0x0036cdc0). docs/engine/boss-kyvia*.md.

use piney_battle::boss::kyvia::hell::{Hell as HellRules, HellHost, HellOut};
use piney_battle::boss::kyvia::magic_ball::{BallHost, BallOut, MagicalBall};
use piney_battle::boss::kyvia::thunder::{Bolt as Strokes, BoltData};
use piney_battle::boss::kyvia::{Follow, Gen, GenOp, Held, Meteorite as Flights, Place, Table};
use piney_battle::rand::Rng;

use crate::boss::{Tables, ff_row, gen_row, va};
use crate::draw::DrawRec;
use crate::drawelm::AnmObj;
use crate::ee::{self, F, ONE, V4, VF0};
use crate::effect::{EFFECT_LAYER, EffectCtrl};
use crate::files::ObjRef;
use crate::particle::{GenParam, GenRef};
use crate::{CharRef, Cx, VecRef, debris, vu};

const THREE: [F; 4] = [0x4040_0000, 0x4040_0000, 0x4040_0000, ONE];
const HALF: F = 0x3f00_0000;
const PI: F = ee::PI;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_PI: F = 0xc049_0fdb;

/// The anchors the fight's moving members publish ([`anchor`]) and the
/// meteors' places (`0x2000_0000 | slot << 4 | meteor`).
const MEMBER: u32 = 0x4000_0000;
const METEOR: u32 = 0x2000_0000;

/// The anchor a generator following `f` reads (a character's place is its
/// own `CharPos`).
pub fn anchor(f: Follow) -> Option<u32> {
    match f {
        Follow::Char(_) => None,
        Follow::DeadSmoke => Some(MEMBER),
        Follow::BeamHead(k) => Some(MEMBER | 0x1_0000 | u32::from(k)),
        Follow::BeamTail(k) => Some(MEMBER | 0x2_0000 | u32::from(k)),
        Follow::Aura(c) => Some(MEMBER | 0x3_0000 | c as u32),
        Follow::Plasma(k) => Some(MEMBER | 0x4_0000 | u32::from(k)),
        Follow::Plasma2(k) => Some(MEMBER | 0x5_0000 | u32::from(k)),
        Follow::Soul(k) => Some(MEMBER | 0x6_0000 | u32::from(k)),
    }
}

/// A table's generator and force field rows (by Infection's addresses).
fn tables_of(t: Table) -> (u32, u32) {
    match t {
        Table::Kyvia => (va::KYVIA, va::KYVIA_FF),
        Table::KyviaDmg => (va::KYVIA_DMG, va::KYVIA_DMG_FF),
        Table::Core => (va::CORE, va::CORE_FF),
        Table::Gomora => (va::GOMORA, va::GOMORA_FF),
        Table::GomoraAura => (va::GOMORA_AURA, va::GOMORA_AURA_FF),
        Table::MissileSmoke => (va::MISSILE_SMOKE, va::MISSILE_SMOKE_FF),
        Table::BurstSmoke => (va::BURST_SMOKE, va::BURST_SMOKE_FF),
    }
}

/// `new ccParticleGenerator(row, ff...)` of `which`, its row changed by
/// `edit` first (the game writes some rows before it makes them), with its
/// `distSW`.
fn make(cx: &mut Cx, t: &Tables, which: Gen, edit: impl FnOnce(&mut GenParam)) -> Option<crate::particle::Generator> {
    let r = which.rows();
    let (gens, _) = tables_of(r.table);
    let (_, ffs) = tables_of(r.ff_table);
    let ff = std::array::from_fn(|k| r.ff.get(k).map_or(0, |&f| ff_row(ffs, u32::from(f))));
    let mut g = t.generator_edited(cx, gen_row(gens, u32::from(r.row)), ff, edit)?;
    g.dist_sw = r.dist_sw;
    Some(g)
}

/// The generators the fight keeps, by the member that holds each.
pub type Kept = std::collections::HashMap<Held, GenRef>;

/// One of the fight's starts (`Out::KyviaParticles`): made where it
/// stands, started, and kept where the fight keeps it.
pub fn start(cx: &mut Cx, t: &Tables, kept: &mut Kept, which: Gen, at: Place, held: Option<Held>) {
    let life = match which {
        Gen::ThunderSmoke(time) => Some(time),
        _ => None,
    };
    let ball_life = match which {
        Gen::Ball(b) => b.life(),
        _ => None,
    };
    let Some(mut g) = make(cx, t, which, |p| {
        // ThunderboltAtk's row 17: its life the thunder's time, radius 50,
        // rate 20 (OUT gcmn 0x004dcf18).
        if let Some(time) = life {
            p.g_life = time;
            p.g_radius = 0x4248_0000;
            p.p_gen_rate = 0x41a0_0000;
        }
        // The ball's rows 12 and 10, their lives written first.
        if let Some(l) = ball_life {
            p.g_life = l;
        }
    }) else {
        return;
    };
    match at {
        Place::At(p) => g.pos = p,
        Place::Line(p, p2) => {
            g.pos = p;
            g.pos2 = p2;
        }
        Place::Follows(f, off) => {
            g.sync_pos_type = false;
            g.sync_pos = Some(anchor(f).map_or_else(
                || match f {
                    Follow::Char(c) => VecRef::CharPos(c as CharRef),
                    _ => unreachable!("a member's anchor"),
                },
                VecRef::Anchor,
            ));
            g.offset = off;
        }
    }
    let r = cx.particles.start(g);
    if let Some(h) = held {
        kept.insert(h, r);
    }
}

/// What the fight does to a generator it keeps: `ParticleKill` (kept on),
/// its `pauseFlag`, or let go.
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

/// `ccRandF`'s source for a flight the manager makes alone: the host's
/// `genrand`.
struct Gen32<'a>(&'a mut dyn crate::Host);

impl Rng for Gen32<'_> {
    fn rand(&mut self) -> i32 {
        self.0.genrand() as i32
    }
}

/// `ccBossEffMeteoriteMissile` in a manager slot: each meteor's fireball
/// (particle's `ANM_x300`, never stepped) and its two trails.
#[derive(Clone, Debug)]
pub struct Meteorite {
    pub rules_slot: i32,
    fire: Vec<Option<AnmObj>>,
    /// Each meteor's trails (`MissileSmokeGenerator` rows 6 and 7, the
    /// game's +0x10c and +0x94), deleted as it lands.
    trails: Vec<[Option<GenRef>; 2]>,
    anchor: u32,
    landed: Vec<bool>,
    /// The first meteor's `Time` at the last `Draw` drawn.
    last_t: Option<F>,
    /// A meteor shower the manager runs alone (no callback, no camera: the
    /// probe's).
    own: Option<Box<Flights>>,
}

impl Meteorite {
    /// The constructor's pictures and trails for `n` meteors (each trail
    /// follows its meteor's place, distSW off).
    pub fn new(cx: &mut Cx, t: &Tables, slot: usize, rules_slot: i32, n: usize) -> Meteorite {
        let anchor = METEOR | (slot as u32) << 4;
        let fire = (0..n).map(|_| AnmObj::new(cx, "particle", "ANM_x300")).collect();
        let mut trails = Vec::with_capacity(n);
        for k in 0..n {
            let mut pair = [None; 2];
            for (j, (row, ff)) in [(6, 5), (7, 6)].into_iter().enumerate() {
                let ff = [ff_row(va::MISSILE_SMOKE_FF, ff), 0, 0, 0];
                if let Some(mut g) = t.generator(cx, gen_row(va::MISSILE_SMOKE, row), ff) {
                    g.dist_sw = false;
                    g.sync_pos_type = false;
                    g.sync_pos = Some(VecRef::Anchor(anchor | k as u32));
                    pair[j] = Some(cx.particles.start(g));
                }
            }
            trails.push(pair);
        }
        Meteorite { rules_slot, fire, trails, anchor, landed: vec![false; n], last_t: None, own: None }
    }

    /// `Create(pos, range, n, 0, 0, 0)`: a shower the manager runs itself.
    pub fn alone(cx: &mut Cx, t: &Tables, slot: usize, pos: V4, range: F, n: usize) -> Meteorite {
        let f = Flights::make(&mut Gen32(&mut *cx.host), pos, range, n);
        let mut m = Meteorite::new(cx, t, slot, -1, n);
        m.own = Some(Box::new(f));
        m
    }

    pub fn runs_alone(&self) -> bool {
        self.own.is_some()
    }

    /// The anchors its trails follow, for a slot let go.
    pub fn anchors(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.trails.len()).map(|k| self.anchor | k as u32)
    }

    /// The manager's `Draw` of a shower it runs itself: the flights'
    /// step, the pictures, the landings' sounds. False once all have lain
    /// 90 frames.
    pub fn run(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables) -> bool {
        let Some(mut f) = self.own.take() else { return false };
        let alive = f.fly(&mut Vec::new());
        self.draw(ctrl, cx, t, &f, true);
        self.own = Some(f);
        alive
    }

    /// The pictures of the `Draw` that left `f` (none twice for one
    /// `Draw`), meteor by meteor as the game draws them: the fireball
    /// while it flies (`SetMatrix_PosRotXYZScale(now, 0, 3)`, transparency
    /// 0.5); at its landing its trails deleted, its sound, and for the
    /// first the burst's smoke and rocks. The landings' sounds are made
    /// here for a shower run alone (`sounds`), else by the fight's rules.
    pub fn draw(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables, f: &Flights, sounds: bool) {
        for (k, &p) in f.now.iter().enumerate() {
            ctrl.anchors.insert(self.anchor | k as u32, p);
        }
        let t0 = f.time.first().copied();
        if self.last_t == t0 {
            return;
        }
        self.last_t = t0;
        for k in 0..f.now.len() {
            if !self.landed[k] {
                let m = vu::pos_rot_xyz_scale(f.now[k], VF0, THREE);
                if let Some(a) = &self.fire[k] {
                    a.draw(cx, m, HALF);
                }
            }
            if f.landed[k] == 0 || self.landed[k] {
                continue;
            }
            self.landed[k] = true;
            for g in self.trails[k].iter().rev().flatten() {
                cx.particles.particle_delete(*g);
            }
            if sounds {
                cx.raise(crate::Event::Sound3dNote { se: 35, pos: f.pos, note: 60 });
            }
            if k == 0 {
                self.burst(ctrl, cx, t, f.pos, f.now[0]);
            }
        }
    }

    /// The first meteor's landing at the shower's place `pos`:
    /// `BurstSmokeGenerator` rows 8 and 9 there, row 11 120 up (distSW
    /// off), and `effSmokeRock(now, rot, 75, 0, 20, -1)` facing away from
    /// the camera.
    fn burst(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables, pos: V4, now: V4) {
        let ffs: [(u8, &[u32]); 3] = [(8, &[11, 12, 13]), (9, &[11, 12, 14]), (11, &[17, 12, 16])];
        for (row, ff) in ffs {
            let mut at = pos;
            if row == 11 {
                let mut up = VF0;
                up[2] = 0x42f0_0000;
                at = ee::vadd(up, pos);
            }
            let ff = std::array::from_fn(|k| ff.get(k).map_or(0, |&r| ff_row(va::BURST_SMOKE_FF, r)));
            if let Some(mut g) = t.generator(cx, gen_row(va::BURST_SMOKE, u32::from(row)), ff) {
                g.pos = at;
                g.dist_sw = false;
                cx.particles.start(g);
            }
        }
        let mut rot = cx.host.camera_rot(VF0);
        rot[2] = ee::sub(rot[2], PI);
        rot[0] = 0;
        rot[1] = 0;
        if !ee::le(rot[2], PI) {
            rot[2] = ee::sub(rot[2], TWO_PI);
        }
        if ee::lt(rot[2], NEG_PI) {
            rot[2] = ee::add(rot[2], TWO_PI);
        }
        debris::eff_smoke_rock(ctrl, cx, now, rot, 0x4296_0000, 20, -1);
    }
}

/// `ccBossEffThunderbolt` in a manager slot: its strokes' segments of
/// particle's `CMP_x012`, on `CLT_x012c1` for a bolt of mode 1 (Kyvia's)
/// and its own palette again for the rest.
#[derive(Clone, Debug)]
pub struct Bolt {
    pub rules_slot: i32,
    clump: Option<ObjRef>,
    /// `CLT_x012` and `CLT_x012c1`.
    cluts: Option<(u32, u32)>,
    /// `Draw`s seen: its `hoge` when last drawn.
    drawn: Option<i32>,
    own: Option<Box<Strokes>>,
}

impl Bolt {
    pub fn new(cx: &Cx, rules_slot: i32) -> Bolt {
        let find = |n: &str| cx.assets.find("particle", n);
        let cluts = find("CLT_x012").zip(find("CLT_x012c1")).map(|(a, b)| (a.object, b.object));
        Bolt { rules_slot, clump: find("CMP_x012"), cluts, drawn: None, own: None }
    }

    /// `Create(pos, time, range, num, dat)` with the caller's offset after
    /// it: a bolt the manager runs itself.
    pub fn alone(cx: &mut Cx, pos: V4, th: (i32, i32, i16), dat: &BoltData, offset: V4) -> Bolt {
        let (time, range, num) = th;
        let mut b = Strokes::new(&mut Gen32(&mut *cx.host), pos, time, range, num, dat);
        b.off_set = offset;
        let mut me = Bolt::new(cx, -1);
        me.own = Some(Box::new(b));
        me
    }

    pub fn runs_alone(&self) -> bool {
        self.own.is_some()
    }

    /// The manager's `Draw` of a bolt it runs itself; false at its end.
    pub fn run(&mut self, cx: &mut Cx, t: &Tables) -> bool {
        let Some(mut b) = self.own.take() else { return false };
        let alive = b.stroke(&mut Gen32(&mut *cx.host));
        self.draw(cx, t, &b);
        self.own = Some(b);
        alive
    }

    /// The pictures of the `Draw` that left `b`: on its first, for a bolt
    /// with its smoke, `MissileSmokeGenerator` row 8 at its place (its life
    /// the bolt's time and its radius the range and 100, at least 350, as
    /// `Draw` writes the row; distSW off); each segment; at its end, with
    /// its burst, `BurstSmokeGenerator` row 10 (ff 15).
    pub fn draw(&mut self, cx: &mut Cx, t: &Tables, b: &Strokes) {
        if self.drawn == Some(b.hoge) {
            return;
        }
        let first = self.drawn.is_none();
        self.drawn = Some(b.hoge);
        if first && b.flg2 != 0 {
            let radius = ee::from_int((b.bottom_range + 100).max(350));
            let edit = |p: &mut GenParam| {
                p.g_life = b.time as i16;
                p.g_radius = radius;
            };
            let ff = [ff_row(va::MISSILE_SMOKE_FF, 9), 0, 0, 0];
            if let Some(mut g) = t.generator_edited(cx, gen_row(va::MISSILE_SMOKE, 8), ff, edit) {
                g.pos = b.center;
                g.dist_sw = false;
                cx.particles.start(g);
            }
        }
        let clut = self.cluts.map(|(own, c1)| (own, if b.mode == 1 { c1 } else { own }));
        if let Some(obj) = self.clump {
            for &matrix in &b.segments {
                cx.draws.push(DrawRec::Clump { obj, matrix, alpha: ONE, layer: EFFECT_LAYER, clut });
            }
        }
        if b.end_flg != 0 && b.flg2 != 0 {
            let ff = [ff_row(va::BURST_SMOKE_FF, 15), 0, 0, 0];
            if let Some(mut g) = t.generator(cx, gen_row(va::BURST_SMOKE, 10), ff) {
                g.pos = b.center;
                g.dist_sw = false;
                cx.particles.start(g);
            }
        }
    }
}

/// `ccBossEffMagicalBall` in a manager slot: its `EffAnm` (xeffect's
/// `ANM_ex31exp3`, stepped once by `InitData` and never again), drawn each
/// `Draw` at the ball's place, at the scale and transparency the ball
/// left it ([`MagicalBall::anm_scale`], [`MagicalBall::anm_transparency`]).
/// Its generators are the fight's (`Out::KyviaParticles`); a ball the
/// manager runs alone (the probe's, with no callback and no blur) makes
/// its own.
#[derive(Clone, Debug)]
pub struct Ball {
    pub rules_slot: i32,
    anm: Option<AnmObj>,
    /// `(EffMode, Delay, ScalePoint)` when last drawn (each `Draw` moves
    /// one of them).
    drawn: Option<(i32, i32, u32)>,
    own: Option<Box<MagicalBall>>,
}

impl Ball {
    pub fn new(cx: &Cx, rules_slot: i32) -> Ball {
        let mut anm = AnmObj::new(cx, "xeffect", "ANM_ex31exp3");
        if let Some(a) = anm.as_mut() {
            a.forward(cx);
        }
        Ball { rules_slot, anm, drawn: None, own: None }
    }

    /// `Create(ran, pos, 0, 0)`: a ball the manager runs itself.
    pub fn alone(cx: &mut Cx, ran: F, pos: V4) -> Ball {
        let b = MagicalBall::new(&mut Gen32(&mut *cx.host), ran, pos);
        let mut me = Ball::new(cx, -1);
        me.own = Some(Box::new(b));
        me
    }

    pub fn runs_alone(&self) -> bool {
        self.own.is_some()
    }

    /// The manager's `Draw` of a ball it runs itself.
    pub fn run(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables) -> bool {
        let Some(mut b) = self.own.take() else { return false };
        let mut kept = Kept::new();
        let alive = b.draw(&mut AloneBall { cx: &mut *cx, t, kept: &mut kept });
        publish(ctrl, &b);
        self.draw(cx, &b);
        self.own = Some(b);
        alive
    }

    /// The `EffAnm` of the `Draw` that left `b` (once a `Draw`).
    pub fn draw(&mut self, cx: &mut Cx, b: &MagicalBall) {
        let now = (b.eff_mode, b.delay, b.scale_point);
        if self.drawn == Some(now) {
            return;
        }
        self.drawn = Some(now);
        if let Some(a) = &self.anm {
            a.draw(cx, vu::pos_rot_xyz_scale(b.pos, VF0, b.anm_scale), b.anm_transparency);
        }
    }
}

/// The places the ball's plasmas' generators follow (`PlaPos[k]`,
/// `PlaPos2[k]`).
pub fn publish(ctrl: &mut EffectCtrl, b: &MagicalBall) {
    for k in 0..b.pla_pos.len() {
        let k8 = k as u8;
        if let (Some(a), Some(a2)) = (anchor(Follow::Plasma(k8)), anchor(Follow::Plasma2(k8))) {
            ctrl.anchors.insert(a, b.pla_pos[k]);
            ctrl.anchors.insert(a2, b.pla_pos2[k]);
        }
    }
}

/// A ball the manager runs alone: no blur, no callback; its sounds, flash
/// and generators its own.
struct AloneBall<'a, 'b> {
    cx: &'a mut Cx<'b>,
    t: &'a Tables,
    kept: &'a mut Kept,
}

impl BallHost for AloneBall<'_, '_> {
    fn has_blur(&self) -> bool {
        false
    }

    fn callback(&mut self, _pos: V4, _flg: i32) {}

    fn out(&mut self, o: BallOut) {
        match o {
            BallOut::SeNote(se, note) => self.cx.raise(crate::Event::SoundNote { se, note: i32::from(note) }),
            BallOut::Blur(_) => {}
            BallOut::Flash(time, color) => {
                self.cx.raise(crate::Event::Flash { time, color, rect: [0, 0, 0x4400_0000, 0x43c0_0000] });
            }
            BallOut::Gen(which, at) => start(self.cx, self.t, self.kept, Gen::Ball(which), at, None),
        }
    }
}

/// `HellSpecialExpress` in a manager slot: its burst, `BungAnm`
/// (xeffect's `ANM_ex31lhit` with `CLT_exx004c1` for `CLT_exx004`), drawn
/// twice each `Draw` that steps it, at `BungPos`, `BungRot` and
/// `BungScale`. Its generators are the fight's (`Out::KyviaParticles`); one
/// the manager runs alone (the probe's: no callback, its own blur) makes
/// its own.
#[derive(Clone, Debug)]
pub struct Hell {
    pub rules_slot: i32,
    anm: Option<AnmObj>,
    own: Option<Box<HellRules>>,
    /// Its own blur (`abgr`, scale) and the generators its souls keep when
    /// it runs alone.
    blur: (u32, F),
    kept: Kept,
}

impl Hell {
    pub fn new(cx: &Cx, rules_slot: i32) -> Hell {
        let mut anm = AnmObj::new(cx, "xeffect", "ANM_ex31lhit");
        if let Some(a) = anm.as_mut() {
            let ccs = &cx.assets.files[a.obj.file].ccs;
            if let (Some(f), Some(t)) = (ccs.find_object("CLT_exx004"), ccs.find_object("CLT_exx004c1")) {
                a.clut.push((f, t));
            }
        }
        Hell { rules_slot, anm, own: None, blur: (0x6580_8080, 0x3f81_47ae), kept: Kept::new() }
    }

    /// `HellSpecialExpressCreate(pos, data)` with no callback and a blur of
    /// its own: the manager runs it itself.
    pub fn alone(cx: &mut Cx, t: &Tables, pos: V4, rot: V4, kyvia_lv: i32) -> Hell {
        let mut me = Hell::new(cx, -1);
        let clip = me.anm.as_ref().map(|a| {
            let an = &cx.assets.files[a.obj.file].anims[a.play.anim];
            (an.frames, an.looping)
        });
        let clips = |name: &str| (name == "ANM_ex31lhit").then_some(clip).flatten();
        let h = HellRules::new(&mut Gen32(&mut *cx.host), pos, rot, None, kyvia_lv, &clips);
        HellRules::start_gens(&mut AloneHell { cx: &mut *cx, t, kept: &mut me.kept, blur: &mut me.blur });
        me.own = Some(Box::new(h));
        me
    }

    pub fn runs_alone(&self) -> bool {
        self.own.is_some()
    }

    /// The burst's CLUT swaps (from, to) by name, as `Init`'s
    /// `ChangeClut` made them.
    pub fn clut_names(&self, assets: &crate::files::Assets) -> Vec<(String, String)> {
        let Some(a) = self.anm.as_ref() else { return Vec::new() };
        let ccs = &assets.files[a.obj.file].ccs;
        let n = |o: u32| ccs.object_name(o).unwrap_or("?").to_string();
        a.clut.iter().map(|&(f, t)| (n(f), n(t))).collect()
    }

    /// The manager's `Draw` of one it runs itself.
    pub fn run(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx, t: &Tables) -> bool {
        let Some(mut h) = self.own.take() else { return false };
        let alive = h.draw(&mut AloneHell { cx: &mut *cx, t, kept: &mut self.kept, blur: &mut self.blur });
        hell_publish(ctrl, &h);
        self.draw(cx, &h);
        self.own = Some(h);
        alive
    }

    /// The burst of the `Draw` that left `h`, when it stepped it.
    pub fn draw(&mut self, cx: &mut Cx, h: &HellRules) {
        if !h.bung_drawn {
            return;
        }
        if let Some(a) = self.anm.as_mut() {
            a.play.time = h.bung_anm.time;
            a.play.posed = h.bung_anm.posed;
            let m = vu::pos_rot_xyz_scale(h.bung_pos, h.bung_rot, h.bung_scale);
            a.draw(cx, m, ONE);
            a.draw(cx, m, ONE);
        }
    }
}

/// The places hell's express's souls' generators follow (`Pos[k]`).
pub fn hell_publish(ctrl: &mut EffectCtrl, h: &HellRules) {
    for (k, &p) in h.pos.iter().enumerate() {
        if let Some(a) = anchor(Follow::Soul(k as u8)) {
            ctrl.anchors.insert(a, p);
        }
    }
}

/// Hell's express the manager runs alone: no callback; its blur its own;
/// its sounds, quakes (their `ccRandF`) and generators.
struct AloneHell<'a, 'b> {
    cx: &'a mut Cx<'b>,
    t: &'a Tables,
    kept: &'a mut Kept,
    blur: &'a mut (u32, F),
}

impl HellHost for AloneHell<'_, '_> {
    fn blur(&self) -> (u32, F) {
        *self.blur
    }

    fn rand(&mut self) -> i32 {
        self.cx.host.genrand() as i32
    }

    fn callback(&mut self, _target: Option<usize>) {}

    fn out(&mut self, o: HellOut) {
        use piney_battle::boss::blur::Set;
        match o {
            HellOut::SeNote(se, note) => self.cx.raise(crate::Event::SoundNote { se, note: i32::from(note) }),
            HellOut::Blur(Set::Abgr(v)) => self.blur.0 = v,
            HellOut::Blur(Set::Scale(v)) => self.blur.1 = v,
            HellOut::Quake(q) => {
                let mut g = Gen32(&mut *self.cx.host);
                for v in q {
                    piney_battle::enemy_ai::rand_f(&mut g, v);
                }
            }
            HellOut::Gen(which, at, held) => start(self.cx, self.t, self.kept, Gen::Hell(which), at, held),
            HellOut::Kill(h) => {
                gen_op(self.cx, self.kept, h, GenOp::Kill);
                gen_op(self.cx, self.kept, h, GenOp::Forget);
            }
            _ => {}
        }
    }
}
