//! Who stands in an area and how they run: the one part of the world that
//! differs between a Root Town and everywhere else. The game has one task
//! list for every area (`ccThSpc`, `ccThFellow`, `ccThEntryCtrl`, ...); what
//! its tasks hold differs: Kite (`ccPlayer`, the battle's character) and the
//! party are every area's, and so is the entry control's NPC list
//! ([`Npcs`]); a town's holds its merchants, dogs, Grunties and walking PCs
//! with the event's NPCs, a field's or a dungeon's the event's NPCs after
//! the battle's enemies, portals and gimmicks. [`Cast`] is that
//! difference; the world holds one as `Box<dyn Cast>`.

use std::any::Any;

use piney_desktop::anm::Ctx;

use super::{CharId, World};
use crate::combat::Show;
use crate::ee::{F, V4};
use crate::grunty::race::{RaceEvent, RaceHost};
use crate::systems::npcs::{NpcCtx, NpcEvent, Npcs};
use crate::town::merchant::SysopEvent;

/// What an area's own characters started at the entry control's slot - a
/// sound, an effect, a line, an ask of the game - in the frame's task
/// order: the Flag Race's (63), then the entry control's lists (64: the
/// gate, the race's flags, the NPC list). They go out as [`Show::Cast`]
/// with the battle's shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    Race(RaceEvent),
    /// The Chaos Gate's `ccSeOn3D` (71 opening, 72 closing).
    GateSound {
        n: i32,
        pos: V4,
    },
    /// An NPC's ([`NpcEvent`]), by its name (`CharId::Npc(code)`).
    Npc {
        code: i32,
        e: NpcEvent,
    },
}

/// What the menus ask of the area's own characters besides `EntryAffect`:
/// the Flag Race's task (menu 88's 0x005ff820(kind), its pause's quit,
/// its end) and the breeder's writes into the Grunty it shows
/// (`growthNum`; `foodMode` and `chatFlag`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    RaceStart(i32),
    RaceQuit,
    RaceDone,
    Growth { code: i32, growth: i8 },
    Food { code: i32, food_mode: u8, chat_flag: Option<u8> },
}

/// What the menus read of the area's own characters (`ccMenuCtrl`'s
/// reads): the entry control's NPC list (each `entParam.id`), the Grunty
/// the breeder's menus show and the Flag Race's state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Seen {
    pub npcs: Vec<i32>,
    pub grunty: Option<GruntySeen>,
    pub race: Option<RaceSeen>,
}

/// A Grunty as the breeder's menus read it (`ccPGuso`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GruntySeen {
    pub who: CharId,
    pub row: i32,
    pub growth: i8,
    pub msg: i8,
    pub exist: u8,
    pub level: i16,
    pub size: i16,
    pub food_num: i32,
    pub chat_flag: u8,
    pub food_mode: u8,
}

/// The Flag Race as the menus read it (its task, `0x0038bd44`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RaceSeen {
    pub state: i8,
    pub rank: i8,
    pub running: bool,
    pub time: i16,
}

/// One of the area's own characters ([`CharId::Npc`], [`CharId::Object`])
/// as the world reads its `ccChar`: its place (+0x40), its base's height,
/// type flags (+0x08) and name (None where the cast has none to give), and
/// whether it is on a command list (`ccCheckTarget`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Own {
    pub pos: V4,
    pub height: F,
    pub flags: u32,
    pub name: Option<Vec<u8>>,
    pub listed: bool,
}

/// What differs between the areas' characters (see the module doc). Each
/// method is one place where the world reaches its characters; a cast with
/// no such characters keeps the default.
pub trait Cast: Any {
    /// The entry control's NPC list, where the cast keeps one.
    fn npcs(&self) -> Option<&Npcs> {
        None
    }
    fn npcs_mut(&mut self) -> Option<&mut Npcs> {
        None
    }
    /// The area's own character `id` names, on the cast's lists (none
    /// where the battle's scene holds every character).
    fn own(&self, _w: &World, _id: CharId) -> Option<Own> {
        None
    }
    /// `ccSortCmnd`'s walk past the battle's lists: the area's own
    /// characters on a command list, as `ccThGameCtrl` (33) reads them
    /// from where they stood at the end of last frame (`player` Kite's
    /// place then).
    fn cmnds(&self, _player: V4, _out: &mut Vec<crate::event::talk::Cmnd>) {}
    /// After `ccThEntryCtrl`'s first slice: what the entry control made of
    /// the event's NPCs.
    fn placed(&mut self, _w: &mut World) {}
    /// `ccThEntryCtrl`'s NPC list as the area keeps it (slot 64, after the
    /// battle's walk of its lists, before `ccThEffect` at 80), and in a town
    /// the Flag Race's task (63) before it; `fades` is the menu's fader.
    fn entry_slot(&mut self, _w: &mut World, _fades: &mut dyn RaceHost) {}
    /// Its lights in the characters' light group (`ccLight`s it holds:
    /// a town's Chaos Gate's), after the map's.
    fn lights(&self, _out: &mut Vec<crate::town::Light>) {}
    /// What the ride's slot (49) reads of the cast's tasks as they last
    /// left them: a town's Flag Race's handling and whether it is over
    /// (0x005ff760).
    fn race_ride(&self) -> Option<(piney_battle::ride::RaceRide, bool)> {
        None
    }
    /// Its draws as its tasks left them (the entry control's NPC list, 64,
    /// after the party's 49 and 50), under the group as the walk left it.
    fn draw(&self, _w: &World, _ctx: &mut Ctx, _to_screen: glam::Mat4, _lights: &super::CastLights) {}
    /// `inviteSpc(id)` (gcmn 0x005a08e0) for a character the registry does
    /// not hold: registered and built where the cast builds newcomers (a
    /// town's Chaos Gate). False when it builds none (outside the towns
    /// the party is built only with the area).
    fn invite_new(&mut self, _w: &mut World, _id: i32) -> bool {
        false
    }
    /// `ccChar::EntryAffect(who, plw, cmnd, p0, p1, 0)` on one of its own
    /// characters (the battle's are the world's): an NPC's `affectFunc`.
    fn affect(&mut self, w: &mut World, who: CharId, cmnd: i16, p: [i16; 2]) {
        if let (CharId::Npc(id), Some(npcs)) = (who, self.npcs_mut()) {
            npc_affect(npcs, w, id, cmnd, p);
        }
    }
    /// One of the menus' asks ([`Ask`]); false when it holds nothing the
    /// ask names.
    fn ask(&mut self, _w: &mut World, _a: Ask) -> bool {
        false
    }
    /// `ccEntryEventMng`'s step for one of `eventMng.entry[]` the cast
    /// places itself (`entry type code marker param`); false when it
    /// makes nothing of it.
    fn event_entry(&mut self, _w: &mut World, _e: [i16; 4]) -> bool {
        false
    }
    /// Whether the event task idles this frame for the cast's tasks
    /// (from Mutation on, while the Flag Race runs: MUT main 0x001cb154).
    fn holds_events(&self) -> bool {
        false
    }
    /// What the menus read of its characters; `shown` the command
    /// target before and now (`cmndTargetPrev`, `cmndTarget`).
    fn seen(&self, _shown: [Option<CharId>; 2]) -> Seen {
        Seen::default()
    }
    /// Its marks on the area's map (`DrawMap`'s).
    fn map_entries(&self, _e: &mut crate::map::Entries) {}
}

