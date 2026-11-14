//! A Root Town's characters: the cast of the one world in area 0
//! ([`crate::world::cast`]). Kite is the battle's `ccPlayer` ([`crate::combat`]),
//! the entry control's NPC list the merchants, dogs, Grunties, walking PCs
//! and the event's NPCs; the Chaos Gate and the Flag Race stand with them.
//! The town's methods take the world's core as `world`.

use std::rc::Rc;

use crate::EventEntry;
use crate::event::entry;
use crate::event::gate::Gate;
use crate::grunty::{self, race};
use crate::town::{dog, merchant};
use crate::world::cast::Out;

/// The town's characters and what they started since the last take.
pub struct TownCast {
    pub(crate) gate: Gate,
    /// `ccEntryCtrl`'s NPC list: `ccSetMerchant(0)`'s merchants first
    /// (town/merchant.rs), then everyone else (the walking PCs).
    pub(crate) merchants: Vec<merchant::Merchant>,
    /// `ccSetDog`'s dogs (Dun Loireag), after the merchants.
    pub(crate) dogs: Vec<dog::Dog>,
    /// `ccSetChibiGuso`'s Grunties (Dun Loireag), after the dogs; the last
    /// one made is `pgPtr`. Their tables, and what they asked for since
    /// the last take with the Grunty's code.
    pub(crate) grunties: Vec<grunty::Grunty>,
    pub(crate) grunty_tables: Option<Rc<grunty::Tables>>,
    pub(crate) npcs: Vec<Box<dyn entry::Npc>>,
    /// The walking PCs `ccEntryRandomNpc` placed (with the state they
    /// share: `ccRand`, the navigation map, the chat groups).
    pub(crate) town_pcs: Option<Rc<std::cell::RefCell<crate::town::pc::TownPcs>>>,
    /// Town NPC entries (`entry 3|4 code marker`) waiting for the entry
    /// control's set-up, and whether it has run.
    pub(crate) event_npcs: Vec<EventEntry>,
    pub(crate) placed: bool,
    pub(crate) pcs: Vec<crate::town::pc::RtownPc>,
    /// What the characters' tasks started in this slot, in their order
    /// ([`Out`]), until it ends ([`Show::Cast`]).
    pub(crate) outs: Vec<Out>,
    // --- the Flag Race (grunty/race.rs, town/ride.rs), from Mutation on ---
    /// Its three flags (`0x00774a90`), its task (`0x0038bd44`) and the
    /// riding Grunty's (`pcgs`); what they asked of the game.
    pub(crate) flags: Vec<race::Flag>,
    pub(crate) race: Option<Box<race::Race>>,
}

impl crate::world::cast::Cast for TownCast {
    fn npc(&self, code: i32) -> Option<&dyn entry::Npc> {
        TownCast::npc(self, code)
    }

    fn npc_mut(&mut self, code: i32) -> Option<&mut dyn entry::Npc> {
        TownCast::npc_mut(self, code)
    }

    fn own(&self, w: &crate::world::World, id: CharId) -> Option<Own> {
        TownCast::own(self, w, id)
    }

    /// The listed NPCs (their posP as their last frame left it), then the
    /// Chaos Gate while Kite is near, its posP `ccTransPosW2P` of its pos
    /// (`ccChgate::main` 0x004592a0), an object of the entry control's.
    fn cmnds(&self, player: crate::ee::V4, out: &mut Vec<talk::Cmnd>) {
        out.extend(self.all_npcs().filter(|n| n.listed()).map(|n| {
            let c = n.char();
            talk::Cmnd::new(CharId::Npc(n.code()), n.flags(), c.width, c.pos_p)
        }));
        if self.gate.frame.near {
            out.push(talk::Cmnd::new(
                CharId::Object(gate::GIMMICK),
                talk::flags::CHAOS_GATE,
                gate::WIDTH,
                crate::chara::char::w2p(self.gate.pos, player),
            ));
        }
    }

