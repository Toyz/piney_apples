---
title: Entering The World - Kite in Mac Anu
status: partial
volumes: INF
covers: INF SLUS_202.67:0x001687a0 ccSetupNewGame, 0x00175d30 ccSaveData::InitSpcParam, 0x00177010 ccSaveData::ChangeEquipment, 0x00176200 ccSaveData::SetSpcBaseMsg, 0x00168960 ccSetupGameCtrl, 0x001674a0 ccGame::ChangeArea, 0x00167380 ccGame::ChangeScene, 0x0019f8e0 WORLD_MAN::GO, 0x001a1190 WORLD_MAN::SetCharPosition, 0x001a4430 ccThFieldDisp, 0x00160610 ccThCamera, 0x00160a00 cameraInit, 0x00160cc0 cameraMain, 0x001611a0 setCameraCtrlType, 0x00161870 cameraPosCalc, 0x00162020 cameraSetEyeLevel, 0x00162fb0 checkCameraDistModValue, 0x00161260 cameraSet, 0x00162cd0 cameraShake, 0x001629b0 cameraShockAbsorber, 0x00162f10 checkCameraShakeRange, 0x00383e30 sfList, 0x0034b390 shakePowerTbl, 0x0034b3a0 shakeCycleTbl, 0x00161610 cameraGetRot, 0x001387a0 ccCam::SetMatrix_PosTarget, 0x00110ca0 sceVu0CameraMatrix, 0x001052c0 ccView::SetView, 0x0014ce50 ccStream::Decode_Hit, 0x00149990 ccSetHitData, 0x00153930 _ccHitCheckLM, 0x00153f60 collisionLM, 0x00153ce0 setNearest, 0x001545e0 collisionQM, 0x001552e0 ccHitResultGrouping, 0x00155600 ccModelHitCheckQ, 0x00155930 ccModelHitCheckQZ, 0x00153e80 checkHitResultAttlibute, 0x00153470 ccCharHit::CollisionDetection, 0x001da8b0 ccGetCameraTransparency, 0x001dab50 RAD2DEG, 0x001dabb0 DEG2RAD, 0x00127c28 sinf, 0x001278e0 cosf, 0x00124330 __ieee754_atan2f, 0x00127600 atanf, 0x001246e8 __ieee754_rem_pio2f, 0x00127d10 tanf, 0x00126728 __kernel_tanf, 0x00127e18 fmodf, 0x00124518 __ieee754_fmodf, 0x00162430 avoidObstacle, 0x001a10c0 WORLD_MAN::GetHeight, 0x00151cb0 ccAnm::SetLightEnv, 0x001a20b0 WORLD_MAN::SetActiveLayer, 0x0019d410 WORLD_MAN::RequestCCS, 0x0013d300 ccClump::GetObjAdrsF, 0x0013b8f0 ccObj::SetModel, 0x0013f220 ccObj::Draw, 0x00138380 ccCoord::_SetLWMatrix, 0x00133a38 rand, 0x001b62e0 ccEntryEventMng, 0x00150670 ccAnm::SetAnmCtrlWork, 0x001397e0 ccOmniLight::Init, 0x00139060 ccLightGrp::AddGrp, 0x001b2bb0 ccEvent::GetSpc, 0x001b2cc0 ccEvent::GetNpc, 0x001b2d50 ccEvent::GetEnemy, 0x001ae7d8 ccEvPcPos, 0x001aed5c ccEvPcRot, 0x001d9ce0 ccGetDirc, 0x00317da0 markerEvTbl, 0x001743d0 ccSaveData::Init, 0x001da710 ccCheckCameraDeg, 0x001da0b0 ccSetDirc, 0x001d9eb0 ccGetDircChg, 0x001d9900 ccInitRand, 0x001b7660 ccRegisterRandomNpc, 0x001b6cf0 ccEntryRandomNpc; INF gcmn.prg:0x0059ff50 ccGetStartPositions, 0x00597850 ccPlayerStart, 0x00597910 ccPlayer::ccPlayer, 0x00598310 ccPlayer::Main, 0x00598af0 ControlMove, 0x005992b0 PadLeverPower, 0x005993c0 AnimCtrl, 0x0059ba20 CameraPosCalc, 0x0059bac0 CameraPosSet, 0x0059b340 CollisionTest, 0x0059ee20 ccSpcChar::HitCheck, 0x0059b900 ccTransPosW2M, 0x0059b8d0 ccSetGroundHeight, 0x005b61d0 ROOTTOWN::GetHeight, 0x00571e00 ccLandHitCheck, 0x006f0560 playerAnimTbl, 0x00421470 ROOTTOWN01::ROOTTOWN01, 0x00423b10 ROOTTOWN01::Draw, 0x00422ba0 DrawFloor, 0x004224c0 DrawObj, 0x00422900 DrawObj2, 0x00422250 DrawBG, 0x0056b1c0 ccChar::Draw, 0x0059d650 ccSpcChar::EquipWeapon, 0x005a1530 ccMakeModelName, 0x00458df0 ccSetChaosGate, 0x00458ec0 ccEntryGimGate, 0x00458f60 ccChgate::ccChgate, 0x00459280 ccChgate::main, 0x004593e0 ccChgate::gateAnm, 0x00459570 chaosGateInfluence, 0x00570d30 ccGetGimmickParam, 0x0061e0f0 gimmickTbl, 0x0059b5a0 ccPlayer::W2PPos, 0x0042fa60 ccEntryObj::routine, 0x00431970 ccThEntryCtrl, 0x00505e20 ccMerchan::main, 0x00517800 ccThGameCtrl, 0x0059cd70 ccPlayerMenuCheck, 0x0059f550 ccSpcChar::CheckControlMode, 0x00518af0 ccSortCmnd, 0x00518cc0 ccSelectTarget, 0x00519240 ccCheckTargetRange, 0x00519630 ccEntryCmnd, 0x005198c0 ccChangeCmndTarget, 0x00519920 ccCheckTarget, 0x00581500 ccAI::SetDircZ, 0x00619460 npcTbl, 0x005057f0 ccSetMerchant, 0x00505590 setMerchant, 0x00505a50 ccEntryRtownMerchant, 0x00505aa0 ccMerchan::ccMerchan, 0x00505fc0 breederAct, 0x005065d0 breederInfluence, 0x00430c90 ccEntryCtrl::entryObject, 0x004317b0 ccEntryCtrl::entryNpc, 0x00430ec0 ccEntryCtrl::initObject, 0x00519700 ccDeleteCmnd, 0x005711f0 ccGetJobWeaponCategory, 0x00647cc0 playerDefaultItemList, 0x00647d20 playerDefaultImportantItemList, 0x00647d80 spcDefaultItemList, 0x005269d0 SetMerchantCamera, 0x005ed880 breederCamPos, 0x005ed8c0 breederCamView, 0x005ed870 merchanAnmPtr, 0x005d4050 fellowAnimTbl, 0x00506640 ccSetRtownPC, 0x005067f0 ccRtownPC::ccRtownPC, 0x00507c20 ccRtownPC::main, 0x00507de0 ccRtownPC::normalMode, 0x005077f0 ccRtownPC::move, 0x005076c0 ccRtownPC::changeTEX, 0x005091c0 rTownNPCInfluence, 0x005130b0 ccSetNaviMap, 0x00513720 ccNavi::RouteSearchByMap, 0x005ee120 markPosTbl, 0x005ed900 rtpcCcsName, 0x005edeb0 rtpcAnmTbl, 0x0051a500 ccCalcTagPosChar, 0x0051a640 ccCalcTagPos, 0x0056b020 ccChar::EntryAffect; INF demo.prg charTbl, 0x001b4ff0 ccThCameraExecute, 0x001b3680 ccEvent::CamCtrl, 0x001b4780 CamzCtrl, 0x001b48e0 CamzMovMain, 0x001b4b20 CamzInpCtrl, 0x001b4c80 CamzInpMain, 0x001617f0 changeCamera, 0x001614d0 cameraSetPos, 0x00161510 cameraSetView, 0x00161550 cameraSetRot, 0x00110b80 sceVu0RotMatrixY, 0x002fb6f0 cameraList, 0x00383dc0 ecam, 0x00383d50 bcam, 0x00161610 cameraGetRot, 0x00161820 cameraSetManual, 0x001631d0 cameraSoftReset, 0x00163230 cameraSoftResetParam, 0x001b2460 ccEvent::MenuBan, 0x001b25c0 MenuClr, 0x001b6d70 ccRegisterEventMng, 0x00159ed0 ccStartThread, 0x00159d40 GoThread, 0x0015a5c0 ccThControl, 0x001d9eb0 ccGetDircChg; INF gcmn.prg:0x004381d0 eneSplineB, 0x004382a0 eneSplineH, 0x0057c5f0 ccAI::ccAI, 0x0057ca00 ccAI::Brains, 0x00580ef0 ManualControl, 0x00583270 ManualMode, 0x005832e0 SetRemoteCmd, 0x00589740 ChangeMode, 0x0059eba0 ccSpcChar::ManualModeAI, 0x0059e950 SetBootStatus, 0x0059ea60 TransferIn, 0x0059ea80 TransferOut, 0x0059f5f0 ccSPC::Initialise, 0x0059f740 EntrySpc, 0x005a00d0 Reboot, 0x0059fe80 SetParty, 0x0059ce00 ccParty::InitParty, 0x0059ce80 AddMember, 0x0059cf60 DelMember, 0x005a08e0 inviteSpc, 0x005a0f50 disbandSpc, 0x00730340 ccSpcManager, 0x00730310 ccPartyManager, 0x0041ae80 ccFellow::Initialize, 0x0041b5f0 ccFellow::Main, 0x0041bc50 ccFellow::Move, 0x0041c670 ccFellow::Action, 0x0059b710 ccPlayer::P2WPos, 0x0057f660 ccAI::ActInTown, 0x00586420 ccAI::ChatMessageSender, 0x0058f810 ccAI::ChatMessageEnteredTown, 0x0041ed60 ccThFellow02, 0x0041ee90 ccThFellow02Delete, 0x0041ad60 ccFellow::~ccFellow, 0x0059d230 ccSpcChar::ccSpcChar, 0x00653c80 naviPointNameTable
worklog: 59
---

# Entering The World - Kite in Mac Anu

