---
number: 105
title: Data Drain's movie: the stream by the target, its resident files, and the enemy drawn into it
date: 2026-09-25
area: ui, video, render, test
files: crates/piney-fieldui/src/menus/drain.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/world.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-battle/src/drain.rs, crates/piney-world/src/foe.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/combat/mod.rs, crates/piney-stream/src/lib.rs, crates/piney-stream/src/event.rs, crates/piney-stream/src/draw.rs, crates/piney-game/src/stream.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md, docs/engine/stream.md
---

# 105. Data Drain's movie: the stream by the target, its resident files, and the enemy drawn into it

[[102]] left the drain movie out: the port went on as with `drainDemo` off.
A new game has it on, and a boss's movie plays whatever the option says.
Skeith's drain at the end of the game is stream 18.

## What the game does

The details are on [the field UI page](../docs/engine/field-ui.md#the-drain-movie).
- **The stream.**
  - An enemy's is chosen by its size (`ccCheckObjectSize`, its row's
    +0x70): 109, 110 or 111. It plays only with `drainDemo` on.
  - Any other target's is chosen by `base->id`, Skeith's being 18.
  - The menu starts `ccThExecuteStream` and breathes without its `Disp`
    until the stream's param goes to -1.
  - A boss's `EntryAffect(21)` comes before the movie, not with the rules
    after it.
- **The models are not in the streams.** `str8100` drew nothing in the
  port: its `#` objects are defined in `DATA.BIN`'s `str8000e` and
  `str8001e`. A field's and a dungeon's file lists (`strcmnFileList`,
  `datadrainFileList`) keep those in memory.
- **`ccThDrainEnemy`.** It draws the target into the stream's scene, on
  its layer and through its draw environment: the enemy's clump on
  animation slot 6, at (0, -900, 130), turned half about z, from stream
  frame 30 to 229.

## The port

- **The menu** (`drain.rs`) runs `movie()`, `movie_wait()` and `rules()`:
  - `Request::Affect` (21) for a boss;
  - `Request::DrainMovie(num)`;
  - two frames later, `Request::DrainEnemy(handle)` for an enemy;
  - then it waits for `FieldUi::drain_movie_done`.
- **`CharInfo::object_size`** is new. The runtime fills it with
  `piney_battle::drain::check_object_size`.
- **`piney_stream::Stream::with_resident`** loads files the place holds.
  **`Stream::step_with`** (and `EventStream`'s and `StreamPlayer`'s) lets
  another task draw into the scene. `draw::to_screen` gives the scene's
  camera.
- **`StreamPlayer::drain`** plays the stream over the two resident files.
- **`piney_world::foe::DrainEnemy`** is the enemy task.
  `Model::draw_with` takes another environment's lights (the stream's).
- **`area.rs`** runs the movie, then `drain_movie_done`, and hides the
  minimap with the world. The town answers a movie at once, since no
  drain happens there.

## Checked

- **`tools/test_fieldui_rs.py test_drain_movie`.**
  - The stream is faked, ending 40 frames after it starts, and
    `ccCheckObjectSize` is answered per character.
  - It covers sizes 4, 3 and 1 with `drainDemo` on (109, 110, 111), and
    a boss with it off (its `EntryAffect(21)`, then 111 for id 14).
  - It compares the stream, the enemy task's start, the frames and the
    rest of the drain with the game's.
  - A boss with id 0 changed the harness's own target window, so the
    test keeps id 14.
- **`session::tests::data_drain_movie_in_a_fight`.** After event 3, with
  `drainDemo` on, the goblin's drain plays stream 109 over the field for
  more than 60 frames, and the enemy task is made. Then come the rules
  and the drop. `data_drain_in_a_fight` still plays none with it off.
- **`drain_movie_shots` (ignored).** It writes every 40th frame to
  /mnt/data/claude/scratch/drainmovie: Kite, the bracelet's petals, the
  beams, the orb, and the goblin small and far below Kite, as placed.
- **The rest.** All six field UI suites, the workspace's tests, clippy,
  fmt and the docs check pass.

**Still unknown:**
- **`Func_str8000`**, the drain streams' effect task (noise, fades, the
  feedback, a sprite), is not ported.
- **The enemy's look** is not compared with the game's pictures: its
  lights are the stream's, worked out at its matrix as for the scene's
  own models.
- **The race's base form** `ccThDrainEnemy` builds is for a state 2
  that no caller sets. What uses it is not known.
- **Streams 37, 44, 67, 69 and 100** (a character's drain by id) were
  not played.