    /// `ccEntryEventMng` in a town: the events' town NPCs, the merchants,
    /// the walking PCs ([`TownCast::place_entries`]).
    fn placed(&mut self, w: &mut crate::world::World) {
        if let Err(e) = self.place_entries(w) {
            tracing::warn!("the town's entries: {e}");
        }
    }

    /// The Chaos Gate's body and ring lights ([`Gate::lights`]).
    fn lights(&self, out: &mut Vec<crate::town::Light>) {
        out.extend(self.gate.lights());
    }

    fn race_ride(&self) -> Option<(piney_battle::ride::RaceRide, bool)> {
        self.race.as_ref().map(|r| (r.ride, r.end))
    }

    /// The Flag Race's HUD and clips, then the entry control's list: the
    /// gate, the race's flags, merchants, dogs, Grunties, walking PCs and
    /// event NPCs, as their tasks left them.
    fn draw(&self, w: &crate::world::World, ctx: &mut Ctx, to_screen: glam::Mat4, seen: &crate::world::CastLights) {
        if let Some(r) = &self.race {
            r.draw(ctx, &w.camera.world_screen);
        }
        let lit = seen.group();
        let lights = &*lit;
        self.gate.draw(&mut ctx.layers, to_screen, &seen.with_own(&self.gate.body_lights()), lights);
        for f in &self.flags {
            f.draw(&mut ctx.layers, to_screen, lights);
        }
        for m in &self.merchants {
            m.draw(&mut ctx.layers, to_screen, lights);
        }
        for d in &self.dogs {
            d.ch.draw(&mut ctx.layers, to_screen, lights);
        }
        for g in &self.grunties {
            g.draw(&mut ctx.layers, to_screen, lights);
        }
        for pc in &self.pcs {
            pc.char.draw(&mut ctx.layers, to_screen, lights);
        }
        for n in &self.npcs {
            n.char().draw(&mut ctx.layers, to_screen, lights);
        }
    }

    /// The Flag Race's task (63), then `ccThEntryCtrl`'s NPC list (64).
    fn entry_slot(&mut self, w: &mut crate::world::World, fades: &mut dyn race::RaceHost) {
        self.entry_list(w, fades);
    }

    /// A newcomer built at the Chaos Gate ([`TownCast::invite_new`]).
    fn invite_new(&mut self, w: &mut crate::world::World, id: i32) -> bool {
        TownCast::invite_new(self, w, id)
    }

    fn affect(&mut self, w: &mut crate::world::World, who: CharId, cmnd: i16, p: [i16; 2]) {
        self.affect_own(w, who, cmnd, p);
    }

    /// The Flag Race started (from the next frame's slot), quit, done
    /// with; the breeder's writes into its Grunty.
    fn ask(&mut self, w: &mut crate::world::World, a: Ask) -> bool {
        match a {
            Ask::RaceStart(kind) => self.race_start(w, kind),
            Ask::RaceQuit => self.race.as_deref_mut().map(race::Race::quit).is_some(),
            Ask::RaceDone => self.race.as_deref_mut().map(|r| r.done = true).is_some(),
            Ask::Growth { code, growth } => self.grunty_mut(code).map(|g| g.growth_num = growth).is_some(),
            Ask::Food { code, food_mode, chat_flag } => self
                .grunty_mut(code)
                .map(|g| {
                    g.food_mode = food_mode;
                    if let Some(c) = chat_flag {
                        g.chat_flag = c;
                    }
                })
                .is_some(),
        }
    }

    fn event_entry(&mut self, w: &mut crate::world::World, [ty, code, marker, param]: [i16; 4]) -> bool {
        self.entry(w, ty, code, marker, param)
    }

    fn holds_events(&self) -> bool {
        self.racing()
    }

