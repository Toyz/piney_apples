---
title: Bosses - Kyvia's fourth fight
status: partial
volumes: QUA
covers: QUA gcmn.prg:0x003d9b70 ccThKyvia04 (bossFunc code 15), 0x003d9c40 ccBossKyvia04::ccBossKyvia04, 0x003da6d0 Action, 0x003dadb0 DeadKyviaVoice, 0x003dafe0 DeadKyviaCam, 0x003db430 DeadKyviaRipus, 0x003db820 Move, 0x003db950 Main, 0x003dbcf0 Think, 0x003dbef0 InitWallGpStart, 0x003dc7a0 DeadCheck, 0x003dc8e0 KyviaSwitchActionPattern, 0x003dcd70 KyviaExOn, 0x003dcf40 SwitchActionPattern, 0x003dd450 SwitchDeadCheck, 0x003dd590 CheckDiscMove, 0x003dda00 KyviaScene, 0x003dde40 act 24, 0x003de350 act 14, 0x003de820 CheckDmgSmokeEff, 0x003deb10 KyviaRndTarget, 0x003dec60 CheckCorePoint, 0x003decf0 SwitchGateON, 0x003defc0 SwitchGateOFF, 0x003df1b0 TogetherCharPos, 0x003df330 KyviaMagicDamage, 0x003df3d0 KyviaMagicalBallDamage, 0x003df4a0 Kyvia_HellE_Damage, 0x003df610 SetBPos, 0x003dfb00 HandAtk, 0x003e0490 LightAtk, 0x003e34c0 MegidFlame, 0x003e3c60 MagicBallAtk, 0x003e4370 ThunderboltAtk, 0x003e5470 BIGMegidFlame, 0x003e5a20 HellE_Atk, 0x003e5dc0 Skill_Atk, 0x003e62f0 ThunderDmg, 0x003e6d10 ThunderSound, 0x003e6ed0 Slave, 0x003e6fa0 EntrySlave, 0x0036ccd0 HellSpecialExpress::Clear, 0x0036cdc0 Init, 0x0036d790 HellStartScene, 0x0036da40 Draw, 0x0037bf40 HellSpecialExpressCreate, 0x003e97b0 kyviaCore::ActionStart_K, 0x003e9b30 kyviaCore::KyviaExModeOn, 0x003ea8b0 kyviaCore::EndFadeEX, 0x003eb490 CheckCountGomoraEX, 0x003ed830 kyviaGomora::KyviaExModeOn_G, 0x0030c620 EVENTAREAB8 body hit off, 0x0030c690 body hit on, 0x00363c60 ccBossCam::QuakeCam, 0x00360ea8 ccBoss::ChangeAction, 0x001225f8 __ieee754_atan2f
worklog: 434
---

# Bosses - Kyvia's fourth fight

