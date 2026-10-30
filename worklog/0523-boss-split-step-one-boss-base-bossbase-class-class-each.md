---
number: 523
title: Boss split, step one: Boss { base: BossBase, class: Class }, each class's rules on both at once, the take/put pairs gone, pinned byte for byte
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/, crates/piney-battle/examples/battle_probe/, crates/piney-game/src/fx.rs, crates/piney-game/src/session/tests/, plans/rust-cleanup.md
started: 2026-10-30T10:35:13-06:00
took: 21m
---

# 523. Boss split, step one: Boss { base: BossBase, class: Class }, each class's rules on both at once, the take/put pairs gone, pinned byte for byte

The first step of section 2's boss split (`plans/rust-cleanup.md`): a
boss is now `Boss { base: BossBase, class: Class }`. A class's rules
hold `ccBoss`'s members and their own at once through a split borrow,
so the 10 classes no longer take their state out of the boss and put it
back around every `Main` and `Affect`.

## The pin

Before any change, every boss harness was run with a recorder:
`probepin.py`, a scratch tool kept outside the repo in
`/mnt/data/claude/scratch`. It wraps `subprocess.run` and keeps each
`battle_probe` request and its whole answer. That gave 288 calls and
417 MB of answers from `test_battle_boss_rs` (Infection),
`innis`/`magus`/`kyvia` (Mutation), `fidchell`/`gorre` (Outbreak) and
`macha`/`tarvos`/`corbenik`/`corbenik_b`/`corbenik_c` (Quarantine).
All of them pass against the game at 9704c02.

After the change, the same requests went to the rebuilt probe, and
every answer was compared byte for byte: 288 calls, none differ. So
each boss's per-frame state, outputs and effects are pinned to what
they were. The game-side runs are the harnesses' own; the replay pins
the port alone.

## The split

- **`BossBase`** is `ccBoss` with `ccBoss01`'s members (Skeith's, kept
  here for now): every field the old `Boss` had but `class`.
  - Its methods (`change_action`, `exec_pattern_index`, `base_main`,
    `base_affect_with`, the targets, the damage, Skeith's acts and so
    on) are `impl BossBase`.
  - `skeith_main` and `skeith_affect` are Skeith's `Main` and `Affect`.
- **`Boss`** keeps what needs the class:
  - `new`, which makes Skeith;
  - `effects_pass`, `main` and `affect`, which match `&mut self.class`
    and hand each class its own state beside `&mut self.base`.
- **The classes.** Innis, Magus, Kyvia, Fidchell, Gorre, Macha, Tarvos
  and Corbenik's three forms take `(b: &mut BossBase, x: &mut Class,
  cx)`.
  - Their `*_of` take helpers and the `b.class = Class::X(x)` puts are
    gone.
  - Their constructors end in `Boss { base: b, class: Class::X(x) }`.
- **The parts.** The gomoras and core, the leaves, the brothers, the
  seeds, the roots and the eyes are bosses of their own characters,
  and still move through `Part` as before. That is `Part<T>`'s item.
  - A new part is `Boss { base, class: Class::Plain }`.
  - `kyvia::plain_boss` makes a `BossBase`.
  - Innis's images hold a `BossBase`.
- **The whole boss.** Where a rule needs a whole boss (the core's
  affect, a brother's affect on its own, `gorre::body`), it still takes
  `&mut Boss`.
- **`PreDrawAnm`.** It no longer asks whether the class is Skeith: only
  Skeith's `Main` reaches the base's.

The field and method paths were rewritten from the compiler's errors
(`b.x` to `b.base.x` on a `Boss`, `&mut` where a reference was wanted),
across piney-battle, its probe, piney-world, piney-game and their
tests. The diff is 1,781 lines added and 1,854 removed in 59 files.

## Still to come in the split

These are later commits:

- the `BossClass` trait with `BaseActs`;
- the base `Main` as ordered building blocks, and one generic
  `base_exec_pattern_index`, `change_next_pattern` and `exec_pattern`
  (the 12 copied frames);
- Skeith's members into a class of their own;
- `Part<T>` for the parts' takes, which remain;
- `enum Fx`;
- the world-side hooks.

## Checks

- The pin: 288 probe calls, every answer byte for byte the same.
- The tests of piney-battle, piney-world, piney-effect and piney-event,
  and piney-game's suite, release.
- clippy with all features, fmt.

**Still unknown:** nothing
