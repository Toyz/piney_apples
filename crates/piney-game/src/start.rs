//! The story's start points (`--mode story:N`): the game where event N of
//! the opening arc opens, with the save and the event task as the game has
//! them there, so that work on a later event need not play the ones before.
//!
//! The save is a new game's (the title's New Game over the boot's
//! `ccSaveData::Init`, the player named Kite) and the event task the boot's
//! (`ccEvent::Init`, `ccStartEvent(1, 0)`). Each event of the story before N
//! is then brought forward by the task's own `ccEventFlagSet(n)` (INF
//! 0x001b6160), as `ccStartEvent` brings an earlier volume's events forward:
//! every block run at level 1, which applies its bookkeeping - block bits,
//! mails, board posts, news, items, the gate and word lists, the call and
//! member bits, `eventStatus`, `end_event` - and nothing the player sees.
//! The first Log in's `ccSetupNewGame`, between events 1 and 2, is made where
//! the story makes it. The mode's set-up then makes its own
//! `ccStartThEvent`, which turns the closed events done, and its passes, in
//! which event N opens.
//!
//! ```text
//! N   brought forward   where it starts
//! 3   1 2               story area 14's field, as event 2's `area 14` and `scene` leave it
//! 4   1 2 3             its dungeon's first room (the entrance's ChangeArea(2, 0))
//! 10  1 2 3 4           the desktop (event 4 closes itself and event 3 there)
//! 11  ... 10            Mac Anu, from Log in
//! 12  ... 11            the desktop
//! 13  ... 12            Mac Anu, from Log in
//! 14  ... 13            the top page (the board), where Log out leaves Mac Anu
//! 15-31 ... N-1         the desktop (16 17 24 25 29 31), Mac Anu from Log in
//!                       (15 18 19 21 27 28) or the board (20 22 23 26 30), as
//!                       each event's first located block has it
//! ```
//!
//! Events 5-9 do not exist; 3 and 4 both open on event 2's end, and event 4
//! closes event 3. The side events are not brought forward: those that open
//! on the story (50 on event 11's end, 55 on event 12's) open in the start's
//! own passes.
//!
//! What the replay leaves as a player would not: `ccEventFlagSet` delivers
//! mails already read (`ReadNewMail`) and board posts at state 3; the fights'
//! experience and drops are not there, but for the story's Data Bugs'
//! virus cores and the story's gate hacks, which [`fights`] adds (event
//! 18's gate needs event 17's drained core). The party of a field or dungeon
//! start, which is `ccPartyManager`'s and not the save's, is made by the
//! party instructions event 2 (and 3) ran: [`party`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::SaveData;
use piney_data::volume::Volume;
use piney_desktop::SaveState;
use piney_event::host::{Host, StoryArea};
use piney_event::vm::Vm;
use piney_world::area::Scene;
use piney_world::party::Spcs;

use crate::session::{InWorld, Resume, Session};

/// Infection's story in its order, each event opening on the one before
/// (`event_done` of it; 3 and 4 both on 2), to the ending (31).
pub const STORY: [i32; 26] =
    [1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31];

/// Mutation's: M201 (the new game's, which the boot opens) to its ending
/// (116), each opening on the one before.
pub const MUT_STORY: [i32; 16] = [101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116];

/// The volume's story in its order (none yet past Mutation).
pub fn story(volume: Volume) -> &'static [i32] {
    match volume {
        Volume::Inf => &STORY,
        Volume::Mut => &MUT_STORY,
        Volume::Out | Volume::Qua => &[],
    }
}

/// The events a start can be made at: Infection's, then Mutation's.
pub const POINTS: [i32; 40] = [
    3, 4, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 101, 102, 103, 104,
    105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
];

/// `charTbl` row 2, Orca.
const ORCA: i32 = 2;

/// Where event `n` opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Story area 14's field.
    Field,
    /// Its dungeon's first room.
    Dungeon,
    /// The desktop's setup.
    Desktop,
    /// A town from Log in: Mac Anu, or [`log_in_town`]'s.
    Town,
    /// The top page and the board.
    Board,
}