`bossFunc` code 15 is Kyvia's last fight, Cubia's fourth: `ccBossKyvia04`
(kyvia04.cpp). Quarantine's event 312 enters it on field 12's disc
(`entry type=7 code=15`). The disc has four places, each with a core of
its own level, closed off by gates; Kite walks from one to the next as
each core falls. At the last the body itself wakes (the EX turn), and the
core comes back once (the rebirth) before the body dies. Names and layouts
are Infection's DWARF (kyvia04.cpp is in Infection's executable, unused);
the rules are Quarantine's code, which names only the thread and the
constructor. The rest come from the vtable at 0x00277fa0 and from each
function's calls in order against Mutation's named copy (the third
fight's method, [boss-kyvia-third.md](boss-kyvia-third.md)).

## Where it comes from

`bossFunc[15]` (0x003d9b70) makes `new ccBossKyvia04(4)` (0x29890 bytes,
Infection's layout as it stands) on `bossTbl` row 15
(`ccGetBossParam(15)`). Quarantine adds two words: +0x29614 `RebirthFlg`
and +0x29618 `RebirthCnt`; `CamRotX` is at +0x29634, the cinema's byte at
+0x29350 and Kite's arrival (`rising`) at +0x29351. The core is the
shared `kyviaCore` (its EX members: +0x293d4 `KyviaExFlg`, +0x29410
`GomoraKpos`); `Kyvia04AnmTbl` and `Kyvia04AnmTbl_EX` have 25 entries
from Mutation on (act 24, `ANM_ex04dmg0`).

`Main` is the third fight's with `cond.hold` cleared after `Slave`.
`Move` is `ccBoss::Move` with the collision's push kept whole (no ground
under the body). The constructor sets the body's hit off
(`bodyHitSW` 0), its place at the origin, `CamRotX` 0.13 and no
`RotXLimit`, and the places:

```text
place  CoreAppearPos              WallPos -> WallPos2
0      (-576, -9903, 910)         (-1633, -9180, 1000) -> (-1205, -8812, 1000)
1      (-2975, -7733, 910)        (-3061, -6186, 1000) -> (-2593, -6346, 1000)
2      (-733, -4163, 900)         (-720, -3601, 1000) -> (16, -4126, 1000)
3      (145, -2176, 900)          -
DeadSetPos (7, -1545, 1000)
```

## The places

`Think` runs `InitWallGpStart` on its first frame (`WallGpFlg`): each
place's light (`CoreAppGp`, `CoreAppGp2`) and walls (`WallGp` at
`WallPos`, `WallGp2` along `WallPos` to `WallPos2`), all but the first
place's paused. Then `CheckDiscMove` and `DeadCheck`, then the turn.

`DiscPos` is Kite's place. While the body waits (`StFlg` 1),
`CheckCorePoint` asks whether Kite is within 250 of
`CoreAppearPos[CoreLvCount]`; once he is, `CheckDiscMove` counts:

- 1: the flash (10, 5, 10), the player held, the place's light killed;
- 20: the party gathered at the place (`TogetherCharPos(pos, 0)`: each
  member 150 off at a random turn, at its own height, which is written
  back into `CoreAppearPos`);
- 30: the core risen (`EntrySlave`, at the next place), `MaxHP` the core's
  and `SwitchHp` a fifth, the player free, the place behind closed
  (`SwitchGateOFF`).

At the last place the 30 is `KyviaScene` instead: the rumble (SE 232
twice) each 40th frame, the last place closed at 10, at 30 the blur
(1.02, `ExARGB`) and a free camera before the body's point (0, -700,
1100), at 155 the body's light and roar (`ANM_ex04act0`), the eye rising
5 a frame (3 from 155), at 280 the camera back; then `KyviaExOn`.

When a core falls (`DeadFlg`) before the last stage, `SwitchGateON`
opens the next place: field 12's `EVENTAREAB8` lets Kite through the
gate's body (0x0030c620, `body[n - 1]`'s `HitDisable`), the walls behind
pause and the next place's run. `SwitchGateOFF` closes it again
(0x0030c690, `HitEnable`) and kills the walls two back. `HitEnable`
appends the model hit at the list's tail; `HitDisable` unlinks it.

The places' turn (`SwitchActionPattern`, at the core's `kAtkFlg` 2 and
state 0), by `AtkPatMode` and place:

```text
mode  place 1          place 2          place 3          place 4
1     arm (44)         arm (44)         ball (42)        ball (42)
2     thunder 0 (50)   thunder 1 (50)   thunder 2 (50)   -
3     thunder 0 (50)   ball (42)        hell (62)        -
4     ball, then 2
```

Mode 3 goes back to 1. The cinema's name is the number.

## The EX turn

`KyviaExOn`: `SetCoreBaseParam(5, -1)` (row 15), `FightFlg` 1,
`EntrySlave(0, 1)` with the core in its EX mode (`KyviaExModeOn`: state
10, alpha 1, at (0, -700, 900), its gomoras at (190, -2154, 950) with
attack time 90 for a dasher), the core's flags cleared, `SwitchHp` a fifth
of the core's `MaxHP`, the EX clips. The core no longer leaves: an attack
ends with `EndFadeEX`.

