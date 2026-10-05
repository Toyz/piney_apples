---
number: 382
title: A level 3 or 4 spell's element hits: its damage calls in ccThEffect reach the battle, so RaJuk Rom's three tornados take HP
date: 2026-10-05
area: battle, render, test
files: crates/piney-world/src/combat/mod.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session/tests/wood_tornado.rs, crates/piney-game/src/session.rs, docs/engine/battle.md, docs/engine/effects.md, BUGS.md
---

# 382. A level 3 or 4 spell's element hits: its damage calls in ccThEffect reach the battle, so RaJuk Rom's three tornados take HP

Issue #48 (ThreePendant, build 86c2571): "Level 2 Wood Tornado attacks
don't seem to have damage collision. This may apply to only enemies.
The Hold is still active, but no damage is caused." The screenshot is a
desert field. Several green tornados with leaves stand over the party,
and a winged foe shows "DATA DRAIN OK".

## Which spell

The wood tornados are `skillTbl` rows 241-244: Juk Rom, BiJuk Rom, RaJuk
Rom and PhaJuk Rom (type 0xc122, `targetRange` 500-700). The foes that
cast them (`enemyTbl` `mag0`/`mag1`):

- BiJuk Rom (level 2): the Green Wyrm (197).
- RaJuk Rom (level 3): the Harpy Queen, Phoenix Queen, Aurora Feather,
  Rainbow Tail, Wood Stock, and the Data Bug Harpy Queen and Wood Stock.
- Juk Rom (level 1): the Thousand Trees.

A level 1 or 2 tornado is one funnel at the target. Level 3 makes three
300 out from it, a third of a turn apart (`ccTornadeElement`, effects.md
"TornadoSystem"). The screenshot shows three, so this is RaJuk Rom from
one of the winged queens, not level 2.

## What the game does

`TornadoSystem` (gcmn 0x00577540) makes the level 1 and 2 hits itself,
every 5 counts from 35 to 75, inside `ccThSkill`. From level 3 it hands
the hits to `ccTornadeElement`, which runs in `ccEffectElementManager::Main`
in `ccThEffect`, a task before `ccThSkill`. `_Level3` (0x004ff4a0) calls
the three-argument `ccSkillDamage` at 0x004ff7e4 on a live target, else the
point form at 0x004ff808:

```text
004ff7d8  move  $a0, $s2            creator
004ff7dc  move  $a1, $s3            target
004ff7e0  lw    $a2, 400($s4)       m_skillPtr
004ff7e4  jal   ccSkillDamage(ccChar *, ccChar *, ccSkill *)
```

That wrapper (0x00573d40) looks for the skill on `SkillEntryTop`. If it
is there, it calls the five-argument `ccSkillDamage` with
`skill->param`, `&skill->acFlag` (+0x54) and `skill->ID`. The timing
tables are at 0x005ed760: `START_2` = 30, 35, 40, 45 for both levels. So
level 3 hits on the element's counts 40 to 80 and level 4 on 50 to 90,
every 5. Fall's, upheaval's and summons' level 3 and 4 elements also make
their own `ccSkillDamage2` calls in `ccThEffect`.

## The cause

piney-effect raised every one of those calls as an event. piney-game
passed on only the ones made during `FxTasks::spell`, that is, the
system's calls in `ccThSkill`. The events from `FxTasks::effect` (the
elements') went into the effect events, and `take_sounds` threw them
away. So every level 3 and 4 tornado, fall, upheaval and summons spell
held its targets for its whole run and hit no one, whoever cast it. This
has been so since the spells were first wired to the battle ([[81]]). It
is not a regression.

The probe also showed that BiJuk Rom works both ways. The Green Wyrm's
hits Kite nine times. Kite's misses or is guarded against the Wyrm,
which is the game's rule: wood attribute 340 and Exdefense 32.

## The fix

The damage calls are now made at the moment the effects make them, in
both passes, through the hook the effect crate already had for this
(`Host::raise`, "for a runtime that must act at once").

- **piney-game.** `BattleHost::raise` turns `SkillDamage`,
  `SkillDamageAt` and `SkillDamage2` into a `SpellDamage` and calls
  `FxWorld::skill_damage`.
- **piney-world.** `FxWorld` now holds the scene mutably, plus an
  optional `FxDamage`: the runs, the frame's env and its affect list.
  `skill_damage` looks up the run by its key and uses that run's
  `acFlag`. When the run is gone it does nothing, as 0x00573d40 does.
  `Combat::frame` gives `ccThEffect` the same affect list that
  `ccThSkill` fills, so an element's affects come before the systems'
  in the frame, as in the game.
- **The systems.** The systems' calls now go the same way. The old
  collect-then-apply loop in `spell_system` is gone, and so is
  `SpellOut::damage`. The damage code's `rand()` draws now fall between
  the effects' own draws, at the place the game makes them.

`boss_shows` clones Fidchell's spell states, because the host now
borrows the scene mutably.

## Checked

- **The session.** `session/tests/wood_tornado.rs` puts a Mimic (no
  element resists wood) beside Kite and lets the two cast on each other:
  - `a_level_three_or_four_tornado_hits_on_its_elements_counts`:
    RaJuk Rom and PhaJuk Rom, both ways. Each HP loss falls on one of the
    element's damage counts from `START_2`, at least two each. Before the
    fix: no hit at all ("243 by the foe true: hits at []").
  - `a_level_one_or_two_tornado_hits_from_count_35`: Juk Rom and BiJuk
    Rom on the system's counts 35-75. It passes before and after.
  - `every_level_three_or_four_element_spell_reaches_its_target`:
    Kite's level 3 and 4 fall (195, 196), thunder fall (259, 260),
    upheaval (203, 204), summons (207, 208) and convergence (215, 216)
    each leave their affect on the Mimic, as a hit or a miss. Before the
    fix 195 failed first.
- **Against the game.** The element's call frames are already checked
  against the game's code by `tools/test_effect_spell_rs.py` ([[79]],
  [[281]]), which counts the events. What was missing was only the
  routing in the runtime, and the session tests cover that.
- The piney-game suite, clippy, fmt and the docs check pass.

**Still unknown:** nothing in the repo. The reporter called it level 2.
The three funnels in the screenshot are level 3's, but without their
pad_log or save it is not certain which foe cast it. The fix covers every
level either way.
