//! Data Drain: the rules of `ccMenuCtrl::DataDrainMenu` (gcmn 0x00532ae0), the
//! menu state machine that plays a drain once Kite picks it (step `ccMenuCtrl`
//! +0x1a): 0 the movie, the transformation and [`drain`]; 1
//! [`side_effect_happens`]; 2, 3 the warning; 10-12 [`side_effect`] and its
//! message (a level lost, [`SideEffect::level_down`]); 20 [`evolution`]. The
//! drops go out one at a time through `DataDrainSubMenu` (0x00535210, menu
//! 29). The steps are in docs/engine/battle.md ("Data Drain").

use piney_data::field::ee;
use piney_data::save::SaveData;

use crate::event::{Event, Events, Who};
use crate::exp::{self, Party};
use crate::param::{cond, elm, ty};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::tables::Tables;

/// `saveData.drainCount` (+0x685e) and the bracelet's evolution
/// (+0x6860).
pub const SAVE_DRAIN_COUNT: usize = 0x685e;
pub const SAVE_DRAIN_EVOLUTION: usize = 0x6860;
/// `saveData.parodyFlag` (+0x842b): no evolution in a Parody game.
pub const SAVE_PARODY: usize = 0x842b;
/// Kite's item list, `saveData.itemList[0]` (+0x30): 40 `ccItemList`.
pub const SAVE_ITEM_LIST: usize = 0x30;

/// What step 0 decided.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drain {
    /// The infection after the drain.
    pub erosion: i16,
    /// The list of item codes won (`ccMenuCtrl` +0x1ac, 17 entries, -1
    /// empty): the aimed target's first, then the area's; a character of
    /// the area that is no foe leaves its entry empty. The item menu hands
    /// out the first `count` from the end.
    pub drops: [i32; 17],
    /// How many of `drops` the game counts (`+0x1f0`).
    pub count: usize,
    /// The target leaves the command lists (`ccDeleteCmnd`): every target
    /// but a boss.
    pub remove_target: bool,
}

/// The item a drain of a protect-broken enemy gives by its roll: the aimed
/// target 96 up `item[2]`, 50 up `item[1]`, below `item[0]`; the others of
/// an area drain the other way round.
fn pick(item: &[i32; 3], roll: i32, aimed: bool) -> i32 {
    let (hi, lo) = if aimed { (item[2], item[0]) } else { (item[0], item[2]) };
    if roll >= 96 {
        hi
    } else if roll >= 50 {
        item[1]
    } else {
        lo
    }
}

/// Step 0 of `DataDrainMenu` after the movie (gcmn 0x00533010-0x00533470):
/// Kite (`kite`) drains `target` with skill `sid` (2 Data Drain, 3 Drain Arc,
/// 4 2128 Drain, 5 Drain Heart): the infection rises
/// ([`exp::add_lv_erosion`]), the target drops by `rand() % 100 + 60` (2128
/// Drain, Drain Heart) or `rand() % 100 + infection / 2` (a boss always
/// `item[0]`), and Drain Arc and Drain Heart take up to 16 more foes with a
/// broken protect in range. Before the movie a boss takes `EntryAffect(21)`
/// ([`drain_start`]).
pub fn drain(
    t: &Tables,
    scene: &Scene,
    save: &mut SaveData,
    kite: usize,
    target: usize,
    sid: i32,
    rng: &mut dyn Rng,
) -> Drain {
    let factor = exp::drain_factor(sid).unwrap_or(0);
    let tg = &scene.chars[target];
    let n = i32::from(tg.level()) - i32::from(scene.chars[kite].level());
    let erosion = exp::add_lv_erosion(t, save.i16(exp::SAVE_EROSION), n, factor);
    save.set_i16(exp::SAVE_EROSION, erosion);
    let mut out = Drain { erosion, drops: [-1; 17], count: 0, remove_target: false };
    let tty = tg.ty();
    if tty & ty::ENEMY != 0 {
        let item = tg.foe_state().map_or([0; 3], |f| f.row.item);
        let roll = if sid >= 4 { rng.rand() % 100 + 60 } else { rng.rand() % 100 + i32::from(erosion) / 2 };
        out.count += 1;
        out.drops[0] = pick(&item, roll, true);
    } else if tty & ty::BOSS != 0 {
        out.count += 1;
        out.drops[0] = tg.foe_state().map_or(0, |f| f.row.item[0]);
    }
    if sid == 3 || sid == 5 {
        let centre = tg.pos_p;
        let range = t.skill(sid).map_or(0, |s| s.target_range);
        let mut k = 0;
        for &c in &scene.ene_list {
            if c == target {
                continue;
            }
            let ch = &scene.chars[c];
            if ch.dead() {
                continue;
            }
            let cty = ch.ty();
            if cty & ty::FOE != 0 && ch.foe_state().is_none_or(|f| f.pp_count == 0) {
                continue;
            }
            let d = crate::geom::plane_dist(t.volume, ch.pos_p, centre);
            if !ee::le(d, ee::add(range, ch.base().width)) {
                continue;
            }
            if cty & ty::ENEMY != 0 {
                let item = ch.foe_state().map_or([0; 3], |f| f.row.item);
                let roll = rng.rand() % 100 + i32::from(erosion) / 2;
                out.count += 1;
                out.drops[k + 1] = pick(&item, roll, false);
            } else if cty & ty::BOSS != 0 {
                out.count += 1;
                out.drops[k + 1] = ch.foe_state().map_or(0, |f| f.row.item[0]);
            }
            k += 1;
            if k >= 16 {
                break;
            }
        }
    }
    out.remove_target = tty & ty::BOSS == 0;
    out
}