    /// The entry control's NPCs (merchants, walking PCs, the rest), the
    /// Grunty of the target before or now, the race.
    fn seen(&self, shown: [Option<CharId>; 2]) -> Seen {
        Seen {
            npcs: self.all_npcs().map(|n| n.code()).collect(),
            grunty: shown.into_iter().flatten().find_map(|who| {
                let CharId::Npc(code) = who else { return None };
                let g = self.grunty(code)?;
                Some(GruntySeen {
                    who,
                    row: i32::from(g.row.id),
                    growth: g.growth_num,
                    msg: g.msg_num,
                    exist: g.exist,
                    level: g.grow.level,
                    size: g.grow.size,
                    food_num: g.grow.food_num,
                    chat_flag: g.chat_flag,
                    food_mode: g.food_mode,
                })
            }),
            race: self.race().map(|r| RaceSeen { state: r.state, rank: r.rank, running: r.running, time: r.time }),
        }
    }

    /// While the race runs (0x005ff790), each flag's place and its mark's
    /// fade (0x005fd100).
    fn map_entries(&self, e: &mut crate::map::Entries) {
        e.racers = self.map_racers();
    }
}

// The town's frame, its ccThGameCtrl, its entries and the event camera's
// view of it (moved from the façade in lib.rs).
use crate::chara::npc;
use crate::event::entry::Npc as _;
use crate::event::{gate, talk};
use crate::world::CharId;
use crate::world::cast::{Ask, GruntySeen, Own, RaceSeen, Seen};
use crate::{area, camera, combat};
use piney_data::save::offset;
use piney_desktop::anm::Ctx;

impl TownCast {
    /// `ccChar::EntryAffect(target, plw, cmnd, p0, p1, 0)` from the menus
    /// on the gate, a Grunty (its `affectFunc`) or a town NPC.
    fn affect_own(&mut self, world: &mut crate::world::World, who: CharId, cmnd: i16, [a1, a2]: [i16; 2]) {
        match who {
            CharId::Object(gate::GIMMICK) => self.gate.influence(cmnd),
            CharId::Npc(code) if self.grunty(code).is_some() => self.grunty_affect(world, code, cmnd, a1, a2),
            CharId::Npc(code) => {
                if let Some(n) = self.npc_mut(code) {
                    n.influence(cmnd);
                }
            }
            _ => {}
        }
    }

    /// Put an NPC on the entry control's list (`ccEntryCtrl::entryNpc`).
    pub fn add_npc(&mut self, npc: Box<dyn entry::Npc>) {
        self.npcs.push(npc);
    }

    /// `ccEntryEventMng` (main 0x001b62e0) for a Root Town, when the town's tasks
    /// start: the events' town NPCs (`ccSetRtownPC`), `ccSetMerchant(0)`'s
    /// merchants (the Chaos Gate is already up), then `ccEntryRandomNpc`
    /// (0x001b6cf0): the walking PCs `ccRegisterRandomNpc(n)` drew
    /// ([`World::init_rand`]), 15 for Kite alone, 14 with Orca. Each goes
    /// onto the entry list and its body onto the character list in that order.
    /// The first frame of play does it if it has not been called.
    pub fn place_entries(&mut self, world: &mut crate::world::World) -> piney_data::Result<()> {
        if self.placed {
            return Ok(());
        }
        self.placed = true;
        let reserved = world.spcs.registry_num + world.regist_npc_num;
        let no = world.scene.town;
        let server = area::SERVER_OF_TOWN.get(no as usize).copied().unwrap_or(0);
        let pool = crate::town::pc::Pool::of(&crate::town::pc::Tables::of(world.volume), server, &world.save.save);
        let rows = crate::town::pc::register_random_npc(&mut world.combat.cc, reserved, &pool);
        let town =
            Rc::new(std::cell::RefCell::new(crate::town::pc::TownPcs::new(world.volume, no, &world.area.core().file)?));
        self.town_pcs = Some(town);
        let mut files = crate::town::pc::Files::new();
        for e in std::mem::take(&mut self.event_npcs) {
            self.place_event_npc(world, &mut files, e)?;
        }
        let player = world.kite().pos();
        let event_npc = world.save.save.u8(offset::EVENT_STATUS + merchant::EVENT_NPC_STATUS) != 0;
        let core = world.area.core_mut();
        let merchants = merchant::set_merchants(
            &world.archive,
            world.volume,
            &core.file,
            &mut core.written,
            &mut core.hits,
            no,
            player,
            event_npc,
        )?;
        self.merchants.extend(merchants);
        // ccSetChaosGate (already up), then ccEntryEventMng's by town: 1
        // ccSetDog and ccSetChibiGuso, 2 and 3 ccSetChibiGuso, then the
        // walking PCs.
        let core = world.area.core_mut();
        self.dogs = dog::set_dogs(&world.archive, world.volume, &core.file, &mut core.hits, no)?;
        self.place_grunties(world, no)?;
        let town = self.town_pcs.clone().expect("placed above");
        for (slot, &r) in rows.iter().enumerate() {
            if r < 0 {
                continue;
            }
            let row = npc::NpcRow::of(world.volume, r as usize)?;
            let model = crate::town::pc::load_model(&world.archive, &mut files, &town.borrow().tables, &row)?;
            self.pcs.push(crate::town::pc::RtownPc::place(
                &town,
                &row,
                &model,
                slot as i32,
                player,
                &mut world.area.core_mut().hits,
                &mut world.combat.cc,
            ));
        }
        Ok(())
    }

