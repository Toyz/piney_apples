//! The entry control's NPC list (`g_entCtrl`'s fourth list, gcmn
//! entctrl.cpp) in every area: `entryNpc` (0x004317b0) appends, and
//! `ccThEntryCtrl` (priority 64, 0x00431970) walks it last, after the
//! enemies, circles and gimmicks, for as many as it held when its turn
//! began: with `objFlag`, `ccEntryObj::routine` then the class's `main`,
//! which draws; a `main` that gives up deletes it (`deleteNpc`, 0x00431860).
//! The class is the row's `entry.func` ([`Npc`]); what each asks of the
//! game goes out in list order as [`NpcEvent`]s.

use std::rc::Rc;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::save::{SaveData, offset};
use piney_data::{Error, Result};
use piney_desktop::layers::Layers;

use crate::camera::Cam;
use crate::chara::char::View;
use crate::chara::npc::{Core, EventClass, NpcRow};
use crate::ee::{F, V4};
use crate::event::talk;
use crate::grunty::{self, Grunty, GruntyEvent, RaceLink};
use crate::hit::Hits;
use crate::town::TownLights;
use crate::town::dog::{Dog, DogEvent};
use crate::town::merchant::{self, Merchant, SysopEvent};
use crate::town::pc::{self, PcEvent, RtownPc, TownPcs};
use crate::world::CharId;
use crate::world::cast::Own;
use piney_battle::rand::Genrand;
use piney_event::host::NpcCommand;

/// The rest of the frame as an NPC's task sees it.
pub struct NpcCtx<'a> {
    /// Kite: `plw->pos` and `plw->dirc` after his task this frame.
    pub player: V4,
    pub player_dirc: V4,
    /// The camera after `CameraPosSet`: for `ccChar::Draw`'s fade and the
    /// view cone.
    pub view: View,
    pub cam: &'a Cam,
    /// The area's collision mesh and character list.
    pub hits: &'a mut Hits,
    /// newlib's `rand()`.
    pub rand: &'a mut crate::Rand,
    /// The game's one `ccRand` (`Combat::cc`).
    pub cc: &'a mut Genrand,
    /// `saveData`: the Event NPC's and a SEARCH PC's `eventStatus`, a
    /// Grunty's growth record and the items it reads.
    pub save: &'a mut SaveData,
    /// From Mutation on, while the Flag Race runs in the town (0x005ff790).
    pub race: Option<RaceLink<'a>>,
}

/// One NPC: the class `npcTbl[row].entry.func` made (`ccEntryRtownMerchant`,
/// `ccEntryRtownPC`, `ccEntryDog`, `ccEntryChibiGuso`).
#[derive(Clone)]
pub enum Npc {
    Merchant(Box<Merchant>),
    Pc(Box<RtownPc>),
    Dog(Box<Dog>),
    Grunty(Box<Grunty>),
}

/// What an NPC's frame asked of the game, by class; a grown Grunty's line
/// (`pgChatTbl`) as its text.
#[derive(Clone, Debug, PartialEq)]
pub enum NpcEvent {
    Merchant(SysopEvent),
    Pc(PcEvent),
    Dog(DogEvent),
    Grunty(GruntyEvent),
    Line(Vec<u8>),
}

impl Npc {
    pub fn core(&self) -> &Core {
        match self {
            Npc::Merchant(m) => &m.core,
            Npc::Pc(p) => &p.core,
            Npc::Dog(d) => &d.core,
            Npc::Grunty(g) => &g.core,
        }
    }

    pub fn core_mut(&mut self) -> &mut Core {
        match self {
            Npc::Merchant(m) => &mut m.core,
            Npc::Pc(p) => &mut p.core,
            Npc::Dog(d) => &mut d.core,
            Npc::Grunty(g) => &mut g.core,
        }
    }

    /// Its name in the port: the row its entry made it from
    /// ([`Core::id`]).
    pub fn id(&self) -> i32 {
        self.core().id()
    }

    /// On a command list (`ccEntryCmnd`): it can be targeted and spoken to.
    pub fn listed(&self) -> bool {
        self.core().entry.listed
    }

