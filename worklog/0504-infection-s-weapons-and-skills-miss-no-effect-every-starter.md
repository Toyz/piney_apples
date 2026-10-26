---
number: 504
title: Infection's weapons and skills miss no effect: every starter they reach is the port's, and every skill row's effects draw in view
date: 2026-10-26
area: render
files: crates/piney-game/src/session/tests/skill_fx.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs, crates/piney-world/src/field_world.rs, BUGS.md, docs/engine/effects.md
started: 2026-10-26T00:23:22-06:00
took: 46m
---

# 504. Infection's weapons and skills miss no effect: every starter they reach is the port's, and every skill row's effects draw in view

BUGS.md: "Infection: some particle effects are missing". The report
guessed the weapons' and the skills'. Worklog 280 had ported every id
with a case in `ccEffect::Main`. The only one seen missing in play was
Skeith's ice rocks, which are not a party effect. Whether anything a
weapon or skill shows was still missing was open.

**The starters, from the code.** Every effect starter in Infection's main
and gcmn was listed with its callers from the relocations: each `eff*`,
`ccParticle*`, `startParticle*`, `ccEnemyEff*`, `cc*ElementGenerate` and
`ccSpcChar::*ArmsEffect`. Each one that a weapon or skill path reaches
has its port and is started where the game calls it:
- `effSkillStart` (`_ccSkillRequest`, `ccPlayer::AnimCtrl`,
  `ccFellow::Action`, `ccEnemy::startSkill`, the bosses), with its exec,
  force and circle rings;
- `effPhysicalSkillHitShockWave` (`ccSkillCheckNote`) and its smokes and
  radiates;
- `ccHitMarkDisp`'s mark, ring and photon; `CalcBattleDamage`'s critical,
  dying, no-damage, guard and protect;
- the recovery system's heal, cure, sanity and resurrect;
- `StartArmsEffect`'s `startParticleEffect2` along a weapon, and the
  trails (`ArmsEffect`, `ClearArmsEffect`, `SetArmsEffectColor`), for
  Kite and the members;
- the five spell systems and their elements, `effMagicAttackSign`, and
  `startParticleEffect` (the fall's);
- `ccEffPawSmoke`; the condition effects' `effAbilityUp` and
  `effAbilityDown`.

**Every skill, cast.** `session/tests/skill_fx.rs` puts Kite, Mia, Elk
and a Mimic in event 19's field. Kite casts each row of `skillTbl` on the
Mimic: 300 rows, with Data Drain's 2-5 left out, since the drain menu
owns their camera. Each cast runs until its run and its effects end.
For every effect slot with an object, on every frame its last
`ccEffect::Main` pass left `dispSW` on, its object must be in the frame's
draws.
- No id failed, on any row. In all, 130 ids with an object were
  started, from the skill-start rings to the summons' drill.
- Particles drew on every cast.
- Each element art lit Kite's blades.
- Two set-up faults showed and were taken out:
  - A slot made in `ccThSkill`, after `ccThEffect`, has `dispSW` on from
    `InitEffect` but no pass yet. The census now counts it as not shown.
  - Data Drain's rows requested bare leave the event camera on.
- The suite runs one skill of each kind: Flame Dance, Wildflower, Repth,
  Gan Rom (tornado), Yarthkins (summons) and Rue Kruz (convergence).
  It also runs Elk's fire art, which lights his staff
  (`ccFellow::Action`'s `StartArmsEffect`).

The draws themselves are the game's: piney-effect's harnesses compare
them in eemu. So nothing a weapon or skill shows is missing. The census
gained `effects_unseen` and `particle_draws`.

Checked: the new tests; piney-game's suite (338) and piney-world's
(127); clippy; fmt.

Docs: BUGS.md, docs/engine/effects.md (Checks).

**Still unknown:** nothing
