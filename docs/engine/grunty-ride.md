---
title: The riding Grunty - ccPucciguso, the Grunty Flute and pgRideFlag
status: partial
volumes: INF, MUT, OUT, QUA
covers: INF gcmn.prg:0x00510650 ccPgAdultCheck, 0x005109c0 ccPuccigusoStart, 0x00510880 ccThPucciguso, 0x00510990 ccThPuccigusoDelete, 0x00510bd0 ccPuccigusoExit, 0x00510f30 ccPucciguso::ccPucciguso, 0x00511410 ccPucciguso::~ccPucciguso, 0x005114e0 ccPucciguso::Main, 0x005119c0 ControlMove, 0x00511ed0 PadLeverPower, 0x00511f80 AnimCtrl, 0x00512360 CollisionTest, 0x005123f0 MapLoopAdjustPos, 0x005124b0 CameraPosCalc, 0x00512540 CameraPosSet, 0x00512670 PawSmoke, 0x00512c80 DrawPG, 0x00512fb0 ccPuccigusoCheckNote, 0x005ee810 pcgsTbl, 0x005ee840 puccigusoCharTbl, 0x005ee870 puccigusoAnimTblPG, 0x006aea50 puccigusoAnimTbl, 0x005ee908 puccigusoAngleTbl, 0x005a0760 ccSpcSleep, 0x005a0850 ccSpcWakeup, 0x005a02e0 ccSPC::Wakeup, 0x005a08b0 ccSpcGetCharPtr, 0x00530264 ImportantItemMenu's proccess 20, 0x0057bc84 ccUseItemRequest's ride wait; INF SLUS_202.67:0x001801d0 ccPgBgmInit, 0x00180360 ccPgBgmEnd, 0x0017e680 ccBgmPlay, 0x001604d0 ccScFade::CheckFade, 0x00160510 ccScFade::DeleteFade, 0x0015c210 ccSprite::Trans, 0x00378cdc pgRideFlag, 0x00378c28 pgR, 0x00378c2c pgDIN, 0x00378c24 pcgs, 0x00378978 camTypeLock; MUT gcmn.prg:0x0052d540 ccThPucciguso, 0x0052d6f0 ccPuccigusoStart, 0x0052d960 ccPuccigusoExit, 0x0052dd10 ccPucciguso::ccPucciguso, 0x0052e3a0 Main, 0x0052e970 ControlMove, 0x0052fd10 AnimCtrl, 0x00530160 CollisionTest, 0x005cbb40 ccSpcSleep, 0x005cbb70 ccSpcWakeup, 0x00531090 the ride is up, 0x005310b0 the ride put down
worklog: 166, 395
---

# The riding Grunty - ccPucciguso, the Grunty Flute and pgRideFlag

A grown Grunty raised in Dun Loireag's pens ([town02.md](town02.md))
earns the Grunty Flute (key item 49). Blown in a field it calls a grown
Grunty: the screen fades to black, the party is put to sleep, and Kite
rides the Grunty (`ccPucciguso`, `source\pgrider.cpp`, gcmn
0x00510880-0x005130ac) with the left stick until the cancel button sets
him down with the party about him. Riding, the enemies go home and pay
no heed, the party's AI keeps still, and the music is `BGM.BIN`'s first
track. The port is `crates/piney-battle/src/ride.rs` (the object's
rules), `crates/piney-world/src/combat/ride.rs` (its frame on the field's
tasks, its two clumps), `crates/piney-world/src/field_world/ride.rs` (the
party asleep and awake, the flags) and `crates/piney-game/src/area/ride.rs`
(the sequence on the fader, the menu and the music).

## The call