/// `ccCheckObjectSize(c)` (gcmn 0x0042e120) for an entry of type `ty`
/// (`entParam.type`, +0x120: 0 an enemy) and row `id` (+0x124): the
/// enemy row's size class (+0x70: 1 large, 3 middle, 4 small), which picks
/// Data Drain's movie. The gimmicks' and NPCs' (their tables' +0x30) are
/// not read here: 0.
pub fn check_object_size(t: &Tables, ty: i32, id: i32) -> i32 {
    if ty != 0 {
        return 0;
    }
    let Some(row) = usize::try_from(id).ok().and_then(|i| t.enemies.get(i)) else { return 0 };
    row.esize
}

/// Before the movie: a boss target takes `EntryAffect(21)` from Kite.
pub fn drain_start(scene: &Scene, kite: usize, target: usize, ev: &mut Events) {
    if scene.chars[target].ty() & ty::BOSS != 0 {
        ev.push(Event::affect(Who::Char(target), Who::Char(kite), 21, 0, 0, 0));
    }
}

/// Step 1: a side effect follows when `rand() % 100 <= infection / 2`.
pub fn side_effect_happens(save: &SaveData, rng: &mut dyn Rng) -> bool {
    let half = i32::from(save.i16(exp::SAVE_EROSION)) / 2;
    half >= rng.rand() % 100
}

/// What step 10 did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SideEffect {
    /// The effect (0-30), `dataDrainErosionTbl[infection / 25][rand() &
    /// 15]`; its message is the warning text's entry `id + 2`.
    pub id: i32,
    /// Kite loses a level at step 11's frame 30 (`ccChar::LevelDown`, with
    /// `ccEntryFlyFontNewLevelDown(23)`).
    pub level_down: bool,
    /// The item code lost (effect 29), -1 for none.
    pub lost: i32,
    /// The effects and numbers step 10 starts, in the game's order.
    pub starts: Vec<SideStart>,
    /// The exp lost (effects 23-27).
    pub exp_lost: i32,
}

/// One of the effects or numbers step 10 starts on a character (its scene
/// index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideStart {
    /// `effHeal(ch, 1)` (effect 0).
    Heal(usize),
    /// `ccEntryFlyFontNewMiss(ch's pos, ch)`: a condition resisted.
    Miss(usize),
    /// `effSkillStartEffect(ch, 0, 1)`: the effect taking hold.
    Effect(usize),
    /// `ccEntryFlyFontNewExp(23, value, pos, ch)` over Kite: the exp lost,
    /// negative (effects 23-27).
    Exp { who: usize, value: i32 },
    /// `effAfterDrain(ch, 0)` on Kite after the exp's number.
    AfterDrain(usize),
}

impl SideEffect {
    /// Who resisted (a MISS over them), in order.
    pub fn missed(&self) -> Vec<usize> {
        self.starts.iter().filter_map(|s| if let SideStart::Miss(c) = *s { Some(c) } else { None }).collect()
    }

