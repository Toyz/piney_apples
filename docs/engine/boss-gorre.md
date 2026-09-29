---
title: Bosses - Gorre
status: partial
volumes: OUT, INF
covers: OUT gcmn.prg:0x00497010-0x0049e590, INF gcmn.prg:0x00497010-0x0049e590 (boss05.cpp), INF gcmn.prg 0x005ebf10 Boss05AnmTbl, 0x005ec060 boss05NormalActTbl, 0x005ec100 boss05SuperActTbl, 0x005ec1c0 boss05EpitaphActTbl, 0x005ec228 @1261, 0x005ec250 @1777
worklog: 296
---

# Bosses - Gorre

Gorre (`ccBoss05`, boss05.cpp) is Outbreak's second phase boss: `bossTbl`
row 4, `bossFunc` code 4, file `x51`. Unlike Fidchell it fights beside two
`ccBoss05Brother`s, each its own character of row 4 too, run the way
Kyvia runs its core and gomoras (each a [`Boss`](boss-kyvia.md) of its
own, taken out of its own character for the frame and put back). Gorre
itself keeps to the centre and orders its brothers; the fight is won by
bringing both brothers down, which drains Gorre in turn. The names and
layouts are Infection's DWARF (Infection's gcmn carries the class, never
entered); the behaviour is Outbreak's code.

## Where it comes from

Event 218, field 5: `entry type=7 code=4`.

## The class

```text
ccBoss05 : ccBoss (0x29350)          size unmeasured
+0x29350 ccBoss05Brother *br0, *br1  the two brothers (the base's unused
                                     clumpx/clumpw slot in Fidchell's
                                     layout)
```

`ccBoss05Brother : ccBoss`, its own members: a formation offset off Gorre
(+0x29400, `Init`'s `offset` `TransPosB2W`'d into world space) and which
of the two it is.

## The constructor

`ccBoss05::ccBoss05` (OUT gcmn ctor): `SetBaseParam(ccGetBossParam(4))`,
`Boss05AnmTbl`; Kite's place less 500 on y. Two `ccBoss05Brother`s are
`new[]`'d together, each `Init`'d with an offset 300 either side of Gorre
on x (`TransPosB2W`'d, so it turns with Gorre's own heading) and its
heading copied off Gorre, then `EntrySlave(0, 0)` twice. `InitCenterPos`,
`InitStageEffect`, `InitBossCamera(200, 1500)`. `ChangeAction(0, 0, 4)`,
`SetPatternMode(0)` (Normal), the first pattern run.

## A frame

`ccBoss05::Main` (OUT gcmn 0x00498110) is its own override, not the
base's: `CalcRealEx(0)` (the base's per-character `CalcReal` plus a
protect-gauge broadcast to both brothers, `effProtect`/`SetProtect` on
each), the centre's distance and heading (`ccGetDist`, its `sqrt.s` -
[`sqrt_of`](boss.md)), the stop's count, `Think`, the party held while
`lockPlayer`, `Move`, the boss camera's `CamMain`, the stage fader,
`PreDrawAnm`, the draws, `hold` cleared. Each brother's own frame (its
`Move`, formation-following, and `Think`) runs after Gorre's own.

`Think` (0x00498670), by act (23 words at `@1462`; acts 9, 10, 17 and 19
are the table's own trap, `sw $zero, 0($zero)`, never reached from the
boss's own tables):

| act | function | act | function |
| --- | --- | --- | --- |
| 0, 15 | `OnThinkNeutral` | 12 | `OnThinkEpitaph` |
| 1, 2 | `OnThinkDmg` | 13 | `OnThinkEpitaphWave` |
| 3 | `OnThinkKerse` | 14 | `OnThinkDead` |
| 4 | `OnThinkWave` | 16 | `OnThinkEscape` |
| 5 | `OnThinkTalk` | 18 | `OnThinkChase` |
| 6 | `OnThinkDataDrainAtk` | 20 | `OnThinkSkill` |
| 7 | `OnThinkDashCenter` | 21 | `OnThinkTornade` |
| 8 | `OnThinkWander` (`ChangeNextPattern` at once) | 22 | `OnThinkMagic` |
| 11 | `OnThinkDrain` | | |

## The patterns

`ExecPatternIndex` (0x004978d0): patterns 2-8 and 12 fall to
`ccBoss::ExecPatternIndex` ([`Boss::base_exec_pattern_index`]), whose act
numbers for those already match Gorre's own; Gorre's own words:

| word | operands | does |
| --- | --- | --- |
| 1 | | the wave: act 13 once drained, else 4 |
| 9, 10 | `type` | a target (as Fidchell's 9/10: a negative type a random down party member, else `SelectTarget(type)`) |
| 9 | `sid` | the second brother casts `sid` on the selected target |
| 10 | | `@1261` (`157, 158, 160`) read (`ccRand() % 3`) and dropped: act 20 |
| 11 | `sid` | the second brother casts `sid` on the first, then on itself |
| 15 | | act 3, Kerse |
| 16 | | act 5, Talk |
| 17 | | act 21, Tornade |
| 18 | | act 6, Data Drain |
| 19 | | act 22, Magic |

## Brother

Ordered into an act by Gorre (`Order`, approximated here as a plain
`ChangeAction`); reports a hit and its own death back to Gorre
(`SendMessage`, approximated as Gorre polling each brother's HP once a
frame rather than the full 2984-byte switch). Its own `Affect` shares the
core/gomora kinds ([`kyvia::part_affect`]).

## Tables

`tables::combat`'s `gorre_normal`, `gorre_super`, `gorre_epitaph`
(`boss05*ActTbl`, each to its `-1`), `gorre_anims` (`Boss05AnmTbl`),
`gorre_skills` (`@1261`, `OnThinkSkill`'s three spells and pattern 10's
draw), `gorre_magic_skills` (`@1777`, `OnThinkMagic`'s four, one drawn).

## Unknown

- `ccBoss05Brother`'s own `OnThinkWave`, `OnThinkEpitaphWave` and
  `OnThinkTornade` (1668, 1476, 1356 bytes) have real per-brother
  formation and camera math not yet decoded past their locals; the port
  holds each brother at its formation place facing Gorre's target
  instead.
- `SendMessage`'s full switch (2984 bytes): only msg 1 (a hit) and 4 (a
  death) are approximated (by Gorre's own poll); msg 7, 9, 13, 21 (a fly
  font, an act 11 order, a stat reset) are not ported.
- `Order`'s special `on` values (3, 4, 5, 12, 14, 15, 21) beyond a plain
  `ChangeAction` are not ported; the port always does a plain act change.
- `OnThinkDead`'s exact camera math (the die direction's matrix work) is
  approximated as a look from Gorre toward its brothers' midpoint.
- `ccBossEffFinalPhotonFlashCreate`'s (`OnThinkKerse`) life is a
  placeholder (45 frames, `EffKind::FinalPhotonFlash`); no picture yet
  (crates/piney-game/src/fx.rs).
- Exact frame counts for `OnThinkKerse`, `OnThinkTalk`, `OnThinkTornade`,
  `OnThinkSkill` and `OnThinkMagic` beyond the ones read from the
  disassembly (30, 75, 90 and similar) are approximate where the full
  instruction stream was skimmed rather than read in full.
