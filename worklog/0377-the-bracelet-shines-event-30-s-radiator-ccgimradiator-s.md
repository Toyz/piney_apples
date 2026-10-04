---
number: 377
title: The bracelet shines: event 30's radiator, ccGimRadiator's rays at Kite's hand, and the rays' lights in the group
date: 2026-10-04
area: render, script, battle, test
files: crates/piney-battle/src/gimetc.rs, crates/piney-battle/src/prim.rs, crates/piney-battle/src/world.rs, crates/piney-battle/src/evparty.rs, crates/piney-battle/src/party_ai.rs, crates/piney-battle/src/weapon.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-battle/examples/weapon_probe.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/cast.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/combat/town.rs, crates/piney-world/src/combat/weapon.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/session/tests/bracelet.rs, tools/test_battle_spawn_rs.py, tools/evscript.py, docs/engine/battle.md, docs/engine/events.md
---

# 377. The bracelet shines: event 30's radiator, ccGimRadiator's rays at Kite's hand, and the rays' lights in the group

Issue #40 (build f6e01cc): "The Bracelet doesn't shine at the start of
Chosen Hopeless Nothingness". The player's picture shows Kite and BlackRose
at night, Kite saying "The bracelet... it's shining...", and nothing on his
arm.

## What makes the shine

Event 30's block 18 runs in field 27 (town 1) at phase 4. After
`menu_ban`, the party's acts and puts and the camera, it runs `radiator 0
2 0 0 0 0 0 0` (case 45, main 0x001ac7c8). Then come 70 frames, BlackRose's
turns, and message 11. It is the only `radiator` on Infection's disc. The
port's area host logged it and did nothing.

Case 45 finds its character by `1 << type` (4 `GetSpc`, 0x18 `GetNpc`,
0x60 `GetEnemy`, else `ccCheckTargetTypeId`); here that is Kite. It then
calls `entryObject(ep, 1)` with:
- gimmick row 19 at (10x, 10y, 10z), turned (0, `DEG2RAD` roty,
  `DEG2RAD` rotz);
- `entRoot` 0, `param[2]` the rtype and `param[3]` the character.

`ccGimEtc`'s constructor, for row 19 with `param[2]` 0, makes `gimRad`
(+0x268) = `new ccGimRadiator(gimRadInfo, param[3])`: a `ccPrimRadiate`
plate of 32 rays. It then calls `setGimRadPos`. Each frame `main` runs
`ctrlGimRadiator`:
- **The hand.** `setGimRadPos(user)` again: the rays and the object at the
  translation of the user's `objHandR` (+0x130) `lwMatrix`, turned by its
  rotation (`ccSetMat2Rot`).
- **Letting go.** When the user's AI (+0x128) leaves manual mode, the rays
  are destroyed (`delLight`) and the object goes.
- **Else** `ccPrimRadiate::main`, with `ccGimRadiator::ctrl` (0x00456220).
  Its act 1 turns the plate 0.18 a frame about x. It pulses the alpha and
  an omni light with `sin(param[0])`, puts the light at the rays' place,
  and draws the rays 5 + 10 sin long from 4 out, 5 wide, banked 0.5,
  `dpLength` and `dpBank` 0.4 sin. The colours are
  `ccFractionalHsv(0x4080ffff, 0.5, 1)` inside, a cyan, fading to 0
  outside.

The full table is in battle.md ("The radiator"). In the scene, `menu_ban`
has put Kite in manual mode, and `menu_clear` at the block's end takes him
out. So the bracelet shines from the instruction, through message 11, to
the end of the block (245 frames in the test).

## The port

- **piney-battle.**
  - `prim::radiator_ctrl` and `GIM_RAD_INFO`.
  - `Radiate::set_light` / `del_light`; `box_ctrl` uses them now.
  - `prim::main` takes the class's ctrl; `box_main` is `main` with
    `box_ctrl`.
  - `gimetc`: `Etc::gim_rad`, `set_gim_rad_pos` and `ctrl_gim_radiator`.
    `param[3]` holds the user's scene index plus 1, 0 for none
    (`rad_user_param`).
  - `World` gains `hand_r` and `control_mode`.
  - `EvParty::radiator_user`.
- **piney-world.**
  - `FieldWorld::radiator` builds the entry and calls
    `Combat::entry_object_n(.., 1)`.
  - The stage answers `hand_r` from the actor's hand as its last draw
    left it. `Actor::keep_hand` runs for the party at the end of each
    combat frame; before any draw it answers the pose it stands in.
  - The stage answers `control_mode` from `Crew::manual_chars`, taken as
    the stage is made.
- **piney-game.** `AreaHost::gimmick` passes `Radiator` on.

## The rays' lights

The rays' `setLight` puts an omni light (`ccLight(4, 1)`) into `cc3d`'s
group every frame, and `delLight` takes it out. The port drew the rays of
the boxes, the idols and the enemies' weapon flashes, but never their
lights (UNKNOWNS [[170]]).
- **The group.** `Combat::rad_lights` (by object) and `Weapons::lights` (by
  enemy and weapon; `WeaponOut::Rad` now says which weapon) keep them.
  `cast_lights` puts them into the group with the springs'
  (`omni_light`).
- **The look.** The radiator's light is what turns Kite cyan. At full
  pulse it is a strong light on everything within 1,000.

## Checked

- **Against the game.** `tools/test_battle_spawn_rs.py`, two new parts:
  - `radiator_entry`: `entryObject(ep, 1)` of row 19 with `param[2]` 0
    and Kite as `param[3]`, his hand at a random turn and place, natively
    and through the probe's `entryn`. The constructor, the radiate's 32
    rays and light and `setGimRadPos` all compare.
  - `etc_frame`: four in ten scenes now hold a radiator. The hand moves
    every frame and his AI leaves manual mode at a random frame
    (`gframeh`). Every word compares each frame: the rays, the light, the
    `ccRand` the rays draw, the light's `AddGrp`/`DelGrp`, the deletion.
  - The probe and harness read the hand and manual flag with the scene.
    A radiate's parts are read by its `pnum`.
  - The whole file passes (6 tests, 150 cases each).
- **The session.** `the_bracelet_shines_in_chosen_hopeless_nothingness`
  (bracelet.rs): event 30 brought to field 27 with BlackRose along. The
  radiator draws 32 rays a frame for 245 frames, 76 of them under message
  11. Their light is in the group every frame they draw, and the first
  ray stays within 30 of Kite's right hand.
- **Shots.** `bracelet_shots` (ignored). Before (the radiator ignored),
  /mnt/data/claude/scratch/i40/before/bracelet-0250.png, as the player's
  picture. After, /mnt/data/claude/scratch/i40/after/bracelet-0250.png:
  the cyan rays at his right wrist, and Kite lit cyan.
  /mnt/data/claude/scratch/i40/before_after_0250.png shows both.
- **Weapons.** `tools/test_enemy_weapon_rs.py` passes.

**Still unknown:** how strong the radiator's light looks on the PS2 (the
light is the game's, through the port's VU1 lighting; a console capture of
this scene would show it).
