---
number: 386
title: A portal's group is drawn from ccRand, which every scene's set-up seeds from the frames since power-on: the giant's portals no longer always give a Mystery Rock and a Mu Guardian
date: 2026-10-07
area: battle, world, ui, test
files: crates/piney-battle/src/rand.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-desktop/src/save.rs, crates/piney-desktop/src/staffroll.rs, crates/piney-desktop/src/lib.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/field_npcs.rs, crates/piney-world/src/mt.rs, crates/piney-fieldui/src/talk.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/menus/inu.rs, crates/piney-game/src/session.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/session/tests/portal_draws.rs, crates/piney-game/src/session/tests/dun_loireag.rs, crates/piney-game/src/session/tests/ending_save.rs, crates/piney-game/src/session/tests/spell_hold.rs, tools/test_battle_spawn_rs.py, docs/engine/battle.md, docs/engine/desktop.md, docs/engine/evarea.md, docs/engine/field-game.md, docs/engine/field-ui.md, UNKNOWNS.md, BUGS.md
resolves: 94, 142
---

# 386. A portal's group is drawn from ccRand, which every scene's set-up seeds from the frames since power-on: the giant's portals no longer always give a Mystery Rock and a Mu Guardian

A follow-up on issue #52 (ThreePendant): on the first floor of Hideous
Someone's Giant every fight was one Mystery Rock and one Mu Guardian,
where the reporter remembered more varied groups. The question was what
the game itself fixes per area.

## The area

Area 16, Hideous Someone's Giant, is `EVENTAREA07`, the post-ending side
event 62's map (entry 177). Its door leads to area 2, field 16: a
dungeon generated from the area's words (dungeon type 7). Floor 1 has 11
rooms and 7 portals (rooms 1, 2, 3, 5, 6, 8 and 9); floor 2 has 7 rooms
and 2. `ccRegisterDifficultyEnemy` registers rows 106 Mystery Rock, 56 Mu
Guardian and 10 Tetra Armor (rank `eventAreaInfo.enemy` 105), all with
`esize` 3.

## What the game fixes and what it draws

- **Fixed by the area.** Where the portals stand (`DUNGEON::SetMagicCircle`
  over the generator's `gimPos`, `fieldrand` from the dungeon seed) and the
  three registered rows, so the species and their levels.
- **Drawn at each opening.** `entryCircleObject` (gcmn 0x00430360) and
  `entryObject` (0x00430c90), all from `ccRand` (main 0x001d9a10):
  - a box 1 time in 8, its row `ccRand() & 1`;
  - else the first row among the three;
  - the count, `abs(ccRand() % 3) + 1`, a one made two on a coin;
  - each member after the first redrawn among the three.

  So one to three of them in any mix.
- **The seed.** `ccInitRand` (main 0x001d9900) runs in every
  `ccSetupGameCtrl` (0x00168c78): every town, field and dungeon room,
  after the fade out and the two held frames. It seeds with 4352, then
  draws `ccRand` and `ccRandS` in turn `ccSys+0x358` times (`ccSys.count`,
  the frames since power-on, DWARF). Its other three callers, the bosses'
  `RndTarget` (gcmn 0x004845f4, 0x004d5344, 0x004e6f34), sit behind a
  retry count set to 0 and never run. `ccRandS`'s `lastRnd` (main
  0x00378aec) starts at 0 and is never reset. Nothing in the desktop seeds
  `ccRand`.

On a console, then, the same walk to the same portal gives a different
group with each arrival, by its own clock.

## The port