    /// An event's town NPC (`ccEntryEventMng`'s types 3 and 4): `npcTbl` row
    /// `code` made as its class and set at the marker. A merchant goes onto
    /// the NPC list before `ccSetMerchant(0)`'s.
    fn place_event_npc(
        &mut self,
        world: &mut crate::world::World,
        files: &mut crate::town::pc::Files,
        e: EventEntry,
    ) -> piney_data::Result<()> {
        let row = npc::NpcRow::of(world.volume, e.code as usize)?;
        let (no, player) = (world.scene.town, world.kite().pos());
        match e.class {
            npc::EventClass::Pc if e.search => {
                // The SEARCH set-up: the row's stage is its event's status
                // (`saveData+0x6483+row`); a row with no marks makes nothing.
                let town = self.town_pcs.clone().expect("the entry control's set-up ran");
                let stage =
                    usize::try_from(e.code - 117).ok().map(|k| world.save.save.u8(offset::EVENT_STATUS + k) as i8);
                let file = &world.area.core().file;
                let set_up = stage
                    .and_then(|st| crate::town::pc::Search::set_up(&town.borrow(), file, i32::from(e.code), st.into()));
                let Some(s) = set_up else { return Ok(()) };
                let model = crate::town::pc::load_model(&world.archive, files, &town.borrow().tables, &row)?;
                let origin = crate::town::pc::Origin::Search(s);
                let pc = crate::town::pc::RtownPc::place_as(
                    &town,
                    &row,
                    &model,
                    origin,
                    player,
                    &mut world.area.core_mut().hits,
                    &mut world.combat.cc,
                );
                self.pcs.push(pc);
            }
            npc::EventClass::Pc => {
                let town = self.town_pcs.clone().expect("the entry control's set-up ran");
                let model = crate::town::pc::load_model(&world.archive, files, &town.borrow().tables, &row)?;
                let at = world.event_marker(e.marker);
                let hits = &mut world.area.core_mut().hits;
                let mut pc =
                    crate::town::pc::RtownPc::place(&town, &row, &model, -1, player, hits, &mut world.combat.cc);
                if let Some(at) = at {
                    (pc.char.pos, pc.char.dirc) = at;
                }
                self.pcs.push(pc);
            }
            npc::EventClass::Merchant => {
                let at = world.event_marker(e.marker);
                let m =
                    merchant::event_merchant(&world.archive, &row, &mut world.area.core_mut().hits, no, player, at)?;
                self.merchants.push(m);
            }
        }
        Ok(())
    }

