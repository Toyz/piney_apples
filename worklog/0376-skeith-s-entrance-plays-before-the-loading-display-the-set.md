---
number: 376
title: Skeith's entrance plays before the loading display: the set-up waits on its phase-2 pass before the load
date: 2026-10-04
area: render, script, test
files: crates/piney-game/src/session.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/skeith.rs, docs/engine/field-walk.md, docs/engine/events.md, docs/engine/stream.md
---

# 376. Skeith's entrance plays before the loading display: the set-up waits on its phase-2 pass before the load

Issue #42 (build f6e01cc): "Skeith's intro cutscene renders slightly
incorrectly". The player's picture shows Skeith over white snow, with
stepped trails round him, a red streak, and a solid black band across the
lower third. The THE WORLD logo sits at the band's right end.

## What the shot is

- **The stream.** `stream_shot --stream 17 --fx` gives the same pictures
  at frames 790-840. Stream 17 is `str9101`, Skeith's entrance ("SKEITH /
  The TERROR of DEATH"). Event 30 plays it from block 21, at phase 2, with
  stream 14 just before it. The party is then entering field 1 from area
  27's dungeon.
- **The band.** The band and the logo are not the stream's. They are the
  loading display's animation, `xdl_load`'s `ANM_xdl_lod1`
  (field-walk.md "The loading display"). Coming from a dungeon it is the
  plain kind, the animation alone.

## The cause

`ccSetupGameCtrl` (main 0x00168960) makes its file list, then calls
`ccEnableThEvent(2)` at 0x00168f94. That call waits until the event task's
pass at phase 2 is done (0x001b5318: phase 3 or negative), and that
includes `ccEventStream`'s whole stream. Only after it comes
`ccLoadResourceFL` (0x00169340), whose `ccFileListLoad` calls
`ccLoadDispInit`. So no display exists while a phase-2 stream plays.
`ccLoadDispTh` (0x0019c290) itself never looks for a stream but 107: it
skips the animation while the gate hack's movie plays.

The port's session made the display in `world_stage`, before the area's
set-up, and stepped it every frame until `GameStart`. Over streams 14 and
17 (2,927 frames) it drew the band.

The docs had it wrong too: events.md said phase 2 runs "after" the mode's
files load. It runs after the file list is made, before the load.

## The fix

- **The session.** It latches `at_load` once the stage's set-up has made
  its passes. That is `Setup::passes_made`: `Off`, `Start` or `Play`;
  a set-up a pass abandons never loads. `AreaMode::setup` is new, beside
  `WorldMode`'s.
- **The display.** It is stepped and drawn only from then on, as is
  `--dvd`'s hold. The stage plays its passes before either.

## The rest of the picture

- **The trails are the game's.** `Func_str9001` (stream 17's task) takes
  cue 15 at step 230. Its last digit 5 is `SetReflex(1.0, 0, 0x48808080)`,
  the feedback at scale 1 and alpha 0x48, with a 60-frame fade ready. No
  x6 cue follows, so the last frame stays at 0x48/0x80 over every frame to
  the end. The packets are checked against the game's code
  (`effects.rs`, all 1,103 passes). The steps are whole frames of
  Skeith's fast motion.
- **The red streak** is a model of the stream, drawn by `stream_shot`
  too.
- **A shot pitfall.** A shot loop that renders only every tenth step
  shows one big ghost. The feedback then reads a picture ten steps old.
  `event_30_stream_shots` renders every step from 700.

## Checked

- `event_30_streams_play_before_the_loading_display` (skeith.rs): event 30
  entered from the dungeon (`event_30_arena_from(2)`), with 30 frames of
  disc held. Neither `str0570` nor `str9101` has a frame with an `xdl_load`
  model in it (2,927 frames before the fix). The band shows for the 30
  frames after them.
- `event_30_stream_shots` (ignored): before,
  /mnt/data/claude/scratch/i42/before/str9101-0830.png (the band, as the
  player's picture); after, /mnt/data/claude/scratch/i42/after/ (none).
- `a_warp_waits_for_the_disc_under_the_loading_display` still passes:
  a warp's passes take no frames, so the hold starts as before.

**Still unknown:** what the feedback's trails look like on a PS2 at this
moment (the packets are the game's; the pixels need a capture of the
console, as stream.md's Unknown says of the feedback).
