---
number: 525
title: Boss split, step three: the base Main as building blocks each class calls in its order, one generic ExecPatternIndex, ChangeNextPattern and ExecPattern
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-10-30T11:24:28-06:00
took: 16m
---

# 525. Boss split, step three: the base Main as building blocks each class calls in its order, one generic ExecPatternIndex, ChangeNextPattern and ExecPattern

The third step of section 2's boss split (`plans/rust-cleanup.md`; 523
split the boss, 524 gave it its traits). The copied frames and pattern
code now come from one place, and each class keeps only what differs.
The 288 pinned probe answers are byte for byte the same, and every boss
harness passes against the game.

## The frame's building blocks

`ccBoss::Main` is now blocks on `BossBase`:

- `prologue`: `CalcReal`, `CalcTargetInfo`, then `centre` (the centre's
  place on the plane, its distance and heading);
- `tick_stop`: `stop`'s 60-frame count;
- `lock_party`: the party held while `lockPlayer`, and the reserved menu
  forbid set or lifted. It returns whether it lifted it.
  - `lock_party_as` takes the forbid as the class reads it (Innis's
    `forbid_now`).
- `move_`, as before;
- `forward_and_draw`: `PreDrawAnm`'s clip forward and `Draw`;
- `Cx::pp_count_tick`: a broken protect's count on a frame. The condition
  stays each class's own; it differs between the classes.

Each class's `Main` calls the blocks in its own order, as before:

- Skeith (`base_main`), Innis, Magus, Macha, Tarvos, Fidchell and the
  three Corbenik forms run them in `ccBoss`'s order.
- Kyvia moves before it locks the party, and `head` (the core's, the
  gomoras' and a brother's) locks before the stop count.
- Gorre runs its `CalcRealEx`, then `calc_target_info` and `centre`, and
  counts the protect inside the lock (`lock_party`, then the tick while
  `lockPlayer`).
- Kyvia drops its cinema when `lock_party` reports the lift.
- A seed and a leaf run `prologue` and `tick_stop` with their own steps
  between.
- Magus's leaf `Move` was a copy of `ccBoss::Move` and is now `move_`.

## One copy of the patterns

- **`base_exec_pattern_index`** was copied into Skeith, Magus, Fidchell,
  Macha and Tarvos as well as Corbenik's one. It is now the generic over
  `Patterns`, and the five copies are gone.
- **`base_change_next_pattern`** is one generic for the ten copies.
  - It is `Patterns::change_next_pattern`'s default.
  - Corbenik's first and second forms, and the killer eye's free
    function, keep their preludes (the countdowns let go, the barrier,
    the pause) before calling it.
- **`exec_pattern`** is the trait's default everywhere.
- **What the copies differed in**, found by diffing them, is per class
  data or a hook:
  - `words`, the table `patTbl` points at (Magus's is always its epitaph
    table; the killer eye's is by its slot);
  - `epitaph`: Magus reads its own member;
  - `BaseActs.still_w`, a third field: the w lane of the `moveVector` the
    base clears. It is 0 in Skeith's (`[0; 4]`) and 1 in every other
    class's (`VF0`).
- **Skeith's patterns** are `impl Patterns for Skeith`.
- **Gorre** falls back on the base as Skeith runs it, as the port had it
  (`base_exec_pattern_index(&mut Skeith, ..)`), and keeps its own
  `ChangeAction` through the trait.
- **The duplicate `calc_target_info`s** of Fidchell and Gorre were the
  base's line for line, and are gone.

## The M2 helpers

These are on `Cx` now, and the per-class copies are gone:

- `target_ok`, `select`, `word` (Fidchell, Gorre, Macha, Magus, Tarvos,
  Innis);
- `cursor` (seven copies, and `BossBase::cursor_off`), `cinema_off`,
  `free_cam` and `quake`.
- Macha's `free_cam` keeps its matrix and hands the place to
  `Cx::free_cam`.
- The pattern tables' `.to_vec()` copies the code touched are slices now.

The diff is about 1,160 lines added and 2,170 removed.

## Checks

- The pin: 288 probe calls replayed, every answer byte for byte the same.
- The tests of piney-battle, piney-world, piney-effect and piney-event,
  and piney-game's full suite (its boss session tests), release.
- The boss harnesses against the game on their volumes:
  - `test_battle_boss_rs` (Infection);
  - `innis`, `magus` and `kyvia` (Mutation);
  - `fidchell` and `gorre` (Outbreak);
  - `macha`, `tarvos` and `corbenik` (all three forms) (Quarantine).
- `test_boss_effect_rs` on all four volumes, and `test_bosscam_rs` on
  Infection, Mutation and Quarantine.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 152 before, 152 after.

**Still unknown:** nothing
