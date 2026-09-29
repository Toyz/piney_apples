---
title: Bosses - Kyvia's second fight
status: partial
volumes: OUT
covers: OUT gcmn.prg:0x00616d60 bossFunc (code 13), 0x004d5870 ccThKyvia02, 0x004d5940 ccBossKyvia02::ccBossKyvia02, 0x004d60f0 Action, 0x004d6b90 DeadKyviaVoice, 0x004d6eb0 DeadKyviaCam, 0x004d7300 Main, 0x004d76a0 Think, 0x004d7790 SwitchActionPattern, 0x004d7b00 SwitchDeadCheck, 0x004d7ca0 CheckDiscMove, 0x004d8410 CheckDmgSmokeEff, 0x004d86e0 KyviaMagicDamage, 0x004d8780 SetBPos, 0x004d8c70 HandAtk, 0x004d94a0 LightAtk, 0x004dc000 MegidFlame, 0x004dc6b0 ThunderboltAtk, 0x004dd3f0 ThunderDmg, 0x004ddd40 ThunderSound, 0x004ddf00 Slave, 0x004ddfd0 EntrySlave, 0x004d0780 ccBossKyvia01::Main, 0x00419c80 EVENTAREAB8::Move, 0x00419d10 EVENTAREAB8::IsMove, 0x00477410 ccBossEffThunderbolt::ccBossEffThunderbolt, 0x004776e0 UnitPoint, 0x00477820 SetBreakPoint, 0x004778b0 Draw, 0x00489540 ccBossEffThunderboltCreate, 0x004f5150 kyviaCore::SetCoreBaseParam, 0x004fa3e0 StepAllGomoraList, 0x004fafa0 kyviaGomora::GomoraInit, 0x004fd810 ListStepUp, 0x00619050 AllGomoraList_2, 0x00618c70 Kyvia02AnmTbl, 0x0061d0d0 KyviaGenerator
worklog: 304
---

# Bosses - Kyvia's second fight

