---
title: Bosses - Macha
status: partial
volumes: QUA, INF
covers: QUA gcmn.prg:0x003a3460-0x003a81e0 (boss06.cpp), 0x003a3640 ccBoss06::ccBoss06, 0x003a3b00 SetPatternMode, 0x003a3bb0 ExecPatternIndex, 0x003a4270 Main, 0x003a4310 Think, 0x003a44a0 Affect, 0x003a4980 OnThinkNeutral, 0x003a4ae0 OnThinkDamage, 0x003a4be0 OnThinkChase, 0x003a4e90 OnThinkDashCenter, 0x003a50a0 OnThinkSkill, 0x003a55e0 OnThinkEscape, 0x003a58a0 OnThinkTransfer, 0x003a5b60 OnThinkBackDash, 0x003a5f10 OnThinkLightBall, 0x003a6ad0 OnThinkWave, 0x003a6d70 OnThinkCharm, 0x003a72f0 OnThinkEyeLaser, 0x003a7500 OnThinkDead, 0x003a75d0 OnThinkDrain, 0x003a7810 OnThinkEpitaphWave, 0x003a7aa0 OnThinkDataDrainAtk, 0x003a7d30 OnThinkMagic, 0x003741b0-0x003746b0 ccBossEffLaserShot, 0x00374440 ccBossEffLaserShot::Draw, 0x003746c0-0x00374b8c ccBossEffLightBallShock, 0x003748a0 ccBossEffLightBallShock::Draw, 0x0037d250 ccBossEffLaserShotCreate, 0x0037d320 ccBossEffLightBallShockCreate, 0x00360d30 ccBoss::ExecPattern, 0x00361e30 ccBoss::ChangeNextPattern, 0x005ec270 BOSS06AnmTbl, 0x005ec2e0 boss06NormalActTbl, 0x005ec330 boss06ExpertActTbl, 0x005ec390 boss6EpitaphActTbl, 0x005ec3f0 @1198, 0x005ec410 @1913
worklog: 443, 444
---

# Bosses - Macha

Macha (`ccBoss06`, boss06.cpp) is Quarantine's first phase boss:
`bossTbl` row 5, `bossFunc` code 5, file `x61`, body `CMP_trall1` (the
drained form `CMP_trall2`). The names and layouts are Infection's DWARF
(Infection's gcmn carries the class, never entered); the behaviour is
Quarantine's code. Its `ccBoss` base is the code Outbreak's and
Quarantine's share ([Fidchell](boss-fidchell.md), [Gorre](boss-gorre.md)).

## Where it comes from

Event 307 (M407), field 6's arena after area 107's dungeon: block 2's
`entry type=7 code=5` makes it, block 4 waits for it to be gone.

## The class

```text
ccBoss06 : ccBoss (0x29350)
anmw (ANM_xx11wave), anmBallCharge (ANM_ex61efc0), anmBall (ANM_ex61efc1)
m_effLaser, m_effDead, m_skillID, m_effStart, m_patMode
m_light (ccOmniLight), m_lightRot, m_stageEff
m_ballPos, m_transferPos, m_ballSpeed
m_orgBlurParamFunc (the blur callback Transfer gives back)
```

`prevActNum`, `m_bLaserEnd` and `m_effMagic` are written and never read.

## The constructor

`SetBaseParam(ccGetBossParam(5))`, `BOSS06AnmTbl`; 500 behind Kite and
50 up, Kite's heading; `OffExit`, `OnDraw`, `OnBodyHit`, `OnCheatHP`;
`ChangeAction(0)`; `ccEntryCmnd`. `InitCenterPos` at the arena's
`DMY_center01`, after-images (32), boss effects (32), the stage effect,
`InitBossCamera(250, 1000)`. The blur is enabled under
`ccBoss03BlurParamCB`. `SetBlendTypeAfterImage(0)` writes +0x292f8, which
nothing reads. `SetPatternMode(0)`.

## The patterns

`SetPatternMode(m)` picks `@1143[m]`: 0 `boss06NormalActTbl`, 2
`boss06ExpertActTbl`, 3 `boss6EpitaphActTbl` (row 1, `boss06SuperActTbl`,
is a lone -1 nothing selects). `ExecPatternIndex` has its own words:

| word | act |
| --- | --- |
| 1 | Wave (4), the Epitaph's wave (13) once drained |
| 2 t | wait `t` frames (30 for -1) in Neutral or the Epitaph |
| 9 ty sid | skill `sid` at a target of type `ty` (-1 a living member by `ccRand`) |
| 10 ty | Skill (22): `@1198` drawn and dropped, `OnThinkSkill` casts 161 |
| 11 sid | skill `sid` on itself |
| 14 | Light Ball (3) |
| 15 | Charm (5) |
| 16 | Eye Laser (6) |
| 17 | Data Drain attack (23) |
| 18 | Magic (24) |

Every operand of its own words (2, 9, 10, 11) is read from `patTbl` at
the running index, whatever table was passed: `ExecPattern(2, 30)` waits
for `patTbl[1]`. The rest go to `ccBoss::ExecPatternIndex`. No table
holds 16, and no affect asks for the back dash (20) or the transfer (21):
the eye lasers, the back dash and the transfer are never reached.

## The acts

`Think`'s switch (`@1261`): 0 Neutral, 1-2 damage, 3 Light Ball, 4 Wave,
5 Charm, 6 Eye Laser, 7 Dash Center, 11 Drain, 12 Epitaph, 13 Epitaph
Wave, 14 Dead, 16 Escape, 18 Chase, 20 Back Dash, 21 Transfer, 22 Skill,
23 Data Drain attack, 24 Magic; any other `ChangeAction(0, 0, 1)`.

- **Wave.** `ANM_xx11wave` at 512; on the body clip's frame 40 a
  WaveShock and a flash (15, 0x00e0ffff); fifteen frames on, boss skill 22
  from Macha's place with its `DRAIN_SP` at 100 (the party's SP drained),
  both drains cleared after. The wave confuses those it hits.
