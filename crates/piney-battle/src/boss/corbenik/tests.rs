//! Corbenik's first form on a small scene: Kite and a member on
//! Quarantine's tables, every clip 60 frames long, the land everywhere.

use piney_data::volume::Volume;

use super::first::{self, First, act};
use super::seed::{self, Seed};
use super::{Pic, ROW, SEEDS};
use crate::affect::{self, AffectCtx};
use crate::boss::{Boss, BossData, BossEnv, CamView, Class, Cx, Out, Tbl, VF0};
use crate::chara::{AffectFunc, Char, Env};
use crate::enemy_ai::{Genrand, IDENTITY, World};
use crate::event::Events;
use crate::exp::Party;
use crate::param::{Base, SpcParam};
use crate::rand::Rand;
use crate::scene::Scene;
use crate::tables::Tables;
use crate::world::CharHit;

const ONE: u32 = 0x3f80_0000;

/// Land under every place.
struct Land;

impl crate::boss::PartHits for Land {
    fn switch(&mut self, _: usize, hit: &mut CharHit, on: bool) {
        hit.sw = on;
    }

    fn collide(&mut self, _: usize, hit: &mut CharHit) -> bool {
        hit.offset = [0; 4];
        false
    }

    fn sync(&mut self, _: usize, _: &CharHit) {}

    fn line(&mut self, _: [u32; 4], ep: [u32; 4], _: u32) -> Option<[u32; 4]> {
        Some(ep)
    }
}

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
    let b = f.with_cx(|cx| first::new(cx, [0, 0, 0, ONE], VF0, VF0)).0;
    f.scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(b));
    Some(f)
}

impl Fight {
    fn with_cx<R>(&mut self, run: impl FnOnce(&mut Cx) -> R) -> (R, Vec<Out>) {
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
            part_hits: &mut Land,
            game_over: false,
            boss_cam: false,
            cam: CamView::default(),
            land: &mut land,
            disc: crate::boss::DiscView::default(),
            gen_alive: &|_| true,
            me: self.me,
            out: Vec::new(),
            ev: Events::new(),
        };
        let r = run(&mut cx);
        (r, cx.out)
    }

    /// One frame of the boss's task: what it asked.
    fn frame(&mut self) -> Vec<Out> {
        self.count += 1;
        let me = self.me;
        let mut b = self.scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let (_, out) = self.with_cx(|cx| b.main(cx));
        self.scene.chars[me].foe_state_mut().unwrap().boss = Some(b);
        out
    }

    /// `EntryAffect(ty, p0, sid)` on `on` from Kite: what it asked.
    fn affect(&mut self, on: usize, ty: i16, p0: i16, sid: i16) -> Vec<Out> {
        let kite = self.party.members[0];
        let clip = |_: &str| Some((60, false));
        let env = Env { count: self.count, menu_type: -1, ..Env::default() };
        let check = |_: usize| 0;
        let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false, cc: None };
        let actx =
            AffectCtx { party: &self.party, menu: true, skill_check: &check, boss: Some(&benv), volume: Volume::Qua };
        let mut ev = Events::new();
        affect::entry_affect(&self.t, &mut self.scene, &actx, on, kite, ty, [p0, sid, 0], &mut self.rand, &mut ev);
        self.scene.chars[on].foe_state_mut().unwrap().boss.as_mut().unwrap().take_pending()
    }

    fn boss(&mut self) -> &mut Boss {
        self.scene.chars[self.me].foe_state_mut().unwrap().boss.as_mut().unwrap()
    }

    fn first(&mut self) -> First {
        match &self.boss().class {
            Class::Corbenik(x) => (**x).clone(),
            _ => panic!("not Corbenik"),
        }
    }

    fn seed(&self, s: usize) -> (Boss, Seed) {
        let b = self.scene.chars[s].foe_state().unwrap().boss.as_ref().unwrap();
        match &b.class {
            Class::CorbenikSeed(x) => ((**b).clone(), (**x).clone()),
            _ => panic!("not a seed"),
        }
    }
}

#[test]
fn stands_behind_kite_with_its_seeds_put_away() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    assert_eq!(f.scene.chars[me].pos[1], 0xc3fa_0000, "500 behind Kite");
    assert_eq!(f.scene.chars[me].pos[2], 0x4120_0000, "10 up");
    assert_eq!(f.boss().pat_tbl, Tbl::Normal);
    assert_eq!(f.boss().stop_time, 60, "the table's first wait");
    let x = f.first();
    assert_eq!(x.entry_seed_num, 1);
    for s in x.seed_me {
        let (b, _) = f.seed(s);
        assert_eq!((b.exit, b.act_num, b.act_forbid), (1, seed::act::WAIT, 3));
        assert!(!f.scene.listed(s), "a seed starts off the lists");
    }
}

#[test]
fn the_seeds_fly_count_down_and_burst() {
    let Some(mut f) = fight() else { return };
    let mut outs = Vec::new();
    for _ in 0..1200 {
        outs.extend(f.frame());
    }
    let thrown = outs.iter().filter(|o| matches!(o, Out::Se3dNote { se: 65, note: 65, .. })).count();
    assert!(thrown >= 3, "the seeds bounce ({thrown})");
    assert!(outs.iter().any(|o| matches!(o, Out::Corbenik(Pic::CountDown { n: 5, .. }))), "5 seconds at first");
    let bursts = outs.iter().filter(|o| matches!(o, Out::Corbenik(Pic::Burst { .. }))).count();
    assert!(bursts >= 3, "the seeds burst ({bursts})");
}