`bossTbl` row 13 is Kyvia's second fight: `ccBossKyvia02` (kyvia02.cpp),
`bossFunc` code 13. The English table names rows 12 and 13 both "Cubia",
its name for Kyvia; the core rows are "Cubia Core". Outbreak's event 211
enters it on field 10's disc (`entry type=7 code=13`, then `absent type=7
code=13`). It is [the first fight](boss-kyvia.md)'s code with its own
numbers, a second stage of the disc and a fourth attack, the
thunderbolts.
Names and layouts are Infection's DWARF; the rules are Outbreak's code.

## Where it comes from

`bossFunc[13]` (0x004d5870) makes `new ccBossKyvia02(2)` (0x29740 bytes)
and runs its `Main` (vtable +0x1c, 0x004d7300) until `CheckExit`, as the
first fight's task does. Outbreak's layout is Infection's with one byte
more at +0x29350 (the cinema flag, Mutation's +0x295f8 in the first
fight): the three pointers after it move 4, the rest 0x10.

```text
class           size     rows (bossTbl)
ccBossKyvia02   0x29740  13 (6000 HP)
kyviaCore       0x295a0  34/35 by attribute at level 2 (2000 HP)
kyviaGomora     0x295f0  20-23 by attribute at level 2 (1400, 1200, 1200, 1300 HP)
```

## What differs from the first fight

Everything not in this list is the first fight's code (Main, Think's
switch but act 6, SwitchDeadCheck, SetBPos, Slave, EntrySlave and
DeadKyviaVoice are the same code; the core and the gomoras are the same
classes, at level 2).

- **The constructor**: file `x02`, clip `ANM_ex02nut0`, the path's dummy
  `OBJ_dummy3_3`, `DiscMaxLV` 2, `ExARGB` 0x70808080, `CoreInit(2)`,
  `SwitchHp` a fifth of the core's HP. At each rise `CheckDiscMove` reads
  the core's `MaxHP` again and makes `SwitchHp` a seventh.
- **Two stages.** After the core's first death `CheckDiscMove` counts 60
  frames: at 1 the quake (20, 20, 20), `ExARGB` and the player held; at
  20 the ride's camera; at 40 the colour 0x50808080; from 41 the quake and
  SE 56 each sixth frame; at 60 the player free, `DiscLV` 2,
  `EVENTAREAB8::Move`, `AtkPatMode` 1. The disc rides on and the core
  rises again. Its sounds are at the disc, and the ride's camera looks at
  the disc itself (the first fight's 150 over it).
- **The attacks** (`SwitchActionPattern`): 1 the arm (cinema 36); 2 the
  thunder of kind 0 on the first stage (46), else the beams (37); 3 the
  thunder of kind 1 on the first stage, back to 1, else the meteors (38);
  4 the thunder of kind 2, back to 2. Boss skills 36, 37, 38 for the
  first fight's 33, 32, 34.
- **The smokes** (`CheckDmgSmokeEff`) only on the last stage, 1000 up and
  300 back. `DmgGp[k]` past its four reads `DeadGp`, then `CamRotX`
  (0.26): a fifth smoke fills `DeadGp` and there is no sixth.
- **The numbers**: the hit's camera 650 back, 1100 up, the eye at -900;
  the arm's camera at 4500 (first stage) or 3500 and the first stage's
  quake (30, 40, 60), no SE 40 at the blow; the beams from 550 back and
  1200 up, tilted 0.35 and eased to 2300, the quake (50, 50, 100) as they
  land; the meteors with stream 73, 550 back and 1100 up, the eye at
  -2800, a quake (20, 50, 80) at 60. The death: the smoke at x -50, 630
  back and 1200 up, x 5 more a frame for 10 frames, the quake (30, 30, 20), the
  blur 1.03; its cameras at 70 (0x50808080, 1.02, 350 back, 1000 up, the
  eye at -900, turned `|ccRandF(1)|`) and 150 (500 up, the eye 2800 back
  and 1000 up, turned `-|ccRandF(1)|`).

## The thunderbolts

`ThunderboltAtk` (act 6): once the core and gomoras rest (`GetActNum`)
they fade out, the stage darkens (`BeginStageEffect(0x64000000, 15, 15,
32277)`), the player is held and the quake is (30, 30, 20). The members'
bolts (`Sdat`) are 3 strokes over 50, 2-4 wide, tilted 0.5 plus up to
0.9. By `ThunderType`:

```text
kind  frames  disc's bolts           the rest
0     90      3 over 100, 1-4 wide    a bolt on each member, smoke under it
1     90      4 over 550, 2-6 wide    a bolt on each member
2     120     15 over 450, 3-12 wide  the camera 3800 overhead (mode 5)
```

The build-up (`ThunderSound`, raising `actCount`) flashes at 10, 25, 30
and 50 and at 80 lets the bolts fall; kind 2's camera is the active
one's place moved a little, looking 10 higher. The strike (`ThunderDmg`) deals boss skill 45 or 46 round
the body five times and at `ThunderTime` (with flashes, SE 68 and 39),
the quake by kind and at half-time, a thunder note each fifth frame
(`ccRand() % 20`). 30 frames on, the core fades back (mode 6 for kind 2,
4 otherwise), the stage lightens over 35 frames.

A bolt (`ccBossEffThunderbolt`) has `Num` strokes: each from a point
within the range (`UnitPoint`: turned `ccRandF(pi)`, `range -
ccRandF(range / 10)` out), 10-17 segments of `CMP_x012` (`SetBreakPoint`:
`abs(ccRand() % 8) + 10`), each 200 long, turned `ccRandF(pi)` about z and
`DefaultAngle + |ccRandF(RandAngle)|` about x, drawn anew every frame: a
stroke draws `ccRand()` for its width, two `ccRandF(pi)` it never uses,
the turn and three `ccRandF(RndPoint)` for its start. The disc's bolts
are raised 500 (the offset the caller writes after `Create`) with a smoke
on their first draw; the effect ends after `Time + 1` draws.

## The core at level 2

`SetCoreBaseParam(2, attr)` takes rows 34 and 35; each call while the
core is dead (`DeadFlg`, its `ResetData`) adds 3000 to both rows' `maxHP`
in the table, for good, so the second stage's core has 5000 HP whichever
attribute it takes. The gomoras take `AllGomoraList_2[0]` (3 3 3 - -, 3 3
3 - -, 0 3 3 3 -) and after the core's death `ListStepUp(1)` gives
`AllGomoraList_2[1]` (1 3 3 3 -, 0 3 3 3 -, 0 0 3 3 1); level 2's rows
and attack times are the first fight's code.

## The port

`piney_battle::boss::kyvia` with `Fight::Second` (`kyvia::new(cx,
fight)`), `kyvia::thunder` (the attack and `Bolt`, whose `Draw` the
manager's pass runs for its `ccRand` draws), `KyviaData::gomora_lists_of`
(`kyvia_gomora_lists_2`, `kyvia02_anims` in the combat group) and
`Core::row_hp`. `piney_world::combat::boss` starts code 13 with `x02`'s
`CMP_trallex02`, `CMP_trall2` and `CMP_ex01gom2`. The bolts have no
picture yet (`Bolt::segments` holds each frame's).

## Checks

`PINEY_VOLUME=outbreak tools/test_battle_kyvia_rs.py` runs Outbreak's
`ccBossKyvia02(2)` natively against battle_probe's `kyvia 2`, as the first
fight's harness does (its "Checks" in [the first fight](boss-kyvia.md)),
with Outbreak's unnamed functions by address, the disc riding on for a
case's frames after each `Move`, `bossTbl` restored for each case, the
thunderbolts run natively (their clump stubbed) and compared by slot,
`EntryFlash`, `ccSeOnNote` and the stage fader's calls compared, and the
second fight's members and thunder compared each frame. 130 cases
(405,579 frames) agree, reaching the body's acts 0-6 and 14, the three
thunders, the core's two deaths. `PINEY_KYVIA_FIGHT=1` runs Outbreak's
copy of the first fight: it agrees with the port as Mutation's does.
`piney-battle`'s `boss::kyvia::tests` hold the second fight's make-up, its
first stage's thunder and a bolt's life. `outbreak_story_survey` plays
event 211 through (`PINEY_SURVEY_GOD`): the first stage's core down near
frame 16,500, the second's near 22,000, the event done.

## Unknown

- Kind 2's camera: `cameraGetPos`, `cameraGetRot` and `cameraGetView` of
  `camID` are taken as the boss camera's; the harness answers all three
  from the case.
- The bolts' picture and the particles are named, not drawn.
- Whether a retried fight meets the rows as the disc has them: the game
  keeps the core's added `maxHP` in `bossTbl`, the port only for the
  fight (`Core::row_hp`).
