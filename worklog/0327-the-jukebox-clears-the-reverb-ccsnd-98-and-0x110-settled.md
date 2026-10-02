---
number: 327
title: "The jukebox clears the reverb; ccSnd +98 and +0x110 settled"
date: 2026-10-01
area: audio, decomp
files: crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-audio/tests/driver.rs, crates/piney-audio/tests/driver_fixture.txt, crates/piney-audio/tests/stream.rs, tools/sound_ee.py, docs/engine/sound.md
resolves: 56
---

# 327. The jukebox clears the reverb; ccSnd +98 and +0x110 settled

Four sound questions were left open: [[56]]'s two, [[184]]'s +0x110, and
[[42]]'s "who asks for `sdCommand` cases 4 and 5". All addresses are INF
SLUS_202.67.

## sdCommand 4 and 5: ccSndChangeData

`sdCommand` (0x001816f0) walks `ccSound.sdRemote[8]` (+0x1c, DWARF). A scan
of the 459 relocations to `ccSnd` in main and the four overlays finds four
writers:
- `ccSetMainVol`: `[0]`;
- `ccSetOutputMode`: `[2]`;
- `ccSndChangeData`: `[3] = 4` at 0x00183628, after the old music is
  stopped and before the load, which every path reaches, the desktop's
  first load (`old` -1) included;
- `ccSndChangeData`: `[4] = 5` at 0x00183888, once the data is in.

Case 4 sets the effect mode to 0x100 (off, area cleared) on both cores
(`sceSdRemote` 0x8130), the effect return volume to 0 (0x8010, params
0x0b80/0x0c80) and disables effects (0x8070, 2 and 3). Case 5 sets mode
0x105 (hall, area cleared), depth 0x7fff and delay 127, return volume
0x3fff, and enables effects.

The game's own run shows when they happen. `tools/sound_ee.py` now logs
`sceSdRemote(0x8130)` as `reverb CORE MODE`. Its `desktop` step now runs
the sound task in `ccSndChangeData`'s breaths, as the game's thread would.
Both 4 and 5 then run in the one `sdCommand` after the load:
`load X; reverb 0 100; reverb 1 100; reverb 0 105; reverb 1 105; seq ...`,
in all 51 desktop rows and the 10 jukebox changes. The volume scenarios no
longer carry a stray pair. The effect is that the reverb starts empty with
the new music: the old music's tail does not ring on.

The port's driver keeps `sd_reverb` (`sdRemote[3]` and `[4]`). `load` sets
it, and `task`, at `sdCommand`'s place, sends `Command::Reverb(false)` then
`Reverb(true)`. The engine turns the reverb off, then gives both cores a
fresh hall. The driver test (`driver_fixture.txt`, regenerated) holds the
order. It fails without the change, since every desktop and jukebox line
now has the four `reverb` entries.

## ccSnd +98

+98 is +0x62, which [the sound page](../docs/engine/sound.md) already
describes. `ccSndMoviePlayer` mode 0 sets it (0x00180c44) and mode 1 clears
it (0x00180e1c). `ccSndStreamCtrl` (0x0017d1a8) and `ccSndStreamSE`
(0x0017caac) do nothing while it is set. `piney-audio`'s stream code has
the same rule: the caller makes no stream sound calls while the Audio
screen's movie plays.

## The port-0 volume branch

Every `SQTBL` row on the four volumes plays on HD ports 1-3, so
`ccSqPlay`'s port-0 branch (the SE volume) is never taken by a sequence:

| table | rows per volume |
| --- | ---: |
| `sqVoltype*` (field) | 71 |
| `sqVolTblDungeon` | 11 |
| `sqVolTblEvent` | 123 |
| `sqVolTblTown` | 10 |

`Wave` uses these same tables. `ccPortVolSet` with port 0, which the event
sound command 8 can ask for, is modelled: `Driver::port_vol_set` gives
port 0 `se_vol`. The SE volume is 0-256, so the 24-bit sign extension never
shows.

## ccSnd +0x110

The DWARF names +0x108-+0x117 `int free[4]`, scratch for the stage's
scene-sound routine (`stageParam` +0x105). Over the 459 `ccSnd`
references, only three places touch the array:
- `bgmChurch` uses `free[0]` and `free[1]`;
- `bgmBreed` uses `free[0]` and `free[3]`;
- `ccSndSQLoad`'s loop clears all four (0x00182448).

`free[2]` (+0x110) is never read: the game leaves it unused. The scan
follows registers loaded from `ccSnd`. A pointer to the array handed to
another function would escape it, and none was seen.

**Still unknown:** whether the real sound thread runs `sdCommand` between
the 4 and the CD load's end, turning the reverb off for the load's length
(the harness loads at once). The port treats the load as instant.
