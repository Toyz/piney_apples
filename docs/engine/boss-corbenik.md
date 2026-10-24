---
title: Bosses - Corbenik
status: partial
volumes: QUA, INF
covers: QUA gcmn.prg:0x003ac9f0-0x003b1a40 (boss08.cpp), 0x003aca30 ccThBoss08, 0x003acbd0 ccBoss08::ccBoss08, 0x003ad000 Main, 0x003ad030 Slave, 0x003ad0d0 Affect, 0x003ad460 Think, 0x003ad5d0 PreDrawAnm, 0x003ad9d0 SendMessage, 0x003adae0 ExecPatternIndex, 0x003adc30 SetPatternMode, 0x003adce0 OnThinkNeutral, 0x003ade20 ChangeNextPattern, 0x003ade90 _calcJumpPos, 0x003adf80 WaveFuncCB, 0x003adff0 OnThinkWave, 0x003ae780 OnThinkEscape, 0x003aea40 OnThinkSeed, 0x003aed10 OnThinkFlash, 0x003af090 OnThinkPollen, 0x003af190 OnThinkBodyPress, 0x003af4b0 OnThinkDie, 0x003af990 OnThinkMagic, 0x003aff40 _ChangeVibeType, 0x003affd0 _BeginBlink, 0x003b00b0 ccBoss08Seed::ccBoss08Seed, 0x003b03b0 ccBoss08Seed::Affect, 0x003b06a0 ccBoss08Seed::Main, 0x003b09f0 ccBoss08Seed::PreDrawAnm, 0x003b0ac0 ccBoss08Seed::Think, 0x003b0b50 ccBoss08Seed::Order, 0x003b0f40 OnThinkFly, 0x003b1440 OnExit, 0x003b1500 OnThinkCountDown, 0x003b17b0 OnThinkFire, 0x003b1970 OnThinkDead, 0x00376ea0 ccBossEffFlush::Draw, 0x00377c80 ccBossEffWave::Draw, 0x0037dd60 ccBossEffWaveCreate, 0x0037dec0 ccBossEffFlushCreate, 0x00360120 ccBoss::Move, 0x005ec630 boss08AnmTbl, 0x005ec690 boss08SeedAnmTbl, 0x005ec6e0 boss08ActNormalTbl, 0x003b1a50 boss08BBlurParamCB, 0x003b1b90 ccThBoss08b_Main, 0x003b1c50 ccBoss08_b::ccBoss08_b, 0x003b2610 ccBoss08_b::Main, 0x003b2a00 Slave, 0x003b2ae0 Affect, 0x003b3020 Think, 0x003b3200 ChangeNextPattern, 0x003b32f0 SetPatternMode, 0x003b33c0 ExecPatternIndex, 0x003b3600 PreDrawAnm, 0x003b3820 IsValidArea, 0x003b38a0 OnThinkNeutral, 0x003b3a40 OnThinkEscape, 0x003b3d00 OnThinkSybly, 0x003b4160 OnThinkWave, 0x003b4500 OnThinkStarDust, 0x003b4cd0 OnThinkBodyPress, 0x003b5b80 the barrier's message, 0x003b5cf0 the death, 0x003b5f60 OnThinkMagic, 0x003b6340 OnThinkSetupBarrier, 0x003b6750 the return, 0x003b6960 OnThinkRandomDrive, 0x003b6f50 _EntryElement, 0x003b7140 _DrawElement, 0x003b74b0 _InitElement, 0x003b7560 OnThinkRootDrain, 0x003b8910 SetupRoot, 0x003b8b00 EntryRoot, 0x003b8c10 ccBoss08Root::ccBoss08Root, 0x003b8e00 ccBoss08Root::Order, 0x003b8e80 ccBoss08Root::Think, 0x003b8fb0 ccBoss08Root::PreDrawAnm, 0x00372e10 ccBossEffTransform, 0x00374bb0 ccBossEffStarDustFlash, 0x00375c60 ccBossEffThunder3, 0x00378600 ccBossEffDrain::Draw, 0x00379600 ccBossEffLaserRain, 0x005ec730 boss08BNormalActTbl, 0x005ec790 boss08BSuperActTbl, 0x005ec7e0 boss08bAnmTbl, 0x005ec840 boss08RootAnmTblA, 0x005ec890 boss08RootAnmTblB, 0x003781e8 boss08DmgBarriar, 0x003b9070 ccThBoss08c_Main, 0x003b9140 _RefrectLaserCB, 0x003b9250 ccBoss08_c::ccBoss08_c, 0x003b9810 Affect, 0x003b99d0 PreDrawAnm, 0x003b9a50 Think, 0x003b9b90 SetPatternMode, 0x003b9c50 ExecPatternIndex, 0x003b9ff0 OnThinkNeutral, 0x003ba210 the death, 0x003ba9c0 OnThinkDataDrain, 0x003bad50 OnThinkEntryEye, 0x003bb460 OnThinkWave, 0x003bb750 OnThinkTeraLaser, 0x003bc430 OnThinkSlaveAttack, 0x003bc6c0 OnThinkWander, 0x003bc900 OnThinkLaser, 0x003bd6c0 ChangeAction, 0x003bd870 Slave, 0x003bd960 SetupSlave, 0x003bdbc0 EntryKiller, 0x003bdc50 CalcTeraLaserPos, 0x003bdf00 SetupRefrectLaserPos, 0x003be760 ResetRefrectLaserPos, 0x003be7c0 CalcLaserTurningPoints, 0x003be920 ccBoss08Eye, 0x003bf360 ccBoss08KillerEye, 0x00376f60 ccBossEffRefrectionLaser, 0x00377d40 ccBossEffEnergyGrow2, 0x003781e0 the tera laser's charge, 0x00378900 ccBossEffTeraLaser, 0x00360630 ccBoss::ExecPatternIndex, 0x00363d70 ccBossCam::SetMode, 0x0050fc70 boss08cAnmTbl, 0x0050fd20 the third form's first table, 0x0050fdd0 boss08CNormalActTbl
worklog: 450, 451, 452, 453
---

