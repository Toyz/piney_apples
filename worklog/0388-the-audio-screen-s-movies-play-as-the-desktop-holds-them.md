---
number: 388
title: The Audio screen's movies play as the desktop holds them: Movie 13's gate-in swirl draws, and Skeith's drain shows Kite and its backdrop
date: 2026-10-07
area: video, render, test
files: crates/piney-game/src/stream.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/world.rs, crates/piney-stream/src/event.rs, crates/piney-stream/examples/stream_shot.rs, crates/piney-stream/tests/stream.rs, crates/piney-stream/tests/stream_fixture.txt, tools/test_stream_rs.py, docs/engine/desktop.md, docs/engine/stream.md, docs/engine/field-ui.md, UNKNOWNS.md
resolves: 52
---

# 388. The Audio screen's movies play as the desktop holds them: Movie 13's gate-in swirl draws, and Skeith's drain shows Kite and its backdrop

Two issues by ThreePendant, both about the desktop's Audio screen,
Movie Mode (`Audio_control::SimplePlayStream`, desktop.prg 0x00407100):

- **#53.** In a movie, Kite's gate-in swirl does not show, though its
  sound plays. The reporter's frames are Movie 13 ("Balmung and ...",
  stream 16, `str0610`). The good frame is the story's step 20, under
  the swirl. The bad one is step 60 or so, after it.
- **#54.** "Data Drain" Skeith's movie shows Skeith alone, on black. That
  is Movie 16 ("Phase 1 Skeith / Epitaph in Stone", stream 18, `str9102`:
  Kite drains Skeith).

## What the game does

`SimplePlayStream` sets the background colour to 0 and starts
`ccThExecuteStream` (33) with the number. The stream then plays as it
does anywhere else:

- **Effects.** `RequestStrPlay` (0x00198fc0) starts `ccThEffectStr` and
  the second `ccThParticle` for every stream. `ccEffectCtrl(1)` takes its
  objects from `PARTICLE.CCS`, which `gcmnFileList` loads on the desktop
  too. So `str0610`'s cue 500 (`effTransferStr`, step 2) draws its rings,
  then the swirl's particles.
- **Resident files.** `ccSetFileListDesktop` (0x00169a20) adds, after
  `ccAddRequestFileListDesktop`, `strcmnFileList` again (skipped as
  listed), `datadrainFileList` (`STR8001E.CCS`) and
  `happyakuyujunFileList` (`STR8800E.CCS`). With `STR8000E.CCS` from
  before, that is every stream common file. `str9102`'s `#` objects
  resolve against them: Kite's body and the backdrop are `str8000e`'s,
  the bracelet's rings `str8001e`'s. Its own `str9102e` holds Skeith.
- **No banner.** `game.status` is 2 on the desktop, so stream 20's
  `Func_str9000` makes none.

The other places' lists differ. A town holds `STR8000E` and `STR8800E`,
a field or dungeon `STR8000E` and `STR8001E`, the title only `STR8000E`.

## Cause

The port played the Audio screen's movies through `StreamPlayer::start`,
the title's path. It gives a stream neither the effect system nor any
resident file:

- #53: the transfer came out as a request only, and nothing drew it.
- #54: stream 18's 177 `#` objects resolved nowhere, so only
  `str9102e`'s Skeith drew.

In the story both play correctly. Stream 16 is the event `stream`
instruction, with effects. Stream 18 is `DataDrainMenu`'s movie, over a
field's two files.

A survey of every Infection stream found the same gap elsewhere. A Ryu
Book's cover (streams 112-119, `str8801`-`str8808`) played in a town
over a field's files. Its backdrop text, sparks and lightning are
`str8800e`'s, which the town holds, so none of them drew.

## Fix

- **`piney_game::stream::FileList`** (`Town`, `Field`, `Desktop`): each
  place's stream common files, oldest first, as its `ccSetFileList*`
  loads them.
- **`StreamPlayer::movie`**: the Audio screen's movie, over
  `FileList::Desktop`, with the stream effects. `drain` (the menus'
  movies) and `event` (the scripts' `stream`) now take the place's list
  too. The town's book cover uses `Town`, the field's drains `Field`, and
  the desktop's, field's and town's scripts their own. `drain` and
  `movie` share one `over`.
- **`EventStream::over`** builds the subtitles on a stream the caller
  made (`Stream::with_resident`).
- **`stream_shot --resident STEMS`** renders a stream over given
  `DATA.BIN` files.

Shots in `/mnt/data/claude/scratch/i5354`: before, `s16_f60_nofx.png`
and `s16_f20_nofx.png` (no swirl); after, `movie13-0020.png` (the swirl,
as the reporter's console frame) and `movie16-0100.png` to `-0300.png`
(Kite, the backdrop and the rings around Skeith). The book cover is
`book112_before.png` and `book112_after.png`.

## Checked

- **`desktop::tests::audio_movie_draws_the_gate_in`**: Movie 13 from the
  desktop, through the menus. The stream effects draw from step 2 on,
  every step to 60. Before the fix: nothing on any step.
- **`desktop::tests::audio_movie_draws_the_drain_s_resident_models`**:
  Movie 16 draws models of `str8000e` and `str8001e`, and none but those
  and `str9102e`. Before the fix it drew `str9102e`'s alone.
- **`stream::tests::every_hash_object_resolves_where_its_stream_plays`**:
  every `#` object of every stream a place plays is defined there. That
  covers the Audio screen's 18 Infection movies over the desktop's list,
  streams 18, 20 and 109-111 over a field's, and 112-119 over a town's.
  Stream 18 over a town's and 112 over a field's do not resolve.
- **`tools/test_stream_rs.py fixture`** now runs stream 18's scene in
  eemu after `str8000e`, `str8001e` and `str8800e` (read from `DATA.BIN`)
  and `str9102e`. `CompleteIndexChunkAdrs` matches across them as on the
  desktop. `scenes_play_as_the_game_plays_them` builds the port's scene
  over the same files: the draw list (512 nodes) and all 350 frames match
  bit for bit, up to 273 objects drawn a frame.
  - The camera there exposed an old gap. At steps 33 and 34 the view's
    `world_screen` was 3.3e-4 of a column off, three times the old bound.
  - Cause: libvu0's `_sceVu0ecossin` makes sine as `sqrt(1 - cos^2)`,
    about 5e-5 off near 0, and the port's `Camera::world_view` uses
    glam's sine. `Scene::world_view_bits`, the game's arithmetic, is
    within 5.4e-7 on every frame, and the test now checks it at 1e-5.
- The fixture's other lines are unchanged.
- `audio_movie_plays_as_a_stream` passes (now on the shared helper).
- piney-game's suite (four threads) and piney-stream's tests, clippy on
  both, fmt, `cairns check` and `tools/docs.py check` pass.

**Still unknown:** the console's own Movie 13 and Movie 16 have not been
seen from the desktop. The reporter's good frame is from the story, and
the game's code says the desktop plays them the same way. A PS2 or PCSX2
capture of the Audio screen would confirm it. The port's draw still turns
the stream camera with glam's sine (`Camera::world_view`), not libvu0's.
It is off by up to 5e-5 radians near a zero turn, under a hundredth of a
pixel. Moving the draw to `world_view_bits` touches every stream's and the
desktop's view.
