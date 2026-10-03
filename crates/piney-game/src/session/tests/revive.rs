//! Issues #14 and #15: a party character felled in a fight with Mimics,
//! under confusion or by a member's blow, its condition marker, and its
//! revival with the Resurrect.

use piney_battle::param::cond;

use super::*;

/// Mia's and Elk's `charTbl` rows.
const MIA: i32 = 1;
const ELK: i32 = 10;
/// Mimic, `enemyTbl` row 240 (its spells confuse).
const MIMIC: i32 = 240;
/// The Resurrect (item category 10, id 5), `ccUseItemRequest`'s code.
const RESURRECT: i32 = 10 << 16 | 5;
/// `ccFellow`'s acts: down (9), lying (10).
const DOWN: i16 = 9;
const LYING: i16 = 10;

/// Story 19's save (Mia's and Elk's addresses given) in story area 14's
/// field with Mia and Elk in the party, once the field plays.
pub(super) fn field_with_mia_and_elk() -> Option<Session> {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let mut s = story_session_on("infection", 19, |start| {
        let save = &mut start.state.save;
        for at in [offset::PARTY_MEMBER_FLAG, offset::PARTY_MEMBER_CALL] {
            save.set_i32(at, save.i32(at) | 1 << MIA | 1 << ELK);
        }
        let mut scene = piney_world::area::Scene::log_in(save);
        scene.change_scene(1, scene.town, 14, -1, -1, -1, save);
        let wm = crate::area::story_world_man(&mut Iso::open(&iso).unwrap(), 14, false).ok();
        let spcs = Some(crate::start::party_of(&[MIA, ELK]));
        start.at = Resume::World(Box::new(InWorld { scene, world_man: wm, spcs }));
    })?;
    for _ in 0..1200 {
        if let Stage::Area(a) = &mut s.stage
            && matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 60)
            && a.world().combat().who(MIA).is_some()
        {
            // The fights below as they go from the game's rand() at 1,
            // whatever the set-up drew before.
            a.world_mut().set_rand(1);
            return Some(s);
        }
        step(&mut s, 1);
    }
    panic!("no field with Mia: {}", Mode::title(&s));
}

