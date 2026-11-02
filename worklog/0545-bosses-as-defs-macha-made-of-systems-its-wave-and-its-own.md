---
number: 545
title: Bosses as defs: Macha made of systems, its wave and its own; its eyes read through the field
date: 2026-11-02
area: battle
files: crates/piney-battle/src/boss/macha.rs, crates/piney-battle/src/boss/macha/tests.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss/systems/mod.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/macha.rs, docs/engine/boss-macha.md, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-01T23:52:57-07:00
took: 22m
---

# 545. Bosses as defs: Macha made of systems, its wave and its own; its eyes read through the field


Section 2's "Bosses as defs and systems": Quarantine's `ccBoss06` on the
runner (worklog 540), after Tarvos (542).

## The def

`MACHA`'s systems are its wave (`systems::wave::Wave`, its `anmw`) and its
own (`macha::Macha`: the light ball's clips and place, the lasers, the
skill, the stage effect, the transfer's blur and place, `m_patMode`),
started as `ccBoss06::ccBoss06` does (no stage effect, the blur under
`ccBoss03BlurParamCB`; the transfer's place the arena's centre once the
constructor has it). Its frame, `ccThBossEffect`'s pass then
`ccBoss06::Main` (QUA gcmn 0x003a4270):

```text
order:  Macha.Pass, Live[Prologue, TickStop, Macha.Think, LockParty, Move,
        ForwardAndDraw, ClearHold], ClearHold, Macha.Late
affect: Macha.Affect
draw:   ccBoss::Draw's body, Wave (its pose)
```

`Late` is the PP count while the party is held or the target erased.
Its own `show` drew the same body as `ccBoss::Draw`'s and the wave's
pose, so the base body and the wave system draw it now. The wave act and
the Epitaph's reach the wave through `macha::wave`.

## The eyes through the field

`OnThinkEyeLaser` takes the eyes' places (`GetSubstAdrsF`,
`OBJ_dummy_r_eye`, `_l_eye`) as its lasers leave: `Cx::field`'s `node`,
the body as last drawn. Macha no longer keeps an `eyes` member read at
the top of each frame and written over by the probe; the probe's field
answers `node` with the case's places, as the runtime's answers with the
drawn body (which a boss frame does not move, so the read at step 0 sees
what the frame's start saw).

`MachaData` is gone: the tables are views of the volume's
`tables::combat` (`macha_*`); `@1198` (pattern 10's draw, which the port
drops) went unread and is not kept. `Class::Macha` and `BossData`'s
field went with it.

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots as at worklog 537, and `macha_shots` (48) as at
  worklog 544: every PNG byte for byte.
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  150, no `dead_code` allows), fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** nothing this step opened; Gorre, Magus, Kyvia, Innis and Corbenik remain to convert, then `Class`, `each_class!` and `BossData` out.
