---
number: 195
title: "The skills' starts, the shock waves and the condition effects in play; DispConditionEffect ported and checked against the game"
date: 2026-09-26
area: battle
files: crates/piney-battle/src/chara.rs, crates/piney-battle/examples/battle_probe/main.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs, tools/test_battle.py, tools/test_battle_rs.py, docs/engine/battle.md, docs/engine/effects.md
---

# 195. The skills' starts, the shock waves and the condition effects in play; DispConditionEffect ported and checked against the game

The rest of 0194's findings. Of piney-effect's `Effects` calls, six were
never made by the runtime:

- `skill_start` and `skill_start_param`;
- `shock_wave`;
- `condition_effect`, `kill_condition_effect` and
  `delete_condition_effect`.

## The skills' starts

`effSkillStart`, the sparks, rings and circle as a skill begins, has
five sources in the game. The runtime dropped all five:

- `_ccSkillRequest`'s calls: `skill_shows` pushed a `Show::SkillStart`
  that no one read, and without the call's target and arguments. It now
  reads the request's `(who, a, b)` list, with the target (a skill on
  someone else) or the caster (an item's own effect), and passes a and b
  on.
- `ccPlayer::AnimCtrl`'s `kite::Out::SkillStart { sid, item }`.
- `ccFellow::Action`'s `fellow::Out::SkillStart { sid, flag }`.
- An enemy's `startSkill`, `enemy_ai::Out::SkillStart(param)`: its
  `type`, and whether it is the normal attack. This goes through
  `skill_start_param`.
- `ccSkillModifyCondition`: `effSkillStart(ch, sid, stype == 2, 1)` on
  each character a buff or debuff took. The combat's skill loop now shows
  these, and the "miss" font (`ccEntryFlyFontNew(2, -1)`) over each
  character that resisted, which was dropped too.

**The shock waves.** `ccSkillCheckNote`'s 0x8002 notes on a physical
skill were counted (`MainOut::shock_waves`) and never read. The skill
loop now shows `effPhysicalSkillHitShockWave(target position, type &
0xfc)` once a note. The whole `Show::Skill` step was read by nobody.

## The condition effects

`ccChar::DispConditionEffect` (gcmn 0x0056f950, 2 KB) was not ported, only
its effect was. That function also sets `conditionNum` (+0x30), which the
characters' tints read (`char_blend`: poison green, paralysis yellow,
...). So in a fight nothing set it, and a poisoned or sleeping character
showed neither a tint nor an effect. The function:

- The dead (other than 0 and 1) lose the effect.
- A party character with the effects switched off, or Kite in the eye
  view, has it deleted.
- Otherwise it walks the conditions by priority: paralysis, sleep,
  confusion, charm, speed, the record's timed stat changes, HP and SP
  regain, poison, curse. The first active one that is already the number
  stays (its effect kept, remade if the effect's own number +0x1c
  differs). Otherwise the first active one becomes the number, and a new
  effect is made.

battle.md, "Which condition shows", has the numbers.

One quirk came out of the check. The old effect is killed only when the
number was not -1. An effect left behind while the number is -1 keeps
running, unheld, as the game overwrites its pointer. The port keeps that
behaviour: `CondFx::Set` against `Replace`.

`piney_battle::chara::disp_condition_effect` is the function, with the
live effect's number as input and `CondFx` as the answer. The combat runs
it for:

- the `DispCondition` rules in `consequence`;
- those inside the characters' own outputs (Kite's, the members', the
  enemies'), scanned before the effect task.

It keeps each character's effect number, and `ClearConditionEffect`
deletes the effect. piney-game's `AreaFx` holds the `ccConditionEffect`s.

**Checks:**

- `tools/test_battle_rs.py disp_condition`: the game's own function in
  eemu, with its effect calls, `ccSpcConditionEffectSW` and
  `checkCameraType` stubbed, against the probe's `dispcond`. 1,000 random
  characters, conditions, stat changes, effects and switches: 0
  mismatches. The first run found the quirk above: 14 in 300.
- `a_poisoned_kite_shows_it`: event 3's field, Kite poisoned. His number
  is 0 from his next frame and the particles come up; once cured it is -1.
- `a_heal_shows_its_light` now also sees Kite's skill start (the
  controller -12) as he casts Repth.

**Still unknown:** The skill starts and shock waves were not compared
with the game's pictures in play. The party's weapon trails
(`ccSpcChar::ArmsEffect`, on `ccLattice`) and `StartArmsEffect`'s
particles are still not ported. Kite's and the members' `ArmsEffect*`
outputs are dropped.
