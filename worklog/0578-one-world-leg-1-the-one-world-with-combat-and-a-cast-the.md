---
number: 578
title: One world, leg 1: the one World with Combat and a Cast, the town's state and logic on it behind a façade
date: 2026-11-03
area: world, build
files: crates/piney-world/src/world.rs, crates/piney-world/src/world/cast.rs, crates/piney-world/src/town/cast.rs, crates/piney-world/src/town/party.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/chara/party.rs, crates/piney-world/src/grunty/race.rs, crates/piney-world/src/town/ride.rs, crates/piney-world/src/event/mod.rs, crates/piney-game/src/area.rs
started: 2026-11-03T18:54:26-07:00
took: 33m
---

# 578. One world, leg 1: the one World with Combat and a Cast, the town's state and logic on it behind a façade

The first leg of the last piece of "One world, one area system"
(`plans/rust-cleanup.md`, section 2), done as a relay on the branch
`one-world` because the whole change has twice not fit one agent's
context. Main gets nothing until the last leg's merge passes the full
checks. This leg built the one world's shape and moved the town's state
and logic onto it; the town's `piney_world::World` is now a façade over
it, which the next legs remove.

## What the one world is

The game has one `WORLD_MAN` and one task list for every area; what its
tasks hold differs. In a Root Town Kite is `ccPlayer` and the entry
control's NPC list holds the merchants, dogs, Grunties and walking PCs; in
a field, dungeon or story map Kite is the battle's character and the
entry control holds enemies, portals and gimmicks, with the event's NPCs
as its stand-ins. So the port's one world is the field world's core (the
old `field::world::FieldWorld`, now `piney_world::world::World`), with
`combat: Combat` in every area (a town's is its party's) and
`cast: Box<dyn world::cast::Cast>` for what differs:

- `BattleCast` (`world/cast.rs`): the event NPCs' classes (`FieldNpcs`)
  and their starts, which were `FieldWorld`'s `npcs` and `npc_shows`.
- `TownCast` (`town/cast.rs`): the gate, Kite's arms, the town's `rand`,
  the NPC list and its queues, the town's party navigation, the Flag
  Race. Its methods (the town's frame, `ccThGameCtrl`, the entries, the
  party instructions, the race, the ride) take the world's core as
  `world`.

`Cast` methods are the places where the world reaches its characters:
`npc` / `npc_mut` (`ccEvent::GetNpc`: every town NPC kind and the field's
classes are `dyn event::entry::Npc`, so `World::char_pos`, `npc_command`
and `set_trans` ask the cast), `placed`, `npcs_frame`, `take_shows`,
`draw_npcs`. The world lends itself to its cast with `World::with_cast`;
tests read a cast's type through `<dyn Cast>::get` only, as
`<dyn AreaMap>::get` ([[572]]).

## The steps (each commit builds)

1. `FieldWorld` moved to `world.rs` as `World`.
2. `Cast` and `BattleCast`.
3. `TownParty` without a `Combat`: the town's world holds the combat, and
   the party's queries by `charTbl` row are `Combat`'s (`member_rec`,
   `member_place`, `member_actor`, `party_members`, ...).
4. The town's world holds the core (`World::town_core`, what
   `ccSetupGameCtrl` for area 0 sets) for its shared state; the town's
   requests are `world::Request::Game` in the core's list.
5. The town's remaining fields are `TownCast`, its logic `impl TownCast`
   (a script moved 115 methods: `self.w.x` to `world.x`, the 38 that read
   no core lost the parameter); the town's `World` forwards to it, and the
   event camera reads the town through `TownScene`.
6. The `TownCast` inside the core's `cast` (`impl Cast for TownCast`); the
   façade reaches it through `with_town`, `town`, `town_mut`, downcasts
   kept for the façade alone until it goes.
7. The town's `StartPos` and its events' party entries are the core's.

## What the code showed on the way

The town's own `rand()` and the party's `combat.rand` are already one
stream: the town copies its `rand` into `combat.rand` and back around
every party call (`TownParty::frame`, `Combat::build_member`, the
registry's builds in `chara/party.rs`), as the game has one newlib
`rand()` (`INF SLUS_202.67:0x00133a38`). The one world keeps
`combat.rand` alone once the two `Rand` types are one. `World::start()`
gives a town the same `(pos, [0, 0, rot, 0])` its own start held
([[577]]).

Of the town façade's 132 methods, 25 have the same body as the one
world's, 41 share a name but differ (the frame, `ccThGameCtrl`, the
targets by kind and code against scene indices, the party instructions,
the save out with its `rand`), and 66 are the town's alone: its per-system
getters and `take_*` queues, which the rules forbid on the one world and
which become generic events and Cast methods.

## Checked

At every step the build checks (`cargo check --workspace
--all-targets`, `cargo test --no-run -p piney-world -p piney-game`) and the
town's tests (piney-world `world`, `dun_loireag`, `later_towns`;
piney-game `world::tests`, `kept_dummies`, `town_return`) passed. At the
leg's end (release): `cargo test -p piney-world` passes (116 + 12 + 3 + 1),
the piney-game suite 347 passed, 152 ignored, as before the leg (the
suite aborts on an allocation under `ulimit -v 8000000`; it runs
uncapped, as the earlier legs ran it). The harness, golden shots and pins are the last leg's.
The relay's handoff (design as built, the method lists, the order of what
is left) is /mnt/data/claude/scratch/world3/HANDOFF.md.

**Still unknown:**
- the town façade (`piney_world::World`, `World::with_town`, `town`, `town_mut`) still stands: piney-game's `WorldMode` and `Stage::World` hold it, not `world::World`
- the 41 methods the town and the one world both have but do differently (the frame, `ccThGameCtrl`, the targets by kind and code against scene indices, the party instructions, the save out) are not yet one method each with the cast answering what differs
- the town's 66 own methods (per-system getters, `take_*` queues) are not yet generic events or Cast methods
- one effect host, one mode and one `HostWorld` are not started
- `TownCast.rand` and `combat.rand` are still two copies of one stream with two `Rand` types
