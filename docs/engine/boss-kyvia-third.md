---
title: Bosses - Kyvia's third fight
status: partial
volumes: QUA
covers: QUA gcmn.prg:0x003d0a60 ccThKyvia03 (bossFunc code 14), 0x003d0b30 ccBossKyvia03::ccBossKyvia03, 0x003d12f0 Action, 0x003d1df0 DeadKyviaVoice, 0x003d2080 DeadKyviaCam, 0x003d24d0 Main, 0x003d2870 Think, 0x003d29e0 SwitchActionPattern, 0x003d3010 SwitchDeadCheck, 0x003d31b0 CheckDiscMove, 0x003d3910 CheckDmgSmokeEff, 0x003d3be0 KyviaMagicDamage, 0x003d3c80 KyviaMagicalBallDamage, 0x003d3d90 SetBPos, 0x003d4280 HandAtk, 0x003d4ae0 LightAtk, 0x003d76e0 MegidFlame, 0x003d7d40 MagicBallAtk, 0x003d7fa0 ThunderboltAtk, 0x003d8d40 ThunderDmg, 0x003d9760 ThunderSound, 0x003d9920 Slave, 0x003d99f0 EntrySlave, 0x0036b4e0 ccBossEffMagicalBall::InitData, 0x0036b880 Draw, 0x0036c170 DrawPlasma, 0x0036c630 SetSpPoint, 0x0036cbc0 Spline, 0x0037ca70 ccBossEffMagicalBallCreate, 0x003ed8b0 kyviaGomora::GomoraInit, 0x003f0120 ListStepUp, 0x005103e0 the third level's gomora lists, 0x00510030 Kyvia03AnmTbl, 0x003657a0 ccThBossEffect, 0x00365ac0 ccBossEffManager::Draw, 0x0015eaa0 ccScFade::SendPacket
worklog: 433
---

# Bosses - Kyvia's third fight