`KyviaSwitchActionPattern` reads the core's flags (its hit: act 1, its
damage: act 2) and at its turn picks by `AtkPatMode`: before the rebirth
thunder 0 (50) and the skills (63) in turn; after it the ball (43), the
skills (63), the beams (52), hell's express (62) and the great meteor
(55), which waits for the menu with the player held.

- `Skill_Atk` (act 23): the player held, the camera 4500 off the core
  turned by `ccRandF(0.15)`, 800 up; at 3, 10, 20, 40 and 50 one of
  `Skill_KYVIA_VARIOUS` from the core on `KyviaRndTarget`'s choice
  (`SelectTarget(|ccRand() % 15|, 1e6)`, else the first member alive).
- `BIGMegidFlame` (act 22): at 30 stream 111, then the flash, the camera
  before the body, the party gathered before it, the blur (0x70808080,
  1.02); at its tenth frame the great meteor's light and the core's boss
  skill 55 on Kite; at 90 the colour back.
- `MagicBallAtk` (act 20): the third fight's ball, gathered over Kite;
  `FadeNum` is the first free `scFadeDef` element when the flash is made
  (status 3), and the act waits on it (`CheckFade`: `cnt < tcnt`). In the
  EX turn the camera is put as `Skill_Atk`'s and the ball shakes the
  camera (10, 10, 10) from its 36th frame. `KyviaMagicalBallDamage`: boss
  skills 74 and 75 round the ball.
- `HellE_Atk` (act 21): hell's express at the place's centre.

## Hell's express

`HellSpecialExpressCreate(pos, HeDataSet{blur, cam, cb, target,
KyviaLV})`: `Init` sets the blur (0x65808080, 1.01) and the camera's mode
5, draws a spline from the place up and back by the camera's turn
(`cameraGetRot(camID)`; `KyviaLV` 3 the other way and further), the eye
1600 behind the place, three souls each 0.3 later, `ANM_ex31lhit` of
xeffect with `CLT_exx004c1` for `CLT_exx004`, and each soul's generators
(rows 18, 19, 21 and the last's 20). `HellStartScene` gathers (rows 26,
27, then 28 at 70) with the camera on the first soul. `Draw` moves each
soul 0.02 a frame along three legs; the first's landing makes row 22;
as the last lands the blur's colour falls a step a frame from 0x50, and
70 frames on the burst (rows 23-25, SE 69, 35, 56), the callback and the
souls' generators killed; the burst's clip twice a frame with a quake
(20, 20, 40); the blur back; the end. `Kyvia_HellE_Damage` strikes each
member alive from Kyvia, the target 80% of its HP and the rest 40% (at
least 400), and moves the act on.

## The rebirth and the death

`DeadCheck`: in the EX turn, the core's fourth death (`DeadCount` 4 and
`DeadFlg`) is the rebirth (act 24); after it, the core's HP out with the
party standing is the body's death (act 14, `ClearSpcCondition`,
`LiveFlg` 0). `SwitchDeadCheck` never kills the body here.

The rebirth: the player held, the body off the lists; at 2 the flash
(`EntryFlash3(5, 5, 5)`: status 9), the blur, the body's light and a free
camera before it; its cries (SE 191, 65 + `ccRand() % 10`) each eighth
frame; at the clip's end stream 85; at 35 `bossTbl` row 15's `maxHP`
written 9999 and `KyviaExOn` again (the core reborn on row 15, the body's
`SetBaseParam(15)`); at 36 the body back on the lists.

