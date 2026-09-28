---
title: Bosses - Innis
status: partial
volumes: MUT
covers: MUT gcmn.prg:0x00470c40 ccBossEntryStart, 0x006192c0 bossFunc, 0x004941f0 ccThBoss02, 0x004942c0 ccBoss02::ccBoss02, 0x00495d50 ccBoss02::Main, 0x00494cd0 ccBoss02::Move, 0x004964b0 ccBoss02::Think, 0x00494e10 ccBoss02::Action, 0x00495448 act 5, 0x004960f0 ccBoss02::Affect, 0x00496a50 ccBoss02::ActionDecision, 0x00496f50 ccBoss02::ActionEnd, 0x00496900 ccBoss02::ResetData, 0x00496fc0 ccBoss02::SwitchActionPattern, 0x006e0c20 its jump table, 0x00497b80 Pattern01, 0x00497c40 Pattern02, 0x00497df0 Pattern03, 0x00497eb0 Pattern05, 0x004980b0 Pattern06, 0x004981d0 Pattern07, 0x00498500 Pattern08, 0x004987d0 Pattern10, 0x00498eb0 Pattern11, 0x00499110 SkillAttack, 0x004996c0 GetRndSkill_INIS, 0x00499770 Checkstate, 0x004997d0 CheckRange, 0x00499950 RndTarget, 0x00499a90 ParticularCount, 0x00499b40 SetCameraEyes, 0x00499c10 CharALLHold, 0x00499d10 DmgCheckCount, 0x0049a000 MagicSquare, 0x0049a7d0 MagicDamage, 0x0049a900 EnemyAttack, 0x0049aa10 AllAttack, 0x0049afa0 IceAttack, 0x0049b560 ThunderAttack, 0x0049bb30 BlazeAttack, 0x0049c1e0 EntrySlave, 0x0049c310 ccBoss02Slave::ccBoss02Slave, 0x0049c520 SetMonster, 0x0049c810 SetStartPosition, 0x0049cce0 ccBoss02Slave::Think, 0x0049d1b0 EnemyBurst, 0x0049d6b0 SetTan, 0x0049d820 SetLotate, 0x00533d90 BreakMirror::MainMirror, 0x00533fd0 BreakMirror::BreakMirror, 0x005340e0 BreakMirror::SetData, 0x00534430 BreakMirror::Mbreak, 0x00534a50 BreakMirror::CheckAnmEnd, 0x004780f0 ccBossEffManager::Draw, 0x00478d50 ccBossEffIceMissile::Draw, 0x00479420 ccBossEffIceMissile::Spline, 0x00479930 ccBossEffLightningMissile::Draw, 0x0047a220 ccBossEffBlazeMissile::Draw, 0x00475ae0 ccBossCam::SetMode, 0x00475c90 ccBossCam::SetFreeCamPosView, 0x004766d0 ccBossCam::CheckMoveCamera, 0x0061d350 InisFieldGenerator, 0x0061d4f0 TornadoGenerator, 0x0061d590 BurstGenerator, 0x00619bd0 Pattern, 0x00619e30 EPITAPH_Pattern, 0x00619b90 boss02AnmTbl, 0x00619f08 Skill_VARIOUS_INIS, 0x00619f20 Skill_DOWNER_INIS, 0x00619f40 Mon1, 0x00619f80 Mon2, 0x00619fc0 Mon3, 0x0061a000 @2702, 0x00619320 _g_cinemaSkillName, 0x00641aa0 bossTbl
worklog: 266
---

# Bosses - Innis