- **Light Ball.** The charge (`anmBallCharge`, 550 up) and the ball
  (`anmBall`) flown at the target with an omni light (0x00ffffff, 500 to
  250000) and the white after-image; on arrival a
  `ccBossEffLightBallShock` (below) and its ring.
- **Charm.** The pink stage effect and skill 159 compelled on each
  member (`ccItemSkillCompel`).
- **Skill.** `SelectTarget`'s type is a leftover register's 4; the
  cinema skill 161 (Ranki Lei), the party held.
- **Magic.** The dark stage effect, a cinema, `@1913[count & 3]` (204,
  220, 228, 260: PhaGan Zot, PhaRue Zot, PhaVak Don, PhaRai Don) at the
  target 45 frames on; 90 to 150 frames by spell before the stage
  effect ends.
- **Chase.** Its SE 224 waits on a frame the clip never shows: never
  played.
- **Escape.** `actDirc` wraps once a frame.
- **Transfer.** The blur's callback swapped for `ccBoss06BlurParamFuncCB`
  (`g_sinRot` += 0x3d0efa35, scale 1 + 0.2 sin, colour 0x70808080), given
  back after.
- **Drain.** HP 6000, the Epitaph table, `ANM_ex6x`'s body. The
  Epitaph's wave blows when the count after its increment is 15.

## Hits

`Affect` handles hit types 1 and 3 itself, the rest the base's. Under the
cheat a hit takes a tenth, never below half the HP; at 0 the conditions
clear and it dies. A type-1 hit in Neutral is a damage act, the wait kept.
At half its gauge (`maxPP` 25000) on the normal table the expert table
comes.

## Its effects

- **`ccBossEffLaserShot`.** A bolt (`xeffect`'s `CMP_ex31les0`) at a
  tenth from an eye to where the target stood, `speed` a frame along
  `Ry(pitch) Rz(heading) Rz(-pi/2)`; where the line from its start meets
  the land (`ccHitCheckLM`, 0x20000000): two particle bursts and rocks,
  and it ends.
- **`ccBossEffLightBallShock`.** `CMP_ex31exp1` growing 10/3 a frame as
  it fades a thirtieth (off on the 30th draw); at random waits
  (`ccRand & 15`) eight bursts: particles, smoke and rocks, sizes by the
  soft float's double sum.

## In the port

- `piney_battle::boss::macha` holds the rules (`Class::Macha`), `eff` the
  two effects (`Fx`), handed out as `Out::Macha(Pic)`. `blur::BlurFunc::Boss06`
  is the transfer's callback. The tables come through `MachaData`
  (`tables::combat`, `piney-gen`'s `macha_*` entries).
- `piney_world::combat::boss` starts it for `entry 7 5` (`MACHA`), its
  body `CMP_trall1`, the drained `CMP_trall2`, the eyes' places from the
  posed body each frame (`OBJ_dummy_r_eye`, `_l_eye`), the ball and its
  light, the waves at their scale, and the laser's land hits through
  `PartHits::line`.
- `piney_effect::macha` draws the bolts, blasts, bursts and dust.

## Checks

`tools/test_battle_macha_rs.py` runs Quarantine's code natively against
`battle_probe` frame by frame: 12 scripted cases and 40 random ones
(121,406 frames), every reachable act, the forced Eye Laser, Back Dash and
Transfer, hit types 1 and 3; its unittests run both effects' lives.
`boss::macha::tests` hold the constructor, the cheat, the expert table and
the shock's life.

`session::tests::macha::macha_shots` (ignored) plays the arena under god:
acts 0-5, 12-14, 16, 18 and 22-24 come, the drain and its death, and event
307 ends at frame 16,952 with its music through it.

`machas_fight_to_its_end` and `event_307_from_its_start` (ignored) play it
with no aid; neither is won yet (worklog 444).

## Unknown

- The ball's, wave's and shock's pictures are not checked against the
  game's draw frame by frame.
- Without god the party (Kite 70, BlackRose 70, Balmung 75) falls before
  the gauge breaks; whether the pilot's tactics or the party's levels
  are short is open (worklog 444).
