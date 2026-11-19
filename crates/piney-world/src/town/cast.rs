//! A Root Town's characters: the cast of the one world in area 0
//! ([`crate::world::cast`]). Kite is the battle's `ccPlayer` ([`crate::combat`]),
//! the entry control's NPC list ([`Npcs`]) the event's NPCs, the merchants,
//! dogs, Grunties and walking PCs; the Chaos Gate and the Flag Race stand
//! with them. The town's methods take the world's core as `world`.

use std::rc::Rc;

use crate::EventEntry;
use crate::event::gate::Gate;
use crate::grunty::{self, race};
use crate::systems::npcs::{Npc, NpcCtx, Npcs};
use crate::town::{dog, merchant};
use crate::world::cast::{self as cast_mod, Out};

/// The town's characters and what they started since the last take.
pub struct TownCast {
    pub(crate) gate: Gate,
    /// `ccEntryCtrl`'s NPC list in `ccEntryEventMng`'s order: the event's
    /// NPCs, `ccSetMerchant(0)`'s merchants, `ccSetDog`'s dogs and
    /// `ccSetChibiGuso`'s Grunties (Dun Loireag), `ccEntryRandomNpc`'s
    /// walking PCs, then any entered later.
    pub(crate) npcs: Npcs,
    /// Town NPC entries (`entry 3|4 code marker`) waiting for the entry
    /// control's set-up, and whether it has run.
    pub(crate) event_npcs: Vec<EventEntry>,
    pub(crate) placed: bool,
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
    fn npcs(&self) -> Option<&Npcs> {
        Some(&self.npcs)
    }

    fn npcs_mut(&mut self) -> Option<&mut Npcs> {
        Some(&mut self.npcs)
    }

    fn own(&self, _w: &crate::world::World, id: CharId) -> Option<Own> {
        TownCast::own(self, id)
    }

    /// The listed NPCs (their posP as their last frame left it), then the
    /// Chaos Gate while Kite is near, its posP `ccTransPosW2P` of its pos
    /// (`ccChgate::main` 0x004592a0), an object of the entry control's.
    fn cmnds(&self, player: crate::ee::V4, out: &mut Vec<talk::Cmnd>) {
        self.npcs.cmnds(out);
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
    fn lights(&self, out: &mut Vec<piney_data::light::Light>) {
        out.extend(self.gate.lights());
    }

    fn race_ride(&self) -> Option<(piney_battle::ride::RaceRide, bool)> {
        self.race.as_ref().map(|r| (r.ride, r.end))
    }

    /// The Flag Race's HUD and clips, then the entry control's lists: the
    /// gate, the race's flags, the NPC list in its order, as their tasks
    /// left them.
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
        self.npcs.draw(&mut ctx.layers, to_screen, lights);
    }

    /// The Flag Race's task (63), then `ccThEntryCtrl`'s NPC list (64).
    fn entry_slot(&mut self, w: &mut crate::world::World, fades: &mut dyn race::RaceHost) {
        self.entry_list(w, fades);
    }

    /// A newcomer built at the Chaos Gate ([`TownCast::invite_new`]).
    fn invite_new(&mut self, w: &mut crate::world::World, id: i32) -> bool {
        TownCast::invite_new(self, w, id)
    }

    /// The Chaos Gate's `Influence`, or an NPC's `affectFunc`.
    fn affect(&mut self, w: &mut crate::world::World, who: CharId, cmnd: i16, p: [i16; 2]) {
        match who {
            CharId::Object(gate::GIMMICK) => self.gate.influence(cmnd),
            CharId::Npc(id) => cast_mod::npc_affect(&mut self.npcs, w, id, cmnd, p),
            _ => {}
        }
    }