`bossFunc` code 14 is Kyvia's third fight: `ccBossKyvia03` (kyvia03.cpp).
Quarantine's event 308 enters it on field 11's disc (`entry type=7
code=14`, then `if absent type=7 code=14`). It is [the second
fight](boss-kyvia-second.md)'s code with a third stage of the disc, the
core at level 3, other numbers, and a fifth attack, the magical ball.
Names and layouts are Infection's DWARF (Infection's executable carries
kyvia03.cpp, unused); the rules are Quarantine's code, which names none of
the class's functions. Their addresses come from the vtable at 0x00277f00
(its constructor's) and from each function's calls in order against
Mutation's named copy.

## Where it comes from

`bossFunc[14]` (0x003d0a60) makes `new ccBossKyvia03(3)` (0x29740 bytes)
and runs its `Main` (vtable +0x1c, 0x003d24d0) until `CheckExit`.
Quarantine's layout is Infection's with the cinema's byte at +0x29350, the
rest 0x10 on, as Outbreak's second fight. Against the second fight's,
`ThunderGp` stands before `MaxHP`, so `MaxHP`, `SwitchHp`, `HpProccess`
and `DmgAnmFlg` are 4 on, and the constructor clears a word DWARF leaves
out at +0x293f4: the last stage's count (below).

```text
class           size     rows (bossTbl)
ccBossKyvia03   0x29740  13 (the constructor's ccGetBossParam(13), as the second's)
kyviaCore       0x295a0  36/37 by attribute at level 3
kyviaGomora     0x295f0  by attribute at level 3
```

## What differs from the second fight

`Main`, `SwitchDeadCheck`, `SetBPos`, `Slave`, `EntrySlave` and
`CheckDmgSmokeEff` are the second fight's code.

- **The constructor**: file `x03`, clip `ANM_ex03nut0`, the path's dummy
  `OBJ_dummy4_4`, `DiscMaxLV` 3, the pitch 0.25, `CoreInit(3)`,
  `SwitchHp` a fifth of the core's HP (and a fifth again at each rise).
  It sets `transDist` (+0x90) 0, which the boss's own `Draw` never reads.
- **Three stages.** `CheckDiscMove` is the second's; its ride's camera
  looks 150 over the disc, as the first's.
- **The attacks** (`SwitchActionPattern`) by `AtkPatMode` and stage:

```text
mode  stage 1           stage 2           stage 3                         next
1     arm (cinema 36)   arm (36)          thunder kind 1 (48)             2
2     arm (36)          thunder 1 (48)    beams (37)                      3
3     thunder 1 (48)    thunder 2 (48)    meteors (53)                    1; at stage 3, 4 while the core's HP < 0.7 MaxHP
4     magical ball (41), act 20                                           2
```

  At the last stage's third mode the turn waits for the menu
  (`CheckMenuType`), the core's `kAtkFlg` kept; at its 30th frame there
  the player is held.
- **The arm**: the camera at 6000, 3000 and 4500 by stage, boss skill 76.
- **The beams**: the point looked at 1300 up, the eye's turn
  `ccRandF(0.2)`; while it swings the point sinks 8 a frame and the eye
  rises 12 (eased to 2500, as the first's), and at its 30th frame, the
  count taken before it is raised, the point rises 180 with the beams'
  start; the quake (20, 30, 45) as they land.
- **The meteors**: the player held from the act's first frame, stream
  108, no quake at 60, skill 38.
- **The thunder** (kinds 1 and 2 only): the members' bolts as the
  second's; the disc's

```text
kind  frames  disc's bolts           points' spread        angle
0     90      4 over 500, 1-4 wide    (0, 0, 50, 0.8)        pi (no smoke)
1     90      10 over 400, 2-6 wide   (100, 100, 50, 0.2)    pi
2     120     15 over 350, 3-12 wide  (50, 50, 50, 1)        pi
```

  The first step sets these each frame it waits; the storm's camera is
  3500 off turned 0.6 over the disc, looking at it, and its free camera
  8 back and 4 down, the view 5 up; a quake (10, 10, 20) at the 30th
  frame of every kind; the thunder (SE 41) at the build-up's 50th frame,
  with the second and fourth strikes and the last; skills 47 and 48 for
  45 and 46, the strike's sound 68 twice in every kind; the last also SE
  56 at 60.
- **Death**: the smoke 1250 up and 720 back at x 0 (no quake, no blur),
  sinking 0.55 more a frame from 135 and rising 3; the first camera at 60,
  650 back; the cries not placed (`ccSeOnNote`).

## The magical ball

`MagicBallAtk` (act 20, 0x003d7d40): once the core rests it goes (state
8) and the player is held; at its state 6 the camera out to 5500, the
blur's colour 0x60808080, SE 191; once the camera is there
`ccBossEffMagicalBallCreate(40, DiscPos, blur, KyviaMagicalBallDamage)`
and the quake vector cleared; then the quake each frame until the ball's
callback moves the act on; the colour back once the camera is; the core
back and the act's end.

`ccBossEffMagicalBall` (0xd0 bytes, bosseff.cpp):

- `InitData`: xeffect's `ANM_ex31exp3` stepped once, particle's
  `EFF_x007` (never drawn), seven plasmas. `SetSpPoint` gives each three
  points about the ball, turned by `ccRandF(pi)` about x and y: 150 +
  |r| out and 350 + |r| down, then 750 down, then `MaxScale` times -17.5
  (`r` a `ccRandF(100)`), and three `RndSBpoint`s it never reads; each
  plasma's `time` is `-|ccRandF(2)|`.
- `EffMode` 1: the ball grows 1.5 a frame (the drawn scale a frame
  behind), SE 61 and 246 at first; past 40, `MissileSmokeGenerator` rows
  12 and 10 given lives 150 and 120 (written into the rows), row 12 twice
  at the ball, and each plasma's two ends (row 10).
- 2: 150 frames; before the 120th, SE 229 each sixth frame and
  `DrawPlasma`; row 14 at the 120th.
- `DrawPlasma`: each plasma runs three Catmull-Rom legs from its third
  point in to the centre, a fifth a frame; past the third, from every
  third plasma but the first, the callback's 0 at the ball. Its ends are
  the leg's point turned by `Rot` and the same turned over by pi about x.
  `Rot` turns 0.03 a frame.
- 3: at 20 SE 56, the callback's 3, the blur 1.06 at 0x60808080, a flash,
  rows 15 (twice) and 16; at 30 the callback's 1, SE 56 and 35; from 21
  `EffAnm` fades 0.05 a frame and the blur's scale falls from 1.059 by
  0.001, then its colour's alpha by 2 a frame; at 81 the blur back
  (0x30808080, 1.0), the callback's 2, and the ball's end, its last fade
  written after.

`KyviaMagicalBallDamage(pos, flg)`: 0 and 1 the core's boss skills 72 and
73 round `pos`; 2 the attack on, the core's state 7, the camera back (mode
4); 3 the quake 30 each way.

The legs 0 and 4 take no spline: `DrawPlasma`'s local keeps what was last
there. A frame's first such plasma reads the stack: the ball's `Draw` is
called by the manager's `Draw` (0x00365ac0) from `ccThBossEffect`'s task,
and the quadword is the top of a frame under the manager's. Each frame
the manager's last call, the cinema's `Draw`, calls
`ccScFade::SendPacket` (0x0015eaa0), whose frame saves the task's `s7`
there (`sq s7, 112(sp)`); the task never changes `s7`, so it is 0. On the
frames `ccSeOnNote` runs first its return address is there, a float
nothing. Either way the point is no vector and the end stands at the ball.

## The core and the gomoras at level 3

`SetCoreBaseParam(3, attr)` takes rows 36 and 37, adding 1500 to both
rows' `maxHP` at each death. `GomoraInit(3)` and `ListStepUp` take the
third level's lists, which Mutation added to `ListStepUp` (0x005103e0 in
Quarantine; Infection's has no case 3). Their code is level 2's.

## The port

- piney-battle: `boss::kyvia` with `Fight::Third` (`kyvia::new(cx,
  fight)`), its numbers by fight, `third_pattern`, and
  `kyvia::magic_ball`: `magic_ball_atk`, `MagicalBall` (`new`, `draw` on
  a `BallHost`, `DrawPlasma`'s local zeroed each frame) and
  `ball_damage`. The manager's pass over Kyvia's effects is
  `Boss::effects_pass`, which piney-world runs before the blur's `Main`
  and the boss's `Main`; a camera mode an effect sets is the camera's
  before the boss reads it.
- piney-gen: `kyvia03_anims`, `kyvia_gomora_lists_3` (from
  `ListStepUp`'s fourth address build), `kyvia_gomora_lists_4`,
  `kyvia04_anims`, `kyvia04_anims_ex`, `kyvia_skills_various`.
- piney-world: code 14 (`KYVIA_THIRD`, `x03`'s `CMP_trallex03`).
- piney-effect: `kyvia::Ball` (the `EffAnm`), the plasmas' anchors, the
  rows' lives; `Ball::alone` for the probe.
- piney-game: Quarantine's story (301-314) for starts at 308 and 312.

## Checks

`PINEY_VOLUME=quarantine tools/test_battle_kyvia_rs.py` runs Quarantine's
`ccBossKyvia03(3)` natively against battle_probe's `kyvia 3` (the second
fight's harness, with Quarantine's unnamed functions by address), the
ball run natively by the manager's pass and its members compared each
frame. 40 cases (120,477 frames) agree, reaching the body's acts 0-6, 14
and 20 and the thunders of kinds 1 and 2. `test_kyvia_effects` in
tools/test_boss_effect_rs.py runs balls alone (no blur, no callback)
against boss_probe's `ball`: 1,036 frames of draws, sounds, flash and
generators agree. `session::tests::kyvia_third::kyvia_s_third_fight_to_its_end`
plays event 308 from field 11's disc: the god's party, the core hurried
and struck by Kite's blows (`EntryAffect`) every 300 frames while it can
be hit. Every attack comes over the three stages, the ball is drawn with
its plasmas' generators following them, the music plays, the body dies
and the event goes on to its end.

## Unknown

- Whether a retried fight meets the rows as the disc has them (as the
  second fight's).