/// Where the start of event `n` is, for the start points.
///
/// Past the opening arc, where each event's first located block is: its
/// `game_status` 2 (the desktop) or 3 (the board), or `in_town` (a town,
/// entered from Log in, where logging out leaves the party empty).
/// Mutation's: the desktop but for M204 and M205 (the board) and M215
/// (`game_status 5`, `in_town 2`: Carmina Gade). M201, the new game's, is
/// the desktop as the boot leaves it, nothing brought forward.
pub fn place(n: i32) -> Option<Place> {
    Some(match n {
        3 => Place::Field,
        4 => Place::Dungeon,
        10 | 12 | 16 | 17 | 24 | 25 | 29 | 31 => Place::Desktop,
        11 | 13 | 15 | 18 | 19 | 21 | 27 | 28 => Place::Town,
        14 | 20 | 22 | 23 | 26 | 30 => Place::Board,
        101..=103 | 106..=114 | 116 => Place::Desktop,
        104 | 105 => Place::Board,
        115 => Place::Town,
        _ => return None,
    })
}

/// The town a [`Place::Town`] start logs in to, when not the save's own
/// (`lastTown`): the town event `n`'s first block is in.
pub fn log_in_town(n: i32) -> Option<u8> {
    match n {
        115 => Some(2),
        _ => None,
    }
}

/// A start: the save and the event task as the story leaves them, and where
/// the session starts.
pub struct Start {
    pub state: SaveState,
    pub vm: Vm,
    pub at: Resume,
}

/// `ccEventFlagSet`'s host: the save, and the story areas `gate_add`,
/// `gate_mark` and `gate_unmark` read (server and words).
struct Replay<'a> {
    save: &'a mut SaveData,
    areas: &'a HashMap<i16, StoryArea>,
}

impl Host for Replay<'_> {
    fn save(&mut self) -> &mut SaveData {
        self.save
    }

    fn story_area(&self, area: i16) -> Option<StoryArea> {
        self.areas.get(&area).copied()
    }

    /// `virus_core` at level 1: `WORLD_MAN::SimGenerateCode` (main
    /// 0x0019e5c0) from the area's words. It sets the field generator's
    /// `seed` and `randcnt` and `WORLD_MAN`'s generated area, not the save;
    /// a start's own scene set-up makes those again, so nothing is kept.
    fn generate_area(&mut self, _words: [Option<i32>; 3]) {}
}

/// The story's Data Bugs, which Kite drains in their events' fights:
/// (event, virus core). Each is the only foe of an `entry_mc 0 CODE`
/// portal, and enemy row CODE drops the same core from all three slots
/// (events 17, 18, 21, 24: rows 201, 115, 235, 224).
const DATA_BUGS: [(i32, i16); 4] = [(17, 12), (18, 13), (21, 15), (24, 16)];

/// The story's gate hacks: (event, story area), the event arriving in the
/// area hacking in (its `gate_hack_anim`) after `GtHackMenu` has taken the
/// area's cores.
const HACKS: [(i32, i16); 3] = [(18, 19), (25, 23), (30, 27)];

