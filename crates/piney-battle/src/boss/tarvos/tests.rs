//! Tarvos's rules on a small scene: Kite and a member on Quarantine's
//! tables, every clip 60 frames long.

use super::*;
use crate::affect::{self, AffectCtx};
use crate::boss::{BossData, BossEnv, CamView};
use crate::chara::{AffectFunc, Char, Env};
use crate::enemy_ai::{Genrand, IDENTITY, World};
use crate::event::Events;
use crate::exp::Party;
use crate::param::{Base, SpcParam};
use crate::rand::Rand;
use crate::scene::Scene;
use crate::tables::Tables;

struct Fight {
    t: Tables,
    data: BossData,
    scene: Scene,
    party: Party,
    me: usize,
    rand: Rand,
    cc: Genrand,
    count: u32,
}

fn pc(ty: i32, id: i16, x: f32) -> Char {
    let p = SpcParam {
        base: Base { ty, id, level: 70, ..Base::default() },
        max_hp: 1300,
        max_sp: 220,
        ..Default::default()
    };
    let mut c = Char::pc(p);
    c.pos = [x.to_bits(), 0, 0, ONE];
    c.pos_p = c.pos;
    c.affect.func = AffectFunc::None;
    c
}

fn fight() -> Option<Fight> {
    if !piney_data::store::work_tables(Volume::Qua).join("combat.bin").is_file() {
        return None;
    }
    let t = Tables::of(Volume::Qua);
    let data = BossData::of(Volume::Qua);
    let mut scene = Scene::default();
    let kite = scene.add(pc(7, 0, 0.0), 0);
    let member = scene.add(pc(6, 1, 300.0), 0);
    let mut ch = Char::foe(t.bosses[ROW].clone());
    ch.condition_num = -1;
    ch.affect.func = AffectFunc::Boss;
    let me = scene.add(ch, 3);
    let party = Party { members: [Some(kite), Some(member), None], ids: [0, 1, -1], num: 2 };
    let mut f = Fight { t, data, scene, party, me, rand: Rand(11), cc: Genrand::seeded(4357), count: 0 };
    let b = f.with_cx(|cx| new(cx, [0, 0, 0, ONE], VF0, VF0));
    f.scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(b));
    Some(f)
}

impl Fight {
    fn with_cx<R>(&mut self, run: impl FnOnce(&mut Cx) -> R) -> R {
        let clip = |_: &str| Some((60, false));
        let env = Env { count: self.count, menu_type: -1, ..Env::default() };
        let check = |_: usize| 0;
        let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false, cc: None };
        let actx =
            AffectCtx { party: &self.party, menu: true, skill_check: &check, boss: Some(&benv), volume: Volume::Qua };
        let mut none = |_| None;
        let mut land = |_| 0;
        let mut cx = Cx {
            t: &self.t,
            data: &self.data,
            scene: &mut self.scene,
            party: &self.party,
            world: World { player: self.party.members[0], frame: &IDENTITY, ..World::default() },
            env: &env,
            actx: &actx,
            rand: &mut self.rand,
            cc: &mut self.cc,
            clips: &clip,
            collide: &mut none,
            part_hits: &mut crate::boss::NoHits,
            game_over: false,
            boss_cam: true,
            cam: CamView { shake: true, ..CamView::default() },
            land: &mut land,
            disc: crate::boss::DiscView::default(),
            gen_alive: &|_| true,
            me: self.me,
            out: Vec::new(),
            ev: Events::new(),
        };
        run(&mut cx)
    }

    /// `EntryAffect(ty, p0, sid)` on the boss from Kite, drawing the
    /// fight's ccRand: what the boss asked.
    fn affect(&mut self, ty: i16, p0: i16, sid: i16) -> Vec<Out> {
        let (me, kite) = (self.me, self.party.members[0]);
        let clip = |_: &str| Some((60, false));
        let env = Env { count: self.count, menu_type: -1, ..Env::default() };
        let check = |_: usize| 0;
        let cc = std::cell::RefCell::new(std::mem::take(&mut self.cc));
        let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false, cc: Some(&cc) };
        let actx =
            AffectCtx { party: &self.party, menu: true, skill_check: &check, boss: Some(&benv), volume: Volume::Qua };
        let mut ev = Events::new();
        affect::entry_affect(&self.t, &mut self.scene, &actx, me, kite, ty, [p0, sid, 0], &mut self.rand, &mut ev);
        self.cc = cc.into_inner();
        self.boss().take_pending()
    }

    fn boss(&mut self) -> &mut Boss {
        self.scene.chars[self.me].foe_state_mut().unwrap().boss.as_mut().unwrap()
    }

    fn tarvos(&mut self) -> Tarvos {
        match &self.boss().class {
            Class::Tarvos(x) => (**x).clone(),
            _ => panic!("not Tarvos"),
        }
    }

    /// One of Tarvos's effects drawn until it ends (at most `n` frames):
    /// the frames and what it asked.
    fn draw_out(&mut self, mut fx: Fx, n: usize) -> (usize, Vec<Out>) {
        let me = self.me;
        let mut b = self.scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let mut outs = Vec::new();
        let mut frames = 0;
        while frames < n {
            frames += 1;
            let (alive, out) = self.with_cx(|cx| {
                let alive = fx.draw(cx, &mut b);
                (alive, std::mem::take(&mut cx.out))
            });
            outs.extend(out);
            if !alive {
                break;
            }
        }
        self.scene.chars[me].foe_state_mut().unwrap().boss = Some(b);
        (frames, outs)
    }
}

