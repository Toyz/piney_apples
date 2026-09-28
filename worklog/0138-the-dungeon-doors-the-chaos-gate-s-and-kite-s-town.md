---
number: 138
title: The dungeon doors', the Chaos Gate's and Kite's town footsteps' sounds
date: 2026-09-26
area: world
files: crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/motion.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session/tests/event22.rs, docs/engine/dungeon.md, docs/engine/field-walk.md, docs/engine/field-game.md
---

# 138. The dungeon doors', the Chaos Gate's and Kite's town footsteps' sounds

This came out of a sweep of the docs' Unknown lists for small gaps a
player would notice. `DungeonArea::move_door` already worked out when a
door opens or shuts, but `FieldWorld` dropped the result, so the dungeon
doors made no sound.

`MoveDoor` (gcmn 0x005cd3d0) picks the sound from two jump tables, @8659
for opening and @8664 for closing. Both are indexed by `DUNGEON.type`
(+0x10, confirmed by DWARF) and give the same sound for each type:

```text
type  0 1 2 3  4 5 6 7  8 9
se    45 46 47 51  45 46 47 51  48 48
```

`DungeonArea::door_se` names the sound. `FieldWorld` sends each door's
sound through the path the weather's sounds already take: an `Op::Se3d`
in the ambient calls, which the area plays at the door with the active
camera's ear.

`the_doors_sound_as_they_open` walks area 26's dungeon (type 3) to event
29's room. The rooms with enemies open their doors when cleared, and the
walk hears sound 51 nine times.

## The Chaos Gate's circle

Mac Anu's Chaos Gate had the same gap. `Gate::step` returned
`ccChgate::gateAnm`'s sound, 71 thirty frames after the circle starts
opening and 72 as it closes, but the town never picked it up. `World`
now keeps each sound with the gate's position (`take_gate_sounds`). The
town plays them as `Se3d` with the active camera's ear, next to the
Administrator's sounds.

`the_gates_circle_sounds` starts in Mac Anu, gives the gate the menu's
command 11 and, 100 frames later, command 0. It hears 71 at frame 31 and
72 at frame 100.

The gate-hack run never reaches 71, because the town sleeps under the
hack before the circle's count gets to 30. The game's count stops the
same way.

## Kite's footsteps in town

In a town Kite is piney-world's `Player`, not the battle's Kite, and its
`anim_ctrl` stepped his animation without reading the notes. So he ran
through Mac Anu without a sound. It now steps with `forward_notes`, as
`_AnimateForward` then `NoteProcess` do, and turns the footstep notes (1
and 2, as `ccPlayer::CheckNote` reads them) into `ActEvent::Step`. The
town keeps each one with his position and the ground's attribute
(`World::take_steps`) and plays it through `se3d::spc_note`, as the
fields play the battle Kite's. `kite_has_footsteps_in_mac_anu` runs him
for 90 frames and hears 12 steps. His running dust in town
(`ccEffPawSmoke`) is still missing.

The same sweep found one stale line, in field-walk.md. It said the
party's field AI was not ported, but the members follow and fight through
the battle's `ccAI`, as the walks' shots show. The line now says so.

**Still unknown:** `ChangeClut`, the door palette swap for clutType 3 and
4, is still not ported.
