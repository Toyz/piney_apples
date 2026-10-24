---
number: 443
title: Macha (ccBoss06) ported from Quarantine's code: its patterns read their operands from its own table, its lasers fly to the land and its light ball's shock bursts, and event 307 ends in the arena
date: 2026-10-24
area: battle, render, volumes, test
files: crates/piney-battle/src/boss/macha.rs, crates/piney-battle/src/boss/macha/eff.rs, crates/piney-battle/src/boss/macha/tests.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/blur.rs, crates/piney-battle/examples/battle_probe/macha.rs, crates/piney-effect/src/macha.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/fx.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/combat.rs, crates/piney-data/src/volume.rs, tools/test_battle_macha_rs.py, docs/engine/boss-macha.md
started: 2026-10-24T01:49:20-06:00
took: 35m
---

# 443. Macha (ccBoss06) ported from Quarantine's code: its patterns read their operands from its own table, its lasers fly to the land and its light ball's shock bursts, and event 307 ends in the arena

Event 307 stopped in field 6's arena: block 2's `entry 7 5` asked for a
boss the port did not have. Macha (`ccBoss06`, boss06.cpp, QUA gcmn
0x003a3460-0x003a81e0) is now ported from Quarantine's own code, with
the names and layouts of Infection's DWARF, whose gcmn carries the class
but never builds it. docs/engine/boss-macha.md has the whole of it.

## What it does

`bossTbl` row 5, file `x61`, body `CMP_trall1` (drained, `CMP_trall2`
with `ANM_ex6x`). Its acts are a light ball, a wave, a charm, eye lasers,
a skill, magic, a Data Drain attack, chases, escapes and a dash to the
centre. It drains to an Epitaph of 6000 HP and dies there.

Five things in its code are not what one would guess:

- **Its own pattern words read their operands from `patTbl`.**
  `ccBoss06::ExecPatternIndex` takes a table and an index, but for its
  own words (2, 9, 10, 11) every operand is loaded from `patTbl` at the
  running index. So `ExecPattern(2, 30)`, which passes a one-off table,
  waits for whatever `patTbl[1]` holds.
- **The skill act's target type is a leftover.** `OnThinkSkill` calls
  `SelectTarget` with `a1`, which still holds 4 from the caller.
- **The Epitaph's wave blows at count 15 after the increment.** The
  count is re-read after `++`, one frame earlier than the source order
  suggests.
- **The chase's SE 224 never plays.** It waits on a clip frame the
  chase's clip never reaches.
- **Three acts are unreachable.** No table holds word 16 (the eye
  lasers), and no affect asks for the back dash (20) or the transfer
  (21). All three are ported and checked by forcing the act.

`SetBlendTypeAfterImage` writes +0x292f8, which nothing reads.
`prevActNum`, `m_bLaserEnd` and `m_effMagic` are written and never read.

Its two effects of its own:

- `ccBossEffLaserShot`: a bolt from an eye, along the aim, until the
  line from its start meets the land (`ccHitCheckLM`, mask 0x20000000).
  There it throws particles and rocks and ends. The port asks the land
  through a new `PartHits::line`.
- `ccBossEffLightBallShock`: a blast growing 10/3 a frame and fading a
  thirtieth (off on its 30th draw). At random waits it throws eight
  bursts, their sizes summed in the soft float's double.

The transfer swaps the blur's callback for `ccBoss06BlurParamFuncCB`
(`blur::BlurFunc::Boss06`: `g_sinRot` += 0x3d0efa35, scale 1 + 0.2 sin,
colour 0x70808080) and gives the old one back after.

## The port

- piney-gen carries six tables per volume: `BOSS06AnmTbl`, the normal,
  expert and Epitaph tables, `@1198` and `@1913` (`macha_*` in
  `tables::combat`). `patterns_through` reads a table up to its closing
  -1, shared with Gorre's.
- `piney_battle::boss::macha` holds the rules (`Class::Macha`) and
  `macha::eff` the two effects. What they draw goes out as
  `Out::Macha(Pic)`.
- `piney_world::combat::boss` starts it for code 5 and poses the eyes
  from the body each frame. It also draws the ball and its light, and
  the waves at their scale.
- `piney_effect::macha` draws the bolts, blasts, bursts and dust from
  `xeffect`.

## Checks

- `tools/test_battle_macha_rs.py` runs Quarantine's code natively
  against `battle_probe` frame by frame:
  - 12 scripted cases (the three unreachable acts forced, hit types 1
    and 3);
  - 40 random cases, 121,406 frames, every reachable act;
  - unittests running both effects' lives.
  All agree.
- `boss::macha::tests`: the constructor, the cheat's tenth, the expert
  table at half the gauge, and the shock's 30 frames and bursts of
  eight.
- `macha_shots` (ignored) plays the arena under god with every frame's
  events through `crate::handle`. Acts 0-5, 12-14, 16, 18 and 22-24
  come; it is drained and dies; event 307 ends at frame 16,952; Macha's
  music plays through 16,705 of those frames. The shots show PhaGan
  Zot's dark stage, the Data Drain attack, and the Epitaph's form dying
  at 129/6000.

On the way: piney-data's `each_volume_has_its_tables` had failed since
[[442]]. From Mutation on, the code carries town06's model table too,
not only Quarantine's. The test now allows it on every later volume.

**Still unknown:** the ball's, wave's and shock's pictures are not
checked against the game's draw frame by frame; nor is the fight won
without god.
