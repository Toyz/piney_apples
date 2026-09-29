use super::*;
use crate::affect::{self, AffectCtx};
use crate::boss::{BossData, BossEnv, CamView, DiscView};
use crate::chara::{AffectFunc, Char, Env};
use crate::enemy_ai::{Genrand, IDENTITY, World};
use crate::event::Events;
use crate::exp::Party;
use crate::param::{Base, SpcParam};
use crate::rand::Rand;
use crate::scene::Scene;
use crate::tables::Tables;
use piney_data::volume::Volume;

/// Kyvia over Kite and a member on its volume's tables, the disc at the
/// origin and the body 3000 off it.
struct Bout {
    volume: Volume,
    t: Tables,
    data: BossData,
    scene: Scene,
    party: Party,
    me: usize,
    rand: Rand,
    cc: Genrand,
    count: u32,
    disc: DiscView,
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

fn fight() -> Option<Bout> {
    bout(Volume::Mut, Fight::First)
}

/// The fight `which` on `volume`'s tables.
fn bout(volume: Volume, which: Fight) -> Option<Bout> {
    if !piney_data::store::work_tables(volume).join("combat.bin").is_file() {
        return None;
    }
    let t = Tables::of(volume);
    let data = BossData::of(volume);
    let mut scene = Scene::default();
    let kite = scene.add(pc(7, 0, 0.0), 0);
    let member = scene.add(pc(6, 1, 300.0), 0);
    let mut ch = Char::foe(t.bosses[which.row()].clone());
    ch.condition_num = -1;
    ch.affect.func = AffectFunc::Boss;
    let me = scene.add(ch, 3);
    let party = Party { members: [Some(kite), Some(member), None], ids: [0, 1, -1], num: 2 };
    let disc = DiscView { moving: true, prev_pos: [0, 0, 0, ONE], marker: [0, 0x453b_8000, 0x4348_0000, ONE] };
    let mut f = Bout { volume, t, data, scene, party, me, rand: Rand(7), cc: Genrand::seeded(4357), count: 0, disc };
    let b = f.with_cx(|cx| new(cx, which));
    f.scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(b));
    Some(f)
}

impl Bout {
    fn with_cx<R>(&mut self, run: impl FnOnce(&mut Cx) -> R) -> R {
        let clip = |_: &str| Some((60, false));
        let env = Env { count: self.count, menu_type: -1, ..Env::default() };
        let check = |_: usize| 0;
        let benv = BossEnv { t: &self.t, data: &self.data, clips: &clip, env: &env, game_over: false };
        let actx =
            AffectCtx { party: &self.party, menu: true, skill_check: &check, boss: Some(&benv), volume: self.volume };
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
            cam: CamView::default(),
            land: &mut land,
            disc: self.disc,
            me: self.me,
            out: Vec::new(),
            ev: Events::new(),
        };
        run(&mut cx)
    }

    /// A frame of the body (the manager's pass and `Main`): what it asked.
    fn frame(&mut self) -> Vec<Out> {
        self.count += 1;
        let me = self.me;
        let mut b = self.scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let out = self.with_cx(|cx| {
            b.main(cx);
            std::mem::take(&mut cx.out)
        });
        self.scene.chars[me].foe_state_mut().unwrap().boss = Some(b);
        out
    }

    fn kyvia(&self) -> &Kyvia {
        let b = self.scene.chars[self.me].foe_state().unwrap().boss.as_ref().unwrap();
        let Class::Kyvia(x) = &b.class else { panic!("not Kyvia") };
        x
    }

    fn part(&self, who: usize) -> &Boss {
        self.scene.chars[who].foe_state().unwrap().boss.as_ref().unwrap()
    }

    fn core(&self) -> &Core {
        let Class::KyviaCore(x) = &self.part(self.kyvia().core).class else { panic!("not the core") };
        x
    }

    fn gomora(&self, k: usize) -> (&Boss, &Gomora) {
        let b = self.part(self.core().gomoras[k]);
        let Class::Gomora(x) = &b.class else { panic!("not a gomora") };
        (b, x)
    }
}

