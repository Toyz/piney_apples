---
number: 524
title: Boss split, step two: the BossClass trait and its one dispatch, Patterns with BaseActs in place of corbenik::Host
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/, docs/engine/boss.md, docs/engine/boss-innis.md, plans/rust-cleanup.md
started: 2026-10-30T11:00:22-06:00
took: 24m
---

# 524. Boss split, step two: the BossClass trait and its one dispatch, Patterns with BaseActs in place of corbenik::Host

The second step of section 2's boss split (`plans/rust-cleanup.md`,
after worklog 523's `Boss { base, class }`). A class's `Main` and
`Affect` are now a trait's, reached through one `match`, and the base's
pattern code calls back into a class through a second trait that took
the place of Corbenik's `Host`. The 288 pinned probe answers are byte for
byte the same.

## The dispatch

- **`BossClass`** (`boss/class.rs`): `main`, `affect` and
  `effects_pass` on `(&mut self, b: &mut BossBase, cx)`.
  - The defaults are what `Boss::main` and `Boss::affect` did for a class
    with no arm: `ccBoss01`'s frame and affect, no effect pass.
  - Every class implements it, the parts too. Corbenik's eyes and roots
    keep the defaults, as they did.
- **`Class::with_rules`** is the one `match`: an arm a class, handing
  `&mut dyn BossClass` to a closure. `Boss::main`, `Boss::affect` and
  `Boss::effects_pass` each go through it; their three matches are gone.
  Skeith and a bare `ccBoss` are the zero-sized `Skeith`.
- **Two affects that took the whole boss** now take its two halves:
  - the Kyvia core's (`kyvia::core::affect`);
  - a Gorre brother's (`affect_alone`). It puts the brother's base and
    members into Gorre's `brothers` for the report and takes them back,
    as the old swap of the whole `Boss` did.
  - So `gorre::brother_affect` and Gorre's empty `affect` are gone (the
    empty one is the trait method, with its note).

## The patterns

- **`Patterns`**: the class's `ChangeAction` (the base's by default),
  `ExecPatternIndex` (the base's by default) and `ExecPattern` (a
  one-word table), with a `const ACTS: BaseActs`.
- **`base_exec_pattern_index`** is Corbenik's one generic copy, now over
  any `Patterns`.
  - The wrappers that fed it (`first`'s and `second`'s `BaseHost`,
    `ThirdHost`, the killer eye's `KillerHost`) and the forms' free
    `exec_pattern`s are gone.
  - Each form implements `Patterns`; the third form keeps its own
    `ChangeAction`.
- **The killer eye's patterns read only what the body tells it.** So its
  `Patterns` is `KillerRules(&KillerView)`, and its `exec_pattern` no
  longer takes the eye's members.
- **`BaseActs`** holds what really differs between the classes' ports of
  `ccBoss::ExecPatternIndex`, found by diffing the six copies:
  - the neutral's forbid on a word the base does not know: 1 for
    Skeith, Magus and Corbenik, 3 for Fidchell, Macha and Tarvos;
  - the dash offset's w lane: 0 for Skeith and Magus, 1 for the rest.
  - So Corbenik's forms are `{ other_forbid: 1, dash_w: 1 }`; the other
    classes take theirs as their copies move onto the trait.
  - The act ids are not among them: every class's copy sets `ccBoss`'s
    numbers (neutral 0, epitaph 12, escape 16, return 17, chase 18,
    dash 7, wander 8). So the base reads Skeith's `act` module, which is
    `ccBoss`'s numbering.
- **The M2 helpers**: `target_ok` is `Cx::target_ok`, and `select` is
  `BossBase::select`. `word` is one function beside the trait.
  Corbenik's copies are gone; the other classes' go in the next step,
  where their pattern code moves onto the trait.

## Checks

- The pin: the 288 recorded `battle_probe` calls replayed against the
  rebuilt probe, every answer byte for byte the same.
- The tests of piney-battle, piney-world, piney-effect and piney-event,
  and piney-game's full suite (the boss session tests in it), release.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 152 before, 152 after.

**Still unknown:** nothing