Innis (`ccBoss02`, boss02.cpp) is Mutation's first boss, fought in event
107 in field 2 of town 2 (`EVENTAREAB0`). It is a `ccBoss` (the base is
[Skeith's page](boss.md)) with its own `Main`, `Think`, `Action` and
`Affect`, three images (`ccBoss02Slave`, each with a `BreakMirror`) and
three missiles. The names and layouts are Infection's DWARF; the
behaviour here is Mutation's code, checked frame by frame against the
port.

## Where it comes from

Event 107's block 17 (`in_field town=2 field=2`, `in_point 3`) runs
`entry type=7 code=1` and `battle_ready`. `ccBossEntryStart(1)` (MUT gcmn
0x00470c40) starts `ccThBossEffect` and `bossFunc[1]` = `ccThBoss02`
(0x004941f0) at priority 66. Block 20 waits on `absent type=7 code=1`,
then sets `eventStatus[0]` to 2 and leaves the scene.

`bossFunc` (MUT 0x006192c0, 16 words) by code:

| code | task |
| --- | --- |
| 0 | 0x00490680 (Skeith) |
| 1 | 0x004941f0 `ccThBoss02` (Innis) |
| 2-7 | 0x0049da60, 0x004a7d00, 0x004ad5f0, 0x004b50a0, 0x004b9be0, 0x004be1f0 |
| 8-11 | 0 |
| 12-15 | 0x004d29a0, 0x004d9430, 0x004e1b60, 0x004eab00 (Kyvia 1-4) |

## The constructor

`ccBoss02::ccBoss02` (0x004942c0), on `bossTbl` row 1 (maxHP 30000,
maxPP 15000):

- `PatEndMode[0..3]`: the indices of the first three 29s in `Pattern`
  (41, 102, 148).
- The place: Kite's plus (0, 1500, 20); `Hipos` its z; Kite's heading.
- `ANM_ex21nut0`, act 0; `cheatHP` on; drawn; the body hit on.
- `DmgWait` 500; `DmgCount` 0 (maxHP as `ccChar::ccChar` left it, before
  `SetBaseParam`); `AFtiming` 3; `LockTargetFlg` 2; `EndFlg`, `PatEndFlg`
  and `PFlg` 1; `Alpha` 1.0.
- The blur (`bossBlur`, the manager's +4): on, never ending, colour
  `DefaultARGB` 0x30808080 (`ExARGB` is 0x60808080).
- `QuakeVector` (5, 5, 5); `InitBossCamera(200, 900)`.
- The wave's `ccAnm` (`ANM_xx11wave` of xeffect).
- Three `ccBoss02Slave` (`bossTbl` row 10, exited, not drawn).
- `vecCenter`: the arena's `DMY_center01`. On the command list.

## A frame

`ccBoss02::Main` (0x00495d50) is `ccBoss::Main` with `Think`, `Action`
and `Move` in that order. Then:

1. With `lockPlayer`, every member is held (affect 5, 32767) and a
   reserved menu forbid applies once no menu is open. Act 5 writes
   `ccMenu.forbid` itself the same frame, which that check reads.
2. `anmStatus` is the clip's `_AnimateForward`, and the transparency is
   `Alpha`.
3. Each image's `ccBoss::Main` runs, its `Think` the slave's.
4. `condition.hold` is cleared.

`Move` (0x00494cd0) is `ccBoss::Move`; the body, once pushed out, is
held at `Hipos`.

## Think and the patterns

`Think` (0x004964b0):

```
AFcou++; past AFtiming: 0
DmgCheckCount()
act 0 or 12, and not ExSpinBackFlg:
  EndFlg 0 and ActionStartFlg 1.0: ActionEnd
  EndFlg 1 and ActionStartFlg 0: ActionDecision
act 0 or 12:
  with a valid target, ActionStartFlg set, and not held (or spinning back,
  or Escape): SwitchActionPattern(MoveFlg)
  held: moveSpd 0
  no valid target: SetCameraEyes, target = RndTarget()
act 1, 2: ParticularCount (MoveFlg 15, 14, 5: Cou + 1; 4: Cou + 2)
act 11: EPITAPH_Flg 1, NowMode 3
act 3-8, 13, 14: nothing; any other act: ChangeAction(0)
```

`ActionDecision` (0x00496a50) picks the run by the protect gauge while
not drained. `PPcount` 0 and `PP` at most 0.2 maxPP: mode 0 from word 0.
`PPcount` 0 and `PP` at most 0.7 maxPP: mode 1 from `PatEndMode[0]` + 1.
Otherwise mode 2 from `PatEndMode[1]` + 1. A mode change restarts its
run. `MoveFlg` 0 takes the next word (`Pattern`, or `EPITAPH_Pattern`
once drained); 7 sets `Cou` 20. `LockTargetFlg` 2 retargets every word,
0 retargets once and becomes 1, 1 or 3 keeps a valid target. Then `Cou`
and `SubNO` 0, `ActionStartFlg` 1.0, `targetPOS` the target.

`RndTarget` (0x00499950) is `SelectTarget(|ccRand() % 15|, 1e6)`, else
the first living PC. `SelectTarget`'s types 2-13 are the base's (boss.md);
14 never picks.

`DmgCheckCount` (0x00499d10): before the drain, once `DmgCount - HP` is at
least a fifth of `DmgWait` (100), Innis spins away (`MoveFlg` 15, off the
targets) from the waits (words 4-6), else `ExSpinBackFlg` 1. After the
drain the same at `DmgWait` (500), to act 12.

`SwitchActionPattern` (0x00496fc0, jump table 0x006e0c20) first faces the
camera (`SetCameraEyes`: the camera's heading plus pi, 256 a step) and,
the first time, starts the four `InisFieldGenerator` rows 2, 3, 0, 1 at
(0, 0, 200). Then the word:

| word | does |
| --- | --- |
| 1 | `Pattern01`: run at the target (an eighth of the gap), until within 500; an after-image at `AFtiming` |
| 2 | `Pattern06`: dash (150 for 20 frames, then 100 inside 900), until within 500 |
| 3 | `Pattern02`: off at pi/3 either side of the camera's heading, 900 farther |
| 4, 5, 6 | `Pattern03`: wait 200, 100, 20 frames, back on the targets |
| 7 | `Pattern07`: once round the target, pi/36 a frame, either way |
| 8, 9 | `Pattern05`: along the camera's heading, 2200 or 500 farther |
| 10 | `Pattern11(2200)`: away holding the party, until 2200 off and the camera still |
| 11 | `Pattern05(n)`, `n` the next word (at least 1) |
| 13 | `Pattern08`: a zigzag, a leg every 10 frames, 31 frames |
| 14 | `Pattern10`: the escape (turn, fade, reappear 1100 from the origin, fade in) |
| 15 | the escape, spinning back |
| 16 | the spin and fall: act 3 (act 13 once drained), SE 223 |
| 17 | `EnemyAttack`: the images |
| 19 | `MagicSquare`: the missiles |
| 20 | `SkillAttack`, the next word its list |
| 21 | the member drain: lock, act 6 |
| 22 | `Checkstate`: not within 500, back two words |
| 25 | `Escape` 1, `LockTargetFlg` 0 |
| 26 | `LockTargetFlg` 2 |
| 27 | `Escape` 1, blur `ExARGB`, lock the player |
| 28 | blur `DefaultARGB`, unlock the player |
| 29 | the run from its start again |
| other | next word |

Words 1-3, 7-11, 13, 17 and 19 take Innis off the command list for the
action (`EntryFlg`); `ResetData` puts it back once the player is free.
`CheckRange` (0x004997d0) is 2200 from `vecCenter` on each axis; a move
past it ends in the escape.

The tables (MUT gcmn; identical on Infection):

```
Pattern 0x00619bd0 (150 words):
  6 3 25 2 11 200 7 16 26 5 1 16 3 25 27 20 0 26 13 25 27 20 0 26 1 16 8 5
  25 27 20 0 26 4 25 2 11 200 7 16 26 29 | 1 25 27 10 28 19 26 6 3 1 16 5 25
  10 17 26 6 1 16 6 25 2 11 200 7 16 26 4 25 27 20 0 26 1 25 27 10 28 19 26
  6 1 5 21 13 25 2 11 200 7 16 26 3 25 27 20 0 26 1 6 29 | 1 16 4 1 25 27 10
  28 19 26 6 1 5 25 27 20 0 26 16 3 1 16 21 3 1 25 27 10 28 19 26 6 25 2 11
  400 7 26 6 5 25 10 17 26 6 29 | 30
EPITAPH_Pattern 0x00619e30 (53 words):
  5 1 25 27 20 0 26 5 3 2 3 16 5 4 25 27 20 0 26 3 1 16 3 1 25 2 11 200 7 16
  26 3 25 27 20 0 26 2 4 5 9 25 2 11 200 7 16 26 1 4 13 16 29
Skill_VARIOUS_INIS 0x00619f08: 156 157 158 159 160 161 162
Skill_DOWNER_INIS  0x00619f20: 163 164 165 166 167 168 169 170 171 172 173 174
boss02AnmTbl 0x00619b90: 0 ex21nut0, 1 2 ex21dmg0, 3 4 ex21nut0, 7 ex21dmg0,
  12 ex2xnut0, 14 ex2xdea0 (the rest none)
Mon1-3 0x00619f40/80/c0: ANM_ex21mon1, mon2, mon3 at act 0 (the rest none)
@2702 0x0061a000: 191 193 191
```

## The acts

| act | what it does |
| --- | --- |
| 0, 12 | the patterns |
| 1, 2 | SE 195 (note 63 or 67 + `ccRand() % 4`); at the clip's end back to 0 (12 once drained) |
| 3, 13 | the spin: 25 frames rising at 50, turning 0.6 a frame; then (13 at once) `AllAttack` |
| 4 | the magic in flight: face the camera, the player locked |
| 5 | the images (below) |
| 6 | the member drain: menu 74 on stream 42 with the target's slot; when it closes, affect 13 on it |
| 7 | the shake: 120 frames of `ex21dmg0` at speed 3840, blur `ExARGB` |
| 11 | drained: cheat HP off, `ANM_ex2xnut0`, act 12, `NO` -1 |
| 14 | dead: off the command list, `BeginDeadEffect(3500, 100)`, 30 frames after it ends, exit |

`AllAttack` (0x0049aa10) drops Innis at 100 a frame to -4. There the
wave starts, the quake is (15, 15, 15) and `SkAtk` is skill row 4. Each
frame after holds every living member within its range, with the quake
for 50 frames. Frame 3 brings a flash (30, 0x80ffffff) and a WaveShock;
30 a SamonRing (model 192) and `BurstGenerator` row 2; 40 skill 4 at
Innis's place. Innis rises 2 a frame to `Hipos`. At the wave's end it is
back on the targets and in act 0 (12).

## The magic: `MagicSquare` and the missiles

`MagicSquare` (0x0049a000) at `Cou` 0 locks the player and sets the boss
camera to mode 5 behind Innis. At 2 it draws `|ccRand() % 3|` (0 ice, 1
lightning, 2 fire), turns cinema 5 + n on, makes the MagicSquare `n` and
begins the stage effect (0x64000000; 15, 15, 32277). At 50 it moves the
eye. Ten frames after, it makes the three missiles, ends the stage effect
(35) and goes to act 4.

Each missile (`ccBossEff{Ice,Lightning,Blaze}Missile`) flies a cubic
Bezier (`Spline`, 0x00479420): Innis, two random points, the target 10
up. Their `t` steps are:

| missile | `t` a frame | after impact (`t` > 0.984) |
| --- | --- | --- |
| ice | 0.02, plus 0.005 past 0.35 | grows 1.0 a frame to 100, fades 0.02 (twice for the first); done below 0 |
| lightning | 0.015 (0x3c75c28f) | the first makes a WaveShock; done 21 frames on |
| fire | 0.013 (0x3c54fdf4) | grows 0.8, fades 0.013; done 21 frames on |

The first missile carries `bossCam` and `MagicDamage` (0x0049a7d0) as its
callback. After impact it shakes the camera (10, 10, 10) each frame.
`MagicDamage(mode)` at its first call is skill row `mode` (5 ice, 6
lightning, 7 fire) on the target. Its first 20 calls shake by
`QuakeVector`. At the 60th it ends the cinema, goes to act 0, unlocks
the player and sets the camera's mode 6.

## `SkillAttack`

`SkillAttack` (0x00499110) at `Cou` 1 locks the player and turns cinema
70 on. It sets mode 5, the eye 800 from the target (toward Innis) and 50
up. The view is Innis's place 250 up with x and y zero: its x and y are
the stack's leftover, `SetCameraEyes`' saved return address 0x00496fe4,
which the FPU reads as a denormal (zero). It leaves the targets and takes
the next word: 0 one of `Skill_VARIOUS_INIS`, 1 one of `Skill_DOWNER_INIS`
(`|ccRand() % n|`), else skill 1. At 30, `InisFieldGenerator` row 4 above
the target and SEs 225, 225, 257. At 60, `ccItemSkillRequest`. At 90 the
cinema off, blur `DefaultARGB`, mode 6, the pitch 0, back on the targets.
Each other frame raises the camera's pitch (+0xe0) by 0.01.

## The images: act 5 and `ccBoss02Slave`

`EnemyAttack` (0x0049a900) draws one `ccRand` (dropped) when `PP` is at
least half `maxPP`, turns cinema 8 on and goes to act 5. Act 5
(0x00495448):

1. Lock the player, forbid the menu, open menu 74 on stream 39 +
   `MonsterID` (39-41, mask 0).
2. Once the menu closes: mode 5, the eye 1500 behind the target and 350
   up, looking at Innis.
3. Each image gets `SetMonster(MonsterID)`, the target's place, and
   `EntrySlave` (the first exited one, ordered out).
4. When all three are done: unlock, mode 6, `MonsterID` + 1 (mod 3), act
   0, cinema off.

An image (`ccBoss02Slave::Think`, 0x0049cce0), ordered out in act 0, takes
Innis's place, faces the other way, is drawn and goes to act 7.
`SetStartPosition` (0x0049c810) places each one from behind the camera:

| image | offset | start count | spins (`ccRandF`) |
| --- | --- | --- | --- |
| 0 | (0, 0, 500) | 0 | 0.04, 0.1, 0.01 |
| 1 | (500, 0, 500), turned by -pi | 30 | 0.01, 0.2, 0.3 |
| 2 | (-500, 0, 500), turned by -pi | 60 | 0.03, 0.09, 0.3 |

In act 7 the count runs down (SE 61 at 1), then each frame it spins
(`SetLotate`) and steps at the target (`SetTan`: 150, 100 within 100,
climbing by `atan2(posP.z, dist)`). Within 70 of the target it bursts:
it stops drawing, and `EnemyBurst` (0x0049d1b0) starts `BurstGenerator`
rows 3 and 4 and makes two SamonRings. Their model is `@2702[MonsterID]`,
scale 1.5, `spoint` 0.5, `tpoint` 0.04. SEs 56 and 66 play, the quake
runs 40 frames ((30, 34, 40), then (10, 8, 7) from 20), and skill row 8
lands at its place. Its mirror then breaks; once the shards are done it
exits.

`BreakMirror` (0x16f0 bytes) throws 60 shards. `SetData` (0x005340e0):
speed 80 + 10 x `|ccRand() % 6|`, spins `ccRandF`(0.4, 0.6, 0.5),
60 `rand()` drawn and dropped, launch 55 degrees, heading
`ccRandF(3.14)`. `Mbreak` (0x00534430) steps each shard with gravity 9.8,
dt 0.6. Below 5 and falling it checks `ccLandHitCheck(pos, 0x20000000)`:
under the ground, its run takes 0.7 and its fall -0.8. A shard shows
while its run is at least 20 and it is above -300; with none showing it
is done. The shards' Z turn accumulates: `Mbreak` never resets its matrix
between shards, so each is turned by all the turns before it. When done,
`MainMirror` calls `SetData` again (another 300 `ccRand` and 60 `rand()`)
and deletes the models.

## `Affect`

`ccBoss02::Affect` (0x004960f0) is the base's gate and switch (boss.md):
nothing but 21 while `actForbid` is 2 or the player is locked. 21 sets HP
and maxHP to maxHP / 10 (3000), `PP` -1, `PPcount` 0. 13 is act 11. A hit
(1, 3) of a spell (`check_type_of` 1) also calls
`effResistantShield(this, 1, -1)`. There is no Super table.

## The cinema's names

`_g_cinemaSkillName` (MUT 0x00619320, 72 rows of 12 bytes: file, texture,
row) names Innis's: rows 5-8 are x21's `TEX_ini_skl` rows 0-3, row 70 its
row 4. Mutation's `CinemaOn` accepts n < 79 (Infection's 61).

## In the port

`piney_battle::boss::innis` holds the rules. `Boss.class` is
`Class::Innis` with the members above, and `Boss::main` sends it there.
The tables come from the build (`tables::combat`: `innis_pattern`,
`innis_epitaph`, `innis_anims`, `innis_various_skills`,
`innis_downer_skills`, `innis_monster_anims`, `innis_ring_models`,
`cinema_skill_names`) through `BossData`. The missiles' flights are rules
(they call `MagicDamage`), run in the manager's pass. The camera's modes,
eye and pitch are `Out::CamMode`, `FreeCam` and `CamPitch`, which
`piney_world::bosscam::BossCam` applies. The blur's colour, the
particles and the shield are `Out::Blur`, `Particles` and `Shield`.

In the field, `FieldWorld::start_boss(code)` reads the event's entry code.
It loads x21's `CMP_trall` and clips, the images' `CMP_ex21mon1`-`3` and
the cinema's names, then makes Innis. Its images are actors while drawn.
The area host answers `present/absent 7 1` from the task's exit.

`tools/test_battle_innis_rs.py` (`PINEY_VOLUME=mutation`) builds
Mutation's `ccBoss02` with its own constructor in eemu. It runs `Main`
with the manager's pass against battle_probe's `innis` request, frame by
frame over random cases. A case scripts Kite's hits, the gauge and its
break, the drain's 13 and 21, menus and the camera. It compares the
acts, words and members, movement, target, HP and gauge, flags,
animations, the images and their mirrors, the effects, the camera's mode,
eye and pitch, the blur, the party's hold, the calls, the menu's words
and both random generators. Its docstring lists what is not the game's.

## Not described yet

- The pictures: the missiles, the SamonRings, the particle generators
  (`InisField`, `Tornado`, `Burst`), the blur, the shield and the mirrors'
  shards are not drawn. The MagicSquare draws only for n 0 (Skeith's).
- The Epitaph's body (`ANM_ex2x*` clips play on x21's `CMP_trall`; which
  clump the game draws them on is not checked).
- Mutation's `CamMain` lift (+0xdc) is always 0 in these rules and not
  ported. `SetTransfer` cases 3 and 4 and `ZrotControlFlg` are left out.
- Act 7 (the shake) on Innis itself: no harness case reached it, and what
  sends Innis there is not checked.