impl dyn Cast {
    /// The cast as its own type: for tests only, as [`crate::area::AreaMap`]'s
    /// `get`. No world or game code reaches a cast's type.
    pub fn get<T: Cast>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref()
    }
}

/// Held in the world's place while a cast's method has the world.
pub(super) struct Lent;

impl Cast for Lent {}

/// [`Npcs::affect`] with the world's `ccRand`, Kite's heading and
/// camera.cpp's list (a Grunty's fixed camera written at the call), and
/// Kite put where the fixed camera puts him.
pub(crate) fn npc_affect(npcs: &mut Npcs, w: &mut World, id: i32, cmnd: i16, p: [i16; 2]) {
    let dirc = w.kite().dirc();
    if let Some((pos, dirc_z)) = npcs.affect(id, cmnd, p, &mut w.combat.cc, dirc, &mut w.camera.list) {
        let mut dirc = dirc;
        dirc[2] = dirc_z;
        w.put_kite(pos, dirc);
    }
}

/// What the NPC list asked this slot, out with the battle's shows in its
/// order (the Administrator's `noise2` the menu's; the camera after an
/// NPC's camera write a [`Show::Ear`] for the sounds after it).
pub(crate) fn npc_outs(w: &mut World, npcs: &mut Npcs) {
    for (code, e) in npcs.take_events() {
        match e {
            NpcEvent::Merchant(SysopEvent::Noise) => w.noise = true,
            NpcEvent::Ear(ear) => w.combat.shows.push(Show::Ear(ear)),
            e => w.combat.shows.push(Show::Cast(Out::Npc { code, e })),
        }
    }
}

/// Outside the towns: Kite and the party are the battle's characters, the
/// entry control's enemy, circle and gimmick lists the battle's
/// ([`crate::combat::Combat`]); its NPC list the event's NPCs, which
/// `ccEntryEventMng` makes among the event's enemies in entry order
/// ([`crate::combat::Combat::start_entries`]).
#[derive(Default)]
pub struct BattleCast {
    pub npcs: Npcs,
}

impl Cast for BattleCast {
    fn npcs(&self) -> Option<&Npcs> {
        Some(&self.npcs)
    }

    fn npcs_mut(&mut self) -> Option<&mut Npcs> {
        Some(&mut self.npcs)
    }

    fn own(&self, _w: &World, id: CharId) -> Option<Own> {
        match id {
            CharId::Npc(id) => self.npcs.own(id),
            _ => None,
        }
    }

    /// The NPC list's listed ones, after the battle's lists.
    fn cmnds(&self, _player: V4, out: &mut Vec<crate::event::talk::Cmnd>) {
        self.npcs.cmnds(out);
    }

    /// The NPC list's turn at the end of the entry control's walk (after
    /// the enemies, circles and gimmicks); what it started goes to this
    /// frame's shows after what the menus' `EntryAffect` asked since the
    /// last slot.
    fn entry_slot(&mut self, w: &mut World, _fades: &mut dyn RaceHost) {
        npc_outs(w, &mut self.npcs);
        if self.npcs.is_empty() {
            return;
        }
        let (player, player_dirc) = (w.kite().pos(), w.kite().dirc());
        let mut cx = NpcCtx {
            player,
            player_dirc,
            cams: &mut w.camera.list,
            hits: &mut w.area.core_mut().hits,
            rand: &mut w.combat.rand,
            cc: &mut w.combat.cc,
            save: &mut w.save.save,
            race: None,
        };
        self.npcs.step(&mut cx);
        npc_outs(w, &mut self.npcs);
    }

    fn draw(&self, _w: &World, ctx: &mut Ctx, to_screen: glam::Mat4, lights: &super::CastLights) {
        self.npcs.draw(&mut ctx.layers, to_screen, &lights.group());
    }

    /// The menus' reads of the NPC list.
    fn seen(&self, _shown: [Option<CharId>; 2]) -> Seen {
        Seen { npcs: self.npcs.ids(), ..Seen::default() }
    }
}