    /// Who shows the effect (`effSkillStartEffect`), in order.
    pub fn shown(&self) -> Vec<usize> {
        self.starts.iter().filter_map(|s| if let SideStart::Effect(c) = *s { Some(c) } else { None }).collect()
    }

    /// Who is healed (`effHeal`), in order.
    pub fn healed(&self) -> Vec<usize> {
        self.starts.iter().filter_map(|s| if let SideStart::Heal(c) = *s { Some(c) } else { None }).collect()
    }
}

/// Kite's stat each of effects 1-6 lowers, by 20 for 900 frames.
const KITE_STATS: [usize; 6] = [elm::P_ATK, elm::P_DEF, elm::P_HIT, elm::M_ATK, elm::M_DEF, elm::M_HIT];

/// The conditions of effects 7-13 (Kite) and 14-20 (every member): the
/// condition, its frames, resisted by body (else spirit), `conditionNum`.
const CONDS: [(usize, i16, bool, i32); 7] = [
    (cond::POISON, 5400, true, 0),
    (cond::PARALYSIS, 450, true, 1),
    (cond::SPEED, 900, true, 2),
    (cond::CHARM, 300, false, 3),
    (cond::CONFUSION, 300, false, 4),
    (cond::SLEEP, 450, false, 5),
    (cond::CURSE, 5400, false, 6),
];

fn condition_on(scene: &mut Scene, c: usize, k: usize, roll: i32, out: &mut SideEffect) {
    let (i, frames, body, num) = CONDS[k];
    let ch = &mut scene.chars[c];
    let tol = i32::from(ch.real()[if body { elm::BODY } else { elm::SPIRIT }]);
    if roll < tol {
        out.starts.push(SideStart::Miss(c));
    } else {
        ch.cond[i] = frames;
        if i == cond::SPEED {
            ch.cond.speed_value = 0x3f00_0000;
        }
        ch.condition_num = num;
    }
    out.starts.push(SideStart::Effect(c));
}

/// Step 10 of `DataDrainMenu` (gcmn 0x005337a4): the side effect. `kite` is
/// the player's character (`plw` +0x20), `party` the party's members. Two
/// draws: the effect, `rand() & 15` into the infection's row, and one
/// resistance roll `rand() % 1001` every condition of the effect is tested
/// against (resisted below the tolerance). The 31 effects are in
/// docs/engine/battle.md ("Side effects"); nothing happens to Kite while he
/// is down.
pub fn side_effect(
    t: &Tables,
    scene: &mut Scene,
    party: &Party,
    kite: usize,
    save: &mut SaveData,
    rng: &mut dyn Rng,
) -> SideEffect {
    let erosion = i32::from(save.i16(exp::SAVE_EROSION));
    let k = (rng.rand() % 16) as usize;
    let row = usize::try_from(erosion / 25).unwrap_or(0).min(4);
    let id = t.drain_erosion[row][k] as i16 as i32;
    let roll = rng.rand() % 1001;
    let mut out = SideEffect { id, lost: -1, ..SideEffect::default() };
    let members: Vec<usize> = party.members.iter().flatten().copied().filter(|&c| !scene.chars[c].dead()).collect();
    let kite_up = !scene.chars[kite].dead();
    match id {
        0 => {
            for &c in &members {
                let ch = &mut scene.chars[c];
                out.starts.push(SideStart::Heal(c));
                ch.hp = ch.max_hp;
                ch.sp = ch.max_sp;
            }
        }
        1..=6 if kite_up => {
            let s = KITE_STATS[(id - 1) as usize];
            let ch = &mut scene.chars[kite];
            if let Some((temp, time)) = ch.temp_time_mut() {
                temp[s] = -20;
                time[s] = 900;
            }
            ch.condition_num = 6 + id;
            out.starts.push(SideStart::Effect(kite));
        }
        7..=13 if kite_up => condition_on(scene, kite, (id - 7) as usize, roll, &mut out),
        14..=20 => {
            for &c in &members {
                condition_on(scene, c, (id - 14) as usize, roll, &mut out);
            }
        }
        21 | 22 => {
            for &c in &members {
                let ch = &mut scene.chars[c];
                if id == 21 {
                    ch.hp = ((i32::from(ch.hp) + 1) / 2) as i16;
                } else {
                    ch.sp = ((i32::from(ch.sp) + 1) / 2) as i16;
                }
                out.starts.push(SideStart::Effect(c));
            }
        }
        23..=27 => {
            let loss = 200 * (id - 22);
            if let Some(p) = scene.chars[kite].spc_mut() {
                let mut v = i32::from(p.base.exp) - loss;
                if v < 0 {
                    if p.base.level >= 2 {
                        v += 1000;
                        out.level_down = true;
                    } else {
                        v = 0;
                    }
                }
                p.base.exp = v as i16;
            }
            out.exp_lost = loss;
            out.starts.push(SideStart::Exp { who: kite, value: -loss });
            out.starts.push(SideStart::AfterDrain(kite));
        }
        28 => {
            for &c in &members {
                let ch = &mut scene.chars[c];
                ch.hp = 1;
                ch.sp = 1;
                out.starts.push(SideStart::Effect(c));
            }
        }
        29 => out.lost = lose_item(save, rng),
        _ => {}
    }
    out
}

