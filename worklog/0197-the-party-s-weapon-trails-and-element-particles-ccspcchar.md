---
number: 197
title: "The party's weapon trails and element particles: ccSpcChar::ArmsEffect on ccLattice"
date: 2026-09-26
area: battle
files: crates/piney-world/src/arms.rs, crates/piney-world/src/lattice.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/cast.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/field_world.rs, crates/piney-battle/src/skill.rs, crates/piney-battle/src/kite.rs, crates/piney-effect/src/lib.rs, crates/piney-effect/examples/boss_probe.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/weapon.rs, tools/test_boss_effect_rs.py, docs/engine/battle.md, docs/engine/boss.md, docs/engine/field-game.md
---

# 197. The party's weapon trails and element particles: ccSpcChar::ArmsEffect on ccLattice

0195 left one gap: the party's weapon trails. Kite's and the members'
frames already emitted four outputs, and nothing consumed them:

- `ArmsEffect` and `ClearArmsEffect`;
- `SetArmsEffectColor`;
- `StartArmsEffect` (Kite's as `ArmsParticles`).

So no one in the party left a ribbon behind a swing, and a fire art did
not light the blades.

## What the game does

battle.md, "The party's weapon trails", has the tables. In short:

- **`EquipWeapon` (0x0059d650)** reads the weapon file's
  `DMY_xdummy_w01`-`w04` besides its models. `Decode_DummyPos`
  (0x0014d4d0) gives each a place with w 1.0. It also makes the job's
  lattices: two `ccLattice(2, 0)` for the twin blades, one for most
  jobs, and a 3-column one for job 4.
- **`ArmsEffect` (0x0059ddd0)**, every drawn frame, runs `ClearCnt`
  first. It puts the weapon points `weaponEffPos` at the hands' matrices
  times the dummies. It lays a row only while swinging (`trajectorySW`)
  or when the weapon has an aura (`ccEquipmentParam` +8). The aura is
  the colour, outside a swing or on the normal attack. Then `Disp`.
- **`StartArmsEffect` (0x0059e530)** starts, for an art with an element,
  `particleEffectTbl[40 + element]` between two weapon points while
  `armsEffectSW` holds.

## The lattice

`piney_world::lattice` held 2 columns only. It now holds up to 3, as
job 4 needs. `ccLattice::Disp` then sends two strips in one packet
(columns 0-1, then 1-2), each starting with ADC. The z sum counts a
middle vertex once (MakePacket's made flags), and the key is the sum over
twice the pairs. The lattice also has:

- `ClearCnt` (0x00579c50) with its +0x72c flag;
- the colour type as the i32 `_SetArmsEffectColor` writes. `MakePacket`
  reads +4 each time, so a change shows at once.

`test_boss_effect_rs.py`'s `test_weapon_trails` drives the game's own
`ccLattice(2, 0)` and `(3, 0)` in eemu as `ArmsEffect` does:

- `ClearCnt` every frame;
- `ClearArmsEffect`'s flag and colour types set at random between
  frames.

Against the probe: 0 mismatches over 600 frames each, in every row,
life, flag, type, strip vertex and sort key. The cross's check still
passes.

## The port

- **`piney_world::arms::Arms`** is the state: the dummies, the
  lattices, the points and the frame's strips. Each `Actor` keeps one as
  `weapon`. It is made at the actor's first call, from the file hung on
  its hands, and re-read by `change_equip`.
- **`Combat::arms_shown`** runs the frame's calls in order, before
  `ccThEffect`. It poses the hands as the actor stands.
- **`field_world`** draws the strips as it draws the cross's
  (`trail_packet`).
- **`piney_battle::skill::start_arms_effect`** is `StartArmsEffect`.
  Kite's `AnimCtrl` now calls it (`test_battle_kite_rs.py` `anim_ctrl`
  and `small` still pass). The combat runs it for a member's
  `fellow::Out::StartArmsEffect`, which was dropped before.
- **`Effects::arms_particles`** starts the particles. The battle host
  answers `weaponEffPos` from the actor's `Arms` (`char_vec` +0x160) and
  `armsEffectSW` from the character (`char_int` +0xe4).

Tests:

- `kites_blades_leave_trails`: all four dummies on Kite's blades; the
  trails after 16 rows; the aura's colour; the clear. Every weapon's aura
  is -1 or 0-7.
- `orcas_swings_leave_trails`: Orca's heavy blade leaves a white trail on
  30 frames against event 3's goblin, and Kite, standing, none.
  `party_trail_shots` shows the ribbon following the blade.
- `flame_dance_lights_his_blades`: Kite's Flame Dance (art 9) turns
  `armsEffectSW` on and starts row 42's generators on both blades. Their
  particles follow the weapon points, and both trails turn red.
  `flame_dance_shots` shows the fire on his blades.

**Still unknown:** The trails and the particles were not compared with
the game's pictures. The shots of the Flame Dance also show a few
orange specks far off. They come from none of the weapon generators,
which were traced, so they are some other effect of the art; which one
is not known. A member's `StartArmsEffect` was not seen in play. The
hands' matrices are the port's pose, not the game's `ccObj` matrices.
