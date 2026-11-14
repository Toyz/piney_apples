---
number: 594
title: An NPC system, leg 2: the walking PC and the Grunty on Core, one NPC list and one event channel in every area, a field's NPC one object
date: 2026-11-14
area: world, decomp, engine
files: crates/piney-world/src/systems/npcs.rs, crates/piney-world/src/chara/npc.rs, crates/piney-world/src/town/pc.rs, crates/piney-world/src/grunty/mod.rs, crates/piney-world/src/town/cast.rs, crates/piney-world/src/world/cast.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs
started: 2026-11-14T07:53:11-07:00
took: 1h 45m
---

# 594. An NPC system, leg 2: the walking PC and the Grunty on Core, one NPC list and one event channel in every area, a field's NPC one object

The second leg of plans/rust-cleanup.md's "An NPC system", on branch
`npc-system` (steps 5-7 of the relay's handoff, scratch/npc/HANDOFF.md).
Every NPC class now holds `chara::npc::Core`, and every area has one NPC
list, `systems::npcs::Npcs`, with one event channel, `Out::Npc`. What the
game's code settled is below; the checks are at the end.

## A field's NPC is one object

The design kept a field's event NPC as two halves: a stand-in on the
battle's entry control (`combat::EventNpc`, an `Obj::Npc` with a scene
character) and the class in `field::npcs::FieldNpcs`, the stand-in copied
from the class each frame. The game has one. `ccEntryEventMng` (INF
SLUS_202.67:0x001b62e0) hands an entry of type 3 or 4 to `ccSetRtownPC`
(0x001b67dc) or `ccSetMerchant` (0x001b67c4); each goes through
`entryObject` to `entryNpc` (gcmn 0x004317b0) and the row's `entry.func`,
which constructs the `ccRtownPC` or `ccMerchan` and appends it to
`g_entCtrl`'s NPC list. In a field or dungeon (`game.area` 1 or 2) the
returned object's `pos` (+0x40) and `dirc` (+0x60) are then written from
the marker's `evPos` (0x001b6888-0x001b6978: the first of the 16 with the
marker's number, else the 16th; Kite's place added for floor and block
9999). That object is the character: its `bodyHit` is the one on the
character list, `ccThEntryCtrl` (gcmn 0x00431970) walks it last, after the
enemies, circles and gimmicks, and it draws in its own `main`. No second
object exists, so the stand-in, `who`, `Combat::npcs`, `event_npc`,
`FieldNpcs` and `Show::Npc` are gone. The port's battle entry control keeps
its fourth list for the battle probe; no world code enters an NPC there.

`ccEntryEventMng`'s first loop (0x001b6304-0x001b69a4) takes `entry[]` in
index order: the party's set-up, an enemy or object of 5 or 6, the boss of
7, an NPC of 3 or 4. Its second (0x001b69b8 on) takes `entryMc[]`. The
port made the portals first, then the enemies, then the NPCs at the
cast's `placed`. `Combat::start_entries` now walks `entry[]` in order, an
NPC made there through the world (`npc`, onto the cast's list and the
character list at that moment), then the portals; the hit list and the
scene indices follow the game's order. The boss's `ccBossEntryStart`
(0x001b6780) still starts after the set-up.

## The Grunty's code

`ccEvent::GetNpc` (INF 0x001b2cc0, MUT 0x001c8150, OUT 0x001be6e0, QUA
0x001c5d60) walks the NPC list for the first whose base type has bit 4
(codes 29 and 158) or bit 3 (any other code) and whose base id (+0x0c) is
the code. npcTbl's types: merchants 0x100-0x1000 and 0x08000000 (the
breeders), the Administrator 0x10, the PCs 0x08, the dogs 0x04000000, the
Grunties 0x02000000 and 0x01000000. So `GetNpc` never finds a merchant, dog
or Grunty, whatever its base. What does name a Grunty is the entry:
`setDog` (gcmn 0x0050ef10) builds `ep.id = id` (0x0050ef48) and
`ccPGuso`'s later `SetBaseParam` calls leave `entParam` (+0x100) alone; the
menus' read of the list is `entParam.id` (+0x124, piney-fieldui's
`World::npcs`). `Core::id` is that (`EntryObj::ent_id`, set from the
row by `Core::new`) and `CharId::Npc` names an NPC by it; `Core::code` is
the base id. `Npcs::get_npc` is `GetNpc` with its masks (the world's
`npc_command`, `set_trans`, `find_char_pos`, `ev_party`); `Npcs::get` is by
name (talk, menus, effects, the merchant camera). `World::base_id` reads
the NPC's base as it is (CheckOperate's code). The Recorder test turned
Kite with `pc_face 3 4`, which in the game finds no one; it now turns him
with `pc_turn` and checks that the face is refused. Row 139's drain reads
`entParam.id` (`+0x124 == 139`, INF gcmn 0x00508d40).

