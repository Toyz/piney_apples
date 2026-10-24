---
number: 442
title: Quarantine's town 5 is town06: a sixth Root Town no symbol names, reached by the gate's town 4 while event 301 holds eventStatus[51], and event 301 now ends
date: 2026-10-24
area: world, volumes, script, test
files: crates/piney-world/src/town06.rs, crates/piney-world/src/town.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/field_world.rs, crates/piney-gen/src/placement/statics.rs, crates/piney-fieldui/src/menus/gate.rs, crates/piney-game/src/session.rs, crates/piney-game/src/world.rs, crates/piney-game/src/loaddisp.rs, crates/piney-audio/src/driver.rs, crates/piney-game/src/session/tests/town06.rs, crates/piney-game/src/session/tests/town_shots.rs, docs/engine/root-towns.md
started: 2026-10-24T01:31:36-06:00
took: 18m
---

# 442. Quarantine's town 5 is town06: a sixth Root Town no symbol names, reached by the gate's town 4 while event 301 holds eventStatus[51], and event 301 now ends

Event 301 stopped in Fort Ouph: its block 7 is in town 5, which the port
could not build and the pilot could not reach. Town 5 is now ported and
reached the game's way. Event 301 ends from its own start with no aid.

## What town 5 is

`WORLD_MAN::GO(0)` switches on `game.town` with six cases (QUA main
0x001b2ac0). The sixth calls gcmn 0x004ec350, a constructor no symbol
names. It asks for `town06` and builds:

- 3 static models from a table at QUA gcmn 0x005ce5b0: `MDL_sr6flo1`,
  `_obj1`, `_obj2`, types 1-3, clip 100,000;
- five clumps: `CMP_sr6met1`, `_bac1`, `_cro1`-`3`;
- one distant light, `LGT_sr6lig1` of `ANM_sr6town1a`;
- Lia Fail's fog and background colour.

Its vtable (0x002785e0) gives `Draw` (0x004ecde0):

- `DrawBG`: the three scrolls nothing reads; `bac1` on `bgLayer[0]`; the
  crosses on refLayer (`SetActiveLayer(6)`); `met1` on `bgLayer[1]`.
- On objLayer: `DrawObj2`, `DrawObj`, `DrawFloor` by type.
- `DrawMap`, which returns at once.

The code after it (0x004eced0 on) is the Grunty race's flags, not the
town.

## Reaching it

Nothing names town 5 as a destination: every `town_move` of the story is
3 or 4. The gate's town menu turns the chosen town 4 into 5 while
`eventStatus[51]` is set (QUA gcmn 0x0047305c).

- Event 301's block 3 sets that byte ("You can now go to Ω Server").
- Block 8 clears it before its `scene` to town 4.
- The check is in Mutation's and Outbreak's `GtTownMenu` too (MUT
  0x0058499c, OUT 0x0058071c), not in Infection's.
- Quarantine's carry calls this code `GtRandomMenu+0x964`: its
  `GtTownMenu` symbol was never carried.

The port's `town_menu` now does the same from Mutation on. The pilot's
`town_row` knows that town 4's row lands in 5.

## What town 5 reads

- **The start.** `SetCharPosition` has cases for towns 0-4 only (QUA
  main 0x001b3ac0). For town 5 `StartPos` (main bss 0x00670190) keeps the
  last scene's start. The session now carries it (`start_before`), and
  `World::enter_after` takes it.
- **The loading card.** `townNameTbl`'s sixth row (QUA main 0x00212afc)
  repeats Lia Fail's words.
- **The music.** `sqDataTown` runs to twelve rows on the disc. Town 5's
  rows 5 and 11 are empty, so the town loads no bank
  (`town_06_has_no_bank`).
- **The gate.** In town 5, block 8 plays on arrival with the menus
  barred, and after 301 nothing sets the byte again. So no input reaches
  town 5's gate or its departure stream.

## The table

piney-gen's statics placement now finds Root Towns whose constructor has
no symbol. It looks for each `GetCCSAdrs("townNN")` outside every named
function, then takes the table the next `STATICMODEL::STATICMODEL` is
given and the loop's `sltiu` bound. That yields `RT_MODELTABLE06` (3
rows) and the scene set `ROOTTOWN06: town06`.

## Checks

- `town06::tests`: the constructor's pieces and a frame's order.
- `event_301_goes_through_town_5`: from 301's start under `StoryPilot`,
  no god. Towns visited: 3, 5, 4. Event 301 ends at frame 16,932.
- `town_shots` (`PINEY_DISC=quarantine PINEY_TOWN=5`): the gate, the
  stone ring and the stairs under banners.
- The survey's 301 is done.

**Still unknown:** nothing
