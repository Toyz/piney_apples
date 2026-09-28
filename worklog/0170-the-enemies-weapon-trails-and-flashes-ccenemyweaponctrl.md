---
number: 170
title: "The enemies' weapon trails and flashes: ccEnemyWeaponCtrl"
date: 2026-09-26
area: battle, world, render, test
files: crates/piney-battle/src/weapon.rs, crates/piney-battle/src/prim.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-battle/src/lib.rs, crates/piney-battle/examples/weapon_probe.rs, crates/piney-battle/examples/battle_probe/enemy_motion.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-world/src/combat/weapon.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/weapon.rs, tools/test_enemy_weapon_rs.py, docs/engine/battle.md
---

# 170. The enemies' weapon trails and flashes: ccEnemyWeaponCtrl

This was the last combat visual not yet ported. Each enemy race's
constructor makes a weapon controller over its type's `ccEnemyWpInfo`
rows. The race's `exclusive()` runs it every frame and its `note()` hands
it every note. The motion layer already raised the two calls, but nothing
ran them. So no enemy's swing left a trail, and no hit flashed.
`battle.md` also said the controller drew from the generators, so fights
drifted without it.

## What the game does

- **The source.** gcmn 0x0043c600-0x0043ece8 holds `ccEnemyWeaponRad::ctrl`,
  `ccEnemyWeapon` (constructor, `init`, `setEdgeRate`, the cells,
  `setCell`, `interpolateCell`, `ctrlCell`, `dispCell`, `makePacketCell`,
  `ctrlRadiate`, `checkWeapon`) and `ccEnemyWeaponCtrl` (constructor,
  `ctrl`, `note`). Around it are `eneSplineH` and `ccPrimPacket`'s
  `makePacket` and `sendPacket`.
- **A weapon.** A weapon is up to four points (edges) on a node, with a
  pool of `life x (between + 1)` cells in a newest-to-oldest list:
  - While the enemy's attack is one the row lists (act 6 and bit
    `0x80 << atkNum`), each frame adds a cell at the node's matrix.
  - Bezier or spline rows then put `between` cells between the second and
    third newest.
  - Every cell fades by `0.4 / life` a frame, and its points' alphas
    compound with it.
- **The drawing.** `dispCell` sends the points on layer 6, edge pair by
  edge pair and newest cell first. There is a strip packet and a line
  packet (the lines 1.1 times as opaque). `makePacket` sets ADC where a
  run starts, where a point is behind the eye, and where both points are
  far off the screen.
- **The flash.** A hit (note 0x8005 in attacks 0, 1, 4, 5) flashes weapon
  `param` for 20 frames. A skill's start (0x8003 in attacks 2, 3) flashes
  every weapon for 80 frames, kept on the node. The flash is a
  `ccPrimRadiate` plate of 8 rays that faces the camera, which
  `ccPrimRadiate::create` turns by `cameraGetRot2` or `cameraGetRot`. It
  swells with `sin`, puts a light in the group, and goes out after
  `life + 2` frames.
- **No random draws.** Nothing in the controller calls `rand()` or
  `ccRand()`. The flash's plate has no `dpLength` or `dpBank`, so
  `createPlate` draws nothing either. The note in `battle.md` that
  fights drift without the controller was wrong for the weapons.
- **One quirk.** A one-edge row divides 0 by 0 in `setEdgeRate`. libgcc's
  soft float (built without NaNs) packs the result with an exponent
  `__unpack_d` never set, so the share is whatever the stack held. The
  harness first failed there, on the 24th table: the same row gave
  1.0019531 on a fresh stack and 0 after the other tables had run.
  Zeroing the stack before each build made them agree. One-edge rows
  draw no trail, so the value is never seen.
- **The tables.** After main's merge every race names its tables: 97
  tables, 225 rows. The V race's rows 271-273 name the animation's `EXT_`
  objects. G's sword, club and rod (types 1-4) are four-edge spline rows
  on every attack.
- **The one chunk.** 0x005db9c0's third row names `DMY_xdummy_w05`.
  `setCell` turns the node's matrix by the chunk's turn. It also copies
  `ApplyMatrix(m, chunk pos)` to what looks like a spare vector but is
  the matrix's last row, so the flash sits at the chunk's place. The
  harness found this: the one failing table after the merge, w 0 where
  the port had 1.

## What was built

- **`piney_battle::weapon`.** The rules: `WeaponCtrl::{new, ctrl, note}`,
  `Weapon` (the cells in an array, the list as indices) and `rad_ctrl`.
  They use the same FPU operation order as the game: the bezier through
  the accumulator, `eneSplineH`'s lanes, the doubles of `setEdgeRate`.
- **`WeaponWorld`.** The world the rules run in: the node's matrix,
  `ccTransPosFW2LW` and the camera's turn. It sits over a new
  `prim::RadWorld`, which the entry control's `Cx` also implements now.
  `Radiate::create` gained the camera-facing turn.
- **The two calls.** They now carry `WeaponAt`: `dispSW`, `actNum`,
  `actCnt`, `atkNum` and the skill's element.
- **`piney_world::combat::weapon`.** It builds a controller from the gcmn
  rows on `entry::Out::Weapon` and drops it on `Destroyed`. It runs the
  notes and frames in show order after the entry control. A node is the
  enemy's body posed at this frame's `Call::Draw` matrix. The trails go to
  `Combat::trails`, which `field_world::draw_trails` draws the way
  `sendPacket` does, with the lines one pixel wide. The flashes' rays join
  the boxes' rays.
- **The check.** `tools/test_enemy_weapon_rs.py` builds each table
  `races.rs` names and a made-up one with the game's constructor in eemu,
  then runs 360 frames of notes and `ctrl`. It compares every weapon, cell,
  flash and `makePacket` call against `weapon_probe`: 98 tables,
  1,188,443 `makePacket` calls, 20,882 flash frames and 6,036 notes, with
  0 mismatches.
  `GameBattle`'s no-op `sceVu0CopyVector` had to be taken off, and
  `strchr` run in Python.
- **The session.** `goblin_swing_draws_a_trail` plays event 3, then the
  east portal's goblin fighting Orca. Its sword swings once, and the trail
  is drawn for 18 frames, 21 pairs at its longest. `weapon_shots` saves
  the frames with a cut-out close-up; an orange (fire) arc shows by the
  goblin.

**Still unknown:** Whether `ccChar::Draw` leaves the node coordinates
stale on a frame it culls the body. The port uses the last drawn matrix
with this frame's pose. The flashes' lights are not put in the scene's
light group, like the boxes'. What the stream's decoded dummy chunk holds
at +0x10 and +0x20 (the port takes the file's place with w 1, and its
degrees as radians).
