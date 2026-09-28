---
number: 161
title: The lakes' sky and fireflies: DrawBG and SetRoom's FIREFLYs
date: 2026-09-26
area: world
files: crates/piney-world/src/dungeon_area/lake.rs, crates/piney-world/src/field_firefly.rs, crates/piney-world/src/field_ambient.rs, crates/piney-world/examples/dungeon_probe.rs, tools/test_dungeon_rt.py, crates/piney-game/src/session/tests/dressing.rs
---

# 161. The lakes' sky and fireflies: DrawBG and SetRoom's FIREFLYs

The lake dungeons (types 8 and 9, field type 4) had no sky, and their
fireflies were listed as not ported. Both are done now.
`docs/engine/dungeon.md`, "The lakes' sky and fireflies", has the details.

The constructor makes the sky's clumps from the dungeon's own file, by
`GetBG`:

- type 8: `CMP_o_bac_l{n}_`, `bac_m`, `clo_l` and `clo_m`;
- type 9: `bac_l`, `bac_m`, `ero_l`, `ero_m` and `ero_s`.

Each is unfogged. Their models are domes about 12,000 across. `DrawBG`
draws them at the room's centre on the five lowest layers (-100 to -60),
scaled 0.5, 1 or 1.5 by the room's size. It also scrolls one material's
V by 0.003 a frame (the clouds). That scroll is one static for the whole
game. The room's pieces are drawn over the domes, so the sky shows only
above the walls and through the doorways. A picture with only the first
eight draws kept showed the domes were there all along, behind the forest.

By night (`GetTime` 2, which for field type 4 is bgnum 2) or with
`ishack` at 3, `SetRoom` ends by making five `FIREFLY`s. Each gets its
constructor's pattern, then a base up to 1,000 on from the room's centre
in x and y (so all five sit in one quarter of the room), then the field's
`SetBasePosition` and `Init`. Their sprites come from `field_eff`, which
the constructor keeps at +0x34c. `DrawEff` moves and draws them after the
glows, as the field's `FIREFLY`s. One thing had to change for that: the
firefly code took the field's bounds (48,000) as a constant, and a dungeon's
are 60,000. `field_ambient::Env` now carries its bounds.

`tools/test_dungeon_rt.py`'s new `test_lake_night` runs six lakes:

- four of type 8, by night and hacked;
- two of type 9 (`sd9`, hacked, by day).

The harness now decodes `field_eff` beside the dungeon's file and sets
`ishack` and +0x34c. It makes the constructor's clumps itself (`new
ccClump`, `Init`, `SetFogSw`) before calling `DrawBG`. Results:

- 51 rooms of `SetRoom` fireflies;
- 960 frames of `DrawEff` with them, with the player at a room's centre
  and then 4,000 off, which sends them back about him;
- 480 frames of `DrawBG`: the V halfword and every clump's matrix.

There were 0 mismatches. Two things came up on the way:

- A firefly's base w is whatever `SetRoom` left on its stack, so only xyz
  is compared.
- `sd9` holds bgnum 0's clumps alone. The one way to a type 9 lake is
  story area 14's words, so no other bgnum reaches it.

`piney-game`'s `a_lake_by_night_has_its_sky_and_fireflies` enters a Δ
lake of bgnum 2 and checks three things: the frame's first four models
are the sky's, the scroll runs, and the fireflies fly.

**Still unknown:** Why decoding `sd9` a second time after `sd4` makes the
interpreter's `DecodeSetup` read a bad pointer; the test runs `sd9` last.
`EntryObject`'s palettes for the lakes' statues and flowers under bgnum 1
to 3. The port's scroll starts at 0 in each dungeon, while the game's
runs on from the last lake.