    /// `eventMng.registNpcNum` after `ccRegisterEventMng` (MUT main
    /// 0x001cc480): `regist_npc16`'s 16 and the town NPC entries of the
    /// pass at phase 0. [`World::place_entries`] reserves it and the
    /// registry's characters before drawing the walking PCs.
    /// An event's town NPC entry: queued for [`World::place_entries`], or
    /// placed at once (at the list's end) once it has run. False when the
    /// entry control makes nothing of it ([`npc::NpcRow::event_class`]).
    pub(crate) fn entry_npc(
        &mut self,
        world: &mut crate::world::World,
        ty: i16,
        code: i16,
        marker: i16,
        param: i16,
    ) -> bool {
        let row = usize::try_from(code).ok().and_then(|r| npc::NpcRow::of(world.volume, r).ok());
        let Some(class) = row.and_then(|row| row.event_class(ty)) else { return false };
        let search = world.volume != piney_data::volume::Volume::Inf && ty == 3 && param == -1;
        let e = EventEntry { class, code, marker, search };
        if !self.placed {
            self.event_npcs.push(e);
            return true;
        }
        let mut files = crate::town::pc::Files::new();
        self.place_event_npc(world, &mut files, e).is_ok()
    }

    /// `ccSetChibiGuso` (gcmn 0x0050f0b0) in towns 1-3 (1-4 from Mutation
    /// on, MUT main 0x001cc370): the rows its growth record gives, each
    /// through `setDog` and `ccPGuso::ccPGuso` onto the list (the record of
    /// a grown one with a kind free starts over in the save).
    fn place_grunties(&mut self, world: &mut crate::world::World, no: i32) -> piney_data::Result<()> {
        let last = if world.volume == piney_data::volume::Volume::Inf { 3 } else { 4 };
        if !(1..=last).contains(&no) {
            return Ok(());
        }
        // The grown ones are made before the record starts over, the young
        // one after.
        let before = world.save.save.clone();
        let rows = grunty::set_chibi_guso(&mut world.save.save, no);
        if rows.is_empty() {
            return Ok(());
        }
        let tables = Rc::new(grunty::Tables::of(world.volume)?);
        let mut bodies = grunty::Bodies::load(&world.archive)?;
        let core = world.area.core_mut();
        for (num, id) in rows.into_iter().enumerate() {
            let g = grunty::set_dog(
                &world.archive,
                &tables,
                &mut bodies,
                &core.file,
                &mut core.hits,
                if id < grunty::ROW_YOUNG { &before } else { &world.save.save },
                no,
                id,
                num as u32,
            )?;
            self.grunties.extend(g);
        }
        self.grunty_tables = Some(tables);
        // From Mutation on, a grown Grunty in each pen: the race's flags
        // (MUT gcmn 0x0052bf1c).
        let g = grunty::Growth::read(&world.save.save, no);
        if world.volume != piney_data::volume::Volume::Inf && g.level == 4 && g.ty.iter().all(|&t| t != 0) {
            for n in 0..3 {
                if let Some(f) = race::Flag::new(&world.archive, world.volume, &world.area.core().file, no, n)? {
                    self.flags.push(f);
                }
            }
        }
        Ok(())
    }

    /// The Grunties `ccSetChibiGuso` placed.
    pub fn grunties(&self) -> &[grunty::Grunty] {
        &self.grunties
    }

    /// The Grunty of `code` (the row it was made from).
    pub fn grunty(&self, code: i32) -> Option<&grunty::Grunty> {
        self.grunties.iter().find(|g| g.code == code)
    }

    pub fn grunty_mut(&mut self, code: i32) -> Option<&mut grunty::Grunty> {
        self.grunties.iter_mut().find(|g| g.code == code)
    }

