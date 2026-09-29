---
title: Bosses - Gorre
status: partial
volumes: OUT, INF
covers: OUT gcmn.prg:0x004a9130-0x004b0b50 (boss05.cpp), 0x004a9130 ccBoss05::ccBoss05, 0x004a9650 CalcRealEx, 0x004a99a0 ExecPatternIndex, 0x004aa100 IsValidArea, 0x004aa210 Main, 0x004aa5f0 Move, 0x004aa6b0 ChangeAction, 0x004aa760 Think, 0x004aa8e0 OnThinkNeutral, 0x004aaa40 OnThinkDrain, 0x004aaaf0 OnThinkDead, 0x004aaf40 OnThinkEscape, 0x004ab210 OnThinkDmg, 0x004ab2e0 OnThinkKerse, 0x004aba30 OnThinkWander, 0x004abb90 OnThinkMagic, 0x004ac060 OnThinkDataDrainAtk, 0x004ac320 OnThinkDashCenter, 0x004ac5d0 OnThinkTornade, 0x004ac850 OnThinkSkill, 0x004acdf0 OnThinkTalk, 0x004ad0c0 OnThinkWave, 0x004ad320 OnThinkChase, 0x004ad5e0 OnThinkEpitaph, 0x004ad740 OnThinkEpitaphWave, 0x004ad980 Slave, 0x004ada60 EntrySlave, 0x004adbb0 SendMessage, 0x004ae990 ccBoss05Brother::Init, 0x004aed70 Move, 0x004aefb0 Main, 0x004af020 Think, 0x004af0e0 OnThinkNeutral, 0x004af1f0 OnThinkEpitaph, 0x004af300 OnThinkEpitaphWave, 0x004af8d0 OnThinkTornade, 0x004afe10 OnThinkWave, 0x004b0490 OnThinkDead, 0x004b05b0 Order, 0x004b07c0 Affect, 0x004b0b00 OnExit, 0x006dfd90 Think's jump table (@1462), 0x006dfdf0 OnThinkKerse's (@1644), 0x006dfe10 SendMessage's (@2268), 0x00618040 boss05NormalActTbl, 0x006180e0 boss05SuperActTbl, 0x006181b0 boss05EpitaphActTbl, 0x00618218 @1261, 0x00618230 @1672, 0x00618240 @1777, 0x00618250 @2074, 0x00384e80 ccBoss05's vtable, 0x00384de0 ccBoss05Brother's, 0x004700a0 ccBoss::OffBossCamera, 0x0046f9e0 ccBoss::InitBossCamera, 0x00472090 ccBossCam::ccBossCam; INF gcmn.prg:0x00497010-0x0049e590 (boss05.cpp), 0x005ebf10 Boss05AnmTbl, 0x005ebf80 boss05SlaveAnmTbl1, 0x005ebff0 boss05SlaveAnmTbl2, 0x005ec060 boss05NormalActTbl, 0x005ec100 boss05SuperActTbl, 0x005ec1c0 boss05EpitaphActTbl, 0x005ec228 @1261, 0x005ec240 @1672, 0x005ec250 @1777, 0x005ec260 @2074
worklog: 296, 297, 303
---

# Bosses - Gorre

Gorre (`ccBoss05`, boss05.cpp) is Outbreak's second phase boss: `bossTbl`
row 4, `bossFunc` code 4, file `x51`. It fights beside two
`ccBoss05Brother`s, each its own character (`bossTbl` rows 40 and 41) run
from Gorre's frame the way Kyvia runs its core and gomoras (each a
[`Boss`](boss-kyvia.md) of its own, taken out of its character for the
frame and put back). Gorre itself is never on the command lists and never
struck; the brothers are, and the three share one HP through
`SendMessage`. The names and layouts are Infection's DWARF (Infection's
gcmn carries the class, never entered; Outbreak's offsets match); the
behaviour is Outbreak's code.

## Where it comes from

Event 218, field 5: `entry type=7 code=4`.

## The classes