It differed. `Combat::new` began `ccRand` from the bare seed
(`Genrand::default()`, as `ccInitRand` with a count of 0) in every scene,
and `lastRnd` at 0. Only the frames spent in the room moved it (the
portal's particles draw `ccRand`), so one path gave one group. The
probe confirmed the report: floor 1's first portal, entered 0, 1, 2, 3,
500 and 12,345 frames after power-on, gave rows 106 and 56 every time.

The same pattern had three more places:
- an event's walking PCs in a field drew from a copy of their own from
  4352 (`TownPcs::outside(.., Mt::default())`);
- NorainuMenu's `talkNum` drew newlib's sequence from 1, made anew with
  each visit ([[142]]);
- the staff roll's `ccRand` was a fresh boot's, and its `srand` took the
  desktop's own count, not the frames since power-on ([[94]]).

## Fix

- **`rand::init_rand(count, &mut last_rnd)`** (piney-battle) is
  `ccInitRand`: the seeded generator, `count` draws, and `lastRnd` moved
  on as many. `rand_s` moved there beside it.
- **`piney_world::Arrival { frames, faded }`.** The session hands each
  scene its `sys_frames` and whether the fade was drawn before it.
  `rand_count()` is the count the set-up's `ccInitRand` step will see,
  the fade's ten frames and the hold's two on. `FieldWorld::enter` seeds
  `combat.cc` from it before a story map's constructor draws.
  `World::init_rand` (was `set_rand_count`) does the same for the town.
- **`SaveState`** carries `rand_s` (`lastRnd`) and `cc` (`ccRand`, as
  the staff roll's `CcRand`) from mode to mode. `state_out` writes them,
  and `with_live_state` lends `cc` to the menus as it does `rand`.
- **One generator.** `FieldNpcs::with_cc` lends the field's `ccRand` to
  the event's walking PCs. `TalkState`'s `MenuCc` loads `ccRand` from
  `SaveState::cc` and stores it back (a harness's numbers stay `Fixed`).
- **The desktop.** The session hands it the frames each step
  (`Desktop::set_count`), so the roll's `srand` and the cursor blink read
  the frames since power-on. Its roll takes `SaveState::cc`.

## Checked

- **Against the game.** `tools/test_battle_spawn_rs.py`'s new `init_rand`
  runs the game's `ccInitRand` in eemu on a random `mt[]`, `mti`,
  `lastRnd` and counts up to 20,000, and compares `ccRand`'s state and
  `lastRnd` (150 cases). A count one off fails it. The whole harness
  passes, as do `tools/test_fieldui_rs.py` and
  `tools/test_fieldui_talk_rs.py` (`DogPages`).
- **`a_portals_group_moves_with_the_frames_since_power_on`.** The same
  walk to floor 1's first portal 0, 1, 2, 3, 500 and 12,345 frames after
  power-on gives Mu Guardian and Tetra Armor; Mu Guardian; Mystery Rock
  and Tetra Armor; Mystery Rock and two Tetra Armors; a box; Mystery Rock
  and Tetra Armor. With the old seeding every one is Mystery Rock and Mu
  Guardian, and the test fails.
- **`each_set_up_seeds_ccrand_from_the_frames_at_its_step`.** The count
  equals `sys_frames` in the step where the set-up's `ccInitRand` runs:
  the first arrival, a room's door, and Mac Anu. `lastRnd` is moved on by
  it, and `state_out().cc` draws on as `combat.cc` does.
- **`a_dogs_talk_line_draws_the_towns_ccrand`**: NorainuMenu's line is
  the town's next word (1 here; the old stand-in gave 3).
- **`the_staff_roll_draws_from_the_frames_and_the_last_scenes_ccrand`**,
  `the_walking_pcs_draw_the_fields_ccrand`, `cc_rand_is_the_saves`,
  `init_rand_draws_the_frames_since_power_on`.
- `a_foes_spell_holds_kite` checked Kite's run in the last of 60 frames
  only. With the Mimic moving otherwise he now reaches a wall by then, so
  the test takes the longest step in those frames.
- The suites of piney-game (257 passed, four threads), piney-battle,
  piney-world, piney-fieldui and piney-desktop, clippy, fmt and the docs
  check pass. The whole stories end, none stalled: Infection's event 31
  at frame 372,000, Mutation's event 116 at 422,100 and Outbreak's event
  219 at 354,900 (399,900, 497,100 and 414,000 at worklog 385).

**Still unknown:** the port's frame count can only be its own. On a
console `ccSys+0x358` at each set-up also counts the disc's load times,
so no group can be matched to a particular console run without that
run's frame counts (a PS2 capture of `ccSys+0x358` at the set-ups, or a
player's pad_log replayed through the launcher, which reproduces only
the port's own clock).
