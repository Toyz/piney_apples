//! Gorre's rules on a small scene: Kite and a member on Outbreak's tables,
//! every clip 60 frames long.

use super::*;
use crate::affect::AffectCtx;
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

    /// A frame of the manager's pass and `Main`. Its own skills end at
    /// once.
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

    fn boss(&mut self) -> &mut Boss {
        self.scene.chars[self.me].foe_state_mut().unwrap().boss.as_mut().unwrap()
    }

    fn gorre(&mut self) -> &mut Gorre {
        let Class::Gorre(x) = &mut self.boss().class else { panic!("not Gorre") };
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
}

#[test]
fn stands_behind_kite_with_two_brothers_beside_it() {
    let Some(mut f) = fight() else { return };
    let me = f.me;
    assert_eq!(f.scene.chars[me].pos[1], (-500.0f32).to_bits(), "500 behind Kite");
    assert!(f.scene.ene_list.contains(&me));
    assert_eq!(f.boss().pat_tbl, Tbl::Normal);
    assert_eq!(f.boss().act_num, act::NEUTRAL);
    let br = f.gorre().brother_me;
    assert_ne!(br[0], br[1], "two distinct brothers");
    for m in br {
        assert!(f.scene.ene_list.contains(&m), "each brother entered");
        let Some(bb) = f.scene.chars[m].foe_state().unwrap().boss.as_ref() else { panic!("no boss on the brother") };
        assert!(matches!(bb.class, Class::GorreBrother(_)));
    }
}

#[test]
fn runs_its_first_pattern_without_panicking() {
    let Some(mut f) = fight() else { return };
    f.until(400, |_| false);
    // Reaching here (no panic, no infinite recursion) is the assertion:
    // the pattern table, Think's dispatch and both brothers' frames all
    // ran across several act changes.
    assert!(f.boss().pat_num != 0 || f.boss().act_num != act::NEUTRAL);
}

#[test]
fn both_brothers_down_drains_gorre() {
    let Some(mut f) = fight() else { return };
    let br = f.gorre().brother_me;
    for m in br {
        f.scene.chars[m].hp = 0;
    }
    f.frame();
    assert_eq!(f.boss().epitaph, 1);
    assert_eq!(f.boss().pat_tbl, Tbl::Epitaph);
}

#[test]
fn both_brothers_down_ends_the_fight() {
    let Some(mut f) = fight() else { return };
    let br = f.gorre().brother_me;
    for m in br {
        f.scene.chars[m].hp = 0;
    }
    f.until(EPITAPH_GRACE as usize + 300, |f| f.boss().exit != 0);
    assert_ne!(f.boss().exit, 0, "Gorre exits once its Epitaph's grace runs out");
    assert!(!f.scene.ene_list.contains(&f.me), "off the lists");
}
