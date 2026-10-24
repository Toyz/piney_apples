//! Macha's rules on a small scene: Kite and a member on Quarantine's
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
            me: self.me,
            out: Vec::new(),
            ev: Events::new(),
        };
        run(&mut cx)
    }

    /// A frame of the manager's pass and `Main`: what it asked. Its own
    /// skills end at once.
    fn frame(&mut self) -> Vec<Out> {
        self.count += 1;
        let me = self.me;
        self.scene.chars[me].skill_id = 0;
        let mut b = self.scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let mut out = b.take_pending();
        out.extend(self.with_cx(|cx| {
            b.main(cx);
            std::mem::take(&mut cx.out)
        }));
        self.scene.chars[me].foe_state_mut().unwrap().boss = Some(b);
        out
    }

    /// `EntryAffect(ty, p0)` on the boss from Kite.
    fn affect(&mut self, ty: i16, p0: i16) {
        let (me, kite) = (self.me, self.party.members[0]);
        let clip = |_: &str| Some((60, false));
        let env = Env { count: self.count, menu_type: -1, ..Env::default() };
        let check = |_: usize| 0;
        let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false, cc: None };
        let actx =
            AffectCtx { party: &self.party, menu: true, skill_check: &check, boss: Some(&benv), volume: Volume::Qua };
        let mut ev = Events::new();
        affect::entry_affect(&self.t, &mut self.scene, &actx, me, kite, ty, [p0, 0, 0], &mut self.rand, &mut ev);
    }

    fn boss(&mut self) -> &mut Boss {
        self.scene.chars[self.me].foe_state_mut().unwrap().boss.as_mut().unwrap()
    }

    /// Frames until `done` holds (at most `n`).
    fn until(&mut self, n: usize, mut done: impl FnMut(&mut Fight) -> bool) -> Vec<Out> {
        let mut outs = Vec::new();
        for _ in 0..n {
            outs.extend(self.frame());
            if done(self) {
                break;
            }
        }
        outs
    }
}

#[test]
fn starts_on_its_normal_table_with_the_cheat() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    assert!(f.scene.ene_list.contains(&me));
    assert_eq!(f.boss().pat_tbl, Tbl::Normal);
    assert_eq!(f.boss().act_num, act::NEUTRAL);
    assert_eq!(f.boss().cheat_hp, 1);
    f.until(600, |f| f.boss().act_num != act::NEUTRAL);
    assert_ne!(f.boss().act_num, act::NEUTRAL, "a pattern within ten seconds");
}

#[test]
fn a_hit_under_the_cheat_takes_a_tenth() {
    let Some(mut f) = fight() else { return };
    f.frame();
    let me = f.me;
    let hp = f.scene.chars[me].hp;
    f.affect(1, 1000);
    assert_eq!(f.scene.chars[me].hp, hp - 100);
}

#[test]
fn half_the_gauge_brings_the_expert_table() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    let max = f.t.bosses[ROW].max_pp;
    f.scene.param_mut(me).unwrap().pp = max;
    f.affect(1, 10);
    assert_eq!(f.boss().pat_tbl, Tbl::Super);
}

#[test]
fn the_shock_fades_in_thirty_frames_and_bursts_in_eights() {
    let Some(mut f) = fight() else { return };
    let mut fx = eff::Fx::Shock(eff::Shock::new([0, 0, 0, ONE]));
    let mut frames = 0;
    let mut bursts = 0;
    while frames < 100 {
        frames += 1;
        let (alive, out) = f.with_cx(|cx| {
            let alive = fx.draw(cx);
            (alive, std::mem::take(&mut cx.out))
        });
        bursts += out.iter().filter(|o| matches!(o, Out::Macha(Pic::Burst { .. }))).count();
        if !alive {
            break;
        }
    }
    // 1.0 less thirty of the float thirtieth (0x3d088889, a hair over)
    // falls under 0 on the 30th draw.
    assert_eq!(frames, 30);
    assert!(bursts > 0 && bursts % 8 == 0);
}