#[test]
fn stands_at_the_marker_with_its_core_and_gomoras() {
    let Some(f) = fight() else { return };
    let me = f.me;
    assert_eq!(f.scene.chars[me].pos, f.disc.marker, "DMY_marker01");
    let x = f.kyvia();
    assert_eq!((x.st_flg, x.atk_pat_mode, x.disc_max_lv), (1, 1, 1));
    let c = f.core();
    assert_eq!(c.gomoras.len(), 5);
    assert_eq!(c.kyvia_lv, 1);
    let core = x.core;
    assert_eq!(f.part(core).exit, 1, "the core waits for the disc");
    assert_eq!(x.max_hp, i32::from(f.scene.chars[core].max_hp));
    assert_eq!(x.switch_hp, x.max_hp / 4);
    assert_eq!(f.scene.chars[core].pos, f.scene.chars[f.party.members[0].unwrap()].pos, "the core at Kite");
    for k in 0..5 {
        let (b, g) = f.gomora(k);
        assert_eq!((b.exit, g.state, g.slave_id), (1, 2, k as i32));
    }
    assert_eq!(f.data.kyvia.gomora_lists[0], [3, 3, 3, 2, 0]);
    assert!(!f.scene.listed(core), "no target before it rises");
}

#[test]
fn the_core_rises_once_the_disc_stops() {
    let Some(mut f) = fight() else { return };
    let out = f.frame();
    assert!(out.iter().any(|o| matches!(o, Out::FreeCam { .. })), "the camera rides with the disc");
    f.disc.moving = false;
    let out = f.frame();
    assert!(out.contains(&Out::CamMode { mode: 6 }));
    for _ in 0..49 {
        f.frame();
    }
    let core = f.kyvia().core;
    assert_eq!(f.kyvia().st_flg, 0);
    assert_eq!(f.part(core).exit, 0, "EntrySlave after 50 frames");
    // Its fade in: 0.02 a frame, then ActionStart puts it on the lists.
    for _ in 0..60 {
        f.frame();
    }
    assert_eq!(f.core().alpha, ONE);
    assert!(f.scene.listed(core));
    assert!(f.core().aura && f.core().aura2);
}

#[test]
fn gomoras_come_out_in_turn() {
    let Some(mut f) = fight() else { return };
    f.disc.moving = false;
    for _ in 0..(50 + 50 + 61 + 31) {
        f.frame();
    }
    let c = f.core();
    assert_eq!(c.gomora_meter, 1, "the first gomora after 60 and 31 frames (`GsCount++ == 30`)");
    let (b, g) = f.gomora(0);
    assert_eq!(b.exit, 0);
    assert_eq!(g.my_attribute, 3, "list 0's first: the dash");
    for _ in 0..(4 * 31) {
        f.frame();
    }
    assert_eq!(f.core().gomora_meter, 5);
    assert_eq!(f.core().gomora_hold, 2);
    let attrs: Vec<i32> = (0..5).map(|k| f.gomora(k).1.my_attribute).collect();
    assert_eq!(attrs, [3, 3, 3, 2, 0]);
}

