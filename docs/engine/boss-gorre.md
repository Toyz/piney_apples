---
title: Bosses - Gorre
status: partial
volumes: OUT, INF
covers: OUT gcmn.prg:0x00497010-0x0049e590, INF gcmn.prg:0x00497010-0x0049e590 (boss05.cpp), INF gcmn.prg 0x005ebf10 Boss05AnmTbl, 0x005ebf80 boss05SlaveAnmTbl1, 0x005ebff0 boss05SlaveAnmTbl2, 0x005ec060 boss05NormalActTbl, 0x005ec100 boss05SuperActTbl, 0x005ec1c0 boss05EpitaphActTbl, 0x005ec228 @1261, 0x005ec250 @1777
worklog: 296, 297
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
(`m_posB`, +0x29400, a plain world-space addend kept in `Brother::offset`
- `ccBoss05::TransPosB2W` only adds the master's own place, no heading
turn, checked against the game) and which of the two it is (`m_slaveID`
via `SetMaster`, read back by `CheckSlaveID`).

## The constructor

`ccBoss05::ccBoss05` (OUT gcmn ctor): `SetBaseParam(ccGetBossParam(4))`,
`Boss05AnmTbl`; Kite's place less 500 on y; `OffBodyHit()` (Gorre itself
takes no direct hit). Two `ccBoss05Brother`s are `new[]`'d together, each
`Init`'d with an offset 300 either side of Gorre on x and its heading
copied off Gorre, `TransPosB2W`'d by the caller into `pos` (the same
place `Init` already computed: no heading turn, see above) - `pos_p` is
left at 0 until the first `Move`. Each brother's own `bossTbl` row is its
own (`ccGetBossParam(id ? 41 : 40)`, not Gorre's row 4: their elemental
resistances differ). `EntrySlave(0, 0)` twice. `InitCenterPos`,
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