    /// The Flag Race started (from the next frame's slot), quit, done
    /// with; the breeder's writes into its Grunty.
    fn ask(&mut self, w: &mut crate::world::World, a: Ask) -> bool {
        match a {
            Ask::RaceStart(kind) => self.race_start(w, kind),
            Ask::RaceQuit => self.race.as_deref_mut().map(race::Race::quit).is_some(),
            Ask::RaceDone => self.race.as_deref_mut().map(|r| r.done = true).is_some(),
            Ask::Growth { code, growth } => self.npcs.grunty_mut(code).map(|g| g.growth_num = growth).is_some(),
            Ask::Food { code, food_mode, chat_flag } => self
                .npcs
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

    /// The entry control's NPCs, the Grunty of the target before or now,
    /// the race.
    fn seen(&self, shown: [Option<CharId>; 2]) -> Seen {
        Seen {
            npcs: self.npcs.ids(),
            grunty: shown.into_iter().flatten().find_map(|who| {
                let CharId::Npc(code) = who else { return None };
                let g = self.npcs.grunty(code)?;
                Some(GruntySeen {
                    who,
                    row: i32::from(g.core.row.id),
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
use crate::event::{gate, talk};
use crate::world::CharId;
use crate::world::cast::{Ask, GruntySeen, Own, RaceSeen, Seen};
use crate::{area, camera, combat};
use piney_data::save::offset;
use piney_desktop::anm::Ctx;

impl TownCast {
    /// `ccEntryEventMng` (main 0x001b62e0) for a Root Town, when the town's tasks
    /// start: the events' town NPCs (`ccSetRtownPC`), `ccSetMerchant(0)`'s
    /// merchants (the Chaos Gate is already up), then `ccEntryRandomNpc`
    /// (0x001b6cf0): the walking PCs the set-up's `ccRegisterRandomNpc(n)`
    /// drew (`World::set_registry_entries`, or a stream's reload since),
    /// 15 for Kite alone, 14 with Orca. Each goes
    /// onto the entry list and its body onto the character list in that order.
    /// The first frame of play does it if it has not been called.
    pub fn place_entries(&mut self, world: &mut crate::world::World) -> piney_data::Result<()> {
        if self.placed {
            return Ok(());
        }
        self.placed = true;
        let no = world.scene.town;
        let rows = match world.random_npcs {
            Some(rows) => rows,
            None => {
                // No set-up pass drew them (no event manager): the draw here.
                let reserved = world.spcs.registry_num + world.regist_npc_num;
                let server = area::SERVER_OF_TOWN.get(no as usize).copied().unwrap_or(0);
                let pool =
                    crate::town::pc::Pool::of(&crate::town::pc::Tables::of(world.volume), server, &world.save.save);
                crate::town::pc::register_random_npc(&mut world.combat.cc, reserved, &pool)
            }
        };
        let file = &world.area.core().file;
        self.npcs.pcs_or(|| crate::town::pc::TownPcs::new(world.volume, no, file))?;
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
        for m in merchants {
            self.npcs.push(Npc::Merchant(Box::new(m)));
        }
        // ccSetChaosGate (already up), then ccEntryEventMng's by town: 1
        // ccSetDog and ccSetChibiGuso, 2 and 3 ccSetChibiGuso, then the
        // walking PCs.
        let core = world.area.core_mut();
        for d in dog::set_dogs(&world.archive, world.volume, &core.file, &mut core.hits, no)? {
            self.npcs.push(Npc::Dog(Box::new(d)));
        }
        self.place_grunties(world, no)?;
        for (slot, &r) in rows.iter().enumerate() {
            if r < 0 {
                continue;
            }
            let row = npc::NpcRow::of(world.volume, r as usize)?;
            let town = self.npcs.pcs_mut().expect("made above");
            let model = crate::town::pc::load_model(&world.archive, &mut files, &town.tables, &row)?;
            let hits = &mut world.area.core_mut().hits;
            let pc =
                crate::town::pc::RtownPc::place(town, &row, &model, slot as i32, player, hits, &mut world.combat.cc);
            self.npcs.push(Npc::Pc(Box::new(pc)));
        }
        Ok(())
    }

    /// An event's town NPC (`ccEntryEventMng`'s types 3 and 4): `npcTbl` row
    /// `code` made as its class onto the NPC list's end (before
    /// `ccSetMerchant(0)`'s merchants at the set-up) and set at the marker.
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
                let town = self.npcs.pcs_mut().expect("the entry control's set-up ran");
                let stage =
                    usize::try_from(e.code - 117).ok().map(|k| world.save.save.u8(offset::EVENT_STATUS + k) as i8);
                let file = &world.area.core().file;
                let set_up =
                    stage.and_then(|st| crate::town::pc::Search::set_up(town, file, i32::from(e.code), st.into()));
                let Some(s) = set_up else { return Ok(()) };
                let model = crate::town::pc::load_model(&world.archive, files, &town.tables, &row)?;
                let origin = crate::town::pc::Origin::Search(s);
                let pc = crate::town::pc::RtownPc::place_as(
                    town,
                    &row,
                    &model,
                    origin,
                    player,
                    &mut world.area.core_mut().hits,
                    &mut world.combat.cc,
                );
                self.npcs.push(Npc::Pc(Box::new(pc)));
            }
            class => {
                let at = world.event_marker(e.marker);
                let hits = &mut world.area.core_mut().hits;
                let cc = &mut world.combat.cc;
                self.npcs.enter_event(&world.archive, files, &row, class, hits, cc, no, player, at)?;
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
            if let Some(g) = g {
                self.npcs.push(Npc::Grunty(Box::new(g)));
            }
        }
        self.npcs.set_grunty_tables(tables);
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

    /// The entry control's NPC list.
    pub fn npcs(&self) -> &Npcs {
        &self.npcs
    }

    /// One of the town's own characters: an NPC on the entry control's
    /// list by its name, the Chaos Gate (`gimmickTbl[16]`) on its object
    /// list, on the command list while near (as the town's candidates).
    pub fn own(&self, id: CharId) -> Option<Own> {
        match id {
            CharId::Npc(id) => self.npcs.own(id),
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
    /// task (63), then `ccThEntryCtrl`'s lists (64), after the battle's walk
    /// of its lists (empty in a town, so the race's slot is not crossed):
    /// the Chaos Gate, the race's flags, the NPC list, all seeing the
    /// active camera (activeCamPtr). What they started goes out with the
    /// battle's shows, after what the menus' `EntryAffect` asked of an NPC
    /// since the last slot.
    fn entry_list(&mut self, world: &mut crate::world::World, host: &mut dyn race::RaceHost) {
        let affected = self.npcs.take_events();
        self.outs.extend(affected.into_iter().map(|(code, e)| Out::Npc { code, e }));
        self.race_slot(world, host);
        let t = world.camera.list.active();
        let f = self.gate.step(world.volume, world.kite().pos(), t.pos, t.deg[1], t.kind == camera::kind::EYE);
        if let Some(se) = f.sound {
            self.outs.push(Out::GateSound { n: se, pos: self.gate.pos });
        }
        let (player, cam, volume) = (world.kite().pos(), world.camera.list.active().clone(), world.volume);
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
        let mut cx = NpcCtx {
            player: world.kite().pos(),
            player_dirc: world.kite().dirc(),
            cams: &mut world.camera.list,
            hits: &mut world.area.core_mut().hits,
            rand: &mut world.combat.rand,
            cc: &mut world.combat.cc,
            save: &mut world.save.save,
            race: self.race.as_deref_mut().map(|r| grunty::RaceLink {
                kind: r.kind,
                server: r.server,
                step: &mut r.intro,
            }),
        };
        self.npcs.step(&mut cx);
        world.combat.shows.extend(self.outs.drain(..).map(combat::Show::Cast));
        cast_mod::npc_outs(world, &mut self.npcs);
        if let Some(r) = self.race.as_deref_mut() {
            world.combat.shows.extend(r.events.drain(..).map(|e| combat::Show::Cast(Out::Race(e))));
        }
    }
}