    /// `pgChatTbl[k]` (a grown Grunty's line over its head).
    pub fn grunty_chat_text(&self, k: usize) -> Vec<u8> {
        self.grunty_tables.as_ref().and_then(|t| t.chat.get(k).cloned()).unwrap_or_default()
    }

    /// `ccChar::EntryAffect(grunty, plw, cmd, a1, a2, 0)`: the Grunty's
    /// `affectFunc` (`dogAction`, `dogAction2`, `dogActionAdult`) at once,
    /// with `ccRand` for the grown ones' lines.
    pub fn grunty_affect(&mut self, world: &mut crate::world::World, code: i32, cmd: i16, a1: i16, a2: i16) {
        let Some(i) = self.grunties.iter().position(|g| g.code == code) else { return };
        let dirc = world.kite().dirc();
        self.grunties[i].affect(cmd, a1, a2, &mut world.combat.cc, dirc);
        let events = std::mem::take(&mut self.grunties[i].events);
        self.apply_grunty_events(world, code, events);
    }

    /// A Grunty's events: the camera (`changeCamera`, `cameraSetPos` and
    /// `cameraSetView` on `camID`'s camera) and Kite's place carried out,
    /// the rest kept for the host.
    fn apply_grunty_events(&mut self, world: &mut crate::world::World, code: i32, events: Vec<grunty::GruntyEvent>) {
        use grunty::GruntyEvent as E;
        for e in events {
            match e {
                E::Camera(n) => world.camera.change_camera(n),
                E::CamPos(v) => world.camera.active_mut().pos = v,
                E::CamView(v) => world.camera.active_mut().view = v,
                E::Player { pos, dirc_z } => {
                    let mut dirc = world.kite().dirc();
                    dirc[2] = dirc_z;
                    world.put_kite(pos, dirc);
                }
                E::Debug(_) => {}
                E::Chat(k) => self.outs.push(Out::Chat { code, text: self.grunty_chat_text(k) }),
                e => self.outs.push(Out::Grunty { code, e }),
            }
        }
    }

    /// Dun Loireag's dogs.
    pub fn dogs(&self) -> &[dog::Dog] {
        &self.dogs
    }

    /// The walking PCs.
    pub fn pcs(&self) -> &[crate::town::pc::RtownPc] {
        &self.pcs
    }

    /// The merchants `ccSetMerchant(0)` placed.
    pub fn merchants(&self) -> &[merchant::Merchant] {
        &self.merchants
    }

    /// The entry control's other NPCs.
    pub fn npcs(&self) -> &[Box<dyn entry::Npc>] {
        &self.npcs
    }

    /// Every NPC on the entry control's list, in its order: the merchants,
    /// the walking PCs, the rest.
    pub fn all_npcs(&self) -> impl Iterator<Item = &dyn entry::Npc> {
        let m = self.merchants.iter().map(|m| m as &dyn entry::Npc);
        m.chain(self.dogs.iter().map(|d| d as &dyn entry::Npc))
            .chain(self.grunties.iter().map(|g| g as &dyn entry::Npc))
            .chain(self.pcs.iter().map(|p| p as &dyn entry::Npc))
            .chain(self.npcs.iter().map(|n| n.as_ref()))
    }

    /// `ccEvent::GetNpc(code)`: the NPC whose base id is `code`.
    pub fn npc(&self, code: i32) -> Option<&dyn entry::Npc> {
        self.all_npcs().find(|n| n.code() == code)
    }

    pub fn npc_mut(&mut self, code: i32) -> Option<&mut dyn entry::Npc> {
        if let Some(m) = self.merchants.iter_mut().find(|m| m.code() == code) {
            return Some(m);
        }
        if let Some(d) = self.dogs.iter_mut().find(|d| d.code() == code) {
            return Some(d);
        }
        if let Some(g) = self.grunties.iter_mut().find(|g| g.code == code) {
            return Some(g);
        }
        if let Some(p) = self.pcs.iter_mut().find(|p| p.code() == code) {
            return Some(p);
        }
        self.npcs.iter_mut().find(|n| n.code() == code).map(|n| n.as_mut() as &mut dyn entry::Npc)
    }

