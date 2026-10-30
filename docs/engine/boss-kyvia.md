---
title: Bosses - Kyvia's first fight
status: partial
volumes: MUT
covers: MUT gcmn.prg:0x006192c0 bossFunc (code 12), 0x00480540 SetupSkillName, 0x00473350 ccBoss::DrawParts, 0x004710b0 ccBossAfterImageEx::Init, 0x00471320 ccBossAfterImageEx::Draw, 0x004d29a0 ccThKyvia01, 0x004d2a80 ccBossKyvia01::ccBossKyvia01, 0x004d4410 ccBossKyvia01::Main, 0x004d47c0 Think, 0x004d48a0 SwitchActionPattern, 0x004d4a80 SwitchDeadCheck, 0x004d4c10 CheckDiscMove, 0x004d5140 CheckDmgSmokeEff, 0x004d3330 Action, 0x004d3cf0 DeadKyviaVoice, 0x004d3fc0 DeadKyviaCam, 0x004d53b0 KyviaMagicDamage, 0x004d5450 SetBPos, 0x004d5930 HandAtk, 0x004d60a0 LightAtk, 0x004d8bb0 MegidFlame, 0x004d91e0 Slave, 0x004d92b0 EntrySlave, 0x004f7e80 kyviaCore::InitData, 0x004f80f0 ResetData, 0x004f82c0 CoreInit, 0x004f8770 SetCoreBaseParam, 0x004f8a30 kyviaCore::kyviaCore, 0x004f8cd0 Action, 0x004f9630 Main, 0x004f99c0 Affect, 0x004f9dc0 StartInit, 0x004f9f40 ActionStart, 0x004fa960 CoreVoice, 0x004faac0 Think, 0x004fae40 ThinkESCAPE, 0x004fb110 ChangeAttribute, 0x004fb1f0 CharHoldON, 0x004fb360 FadeCore, 0x004fb550 FadeCheck, 0x004fb5e0 GetGomoraState, 0x004fb860 JudgmentDmgAnm, 0x004fb8e0 CheckCountAtkTime, 0x004fbba0 CheckCountGomora, 0x004fc300 SetCoreState, 0x004fc4c0 GetActNum, 0x004fc5e0 SwitchActionPattern, 0x004fc8b0 MoveSpline, 0x004fca70 SetSpPoint, 0x004fd630 Slave, 0x004fd700 EntrySlave, 0x004fd980 KillGomora, 0x004fdc90 EntryGomora, 0x004fdda0 StartGomora, 0x004fde50 CheckGomoraSkillAct, 0x004fdf10 kyviaGomora::InitData, 0x004fe260 ResetData, 0x004fe460 GomoraInit, 0x004fe6c0 GetAttribute, 0x004fe740 StartInit, 0x004ff3d0 SetGomoraState, 0x004ff480 Affect, 0x004ff800 Action, 0x005004d0 Main, 0x00500850 Move, 0x005009a0 DepartureMove, 0x00500be0 NextGList, 0x00500c50 ListStepUp, 0x00500d50 Think, 0x00501370 TimeAuraCount, 0x005017c0 CheckCountAtkTime, 0x00501de0 RndTarget, 0x00501f20 GetRndSkill, 0x00501fe0 MoveGomora, 0x00502220 FadeGomora, 0x00502530 MoveSpline, 0x00502670 SetSpPoint, 0x00503640 SetAtkPoint, 0x0047aa90 ccBossEffMeteoriteMissile::ccBossEffMeteoriteMissile, 0x0047b330 Draw, 0x0047b910 Spline, 0x0048d9f0 ccBossEffMeteoriteMissileCreate, 0x00475ae0 ccBossCam::SetMode, 0x00475a40 SetRotXLimit, 0x00476380 SetTransfer, 0x00475cf0 Pad_Control, 0x004755a0 CamMain, 0x00500850 kyviaGomora::Move, main 0x001534f0 ccCharHit::CollisionDetection, 0x0061b190 Kyvia01AnmTbl, 0x0061b360 kyviaCoreAnmTbl, 0x0061b620 kyviaGomoraAnmTbl, 0x0061b510 AllGomoraList_1[0], 0x0061f650 KyviaGenerator, 0x0061fce0 KyviaDmgGenerator, 0x0061f3d0 CoreGenerator, 0x0061eec0 GomoraGenerator, 0x0061f1b0 GomoraAuraGenerator, 0x0061d900 MissileSmokeGenerator, 0x0061e290 BurstSmokeGenerator, 0x00472e50 ccBoss::Draw; MUT SLUS_205.62:0x00150fd0 ccAnm::SetAnm, 0x0014e760 ccStream::DecodeF_Morpher
worklog: 272, 420, 425, 426, 427
---

