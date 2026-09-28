//! An event's NPCs outside the towns: the town PCs (`entry 3`, `npcTbl`
//! rows 83-90: Meg and the other named players) and the Administrator
//! (`entry 4 29`) that story events stand in fields and dungeons.
//!
//! `ccEntryEventMng` makes them as it does in a town (`ccSetRtownPC`,
//! `ccSetMerchant`: a `ccRtownPC` or `ccMerchan` on the entry control's NPC
//! list) and puts them at the marker's `evPos`. There is no navigation map
//! outside the towns, and none is needed: an event's PC starts in its event
//! mode (`param[0]` -1) and only walks where the event's instructions send
//! it.
//!
//! The port keeps each as two halves. The entry control holds a stand-in
//! ([`crate::combat::EventNpc`]: the row's base, on the command lists as
//! the class leaves it) for what the battle's side reads - the command
//! target, `talked_to`, the effects that follow a character. The world
//! holds the class itself ([`RtownPc`], [`Merchant`], the towns' ports),
//! which moves, animates, collides and draws; after each frame the stand-in
//! is put where the class stands.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::volume::Volume;
use piney_desktop::layers::Layers;

use crate::body::{Body, TRALL};
use crate::combat::EventNpc;
use crate::ee::{ONE, V4};
use crate::entry::{Npc, NpcCtx};
use crate::hit::Hits;
use crate::merchant::{Merchant, SysopEvent};
use crate::mt::Mt;
use crate::npc::NpcRow;
use crate::rtownpc::{self, RtownPc, TownPcs};
use crate::town::TownLights;

/// The class an event NPC runs.
pub enum Class {
    Pc(Box<RtownPc>),
    Merchant(Box<Merchant>),
}

/// One event NPC: its stand-in's scene index and its class.
pub struct FieldNpc {
    pub who: usize,
    pub class: Class,
}

impl FieldNpc {
    pub fn npc(&self) -> &dyn Npc {
        match &self.class {
            Class::Pc(p) => p.as_ref(),
            Class::Merchant(m) => m.as_ref(),
        }
    }

    pub fn npc_mut(&mut self) -> &mut dyn Npc {
        match &mut self.class {
            Class::Pc(p) => p.as_mut(),
            Class::Merchant(m) => m.as_mut(),
        }
    }

    /// Its heading as the stand-in keeps it (`ccChar.dirc`).
    pub fn dirc(&self) -> V4 {
        self.npc().char().dirc
    }
}

/// What a frame of the NPCs left for the world.
#[derive(Default)]
pub struct Frame {
    /// What the NPCs started, by stand-in: a PC's `effTransfer`
    /// ([`rtownpc::PcEvent::Transfer`]; its chat lines are the town's) and
    /// the Administrator's `sysopeAct` starts.
    pub starts: Vec<(usize, SysopEvent)>,
    /// Stand-ins whose class is done with them (`main` returned 1: act 5's
    /// end), for the entry control to delete.
    pub gone: Vec<usize>,
}

/// The field's event NPCs.
#[derive(Default)]
pub struct FieldNpcs {
    /// The walking PCs' shared state (`ccRtownPC`'s tables), made with the
    /// first PC.
    town: Option<Rc<RefCell<TownPcs>>>,
    pub list: Vec<FieldNpc>,
}

impl FieldNpcs {
    /// The classes for the stand-ins `placed`, each at `pos(who)` facing
    /// `dirc(who)` as `ccEntryEventMng` left the stand-in: area `area` of
    /// town `town`, bodies from `archive`, the `volume`'s `npcTbl`, in the
    /// character list of `hits`, Kite at `player`. A row whose class or
    /// files are missing is left to its stand-in, and named in the result.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        &mut self,
        archive: &Arc<Archive>,
        volume: Volume,
        placed: &[EventNpc],
        area: i32,
        town: i32,
        hits: &mut Hits,
        player: V4,
        at: impl Fn(usize) -> (V4, V4),
    ) -> Vec<String> {
        let mut missing = Vec::new();
        let mut files = rtownpc::Files::new();
        for n in placed {
            let (pos, dirc) = at(n.who);
            match self.make(archive, volume, &mut files, n, area, town, hits, player) {
                Ok(mut class) => {
                    let ch = match &mut class {
                        Class::Pc(p) => &mut p.char,
                        Class::Merchant(m) => &mut m.ch,
                    };
                    // ccEntryEventMng: pos (+0x40) and dirc (+0x60) alone.
                    ch.pos = pos;
                    ch.dirc = dirc;
                    self.list.push(FieldNpc { who: n.who, class });
                }
                Err(e) => missing.push(format!("entry {} {}: {e}", n.ty, n.code)),
            }
        }
        missing
    }

    #[allow(clippy::too_many_arguments)]
    fn make(
        &mut self,
        archive: &Arc<Archive>,
        volume: Volume,
        files: &mut rtownpc::Files,
        n: &EventNpc,
        area: i32,
        town: i32,
        hits: &mut Hits,
        player: V4,
    ) -> Result<Class> {
        let row = NpcRow::of(volume, n.code.max(0) as usize)?;
        if n.ty == 3 {
            let t = match &self.town {
                Some(t) => t.clone(),
                None => {
                    let t = Rc::new(RefCell::new(TownPcs::outside(volume, area, town, Mt::default())?));
                    self.town = Some(t.clone());
                    t
                }
            };
            let model = rtownpc::load_model(archive, files, &t.borrow().tables, &row)?;
            return Ok(Class::Pc(Box::new(RtownPc::place(&t, &row, &model, -1, player, hits))));
        }
        let body = Rc::new(Body::read(archive, &row.stem(), TRALL)?);
        // setMerchant for 29 and 158: the origin, heading (0, 0, 0, 1).
        let m = Merchant::at(&row, body, [0, 0, 0, ONE], [0, 0, 0, ONE], hits, town, player)?;
        Ok(Class::Merchant(Box::new(m)))
    }

    /// One frame of the entry control's NPC list, in order.
    pub fn step(&mut self, ctx: &mut NpcCtx) -> Frame {
        let mut out = Frame::default();
        for n in &mut self.list {
            n.npc_mut().step(ctx);
            match &mut n.class {
                Class::Pc(p) => {
                    let t = p.events.iter().filter(|e| **e == rtownpc::PcEvent::Transfer).count();
                    out.starts.extend(std::iter::repeat_n((n.who, SysopEvent::Transfer), t));
                }
                Class::Merchant(m) => {
                    out.starts.extend(m.sysop_events.drain(..).map(|e| (n.who, e)));
                    if m.gone {
                        out.gone.push(n.who);
                    }
                }
            }
        }
        self.list.retain(|n| !out.gone.contains(&n.who));
        out
    }

    /// Each one's `ccChar::Draw`, lit by `lights`.
    pub fn draw(&self, layers: &mut Layers, to_screen: glam::Mat4, lights: &TownLights) {
        for n in &self.list {
            n.npc().char().draw(layers, to_screen, lights);
        }
    }

    /// The NPC with `npcTbl` row `code`.
    pub fn by_code(&self, code: i32) -> Option<&FieldNpc> {
        self.list.iter().find(|n| n.npc().code() == code)
    }

    pub fn by_code_mut(&mut self, code: i32) -> Option<&mut FieldNpc> {
        self.list.iter_mut().find(|n| n.npc().code() == code)
    }
}
