---
number: 478
title: Issue 7's foe side: Mad Grass under blows at the edge of a member's range, the game's motion against the port's
date: 2026-10-25
area: battle, test
files: tools/test_battle_enemy_motion_rs.py, docs/engine/battle.md, BUGS.md
resolves: 477.3
started: 2026-10-25T16:01:26-06:00
took: 5m
---

# 478. Issue 7's foe side: Mad Grass under blows at the edge of a member's range, the game's motion against the port's

[[477]] left #7 with one open point: whether the foes of the reporter's
fight moved about the members' territory as the port's did. The video's
foes are Mad Grass (INF `enemyTbl` row 219, race P, `EPG1`), and a plant
looks as if it should stand still, which would leave the edge loop
nothing to do.

`tools/test_battle_enemy_motion_rs.py` gained `mainedge`: `ccEnemy::main`
(gcmn 0x00432cd0) run natively over 300-600 frames for Mad Grass and up to
two other foes, Kite placed 60-200 from it, the members about 1500 or 1800
(BlackRose's and a Wavemaster's territory), all of them standing or
drifting slowly, and an affect landing on a foe in 15% of the frames
(mostly kind 1, a blow), against the port's `enemy_motion` through
battle_probe. 200 cases: every frame equal (the enemies' fields, their
body hits, the characters, the command lists, both generators, every call
in order). Two Mad Grass in one case are not run: the harness keys a row's
constructor template by the row, and two of one row mismatch on the
template, not on the rules (17 of 30 such cases failed on it; with
distinct rows 30 of 30 passed).

The game's own Mad Grass moves. In 60 of these runs its drawn position
moved a median of about 650 and at most about 4,900 from where it began,
under Kite's blows and its own acts; only 6 of 60 stayed put. So a
member halted at 1800 from it is pushed back across her territory again
and again, and the member side ([[477]]'s `RunFieldEdge`) turns each of
those into a line. Both halves of the video's chatter are the game's code,
compared frame by frame.

The other steps of the clock: the enemy motion harness's unit tests pass
(5, `mainedge` among them).

**Still unknown:** nothing
