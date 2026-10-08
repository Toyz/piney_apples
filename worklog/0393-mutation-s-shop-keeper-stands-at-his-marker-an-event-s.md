---
number: 393
title: Mutation's Shop Keeper stands at his marker: an event's entry 4 is the Administrator's ccMerchan, and every town NPC entry is made by its row's class
date: 2026-10-08
area: world, volumes, test
files: crates/piney-world/src/lib.rs, crates/piney-world/src/event.rs, crates/piney-world/src/npc.rs, crates/piney-world/src/merchant.rs, crates/piney-world/src/field_npcs.rs, crates/piney-world/tests/world.rs, crates/piney-world/examples/world_probe.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/shop_keeper.rs, tools/test_world_rs.py, docs/engine/field-game.md, docs/README.md, UNKNOWNS.md, BUGS.md
---

# 393. Mutation's Shop Keeper stands at his marker: an event's entry 4 is the Administrator's ccMerchan, and every town NPC entry is made by its row's class

Issue #60 (Mutation, build 4cce396): when the Shop Keeper trades Kite's
Book of Law for the Book of Absolute, his lines play but no one stands
by the gate. The port's console said `events: entry 4 29 at marker 6 not
placed`.

## The event

Event 101 (M201), the new game's, in Dun Loireag (town 1, `town02`):

- Block 24 ends Chosen Hopeless Nothingness's dungeon: `status_set 0 1`,
  then `scene 0 1`.
- Block 25 (`in_town 1`, phase 0, `eventStatus[0] == 1`) registers
  `entry 4 29 6 0`: `npcTbl` row 29, at marker 6.
- Block 27 (phase 4) turns BlackRose, Kite and him to face each other,
  puts the camera on him (`camera 2 29`) and has him say three lines
  ("Shop Keeper" is the messages' speaker). `item_del 15 60`, `item_add
  15 69` trade the books, and `npc_act 29 5` sends him off.

Row 29 is "Administrator" on every volume: flags 0x10, `CTR1.CCS`,
`ccEntryRtownMerchant`. It is the Administrator's `ccMerchan`.

## What the game does

`ccEntryEventMng` (MUT main 0x001cb990, INF 0x001b62e0), for each
registered entry:

- The bit `1 << type`, 0x18, takes types 3 and 4. 0x10 (type 4) calls
  `ccSetMerchant(code)` (MUT gcmn 0x00520910). In any area, a non-zero
  code makes only 29 or 158 (`setMerchant`, 0x005205c0: no dummy, the
  origin).
- 0x08 (type 3) calls `ccSetRtownPC(code, -1)` (0x00521cf0). `$a1` still
  holds the loop's `li $a1, -1` (MUT 0x001cb9dc, INF 0x001b6318). That
  answers [[65]]'s and [[132]]'s question.
- Both are an `entryObject` of type 2, which calls
  `npcTbl[code].entry.func`. So the row picks the class: `entry 3 29`
  (Outbreak's event 209, in Fort Ouph and outside) is a `ccMerchan` too.
- If an object was made, its `pos` (+0x40) and `dirc` (+0x60) are
  overwritten. A marker below 0 gives `vf0` for both. In a town, marker
  N gives `markerEvTbl[N]`'s dummy: its position and its whole rotation
  (w 0). Outside a town, the event position.
- From Mutation on, a type 3 with `param` -1 goes to a new set-up
  instead (0x00521e50, the SEARCH events' PCs). It is not moved to a
  marker.
- `ccRegisterEventMng` (MUT 0x001cc480) adds 1 to `registNpcNum` for
  every type 3 or 4 entry, made or not. `ccRegisterRandomNpc` then makes
  16 minus Kite, the party and those.

`ccMerchan::main` changed in Mutation (MUT gcmn 0x005210b0; Outbreak and
Quarantine are the same):

- Only the Grunt Shops (`breederAct`), 29 and 158 (`sysopeAct`) and the
  new rows 175-179 (0x00521a00) act and run `CollisionDetection`.
- The other shops do neither. Infection runs `breederAct` and the
  collision for all of them.

## Cause

`World::entry_npc` accepted only type 3 of a row with the PC flag (0x08).
Every `entry 4` in a town was refused, and so was a type 3 of a
merchant's row. The fields' event NPCs (`FieldNpcs::make`) chose the
class by the type, so `entry 3 29` outside a town tried to build a PC
from `CTR1.CCS`.

## Fix

- **The class.** `NpcRow::event_class(ty)` gives `EventClass::Pc` or
  `Merchant` from the row's `entry.func`, or None. Type 4 makes something
  only for 29 and 158 (`merchant::SYSOPE`, now also documented as
  `ccSetMerchant`'s codes). The town and the fields both use it.
- **The town.** `World::entry_npc` queues an `EventEntry` (class, code,
  marker) and counts every type 3/4 entry in `regist_npc_num`.
  `place_entries` makes them first, then extends `merchants` with
  `ccSetMerchant(0)`'s. The Administrator is first on the NPC list, as in
  the game.
- **The marker.** `event::event_marker_in` returns `vf0` below 0, else
  `marker_dummy_in`: the dummy's position and whole rotation.
  `marker_in` (the z heading) is built on it. Event PCs now take the
  whole rotation too, as the game copies it.
- **The merchant.** `merchant::event_merchant` is `ccSetMerchant(29)`
  followed by the marker. `World` and `world_probe` both use it.
- **Mutation's main.** `Merchant::main` runs `breederAct` and the
  collision for every merchant on Infection only. From Mutation on it
  does so for the breeders and 29/158 alone.

## Checked

- **Reproduced and fixed.**
  - `mutations_shop_keeper_stands_at_marker_6` (piney-game). It brings
    event 101 to the end of block 24 and logs Kite and BlackRose in to
    Dun Loireag. The Administrator must be first on the NPC list, at
    marker 6's dummy (rotation x, y, w copied; z then turned to Kite by
    block 27's `npc_face`), with `ANM_ctr1nut0`, and drawn while he
    speaks.
  - Before the fix it fails with `entry 4 29 not placed`.
- **Shots** (`shop_keeper_shot`, ignored):
  - `/mnt/data/claude/scratch/i60/before/mut-shop-keeper.png`: Kite and
    BlackRose by the gate, no one else. This is the report's picture.
  - `/mnt/data/claude/scratch/i60/mut-shop-keeper.png`: the Shop Keeper
    left of Kite by the gate, as in the PS2 video, at "You are the player
    that won the Power Up Campaign, right?".
- **Every entry.** `every_event_npc_entry_makes_a_class` walks every
  `entry 3|4` in every volume's scripts (649; each disc carries every
  volume's events). The area of each comes from the scene tags kept from
  block to block.
  - With the old rules, 12 a volume were not placed: 8 in towns, 4
    outside.
  - In towns those are `entry 4 29` in events 101, 106, 107, 113, 301,
    309 and 313, and Outbreak's `entry 3 29` in event 209. Outside, event
    209's other two and events 216 and 218.
  - The ones each disc plays: Mutation 4 (101, 106, 107, 113), Outbreak
    6 (209 three times, 216, 218), Quarantine 3 (301, 309, 313).
  - With the new rules none are refused.
- **Against the game's own code** (tools/test_world_rs.py,
  `MerchantsAgainstGame`, eemu):
  - The new `test_event_merchant` runs the game's `ccEntryEventMng` on
    an `eventMng` holding `entry 4 29 6`, in town 1. The gate, the dogs,
    the Grunties and the walking PCs are left out. The port's side is
    `world_probe merchstart` with the town and the entries.
  - The two lists match: ids 29 then 5-10, positions, `posP`, headings,
    `defaultDirc`, clips and `hit_sw`. So do 600 frames of `routine` and
    `ccMerchan::main` (with `sysopeAct`, a talk, a farewell and a turn).
  - It passes on Mutation and Infection.
  - The harness's entry control now has the town as its `areaNum`
    (`initObject` compares it). Town 0 had hidden this.
  - `test_merchants` now also passes on Mutation. Before the main fix it
    differed at frame 0 in `actProcess` and the collision offset. Its
    "touching" and talk-clip coverage is Infection's only.
  - Outbreak's harness module does not load
    (`__vt__10ROOTTOWN01` has no carry), as before.
- `the_entry_controls_set_up` (piney-world) now registers the
  Administrator at marker 5 and an `entry 4 40` that makes nothing. The
  merchants are 29, 0-4. The walking PCs are 13: the event PC and 12
  chosen with all three entries counted.
- piney-game's suite (four threads), piney-world's tests, clippy on
  piney-world and piney-game, fmt, `cairns check` and `tools/docs.py
  check` pass.

**Still unknown:** two later-volume pieces this work turned up. Both are
in the repo's reach but are features of their own.
- Mutation's `ccRtownPC` (MUT gcmn 0x00522040, 740 bytes longer than
  Infection's; `main` and `normalMode` differ too) is not ported. The
  differences:
  - the walking PCs' body kind is 8 (Infection 2);
  - `rtpcCcsName` has 50 names;
  - rows 159 and up have texture tables of their own;
  - the SEARCH PCs' `entry 3 CODE MARKER -1` set-up (0x00521e50: a
    landmark or a dummy, from tables 0x0061c890 and 0x0061c8c0, by
    `saveData+0x6483+row`, then `param` 1 and the state) is not ported.
    The port stands them at their marker. The SEARCH chain plays on
    Outbreak.
- From Mutation on, `ccSetMerchant(0)` also makes row 175 + town ("Event
  NPC", flags 0x10000000) when `saveData+0x652c` is set. It has its own
  act (0x00521a00) and affect (0x00521990). The port makes none, and
  what sets the byte is not traced.