# Bosses - Corbenik

Corbenik (`ccBoss08`, boss08.cpp, boss08_b.cpp, boss08_c.cpp) is
Quarantine's last phase boss: `bossFunc` code 7, file `x81`, fought in
event 313 on field 8. The names and layouts are Infection's DWARF
(Infection's gcmn carries the classes, never entered); the behaviour is
Quarantine's code over the `ccBoss` base Outbreak's and Quarantine's
share ([Macha](boss-macha.md), [Tarvos](boss-tarvos.md)).

## The three forms

`ccThBoss08` (the boss task) reads `saveData` byte 0x652f. At 1 it plays
the third form (`ccThBoss08c_Main`: `ccBoss08_c`, its eye and its killer
eyes). Otherwise it makes the first form (`ccBoss08`, `bossTbl` row 7,
with its eight seeds), runs it until `CheckExit`, resets the menu, flashes
(`EntryFlash(30, 0x80e0ffff)`), deletes it and plays the second form
(`ccThBoss08b_Main`: `ccBoss08_b` and its roots) in the same task. Event
313's block 15 makes it with `eventStatus[0]` 1 (forms one and two),
block 19 with 2 (form three).

All three forms are ported in `crates/piney-battle/src/boss/corbenik/`:
the first in `first.rs` and `seed.rs`, the second in `second.rs` and
`stardust.rs`, the third in `third.rs`, `eye.rs` and `laser.rs`, and the
shared effects in `eff.rs`.

## The first form's class

```text
ccBoss08 : ccBoss (0x1732c0)
+0x29350 m_seedTbl[8] (ccBoss08Seed, 0x293d0 each)
+0x1731d0 m_effFlash   +0x1731d4 m_bossEff (the dead effect)
+0x1731d8 m_stageEff   +0x1731e4 m_dmgCount
+0x173260 m_jumpData (sp, ep, cp, time)
+0x1732a8 m_patMode    +0x1732ac m_deadFlag (never set)
+0x1732b0 m_pauseCntDownFlag (the seeds' counts held)
+0x1732b4 m_entrySeedNum
```

The constructor makes the eight seeds first (row 44, exited, off the
lists, act 0 held off with `ChangeAction(0, 3, 4)`), then the body: 500
behind Kite and 10 up, his heading, `OnCheatHP` (never read: `Affect`
takes no cheat), `ccEntryCmnd`, after-images (30), the stage effect,
`InitCenterPos`, boss effects (128). No boss camera and no blur:
`cmn_bossCam` stays 0, so the wave's quake never runs. `m_stageEff` is
left as the object has it (0 on a zeroed heap); it is read only after a
`BeginStageEffect` or the death's -1. `ccBoss08::cmn_thisPtr` is set for
the wave's callback.

## The patterns

`boss08ActNormalTbl`: `2 60, 3, 15 8, 2 150, 14, 2 150, 1, 15 8, 3, -1`.
`ExecPatternIndex` adds 1 (the wave, act 6), 14 (the magic, act 20, the
seeds' counts held) and 15 n (the seeds, act 21, `m_entrySeedNum` n, never
read); the rest are the base's (2 the wait, 3 the escape).
`ChangeNextPattern` lets the counts go on, then the base's.

## The acts

- **Neutral** (0): still; waits out `stopTime` (-1 for ever, -2 while its
  skill runs), then the next pattern at `SelectTarget(0)`.
- **Wave** (6): at the target, SE 228 and two `MagicSquareGenerator`
  generators (row 7, row 0), off the targets. 60 frames on, three leaps,
  each turning 2 pi / 3 (the first from the centre's way), each a cubic of
  `sp, cp, cp, ep` through `W2P`/`P2W` (1000 out, the control 500 out and
  1000 up, 0.04 a frame; SE 223, SE 224 at the top). Each landing makes a
  WaveShock and a `ccBossEffWave` (`ANM_xx11wave` at 512) whose callback
  hits with boss skill 27 at the clip's frame 8.
- **Escape** (16): 90 frames away from the target at 20, its way turned a
  quarter off the target every 32 frames, toward the centre outside the
  base's box.
- **Seed** (21): SE 51 (note 55) at clip frame 25; SE 61, 35 (note 65)
  and 256 at 50; from 50 every other frame the first exited seed thrown
  (`Order(3)`), the first throw with `g_seedShotSmokeGen` 500 up. The next
  pattern once three are thrown and the clip is over.
- **Magic** (20): at the target, off the lists with its seeds, the party
  locked, the cinema of skill 252, the dark stage (0x64000000, 15, 15,
  3000); camera 2 2500 out and 500 up from the target along the centre's
  way, looking 100 over it. Skill 252 on it 60 frames on; the stage back
  160 frames after, the view shaken from 95 (a `ccRandF(100, 50)` kick at
  95, 115, 135 and 155, times -0.9 a frame, 0 under 10); 15 frames on all
  back (camera 1) and the next pattern.
- **Die** (14): off the lists with its seeds, the party locked, the dead
  effect; camera 2 2000 out and 50 up from its place at height 200 (turned
  by its heading, the centre's), the active camera's type 3; the counts
  held. From 60 frames the eye moves 10 a frame across the centre's way;
  at 150 `EntryFlash3(100, 20, 1, 0x80ffffff)` (its element in
  `m_stageEff`), `panelStatus` 3, its command gone; at 250 menu 74's
  stream 106 (mask 512), the body hidden; once the menu closes, out and
  camera 1.
- The flash (3), the pollen (4) and the body press (5) are in `Think` but
  nothing enters them: no table holds their words, and the flash's one
  way in, `_ChangeVibeType(4)`, is never called (the constructor's
  `_ChangeVibeType(0)` is the only call). The blink, the vibe's scale
  (stays 1), the energy grow and the pollen's generator are theirs alone.
  The port leaves them out; `Think` does nothing in them.
- Any other act: `ChangeAction(0, 1, 4)`. The base's words no table
  holds reach acts 7, 12, 17, 18 and 20 (20 is the magic).

## Affect

Nothing while held off (`actForbid` 2) or the player is locked; 6 and 5
the stop; 7 and 9 a heal (`ccEntryFlyFontNum` 20); a blow (1, 3), unless
the party is out or the game is over: its number, SE 195, the mark, the
HP down whole. At 0: the counts held, each seed not exited to its death
(`ChangeAction(14, 2, 4)`), still, and the body to act 14. Otherwise
`m_dmgCount` up and a type-1 blow its damage act (never: every pattern
holds act changes off with forbid 3). 13 and 21 do nothing: no drain.

## The seeds

`ccBoss08Seed` (row 44) lives in its own `ccChar` (the port: a character
of its own, `Class::CorbenikSeed`). `Main` is the base's in another
order (the lock before the stop); `Move` is the base's (its body hit
pushed out, the push's height dropped).

- `Order(3)`, thrown: 500 over the body; `moveVector` 60 times
  `Rz(ccRandF(pi)) Ry(-(1.2566 - ccRandF(0.3142))) (1, 0, 0, 1)` (all
  four lanes); `rand() >> 3` gives the spins (`m_rotSpeed[0]` and `[2]`,
  bits 12-15 of `r >> 4` and `r >> 8`); back on (exit off, drawn, body on),
  HP full, opaque, act 3.
- Fly (3): falls 4 a frame, spins (`RAD2DEG`/`DEG2RAD`). Going down below
  -7 at 75 or under: put at 50, SE 65 (note 65), `ccHitCheckLM` 100 above
  to 100 below; no land (-1) and it fades out over 15 frames to `OnExit`;
  else spins and throw times 0.8 (the fall times -0.5), small ones
  zeroed; still (fall under 16) it lies down (heading 0, pi/2) and
  `Order(4)`.
- `Order(4)`: on the lists, `expCount` 150, blink 30, the charge
  (`g_energyChargeGenerator`), act 4.
- Count down (4): the count over it (`(expCount + 29) / 30`); unless the
  body holds the counts, a frame off, past 0 `Order(5)`; an after-image
  (`EntryAfterImageAnm`, colour 0x005050ff at 100, scale 3 to 9) each
  time its blink runs out, sooner as the count falls (`expCount * 30 /
  150`, at least 10).
- `Order(5)`: the charge deleted; once, act 5.
- Fire (5): `ccBossEffFlush(pos, 1000, 0)` (ten draws); then
  `effExplode3`, SE 35 (note 53), every other seed within boss skill 31's
  `targetRange` not yet set off is set off (`SendMessage`, exited ones
  too), off the targets and the body off; 15 frames on boss skill 31
  there and `OnExit`.
- Dead (14): off the lists, the body off, fades over 30 frames.
- `Affect`: the base's guards (21 past them); a blow (1, 3) its number,
  the mark, the HP down (a cheat would take a fifth, never under half; no
  seed has it), at 0 its conditions cleared and act 14; then a spell
  (`ccSkillCheckType` 1) shows `effResistantShield` (x a quarter, 100
  down). The damage act never comes (forbid 3). Other types: the base's.

## Checked against the game

`tools/test_battle_corbenik_rs.py` runs `ccBoss08`'s constructor and
`Main` natively, the WaveShock and the dead effect stand-ins, the wave
and the flush native, scripted blows on the body and on the seeds, the
land ending past a case's x. 46 cases (137,730 frames) agree, reaching
acts 0, 6, 7, 12, 14, 16, 17, 18, 20, 21 and the seeds' 0, 3, 4, 5, 14.

## The second form

`ccThBoss08b_Main` news `ccBoss08_b` (0xfa5e0) in the task the first form
left, after a frame's `Breath`; the first form's effects stay in the
manager. The constructor makes five roots first (`ccBoss08Root`, row 8,
each `ccRand() & 1` picking `boss08RootAnmTblA` or `B`, exited, at 512),
then the body: row 8, 500 behind Kite and 10 up, `OnCheatHP` (unread),
`CMP_trall2` with `ANM_xx11wave`, `ANM_ex8natc4` (the root drain's own
root), `OBJ_ex8bria` (the barrier) and `boss08DmgBarriar`
(`ANM_ex8bdmg0`), the boss camera (200, 1000), the blur on, sixteen
elements on the body's clip. Quarantine's class has three words more than
Infection's DWARF (+0x294b4-+0x294bc: the message's wait, shown, run out).

Its tables: `boss08BNormalActTbl` `2 60, 4, 1, 2 90, 17, 2 120, 16, 3, 18,
2 150, 4, 1, 3, 14, 2 150` and from half its HP `boss08BSuperActTbl` `2
60, 4, 1, 2 90, 17, 3, 18, 2 150, 4, 1, 3, 15, 2 150`. Its own words: 1
the wave (act 3), 14 the magic (6), 15 the star dust (5), 16 the press
(4), 17 the sybly (21), 18 the root drain (22), 19 the barrier (23). The
base's 4 (the chase, 18) has no `Think`: the next frame is the neutral.

- **Neutral** (0) turns to its target (256).
- **Wave** (3): `ANM_ex8act3` from its frame 1; SE 172 (note 63) at
  frames 7-46, 223 at 59, 224 at 101; at 105 a flash and a WaveShock, at
  110 boss skill 28; from 105 the quake (50) and the wave drawn; both clips
  over, the next pattern.
- **Sybly** (21): skill 157 compelled on every member up, under its cinema
  and camera.
- **Magic** (6): skill 244 at the target.
- **Star dust** (5): at `SelectTarget(3)` (the 3 its step switch leaves in
  `$a1`: the member with the least HP); the camera 2000 out, the blur
  eased; `ccBossEffStarDustFlash`, then boss skill 30 at the target.
- **Press** (4): to 500 past the target, vanished into sixteen images
  circling the target; sixteen elements shot across it every 8 frames to
  90, each that arrives boss skill 29 and a ring (195).
- **Root drain** (22): at `SelectTarget(1)` (the farthest), from 1500
  past it; the ground breaks, the five roots come up between it and the
  target every 8 frames (each a quake of 30 frames), the camera circles;
  then `ccBossEffRootDrain` at the target and boss skill 57 every fifth
  frame from 45 to 95 with its HP and SP drains at 100 (their
  `EntryAffect`s on the boss itself at once, inside the damage).
- **Barrier** (23, once the body is at a fifth of its HP): the transform
  (`ccBossEffTransform`, 272 draws), `bossTbl` row 48's stats (its
  `Exdefense` written 256 into the table) and the barrier up.
- **Return** (7, the base's word 6) and the **random drive** (20, the
  base's word 7): no table holds them.

Its HP never kills it. A blow on the barrier shows 0 and takes the
barrier's own count (the HP the body had when it was asked for): out, the
death (14); under 4000 the first time, menu 74's stream 94 (act 24) and a
3630-frame wait, after which the next pattern is the death. The drain (13)
sets a tenth of its HP and the gauge -1. The death hides the body and
exits once no menu is open.

The star dust (0x180) gathers 64 stars (`ccRandF` thrice each), raises
them, strikes the core with a bolt each (`ccBossEffThunder3`, 24 `ccRandF`
a draw), pulls them in (a ring each, `190 + ccRand() & 7`), bursts, drops
the camera from 3000 over the target and rains 32 lasers
(`ccBossEffLaserRain`, starts within 2000 at 1000 up, ends within 1000,
each a trail of sixteen segments 100 apart).

`tools/test_battle_corbenik_b_rs.py` runs `ccBoss08_b` natively: the
WaveShock and the rings stand-ins; the star dust, its bolts and rain, the
transform and the drain native; a particle generator living the case's
frames. Its cases agree frame by frame and reach acts 0, 3, 4, 5, 6, 14,
16, 18, 20, 21, 22, 23 and 24.

The boss camera is free while `ResetFlg` (+0x04) is 5: `SetMode` writes
its mode there, so the form's own `SetMode(5)` (the star dust, press, root
drain, sybly, barrier and death acts) frees it until a `SetMode(6)` puts
it back. Only then do `SetFreeCamPosView` and the straight writes to its
eye (+0x20) and view (+0x10) land: the star dust's follow of its core, the
press's eye at frame 120, the root drain's rising eye. The harness runs
`SetMode` and `SetFreeCamPosView` as the game's do on its camera and
compares the camera every frame; those writes agree.

## The third form

`ccThBoss08c_Main` news `ccBoss08_c` (0xa4fb0 in Quarantine, 0xa4f30 in
Infection's DWARF) and runs its `Main` after each `Breath`. The
constructor builds the three killer eyes inside it first
(`ccBoss08KillerEye`, row 9, out, at Kite less 500 on y with his heading,
their clip 0 and after-images set), then the body: row 9, 500 behind Kite
turned to face him, drawn and hit, `x81`'s `DMY_dummy1` kept as the eye
dummy (a `ccDummyPosChunk`: its place at +0x10, w 1), the wave clip, on
the lists, sixteen eyes, the centre, the stage fader, act 0,
`SetPatternMode(0)`, the boss camera (`InitBossCamera(980, 1500)`, its
pitch `atan2f(500, 1000)`, range 1000, `SetRotXLimit(1.0)`) and the blur
(on, `0x48808080`, scale 1.01).

Quarantine's class has its own members past Infection's:

```text
+0x29350 anmw            +0x29354 dmyEye
+0x29358 m_effLaser[16]  +0x29398 m_genEnergy
+0x293a0 m_killerEyeTbl[3] (0x293e0 each)
+0xa4f44 m_stageEff      +0xa4f48 m_patMode   +0xa4f4c m_bossEff
+0xa4f60 m_eyeCenterOffset   +0xa4f70 m_oldPos
+0xa4f80 m_pauseSlaveFlag    +0xa4f88 m_attackSlave
+0xa4f8c m_slvAtkMode (always 1)
+0xa4f90 the half-HP turn taken   +0xa4fa0 where it dies
```

### The eyes

`SetupSlave` news sixteen `ccBoss08Eye` (0x293d0 each; each constructor
draws three `ccRandF(1 degree)` for the turn its offset takes each frame),
then sets each on the body: `SetMaster`, `Order(0)`, and an offset of
`|ccRandF(1000)| + 4500` out, pitched down by `ccRandF(pi)` held to 30-150
degrees and turned round by another `ccRandF(pi)`. An eye follows every
act the body takes (`ChangeAction` sends each `Order(n)`, which is its
`ChangeAction(n, 3, 0)`; `Order(23)` is a reflection's flash). In its
neutral it rides the dummy plus its offset, facing the body; in the laser
it faces the body's target, in the tera laser its own. Any other act puts
it back to the neutral.

The dummy's place is `ApplyMatrix(anm, dmyEye->pos)`: the body clip's
world matrix as its last draw left it (`ccBoss::Draw` draws the clip while
its transparency is 0.05 or more, and the draw computes the matrix from
the `SetMatrix_PosRotZYX` its `PreDrawAnm` set). The body's own
`PreDrawAnm` runs before its draw, so its eye centre offset is a frame
behind; its `Slave` runs after the draw.

### The killer eyes

`EntryKiller(3)` (the eye entry) gives each killer eye that is out a new
life (`Generate`): its member (slot `k % 3`, else the nearest), out of
the dummy with the body's heading, row 45 + `k % 3` (45, 46, 47: guards by
`Exdefense`), drawn and on but no body hit, unseen, act 3. It fades in
over 90 frames drifting along its heading, is thrown at 120 (30 a frame,
`@2448` sixths of pi off a quarter turn from its heading, spinning by
`ccRandF(pi/2)`; one `rand()` drawn and dropped), bounces to a stop, rises
to 200 facing its target and at 90 takes its table: `boss08KillerEyeActTbl`
(`2 30, 7 240, 17`), `boss08DownnerEyeActTbl` (`7 480`) or
`boss08RipsEyeActTbl` (`7 960`). Its pattern 7 is the wander (act 8, its
length the next word, in `actProccess`), 17 the press (act 4): it flies at
its target, draws back, dives on it (boss skill 60), flies back. 14 asks a
skill of `@2388` by `ccRand() % 13` on its target, 15 compels one of
`@2391`, 16 asks skill 154 on the body; the body's slave attack calls them
(`Attack()` by row: 45 14, 46 15, 47 16). While the body pauses its slaves
a killer eye's next pattern is 2 -1, which holds it in its neutral for
good: from the body's first pause on, the killer eyes act only when the
body's slave attacks call them. A blow takes its HP (its own `Affect`:
SE 175, the guard's shield); at 0 it dies into a root drain at its place
and is out once the drain is over.

### The patterns and acts

`SetPatternMode(0)`'s table (Infection's `boss08CNormalActTbl`):
`2 90, 17, 7, 18 45, 2 150, 1, 2 300, 18 47, 7, 2 150, 2 -3, 14, 2 150,
1, 2 150, 18 45, 2 150, 18 46`. At half its HP (the first blow there:
stream 88) `SetPatternMode(1)`, Quarantine's new `boss08CNormalActTbl`:
`2 60, 17, 7, 18 45, 1, 2 90, 18 47, 7, 2 150, 2 -3, 14, 2 150, 1, 2 150,
2 -3, 16, 2 150, 17, 18 45, 2 150, 18 46, 7, 2 150, 2 -3, 15, -1`. Every
word lets the slaves go first. Its own words: 1 the wave (act 6), 14 the
laser (3), 15 the data drain (4), 16 the tera laser (5) (those three
pausing the slaves), 17 the eye entry (20) once every killer eye is out,
else the next word, 18 n a slave attack (21) by row n. The rest go to the
base's `ExecPatternIndex` from the word it was called at: after a 17
passed over, the base reads the 17 again and takes the neutral, and the
next call starts at the word after it. The base's 7 is the wander (act 8,
round the centre for 300 frames). Its neutral's wait -3 holds while a
killer eye is in and neither in its neutral nor wandering.

