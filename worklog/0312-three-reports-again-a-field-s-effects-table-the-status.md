---
number: 312
title: Three reports again: a field's effects table, the Status portraits, a member spoken to while running
date: 2026-09-30
area: ui, render, world
files: crates/piney-game/src/fx.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/town_return.rs, BUGS.md
---

# 312. Three reports again: a field's effects table, the Status portraits, a member spoken to while running

GitHub issues 2, 3 and 5 (Infection), called fixed too early. The reporter
builds from source; the screenshots are from a build after the fixes named
(10d3d88, 6edf655, d894a29). None of the three pictures was reproduced.
One real fault turned up on the way, in #5's census. Addresses are INF.

## Issue 5: what outlives a Data Drain

The report's paralysis is a scroll: Kite's skills (issue 1's screenshot)
have no Suvi Lei, and issue 10 is about spell scrolls. The Hanged Man is
`itemTblD` row 37, skill 157, category 11 (`tools/battle.py skills`: 157
Suvi Lei, paralysis 900 frames on a foe).

Tried on event 3's goblin, each drained once paralysed:
- The Hanged Man from PERSONAL's Items page 11 (`drain_with`, row 5);
- the same use through `FieldWorld::use_item` and `item_step`;
- Suvi Lei cast by Kite (`skill_request`);
- the drain with and without its movie.

Every time the sparks (two generators of row 198, `killFlag` 1) went at the
drain, with `ClearConditionEffect` (gcmn 0x00570180). No condition show was
lost between the battle and the effects. Nothing followed the drained foe.

The census also held two effects that never ended: 12 (`effAfterDrain`,
`ANM_xdhdref0`) and 22 (`effProtect`, `ANM_xdhpro*`). Both had no object.
`ccEffectCtrl`'s constructor (main 0x001c3030) looks each id up in
`effectTbl` (173 rows) when `game+0x14`, the area, is not 0 (0x001c31bc).
Only a Root Town takes `effectTbl2` (4 rows). The port's `EffectCtrl::new`
is the town's; `AreaFx` never switched it. So in every field and dungeon:
- no effect with an object drew (hit rings, the skill circles, protect
  breaks, the drain's wave);
- the clip-ended ones held their slot for good.

`AreaFx::new` now sets `town` false. 12 and 22 end with their clips
(`a_foe_paralysed_by_the_hanged_man_and_drained_leaves_nothing`; [22, 12]
still running 120 frames on without the fix). They never drew, so they are
not the report's sparks.

`FxCensus::char_generators` counted only `killFlag` 0. A condition effect's
generators are `killFlag` 1, so `a_drained_paralysed_foe_keeps_no_sparks`
could not see the sparks it named. It counts both now.

## Issue 3: the Status portrait

`StatusMenuDisp` (gcmn 0x005362d0) walks `ccPartyManager`'s slots to the
page and draws that slot's `menuFace`. `SetMenuFace` (0x00526940) and the
constructor's `ccCheckMenuFaceNameParty` (0x0056a930) set it. Every row of
`menuFaceCcsList` is a texture of `xwin_f00`, and all 19 read.

Tried: Orca, and BlackRose with Mistral, called by the Party menu; a field
by the console and back by `town 0`; back by PERSONAL's Gate Out. Each page
drew its slot's face with the member's row
(`the_status_portraits_come_back_from_a_field`). A headless GS render of
Orca's page before and after the field showed the face both times.

## Issue 2: a member spoken to while running

The greeting stopped Mistral within a frame of the press, and she faced
Kite, in every state reached:
- to a shop or a landmark (`actType` 5-10), the stick held through the
  press or not;
- fresh from the gate;
- after Kite on the CHAT follow order (3, 94, 97;
  `mistral_running_after_kite_stops_when_spoken_to`);
- on her way out after Remove, with the registry not full (2).

After event 14 (the story pilot, god on) Mistral is not in Mac Anu at all.

The screenshot's picture needs her in reach but out of view.
`ccCheckTargetRange` (gcmn 0x00519240) takes anyone within 60 plus both
widths, whatever the heading. In 2 of 4 menu tries she stood ahead of Kite
yet off the screen (`calc_tag_pos` mode 0).

**Still unknown:**
- the reporter's steps for all three, which a `--pad-log` would give;
- which field and foe #5's sparks came from;
- whether the game's camera shows a member so near;
- worklog 311's walking PC pushing a talking Mistral, not looked into.
