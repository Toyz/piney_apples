---
number: 530
title: Kyvia's core's wave drawn at the core with its own anmw: Out::Wave from act 13, which no game code sets, forced in a harness case and a shot
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss/kyvia/core.rs, crates/piney-battle/examples/battle_probe/kyvia.rs, tools/test_battle_kyvia_rs.py, crates/piney-game/src/session/tests/kyvia_fx.rs, crates/piney-game/src/session/tests/kyvia_stage.rs, docs/engine/boss-kyvia.md
resolves: 529
started: 2026-10-30T13:08:55-06:00
took: 15m
---

# 530. Kyvia's core's wave drawn at the core with its own anmw: Out::Wave from act 13, which no game code sets, forced in a harness case and a shot

Worklog 529 found that Kyvia's core's wave (`kyviaCore::Action`'s act 13)
was drawn at Kyvia's place at time 0: the core emitted `Out::DrawWave`
from inside Kyvia's frame, and the field resolved it with Kyvia's class,
which has no wave. The core now emits its own `Out::Wave`.

## The game's code

`Action` (MUT gcmn 0x004f8cd0), case 13 (0x004f8fc8):

- step 0: `SetAnm(GetChunkAdrsF("ANM_xx11wave"))` on the core's own
  `anmw` (+0x29358);
- step 1: `EntryAffect(this, 1, 10)` on each member when `actCount` was
  25, `_AnimateForward(frameSpd)` while a clip is set,
  `anmw.SetMatrix_PosRotXYZ(this->pos, this->dirc)`, `DrawParts(anmw, 0)`
  through the vtable, and act 12 once the clip ends.

So the wave is the core's, at the core, on its own `anmw`. The port
already ran the step; only the out was wrong.

**No code of the game's sets act 13.** A scan of Kyvia's code (gcmn
0x004d2000-0x00504000) finds no `ChangeAction` with 13 and no store of
13 to `actNum` (+0x1a2). The core's `Affect` turns affect kind 13 into
`ChangeAction(11)`, the drain. The act is dead in the game, and in the
port, whose code never sets it either. No fight shows the bug.

## The change

- `core.rs`: act 13 emits `Out::Wave { who: core, time: anmw's posed,
  pos: the core's, dirc: the core's, scale: 1 }`. The field draws it as
  every other `Out::Wave`.
  - The game's matrix is `PosRotXYZ` and the field's `PosRotZYX`. The
    core's x and y turns are 0, so the two are the same.
- The Kyvia probe prints `Out::Wave` as the harness records `DrawParts`
  (`["draw_parts", frame, 0]`), as it does the arm's.

## The harness case

`tools/test_battle_kyvia_rs.py`:

- a script kind 3 forces the core's act: `actNum`, `actProccess` and
  `actCount` written on the game's core, the same fields on the port's
  (the probe's `kyvia` script);
- `test_core_wave` takes the first two cases of seeds 7400 on with 700
  frames or more and forces act 13 at frame 250. It checks every frame
  against the game and that the core reached act 13.
- With the old `DrawWave` the case fails at frame 251 on `out`: the game
  draws the parts, the port nothing. With the fix it passes.
- The other cases never reach act 13, so their answers are unchanged:
  the 288 pinned calls replay byte for byte, and no pins were re-recorded.

## The shot

`kyvia_core_wave_shots` (piney-game, ignored) forces the act on the
fight's frame 400 and shoots every second frame of 60. The session tests'
fight helper takes a `poke` on the fight, its core and the frame in place
of its `hurry` flag (`hurry` is now one such poke). The test asserts the
wave is at the core's place (within a unit: the core drifts a step after
its `Action`) on more than 30 frames.

## Checks

- `test_battle_kyvia_rs` against the game on Mutation, `test_core_wave`
  with it; the old behaviour failing it.
- The pin: 288 probe calls replayed, every answer byte for byte the same.
- `kyvia_core_wave_shots`.
- The tests of piney-battle and piney-world, and piney-game's full suite,
  release.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 151 before, 151 after.

**Still unknown:** nothing
