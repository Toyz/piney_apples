---
number: 395
title: Mutation's Flag Race runs: the breeder takes 100 GP, the chosen Grunty walks off, Kite rides it through Dun Loireag for three flags against the clock, and the time is ranked and its prize given
date: 2026-10-08
area: ui, world, audio, volumes, test
files: crates/piney-world/src/race.rs, crates/piney-world/src/town_ride.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/grunty.rs, crates/piney-world/src/merchant.rs, crates/piney-world/src/draw.rs, crates/piney-world/src/ee.rs, crates/piney-world/src/map/mod.rs, crates/piney-world/src/combat/ride.rs, crates/piney-world/examples/race_probe.rs, crates/piney-world/examples/grunty_probe.rs, crates/piney-battle/src/ride.rs, crates/piney-battle/examples/ride_probe.rs, crates/piney-fieldui/src/menus/flag_race.rs, crates/piney-fieldui/src/menus/breeder.rs, crates/piney-fieldui/src/menus/mod.rs, crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/src/render.rs, crates/piney-fieldui/src/spr.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/world.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-game/src/world.rs, crates/piney-game/src/town_fx.rs, crates/piney-game/src/session/tests/flag_race.rs, crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-audio/src/scene.rs, crates/piney-desktop/src/view.rs, crates/piney-gen/src/race.rs, crates/piney-gen/src/talk.rs, crates/piney-gen/src/data.rs, crates/piney-gen/src/lib.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/race.rs, crates/piney-data/src/tables/fieldui.rs, crates/piney-data/src/tables/mod.rs, crates/piney-data/src/pack.rs, tools/test_race_rs.py, tools/test_fieldui_talk_rs.py, tools/test_grunty_rs.py, tools/test_ride_rs.py, docs/engine/flag-race.md, docs/engine/grunty-ride.md, docs/engine/field-ui.md, docs/engine/town02.md, UNKNOWNS.md, BUGS.md, GAPS.md
resolves: 394
---

# 395. Mutation's Flag Race runs: the breeder takes 100 GP, the chosen Grunty walks off, Kite rides it through Dun Loireag for three flags against the clock, and the time is ranked and its prize given

Issue #56, second part. Entry 394 made Mutation's breeders offer Flag
Race and Rankings. Flag Race still closed at once: the race itself was
not ported. The page is [flag-race.md](../docs/engine/flag-race.md).

## What the game does

- **Menu 88** (MUT gcmn 0x0058a4f0) is a script of about 40 steps:
  - The greeting, then "100 GP?". The player picks one of three Grunties
    (page 0x0058c310: names and stars in kt 3), then "Start?".
  - It starts the race's task (0x005ff820) and waits. Cancel pauses
    (Continue or Quit).
  - At the end: the time, the rank blinking on the Rankings page, the
    result's record, the prize (Get Item, menu 29). A town's first win
    adds its wallpaper, and the grand slam after a first in every town.
- **The race** (task 0x005ff680 `PG_RACE`, object 0x005fd120 at main
  0x0038bd44):
  - Set-up: the ride's town path (`ccPuccigusoStart` in a town).
  - The intro: the camera on the chosen Grunty, which `adultMain` walks
    off by the race's step.
  - The countdown (se 236-239), the timer and three flags (gimmick 45,
    made by `ccSetChibiGuso` at level 4 with three pens).
  - The finish or quit, with the cup's or "out" clips. `main 0x0017a860`
    ranks the time.
  - The end puts Kite back by the merchant.
- `ccThEvent` (main 0x001cb154) waits while the race runs. The map draws
  the racers. `ccPgBgmInit` in a town sets the race's music flag, and
  `bgmBreed` then leaves the music alone.

## Cause

The port had no menu 88, no race and no town ride.

## Fix

- **`piney_world::race`** (new):
  - `Race`: the task's loop as a resumable state machine (`Pc`, one
    breath a frame): set-up, intro, countdown, clips, HUD, finish, quit,
    end.
  - `Flag`, `enter_time`, `split`, `before_camera_at`.
  - Outward it speaks `RaceEvent`s, and it fades through a `RaceHost`.
