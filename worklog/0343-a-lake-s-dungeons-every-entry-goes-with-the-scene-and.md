---
number: 343
title: A lake's dungeons: every entry goes with the scene, and entryFlag places them again
date: 2026-10-01
area: world, battle
files: crates/piney-world/src/field_world.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-game/src/session.rs, docs/engine/dungeon.md, docs/engine/field-walk.md
resolves: 191
---

# 343. A lake's dungeons: every entry goes with the scene, and entryFlag places them again

[[191]] left two questions about a lake's way between its dungeons. The
first: `come_back` builds the room's doors open, where the game asks
`ccCheckActiveObject`. The second: what reads `WORLD_MAN+0x58[n]`, which
`Enter` and `GoField` clear. Both answers came out of `ccThEntryCtrlDelete`
and `WORLD_MAN::EntryGimmick`. The port had the lake's portals, boxes and
idols wrong on the way back: carried over where the game deletes them,
and never placed again.

## +0x58[n] is EntryGimmick's entryFlag

`WORLD_MAN::EntryGimmick` (INF main 0x001a1f20), which `ccThEntryCtrl`'s
set-up calls:

```
WORLD_MAN.flag 1 (a field):   game.field 0: entryFlag[0] (+0x54) 0 ->
                                  WORLD::SetFood, SetMagicCircle, SetSpecialObj
                              else: +0x124 0 and +0x54 0 -> the same
WORLD_MAN.flag 2 (a dungeon): entryFlag[1 + game.dungeon] (+0x58 + 4 n) 0 ->
                                  DUNGEON::SetItemBox,
                                  SetMagicCircle (not with volumeNum 2 and game.field 27),
                                  SetIDOL; then the flag = 1
                              EntryBreakObject every time
```

A scan of every `WORLD_MAN` method and every main function that loads
`worldman` finds three other stores to +0x58, all clears of the dungeon
being left: `Enter` on a lake's stairs down (0x0019e240) and on dungeon
1's floor-0 stairs up (0x0019e104), and `GoField` from dungeon 1
(0x0019e45c). The `sw $zero, 0x58` in `ccSetupGameCtrl` (0x00168c3c) is
`game`'s, `inBattle`.

## A new scene keeps nothing

`ccThEntryCtrlDelete` (gcmn 0x00431d10) keeps the circles and the objects
not owned by a circle or a corpse in `g_entryList` only when
`ccGame::CheckSceneReplace()` (main 0x00167580) is false. That function
returns 1 when any of `CompArea`, `CompTown`, `CompField`, `CompDungeon`
finds the value differs from its previous one (+0x14/+0x18,
+0x20/+0x38, +0x24/+0x3c, +0x28/+0x40). Otherwise it deletes every
object. A room of the same dungeon is not a new scene. A lake's other
dungeon is, since `game.dungeon` changes. So at that change the lists are
emptied, and on the way back `EntryGimmick` sees the cleared flag and
places the lake's boxes, portals and idols again.

That also answers the doors. `GO(2)`'s `SetRoom(0, lastRoom)` runs before
`ccThEntryCtrl` starts, with the lists empty, so `SetDoor`'s
`ccCheckActiveObject(0, lastRoom)` (gcmn 0x0042e010: 0 when an entry of
either list is at that floor and block) answers 1 and builds them open, as
`come_back` does. `MoveDoor` then shuts them each frame
`ccCheckActiveObject()` finds an entry switched on in the room.

## The port

`FieldWorld::keep_entries` called `EntryCtrl::keep(cx, true)` (the "left
for a moment" case) on every way out of a dungeon. It now passes
`!Scene::changed()`. That function is `ccSetupGameCtrl`'s test and the
same four comparisons. `DungeonArea::enter` clears `gimmicks_placed` on the
lake's two stairs, and `FieldWorld::go_field` clears it in dungeon 1.

Before, coming back up the lake kept its entries from the first visit and
did not place them again. `a_lake_keeps_both_its_dungeons` now plays each
visit of area 33 to `Play(3)`. With the flag cleared as `Enter` clears it,
the lake holds 3 portals and 10 gimmicks on both visits. With the clear
but without the replace rule, the second visit had 6 and 20.
`a_type_4_area_goes_down_into_its_second_dungeon_and_back` checks the two
stairs' clears.

**Still unknown:** a lake's way between its two dungeons is still not run
against the game's own (`Enter`, `GO(2)`, `GoField` read from the code);
whether `SetItemBox` places a box already opened again on the way back (a
second take of its item) is not checked.
