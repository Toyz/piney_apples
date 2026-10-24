---
number: 446
title: The dungeon walk no longer puts Kite past what stops him: it finds a way by the room's path-finding cells, then by the walls themselves, and goes on to the doorway's next point when a pillar holds the first
date: 2026-10-24
area: test, world
files: crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/fairy_orb.rs, crates/piney-game/src/session/tests/macha.rs, crates/piney-game/src/session/tests/town06.rs, crates/piney-game/src/session/tests/party_leave.rs, crates/piney-game/src/session/tests/skeith.rs, crates/piney-game/src/session/tests/side_events.rs
took: 35m
---

# 446. The dungeon walk no longer puts Kite past what stops him: it finds a way by the room's path-finding cells, then by the walls themselves, and goes on to the doorway's next point when a pillar holds the first

The story pilot's dungeon walk (`Walker`, shrine.rs) moved Kite with
`pc_put` when four checks in a row found him stopped. It put him in the
doorway of the room's next leg. That is a position aid, and it is gone.
Kite now walks every step.

## What it does instead

When a check finds Kite stopped, the walk alternates two searches.

- **The room's cells.** This is `SetPathFindingMap`'s `buf`, the map the
  members' `PathFindingInDungeon` walks: per 300-unit cell, bits 1, 2, 4
  and 8 for a walkable neighbour at y - 1, x + 1, y + 1, x - 1. The walk
  searches it breadth first, within 48 cells, from Kite's cell to the
  goal's (or to the reached cell nearest it). It then follows the cell
  centres (`route`). The port keeps the map in `Combat::keep.path`,
  rebuilt on each room's entry as the game rebuilds it.
- **The walls themselves.** The map calls a cell open when a wall runs
  through it. In area 18's dungeon, room (2, 1) has a wall at x 16,235
  across a cell the map links on. So the second search (`detour`) uses
  points 150 apart within 4,500 of Kite or the goal. It goes from each
  point to its eight neighbours where the old waypoint test (three wall
  lines 50 apart, at 30 and 95 up) finds nothing between.

Where neither search finds a way, the old waypoint is used: a point of
the room that sees the goal.

## The doorway's first point

What the put had hidden: a door leg's first point, 900 inside the
doorway, often lies inside a pillar or a fence. Event 17's walk in area
18 met this in three rooms: (2, 1), (2, 5) and (3, 5). No walk reaches
such a point, so the walk sat beside it forever. When the wall search
cannot get within 150 of the point, the walk now takes the leg's next
point, 1,500 beyond the doorway, and searches for a way to that.

## Checks

The four suite tests that walked through such rooms now pass with no
put: `event_17_walks_kite_to_the_portal`,
`event_29_meg_and_the_administrator`, `the_rooms_have_their_breakables`
and `kite_stops_at_area_31_s_cure_box`. The piney-game suite passes.

Quarantine's survey (god, 40,000 frames each) ends the same events as
before: 301-306 and 309, with the others stopping where they did (worklog
441). Infection's, Mutation's and Outbreak's whole runs were not re-run.

`StoryPilot::after` existed only to make the put, and is gone with it.
`PINEY_DEBUG_WALK` prints each stopped check: the room, Kite, the goal,
his cell's links and the way found.

**Still unknown:** the other three discs' whole runs have not been
re-walked without the put. A room where neither search finds a way
would now stall the run instead of being skipped over.
