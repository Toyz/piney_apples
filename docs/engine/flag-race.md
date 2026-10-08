---
title: The Flag Race - menu 88, the race's task and its flags
status: partial
volumes: MUT, OUT, QUA
covers: MUT gcmn.prg:0x0058a4f0 Flag Race (menu 88), 0x0058c310 its page, 0x0058c930 Rankings (menu 89), 0x0058ca90 its page, 0x005ff820 the race's start, 0x005ff680 its task (PG_RACE), 0x005ff790 the race runs in a town, 0x005fd120 the race's constructor, 0x005fd6b0 its destructor, 0x005fd820 the set-up, 0x005fdaf0 the intro, 0x005fded0 the race's loop, 0x005fdd10 the countdown, 0x005fe290 the finish, 0x005fe830 the rank, 0x005feab0 the quit and time over, 0x005fee10 the HUD, 0x005ff3d0 a time's split, 0x005ff880 the camera-relative place, 0x005ff520 the restart, 0x005fca40 a flag's entry, 0x005fcba0 the flag's constructor, 0x005fcb40 its entry function, 0x005fce60 the flag's main, 0x00774a90 the flags, 0x005283a0 ccPGuso::adultMain's race steps, 0x00531090 the ride is up, 0x005310b0 the ride put down, 0x006d80b0 the flags' markers, 0x006d80e0 Kite's start, 0x006d8130 his heading, 0x006d8330 the ride by kind, 0x006d83c0 by server, 0x006d84a0 0x006d8410 the intro's camera by kind, 0x006d8580 0x006d8530 by server, 0x006d8280 the countdown's height, 0x006d82c0 Kite's place after, 0x006d8140 0x006d8190 the Grunty's at the finish, 0x006d81e0 0x006d8230 the finish's camera, 0x006d82a0 the cups' height, 0x006d8310 0x006d8320 the rank's clips, 0x006d85d0 the timer's cells, 0x0061ce80 0x0061ce30 where the chosen one walks, 0x00682230 the prizes, 0x00682280 the other prizes, 0x006822d0 the wallpapers, 0x00660b90 the greeting, 0x00660cc0 the results; MUT SLUS_205.62:0x0017a860 the race's time entered, 0x001cb154 ccThEvent's wait, 0x0038bd40 the race's task, 0x0038bd44 the race
worklog: 395
---

# The Flag Race - menu 88, the race's task and its flags

From Mutation on, a town whose three pens hold grown Grunties races them.
Its breeder lists Talk, Flag Race and Rankings once the race's mail is
read (worklog 394, [field-ui.md](field-ui.md)); its Talk (`TalkMenu`,
MUT gcmn 0x0056d2a8) then says the race's line (`race::talk_va`, MUT
0x00660b40) once the menu counted three grown Grunties and mail 324 is
at 3 or more (worklog 399). Flag Race (menu 88) takes
100 GP, lets the player pick one of three Grunties and starts the race.
Kite rides it ([grunty-ride.md](grunty-ride.md)) through the town and
picks up three flags against the clock. The time is ranked against the
town's three best, and a new rank wins a prize. Rankings (menu 89) shows
the three best. The addresses below are Mutation's. Outbreak's and
Quarantine's code is the same, found by the generator (`piney-gen`
`race.rs`) from the task that names itself `PG_RACE`.

## Menu 88: Flag Race

`ccMenuCtrl` menu 88 (MUT gcmn 0x0058a4f0) is a script over the list's
`proccess`:

```
0      the breeder's greeting (record 0x00660b90) by cmndTarget's name
2-3    "It costs 100 GP" Yes/No; Yes and gold >= 100: gold -= 100
5-6    else "come back" / "no money" (texts 1, 2), back to the breeder
10-11  the three Grunties (page: exceptionDisp 1), cancel to the breeder
12-13  "Start?" Yes: EntryAffect(target, plw, 0)
20     the race started: 0x005ff820(row - 145) for the list's Grunty
21     while running: cancel sleeps all, "Paused" Continue/Quit (22-27)
28-32  the race over: the time ("MM:SS.hh"), a rank 1-3 blinks on the
       Rankings page (exceptionDisp 2)
40-44  the result's record by rank (8 for a first win), "prizes gone"
       after three; the prize item to Get Item (menu 29), the race's
       done flag (+0xa5) set, ccMenu.forbid 1
50-57  a first rank 1: the town's wallpaper (bookWallPaperAdd), and the
       grand slam's after rank 1 in every town
60     the last record, back to the breeder
```

