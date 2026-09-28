---
number: 164
title: A field's gimmicks: SetFood, SetMagicCircle and SetSpecialObj
date: 2026-09-26
area: world
files: crates/piney-battle/src/entry.rs, crates/piney-world/src/field_area.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-effect/src/gimmick.rs, tools/test_battle_spawn_rs.py, crates/piney-game/src/session/tests/field_gims.rs
---

# 164. A field's gimmicks: SetFood, SetMagicCircle and SetSpecialObj

The port's fields held only the event's entries. `WORLD_MAN::EntryGimmick`'s
field branch never ran there. That branch makes the random fields' magic
portals and enemies, so the fields had no fighting outside the story's
own circles. `world_set_magic_circle` was ported and checked (`world_mc`)
long ago, but the runtime never called it. Now all three setters run
each time a field is entered. `docs/engine/battle.md`, "Where the
entries come from", has the rules.

**SetFood** (gcmn 0x005aa990):

- A lake field gets the Spring of Myst (gimmick 20) at the lake's `wp`.
  `WORLD.water` is the lake's index among the `FOBJECT2`s, and index 0 is
  always the dungeon entrance.
- Each object with one to three `OBJ_xgfood0a*` nodes gets `fieldrand(n)
  + 1` foods of the field type's row, on nodes picked by `ccRand() % n`.

`ccRand()` is signed. A negative remainder indexes the stack below the
`used` array, where the search's count and table pointer sit, never 0. So
the game just draws again. The first comparison showed exactly that: the
port and the game picked different nodes until the port retried
negatives too.

**SetSpecialObj** (gcmn 0x005aae90) places gimmick 21 (`ENTRANCE`, the
swirl `effDungeonEntrance`) at the two in-points, which are
`GetDungeonMarkerPoint`'s `DMY_inpoint0/1` of the field's file (w 1)
plus the entrance's `wp`. A list of story areas gets none. It then puts
symbols (gimmick 18) on `OBJ_xgsymb*` nodes when `fieldrand(100) >= 41`.
All ten field files carry those nodes. This corrects worklog 0163 and the
docs: row 21 is placed in Infection, in every random field.
`effDungeonEntrance` (main 0x001d0c90) is ported: generator 238 with
`pTexMod` from `effDungeonEntranceColor[2]` (0x67). Its store sits in a
branch's delay slot, so -1 is stored too.

**The runtime.** `FieldArea::field_gims` reads the objects as `Generate`
left them. The node places come from the anm's controllers, posed with
EE rounding by `dungeon_area::pose_in` (split out of the dungeon's posing)
with the identity at the root, or from a bare clump's nodes, which are at
their rest (place 0). The root is the identity because nothing has drawn
the field yet. The entry task (priority 64) runs its set-up before
`ccThFieldDisp` (96) in the first walk. `FOBJECT::Draw` would otherwise
have set the anm's root to the object's place, and adding `wp` would then
count it twice. `Combat::start_entries` runs the three setters on the
field's `fieldrand` and gives it back to the field, so the weather goes on
drawing from where they left it, as the game's does.

**Checks.**

- `tools/test_battle_spawn_rs.py`'s new `field_gims` runs the game's
  `SetFood` and `SetSpecialObj` natively on a `WORLD` of random objects.
  The objects are `fobj2` and chips' `fobj`, with and without anms, with
  0-5 food and 0-3 symbol nodes, a lake or none, and areas in and out of
  the list. `GetSubstAdrs` and `GetObjAdrsF` are hooked to answer each
  object's nodes, whose coords carry the places. All 300 cases were
  equal. The spawn suite is green.
- piney-game's `a_field_has_its_portals_entrance_and_foods` and
  `a_lake_field_has_its_spring` enter random Δ fields by the gate. Type 5
  had 8 portals, 24 enemies, some thirty foods, the two swirls and six
  symbols. Type 10 had 12 portals, 11 enemies, the spring, some thirty
  foods and four symbols.
- `field_gims_shots` (ignored) shows the snow field's first fight: BATTLE
  MODE ON, a moth downed for 60 EXP.

`test_dungeon_rt` (10 tests, with the posing now shared) and 605
workspace tests pass.

**Still unknown:** The node places in the field's objects are the
dungeon posing's, which was checked on rooms, not on field objects
against the game's `_SetLWMatrix`. Where the foods stand in the game was
not looked at in a picture (the sums put them 250-370 from their
objects' centres). The game's result for a field with no dungeon entrance
(the in-points then 0) is unknown.
