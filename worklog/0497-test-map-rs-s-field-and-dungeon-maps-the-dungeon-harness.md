---
number: 497
title: test_map_rs's field and dungeon maps, the dungeon harness, test_gate_out_rs and test_desktop_announce_rs run on all four volumes; the later gate-out IDs, volumeNum in the generator and DispInfo's extra frame ported
date: 2026-10-25
area: test
files: tools/test_dungeon_rt.py, tools/test_map_rs.py, tools/test_desktop_announce_rs.py, crates/piney-world/examples/dungeon_probe.rs, crates/piney-world/examples/map_probe.rs, crates/piney-world/examples/world_probe.rs, crates/piney-world/src/combat/gate_out.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/field_world.rs, crates/piney-event/src/vm/exec.rs, crates/piney-game/src/desktop.rs
started: 2026-10-25T21:18:53-06:00
took: 2h
---

# 497. test_map_rs's field and dungeon maps, the dungeon harness, test_gate_out_rs and test_desktop_announce_rs run on all four volumes; the later gate-out IDs, volumeNum in the generator and DispInfo's extra frame ported

The coordinator asked for four Infection-only harnesses to run on
Mutation, Outbreak and Quarantine: test_map_rs's field and dungeon maps,
test_gate_out_rs and test_desktop_announce_rs. Any port difference they
then showed was to be fixed. The dungeon map test runs on
tools/test_dungeon_rt.py's DUNGEON, so that whole harness now runs on
every volume too.

**The harnesses.**
- **test_dungeon_rt.py.** From Mutation on, DUNGEON is 0xce0 bytes
  (docs/engine/dungeon.md, "Other volumes"):
  - The per-floor arrays are `new[]` blocks, sized by the floor count at
    +0x430: realmap, rotate, pos, animIdx, minimap, roomdata, smallmap
    and startpos (now [floors][2]). MUT gcmn 0x005e3258 allocates them.
  - room[15][15] sits inline at +0x438; UpRoom, DownRoom, edit, center
    and envLight moved.
  - Every field from object[10] on sits 0xd2c30 lower: ccs at +0xc54,
    the leaf anm at +0xc8c.
  - `make` lays out a later DUNGEON the way the constructor does, with
    its blocks at ARRAYS. The rows' unused floor is the floor count, and
    EditDungeon has 90 rows.
  - The readers go through `at()` and `D()`.
  - `go_back` finds GO's way back by its code, not by Infection's
    addresses; `volumeNum` is the volume's own.
  - On Outbreak and Quarantine `DrawBG` keeps no `init$8918`.
  - A LEAF's centre w is whatever SetRoom's stack held (1.0 on Outbreak),
    and nothing reads it, so it is left out of the comparison.
- **The probes.** dungeon_probe and map_probe build the dungeon on the
  disc's tables (`new_banned` with its volume). Their walk's player and
  camera take the disc's square roots, as does world_probe's
  GateHackingOut camera (Outbreak's and Quarantine's `sqrt.s`). `eventdata`
  reads the disc's EditDungeon.
- **test_map_rs.py.**
  - `paint_span` finds the height-map loop by its code. Outbreak's
    recompiled `WORLD::Generate` keeps WORLD in `$s2`, not `$s0`.
  - The dungeon map takes its arrays through the layout.
  - The arrow pulse is the static `DrawMap` itself passes to `ccRotate`;
    `r$8238` exists only in Infection's names.
- **test_desktop_announce_rs.py.**
  - One fixture a volume (`announce_fixture_mut/_out/_qua.txt`).
  - The later save's extension sits behind its pointer.
  - From Mutation on there are 127 `eventAreaInfo` rows.
  - Each volume runs as its own first story event (101, 201, 301), one
    the port's pass walks.
  - The Rust replay (`announcements_match_the_game`) runs each disc.
    Like the harness, it starts from NewGame with no start event.

**The port differences found and fixed.**
- **`volumeNum` in a random dungeon.** `DungeonArea::new_banned` passed
  volume 1 to the generator on every disc. `ChooseRoomSize` reads it on
  servers 0-1. It now passes the volume's number.
- **The gate-hacked arrival from Mutation on.**
  - `ccAddRequestFileListSpc` (MUT gcmn 0x005cc4fc) maps fields 100-106
    and 108 to IDs 19-26 (`x7476cam` to `x7484cam`; read from
    `GateHackOutCcsName` on each disc). The port knew Infection's 19.
    `LATER_GATE_OUT` now carries them, and the lookups take the volume.
  - `ccPlayer::ccPlayer` (MUT gcmn 0x005c2f94) sets `ghoFlag` for any ID.
    It no longer asks `volumeNum < 4`, which Infection always passed, so
    the port's constant goes.
- **`DispInfo` on the desktop.** From Mutation on, a `ccBreathThread(1)`
  follows `dtMenu +0x10 = 1` (MUT main 0x001c7d68), so the announcement
  ends 11 frames after the close, not 10.

**Checked**, each on Infection, Mutation, Outbreak and Quarantine:
- tools/test_dungeon_rt.py: all 12 tests;
- tools/test_map_rs.py: all 10 (the town tests skip on Infection as
  before);
- tools/test_gate_out_rs.py: all 3;
- tools/test_desktop_announce_rs.py and the Rust replay of its four
  fixtures;
- piney-world's and piney-event's tests and piney-game's suite (335).

Docs: dungeon.md, map.md, field-walk.md (the gate-out IDs and the ghoFlag
rule), desktop.md and event-vm.md (DispInfo's frame, the fixtures).

**Still unknown:** nothing
