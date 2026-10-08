---
number: 403
title: The Grunty's search in a field: Mutation's ride carries it but never starts it, Outbreak and Quarantine start it with Triangle, and the Grunty sniffs out foods, the dungeon or portals, says so and leads Kite there
date: 2026-10-08
area: world, volumes, build, test
files: crates/piney-battle/src/ride.rs, crates/piney-battle/examples/ride_probe.rs, crates/piney-world/src/combat/ride.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/field_world/ride.rs, crates/piney-world/src/town_ride.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/ride.rs, crates/piney-gen/src/ride_seek.rs, crates/piney-gen/src/lib.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/combat.rs, crates/piney-data/src/pack.rs, tools/test_ride_rs.py, docs/engine/grunty-ride.md, UNKNOWNS.md
resolves: 395
---

# 403. The Grunty's search in a field: Mutation's ride carries it but never starts it, Outbreak and Quarantine start it with Triangle, and the Grunty sniffs out foods, the dungeon or portals, says so and leads Kite there

[[395]] left part of Mutation's field ride unported: a "charge" (MUT gcmn
0x0052f790), a "knockback" (0x0052f250), "Grunty chat" (0x00530e50,
0x00530dd0) and a `+0x1e0` range. Read whole, they are one feature, the
Grunty's search. The page is
[grunty-ride.md](../docs/engine/grunty-ride.md) ("The search in a field").

## What the game does

- **0x0052f790 is the search, not a charge.** It goes by the kind's type
  (0x0061d180):
  - kind 0 finds the nearest Grunty food among `g_entCtrl`'s gimmicks
    (base type 0x800000, `gimmickTbl` rows 22-37);
  - kinds 1, 3, 5 and 7 find the field's dungeon (`WORLD_MAN.dungeonPos[0]`),
    unless the event area is one of the 20 with none (main 0x001b1600);
  - kinds 2, 4, 6 and 8 find the nearest magic portal.

  It sets the target (+0x1d0) and the name (+0x24c). It answers -1
  within 100 past the range, 1 farther, 0 for none. The ranges
  (`+0x1e0`, 0x0061d1a8 by type) are 1000, 2400 and 1000.
- **0x0052f250 is the run, not a knockback.** `Main` calls it for
  `ControlMove` while +0x158 is set:
  - Triangle calls it off, and the stick takes over;
  - the Grunty stands 20 frames, then runs at 1.3 of its speed toward the
    target;
  - within the range it stops and says it is there.
- **`ControlMove`** keeps the search's idle count. Standing 40 frames, the
  Grunty sniffs again. Triangle calls the search off.
- **The chat** is the Grunty's line in a balloon over itself:
  - 0x00530e50 copies the kind's found, none, cancel or near line
    (0x00684b50-0x00684be0) into +0xe1, `#a` the name, and cuts it at 79;
  - 0x00530dd0 opens it at the end of `Main`, outside a town.

  Kind 0 says "Mon ami! I smell Piney Apple this way!".
- **The constructor** sets the range, `hold` 10 and the target (0, 0, 0, 1).
  It also calls `ccEntryCmnd(this)`, which Infection's does not: the
  balloon needs the ride on a command list (`ccCheckTarget`). No target is
  chosen from it, as `ccSelectTarget` (gcmn 0x00536018) runs only with no
  menu open, and the flute's `WaitRide` (or the race's menu) stays open.
- **Mutation never starts it.** Flag bit 6 starts the search, and no
  instruction in Mutation's executable or overlay sets it: every store to
  a +0xe0 flag byte that sets bit 6 is ccFellow's, ccPlayer's or
  ccSpcChar's own, and the ride's code only clears it. So on Mutation the
  search, the run and the lines are dead code.
- **Outbreak and Quarantine start it.** Their `ControlMove` opens with it
  (OUT gcmn 0x00529834, QUA 0x0041c144). In a field with the search off,
  Triangle sniffs:
  - found: the search on, +0x158 40 (a count down before the stick may
    take over), `hold` 20, the found line;
  - none: the none line.

  Every line was reworded ("I think the dungeon is this way, clang!").
  The run and the search are otherwise Mutation's, with `sqrt.s`.

## Fix

- **Tables.** `piney-gen` finds them per volume (`ride_seek.rs`):
  - the constructor's first two unnamed tables are the types and the
    ranges;
  - from `ControlMove` to `PadLeverPower`, the four line tables come in
    first-use order.

  `tables::combat` has the `ride_seek_*` tables, empty on Infection.
  `DATA_VERSION` is 31.
- **`piney_battle::ride`**:
  - `Seek` (+0x158-+0x160, +0x1d0, +0x1e0, +0x24c) and `Ride::chat`;
  - `seek` (0x0052f790 and its three finders) and `seek_move` (0x0052f250);
  - `ControlMove`'s parts and Outbreak's start;
  - `say` (0x00530e50) and `open_chat` (0x00530dd0);
  - `Out::EntryCmnd`, `Out::Chat` and `Input::push`, `pow_r`.

  The `RideWorld` gains `seek_list`, `dungeon`, `w2p` and `kite_name`.
- **piney-world**:
  - the field's `RideWorld` reads the entry control's gimmicks and
    circles, the event area and `dungeonPos` (`SeekView`, `Tasks::dungeon`);
  - `RideObj::listed` follows `ccEntryCmnd` and the dismount's
    `ccDeleteCmnd`.
- **piney-game** opens the Grunty's balloon in task order with the
  party's (handle `3 << 24`), placed by `FieldWorld::ride_chat_point`.

## Checked

- **Against the game's code** (tools/test_ride_rs.py on Mutation, Outbreak
  and Quarantine). The states now take:
  - flag bits 6 and 7 and the search's members;
  - the entry lists (foods or not, near and far, on, going), the event
    area, `dungeonPos`, the player's place, Triangle, L1, R1 and `powR`.

  The run and the search also run alone. The lines (`OpenChat`) and the
  answers are compared. All 15 cases pass on all three, and Infection's 12
  are unchanged.
  - A count of 300 cases on Mutation: the search answered -1 (83), 1 (105)
    and 0 (112); `Main` opened all four kinds of line, `#a` names among them.
  - On Outbreak 68 of 400 `ControlMove` cases took the Triangle start, 42
    of them finding something.
- **Played** (piney-game, `ride.rs`):
  - `mutations_grunty_never_searches_a_field`: a new game in a field of
    Dun Loireag's server, the flute and a kind-1 Grunty. Triangle and
    standing start nothing over 200 frames, and the ride is listed.
  - `outbreaks_grunty_leads_kite_to_the_dungeon`: Triangle, the found line
    over the ride, the target the entrance. It stands 20 frames, runs from
    (10200, 600) toward (10200, 4800), and stops at frame 54, 2392 short,
    with the near line.
- **Other harnesses.** tools/test_race_rs.py and tools/test_grunty_rs.py
  pass on Mutation (the town ride shares the constructor).
- **The rest.** A unit test checks the lines: `#` only as `#a`, and each
  fits the cut with the longest name. piney-game's suite (four threads),
  the tests of piney-battle, piney-world, piney-gen and piney-data, clippy,
  fmt, `piney-gen gen --check`, `cairns check` and `tools/docs.py check`
  pass.

**Still unknown:** nothing. The game's copy would loop on a `#` not before
`a`, which no line has. The port copies it.
