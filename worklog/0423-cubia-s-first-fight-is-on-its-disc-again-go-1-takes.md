---
number: 423
title: Cubia's first fight is on its disc again: GO(1) takes eventAreaNumber from game.field, so area 47's last door makes EVENTAREAB8, and the hacked field's data bugs no longer fly over the fight (#72)
date: 2026-10-22
area: world, battle, test
files: crates/piney-world/src/field_world.rs, crates/piney-world/src/story_map.rs, crates/piney-data/src/area/mod.rs, crates/piney-game/src/session/tests/kyvia_stage.rs, docs/engine/evarea.md
started: 2026-10-22T15:05:58-06:00
took: 25m
---

# 423. Cubia's first fight is on its disc again: GO(1) takes eventAreaNumber from game.field, so area 47's last door makes EVENTAREAB8, and the hacked field's data bugs no longer fly over the fight (#72)

Issue #72 (ThreePendant, build 20db4ca): on Mutation, Cubia's (Kyvia's)
first fight played in Merciless Grieving Furnace's lava field, Kyvia's
body grey and far off in the sky, its gomoras on the lava, and a string
of garbled glyphs (`o e a 0 / ¥ = * Y :`) flying over the field. The
fight could still be won.

## The cause

Event 108's block 21 (`in_field town=2 field=9`, `entry type=7 code=12`)
fights on field 9, `EVENTAREAB8`, the disc. The party reaches it through
area 47's last door: `GotoNextRoom` answers -255 and `WORLD_MAN::Enter`
calls `ChangeArea(1, 9)` (10 from Outbreak on). The `WORLD_MAN` stays
area 47's. In the game that does not matter: `GO(1)` (MUT main
0x001b4740) sets `eventAreaNumber = game.field` at 0x001b546c, looks that
row up (127 rows; 71 and, from volume 3, 47 by their substitutes), and
reads its `model` at 0x001b5700. Field 9's row has model 1, so the switch
by number at 0x001b582c makes `EVENTAREAB8`.

The port's `FieldWorld::enter` handed `story_map::build` the model of the
`WORLD_MAN` it was given: area 47's, 0. So it generated area 47's field
(its seed, its lava type) and fought Kyvia there. The test helpers that
put a save on the disc (`event_108_disc`, `on_the_disc`) called
`ev_area_number` themselves, which is why `kyvia_fx`, `event_108_ends_with_kyvia`
and the autopilot's runs never saw it; Skeith's arena (fields 1-8) was
only right because `build` tested the arena's numbers before the model.

## The fix

`FieldWorld::enter` now takes the field's own number and row
(`WorldMan::with_event_number(game.field)`) for area 1 before anything
reads the model, as `GO(1)` does, and `story_map::build` follows the
game's order: model 0 a generated field, else the map by number.
`numbered_story_maps_have_model_1` (piney-data) checks that fields 1-13
have a row of model 1 on all four volumes, so the arenas need no test of
their own.

## The glyphs

They are the game's own: area 47 is a hacked area, and a hacked
generated field flies data bugs (`FIREFLY`'s hacked kind,
docs/engine/field.md): `EFF_sfzdigi0` with a trail of `field_eff`'s `TEX_sfzdigi1` glyphs (`e 0 1 ¥ > < w = a m / : {
* ; f`), one each sixth frame. Found by listing every texture the fight's
frames draw with, wrong stage against right: only the lava field draws
`field_eff`'s `TEX_sfzdigi0` and `TEX_sfzdigi1`. On the disc there are
none.

## The lag

Measured over 1,200 frames of the fight from the door (headless,
`kyvia_stage_shots`): a session step 1.39 ms in the lava field, 0.77 ms
on the disc; the picture (`piney_gs` render and read-back) 6.25 ms, then
4.51 ms. The generated field's terrain, objects and data bugs were the
extra.

## Checks

`session::tests::kyvia_stage` (new):

- `area_47s_door_leads_to_kyvias_disc`: from area 47's dungeon on area
  47's `WORLD_MAN`, the door's `ChangeArea(1, n)` to field 9 (Mutation's
  event 108) and field 10 (Outbreak's event 211): `eventAreaNumber` is
  the field, the place is `DiscArea`, and the event makes Kyvia. Before
  the fix `eventAreaNumber` stayed 47.
- `no_data_bugs_over_kyvias_disc`: 1,500 frames of the fight draw no
  `TEX_sfzdigi1` glyph (36,114 draws before the fix).
- `kyvia_stage_shots` (ignored) writes the fight and times it.

Shots: /mnt/data/claude/scratch/cubia/before/fight-0400.png (the lava
field, the camera in a rock by the dungeon's mouth) and
/mnt/data/claude/scratch/cubia/after/fight-0400.png (the disc, "Cubia
Core" targeted).

**Still unknown:** Quarantine's two later Cubia fights (events 308 and 312, bossFunc 14 and 15 on fields 11 and 12) reach the same GO(1), but ccBossKyvia03 and 04 are not ported (GAPS.md, Quarantine's own work).