```text
ccBoss05 : ccBoss (0x29350)               size 0x29410
+0x29350 ccBoss05Brother *m_brotherP, *m_brotherM   (br0, br1)
+0x29364 m_effDead   +0x29368 m_effStart  +0x2936c m_skillID
+0x293b0 m_effAdrs   +0x293b4 m_stageEff
+0x293d0 m_transparency (the Kerse's fade)
+0x293e0 m_talkPos, +0x293f0 m_talkDirc (where the Kerse shows it)
+0x29400 m_actSubCount, m_actSubProccess, +0x29408 m_patMode

ccBoss05Brother : ccBoss                  size 0x294e0
+0x29350 m_parent            +0x29354 m_effDead
+0x293c0 m_moveSpdB  +0x293d0 m_moveVectorB  +0x293e0 m_moveDircB
+0x29400 m_posB (the place off Gorre's)   +0x29430 m_prevPosB
+0x29410 m_invOffset, +0x294d4 m_bTransInvOffset (msg 1's; never sent)
+0x29440 m_bTornadeGenFlag   +0x29444 m_sendMsgFlag[8]
+0x29464 m_fRadAccel, m_fRadSpd, m_fRad, m_fRadius
+0x29474 anmw (the wave's own clip)       +0x294d0 m_bWaveEnd
```

## The constructor