## The list, its order and its events

`Npc` is the class by `entry.func` (Merchant, Pc, Dog, Grunty), with
`core()` and the vtable's `main` and `affectFunc` dispatched in one match.
`Npcs::step` is `ccThEntryCtrl`'s turn: for as many as the list held,
each with `objFlag`, its step (`routine`, then `main`); a merchant whose
`main` gives up is deleted there, and a deletion ends the walk short of
the count as the game's null `next` does. After each Grunty,
`inuCheckNote` on its notes with `pgPtr`, the last Grunty made. The
Grunties' `objFlag` is now set by `setDog`'s `initObject` as the
merchants' and dogs' are; every NPC is checked as `ccThEntryCtrl` checks.

The town's order is the game's: the event's NPCs, `ccSetMerchant(0)`'s
merchants, `ccSetDog`, `ccSetChibiGuso`, `ccEntryRandomNpc`. Before, an
event's PCs stepped and drew after the merchants, dogs and Grunties.

Each class's asks go out in list order as `Out::Npc { code, e: NpcEvent }`
(`Merchant(SysopEvent)`, `Pc(PcEvent)`, `Dog(DogEvent)`, `Grunty`, `Line`
for a grown Grunty's `pgChatTbl` line) in every area; the Administrator's
`noise2` goes to the menu, a Grunty's camera writes and Kite's place are
applied after the list. What the menus' `EntryAffect` asked of an NPC goes
out first in the next slot, as the town's queue did. A field's PCs now
sound their steps and show their chat, crystal and drain: `rtpcCheckNote`
(gcmn 0x00509160) is the anm's note callback wherever the PC animates.
The effects find an NPC by `CharId::Npc(code).handle()` through the cast
in every area (`char_pos`, `char_height`, `check_target`); `char_dirc` is
read by none of the NPCs' effects (`effTransfer`, `effAfterDrain`,
`effEvolvePG`, the force rings: position and height only).

## The classes on Core

`RtownPc` and `Grunty` hold a `Core` (the PC's `actNum` +0x2a4 and
`actProcess` +0x2a6 were shorts; every write is a small constant or a count
from 0, so i32 changes nothing). The walking PCs' state (`TownPcs`,
rtownnpc.cpp's globals) is owned by the list and lent `&mut` to each PC;
its copy of `eventStatus` went, a SEARCH PC reading the save as the game
does. `GruntyCtx` is `NpcCtx` with the save and the race link. Each
class's turn round a body it touches is `Touch::avoid` (the same code in
`ccRtownPC::move` 0x005077f0, `ccDog::move` 0x00509a80, `ccPGuso::move`
0x0050bca0); the dog's and the Grunty's stuck check (posCnt, oldPos, 120
frames, 50 on an axis) is `chara::npc::Stuck`, the PC's own (60 frames, a
distance, `returnRoute`) staying its own. `Char::disp` (never false, no
part of `ccChar::Draw`, gcmn 0x0056b1c0) and the `event::entry::Npc` trait
went, with `TownCast::npcs`/`add_npc` and the per-kind vectors.

## Checks

On the tip (code at 5acb45f; the clippy fix after the harness and shots
only dropped two no-op `drop`s and named the callback's type), 2026-11-14:
the towns', fields', story maps', events' and world/party harnesses
(test_town02_rs, test_town03_rs, test_grunty_rs, test_merchcam_rs,
test_race_rs, test_field_rt, test_field_ambient_rs, test_map_rs,
test_world_rs, test_lights_rs, test_evarea_rs, test_evchar_rs,
test_area_rs, test_party_rs, test_gate_out_rs; four volumes) 60/60, each
line as leg 1's; the golden shots against the same day's base, 4550 files,
0 differ (no shot has an event's PC beside the town's NPCs where the order
shows, nor a field PC's step dust in frame); the battle pins 290 calls, 0
differ; crates 39 ok; the piney-game suite 346 passed, 0 failed, 152
ignored (its first run caught the walk indexing past a deletion, in
event 301 through town 5); gen --check, clippy, fmt, cairns and docs
clean.

**Still unknown:** nothing about the NPCs. `ccEntryEventMng` also starts a
boss (`entry 7`, `ccBossEntryStart` at 0x001b6780) inside its `entry[]`
loop, where the port starts it after the set-up; whether that order shows
(the boss's characters against the event's enemies' scene indices) is not
settled here. The docs (field-game.md's "The port keeps each such NPC as two
halves", battle.md's NPC lines) and the plan's tick are leg 3's.
