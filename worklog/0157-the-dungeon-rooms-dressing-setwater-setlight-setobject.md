---
number: 157
title: "The dungeon rooms' dressing: SetWater, SetLight, SetObject, SetAnmObject"
date: 2026-09-26
area: world
files: crates/piney-world/src/dungeon_area/dress.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/examples/dungeon_probe.rs, crates/piney-world/src/field_ambient.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/fx/ambient.rs, crates/piney-game/src/session/tests/dressing.rs, tools/test_dungeon_rt.py, docs/engine/dungeon.md
---

# 157. The dungeon rooms' dressing: SetWater, SetLight, SetObject, SetAnmObject

`DUNGEON::SetRoom` (gcmn 0x005c1ca0) builds the room's anm and its hits,
then calls `SetWater`, `SetLight`, `SetObject`, `SetDoor` and
`SetAnmObject`. The port had only `SetDoor`. Its rooms were bare: no
lamps, no lava sparks, no water, no moving floors or walls, no lake
statues. The characters were lit by the distant light alone.

## What the four do

All four find dummies in the room's anm with `ccAnm::GetSubstAdrs`, which
matches an ExtObj copy by its target's name (a trailing `*` a prefix, else
the whole name). They place a piece at the dummy's own matrix turned by the
room's `rotate` and moved to its centre, as `SetDoor` places its doors.

- **`SetWater`** (0x005c3f30): three anms of the type's `ANM_sdNaf0_a` at
  the first `OBJ_0paf0_`, moved but not turned, fog off, no hits. A `SNOW`
  of type 3 (an ember) sits at each `OBJ_o_magma*`, at z -100, with
  `DUNGEON::ChangeClut(eff, "c1"/"c2")` under clutType 3 or 4.
- **`SetLight`** (0x005c4560): at each `OBJ_o_light_m0*`, `OBJ_o_light_s0*`
  and (types 2 and 6) `OBJ_o_altar_l0*`, one to three `ROOMLIGHT`s by
  `type & 3`. Each is a glow at a fixed vector through the dummy's world
  matrix, a `fieldrand` pattern, a second glow for some, and a
  `ccOmniLight` (priority 1, 100 to 250) at the dummy in `cc3d`'s group.
- **`SetObject`** (0x005c70c0): the lakes' statues and flowers
  (`EntryObject`'s `ccClump`s at `OBJ_0ps0*`-`3*`, `OBJ_0ps4*`'s by
  `fieldrand(4)`); types 3 and 7's walls at `OBJ_0paw0*` and `OBJ_0paw1*`.
- **`SetAnmObject`** (0x005c3ad0): types 0-3's animated objects (`sd1ag0`,
  `sd2ag0`, `sd3ag0`, `sd5ag0`), with hits.

Each frame `DrawWater` scrolls waters 1 and 2 and draws water 0 over the
frame buffer. `DrawEff` moves and fades the sparks and runs each glow's
patterns; the light's intensity follows the pattern's transparency. Then
the clumps and anms are stepped and drawn.

## Two surprises

The walls' matrix goes to `anmobj[s0]`, `s0` the dummy's index in its own
search, not to the new anm (`anmobj[s1]`). A room with both kinds of wall
moves its paw0 walls to the paw1 dummies after their hits are placed, and
leaves the paw1 walls at the unit matrix with their hits at the world's
origin. The port does the same.

sd3's moving floor (`ANM_sd3ag0_a`) put an extra hit on the port's list:
`OBJ_o_move_l0_`'s model `MDL_o_move_l0_` is a five-word chunk with no
mmats and a Hit chunk. `ccStream::Decode_Model` makes no model of it (its
data word stays 0), `ccObj::Init` gives the object none, and its hit never
joins. `RoomFile` now leaves out models without mmats.

## The port

`crates/piney-world/src/dungeon_area/dress.rs` (a child of
`dungeon_area`): `Dressing` (water, sparks, room lights, clumps, walls,
animated objects), the four set-ups in `set_room_with` with
`DungeonArea.rng` as `fieldrand`, and `draw_water`, `draw_eff` and the
pieces' draw in `DungeonArea::draw`, which now takes the camera's
`world_screen` and returns `DrawEff`'s sprites. The pieces join the hit
list (`joined`, which was `door_hits`) in the game's order; new doors
keep them in place. `field_ambient::Sprite` gained `clut`
(`ccEff::ChangeClut`'s (new, old)), which `piney-game`'s ambient effects
apply only while the Eff's palette is the old one.

## Checked

`tools/test_dungeon_rt.py` now runs the game's four set-ups instead of
stubbing them, and `dungeon_probe`'s `room` reports the dressing.
`test_dungeons` compares each room's water, sparks (with the palette), room
lights and omni lights, walls, animated objects and `fieldrand` after it,
over 672 rooms: 6 waters, 738 sparks (291 recoloured), 1,183 room lights,
40 walls, 134 animated objects. A story dungeon's `Generate` draws for its
gimmicks, which the port does not, so its `fieldrand` is taken from the
game's first. `test_lake_dressing` runs four lake dungeons (34 rooms, 18
statues and flowers, 3 waters). `test_draw_eff` runs 70 frames of `DrawEff`
in 60 rooms. Every hit-list check still passes with the dressing's hits. 0
mismatches.

`piney-game`'s `delta_and_theta_rooms_are_dressed` walks Kite through 28
rooms of Δ and Θ random dungeons. `dressed_room_shots` (ignored) takes
pictures of rooms with sparks, walls, animated objects, statues and water
(default `/mnt/data/claude/scratch/dressing`).

**Still unknown:** the lakes' fireflies (`FIREFLY`, made by night, drawing
`fieldrand`), tree and leaves are not ported, nor `GetBG`'s palettes for
the lake clumps. What `ccObj::Duplicate` copies of the water's object is
assumed (each water's own materials), and no picture of the water or the
glows is compared with the game's. A story dungeon's `fieldrand` after
`Generate` is not the game's.