From the bulletin board's New Game to Kite standing in the first Root Town,
Mac Anu (`town01`), walking under the pad with the camera behind him: the
set-up the game runs, the tasks it leaves running, and what each does in a
frame. `crates/piney-world` is the port; `tools/test_world_rs.py` checks it
against the game's own functions run in `tools/eemu.py` (see
[Checks](#checks)). The model draw and the town's pieces are on
[the render page](render.md) and [town assembly](statics.md).

## The way in

TOPPAGE's New Game (`ccThToppageCtrl::_Exit`, toppage 0x00401458) calls
`ccGame::ChangeRequest(5, 8)` and `ccGame::ChangeArea(0, (s8) saveData->lastTown)`:

```
ChangeRequest(5, 8)     request[] <- 5; InitScene: area .. block and every *Prev = -1
ChangeArea(0, t)        ChangeScene(0, t, -1, -1, -1, -1)          0x001674a0
ChangeScene(a, t, ...)  townPrev = town; town = t; lastTown = t    0x00167380
                        server = {0,1,2,3,4,0,1,2}[town] (0x00306dc0)
                        field, dungeon, floor, block: prev = old, new unless < -1
                        areaPrev = area; area = a
                        inBattle = inBattleCnt = 0; inBattleDist = 2200
                        ChangeRequest(6, 7)
```

`ccThMother` then takes request 5 and request 6 in turn.

**`ccSetupNewGame`** (main 0x001687a0): `ccAllSoundOff`; `game.status = 4`;
the layers flipped off; `ccDeleteAllThread`; background black;
`ccBreathThread(2)`; loads `cdrom0:\DATA\GCMN.PRG` on `ccThLoadOverlay`
(priority 17) and waits; **`SetFrameRate(2)`**, 30 frames a second from here;
`ccSaveData::InitSpcParam` (equipment and default items while
`newGameFlag` is 0); `newGameFlag` (+0x6770) = 1; `SetSpcBaseMsg`;
`WORLD_MAN::Init` (a random area word triple, seeded from `ccSys.count`);
`ccSPC::Initialise` (registry 0 = Kite, `charTbl` row 0);
`ccParty::InitParty` (Kite alone); `cameraInit(0)`.

`ccSaveData::InitSpcParam` (main 0x00175d30) gives a new game its
starting kit. It does nothing once `newGameFlag` is 1:

```
newGameFlag 0 or 2: for member i in 1..18 (with 2, only those whose bit i of
                    partyMemberFlag is clear)
    ChangeEquipment(i, ccGetJobWeaponCategory(spcParam[i].job), -1)
    ChangeEquipment(i, 6, -1) .. ChangeEquipment(i, 9, -1)
    spcDefaultItemList[i][0..10] (gcmn 0x00647d80, 40 bytes a member):
        AddItem(i, category, id, num) for each whose category is not negative
newGameFlag 0 only, Kite:
    ChangeEquipment(0, ccGetJobWeaponCategory(spcParam[0].job), -1), then 6 .. 9
    AddItem(0, category, id, num) for each of playerDefaultItemList[24]
        (0x00647cc0) and playerDefaultImportantItemList[24] (0x00647d20)
    parodyFlag: spcParam[0].level = 1, 30, 50 or 70 (a short) and
        ccSetLevelParam(&spcParam[0], 20, 50, 70 or 90) for volumeNum 1-4
        (Infection's is 1); AddItem(0, 15, k, 50) for k in 0..12
```

`ChangeEquipment(sid, cat, -1)` (main 0x00177010, `ccChangeEquipment` on
the member's `equipment` and `skillList`) changes no piece. It adds to the
skill list the skills of the piece `NewGame` copied from `charTbl` into
that slot, and the sets' skills. That is where the starting skills come
from. `ccGetJobWeaponCategory(job)` (gcmn 0x005711f0) is the job for 0-5,
else 0.

Kite after `Init`, `NewGame(1)`, `NewGame(0)`, `InitSpcParam` and
`SetSpcBaseMsg`, measured in eemu:

- skills 6, 150 and 233 (Saber Dance, Repth, Vak Kruz);
- armour 20, 20, 20, 20 (Nomad's Hood, Leather Armor, Leather Gloves,
  Safety Shoes) and weapon 0 (Amateur Blades);
- eight stacks of five or two (category 10 ids 0, 3, 4, 5 and 18;
  category 13 ids 0 and 2; category 11 id 56) and no key items;
- level 1, 63/13.

With Parody he is level 20, 405/70, and has key items 0-11 at 50 each.
Each other member gets its job's and its armour's skills (member 1: 34 and
150) and its `spcDefaultItemList` row.

The save a new game arrives with (from `ccSaveData::Init` and `NewGame`, run
in eemu): `lastTown` 0, `crisis` (+0x6772) 0, `plcol` (+0x6771) 0,
`cameraMode` (+0x8431) 3, `camType` (+0x8429) 0 (scheme A-1), and Kite's
`spcParam[0]` from `charTbl` (demo.prg 0x0040dc80): `ccsname` "ctu1body",
type 7, height 160, width 45, velocity 27.5, job 0. Town 0 is Mac Anu.

**`ccSetupGameCtrl`** (main 0x00168960), in order, for area 0:

```
ccSoundFadeOut (the scene changed); enableReset = 0; ccStoreSpcCondition
fade out: EntryFade(10, 0, 0x80000000, 0, 0, 512, 384); SendPacket + Breath(1) until done
ccAllSoundOff; game.status = 5; font and layers flipped; ccDeleteAllThread
background black; two more frames of the held fade
ccInitRand; ccStartThEvent; WORLD_MAN::SetEventData; ccEnableThEvent(0)
SetSpcItemTown, SetTradeItemTown; ccSetFileListTown        (the files, below)
ccFileExistCheck(0); ccEnableThEvent(2); ccSndSQLoad(2)
ccLoadResourceFL                                            (waits a frame at a time)
enableReset = 1; initHitCheck                              (empties both hit lists)
start: ccThGameCtrl 33, ccThMenu 34; Breath(1); ccThEntryCtrl 64, ccThFieldDisp 96,
       ccThCamera 40, ccThSpc 48, ccThEffect 80, ccThParticle 98   (ccThSkill 82 not in a town)
WORLD_MAN::GO(0)                                            ROOTTOWN01, layers, bounds +-24000
ccGetStartPositions(1, 2, 3); rebootSpcManager (ccPlayerStart: ccThPlayer 49)
ccEnableThEvent(4)
fade in: ContinueFade(n, 10, 0); SendPacket + Breath(1) until done; DeleteFade
setupMode = 0; ccSndBgmCtrl
```

The fader is `ccGame.fade` on `ccGame.layer` (priority 254). The files
(`ccSetFileListTown` 0x00169690): the common and gcmn lists (`CTU1BODY.CCS`
among them), `STR8000E`, the event-registered characters' bodies and
weapons (Kite's `cwdhsw02`), `xallow0`, `town01` (`town01d` in crisis),
`WAT1`, `STR8800E`.

## Tasks and the frame

The kernel runs ready tasks in ascending priority number; `ccThControl`
wakes every task each frame, so a frame is each task's work between two
`Breath`s in this order. The pad is read at the end of the previous frame.

| prio | task | in a town |
| ---: | --- | --- |
| 32 | `ccThEvent` | the event scripts (phase 5 pass) |
| 33 | `ccThGameCtrl` (gcmn 0x00517800) | game over, targets, the menu buttons |
| 33 | `ccThCameraExecute` (0x001b4ff0) | the event camera, while an event runs it (started after `ccThGameCtrl`) |
| 34 | `ccThMenu` | menus |
| 40 | `ccThCamera` (0x00160610) | `cameraMain`: L2, the reset button, the reset's turn |
| 48 | `ccThSpc`, `ccThAISystem` | party and AI bookkeeping |
| 49 | `ccThPlayer` (gcmn 0x005977d0) | `ccPlayer::Main`: Kite, and the camera's placement |
| 50 | `ccThFellow02` | Orca (event 2) |
| 64 | `ccThEntryCtrl` (gcmn 0x00431970) | merchants, the Chaos Gate, the walking PCs |
| 80, 98 | `ccThEffect`, `ccThParticle` | effects |
| 96 | `ccThFieldDisp` (main 0x001a4430) | `ROOTTOWN01::Draw` |

The set-up's `Breath` in the fade loop is frame F0: the new tasks set
themselves up (the camera's `cameraInit(0)` from the start position). F1
is the first frame they all run; `ccThFieldDisp` breathes twice before its
loop, so the town first draws on F2. The fade in runs F0-F9.

## The arrival

`ccGetStartPositions(1, 2, 3)` (gcmn 0x0059ff50) fills `StartPos[]`
(0x00730420, `STPOS` 0x40 bytes: type, xp, yp, zp, rot_z, then pos[4] at
+0x20 and dirc[4] at +0x30) through `WORLD_MAN::SetCharPosition`, which in
a town takes a fixed spot by town number:

| town | position | heading |
| ---: | --- | --- |
| 0 Mac Anu | (0, 5600, 600) | 0 |
| 1 | (0, 3500, 0) | 0 |
| 2 | (0, 600, 0) | 0 |
| 3 | (0, 6000, 0) | 0 |
| 4 | (600, -3900, 0) | pi |

Heading 0 faces -y: in Mac Anu that is the gate plaza, 700 units south of
the Chaos Gate (`DMY_gate` (0, 6300, 600)), looking over the town.
`ccPlayer::ccPlayer(0)` (gcmn 0x00597910) copies it, and in a town starts
act 13 with `transferLag` 1 and `restraintSW` set: the arrival. Act 13
plays `nut2`; `actCnt` stays 0 for two frames, `effTransfer` fires at 30,
`cloak` is 0 until 50 and rises by 1/20 a frame to 1 at 70; at 71
`anmFlag` is forced and the next frame turns act 13 into act 2 (idle,
`nut2` going on) with `restraintSW` cleared, `ccEntryCmnd` and
`bodyHit.HitEnable`: 74 `AnimCtrl` frames in all, after which he can move.

On a real new game event 2 ("MG0020 TEACH_T") runs at phase 0 and 4: it
registers Orca at `DMY_marker_ev03` (-300, 6150, 600) and sets Kite's
`bootParam` to 6 (`pc_mode -3 6`), so the player is built in act 14,
unseen and under manual control ([the party under the event
scripts](#the-party-under-the-event-scripts)); `pc_act 0 3` at frame 100
has `ManualControl`'s `TransferIn` start act 13, the arrival above, while
the event's own camera ([the event camera](#the-event-camera)) looks on.
The runtime's event host and its timeline are on [the event interpreter
page](event-vm.md#the-world). Without the event task (no `bootParam`) the
player arrives in act 13 at once and is free after the 74 frames.

## Kite

`ccPlayer::Main` (gcmn 0x00598310), for a player alone in a town:

```
movePos = (0, 0, 0, 1)
ControlMove                          (progCtrlFlag 0, not dead)
r = HitCheck(movePos)                the move slid along the walls
pos.x += movePos.x; pos.y += movePos.y
CollisionTest: pos.z = posP.z = ccLandHitCheck(pos, 0x20000001)
hitAttribute = checkHitResultAttlibute()
MapLoopAdjustPos: AddCenter(movePos); W2MPos (no wrap in a town: w = 1)
CameraPosCalc: posView = posEye = pos + (0, 0, 140); angle.z = dirc.z
CameraPosSet: type 1 -> cameraSetEyeLevel(posEye, angle), lostHeadFlag, dirc.z = angle.z
              else   -> cameraSetManual(posView) (type 3: cameraPosCalc)
              cameraSet()
AnimCtrl
anm->SetMatrix_PosRotZYX(pos, dirc)  T(pos) Rx Ry Rz
transparency = setTransparency = lostHeadFlag ? 0 : cloak
ccChar::Draw (dispSW); stopCnt, cycle
```

**`ControlMove`** (0x00598af0), `ctrlType` 0:

- `lever = PadLeverPower(powL)` (0x005992b0): below 64 nothing; 64-127 eased
  to `(v - 64) * 3 / 2 + 32`; from 128 as it is. For the first four frames
  of a lean (`targetCount`) a targeted character (`cmndTarget`) makes it 0.
- `nowSpeed = 0; speedRate = lever / 255`. Nothing more while a skill,
  `pauseSW` or `restraintSW` holds him (the arrival).
- Lever 0: `moveFlag = 0`. Otherwise `moveFlag = 1`, `runFlag = lever > 230`.
- The heading: `a = dircL`, or pi (straight ahead) while the camera's reset
  flag is up and the stick stays within 2047/65536 of its direction (else
  the flag drops); `s = RAD2DEG(rot.z) - RAD2DEG(pi + a)` (16-bit), with
  `rot.z` the camera's heading (`cameraGetRot(1)`: `tcam.rot`).
- Walking: `speedRate = min(lever / 140, 1.3)`, speed `speedRate *
  (speedValue * 3.1)` (`tsp` 0x003782b0), at most 4.03 a frame. Running:
  `speedRate * (speedValue * velocity)`, 27.5 at full lean for Kite.
- `movePos = (v sinf(DEG2RAD(s)), -v cosf(DEG2RAD(s)))`; `dirc.z =
  DEG2RAD(s)` unless the camera is the eye view. The turn is immediate.

`RAD2DEG(r) = (short) fptosi(32768 (pi + r) / pi - 32768)` and
`DEG2RAD(s) = pi s / 32768` (main 0x001dab50, 0x001dabb0); they do not
round-trip. Movement stops dead when the stick is let go (`movePos` is
reset each frame).

**Acts** (`AnimCtrl` 0x005993c0). Every act plays one animation of
`playerAnimTbl` (gcmn 0x006f0560, 26 names of 21 bytes, all in
`ctu1body`); changing act cuts to frame 0 of the new one (`ccAnm::SetAnm`
never blends). In a town:

| act | animation | when |
| ---: | --- | --- |
| 13 | `nut2` | the arrival, above |
| 2 | `nut2` (76 frames, loop) | standing |
| 3 | `nut3` (71, once) | after 451 frames standing (`reactCnt`, reset to `rand() % 60`) |
| 4 | `nut4` (71, loop) | after 3 ends, until he moves |
| 6 | `wal0` (37, loop) | walking |
| 5 | `run0` (21, loop) | running |

Starting to move sets act 6 and, the same frame, 5 if running; stopping
sets act 2 (0 outside towns). While walking or running `frameSpd =
fptoui(256 ((k speedRate) speedValue))`, k 1.1 walking and 1.375 running;
otherwise 256.

**Drawing** (`ccChar::Draw` 0x0056b1c0) on the character layer (10):
`transparency = setTransparency * ccGetCameraTransparency(pos, width,
height, 4000, 400, &hide)` (0x001da8b0), which fades him out as the camera
comes within 1.25 r (r = the camera's pitch blend of width and height, at
least 180) and beyond 4000; nothing drawn below 0.05 unless that near fade
applies. The clump `CMP_trall` is posed by the act's animation under
`T(pos) Rx Ry Rz(dirc)`; the only drawn mesh is `MDL_ckitbody` (lit,
skinned), with the Amateur Blades (`cwdhsw02`, `MDL_cwdhsw02r` / `l`) in
his hands. Before the Data Drain bracelet (`plcol` 0) the
constructor swaps `CLT_ctu1body` for `CLT_ctu1bodyc1`. Lights: up to three
of the town's group reaching his position, omni lights (priority 0) before
the distant one (-1); on ground with attribute bit 0x40000 the distant
light is dimmed to 0.3 while he draws.

**The weapons.** `ccPlayer::ccPlayer` (gcmn 0x00597910) looks the hands up
among the clump's own nodes by name (`ccClump::GetObjAdrsF` main
0x0013d300: `OBJ_t0 l hand` into +0x12c at 0x00598038, `OBJ_t0 r hand`
into +0x130 at 0x00598060); `ccSpcChar::EquipWeapon` (gcmn 0x0059d650)
takes the weapon's file and, for job 0 (twin blades, jump table 0x006f09e0
to 0x0059d854), makes `ccModel`s of `MDL_<file>r` and `MDL_<file>l`
(`ccMakeModelName` 0x005a1530, suffixes "r" and "l") and `ccObj::SetModel`s
(main 0x0013b8f0) them onto the right and the left hand in place of the
hands' own empty models. From then on a blade is one of the clump's models:
`ccAnm::Draw` draws every node the act's animation poses through
`ccObj::Draw` (main 0x0013f220) at the node's world matrix
(`ccCoord::_SetLWMatrix` 0x00138380) - the hand's matrix itself, no offset,
scale or dummy of its own, the body's transparency and lights, in every act
and area. `ctu1body` also has an object of each node's name in every
animation (ExtObj copies, `ANM_ctu1atc0`'s first) that nothing poses: a
blade hung on one of those stands at his feet, which is what the port did
before. The weapon file's own clumps and objects (`CMP_cwdhsw02r`,
`OBJ_cwdhsw02r`) are never built; its `DMY_xdummy_w01`-`w04` only place
the weapon trails (`ccSpcChar::ArmsEffect` 0x0059ddd0, [battle.md, the
party's weapon trails](battle.md#the-partys-weapon-trails)).

## Collision

A town's scene file has one Hit chunk (0x0b00); Mac Anu's is
`HIT_sr1town1hit` on `MDL_floor_02`, 828 triangles in 7 groups, in world
coordinates. The payload:

```
u32 object (HIT_), u32 parent (the model), u16 groups, s16 pad, u32 total vertices
per group:  u32 vertexNum, u32 attribute,
            f32 tri[vertexNum/3][3][3], then f32 normals[vertexNum/3][3][3] (ignored)
```

`ccSetHitData` (0x00149990) gives each triangle its box, the unit normal
`NORM(CROSS(v2 - v1, v3 - v1))`, the plane `d = -(n . v1)`, unit edges and
their lengths, and a class in the attribute's top bits: from the normal's
elevation `a = fptoui(RAD2DEG-like(atan2f(n.z, |n.xy|))) & 0xffff`, floor
(0x20000000) for 7169 <= a < 25600 (about 39 degrees up or more), else wall
(0x40000000). Mac Anu has 301 floors and 527 walls; bit 0 is on every
polygon (the player's masks), bit 2 the camera's, bits 0x00f0f0f0 the
ground's type (footsteps), 0x40000 a shaded ground. `ROOTTOWN01`'s
`STATICMODEL` for `MDL_floor_02` registers it (`ccModelHit::HitEnable`), the
only hit model in the town.

The queries (libhit, main 0x00152ec0-0x00155c60), all in EE floats:

- **Segment** `_ccHitCheckLM(sp, ep, mask, maskType)`: every polygon passing
  the mask (type 0: any bit; 1: all bits) and the boxes, crossed front to
  back (`ds >= 0`, `de <= 0`), inside all three edges (tolerance 0.001):
  the nearest (last of equal) is the result, `ep` becomes its point.
- **Ground** `ccLandHitCheck(pos, 0x20000001)` (gcmn 0x00571e00): the segment
  from 105 above `pos` to 1000 below, floors only; its z, or `pos.z` on a
  miss. He steps up at most 105 a frame.
- **Sphere** `ccModelHitCheckQ`/`QZ(offset, pos, r, mask, maskType)`: faces
  within r in front, else edges, else vertices; results grouped by face
  normal (`ccHitResultGrouping`, nearest of each group); `offset += nv (r -
  dist)` for each group, z zeroed for Q.
- **The move** `ccSpcChar::HitCheck(movePos)` (gcmn 0x0059ee20): the new
  position landed, a sphere of radius `width + nowSpeed` at 95 above it
  pushed out of the walls (twice, averaging the two pushes when still
  touching), then the chest-height segment (+95) from the old position to
  the new against the walls: a crossing stops him dead.

## The camera

`tcam` (0x00383ce0, `CAMERA` 0x70 bytes: pos, view, rot, rot2, rot3, cptr,
dist, `short deg[2]` heading and pitch, type, resetFlag, resetDirc) is the
field camera, camera id 1. Type 3 follows him, type 1 is the eye view.

- **`cameraInit(0)`** (0x00160a00): view = start + (0, 0, 120); deg =
  (RAD2DEG(heading), 1512); dist 970; pos placed behind:
  `off = (h sinf(a), -h cosf(a), dist sinf(rx))`, `h = dist cosf(rx)`,
  `a = DEG2RAD(RAD2DEG(rz) + 0x8000)`.
- **`cameraMain`** (0x00160cc0), before he moves: L2 (push, no menu) swaps
  type 3 and 1 and stores it in `saveData.cameraMode`; the reset button (R2
  for type A, L1 for B) remembers his heading and, while it runs, turns
  `deg[0]` a quarter of the way there each frame (at least 128/65536, at
  most 20480), brings the pitch back to 1512 and the distance a quarter of
  the way back to 970, until all three arrive. With the party wiped out
  (`checkPartyAnnihilation`) camera 1 in the eye view goes back to
  following and the buttons are read that frame; any other camera skips
  them and only the reset's turn runs (0x00160cd0).
- **`cameraPosCalc`** (0x00161870), from `CameraPosSet`, after he moved:
  the zoom (`checkCameraDistModValue`), clamped to 280-1950; the turn and
  pitch by scheme; the pitch at least `896 (1 - (1950 - dist) / 1670)` and
  at most 8191; the camera placed behind `posView`; in a field
  (`game.area` 1) kept over the ground by `avoidObstacle`; then pulled in
  front of the first polygon with bit 2 on the line from his head
  (`ccHitCheckLM(t, P, 4)`: 10 short of it, at least 0.1) and, in a town
  (`game.area` 0), pushed out of walls within 25 (`ccModelHitCheckQZ`).
  Every polygon of Mac Anu's hit mesh has bit 2, floors (the stairs, the
  bridges) as well as walls, so the line check is the town's floor
  handling: the camera stops just short of a floor between it and his head.
  The pull-in leaves the camera a few units off a floor it grazes, nearer
  than the view's near plane (w 8) to the polygons around it: those draw
  cut at the near plane ([render](render.md#culling-and-clipping)).
- **`avoidObstacle`** (0x00162430, fields only): the heading folded into
  0-45 degrees gives a step of `300 / cosf(a)` along the ground (a quarter
  of it when the camera is nearer than one step), climbing at the pitch
  (`step tanf(rot.x)`); from the target toward the camera it walks
  `fptosi(|cp - tp| / |step|)` steps (one fewer when `fmodf` leaves less
  than a quarter step) and asks the ground under each
  (`ccTransPosW2M`, `ccSetGroundHeight` -> `WORLD_MAN::GetHeight`); the
  first point less than 50 above the ground is lifted to 50 above it and
  becomes the camera; if none, the camera itself is checked the same way.
  A Root Town's `ROOTTOWN::GetHeight` (gcmn 0x005b61d0) is 0, and the
  town never calls it.

| scheme (`camType`) | right stick X | right stick Y | L1 / R1 | R2 | reset |
| --- | --- | --- | --- | --- | --- |
| A-1 (0) | turn, left + | zoom (down out), pitch held at 1512 | turn when the stick is still: 500, or 384 + 2(p - 10) with pressure | | R2 |
| A-2 (1) | turn, left - | as A-1 | as A-1, reversed | | R2 |
| B-1 (2) | turn, left + | pitch | R1 zoom in | zoom out: 125.25, or 30 + 0.75(p - 128) with pressure | L1 |
| B-2 (3) | turn, left - | pitch | as B-1 | as B-1 | L1 |

A stick turn is `fptosi(powR * 2 sinf(dircR))` a frame (up to 509 of 65536).
`p` is the button's pressure (`ccPad.pow`, DualShock 2 pressure mode); a pad
without it reads 0, the fixed rate.

- **`cameraSetEyeLevel`** (0x00162020): from his eyes, 280 ahead at his
  `angle`, turned by the stick (2.5 sinf) and L1/R1, pitch within +-7936.
- **`cameraSet`** (0x00161260): camera 1's eye and target go through the
  screen shake (below; its offset is zero with none); the eye and target taken through the map wrap (a no-op in a town,
  but `(p - player) + player` rounds); `ccCam::SetMatrix_PosTarget`:
  `sceVu0CameraMatrix(P, V - P, up)` inverted (x = NORM(yd x zd), z =
  NORM(zd), y = z x x); `ccView::SetView` makes `world_screen =
  view_screen * world_view` with the field's `view_screen` (fov 45, near
  8, far 2^20, Z 1..2^28; scrz 0x441a827a, scy 0x44ffffff).

### The screen shake (`cameraShake` 0x00162cd0, `cameraShockAbsorber` 0x001629b0)

camera.cpp keeps eight shocks, `sfList[8]` (`camShockForce`: 4-bit
`power`, `cycle` and `rot`, `time`, a 16-bit `dirc`). `ccThCamera`'s start
frees them (power -1) and zeroes `vibrateForce` and `vibrateOffset`.
`vibrateCycle`, `vibrateRotate`, `vibrateMatrix` and `oscillator` start at
the executable's 0 and are never reset.

```text
cameraShake(power, cycle, time, dirc)
    a held shock that is stronger and longer: the new one is dropped
    else the first held one it matches or beats in both takes its slot,
    else the first free slot (none: dropped)
    dirc 0: dirc 0, rot 0; 1: 0x4000, rot 0; 2: (rand() << 7) & 0xf800,
    rot 1 (a turning shake); 3: both left as they were
cameraShockAbsorber (the end of every cameraMain)
    the first held shock of the highest power: if shakePowerTbl[power]
    (10, 20, 30) beats vibrateForce, it becomes the force, with
    shakeCycleTbl[cycle] (8, 6, 4) the cycle, vibrateRotate its slot
    (-1 without rot) and vibrateMatrix Ry(dirc)
    vibrateRotate's slot turns 0x1000 a frame (vibrateMatrix after it)
    oscillator += 0x10000 / cycle; vibrateOffset = (0, 0, force
    sinf(oscillator)); a crossing of 0 keeps a fifth of the force; under
    1 force and oscillator are 0
    every held shock's time counts down; at 0 its slot is free
cameraSet, camera 1
    a = atan2(d.x, -d.y) in double, d = view - pos across the ground
    o = 0.5 (Rz(a) vibrateMatrix) vibrateOffset
    eye = pos + o; target = (view + o) + o min(|view - pos| / 280, 1)
```

The first `cameraShockAbsorber` before any shake divides by
`vibrateCycle`, still 0; the force is 0, so the oscillator is reset
after it. The shakes come from the enemies' note 0x8002
(`cameraShake(kind, 2, 20, 2)` when `checkCameraShakeRange` passes: in
the camera's 67.5 degrees and nearer its eye than 2000), the criticals
(`cameraShake(0, 2, 2, 0)`) and the spells' and effects' blasts. The
port is piney-world's `camera::Shake`. The effects' shakes reach it after
the frame (`FieldFx::take_shakes`), and the enemies' at their note.
`tools/test_camera_shake_rs.py` runs the game's `cameraShake`,
`cameraShockAbsorber` and `cameraSet` (`SetMatrix_PosTarget` taken) over
12 runs of 400 frames of random shakes, with `rand()` shared. The slots,
the force, cycle, turn, matrix, offset, oscillator, `rand()`'s state, and
4,800 eyes and targets were all equal.

## The event camera

The event scripts' camera instructions ([events](events.md), codes 22-36,
40-44, 48-50) drive a second camera, `ecam`, through a task of their own,
`ccThCameraExecute`. The port is `crates/piney-world/src/evcam.rs`;
`tools/test_evcam_rs.py` checks it against the game (see [Checks](#checks)).

### Three cameras

camera.cpp keeps `tcam` (0x00383ce0), `bcam` (0x00383d50) and `ecam`
(0x00383dc0), each a `CAMERA`. `cameraList` (0x002fb6f0) is {tcam, tcam,
bcam, ecam}; `changeCamera(n)` (0x001617f0) sets `camID` (0x0037897c, s16)
and `activeCamPtr` (0x0037896c) = `cameraList[n]`. ccThCamera's set-up
(0x00160610), after `changeCamera(1)` and `cameraInit(0)`, copies tcam's
pos, view, rot, rot2, rot3, cptr, dist, deg, type, resetFlag and resetDirc
into ecam (0x00160730) and ecam's into bcam (to 0x00160990); all three share
tcam's `ccCam`, so there is one view matrix.

What reads the active camera rather than tcam (from the code):

| reader | reads |
| --- | --- |
| `cameraSet` (0x00161260) | camera 1: pos and view through the screen shake's offset; any other: its pos and view as they are |
| `ROOTTOWN01::Draw` (gcmn 0x00423b50) | `cameraGetPos(camID)`, the draw's eye |
| `ccGetCameraTransparency` (0x001da8b0) | the active camera's pos and deg[1] |
| `ccCheckCameraDeg` (0x001da710) | its pos and view |
| `checkCameraType` (0x00160cb0) | its type: ccChar::Draw's hide, ccCheckTargetRange, AnimCtrl, ControlMove |
| `ControlMove` (gcmn 0x00598af0) | its resetFlag and resetDirc; the heading `cameraGetRot(camID)` |
| `cameraGetRot(out, n)` (0x00161610) | n 1: tcam.rot (all lanes); any other: x = `atan2f(pos.z - view.z, |view - pos| across)`, z = `atan2f(view.x - pos.x, -1 (view.y - pos.y))`, y and w left as `out` had them |
| `cameraMain` (0x00160cc0) | L2 only with camID 1 and `puppetShow` 0; the reset button not with camID 2 or 3 and only for an active type 3; the reset's turn works on tcam while the active camera is type 3 |
| `CameraPosSet` (gcmn 0x0059bac0) | the active type: 1 calls `cameraSetEyeLevel` (0x00162020: nothing unless camID 1), and sets `lostHeadFlag` and `dirc.z = angle.z` only with camID 1; any other type calls `cameraSetManual` (0x00161820: `cameraPosCalc` on tcam only with camID 1 and type 3) |

So while the event camera is active tcam stands still, Kite walks relative
to ecam's heading, and characters fade by ecam.

### State

`eventMng` (*0x00378a94, `ccEvent` 0x7e0 bytes): `cam` at +0x440,
`puppetShow` +0x78c, `normalCamID` +0x7b4, `camTscb` +0x7b8.

```
ccEvCamCtrl (0x310 bytes)
+0x00 s16 vpCtrl       +0x02 s16 cpCtrl      +0x04 s32 charType   +0x08 s32 charID
+0x0c f32 height       +0x10 f32 vpRate      +0x14 pad[12]
+0x20 f32 vp[4]        +0x30 f32 vpTarget[4]
+0x40 f32 cpRate       +0x44 f32 dist        +0x48 f32 distTarget +0x4c f32 rot[2] (pitch, heading)
+0x54 s16 rotTarget[2] +0x58 pad[8]          +0x60 f32 cp[4]
+0x70 ccEvCamz zcam[2] ([0] the look-at point, [1] the camera)
ccEvCamz (0x150 bytes)
+0x00 s16 ctrlType  +0x02 s16 spdType  +0x04 s16 inpNum  +0x06 s16 pad  +0x08 f32 inpAlpha
+0x0c f32 rate      +0x10 f32 cnt      +0x14 pad[12]
+0x20 f32 now[4]    +0x30 f32 start[4] +0x40 f32 target[4]  +0x50 f32 inpPos[16][4]
```

`ccEvent::Init` (0x001a6ca0, from `ccStartThEvent` in each area's set-up)
clears `camTscb` and `puppetShow` but not `cam` or `normalCamID`.

### The instructions

`ccEvent::Execute` (0x001a8d20, jump table 0x00355fd0), at level 2 or more;
below that a handler only steps over its operands. Positions and distances
are `10.0 * (float)v`. "Char": by `1 << type`: bit 4 `GetSpc(code)`
(0x001b2bb0), 0x18 `GetNpc` (0x001b2cc0), 0x60 `GetEnemy` (0x001b2d50),
else `ccCheckTargetTypeId(1 << type, code)` (gcmn 0x005199e0: the command
lists by base flags and id); none gives `ccPartyManager.memberChar[0]`
(0x00730310); the character's +0x40, all four lanes. "Marker": in a town
`GetChunkAdrsF(roottown stream, markerEvTbl[m], 0) + 0x10` (0x00317da0), in a
field or dungeon the `evPos` (+0x1c0) numbered m, its +0x10; else nothing.
All but camz_speed and camz_point then `ccStartThread(ccThCameraExecute,
33, 0x800)` unless `camTscb` is set.

| code | handler | name | sets |
| ---: | --- | --- | --- |
| 22 | 0x001aab54 | cam_look_pos | vpCtrl 0, vpRate 0, vpTarget.xyz |
| 23 | 0x001aabf8 | cam_look_char | vpCtrl 0, vpRate 0, vpTarget = char, z + height |
| 24 | 0x001aad2c | cam_follow_char | vpCtrl 1, vpRate 0, charType, charID, height; vpTarget as 23 |
| 25 | 0x001aae9c | cam_look_marker | vpCtrl 0, vpRate 0, vpTarget = marker |
| 26-28 | 0x001aafc8, 0x001ab0cc, 0x001ab264 | ..._half | vpCtrl 0, vpRate 0; vpTarget += (t - vpTarget) / 2 (xyz), w 1 |
| 29 | 0x001ab3ec | cam_pan_pos | vpCtrl 0, vpRate = rate, vpTarget.xyz |
| 30 | 0x001ab4a0 | cam_pan_char | vpCtrl 0, vpRate = rate, vpTarget = char + height |
| 31 | 0x001ab5e4 | cam_pan_follow | vpCtrl 1, vpRate = rate, charType, charID, height (no lookup) |
| 32 | 0x001ab680 | cam_pan_marker | vpCtrl 0, vpRate = rate, vpTarget = marker |
| 33 | 0x001ab7b8 | cam_orbit | cpCtrl 2, cpRate 0, rotTarget (rotx, roty), distTarget |
| 34 | 0x001ab840 | cam_orbit_move | cpCtrl 2, cpRate = rate, as 33 |
| 35 | 0x001ab8d8 | cam_orbit_turn | cpCtrl 3, cpRate = DEG2RAD(rate), as 33 |
| 36 | 0x001ab96c | cam_mode4 | cpCtrl 4 |
| 37-39 | 0x001ab9b4, 0x001abbe8, 0x001abe18 | teach_camera1-3 | cpCtrl 5 / 6 when the prompt opens; 7 when the reset button (R2 type A, L1 type B) is pushed |
| 40 | 0x001ac1c8 | camz_set | both zcam: ctrlType 8, spdType 0, rate 0, cnt 0, target (w 1); vpCtrl 8, vpRate 0, vpTarget; cpCtrl 8, cpRate 0 |
| 41 | 0x001ac348 | camz_move | zcam[0]: ctrlType 8, rate vprate, cnt 0, target, start = now = vp; vpCtrl 8, vpRate vprate, vpTarget; zcam[1] from cp with cprate; cpCtrl 8; then vpRate = cprate (the store meant for cpRate); spdType kept |
| 42 | 0x001ac544 | camz_speed | zcam[0].spdType, zcam[1].spdType |
| 43 | 0x001ac578 | camz_point | num clamped to 16 in the script; inpPos[num] of each (w 1); 16 writes past the array (zcam[0]'s onto zcam[1]'s head, zcam[1]'s onto `currentOpen` +0x750) |
| 44 | 0x001ac6b0 | camz_path | num clamped likewise; both zcam: ctrlType 9, inpNum, inpAlpha = alpha / 10, rate, cnt 0; vpCtrl 9, cpCtrl 9, vpRate vprate then cprate |
| 48 | 0x001ac028 | camera | vpCtrl 1, vpRate 0, charType, charID, height, vpTarget = char + height; cpCtrl 2, cpRate 0, rotTarget, distTarget |
| 49 | 0x001acb3c | camera_end | `changeCamera(1)`, `camResetFlag` (0x00378974) 0, `ccDeleteThread(camTscb)`, camTscb 0 |
| 50 | 0x001acb7c | camera_end_reset | `changeCamera(normalCamID)`, camResetFlag 0, `cameraSoftReset`, the task deleted |

`cameraSoftReset` (0x001631d0) calls `cameraSoftResetParam(RAD2DEG(plw->dirc.z),
1512, 970)` (0x00163230): a dist of -1 is 970, clamped 280-1950; the pitch
1512 for a type A scheme, else below 8192 and at least `fptosi(896 - 896
(1950 - dist) / 1670)`; then tcam's rot, deg and dist.

`menu_ban` (`MenuBan` 0x001b2460): `puppetShow = 1`, `dneFlag` (0x00378cd8)
= 1, `normalCamID = checkCameraID()` (0x00160ca0); it also zeroes +0x90 of
each ccSpcManager character (0x0073035c, 5 x 0x2c), so ccChar::Draw's hide
starts set. `menu_clear` (`MenuClr` 0x001b25c0) clears both flags and leaves
the camera. While `puppetShow` is set, `cameraMain` ignores L2 and
`cameraSetEyeLevel` (0x00162050) does not turn: it takes the player's
`angle` as it is and stores `tcam.deg[0]` from a stack slot it never wrote.

### The task

`ccThCameraExecute` (0x001b4ff0, "EV CAM EXEC"): `changeCamera(3)`, then
each frame `CamCtrl(eventMng)` and `Breath(1)`; the first CamCtrl runs in
the same activation. `GoThread` (0x00159d40) starts a thread at once when its
priority (33) is not above the caller's (`ccThEvent` 32), without
preempting it; `ccThControl` (0x0015a5c0, priority 1) wakes tasks in
creation order (ccStartThread appends at `tscbEnd`, 0x00378948). The camera
task therefore runs in the frame of the instruction and every frame after,
after `ccThGameCtrl` (33, created earlier) and before `ccThMenu` (34),
`ccThCamera` (40) and `ccThPlayer` (49). `ccDeleteThread` (0x00159f70) sets
`del`; `Breath` (0x00159e10) sleeps while it is set, so after camera_end
CamCtrl does not run again, that frame included. The first-in first-out
order within a priority is the EE kernel's, inferred.

### CamCtrl

```
CamCtrl (0x001b3680)          work area s = ccSys+0x25c (0x70 bytes)
vpCtrl 8 / 9: CamzCtrl / CamzInpCtrl; return
vpCtrl 1: vpTarget = char(charType, charID), z += height
vpRate != 0: d = vpTarget - vp, d.xyz /= vpRate, vp += d, vp.w = 1, vpRate -= 1 (>= 0)
else vp = vpTarget
cameraSetView(vp, camID)                                  0x00161510
cpCtrl (table 0x00356270):
 2, cpRate != 0: w.xy = DEG2RAD(rotTarget) - rot, each above pi -(2pi - w), below -pi 2pi + w;
                 w.z = distTarget - dist; w.xyz /= cpRate; rot[0] += w.x, rot[1] += w.y
                 (each then wrapped against pi/2 in s: above -(pi - w), below pi + w);
                 dist += w.z; cpRate -= 1 (>= 0)
 2, cpRate 0:    rot = DEG2RAD(rotTarget), dist = distTarget
 3:              rotTarget[1] += RAD2DEG(cpRate); rot = DEG2RAD(rotTarget); dist = distTarget
 5:              type A: R1 held +500 (384 + 2 (p - 10) with pressure p >= 11), L1 -500
                 (-384 - 2 (p - 10)), summed, camRevLR negates; heading += it; pitch 1512.
                 type B: heading -= fptosi(powR 2 sinf(dircR)) (camRevLR), pitch -=
                 fptosi(powR 2 cosf(dircR)) (camRevUD), kept 896-8191
 6:              as 5, then the zoom: type A the stick's powR cosf(dircR) past +-64,
                 (f -+ 32) 0.4, camRevUD, added to dist; type B assignPAD +0x8412 held:
                 dist -= 30 (+ 0.75 (p - 128) for p >= 129, + 95.25 for p 0; R1's p),
                 +0x8414: dist += the same with R2's p; dist kept 280-1950
 7:              cameraMain's reset toward plw->dirc.z: the step (to +-20480, at least
                 +-128) / 4 added, snapped within 33; the pitch a quarter of the way to
                 1512, then snapped within 128; dist snapped to 970 within 4, then a
                 quarter of the way
 other:          nothing
cpCtrl != 4: cp = RotZ(rot[1]) RotX(rot[0]) (0, dist, 0, 1) + vp   (w 2); cameraSetPos
cameraGetRot(s+0x20, camID); cameraSetRot(s+0x20, camID)             0x00161550
```

The rotation is VU0 (`sceVu0RotMatrixX` 0x00110ad8, `Z` 0x00110a30, sin and
cos from `_sceVu0ecossin` 0x001109b8). ecam.rot's y is w.y after the pi/2
wrap (cpCtrl 2 with a rate), `plw->dirc.y` (cpCtrl 7), otherwise what the
work area held; its w likewise.

`CamzCtrl` (0x001b4780): zcam[0] then zcam[1], each a rate of 0 snapping
`now` to `target`, else `CamzMovMain`; `vp` and ecam's view = zcam[0].now;
`cp` and ecam's pos = zcam[1].now; ecam.rot = `cameraGetRot` over
zcam[1].now (its y and w are the point's).

`CamzMovMain` (0x001b48e0): `cnt < rate`: cnt += 1; `lper = cnt / rate`;
spdType 3 `now = start + (target - start) lper`; 2 `now = start + (target -
start) sinf(pi (0.5 lper))`; any other `now += (target - now) / rate` and,
unless spdType is 1, rate -= 1 (at least 0, then a snap to the target);
now.w 1.

`CamzInpCtrl` (0x001b4b20) is CamzCtrl with `CamzInpMain` (0x001b4c80):
`cnt < rate`: cnt += 1 (rate never counts down); `f = (inpNum - 1) cnt /
rate`, s its whole part (`modff` 0x00127ad0, `fptosi`). spdType 4: s =
max(s - 1, 0), at most inpNum - 4; `now = eneSplineB(inp[s..s+3], (f - s) /
3)` (gcmn 0x004381d0: the cubic Bernstein blend in multiply-adds, w 1). Any
other: s - 1 < 0 uses `((inp1 - inp2) + inp1, inp0, inp1, inp2)` at f;
otherwise f -= s and the points are `inp[s-1], inp[s], inp[s+1]` and
`inp[s+2]`, or `(inp[s] - inp[s+1]) + inp[s+1]` when s - 1 > inpNum - 4;
`eneSplineH` (gcmn 0x004382a0): tangents `(1 - alpha)/2 (((p1 - p0) + p2) -
p1)` and the same from p1, p2, p3; h00 `1 + (2t^3 - 3t^2)`, h10 `t + (t^3 -
2t^2)`, h11 `t^3 - t^2`, h01 `-2t^3 + 3t^2`; w 1. Fewer than four points
under spdType 4 read inp[-1..-4] (the struct's target, start, now, cnt); a
path of 16 read at its end reads inp[16].

Event 2 uses camz_set, camz_speed 2 2, camz_move 120/120 and 20/20, and
`camera` on Orca; it never ends the camera (the area change does), and its
second menu_ban finds camID 3.

The port: `EventCam::command` (the handlers), `frame` (the task),
`cam_ctrl`, `teach`, `menu_ban`, `next_area`; `Camera` holds tcam, ecam,
bcam, `cam_id` and `puppet_show`; `World::event_camera`, `menu_ban_camera`,
`teach_camera`, `set_event_cam`; the task runs after `ccThGameCtrl` in
`World::frame`, and everything that looks from the camera uses the active
one.

## What the town draws

`ROOTTOWN01::Draw` (gcmn 0x00423b10), every piece on `objLayer`
(priority 0), for a camera eye `c`:

```
clip[2] = c.y > 1600; clip[0] = the gate plaza (|x| < 600, 4900 < y < 6900);
clip[1] = the south plaza (|x| < 1200, -6300 < y < -4600)
ships (static objects of type 0), within 7500 (xy) of the eye
DrawBG: the sky clump CMP_sr1bac1 at the origin; in crisis (town01d) also
        CMP_sr1dat1_1, _2, _3 on layers -80, -70, -60, _2 scrolling in U,
        _3 in U and V (0.002 a frame, v$1334)
DrawObj2: the rows of type 3, skipping some by clip (and south-west of
          (-3000, -3200)); the flags and banners within 5500
DrawObj: the rows of type 2; rows 11 and 8 swap to their far versions (29,
         30) beyond y 1600 / 2400 (2000 / 2200 on a plaza)
water: wat1's three copies of ANM_sr1wat1_a; 1 scrolls U, 2 scrolls V by
       0.005 a frame; copy 0 samples the frame buffer at screen UVs
DrawFloor: the rows of type 1 by clip
DrawMap: the minimap (layer 50 and the font layer; xallow0 is its arrow)
the tower (row 31, type 0)
```

Each pass walks all 32 rows and tests the row's type, so a skip list
naming a row of another type does nothing: `DrawObj2`'s row 12 north of
2600 and `DrawObj`'s row 23 north of 1600 are such, and so is `DrawObj`'s
test of `clip[2]` on the south plaza, which cannot be set there. The whole
choice, run in eemu for camera eyes all over both towns, is
`Town::select` piece for piece ([Checks](#checks)).

No static model is culled by distance; a static object's animation steps
only while it is drawn. The lights and ambient come from `ANM_sr1town1a`
(its controllers at frame 1): ambient (113, 113, 120)/255, the distant
light `LGT_sr1lig1` and five omni lights at the merchants' corners. A light
record of an Anime chunk is a set of controllers picked by 3-bit fields of
its flags, as `ccAnmChunk::ConvLCNum2ALCNum` reads them: 0x0603 distant
(rotation bits 3-5, colour 6-8, intensity 9-11), 0x0609 omni (position
0-2, colour 6-8, intensity 9-11, fall-off start 12-14 and end 15-17); an
absent intensity is 1 and absent distances 0 (`SetAnmCtrlWork`). The
town's omni lights (flags 0x9041) carry no intensity: full to 600, nothing
past 800. The fog is `SetFog(1000, 7500, 0, 85, 0x00144870)`. Town models
are unlit; characters and the Chaos Gate use the lights.

## The Chaos Gate

`ccThEntryCtrl`'s set-up runs `ccEntryEventMng` (main 0x001b62e0), which in
a town calls `ccSetMerchant(0)` and `ccSetChaosGate` (gcmn 0x00458df0): an
entry of gimmick 16 (`gimmickTbl[16]`, "Chaos Gate", `CHGATE.CCS`, height
100, width 60) at `DMY_gate`'s position (0, 6300, 600), heading
`DEG2RAD(-32768)`. Its `ccChgate` (0x00458f60) holds two `ccAnm`s, each
`SetLightEnv(1)` and `SetFogSw(0)`:

```
+0xd4   CMP_xmgtwav0 posed by ANM_xmgtcir1 (91 frames, looping): the ring
        spinning about z, its wave and the gold ring on the floor
+0x1e4  CMP_xmgtcir0 with ANM_xmgtcir2 (the magic circle); stepped and drawn
        only while the gate is used
```

Each frame `ccThEntryCtrl` (64) runs `ccEntryObj::routine` (the command
target; the gate never fades in or out) and `ccChgate::main` (0x00459280):

```
gateAnm                          the circle's state (below)
d = |W2PPos(pos)|                x and y less Kite's, the gate's own z
if d < 4500:
    the gate's anm steps; SetMatrix_PosRotZYX(pos, dirc)
    SetActiveLayer(5)            layer +0x4cc, priority 10
    ccChar::Draw                 transparency = cloak (1) *
                                 ccGetCameraTransparency(pos, 60, 100, 4000,
                                 400); skipped under 0.05 unless the camera's
                                 nearness faded it; ends on SetActiveLayer(0)
    if state: the circle steps (its end noted) and draws, on sysLayer
```

`ccGetCameraTransparency` takes both points through `W2PPos`, so it
measures from the camera to (gate x, y, 600): the gate fades inside
1.25 x max(180, 60 t + 100 (1 - t)) (t = (8192 - pitch) / 8192), except in
the eye view, and beyond 4000. At the arrival the camera is 382 from it and
the gate stands between the camera and Kite.

`SetLightEnv(1)` makes `ccAnm::SetAnm` put the animation's light objects in
the draw environment's group (`ccLightGrp::AddGrp` into `ccDrawEnv::active
+ 0x80`, after the town's lights of equal priority) with the anm as their
parent. `ANM_xmgtcir1`'s `LGT_sr1omn01` is an omni 166.8 above the gate,
its colour going between (110, 137, 214) and (104, 174, 203) over the
loop, full to 400 and nothing past 500: it lights the gate's lit models
and anyone in the ring. The circle's light keeps `ccOmniLight::Init`'s
black (intensity 1, no fall-off) until the circle first steps.

**The light group.** Towns, fields, dungeons and event areas light their
characters the same way.
- **Priorities.** `ccCreateLight` (0x001389b0) gives a distant light
  priority -1 and the other kinds 0; a boss's `ccLight(4, 1)` has 1.
- **The order.** `ccLightGrp::AddGrp` (0x00139060) puts a light after the
  last one of greater or equal priority, so the group runs in descending
  priority, equal ones in the order added. A town's table names its
  distant light first (`town00Light`, `town02Light`, `eventareaB0Light`),
  yet it comes after every omni light.
- **The slots.** `ccDrawEnv::SetLightMatrix` (0x00105900) walks the group
  (after a second one at +0x88, when there is one). It keeps three
  records of (priority, slot), all -128 at first. A light above the
  lowest record's priority is asked `CheckRange`. If it answers, it takes
  its place among the three by priority (strictly greater goes higher)
  and the slot of the record it pushes out. In a descending group the
  first three that answer take slots 2, 1 and 0, and a later light of
  equal priority is not asked.
- **The port.** `piney_world::chara::light_matrix` sorts the lights
  stably by descending priority and takes the first three that answer.
  `tools/test_lights_rs.py` checks it against the game's code: groups
  built with the game's `ccCreateLight` and `AddGrp`, then
  `SetLightMatrix` at a point, over random groups of distant, omni and
  boss lights. The slots, directions and colours match.

`gateAnm` (0x004593e0) by state (+0x1ec), which `chaosGateInfluence`
(0x00459570) sets from the gate menu's command (+0x9c):

```
command 11: state 0 -> 1, else -> 3        command 0: -> 5
1  SetAnm(ANM_xmgtcir2); count = 0; -> 2
2  when the circle has ended -> 3; count++, sound 71 (ccSeOn3D) as it passes 30
3  SetAnm(ANM_xmgtcir3) (193 frames, looping); -> 4
5  SetAnm(ANM_xmgtcir4); sound 72; -> 6
6  when the circle has ended -> 0
```

## Everything else in Mac Anu

`WORLD_MAN::RequestCCS` (main 0x0019d410) loads only `xallow0`, `town01`
(or `town01d`) and `WAT1` for a town, and every model in `town01` belongs
to one of `ROOTTOWN01`'s pieces (the sky's clump, 32 model rows, 11
object rows; the `MDL_sr1sim*` morph targets are never drawn themselves).
What else draws on a new game's first minutes besides the town and Kite,
by task:

| task | what | class, address | file | kind |
| --- | --- | --- | --- | --- |
| `ccThEntryCtrl` (64) | the Chaos Gate at `DMY_gate` | `ccChgate` gcmn 0x00458f60 / 0x00459280 | `CHGATE.CCS` | prop (ported) |
| | five merchants (npcTbl 0-4: Weapon Shop, Elf's Haven, Item Shop, Magic Shop, Recorder) at `DMY_merchant1`-`5`, drawn within 4600 of Kite and in the camera's view (`ccCheckCameraDeg(pos, 12288)`) | `ccMerchan` gcmn 0x00505aa0 / 0x00505e20, `setMerchant` 0x00505590 | `CTR1.CCS` (`CMP_trall`, `ANM_ctr1nut0`, `act0`, `act2`) | NPCs |
| | 14 walking PCs from npcTbl 30-79 by the Mersenne Twister, routing between `markPosTbl` landmarks, hidden beyond 4400 | `ccRtownPC` gcmn 0x00507c20, `ccEntryRandomNpc` main 0x001b6cf0 | `CTBU2.CCS`, `CTWM3.CCS`... | NPCs |
| `ccThFellow02` (50) | Orca, event 2 | gcmn 0x0041ed60 | the event's character file | NPC |
| `ccThEffect` (80) | the arrival's effect around Kite | `effTransfer` main 0x001ce020, `ccEffect::InitEffect` | the common effects | effect |
| `ccThParticle` (98) | particles (not traced) | main 0x001bc2f0 | | effect |
| `ccThFieldDisp` (96) | water copy 0 (frame buffer at screen UVs), the minimap | `waterUVModifi2` gcmn 0x005025d0, `ROOTTOWN01::DrawMap` 0x00422d00 | `wat1`, `TEX_sr1map1`, `xallow0` | town, HUD |
| `ccThMenu` (34), `ccThGameCtrl` (33) | menus, the command target | gcmn 0x005280d0, 0x00517800 | | HUD |

The merchants' booths, the Recorder's among them (-850, 3600, 300, under
the scroll sign), are town geometry; the town's five omni lights stand by
them. Town 1 adds `ccSetDog` and `ccSetChibiGuso`, towns 2-4
`ccSetChibiGuso` (town 4 has no walking PCs).

Water 0 draws with the picture behind it: `waterUVModifi2` (reach 9000)
writes each vertex's screen place into its texture coordinates, and
`ccLayer::MakePacketDrawBuffTrans` copies the frame buffer into its
texture just before it draws, both as in Dun Loireag
([its page](town02.md#the-water-and-the-frame-buffer-copy)).

## Dun Loireag

The second Root Town, `game.town` 1, is `ROOTTOWN02`; its class, draw,
clouds and lens flare are [its own page](town02.md), which also lists what
the town's game takes from the town number: the start (0, 3500, 0), the
gate at (0, 4200, 0), the collision (`HIT_sr2town1hit`), the merchants
(rows 5-10), the walking PCs' landmarks (`markPosTbl[k][1]`,
`naviMapTown2`), the event markers, the music (`sqDataTown` row 1) and the
server (1). `World::enter` builds either town from the save's `lastTown`,
and the session changes scene between them (the Chaos Gate's Other
Servers, the scripts' `scene`).

## Characters

Every character on the field is a `ccChar` (gcmn chara.cpp, 0xe0 bytes);
the entry control's are `ccEntryObj`s (entctrl.cpp, 0x1d0 bytes) and their
classes extend those (`ccGimmick`, `ccMerchan` 0x210, `ccRtownPC` 0x2f0):

```text
ccChar     +0x00 base (ccCharBaseParam: +0x08 type flags, +0x0c id, +0x18
                 height, +0x1c width, +0x20 msg)  +0x40 pos  +0x50 posP
           +0x60 dirc  +0x80 hitAttribute  +0x88 transparency
           +0x8c setTransparency  +0x90 transDist  +0x94 affectFunc
           +0xbc cmndLink  +0xc0 cmndSort  +0xc4 cmndDist  +0xc8 cmndDirc
           +0xcc ccsc  +0xd0 clump  +0xd4 anm
ccEntryObj +0xe0 flags (dispSW bit 4, cmndFlag bit 6)  +0xe4 plDist
           +0xe8 plDirc  +0xec alpha  +0xf0 fadeFlag  +0x100 entParam
           (ccEntryParam 0x60: pos, dirc, type, id, area ... param[4])
           +0x160 bodyHit (ccCharHit)  +0x1b0 anmTbl
```

`npcTbl` (gcmn 0x00619460) holds the 175 town NPCs, 0x70 bytes each: the
base parameters (`ccNpcParam`) and a `ccEntry` (the constructor the entry
control calls - `ccEntryRtownMerchant` 0x00505a50 or `ccEntryRtownPC`
0x005067a0 -, a palette name, and the file: `CTR1.CCS` for Mac Anu's
five shops, rows 0-4; `CTBU2.CCS`, `CTWM3.CCS`... for the walking PCs,
rows 30-79). Their height is 180 and their width 100 (shops) or 45.

The port's layer: `Body` (body.rs) is a file's `CMP_trall` clump with
models hung on its nodes (`ccObj::SetModel`: Kite's blades) and palette
swaps, drawn by `ccAnm::Draw`'s rule - every clump node with a model, in
the clump's order, at its world matrix; `Char` (char.rs) is `ccChar` plus
what `ccEntryObj` adds that drawing needs, with `ccChar::Draw`'s camera
fade (`ccGetCameraTransparency` with the base size, through
`ccPlayer::W2PPos`) and lights. Kite's `Kite` draws through `Body`; the
merchants and walking PCs are `Char`s behind the `Npc` trait (entry.rs:
their code, flags, one frame of `ccEntryObj::routine` and `main`, whether
they are on a command list, their `affectFunc`, the talk's line, and the
event instructions their own event mode takes).

### The merchants

`ccEntryEventMng` (main 0x001b62e0) calls `ccSetMerchant(0)` (gcmn
0x005057f0) before `ccSetChaosGate` and `ccEntryRandomNpc`. In area 0 it
runs `setMerchant(id)` (0x00505590) for the town's rows (town 0 rows 0-4,
town 1 5-10, 2 11-16, 3 17-22, 4 23-28): an entry parameter of type 2
(NPC) at the dummy jump table 0x006ac890 names (position +0x10, heading
+0x20), then `ccEntryCtrl::entryObject`, which drops it on the floor
(`ccLandHitCheck(pos, 0x20000002)`) and calls `npcTbl[id].entry.func`
(`ccEntryRtownMerchant` 0x00505a50) and `initObject`.

| row | name | flags | dummy | position | heading |
| --- | --- | --- | --- | --- | --- |
| 0 | Weapon Shop | 0x100 | `DMY_merchant1` | (2400, -2450, 0) | -180 |
| 1 | Elf's Haven | 0x200 | `DMY_merchant4` | (2400, 2450, 0) | 0 |
| 2 | Item Shop | 0x400 | `DMY_merchant3` | (-2400, 2450, 0) | 0 |
| 3 | Magic Shop | 0x800 | `DMY_merchant2` | (-2400, -2450, 0) | -180 |
| 4 | Recorder | 0x1000 | `DMY_merchant5` | (-850, 3600, 300) | 90 |

`ccMerchan::ccMerchan` (0x00505aa0, 0x210 bytes): `CMP_trall` of
`CTR1.CCS`, `anmTbl = merchanAnmPtr[town]` (0x005ed870; Mac Anu's
`ANM_ctr1nut0` idle, `ANM_ctr1act0`, `ANM_ctr1act2`), a body hit of
radius 35 and height 120 (type 2), no palette swap. Ids 10/16/22/28 (the
Grunt shops) and 29/158 (administrators) take `breederInfluence`
(0x005065d0: commands 14/15 turn toward Kite at 64 playing `anmTbl[2]`,
0 turns back to the default heading at 128); the rest keep
`ccGimmickAffect` (0x00453400), which only sets `affectFlag`. Mac Anu's
five never turn.

Each frame `ccThEntryCtrl` runs its enemies, magic circles, gimmicks (the
Chaos Gate), then its NPC list in entry order - the merchants first. For
each, `ccEntryObj::routine` (0x0042fa60): an event's turn (`grotDeg`,
`grotSpd`) through `ccSetDirc`, `plDist` and `plDirc` toward Kite,
`dispSW` within 7000, beyond 10000 frozen and off the command list
(`ccDeleteCmnd`), else `ccEntryCmnd` (the object list), and a fade when
`fadeFlag` is set. Then `ccMerchan::main` (0x00505e20): `posP`,
`ccCheckCameraDeg(pos, 12288)` (main 0x001da710: within 67.5 degrees of
the camera's line of sight, on the ground), `breederAct`, the body hit
(`ccCharHit::CollisionDetection` 0x00153470 against the other bodies and
the town), and only when `|posP| < 4600` and in view: the animation
steps, the matrix is set and `ccChar::Draw` draws it (layer 5, then 0).

### The walking PCs

The other "players" wandering Mac Anu are `ccRtownPC`s (gcmn rtownnpc.cpp,
0x00506640-0x0050933c, 0x2f0 bytes over `ccGimmick`).

- Who: `ccSetupGameCtrl`'s `ccInitRand` (main 0x001d9900) seeds the
  game's MT19937 with 4352 and draws it `ccSys+0x358` times (frames since
  boot). `ccRegisterRandomNpc(n)` (main 0x001b7660) then takes `16 - n`
  distinct `npcTbl` rows `30 + |ccRand() % 50|` (the next free row on a
  repeat), `n` the event's NPCs plus the registered party: 15 PCs for
  Kite alone, 14 with event 2's Orca.
- Where: `ccEntryRandomNpc` (main 0x001b6cf0), after the merchants and the
  gate, gives slot `k` to `ccSetRtownPC(row, k)` (gcmn 0x00506640):
  `markPosTbl[k][town]` (0x005ee120) names four landmarks - the start, the
  first target (-1: stand and chat), a chat group and a place in it. The
  constructor (0x005067f0) picks the model type by the file's name
  (`rtpcCcsName` 0x005ed900), its acts (`rtpcAnmTbl` 0x005edeb0) and
  weapons (`rtpcWeaponName` / `tvpcWeaponName`), and `changeTEX`
  (0x005076c0) swaps `MAT_tex` for a variant; `initObject` puts its body
  (kind 2) on the character list.
- Routes: ccnavi.cpp (0x005130b0-0x00514888). Mac Anu's 72 landmarks are
  its `DMY_markerNN` dummies, on 6 main lines; `RouteSearchByMap`
  (0x00513720) starts at the nearest landmark (within 10000 and 301 in
  height) and goes direct or junction - main lines - junction, at most 4
  lines deep.
- Each frame: `routine`, then `ccRtownPC::main` (0x00507c20). A PC hides
  beyond 4400 of Kite (its `posP` keeps z, so its height counts), at most
  10 show at once (walkers aside), its body is pushed out of the walls
  within 300 of Kite and out of every polygon otherwise, and it animates
  and draws only in the view cone, through `ccChar::Draw`'s fade. `move`
  (0x005077f0) turns a quarter of the way a frame, walks 10.5 a frame,
  sidesteps +-0.5 rad while pressed against a body, and turns back after
  60 frames spent within 100. Acts: 6 a walker's start, 3 walking, 0
  hidden, 1 waiting unseen, 4 at a shop facing its merchant, 5 a chat
  group taking turns to speak, 7 the gate (not reachable in Mac Anu), 2
  spoken to.
- Talking: `rTownNPCInfluence` (0x005091c0): 14 (`PcMenu` 0x00541e90
  opening) and 15 (`TalkMenu`) stop it facing Kite (act 2; a chat group
  holds still); 0 sends it back to its walk or group. `PcMenu` shows
  `base->msg[0]`. A PC comes onto the command list (`ccEntryCmnd`) as it
  starts walking (acts 1 and 6 to 3), in a chat group and at the gate's
  far side, and off it (`ccDeleteCmnd`) as it hides; `routine` never puts
  it back (its `cmndFlag` stays set).

`ccCharHit::CollisionDetection` (0x00153470) pushes a body on the
character list (`ccCharHitTop`, in `HitEnable` order: the merchants, the
walking PCs, Kite at the end of his arrival) out of every other listed
body whose kind its mask names by `r1 + r2 - d` along the line between
the centres, and records the kinds touched (`hitResultCharType`);
`ccSpcChar::HitCheck` sets its result's bit 0 when a PC was touched, and
with Kite's AI in manual mode masks him with -8 so he passes through them
(a new game's Kite is not in manual mode).

### What the event scripts use

A character is named by an event `type` and `code` (`ccEvent::GetSpc`
0x001b2bb0, `GetNpc` 0x001b2cc0, `GetEnemy` 0x001b2d50): types 0-2 the
party (code a `charTbl` row: 0 Kite, 2 Orca; -1/-2 a party slot, -3 the
party), 3-4 a town NPC (code its `npcTbl` row; 29 and 158 are
merchant-like), 5-6 an enemy or object. `ccEntryEventMng` (main
0x001b62e0) places the event's registered entries when an area starts:
the party at a marker (`ccEntryCmnd` when `param` is 5), types 3/4 through
`ccSetRtownPC(code, marker)` / `ccSetMerchant(code)`, 5/6 through
`ccEntryCtrl::entryObject` at an event position, 7 a boss. A marker in a
Root Town is `markerEvTbl[marker]` (gcmn 0x00317da0: 0 `DMY_gate`, 1-30
`DMY_marker_ev01`-`30`, 31 `DMY_marker71`, 32 `DMY_marker30`), a dummy of
the town's file: position (+0x10) and rotation (+0x20; z the heading).

Events 2-4 use `entry` (event 2: Orca, type 2 code 2, marker 3, param 5),
`add_target`, `pc_act`, `pc_mode`, `pc_turn`, `pc_face`, `pc_put`,
`pc_walk_dir` and `pc_command`. What they do to a character:

- `pc_put` / `npc_put`: the position `10 (x, y, z)`; `pc_put_marker` /
  `npc_put_marker` (`ccEvPcPos` 0x001ae7d8): the marker's position, then
  `ccAI::SetDircZ(RAD2DEG(marker.rot.z))`.
- `pc_turn` / `npc_turn` (`ccEvPcRot` 0x001aed5c): `chg` 0 turns at once
  (`ccAI::SetDircZ` 0x00581500: `ai+0x9a = dirc`, `dirc.z = DEG2RAD(dirc)`);
  otherwise the AI turns toward it at `chg` a frame (1 meaning 64:
  `ai+0x9c`).
- `pc_face` / `npc_face` (0x001aee30): the same toward the heading
  `ccGetDirc` (0x001d9ce0: `pi/2 + atan2f(dy, dx)` wrapped into
  (-pi, pi]) gives from the character to the other.
- `pc_act`, `pc_mode`, `pc_turn` and `pc_face` with a gradual turn drive
  the party's AI under manual control, below. `pc_walk_*` set remote
  command 1 (2 for `pc_run_pos`) and a straight goal (`SetGoalPos(v, 0)`,
  `gPoint` -1), and `ccAI::ManualControl` walks there by `MoveP2P`: a
  step toward the goal, facing it, until the ground distance less the
  body's radius is under 50 (the radius widens on the run: Kite's 45 is
  72.5 running). Then the body stands, the command is done with
  `remoteFlag` 1 and `gDeg` the heading it had. In a town a member's walk
  is the battle's `ManualControl` (the town's party runs on the battle's
  machinery) and Kite's the town's own (`piney_world::ai`). `pc_command`
  puts the characters on the command list or takes them off. Event 21
  walks Elk up to Kite in Mac Anu this way, and event 30 walks Kite and
  BlackRose in Dun Loireag. `npc_act` and `npc_walk_*` are a PC's event
  mode (`ccRtownPC::eventMode`).

The condition `near_marker marker bounds comp` (`ccEvent::CheckOpen`,
main 0x001a7d9c) measures `ccGetDist(ccTransPosW2P(pos), plw->posP)`, on
the ground, against `10 bounds`: `pos` is `markerEvTbl[marker]`'s dummy
in a town, the `evPos` of that number in a field or dungeon, and the
player's frame wraps through the map's bounds. Event 13 waits on it in
Mac Anu (marker 31, 80) and event 17 in area 18's dungeon (marker 1, 250:
Mia's line sending Kite to the magic portal, block 14, before the Data
Drain's lesson). The port's town answers it with `World::player_distance`,
the fields and dungeons with `FieldWorld::player_distance` through
`AreaHost`; `event_17_walks_kite_to_the_portal` plays event 17 from its
dungeon's entrance through block 14, the portal's fight and block 17.

`item_add_menu` while an event plays opens the item-get menu (29; the
menu gives the item) and waits for it to close, then sets `ccMenu +0x0c`
(`panelStatus`) 3, in a town as in a field: event 16's contest prize in
Mac Anu (the Book of Law, `item_add_menu 0 15 60 1`) comes this way.

A party member's acts are `fellowAnimTbl[code - 1]` (gcmn 0x005d4050,
22 names a row: 0-2 `nut0`-`nut2` standing, 3-4 fidgets, 5 `run0`, 6
`wal0`, 7-10 damage and down, 15-16 attacks, 17-21 magic and skills); its
flags, height and width are `saveData.spcParam[code]`'s base (+0x08: 6,
+0x18, +0x1c), its file `charTbl[code]`'s (DEMO.PRG 0x0040dc80, 92 bytes a
row, the name at +4).

The port's `World` answers the interpreter's host (`piney-event`'s `Host`)
through `entry`, `remove`, `marker`, `command_target_ref`, `pc_command`,
`npc_command` and `set_event_targets` (event.rs). `entry(type, code,
marker, param)` registers a party member (`EntrySpc`, its `bootParam`),
which the town's set-up then builds (`ccSPC::Reboot`, below) and puts at
the marker (on the command list when `param` is 5; after the set-up both
at once), or moves Kite for code 0, and queues a town PC (type 3) for the
entry control's set-up,
`World::place_entries` (run by the first frame of play if the caller has
not): as `ccEntryEventMng` does, the event's town NPCs first
(`ccSetRtownPC`, then the marker's position and rotation), the merchants,
then the walking PCs, whose number counts Kite, the party members and
those NPCs; `remove(2, code)` takes one out (`ccParty::DelMember`,
`ccSPC::DelSpc`; `remove` of types 5/6 and -1 deletes objects and
enemies, of other types nothing). Puts, instant turns and
faces for Kite, the fellows and the NPCs are done there; the rest goes to
an NPC's `Npc::command` or is declined (false). The party's own gradual
turns, acts and modes go through party.rs and ai.rs, below.

Talking to a character goes to the event, not its menu, as
`ccEvent::CheckOperate(9)` decides: one of `eventMng.target[]`'s (type,
code) pairs has its type as a bit of the character's base type flags
(`cmndTarget->base->type`) and the same code. So the Chaos Gate (type 13,
a gimmick) can be one as well as a PC or an NPC; event 11's block 6 waits
on it. `ccCheckTarget` (the talked-to character still placed) is
`World::target_alive`. The town host also plays the scripts' `stream`
(`ccEventStream`, with its subtitles and music) while the call waits, its
frames shown instead of the town's, and hands the menus the registry
(`ccSpcManager.registry[registryNum]`) that PERSONAL's Party > Add reads.

### The event's NPCs outside the towns

Story events stand town PCs and the Administrator in fields and dungeons
too (`entry 3|4` after a `set in_field` or `set in_dungeon`):

```text
event 14  field 17: entry 3 84; its dungeon: entry 4 29
event 17  field 18's dungeon: entry 4 29 (add_target, talked_to)
event 26  field 14: entry 3 83
event 27  field 24's dungeon: entry 3 85 (add_target)
event 29  field 26's dungeon, point 1: entry 3 86 (Meg), entry 4 29
S109      field 33's dungeon 1: entry 3 87-90
```

`ccEntryEventMng` makes them as in a town, by the type's bit: 0x08 (type
3) `ccSetRtownPC(code, ...)`, 0x10 (type 4) `ccSetMerchant(code)`. Both
are an `entryObject` of type 2 on the current area, floor and block
(`entRoot` -1; `ccSetRtownPC` puts its marker argument in `param[0]`,
-1 here, and takes the PC off the command list with `ccDeleteCmnd`;
`setMerchant` leaves ids 29 and 158 at the origin with no dummy). The
entry control calls `npcTbl[code].entry.func` (`ccEntryRtownPC`
0x005067a0, `ccEntryRtownMerchant` 0x00505a50), so these are the towns'
own classes: `ccRtownPC` (0x2f0 bytes) and `ccMerchan` (0x210). Then
`ccEntryEventMng` (0x001b6750-0x001b6980) writes the object's position
(+0x40) and heading (+0x60) alone:

```text
marker < 0       (0, 0, 0, 1) both
area 0 (a town)  markerEvTbl[marker]'s dummy: position (+0x10), rotation (+0x20)
area 1, 2        the evPos numbered marker of eventMng's 16 (+0x1c0, 0x20 each;
                 none: the last looked at, the 16th); with floor and block 9999
                 Kite's position added; heading (0, 0, evPos.dirc, 1)
```

A PC with `param[0]` -1 starts in its event mode (`+0x1e0` 1, `+0x2a8`
2) and never reads a navigation map, so none is needed outside the towns;
it moves only as `npc_*` instructions send it. `ccMerchan::ccMerchan`
gives id 29 `sysopeAnmTbl` (0x005ed850) and 158 `quizmanAnmTbl`
(0x005ed860), whatever the town: the same clip names as `merchan1AnmTbl`
and `merchan2AnmTbl` (`ANM_ctr1nut0`, `act0`, `act2`; `ctr2`). Both
take `breederInfluence` and a live `bodyHit`, and `main` runs
`sysopeAct` for them.

The instructions event 29 uses on them: `npc_put`, `npc_turn` (`chg` 0),
`trans_off 3|4` (`ccChar` +0x90 off: no fade as the camera comes
close), `npc_face` at another NPC (type 3 or 4, found with
`ccEvent::GetNpc`: mask 0x10 for codes 29 and 158, else 0x08) or at Kite,
gradual (`chg` 128, 256), `npc_act 29 4` (the Administrator's
`effTransfer` and fade) and `npc_act 86 4` (Meg's event act 4: her
`effTransfer`, then gone). The party's `pc_walk_char 3 86` and `pc_face 3
86` find her the same way.

The port keeps each such NPC as two halves (piney-world `field_npcs.rs`).
The battle's entry control holds a stand-in (`combat::EventNpc`: an NPC
entry with the row's base, `entry::npc_char`) for what the battle's side
reads: the command lists and target (`talked_to`), `GetNpc` for the
party's instructions, and the character the effects follow. Its body stays
out of the hit list. The world holds the class itself (`RtownPc` over a
`TownPcs::outside` with no landmarks; `Merchant::at`), which steps after
the battle's tasks with the field's collision, is drawn with the field's
lights and fog, and takes the NPC instructions (`FieldWorld::npc_command`,
`set_trans`; `AreaHost::npc`, `trans`). Each frame the stand-in is put
where the class stands, on or off the command lists as the class is; the
starts (a PC's `PcEvent::Transfer`, the Administrator's `SysopEvent`s) go
to the next frame's effects as `Show::Npc` (effTransfer; act -5's force
rings, tornado rings and sound 217), `noise2` to the menus; a merchant's
`main` returning 1 (act 5's end) deletes the stand-in (`deleteNpc`,
gcmn 0x00431860).

Talking to one goes through `ccThGameCtrl` as in a town. The action
button's `CheckOperate(9)` makes the command target `operateTarget`, and
a target on `eventMng.target[]` (`add_target`) is the event's: no menu
opens, and the event's `talked_to` holds for that frame. The port's
field does the same test (`FieldWorld::set_event_targets`, the target's
base type flags and id), and the area hands the event the target
(`operate_target`). `event_17_talks_to_the_administrator` walks Kite up to
the Administrator after event 17's block 8 and talks to him twice; block
9 (`if talked_to 4 29`, `npc_face`, line 10) plays each time.

### The party under the event scripts

`ccSpcManager` (gcmn 0x00730340, a `ccSPC`) registers the party characters
an area has; `ccPartyManager` (0x00730310, a `ccParty`) is the party:

```text
ccSPC          +0x00 registry[5] (ccSPCRegistry, 0x2c each)  +0xdc registryNum
ccSPCRegistry  +0x00 id (charTbl row, -1 free)  +0x04 partyFlag  +0x08 bootParam
               +0x0c reserveWeapon  +0x0e oldWeapon  +0x10 equip  +0x1c charPtr
               +0x20 body  +0x24 weapon  +0x28 weaponNew
ccParty        +0x00 memberChar[3]  +0x0c memberID[3] (-1 empty)  +0x18 num
```

`ccSetupNewGame` runs `ccSPC::Initialise` (0x0059f5f0: registry 0 Kite,
partyFlag 1, bootParam 0, registryNum 1) and `ccParty::InitParty`
(0x0059ce00: Kite alone, num 1). An area's set-up then:

1. The phase 0 pass runs `pc_mode` (Execute case 65, main 0x001adf74,
   level 2 only): `bootParam` of the registered id `pc` (the first slot,
   whole word compared), of each party member's (-3), or of slot `-pc`'s
   member.
2. `ccSetFileListTown` calls `ccRegisterEventMng(0)` (main 0x001b6d70):
   each `entry` of types 0-2 is `EntrySpc`'d (0x0059f740: its slot, or the
   first free one, registryNum + 1, -1 when full), and that slot's
   `bootParam` becomes the entry's `param` (event 2's Orca: 5). registryNum
   is then added to registNpcNum for `ccRegisterRandomNpc`.
3. `rebootSpcManager` runs `ccSPC::Reboot` (0x005a00d0): for each slot
   `charPtr = ccSpcStart[id](slot, 0)` (0x00654070), then `new ccAI(id)`
   with `ChangeMode(1, 1)`, and `ManualMode` when bootParam has bit 2;
   finally `SetParty` (0x0059fe80) fills memberChar. `ccFellow::Initialize`
   stands a character at `StartPos[slot]`, which `ccGetStartPositions(1,
   2, 3)` (gcmn 0x0059ff50) filled for each registered slot 1-4: a party
   member at its party slot's `SetCharPosition` place (in a town the
   start + (200, 100), + (-200, 100), + (0, 300), facing as Kite does),
   anyone else at the origin facing 0.

The registry and the party are the game's globals: back from a field, the
town's set-up builds the party the field left (the port's session hands
the area's `Spcs` to the town, every `bootParam` 0 as `ccThSpcDelete`
leaves it). `piney-game`'s `the_town_takes_the_party_back` goes from area
31 to Mac Anu with Piros: the town's party is [0, 8, -1] and he arrives at
(200, 5700, 600).
4. `ccEntryEventMng`'s party step (main 0x001b6304-0x001b654c) puts each
   entry's character at its marker (the dummy's position, then
   `ccAI::SetDircZ(RAD2DEG(rot.z))`; the origin without a marker), with
   `ccEntryCmnd` when `param` is 5.

The constructors apply bootParam through `ccSpcChar::SetBootStatus`
(0x0059e950) before Reboot makes the AI:

| bit | effect |
| --- | --- |
| 2 | ManualMode if there is an AI (there is not yet; Reboot calls it); the constructors skip ccEntryCmnd |
| 0 | act 2; the body on the character list when alive (dead 0) |
| 1 | act 14 (out of sight), transferLag 0, transparency and cloak 0, the body off the list |
| 3 | act 13 (arriving), transferLag 1, transparency and cloak 0, the body off the list |

After that, restraintSW is set unless the act is 2. Event 2's `pc_mode -3
6` starts Kite in act 14, invisible, under manual control and off the
command list. `ccFellow::Initialize` (0x0041ae80) gives a fellow in a town
act 13 after `(rand() % 4) * 5` frames at `StartPos[slot]` (the origin
when not in the party); Orca's bootParam 5 then stands him in act 2 with
his body first on the character list.

`ccAI` (0x260 bytes):

```text
+0x00 manualSW 0, followSW 1, talkFlag 2, runFlag 3, remoteFlag 4, goBackFlag 5,
      inviteFlag 6, selfFlag 7
+0x01 battleFlag (2 bits), targetFlag (2), chatCmdFlag (3), firstTime
+0x03 skillMask   +0x06 strategy   +0x08 mode   +0x0c modeOld   +0x10 count
+0x18 bodyPtr     +0x74 remoteCmd  +0x76 chatCmd  +0x80 arrivalChatCnt
+0x9a gDeg (u16)  +0x9c gRotSp     +0xb4 detourCnt  +0xb6 levelOld
+0x160 sysMsg (id -1 until SysMsgEntry)
```

The constructor (0x0057c5f0) leaves mode 0, gRotSp 64, chatCmd -1 and
arrivalChatCnt 150 in a town.

- `ManualMode` (0x00583270): manualSW 1, gDeg = RAD2DEG(dirc.z), gRotSp
  64, remoteCmd 0; remoteFlag kept.
- `SetRemoteCmd` (0x005832e0): remoteCmd and remoteFlag 1; command 0 also
  takes gDeg from the heading; command 5 marks a character outside the
  party (partyFlag 0) as leaving (-2).
- `SetDircZ` (0x00581500): gDeg = dd and dirc.z = DEG2RAD(dd), at once.
- `ManualModeAI(1)` (0x0059eba0): unless the party is wiped out, a dead
  character is revived and acts 9-11 are stood up; then ManualMode.

While manualSW is set, `ccPlayer::Main` runs `ccAI::Brains` instead of
`ControlMove` and moves from the flags `ManualControl` left
(0x005985bc-0x005986f0): speedRate 140/140 walking, 100/100 running, 0
standing; `movePos = (v sinf(z), -v cosf(z))`, v = speedRate * speedValue
* tsp walking, or the speed running. `HitCheck` masks the body with -8 (it
passes through everyone), and in act 14 or dead 4 takes it off the list;
`AnimCtrl` holds the fidget off and turns act 0 into 1.

`Brains` (0x0057ca00): nothing in mode 0; with talkFlag it turns the body
toward gDeg at 64 and stops; otherwise SysMsgEntry or ReadSysMsg (nothing
with no message queued), levelCheck (only levelOld under manual control),
ChatCommand when partyFlag is 1 (with no chat command, out of battle: the
leader's strategy is cleared), else skillMask = 3; a living character in
mode 6 goes back to 1, a ghost (dead 4/5) to mode 6; then ManualControl,
ChatMessageSender (arrivalChatCnt counts down; ChatMessageEnteredTown is
silent under manual control), count + 1, detourCnt - 1.

`ManualControl` (0x00580ef0, switch 0x006f01a0):

| remote | what |
| --- | --- |
| 0 | moveFlag and runFlag 0; `ccSetDirc(&dirc.z, DEG2RAD(gDeg), gRotSp)`; remoteFlag while not stopped at gDeg. gDeg is read unsigned against the sign-extended RAD2DEG, so 0x8000 or more never counts as reached |
| 1, 2 | walk (1 running, 2 walking) by MoveP2P, or in a town by the town navigator's points (`TownNavigatorPoint`; piney-battle's `ai_move::manual_control`) |
| 3 | act 2: done; act 14: TransferIn (0x0059ea60: act and old act 13, actCnt 0; the arrival plays on the animation already running) |
| 4 | act 14: done; act 12: wait; else TransferOut |
| 5 | act 14: nothing; act 12: wait; else TransferOut, then resignParty (partyFlag 1) or disbandSpc, then partyFlag -2 |
| 6 | act 14 (old -1); transparency, setTransparency and cloak 0; done |
| 7 | act 14 becomes act 2 (old -1), the body back on the list when alive; transparency, setTransparency and cloak 1; dispSW 1; done |

`TransferOut` (0x0059ea80), when shown and not faded: act 12 (keeping the
animation from act 2 or 4), anmFlag 0, actCnt 0, moveFlag 0, the body off
the list; off the command list too unless under manual control. AnimCtrl's
act 12: at actCnt 1 effTransfer and the body off the list; cloak 1 until
21, then (41 - actCnt) / 20; at 141 anmFlag set, and the act becomes 14.

`ccGetDircChg` (main 0x001d9eb0) compares `spd & 0xf0000` with 1 and 2,
which it can never equal, so a negative rate takes the whole difference
(at most 24576). Once the step is 1, `RAD2DEG(DEG2RAD(s + 1))` can
truncate back to s: a turn stops short (event 2's Kite ends 8 units short
of Orca) with remoteFlag still set.

The instructions (all at level 2):

- `pc_act` (case 60, 0x001ad0c0; tables 0x00355fa0 for pc >= 0, 0x00355f70
  for -3, 0x00355f40 for -1/-2): act 0 manual off; act 1 ManualModeAI(1)
  when Kite is named by his id, else manual off; acts 2 and 8
  ManualModeAI(1) and SetRemoteCmd(0); acts 3-7 ManualModeAI(1) and
  SetRemoteCmd(act); then noDeathFlag (+0xe0 bit 7) set for acts 1, 5 and
  8 (1 and 8 also ccEntryCmnd), cleared otherwise.
- `pc_turn` / `pc_face` (71 0x001aed5c, 72 0x001aee30): on `GetSpc(pc)`,
  or `memberChar[-pc]` for a slot (-3 reads memberID[0] there, so no one);
  chg 0 is SetDircZ at once; otherwise gRotSp = chg (1 meaning 64) and gDeg
  the heading, turned by remote 0. pc_face's target by `1 << type`: & 4
  GetSpc, & 0x18 GetNpc, & 0x60 GetEnemy; types 0 and 1 name no one.
- `party_add` (77) is `ccParty::AddMember` (0x0059ce80), as is the menu's
  `ccThPartyAdd` (gcmn 0x0053ba20): the first empty slot takes `inviteSpc`
  (0x005a08e0): the registry's partyFlag 1; the character's partyFlag -2
  turns recall on, -1 and 0 become 1; num + 1. A member the registry does
  not hold (called by his address from PERSONAL, Party, Add) is made
  there first: `EntrySpc` (none with the registry full), his file queued
  (`spcTempFileList`, type 9, `ccLoadFLAddOne`), the registry's partyFlag
  1 and the character built by `ccSpcStart[id](slot, 0)` with a new
  `ccAI` (`ChangeMode(1)`), standing at `StartPos[slot]`:
  `ccGetChgatePosDirc` copies the Chaos Gate's +0x40 position and +0x60
  heading; the angle `DEG2RAD(RAD2DEG(heading) + inviteOffsetTbl[slot])`
  (the table, gcmn 0x00654130, u16 by slot: 0, 0x2000, 0xe000, 0x4000,
  0xa000) goes through `fptosi` before `sinf` and `cosf`, so whole
  radians only; x gains 150 sin, y loses 150 cos, w 1; he faces half a
  turn from the gate.
- `party_remove` (78) is `DelMember` (0x0059cf60, slots 1 and 2 only),
  which calls `disbandSpc` (0x005a0f50: -2 if recalled, else -1).
- `menu_ban`, the party part: each member's manualSW saved in
  eventMng.spcMode (+0x79c), then ManualModeAI(1) and SetRemoteCmd(0);
  every registered character off the command list, transDist 0 (no near
  fade). `menu_clear`: every registered character back on the command list
  unless in act 14, transDist 1, skill 0; in a town each member returns to
  its spcMode, Kite's manual control always off.

`ccFellow::Main` (0x0041b5f0) under manual control: posP and back
(W2PPos, then P2WPos 0x0059b710: `(x - px) + px`); a recalled leaver
returns (act 13, partyFlag 1), a leaver in act 14 sets exitFlag
(the next frame `ccThFellow02`, gcmn 0x0041ed60, wakes to it instead of
running Main, calls `expulsionSpc(listNum)` and deletes the task, whose
`ccThFellow02Delete` (0x0041ee90) runs `~ccFellow` (0x0041ad60: the
registry's and the party's pointers to it cleared, the AI deleted, the
body off the character list, `ccDeleteCmnd`) and then `DelSpc(listNum)`
unless partyFlag is 1); Brains;
`ccFellow::Move` (0x0041bc50: moveFlag * speedRate * speedValue * (3.0
walking, or the speed running) along the heading); HitCheck; the floor;
posP and back again; `ccFellow::Action` (0x0041c670): AnimCtrl with its own
ended-act table (0x006a50d0; acts 12 and 13 also zero actCnt), frameSpd
256 speedRate speedValue walking and 256 (1.375 speedRate) speedValue
running; transparency = cloak, then Draw.

The port: party.rs, ai.rs, `Player::build` and the manual branch of
`Player::main` for Kite; the members are the battle's characters (see
below).

### The party in a town

The town's party members are the battle's characters, run by
`piney-battle`'s machinery as the fields run theirs
(`piney_world::town_party::TownParty`, `combat::town`): a `Combat` of area
0 holds them, each built by `Combat::add_member` (`ccFellow::Initialize`
and Reboot's AI, `arrivalChatCnt` 150), and each frame (`TownParty::frame`,
after `ccThPlayer`) runs `ccThSpc`, `ccThAISystem` and each member's
`ccFellow::Main` (`piney_battle::fellow::Frame::main` with the
`party_motion::Movement` runtime), whose `ccAI::Brains` takes a member out
of manual control through `ccAI::ActInTown` (gcmn 0x0057f660,
`piney_battle::ai_move`) and one under it through `ManualControl`. The
event instructions reach them as the field's do: `TownChars` lends Kite
(the town's `Player`) by id 0 and the members through the battle's records
(`combat::spc::SpcRec`), written back after.

Kite stays the town's own `Player`. The battle's scene holds a stand-in
for him (`Combat::add_leader`: his record, his AI, on the command list
unless bootParam bit 2) that `Combat::mirror_leader` brings up to date
before the members' frames: his place (posP z only, as his own frame
leaves it), heading, act, the step's flags, `dead`, partyFlag, dispSW, his
command list membership and his AI's manualSW. It has no body on the
character list (the player's is there) and no animation.

`ActInTown` out of manual control (checked in eemu by
`tools/test_battle_navi_rs.py`): `actType` 95 then 96 waits for Kite; with
no invitation (inviteFlag, a recall) and no chat order it rests (99), then
walks at random to one of the five shops (5-9: `naviPointNameTable`, gcmn
0x00653c80, eight names, 2-7 `DMY_merchant1`-`6`, 1 `DMY_gate`, whose
dummies are the town file's) or to a landmark (10, `ccSetNaviMap`'s),
rests there (11) and chooses again. The CHAT menu's order 9 (follow) makes
it 3 (go to Kite): within 200 of him 94 (follow), within 150 97 (at his
side). The routes are `ccNavi::RouteSearchByMap` over the landmarks
(piney-world's `NaviMap`, handed to the battle's `TownMap`); remote
command 5 in a town reaches the registry (`resignParty` / `disbandSpc`).
`ccAI::ChatMessageSender` (0x00586420): nothing once the party is wiped
out; a line asked for is opened (chatRequest cleared); `arrivalChatCnt`
counts down past 0, where out of a fight the arrival's line is said
(`ChatMessageEnteredTown` 0x0058f810 in a town, from
`arriveDeltaMessages` and the other servers' tables, silent under manual
control; `ChatMessageEnteredField` in a field reached from a town). The
lines are [the battle's chat lines](battle.md#the-chat-lines), opened in the
member's balloon.

The constructors draw `rand()` in order: `ccSpcChar::ccSpcChar` (0x0059d230)
`cycle = rand() >> 3`; `ccFellow::Initialize` `atkDellay = (rand() >> 3) &
31`, then (not hacked) `transferLag = (rand() % 4) * 5`; Reboot's `ccAI`
`count = rand() >> 3` and the message counters from one more. The port
draws the same for the members and Kite, in the fields and the town (the
town's Kite takes his cycle and his AI's count from his stand-in's draws).

`piney-game`'s `piros_walks_mac_anu_on_his_own` and
`piros_follows_kite_in_mac_anu` take Piros back to Mac Anu (event 22's
area 31): alone he waits, rests and walks off; told to follow (chat 9)
while Kite runs down the plaza, he keeps within 400 of him.


## Talking

`ccThGameCtrl` (gcmn 0x00517800, priority 33: before the camera and the
player) keeps the command target. Characters that can be spoken to or
acted on put themselves on a command list with `ccEntryCmnd` (0x00519630):
the party's (base type & 7; Kite is on it), enemies' (& 0xe0) or everyone
else's. Each frame, after the task's first 5:

```text
ccSortCmnd (0x00518af0)        each listed character but the leader:
                               d = posP - leader.posP (z 0), cmndDist =
                               sqrtf(d.d), cmndDirc = atan2f(d.y, d.x);
                               those within 3000 into one list by distance
ccSelectTarget(mode)           mode 0 with no target, 2 on a new lean of the
  (0x00518cc0)                 left stick past 64, else 1; each candidate's
                               ccCheckTargetRange (0x00519240): 1 within
                               60 + both widths, or within 300 + both widths
                               and 1.2 rad of straight ahead (the eye view:
                               500, or 300 for shops and gimmicks, and
                               0.49 rad); enemies first, then everyone else
                               (type & 0x070000ef clear), then the party and
                               the walking PCs (& 0x0700000f); mode 2 steps
                               cmndTargetPriNum through them
ccChangeCmndTarget (0x005198c0)
the action button              assignPADaction (saveData+0x8404, X; the other
                               field buttons are personal +0x8406 triangle,
                               chat +0x8408 square, option +0x840a start,
                               map +0x840c select: ccSaveData::Init
                               0x001743d0) pushed with a target, unless an
                               event intercepts operation 9: ccMenu opens
                               with the type the target's flags pick and
                               the player's +0xe0 bit 0 (pauseSW) holds him
                               until the menu closes (state 5, waiting for
                               ccMenuCtrl::CheckMenuType() == -1)
```

| target flags | `ccMenu` type | what |
| --- | --- | --- |
| 0x6 | 21 | a party member |
| 0x8 | 22 | a walking PC: its line |
| 0x10 | 23 | an administrator |
| 0x100, 0x400, 0x800 | 24 | the weapon, item, magic shop |
| 0x1000 | 25 | the Recorder (saving) |
| 0x200 | 26 | Elf's Haven |
| 0x2000 | 28 | the Chaos Gate |
| 0x4000 ... 0x200000 | 4128-4135 | field objects |
| 0x400000, 0x800000, 0x4000000, 0x2000000, 0x1000000 | 40, 4139, 44, 45, 46 | more objects |
| 0xe0 | - | an enemy: a skill request |

### The menu buttons

State 0 of the task runs only while `ccPlayerMenuCheck()` (gcmn
0x0059cd70: the party not wiped out, `ccSkillCheck(plw) < 2`, the
player's act neither 12 nor 13) passes, `ccLoadDispCheck()` is 0, no menu
is open (`CheckMenuType() == -1`) and `openReqNum & 0xfff` is not 74. After
the target, `ccSpcChar::CheckControlMode(plw)` (0 in play), and in this
order (gcmn 0x00517ff0 - 0x00518a98):

| check | then |
| --- | --- |
| `ccMenu.forbid` (`menu_ban`) | only chat, and only with `forbidChatExcept` |
| chat pushed, operation 10 | `openReqNum` 3, state 2 |
| option pushed, operation 12 | `openReqNum` 12, state 3 |
| sleep, paralysis, confusion or charm on the player | nothing (`plAttack` and `cmndTargetFix` cleared) |
| personal pushed, operation 11 | `openReqNum` 0 in a town; 2 on fields 1-12 and 67, and on 13 while operation 14 is held, else 1; 2 in a dungeon, 1 on its special floors (types 8, 9); state 1 |
| the player dead | nothing (as above) |
| action pushed, operation 9, a target | the target's menu (the table), state 5 |
| otherwise | `plAttack` cleared unless `ccSkillCheck(plw) == 1` |

Each sets `mode` 0 (the other tasks sleep) and `firstTime` 1; the action
button's menus set `mode` 1 (Kite held by `pauseSW` instead) but for a
party member outside a town (21) and a fountain (40). States 1-4 become 5,
which waits for `CheckMenuType() == -1` and clears `pauseSW`. The map
button (select, operation 13, `WORLD_MAN::ChangeMapMode`) comes before
`ccPlayerMenuCheck`; it and the map are `docs/engine/map.md`.

The port takes the buttons in `talk::Targeting::step` (`talk::Input`, with
`World::set_menu_view` passing what `ccMenu` shows) and raises
`TalkRequest::Open { menu }` or `ClearAttack`. The runtime
(`piney-game`'s world mode) sets them on the field UI and calls
`World::close_menu` when its menu type is -1 again.

`posP` is `ccPlayer::W2PPos(pos)`: the position less Kite's on the ground
(wrapped into the town's bounds), so the leader's own is (0, 0, z) - as
the game leaves his in a town, checked in eemu - and the others' are
where they stood at the end of the last frame relative to him.

The port (talk.rs, `World::game_ctrl`) raises a `TalkRequest` -
`Talk { npc, msg, line }` for menu 22, `Shop { npc, shop, msg, line }`
for 24-26,
`Menu { menu, kind, code }` otherwise, `Event { kind, code }` for a
character an event took over with `add_target` - from `World::take_talk`,
and holds Kite until `World::close_menu`. The menus themselves are the
field UI's. The PC and shop menus open with `EntryAffect(target, plw,
14)` and close with `EntryAffect(target, plw, 0)` (`ccChar::EntryAffect`
0x0056b020 calling the target's `affectFunc`), sent by the menus
themselves as their list opens and closes (and 15 as TalkMenu speaks);
the field UI raises them as `Request::Affect` and the runtime passes them
to `World::affect`, which calls the NPC's `Npc::influence`, so a PC stops
and turns to Kite while spoken to and walks on after. The shop pages drop
the command target and give it back (`ccChangeCmndTarget`), which the
runtime passes to `World::change_command_target`.

What the field UI reads of this: `World::command_sorted` (the
`cmndSortRoot` chain: kind, code, `cmndDist`, `cmndDirc`),
`World::targeting` (`cmndTarget`, `cmndTargetPrev`, `cmndTargetPriNum`),
`World::char_info` (name, type flags, height, a party member's
`spcParam` offset) and `World::tag_pos`: `ccCalcTagPosChar` (gcmn
0x0051a500) - `ccCalcTagPos` (0x0051a640) puts the character's position
plus an offset (0.45 of its height for the target cursor, 0.9 for the
life bar) through `sceVu0RotTransPers(world_screen)` and, with the depth
in (0, 0x0fffffff), gives `fptosi(x - 28672) / 16`, `(y - 29184) / 16` in
the frame buffer's pixels; mode 0 answers 1 inside (-127..640, -31..480),
else 0, other modes 1 inside (-19..532, -15..464), else 2. While a menu
has the tasks asleep (`ccSleepAllThread`), `World::set_asleep(true)`
stops every task and the frame still draws everyone where they stand;
`World::step_into` steps into a frame the caller finishes, so the HUD can
draw on its own layers under the fade.

A shop's menu (called from 0x00528410 on: `VenderMenu` 0x00542840,
`RecorderMenu` 0x00542df0, `FairyshopMenu` 0x00543390) opens with
`EntryAffect(target, plw, 14)`, the greeting `ccMessage::Open(ccMsg,
base->msg[game.server], base->name, -1, -1)` and `SetMerchantCamera`
(below); cancelling sends `EntryAffect(..., 0)`, `changeCamera(1)` and
closes the message. `Shop`'s `msg` is the merchant's table and `line` the
server (0 in Mac Anu): the Weapon Shop's 0x00631920, Elf's Haven
0x00631970, the Item Shop 0x006319c0, the Magic Shop 0x00631a10, the
Recorder 0x00631a50 ("Welcome!", "I can save your data.").

### The shop's camera

`ccMenuCtrl::SetMerchantCamera()` (gcmn 0x005269d0, a static member)
turns the view to the character spoken to, `cmndTarget`. It works in 0x70
bytes of the `ccSys` scratch (+0x25c, taken and given back). It fills
camera 3, `ecam` (`cameraList[3]`), through `cameraSetView`,
`cameraSetPos` and `cameraSetRot` (0x00161510, 0x001614d0, 0x00161550),
which copy a whole vector to `cameraList[n]` +0x10, +0x00 and +0x20:

```
changeCamera(3)
if cmndTarget->base->type & 0x8000000:        a Grunt Shop's breeder (npcTbl 10, 16, 22, 28)
    view = breederCamView[game.server - 1]      0x005ed8c0, 16 bytes a server
    pos  = breederCamPos[game.server - 1]       0x005ed880
    r = the scratch at +0x20, as it was
else:
    view = cmndTarget->pos (+0x40); view.z += 100.0
    r = cmndTarget->dirc (+0x60)
    r.y -= 0x3e060a92 (0.1309); r.z -= 0x3faf7641 (1.3708)
    if r.z < -pi (0xc0490fdb): r.z += 2 pi (0x40c90fdb)
    else if not r.z <= pi:     r.z -= 2 pi
    M = sceVu0UnitMatrix, then sceVu0RotMatrixX(r.x), Y(r.y), Z(r.z): Rz Ry Rx
    pos = M (500, 0, 0, 1) + view          sceVu0ApplyMatrix, sceVu0AddVector: all four
                                           lanes, so pos.w = 1 + view.w (2.0)
cameraGetRot(r, 3); cameraSetRot(r, 3)    x the pitch, z the heading of view - pos;
                                           y and w as r had them
```

So for anyone but a breeder the eye stands 500 from a point 100 above the
character's feet, along its heading turned by 1.37 and pitched by 0.13, and
looks at that point. For a breeder, rot's y and w are whatever the last
user of the scratch left at +0x24 and +0x2c. Nothing that reads the active
camera (the table under [Three cameras](#three-cameras)) reads them. Every
Mac Anu merchant takes the general branch, since their types are 0x100 to
0x1000 and the breeders are in the other towns. With camera 3 active,
`cameraMain` and `ccPlayer::Main` keep moving `tcam`, and `cameraSet`
draws through `ecam`'s pos and view as they are. `changeCamera(1)`, when the
shop closes, brings back `tcam`, which has kept following Kite.

The port: `Camera::set_merchant(MerchantView)` (camera.rs; the breeder's
table rows from `camera::breeder_view`), `World::set_merchant_camera(kind,
code, server)` and `World::change_camera(1)`. `crates/piney-game`'s world
mode calls them for the shop pages' `TalkReq::MerchantCamera` and
`TalkReq::Camera(1)`. The port runs the menu after the world's tasks, so
camera 3 is drawn one frame later than in the game, where `ccThMenu` (34)
runs before the camera (40) and the player (49). For a breeder the port
writes 0 to rot's y and w.

## The port

`crates/piney-world`: `World::enter(iso, archive, SaveState)`, then
`step(&Pad) -> Frame` once a frame at frame rate 2, `take_requests()`.

```
ee      EE floats on bit patterns, sceVu0 vector routines, RAD2DEG / DEG2RAD
        (newlib's sinf, cosf, tanf, atan2f, fmodf are piney_data::libm)
hit     the Hit chunk and every query above, HitCheck
player  ccPlayer::Main, ControlMove, PadLeverPower, ccGetCameraTransparency
motion  AnimCtrl's town acts, frameSpd, the arrival fade
camera  tcam, ecam, bcam, camID, cameraInit, cameraMain, cameraPosCalc,
        avoidObstacle, cameraSetEyeLevel, cameraSet, cameraGetRot, the soft reset
evcam   the event camera: the Execute handlers, ccThCameraExecute, CamCtrl,
        CamzCtrl and the paths, MenuBan's camera part
town    what every Root Town class builds; ROOTTOWN01: pieces, passes, clip
        rules, LODs, water (waterUVModifi2 and the copy), crisis sky, lights
        (Town::select is Draw's choice, Town::draw draws it)
town02  ROOTTOWN02, Dun Loireag (cloud: CLOUD; lensflare: LENSFLARE)
gate    ccChgate: the Chaos Gate, its circle, its light
chara   Kite's clump, skin, the blades on his hand nodes, lights, palette
body    a file's CMP_trall clump with models on its nodes, drawn as ccAnm
char    ccChar: a body at a place, its animation, ccChar::Draw's fade
entry   the entry control's Npc trait and what a frame gives it
npc     npcTbl's rows
merchant  ccSetMerchant, ccMerchan, ccEntryObj::routine
rtownpc ccRandomNpc, ccRtownPC: the walking PCs (mt: ccRand; navi: ccNavi)
fellow  a party member an event placed (charTbl, fellowAnimTbl), ccFellow
        under manual control
party   ccSpcManager and ccPartyManager, pc_act / pc_mode / pc_turn /
        pc_face, MenuBan's party part
ai      ccAI under manual control: Brains, ManualControl, the boot status
event   markers, entry and remove, pc_* and npc_* puts, turns and faces
talk    ccThGameCtrl's command target, the action button, TalkRequest
draw    ccModel::Draw into the field's layers through world_screen; unlit
        rigid models clip near the camera (mc_DrawTriSFast), lit and
        skinned ones drop what leaves the view
```

The logic (camera, movement, collision, acts, the view matrix) is the
game's to the bit; the drawing uses f32 matrices.

`crates/piney-game` runs `ccSetupNewGame`'s part in the save at Log in
(the top page's Log in, and `--mode world`, `field:N` and `dungeon:N`):
`InitSpcParam`, `newGameFlag` = 1 and `SetSpcBaseMsg`
(`piney_fieldui::newgame`). In Mac Anu it runs `ccChar::CalcReal(1)` on the
party after the world's tasks each frame, as `ccPlayer::Main` and
`ccFellow::Main` do, so the members' `real` and `tune`, which Status shows,
are in the save.

## Checks

`tools/test_world_rs.py` runs the game's code in eemu (VU0 macro mode from
`tools/test_anim.py`) over the real collision mesh, decoded by the game's
`Decode_Hit` and registered as `STATICMODEL` does, beside the
`world_probe` example:

- `sinf`, `cosf`, `tanf`, `atan2f`, `fmodf` on 7,500 arguments;
  `ControlMove` on 2,000 random states; `ccLandHitCheck` at 300 points of
  `town01` and of `town01d`; `avoidObstacle` on 1,500 target, camera and
  sloping-ground cases (its `ccSetGroundHeight` answered with the same
  plane in EE arithmetic on both sides);
- frame by frame, `cameraMain` then `ccPlayer::Main` from the arrival:
  320 frames walking, running and turning the camera; 640 frames standing
  (the fidget); 420 frames of button presses in each of the four schemes
  (resets, the eye view, zoom and turns with and without pressure);
  480 frames at each of six feet of stairs and slopes (the ground where
  the camera sits at least 150 above his), zoomed out and pitched down,
  the camera turned round and him run up at it, so the line check pulls
  the camera in against the steps; and random pads from random starts in
  both towns. Compared each frame: his
  position, heading, move, speeds, flags, act, act counters, animation
  time, `frameSpd`, cloak, transparency, ground attribute, whether he is
  drawn; the camera's position, target, rotations, distance, angles, type,
  reset state, `world_view` and `world_screen`. 0 mismatches.
- Kite's blades: one of `ctu1body`'s animations compiled and played by the
  game (`tools/test_anim.py`'s `GameAnim`) onto one `ccObj` per node, the
  hands found by `ccClump::GetObjAdrsF` and armed by
  `ccSpcChar::EquipWeapon`, each node drawn by `ccObj::Draw` with
  `ccModel::Draw` catching each blade and the matrix it goes out with,
  beside `Kite::weapons`, frame by frame over the town's acts (and a few
  battle ones) at several frame speeds, at the arrival and at random
  places and headings: which hand gets which model, and each blade's
  matrix within 1e-4 per rotation element and 0.01 in position (the
  game's VU0 products truncate; the port draws with f32 matrices).

The town and the gate (`PropsAgainstGame`), each against `world_probe`'s
`town` and `gate` commands:

- `ROOTTOWN01::Draw` over the pieces the game's own `STATICMODEL` and
  `STATICOBJECT` constructors make from its tables and the town's dummies,
  for 953 camera eyes in `town01` (a grid, the clip and LOD edges, random)
  and 713 in `town01d`, one after another: the clip flags, the water's and
  the crisis sky's offsets, and every piece in order - model rows with their
  matrices, static objects with their animation times, the sky, the crisis
  clumps and their layers, the waters, the map - and each static object's
  root matrix. 0 mismatches.
- `ccChgate::ccChgate`, then 800 frames of `routine` and `main` with Kite
  wandering out past 4500 and back, the camera at him, at the gate and far
  off, the eye view, and the menu's commands: whether it steps and draws,
  its transparency, both animations' times, `gateAnm`'s state, count and
  ended flag, its sounds, its root matrix and the layers it sets. 0
  mismatches.

The walking PCs (`TownPcsAgainstGame`): `ccInitRand` and
`ccRegisterRandomNpc` for 168 (count, reserved) pairs (the rows and the
next `ccRand`); 400 `RouteSearchByMap` / `GetDestination` searches (route,
step, landmark, distance, heading, destination); 700 `ccSpcChar::HitCheck`
calls with 0-5 bodies round Kite, in and out of manual mode; and 3,720
frames over five scenes (the gate plaza, a chat group, the shops, an
event-driven PC, a walker held in place) with 14-16 PCs built by the
game's own constructor: `cameraMain`, Kite's whole `Main`, then each PC's
`routine` and `main`, comparing Kite and what his HitCheck touched, and
every PC's position, heading, act, counters, targets, route, animation,
transparency, body, list membership, chats and transfers, and the shared
state down to the MT's position. 0 mismatches.

The merchants (`MerchantsAgainstGame`): the game's own `ccSetMerchant(0)`
(`entryObject`, `entryNpc`, `initObject`, the `ccMerchan` constructor, both
floor checks on the decoded `town01`), `ccEntryCmnd` and `ccDeleteCmnd`
on the real command lists, Kite's body registered after theirs, then
1,500 frames of `routine` and `main` for all five beside
`merchant::step_all`: Kite touring the booths, past 7000 and 10000 and
into their bodies, the camera behind him, at a merchant and away, the eye
view, `affectFunc` 14 and 0, `breederInfluence` 14, 15, 7 and 0, event
turns and fades. Compared: placement, `posP`, heading, `plDist`,
`plDirc`, the flags, list membership, fade and turn state,
transparencies, the view cone, whether each steps and draws, its
animation and time, its act, body-hit flags and push, its root matrix
and layers. 0 mismatches.

Talking (`TalkAgainstGame`): 600 random scenes of characters (the party,
enemies, merchants, PCs, the gate; random flags, widths and places) built
in the game's memory and put on its lists by `ccEntryCmnd`, then
`ccSortCmnd`, `ccCheckTargetRange` and `ccSelectTarget` in each mode
against `world_probe`'s `target`: the sorted order, distances, headings,
ranges and the target chosen, 0 mismatches; `ccGetDirc` then `RAD2DEG`
(what `pc_face` turns to) on 1,505 pairs of points against `dirc`;
`ccCalcTagPosChar` on 800 points round Kite (in view, off screen and
behind the camera, both modes) through the `world_screen` of 100 frames
of walking and camera turns, against `tagpos`.

What the harness runs in Python in place of the game: the AI and
conditions (inactive for a player alone), effects, the anm's matrices and
draw, and `_AnimateForward` (tools/anim.py's playback, which
`tools/test_anim.py` checks against the game); for the town and the gate
also the stream (`GetChunkAdrsF` serves each name a block, a dummy as its
decoder leaves it), `ccModel`, `ccClump` and `ccAnm` set-up and draws
(recorded, not run) and the sounds (recorded); the lights the anms put in
the draw environment are not compared. `crates/piney-world/tests`
checks the collision mesh, the arrival's 74 frames, running down the
stairs, and a wall; `camera.rs`'s test holds values from the game's own
camera code.

A new game's save (`NewGameAgainstGame`, `tools/test_save_init_rs.py`,
beside the `newgame_probe` example) is checked end to end. The game side
runs, in eemu, the boot (ccSound's and ccSaveData's constructors and
`Init(1)`), then `NewGame(1)`, `parodyFlag` for the Parody run and
`NewGame(0)` with DEMO.PRG in. With GCMN.PRG in it runs `InitSpcParam`,
sets `newGameFlag` to 1 and runs `SetSpcBaseMsg`. The port's side is the
save `--mode world` enters with (`piney_fieldui::newgame::new_game_save`
and, at Log in, `setup_new_game`). All 0x8530 bytes are compared, with and
without Parody: 0 differences.

The shop's camera (`MerchantCameraAgainstGame`, `tools/test_merchcam_rs.py`,
beside the `merchcam_probe` example), over test_evcam_rs's set-up (the
game's own ccThCamera): `SetMerchantCamera` on a `cmndTarget` built in
the game's memory (its type, pos and dirc; `ccGame.server`) and the scratch
cleared, for Mac Anu's five merchants as `ccSetMerchant(0)` places them,
the breeder's table on servers 1-4, and 24 random characters whose
headings take each branch of the wrap. Each case compares camID, tcam and
ecam after the call. It then runs frames of cameraMain and ccPlayer::Main
on camera 3, idle and with the sticks and camera buttons, comparing camID,
tcam, ecam, world_view and world_screen. `cameraGetRot(out, 3)` is checked
with random `out`. Last come `changeCamera(1)` and frames on camera 1.
In all, 33 characters and 487 frames, bit for bit. 0 mismatches.

The event camera (`EventCameraAgainstGame`, `tools/test_evcam_rs.py`,
beside the `evcam_probe` example): the game's own ccThCamera for the
set-up, each instruction through `ccEvent::Execute` at level 2
(ccStartThread and ccDeleteThread noted), `MenuBan`/`MenuClr`, the
characters found by the game's GetSpc, GetNpc, GetEnemy and
ccCheckTargetTypeId, then each frame ccThCameraExecute's first run or
CamCtrl, cameraMain and ccPlayer::Main. Event 2's camera from the arrival
with the script's values and waits, then camera_end and walking (825
frames); camera_end_reset in the four schemes with a reset under way; 40
random runs of instruction sequences over random pads (every instruction,
bans, moving characters); following characters that move each frame;
every spdType and rates 0-200 with camz_set mid-move and paths of 1-15
points in both curves; the tutorial's modes in every scheme. 2,643
instructions, 55,433 frames, 58,293 comparisons of the whole ccEvCamCtrl,
the task, normalCamID, puppetShow, camID, tcam, ecam, bcam, the reset
state, world_view, world_screen and Kite. 0 mismatches. Two things are
held equal by the harness rather than the port: CamCtrl's work area
(`ccSys+0x25c`) is cleared before each call, and tcam.deg[0] after an eye
view frame during a puppet show is masked (the game stores it from an
unwritten stack slot).

The characters' event commands (`tools/test_evchar_rs.py`, beside the
`evchar_probe` example): Kite built as `Field.start` does, then the
game's own `ccSPC::Initialise`, `ccParty::InitParty`, `SetBootStatus`, a
real `ccAISystem` and the `ccAI` constructor with `ChangeMode(1, 1)` and
`ManualMode`; Orca built by the game's `ccFellow02` constructor through
`EntrySpc` and placed at marker 3 with the real `SetDircZ`; every
instruction through `ccEvent::Execute` at level 2; each frame
`cameraMain`, `ccPlayer::Main` and Orca's task, `ccThFellow02`
(`ccFellow::Main`, or its end once exitFlag is set: `expulsionSpc`,
`~ccFellow`, `DelSpc`). The port's side is the town's party as the World
runs it (`TownParty::build`, `with_chars`, `frame`). Compared every frame:
Kite's position, heading, move, speeds, flags, act and counters,
transferLag, cloak, transparencies, animation time, frameSpd, ground
attribute, whether drawn, dispSW, transDist, noDeathFlag, partyFlag and
his list memberships; both AIs' manualSW, talkFlag, remoteFlag,
remoteCmd, gDeg, gRotSp, mode, modeOld, battleFlag, arrivalChatCnt,
skillMask, inviteFlag and detourCnt; the camera and world_screen; the
same for Orca with posP, his hit position, animation and exit; the
registry, the party, the character list's order; and that no AI message
is sent. Event 2's sequence (act 14, `pc_act 0 3`, the arrival, `pc_face`
chg 64 toward Orca, `party_add`, a second `menu_ban`, `pc_turn` chg 0),
random turns (chg 0, 1, 10, 64, 300, -5, headings on both sides of
0x8000), `pc_act` 0-9 on pc 0, -3, 2 and -1 under bootParams 0, 1, 3, 4,
5, 6, 8 and 12, `pc_mode` on every branch with `menu_ban` / `menu_clear`,
`party_add` and `party_remove` under random pads, Kite caught walking or
running when `pc_act 3` lands, and no bootParam (the old arrival): 39
scenarios, 311 instructions, 12,330 frames, 0 mismatches; a coverage set
asserts remote commands 3-7, acts 12, 13 and 14 for both and Orca's exit
were reached. `crates/piney-world/tests/world.rs`'s `event_two_characters`
runs event 2's character flow through the World API.

## Unknown

- Water 0's copy of the frame buffer is the whole frame, not the game's
  128 x 128 ([Dun Loireag](town02.md#the-water-and-the-frame-buffer-copy)).
  The town's fog is VU1's by each vertex's depth (`piney_draw::DepthFog`
  on the town's `TownLights`, worklog 0145) but is not compared with the
  game's pictures. Kite's running steps raise their dust in a town as in
  the fields (`CheckNote`'s `ccEffPawSmoke` at his feet, `Kite::feet`,
  `World::take_paw_smokes`). The town's sounds that are ported: the
  members' lines (`ChatMessageEnteredTown`,
  `piros_says_his_arrival_line_in_mac_anu`), the Administrator's
  `sysopeAct` 217, the Chaos Gate's circle (71 as it opens, 72 as it
  closes, at the gate: `World::take_gate_sounds`,
  `the_gates_circle_sounds`), and Kite's footsteps (his notes 1 and 2
  through `ccPlayer::CheckNote`'s `ccSeSetParamSPC`, by the ground's
  attribute: `World::take_steps`, `kite_has_footsteps_in_mac_anu`).
- The lights' values as the areas' animations leave them (positions,
  colours, fall-off at frame 1) are not checked against the game; the
  group's order and slots are (the light group, above).
- `SetLightMatrix` first walks a second group, `ccDrawEnv` +0x88, when
  there is one. What fills it is not traced, and the port has none.
- Nothing compares the walking PCs' presentation with the game's
  pictures. What they show: their `changeTEX` variants (`MAT_tex` takes
  the row's texture through `Body::tex_swaps` and the draw list's
  `ModelDraw::tex_swaps`, `ccClump::ChangeTex`); their chat bubbles and
  gate transfers (`World::take_pc_events`); their steps, `rtpcCheckNote`
  (gcmn 0x00509160) on notes 1 and 2, `ccSeSetParamPC(param, pc,
  ccsType)` for the sound and `ccEffPawSmoke(pc, +0x2c0)` for the dust on
  every step (`PcEvent::Step`, `the_walking_pcs_have_footsteps`).
  `ccSys+0x358`, which picks the PCs, is an input (`set_rand_count`, 0 by
  default); `rtownnpc.cpp`'s globals start at zero, where the game keeps
  them across visits; a walker's first heading reads a `nextPos` the game
  never sets (taken as zero); event mode -3 (its virus crystal,
  `effVirusCrystal`) is not ported: only volume 2's event 114 gives it.
- Whether the Chaos Gate has a body on the character list. (A party
  member an event places does, from `SetBootStatus` bit 0.)
- The event's NPCs outside the towns: what `ccSetRtownPC`'s second
  argument is when `ccEntryEventMng` calls it (the port passes -1, as for
  the towns); the class steps after the battle's tasks rather than in the
  entry control's NPC turn, and its starts reach the effects a frame
  late.
- `inviteSpc` of an unregistered character in a field or dungeon (the
  town's is ported), and `StartPos` for members already in the party when
  an area starts.
- `ccThSpc` and `ccThAISystem` (priority 48) run in the port before a
  leaver's task ends; in the game the task ends at its own slot (50), after
  them.
- The event camera: ecam.rot's y and w, and cp in cpCtrl 4, come from
  CamCtrl's work area (`ccSys+0x25c`) as the frame's earlier code left it
  (taken as zero; nothing known reads them); cameraSetEyeLevel during a
  puppet show stores tcam.deg[0] from an unwritten stack slot (the port
  leaves it); camz_point 16 on zcam[1] and a 16-point path read at its end
  touch `ccEvent.currentOpen` (not modelled); cam_look_marker_half with no
  marker adds stack garbage (the port leaves the target); types 0-1 and 7
  and up are found on the command lists by base id, which the port's World
  answers only for the party (0-2 by code) and the NPCs; whether a task
  started while ccSleepAllThread holds the others runs; the EE kernel's
  first-in first-out order within a priority is inferred; only a town's
  markers (not a field's `evPos`) are checked.
- The CPU renderer (`piney_desktop::soft`) still drops triangles behind
  the camera instead of cutting them at the near plane.
- `rand()`'s sequence is shared with every caller in the game, so when the
  fidget starts cannot match the game's.
- The ground attribute's type bits when no result qualifies come from a
  stale register (`$a2`); the port takes the nearest contact's x, which
  matches after `CollisionTest`.
- `ccSetChaosGate` leaves its entry's +0x44 unset, so whether `entryObject`
  sets the gate on the ground depends on the stack; the ground under
  `DMY_gate` is at 600, its own z, either way.
- The anm lights' colour and float controllers are evaluated between keys
  in f32 here; the game's `ccAnmCtrlColor_Set` steps through its segments
  (`ccSetBlendColor`) and has not been run against this.
