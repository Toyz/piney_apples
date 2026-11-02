---
number: 546
title: Bosses as defs: Gorre made of systems, its brothers, its own and its Kerse; a part's own a system on its def's list
date: 2026-11-02
area: battle
files: crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/gorre/brother.rs, crates/piney-battle/src/boss/gorre/message.rs, crates/piney-battle/src/boss/gorre/tests.rs, crates/piney-battle/src/boss/part.rs, crates/piney-battle/src/boss/systems/mod.rs, crates/piney-battle/src/boss/defs.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/gorre.rs, crates/piney-game/src/session/tests/, docs/engine/boss-gorre.md, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-11-02T00:14:53-07:00
took: 20m
---

# 546. Bosses as defs: Gorre made of systems, its brothers, its own and its Kerse; a part's own a system on its def's list


Section 2's "Bosses as defs and systems": Outbreak's `ccBoss05` and its
two `ccBoss05Brother`s on the runner (worklog 540), after Macha (545).

## The def

`GORRE`'s systems are its brothers, its own and its Kerse:

- `gorre::Brothers`: the brothers' characters (+0x29350, +0x29354), each
  brother a boss of its own lent out of its character for Gorre's frame
  (`Hook::Lend`, `Hook::Return`, new hooks the part bosses share) and
  drawn as a part; `Tick` counts their effect slots (the game's manager
  is one array for the three), `Action` is `Slave` (each brother's
  `Main`).
- `gorre::Gorre`: `m_patMode`, the stage effect, the spells and their
  start, the boss camera (on from the constructor).
- `gorre::Kerse`: the fade (`m_transparency`), where it shows Gorre
  (`m_talkPos`, `m_talkDirc`) and its flash. It draws Gorre's body at that
  place in its step 2; Gorre's own draw leaves the body unshown otherwise.

```text
order:  Brothers.Lend, TickEffects, Brothers.Tick, Live[Gorre.Think,
        Brothers.Action, ClearHold], Brothers.Return
affect: (none: ccBoss05::Affect is empty)
draw:   Brothers (the parts), Gorre (its body, not shown), Kerse
```

`Gorre.Think` is `ccBoss05::Main` up to `Slave` (`CalcRealEx`, the target
and the centre, the stop, the hold from the brothers, `Think`, the lock
and its PP count, `Move`, the count to both brothers). Gorre's code takes
a view of the three (`gorre::G`: its own, the lent brothers, the Kerse;
`Others::pair_mut` gives two siblings at once), and the pattern runner
runs on that view (its `ChangeAction` orders both brothers).

## A part's own is a system

`BROTHER`'s systems are its own (`gorre::brother::Brother`), its affect
its `Affect` (`ccBoss05Brother::Affect`: Gorre and the other brother lent
for the report, `Parts::take_with` with the struck brother in its own
place). `Part<T>` now lends a part's own off its boss's system list
(`Systems::take`) and puts it back at its place in the def's
(`Systems::put`); the parts still classes keep their `Class` variant
until they convert (`PartClass`'s two impls). `Class::Gorre`,
`Class::GorreBrother`, `GorreData` and `BossData`'s field are gone: the
tables are views of the volume's `tables::combat` (`gorre_*`). piney-game's
tests find the brothers through `Brothers` and a brother by its def.

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots as at worklog 537, and the four `gorre_*_shots`
  (274) as at worklog 544: every PNG byte for byte.
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  150, no `dead_code` allows), fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** nothing this step opened; Magus, Kyvia, Innis and Corbenik remain to convert, then `Class`, `each_class!` and `BossData` out.
