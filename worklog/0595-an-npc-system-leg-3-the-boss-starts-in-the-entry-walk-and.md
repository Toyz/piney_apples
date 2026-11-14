---
number: 595
title: An NPC system, leg 3: the boss starts in the entry walk and is made after it, the battle's NPC list gone with GetNpc written once, tests for the list's order and a dungeon PC's steps
date: 2026-11-14
area: world, battle, decomp, test
started: 2026-11-14T09:40:02-07:00
took: 2h 18m
---

# 595. An NPC system, leg 3: the boss starts in the entry walk and is made after it, the battle's NPC list gone with GetNpc written once, tests for the list's order and a dungeon PC's steps

The third and last leg of plans/rust-cleanup.md's "An NPC system", on
branch `npc-system` (rebased onto main dead2e3, the rustix bump; the
branch never touched Cargo.lock). It settled the two things leg 2 found
and left ([[594]]), added tests for leg 2's two fixes, and wrote the docs.

## The boss starts in the entry walk, and is made after it

`ccEntryEventMng` calls `ccBossEntryStart(code)` for an `entry 7` inside
its `entry[]` loop (INF SLUS_202.67:0x001b6780, MUT SLUS_205.62:0x001cbe78,
OUT SLUS_205.63:0x001c2548, QUA SLUS_205.64:0x001c9bc8), storing the
task in `eventMng` +0x7bc and the code in +0x08 (`bossEntry`). The port
started the boss after the whole set-up, which looked like a different
order. It is not. `ccBossEntryStart` (INF GCMN.PRG:0x0045b2a0, MUT
0x00470c40, OUT 0x0046c540, QUA 0x0035ee50, 100 bytes on each) only calls
`ccStartThread(ccThBossEffect, 66, 2048)` and
`ccStartThread(bossFunc[code], 66, 4096)` and clears the task's +0x14.
`ccTscb::GoThread` (INF SLUS_202.67:0x00159d40; MUT 0x00159e40, OUT
0x00158c80, QUA 0x00158b10) compares the new priority with the caller's
(`slt` at INF 0x00159dc0): 66 is not above `ccThEntryCtrl`'s 64
(`ccSetupGameCtrl` starts it with `li a1, 64` on all four volumes), so
`StartThread` readies the task without running it. The boss itself is
made in the task's first pass: `ccThBoss01` (INF GCMN.PRG:0x0047b340) news
`ccBoss01` and only then `Breath`s; `ccThBossEffect` (0x00461410) news the
`ccBossEffManager` the same way. Both run after `ccThEntryCtrl`'s slice
(and `ccThExecuteStream`'s, 65), so the boss's character, its `ccRand`
draws and its index come after every entry, NPC and portal
`ccEntryEventMng` made, on every volume - as the port already did. The
port now reads the `entry 7` in the same walk as the game
(`Combat::start_entries` hands its code back in `Entered`, with the
field's `fieldrand`), and the world makes the boss after the slot; the
world's own search for an `entry 7` went. Every event block has at most
one `entry 7` (tools/evscript.py dumps of all four volumes: twelve events
with one; OUT's and QUA's event 413 has two, in two blocks that each set
their own scene), so one code is all a walk can start.

## The battle's NPC list was a leftover

The battle crate's `EntryCtrl` kept `g_entCtrl`'s fourth list (Kind::Npc,
Obj::Npc, entryNpc, deleteNpc, the Seam's `npc_main` and `make_npc`,
`npc_char`, `SpawnTables::npcs`) and `evparty::get_npc` walked it. No port
path entered an NPC there: every NPC is the world's class, made through
the world's one list, and nothing else reaches `entryObject` with type 2.
A circle could in principle (`entryCircleObject` uses `param[1]` as the
type), but a circle's `param[1]` is -1 from `WORLD::SetMagicCircle` and
`DUNGEON::SetMagicCircle` (`ccEntryParamClear`) or the event's `entryMc`
type, which `ccEntryEventMng`'s second loop takes only for 0 and 1 (INF
SLUS_202.67:0x001b69d8, MUT 0x001cc0f8, OUT 0x001c27c8, QUA 0x001c9e48).
Every `npcTbl` row's `esize` is 1 (175 of 175, INF), so each NPC setter
(`setMerchant` INF GCMN.PRG:0x005057c4, `ccSetRtownPC` 0x0050675c,
`ccSetDog`'s four calls, all `entryObject(ep)`) makes one. So the list,
and what test_battle_spawn_rs compared of it (entryNpc, deleteNpc, the NPC
walk with a stubbed main), tested code no game path in the port runs; it
went. The battle's control holds three lists (`entry::LISTS`,
`Kind::ALL`), the harness leaves the game's NPC list empty and asserts it
stays so (it is the world's).

What the probe compared that is real is `ccEvent::GetNpc` (INF
SLUS_202.67:0x001b2cc0, MUT 0x001c8150, OUT 0x001be6e0, QUA 0x001c5d60),
which the world's list had its own copy of. It is now written once,
`evparty::get_npc` over a list's base types and ids, and
`Npcs::get_npc` calls it; `EvParty::char_pos` finds an NPC's place through
the world's list for type 0x18 (`find_char` holds only the battle's
characters). test_battle_event_rs's probe answers `GetNpc` from the NPC
list the harness lays out through the same function. `radiator` finds no
NPC: every script's `radiator` names Kite (type 2, code 0, one per
volume), so nothing reaches the case.

