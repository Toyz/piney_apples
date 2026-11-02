---
number: 544
title: BossDef identity: the defs statics, one address each, no copies
date: 2026-11-01
area: battle
files: crates/piney-battle/src/boss/defs.rs, crates/piney-world/src/combat/boss.rs, crates/piney-game/src/session/tests/tarvos.rs, crates/piney-game/src/session/tests/macha.rs, crates/piney-game/src/session/tests/corbenik.rs, docs/engine/boss.md
started: 2026-11-01T23:28:02-07:00
took: 25m
---

# 544. BossDef identity: the defs statics, one address each, no copies

`BossDef`'s `==` compares addresses (one def is one boss), but the defs
were `const`s: every `&TARVOS` makes a promoted copy of its own, so a
reference taken in piney-game need not equal the one piney-battle's
`tarvos::new` stored. Worklog 542's first try at Tarvos's arena predicate
(`b.def == &defs::TARVOS`) matched nothing and its shot test failed on
"act 0 met"; it was worked around by comparing classes.

- Every def is a `static` (`..BASE_BOSS` reads the static's fields), and
  `BossDef` is no longer `Clone` or `Copy`, so no def can be copied out
  and compared: a def is its address, from any crate. The two places that
  copied one (piney-world's Kyvia skill-name test) take a reference.
- `defs::tests::each_def_one_address`: every def in `ALL` distinct,
  `BOSSES` among them, `BossDef::of(6)` the `TARVOS` static, `BASE_PART`
  not `BASE_BOSS`.
- piney-game's arena predicates name their def: Tarvos's
  `b.def == &defs::TARVOS`, Macha's `&defs::MACHA`, Corbenik's its three
  forms' defs (no `Class` match left in them).

## Checks

- The pins: 290 recorded `battle_probe` calls replayed against the new
  release probe, none differ.
- Against the game: the eleven `test_battle_*_rs` on their volumes,
  `test_boss_effect_rs` on all four, `test_bosscam_rs` on Infection,
  Mutation and Quarantine.
- The golden boss shots: every PNG byte for byte as at worklog 537;
  `tarvos_under_god` passes through the def predicate. New baselines for
  the conversions ahead taken at this tree (`macha_shots`, `innis_shots`,
  `innis_unforced_shots`, the four `gorre_*_shots`).
- piney-battle's, piney-world's, piney-effect's, piney-data's and
  piney-event's tests, piney-game's suite (release).
- `piney-gen gen --check`, clippy with all features (`too_many_arguments`
  150, no `dead_code` allows), fmt, `cairns check`, `tools/docs.py check`.

**Still unknown:** nothing this step opened.