The prize counts are `saveData +0x8462 + 3 (server - 1) + rank - 1`. A
count of 3 or more gives the "other" prize table (0x00682280) instead of
0x00682230. Each table row is 5 entries of `{i16 id, i8 category}`. The
wallpapers are `0x006822d0[server]` (0 for the grand slam), set in
`dtWallpaperList` (`saveData +0x2238`).

The page (0x0058c310) draws the three Grunties as Rankings' window: each
Grunty's name and its three stars (speed, acceleration, turning) in the
large fixed kanji (kt 3, `SetClmLarge(2, 0, 0, 8)`). A star is at x 21
for 3, 24 when the first is 0, else 28. Case 2 of `ExceptionDisp` is the
Rankings page with the new rank blinking (`count % 24` folded at 13,
alpha `a * b / 12`, colour 6).

## The race's task and object

0x005ff820(kind) starts the task 0x005ff680 (priority 63, stack 4096) when
`game.area` is 0 and no race runs. The task names itself `PG_RACE`, makes
the race (176 bytes, 0x005fd120) at `0x0038bd44` and runs its loop
0x005fded0. Then it waits while the ride is up (0x00531090), lifts
`ccMenu.forbid`, deletes the race and clears both globals. 0x005ff790 is
"a race runs in this town": `game.area == 0 && race != 0`.

The race's fields:

```
+0x04 ccMenu's menuFade      +0x08..+0x30 the clips (xp_text, xp_cup)
+0x34 the timer's mask (8)   +0x38..+0x40 flag icons TEX_xpflag02-04
+0x44..+0x4c split timers    +0x50 layer 200 (clips) +0x54 layer 100
+0x58..+0x60 the flags       +0x64..+0x70 the ride's handling
+0x74 kind (row - 145)       +0x7c server     +0x80 phase  +0x84 sub
+0x88 counter  +0x8c/+0x8e the countdown (4)  +0x90 time (frames)
+0x92..+0x96 splits          +0x98 the intro's step
+0x9a..+0x9d the time's digits  +0x9e flags taken  +0x9f..+0xa1 order
+0xa2 rank  +0xa3 state (1 over, 2 done)  +0xa4 running (pausable)
+0xa5 done (the menu's)  +0xa6 restart (never set)  +0xa7 end
```

The loop:

- **Set-up** (0x005fd820). `ccPgBgmInit` and a 30-frame fade out. Kite
  is put at his start (0x006d80e0, heading 0x006d8130 by server) and
  paused (`plw` bit 0). `ccPuccigusoStart(kind)` takes the town path of
  [grunty-ride.md](grunty-ride.md).