/// What the story's fights and gate hacks before event `n` leave in the
/// save, which the replay (blocks, not fights or the gate's menu) does not:
/// each Data Bug drained gives its core; each hack sets its area's
/// `protectArea` bit and takes the cores its `protect` row asks for (as
/// `virus_core` does at level 1). A start at a hack's event is given the
/// cores that gate asks for beyond the drained ones (the letters a player
/// gathers from ordinary foes), so that the hack can be made.
///
/// Past Infection's listed hacks the same holds for any start: each story
/// area event `n`'s own blocks go to (`goes`) that is still protected gets
/// the cores its gate asks for beyond those held (Mutation's area 45 at
/// event 105, two of core 1).
fn fights(save: &mut SaveData, areas: &HashMap<i16, StoryArea>, story: &[i32], n: i32, goes: &[i16]) {
    use piney_data::save::offset;
    use piney_event::ScriptSave;
    let held = |save: &SaveData, id: i32| i32::from(save.u8(offset::IMP_ITEM_LIST + id as usize) as i8);
    for &e in story.iter().take_while(|&&e| e < n) {
        for &(_, area) in HACKS.iter().filter(|h| h.0 == e) {
            save.set_list_bit(offset::PROTECT_AREA, 5, i32::from(area));
            for (id, count) in areas.get(&area).map(|a| a.protect_items).unwrap_or_default() {
                if count != 0 {
                    save.del_item(0, 15, id as i16, count as i16);
                }
            }
        }
        for &(_, core) in DATA_BUGS.iter().filter(|d| d.0 == e) {
            save.add_item(0, 15, core, 1);
        }
    }
    let hacked = |save: &SaveData, area: i16| {
        save.word_at(offset::PROTECT_AREA, 5, i32::from(area) / 32).is_some_and(|w| w & (1 << (area % 32)) != 0)
    };
    let listed = HACKS.iter().filter(|h| h.0 == n).map(|h| h.1);
    let own: Vec<i16> = goes.iter().copied().filter(|&a| !hacked(save, a)).collect();
    let mut topped: Vec<i16> = Vec::new();
    for area in listed.chain(own) {
        if topped.contains(&area) {
            continue;
        }
        topped.push(area);
        for (id, count) in areas.get(&area).map(|a| a.protect_items).unwrap_or_default() {
            let short = count - held(save, id);
            if count != 0 && short > 0 {
                save.add_item(0, 15, id as i16, short as i16);
            }
        }
    }
}

/// The story areas event `n`'s blocks go to: the field of an `in_field`,
/// `in_dungeon` or field `scene` setting, and a `gate_words` area.
fn goes(vm: &Vm, n: i32) -> Vec<i16> {
    use piney_event::ir::{Cond, Tag};
    let Some(script) = vm.library().script(n) else { return Vec::new() };
    let mut out = Vec::new();
    for b in &script.blocks {
        for t in &b.tags {
            match *t {
                Tag::InField { field, .. } | Tag::InDungeon { field, .. } => out.push(field),
                Tag::Scene { area: 1 | 2, field, .. } => out.push(field),
                _ => {}
            }
        }
        for c in &b.conds {
            if let Cond::GateWords { area, .. } = *c {
                out.push(area);
            }
        }
    }
    out.retain(|&a| a >= 0);
    out.dedup();
    out
}

/// The party a field or dungeon start carries: `ccSpcManager` and
/// `ccPartyManager` as the party instructions before event `n` leave them.
///
/// ```text
/// Log in           ccSPC::Initialise, ccParty::InitParty: Kite, bootParam 0
/// event 2 block 0  entry 2 2 3 5: ccRegisterEventMng, EntrySpc(2), bootParam 5
///         block 1  pc_mode -3 6 at phase 0: the party is Kite alone, his bootParam 6
/// Mac Anu          ccSPC::Reboot's SetParty: memberChar[0] Kite
/// event 2 block 2  menu 75: Orca invited, ccParty::AddMember(2)
/// event 3 block 2  pc_mode -3 4 (story:4): Kite's and Orca's bootParam 4
/// ```
///
/// `AddMember`'s `inviteSpc` also sets the character's own `partyFlag`; the
/// next area's `ccSPC::Reboot` builds it from the registry's, so only the
/// managers' part is kept: the registry's `partyFlag`, the slot's
/// `memberID` and `memberChar`, and `num`.
pub fn party(n: i32) -> Spcs {
    let mut spcs = Spcs::new_game();
    let i = spcs.entry_spc(ORCA);
    if let Ok(i) = usize::try_from(i) {
        spcs.registry[i].boot_param = 5;
        spcs.pc_mode(-3, 6);
        spcs.set_party();
        if let Some(slot) = spcs.member_id.iter().position(|&m| m == -1) {
            spcs.registry[i].party_flag = 1;
            spcs.member_id[slot] = ORCA;
            spcs.member_char[slot] = Some(ORCA);
            spcs.num += 1;
        }
    }
    if n >= 4 {
        spcs.pc_mode(-3, 4);
    }
    spcs
}

