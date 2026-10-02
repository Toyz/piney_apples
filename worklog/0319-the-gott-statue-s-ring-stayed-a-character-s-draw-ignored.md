---
number: 319
title: "The Gott statue's ring stayed: a character's draw ignored its objects' animated transparency"
date: 2026-10-01
area: render
files: crates/piney-world/src/body.rs, crates/piney-world/src/foe.rs, crates/piney-game/src/session/tests/side_events.rs
---

# 319. The Gott statue's ring stayed: a character's draw ignored its objects' animated transparency

Issue #19 (INF, the tutorial dungeon, event 4 `TEACH-D`): after the Gott
statue falls, its yellow ring stays in the room. A player's video showed the
same thing on that day's build.

## Not the effects

First I ruled out the effects. Kite opened the statue in point 5's room
(block 7 run, the item window shut). From then on:
- the glow `effStatueOfGod`: its generators end at the frame `effsw` goes 0
  (`ccGimIdol::main`, gcmn 0x00459bfc). The census counts them now
  (`effsw_generators`);
- the particles: 218 just after the opening, the room's usual 4-5 within a
  few hundred frames;
- the box's rays (`ccPrimRadiate`): they grow, shrink and switch off.

## The model

The ring is part of the statue's own model: `OBJ_o_magic_m0_` in
`xgs?bod1.cmp` (`MAT_mag1`, `TEX_xgs?mag1`). The fall clip `ANM_xgs?dwn0`
(141 frames) and the open clip `ANM_xgs?nut1` key its transparency to 0. The
fall also fades both treasure objects, `OBJ_o_treasure_m0_` and `_m1_`, to
0: `tools/anim.py pose`, checked against the game's code by
`tools/test_anim.py`.

`ccAnm::Draw` (main 0x001524d0) multiplies each node's
`ccCoord::_GetTransparency` (main 0x00138490) into its models: the node's
own animated transparency times its parents'.

The enemies' model draw (`foe.rs`, `Model::node_alphas`) already did this.
`Body::draw`, which draws every character and gimmick actor (the statue
among them), gave each object the actor's one transparency. So the ring
and the lid stayed drawn after the fall.

`parents` and `node_alphas` now live on `Body`, and `Model`'s call them.
`Body::draw` multiplies each node's value in. Attached models (the blades)
keep their own.

## Test

`the_tutorial_statue_s_glow_ends` plays to the statue's room with the story
pilot and opens the statue as a player does. Then it checks two things:
- no `effsw`-switched generator lives after the switch drops;
- the room drawn 600 frames on has the ring's band free of yellow.

Without the fix it counts 4,478 yellow pixels there. With it, under 200. The
piney-world and piney-game suites pass.

**Still unknown:**
- Every character drawn through `Body` now fades with its clips' transparency
  keys. No other clip was checked by eye for a change this brings.
