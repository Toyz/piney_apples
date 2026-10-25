---
number: 473
title: fieldrand is one stream and gcmn's scrolls outlive their maps: the story maps' clouds draw on from the scene before, WORLD_MAN::Init seeds it at log in, and EA_moveTex02 and 04-07 carry across visits
date: 2026-10-25
area: world, engine, test
files: crates/piney-desktop/src/save.rs, crates/piney-desktop/src/lib.rs, crates/piney-data/src/area/mod.rs, crates/piney-data/src/dungeon/random.rs, crates/piney-data/examples/area_probe.rs, crates/piney-world/src/story_map.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/town.rs, crates/piney-world/src/town02.rs, crates/piney-world/src/town03.rs, crates/piney-world/src/town04.rs, crates/piney-world/src/town05.rs, crates/piney-world/src/evarea.rs, crates/piney-world/src/evarea01.rs, crates/piney-world/src/evarea07.rs, crates/piney-world/src/evarea_b0.rs, crates/piney-world/src/evarea_b8.rs, crates/piney-world/src/evarea_sky.rs, crates/piney-fieldui/src/words.rs, crates/piney-fieldui/src/menus/gate.rs, crates/piney-fieldui/src/lib.rs, crates/piney-game/src/session.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/start.rs, crates/piney-game/src/session/tests/mirror_world.rs, tools/test_area_rs.py, docs/engine/evarea.md, docs/engine/town02.md, docs/engine/field-walk.md, UNKNOWNS.md
started: 2026-10-25T09:33:41-06:00
took: 24m
---

# 473. fieldrand is one stream and gcmn's scrolls outlive their maps: the story maps' clouds draw on from the scene before, WORLD_MAN::Init seeds it at log in, and EA_moveTex02 and 04-07 carry across visits

Worklog 472 left two things the sky maps did not model: the clouds'
random state at their first draw, and the sky scroll across visits. The
code decides both.

## The clouds' generator

`CLOUD::Init`, `SetPos` and `Move` draw from `fieldrand` (MUT main
0x001b0da0). It is not the game's `rand()` and not `ccRand`. It is a
third generator: `seed = (seed * 109 + 1021) mod 0xfffffffe`, global
`seed` (0x0038aca0, 13 in the executable) and `randcnt` (0x0038ba00).
Scanning every gp access to `seed` gives its writers:
- **`WORLD_MAN::Init`** (0x001b1720, in `ccSetupNewGame`): seed =
  `ccSys.count`. It draws c % 28, b % 28 and a % 29 into `word_c1`,
  `word_b1` and `word_a1`, then runs `SimGenerateCode`. It goes again
  (seed + 1 first) while the words make a story area on `game.server`,
  or while the field is a lake. Then `randcnt` = 0.
- **`SimGenerateCode`**: seed = the code, then 13 draws. It runs at the
  gate menu's looks at the words, `SetGenerateCode`, and instruction 118
  past 13.
- **`GO(1)`** for a generated field (0x001b5710): seed = `fieldSeed`. A
  story map gets nothing.
- **`GO(2)`**: a new dungeon gets `dungeonSeed[n]` (0x001b5c2c). A kept
  dungeon keeps the seed across its calls (0x001b5d40, 0x001b5d9c).

`randcnt` is read only by its own increments.

So it is one shared stream. The towns' clouds and glows draw from it,
and so do the fields, the dungeons and every story map. A story map's
first draws go on from the scene before. From the Chaos Gate, the Mirror
World's clouds start at area 66's code, moved on by 13.

## The port

**One stream.** `SaveState::field_rand` is the stream, from
`Rng::POWER_ON` (13, 0). Every mode takes it and hands it on:
- A town builds with it (`Town::open_with`; `ROOTTOWN05`'s constructor
  draws its glows from it). `World::state_out` hands back its class's.
- `FieldWorld::enter` hands it to the story maps through `At`.
  `EVENTAREA01`, `07`, `B0`, `B8` and the sky maps take it in their
  constructors (`StoryMap::field_rand`).
- A kept dungeon goes on from the game's seed.
- `FieldWorld::state_out` hands back the place's: field, dungeon, or a
  map that draws.

**The writers.**
- `session::log_in` is `ccSetupNewGame`. It runs
  `piney_data::area::world_man_init` with `ccSys.count` = the request's
  frame + 3: `ccBreathThread(2)`, and one frame waiting on GCMN.PRG.
- The gate menu's `generate` sends `Request::FieldSeed`, which the town
  or field applies. The fieldui `Generated` now carries the seed.
- The words' and `area n`'s scene changes apply `AreaCode::field_rand`
  (`area::sim_field_rand`). `area n`'s `scene` follows in the same frame
  and puts every task to sleep, so no draw comes between.

**gcmn's scrolls.** `EA_moveTex02` and `04`-`07` are `gcmn.prg`'s data:
seven rows (`evarea::scroll`) in `SaveState::move_tex`.
- `story_map::build` gives a new map its rows (`take_scrolls`).
  `FieldWorld::state_out` writes them back (`put_scrolls`).
- Area 15's kept map keeps its own through the church's door.
- `log_in` (GCMN.PRG loaded again) sets them to 0.

So a scroll lasts from log in, through every scene, until a game over
or the desktop.

## Checks

- `tools/test_area_rs.py` `test_world_man_init`: `WORLD_MAN::Init` in
  eemu against `world_man_init`, 300 counts across the five servers,
  `randcnt` 5 before. 0 mismatches on all four volumes.
- `area::tests::world_man_init_picks_a_plain_field`: the first groups are
  29, 28 and 28 words on every volume.
- `the_mirror_worlds_clouds_draw_on_from_the_gates_words` (session):
  Dun Loireag, the gate's words for area 66. The map's `fieldrand` before
  its first stepped draw is the code's seed.
- `the_mirror_worlds_scroll_carries_across_visits`: field 66, Gate Out,
  back. The new map's `EA_moveTex04` is the scroll the last one left; the
  town carries it. Log in sets it to 0.
- `the_holy_grounds_scroll_carries_across_visits` (Infection, area 15):
  the same for `EA_moveTex02`.
- The three session tests fail with the map's stream back at 0 and the
  scrolls not taken.
- The piney-game suite (330 passed), the piney-world, piney-data,
  piney-fieldui and piney-desktop tests, `test_town02_rs`,
  `test_town03_rs`, clippy and fmt pass.
- `test_evarea_rs`'s `test_frames` errors on a key, `arms`, that the
  game harness reports and `evarea_probe` does not. This change touches
  neither side.

**Still unknown:**
- How long GCMN.PRG's load holds `ccSetupNewGame` on a console: the
  port's load takes one frame of the wait, so `ccSys.count` at
  `WORLD_MAN::Init` is the request's frame + 3. On the disc it is that
  plus the read.
- The towns' own scroll statics across visits (town02.md, [[97]]): not
  gcmn's `EA_moveTex` rows, left as they were.
- `test_evarea_rs`'s missing `arms` key in `evarea_probe`'s frames, which
  this change did not touch.
