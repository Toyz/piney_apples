---
number: 562
title: The canals' sound by the town's type, not by name; golden shots of the story maps and fields
date: 2026-11-02
area: world
files: crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-audio/src/scene.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/map_shots.rs, docs/engine/sound.md
started: 2026-11-02T07:12:49-07:00
took: 1h 57m
---

# 562. The canals' sound by the town's type, not by name; golden shots of the story maps and fields

Section 2's next items start with what still named a town: the town mode
fed `ccSoundMain`'s scene sounds `mac_anu: town.def == &MAC_ANU`. Before
the story maps' and fields' steps, their golden shots.

## What the sound thread tests

`waterTest` (0x0017bac0) plays the canals while `WORLD_MAN +0x430` (a town
built) is set and `GetTownType()` (`game.town`) is 0. The sound driver
tests the same number twice more: `ccSndSQLoad`'s town case sets `+0x105`
to 1 for town type 0 (3 for the others, which clear `free[4]`), and
`ccSndBgmCtrl` sets it back to 1 and restarts the loop in town type 0. So
the input is the built town's type, not a flag:

- `piney_audio::scene::SceneInput::town`: `GetTownType()` while a town is
  built, None outside one (the area mode). The town mode hands over its
  def's `no`.
- `piney_audio::WATER_TOWN` (0): the one number the driver compares, in
  `waterTest`, `SqContext::pick` and `bgm_ctrl`, as the game compares it.

No code outside the sound driver names a town for it.

## Golden shots for the steps to come

`session::tests::map_shots`, recorded at 5ab9b22 and twice to be sure
they are deterministic (548 PNGs, all equal):

- `story_map_shots`: every field `GO(1)` makes a story map for (1-13, 15,
  16, 43, 66, 67, 91) on each volume, a new game put there and walked
  (frames 10, 60, 150, 280, 400 of play).
- `field_type_shots`: the first random field of Delta of each field type
  and background row (type 4 makes only dungeons), frames 30, 120, 250.

The checks' shot set also takes, from the same commit, `breath_shots`,
`field_gims_shots`, `dressed_room_shots`, `ocarina_lake_shots`,
`event_11_church_shots`, `event_11_warp_shots`, `kyvia_stage_shots`,
`skeith_shots` and `macha_shots`.

## Found on the way

`dressing::dressed_room_shots` is not deterministic: run three times on
the same game code (once at 5ab9b22, twice with this entry's audio
change, which draws nothing), one run's area `t0-0-300-26` (Mac Anu, the
walk to its lake's rooms) went otherwise from some frame on - Kite dead
in both, but standing elsewhere, the ENEMY sign moved - in 63 of its 219
pictures; the other two runs are byte for byte the same. The shots stay
in the set, read with that in mind until it is found.

## Checks

- Golden shots, every PNG byte for byte: the towns' set of worklog 560
  and 561, and the set above (`dressed_room_shots` on a second run, see
  above).
- Against the game, on all four volumes: `test_town02_rs`,
  `test_town03_rs`, `test_world_rs`, `test_lights_rs`, `test_gate_out_rs`,
  `test_party_rs`, `test_grunty_rs`, `test_merchcam_rs`, `test_race_rs`,
  `test_map_rs`, `test_evarea_rs`, `test_field_ambient_rs`,
  `test_field_rt`, `test_dungeon_rt`, `test_area_rs`.
- piney-audio's, piney-world's, piney-data's, piney-desktop's,
  piney-battle's, piney-effect's and piney-event's tests, piney-game's
  suite (release); `piney-gen gen --check`, clippy with all features,
  fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** what in the dungeon walk of `dressed_room_shots` differs between runs of the same code.