## Tests for leg 2's fixes

`an_events_pc_steps_and_draws_before_the_towns_npcs` (piney-game,
session::tests::npc_list): Infection's event 24 block 4 in Dun Loireag
(`entry 3 82 4`) puts PC 82 first on the town's list, then six merchants,
four dogs, a Grunty and thirteen walkers, the order `ccEntryEventMng`
enters them (INF SLUS_202.67:0x001b62e0). With the event's NPCs placed
after the walkers (the old order) it fails: the list starts with a
merchant.

`a_dungeons_event_pcs_sound_their_steps`: Infection's side event 59 with
blocks 0-16 run and the party put straight into event point 5's room
(floor 3, room 2 of story area 33's second dungeon, read from
`WORLD_MAN::SetEventData`'s rooms): block 17 enters PCs 87-90 and block
18 walks each 60 on (`npc_walk_dir`). Their steps (`rtpcCheckNote`, INF
GCMN.PRG:0x00509160) sound where they stand: 11 sounds with a note
(se 202, note 67), beside the four transfers (se 76, no note) of their
`npc_act 4`. With the field's cast keeping only a PC's `Transfer` (the
old field path) the test fails: no step sounds. No harness reached either
scene on the game's side (MerchantRun stubs `ccSetDog`, `ccSetChibiGuso`
and `ccEntryRandomNpc`; no harness runs `ccRtownPC` outside a town), so
both are piney-game tests.

## Docs

field-game.md's port notes describe the NPC classes on `Core`, the one
`Npcs` list in every area, `Out::Npc` and `CharId::Npc`, a field's NPC as
one object and the boss's start; battle.md the three lists and `GetNpc`
once; effects.md the NPCs' transfers through `Out::Npc`. The plan's "An
NPC system" and section 3's `Rc<RefCell<TownPcs>>` are ticked with
[[593]], [[594]] and this entry.

## Checks

On the tip, 2026-11-14: the town, field, story map, world and party
harnesses (test_town02_rs, test_town03_rs, test_world_rs, test_lights_rs,
test_gate_out_rs, test_party_rs, test_grunty_rs, test_merchcam_rs,
test_race_rs, test_map_rs, test_evarea_rs, test_field_ambient_rs,
test_field_rt, test_dungeon_rt, test_area_rs; four volumes) 60/60, with
test_evchar_rs, test_evcam_rs, test_bosscam_rs, test_battle_event_rs and
test_battle_kite_rs 20/20, each line as leg 2's where leg 2 ran it.
test_battle_spawn_rs passes on Infection and Mutation; on Outbreak and
Quarantine it cannot start, as on main: its trampoline jumps into
`ccThEntryCtrl`'s loop by `inf_va(0x00431B08)` (the loop head, `move a0,
s1` before `Breath`), and the symbol carry names no `ccThEntryCtrl` on
those two (Mutation's is at GCMN.PRG:0x00447300), so the address has no
counterpart there. test_battle_event_rs's
bulk run, 300 cases of each of its 14 checks on Infection, 0 mismatches.
The golden shots against the morning's base: 4550 files, 0 differ. The
battle pins: 290 calls, 0 differ. Crates 39 ok, 0 failed; the piney-game
suite 348 passed (346 and the two new), 0 failed, 152 ignored; gen
--check, clippy and fmt clean.

**Still unknown:**
- where `ccThEntryCtrl` (INF GCMN.PRG:0x00431970, MUT 0x00447300) is on Outbreak and Quarantine, which the symbol carry does not name, so test_battle_spawn_rs's trampoline (INF 0x00431b08) cannot reach their frame loop