- **Wave** (6): `ANM_xx11wave` from its start at 512; by the body's clip,
  SE 255 at 61, from 70 the quake and the wave drawn, at 73 a flash and a
  WaveShock, at 85 boss skill 59.
- **Eye entry** (20): the camera 1500 out from the dummy, the eye's birth
  there (`ccBossEffGenerateEyeCreate`), `EntryKiller(3)`; at 90 the camera
  3000 out, drawn back 6.67 a frame to 220 (while the camera is free).
- **Laser** (3): the dark stage, the body unseen 1000 past the target; the
  eyes fly from 1500 behind the target to three rings round it
  (`SetupRefrectLaserPos(50, 90)`: `@2016` radii 1000, 1500, 2000 at
  `@2017` heights 2000, 1000, 250; 3, 5 and 8 eyes; each slowing from 50
  to arrive in 90 frames); the charge at the dummy; then `16 / n` lasers
  at each of the n members alive (`ccBossEffRefrectionLaser`, 200 a draw
  from the dummy through 3-5 eyes picked by `ccRand` to the member, a ring
  trio and a flash at each eye, a burst and SE 35 where it ends); all
  over, boss skill 58 at the target. `ResetRefrectLaserPos` never steps
  past the first eye: only it goes back to where it was.
- **Tera laser** (5): from 1500 past the target the body rises out of
  sight, the eyes and killer eyes fade (the killer eyes out at 60); at 30
  the tera laser falls from 3000 over the origin onto the target
  (`ccBossEffTeraLaser`: its charge grows sparks for 170 draws, 11 every
  fourth draw by three `ccRandF` each, then a flash, the beam falling 200
  a draw, a ring and bursts, boss skill 56 at 25, three bursts; the beam
  thinning out).
