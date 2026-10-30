//! Hell's express (`HellSpecialExpress`, bosseff.cpp; QUA gcmn `Clear`
//! 0x0036ccd0, `Init` 0x0036cdc0, `HellStartScene` 0x0036d790, `Draw`
//! 0x0036da40, made by `HellSpecialExpressCreate` 0x0037bf40), act 21 of
//! Kyvia's fourth fight: three souls ride a spline from high over the
//! place down to it, the camera on the first; as the last lands the
//! callback strikes the party (`Kyvia_HellE_Damage`) and `ANM_ex31lhit`
//! bursts there. docs/engine/boss-kyvia-fourth.md.

use piney_data::field::ee;

use super::{Follow, Gen, GenOp, Held, Place, beam};
use crate::boss::blur::Set;
use crate::boss::{Anm, BossBase, Cx, EffKind, Out, VF0};
use crate::enemy_ai::rand_f;
use crate::geom::{self, V4};
use crate::rand::Rng;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_PI: F = 0xc049_0fdb;
/// `SoulNum`: the souls `Init` makes.
const SOULS: usize = 3;

/// Its particle generators (`MissileSmokeGenerator` rows, their pictures
/// piney-effect's), all with `distSW` off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HellGen {
    /// Each soul's three (`Sgp`, `Kgp`, `Pgp`: rows 18, 19 and 21) and the
    /// last's own (`Ogp`, row 20), following its place.
    Soul(u8),
    Last,
    /// `HellStartScene`'s: rows 26 and 27 at once, 28 at its 70th frame.
    Start(u8),
    /// The first soul landing (row 22) and the burst (rows 23-25).
    Landed,
    Burst(u8),
}

impl HellGen {
    /// Its row and force fields.
    pub fn rows(self) -> (u8, &'static [u8]) {
        match self {
            HellGen::Soul(0) => (18, &[13]),
            HellGen::Soul(1) => (19, &[13]),
            HellGen::Soul(_) => (21, &[17]),
            HellGen::Last => (20, &[16]),
            HellGen::Start(0) => (26, &[19, 20]),
            HellGen::Start(1) => (27, &[10, 21]),
            HellGen::Start(_) => (28, &[]),
            HellGen::Landed => (22, &[9]),
            HellGen::Burst(0) => (23, &[9, 17]),
            HellGen::Burst(1) => (24, &[13, 18]),
            HellGen::Burst(_) => (25, &[17]),
        }
    }
}

/// `HellSpecialExpress`'s members (Infection's DWARF names).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hell {
    /// `HellDataSet`: the callback's target and `KyviaLV` (the place,
    /// 0-3); its blur and camera are the fight's.
    pub target: Option<usize>,
    pub kyvia_lv: i32,
    /// `Spoint1`-`4`: the souls' spline, `Spoint4` the place.
    pub spoint: [V4; 4],
    /// `targetCamPos`: the eye while the souls fall.
    pub target_cam_pos: V4,
    /// `Pos[k]`, `time[k]`, `ActProccess[k]` of each soul.
    pub pos: [V4; 5],
    pub time: [F; 5],
    pub act_proccess: [i32; 5],
    pub end_count: i32,
    pub soul_num: i32,
    pub hell_act: i32,
    pub hell_cou: i32,
    /// `BungStartFlg`, `BungPos`, `BungRot`, `BungScale` and `BungAnm`
    /// (`ANM_ex31lhit` of `xeffect`, `CLT_exx004c1` for `CLT_exx004`).
    pub bung_start: u8,
    pub bung_pos: V4,
    pub bung_rot: V4,
    pub bung_scale: V4,
    pub bung_anm: Anm,
    /// `m_bEnabled`.
    pub enabled: bool,
    /// `BungAnm` drawn by the last `Draw` (twice).
    pub bung_drawn: bool,
}

/// An angle into -pi..pi, once each way.
fn wrap(mut v: F) -> F {
    if !ee::le(v, PI) {
        v = ee::sub(v, TWO_PI);
    }
    if ee::lt(v, NEG_PI) {
        v = ee::add(v, TWO_PI);
    }
    v
}