    /// One of the town's own characters: an NPC on the entry control's
    /// list by its code, the Chaos Gate (`gimmickTbl[16]`) on its object
    /// list, on the command list while near (as the town's candidates).
    pub fn own(&self, world: &crate::world::World, id: CharId) -> Option<Own> {
        match id {
            CharId::Npc(code) => {
                let n = self.npc(code)?;
                // A Grunty's base as it has grown.
                let name = match self.grunty(code) {
                    Some(g) => Some(g.row.name.clone().into_bytes()),
                    None => npc::NpcRow::of(world.volume, code as usize).map(|r| r.name.into_bytes()).ok(),
                };
                let c = n.char();
                Some(Own { pos: c.pos, height: c.height, flags: n.flags(), name, listed: n.listed() })
            }
            CharId::Object(gate::GIMMICK) => Some(Own {
                pos: self.gate.pos,
                height: gate::HEIGHT,
                flags: talk::flags::CHAOS_GATE,
                name: Some(b"Chaos Gate".to_vec()),
                listed: self.gate.frame.near,
            }),
            _ => None,
        }
    }

    pub fn gate(&self) -> &Gate {
        &self.gate
    }

    // --- the event camera (camera/evcam.rs) ------------------------------------------

    /// A camera instruction of the event scripts (`Host::camera`, at play
    /// level): `ccEvent::Execute`'s handler, which starts the event
    /// camera's task (it first runs in this frame's tasks, after
    /// `ccThGameCtrl`) or, for `camera_end` and `camera_end_reset`, stops
    /// it and gives the field camera back. Every [`CameraCommand`] is
    /// handled.
    pub fn event_camera(&mut self, world: &mut crate::world::World, c: piney_event::host::CameraCommand) -> bool {
        world.event_camera_with(self, c);
        true
    }

