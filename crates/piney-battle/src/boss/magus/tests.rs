//! Magus's rules on a small scene: Kite and a member on Mutation's tables,
//! every clip 60 frames long.

use super::leaf::{Leaf, act as leaf_act};
use super::*;
use crate::affect::{self, AffectCtx};
use crate::boss::{BossData, BossEnv, CamView, VF0};
use crate::chara::{AffectFunc, Char, Env};
use crate::enemy_ai::{Genrand, IDENTITY, World};
use crate::event::Events;
use crate::exp::Party;
use crate::param::{Base, SpcParam};
use crate::rand::Rand;
use crate::scene::Scene;
use crate::tables::Tables;
use piney_data::volume::Volume;

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
    if !piney_data::store::work_tables(Volume::Mut).join("combat.bin").is_file() {
        return None;
    }
    let t = Tables::of(Volume::Mut);
    let data = BossData::of(Volume::Mut);
    let mut scene = Scene::default();
    let kite = scene.add(pc(7, 0, 0.0), 0);
    let member = scene.add(pc(6, 1, 300.0), 0);
    let mut ch = Char::foe(t.bosses[ROW].clone());
    ch.condition_num = -1;
    ch.affect.func = AffectFunc::Boss;
    let me = scene.add(ch, 3);
    let party = Party { members: [Some(kite), Some(member), None], ids: [0, 1, -1], num: 2 };
    let mut f = Fight { t, data, scene, party, me, rand: Rand(11), cc: Genrand::seeded(4357), count: 0 };
    let mut b = f.with_cx(|cx| new(cx, [0, 0, 0, ONE], VF0, VF0));
    if let Class::Magus(x) = &mut b.class {
        for (k, p) in x.leaf_pos.iter_mut().enumerate() {
            *p = [(k as f32 * 40.0 - 200.0).to_bits(), (-500.0f32).to_bits(), 300.0f32.to_bits(), ONE];
        }
    }
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
            AffectCtx { party: &self.party, menu: true, skill_check: &check, boss: Some(&benv), volume: Volume::Mut };
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
            boss_cam: false,
            cam: CamView::default(),
            land: &mut land,
            disc: crate::boss::DiscView::default(),
            me: self.me,
            out: Vec::new(),
            ev: Events::new(),
        };
        run(&mut cx)
    }

    /// A frame of the manager's pass and `Main`: what it asked.
    fn frame(&mut self) -> Vec<Out> {
        self.count += 1;
        let me = self.me;
        let mut b = self.scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let mut out = b.take_pending();
        out.extend(self.with_cx(|cx| {
            b.main(cx);
            std::mem::take(&mut cx.out)
        }));
        self.scene.chars[me].foe_state_mut().unwrap().boss = Some(b);
        out
    }

    /// `EntryAffect(ty, p0)` on `on` from Kite.
    fn affect_on(&mut self, on: usize, ty: i16, p0: i16) {
        let kite = self.party.members[0];
        let clip = |n: &str| Some((60, n.contains("nut")));
        let env = Env { count: self.count, menu_type: -1, ..Env::default() };
        let check = |_: usize| 0;
        let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false };
        let actx =
            AffectCtx { party: &self.party, menu: true, skill_check: &check, boss: Some(&benv), volume: Volume::Mut };
        let mut ev = Events::new();
        affect::entry_affect(&self.t, &mut self.scene, &actx, on, kite, ty, [p0, 0, 0], &mut self.rand, &mut ev);
    }

    fn boss(&self) -> &Boss {
        self.scene.chars[self.me].foe_state().unwrap().boss.as_ref().unwrap()
    }

    fn magus(&mut self) -> &mut Magus {
        let b = self.scene.chars[self.me].foe_state_mut().unwrap().boss.as_mut().unwrap();
        let Class::Magus(x) = &mut b.class else { panic!("not Magus") };
        x
    }

    fn leaves(&mut self) -> Vec<usize> {
        self.magus().leaves.clone()
    }

    fn leaf(&self, l: usize) -> (&Boss, &Leaf) {
        let b = self.scene.chars[l].foe_state().unwrap().boss.as_ref().unwrap();
        let Class::MagusLeaf(x) = &b.class else { panic!("not a leaf") };
        (b, x)
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
fn stands_behind_kite_with_twelve_leaves_off_the_lists() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    assert_eq!(f.scene.chars[me].pos[1], (-500.0f32).to_bits(), "500 behind Kite");
    assert!(f.scene.ene_list.contains(&me));
    let leaves = f.leaves();
    assert_eq!(leaves.len(), LEAVES);
    for &l in &leaves {
        assert!(!f.scene.listed(l));
        assert_eq!(f.leaf(l).0.exit, 1);
        assert_eq!(f.leaf(l).0.act_num, leaf_act::NEUTRAL);
    }
    assert_eq!(f.magus().reserve_leaf_drop, 1);
    assert_eq!(f.magus().dmg_count, 100);
    f.frame();
    assert_eq!(f.boss().act_num, act::LEAF_DROP, "the first neutral drops the leaves");
}