Key Items (`ImportantItemMenu`, menu 6,
[field-ui.md](field-ui.md#key-items-importantitemmenu-menu-6)) takes the flute to
proccess 20 only in a field (not fields 13 and 67, not in battle). From pen
`rand() % 3` round the three it asks `ccPgAdultCheck(game.server, pen)`
(gcmn 0x00510650, the last of `pgbreed.cpp`) for the first grown Grunty:

```text
server 0 or pen >= 3     -1
server outside 1-4       -146 (the switch's -1, less 145)
growth[server].type[pen] (saveData +0x2194 + 0x18 server + 0xe + 2 pen)
  0                      -1
  pen 0                  0
  pen 1, 2               2 server + pen - 2 (1-8)
```

`game.server` is the server of the town whose gate led to the field
(`ChangeScene`'s table: Mac Anu's Delta 0, Dun Loireag's Theta 1, ...), so
the flute calls Dun Loireag's Grunties only in Theta's fields; in Delta's
it finds none ("There are no Grunties you can call ..."). With a
kind `n` it calls `ccUseItemRequest(plw, plw, 0xf0031, n)` and shuts the
menu when the call returns (after the ride).

The use (`piney_battle::item`'s `grunty`) is: the player's `noDeathFlag`
set (kept), `CloseMenuDisp`, `panelStatus` 3, `ccWakeAllThread`, a frame,
`StillOff`, 8 frames, the message closed at once; then, on the menu task,
`while (checkPartyAnnihilation() || compulsionGameOver)` a frame of
`ccDamUprStr::CtrlAll`, `ccChatMsg::Disp(0)`, `ccSprite::Trans` of the
menu's sprite and `menuWin`, `ccBreathThread(1)` (no `ccMenuCtrl::Disp`:
nothing of the menu is drawn, `Trans` only loads the sprites' textures);
sound 81; `ccClearConditionAllEnemy`; `ccPuccigusoStart(n)` (kinds past 8
as 0); `plw.pauseSW` 0; the same frame loop `while (pgRideFlag)`; the
`noDeathFlag` back.

## ccPuccigusoStart

`ccPuccigusoStart(kind)` (0x005109c0) runs on the menu task:

```text
game.area != 1 or pgR     nothing
ccMenu forbid (+0xfe) 1, panelStatus (+0xc) 3
pgRideFlag 1, pgR 1, pgDIN 0, camTypeLock 1
ccPgBgmInit
f = scFadeDef->EntryFade(10, 0, 0x80000000, 0, 0, 512, 384)
while CheckFade(f): ccBreathThread(1)          the screen to black
ccSpcSleep
for i in 0..5: ch = ccSpcGetCharPtr(i)          the registry's characters
  partyFlag (+0xe0 bits 14-16) 1: HitDisable (bodyHit +0x1a0),
                                  ClearConditionEffect
ccLoadFLAddOne({11, puccigusoCharTbl[kind]})
tcb = ccStartThread(ccThPucciguso, 49, 0x800); tcb +0x14 = kind
ContinueFade(f, 15, 0); while CheckFade(f): ccBreathThread(1)
DeleteFade(f); camTypeLock 0; ccBgmPlay(0)
```

`CheckFade(i)` (main 0x001604d0) is the element's `cnt < tcnt`;
`DeleteFade(i)` sets its status 0. `ccSpcSleep` (0x005a0760): each
registered character (`ccSpcManager` +0x1c + 44 i, id not -1) standing
(`moveFlag` 0, `stopFlag` 1, `runFlag` 0), off its command list
(`ccDeleteCmnd`) and its task asleep - Kite's `ccThPlayer` included, so
nothing of Kite or the party is drawn while he rides.
`puccigusoCharTbl` (0x005ee840) is `cdogbod0` and `cdogbod2`-`cdogbod9`
(kind 0-8); `ccPgBgmInit` is below. `camTypeLock` keeps `cameraMain` from
switching the camera's type on L2.

## The object

`ccThPucciguso` (0x00510880, the task named `PUCCIGUSO`, its delete
function `ccThPuccigusoDelete`) makes `pcgs = new ccPucciguso(kind)`
(0x1d0 bytes over `ccChar`), runs `Main` 30 times, then `Main` each frame
until, at the top of a frame, the pad's push (`ccSys->pad[0]` +0x2d0) has
a bit of `saveData.assignPadCancel` (+0x8410) and `pgRideFlag` is set:
then `ccPuccigusoExit` and the task ends. So the cancel button is looked
at from the 32nd frame.

The constructor (0x00510f30):

```text
+0x00 base        pcgsTbl (0x005ee810: +0x18 height 120, +0x1c width 120)
+0x40 pos         plw.pw's; +0x50 posP FZeroPosition; +0x60 rot plw.pw's
+0xe0 flags       bit 3 (stopFlag) only: 0 pauseSW, 2 moveFlag, 4 runFlag,
                  5 the eye view hides Kite; 1 unused
+0xe2 act         2; +0xe4 actOld -1; +0xe6 the clips' end 0
+0xe8             440: the idle count (the first fidget 11 frames in)
+0xec speed       60.0; +0xf4 nowSpeed 0; +0xf8 setTransparency 1.0
+0x100 kind; +0x104 the frame count, rand() >> 3
+0x140 angle, +0x150 movePos, +0x160 the eased move: (0, 0, 0, 1)
+0x170 bodyHit    ccCharHit: mask2 0x40000001 (the walls), type 0x1000000,
                  radius 100, height 100, pos the ride's; SetHitSW(1)
+0xcc/d0/d4       ctu1body's stream, a CMP_trall clump, its ccAnm
+0x1c0/c4/c8      the kind's stream, clump and ccAnm (its note function
                  ccPuccigusoCheckNote)
```

Both players start on act 2's clips. The acts' clips are
`puccigusoAnimTbl` (0x006aea50, 21-byte rows) for Kite and
`puccigusoAnimTblPG` (0x005ee870) for the Grunty, whose eighth character
the constructor overwrites with the kind's digit (`'0'` for kind 0, `'0' +
kind + 1` for 1-8: `cdg1` is the young Grunty's):

| act | Kite | Grunty | |
| ---: | --- | --- | --- |
| 0, 2, 4 | `ANM_ctu1dgn0` | `ANM_cdgNnut0` | standing |
| 1, 3 | `ANM_ctu1dgn1` | `ANM_cdgNnut1` | the fidget |
| 5 | `ANM_ctu1dgr0` | `ANM_cdgNrun0` | running |
| 6 | `ANM_ctu1dgw0` | `ANM_cdgNwal0` | walking |

## A frame: Main

`ccPucciguso::Main` (0x005114e0):

```text
flags.pauseSW = plw.pauseSW; movePos = (0, 0, 0, 1)
ControlMove
hp = pos + movePos (x, y, z; w 1); hp.z = ccLandHitCheck(hp, 0x20000001)
body.pos = hp; body.radius = 100 + nowSpeed
CollisionDetection:
  hp = pos + movePos + offset; its ground; CollisionDetection again:
    hp2 = hp + offset, its ground; hp = (hp + hp2) / 2, its ground
  d = ccHitCheckLM2(pos + height, hp + height, 0x40000001)
  a wall: d -= radius; past 0 halved; movePos = d * normalize(hp - pos, z 0)
  none: movePos = hp - pos
else: a wall between pos and hp (raised by the height): movePos = 0
pos.x += movePos.x; pos.y += movePos.y
CollisionTest; hitAttribute = checkHitResultAttlibute(); MapLoopAdjustPos
both clumps' SetMatrix_PosRotZYX(pos, rot)
plw.pw->pos = pos, ->rot = rot, plw.pauseSW = flags.pauseSW
CameraPosCalc; CameraPosSet; AnimCtrl
plw +0x10 = the active camera's eye
transparency = setTransparency = eye view hides Kite ? 0 : +0xf8
ccChar::Draw (Kite's clump); DrawPG (the Grunty); the count + 1
```

`CollisionTest` (0x00512360) stands it on the ground (`pos.z`,
`posP.z`); with `dneFlag` 0 and the ground's attribute bit 0x80000 (a
dungeon's way in) `pgR` 0, `pgDIN` 1 and `WORLD_MAN::Enter(pos)`.
`MapLoopAdjustPos` (0x005123f0) is `WORLD_MAN::AddCenter(movePos.x,
movePos.y)` and `ccTransPosW2M(pos, pos)` (Kite's `W2MPos`: the field's
wrap), stood again when it wrapped. `CameraPosCalc` (0x005124b0) puts the
camera's target 190 above the ride and the eyes 250 above, the eye view's
heading the ride's; `CameraPosSet` (0x00512540) by
`activeCamPtr->type`: 1, `cameraSetEyeLevel(posEye, angle)` and with
`checkCameraID()` 1 the ride turns to the view's heading and Kite is not
drawn (flag bit 5); otherwise `cameraSetManual(posView)`; then
`cameraSet()`.

### The stick: ControlMove

`ControlMove` (0x005119c0) reads `powL` through `PadLeverPower`
(0x00511ed0: under 64 nothing, 64-127 to `(p - 64) * 3 / 2 + 32`, from 128
as it is - `ccPlayer`'s without the target's first frames); `pauseSW`
makes it 0; `nowSpeed` 0.

- No lean: `moveFlag` 0; the eased move (+0x160) goes 1/8 of the way to
  nothing (`e += 0.125 (0 - e)`), `movePos` takes it, its z 0; every
  fourth frame (`count & 3` 0) with its length over 10, `PawSmoke` from
  all four legs thrown along `RAD2DEG(atan2(e.y, e.x)) + 16384` (the
  `atan2` in doubles).
- A lean: `speedRate = power / 255`, `moveFlag` 1, `runFlag` past 240; the
  stick's heading, straight ahead while a camera reset runs and stays
  within 2047 of it (`abs(reset - stick)`, cut to a short), else the reset
  ends; the heading `(short)(RAD2DEG(pi + camera.z) - 32768) -
  RAD2DEG(pi + stick)`; walking `6.2 * min(power / 140, 1.3)`, running
  `60 * power / 255` (+0xec); `movePos` that along the heading, then eased
  0.08 of the way (`e += 0.08 (move - e)`) and `movePos` the eased move.

It turns to the heading when leaning and not in the eye view.

### The acts: AnimCtrl, the notes, the dust

`AnimCtrl` (0x00511f80):

```text
act < 4: the idle count + 1; at 451 back to 0, 2 -> 3, 0 -> 1
else the count 0
a clip ended: 3 -> 4, 1 -> 2
stopFlag and moveFlag: stopFlag 0, act 6
neither: stopFlag 1, 5 or 6 -> 2
moving: runFlag and 6 -> 5; not and 5 -> 6
a new act: both clips set
frameSpd: act 6 fptoui(2 * 256 speedRate), act 5 fptoui(256 speedRate),
          else 256 - both players
both step (the second's end kept), the Grunty's notes processed
actOld = act
```

`ccPuccigusoCheckNote` (0x00512fb0), the Grunty's note function: notes 1
and 2 are `ccSeSetParamInu(param, pcgs)` (the dogs' table, `inuSeData`:
params 0 and 1 the footsteps on the ride's ground attribute
([sound.md](sound.md)), the others the Grunty's cries); running (act 5)
with dust from all four legs along its heading, a fidget's (act 3) param 0
with dust from the front two. Other notes do nothing.

`PawSmoke(dirc, legs)` (0x00512670): for each leg in `legs` (1 `OBJ_t0 fl
leg2`, 2 `fr leg2`, 4 `rl leg2`, 8 `rr leg2`) the node's world place
(`GetObjAdrsF`, `_SetLWMatrix` or its local matrix at the root) and a
speed `(0, 0.1 * 0.075 * -speed * (10 - rand() % 5), 0)` turned by `dirc`
- one `rand()` a leg, the last leg's taken, life 35 (front) or 37 (rear),
size 3. On the ground under it (`ccLandHitCheck(pos, 0x20000000)`), by
`checkHitResultAttlibute() & 0xf0f0f0`: 0xb0c000, 0xc0d000, 0x60b0d0,
0x70c0e0, 0x8080f0 no dust; 0x304050, 0x405060, 0x507080, 0x7090a0,
0x90b0c0, 0xc0c0c0, 0xd0d0d0, 0xe0e0e0, 0x404040, 0x505050, 0x606060
type 4; else type 133: `effSmoke(pos, v, 3.0, life, type, 512, 32)`.

### The draw: DrawPG

Kite's clump is drawn by `ccChar::Draw` as any character is
([field-game.md](field-game.md)), at its base's size (120). `DrawPG`
(0x00512c80) is the same for the Grunty's `ccAnm`: on the character layer;
in the eye view at full (+0x88 and +0x8c 1.0, the fade 1.0); the fade
`ccGetCameraTransparency(pos, 120, 120, far, len, &hide)` at the town's
4000 over 400 in area 0 and the field's 7000 over 600 elsewhere;
`transparency = setTransparency * fade`; the draw environment's alpha
`(fptosi(256 t) + 1) >> 1` of `setTransparency` when `hide` and of the
transparency otherwise, +0xa0 three times the height (the shadow's
length); nothing drawn under 0.05 unless `hide`; on ground with attribute
bit 0x40000 the ambient halved and the distant light asleep for the draw.
Its fog blend (`ccChar` +0xa8-+0xac) is never set on the ride.

## The dismount: ccPuccigusoExit

`ccPuccigusoExit` (0x00510bd0), from the task (or `ccThPuccigusoDelete`
when the task is deleted while `pgRideFlag` is set, as a scene change
does):

```text
pgDIN (a dungeon's way in), or game.area != 1:   ccPgBgmEnd(1)
else:
  camTypeLock 1; ccDeleteCmnd(pcgs); HitDisable(pcgs body)
  f = EntryFade(10, 0, 0x80000000, ...); while CheckFade(f): Main, breath
  pos, rot = plw.pw's
  for i in 0..5: ch = ccSpcGetCharPtr(i), partyFlag 1:
    not down (condition.dead +0x08 0): HitEnable
    base->id (+0xc) not 0 (not Kite): rot = plw's; pos = plw.pos +
      150 (sin a, -cos a), a = RAD2DEG(rot.z) + puccigusoAngleTbl[i]
  ccSpcWakeup; ccMenu panelStatus 1
  ContinueFade(f, 15, 0); while CheckFade(f): breath; DeleteFade(f)
  ccMenu forbid 0; ccPgBgmEnd(0)
~ccPucciguso; ccFileListDeleteOne({11, puccigusoCharTbl[kind]})
pgRideFlag 0, pgR 0, camTypeLock 0
```

`puccigusoAngleTbl` (0x005ee908) is 0, 24576, -24576, 28672, -28672: the
first two members 135 degrees either side behind Kite. `ccSpcWakeup` is
`ccSPC::Wakeup` (0x005a02e0): each registered character in the party with
`skillID`, `skillStatus` 0, `actNum` 0 and `actNumOld` -1,
`armsEffectSW` 0, `restraintSW`, `trajectorySW`, `pauseSW`, `moveFlag`,
`runFlag` 0 and `stopFlag` 1, `ConditionAdjustment`, the weapon trails'
counts cleared, `ccEntryCmnd` and its task awake; then
`ccAISysMsgSend(0x1000e, -1, Kite's AI's id, 0xffff, 0, 15)`, which a
member answers with `ChatMessageQuitPucciguso` ("This is a weird Grunty."
and the like, [battle.md](battle.md)).

The menu task's `WaitRide` loop sees `pgRideFlag` 0 on the next frame,
puts the player's `noDeathFlag` back and shuts the menu.

## The music

`ccPgBgmInit` (main 0x001801d0): `ccSnd` +0x136 1 and the first free voice
slot (+0x140) set to 0x120 - channel 0 stopped, as `ccEvVoiceStop` does -
then sequence 0's fade and, with two or more sequences, sequence 1's set
inline as `ccSqFade(n, 0, 8, 3)` would: out over 8 frames and stopped.
`ccBgmPlay(0)` (0x0017e680) queues `BGM.BIN` track 0 (`+0x13c` 0, +0x135
1, `bgmWavCmd` 0x8020) unless a command is queued.

`ccPgBgmEnd(n)` (main 0x00180360): unless a `BGM.BIN` command is queued,
`bgmWavCmd` 0x120 (channel 0 stopped); with `n` 0 and play type 0 or 1,
`ccSqPlayVol(0, 0)` and sequence 0's fade set inline as `ccSqFade(0, 256,
10, 1)`: the field's music back in over 10 frames; then +0x60 (the battle
music) 0. `bgmChange` does nothing while `pgRideFlag` is 1
([sound.md](sound.md#battle-music-bgmchange)).

## What reads pgRideFlag

- `ccEnemy::selectTarget` (gcmn 0x00436cb0): with no one to target it
  sends the enemy home without its wait (`actCnt` 1) unless riding
  ([battle.md](battle.md)).
- `ccAI::Reconnoiter` (0x00592aa0): riding, the party's AI does not look
  about.
- `ccMenuCtrl::Disp` (+0x5350, the noise), `ccUseItemRequest` (the ride's
  wait), `ccSound::bgmChange`.
- `ccThPlayer` clears `pgRideFlag` and `pgR` as Kite's task starts (a new
  scene).

## From Mutation on: the town and the race

Mutation's ride (MUT gcmn 0x0052d540 on) is Infection's with a town path
for the Flag Race ([flag-race.md](flag-race.md)):

- `ccPuccigusoStart` (0x0052d6f0) also starts in a town (`game.area` 0).
  There it plays no music and no fades, and sleeps Kite alone
  (`ccSpcSleep(town)`, which the symbols call `ccSpcWakeup`, 0x005cbb40).
  It slots 1 member, not 5. The task gets the kind (+0x14) and the town
  flag (+0x18).
- `ccThPucciguso` (0x0052d540) in a town sets the ride's pause flag at
  the start. It ends when the race's `+0xa7` is set, not on cancel.
- `ccPuccigusoExit` (0x0052d960) in a town has no fades and wakes Kite
  alone (0x005cbb70). The panel, the forbid and the music stay the race's.
- The object (0x0052dd10) is 125 high. Its handling (top speed,
  acceleration, the turn's ease on and off) comes from the race
  (`+0x64..+0x70`) when one runs. `Main` and `ControlMove` read the pad
  through the race's pause. In a town, `CollisionTest` skips the field's
  loop and `AnimCtrl` reads the race's input.
- 0x00531090 asks whether the ride is up; 0x005310b0 puts it at a place
  and heading (the race's finish).

The port: `piney_battle::ride`'s `Ride::later`, `main_later`,
`control_move_later` and `RaceRide`, and `piney_world::town_ride`
(`ride_start_town`, `ride_slot`, `ride_exit_town`, `ride_place`, the
town's `RideWorld`).

## The port

`piney_battle::ride` is the object and its functions: `Ride::new`,
`main`, `control_move`, `pad_lever_power`, `anim_ctrl`, `check_note`,
`paw_smoke`, `collision_test`, `map_loop_adjust_pos`, `camera_pos_calc`,
`camera_pos_set`, `draw_pg`, `exit_place` (a member's place) and
`adult_check`, over a `RideWorld` (the collision, the camera, the two
players, the legs, `ccChar::Draw`) with the rest as `Out`s.

`piney_world::combat::ride` keeps `pgRideFlag`, `pgR`, `pgDIN`, the
party's sleep and the object (`Combat::ride`); `ride::frame` runs where
`ccThPucciguso` does in `Combat::frame` (after the sleeping `ccThPlayer`,
before the members'), over the frame's `Stage`, the object made on its
first frame over Kite's own `ctu1body` (his clut swaps) and the kind's
`CMP_trall`. While the party sleeps `kite::main` and the members' frames
do not run and nothing draws them; the flag goes to `selectTarget`
(`enemy_ai::World::ride`) and the party's AI (`Game::pg_ride_flag`). Its
sounds (`ccSeSetParamInu`) and dust (`effSmoke`) are `Show::Ride`s for
piney-game's sound and effects; its two clumps are drawn with the cast
(`Char::draw`, each with its shadow).

`FieldWorld`'s `ride_start`, `ride_sleep_party`, `ride_create`,
`ride_start_done`, `ride_exit_begin`, `ride_exit_wake` and `ride_end` are
the slices of `ccPuccigusoStart` and `ccPuccigusoExit` that touch the
field; piney-game's `area/ride.rs` runs the sequence on `scFadeDef`, the
menu (`forbid`, `panelStatus`) and the music (`Event::PgBgm`:
`Audio::pg_bgm`, the driver's `pg_bgm_init` and `pg_bgm_end`; `BGM.BIN`
track 0 by `Event::BgmStream`). The menu's side (`useitem.rs`): the flute
is `call_arg` with `Resume::Flute`; `WaitParty` and `WaitRide` breathe
with the chat balloons only; `Pucciguso(n)` breathes while the start runs
(`World::pg_starting`).

The menu task runs before the field's tasks in the game and after them
in the port, so the start's slices past its fades run before the menu's
frame and the task's before the field's; the menu sees `pgRideFlag` as it
was when the frame began. The ride's task first runs a frame later than
the game's (the screen is black then), and so the cancel button is looked
at a frame later too.

## Checks

`tools/test_ride_rs.py` runs the object's functions in the game's code in
eemu against the `ride_probe` example: the tables; the constructor for
every kind; `Main` for 1-80 frames on random states (a field's map and its
edges, any act and flags, the camera's types, resets and eye view, the
script's ground, bodies, walls, clips' ends, notes and legs), every member,
`pgR`/`pgDIN`, `plw`, the camera's reset and `rand()` compared after each
frame with every call in order; `ControlMove`, `AnimCtrl`, `DrawPG`,
`ccPuccigusoCheckNote` and `PawSmoke` alone; `PadLeverPower`;
`ccPgAdultCheck` over servers -1 to 6 and pens -1 to 3; and
`ccPuccigusoExit` and `ccPuccigusoStart` run natively (their fades at
once): the members' places and headings, and the order of their calls,
which the port's sequence follows. `tools/sound_ee.py`'s fixtures run
`ccPgBgmInit` and `ccPgBgmEnd` in fields of each play type
(`crates/piney-audio/tests/driver.rs`, `voice.rs`).

`the_grunty_flute_rides_and_dismounts` (piney-game session): a field of
Dun Loireag's server with the flute and a grown Grunty in pen 1; PERSONAL,
Key Items, the flute; the fade, the party asleep, the Grunty where Kite
stood, the music; the stick up: it runs and carries Kite; let go, it
stands; Circle: the fade with `Main`, the members 150 from Kite, the
flags cleared, the menu shut and unbanned, Kite walking again.
`ride_shots` (ignored) takes the fade, the ride standing and running,
the dismount and the party after it (`scratch/ride/shots`).

## Unknown

- The ride's task runs its first `Main` a frame after the game's, and the
  menu's `WaitRide` and start slices are ordered as above rather than by
  the tasks' priorities; nothing shows it on the black screen, but the
  cancel button is looked at a frame later than in the game.
- `ccSpcSleep` sleeps every registered character and `ccSPC::Wakeup`
  wakes only those in the party; the port sleeps and wakes the whole party
  at once (in Infection's fields the registry is the party).
- `plw +0x10` (the active camera's eye, which `Main` copies) is not kept:
  nothing the port runs reads it.
- `ccDamUprStr::CtrlAll` in the menu's waits (the flying numbers' step)
  is not run there; no number flies while the Grunty comes or rides.
- The Grunty's leg nodes are taken from the pose as the port last evaluated
  it; the game's `_SetLWMatrix` uses the node matrices the last draw left.
- `pgDIN`'s way (a dungeon's entrance ridden into) and the flute outside
  a field are not played through in a session; the entrance is
  `WORLD_MAN::Enter` as Kite's.
- From Mutation on, a field ride's charge (the bit 6 test at MUT gcmn
  0x0052f790, the knockback 0x0052f250, `+0x158`..`+0x160`), its Grunty
  chat (0x00530e50, 0x00530dd0) and the `+0x1e0` range are not ported.
  They act only in a field, which no later volume's session rides yet.
  Porting them needs those functions read and a Mutation field ride in
  the ride harness.
