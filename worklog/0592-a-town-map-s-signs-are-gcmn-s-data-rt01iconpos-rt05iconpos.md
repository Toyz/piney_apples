---
number: 592
title: A town map's signs are gcmn's data: RT01ICONPOS-RT05ICONPOS carry from visit to visit until GCMN.PRG loads again
date: 2026-11-14
area: world, decomp, volumes, test
files: crates/piney-desktop/src/save.rs, crates/piney-world/src/map/town.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/area/map.rs, crates/piney-world/src/world.rs, crates/piney-world/examples/map_probe.rs, crates/piney-game/src/session.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/mirror_world.rs, crates/piney-game/src/session/tests/ending.rs, tools/test_map_rs.py, docs/engine/map.md, docs/engine/root-towns.md
started: 2026-11-14T05:10:39-07:00
took: 1h 11m
resolves: 591
---

# 592. A town map's signs are gcmn's data: RT01ICONPOS-RT05ICONPOS carry from visit to visit until GCMN.PRG loads again

[[591]] left one thing of the town maps open: each class's sign table is
GCMN.PRG's own data, which the game keeps from visit to visit, while the
port read it afresh at every `TownMap::open`. The code settles its
lifetime, and the port now holds the five tables for exactly that long.

## The tables

Five tables of six `ICONPOS` (0x24 bytes, 216 a table), one per town
class, in GCMN.PRG's data on every volume:

```text
table        INF gcmn    MUT gcmn    OUT gcmn    QUA gcmn
RT01ICONPOS  0x005d4ad0  0x00602db0  0x006007e0  0x004f7b60
RT02ICONPOS  0x005d4e90  0x00603170  0x00600ba0  0x004f7f20
RT03ICONPOS  0x005d52c0  0x006035a0  0x00600fd0  0x004f8350
RT04ICONPOS  0x005d5570  0x00603850  0x00601260  0x004f85e0
RT05ICONPOS  0x005d57c0  0x00603aa0  0x00601490  0x004f8810

+0x00 x, y  +0x08 u, v  +0x10 tmpx, tmpy  +0x18 tmpx2, tmpy2  +0x20 alpha
```

Read through `tools/image.py` from each disc's GCMN.PRG, the 20 tables
are equal across the four volumes: every row has `tmp` and `tmp2` -1
and alpha 128, u 231 and v 120, 96, 48, 24, 72, 0, and each town's own
places.

`tools/disasm.py xrefs --scan` finds every reference in the class's own
`DrawMap` and nowhere else, main and gcmn, on all four volumes: INF 21,
25, 21, 25, 25 relocations; MUT 21, 25, 23, 25, 25 and OUT and QUA 32,
32, 30, 32, 32 references without relocations, all inside `DrawMap`.
The constructors never touch them, and Quarantine's sixth town
(`ROOTTOWN06::DrawMap`, QUA gcmn 0x004ecdd0) has no table. `DrawMap`
reads x, y, u, v and writes the rest; in INF `ROOTTOWN01::DrawMap`:
`tmp` at 0x0042316c, 0x004231c8 (right of the middle) and 0x00423288,
0x004232e4 (mirrored), `tmp2` at 0x0042347c, 0x004234e8, 0x004235cc,
0x00423638, alpha up by 4 to 128 at 0x00423680 and 0x00423694, down
by 4 to 0 at 0x004238bc and 0x004238cc while a sign last drawn fades
out where it stood, and 0 at 0x004238d8 otherwise. The port's
`TownMap::draw` was already this draw (test_map_rs compares the table
after every frame).

## What loads GCMN.PRG

So a table lives from a load of GCMN.PRG to the next. Two things load
it. `ccSetupNewGame` (INF main 0x001687a0) at log in. And the event
instruction `overlay 3` (114, `ccEvOverlay` 0x001b0318, through
`ccLoadOverlay`): `tools/evscript.py dump` of all four volumes finds it
in nine scripts, the same nine on each, all on the desktop (`game_status`
2) before their movies: events 1, 4 (block 9), 31, 101, 116, 201, 219,
301 and 314. Each goes on on the desktop except event 314's block 2,
Quarantine's ending, whose `mode 5` and `scene` go into field 126
without a log in. The port's desktop host did nothing for `overlay`, and
its desktop-to-field-126 path skips `session::log_in`, so the story
maps' `move_tex` (gcmn's data too, [[473]]) was not set back there
either. A soft reset loads DEMO.PRG over the window, and the world comes
back only through log in.

## The port

`SaveState::town_signs` holds the five tables, a slot per class
(`piney_world::map::town::SignTable`, `MinimapDef::signs` in place of
the name string), each `Option<Signs>` (`[Iconpos; 6]`, the build's
DWARF-typed row) and None while the file's rows hold. As [[591]] did for
the pulses, no town map holds its signs: `TownMap::image` is the file's
table (the build's `town_icons`), and `TownMap::draw` draws in
`MapFrame::signs`, the world's live tables from `World::map_parts`,
filling its slot from the image at its first draw. Every mode hands the
state on, the soft reset carries it (`carry_memory`), and the new
`SaveState::load_gcmn` sets the signs back to None and `move_tex` to 0;
`session::log_in` and the desktop host's `load_overlay(Gcmn)` call it.
The port's local `IconPos` copy of the row is gone; the draw works on
`Iconpos` directly.

## Checks

- `dun_loireags_signs_carry_across_visits` (piney-game, Mutation): Kite
  walks south 300 frames until the two northern signs have faded to 0,
  goes to area 66 and back by Gate Out. The field carries `RT02ICONPOS`
  (alpha 0, 0, 128, 128, 128, 128, every sign placed), the town back
  holds it as left, the northern sign fades in 0, 4, .. 128, and log in
  sets every table back to None. Before, the town back read the file's
  rows: every sign at once at 128.
- `test_map_rs`'s new `test_revisits`: Mac Anu and Dun Loireag walked 40
  frames, the map opened again (the game's masks made anew by the
  harness, its table left in memory) and walked 60 frames from the far
  end; the tables and every packet equal on all four volumes. With the
  probe reading the file's rows at each open, it fails at the second
  open.
- `a_revisited_towns_signs_fade_out_where_they_stood` (piney-world):
  Mac Anu drawn at the north end, then opened again at the south end:
  the four northern signs draw at 128 and step to 124 (12 sign packets);
  after `load_gcmn`, they drop to 0 at once (4 packets).
- `event_314_plays_through_field_126` now starts with a scroll and
  Dun Loireag's signs left set: in field 126 both are the file's again
  (block 2's `overlay 3`). With the desktop host's `load_overlay` back to
  doing nothing it fails.
- `loading_gcmn_brings_back_its_data`, and `a_soft_reset_carries_memory`
  with the signs (piney-desktop).
- Harnesses test_map_rs, test_town02_rs, test_town03_rs, test_grunty_rs,
  test_merchcam_rs, test_race_rs, test_world_rs, test_area_rs,
  test_lights_rs, test_gate_out_rs, test_party_rs and test_evarea_rs on
  all four volumes: 48 of 48 pass, as the baseline.
- Golden shots against 6b5e337's of the same day: 4,550 files, none
  differ (no shot walks a town and comes back to it).
- Battle pins: 290 calls over eleven fights, none differ. The piney-game
  suite 346 passed, 0 failed, 152 ignored; the crate tests, `gen
  --check`, clippy, fmt, `cairns check` and `docs.py check` pass.

**Still unknown:** nothing. The world's event host still only logs
`overlay`, which no script runs outside the desktop.