#[test]
fn a_hit_on_the_core_shows_its_resistance() {
    let Some(mut f) = fight() else { return };
    f.disc.moving = false;
    for _ in 0..160 {
        f.frame();
    }
    let core = f.kyvia().core;
    let kite = f.party.members[0];
    let hp = f.scene.chars[core].hp;
    let clip = |_: &str| Some((60, false));
    let env = Env { count: 1, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let benv = BossEnv { t: &f.t, data: &f.data, clips: &clip, env: &env, game_over: false };
    let actx = AffectCtx { party: &f.party, menu: true, skill_check: &check, boss: Some(&benv), volume: Volume::Mut };
    let mut ev = Events::new();
    // A normal attack (skill 1): physical, which attribute 0 resists.
    affect::entry_affect(&f.t, &mut f.scene, &actx, core, kite, 1, [100, 1, 0], &mut f.rand, &mut ev);
    assert_eq!(f.scene.chars[core].hp, hp - 100);
    let b = f.part(core);
    assert_eq!(b.act_num, core::act::DAMAGE);
    let shield = crate::event::Event::ResistantShield { on: crate::event::Who::Char(core), magic: 0 };
    assert!(ev.contains(&shield), "{ev:?}");
}

#[test]
fn the_meteors_fall_on_the_disc() {
    let Some(mut f) = fight() else { return };
    let (pos, t) = f.with_cx(|cx| {
        let m = Meteorite::new(cx, VF0, 0x43fa_0000, 7);
        (m.end.clone(), m.step.clone())
    });
    assert_eq!(pos.len(), 7);
    for (p, s) in pos.iter().zip(&t) {
        let r = f32::from_bits(p[0]).hypot(f32::from_bits(p[1]));
        assert!((0.0..=500.0).contains(&r), "within the range: {r}");
        assert!(dp(*s) >= 0.03, "a step of 0.03 or more");
    }
}

#[test]
fn the_second_fight_has_two_stages_and_a_level_2_core() {
    let Some(f) = bout(Volume::Out, Fight::Second) else { return };
    let x = f.kyvia();
    assert_eq!((x.fight, x.disc_lv, x.disc_max_lv, x.ex_argb), (Fight::Second, 1, 2, 0x7080_8080));
    assert_eq!(f.core().kyvia_lv, 2);
    assert_eq!(x.switch_hp, x.max_hp / 5, "a fifth of the core's HP");
    assert!(x.thunder.is_some());
    assert_eq!(f.part(f.me).anm.clip.as_deref(), Some("ANM_ex02nut0"));
    // AllGomoraList_2: two steps of three lists, none after.
    let d = &f.data.kyvia;
    assert_eq!(d.gomora_lists_of(2, 0).map(<[_]>::len), Some(3));
    assert_eq!(d.gomora_lists_of(2, 1).unwrap()[2], [0, 0, 3, 3, 1]);
    assert!(d.gomora_lists_of(2, 2).is_none());
}

#[test]
fn the_first_stage_calls_thunder_for_the_beams() {
    let Some(mut f) = bout(Volume::Out, Fight::Second) else { return };
    let me = f.me;
    let core = f.kyvia().core;
    let mut b = f.scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
    let (act, kind) = f.with_cx(|cx| {
        let mut x = kyvia_of(&mut b);
        x.atk_pat_mode = 2;
        let mut t = Tree::take(cx, core);
        if let Some(c) = t.core.as_mut() {
            c.x.k_atk_flg = 2;
            c.x.core_state = 0;
        }
        switch_action_pattern(&mut b, &mut x, &mut t, cx);
        t.put(cx);
        let kind = x.thunder.map(|t| t.kind);
        (b.act_num, kind)
    });
    assert_eq!(act, act::THUNDER);
    assert_eq!(kind, Some(thunder::Kind::Strike));
}

#[test]
fn a_thunderbolt_lives_its_time_and_one_more_draw() {
    let Some(mut f) = bout(Volume::Out, Fight::Second) else { return };
    let dat = thunder::BoltData { rand_scale: 4, default_scale: 1, rand_angle: ONE, eff_sw: 1, ..Default::default() };
    let draws = f.with_cx(|cx| {
        let mut bolt = thunder::Bolt::new(cx, VF0, 90, 100, 3, &dat);
        assert!(bolt.break_point.iter().all(|&n| (10..=17).contains(&n)), "{:?}", bolt.break_point);
        let mut n = 1;
        while bolt.draw(cx) {
            n += 1;
        }
        let segments: i16 = bolt.break_point.iter().sum();
        assert_eq!(bolt.segments.len(), 3 + segments as usize);
        n
    });
    assert_eq!(draws, 91);
}
