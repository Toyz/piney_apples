---
number: 280
title: Skeith's ice rocks fell through the floor: ccEffect::Main's last cases ported
date: 2026-09-28
area: battle, render, volumes, test
files: crates/piney-effect/src/effect.rs, crates/piney-effect/src/debris.rs, crates/piney-effect/src/fall.rs, crates/piney-effect/src/shockwave.rs, crates/piney-effect/examples/spell_probe.rs, tools/test_effect_skill_rs.py, docs/engine/effects.md
---

# 280. Skeith's ice rocks fell through the floor: ccEffect::Main's last cases ported

A play test reported missing particle effects. INF `ccEffect::Main`
(SLUS_202.67:0x001c3e10) has two passes. The first is a jump table
(0x00374cf0, 188 entries on id + 15), which gives 161 ids a case. The
second is a compare chain from 0x001c77f8: 124 ids, 34 cases. Set beside
the port's `pre` and `post` (`crates/piney-effect/src/effect.rs`), four
kinds of id had game code and no port case. With no case, an effect only
draws and ages.

| ids | object | the game's case | who makes it |
| --- | --- | --- | --- |
| 0 | ANM_x300 | both of the meteors' cases: first switch 0x001c41a0, chain 0x001c8900 (the `beqz` before 6, 88 and 87) | nothing |
| 113 | CMP_x042 (80's model) | first switch 0x001c5a80 | nothing |
| 15-18 | CMP_x202a-d | chain 0x001c839c | `effIceRock` (0x001ceaa0) |
| -11..-8 | none | chain 0x001c9b34: `sw $zero, 0($zero)`, then the end | nothing |

The port already had id 0's first-switch case: `fall::meteor_pre`'s doc
said "shared with id 0". But the match arm left 0 out, and so did the
second pass's arm.

## Who makes them

A scan of every `jal` to `InitEffect` or `ccNewEffect` in each volume's
main and its four overlays found 88 calls. Each carries the id register's
last constant; all four volumes have the same calls, functions and ids.
None makes 0, 113 or -11..-8. Over INF's effect code (main
0x001bc000-0x001dc000), the only store to +0x6c is `InitEffect`'s. So
those three are dead in every volume. They are ported for completeness.

The ice rocks are live. `effIceRock` has five calls in every volume:
- three in `ccBossEffIceBreak::Draw` (gcmn 0x0046cc10). Skeith's magic
  (`ccBoss01::OnMagicAtk`) makes one IceBreak at each living member, and
  so does Fidchell's `ccBoss04::OnIceBreak`. Each IceBreak throws twelve
  times: 64 rocks.
- two in `ccBossEffIceMissile::Draw` (gcmn 0x00462d00), at Innis's ice
  missile's impact (`ccBoss02::IceAttack`): 40 rocks.

The port makes IceBreak's ([[117]]). The missile's picture is not
ported, and Fidchell is not.

## The ice rocks' case

Without the chain case the rocks only tumbled (first switch 0x001c4034).
They fell through the floor until z < -500. There `debris_pre` sets flags
3 and the count to 80, so each rock vanished 10 frames later without a
fade.

0x001c839c is the upheaval rocks' 0x001c9b44 (42-45) word for word:
- the land met one frame in four;
- a bounce of 0.75, with z by -0.75;
- rest at life - 10;
- FadeOut(10, lifeTime).

So `debris::ice_rock_post` is `bounce_post(0.75, -0.75)`. [[117]]'s boss
harness compared the rocks' slots, but it never runs
`ccEffectCtrl::Main`, so the rocks never stepped there.

## 113 and -11..-8

The reading of 0x001c5a80 holds, with its exact arithmetic:
- x and y are 2.6 (0x40266666);
- while cnt < 5, z = 2.5 + cnt (2.2 - 2.5) / 5;
- while cnt < life - 15, z = 2.2;
- after that, z = 0.5 + (2.2 - 0.5)(life - cnt) / 15. At cnt = life - 15
  this is 0x400ccccc, an ulp under 2.2;
