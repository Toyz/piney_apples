---
number: 378
title: A rare weapon's aura trails in the Root Towns: ccPlayer::Main's and ccFellow::Main's ArmsEffect run there too
date: 2026-10-04
area: render, world, test
files: crates/piney-world/src/arms.rs, crates/piney-world/src/player.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/town_party.rs, crates/piney-world/src/combat/town.rs, crates/piney-world/src/field_world.rs, crates/piney-world/examples/world_probe.rs, crates/piney-game/src/session/tests/weapon.rs, tools/test_world_rs.py, docs/engine/battle.md, docs/engine/field-game.md
---

# 378. A rare weapon's aura trails in the Root Towns: ccPlayer::Main's and ccFellow::Main's ArmsEffect run there too

Issue #47 (ThreePendant): "Rare Weapons don't have their special color
coming off of them in towns." The player's picture is Mac Anu by day.
Kite, seen from behind, holds a red blade, with Gardenia in the party.
Nothing coloured comes off the blade.

## What the colour is

It is the aura of [[197]]. `ccEquipmentParam` +8 (`auraSW`) is -1 for a
plain weapon and 0-7 for the rare ones (Crimson Raid 4, red; Moon Knives
7; Golden Yasha 5; and so on in every job's table). `ccSpcChar::ArmsEffect`
(0x0059ddd0) lays a lattice row from the weapon's dummies every frame the
weapon has an aura, swinging or not. The rows are in the aura's colour
outside a swing. `ccLattice::Disp` sends the ribbon once 16 rows are made.
So a rare weapon trails its colour wherever its holder walks.

## Why the towns had none

The game calls `ArmsEffect` from only two places (xrefs):
- `ccPlayer::Main` at 0x00598a00;
- `ccFellow::Main` at 0x0041bafc.

Both are the same code in a town and in a field. After `ccChar::Draw`,
while `dispSW` is on, Main calls `ArmsEffect` when drawn, alive (or a
ghost at `dead` 4) and in none of acts 12-14 (and 24 for Kite). Otherwise
it calls `ClearArmsEffect`. `ArmsEffect` reads nothing of the area, and
the lattice goes on the effect layer, which the town draws.

The port ran the calls only in the field's combat frame
(`Combat::arms_shown`):
- **Kite in a town** is the town's own `Player`. `Player::main` stopped
  at the draw test and never decided the call. Nothing laid his trails.
- **The members in a town** run `ccFellow::Main` through
  `Combat::town_frame`. Their frames did push `ArmsEffect`, but nothing
  ran it, and the town never drew a strip.

The walking PCs (`ccRtownPC`, a `ccGimmick`) and the merchants are not
`ccSpcChar`s. The game gives them no trails, so they were not affected.
[[377]]'s ray lights do not interact: the lattice is unlit and adds no
light.

## The port

- `arms::ArmsCall` (Effect or Clear) is what a frame asks.
  `Arms::send` puts the frame's strips on the effect layer
  (`trail_packet`, now shared with the field).
- `Player::arms`: `ccPlayer::Main`'s tail, None with `dispSW` off.
- `World` keeps `kite_arms`. `EquipWeapon`'s dummies come from his
  `Kite` model at entry and again on `change_equip`.
  `World::kite_arms_frame` lays the rows from his hands as posed now,
  with the save's weapon's aura and `trajectorySW` off (no swings in a
  town). The town sends them after his draw, and nothing while the tasks
  sleep.
- `Combat::town_frame` runs `arms_shown` over the members' calls, and
  `TownParty::send_trails` sends their strips. `TownParty::change_weapon`
  re-reads the dummies, as the field's `change_equip` does.

## Checked

- **Against the game.** `tools/test_world_rs.py`'s frame runs now also
  compare which of `ArmsEffect` and `ClearArmsEffect` the game's
  `ccPlayer::Main` calls each frame (hooked by return address, so
  `AnimCtrl`'s clears do not count) with the probe's `Player::arms`. The
  walk run sees both. `WorldAgainstGame` passes, all 7 tests. Turning act
  13 to Effect in the port makes it fail at the arrival.
- **The session** (weapon.rs):
  - `a_rare_weapons_aura_trails_in_town_and_field`: a new game's Kite
    with Crimson Raid stands 60 frames, then walks 90, in story area 14's
    field and in Mac Anu. The red ribbon is drawn on 63 of the walking
    frames in both. The arrival (act 13, cleared) runs 12 frames into the
    walk, and the ribbon shows 16 rows later. Before the fix the town drew
    it on none.
  - `a_members_rare_weapon_trails_in_town`: Orca, invited in Mac Anu with
    Jinsaran (his job's first aura 4), draws the red ribbon on 82 of 90
    frames as he follows. Without `arms_shown` in the town frame he draws
    it on none.
  - The suite passes (240).
- **Shots.** `rare_weapon_aura_shots` (ignored).
  - Before: /mnt/data/claude/scratch/aura/before/aura-town-45.png
    (-near.png enlarged), no colour on the blades.
  - After: /mnt/data/claude/scratch/aura/after/aura-town-45.png and
    -near.png, the red ribbon on the blade.
  - The field, the same before and after:
    /mnt/data/claude/scratch/aura/after/aura-field-45.png.

**Still unknown:** how the town's aura looks on the PS2 next to the port's.
The calls and the lattice are checked against the game's code, but the
picture was not compared. Comparing it would take a console capture of
Kite walking in Mac Anu with a rare weapon. The hands' matrices are still
the port's pose, as in [[197]].