- **Data drain** (4): stream 107 on the member's slot at 45, then the
  drain.
- **Slave attack** (21): the killer eye of the row (-1: one of `@1714` by
  `ccRand() % 3`) attacks; the next frame, the next pattern.
- **Death** (14): the camera 2000 out, `BossDeadGenerator` thrice over 95
  frames, at 20 more a fade to black, at 140 the killer eyes and the body
  out.

`tools/test_battle_corbenik_c_rs.py` runs `ccBoss08_c` natively with its
eyes, killer eyes, lasers, eye birth and charge; the WaveShock and the
rings are stand-ins, `ccBossEffExplode2Create` and the arena's clump
switch only note themselves.

## In the field

`piney_world::combat::corbenik` and `boss.rs` run the forms where event
313 makes them (`entry 7 7`, field 8's arena):

- **The start.** `ccThBoss08` reads `saveData` +0x652f (`eventStatus[55]`,
  which block 18 adds 1 to): 1 makes the third form (row 9, its sixteen
  eyes and three killer eyes, and its camera: `InitBossCamera(980, 1500)`,
  the pitch `atan2f(500, 1000)` at +0xe0, the range 1000 at +0xd8, the
  eye raised by `1500 tanf(pitch)` at +0xdc, `SetRotXLimit(1.0)`); else the
  first form (row 7, its eight seeds, no boss camera).
- **The second form.** Once the first form's `CheckExit` holds, in the
  same frame (QUA gcmn 0x003acaf8): the menu's forbid, `forbidChatExcept`
  and `cursolOff` cleared, `EntryFlash(30, 0x80e0ffff)`, the first form and
  its seeds deleted (the seeds' bodies leave the field's list), then
  `ccBoss08_b` on the same character with the effect manager as it was,
  its five roots, and `InitBossCamera(200, 1000)`. The task's +0x14 (what
  `absent 7` reads) is set only when the second form exits.