impl Hell {
    /// `Clear` and `Init(pos, dataSet)` (QUA gcmn 0x0036cdc0): the spline
    /// from `pos` up and back by the camera's turn (`rot`, the active
    /// camera's: `KyviaLV` 3 turns it the other way and further), the eye
    /// 1600 behind the place, three souls each 0.3 later than the one
    /// before, the burst's clip.
    pub fn new(
        cc: &mut dyn Rng,
        pos: V4,
        rot: V4,
        target: Option<usize>,
        kyvia_lv: i32,
        clips: &dyn Fn(&str) -> Option<(u32, bool)>,
    ) -> Hell {
        let mut h = Hell { target, kyvia_lv, enabled: true, ..Hell::default() };
        h.spoint = [pos; 4];
        let k = if kyvia_lv == 3 { 0xc000_0000 } else { 0x3f99_999a };
        let leg = |cc: &mut dyn Rng| {
            let r = rand_f(cc, 0x3f00_0000);
            let turn = wrap(ee::add(rot[2], ee::add(PI, r)));
            let m = geom::rot_matrix_z(&geom::unit_matrix(), turn);
            let m = geom::rot_matrix_x(&m, rand_f(cc, 0x3e4c_cccd));
            geom::apply_matrix(&m, [0, ee::mul(0x4448_0000, k), 0, ONE])
        };
        let mut v = leg(cc);
        v[2] = ee::add(v[2], 0x4348_0000);
        if kyvia_lv == 2 {
            v[2] = ee::add(v[2], 0x4396_0000);
        }
        h.spoint[2] = geom::vadd(h.spoint[3], v);
        let mut v = leg(cc);
        v[2] = ee::add(v[2], 0x4348_0000);
        if kyvia_lv == 2 {
            v[2] = ee::add(v[2], 0x4348_0000);
        }
        h.spoint[1] = geom::vadd(h.spoint[2], v);
        let mut v = leg(cc);
        v[2] = ee::sub(v[2], 0x4348_0000);
        h.spoint[0] = geom::vadd(h.spoint[1], v);
        let m = geom::rot_matrix_z(&geom::unit_matrix(), rot[2]);
        let z = ee::add(0x42c8_0000, ee::mul(0x4396_0000, ee::sub(ONE, k)));
        let v = geom::apply_matrix(&m, [0, 0x44c8_0000, z, ONE]);
        h.target_cam_pos = geom::vadd(h.spoint[3], v);
        h.act_proccess = [1; 5];
        h.end_count = 70;
        h.soul_num = SOULS as i32;
        h.bung_anm = Anm::default();
        h.bung_anm.speed = 256;
        h.bung_anm.set("ANM_ex31lhit", clips);
        h.bung_pos = h.spoint[3];
        h.bung_rot = VF0;
        h.bung_scale = [0x4100_0000, 0x4100_0000, 0x4100_0000, ONE];
        h.time[1] = 0xbe99_999a;
        h.time[2] = 0xbf19_999a;
        h
    }

    /// The generators `Init` makes after its members: each soul's three
    /// and the last's.
    pub fn start_gens(host: &mut dyn HellHost) {
        for k in 0..SOULS as u8 {
            for (j, g) in [HellGen::Soul(0), HellGen::Soul(1), HellGen::Soul(2)].into_iter().enumerate() {
                host.out(HellOut::Gen(g, Place::Follows(Follow::Soul(k), VF0), Some(Held::Soul(k, j as u8))));
            }
        }
        let last = (SOULS - 1) as u8;
        host.out(HellOut::Gen(HellGen::Last, Place::Follows(Follow::Soul(last), VF0), Some(Held::LastSoul)));
    }

    /// `HellStartScene` (QUA gcmn 0x0036d790): the gathering and the camera
    /// on the first soul; 70 frames; then true.
    fn start_scene(&mut self, host: &mut dyn HellHost) -> bool {
        match self.hell_act {
            0 => {
                for k in 0..2 {
                    host.out(HellOut::Gen(HellGen::Start(k), Place::At(self.spoint[0]), None));
                }
                let s = self.spoint;
                self.pos[0] = beam(self.time[0], [s[0], s[0], s[1], s[2]]);
                let view = geom::vadd([0, 0, 0x4348_0000, ONE], self.pos[0]);
                host.out(HellOut::FreeCam { pos: self.target_cam_pos, view });
                self.hell_act += 1;
                host.out(HellOut::SeNote(174, 36));
                false
            }
            1 => {
                self.hell_cou += 1;
                if self.hell_cou == 70 {
                    self.hell_act += 1;
                    self.hell_cou = 0;
                    host.out(HellOut::Gen(HellGen::Start(2), Place::At(self.spoint[0]), None));
                }
                if self.hell_cou == 10 {
                    host.out(HellOut::SeNote(174, 38));
                }
                false
            }
            2 => true,
            _ => false,
        }
    }

