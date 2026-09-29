---
title: Bosses - Fidchell
status: partial
volumes: OUT, INF
covers: OUT gcmn.prg:0x004a34b0 ccThBoss04, 0x004a3590 ccBoss04::ccBoss04, 0x004a3d00 Affect, 0x004a3f00 Think, 0x004a40b0 ExecPatternIndex, 0x004a46b0 OnThinkNeutral, 0x004a4810 OnThinkDmg, 0x004a48e0 OnThinkWave, 0x004a4b70 OnThinkDrain, 0x004a4cb0 OnThinkPrediction, 0x004a5550 OnThinkDrainAtk, 0x004a57e0 OnThinkExecPrediction, 0x004a60b0 OnMeteoSworm, 0x004a6330 OnIceBreak, 0x004a6ae0 OnThunderStorm, 0x004a6d20 OnGroundQuake, 0x004a6fc0 OnMagicCamera(ccChar *), 0x004a7160 OnMagicCamera(vp, d, offset, cam), 0x004a7300 OffMagicCamera, 0x004a7360 OnThinkBackDash, 0x004a76b0 OnThinkJump, 0x004a76c0 OnThinkWander, 0x004a76f0 OnThinkChase, 0x004a79a0 OnThinkEscape, 0x004a7b40 OnThinkTransfer, 0x004a8140 OnThinkEpitaph, 0x004a82a0 OnThinkEpitaphWave, 0x004a8530 OnThinkDashCenter, 0x004a8750 OnThinkDead, 0x004a8860 OnThinkSkill, 0x004a8c40 OnThinkMagic, 0x006dfc30 Think's jump table, 0x006dfcb0 OnThinkPrediction's, 0x006dfce0 OnThinkTransfer's, 0x00617c30 boss04NormalActTbl, 0x00617ce0 boss04SuperActTbl, 0x00617da0 boss04EpitaphActTbl, 0x00617e20 @1038, 0x00617e30 @1039, 0x00617e40 boss0xAnmTbl, 0x00617ea8 @1272, 0x00617ec0 @1441, 0x00617ed0 @2048, 0x00617ee0 @2081, 0x0046d490 ccBoss::Main, 0x0046d940 ccBoss::Affect, 0x0046dd20 ccBoss::ExecPatternIndex, 0x0046e490 ccBoss::ChangeNextPattern, 0x0046e590 ccBoss::ChangeAction, 0x0046f100 ccBoss::IsValidArea, 0x0046ffb0 ccBoss::CalcEffectCameraPos, 0x00470150 ccBoss::BeginDeadEffect, 0x004733b0 OnCinemaMode (a skill's name), 0x0047c0a0 CinemaOn (a skill's name), 0x00472550 the skill name's row, 0x00417db0 EVENTAREAB0::SwitchLayer, 0x00489de0 ccBossEffMeteoSwormCreate, 0x0047d290 ccBossEffMeteoSworm::ccBossEffMeteoSworm, 0x0047d800 ccBossEffMeteoSworm::Draw, 0x0047dab0 ccBossEffMeteo2::Init, 0x0047dd80 ccBossEffMeteo2::Draw, 0x0047df80 ccBossEffMeteo2::Move, 0x00489fa0 ccBossEffThunderStormCreate, 0x0047efd0 ccBossEffThunderStorm::ccBossEffThunderStorm, 0x0047f290 ccBossEffThunderStorm::Draw, 0x0047e660 ccBossEffThunder2::ccBossEffThunder2, 0x0047e9a0 ccBossEffThunder2::Draw, 0x0047ee60 ccBossEffThunder2::Shock, 0x0048a3a0 ccBossEffRockTowerCreate, 0x0047fcd0 ccBossEffRockTower::ccBossEffRockTower, 0x004800f0 ccBossEffRockTower::Draw, 0x00489820 ccBossEffMagicSquareCreate, 0x00477ef0 ccBossEffLight::ccBossEffLight, 0x00478260 ccBossEffLight::Draw, 0x0047bed0 SetupSkillName, 0x004a361c m_predTxt's layer, 0x004a51e0 the prediction's text drawn, 0x00472b90 ccBoss03BlurParamCB; OUT SLUS_205.63:0x00136f90 ccSetColor, 0x00137c30 ccOmniLight::Init, 0x001e6ee0 ccGetDist3D, 0x001dae80 effSmoke, 0x001e2000 effRadiateSomething2; OUT SLUS_205.63:0x001e6e70 ccGetDist; INF gcmn.prg:0x00491af0-0x00496ee4 boss04.cpp, 0x005ebc20 boss04NormalActTbl, 0x005ebcf0 boss04SuperActTbl, 0x005ebdc0 boss04EpitaphActTbl, 0x005ebe40 @1038, 0x005ebe50 @1039, 0x005ebe60 boss0xAnmTbl, 0x005ebec8 @1272, 0x005ebee0 @1441, 0x005ebef0 @2048, 0x005ebf00 @2081
worklog: 294, 295
---

# Bosses - Fidchell

Fidchell (`ccBoss04`, boss04.cpp) is Outbreak's first phase boss:
`bossTbl` row 3, `bossFunc` code 3, file `x41`. It is a `ccBoss` (the base
is [Skeith's page](boss.md)) running the base's `Main`, `Move`,
`ChangeAction` and `ChangeNextPattern` with its own `Think`, `Affect` and
`ExecPatternIndex`. It predicts one of four spells (its text and voice),
lays that spell's skill on every member, then casts it. The names and
layouts are Infection's DWARF (Infection's gcmn carries the class, never
entered); the behaviour here is Outbreak's code, checked frame by frame
against the port.

## Where it comes from

Event 207 (M307), block 9 (`in_field town=3 field=4`): `entry type=7
code=3` and `battle_ready`. Block 11 waits on `absent type=7 code=3`, then
`scene` to town 3. `ccThBoss04` (OUT 0x004a34b0) news a `ccBoss04`
(0x29580 bytes), runs its `Main` (the base's, vtable +0x1c) each frame
until `CheckExit`, then sets its parameter's +0x14.

## The class

```text
ccBoss04 : ccBoss (0x29350)          size 0x29580
+0x29350 ccClump *clumpx, *clumpw    0 (never set)
+0x29360 PRED_TEXT m_predTxt         layer (254), anmText (ccAnm), pos, dirc
+0x294a0 ccAnm *anmw                 the wave (xeffect's ANM_xx11wave)
+0x294a4 ccIBossEff *m_effMagic      the effect a spell waits on
+0x294a8 ccIBossEff *m_effDead
+0x294ac ccBufferReverce *m_br       the ice's reversed layer (deleted, kept)
+0x294b0 ENTRYAFTERIMAGE_T m_eai     20 frames, alpha 100, 1.0 by -0.05
+0x29500 ccEffect *m_effStart        effSkillStart's controller
+0x29504 int actSubProccess, +0x29508 actSubCount    a spell's steps
+0x2950c int m_stageEffId, +0x29510 m_predId, +0x29514 m_reservePred
+0x29518 float m_rotZ, +0x2951c m_transparency       OnThinkTransfer's
+0x29520 int m_skillID
+0x29530 PREDICTION_T m_predInfo     camPos, camView, quakeSW, quakeOffset,
                                     transparency, actCount, revLayer
+0x29570 int m_patMode               0 normal, 1 super, 2 Epitaph
```

## The constructor

`ccBoss04::ccBoss04` (OUT 0x004a3590): `SetBaseParam(row 3)`,
`boss0xAnmTbl`; exit off, drawn, body hit and cheat HP on; at Kite's place
500 back on y and 10 up, his heading. `ChangeAction(0, 0, 4)`,
`ccEntryCmnd`, `InitCenterPos` (`DMY_center01`), `InitStageEffect`,
`InitBossCamera(250, 1500)` and then `bossCam->MaxRenge` = 600 (Infection:
`InitBossCamera(200, 1500)`, the range left). The blur on, never ending,
under `ccBoss03BlurParamCB` (its own pulsing, colour 0x48808080). Then
the Normal table from its start: a reserved prediction would be cast at
once (pattern 15), else `patIndex = ExecPatternIndex(Normal, 0)`.

## A frame

The base's `Main` (OUT 0x0046d490): `CalcReal`, `CalcTargetInfo`, the
centre's distance and heading, `stop`'s count, `Think`, the base's empty
`Action`, the party held while `lockPlayer` (and the menu's forbid), `Move`,
the boss camera's `CamMain`, the stage fader, `PreDrawAnm` (the clip on),
the draws, the base's empty `Slave`, `hold` cleared. Outbreak's
`ccGetDist` (main 0x001e6e70) takes the FPU's `sqrt.s` (truncating) where
Infection's calls newlib's `sqrtf`.

`Think` (0x004a3f00), by act (26 cases, table 0x006dfc30; any other is
`ChangeAction(0, 0, 4)`):

| act | function | act | function |
| --- | --- | --- | --- |
| 0 | `OnThinkNeutral` | 13 | `OnThinkEpitaphWave` |
| 1, 2 | `OnThinkDmg` | 14 | `OnThinkDead` |
| 3 | `OnThinkPrediction` | 16 | `OnThinkEscape` |
| 4 | `OnThinkWave` | 18 | `OnThinkChase` |
| 5 | `OnThinkExecPrediction` | 20 | `OnThinkWander` |
| 6 | `OnThinkDrainAtk` | 21 | `OnThinkTransfer` |
| 7 | `OnThinkDashCenter` | 22 | `OnThinkBackDash` |
| 11 | `OnThinkDrain` | 23 | `OnThinkJump` (nothing) |
| 12 | `OnThinkEpitaph` | 24, 25 | `OnThinkSkill`, `OnThinkMagic` |

No pattern reaches 20, 21 or 23; 8 and 17 (the base's wander and return)
fall to the default.

## The patterns

`ExecPatternIndex` (0x004a40b0), the words of its own:

| word | operands | does |
| --- | --- | --- |
| 1 | | the wave: act 13 once drained, else 4 |
| 9, 10 | `type` | a target: a negative type picks at random among the members on the lists whose `dead` is set (a first `ccRand` dropped), else `SelectTarget(type)`; a bad one falls back on `SelectTarget(3)`, then a 30-frame wait |
| 9 | `type sid` | `ccItemSkillRequest(this, target, sid, 1)`, then wait for the skill |
| 10 | `type` | act 24 (`ccRand() % 3` drawn and dropped) |
| 11 | `sid` | the skill on itself, flag 1, then wait |
| 14 | | reserve the prediction, act 3 |
| 15 | | the reserved spell, act 5 |
| 16 | | act 6 (a member drained) |
| 17 | | act 22 (the back dash) |
| 18 | | act 25 (a spell of `@2081`) |

The rest are the base's (OUT 0x0046dd20: 2 wait, 3 escape, 4 chase, 6 a
dash of `d` across the centre as act 7). The tables (Outbreak's; each to
its -1):

```text
Normal  0x00617c30: 2 60 | 4 | 1 | 2 90 | 17 | 3 | 10 0 | 2 60 | 1 | 6 1000 | 3 | 14 | 3 |
                    10 0 | 2 60 | 4 | 2 60 | 1 | 3 | 15 | 2 30 | 6 1000 | 2 90 | 4 | 2 90 |
                    1 | 2 120 | 17
Super   0x00617ce0: 2 60 | 16 | 3 | 17 | 2 30 | 10 0 | 2 90 | 6 1500 | 4 | 2 60 | 1 | 2 60 |
                    14 | 3 | 10 0 | 2 60 | 4 | 2 60 | 1 | 3 | 15 | 2 30 | 6 1000 | 2 90 |
                    4 | 2 90 | 1 | 2 120 | 17
Epitaph 0x00617da0: 2 60 | 6 1000 | 3 | 18 | 2 90 | 4 | 1 | 3 | 3 | 10 0 | 2 90 | 17 | 3 |
                    18 | 2 90 | 1 | 3 | 17 | 3 | 11 151 | 2 60 | 3
@1038 0x00617e20: ANM_ex41txt1, txt2, txt4, txt3     @1039 0x00617e30: 0 1 3 2
@1441 0x00617ec0: 171 170 173 169    @2048 0x00617ed0: 157 158 160
@2081 0x00617ee0: 227 219 203 259    @1272 0x00617ea8: 157 158 160 (unused)
boss0xAnmTbl 0x00617e40 (26): 0 ANM_ex41nut, 4 ANM_ex41act2, 12 ANM_ex4xnut,
                    13 ANM_ex4xact1, 14 ANM_ex4xdea0, the rest none
```

Infection's Normal and Super tables are longer (52 and 50 words): more
back dashes and skills between the prediction (14) and its cast (15).

## The prediction and the spell

`OnThinkPrediction` (act 3, 0x004a4cb0), a step each (`actCount` counted
before the switch, the old count read):

1. At count 0: the nearest target (none: a wait of -1); locked, off the
   targets, the cursor off; camera 3 1500 behind the target's way from
   Fidchell and 50 up, looking at Fidchell 300 up; `m_predId =
   ccSys.count & 3`; cinema 64, 65, 67, 66 by it. At 45: `SwitchLayer`,
   the stage darkened (0x64000000, 15, 15, 30000).
2. At 15: `ccEvVoiceStop`.
3. The text `@1038[m_predId]` of x41 on its own layer, 1000 ahead of
   camera 3's eye; `ccEvVoiceRequest(-40, @1039[m_predId])`.
4. The text played to its end (turned by the camera); then camera 3 by
   `OnMagicCamera` 1500 off the nearest target, 100 up, looking 200 over it.
5. At 0: `ccItemSkillCompel(this, member, @1441[m_predId], 0)` on each
   member alive; at 60 on.
6. The stage back.
7. The count stepped twice a frame; at 15 (the first step's value):
   `SwitchLayer`, unlocked, on the targets, cursor on, camera 2 twice
   (`OffMagicCamera(0)`), cinema off, the next pattern.

`OnThinkExecPrediction` (act 5, 0x004a57e0): off the targets, locked (chat
excepted), cinema 14, 15, 17, 16 by `m_predId`; the eye 1800 off Fidchell
on the centre's line and 50 up, looking 300 over it (camera 3, or for the
quake the boss camera's mode 5); Fidchell turned to the centre. At 30
`ANM_ex41act1` with SEs 233, 196 and 196 (note 65) at 0, 45, 70 of its
count; at its end, turned to the target, the stage darkened (0x7fff; not
for the ice), the eye 2000 behind the target, looking 200 over it. Then
the spell until it ends, the stage back, and a 15-frame fade in
(`m_predInfo.transparency` +1/15) to camera 2 (the quake: mode 6), the
layer back, unlocked, the next pattern.

| `m_predId` | spell | what | then |
| --- | --- | --- | --- |
| 0 | `OnMeteoSworm` (0x004a60b0) | magic square 2, Fidchell hidden; at 30 `OnMagicCamera`, its eye 100 up and view 200 up; 50 meteors from 1000 behind the target and 3000 up to within 500 of it (`ccBossEffMeteoSworm`), the view following the last | boss skill 14 at Fidchell; camera 2 |
| 1 | `OnIceBreak` (0x004a6330) | Skeith's magic: magic square 0, SE 225; at 30 force generators (8 photons, 100) 1000 up over each member alive, SE 226; SE 227 at 130; once they are gone the ring (35) at the target; once it is gone the ice at each member on the lists (not only the living), Fidchell hidden, the layer switched, the stage back, SE 228; the screen reversed while the ice lasts | boss skill 2 at the target; SEs 66 (note 52), 65, 56 there |
| 2 | `OnThunderStorm` (0x004a6ae0) | magic square 1; at 30 16 thunders 200 round the target (`ccBossEffThunderStorm`), Fidchell hidden, a flash (15, 0x80e0ffff) | boss skill 17 at Fidchell; camera 2 |
| 3 | `OnGroundQuake` (0x004a6d20) | 128 rock towers within 400 of the target (`ccBossEffRockTower`), Fidchell hidden, the free camera 1000 off the target on the centre's line and 500 up, looking at its foot; a `QuakeCam(35, 35, 0)` each frame | boss skill 16 at Fidchell |

Infection's `OnThinkExecPrediction` has no cinema, keeps camera 3 behind
the target from the start, and leaves Fidchell's heading; its ice ends
with `ChangeNextPattern` (a second one, Outbreak dropped it) and its SEs at
Fidchell; its thunder storm moves camera 3 by `OnMagicCamera` (Outbreak
computes the vectors and never calls it).

## The other acts

- **The wave** (acts 4 and 13): `ANM_xx11wave` at speed 512; by the
  body's frame SE 223 at 25, SE 224 at 42 (35), from 44 (40) on the wave
  drawn, at 45 (40) the WaveShock and a flash, at 59 (55) boss skill 13 at
  Fidchell; a `QuakeCam(15, 15, 15)` each frame
  `checkCameraShakeRange(pos)` answers yes; the next pattern once both
  clips end.
- **The member drain** (act 6): locked, off the targets, cinema -1; at 45
  menu 74 on stream 68 for the target's slot; once the menu closes, affect
  13 on the target, unlocked, cinema off.
- **The chase** (act 18): at 100 toward where the target stood, off the
  targets, SE 57 (note 55); an after-image every fourth frame (SE 229 at
  the first); over within 200 or after 151 frames; out of the arena
  (`IsValidArea`), a dash of 500.
- **The escape** (act 16): at 100 (50 from frame 61) along whatever
  `moveDirc` was, for 150 frames while the target is within 1500; out of
  the arena after 30 a dash of 1500.
- **The back dash** (act 22): off the targets, at 150; each tenth frame
  60 degrees either side of the camera's heading, with `effSmoke` dust;
  out of the arena to the centre; 31 frames. At frame 0 it turns to the
  camera's heading plus pi only when it is already out of the arena.
- **The dash** (act 7): the base's dash at 50 to `dashPos`, 121 frames at
  most, SE 229, then SE 57 (note 55) and 229 at 3.
- **The skill** (act 24): camera 3 1000 off the target at 200 high, one of
  `@2048` by `ccRand() % 3`, `effSkillStart`, locked, the skill's cinema
  (`OnCinemaMode` with a skill id, OUT 0x004733b0); after 30 frames and
  the start's `endFlag`, `ccItemSkillCompel(this, target, sid, 0)`; 65
  frames on, camera 2.
- **The spell** (act 25): the stage darkened (3000), camera 3 as the
  skill's, `@2081[ccSys.count & 3]` with its cinema; at 60
  `ccItemSkillRequest(this, target, sid, a3)` (a3 as the frame left it:
  the harness measures 1); the stage back 60, 150 or 90 frames on (227;
  203, 219; 259); 15 on, camera 2.
- **The transfer** (act 21, unreached): fade out spinning (pi/5 a frame),
  to the centre 1000 up, a force generator (16 photons, 120, CLUT 5), down
  as it fades in, then act 0. Its second turn test is against pi, not -pi.
- **Death** (act 14): off the lists, `BeginDeadEffect(2000, 2000)`; exit
  once the dead effect and the clip end.

## Hits and the drain

`Affect` (0x004a3d00) is the base's (a hit's HP a tenth under cheat HP,
never below half; 13 to act 11; 21 to a tenth, the gauge -1), then: 21
sets HP and max HP to 5000; a hit (1 or 3) in the normal table with the
gauge at half `maxPP` or more starts the Super table (`m_patMode` 1). Act
11 (`OnThinkDrain`, 0x004a4b70): cheat HP off, the Epitaph table
(`m_patMode` 2), act 12 (`epitaph` set). A prediction reserved at either
switch is cast at once.

## The spells' effects

The boss waits on `m_effMagic->m_bEnabled`; the manager's `Draw` runs each
effect before the boss's task.

- **`ccBossEffMeteoSworm(sp, ep, n, radius, v, &camView)`** (0x0047d290,
  `Draw` 0x0047d800): for all but the last meteor an end `ccRandF(radius)`
  off `ep` at `ccRandF(pi)`, a pull `2 + |ccRandF(2)|` (in doubles), a wait
  `(ccRand() & 31) + 31`; the last straight to `ep`, pull 3, wait 31. A
  meteor (`ccBossEffMeteo2`, `Init` 0x0047dab0, `Move` 0x0047df80) flies
  from `sp` at `v` along the pitch and heading to its end, the pull added
  each step, and lands at or under its end's height (`effSmokeRock`, two
  `ccParticleExplode`, SEs 35 and 40). Each meteor's first step plays SE
  57 if its index is a multiple of 8. The last carries an omni light and
  writes the boss's `m_predInfo.camView` (200 over it). Done when none
  waits or flies.
- **`ccBossEffThunderStorm(pos, radius, n)`** (0x0047efd0, `Draw`
  0x0047f290; the `Create` plays SE 41 note 67 first): `n` bolts
  (`ccBossEffThunder2`, 10 `ccRand` each) `radius` round `pos` at
  `ccRandF(pi)`, waiting `(k / 3) * 5 + 15`; each starts with SE 68 (odd
  ones SE 41 note 67 too). A bolt grows a segment a `Draw` (16 `ccRandF`
  and a `ccRand` a segment), strikes at the ninth (`Shock`: a flare, four
  radiating rocks, five bursts; 16 draws; SE 35 for odd bolts), fades
  1/15 a `Draw` and 31 `Draw`s after. The first bolt gone brings magic
  square 1 and five more rumbles (SE 68, note `63 - (ccRand() & 7)`) at 4,
  23, 39, 48 and 57; done 85 frames after it once all are gone.
- **`ccBossEffRockTower(pos, n)`** (0x0047fcd0, `Draw` 0x004800f0): each
  tower `ccRandF(400)` off `pos` at `ccRandF(pi)`, 1000 under, scaled `1 +
  ccRandF(0.5)`, turned `ccRandF(pi)`, waiting `(5k >> 2) + 15`; it rises
  33.3 a frame to 100 under (`effSmoke`, `effRadiateSomething2` with a
  `ccRandF(pi)`) and fades 1/15. From its 31st `Draw`, every twelfth a
  rumble (SE 56, note `(|ccRand()| & 7) + 60`). Done once every tower is
  gone.
- The magic squares live 91 `Draw`s (n 0, 2) or 71 (n 1); the force
  generators `life + (num - 1) / 2 * 10 + 3` (133 and 193); the ring 20,
  the ice 123, the WaveShock 45, the dead effect 120 ([boss.md](boss.md)).

## The spells' pictures

Each `Draw` above also draws, on the effect layer (the manager's
`SetActiveLayer(3)`):

- **A meteor** (`ccBossEffMeteo2::Draw` 0x0047dd80, after its `Move`):
  the matrix `RotY(m_rotLight)`, `RotZ(pi/2)`, `RotY(pitch)`,
  `RotZ(heading)`, `RotZ(-pi/2)`, then its place. At it `m_anmFire`
  (`particle`'s `ANM_x300`, stepped, set again at its end) and the glow
  `m_cmpLight` (`CMP_x100`); `m_rotLight` then turns by pi/30. The swarm's
  light (a `ccOmniLight`, `ccSetColor(0x80ffffff, 1.0)`: red, a little
  blue; intensity 1, full to 0, nothing past 500) sits at the last
  meteor, in the light group from the constructor until that meteor has
  landed.
- **A bolt** (`ccBossEffThunder2::Draw` 0x0047e9a0): ten points, the top
  1000 up over its place, its place the last; each `Draw` sets the eight
  between, point k+1 `(ccRandF(100), 0, -100 (k + 1))` turned by
  `ccRandF(pi)` from the top. For each segment grown so far, from point s
  to s+1 (through `ccTransPosW2P`): `CMP_x012` (its CLUT set to
  `@4154[0]` "CLT_x012", its own) scaled (3, length / 900, 3), turned
  `RotZ(-pi/2)`, `RotY(pitch)`, `RotZ(heading)`, `RotZ(-pi/2)`, at point s,
  at the bolt's transparency; then `EFF_x011` (`CLT_x011c2`) at point s,
  3 wide, 0.025 of the length tall, turned by pitch less pi/2, the
  pattern `ccRand() % patNum`. The strike's calls are
  `effFlareRing(pos 10 up, 2)`, four `effRadiateSomething2(pos, (-pi/2,
  0, ccRandF(pi)), 45, 4, 4)` and five `ccParticleExplode(pos,
  (ccRandF(10), ccRandF(10), 10 (ccRand() & 1)), 1, 5.0)`.
- **A tower** (`ccBossEffRockTower::Draw` 0x004800f0): every tower each
  `Draw`, before it moves: `CMP_x101b` (the constructor's `"abcd"[1]`) at
  `SetMatrix_PosRotZYXScale(pos, (0, 0, turn), scale)` at its
  transparency. A tower out of the ground makes `effSmoke(pos, 0, 10, 60,
  114, 512, 32)` and `effRadiateSomething2(pos, (-pi/2, 0, ccRandF(pi)),
  45, 4, 4)`.
- **A landing** (`Move`): `effSmokeRock(pos, (-pi/2, 0, 0), 45, 2, 1,
  -1)` and `ccParticleExplode(pos, (0, 0, 10), 1, 5.0)`, then at 15 up.
- **The magic squares** (`ccBossEffMagicSquareCreate` 0x00489820): n 1
  plays SE 68 at pos in fields 4-8, else SE 41 at note 57, starts
  `MagicSquareGenerator` row 2 (force fields 3, 4) 350 up and makes
  `ccBossEffLight(pos, 1, 60, 10)`; n 2 plays SE 35 at note 48, starts
  rows 6 (fields 8, 7) and 5 (6, 10, 9) and makes `ccBossEffLight(pos, 2,
  80, 10)`. The light (`Draw` 0x00478260) by its mode: 1 faces the
  camera's heading (a `ccRandF(0.5)` tilt drawn and lost), counts, and is
  grey (0x555555, its light moved) every third `Draw` and orange
  (0xffab44) between; 2 swings 200 out tilted by -1.5393804 at the
  camera's heading, which after 15 `Draw`s falls behind by 0.6 a `Draw`,
  and pulses red as mode 0 pulses blue.
- **`ccSetColor(out, c, s)`** (main 0x00136f90): with no alpha byte r, g,
  b over 255; with one, an HSV colour (hue the low byte in sixths of 256,
  saturation and value the next two), w kept. Mode 0 and 2's colour goes
  to it through an inline `cvt.w.s` on Outbreak (Infection's `fptoui`
  gives 0 for a negative count): a falling count below 0 is a colour with
  an alpha byte, a grey.

## The prediction's text, its voice and the skills' names

- **The text** (`m_predTxt`, step 3 at 0x004a51e0): `anmText` is x41's
  `ANM_ex41txtN` with `SetRenderState(CCRS_ZENABLE, 0)`, stepped and drawn
  each frame with `ccLayer::active` its own layer (254 on sysLayer's view,
  made in the constructor at 0x004a361c). Its `pos` and `dirc` are
  written, never read: the animation's own camera (`F_Camera`) places
  it, so it stands on the screen.
- **The voice**: `ccEvVoiceStop()` at step 1, then `ccEvVoiceRequest(-40,
  @1039[m_predId])` at step 2: event -40 is a field voice group
  (`ccVoiceRequest`), Fidchell's lines.
- **A skill's name** (`OnCinemaMode` with a skill 0x004733b0, `CinemaOn`
  0x0047c0a0): OUT 0x00472550 picks a row by `game.field` (+0x24) and
  skill, null where the row has no file or texture, and `SetupSkillName`
  (0x0047bed0, as Infection's) shows it. Field 4's rows are Fidchell's:
  157, 158, 160 (`@2048`) and 227, 219, 203, 259 (`@2081`); fields 2, 5,
  6, 7 and 8 have rows of their own.

## In the port

`piney_battle::boss::fidchell` (`Class::Fidchell`) is the rules;
`fidchell::eff` the three spells as the rules keep them (`EffKind::
MeteoSworm`, `ThunderStorm`, `RockTower`: their lives, `ccRand` draws,
sounds, the meteor's camera view, and what each `Draw` drew: a meteor
moved, a bolt's points, segments, transparency and patterns' draws, a
tower's place and transparency before it moved). They run on a small
`eff::Host` (`ccRand`, the frame, `ccGetDist`, the calls): the boss's `Cx`
in the fight. The tables are the combat group's `fidchell_*` entries
(data version 16) and `cinema_skill_rows` (17: OUT 0x00472550 run in eemu
by piney-gen for fields 0-31 and skills under 512). The base's `Main` is ported with
Outbreak's `ccGetDist` (`sqrt_of`: `sqrt.s` on Outbreak and Quarantine);
the skill damage's range test takes it too. New `Out`s: `Fidchell(Pic)`
(the text, voices, dust, a meteor's landing, rocks, a bolt's strike),
`CinemaSkill` and `CamMaxRange`; `CamView.shake` is
`checkCameraShakeRange` of the boss's place. Camera 3's eye and view are
kept by the rules (`Fidchell::cam3`), which its own `cameraGetPos(3)` and
`cameraGetView(3)` read back. `effSkillStart`'s controller is taken to end
8 frames on (`SKILL_START_LIFE`).

In the field (`piney_world::combat::boss`, code `FIDCHELL`): x41's
`CMP_trall1`, and `CMP_trallx` under the Epitaph's `ANM_ex4x*` clips; the
boss camera `InitBossCamera(250, 1500)` with `MaxRenge` 600; the cinema's
names from x41's `TEX_fid_skl` (rows 14-17, 64-67), and a skill's from
`cinema_skill_rows` for the fight's field (`BossLook::skill_names`). The
text is a `piney_desktop::anm::Anm` (`zenable` off) set from `Pic::Text`,
drawn on layer 254 through the frame's view and its own camera. The
voice goes to the event voice path (`Event::Voice { event: -40, .. }`,
`Event::VoiceStop`).

The pictures are piney-effect's `fidchell` (in `boss::BossEffects`, a
`BossEff::Spell`): after the boss's task (`FxTasks::boss_shows`) each spell
in the boss's effect slots is drawn from its rules' state
(`sync_spells`); the rules' `Pic`s (dust, landings, strikes, towers'
rocks) are started as the effects' starters. The magic squares 1 and 2 and
the lights' modes 1 and 2 are piney-effect's `boss`, with `ccSetColor`'s
HSV and the volume's float conversion. A boss's (and a gomora's)
`Out::SkillStart` is `effSkillStart(who, sid, 0, 0)`. Beside the game's
order: the pictures' own draws of `ccRand` (a mode-1 light's) come in the
effects' pass before the boss's, and a storm's magic square is made after
its `Draw`, its first `Draw` a frame later.

## Checks

`tools/test_battle_fidchell_rs.py` (`PINEY_VOLUME=outbreak`) builds the
game's `ccBoss04` and runs the base's `Main` natively against
battle_probe's `fidchell`, the three spells' effects natively too: 40
cases (125,426 frames) agree on the acts, movement, place, target, HP,
gauge, flags, clips, every own member, the camera's range, the effects'
slots, the party, the calls, the menu, `rand()` and `ccRand`'s index; acts
0, 3-7, 12-14, 16, 18, 22, 24 and 25 and all four spells are reached.
`test_effect_lives` runs the stand-ins' `Create`s and `Draw`s natively
(nine lives). `boss::fidchell::tests` hold the constructor, the prediction
and its spell, each spell, the Super table and the drain to death. Under
the story pilot (`outbreak_story_survey`, event 207, 120,000 frames with
`PINEY_SURVEY_GOD`) the event ends done.

`tools/test_boss_effect_rs.py`'s `test_fidchells_spells` (Outbreak's disc)
runs the game's `ccBossEffMeteoSwormCreate` (50 meteors),
`ccBossEffThunderStormCreate` (16 bolts), `ccBossEffRockTowerCreate` (128
towers) and `ccBossEffMagicSquareCreate` 1 (fields 4 and 2) and 2, then
the manager's pass, against `boss_probe` (whose manager runs the rules
itself): meteo 192, storm 312, tower 436, square 1 288 and square 2 184
frames agree on every draw, sound, generator, light (the swarm's with its
intensity and fall-off), slot, `rand()` and `genrand` count. Skeith's
cases now pass on Outbreak too. The session test
`session::tests::fidchell::fidchell_spells_show` forces the prediction,
each spell and a skill in the arena of event 207 (block 9) and sees the
text drawn, the voice asked (-40), every spell's objects drawn, the skill's
start and its cinema name; `fidchell_shots` (ignored) writes the frames.

## Unknown

- The text's picture, the voice and the skills' names are not checked
  against the game's code frame by frame (the text's draw is the desktop
  `ccAnm`'s, checked there; the names' rows are the game's own lookup).
- `effSkillStart`'s controller's real end is not run with the boss; any
  end under 30 frames gives the same fight.
- Acts 20, 21 and 23 are reached by no pattern; the transfer is ported
  but unchecked.
