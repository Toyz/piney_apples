---
number: 132
title: The event's NPCs outside the towns: Meg and the Administrator in the dungeons
date: 2026-09-25
area: world
files: crates/piney-world/src/field_npcs.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/merchant.rs, crates/piney-world/src/rtownpc.rs, crates/piney-battle/src/entry.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session/tests/shrine.rs, docs/engine/field-game.md
---

# 132. The event's NPCs outside the towns: Meg and the Administrator in the dungeons

Some story events put town PCs and the Administrator in fields and
dungeons: events 14, 17, 26, 27 and 29, and S109. The port dropped them.
The area handed the entry control only the event's enemies and portals
(types 5 and 6), so every `npc_*` instruction fell to the host's default.
Event 29's dungeon scene played to an empty room: Meg's "Hello", the
Administrator's leave, BlackRose's "She's gone...". This entry builds
those NPCs and runs them
([field-game.md](../docs/engine/field-game.md#the-events-npcs-outside-the-towns)).

## What the game does

- **Same classes as the towns.** `ccEntryEventMng` dispatches types 3
  and 4 by bit, 0x08 and 0x10, to `ccSetRtownPC` and `ccSetMerchant`.
  Both make an `entryObject` of type 2, which `entryNpc` hands to
  `npcTbl[code].entry.func`. That gives the towns' own `ccRtownPC` and
  `ccMerchan`, in any area.
- **Position and heading only.** After it makes the object,
  `ccEntryEventMng` writes the position (+0x40) and heading (+0x60) and
  nothing else. Outside the towns they come from the marker's `evPos`
  (Kite's position added for 9999/9999). With no such number, the loop
  leaves its pointer on the 16th slot.
- **No navigation map needed.** A PC made with `param[0]` -1 starts in
  its event mode, so it only moves when an instruction tells it to.
- **The Administrator's clips.** `ccMerchan::ccMerchan` gives ids 29 and
  158 their own clip tables, `sysopeAnmTbl` and `quizmanAnmTbl`. They
  hold the same clip names as Mac Anu's and the second town's merchants.
  The port had them as "not ported" (`anm_tbl` None), so a town could
  not have made him either.

## What the port does now

- **Two halves per NPC.**
  - *The stand-in.* The battle's entry control holds a stand-in for each
    NPC (`combat::EventNpc`, made by `event_npc` in `start_entries`). It
    carries the npcTbl row's base (`entry::npc_char`, moved into
    piney-battle from the battle probe), and its body is out of the hit
    list. The command lists, `GetNpc` (the party's `pc_face 3 86`,
    `pc_walk_char`) and the effects all read the stand-in.
  - *The class.* The world keeps the class itself in
    `field_npcs::FieldNpcs`: an `RtownPc` over the new
    `TownPcs::outside`, or a `Merchant` from the new `Merchant::at`.
- **Each frame.** The classes step after the battle's tasks, over the
  field's collision and newlib `rand`. Each stand-in is then moved to its
  class's position, heading and command-list state. The NPCs are drawn
  with the characters' lights.
- **Instructions.** `FieldWorld::npc_command` and `set_trans` take the
  NPC instructions the way the town does; `AreaHost` gains `npc` and
  `trans`. `char_pos` answers types 3 and 4, for faces and the event
  camera.
- **Effects.** The NPCs' starts reach the next frame's effects as
  `Show::Npc`: a PC's `effTransfer`, and the Administrator's
  `effTransfer`, act -5 rings and sound 217. His `noise2` goes to the
  menus. When act 5 ends, `deleteNpc` removes the stand-in.

## Checks

- **`event_29_meg_and_the_administrator`.** It starts at event 29 in area
  26's dungeon and walks to point 1: floor 2, room 2, from `SetEventData`.
  There both NPCs stand at their markers. Every instruction of block 15
  is carried out and none is unported: the puts, turns and faces, `trans
  4 29 false`, `npc_act 29 4` and `npc_act 86 4`. The block plays through
  and the party leaves.
- **The walker.** The walk needed two fixes. Floor 1's room 2 is a
  two-level ring, where Kite enters in a trench under a ledge.
  `walk_to`, the shrine walker made general, now:
  - looks for a waypoint among the room's cells, by line of sight at the
    waist;
  - after four stopped checks in one room, puts Kite in front of the
    leg's door with the event's own `pc_put`, and logs it.
- **`event_29_shots`** (ignored; `PINEY_ROOM`, `PINEY_EVERY`). The shots
  show:
  - Meg, a silver-haired Heavy Blade, and the Administrator, big, in
    green, facing each other in the room;
  - the Administrator's transfer ring over him, then gone, with its
    sparks;
  - Meg's transfer rings as she fades, frames 790-800.

## Left

- Talking to the Administrator in a field is not tried yet. That is
  event 17's `add_target 4 29` and `talked_to`, through the stand-in on
  the object list.
- The test's party is the story start's (Orca), not BlackRose, so her
  `pc_*` instructions find no one.
- Events 14, 17, 26, 27 and S109 are not played through.

**Still unknown:** the second argument `ccEntryEventMng` passes
`ccSetRtownPC`. The port uses -1, as in the towns. The classes also step
outside the entry control's NPC turn, so their effects start a frame
later than the game's.