    /// `affectFunc` (`ccChar` +0x94) as `ccChar::EntryAffect` calls it with
    /// `cmd` and the menu's two values: the merchants' (`breederInfluence`
    /// and the rest), the walking PCs' `rTownNPCInfluence` (with their
    /// state `pcs`), the dogs' `dogAction`, a Grunty's (`dogAction`,
    /// `dogAction2`, `dogActionAdult`, which read `ccRand` and Kite's
    /// heading).
    fn affect(&mut self, cmd: i16, [a1, a2]: [i16; 2], pcs: Option<&mut TownPcs>, cc: &mut Genrand, kite_dirc: V4) {
        match self {
            Npc::Merchant(m) => m.influence(cmd),
            Npc::Pc(p) => {
                if let Some(t) = pcs {
                    p.influence(cmd, t);
                }
            }
            Npc::Dog(d) => d.action(cmd),
            Npc::Grunty(g) => g.affect(cmd, a1, a2, cc, kite_dirc),
        }
    }

    /// What the talk shows once the menu opens: the message table
    /// (`base->msg`) and the line, when the class chooses one (a shop's
    /// greeting, a walking PC's first line).
    pub fn talk_line(&self) -> Option<(u32, i32)> {
        match self {
            Npc::Merchant(m) => m.talk_line(),
            Npc::Pc(p) => p.talk_line(),
            Npc::Dog(_) | Npc::Grunty(_) => None,
        }
    }

    /// An event's NPC instruction through the class's own event mode, with
    /// the marker it names (position, heading) and the character it faces:
    /// true when handled. The world does puts and instant turns itself
    /// when this declines.
    pub fn command(&mut self, c: &NpcCommand, marker: Option<(V4, F)>, target: Option<V4>) -> bool {
        match self {
            Npc::Merchant(m) => m.command(c, marker, target),
            Npc::Pc(p) => p.command(c, marker, target),
            Npc::Dog(_) | Npc::Grunty(_) => false,
        }
    }

    /// What its `main` drew this frame.
    fn draw(&self, layers: &mut Layers, to_screen: glam::Mat4, lights: &TownLights) {
        match self {
            Npc::Merchant(m) => m.draw(layers, to_screen, lights),
            Npc::Grunty(g) => g.draw(layers, to_screen, lights),
            Npc::Pc(_) | Npc::Dog(_) => self.core().ch.draw(layers, to_screen, lights),
        }
    }
}

/// The list, with what its classes share: the walking PCs' state
/// (`rtownnpc.cpp`'s globals) and the Grunties' tables.
#[derive(Default)]
pub struct Npcs {
    list: Vec<Npc>,
    pcs: Option<TownPcs>,
    grunty_tables: Option<Rc<grunty::Tables>>,
    /// What the list's tasks asked since the last take, by NPC, in order.
    events: Vec<(i32, NpcEvent)>,
    /// The Grunties' camera writes and Kite's place, for the world after
    /// the list (each one's `main` writes them as it runs; the others'
    /// fades this frame see the camera before them).
    after: Vec<GruntyEvent>,
}

impl Npcs {
    /// `entryNpc`: onto the list's end.
    pub fn push(&mut self, npc: Npc) {
        self.list.push(npc);
    }

