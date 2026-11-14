---
number: 591
title: The minimaps' arrow pulses run on from power-on as the game's statics
date: 2026-11-14
area: world, decomp, volumes, test
files: crates/piney-world/src/map/mod.rs, crates/piney-world/src/map/town.rs, crates/piney-world/src/map/field.rs, crates/piney-world/src/map/dungeon.rs, crates/piney-world/src/town/mod.rs, crates/piney-world/src/world.rs, crates/piney-desktop/src/save.rs, tools/test_map_rs.py
resolves: 590
started: 2026-11-14T03:22:55-07:00
took: 1h 46m
---

# 591. The minimaps' arrow pulses run on from power-on as the game's statics

[[590]] left the minimap pulses' phase across a change of scene open: the
port started each map's at 0. The game never does. Every map draw that
pulses its arrow keeps the angle in a function static of its own in the
executable's bss, zeroed once by `_start` at power-on, and nothing resets
it. The port now keeps the seven as slots of `SaveState::scroll_statics`,
stepped in place each drawn frame, so a pulse goes on across areas, visits,
log ins and soft resets as the game's does.

## The statics, on all four volumes

`ccRotate(r, 0.3134)` (INF and MUT main 0x00102190, OUT and QUA
0x001022a0) has eleven callers on Infection and Mutation and ten on
Outbreak and Quarantine (`disasm.py xrefs --scan`): `ccObjPart0580::Ctrl`
twice, `EVENTAREAB8::DrawRock` (INF, MUT only), `DUNGEON::SetIDOL`, and
the seven map draws. The address passed is each
draw's static:

```text
draw                  static   INF         MUT         OUT gp      QUA gp
ROOTTOWN01::DrawMap   r$1560   0x00378b4c  0x0038bad4  0x00386ea0  0x00279798
ROOTTOWN02::DrawMap   r$1488   0x00378b64  0x0038baec  0x00386eb4  0x002797ac
ROOTTOWN03::DrawMap   r$1569   0x00378b78  0x0038bb00  0x00386ec8  0x002797c0
ROOTTOWN04::DrawMap   r$1407   0x00378b88  0x0038bb10  0x00386ed8  0x002797d0
ROOTTOWN05::DrawMap   r$1313   0x00378ba0  0x0038bb28  0x00386eec  0x002797e4
WORLD::DrawMiniMap    r$3072   0x00378d04  0x0038bc8c  0x00387044  0x0027993c
DUNGEON::DrawMap      r$8238   0x00378d78  0x0038bd00  0x00387070  0x00279968
```

All lie past main's file image (INF ends 0x00378880, MUT 0x0038b800, OUT
0x00386b80, QUA 0x00279480), in the range `_start` (0x00100008-0x00100030)
clears with `sq $zero` up to `end` (INF 0x00730600, MUT 0x00774b00, OUT
0x00773780, QUA 0x00670300). Infection and Mutation guard each with a byte
static the word after it (`init$1561`, `init$1489`, `init$1570`,
`init$1408`, `init$1314`, `init$3073`, `init$8239`): the first call
stores 0 in `r` and sets the byte (INF `WORLD::DrawMiniMap`
0x005a3960-0x005a3974, `ROOTTOWN01::DrawMap` 0x00422d58-0x00422d6c,
`DUNGEON::DrawMap` 0x005cc18c-0x005cc1a0), the value bss already gave it.
Outbreak and Quarantine have no guard: their draws go straight to
`ccRotate` on the zeroed word. Either way the init value is 0 at power-on.

Nothing else touches them. On Infection the relocations name only the
three instructions in each draw (the guard's `sw $zero`, the `addiu $a0,
$gp` to `ccRotate`, the `lwc1` after); a full instruction scan of main
with each of gcmn, demo, desktop and toppage finds no other reference to
any of the seven, and the same scan on Mutation, Outbreak and Quarantine
finds only each draw's own (three with the guard, two without).
`LoadExecPS2` and `ExecPS2` have no callers (INF relocations), and GCMN.PRG's load at log in
brings the overlay's code and data, not main's bss. So no new game, load,
log in or soft reset resets a pulse; only power-off does. Quarantine's
`ROOTTOWN06::DrawMap` (0x004ecdd0) returns at once and has none. The
story maps draw no minimap (`WORLD::DrawMiniMap`'s one caller is
`WORLD::Draw`, 0x005a98a4; `DUNGEON::DrawMap`'s is `DUNGEON::Draw`,
0x005cef78). `alpharate$3075` and `alpharate$8241` (0.7) are guarded the
same way but never change.

## The port

`town::scroll_statics` gains `TOWN01_PULSE` .. `TOWN05_PULSE`,
`WORLD_PULSE` and `DUNGEON_PULSE` (slots 28-34, `SCROLL_STATICS` 35), the
same kind of word as the scroll statics beside them ([[474]]'s rule: the
executable's own small data, 0 at power-on, never set again), so
`carry_memory` already keeps them through the soft reset. `MinimapDef`
names its class's slot (`pulse`). The maps hold no `r` any more: each
`draw` takes its pulse as `r: &mut F`, and the minimap's frame carries the
world's live statics (`MapFrame::statics`, lent by `World::map_parts`
beside the area), so `town_minimap`, `field_minimap` and `dungeon_minimap`
step the word where it lives and `state_out` hands it to the next mode
with the rest of the save. `map_probe` keeps one statics array for its
whole run, as the game does from power-on.

## Checks

`tools/test_map_rs.py`'s dungeon case no longer stores 0 in `r$8238` as
each new `DUNGEON` is attached: the game's static and the probe's both run
on across the 24 dungeons (25 from Mutation), 2,594 packets on Infection,
2,696 on the others, none differing. As a control, the probe made to zero
its pulse with each dungeon fails at the second dungeon (the arrow's alpha
128 against the game's 20). `dun_loireags_scrolls_carry_across_visits`
now also checks that the field (area 66, a story map) carries `r$1488`
well past 0's first steps and that Dun Loireag, entered again by Gate Out,
steps on from it rather than from 0.

On the tip: the harnesses test_map_rs, test_field_rt, test_dungeon_rt,
test_town02_rs, test_town03_rs, test_evarea_rs, test_world_rs,
test_area_rs and test_field_ambient_rs on all four volumes, 36/36 rc=0,
every line as the base's; the battle pins 290 calls, 0 differ; piney-game
345 passed, 0 failed, 152 ignored; the other crates' 39 test binaries ok;
`gen --check` 29 current; clippy and fmt clean. Golden shots against
world2/shots/base (41b4153, made today): 4,550 files, 8 differ, all
ocarina_lake_shots (frames 0, 60, 120, 150-270). That test goes from a
lake dungeon down the stairs into a second dungeon, whose `DUNGEON::DrawMap`
now goes on from the lake's `r$8238`; the differing pixels lie only in
x 405-433, y 114-139, the player's arrow and the DOWN stairs sign, both
drawn at the pulse's alpha. Frames 30 and 90 are equal: the two pulses
give the same alpha there.

**Still unknown:**
- `RT01ICONPOS` (`RT02ICONPOS` ..; INF gcmn 0x005d4ad0, 0x005d4e90) is GCMN.PRG's own data, which the game keeps from visit to visit until the next log in; the port reads it afresh at each visit's `TownMap::open`, so a returning player sees every sign at alpha 128 with no places