/// Effect 29's search of Kite's item list.
fn lose_item(save: &mut SaveData, rng: &mut dyn Rng) -> i32 {
    let at = |i: usize| SAVE_ITEM_LIST + 4 * i;
    let take = |save: &mut SaveData, i: usize| -> i32 {
        let id = save.i16(at(i));
        let cat = save.u8(at(i) + 2) as i8;
        let code = (i32::from(cat) << 16) | i32::from(id as u16);
        save.set_i16(at(i), -1);
        save.set_u8(at(i) + 2, 0xff);
        save.set_u8(at(i) + 3, 0);
        code
    };
    let start = (rng.rand() % 20) as usize;
    for i in start..40 {
        let cat = save.u8(at(i) + 2) as i8;
        let id = save.i16(at(i));
        if cat < 0 || id < 0 {
            continue;
        }
        if cat == 15 {
            break;
        }
        return take(save, i);
    }
    for i in 0..40 {
        let cat = save.u8(at(i) + 2) as i8;
        let id = save.i16(at(i));
        if cat < 0 || id < 0 || cat == 15 {
            continue;
        }
        return take(save, i);
    }
    -1
}

/// Step 20 of `DataDrainMenu` (gcmn 0x00534c6c): the drain count rises (at
/// most 10000); the bracelet grows at 10, 20, ... 80 drains, one stage at
/// a time (`evolution` 0-7), each adding key item 273-280 (category 15)
/// to Kite's list; stages 8-10 at 80, 160, 240 drains belong to volumes
/// 2-4 (`volume`, `volumeNum`) and add nothing. Not in a Parody game.
/// Returns the new stage (1-11) when it grew, and the key item added.
pub fn evolution(save: &mut SaveData, volume: i32) -> (Option<i32>, Option<i32>) {
    let n = save.i16(SAVE_DRAIN_COUNT).wrapping_add(1).min(10000);
    save.set_i16(SAVE_DRAIN_COUNT, n);
    let stage = save.i16(SAVE_DRAIN_EVOLUTION);
    let parody = save.u8(SAVE_PARODY) != 0;
    let n = i32::from(n);
    let stage = i32::from(stage);
    for s in 0..8 {
        if n >= 10 * (s + 1) && stage == s {
            if parody {
                return (None, None);
            }
            save.set_i16(SAVE_DRAIN_EVOLUTION, (s + 1) as i16);
            return (Some(s + 1), Some((15 << 16) | (273 + s)));
        }
    }
    for (s, (need, vol)) in [(80, 2), (160, 3), (240, 4)].into_iter().enumerate() {
        let s = s as i32 + 8;
        if n >= need && stage == s && volume >= vol {
            save.set_i16(SAVE_DRAIN_EVOLUTION, (s + 1) as i16);
            return (Some(s + 1), None);
        }
    }
    (None, None)
}

/// Kite's item list slot `i` (`ccItemList`).
pub fn item_slot(save: &SaveData, i: usize) -> (i16, i8, i8) {
    let a = SAVE_ITEM_LIST + 4 * i;
    (save.i16(a), save.u8(a + 2) as i8, save.u8(a + 3) as i8)
}
