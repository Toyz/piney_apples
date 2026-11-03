---
number: 568
title: Golden shots of the fields and dungeons on every volume, every frame's picture and events as digests
date: 2026-11-02
area: test, world
files: crates/piney-game/src/session/tests/field_shots.rs, crates/piney-game/src/session/tests/dressing.rs, crates/piney-game/src/session.rs
started: 2026-11-02T18:17:47-07:00
took: 2h 22m
---

# 568. Golden shots of the fields and dungeons on every volume, every frame's picture and events as digests

Before the fields and dungeons become defs (`plans/rust-cleanup.md`,
section 2, the map's own systems, its step 3), golden shots of what that
step touches, beside `map_shots::field_type_shots` (Infection's field
types at three frames).

## The shots

`session::tests::field_shots` (`field_shots.rs`), two ignored tests over
every volume. A run is a new game on the disc logged in on a server, its
crisis byte set or not, put in the area of a word triple
(`WorldMan::set_generate_code`) or a story area's dungeon, the left stick
still, then ahead, then turning (`map_shots::shoot`'s), the tasks put to
sleep for twenty frames (the pictures drawn again as they stood). Every
frame is recorded: the rendered picture's FNV-1a digest, the frame's
events' (their `Debug` text) and their count, one line a frame in
`{name}.frames`; a few frames are PNGs to look at.

- `field_shots`: each field type (4 has none) and background row with a
  random area on server 0, plain and, for the types with a hacked
  background, in the crisis (the data bugs' skies, type 6's aurora): 300
  frames, 800 in a storm (its first flash comes at `DrawRain`'s 750th);
  and TOBJ on each server (the first area whose layout leaves `fieldrand`
  at 86 or more of 100, `Generate`'s roll). 468 runs, 2,320 files.
- `dungeon_shots`: each dungeon type, its first room and up to four rooms
  with sparks, water, walls, animated objects or statues
  (`dressing::rooms_with_pieces`, by `RoomSelect`), 150 frames each.
  Types 0-3 and 8 from the first random area making them on any server;
  the lakes also by night and in the crisis; types 4-7 and 9, which no
  random area makes (`SetDungeonTypeFromField`'s story rows), from the
  first story area whose dungeon 0 or 1 is of them (18, 20, 21, 16, 48 on
  every disc); and area 101's dungeon, its event room of type 32 (the
  tree and the leaves). 40 runs, 412 files.

`scratch/fdef/shots.sh` takes both into the shots the clean-ups compare.

## Twice the same

Both sets recorded twice at this change, each pair of files byte for byte
the same (2,732 files). The harness baseline for the step, at fb8a181:
all fifteen EE harnesses on the four volumes pass (60 of 60). The clock
ran over that baseline's two hours, in which the step's code was drafted.

**Still unknown:** nothing