    /// The list in its order.
    pub fn iter(&self) -> impl Iterator<Item = &Npc> {
        self.list.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The NPC named `id` ([`Npc::id`]).
    pub fn get(&self, id: i32) -> Option<&Npc> {
        self.list.iter().find(|n| n.id() == id)
    }

    pub fn get_mut(&mut self, id: i32) -> Option<&mut Npc> {
        self.list.iter_mut().find(|n| n.id() == id)
    }

    /// `ccEvent::GetNpc(code)` (INF main 0x001b2cc0, MUT 0x001c8150, OUT
    /// 0x001be6e0, QUA 0x001c5d60): the first on the list whose base type
    /// has bit 4 (the Administrator, for codes 29 and 158) or bit 3 (a
    /// PC), and whose base id (+0x0c) is `code`. No merchant, dog or Grunty
    /// is one.
    pub fn get_npc(&self, code: i32) -> Option<&Npc> {
        let i = self.find_npc(code)?;
        self.list.get(i)
    }

    pub fn get_npc_mut(&mut self, code: i32) -> Option<&mut Npc> {
        let i = self.find_npc(code)?;
        self.list.get_mut(i)
    }

    fn find_npc(&self, code: i32) -> Option<usize> {
        piney_battle::evparty::get_npc(self.list.iter().map(|n| (n.core().row.flags, n.core().code())), code)
    }

    /// The walking PCs' state, once made.
    pub fn pcs(&self) -> Option<&TownPcs> {
        self.pcs.as_ref()
    }

    pub fn pcs_mut(&mut self) -> Option<&mut TownPcs> {
        self.pcs.as_mut()
    }

    /// The walking PCs' state, made by `make` with the first PC.
    pub fn pcs_or(&mut self, make: impl FnOnce() -> Result<TownPcs>) -> Result<&mut TownPcs> {
        match self.pcs {
            Some(ref mut t) => Ok(t),
            None => Ok(self.pcs.insert(make()?)),
        }
    }

    pub fn set_grunty_tables(&mut self, t: Rc<grunty::Tables>) {
        self.grunty_tables = Some(t);
    }

    /// The merchants, the walking PCs, the dogs and the Grunties on the
    /// list, each in its order.
    pub fn merchants(&self) -> impl Iterator<Item = &Merchant> {
        self.list.iter().filter_map(|n| if let Npc::Merchant(m) = n { Some(m.as_ref()) } else { None })
    }

    pub fn walkers(&self) -> impl Iterator<Item = &RtownPc> {
        self.list.iter().filter_map(|n| if let Npc::Pc(p) = n { Some(p.as_ref()) } else { None })
    }

    pub fn dogs(&self) -> impl Iterator<Item = &Dog> {
        self.list.iter().filter_map(|n| if let Npc::Dog(d) = n { Some(d.as_ref()) } else { None })
    }

    pub fn grunties(&self) -> impl Iterator<Item = &Grunty> {
        self.list.iter().filter_map(|n| if let Npc::Grunty(g) = n { Some(g.as_ref()) } else { None })
    }

    /// The Grunty named `id`.
    pub fn grunty(&self, id: i32) -> Option<&Grunty> {
        self.grunties().find(|g| g.core.id() == id)
    }

    pub fn grunty_mut(&mut self, id: i32) -> Option<&mut Grunty> {
        self.list.iter_mut().find_map(|n| match n {
            Npc::Grunty(g) if g.core.id() == id => Some(g.as_mut()),
            _ => None,
        })
    }

    /// An event's NPC (`ccEntryEventMng`'s types 3 and 4): row `row` made
    /// as its class (`ccSetRtownPC(code, -1)` or `ccSetMerchant(code)`) onto
    /// the list's end, then put at `at` (the marker's `pos` and `dirc`
    /// alone, +0x40 and +0x60, INF main 0x001b6960). A PC needs the walking
    /// PCs' state made.
    #[allow(clippy::too_many_arguments)]
    pub fn enter_event(
        &mut self,
        archive: &Arc<Archive>,
        files: &mut pc::Files,
        row: &NpcRow,
        class: EventClass,
        hits: &mut Hits,
        cc: &mut Genrand,
        town: i32,
        player: V4,
        at: Option<(V4, V4)>,
    ) -> Result<()> {
        let mut npc = match class {
            EventClass::Pc => {
                let t = self.pcs.as_mut().ok_or_else(|| Error::NotFound("the walking PCs' state".into()))?;
                let model = pc::load_model(archive, files, &t.tables, row)?;
                Npc::Pc(Box::new(RtownPc::place(t, row, &model, -1, player, hits, cc)))
            }
            EventClass::Merchant => {
                Npc::Merchant(Box::new(merchant::event_merchant(archive, row, hits, town, player, None)?))
            }
        };
        if let Some((pos, dirc)) = at {
            let ch = &mut npc.core_mut().ch;
            (ch.pos, ch.dirc) = (pos, dirc);
        }
        self.push(npc);
        Ok(())
    }

    /// `ccThEntryCtrl`'s turn at the list: each NPC with `objFlag`, for as
    /// many as the list held, its class's step (`routine`, then the
    /// vtable's `main`); a merchant whose `main` gives up is deleted there
    /// and then, its body off the character list. After each Grunty,
    /// `inuCheckNote` on the notes its anms passed, with `pgPtr` the last
    /// Grunty made.
    pub fn step(&mut self, ctx: &mut NpcCtx) {
        let status_on = ctx.save.u8(offset::EVENT_STATUS + merchant::EVENT_NPC_STATUS) != 0;
        let mut k = 0;
        for _ in 0..self.list.len() {
            // A deletion leaves the walk's next link null short of the count.
            let Some(npc) = self.list.get_mut(k) else { break };
            if !npc.core().entry.obj_flag {
                k += 1;
                continue;
            }
            let id = npc.id();
            let mut notes = Vec::new();
            let gone = match npc {
                Npc::Merchant(m) => {
                    m.status_on = status_on;
                    m.step(ctx);
                    self.events.extend(m.sysop_events.drain(..).map(|e| (id, NpcEvent::Merchant(e))));
                    m.gone
                }
                Npc::Pc(p) => {
                    if let Some(t) = self.pcs.as_mut() {
                        p.step(ctx, t);
                        self.events.extend(p.events.iter().map(|&e| (id, NpcEvent::Pc(e))));
                    }
                    false
                }
                Npc::Dog(d) => {
                    d.step(ctx);
                    self.events.extend(d.events.iter().map(|&e| (id, NpcEvent::Dog(e))));
                    false
                }
                Npc::Grunty(g) => {
                    g.step(ctx);
                    notes = std::mem::take(&mut g.notes);
                    let events = std::mem::take(&mut g.events);
                    self.grunty_events(id, events);
                    false
                }
            };
            let pg = self.list.iter().rposition(|n| matches!(n, Npc::Grunty(_)));
            if let Some(p) = pg
                && matches!(self.list[k], Npc::Grunty(_))
                && let Npc::Grunty(g) = &mut self.list[p]
            {
                grunty::inu_check_note(&notes, g, p == k, ctx.rand);
                let (pid, events) = (g.core.id(), std::mem::take(&mut g.events));
                self.grunty_events(pid, events);
            }
            if gone {
                self.list.remove(k);
            } else {
                k += 1;
            }
        }
    }

    /// A Grunty's asks: its line as text, its camera writes and Kite's
    /// place kept for after the list, the debug line dropped, the rest out
    /// in order.
    fn grunty_events(&mut self, id: i32, events: Vec<GruntyEvent>) {
        for e in events {
            match e {
                GruntyEvent::Camera(_)
                | GruntyEvent::CamPos(_)
                | GruntyEvent::CamView(_)
                | GruntyEvent::Player { .. } => self.after.push(e),
                GruntyEvent::Debug(_) => {}
                GruntyEvent::Chat(k) => {
                    let text = self.grunty_tables.as_ref().and_then(|t| t.chat.get(k).cloned()).unwrap_or_default();
                    self.events.push((id, NpcEvent::Line(text)));
                }
                e => self.events.push((id, NpcEvent::Grunty(e))),
            }
        }
    }

    /// `ccChar::EntryAffect(npc, plw, cmd, a1, a2, 0)` on the NPC named `id`
    /// ([`Npc::affect`]); what a Grunty asks is taken as from its frame.
    pub fn affect(&mut self, id: i32, cmd: i16, args: [i16; 2], cc: &mut Genrand, kite_dirc: V4) {
        let Some(i) = self.list.iter().position(|n| n.id() == id) else { return };
        let npc = &mut self.list[i];
        npc.affect(cmd, args, self.pcs.as_mut(), cc, kite_dirc);
        if let Npc::Grunty(g) = npc {
            let events = std::mem::take(&mut g.events);
            self.grunty_events(id, events);
        }
    }

    /// What the list asked since the last take, by NPC, in order.
    pub fn take_events(&mut self) -> Vec<(i32, NpcEvent)> {
        std::mem::take(&mut self.events)
    }

    /// The Grunties' camera writes and Kite's place since the last take.
    pub fn take_after(&mut self) -> Vec<GruntyEvent> {
        std::mem::take(&mut self.after)
    }

    /// Each one's draw in list order, as its `main` left it.
    pub fn draw(&self, layers: &mut Layers, to_screen: glam::Mat4, lights: &TownLights) {
        for n in &self.list {
            n.draw(layers, to_screen, lights);
        }
    }

    /// `ccSortCmnd`'s candidates of the list: those on the command list, at
    /// their posP as their last frame left it. They come after the battle's
    /// lists, where the game has them on `objList` in the order each was
    /// listed; `ccSortCmnd` places each before the first farther one, so
    /// only an exact tie of distance could tell.
    pub fn cmnds(&self, out: &mut Vec<talk::Cmnd>) {
        out.extend(self.list.iter().filter(|n| n.listed()).map(|n| {
            let c = n.core();
            talk::Cmnd::new(CharId::Npc(n.id()), c.row.flags, c.ch.width, c.ch.pos_p)
        }));
    }

    /// The NPC named `id` as the world reads its `ccChar`: its place, its
    /// base's height, type flags and name (a Grunty's as it has grown).
    pub fn own(&self, id: i32) -> Option<Own> {
        let n = self.get(id)?;
        let c = n.core();
        Some(Own {
            pos: c.ch.pos,
            height: c.ch.height,
            flags: c.row.flags,
            name: Some(c.row.name.clone().into_bytes()),
            listed: n.listed(),
        })
    }

    /// Each one's name in list order (the menus' reads of each
    /// `entParam.id`).
    pub fn ids(&self) -> Vec<i32> {
        self.list.iter().map(Npc::id).collect()
    }
}