/// The save and the event task at event `n`'s start, and where it is.
pub fn build(iso: &Path, n: i32) -> Result<Start, String> {
    let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
    let volume = disc.volume().map_err(|e| e.to_string())?;
    let story = story(volume);
    let place = place(n).filter(|_| story.contains(&n)).ok_or(format!(
        "story:{n}: {volume:?}'s start points are {:?}",
        story.iter().filter(|&&e| place(e).is_some()).collect::<Vec<_>>()
    ))?;
    let areas = crate::story::from_tables(piney_data::area::AreaTables::of(volume));
    let mut state = crate::world::new_game_state(&mut disc)?;
    let mut vm = crate::desktop::boot(&mut disc, &mut state)?;
    for &e in story.iter().take_while(|&&e| e < n) {
        vm.flag_set(e, &mut Replay { save: &mut state.save, areas: &areas });
        // The first Log in, which the story's second event opens after.
        if e == story[0] {
            crate::session::log_in(iso, &mut state);
        }
    }
    let goes = goes(&vm, n);
    fights(&mut state.save, &areas, story, n, &goes);
    let at = match place {
        Place::Desktop => Resume::Desktop,
        Place::Board => Resume::Board,
        Place::Town => {
            if let Some(town) = log_in_town(n) {
                state.save.set_u8(piney_data::save::offset::LAST_TOWN, town);
            }
            Resume::World(Box::new(InWorld { scene: Scene::log_in(&mut state.save), world_man: None, spcs: None }))
        }
        Place::Field | Place::Dungeon => {
            // Log in, then event 2's `area 14` (ccEvAreaCodeAdd: the area
            // made from its words) and `scene 1 0 14 -1 -1 -1`.
            let mut scene = Scene::log_in(&mut state.save);
            let wm = crate::area::ev_area_world_man(iso, 14, scene.server, &state.save)?
                .ok_or("area 14: no words make it")?;
            scene.go(piney_data::area::Go::ChangeScene([1, 0, 14, -1, -1, -1]), &mut state.save);
            if place == Place::Dungeon {
                scene.change_area(piney_world::area::kind::DUNGEON, 0, &mut state.save);
            }
            Resume::World(Box::new(InWorld { scene, world_man: Some(wm), spcs: Some(party(n)) }))
        }
    };
    Ok(Start { state, vm, at })
}

/// The session for `--mode story:N`.
pub fn session(iso: PathBuf, archive: Arc<Archive>, card: Option<PathBuf>, n: i32) -> Result<Session, String> {
    let Start { state, vm, at } = build(&iso, n)?;
    Session::resume(iso, archive, card, state, vm, at)
}

#[cfg(test)]
mod tests {
    use piney_data::save::offset;

    use super::*;

    fn iso() -> Option<PathBuf> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
        p.exists().then_some(p)
    }

    fn cores(save: &SaveData, id: usize) -> i8 {
        save.u8(offset::IMP_ITEM_LIST + id) as i8
    }

    fn hacked(save: &SaveData, area: usize) -> bool {
        save.i32(offset::PROTECT_AREA + 4 * (area / 32)) as u32 & (1 << (area % 32)) != 0
    }

    /// Event 18 hacks area 19's gate with two Virus Core M: event 11's
    /// and event 17's Data Bug's. Its start holds both, area 19 still
    /// protected; event 19's has used them, area 19 open, and holds event
    /// 18's Data Bug's Virus Core N.
    #[test]
    fn the_starts_carry_the_drained_cores() {
        let Some(iso) = iso() else { return };
        let s18 = build(&iso, 18).unwrap().state.save;
        assert_eq!(cores(&s18, 12), 2, "Virus Core M at story 18");
        assert!(!hacked(&s18, 19));
        let s19 = build(&iso, 19).unwrap().state.save;
        assert_eq!(cores(&s19, 12), 0, "Virus Core M after the hack");
        assert_eq!(cores(&s19, 13), 1, "Virus Core N from event 18's Data Bug");
        assert!(hacked(&s19, 19));
    }
}