`ccBoss05::ChangeAction` (0x004985c0) overrides the base: the base's own
`ChangeAction`, then both brothers `Order`'d into the same act
([`gorre::change_action`], wrapping every `Boss::change_action` call in
this file bar the constructor's first, before the brothers exist) -
`m_actSubCount`/`m_actSubProccess` cleared after.

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

`Order(on)` (0x0049e020): a plain `ChangeAction` for most `on` (forbid 3,
af true); `on` 15 or 0 (a plain neutral) with forbid 1; `on` 3 or 5
(Gorre's own Kerse and Talk) become act 15, unnamed in [`brother::act`]
(a plain watch - `Think`'s default, `OnThinkNeutral`'s twin). `on` 4
(`WAVE`) also resets `m_bWaveEnd` and sets the anm to "ANM_xx11wave"
first, not ported here (harmless: `OnThinkWave`'s own case 0 sets the
same anm). Reports a hit and its own death back to Gorre (`SendMessage`,
approximated as Gorre polling each brother's HP once a frame rather than
the full 2984-byte switch). Its own `Affect` shares the core/gomora kinds
([`kyvia::part_affect`]).

`OnThinkNeutral` (0x0049cb50) and `OnThinkEpitaph` (0x0049cc50, act 12):
stopped or held (`cond::HOLD`), a report back (`SendMessage` msg 3, not
ported) and (`OnThinkNeutral` only) no turn; else `ccSetDirc`'s (mode
256) smoothed turn to face the master (`ccGetDirc(pos_p, master.pos_p)`),
then (`OnThinkEpitaph` always faces first, whether or not it goes on to
orbit) `m_posB` turned a fixed 0x3c8efa35 radians around Z - `Brother`'s
own slow orbit of Gorre while idle. `m_fRadius` set to 1.0 after, not
ported (Tornade's own radius, unused elsewhere).

## Tables

`tables::combat`'s `gorre_normal`, `gorre_super`, `gorre_epitaph`
(`boss05*ActTbl`, each to its `-1`), `gorre_anims` (`Boss05AnmTbl`),
`gorre_brother_anims`/`gorre_brother2_anims` (`boss05SlaveAnmTbl1`/`2`,
each brother's own clip by act), `gorre_skills` (`@1261`, `OnThinkSkill`'s
three spells and pattern 10's draw), `gorre_magic_skills` (`@1777`,
`OnThinkMagic`'s four, one drawn).

## Checks

`tools/test_battle_gorre_rs.py` (`battle_probe`'s `gorre`, `crates/piney-battle/examples/battle_probe/gorre.rs`)
runs `ccBoss05::ccBoss05` and `Main` (which runs each brother's own `Main`
under it) natively against the port, frame by frame, as
`test_battle_fidchell_rs.py` does; not yet at 0 mismatches over `bulk 20`
(docs/engine/boss.md's harness pattern). A random case currently parts
company around 90-100 frames in, in `Brother`'s own `m_posB` orbit: the
port's angle drifts about one `ORBIT_STEP` ahead of the game's over that
span, an off-by-one not yet found (`brother::orbit`/`on_neutral`).
`ccBossCam`'s `MaxRenge` (+0xD8) is not compared: under the stand-in
`InitBossCamera` it reads 0 here, while Fidchell's own harness reads 600
under the identical stand-in - the real mechanism setting it (not
`InitBossCamera` itself, which is fully stood in and does not run the
real `ccBossCam` constructor) is not yet found either.

## Unknown

- `Brother`'s own orbit drift (a whole `ORBIT_STEP` after ~90-100 frames,
  see Checks) and `ccBossCam`'s `MaxRenge` source are open.
- `ccBoss05Brother`'s own `OnThinkWave`, `OnThinkEpitaphWave` and
  `OnThinkTornade` (1668, 1476, 1356 bytes) are decoded past their locals
  (`m_moveSpdB`/`m_moveDircB`/`m_moveVectorB` ease `m_posB`; a
  `SendMessage` msg 2 once close enough to formation ends the Wave;
  Tornade spins `m_posB` by an accelerating/decelerating `m_fRad` each
  frame, msgs 5/6/7 along the way) but not yet wired into the port: the
  brothers hold their formation place facing the master instead during
  those three acts, and `Gorre::on_wave`/`on_tornade` still hold their own
  90/60-frame timers rather than waiting on the brothers' own
  `SendMessage`.
- `SendMessage`'s full switch (2984 bytes): only msg 1 (a hit) and 4 (a
  death) are approximated (by Gorre's own poll); msg 2 (Wave done), 3 (a
  stopped/held brother), 5/6/7 (Tornade's own), 9, 13, 21 (a fly font, an
  act 11 order, a stat reset) are not ported. Its own first branch (on
  msg `156($a1)`, values 21/13/9/7/3/1) is only skimmed.
- `OnThinkDead`'s exact camera math (the die direction's matrix work) is
  approximated as a look from Gorre toward its brothers' midpoint.
- `ccBossEffFinalPhotonFlashCreate`'s (`OnThinkKerse`) life is measured
  (72 frames, `EffKind::FinalPhotonFlash`); no picture yet
  (crates/piney-game/src/fx.rs).
- Exact frame counts for `OnThinkKerse`, `OnThinkTalk`, `OnThinkTornade`,
  `OnThinkSkill` and `OnThinkMagic` beyond the ones read from the
  disassembly (30, 75, 90 and similar) are approximate where the full
  instruction stream was skimmed rather than read in full.
- `outbreak_story_survey` (event 218, `PINEY_SURVEY_GOD=1`,
  `PINEY_SURVEY_FRAMES=150000`) does not reach "done": it reports `blocks
  0xe8f`, stuck cycling menus in field 5. Not run against the
  pre-session code to say whether this is new (the data store's own
  format changed underfoot with `gorre_brother_anims`, so a clean A/B
  needs a full rebuild of both trees); Kite's own `walk_to` also logged
  getting stuck in two rooms during the same run, which may be
  unrelated to Gorre.
