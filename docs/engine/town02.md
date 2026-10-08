---
title: Dun Loireag - ROOTTOWN02, its clouds, lens flare and water
status: partial
volumes: INF
covers: INF gcmn.prg:0x004240c0 ROOTTOWN02::ROOTTOWN02, 0x004266a0 ROOTTOWN02::Draw, 0x00425130 ROOTTOWN02::DrawBG, 0x00425500 DrawObj, 0x004255e0 DrawObj2, 0x004256c0 DrawFloor, 0x00375ff0 ROOTTOWN02 vtable, 0x005d4bb0 RT_MODELTABLE02, 0x005d4d40 RT_OBJTABLE02, 0x005d4e20 town02Light, 0x00504400 CLOUD::SetPos, 0x00504860 CLOUD::Init, 0x00504a10 CLOUD::Move, 0x00504ce0 CLOUD::Draw, 0x00503090 LENSFLARE::LENSFLARE, 0x00503360 LENSFLARE::Draw, 0x005ed7e0 @973, 0x005025d0 waterUVModifi2, 0x00509f30 ccSetDog, 0x0050f0b0 ccSetChibiGuso, 0x00509390 ccDog::ccDog, 0x00509680 ccDog::main, 0x00509a80 ccDog::move, 0x0050a1f0 dogAction, 0x0050a260 inuCheckNote, 0x0050a2e0 ccDog::effect, 0x005ee220 markPosTbl, 0x005ee180 inuAnmTbl, 0x0054a680 ccMenuCtrl::NorainuMenu, 0x0050ef10 setDog, 0x0050ac00 ccPGuso::ccPGuso, 0x0050b3c0 ccPGuso::main, 0x0050bba0 ccPGuso::adultMain, 0x0050bca0 ccPGuso::move, 0x0050c0f0 ccPGuso::paramCalc, 0x0050c8f0 ccPGuso::evoAct, 0x0050c960 evoActChibi2, 0x0050cbb0 evoActPon1, 0x0050d5a0 evoActPon2, 0x0050dd30 evoActAdult, 0x0050e820 runAway, 0x0050e990 pgDead, 0x0050eac0 adultSetup, 0x0050eda0 changeAnmPG, 0x0050f390 dogAction, 0x0050f720 dogAction2, 0x0050f9d0 dogActionAdult, 0x0050fec0 inuCheckNote (the Grunties'), 0x0050ff30 burpEff, 0x005100e0 evoEff, 0x005102a0 ccPGuso::effect, 0x00510490 moveCam, 0x005ee300 pgAnmPtr, 0x005ee310 pgAnmPtrAdult, 0x005ee340 ccsTblPG, 0x005ee3c0 markPosTbl (pgbreed), 0x005ee3e0 adultGusoParam, 0x005ee440 pgSizeParam, 0x005ee450 foodTbl, 0x005ee510 needFoodTbl1, 0x005ee530 needFoodTbl2, 0x005ee550 camPosDef, 0x005ee590 camViewDef, 0x005ee5d0 inuPos, 0x005ee610 playerPos, 0x005ee650 pgChatTbl; INF SLUS_202.67:0x001d0f00 effEvolvePG, 0x001d0fd0 effGrowPG, 0x00108e30 ccLayer::MakePacketDrawBuffTrans, 0x0013f3b0 ccObj::CheckBoundingBox, 0x00129d08 fptoui, 0x0019c460 fieldrand, 0x0019ccb0 WORLD_MAN::Init, 0x001a1190 WORLD_MAN::SetCharPosition, 0x001b62e0 ccEntryEventMng, 0x00160cb0 checkCameraType, 0x00161700 cameraGetRot2, 0x001615d0 cameraGetView
worklog: 96, 142, 162, 395
---

# Dun Loireag - ROOTTOWN02, its clouds, lens flare and water

Dun Loireag is the second Root Town, `game.town` 1 on the server Theta
(`ChangeScene`'s server table gives 1). The story reaches it through event
20 ("ML0111"): its `town_move 1` adds the town to `townMoveFlag`
(`saveData+0x2234`), which the Chaos Gate's Other Servers page lists, and
its block 17's `scene 0 1` sends the party there; events 22-30 have blocks
for it (`in_town 1`). `WORLD_MAN::GO(0)` builds `ROOTTOWN02` for town 1
and `ccThFieldDisp` calls its `Draw` each frame, as it does
`ROOTTOWN01`'s in Mac Anu ([Town and event-area assembly](statics.md),
[the field game](field-game.md#what-the-town-draws)). The port is
`crates/piney-world/src/town02.rs` with `cloud.rs` and `lensflare.rs`.

## The constructor

`ROOTTOWN02::ROOTTOWN02` (gcmn 0x004240c0):

```text
+0x00 1, +0x04 0               +0x04: the clouds are made by the first Draw
mapLayer                       ccLayer::Init(50, 0) at WORLD_MAN +0x490,
                               SetFrame(340, 36, 160, 160, 80, 80, 1, 1)
+0x7c..+0x88                   the map's mapw 140, maph 256, stepw 1.28,
                               steph 1.2921
+0x1a4 = GetCCSAdrs            town02d when saveData+0x6772 (crisis) is set,
                               else town02
cc3d->SetFog(1500, 10000, 0, 75, 0x00f0c080)
ccSys.bgColor (+0x18) = 0x00f0c080, with the GS values at +0xdd0, +0xce0
+0xa0  20 STATICMODELs         RT_MODELTABLE02 (0x005d4bb0)
+0x168 9 STATICOBJECTs         RT_OBJTABLE02 (0x005d4d40)
+0x1a8 0                       (Mac Anu's sky clump slot)
+0x244 CMP_sr2bac1             the sky            each clump new'd, ccCoord,
+0x248 CMP_sr2clo_1_1          a cloud layer      Init; SetFogSw(0)
+0x24c CMP_sr2clo_1_2          a cloud layer
+0x250 CMP_sr2sun1             the sun
crisis: +0x70..+0x78           CMP_sr2dat1_1 .. _3, SetFogSw(0)
+0x10, +0x14, +0x18            the map's masks (map.md)
+0x9c  a ccAnm, SetLightEnv(1), SetAnm(ANM_sr2bac1a), one _AnimateForward;
       GetAmbient -> SetAmbient; for town02Light's six rows (0 distant:
       AddGrp and WORLD_MAN::SetLightDirection; 1 omni: AddGrp)
       LGT_sr2lig1, LGT_sr2omn01 .. 05, kept at +0x20.. (count +0x1c)
+0x8c..+0x98                   BLT_bg, BLT_obj, BLT_obj2, BLT_floor
+0x240 = GetCCSAdrs(town_z)
+0x23c new LENSFLARE(town_z)
+0x1d8 25 new CLOUD            (the constructor is empty)
+0x1b0..+0x1b8 three ccAnms of ANM_sr2wat1a (town02's own), each
       SetFogSw(0), OBJ_sr2wat00 Duplicate(8200, then 8192, 8192),
       SetMatrix_PosRotZYX(@988 (0, 4200, 0), @989 (0, 0, 0)), one
       _AnimateForward
+0x1bc a ccTexChunkDesc (128 x 128, 32 bits); +0x1d4 its ccTexChunk,
       ccModel::ChangeTex into water 0's OBJ_sr2wat00 model
```

`RT_MODELTABLE02`'s 20 rows are the five districts' `MDL_floor_NN`,
`MDL_obj_NN` and `MDL_obj2_NN` at `DMY_floor_NNpos` and the five shops'
`MDL_sr2wep1` ... `MDL_sr2sav1` (type 2) at their own dummies; no row is
of type 0. `RT_OBJTABLE02`'s nine: the windmills `ANM_sr2win1a` and
`ANM_sr2win2a` at `DMY_sr2win1_1pos` ... `DMY_sr2win2_4pos` (type 2), and
the balloons `ANM_sr2bal1a` at the origin (type 3; the row's clump
`CMP_sr2bal1` is not in the file). Every clip is 100000. The water's model
`MDL_sr2wat00` is one rigid mmat of 330 vertices, vertex scale 900: a
pool 1800 across under the Chaos Gate.

## Draw

`ROOTTOWN02::Draw` (0x004266a0), the vtable at 0x00375ff0 (+0x08 Draw,
+0x0c DrawBG, +0x10 DrawObj, +0x14 DrawObj2, +0x18 DrawFloor, +0x1c
DrawMap):

```text
+0x04 0: +0x04 = 1; CLOUD::Init(town_z, 0) for the 25
eye = cameraGetPos(camID)
SetActiveLayer(1)
water 1: SetUV(vftoi12(u), 0, NULL, 2)          u = u$1777 (0 first time)
water 2: SetUV(0, vftoi12(u), NULL, 1)
SetActiveLayer(3)
waterUVModifi2(water 0's OBJ_sr2wat00, eye, 32000)
water 2, water 1: _AnimateForward, Draw           on effLayer
SetActiveLayer(1)
water 0: _AnimateForward, Draw                   on objLayer
ccLayer::MakePacketDrawBuffTrans(objLayer, +0x1d4)
u += 0.005; u = 0 unless u < 1
DrawBG
SetActiveLayer(1); DrawObj2; DrawObj; DrawFloor; DrawMap; the type-0 rows
SetActiveLayer(3)
LENSFLARE::Draw(town02, "DMY_sr2lig_1point", 0)
+0x04: each CLOUD's Move, then Draw
SetActiveLayer(0)
```

`DrawBG` (0x00425130):

```text
v$1346 += (0.01, 0.02); v2$1347 += 0.001; each back to 0 past 1
MAT_sr2clo_1 V = vftoi12(v$1346[0]) - its crop V; MAT_sr2clo_2 V likewise [1]
bgLayer[0] (+0x494, -100): CMP_sr2bac1 at the origin
bgLayer[1] (+0x498, -90):  CMP_sr2sun1 at DMY_sr2lig_1point (-13600, -16000, 1200)
SetActiveLayer(3):         CMP_sr2clo_1_1, CMP_sr2clo_1_2 at the origin
crisis: MAT_sr2dat1_2 U, MAT_sr2dat1_3 U and V = vftoi12(v2) - crop;
        CMP_sr2dat1_1 .. _3 on bgLayer[2] .. [4] (-80, -70, -60)
```

`DrawObj2`, `DrawObj` and `DrawFloor` (0x004255e0, 0x00425500, 0x004256c0)
walk the 20 rows for type 3, 2 and 1 (the first two then the nine static
objects of their type), between `LockBlt` and `MakePacketLoadData` of their
`BLT_` chunk, with none of Mac Anu's clip rules. `SetActiveLayer(3)` is the
effect layer (+0x4c8, priority 20): the two cloud layers, waters 1 and 2,
the flares and the clouds draw over the characters' layer (10).

The scrolls are statics: `u$1777` and `v2$1347` start at 0 on their first
use after power-on, `v$1346` in the data (0), and they carry on across
visits.

## The water and the frame-buffer copy

Water 0 draws with the picture behind it, both towns alike (Mac Anu's
`Draw` calls the same pair, on objLayer, with a reach of 9000 and its
`wat1` water rooted at the identity).

`waterUVModifi2(obj, eye, reach)` (gcmn 0x005025d0):

```text
ccObj::CheckBoundingBox(obj) 0: return 0   (a model without a Bbox, as
                                            both towns' waters, is in)
d = sqrt((eye - obj+0x70).x^2 + (..).y^2) in double   (+0x70: the
                                            coordinate's local translation)
d > reach: return 0
scale = model+0xc / 4096; the first mmat's n vertices, s16 positions:
  p = (x, y, z) scale, w 1; q = obj's world matrix p
  (sx, sy) = sceVu0RotTransPers(sysLayer's view +0xd0, q), each / 16
             toward zero
  u = (sx - 1792) / 512; v = (sy - 1824) / 448
  the vertex's ST = (fptoui(256 u), fptoui(256 v)), written into the model
return 1
```

`fptoui` is libgcc's soft-float one (main 0x00129d08): truncation, 0 for a
negative value, all ones from 2^32. The written coordinates stay in the
model until the next call within reach.

`ccLayer::MakePacketDrawBuffTrans(layer, tex)` (main 0x00108e30) queues a
GS packet at the front of the layer's list, as a model's draw does: context
2 with FRAME_2 the texture's page, SCISSOR_2 its 128 x 128, TEX0_2 the draw
buffer and TEX1_2 0 (nearest), one sprite from UV (the layer view's scissor)
to XY (0, 0)-(128, 128). Sent right after water 0 on objLayer, it runs
after every opaque model sent later (the whole town's) and just before
water 0: water 0 samples the town as drawn this frame, at its unbent
vertices' screen places, and its morph moves the picture - a refraction.

## CLOUD

`CLOUD` (gcmn 0x00504380-0x00504d94, 0x40 bytes): +0x00 angle (degrees),
+0x04 its step, +0x08 type, +0x0c the pattern, +0x10 speed, +0x20 pos,
+0x30 its `ccEff`. `Init(s, type)` makes the `ccEff` of `EFF_srzsmo1` (type
2: `EFF_sfsmo1`) with `Init(chunk, 1)`, `SetRenderState(ZENABLE, 0)` and
PRIM's fog bit off, sets the step 0.25 (type 2: 5) and calls `SetPos`.
Every random number is the area generator's `fieldrand` (main 0x0019c460:
`seed = (seed 109 + 1021) mod 0xfffffffe`, `seed % n`):

```text
SetPos   scale 2 x 2; by type:
  0  pos = the player's pos + (fieldrand(5000) - 2500, fieldrand(5000) -
         2500, fieldrand(400) - 200); transparency 0.8; speed
         fieldrand(15) + 5
  1  (fieldrand(25000) - 12500, likewise, -800 - fieldrand(300)); speed
         fieldrand(100) + 100
  2  scale 0.5; (fieldrand(15000) - 7500, likewise, 0); speed
         fieldrand(20) + 60
  3  transparency 0.8; x, y = the player's +- (fieldrand(1000) + 500) (+
         when fieldrand(100) > 50); z likewise type 0; speed fieldrand(15)
         + 20
  angle = fieldrand(360); rotate = (short)(angle 182.04445) pi / 32768;
  pattern = fieldrand(patNum)
Move     pattern + 1, 0 at patNum; angle += step, 0 at 360; rotate
  0  x += speed; SetPos when farther than 5000 from the player (W2P, on
         the ground, sqrt in double)
  1  y -= speed; SetPos below -25000
  2  x, y -= speed; W2P then P2W; z = WORLD_MAN::GetHeight(x, y), at
         least 0
  3  x += speed; SetPos beyond 3000
Draw     ccCheckCameraDeg(pos, 12288) and nearer than 7000 on the ground:
         ccEff::Draw(pos, pattern)
```

`EFF_srzsmo1` (in `town_z`) has 101 patterns, the transparency of each
rising and falling over the cycle; its texture is a square of mist of
alpha about 0.35 with a narrow clear border. Dun Loireag's 25 are type 0,
made about the player where he stands at the first `Draw` (the arrival's
start). `EVENTAREA04`-`07`, `EVENTAREAB8` and `ROOTTOWN04` use the class
too. `fieldrand`'s `seed` is a global: `WORLD_MAN::Init` (main 0x0019ccb0)
seeds it with the frame count and the area words it picks, and every
field and dungeon since reseeds it; the executable holds 13.

## LENSFLARE

`LENSFLARE::LENSFLARE(s)` (gcmn 0x00503090) makes six `ccEff`s of
`EFF_sflenz_2` .. `_6` and `_1`, each `Init(chunk, 1)`,
`SetRenderState(ZENABLE, 0)` and PRIM's fog bit off. `Draw(s, name, mode)`
(0x00503360):

```text
eventMng.puppetShow (+0x78c) 1: nothing
sun = GetChunkAdrsF(s, name) +0x10
mode 0: nothing unless ccCheckCameraDeg(sun, 12288)
for k in 0..6:
  ahead = RotMatrix(cameraGetRot(1), or cameraGetRot2(1) when
          checkCameraType() is 1) (0, -2500, 0, view.w) + eye; ahead.z += 500
  v = (sun - ahead) (-0.2, -0.2, 0) + ahead; v.z = -200
  ccEff::Draw(flare k, (v - sun) @973[k] + sun, 0)
                         @973 = 0.2, 0.3, 0.5, 0.55, 0.75, 0.8
mode 1 (a field's WORLD) takes the sun through P2W and each flare through
W2P and back
```

`EVENTAREA02::DrawLensFlare` (area 15, [its page](evarea.md)) draws the
same flares without the puppet-show and camera-degree tests. `ROOTTOWN04`,
`EVENTAREA04`, `05`, `07` and `WORLD::Init` make a `LENSFLARE` too.

## The town's game

What `ccSetupGameCtrl` and the entry control do by `game.town` 1:

- **The start.** `WORLD_MAN::SetCharPosition` (main 0x001a1190) puts the
  player at (0, 3500, 0) facing south (heading 0), 700 south of the gate;
  the party's others at +-200 in x, 100 north. Log in, the gate's warp and
  the way back from a field all start there.
- **The Chaos Gate.** `ccSetChaosGate` at `DMY_gate`: (0, 4200, 0), over
  the pool.
- **The collision.** `HIT_sr2town1hit`, town02's one Hit chunk (the same
  in `town02d`): 1,356 triangles, 354 floors and 1,002 walls. The start
  stands on the gate plaza at 0.
- **The merchants.** `ccSetMerchant(0)` places `npcTbl` rows 5-10 at the
  dummies `setMerchant`'s table names: Weapon Shop (`DMY_merchant1`,
  (-1550, 300)), Elf's Haven (`4`), Item Shop (`3`), Magic Shop (`2`),
  Recorder (`5`) and Grunt Shop (`6`, (-2200, -5800)), their `anmTbl`
  `merchanAnmPtr[1]` (`ANM_ctr2nut0`, `act0`, `act2`); the Grunt Shop is a
  breeder (`breederInfluence`, the breeder camera of server 1).
- **The dogs and the chibi Grunties.** In town 1 `ccEntryEventMng` calls
  `ccSetDog` (gcmn 0x00509f30: four dogs, `npcTbl` 141-144, `ccDog`,
  [below](#the-dogs)) and `ccSetChibiGuso` (0x0050f0b0: the chibi
  Grunties the save's Grunty records for the town hold, `npcTbl`
  145-157, [below](#the-grunties)) before `ccEntryRandomNpc`; towns 2 and
  3 call `ccSetChibiGuso` alone.
- **The walking PCs.** `ccRegisterRandomNpc` does not look at the town:
  the same 15 of rows 30-79 for Kite alone. Each slot's landmarks are
  `markPosTbl[k][1]`, the routes `naviMapTown2` over town02's
  `DMY_markerNN`. Act 7, the gate, is for town 0 only (`ccRtownPC::move`
  tests `game.town`).
- **The event markers.** `markerEvTbl` names dummies of the town's file:
  town02 has `DMY_gate`, `DMY_marker_ev01`-`06`, `20` and `21`,
  `DMY_marker71` and `DMY_marker30` (markers 0-6, 20, 21, 31, 32).
- **The sound.** `ccSndSQLoad(2)` loads `sqDataTown` row 1 (6 in crisis).
- **The picture.** `ccSys.bgColor` 0x00f0c080 clears the frame to a pale
  blue, the fog's colour.
- **The map.** `ROOTTOWN02::DrawMap` ([the minimap](map.md)).

## The dogs

`ccSetDog` (gcmn 0x00509f30) runs in area 0 and town 1 only. It sets
`ccDog::inuNum` to 0 and makes four entries (type 2, `npcTbl` rows
141-144: Johnny, Sal, Lottel and Suzie, base type 0x04000000,
`CDOGBOD1.CCS`, 180 high and 50 wide, `entRoot` -1). Each stands at the
first dummy of its route, `markPosTbl[inuNum]` (gcmn 0x005ee220). The
constructor counts `inuNum` up, so the four take routes 0-3.

| dog | route (`markerTbl1`-`4`, town02's `DMY_markerNN`) |
| --- | --- |
| 141 | 46, 45, 44, 09, 46, 67, 10, 49, 67 |
| 142 | 03, 06, 48, 47, 08, 07, 05, 04 |
| 143 | 18, 19, 20 |
| 144 | 26, 27, 28, 29, 28, 27 |

`ccDog::ccDog` (0x00509390) is a `ccGimmick`. It sets the file's
`CMP_trall` with `inuAnmTbl[0]` (`ANM_cdg1nut0`; the others are `nut1`,
`run0` and `wal0`), `inuCheckNote` as the note callback and `dogAction`
as `affectFunc`. Its `bodyHit` has radius 65, height 120, kind 2 and
`mask2` 0x40000001, and is put on the character list (`HitEnable`). It
starts at act 3, walking to the route's second dummy, turned toward it.

```
ccDog::main    posP; by act (+0x204; +0x210 its step):
                 3  anm run0 once, then move (runs, 12 a frame)
                 4  anm wal0 once, then move (walks, 1.5 a frame)
                 1  anm nut1 once, turning to face Kite (ccSetDirc 64)
                 2  anm nut0 once, the same
                 5  anm nut1, 101 frames, then act +0x20c (3 or 4)
               mask2 0x40000001 nearer Kite than 300, else 1;
               CollisionDetection: pushed out (pos and posP), the touch
               flag and count (+0x218, +0x21a), else both 0; the anm
               forward, NoteProcess, ccChar::Draw
ccDog::move    the heading turned a sixteenth of the way to the target
               (ccGetDircChg 256); within 4096 of it:
                 at the target (nearer than 80) or stuck (+0x21c past
                 120): at the target, rand() & 1 picks walk (4) or run
                 (3), through act 5 when it changes, and the next dummy
                 (the list wraps); stuck, back to the last one; +0x21c 0
                 else a step toward the target, turned -0.5 in a touch's
                 first 40 frames and +0.5 in 60-99 (the count wraps at
                 120)
               the ground (0x20000002) and its attribute; each 120th frame,
               not 50 from where it was in x, y and z: stuck
dogAction      affectType 0: act 4; 14: act 1; 15: act 2 (step 0)
inuCheckNote   a note 1 or 2 of param p: p != 0 ccSeSetParamInu(p) (1 a
               step, 3 and 5 barks), p 1 also ccDog::effect
ccDog::effect  running and more than 0.05 seen: effSmoke(pos, Rz(dirc)
               (0, 0.1 (0.075 (-10) (10 - rand() % 5)), 0, 1), 3.0, 10, 1,
               512, 32)
```

The action button opens `NorainuMenu` (44) on a dog: it sits and faces
Kite, Talk makes it sit up, and it walks on when the menu shuts
([field-ui.md](field-ui.md#dun-loireags-dogs-norainumenu-44)).
`ccRegisterInu` (0x0050a5c0, from `ccRegisterRegularNpc`) and
`ccAddRequestFileListInu` (0x0050a460) register the rows and files for
loading, which the port does as it places them.

`piney_world::dog` is `ccSetDog` (`set_dogs`), the class (`Dog`) and its
sounds and dust (`DogEvent`); `World` keeps the dogs after the merchants,
as the entry control's list does, and `piney-game` plays their notes
(`se3d::inu_note`) and dust (`effSmoke`).

## The Grunties

`ccSetChibiGuso` (gcmn 0x0050f0b0) runs in area 0. The town's record is
`saveData.growth[game.town]` (+0x2194, a `GROWTH_PARAM` of 0x18 bytes:
level, size, smell, crooked, cruel, iq, pure, `type[3]`, and at +0x14 the
food line it asks for). Each set `type[k]` places a grown Grunty through
`setDog(row)` (0x0050ef10): kind 0 is row 145, kind 1 row 144 + 2 town
and kind 2 row 145 + 2 town, at `DMY_cdog0`, `cdog1` and `cdog2` with
the dummy's rotation. A record at level 4 with a kind free starts over
(level to pure zeroed and saved); then a level 0-3 places the young one,
row 154 + level (Little Grunty 154-155, Grunty the Kid 156-157), at its
route's first dummy. The grown ones are made before the record starts
over, the young one after.

| town | young route (`markPosTbl`, gcmn 0x005ee3c0) | sizes that start levels 1, 2, 3, grown (`pgSizeParam`) |
| --- | --- | --- |
| 1 Dun Loireag | `DMY_marker34`, `35`, `36` | 5, 10, 20, 30 |
| 2 | `DMY_marker27`, `26`, `30` | 7, 15, 25, 40 |
| 3 | `DMY_marker118`, `119`, `120` | 7, 15, 25, 40 |

`ccPGuso::ccPGuso` (0x0050ac00, pgbreed.cpp, 0x330 bytes over
`ccGimmick`) loads the entry's stream (`cdogboda`; `cdogbodb`, the next
body, as `anmB` too; a grown kind's `ccsTblPG[kind]`, `cdogbod0` and
`cdogbod2`-`9`, as `anmC` when it grows), sets `inuCheckNote` as the
note callback, the body (radius 65, height 60, kind 2, 16 from Mutation
on, on the character list; its `mask2` stays `ccCharHit`'s -1, which no
wall's attribute holds whole, so walls never push it) and copies the
record. From Mutation on the race moves a grown one
([flag-race.md](flag-race.md)). The young one walks its route (act 3) at
`pgScale` 1, 1.5, 1 or 1.2 by level; a grown one stands (act 9).
`pgPtr` is the last Grunty made: `inuCheckNote` acts on it, whichever
Grunty's anm passed the note.

```
main          posP; by act:
                0  sits facing Kite (menu open), 1 rests 151 frames, 2 sits
                   up (a line), 3 walks the route (move: to 80 of each
                   dummy, 1.5 a frame, the ground and its attribute, stuck
                   after 120 frames not 50 from where it was), 4, 5 wait
                6  eats (the menu's affect 19): the eat anm, burpEff's
                   smoke, then paramCalc; growing (evolevel) evoAct
                7  evoActAdult; 9 adultMain (a placed grown one)
              pushed out of others (CollisionDetection); then by dmylevel
              the anm stepped, its notes, drawn: 0-1 the main anm
              (ccChar::Draw, scaled), 2-3 anmB (ccAnm::Draw), 4 anmC; a
              young one whose foodMode and chatFlag are set says
              pgChatTbl's line every 211 frames (OpenChat)
paramCalc     foodTbl[food] x N into size .. pure; the level the size has
              reached (evoAct 1-3), or grown: all five stats within 7 of
              adultGusoParam[town]: adultType 1, any beyond 10: 0, else
              2, then adultSetup and act 7; the food it asks for next
              (needFoodTbl1/2 by the stat furthest off, ccRand() & 3):
              line 3 + 2 (food - 1), 33 when all are within 7; saved
evoAct        1 evoActChibi2 (level 0 to 1, row 155, voice -2, 37), 2
              evoActPon1 and 3 evoActPon2 (the body changes: cross-fade
              of the main anm and anmB, voice -3, 37): the stretch (y up
              0.01, z by 0.04 then down 0.02), effEvolvePG at the anm's
              frame 30, the new scale, saved
adultSetup    kind 0, town 2 - 1 or town 2 by adultType; its base; exist
              when the record has the kind already, else the kind set;
              its body and anmC; inuCount[kind] (+0x7474) up
evoActAdult   growthNum 2 at 30, effGrowPG at 20, the change at 121 (its
              voice), stretched until effEvolvePG at 50; level 4, its line
              (exist 5, key item 49 held @1899's, else 1), growthNum 3;
              once the menu is done (4): exist - it looks back, runs off
              (runAway: 12 a frame to a far place, bobbing) and flattens
              (pgDead's puff), growthNum 5 - or it walks the young one's
              route with dogActionAdult
moveCam       the fixed camera (camPosDef, camViewDef by town) eased
              toward it while it eats and grows
dogAction     (young) 14 sits (menu open), 15 sits up with its line (7
              grown, 2 never fed, else its food line), 0 walks on, 11 the
              fixed camera - changeCamera(3), the Grunty at inuPos and
              Kite at playerPos facing it - or back (changeCamera(1));
              19 eats food a1, a2 of it (act 6, growthNum 1)
dogAction2    (placed grown) 15 a line of its row's four (ccRand() & 3),
              14 line 7
dogActionAdult (grown in front of Kite) 15 sits up with a line of its
              kind's four, 14 sits (line 7), 0 walks on, 11 as dogAction's
inuCheckNote  a note 1 or 2: param 0 or 1 ccPGuso::effect on pgPtr (a
              grown one running off: effSmoke dust and sound 53), larger
              ccSeSetParamInu(param, pgPtr)
```

The action button opens `InuMenu` (46) on a young one and `OtonainuMenu`
(45) on a grown one; Give Food (`BreedingMenu`, 56) feeds a young one
([field-ui.md](field-ui.md#the-grunties-otonainumenu-45-inumenu-46)).
The foods are key items 26-41; the dungeons' `ccGimFood` gives them
(`FoodMenu`, 43, [field-ui.md](field-ui.md#the-field-objects-32---39-43-67)),
and `BreedingMenu` gives the Grunty Flute (key item 49) when a new grown
kind appears: blown in a field of this server it calls the grown one to
ride ([grunty-ride.md](grunty-ride.md)).

`piney_world::grunty` is `ccSetChibiGuso` (`set_chibi_guso`), `setDog`
(`set_dog`) and the class (`Grunty`: `main`, the affect functions,
`inu_check_note`), with its requests as `GruntyEvent`s. `World` keeps the
Grunties after the dogs and before the walking PCs, steps each (then
`inuCheckNote` on the last one made), carries out their camera and Kite's
place after the list, and draws them through `ccChar::Draw`'s shadow.
`piney-game` plays their notes (`se3d::inu_note`), sounds, smoke,
`effEvolvePG` and `effGrowPG` (piney-effect's `pg`), voices (a grown
kind's through `ccCheckVoiceGrp`) and lines, and hands the menus the
Grunty they read.

## The port

`piney_world::town::Town` is a town's `Base` (what every class's
constructor builds from its tables: the file, the static models and
objects, the lights, the fog, the hits) and its class, a type of its own
per `ROOTTOWN` (`town01::MacAnu`, `town02::DunLoireag`, ...) behind the
`RootTown` trait, the class's virtual `Draw`. `Town::open` builds the class
by town; each class's `select` is its `Draw` up to its draws, as its own
`Piece`s (with a `TownView`: the eye, the player, the active camera, the
flare's camera, `puppetShow`, `world_screen`), and `RootTown::draw`
draws them and hands back the `ccEff`s (`TownSprite`) that `piney-game`'s
`town_fx` draws through piney-effect's `ccEff::Draw`. `town::water_st` is
`waterUVModifi2`, `WaterZero` the texture coordinates it leaves; water 0
draws with them and the frame buffer as its texture (`VertexEdits::st`
and `tex`, which `piney-gs` copies when the model's command starts, at the
same place in the draw order as the game's copy). `cloud::Cloud` is
`CLOUD` (all four types), `lensflare::points` and `draw` `LENSFLARE::Draw`.
`World::enter` builds the town of the save's `lastTown` (0 or 1) and
starts Kite at `start_position`; the session changes scene to either town.

The port's copy is the whole 512 x 448 frame sampled at the same
fractions, not the game's 128 x 128 point-sampled one, so the refraction
is sharper than the game's. The clouds' `fieldrand` starts from 13 with
each town (`DunLoireag::rng`); the scrolls start at 0.

## Checks

`tools/test_town02_rs.py` runs the game's own code in eemu beside
`town02_probe`:

- the constructor, `ROOTTOWN02::ROOTTOWN02` itself over town02 and town_z
  (and town02d): `SetFog`'s arguments and `ccSys.bgColor`, the files, the
  20 `STATICMODEL`s' rows, types, models and positions, the nine
  `STATICOBJECT`s' roots and times, the light group's order and the
  distant light, the water's times, the six flares' `Init` and render
  states;
- `ROOTTOWN02::Draw` for 700 frames (and 200 in crisis from another seed),
  the player wandering the town and beyond, the camera every way and at
  the sun, the eye view and puppet shows: every piece in order with its
  layer (the waters, the copy, the sky, the sun at its dummy, the cloud
  layers, the crisis clumps, 20 rows, the objects with their times, the
  map, the flares at their places, each cloud drawn with its place and
  pattern) and after each frame `SetUV`'s value, the cloud layers' and
  crisis materials' offsets, all 25 clouds (angle, pattern, speed,
  position, turn, transparency), `fieldrand`'s seed and the three scrolls:
  none differ (5,940 cloud draws, 2,052 flares, clouds sent back in 7
  frames);
- `waterUVModifi2` on Mac Anu's water (4,714 vertices, reach 9000) and
  Dun Loireag's (330, 32000) for 300 camera places and views each: every
  texture coordinate, and the calls out of reach.

`tools/test_map_rs.py` checks `ROOTTOWN02::DrawMap`. `piney-world`'s
`tests/dun_loireag.rs` enters the town, checks the start, the gate, the
merchants, the walking PCs, the collision's counts and the markers, and
runs; `piney-game`'s `session::dun_loireag` warps from Mac Anu to Dun
Loireag and back through the gate's Other Servers as a player picks them,
and plays `--mode story:22` from the board through Log in and the gate
until event 22's Dun Loireag blocks run. `dun_loireag_shots` (ignored)
takes pictures. `the_dogs_walk_their_routes_and_sit_to_talk` lets the
four dogs run for 20 seconds, each reaching its next dummy, then puts
Kite before one and presses the action button: `NorainuMenu` opens and
the dog sits, Talk and back, and cancel sends it walking on.

`tools/test_grunty_rs.py` runs the game's `ccSetChibiGuso` on records of
every kind (the rows, the record left) and `ccPGuso`'s constructor and
`main` in eemu over town02 beside `grunty_probe`, frame by frame from a
random walk of Kite and the camera with the menus' affects at their
frames: walking the route, talking, eating, each growing up (level 0 to
1, 1 to 2, 2 to 3, 1 to 3), a young one growing into a new kind, a kind
the town has (running off), one near a kind, the chat lines, grown ones
of rows 145-147. Compared each frame: every member (acts, counts,
position, heading, scale, the record, the camera's), the anms' clips and
frames, the notes, the draws (which anm, transparency, scaled), and the
events in order (sounds and their notes at `pgPtr`, smoke, `effEvolvePG`
and `effGrowPG` with the character's place and height, voices, chat
lines, the camera, Kite's place). `runAway`'s wall test reads the
collision's last result, which the harness does not model; that one
frame's `checkHitResultAttlibute` is skipped (act 7, evonum 6 and on).
The menus are `tools/test_fieldui_talk_rs.py`'s `GruntyPages`.

`the_grunty_eats_and_grows` (session): a new game in Dun Loireag with
three of the first food. The young Grunty walks its route; Kite before it
and the action button open `InuMenu`, the fixed camera, it sits; Give
Food, the food three times, OK: it eats, grows to Little Grunty (row
155, level 1, size 6, its voice), the food taken; Talk, and back out:
the field camera and its walk. `the_grown_grunty_talks` places a grown
one (row 145) at `DMY_cdog0` from a record with kind 0 (the record starts
over and a young one comes too) and talks to it through `OtonainuMenu`.
`grunty_shots` (ignored) takes the walk, the menu, the change and the
grown Little Grunty (`scratch/grunties/shots`).

## Unknown

- The Grunties' camera writes (`cameraSetPos`, `cameraSetView`,
  `changeCamera`) and Kite's place land in the port after the entry
  control's list, a frame's NPCs later than the game's; the NPCs after
  them see the camera as it was.
- `effEvolvePG`'s and `effGrowPG`'s particles are not compared with the
  game's (only their calls are).
- The dogs' walks are not compared with the game's frame by frame. The
  game's `+0x21c` (the stuck count) is not set by the constructor; the
  port starts it at 0.
- `EFF_srzsmo1`'s alpha test: `SetBlendType(0)` gives the clouds ATST
  GEQUAL with `Draw`'s reference and AFAIL FB_ONLY; most of the texture is
  below the reference, so it is drawn by colour alone. The port draws the
  same; what the picture looks like on the console is not checked.
- The frame-buffer copy's size and point sampling (above).
- Whether water 0's coordinate has a local translation: the animation has
  no record for it, so the port takes the root alone (`(0, 4200, 0)`,
  `+0x70` zero) for the reach; checked only as inputs to `waterUVModifi2`.
- `fieldrand`'s state on entering the town, and the scrolls across visits
  (above).
