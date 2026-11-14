//! Who stands in an area and how they run: the one part of the world that
//! differs between a Root Town and everywhere else. The game has one task
//! list for every area (`ccThSpc`, `ccThFellow`, `ccThEntryCtrl`, ...); what
//! its tasks hold differs: Kite (`ccPlayer`, the battle's character) and the
//! party are every area's; in a town the entry control's NPC list holds the
//! merchants, dogs, Grunties and walking PCs, outside the towns enemies,
//! portals and gimmicks, with the event's NPCs as its stand-ins.
//! [`Cast`] is that difference; the world holds one as `Box<dyn Cast>`.

use std::any::Any;

use piney_desktop::anm::Ctx;

use super::{CharId, World, tasks};
use crate::camera::CamPad;
use crate::combat::Show;
use crate::ee::{F, ONE, V4};
use crate::event::entry::Npc;
use crate::field::npcs::FieldNpcs;
use crate::grunty::GruntyEvent;
use crate::grunty::race::{RaceEvent, RaceHost};
use crate::town::dog::DogEvent;
use crate::town::pc::PcEvent;

/// What a town's own characters started at the entry control's slot - a
/// sound, an effect, a line, an ask of the game - in the frame's task
/// order: the Flag Race's (63), then the entry control's list (64: the
/// gate, the race's flags, the merchants, dogs, Grunties, walking PCs).
/// They go out as [`Show::Cast`] with the battle's shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    Race(RaceEvent),
    /// The Chaos Gate's `ccSeOn3D` (71 opening, 72 closing).
    GateSound {
        n: i32,
        pos: V4,
    },
    Dog {
        code: i32,
        e: DogEvent,
    },
    Grunty {
        code: i32,
        e: GruntyEvent,
    },
    Pc {
        code: i32,
        e: PcEvent,
    },
    /// A line in a balloon over one of its NPCs (a grown Grunty's
    /// `pgChatTbl` line).
    Chat {
        code: i32,
        text: Vec<u8>,
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
    /// `ccEvent::GetNpc(code)`: the event NPC (types 3 and 4) whose base id
    /// is `code`.
    fn npc(&self, code: i32) -> Option<&dyn Npc>;
    fn npc_mut(&mut self, code: i32) -> Option<&mut dyn Npc>;
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
    /// characters (the battle's are the world's): an NPC's `Influence`.
    fn affect(&mut self, _w: &mut World, who: CharId, cmnd: i16, _p: [i16; 2]) {
        if let CharId::Npc(code) = who
            && let Some(n) = self.npc_mut(code)
        {
            n.influence(cmnd);
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

impl Cast for Lent {
    fn npc(&self, _code: i32) -> Option<&dyn Npc> {
        None
    }
    fn npc_mut(&mut self, _code: i32) -> Option<&mut dyn Npc> {
        None
    }
}

/// Outside the towns: Kite and the party are the battle's characters, the
/// entry control's lists the battle's ([`crate::combat::Combat`]); the
/// event's NPCs are their classes over the entry control's stand-ins
/// ([`crate::field::npcs`]).
#[derive(Default)]
pub struct BattleCast {
    /// The event's NPCs' classes.
    pub npcs: FieldNpcs,
}

impl Cast for BattleCast {
    fn npc(&self, code: i32) -> Option<&dyn Npc> {
        self.npcs.by_code(code).map(|n| n.npc())
    }

    fn npc_mut(&mut self, code: i32) -> Option<&mut dyn Npc> {
        self.npcs.by_code_mut(code).map(|n| n.npc_mut())
    }

    /// The classes of the event's NPCs the entry control has just made,
    /// where `ccEntryEventMng` put their stand-ins.
    fn placed(&mut self, w: &mut World) {
        let placed = w.combat.npcs.clone();
        if placed.is_empty() {
            return;
        }
        let (area, town) = (w.scene.area, w.scene.town);
        let player = w.kite().pos();
        let chars = &w.combat.scene.chars;
        let objs = &w.combat.ctrl.objs;
        let at = |who: usize| {
            let dirc = match objs.get(who) {
                Some(piney_battle::entry::Obj::Npc(o)) => o.dirc,
                _ => [0, 0, 0, ONE],
            };
            (chars[who].pos, dirc)
        };
        let hits = &mut w.area.core_mut().hits;
        let (archive, volume, cc) = (&w.archive, w.volume, &mut w.combat.cc);
        let missing = self.npcs.build(archive, volume, &placed, area, town, hits, player, cc, at);
        for m in missing {
            tracing::warn!("the event's NPCs: {m}");
        }
    }

    /// One frame of the event's NPCs, at the end of the entry control's
    /// walk: each class steps, its stand-in follows (position, heading, the
    /// command lists), their starts go to this frame's effects, and a class
    /// done with its NPC has the stand-in deleted.
    fn entry_slot(&mut self, w: &mut World, _fades: &mut dyn RaceHost) {
        if self.npcs.list.is_empty() {
            return;
        }
        let a = w.camera.active();
        let view = crate::chara::char::View {
            player: w.kite().pos(),
            cam: a.pos,
            deg1: a.deg[1],
            eye: a.kind == crate::camera::kind::EYE,
        };
        let (player, player_dirc) = (w.kite().pos(), w.kite().dirc());
        let hits = &mut w.area.core_mut().hits;
        let mut cx = crate::event::entry::NpcCtx {
            player,
            player_dirc,
            view,
            cam: w.camera.active(),
            hits,
            rand: &mut w.combat.rand,
            cc: &mut w.combat.cc,
        };
        let out = self.npcs.step(&mut cx);
        for n in &self.npcs.list {
            let c = &mut w.combat;
            let Some(ch) = c.scene.chars.get_mut(n.who) else { continue };
            ch.pos = n.npc().char().pos;
            ch.pos_p = n.npc().char().pos_p;
            if let Some(piney_battle::entry::Obj::Npc(o)) = c.ctrl.objs.get_mut(n.who) {
                o.dirc = n.dirc();
            }
            let listed = c.scene.listed(n.who);
            if n.npc().listed() && !listed {
                piney_battle::fellow::entry_cmnd(&mut c.scene, n.who);
            } else if !n.npc().listed() && listed {
                piney_battle::fellow::delete_cmnd(&mut c.scene, n.who);
            }
        }
        for (who, e) in out.starts {
            match e {
                crate::town::merchant::SysopEvent::Noise => w.noise = true,
                _ => w.combat.shows.push(Show::Npc(who, e)),
            }
        }
        if !out.gone.is_empty() {
            let info = w.task_info();
            let mut x = tasks(&mut *w.area, &mut w.camera, &mut w.save.save, CamPad::default(), &info);
            w.combat.with_entry_cx(&mut x, |ctrl, cx| {
                for &who in &out.gone {
                    ctrl.delete_npc(cx, who);
                }
            });
            w.combat.npcs.retain(|n| !out.gone.contains(&n.who));
        }
    }

    fn draw(&self, _w: &World, ctx: &mut Ctx, to_screen: glam::Mat4, lights: &super::CastLights) {
        self.npcs.draw(&mut ctx.layers, to_screen, &lights.group());
    }
}