- **The intro** (0x005fdaf0) runs on +0x98: the camera at the kind's
  place (or the server's for kind 0), bgm 3. Steps 2 to 5 are the chosen
  Grunty's own (below). The intro ends at step 6, 61 frames after step 5,
  with a 30-frame fade from frame 30.
- **The countdown** (0x005fdd10): 3, 2, 1, GO every 30 frames (se 236 to
  239). At 0 the race runs: `+0xa4` 1, Kite unpaused, the map on, bgm 0.
  The count's clip is drawn 800 before the camera (0x005ff880).
- **Phase 1**: the GO clip for 30 frames, then phase 2 once the race is
  over (state 1: three flags, the time out, or Quit).
- **Phase 2**: the finish (three flags, 0x005fe290) or the quit and time
  over (0x005feab0). The finish puts the Grunty and the camera at the
  finish and enters the time. Rank 0 or 4 plays bgm 6 and `ANM_xp_out_a`,
  ranks 1-3 bgm 4 and the cup's clips for 191 frames, then bgm 5. The
  quit plays `ANM_xp_over_a` only for time over.
- **The HUD** (0x005fee10): while running, a flag taken stores its split.
  The third ends the race (state 1). Time stops at 18000 (10 minutes).
  The timer is 8 cells of `TEX_xp_tim_1` at 0x006d85d0. Each flag taken
  shows its icon and split.
- **The end**: a 30-frame fade out and `BgmStop`. `+0xa7` and `+0x98 = 7`
  send the ride home and the Grunty back to its pen. Kite is put at
  0x006d82c0 facing the merchant's dummy. The panel and map come back,
  the race's music flag (`ccSnd +0x13a`) is cleared and sq 1 fades in.

`main 0x0017a860` enters a time into the town's three records (`saveData
+0x8432`, 12 bytes a server). It returns the rank it took, 4 within 30
frames of the third, else 0.

## The flags (gimmick 45)

`ccSetChibiGuso` enters three flags (0x005fca40) when the town's growth
level is 4 and the three pens are full. They stand at the town's markers
(0x006d80b0). Each is a `ccEntryObj` with a 512-byte body
(0x005fcba0). Its main (0x005fce60), state +0x1d4:

- 0: Kite nearer than 200 takes it (se 240 at it, `effOpenBox`, the
  race's order).
- 1: it fades over 16 frames.
- 2: hidden.

It is drawn nearer than 8500 and not hidden, with its shadow.

## The chosen Grunty

`ccPGuso::adultMain` (MUT 0x005283a0) asks 0x005ff790 first. The pen
Grunty whose `row - 145` is the race's kind follows `+0x98`:

```
1  the camera looks 100 above it
2  HitDisable, off the lists, step 3, its local id the kind
3  after 6 frames anm 1, step 4
4  after 91 frames anm 2, step 5, its goal 0x0061ce80[kind]
   (0x0061ce30[server] for kind 0)
5  walks there 12 a frame, turning 256 at most, the camera on it
6  gone: nothing more that frame
7  HitEnable, listed, back on its dummy, anm 0
```

Other pen Grunties go on as before. Without a race, Mutation's
`dogAction2` turns the Grunty to Kite (affect 14, `act_process` 1) and
back (affect 0, 2). From Mutation on a Grunty is not pushed out of
others, and its main (0x00527f78) lands it each frame but in acts 3
and 7.

`ccThEvent` (main 0x001cb154) does nothing while a race runs in the town.

## The port

- `piney_world::race`: `Race` (the object, the task's loop as a
  resumable `Pc` state machine, one breath a frame), `Flag`,
  `enter_time`, `split`, `before_camera_at`. It talks to the menu's fade
  through `RaceHost`, and to the game through `RaceEvent`s (se, bgm,
  `effOpenBox`, the ride's notes and dust, panel, map, forbid).
- `piney_world::town_ride`: the riding Grunty's town path, a `RideWorld`
  over the town's ground.
- `piney_world::grunty`: `RaceLink` and `race_step`.
- `piney_fieldui::menus::flag_race`: menu 88 and its page. The Rankings
  blink is in `breeder.rs`.
- piney-game: `MenuFader` (the `RaceHost`), the events to sound and
  effects, the map's racers, the event task's wait.
- piney-gen `race.rs`: every table, found from the code.

## Checks

- `tools/test_race_rs.py` (Mutation, Outbreak, Quarantine): `split`,
  `enter_time`, the HUD's cells, the countdown, the camera-relative place
  and the flag's main against the game's code.
- `tools/test_fieldui_talk_rs.py`: Flag Race declined, paused, and its
  results by rank, against menu 88.
- `tools/test_grunty_rs.py` (Mutation): the raced Grunty's steps 1-7 and
  another kind's race against `adultMain`.
- `tools/test_ride_rs.py`: `ccPuccigusoStart` and `Exit`'s town path.
- piney-game `flag_race`: `mutations_flag_race_runs_and_quits`,
  `mutations_flag_race_won`, `mutations_flag_race_against_the_towns_racers`
  (a new save's time ranked against Dun Loireag's racers);
  `flag_race_shots` (ignored) takes the Grunties' page, the countdown, the
  ride, the result's clip and the rankings. Kite walks to the breeder and
  rides to each flag (the nearest first) as a player would: the way round
  the town's walls planned each second (`town_walk::TownWalker`, the
  Grunty's width either side), the stick toward its next turn. No test
  sets the ride or Kite down.

## Unknown

- The race's task is made in the frame the menu asks. The port runs its
  first breath that frame; the game's task priority (63) is not modelled.
- `+0xa6` (the restart, 0x005ff520) is never set by any code found.
