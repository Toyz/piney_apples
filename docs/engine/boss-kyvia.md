---
title: Bosses - Kyvia's first fight
status: partial
volumes: MUT
covers: MUT gcmn.prg:0x006192c0 bossFunc (code 12), 0x004d29a0 ccThKyvia01, 0x004d2a80 ccBossKyvia01::ccBossKyvia01, 0x004d4410 ccBossKyvia01::Main, 0x004d47c0 Think, 0x004d48a0 SwitchActionPattern, 0x004d4a80 SwitchDeadCheck, 0x004d4c10 CheckDiscMove, 0x004d5140 CheckDmgSmokeEff, 0x004d3330 Action, 0x004d3cf0 DeadKyviaVoice, 0x004d3fc0 DeadKyviaCam, 0x004d53b0 KyviaMagicDamage, 0x004d5450 SetBPos, 0x004d5930 HandAtk, 0x004d60a0 LightAtk, 0x004d8bb0 MegidFlame, 0x004d91e0 Slave, 0x004d92b0 EntrySlave, 0x004f7e80 kyviaCore::InitData, 0x004f80f0 ResetData, 0x004f82c0 CoreInit, 0x004f8770 SetCoreBaseParam, 0x004f8a30 kyviaCore::kyviaCore, 0x004f8cd0 Action, 0x004f9630 Main, 0x004f99c0 Affect, 0x004f9dc0 StartInit, 0x004f9f40 ActionStart, 0x004fa960 CoreVoice, 0x004faac0 Think, 0x004fae40 ThinkESCAPE, 0x004fb110 ChangeAttribute, 0x004fb1f0 CharHoldON, 0x004fb360 FadeCore, 0x004fb550 FadeCheck, 0x004fb5e0 GetGomoraState, 0x004fb860 JudgmentDmgAnm, 0x004fb8e0 CheckCountAtkTime, 0x004fbba0 CheckCountGomora, 0x004fc300 SetCoreState, 0x004fc4c0 GetActNum, 0x004fc5e0 SwitchActionPattern, 0x004fc8b0 MoveSpline, 0x004fca70 SetSpPoint, 0x004fd630 Slave, 0x004fd700 EntrySlave, 0x004fd980 KillGomora, 0x004fdc90 EntryGomora, 0x004fdda0 StartGomora, 0x004fde50 CheckGomoraSkillAct, 0x004fdf10 kyviaGomora::InitData, 0x004fe260 ResetData, 0x004fe460 GomoraInit, 0x004fe6c0 GetAttribute, 0x004fe740 StartInit, 0x004ff3d0 SetGomoraState, 0x004ff480 Affect, 0x004ff800 Action, 0x005004d0 Main, 0x00500850 Move, 0x005009a0 DepartureMove, 0x00500be0 NextGList, 0x00500c50 ListStepUp, 0x00500d50 Think, 0x00501370 TimeAuraCount, 0x005017c0 CheckCountAtkTime, 0x00501de0 RndTarget, 0x00501f20 GetRndSkill, 0x00501fe0 MoveGomora, 0x00502220 FadeGomora, 0x00502530 MoveSpline, 0x00502670 SetSpPoint, 0x00503640 SetAtkPoint, 0x0047aa90 ccBossEffMeteoriteMissile::ccBossEffMeteoriteMissile, 0x0047b330 Draw, 0x0047b910 Spline, 0x0048d9f0 ccBossEffMeteoriteMissileCreate, 0x00475ae0 ccBossCam::SetMode, 0x00475a40 SetRotXLimit, 0x00476380 SetTransfer, 0x0061b190 Kyvia01AnmTbl, 0x0061b360 kyviaCoreAnmTbl, 0x0061b620 kyviaGomoraAnmTbl, 0x0061b510 AllGomoraList_1[0], 0x0061f650 KyviaGenerator, 0x0061fce0 KyviaDmgGenerator, 0x0061f3d0 CoreGenerator, 0x0061eec0 GomoraGenerator, 0x0061f1b0 GomoraAuraGenerator, 0x0061d900 MissileSmokeGenerator, 0x0061e290 BurstSmokeGenerator
worklog: 272
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
(+0xe0) 0.26, `SetRotXLimit(0.6)`, +0x139 1 (the pad's sway). The arm's
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

## The body's attacks

`SwitchDeadCheck` (0x004d4a80) reads the core's flags: `kAtkFlg` 1 hurts
the body (act 1: a smoke on it each quarter of the core's HP lost, the
camera round it, `deg` 0), then 3, then 2; `kAtkFlg` 2 with the core
back (state 0) starts the next attack in turn (`SwitchActionPattern`,
0x004d48a0): `HandAtk` (cinema 33), `LightAtk` (32), `MegidFlame` (34).
`kDmgFlg` 4 (a hurt core, one time in five) makes the body flinch (act 2).

Each attack waits for the core and its gomoras at rest, fades them out
(`SetCoreState(8)`; the test there, `CoreState != 8 || CoreState != 6`,
always holds), and fades them back (7) when done.

- `HandAtk` (0x004d5930): camera mode 3 at range 3100 until its
  `ResetFlg` clears; the arm (`ANM_ex0batc0`) swept from the camera's
  place; at its frame 48 boss skill 33 by the core on every member.
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

## The port

`piney_battle::boss::kyvia` (the body), `kyvia::core`, `kyvia::gomora`.
The core and each gomora are a character and `Boss` of their own
(`AffectFunc::Boss`), taken out of their characters for the body's frame
(`Tree`). The tables (`kyvia01_anims`, `kyvia_core_anims`,
`kyvia_gomora_anims`, `kyvia_various_skills`, `kyvia_downer_skills`,
`kyvia_gomora_lists`) are the build's combat group.
`piney_world::combat::boss` starts it for code 12 with the disc
(`DiscView`: `IsMove`, `discPrevPos`, `DMY_marker01`), draws the core and
gomoras (`CMP_trall2`, `CMP_ex01gom2` of x01), runs the queued affects of
all three, and `BossCam` has modes 3 and 4.

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
identity; `CollisionDetection` finds nothing and `HitEnable`/`HitDisable`
only set the flag.

`PINEY_VOLUME=outbreak PINEY_KYVIA_FIGHT=1` runs Outbreak's copy of the
fight (its `Main` and the disc's `Move` and `IsMove` by address): the
same code, as the port has it.

`piney-battle`'s `boss::kyvia::tests` and `piney-game`'s
`event_108_ends_with_kyvia` (the event from block 21 to block 23 on the
disc) hold the rest.

## Unknown

- The gomoras' pushes on each other (their `CollisionDetection`) are not
  ported, nor the pad's sway (+0x139) or the arm's picture (`DrawParts` of
  `anmw`); the particles are named, not drawn.
- The EX mode (`KyviaExFlg`) and levels 3-5 are Kyvia's later fights; the
  shared code for them is ported as far as levels 1 and 2 read it, and
  the gomora lists for those two. Level 2 is [the second
  fight](boss-kyvia-second.md)'s.
- Which of x01's gomora clumps (`CMP_ex01gom2`-`4`) each attribute draws.