    /// The tasks between the members' and the effects: the Flag Race's
    /// task (63), then `ccThEntryCtrl`'s list (64), after the battle's walk
    /// of its lists (empty in a town, so the race's slot is not crossed):
    /// the Chaos Gate, the race's flags, the merchants, dogs, Grunties,
    /// walking PCs and event NPCs, all seeing the active camera
    /// (activeCamPtr). What they started goes out with the battle's shows.
    fn entry_list(&mut self, world: &mut crate::world::World, host: &mut dyn race::RaceHost) {
        self.race_slot(world, host);
        let t = world.camera.active();
        let f = self.gate.step(world.volume, world.kite().pos(), t.pos, t.deg[1], t.kind == camera::kind::EYE);
        if let Some(se) = f.sound {
            self.outs.push(Out::GateSound { n: se, pos: self.gate.pos });
        }
        let (player, cam, volume) = (world.kite().pos(), world.camera.active().clone(), world.volume);
        for f in &mut self.flags {
            if f.step(volume, player, &cam).taken {
                let (n, pos) = (f.n, f.pos);
                self.outs.push(Out::Race(race::RaceEvent::Se3d(240, pos)));
                if let Some(r) = self.race.as_deref_mut() {
                    r.flag_taken(n);
                }
                self.outs.push(Out::Race(race::RaceEvent::OpenBox(pos)));
            }
        }
        let t = world.camera.active();
        let view = crate::chara::char::View {
            player: world.kite().pos(),
            cam: t.pos,
            deg1: t.deg[1],
            eye: t.kind == camera::kind::EYE,
        };
        let mut cx = entry::NpcCtx {
            player: world.kite().pos(),
            player_dirc: world.kite().dirc(),
            view,
            cam: world.camera.active(),
            hits: &mut world.area.core_mut().hits,
            rand: &mut world.combat.rand,
            cc: &mut world.combat.cc,
        };
        let status_on = world.save.save.u8(offset::EVENT_STATUS + merchant::EVENT_NPC_STATUS) != 0;
        for m in &mut self.merchants {
            m.status_on = status_on;
        }
        merchant::step_all(&mut self.merchants, &mut cx);
        // The Administrator's starts (as a field's event NPC's, on his
        // handle), and his (or the Event NPC's) deletion at the act's end.
        world.combat.shows.extend(self.outs.drain(..).map(combat::Show::Cast));
        for m in &mut self.merchants {
            let who = CharId::Npc(m.id).handle() as usize;
            for e in m.sysop_events.drain(..) {
                match e {
                    merchant::SysopEvent::Noise => world.noise = true,
                    e => world.combat.shows.push(combat::Show::Npc(who, e)),
                }
            }
        }
        self.merchants.retain(|m| !m.gone);
        for d in &mut self.dogs {
            d.step(&mut cx);
            let code = d.code();
            self.outs.extend(d.events.iter().map(|&e| Out::Dog { code, e }));
        }
        // ccSetChibiGuso's Grunties: each one's frame, then inuCheckNote
        // on the notes its anms passed, on pgPtr (the last one made).
        let mut pg_events = Vec::new();
        let last = self.grunties.len().wrapping_sub(1);
        for i in 0..self.grunties.len() {
            let mut gx = grunty::GruntyCtx {
                player: cx.player,
                player_dirc: cx.player_dirc,
                view: cx.view,
                hits: &mut *cx.hits,
                rand: &mut *cx.rand,
                cc: &mut *cx.cc,
                save: &mut world.save.save,
                race: self.race.as_deref_mut().map(|r| grunty::RaceLink {
                    kind: r.kind,
                    server: r.server,
                    step: &mut r.intro,
                }),
            };
            let g = &mut self.grunties[i];
            g.step(&mut gx);
            let code = g.code;
            let events = std::mem::take(&mut g.events);
            let notes = std::mem::take(&mut g.notes);
            pg_events.push((code, grunty_outs(&mut self.outs, code, events)));
            let pg = &mut self.grunties[last];
            grunty::inu_check_note(&notes, pg, i == last, cx.rand);
            let (code, events) = (pg.code, std::mem::take(&mut pg.events));
            pg_events.push((code, grunty_outs(&mut self.outs, code, events)));
        }
        if let Some(t) = &self.town_pcs {
            let at = offset::EVENT_STATUS;
            t.borrow_mut().event_status.copy_from_slice(&world.save.save.bytes()[at..at + 80]);
        }
        for pc in &mut self.pcs {
            pc.step(&mut cx);
            let code = pc.code();
            self.outs.extend(pc.events.iter().map(|&e| Out::Pc { code, e }));
        }
        for n in &mut self.npcs {
            n.step(&mut cx);
        }
        // The Grunties' camera writes and Kite's place, after the list (the
        // game's land as each one's main runs; the others' fades this
        // frame see the camera before them).
        for (code, events) in pg_events {
            self.apply_grunty_events(world, code, events);
        }
        if let Some(r) = self.race.as_deref_mut() {
            self.outs.extend(r.events.drain(..).map(Out::Race));
        }
        world.combat.shows.extend(self.outs.drain(..).map(combat::Show::Cast));
    }
}

/// A Grunty's frame's asks: its sounds, effects, voices and lines go out
/// in the list's order ([`Out::Grunty`]); its camera writes and Kite's
/// place come back, applied after the list.
fn grunty_outs(outs: &mut Vec<Out>, code: i32, events: Vec<grunty::GruntyEvent>) -> Vec<grunty::GruntyEvent> {
    use grunty::GruntyEvent as E;
    let (kept, out): (Vec<_>, Vec<_>) = events
        .into_iter()
        .partition(|e| matches!(e, E::Camera(_) | E::CamPos(_) | E::CamView(_) | E::Player { .. } | E::Debug(_)));
    outs.extend(out.into_iter().map(|e| Out::Grunty { code, e }));
    kept
}