- then `FadeInOut(5, 0, life)`. At cnt = life its fade-out is 0 / 0,
  which the EE answers with 0x7fffffff, so the transparency is the EE's
  largest number for that one frame, then 0.

-11..-8's chain case stores 0 to address 0, the game's "cannot happen".
It then goes to the end, as every case does. The port gives these ids an
empty arm with a comment.

## The check

`tools/test_effect_skill_rs.py` has a new group, `ids`, on the spell
harness's machine: `ccEffectCtrl::Main` each frame and a flat land for
`ccLandHitCheck2`. Its starters:
- `effIceRock`, called directly;
- 0, 113 and -11..-8, made by `ccNewEffect` and set up as a maker would
  set them, since the game has no maker. 0 is set up the way
  `effMeteoFireBall2` sets up a meteor.

`spell_probe` gained the matching `icerock`, `spawn` and `set` requests.
The sweep throws an IceBreak's twelve at once, makes the meteor 0 for
each element (and one not falling), 113 with lives 0-40 and -1, and each
of -11..-8.

| run | cases | frames | draws | slot-frames | generators | events |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| INF, sweeps + 60 random | 64 | 5,604 | 19,453 | 31,403 | 610 | 64 |
| MUT (`PINEY_VOLUME=mutation`), sweeps + 30 | 34 | 2,786 | 10,079 | 15,968 | 265 | 25 |

Both have 0 mismatches.

Eight one-change mutations each fail the check:
- the rocks bouncing as the debris do;
- the rocks' case dropped. This is the port as it was: it parts at frame
  8, the first landing check;
- 0 dropped from the first pass, and from the second;
- 113's case dropped;
- 113's start;
- 113's fade-out time;
- 113's hold.

Running on MUT first failed on a probe bug, not a port bug. The probe
named a generator's force field "the row's own" by INF's
`particleForceFieldTbl` address. It now takes the volume's address from
`tables::effect`. The whole skill harness still passes on INF (7 tests),
and so do the spell harness's fall test and `tools/test_boss_effect_rs.py`.

Unit tests:
- `debris::tests::ice_rocks_land_rest_and_fade`;
- `shockwave::tests::the_held_wave_stretches_holds_and_sinks`;
- `fall::tests::the_meteor_animation_falls_and_lands`.

## The other volumes

`tools/voldiff.py diff Main__8ccEffectFv --volume MUT`: MUT's cases for
these ids match INF's, up to addresses. So do the jump table's targets
(as offsets from `Main`) and the chain's entries. No per-volume code was
needed. The diff shows three other changes in MUT's `Main`, none for
these ids:
- **The convergence pieces (102-112).** Their first-switch case aims at
  `posT` (+0x50), not at the target's middle (INF: its pos plus half its
  height).
- **The controller -18.** Its chain case passes `&posT` to
  `effSkillChargeObj` as a new second argument.
- **The drill (168).** Its first switch sets `endFlag` after deleting its
  two `ccAnm`. INF leaves it running out its life.

On OUT and QUA the `ids` group parts at the first draw the distance fade
touches. Their recompiled `Main` has `sqrt.s` (OUT 0x001d39fc) where
INF's calls newlib's `sqrtf` (0x001c7404). The two answers are an ulp or
two apart, and this touches every effect, not only these ids.

**Still unknown:** what ids 0, 113 and -11..-8 were for, since nothing
makes them. What a real PS2 keeps at address 0, the word the -11..-8 case
would overwrite. The MUT differences above, which are not ported per
volume:
- the convergence pieces' aim and `effSkillChargeObj`'s new argument
  (MUT's `effSkillChargeObj` body is not read);
- the drill's `endFlag`.
OUT's and QUA's `sqrt.s` in the distance fade. Innis's ice missile and
Fidchell's IceBreak, which are not ported, so nothing in the port throws
their rocks.
