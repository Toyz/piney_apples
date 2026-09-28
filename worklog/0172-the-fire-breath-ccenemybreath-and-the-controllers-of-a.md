---
number: 172
title: "The fire breath: ccEnemyBreath, and the controllers of a field's first enemies"
date: 2026-09-26
area: world
files: crates/piney-battle/src/breath.rs, crates/piney-battle/examples/breath_probe.rs, tools/test_enemy_breath_rs.py, crates/piney-world/src/combat/breath.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/weapon.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/fx/ambient.rs, crates/piney-game/src/session/tests/field_gims.rs
---

# 172. The fire breath: ccEnemyBreath, and the controllers of a field's first enemies

Worklogs 0168 and 0171 handed the dogs', wyrms' and dragons' breaths to
the world as calls, and the world did nothing with them. Now the breath
is ported (`piney_battle::breath`), checked against the game, and drawn.
`docs/engine/battle.md`, "The fire breath", has the rules.

**The breath** is a small particle system. It has 64 flames, each with a
`ccEff` from the enemy's own file. A breath attack starts a flame at the
head or neck node every frame (or every other frame) of its window. The
flames fan out by their share of the attack, grow, and fade in while
flying. A wall stops them, and the ground (15 sizes up) catches them.
Once fallen they sink, grow again and fade out. A landed or blocked flame
bursts on its 1st, 3rd and 5th frame after: `effSmoke`, with three
`ccRandF` scatters. Two helpers read the flames' turns: `ccSetMat2Rot`
(the Euler turn of the node's matrix) and `mov2rot` (the turn of a step).
For a kind 0 breath, `init` turns the node's own world matrix in place and
clears its translation. The port does the same to the matrix it is given.

**The check.** `tools/test_enemy_breath_rs.py` builds a breath with the
game's `initBreath`, then runs 400 frames of `ctrlBreath` and
`setBreath`. The rows are the game's four breath rows or made-up ones.
The enemy's attack frame counts through them, and its display and fade
change now and then. The node turns and moves every frame. The walls and
ground answer from the harness's dice. After each step the
`breath_probe` example's breath (every flame and its effect), the node's
matrix, the draws, the bursts and ccRand's state are compared. All 24
cases were equal on the first run: 12,717 flames drawn, 1,453 bursts.

**In the runtime** (piney-world `combat::breath`):

- The breaths run in the effects' pass over the frame's shows, as the
  weapon trails do. A flame starts at its node as this frame drew the
  enemy.
- The flames become ambient sprites on layer 3, from the enemy's file,
  with the palette set outright (`Sprite::clut` with an empty old name).
- The bursts are `effSmoke` puffs.
- The turtles' foot dust (`Call::FootDust`, worklog 0169) now reaches
  the effects too. The dust pass looks the foot up by name in the
  enemy's body as last drawn, and makes it a dust ring at the node's
  place, from the `eet1DustInfo` row and the enemy's `eneSmoke`.

**A bug on the way.** The field's first enemies never had their breaths:
`start_entries` pushed the constructors' outputs as shows, and
presentation took them before the first frame's passes saw them. The same
held for the weapon trails' controllers and the dust controllers of
every enemy a field or dungeon placed on entry (the portals' enemies,
made in the frame, were fine). `start_entries` now makes the three
controllers itself.

**The pictures.** `breath_shots` (ignored) searches the level 3-5 areas
of Δ in a shuffled order for one with a breather. It keeps Kite alive and
the breathers unkillable (Orca fells a Red Wyrm at a blow), then films
the fight. A Red Wyrm (row 199) breathed at frame 2569: its fire at the
head, then a red stream at Kite, and the bursts' smoke. 720 flames were
drawn, with 45 bursts. `a_higher_fields_other_races_move` and
`higher_field_shots` share a steering helper now. 615 workspace tests
pass.

**Still unknown:** Whether the flames look as the game's do was not
compared with a picture of the game. The sprite's turn (+0x28) stays 0,
and the flame's own turn is only used to step it. The runtime's bursts
draw `ccRand` after the entry control's frame, not inside it. The weapon
trails and dust of a field's first enemies were not looked at in a
picture after the fix.