The death (act 14) runs only after the rebirth: the cinema 71, the boss's
gauge on the body (`ccMenu`+0x238, Quarantine's `dummyTarget`) filling
with the core's HP (+20 a frame), the flash, the blur, `ccSqFade`, a quake
(20, 10, 20); the party gathered at `DeadSetPos` at 11; the smoke, the
cries (`DeadKyviaVoice`'s 20 notes, unplaced), `DeadKyviaCam`, and
`DeadKyviaRipus`'s lights 700 before and 1500 up at 30, 50 and 170; then
the exit.

## Words the code reads but never writes

- `Skill_Atk`'s and the EX ball's eye vector: only y and z are written.
  Its x word is `KyviaSwitchActionPattern`'s `s0` (the `kAtkFlg` it read,
  2), saved there by `ccBoss::ChangeAction`'s prologue (0x00360ea8); 2 is a
  denormal and EE's FPU and VU0 read it as 0. Its w is 0.
- In the EX ball, the eye's x word is instead the work pointer
  `ccGetDirc` passes, saved by `__ieee754_atan2f` (0x001225f8): a heap
  address, a float near 1e-38. It shows only when `ccRandF(0.15)` gives
  exactly 0 (the turn's sine 0) and the core stands at x 0, as the EX
  core does; the eye is then 1e-38 off 0, which no view matrix keeps.
  The port takes 0; the battle harness writes 0 there in that case.

A poisoned stack (`PINEY_STACK_POISON`) in the harness shows every other
local the fight reads is written by the code first.

## Bosses' rows across fights

The rebirth's 9999 and the cores' added `maxHP` are written into
`bossTbl` (gcmn's data). They last while GCMN.PRG stays in the overlay
window, and only as long:

- a game over (`ccOpenGameOverMenu`) is `ChangeRequest(1, 7)`, the soft
  reset to the title, whose DEMO.PRG takes the window; play goes on only
  through `ChangeRequest(5)`, which loads GCMN.PRG again from the disc, its
  data with it ([overview](overview.md), [the overlay
  format](../formats/prg.md)). A retried fight meets the disc's rows;
- between the third fight (event 308) and this one, event 311's block 1
  runs only on the desktop (`game_status` 2, its mail 77) and its block 2
  opens area 108 on mail 79, so DESKTOP.PRG takes the window and the log
  in loads GCMN.PRG again. This fight's third place meets rows 36 and 37
  as the disc has them, not as the third fight left them.

So the port keeps the rows' changes on the fight (`Core::row_hp`,
`row15_hp`).

## The port

- piney-battle: `boss::kyvia` with `Fight::Fourth` (code 15, row 15,
  level 4, skills 44, 52, 38) and `kyvia::fourth` (the places, gates,
  scene, EX turn, rebirth, death, `BIGMegidFlame`, `Skill_Atk`,
  `HellE_Atk`, the fourth ball), `kyvia::hell` (`Hell` on a `HellHost`,
  `HellGen`), the core's and gomoras' EX mode, the acts 20-24.
- piney-gen: `kyvia04_anims` and `kyvia04_anims_ex` read to 25 entries
  from Mutation on.
- piney-world: code 15 (`KYVIA_FOURTH`, `x04`'s `CMP_trallex04`), the
  camera's eye (`Out::CamEye`), the gates on field 12's bodies
  (`Out::FieldBodyHit`), the fade elements the ball waits on, the boss
  gauge's `dummyTarget`.
- piney-effect: `kyvia::Hell` (the burst's `ANM_ex31lhit` with its
  palette swap, the souls' anchors); `Hell::alone` for the probe.
- piney-game: hell's express's picture, `dummyTarget` on the menu.

## Checks

- `PINEY_VOLUME=quarantine PINEY_KYVIA_FIGHT=4 tools/test_battle_kyvia_rs.py`
  runs `ccBossKyvia04(4)` natively against battle_probe's `kyvia 4` with
  Kite walked to each place, hell's express's members compared each frame:
  the unit cases and `bulk 25` (163,336 frames) agree, reaching acts 0-4,
  6, 14 and 20-24, the thunders and the gomoras.
- `test_kyvia_effects` (tools/test_boss_effect_rs.py) runs hell's express
  alone at each `KyviaLV` against boss_probe's `hell`: the Create's
  generators, ccRand and palette swap, and 3,224 frames of draws, sounds,
  generators following the souls, kills and its end agree.
- `session::tests::kyvia_fourth::kyvia_s_fourth_fight_to_its_end` plays
  event 312 from field 12's disc with the console's god, Kite set at each
  place while the body waits, the core hurried and struck: the four
  places, the EX turn, the rebirth, every act, hell's express drawn, the
  music, the body's death, and the event on to Lia Fail.

## Unknown

Nothing.