- **The bodies.** Each form and part is drawn as the clump of x81 its
  clip poses. The seeds are drawn 50 up at 3x and the roots at 1.5x
  (`SetMatrix_PosRotZYXScale`), the eyes turned XYZ in their flash's fog
  (0xffffff).
- **The pictures.** These come from the rules' `Pic`s:
  - In the field, from x81: the barrier object and its hit clip at the
    body's `OBJ_ex8body2` as it was last drawn, the press's images, the
    elements (the neutral clip one step on, turned XYZ), the drain's root.
  - In the field, from xeffect: the laser rain's and the reflected
    lasers' segments (`OBJ_ex31les1`, and `les0` in fog 50 at 0xff0000,
    one way round for each effect) and their heads' `ANM_ex31les3`.
  - The effects draw the rest: the sprites (the flush, the stars, the
    core, the sparks), the clumps (`CMP_x012` bolts, the tera beam, the
    transform's lays on `CLT_ex8efc2c1`, the heads' `CMP_ex31les3`), and
    every generator, burst, shield, smoke and radiate.
  - The seeds' counts and after-images are drawn as Magus's leaves'.
- **The generators the rules keep.** Each generator the rules keep (the
  root drain's last, the laser's charge, the tera laser's glow and fall)
  is held by its key. The effects answer `ccCheckParticleGenerator` for
  each key, and the boss's next frame reads the answer
  (`Combat::gens_alive`).

A part's `ccDeleteCmnd` comes with its boss's outs. The world used to
take that out off the boss itself, which dropped the first form from the
lists after its first wave. The rules take the character off the lists
themselves, so the world no longer acts on that out.
`session::tests::corbenik` plays the forms under god from the arena.