#[test]
fn stands_behind_kite_under_the_physical_guard() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    assert_eq!(f.scene.chars[me].pos[1], 0xc3fa_0000, "500 behind Kite");
    assert_eq!(f.boss().pat_tbl, Tbl::Normal);
    assert_eq!(f.boss().cheat_hp, 1);
    let x = f.tarvos();
    assert_eq!((x.pattern_mode, x.def_mode, x.stage_eff), (0, 0, -1));
    let guard = f.t.bosses[GUARD_ROWS[0]].elm;
    assert_eq!(f.scene.param(me).unwrap().real, guard, "the guard's stats");
}

#[test]
fn the_guard_holds_off_blows_of_its_kind() {
    let Some(mut f) = fight() else { return };
    // Skill 1, the normal attack, is physical (ccSkillCheckType 0).
    let out = f.affect(1, 10, 1);
    assert!(out.contains(&Out::Tarvos(Pic::Shield { magic: 0 })));
    let out = f.affect(3, 10, 0);
    assert!(!out.iter().any(|o| matches!(o, Out::Tarvos(Pic::Shield { .. }))), "no skill, no shield");
}

#[test]
fn half_the_gauge_brings_the_super_table() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    let max = f.t.bosses[GUARD_ROWS[0]].max_pp;
    f.scene.param_mut(me).unwrap().pp = max;
    f.affect(1, 10, 0);
    assert_eq!(f.boss().pat_tbl, Tbl::Super);
    assert_eq!(f.tarvos().pattern_mode, 1);
}

#[test]
fn the_drain_drops_the_guard_for_the_epitaph() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    f.affect(21, 0, 0);
    assert_eq!(f.boss().act_num, act::EPITAPH);
    assert_eq!(f.boss().pat_tbl, Tbl::Epitaph);
    assert_eq!(f.tarvos().def_mode, 2);
    assert_eq!((f.scene.chars[me].hp, f.scene.chars[me].max_hp), (EPITAPH_HP, EPITAPH_HP));
    assert_eq!(f.scene.param(me).unwrap().pp, -1);
}

#[test]
fn the_spike_rain_reaches_kite_and_fades() {
    let Some(mut f) = fight() else { return };
    let kite = f.party.members[0].unwrap();
    let fx = f.with_cx(|cx| Fx::Spikes(Box::new(eff::SpikeRain::new(cx, kite))));
    let (frames, out) = f.draw_out(fx, 600);
    assert!(frames < 600, "the rain ends");
    let first = out.iter().filter(|o| matches!(o, Out::Se3d { se: 63, .. })).count();
    assert_eq!(first, 1, "the first flight sounds once");
    let erupts = out.iter().filter(|o| matches!(o, Out::Tarvos(Pic::Eruption { .. }))).count();
    assert_eq!(erupts, 4, "every eighth spike erupts");
}

#[test]
fn the_phantom_rings_and_bursts_its_fourteen_lights() {
    let Some(mut f) = fight() else { return };
    let fx = f.with_cx(|cx| Fx::Phantom(Box::new(eff::Phantom::new(cx, [0, 0, 0, ONE], 0))));
    let (frames, out) = f.draw_out(fx, 2000);
    assert!(frames < 2000, "the phantom ends");
    let ring = out.iter().filter(|o| matches!(o, Out::Effect { kind: EffKind::AutoSamonRingOf { n: 195, .. }, .. }));
    assert_eq!(ring.count(), 1);
    let bursts = out.iter().filter(|o| matches!(o, Out::Tarvos(Pic::Burst { .. }))).count();
    assert_eq!(bursts, 14);
    assert_eq!(out.last(), Some(&Out::CamMode { mode: 6 }), "the camera back last");
}