#[test]
fn a_dropped_leaf_lands_on_the_lists_and_counts_down() {
    let Some(mut f) = fight() else { return };
    let leaves = f.leaves();
    let l0 = leaves[0];
    f.until(200, |f| f.leaf(l0).1.drop != 0);
    assert_eq!(f.leaf(l0).0.act_num, leaf_act::DROP);
    assert_eq!(f.scene.chars[l0].pos[2], 0, "on the ground under its place");
    f.until(100, |f| f.leaf(l0).0.act_num == leaf_act::COUNT_DOWN);
    assert!(f.scene.listed(l0), "landed: on the lists");
    assert_eq!(f.leaf(l0).1.exp_cntr, 450);
}

#[test]
fn a_leaf_hit_to_death_fades_and_goes() {
    let Some(mut f) = fight() else { return };
    let l0 = f.leaves()[0];
    f.until(400, |f| f.scene.listed(l0));
    f.affect_on(l0, 1, 30000);
    assert_eq!(f.scene.chars[l0].hp, 0);
    assert_eq!(f.leaf(l0).0.act_num, leaf_act::DIE);
    let outs = f.until(60, |f| f.leaf(l0).0.exit != 0);
    assert_eq!(f.leaf(l0).0.exit, 1);
    assert!(!f.scene.listed(l0));
    assert!(outs.iter().any(|o| matches!(o, Out::Effect { kind: EffKind::LeafRing, .. })));
}

#[test]
fn a_leaf_left_alone_bursts_and_magus_rises_then_falls() {
    let Some(mut f) = fight() else { return };
    let l0 = f.leaves()[0];
    let mut rose = false;
    let outs = f.until(2000, |f| {
        rose |= f.boss().act_num == act::RISE;
        rose && f.boss().act_num == act::NEUTRAL
    });
    assert!(rose, "the countdown's end lifts Magus");
    assert_eq!(f.boss().act_num, act::NEUTRAL);
    assert_eq!(f.leaf(l0).1.explode, 1);
    assert_eq!(f.leaf(l0).0.exit, 1);
    assert!(outs.contains(&Out::Cinema(Some(10))));
    assert!(outs.contains(&Out::CameraChange(1)), "camera 1 once down");
}

#[test]
fn drained_to_the_epitaph_then_dies_and_exits() {
    let Some(mut f) = fight() else { return };
    f.frame();
    let me = f.me;
    f.affect_on(me, 13, 0);
    f.frame();
    assert_eq!(f.boss().act_num, act::EPITAPH);
    assert_eq!(f.magus().epitaph, 1);
    assert_eq!(f.boss().cheat_hp, 0);
    for l in f.leaves() {
        assert_eq!(f.leaf(l).0.exit, 1, "the leaves go");
    }
    f.affect_on(me, 21, 0);
    assert_eq!(f.scene.chars[me].hp, 4500);
    let mut dead = false;
    for _ in 0..400 {
        if f.boss().act_num != act::DIE {
            f.affect_on(me, 1, 1000);
        }
        f.frame();
        dead |= f.boss().act_num == act::DIE;
        if f.boss().exit != 0 {
            break;
        }
    }
    assert!(dead, "hit to death");
    assert_eq!(f.boss().exit, 1, "the dead effect over, the task's exit");
    assert!(!f.scene.ene_list.contains(&me));
}

#[test]
fn past_half_the_gauge_the_laser_comes() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    let max = f.t.bosses[ROW].max_pp;
    f.scene.chars[me].foe_state_mut().unwrap().pp = max;
    f.affect_on(me, 1, 10);
    assert_eq!(f.magus().reserve_laser, 1);
    assert_eq!(f.magus().high_drive, 1);
    f.frame();
    assert_eq!(f.boss().act_num, act::LASER, "no leaf down: the laser");
    let outs = f.until(600, |f| f.boss().act_num == act::NEUTRAL);
    assert_eq!(f.boss().act_num, act::NEUTRAL);
    assert_eq!(f.magus().cam_id, 1);
    assert!(outs.contains(&Out::Cinema(Some(9))));
    assert_eq!(outs.iter().filter(|o| matches!(o, Out::Magus(Pic::Thunder { .. }))).count(), 12);
    assert_eq!(f.boss().lock_player, 0);
}