#[test]
fn its_death_takes_the_seeds_and_holds_their_counts() {
    let Some(mut f) = fight() else { return };
    for _ in 0..200 {
        f.frame();
    }
    let me = f.me;
    let live: Vec<usize> = f.first().seed_me.into_iter().filter(|&s| f.seed(s).0.exit == 0).collect();
    f.scene.chars[me].hp = 1;
    f.affect(me, 1, 10, 0);
    assert_eq!(f.boss().act_num, act::DIE);
    assert!(f.first().pause_cnt_down);
    for s in live {
        assert_eq!(f.seed(s).0.act_num, seed::act::DEAD);
    }
    let mut stream = false;
    for _ in 0..300 {
        stream |= f.frame().iter().any(|o| matches!(o, Out::StreamMenu { stream: 106, mask: 512 }));
    }
    assert!(stream, "menu 74's stream 106 at 250 frames");
}

#[test]
fn a_spell_on_a_seed_shows_its_shield() {
    let Some(mut f) = fight() else { return };
    // A seed lying still, back on the lists.
    let mut s = None;
    for _ in 0..600 {
        f.frame();
        s = f.first().seed_me.into_iter().find(|&k| f.scene.listed(k));
        if s.is_some() {
            break;
        }
    }
    let s = s.expect("a seed counts down");
    let spell = (1..f.t.skills.len() as i32).find(|&k| crate::skill::check_type_of(&f.t, k) == 1).unwrap();
    let out = f.affect(s, 1, 5, spell as i16);
    assert!(out.contains(&Out::Corbenik(Pic::SeedShield { seed: s })));
    let out = f.affect(s, 1, 5, 1);
    assert!(!out.iter().any(|o| matches!(o, Out::Corbenik(Pic::SeedShield { .. }))));
    assert_eq!(f.seed(s).0.act_num, seed::act::COUNT_DOWN, "its count goes on");
    assert_eq!(f.first().seed_me.len(), SEEDS);
}

#[test]
fn a_flush_shrinks_in_ten_steps() {
    let mut fl = super::eff::Flush::new(VF0, 0x447a_0000, 0);
    let Some(mut f) = fight() else { return };
    let mut n = 0;
    while f.with_cx(|cx| fl.draw(cx)).0 {
        n += 1;
        assert!(n < 100);
    }
    assert_eq!(n, 10, "ten draws on, off at the eleventh");
}

// --- the second form ----------------------------------------------------------------------------

fn fight_b() -> Option<Fight> {
    let mut f = fight()?;
    let me = f.me;
    let t = f.t.bosses[super::second::ROW].clone();
    let mut ch = Char::foe(t);
    ch.condition_num = -1;
    ch.affect.func = AffectFunc::Boss;
    f.scene.chars[me] = ch;
    let b = f.with_cx(|cx| super::second::new(cx, [0, 0, 0, ONE], VF0, VF0, Default::default())).0;
    f.scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(b));
    Some(f)
}

impl Fight {
    fn second(&mut self) -> super::second::Second {
        match &self.boss().class {
            Class::CorbenikB(x) => (**x).clone(),
            _ => panic!("not the second form"),
        }
    }
}

#[test]
fn the_second_form_stands_with_its_roots_put_away() {
    let Some(mut f) = fight_b() else { return };
    let x = f.second();
    assert!(!x.barrier);
    assert_eq!(f.boss().pat_tbl, Tbl::Normal);
    for r in x.root_me {
        let b = f.scene.chars[r].foe_state().unwrap().boss.as_ref().unwrap();
        assert_eq!(b.exit, 1, "a root starts exited");
    }
}

#[test]
fn its_hp_brings_the_super_table_and_asks_for_the_barrier() {
    let Some(mut f) = fight_b() else { return };
    let me = f.me;
    let max = f.scene.chars[me].max_hp;
    f.scene.chars[me].hp = max / 2 + 1;
    f.affect(me, 1, 2, 0);
    assert_eq!(f.boss().pat_tbl, Tbl::Super);
    let before = f.scene.chars[me].hp;
    f.affect(me, 1, before - max / 5, 0);
    let x = f.second();
    assert!(x.reserve_setup_barrier);
    assert_eq!(x.life_counter, i32::from(before), "the barrier's count is the HP before the blow");
}

#[test]
fn the_barrier_takes_the_blows_and_its_count_kills() {
    let Some(mut f) = fight_b() else { return };
    let me = f.me;
    let hp = f.scene.chars[me].hp;
    if let Class::CorbenikB(x) = &mut f.boss().class {
        x.barrier = true;
        x.life_counter = 4100;
    }
    let out = f.affect(me, 1, 200, 0);
    assert!(out.contains(&Out::FlyFont { kind: 2, n: 0 }), "a blow on the barrier shows 0");
    assert_eq!(f.scene.chars[me].hp, hp, "the body's HP untouched");
    let x = f.second();
    assert!(x.msg_shown && x.msg_count == 3630, "under 4000 the message and its wait");
    assert_eq!(f.boss().act_num, super::second::act::BARRIER_MSG);
    f.affect(me, 1, 30000, 0);
    assert_eq!(f.boss().act_num, super::second::act::DIE, "the count out: the death");
}