    /// One `Draw` (QUA gcmn 0x0036da40): after the scene, each soul along
    /// its three legs (0.02 a frame), the camera following the first; the
    /// first landing's light; as the last lands, the blur's colour fading
    /// to 0x60, then (after 70 frames) the burst, the callback and the
    /// souls' generators killed; the burst's clip, the blur back, the end.
    /// False once `m_bEnabled` clears.
    pub fn draw(&mut self, host: &mut dyn HellHost) -> bool {
        self.bung_drawn = false;
        if !self.start_scene(host) {
            return self.enabled;
        }
        if self.bung_start != 0 {
            let ended = self.bung_anm.forward();
            if !ended {
                self.bung_drawn = true;
                host.out(HellOut::DrawBung);
                host.out(HellOut::Quake([0x41a0_0000, 0x41a0_0000, 0x4220_0000]));
                return self.enabled;
            }
            let (abgr, scale) = host.blur();
            if !ee::lt(scale, ONE) {
                host.out(HellOut::Blur(Set::Scale(ee::sub(scale, 0x3a83_126f))));
            }
            if abgr < 0x3080_8080 {
                host.out(HellOut::Blur(Set::Scale(ONE)));
                self.enabled = false;
                host.out(HellOut::CamMode(6));
            } else {
                host.out(HellOut::Blur(Set::Abgr(abgr.wrapping_add(0xff00_0000))));
            }
            return self.enabled;
        }
        self.hell_cou += 1;
        if self.hell_cou == 15 {
            let n = 55 + host.rand() % 5;
            host.out(HellOut::SeNote(165, n as u8));
            self.hell_cou = 0;
        }
        let s = self.spoint;
        for k in 0..self.soul_num as usize {
            if !ee::lt(self.time[k], 0) {
                if !ee::le(self.time[k], ONE) {
                    self.time[k] = 0;
                    self.act_proccess[k] += 1;
                    if self.act_proccess[k] == 4 {
                        host.out(HellOut::SeNote(56, 60));
                    }
                }
                match self.act_proccess[k] {
                    1 => self.pos[k] = beam(self.time[k], [s[0], s[0], s[1], s[2]]),
                    2 => self.pos[k] = beam(self.time[k], [s[0], s[1], s[2], s[3]]),
                    3 => self.pos[k] = beam(self.time[k], [s[1], s[2], s[3], s[3]]),
                    4 => self.landed(host, k),
                    _ => {}
                }
                if k == 0 {
                    let view = geom::vadd([0, 0, 0x4348_0000, ONE], self.pos[0]);
                    host.out(HellOut::FreeCam { pos: self.target_cam_pos, view });
                }
            }
            if self.act_proccess[k] < 4 {
                self.time[k] = ee::add(self.time[k], 0x3ca3_d70a);
            }
        }
        self.enabled
    }

    /// A soul at its fourth step: the first's light (its step on); the
    /// last's, the colour down a step a frame from 0x50, and after 70
    /// frames once it is under 0x60808081 the burst.
    fn landed(&mut self, host: &mut dyn HellHost, k: usize) {
        if k == 0 {
            self.act_proccess[k] += 1;
            host.out(HellOut::Gen(HellGen::Landed, Place::At(self.spoint[3]), None));
        }
        if k + 1 != self.soul_num as usize {
            return;
        }
        let (mut abgr, _) = host.blur();
        if abgr >= 0x5080_8080 {
            abgr = abgr.wrapping_add(0xff00_0000);
        }
        host.out(HellOut::Blur(Set::Abgr(abgr)));
        let n = self.end_count;
        self.end_count -= 1;
        if n > 0 || abgr >= 0x6080_8081 {
            return;
        }
        self.bung_start = 1;
        host.out(HellOut::SeNote(69, 60));
        host.out(HellOut::SeNote(35, 48));
        host.out(HellOut::SeNote(56, 55));
        for j in 0..3 {
            host.out(HellOut::Gen(HellGen::Burst(j), Place::At(self.spoint[3]), None));
        }
        host.callback(self.target);
        host.out(HellOut::Kill(Held::LastSoul));
        for j in 0..self.soul_num as u8 {
            for g in 0..3 {
                host.out(HellOut::Kill(Held::Soul(j, g)));
            }
        }
    }
}

