---
number: 450
title: Corbenik's first form (ccBoss08) and its eight seeds are ported from Quarantine's code and agree with it frame by frame
date: 2026-10-24
area: battle, volumes, test
files: crates/piney-battle/src/boss/corbenik.rs, crates/piney-battle/src/boss/corbenik/first.rs, crates/piney-battle/src/boss/corbenik/seed.rs, crates/piney-battle/src/boss/corbenik/eff.rs, crates/piney-battle/src/boss/corbenik/tests.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/corbenik.rs, crates/piney-gen/src/manifest.rs, crates/piney-game/src/fx.rs, tools/test_battle_corbenik_rs.py, docs/engine/boss-corbenik.md
started: 2026-10-24T07:28:00-06:00
took: 44m
---

# 450. Corbenik's first form (ccBoss08) and its eight seeds are ported from Quarantine's code and agree with it frame by frame

Event 313 asks for `bossFunc` code 7: Corbenik. `ccThBoss08` plays three
forms. With `saveData` byte 0x652f at 1 it plays the third form
(`ccBoss08_c`). Otherwise it plays the first form (`ccBoss08`) until it
exits, then flashes and plays the second form (`ccBoss08_b`) in the same
task. This entry ports the first form, its seeds and its two effects
from Quarantine's gcmn (0x003acbd0-0x003b1a40):
`crates/piney-battle/src/boss/corbenik/`. The names and layouts come
from Infection's DWARF. docs/engine/boss-corbenik.md has the whole of it.

## What it does

The body is `bossTbl` row 7, file `x81`. Its table waits, runs off,
throws seeds, casts skill 252 and leaps:
`2 60, 3, 15 8, 2 150, 14, 2 150, 1, 15 8, 3`.

- The leap is three cubic jumps, each turning 2 pi / 3. Each landing
  makes a WaveShock and a `ccBossEffWave`. The wave's callback hits with
  boss skill 27 at the clip's frame 8.
- The seeds are eight bosses of row 44, each on a character of its own
  (`Class::CorbenikSeed`). Thrown, they fly and bounce. Lying still, they
  count 150 frames down, flush (`ccBossEffFlush`) and burst with boss
  skill 31. A burst sets off every other seed in skill 31's
  `targetRange`. The count is held while the body casts or dies.
- At 0 HP the body dies: the seeds fade, a camera of its own turns, and
  menu 74 shows stream 106.

What the code does that one would not guess:

- Death sets `m_pauseCntDownFlag`, not `m_deadFlag`. Nothing ever sets
  `m_deadFlag`.
- The death camera's eye is figured from the view (its place at height
  200), not from its place.
- `m_stageEff` is never set by the constructor. It is read only after a
  `BeginStageEffect`, or after the death puts `EntryFlash3`'s element in
  it.
- A seed's spins live in `m_rotSpeed[0]` and `[2]`; `[1]` and `[3]` are
  never touched.
- Every pattern holds act changes off (forbid 3), so neither the body's
  nor a seed's damage act ever plays.
- The flash, the pollen and the body press sit in `Think`, but nothing
  enters them. No table holds their words. The flash's one way in is
  `_ChangeVibeType(4)`, which nothing calls. The port leaves them out, and
  with them the blink, the vibe's scale, the energy grow and the pollen.

## Checked against the game

`tools/test_battle_corbenik_rs.py` runs `ccBoss08`'s constructor and
`Main` natively in the Rust VU0 machine. The WaveShock and the dead
effect are stand-ins; the wave and the flush run natively. The scripts
land blows on the body and on the seeds, by skills and by spells, and
end the land past a case's x.

- 46 cases (137,730 frames) agree.
- They reach acts 0, 6, 7, 12, 14, 16, 17, 18, 20 and 21, and the seeds'
  0, 3, 4, 5 and 14.

Five unit tests cover the body standing behind Kite, the seeds' flight,
count and burst, the death taking the seeds, the shield a spell shows on
a seed, and the flush's ten steps.

**Still unknown:**
- None of the world's side yet: the seeds as actors, their pictures
  (`EFF_x000` flush, the count, the charge's generator), the leap and wave
  in the arena, the event's two forms in one task. These come with the
  second form.
