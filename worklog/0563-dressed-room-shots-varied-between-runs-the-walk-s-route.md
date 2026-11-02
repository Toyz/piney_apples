---
number: 563
title: dressed_room_shots varied between runs: the walk's route broke ties in a HashMap's order
date: 2026-11-02
area: world
files: crates/piney-game/src/session/tests/shrine.rs
resolves: 562
started: 2026-11-02T11:24:04-07:00
took: 4m
---

# 563. dressed_room_shots varied between runs: the walk's route broke ties in a HashMap's order

Worklog 562's open question: `dressing::dressed_room_shots` gave one of
two sets of pictures on the same code (63 of 219 different, all in area
`t0-0-300-26`, Mac Anu's lake walk).

## The cause

Not the game. The shots' walk (`session::tests::shrine::walk_to`) plans
its way with two breadth-first searches, `detour` (a straight-line grid
round the room's walls) and `route` (the room's path-finding links). Each
ends at the reached cell nearest its goal, `back.keys().min_by_key(..)`
over a `HashMap`: with two cells at the same distance, the first in the
map's order won, and a `HashMap`'s order is seeded afresh in every
process. On that lake floor two cells tie, so the walk took one of two
ways, met its foes otherwise, and the pictures followed.

## The fix

Both searches break a tie by the cell itself (`min_by_key(|c| (dist(c),
*c))`), the smallest (x, y). The walk is the same in every run;
`dressed_room_shots` and `shrine_map_shots` (the other shot set on these
walks) were recorded again at worklog 562's commit, twice, each pair
byte for byte the same. `shrine_map_shots` is as before; of
`dressed_room_shots`, the 63 pictures of that area are the second of the
two sets seen.

## Checks

- `dressed_room_shots` and `shrine_map_shots` twice, every PNG the same.
- piney-game's suite (release), clippy, fmt, `cairns check`,
  `tools/docs.py check`.

**Still unknown:** nothing.
