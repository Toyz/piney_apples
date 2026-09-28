---
number: 117
title: "Skeith's pictures: the effects, the cross's trail, the reversed layer and the arena's fireflies"
date: 2026-09-25
area: battle, render, world
files: crates/piney-effect/src/boss.rs, crates/piney-effect/src/lib.rs, crates/piney-effect/src/effect.rs, crates/piney-effect/src/particle.rs, crates/piney-effect/src/debris.rs, crates/piney-effect/examples/boss_probe.rs, crates/piney-world/src/lattice.rs, crates/piney-world/src/firefly.rs, crates/piney-world/src/evarea_b0.rs, crates/piney-world/src/evarea.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-world/examples/evarea_probe.rs, crates/piney-battle/src/boss.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/skeith.rs, tools/test_boss_effect_rs.py, tools/test_evarea_rs.py, docs/engine/boss.md, docs/engine/evarea.md
---

# 117. Skeith's pictures: the effects, the cross's trail, the reversed layer and the arena's fireflies

Skeith fought in its arena ([[113]], [[115]]), but the picture was thin.
The rules made effects that nothing drew. The cross left no trail, the
magic's inverted screen never came, and the arena's 54 fireflies were
stand-ins. The boss also stood frozen in one pose. All of it is drawn now
([bosses](../docs/engine/boss.md#the-effects-pictures),
[the story maps](../docs/engine/evarea.md#the-fireflies-firefly2)).

## The effects

`piney_effect::boss` is `ccBossEffManager` and the six effects Skeith
makes: WaveShock, MagicSquare (with its `ccBossEffLight`), ForceGenerator
(`ccBossEffBrightMagicSquare`'s photons), AutoSamonRing, IceBreak and
Dead. Each has its `Create`, its constructor and its `Draw`. Their
generators go to the particle system.

- **A photon's generator** follows the photon (`syncPos`). The port
  publishes the photon's place as an anchor (`VecRef::Anchor`) that the
  generator reads each step.
- **The light** is a real omni light. Its blue rises and falls over 80
  frames, turned with the camera's heading (`Host::camera_rot`). The field
  lights the characters with it beside the arena's lights.
- **The ice rocks** are `effIceRock` in the effect control, ids 15-18.
- **`ccIBossEff::cmn_bossCam`**: none of the six reads it, so `BossRun.cam`
  is not needed. Beside their parameter tables, the globals their Draws
  read are `camID`, `cc3d` and `scFadeDef`.
- **The order.** The manager's task runs before the boss's. The field
  calls the manager's pass before `ccBoss01::Main`, and makes the boss's
  `Out::Effect`s right after it; in the game the Creates run inside
  `Main`. Of the Creates, IceBreak's constructor draws one `ccRand()`,
  which the port draws when it makes the effect.
- **Sounds and flashes** reach the area mode. A note with no position
  (`ccSeOnNote`) became `SeNote`.

`tools/test_boss_effect_rs.py` (new) runs the game's Creates and the
manager's `Draw` loop in eemu on the effect machine, against
`boss_probe`:

- each of the six kinds, two cases each;
- `OnMagicAtk`'s own order of Creates, a WaveShock and the dead effect
  alongside, 420 frames.

It compares every draw, sound, flash, generator start and kill, the
light, `m_bEnabled`, the ice rocks' slots, `rand` and the `ccRand`
count, frame by frame. It found 0 mismatches.

Making the harness honest took three fixes:

- the force fields a generator is made with are heap
  `ccParticleForceField` objects, so the harness reads each one's
  parameter row through it;
- `ccCoord`'s stubbed constructor left every `ccAnm`'s transparency 0,
  and it now sets the defaults;
- the probe's empty answers had shifted the case numbering.

## The frozen boss

The actor drew `CMP_trall` at `play.posed`, which nothing set, so every
frame showed the clip's first pose. `piney_battle::boss::Anm` now keeps
`posed`, the frame `_AnimateForward` posed (as `forward_clip`'s
`pose_at`). The actor, the after-images and the wave draw at it.

## The cross's trail

`ccBoss01::DrawCross` runs at the end of `Main`. In act 3 it lays a new
row of `ccLattice(2, 7)` from the sword's two dummies (`DMY_xdummy_w01`,
`_w02`) under `OBJ_ex11swd`'s last world matrix, and every frame it
calls `Disp`. `piney_world::lattice` is `ccLattice`:

- 16 rows as a ring;
- life 6;
- the alpha fades under life 5;
- nothing is sent until 16 rows have been made;
- ADC for the first pair and for vertices behind the eye or off the
  guard band.

The field sends it on effLayer, sorted by mean z, as triangles, depth
GREATER with no write. In `skeith_act_shots` (act 3) it shows as a
magenta ribbon behind the swing.

`test_skeiths_trail` (in `test_boss_effect_rs.py`) drives the game's own
`ccLattice(2, 7)` in eemu the way `DrawCross` does: 605 frames of cross
runs and idle runs, 443 of them sending a strip. It compares the lattice
after every call and, after every `Disp`, the strip `SendPacket` sent:
each vertex's RGBA and ADC as `MakePacket` wrote it, and the sort key.
They match. Getting there turned up three things:

- **The stand-in projection was off-centre.** `MakePacket`'s clip
  measures from (0x7000, 0x7200), not (0x9000, 0x8e00) as a first reading
  of its `addiu` immediates suggested. The port already had 0x7000 and
  0x7200 (`XYOFFSET`); only the harness's stand-in was wrong.
- **The key was an ulp out.** The port had summed and divided in IEEE
  singles. It now sums each vertex's z with the EE's add, as `MakePacket`
  does, and divides by `2.0 * rows` with the EE's divide, as `SendPacket`
  does. The key matches to the bit.
- **`SendPacket`'s "two or more" counts rows, not vertices.** The port
  tested `out.len() < 2` on vertices. That was the same in effect, since
  a head apart from the tail always gives two rows, and it is now literal.

## The reversed layer

`ccBufferReverce::MakePacket` is one white sprite over the view, blended
`(Cs - Cd) * 128/128`. That is 255 less the frame: the picture inverted.
Its alpha test fails into the frame buffer only, with no depth. It goes
on the boss's own layer, priority 1, while the rules report
`Out::Reverse`. In the magic's last part the arena turns negative (green
sky to pink) under the ice crystal, and the HUD stays as it was.

## The fireflies

`FIREFLY2` is ported for type 0, the arena's kind (`piney_world::firefly`).
The arena constructor makes the 54, each over `DMY_marker(k % 18 + 1)`.
They draw from the arena's `fieldrand` (six draws each, 324 in all), so
`Arena::new_seeded` takes the seed before they are made.
`EVENTAREAB0::Draw` calls each one's `Draw`, then its `Move`, between
model 5 and the bobbing models. What the reading showed:

- **Out of reach, a firefly is frozen.** `Move` returns at once while its
  position relative to the player is more than 1000 across. `Draw` skips
  it too.
- **Kite at the centre sees few or none.** The markers are 1200 or more
  out. `skeith_act_shots` gained `PINEY_WALK` to walk Kite off. At
  (1742, 3302) up to 15 sprites are drawn, fireflies and sparks, as small
  pale dots.
- **The rebase is relative.** At life 0 a firefly calls
  `SetBasePosition` with its position relative to the player, so it
  starts again near the world's origin. For the arena that is the
  centre.
- **They rise.** The constructor's velocity is (0, 0, 1). The turns set
  only x and y, so a firefly climbs a unit a frame for as long as it
  lives.
- **Transparency.** `Draw` sets a fading firefly's transparency to 0.1
  times its life, up to 9.9. `Move` then sets it to life / 100, which the
  next `Draw` overwrites. The sprite's alpha saturates.
- **Sparks.** A firefly makes a spark with probability 39/100 a frame
  while it has under three: a drifting one (life 60-89, scale 0.75) or a
  quick one (life 5-9, speed 2, scale 1.2).
- **Render state.** A spark or firefly keeps `ccEff::Init(chunk, 1)`'s
  state: fog and the depth test. `StorySprite` now carries the pattern,
  scale and transparency, and whether it is a flare, whose constructor
  turns both off.

`ArenaAgainstGame` drops the stand-ins and runs the game's own
`FIREFLY2` code in eemu. It makes 900 draws, the player walking among
the markers and the origin, and compares every firefly and spark
`ccEff::Draw` (position, pattern, transparency, scale, layer) and
`fieldrand`'s seed after each frame. They match.

**Still unknown:**
- **`ccBossBlur`, the cinema and `hold 7`** are not ported.
- **Type-1 fireflies**, the fields' (`SetBasePosition2`, the camera-based
  rebase in `Move`), are not ported.
- **Only the eye has checked the picture.** The trail's projection and
  its off-screen ADC, the reversed sprite, and the light's effect on the
  characters' shading have not been set beside the game's frames.
- **The rules and the effects are not checked together.** In the game
  the boss's choices and the effects' generators share `rand`. The field
  port shares it too, but no test runs the rules and the effects together
  against the game. Nor was it checked whether anything later in `Main`
  draws `ccRand` after IceBreak's Create, which the port's later Create
  would reorder.
