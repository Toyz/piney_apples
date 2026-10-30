---
number: 534
title: Skeith out of the base: boss/skeith.rs, no boss derives from ccBoss01, patMode and m_actDirc each class's own, the default Affect ccBoss's
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/, crates/piney-battle/examples/battle_probe/, crates/piney-world/src/combat/, crates/piney-game/src/session/tests/skeith.rs, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-10-30T14:44:14-06:00
took: 23m
---

# 534. Skeith out of the base: boss/skeith.rs, no boss derives from ccBoss01, patMode and m_actDirc each class's own, the default Affect ccBoss's

Section 2's Skeith item (`plans/rust-cleanup.md`). Skeith was the first
boss ported, so `boss.rs` had grown up as his code: his tables, his
constructor as `Boss::new`, `BossBase::skeith_main`, his acts on the base,
`SkeithPatterns` in `class.rs`, `patMode` and `m_actDirc` on `BossBase`.
He is a class like the others now, and what the base kept of him is
settled from the game's code.

## What derives from `ccBoss01`: nothing

Infection's DWARF carries every boss class (boss01-08.cpp, Kyvia's, the
slaves and parts). Each one's base is `ccBoss`; none is `ccBoss01`. So
nothing of Skeith's is shared, and the base's Skeith-isms were wrong for
the others where they reached them:

- **`patMode`** (`m_patMode`) is a member of `ccBoss01`, `04`, `05`, `06`,
  `08`, `08_b`, `08_c`, not of `ccBoss`. Skeith's and Gorre's lived on
  `BossBase`; both are their classes' own now (Fidchell, Macha and
  Corbenik's had theirs already).
- **`m_actDirc`** (+0x29650) is `ccBoss01`'s: Skeith's escape stores to
  `+0x29658` (gcmn 0x0047d394). Magus's escape (MUT gcmn 0x004a2a4c)
  writes `+0x1c0` and `+0x1c8`, `ccBoss::actDirc`. So the base field is
  `act_dirc` (`ccBoss`'s, Magus's and Corbenik's turns) and Skeith has his
  own `act_dirc`.
- **`Affect`**: the default `BossClass::affect` ran `ccBoss01::Affect`
  ("as the port has always run it"). The vtables say `ccBoss::Affect` for
  Kyvia's bodies (INF DWARF, and MUT's `__vt__13ccBossKyvia01` +0x24 is
  0x004720d0 `Affect__6ccBossFv`), Corbenik's eyes and roots, and a bare
  slave. The default is `ccBoss::Affect` now. Gorre's own was already
  empty (OUT 0x004aa200 `jr ra`).
- **Gorre's pattern fallback** went through `SkeithPatterns`. OUT's
  `ccBoss05::ExecPatternIndex` jumps to `ccBoss::ExecPatternIndex`
  (0x0046dd20), whose calls back go through Gorre's own vtable: his
  `ChangeAction` (both brothers ordered), his `ExecPatternIndex`, and his
  `still_w` (`sqc2 vf0`: w 1.0, where Skeith's copy has 0). It is
  `base_exec_pattern_index(gorre, ..)` now, and the act-proccess trick that
  stood in for his `ChangeAction` is gone. The same code builds the dash's
  vector from `vf0` too (0x0046df4c), so Gorre's `dash_w` is 1.0, not 0.

## The move

- `boss/skeith.rs`: `SkeithData` (with its `words(Tbl)`), `Skeith` (with
  `pat_mode` and `act_dirc`), `skeith::new` (was `Boss::new`), its
  `Shape`, `BossClass` (`main`, `affect`) and `Patterns` impls, and
  `Think`, `Action`, the 14 `OnThink`/`On*Atk` acts, `IsValidArea`, its
  wrap and `neutral_act`, as functions of `(b, s, cx)`.
- `BossBase` keeps `ccBoss` only. `boss.rs` is 2,122 lines (2,997).
- The world names no boss for the cross: `Shape::sword_trail` says whether
  a class keeps a trail and adds a row this frame (Skeith's `DrawCross`);
  `combat/boss.rs` reads it. `BattleData.skeith`, a second copy of
  `BossData.skeith`, is gone.
- A test: a drain on Kyvia's body leaves a tenth of its HP and its pattern
  table (`ccBoss::Affect`), where Skeith's would set 4500.

## Behaviour

The pinned 290 probe answers are byte for byte the same, and every boss
harness agrees with the game: the cases they run never reach the changed
paths (an affect 21 or a Super switch on Kyvia, Corbenik's eyes and roots
or a slave; Gorre's base patterns 2-8 and 12, where the w lane and the
nested `ExecPattern` differ). The changes follow the vtables.

## Left for the bosses-as-defs item

`BossData`'s field per boss, `Class`'s default (Skeith), the world's
`SKEITH_DEF` as the other rows' base and the fallback look for a code the
port lacks, the shared `act` numbers named after Skeith's, `FAR`,
`SKILL_*`. They go with `BossDef` and the boss systems (plan, section 2),
added to the plan here with the standing rule that a step touching a kind
builds its defs and systems.

The diff is about 1,000 lines added and 960 removed (most of it moved).

## Checks

- The pins re-recorded at dbf6097 and replayed: 290 calls, 0 differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- `skeith_shots`: 285 PNGs byte for byte the same.
- piney-battle's, piney-world's, piney-effect's, piney-event's and
  piney-game's tests, release.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 151 before, 151 after.

**Still unknown:** nothing