# Bosses - Kyvia's first fight

Kyvia's first fight (`ccBossKyvia01`, kyvia01.cpp) is `bossFunc` code 12,
fought in Mutation's event 108 on the disc of field 9 (`EVENTAREAB8`,
[the story maps](evarea.md)). It is three classes of `ccBoss` (the base
is [Skeith's page](boss.md)): the body, which stands off the disc and
strikes; its core (`kyviaCore`, kyviacore.cpp), the one the party fights;
and the core's five gomoras (`kyviaGomora`, kyviagomora.cpp). The names
and layouts are Infection's DWARF; the rules are Mutation's code, checked
frame by frame against the port.

## Where it comes from

Event 108's block 21 (`in_field town=2 field=9`) runs `entry type=7
code=12` and `battle_ready`. `ccBossEntryStart(12)` starts `ccThBossEffect`
and `ccThKyvia01` (0x004d29a0). The task makes `new ccBossKyvia01(1)`
(0x296a0 bytes) and runs its `Main` (vtable +0x1c) until `CheckExit`
(+0x4c), then sets its parameter's +0x14. Block 23 waits on `absent type=7
code=12`, then sets `eventStatus[0]` to 2.

```text
class           size     own members from  rows (bossTbl)
ccBossKyvia01   0x296a0  0x29350           12 (5000 HP)
kyviaCore       0x295a0  0x29350           32/33 by attribute at level 1 (5000 HP)
kyviaGomora     0x295f0  0x29350           16-19 by attribute at level 1 (row 16: 1000 HP)
```

Mutation adds one byte to the body's layout: +0x295f8, set when the body
turns the cinema on for an attack and cleared (with `OffCinemaMode`) once
the menus are free again.

## The constructors

**The body** (0x004d2a80, level 1): `SetBaseParam(row 12)`; `StFlg` 1,
`timeMode` 1, `AtkPatMode` 1, `DiscLV` and `DiscMaxLV` 1, `LiveFlg` 1,
`DiscExFlg` 1, `AFtiming` 2; exit off, drawn, body hit (radius 400) and
cheat HP on; `ANM_ex01nut0` in act 0. It stands at `DMY_marker01` of the
event map's stream (`dummypos`). `InitBossCamera(450, 1050)`, the pitch
(+0xe0) 0.26, `SetRotXLimit(0.6)`, +0x139 1 (`ZrotControlFlg`: the stick
turns the camera, [below](#the-camera)). Its body hit (radius 400, its
row's height and kind 0x80) goes into the hits' list. The arm's
animation `ANM_ex0batc0` (`anmw`). The blur (the effect manager's +4):
`m_enabled` (+0x24) 1, `m_exit` (+0x28) 0, `m_scale` (+0x0c) 1.0,
`m_abgr` (+0x1c) `DefaultARGB` 0x30808080 (`ExARGB` 0x60808080). Then one
core (`CORE_THIS`): `CoreInit(1)`, `SetMaster(body, 0)`, `kAtkFlg` 0.
`MaxHP` is the core's, `SwitchHp` a quarter of it, `QuakeVector` (10, 10,
10). The body is never on the command lists.

**The core** (0x004f8a30, `InitData` 0x004f7e80): at Kite's place, not
drawn, exited. `timeCount` 0.005, `timeFlg` 1.0, `AFtiming` 10, `DmgWait`
1000, `Dmgtime` 3000, `GsRimit` 30, `GcuntRimit` 60, `FadeFlg` 1,
`GomoraLock` 1. `CoreInit(lv)` (0x004f82c0): `SetCoreBaseParam(lv,
attribute)` (0x004f8770: level 1 rows 32 and 33, 2 34/35, 3 36/37, 4
38/39; at levels 2-4 a core that has died gets 3000, 1500 or 1000 more
max HP, added to both rows of the table for good; level 5 row 15),
`ANM_ex0cnut0`, then five gomoras (`GomoraInit(lv)`, `SetMaster(core, k)`,
state 2).

`LifeFlg = &GomoraFlg[i] != 0` compares the array's address, not its
byte, so every gomora starts with `LifeFlg` 1.

**A gomora** (0x004ff220, `InitData` 0x004fdf10): `SetBaseParam(row 16)`,
`state` 2, `AFtiming` 2, `DeadVoiceFlg` 1, `QuakeVector` (10, 10, 20).
`GomoraInit(lv)` (0x004fe460): `MasterList` = `AllGomoraList_lv[0]`,
`dirc.x` pi/2, `ANM_ex0gnut0`.

## The disc and the core's rise

`CheckDiscMove` (0x004d4c10) reads `discPrevPos` (+0xca0 of the event map)
into `DiscPos` every frame. While `StFlg` is 1 and `EVENTAREAB8::IsMove`,
the camera is in mode 5 (set once), 1500 off the disc turned toward the
body and 500 up, looking 150 over the disc; the body's cry (SE 232) plays
at its neutral clip's frames 20 and 60. When the disc stops: the blur at
1.05 scale in `ExARGB`, the quake (20, 20, 20) for 20 frames, SE 56,
camera mode 6 and `InitLock`; at 21 frames the colour back, at 35 the
scale 1.01, at 50 `StFlg` 0 and `EntrySlave(0, 1)`: the core out
(`SetEnterPos(DiscPos)`).

The core's `Think` (0x004faac0) fades it in: at `Alpha` 0 `StartInit`
(0x004f9dc0: 5 off the disc turned by `ccRandF(pi)`, drawn, `Core` row 0,
`SetSpPoint`), then 0.02 a frame; at 1 `ActionStart` (0x004f9f40): on the
command lists, its two auras (`Core` rows 7 and 8, 250 up; the other
attribute's not emitting), `DmgWait` 800 at level 1. Its heading turns
0.03 a frame.

## The core's life

Each frame (`Think`) the core rides a Catmull-Rom spline through
`SpVec1`-`4` (`MoveSpline`, 0x004fc8b0: the weights through
`sceVu0ApplyMatrix` with the points as rows), `time` by `timeCount`, a
new path (`SetSpPoint`, 0x004fca70) every fourth part: from where it is to
three points round `DiscPos` raised by |r350| + |r150|, one of three
patterns (`|ccRand() % 3|`). It picks a target with `SelectTarget(ccRand()
% 15, 999999)` (the sign kept).

`CheckCountGomora` (0x004fbba0): after 60 frames, a gomora every 31
(`GsCount++ == GsRimit`, the old value compared) until five are out
(`EntryGomora`, `StartGomora`); once all are gone again, the next lists,
and `NowDmgtime` 1800 more.

`CheckCountAtkTime` (0x004fb8e0), while no gomora is coming out and the
core is not away: `NowDmgtime++ < Dmgtime` (3000) else the time is up;
or `DmgCount - HP >= DmgWait` (800). Either kills the gomoras
(`KillGomora`); once none is left and the core is not held, the core goes
away (state 4) with `kAtkFlg` 2 (time) or 1 (damage).

`ThinkESCAPE` (0x004fae40), state 4: the core off the targets, its aura
out, five times as fast along its path, fading 0.015 a frame with
after-images; once faded and two parts on, it comes back with the other
attribute (`ChangeAttribute`: the other row, its HP kept).

`Affect` (0x004f99c0): `ccBoss::Affect`'s kinds without the party checks;
a hit shows `effResistantShield` when its skill's type
(`ccSkillCheckType(affectParam[1])`) is the kind the attribute resists (0
physical on attribute 0, 1 magic on attribute 1).

`Action`'s act 13 (0x004f8fc8), the core's wave: `ANM_xx11wave` set on its
own `anmw` (+0x29358), then each frame `_AnimateForward`,
`SetMatrix_PosRotXYZ(pos, dirc)` of the core and `DrawParts(anmw, 0)`, an
`EntryAffect(this, 1, 10)` on each member at its frame 25, and act 12 at
its end. No code of the game's sets act 13 (no `ChangeAction(13)` in
Kyvia's code, no write of 13 to `actNum`; affect 13 drains, act 11), so
it is never seen. The harness forces it (`test_core_wave`); until worklog
530 the port drew it at Kyvia's place at time 0.

## The body's attacks

`SwitchDeadCheck` (0x004d4a80) reads the core's flags: `kAtkFlg` 1 hurts
the body (act 1: a smoke on it each quarter of the core's HP lost, the
camera round it, `deg` 0), then 3, then 2; `kAtkFlg` 2 with the core
back (state 0) starts the next attack in turn (`SwitchActionPattern`,
0x004d48a0): `HandAtk` (cinema 33), `LightAtk` (32), `MegidFlame` (34).
Their names are rows 0-2 of `TEX_kbi_skl` ("Legion's Reach", "Ark
Bullet", "Megiddo Flame"). `_g_cinemaSkillName` names no file for them:
`SetupSkillName` (MUT gcmn 0x00480540) reads x01 to x04 in fields 9 to 12
whatever the row says. The port loaded only rows naming the boss's file,
so until worklog 425 the cinema showed its bars without the names (#72).
`kDmgFlg` 4 (a hurt core, one time in five) makes the body flinch (act 2).

Each attack waits for the core and its gomoras at rest, fades them out
(`SetCoreState(8)`; the test there, `CoreState != 8 || CoreState != 6`,
always holds), and fades them back (7) when done.

- `HandAtk` (0x004d5930): camera mode 3 at range 3100 until its
  `ResetFlg` clears; the arm (`ANM_ex0batc0`) swept from the camera's
  place; at its frame 48 boss skill 33 by the core on every member. The
  arm's place (`tesPOS`, `tesDIRC`) is set once on the anm
  (`SetMatrix_PosRotZYX`); each frame of the sweep, every `AFtiming`-th
  frame first, `EntryAfterImageAnm(anmw, tesPOS, tesDIRC, &t)` (+0x29620,
  0x004d5d70), then `_AnimateForward` and `DrawParts(anmw, 0)` (vtable
  +0x30, `ccBoss::DrawParts` 0x00473350: objLayer, no blend,
  `ccAnm::Draw`). The arm's after-image `t`: colour 0x00525221 at 30,
  transparency 0.5 by -0.02 a draw, no scale (`maxFrame` 80 is counted
  down and never read). `ccBossAfterImageEx::Init` (0x004710b0) copies the
  anm's clip at its frame (`_AnimateFrame`; at frame 0 one step on);
  `Draw` (0x00471320) moves the blend's rate by `colorSpd` (held 0-100)
  and the transparency by `transSpd` (dead at 0 or 1), grows the scale
  from `ss` to `es` over `scaleTime` draws, then draws in the blend.
- `LightAtk` (0x004d60a0): three beams from 350 back and 650 up, splines
  through six points about the disc (`SetBPos`) to the members; boss skill
  32 on each.
- `MegidFlame` (0x004d8bb0): mode 3 at 2500, stream 48, then seven meteors
  (`ccBossEffMeteoriteMissileCreate(DiscPos, 500, 7, 0, KyviaMagicDamage,
  bossCam)`): each a Bezier from 500 back (turned -2.5525441 about x) to a
  point within 500 of the disc, steps of 0.03 to 0.06. The first's landing
  calls `KyviaMagicDamage` (boss skill 34 on each member) 20 and 60 frames
  on; the effect ends once all have lain 90 frames.

The counters in these steps are compared before they are raised
(`actCount++ == 40` and the like).

## The gomoras

A gomora's attribute is `MasterList[NowListNum][slaveID]`
(`AllGomoraList_1[0]`, three lists: 3 3 3 2 0, 0 1 1 1 2, 3 3 0 1 2; 4 is
none). Out of the core (`StartInit`, 0x004fe740) on a spline of three
points up and away, 30 steps a frame, then round the disc on paths of four
patterns. As its attack time (`AtkWait`: 270, 270, 600, 150 by attribute)
runs out its aura grows (0.8, 0.5, 0.3 of it); at `AtkWait-- == 0` (the
old value), unless another gomora is casting:

```text
0  heals the core: effSkillStart(295), ccSkillRequestParam(core, 295, 200)
1  a skill of Skill_VARIOUS on a member
2  a skill of Skill_DOWNER on a member
3  the dash (act 3): the target held, out and back on two splines of
   AtkVec1-8, boss skill 35 at 0.72 of the third part
```

`KillGomora` sets a gomora's HP 0 and act 14; its death fades it
(`FadeGomora`, 0.05 a frame) and it waits again (state 3, exited).

**The pushes.** Once out (`Think`'s step 2, MUT gcmn 0x00500f9c) a
gomora's `bodyHit` (+0x240) takes its row's width as radius and height,
kind and mask 0x80 (bosses) and `mask2` 0 (no ground), and goes into the
hits' list (`HitEnable`); the dash takes it out and puts it back, act 14
takes it out. `kyviaGomora::Move` (0x00500850) moves the gomora as
`ccBoss::Move` does, then puts the hit 100 below it and runs
`CollisionDetection` (main 0x001534f0): the push out of every other
listed body of a boss kind, `r1 + r2 - d` along the line between the
centres, summed in the list's order. Pushed, the gomora and its `posP`
move by the whole push (its height too, which `ccBoss::Move` drops) and
the hit stands at the gomora's place itself, not 100 below, until the next
`Move`. The others it meets are the gomoras out before it in the frame
and the body (radius 400, standing off the disc); the core's body is never
listed (`bodyHitSW` 0).

## Death

The core at 0 HP (act 14, 0x004f9184): the gomoras killed, its particles,
fading 0.02 a frame with its cries; at nothing `DeadFlg` 1, exited, reset.
`CheckDiscMove` then: `DiscLV < DiscMaxLV` (never at level 1) would move
the disc on (`EVENTAREAB8::Move`); else `LiveFlg` 0, and `SwitchDeadCheck`
puts the body in act 14: its smoke, cries (`DeadKyviaVoice`), cameras
(`DeadKyviaCam`), the music out (`ccSqFade(0, 0, 30, 3)`), then exit.

## The camera

Mutation's `SetMode(mode, range)` keeps the range (+0xd0); modes 3 and 4
in `SetTransfer` (0x00476380) take the start on their first frame (3: the
eye's y off the view, kept that far from the range; 4: back to what 3
left, +0x134) and then ease `MoveTransfer.y` a quarter of the way a frame,
snapping within 8 and clearing `ResetFlg`. `SetRotXLimit(v)` sets +0x114
`v` and +0x118 `1 - v`.

Mutation's `SetMode` also keeps `ExLock` (+0x138): mode 3 sets its bit 4
and mode 4 clears it, mode 5 sets bit 0 and mode 6 clears it. While
either bit is set `Pad_Control` (0x00475cf0) reads nothing: from the end
of an attack's mode 3 ease until its mode 4 the stick neither zooms nor
turns.

**The stick's turn** (`ZrotControlFlg`, +0x139, Kyvia's constructor):
after the zoom and the resets, `Pad_Control` turns `ZrotCont` (+0x110),
the eye's turn about the player in `CamMain`, held to +-0.5:

- the right stick's sideways lean, `sinf(dirc) * pow` past 1 (as a
  double), times 0.00023 (0x39712c28): added in schemes 0 and 2,
  taken off in 1 and 3;
- in schemes A only, without it, L1 or R1 by 0.015, plus 0.0008 a step
  of pressure past 155 (0.08 more at no pressure): L1 adds in scheme 0,
  R1 in scheme 1, and the other takes off.

Infection's `Pad_Control` has the same turn; no Infection boss sets the
flag. Mutation's `CamMain` (0x004755a0) also raises the eye by its +0xdc
times `1 - t` (the zoom's share; 0 but for Quarantine's `ccBoss08_c`)
and sets `CamPos.w` 1. Mutation's `ccBossCam` is 0x150 bytes: that float
at +0xdc puts `Xrot` to `count` 4 further (+0xe0-+0xec) and, aligned,
the rest from `RotZ` 0x10 further (`ZrotCont` +0x110, `CameraType` +0x11c,
`ExLock` +0x138, `ZrotControlFlg` +0x139, `tempChar` +0x13c).

## The pictures

The fight's particle generators, where each is made (Mutation's code;
Outbreak's is the same, with the second fight's layout 0x10 further on).
The rows are of the fight's tables, the force fields in brackets.
"Follows" is `syncPos` with `syncPosType` 0. "Kept" is the member that
holds the pointer:

| maker | table row (ff) | where | kept |
| --- | --- | --- | --- |
| `LightAtk` step 2 | Kyvia 0 (0) | at `LOffset` | +0x29500 |
| `LightAtk` step 4, count 30 | Kyvia 7 (10, 11) | at `LOffset` | - |
| the same, each beam | Kyvia 5 (1) / 5 (8) | follows `LBpos[k]` / `LBposSub[k]` | +0x29504 / +0x29510 + 4k |
| `LightAtk` step 5's end, each beam | Kyvia 3 (12, 4) | at `LBpos[k]` (a member standing) | +0x29500 |
| `CheckDmgSmokeEff` | KyviaDmg 0 (0, 1) | at the smoke's place | `DmgGp[HpProccess]` |
| act 14 step 0 | KyviaDmg 1 (2, 3) | follows `DedDmgVec` | `DeadGp` |
| the core's `StartInit` | Core 0 (0) | follows the core | - |
| the core's `ActionStart` | Core 7 / 8 (3) | follows the core, offset (0, 0, 250) | +0x29360 / +0x29364 |
| the core's act 14 | Core 1, 2 (1) | at `DeadPos` | - |
| a gomora's `StartInit` | Gomora 5 (6, 5), 6 (7, 5) | 200 over the core | - |
| a gomora's dash | Gomora 0 (10, 9) | 60 over `AtkVec[3]` | - |
| a gomora's act 14 | Gomora 1 (0, 2, 4), 2 (3, 4) | 250 over it | - |
| `TimeAuraCount` | GomoraAura attribute (level) | follows `AuraPos` | +0x293e0 |
| `CheckCountAtkTime` | GomoraAura 4 + attribute (0) | at `AuraPos` | +0x293e0 |

What the fight does to those it keeps:

- `LightAtk` kills each beam's head and tail before its burst.
- Act 14 kills the four smokes at its count 60 and `DeadGp` at step 2.
- The core's death kills both auras and clears them.
- A gomora's death kills its aura.
- A grown aura (levels 1 and 2) and an attack's aura kill the one before.
  The attack clears the pointer once its skill is asked for.
- The core's other attribute's aura is paused (+5 bit 0, `pauseFlag`) by
  `ActionStart`. The escape pauses the core's own; `SetCoreState(7)`
  frees it.

**The meteors.** `ccBossEffMeteoriteMissile` (0x0047aa90, `Draw`
0x0047b330; Outbreak's 0x00476510 and 0x00476c90 are the same code):

- The constructor makes one particle `ANM_x300` per meteor.
- It starts two trails per meteor, `MissileSmokeGenerator` rows 6 (5)
  and 7 (6). Each follows its meteor's place (`now[k]`, the array at
  +0x60), syncPosType and distSW off.
- In flight each `Draw` draws the fireball at `SetMatrix_PosRotXYZScale(now,
  0, 3)`, transparency 0.5, and never steps it.
- At a meteor's landing its trails are deleted (`ParticleDelete`, +0x94
  then +0x10c), then SE 35 note 60 plays at the shower's place.
- The first's landing also starts `BurstSmokeGenerator` rows 8 (11, 12,
  13) and 9 (11, 12, 14) at the place and 11 (17, 12, 16) 120 up, all
  distSW off.
- It then throws `effSmokeRock(now, rot, 75, 0, 20, -1)`. `rot` is camera
  2's (`cameraGetRot(r, 2)`) turned by -pi, x and y 0.

**The bolts** (the second fight's,
[boss-kyvia-second.md](boss-kyvia-second.md)).

## The port

`piney_battle::boss::kyvia` (the body), `kyvia::core`, `kyvia::gomora`.
The core and each gomora are a character and `Boss` of their own
(`AffectFunc::Boss`), taken out of their characters for the body's frame
(`Tree`). The tables (`kyvia01_anims`, `kyvia_core_anims`,
`kyvia_gomora_anims`, `kyvia_various_skills`, `kyvia_downer_skills`,
`kyvia_gomora_lists`) are the build's combat group.
`piney_world::combat::boss` starts it for code 12 with the disc
(`DiscView`: `IsMove`, `discPrevPos`, `DMY_marker01`), draws the core and
gomoras (`CMP_trall2`, `CMP_trall3` of the fight's file), runs the queued
affects of all three, and `BossCam` has modes 3 and 4.

The pictures (worklog 420):

- The rules name each start as `Out::KyviaParticles`: a `kyvia::Gen` by
  maker (its rows), where it stands (`Place`, `Follow`) and the member that
  keeps it (`Held`). `Out::KyviaGen` kills, pauses or lets go of a kept
  one.
- piney-effect's `kyvia::start` and `gen_op` make them. The followed
  places are anchors that piney-game publishes each frame from the
  fight's state (`kyvia_shows`).
- The meteors' flights are `Meteorite::make` and `fly`, a pure step with
  its calls. piney-effect's `kyvia::Meteorite` draws them after the
  boss's task, with their trails and landings, as Innis's missiles are
  drawn.
- The tables are the effect group's `boss_kyvia*`, `boss_core*` and
  `boss_gomora*`.

The gomoras' shapes (worklog 430). No gomora picks one of
`CMP_ex01gom2`-`4`:

- `CoreInit(lv)` (MUT gcmn 0x004f82c0) and `GomoraInit(lv)` (0x004fe460)
  take their stream (+0xcc) by the core's level, `GetCCSAdrs("x01")` to
  `"x04"`, the attribute aside. So every gomora of a fight has the same
  stream and clips.
- `ccBoss::Draw` (0x00472e50) draws the anm (+0xd4) with `ccAnm::Draw`.
  The anm draws its clip's objects: `ANM_ex0gnut0`, `atc0`, `dmg0` and
  `mag0` move `OBJ_trall3`, `OBJ_ex01gom1` and `OBJ_ex31gomc`, the nodes
  of `CMP_trall3`.
- `ccAnm::SetAnm` (main 0x00150fd0) makes each anm its own `ccMorpher`,
  and `DecodeF_Morpher` sets it from the clip's `F_Morpher` records.
  `MPH_ex01gom1` bends `MDL_ex01gom1`: in `ANM_ex0gnut0` toward `gom2`
  (frames 1-10), on to `gom3` (to 20) and `gom4` (to 30), then back to
  the base (40). The attacks, damage and magic clips bend it toward
  `MDL_ex01atc1`, `atc02` and `atc03`.
- Each gomora bends by its own clip and frame, so gomoras out together
  differ. The core (`MPH_ex0c_out`) and the body (`MPH_ex0bbod1`, `2`)
  bend the same way.

In the port, `kyvia::stream(lv)` is the stream and `Fight::file` its
fight's. `piney_world::chara::body::Body` keeps its file's morphers and draws
each model with its clip's weights (`Body::morph_of`), as `foe::Model`
did; before, a body was drawn unbent, and the gomoras as the bare
`MDL_ex01gom1` of `CMP_ex01gom2`.

The pushes and the camera (worklog 427):

- A gomora keeps its `bodyHit` (`Gomora::hit`) and reaches the list
  through `Cx::part_hits` (`boss::PartHits`: `switch`, `collide`,
  `sync`); `piney_world::combat::boss` answers it from the field's hits
  (`hit::Hits`, the body's entry radius `kyvia::BODY_RADIUS`). The push is
  `piney_battle::world::char_push`, which `Hits::collision_detection`
  uses too. After the body's push its entry is moved with it, for the
  gomoras that move after it in the frame.
- `piney_world::camera::bosscam::BossCam` keeps `ex_lock`, `zrot_control` (from
  `Out::CamSway`) and Mutation's `lift`; `pad_control` reads nothing under
  `ExLock` and turns `zrot_cont` with the flag.

## Checks

`tools/test_battle_kyvia_rs.py` builds the game's `ccBossKyvia01(1)` with
its core and gomoras and runs its `Main` natively (with the effect
manager's pass), against battle_probe's `kyvia`. A case has one to three
members, a disc riding in for up to 200 frames, 300-2500 frames, Kite's
hits on the core or a gomora (and heals) while they are targetable, the
menu's type and the camera's turn changing. Compared each frame: the
body's acts, flags, clips and own members; the core's and each gomora's
acts, places, turns, HP, targets, lists, clips and own members; the
effects' slots; the boss camera's mode, lock, eye, view and pitch; the
blur; the party's HP and hold; the calls; the menu; `rand()` and
`ccRand`'s index. 410 cases (561,061 frames) agree; every act of the
body (0-5, 14), the core (0-2, 14) and the gomoras (0-4, 14) is reached.

Stood in for there: the party's affects are off (hits compared by what
`EntryAffect` stores); the files and clips are stand-ins at the clips'
lengths; the gomoras' skills are not run (`ccSkillRequestParam` sets the
caster's `skillID` and `skillStatus` 1 as `_ccSkillRequest` does, and
nothing clears them); the meteors run natively with their drawing,
particles and `effSmokeRock` stubbed; the camera's `CamMain` never runs
and a mode 3 or 4 is cleared after the case's delay; W2P/P2W are
identity.

Each part's stream (+0xcc, the name `CoreInit` and `GomoraInit` asked
`GetCCSAdrs` for) is compared every frame against `kyvia::stream` (worklog
430), and `test_parts_shapes` steps the parts' clips of the fight's file
in the game's `ccAnm` (the morpher's targets and weights each step)
against `tools/anim.py`, whose weights the port draws with.

The calls include the arm's: each `DrawParts` with the anm's frame and
flag, and each `EntryAfterImageAnm` with the arm's frame, place, turn and
all of its `ENTRYAFTERIMAGE_T` (worklog 426: 200 Mutation cases, 120 of
Outbreak's second fight and 40 of its first, act 3 in each run).
`session::tests::kyvia_fx::kyvias_arm_sweeps_with_its_after_images` sees
the arm drawn with its after-images on both discs.

The hits' list runs natively from an empty list each case (`SetHitSW`,
`HitEnable`, `HitDisable`, `CollisionDetection`; `ccModelHitCheckQ`
finds no ground), against battle_probe's own list of the body's and the
gomoras' bodies (worklog 427). Each gomora's `bodyHit` (switch, masks,
kind, radius, height, place, push) is compared every frame, and the boss
camera's `ExLock` and `ZrotControlFlg`: 120 Mutation cases, 60 of
Outbreak's second fight and 30 of its first agree, gomoras pushed in each
run. `tools/test_bosscam_rs.py`'s `test_sway` runs the game's
`Pad_Control` with the flag on all four schemes (on Mutation and Outbreak
with `SetMode` 3 and 4 between): `ZrotCont` and the cameras agree.
`session::tests::kyvia_fx::kyvias_gomoras_push_each_other_apart` and
`the_stick_turns_kyvias_camera` see both in play.

`PINEY_VOLUME=outbreak PINEY_KYVIA_FIGHT=1` runs Outbreak's copy of the
fight (its `Main` and the disc's `Move` and `IsMove` by address): the
same code, as the port has it.

The harness also compares each generator the fight starts (its force
fields, distSW, where it stands or what it follows and its offset, the
member that keeps it), each `ParticleKill` of a kept one, the kept
pointers and the core's auras' `pauseFlag`.

`tools/test_boss_effect_rs.py`'s `test_kyvia_effects` runs the meteors'
`Create` (no callback, no camera) and the manager's pass against
boss_probe's `meteorite` on both discs: the fireballs' draws, the trails
and what they follow, the deletes, the bursts, the rocks' effect slots,
the sounds and `genrand`.

`piney-battle`'s `boss::kyvia::tests` and `piney-game`'s
`event_108_ends_with_kyvia` (the event from block 21 to block 23 on the
disc) hold the rest. `session::tests::kyvia_fx` plays the fight with the
core's time cut short: the meteors drawn, the beams' generators following
their ends and the auras following the core.

## Unknown

Nothing. Levels 2-5 and the EX mode (`KyviaExFlg`) are Kyvia's later
fights, ported with them: level 2 [the second
fight](boss-kyvia-second.md)'s, level 3 [the third](boss-kyvia-third.md)'s,
levels 1-5 and the EX mode [the fourth](boss-kyvia-fourth.md)'s.