/// What hell's express does besides its own members.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HellOut {
    SeNote(i32, u8),
    Blur(Set),
    /// `bossCam->SetFreeCamPosView(pos, view)`, `SetMode(n)`, `QuakeCam(q)`
    /// (its three `ccRandF` drawn by the host).
    FreeCam {
        pos: V4,
        view: V4,
    },
    CamMode(i32),
    Quake([F; 3]),
    /// `new ccParticleGenerator` and its start (where, and the member it is
    /// kept in), and a kept one killed and let go
    /// (`ccCheckParticleGenerator` then `ParticleKill`).
    Gen(HellGen, Place, Option<Held>),
    Kill(Held),
    /// `BungAnm` drawn twice this frame.
    DrawBung,
}

/// Where hell's express runs: the blur it reads, `ccRand`, its callback.
pub trait HellHost {
    /// The blur's colour and scale as it reads them.
    fn blur(&self) -> (u32, F);
    fn rand(&mut self) -> i32;
    fn callback(&mut self, target: Option<usize>);
    fn out(&mut self, o: HellOut);
}

/// The fight's: the manager's blur, `Kyvia_HellE_Damage`.
pub(crate) struct FightHell<'a, 'c> {
    pub b: &'a mut BossBase,
    pub cx: &'a mut Cx<'c>,
    pub slot: usize,
    /// The blur as this frame's writes left it.
    pub abgr: u32,
    pub scale: F,
}

impl HellHost for FightHell<'_, '_> {
    fn blur(&self) -> (u32, F) {
        (self.abgr, self.scale)
    }

    fn rand(&mut self) -> i32 {
        self.cx.cc.rand()
    }

    fn callback(&mut self, target: Option<usize>) {
        super::fourth::hell_damage(self.b, self.cx, target);
    }

    fn out(&mut self, o: HellOut) {
        let cx = &mut *self.cx;
        match o {
            HellOut::SeNote(se, note) => cx.out(Out::SeNote { se, note }),
            HellOut::Blur(s) => {
                match s {
                    Set::Abgr(v) => self.abgr = v,
                    Set::Scale(v) => self.scale = v,
                    _ => {}
                }
                cx.out(Out::Blur(s));
            }
            HellOut::FreeCam { pos, view } => cx.out(Out::FreeCam { pos, view }),
            HellOut::CamMode(mode) => cx.out(Out::CamMode { mode }),
            HellOut::Quake(q) => cx.quake_vec(q),
            HellOut::Gen(g, at, held) => super::particles(cx, Gen::Hell(g), at, held.map(|h| slot_held(h, self.slot))),
            HellOut::Kill(h) => {
                let h = slot_held(h, self.slot);
                super::gen_op(cx, h, GenOp::Kill);
                super::gen_op(cx, h, GenOp::Forget);
            }
            HellOut::DrawBung => {}
        }
    }
}

/// A soul's generator as the fight keeps it (one hell's express at a time).
fn slot_held(h: Held, _slot: usize) -> Held {
    h
}

/// `HellSpecialExpressCreate(pos, &data)` into the manager: the camera
/// (its free mode) and the blur (0x65808080 at 1.01) set, the generators
/// made.
pub(super) fn create(b: &mut BossBase, cx: &mut Cx, pos: V4, target: Option<usize>, kyvia_lv: i32) {
    let rot = cx.cam.rot;
    let h = Hell::new(cx.cc, pos, rot, target, kyvia_lv, cx.clips);
    let kind = EffKind::HellExpress;
    let id = b.effects.create(kind);
    let slot = usize::try_from(id).ok();
    {
        let mut host = Setup { cx, slot: slot.unwrap_or(0) };
        Hell::start_gens(&mut host);
    }
    cx.out(Out::Blur(Set::Abgr(0x6580_8080)));
    cx.out(Out::Blur(Set::Scale(0x3f81_47ae)));
    cx.out(Out::CamMode { mode: 5 });
    if let Some(e) = slot.and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
        e.hell = Some(Box::new(h));
    }
    cx.out(Out::Effect { id, kind, pos, dirc: VF0 });
}

/// `Init`'s generators.
struct Setup<'a, 'c> {
    cx: &'a mut Cx<'c>,
    slot: usize,
}

impl HellHost for Setup<'_, '_> {
    fn blur(&self) -> (u32, F) {
        (0, ONE)
    }
    fn rand(&mut self) -> i32 {
        self.cx.cc.rand()
    }
    fn callback(&mut self, _: Option<usize>) {}
    fn out(&mut self, o: HellOut) {
        if let HellOut::Gen(g, at, held) = o {
            super::particles(self.cx, Gen::Hell(g), at, held.map(|h| slot_held(h, self.slot)));
        }
    }
}