pub(super) fn step(s: &mut Session, n: u32) {
    let mut pad = Pad::default();
    for _ in 0..n {
        pad.read(&Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        s.step(&pad);
        s.take_events();
    }
}

pub(super) fn area(s: &mut Session) -> &mut crate::area::AreaMode {
    match &mut s.stage {
        Stage::Area(a) => a,
        _ => panic!("left the field"),
    }
}

fn char(s: &mut Session, who: usize) -> piney_battle::Char {
    area(s).world().combat().scene.chars[who].clone()
}

/// `who` confused, at 1 HP, a Mimic put beside it, the party's others
/// kept up (`noDeathFlag`); stepped until `who` falls: who felled it. The
/// Mimic's spells confuse the party too, so the blow can be a member's.
fn felled_beside_a_mimic(s: &mut Session, who: usize) -> Option<usize> {
    let w = area(s).world_mut();
    let c = w.combat_mut();
    let party: Vec<usize> = c.members.iter().map(|m| m.1).collect();
    for &p in &party {
        c.scene.chars[p].no_death = p != who;
    }
    let ch = &mut c.scene.chars[who];
    ch.cond.v[cond::CONFUSION] = 3000;
    ch.hp = 1;
    let mut at = ch.pos;
    at[0] = (f32::from_bits(at[0]) + 150.0).to_bits();
    w.put_enemy(MIMIC, at, 0).expect("the Mimic");
    for _ in 0..3000 {
        step(s, 1);
        let ch = char(s, who);
        if ch.cond[cond::DEAD] != 0 {
            return ch.affect.person;
        }
    }
    panic!("{who} never fell: {:?}", char(s, who).cond);
}

/// Kite's Resurrect on `who` as the Items menu uses it
/// (`ccUseItemRequest`, its steps on the world).
fn resurrect(s: &mut Session, who: usize) {
    let w = area(s).world_mut();
    let kite = w.combat().kite.unwrap();
    for st in w.use_item(who, RESURRECT, 0) {
        w.item_step(&st, kite, who);
    }
}

/// Stepped until `who` lies down (`dead` 3, act 10), a few frames more.
fn lying(s: &mut Session, who: usize) {
    for _ in 0..200 {
        step(s, 1);
        if char(s, who).cond[cond::DEAD] == 3 {
            step(s, 5);
            assert_eq!(char(s, who).spc_char.act_num, LYING);
            return;
        }
    }
    panic!("{who} never lay down: {:?}", char(s, who).cond);
}

/// Issue #14: Mia, confused, felled in a fight with a Mimic (its spells
/// turn Kite's blows on her), plays the down act (9, as
/// `ccFellow::Influence` sets it, gcmn 0x0041c07c) and lies down; revived
/// by the Resurrect while she lies, affect 20 (gcmn 0x0041c318) sets act
/// 2, and after `dead` 5's 78 frames she is up. Then the same felled by
/// Elk's blow. The copy of her record the chat lines change
/// (`drain_chats`) wrote her old act back: she never fell and, revived
/// while lying, lay on with full HP.
#[test]
fn a_member_felled_and_revived_while_lying_gets_up() {
    let Some(mut s) = field_with_mia_and_elk() else { return };
    let (mia, elk) = {
        let c = area(&mut s).world().combat();
        (c.who(MIA).unwrap(), c.who(ELK).unwrap())
    };
    let by = felled_beside_a_mimic(&mut s, mia);
    eprintln!("Mia felled by {by:?}");
    step(&mut s, 10);
    let ch = char(&mut s, mia);
    assert_eq!((ch.cond[cond::DEAD], ch.spc_char.act_num), (2, DOWN), "the down act");
    lying(&mut s, mia);
    resurrect(&mut s, mia);
    step(&mut s, 120);
    let ch = char(&mut s, mia);
    assert_eq!((ch.cond[cond::DEAD], ch.hp), (0, ch.max_hp));
    assert!(!matches!(ch.spc_char.act_num, DOWN | LYING), "Mia still down: act {}", ch.spc_char.act_num);
    // Felled by a member's blow (Elk's EntryAffect(1)).
    area(&mut s).world_mut().entry_affect(mia, Some(elk), 1, [9999, 0, 0]);
    step(&mut s, 1);
    assert_eq!(char(&mut s, mia).spc_char.act_num, DOWN);
    lying(&mut s, mia);
    resurrect(&mut s, mia);
    step(&mut s, 120);
    let ch = char(&mut s, mia);
    assert_eq!(ch.cond[cond::DEAD], 0);
    assert!(!matches!(ch.spc_char.act_num, DOWN | LYING), "Mia still down: act {}", ch.spc_char.act_num);
}

/// Generators following `who` (the condition marker's among them).
pub(super) fn generators_on(s: &mut Session, who: usize) -> usize {
    area(s).world().fx().census().char_generators.iter().filter(|&&g| g as usize == who).count()
}

/// After a fall: no condition effect held and, once the blow's own sparks
/// are done (they outlast the game over's first frames), none drawn.
fn no_marker(s: &mut Session, who: usize, worn: usize, how: &str) {
    step(s, 40);
    let c = area(s).world().combat();
    let act = c.scene.chars[who].spc_char.act_num;
    assert_eq!(c.condition_effect(who), None, "{who} ({how}): its marker outlived the fall (act {act})");
    assert_eq!(generators_on(s, who), 0, "{who} ({how}): its marker still drawn ({worn} generators before)");
}

/// Issue #15: Kite and Mia, confused (the "?" marker, condition effect 4),
/// felled by poison in their own frames, Mia by a blow of Kite's (the
/// Mimic's spells confuse him), Kite by a Mimic's: `Influence` and
/// `ccFellow::Influence` end the marker at once (`ClearConditionEffect`,
/// gcmn 0x0059afb8), and one down shows none again (`CalcReal(dead)`).
/// The world dropped the call from the frames: Mia's stayed for good.
#[test]
fn the_fallen_keep_no_condition_marker() {
    for kite in [false, true] {
        let Some(mut s) = field_with_mia_and_elk() else { return };
        let who = {
            let c = area(&mut s).world().combat();
            if kite { c.kite.unwrap() } else { c.who(MIA).unwrap() }
        };
        {
            let c = area(&mut s).world_mut().combat_mut();
            let ch = &mut c.scene.chars[who];
            ch.cond.v[cond::CONFUSION] = 3000;
            ch.cond.v[cond::POISON] = 3050;
            ch.hp = 1;
        }
        step(&mut s, 30);
        assert_eq!(area(&mut s).world().combat().condition_effect(who), Some(4), "the confusion's marker");
        let worn = generators_on(&mut s, who);
        for _ in 0..200 {
            step(&mut s, 1);
            if char(&mut s, who).cond[cond::DEAD] != 0 {
                break;
            }
        }
        assert_ne!(char(&mut s, who).cond[cond::DEAD], 0, "{who} not felled by the poison");
        no_marker(&mut s, who, worn, "poison");
    }
    let Some(mut s) = field_with_mia_and_elk() else { return };
    let (mia, kite) = {
        let c = area(&mut s).world().combat();
        (c.who(MIA).unwrap(), c.kite.unwrap())
    };
    let by = felled_beside_a_mimic(&mut s, mia);
    assert_eq!(by, Some(kite), "Mia felled by Kite's blow");
    no_marker(&mut s, mia, 0, "Kite's blow");
    // The report's picture: Kite confused, felled by a Mimic's blow in
    // its frame (Mia and Elk down already).
    let Some(mut s) = field_with_mia_and_elk() else { return };
    let elk = area(&mut s).world().combat().who(ELK).unwrap();
    for m in [mia, elk] {
        area(&mut s).world_mut().entry_affect(m, None, 1, [9999, 0, 0]);
    }
    area(&mut s).world_mut().combat_mut().scene.chars[kite].cond.v[cond::CONFUSION] = 3000;
    step(&mut s, 30);
    let worn = generators_on(&mut s, kite);
    let by = felled_beside_a_mimic(&mut s, kite);
    assert!(by.is_some_and(|b| b != mia && b != elk), "Kite felled by {by:?}");
    no_marker(&mut s, kite, worn, "a Mimic's blow");
}
