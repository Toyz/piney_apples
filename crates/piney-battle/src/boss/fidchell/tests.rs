//! Fidchell's rules on a small scene: Kite and a member on Outbreak's
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
    let p =
        SpcParam { base: Base { ty, id, level: 50, ..Base::default() }, max_hp: 900, max_sp: 90, ..Default::default() };
    let mut c = Char::pc(p);
    c.pos = [x.to_bits(), 0, 0, ONE];
    c.pos_p = c.pos;
    c.affect.func = AffectFunc::None;
    c
}

fn fight() -> Option<Fight> {
    if !piney_data::store::work_tables(Volume::Out).join("combat.bin").is_file() {
        return None;
    }
    let t = Tables::of(Volume::Out);
    let data = BossData::of(Volume::Out);
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
        let clip = |n: &str| Some((60, n.contains("nut")));
        let env = Env { count: self.count, menu_type: -1, ..Env::default() };
        let check = |_: usize| 0;
        let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false };
        let actx =
            AffectCtx { party: &self.party, menu: true, skill_check: &check, boss: Some(&benv), volume: Volume::Out };
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
        let clip = |n: &str| Some((60, n.contains("nut")));
        let env = Env { count: self.count, menu_type: -1, ..Env::default() };
        let check = |_: usize| 0;
        let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false };
        let actx =
            AffectCtx { party: &self.party, menu: true, skill_check: &check, boss: Some(&benv), volume: Volume::Out };
        let mut ev = Events::new();
        affect::entry_affect(&self.t, &mut self.scene, &actx, me, kite, ty, [p0, 0, 0], &mut self.rand, &mut ev);
    }

    fn boss(&mut self) -> &mut Boss {
        self.scene.chars[self.me].foe_state_mut().unwrap().boss.as_mut().unwrap()
    }

    fn fid(&mut self) -> &mut Fidchell {
        let Class::Fidchell(x) = &mut self.boss().class else { panic!("not Fidchell") };
        x
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

    /// The spell `pred` cast at once (pattern 15 with `m_predId` set).
    fn cast(&mut self, pred: i32) -> Vec<Out> {
        self.fid().pred_id = pred;
        let me = self.me;
        let mut b = self.scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let mut x = fidchell_of(&mut b);
        self.with_cx(|cx| exec_pattern(&mut b, &mut x, cx, 15, 0));
        b.class = Class::Fidchell(x);
        self.scene.chars[me].foe_state_mut().unwrap().boss = Some(b);
        self.until(3000, |f| f.boss().act_num != act::EXEC_PREDICTION)
    }
}

#[test]
fn stands_behind_kite_and_waits_its_first_pattern() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    assert_eq!(f.scene.chars[me].pos[1], (-500.0f32).to_bits(), "500 behind Kite");
    assert_eq!(f.scene.chars[me].pos[2], 10.0f32.to_bits(), "10 up");
    assert!(f.scene.ene_list.contains(&me));
    assert_eq!(f.boss().pat_tbl, Tbl::Normal);
    assert_eq!(f.boss().act_num, act::NEUTRAL);
    assert_eq!(f.boss().stop_time, 60, "the table's first word: wait 60");
    assert_eq!(f.boss().cheat_hp, 1);
    f.until(80, |f| f.boss().act_num != act::NEUTRAL);
    assert_eq!(f.boss().act_num, act::CHASE, "then word 4: the chase");
}

#[test]
fn the_prediction_lays_its_skill_on_the_members_then_its_spell_comes() {
    let Some(mut f) = fight() else { return };
    let mut predicted = None;
    let outs = f.until(6000, |f| {
        if f.boss().act_num == act::PREDICTION && predicted.is_none() && f.boss().act_proccess > 0 {
            predicted = Some(f.fid().pred_id);
        }
        predicted.is_some() && f.boss().act_num == act::EXEC_PREDICTION
    });
    let Some(p) = predicted else { panic!("no prediction") };
    assert_eq!(f.boss().act_num, act::EXEC_PREDICTION, "pattern 15 casts it");
    assert!(outs.contains(&Out::Cinema(Some(PRED_CINEMA[p as usize]))));
    let sid = f.data.fidchell.pred_skills[p as usize];
    assert_eq!(outs.iter().filter(|o| matches!(o, Out::Skill(_, s) if s.sid == sid)).count(), 2, "each member");
    assert!(outs.iter().any(|o| matches!(o, Out::Fidchell(Pic::Voice(_)))));
    assert_eq!(f.fid().reserve_pred, 0);
}

#[test]
fn each_spell_makes_its_effect_and_ends_unlocked() {
    let kinds = [
        |k: &EffKind| matches!(k, EffKind::MeteoSworm { n: 50 }),
        |k: &EffKind| matches!(k, EffKind::IceBreak),
        |k: &EffKind| matches!(k, EffKind::ThunderStorm { n: 16 }),
        |k: &EffKind| matches!(k, EffKind::RockTower { n: 128 }),
    ];
    for (pred, made) in kinds.iter().enumerate() {
        let Some(mut f) = fight() else { return };
        f.frame();
        let outs = f.cast(pred as i32);
        assert_ne!(f.boss().act_num, act::EXEC_PREDICTION, "spell {pred} ends");
        assert!(outs.iter().any(|o| matches!(o, Out::Effect { kind, .. } if made(kind))), "spell {pred}'s effect");
        assert!(outs.contains(&Out::Cinema(Some(SPELL_CINEMA[pred]))));
        assert_eq!(f.boss().lock_player, 0);
        assert_eq!(f.boss().draw_sw, 1, "shown again");
        assert_eq!(f.boss().transparency, ONE, "faded back in");
    }
}

#[test]
fn half_the_gauge_brings_the_super_table() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    let max = f.t.bosses[ROW].max_pp;
    f.scene.chars[me].foe_state_mut().unwrap().pp = max;
    f.affect(1, 10);
    assert_eq!(f.boss().pat_tbl, Tbl::Super);
    assert_eq!(f.fid().pat_mode, 1);
}

#[test]
fn drained_to_the_epitaph_then_dies_and_exits() {
    let Some(mut f) = fight() else { return };
    f.frame();
    let me = f.me;
    f.affect(13, 0);
    f.frame();
    assert_eq!(f.boss().act_num, act::EPITAPH);
    assert_eq!(f.boss().epitaph, 1);
    assert_eq!(f.boss().cheat_hp, 0);
    assert_eq!(f.boss().pat_tbl, Tbl::Epitaph);
    f.affect(21, 0);
    assert_eq!(f.scene.chars[me].hp, EPITAPH_HP);
    let mut dead = false;
    for _ in 0..3000 {
        if f.boss().act_num != act::DEAD && f.boss().lock_player == 0 {
            f.affect(1, 1000);
        }
        f.frame();
        dead |= f.boss().act_num == act::DEAD;
        if f.boss().exit != 0 {
            break;
        }
    }
    assert!(dead, "hit to death");
    assert_eq!(f.boss().exit, 1, "the dead effect over, the task's exit");
    assert!(!f.scene.ene_list.contains(&me));
}