- **`piney_world::town_ride`** (new): `ccPuccigusoStart`, the task and
  `Exit` in a town, over a town `RideWorld`. Mutation's ride is in
  `piney_battle::ride`: height 125, the race's handling (`RaceRide`),
  `main_later`, `control_move_later`.
- **`grunty.rs`**:
  - `RaceLink` and `race_step`: `adultMain`'s steps 1-7.
  - Mutation's turns to Kite and back, its ground set in `main`, no push
    out of others.
  - Each note's position as the note fired (`NoteAt`).
  - **A port bug on every volume:** a Grunty's body had `mask2`
    0x40000001 and kind 2. The game leaves `mask2` at `ccCharHit`'s -1,
    which no wall matches. Its kind is 16 from Mutation on. The grunty
    harness's new race scenarios showed it: walking off, the port's
    Grunty touched a wall the game's does not.
- **`piney_fieldui::menus::flag_race`** (new): menu 88 and its page. The
  Rankings blink is `/ 12` folded at 13 (the page said `/ 6`). Settings
  carry their kanji type (kt 3, large fixed).
- **piney-game**:
  - `MenuFader` (the race's fades on `ccMenu.menuFade`).
  - The race's events to sound, effects (`effOpenBox`), panel, map and
    forbid.
  - The event task's wait while racing, the map's racers, and
    `RaceStart`, `RaceQuit` and `RaceDone` from the menu.
- **piney-audio**: `PgBgm::InitTown` and `PgBgm::RaceEnd`, and
  `race_music` in `bgm_breed`.
- **piney-gen**: every table found from the code (`race.rs`). The talk
  roots take the race's records. Data version 27.

## Checked

- **Against the game's code** (eemu):
  - `tools/test_race_rs.py` on Mutation, Outbreak and Quarantine: split,
    enter, hud, count, camera, flag.
  - `tools/test_fieldui_talk_rs.py`: `test_flag_race_declined`,
    `test_flag_race_paused`, `test_flag_race_results`. 41 tests pass on
    Mutation and Infection.
  - `tools/test_grunty_rs.py`, now volume-aware: passes on Mutation and
    Infection. Its new scenarios are "raced 145/146/147" and "not raced".
  - `tools/test_ride_rs.py`: Start and Exit with the town path, all four
    volumes.
- **Played** (piney-game `flag_race`):
  - `mutations_flag_race_runs_and_quits`: pay, pick, start, pause, quit.
  - `mutations_flag_race_won`: ride to the three flags, a rank 1, the
    wallpaper, the prize. The test steers straight at each flag. After
    150 frames stuck on a wall it puts the ride at the flag (`ride_put`,
    a test hook, not the game's).
- **Shots** (`flag_race_shots`, ignored), in
  `/mnt/data/claude/scratch/i56/`: `mut-race-grunties.png`,
  `mut-race-countdown.png`, `mut-race-riding.png`, `mut-race-cup.png`,
  `mut-race-rankings.png`.
- piney-game's suite (four threads), the touched crates' tests,
  `piney-gen gen --check`, clippy, fmt, `cairns check` and
  `tools/docs.py check` pass.

**Still unknown:** none of #56 is left open. These remain:
- Menus 90 and 91 (MUT gcmn 0x0058d470, 0x0058d9f0) are not ported, and
  who opens them is not traced.
- The race's task runs its first breath in the frame the menu asks. The
  game's priority 63 is not modelled.
- `+0xa6` (the restart, 0x005ff520) is set by no code found.
- A field ride's charge, Grunty chat and `+0x1e0` range from Mutation on
  (MUT gcmn 0x0052f790, 0x0052f250, 0x00530e50) are not ported. Porting
  them needs a later volume's field ride in the ride harness.
- The grunty harness fails on Outbreak and Quarantine before reaching a
  Grunty: `test_world_rs`'s `__vt__10ROOTTOWN01` is missing there.