`ccBoss05::ccBoss05`: `SetBaseParam(ccGetBossParam(4))`, `Boss05AnmTbl`;
Kite's place less 500 on y; no body hit; `cheatHP` on. No `ccEntryCmnd`:
Gorre stays off the command lists (nothing targets or affects it). Two
brothers `new[]`'d, each `Init`'d 300 either side on x (`m_posB`, a plain
world-space addend: the place is Gorre's plus it, no heading turn), its
own row (40 or 41), `boss05SlaveAnmTbl1`/`2`, `anmw` on `ANM_xx11wave`,
`ccEntryCmnd`. `EntrySlave(0, 0)` twice (`OffExit`, `OnDraw`, `Order(0)`).
`InitBossCamera(200, 1500)`: the `ccBossCam` constructor's `MaxRenge`
(+0xd8, 1000.0) is left as it is (Fidchell's constructor alone sets its
own, 600, after). `ChangeAction(0)`, `SetPatternMode(0)`.

## A frame

`ccBoss05::Main`: `CalcRealEx(0)` (the base's `CalcReal`, then the
protect gauge's picture on both brothers), `CalcTargetInfo`, the centre,
the stop's count, `hold` set while either brother's is, `Think`, the party
held while `lockPlayer` (and a broken protect's count run on), `Move` (no
body hit), the broken protect's count handed to both brothers, `Slave`
(each brother's `m_dircB` toward Gorre, then its `Main`), `hold` cleared.
No `PreDrawAnm` and no `Draw`: nothing in Outbreak calls
`ccBoss05::PreDrawAnm` (nor does its task, `ccThBoss05`: `Main`, then
`CheckExit`), so Gorre's own body is neither animated nor drawn except in
the Kerse's step 2, which forwards its clip and draws it at `m_talkPos`
(`gorre::body`). The two brothers are what is seen of the fight.

`ccBoss05::ChangeAction`: the base's, then `Order(act)` on both brothers
(whether or not the base took it), `m_actSub*` cleared.

`Think` by act (`@1462`; 9, 10, 17 and 19 the table's trap):

| act | function | act | function |
| --- | --- | --- | --- |
| 0, 15 | `OnThinkNeutral` | 12 | `OnThinkEpitaph` |
| 1, 2 | `OnThinkDmg` | 13 | `OnThinkEpitaphWave` |
| 3 | `OnThinkKerse` | 14 | `OnThinkDead` |
| 4 | `OnThinkWave` | 16 | `OnThinkEscape` |
| 5 | `OnThinkTalk` | 18 | `OnThinkChase` |
| 6 | `OnThinkDataDrainAtk` | 20 | `OnThinkSkill` |
| 7 | `OnThinkDashCenter` | 21 | `OnThinkTornade` |
| 8 | `OnThinkWander` | 22 | `OnThinkMagic` |
| 11 | `OnThinkDrain` | | |

## The patterns

`ExecPatternIndex`; 2-8 and 12 fall to the base's, whose `ChangeAction`
is Gorre's through the vtable (the brothers ordered too):

| word | operands | does |
| --- | --- | --- |
| 1 | | the Wave: act 13 once `epitaph`, else 4 |
| 9 | `type`, `sid` | a target (a negative type: a member standing, by `abs(ccRand()) % n` after a dropped draw; else `SelectTarget(type)`, then type 3); the second brother casts `sid` on it; wait 60 |
| 10 | (skipped) | act 20, the Skill |
| 11 | `sid` | the second brother casts `sid` on the first, then on itself; wait 60 |
| 15, 16, 17, 18, 19 | | acts 3 Kerse, 5 Talk, 21 Tornade, 6 Data Drain, 22 Magic |

The tables (`boss05*ActTbl`, 39, 47 and 26 words) hold pattern 10's
operand as a -1, so each is its full size, not "up to the first -1".

## The acts

- Neutral / Epitaph: turn to the target, the wait, `SelectTarget(0)`, the
  next pattern.
- Wave (4, 13): both brothers untargetable and without a body; from frame
  115 the camera's quake (15) while in shake range, at 115 a flash and a
  `WaveShock` at each brother; the brothers' own Wave ends it (msg 2).
- Kerse (3), seven steps (`@1644`): a target (type 11), the party held,
  `SwitchLayer`, the stage fader, the cinema (19), camera 3 (1500 off the
  target at height 50, looking at it at 200), `m_talkPos` 500 the other
  way; the brothers fade out (a fifteenth a frame) and undrawn; Gorre's
  own clip runs, sounds at its frames 0 and 75, from 80 Gorre fades (a
  thirtieth) into the `FinalPhotonFlash` at the target; 15 frames on the
  second brother's skill 19 there; once the flash is gone the brothers
  drawn and fading back; 15 on, the fader out; 15 more, camera 2, Gorre
  undrawn (`OffDraw`) and neutral (forbid 3).
- Talk (5) / Data Drain (6): a target (type 2 / 0), the party held, the
  cinema; at 45 the stream (71 / 72); once the menu shuts, the second
  brother's skill 20 on it and a flash (Talk) or `EntryAffect` 13 (Data
  Drain); the next pattern.
- Dash (7): the base's dash to `dashPos` at 50, a trail of both brothers,
  both held; within 200 or 121 frames on, the next pattern.
- Wander (8): a way drawn (`ccRandF(pi)`), 120 frames at 30.
- Escape (16): a quarter turn off the target's way, redrawn every 32
  frames, at 100 then 30; the next pattern at 300.
- Chase (18): at a target (type 7) at 100; within 300 or 60 frames on,
  the next pattern; out of the arena (`IsValidArea`: inside 2500 plus the
  centre's x and y of 0), a dash of 500.
- Skill (20): one of `@1777` (157, 158 the first brother's, 159, 160 the
  second's) by `ccRand() & 3`, started (`effSkillStart`) under the fader,
  camera 3 and the spell's cinema; once the start ends the caster's
  `ccItemSkillCompel`; 50 on the fader out; 15 more the next pattern.
- Tornade (21): a target (type 3), the party held, the brothers
  untargetable, the cinema (68), camera 3 1500 off the target; the
  brothers do the rest (msgs 5, 7, 6).
- Magic (22): one of `@1672` by `abs(ccRand()) & 3`, the second brother's
  request at 15; 60 on the brothers back; 15 more, the next pattern.
- Drain (11): act 12 and the Epitaph table, `m_patMode` 2.
- Dead (14): no body; both brothers ordered dead and set 300 either side
  across the way to the centre; camera 3 3000 off Gorre at height 100,
  looking at it 500 up. The brothers' own deaths end the fight (msg 4).

## Brother

`Order(on)`: 0 and 15 a neutral (forbid 1); 3 and 5 act 15, a watch; 4
`m_bWaveEnd` cleared and `anmw` on the wave's clip first; anything else
that act, forbid 3. `Think` has cases for 0, 4, 12, 13, 14 and 21 only:
in any other act (Gorre's own, ordered along) a brother does nothing.

`Move`: `m_posB` stepped by `m_moveSpdB` along `m_moveDircB`, moved by
`m_moveVectorB` (always (0, 0, 0, 1)), the place Gorre's plus `m_posB`,
the base's speed on top.

- Neutral (0) / Epitaph (12): stopped or held, msg 3; else a turn toward
  Gorre and `m_posB` turned 0x3c8efa35 around Z (the Epitaph's turns
  first whatever holds it).
- Wave (4) / EpitaphWave (13): untargetable, no body; out at 50 past 500;
  30 frames still (a sound, the Wave's only); half a turn round Gorre at 5
  degrees a frame from `m_prevPosB` (a trail every fourth frame, or every
  sixth for the Epitaph's); the strike's clip (`ANM_ex51act2`/
  `ANM_ex52act2` by id, the Epitaph's `ANM_ex5xact`): sounds at its
  frames 15 and 35, from 40 the wave drawn (`anmw` forward, then at double
  speed) and skill 18 at 55; both clips over, the act's clip; 15 frames
  on, home at 50 to within 300, msg 2, targetable (and, the Wave's only,
  the body back and a turn to its way each frame).
- Tornade (21): facing Gorre; from 15 frames on a fade out, msg 5 at 0;
  from 15 on a fade back; then 211 frames of `m_posB` turned by `m_fRad`
  (from half a degree, quickening by `m_fRadAccel` to a quarter turn), a
  whoosh on 26 fixed frames, msg 7 at 120, msg 6 after.
- Dead (14): the dead effect at its place, off the lists; once the
  effect is out of the manager, msg 4 and `OnExit` (out, off the lists, no
  body).
- `Affect`: msg 0 (a drain only before its own Epitaph), then 5/6 `stop`,
  and a hit's shield (row 40 against `ccSkillCheckType` 0, row 41 against
  1), number and mark.

## SendMessage

`SendMessage(br, msg)`, by `msg` (`@2268`); 2, 4, 5, 6 and 7 act only
once both brothers' `m_sendMsgFlag[msg]` are set, and clear both (6
leaves them set: the next Tornade's first report ends it at once):

| msg | from | does |
| --- | --- | --- |
| 0 | `Affect` | by the brother's affect type (below) |
| 1 | nothing | the other brother pulled by the first's push (`m_invOffset`) |
| 2 | the Wave home | the party freed, the brothers targetable, the next pattern |
| 3 | a stopped or held brother | the other held too (`EntryAffect` 5) |
| 4 | a brother's death | Gorre's `OnExit` |
| 5 | the Tornade faded | Gorre at the target, camera 3 (1500 off it, 100 up, looking 200 up) |
| 6 | the Tornade over | camera 2, the party freed, the cinema off, the next pattern |
| 7 | 120 into the spin | the second brother's `ccItemSkillRequest` on the target, one of `@2074` by `ccRand() & 3` |

Msg 0, by type: 1 and 3 (a hit) take the three's one HP - under `cheatHP`
a tenth, never under half, and the broken protect shared (the struck
brother's count is Gorre's, a fresh break of 300 breaks the other's, its
gauge Gorre's and the other's); at 0 the boss camera off
(`OffBossCamera`), the party's conditions cleared and the party held, the
brothers untargetable, the music out and Gorre's Dead; else, if Gorre's
own last affect type is 1 (never: nothing reaches Gorre), a hit act. A
hit with the gauge at half its maximum turns to the Super table. 7 and 9
heal the three. 13 is the Drain (`ChangeAction(11, 2)`, `cheatHP` off).
21 gives both brothers row 4's stats and Gorre's own `param`, the three
at 6000 HP, their gauges -1.

## Tables

`tables::combat`: `gorre_normal`, `gorre_super`, `gorre_epitaph` (each
its full size), `gorre_anims`, `gorre_brother_anims`/`gorre_brother2_anims`,
`gorre_skills` (`@1777`), `gorre_magic_skills` (`@1672`),
`gorre_tornade_skills` (`@2074`). `@1261` (157, 158, 160) is read only by
Infection's pattern 10; Outbreak's skips it.

## Checks

`tools/test_battle_gorre_rs.py` (`battle_probe`'s `gorre`) runs
`ccBoss05::ccBoss05` and `Main` (each brother's `Main` under it) natively
against the port, frame by frame, with scripted affects on Gorre and on
both brothers (hits, drains, heals, the drain's 21), the menu and the
camera; compared: Gorre's and each brother's generic fields, `m_posB`,
the gauges through each character's own `param`, the lists, `MaxRenge`,
the effects alive, the party, the calls, the menu, both generators. 0
mismatches over `bulk 100` from 9900, 20000 and 40000 (every act but 1,
2, 8, 11 (within a frame) and 15; 1 and 2 are unreachable, see above).
`outbreak_story_survey` finishes event 218 (`PINEY_SURVEY_GOD=1`,
150,000 frames): the shared gauge broken, a brother drained, the three
down.

## Unknown

- `effResistantShield`'s own picture: the brother's shield is shown
  (`Out::Shield`), its vector tweak (x halved, z times 1.3 plus 200) not.
- After msg 0's 21 the brothers share Gorre's `param` in the game; the
  port writes the same values to all three but keeps three: a later change
  to one gauge (none happens in the fight once it is -1) would not reach
  the others.
- `ccBossEffFinalPhotonFlashCreate`'s picture (its life, 72, is measured).
