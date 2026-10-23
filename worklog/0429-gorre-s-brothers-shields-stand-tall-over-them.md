---
number: 429
title: Gorre's brothers' shields stand tall over them: ccBoss05Brother::Affect's effResistantShield of its own kind, its scale halved in x and 1.3 times in z and its offset 200 higher, checked against the game's Affect on the element manager
date: 2026-10-22
area: battle, render, test, tooling
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/innis.rs, crates/piney-battle/src/boss/gorre/brother.rs, crates/piney-battle/examples/battle_probe/boss.rs, crates/piney-battle/examples/battle_probe/gorre.rs, crates/piney-effect/src/shield.rs, crates/piney-effect/src/lib.rs, crates/piney-effect/examples/spell_probe.rs, crates/piney-game/src/fx.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session/tests/boss_fx.rs, tools/test_effect_skill_rs.py, tools/test_battle_gorre_rs.py, tools/test_battle_innis_rs.py, docs/engine/boss-gorre.md, docs/engine/effects.md
started: 2026-10-22T18:25:43-06:00
took: 17m
---

# 429. Gorre's brothers' shields stand tall over them: ccBoss05Brother::Affect's effResistantShield of its own kind, its scale halved in x and 1.3 times in z and its offset 200 higher, checked against the game's Affect on the element manager

Worklog 303 showed the shield on Gorre's brothers but not its shape, and
boss-gorre.md listed "the vector tweak (x halved, z times 1.3 plus 200)"
as Unknown. The port drew the plain `effResistantShield(this, 1, -1)` of
a spell on both brothers.

## What the game does

`ccBoss05Brother::Affect` (OUT gcmn 0x004b07c0), for a hit (types 1 and
3):

- Row 40 with `ccSkillCheckType(param[1]) == 0` (physical) calls
  `effResistantShield(this, 0, -1)`: particle's `ANM_x070`.
- Row 41 with type 1 (a spell) calls `effResistantShield(this, 1, -1)`:
  `ANM_x069`.
- The call returns the element (0 for none). With one (0x004b090c,
  0x004b09d4), `Affect`:
  - copies `m_scale` (+0x110), halves x and multiplies z by 1.3
    (0x3fa66666), and writes it back;
  - copies `m_offset` (+0xd0), adds 200 to z, and writes it back.

For a boss, `effResistantShield` gives the scale `(w, w, h/2) * 0.01` and
the offset `(0, -w/2, h/2)`. So each brother's shield is half as wide and
1.3 times as tall, and stands 200 higher. The worklog's "plus 200" is on
the offset, not the scale. A first port that took the second copy the
other way round (the offset made from the scale) was caught by the
effect harness at once.

The port's brother always asked for the spell's shield, so row 40's
showed `ANM_x069`. The rules harness had stubbed the call without its
arguments and never hit a brother with a spell.

## The port

- piney-battle: `Out::Shield { magic, stretched }`. Innis's is `{ 1,
  false }`. A brother's is magic 0 for row 40 and 1 for row 41, with
  `stretched` true.
- piney-effect: `shield::eff_brother_shield` (with
  `Effects::brother_shield`) is `eff_resistant_shield` followed by the
  reshaping on the element's `Base::anim`.
- piney-game: `fx.rs` sends a stretched shield there.
  `FxCensus::shields` lists the shields' elements (target, scale, offset).

## Checks

- `tools/test_effect_skill_rs.py`, a new starter `brothershield`
  (Outbreak on). It runs the game's `ccBoss05Brother::Affect` natively on
  a character of row 40 or 41, hit (type 1) by the first skill of the
  kind it resists. `SendMessage` to Gorre is hooked out, and
  `checkPartyAnnihilation` answers 1 so the number and mark are left out.
  The element is then on the game's manager, against spell_probe's
  `brothershield`, frame by frame, every element member and draw
  compared.
  - The shield group's sweep has 8 of them (rows 40 and 41 on each
    character type), and a quarter of the random shield starts.
  - Outbreak `bulk 60 shield` (65 cases, 2,331 frames) agrees, and so
    does Quarantine's `bulk 20` (885 frames).
  - Infection and Mutation, which have no brothers, still agree.
- `tools/test_battle_gorre_rs.py`:
  - records each `effResistantShield` with its magic;
  - adds brother hits with skill 2, a spell (kind 11);
  - counts the shields by kind.
  - `bulk 12`: 43,540 frames agree, with 116 physical shields and 44 magic.
  - `tools/test_battle_innis_rs.py` records Innis's magic too: `bulk 8`
    agrees.
- `session::tests::boss_fx::gorres_brothers_show_their_stretched_shields`
  (event 218 in the arena, each brother hit every 40 frames by the kind
  it resists): shields on row 40 in 1,125 frames and on row 41 in 1,125,
  every one reshaped.
- piney-battle, piney-effect and piney-world tests, piney-game's suite
  (four threads), clippy, fmt.

Shots (`gorre_shield_shots`, /mnt/data/claude/scratch/a429):
`gorre-shield-00524.png` shows the two tall shields, `ANM_x070` and
`ANM_x069`, over the brothers.

**Still unknown:** nothing. The brothers' three copies of Gorre's param after msg 0's 21 (boss-gorre.md, Unknown) are another part of the fight.
