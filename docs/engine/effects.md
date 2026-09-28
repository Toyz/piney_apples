---
title: The field's effects - ccThEffect and ccThParticle
status: partial
volumes: INF
covers: INF SLUS_202.67:0x001c2e60 ccThEffect, 0x001c3030 ccEffectCtrl::ccEffectCtrl, 0x001c38a0 ccEffectCtrl::Main, 0x001c2f70 ccThEffectStr, 0x001c39c0 ccEffectCtrl::MainStr, 0x001d8810 ccEffect::MainStr, 0x001cc620 effHitMarkStr, 0x001ce220 effTransferStr, 0x001c0620 ccParticle::MainStr, 0x001c2a20 ccParticleCtrlAddGenerator, 0x001c2af0 ccParticleCtrlGenerateParticle, 0x001c0e10 ccParticleCtrl::InitParticleCtrl, 0x001c3a60 ccNewEffect, 0x001c3af0 ccEffect::InitEffect, 0x001c3e10 ccEffect::Main, 0x001cc0c0 FadeIn, 0x001cc120 FadeOut, 0x001cc190 FadeInOut, 0x001ce020 effTransfer, 0x001cdef0 effTransferRing, 0x001ce120 effWarpTransfer, 0x0013ba20 ccEff::Init, 0x0013bc00 ccEff::SetBlendType, 0x0014cca0 ccStream::Decode_Eff, 0x001da710 ccCheckCameraDeg, 0x00138050 ccCoord::SetMatrix_PosRotXYZScale, 0x00138120 ccCoord::SetMatrix_PosRotZYXScale, 0x002fd4f0 effectCCSTbl, 0x0033fa60 effectTbl2, 0x0033f230 oneVector, 0x00348580 alphaBlendTbl, 0x0013bcb0 ccEff::Draw, 0x0013bc90 ccEff::Draw(pos, pat), 0x0013bb40 ccEff::SetRenderState, VU1 micro 0x595 mc_DrawEff, 0x001052c0 ccView::SetView, 0x001107b0 sceVu0InversMatrix, 0x00108260 ccLayer::Init, 0x001081d0 ccDLSort::Add, 0x001054d0 ccDrawEnv::Reset, 0x00105820 ccDrawEnv::SetFog, 0x0013f470 ccClump::Draw, 0x0013d7d0 ccClump::SetTransparency, 0x0013c5e0 ccClump::Init, 0x0013f220 ccObj::Draw, 0x00138380 ccCoord::_SetLWMatrix, 0x00138490 ccCoord::_GetTransparency, 0x001524d0 ccAnm::Draw, 0x001cc3e0 effHitMark, 0x001cc410 effHitRing, 0x001cc550 effHitPhoton, 0x001cf250 effProtect, 0x001cc890 effAttributeGuard, 0x001ccb40 effAttributeCritical, 0x001d9510 effCheckAttributeGuardEntry, 0x001d9560 effAddAttributeGuardEntry, 0x001d95c0 effDelAttributeGuardEntry, 0x001bca90 ccParticleAttributeCritical, 0x001bcc40 ccParticleAttributeGuard, 0x001bc540 ccParticleCritical, 0x001bc6d0 ccParticleDying, 0x001bc8c0 ccParticleNoDamage, 0x00162f10 checkCameraShakeRange, 0x001633c0 ccTransPosW2CZW, 0x00162cd0 cameraShake, 0x0015aed0 ccSprite::MakePacketStr, 0x0015cde0 MakeSignedNum, 0x0015cfc0 sdec2str, 0x001cdde0 effLevelUp, 0x001d79f0 effDrainCtrl, 0x001d7c40 effDrain, 0x001cd380 effAfterDrain, 0x00155210 normal2Angle, 0x00123b38 __ieee754_acosf, 0x001274c8 sin, 0x00126ee0 cos, 0x001d9620 genrand, 0x001d9a90 ccRandF, 0x0033f240 effectTbl, 0x003feeb0 effAttributeGuardEntry, 0x00377eec panelCamDist; INF gcmn.prg:0x0059b9c0 ccTransPosFW2LW, 0x0059b5a0 ccPlayer::W2PPos, 0x0059b710 ccPlayer::P2WPos, 0x00421630 ROOTTOWN01's SetFog, 0x005714e0 ccHitMarkDisp, 0x0051a8c0 ccInitFlyFont, 0x0051a900 ccEntryFlyFontNum, 0x0051aa10 ccCtrlFlyFont, 0x0051afc0 ccEntryFlyFontNew, 0x0051aea0 ccEntryFlyFontNewExp, 0x0051af20 ccEntryFlyFontNewLevelDown, 0x0051af70 ccEntryFlyFontNewMiss, 0x0051add0 Int2StrFF, 0x0072ebd0 flyFontCtrl, 0x0051b710 ccDamUprStr::ResetAll, 0x0051b780 ccDamUprStr::CtrlAll, 0x0051bb20 ccDamUprStr::DrawAll, 0x0051bb80 ccDamUprStr::AddStr, 0x0051b0b0 ccUprollStr::ccUprollStr, 0x0051b140 ccUprollStr::AddStr, 0x0051b530 ccUprollStr::Ctrl, 0x0051b320 ccUprollStr::Draw, 0x0052170c ccMenuCtrl::Disp (the numbers), 0x0059a7f4 ccPlayer::AnimCtrl (the transfers), 0x00598404 ccPlayer::Main (level up), 0x004353f0 ccEnemy::interruptThink, 0x004313f0 ccEntryCtrl::entryMagicCircle, 0x00455820 ccEntryGimCircle, 0x00455900 ccMagicCircle::ccMagicCircle, 0x00455b60 ccMagicCircle::main, 0x00455710 ccMagicCircle::createPart, 0x00455510 ccMagicCircle::setPart, 0x004548e0 ccMcPart::main, 0x00454840 ccMcPart::fade, 0x0042e670 ccEntryChangeCLUT, 0x001d1090 effMagicAttackSign, 0x001cd4e0 effMeteoFireBall2, 0x001ccec0 effFlareRing, 0x001d80c0 effSkillBreakSE, 0x001d3d80 effSkillTornadeRingsPos, 0x001d57b0 effSkillTornadeObjPos, 0x001d48b0 effSkillTornadeSmoke, 0x001d5da0 effSkillChargeObject, 0x001d5eb0 effSkillChargeObj, 0x001d12a0 effUpheavalFragment, 0x001d1740 effUpheavalFlash, 0x001d1aa0 effSummonsRing, 0x001d1d30 effSummonsElement, 0x001d1f10 effSummonsShockWave, 0x001d22b0 effSmmonsFragment, 0x001d8190 effSBLockon, 0x001d8410 effTCDrillMissile, 0x001cda30 effSmokeRock, 0x001d4a20 effDarkSmoke, 0x001d4cd0 effRadiateSomething, 0x001d5110 effRadiateSomething2, 0x001d6410 effSmokeIce, 0x001cc240 ccEffect2::InitEffect, 0x001cc290 ccEffect2::Main, 0x00123f78 __ieee754_asinf; INF gcmn.prg:0x005731d0 ccSkill::Main, 0x00577a30 FallSystem, 0x00577540 TornadoSystem, 0x00578060 ConvergenceSystem, 0x005787c0 UpheavalSystem, 0x00579000 SummonsSystem, 0x004e8da0 ccEffectElementManager::Main, 0x004fef30 ccFallElementGenerate, 0x00500bd0 ccSkillTornadeElementsGenerate, 0x004ffd10 ccConvergenceElementGenerate, 0x005007e0 ccUpheavalElementGenerate, 0x005006b0 ccSummonsElementGenerate, 0x00500dc0 ccThunderBoltElement, 0x004fbbe0 ccRingElement, 0x005000b0 effExplode3, 0x004fff50 effEnergyGrow, 0x005002a0 effThunderShock; INF SLUS_202.67:0x001d35f0 effSkillStart, 0x001d3650 effSkillStart(ccSkillParam), 0x001d39d0 effSkillStartEffect, 0x001d2d80 effSkillExecRing, 0x001d2f50 effSkillExecForceRing, 0x001d3100 effSkillExecForceRing2, 0x001d32b0 effSkillExecSummonsCircle, 0x001d3450 effSkillExecCircle, 0x001d2790 effPhysicalSkillHitShockWave, 0x001ccd20 effHeal, 0x001cccf0 effHealSkill, 0x001cef50 effCure, 0x001cf0d0 effSanity, 0x001cedd0 effResurrect, 0x001ced40 effOpenBox, 0x001d0ae0 effFountain, 0x001d0be0 effBossRoomEntrance, 0x001d0c90 effDungeonEntrance, 0x003401f0 effDungeonEntranceColor, 0x001d0980 effRemoveTrap, 0x001d0f00 effEvolvePG, 0x001d0fd0 effGrowPG, 0x001d7380 effAbilityUp, 0x001d7630 effAbilityDown, 0x001d78c0 checkEffAbilityColor, 0x0033fdf0 tableAbilityUpS1-S4, 0x0033fe70 tableAbilityDownS1-S3, 0x00340170 the cures' generator rows, 0x00377ef0 the rings' and waves' tables, 0x001d9ce0 ccGetDirc; INF gcmn.prg:0x00573a10 ccSkillCheckNote, 0x00573be0 ccSkillCheckType, 0x00573c30 _ccSkillCheckType, 0x00501ab0 effResistantShield, 0x00501cb0 effResistantShield(scale), 0x00501970 ccResistantShieldElement::Main, 0x005018e0 ~ccResistantShieldElement, 0x0059b980 ccTransPosP2W, 0x0056f950 ccChar::DispConditionEffect, INF SLUS_202.67:0x001ceaa0 effIceRock; INF gcmn.prg:0x00462d00 ccBossEffIceMissile::Draw, 0x0046cc10 ccBossEffIceBreak::Draw
---

# The field's effects - ccThEffect and ccThParticle

Everything the field draws for a moment and forgets: the rings around Kite
as he arrives, hit sparks, the numbers over a character, what shows as a
skill starts, the spells, heals and cures, deaths, level ups, Data Drain.
Two tasks run them, both started by
`ccSetupGameCtrl` with the field's others ([the field game](field-game.md#tasks-and-the-frame)):

| prio | task | owns |
| ---: | --- | --- |
| 80 | `ccThEffect` (main 0x001c2e60) | `ccEffectCtrl` (`effc`, gp 0x00378abc): 500 `ccEffect`s, 100 `ccEffect2`s; then `ccEffectElementManager` |
| 98 | `ccThParticle` (main 0x001bc2f0) | `ccParticleCtrl`: the particle generators and their particles |

Whoever an effect is for starts it with one of main effect.cpp's `eff*`
functions (`effTransfer` from Kite's arrival act, `effHitMark` from a hit,
...); the tasks then move, draw and end it. `crates/piney-effect` is the
port, checked against the game's own code in eemu by
`tools/test_effect*_rs.py`. The particle system, `ccThParticle`, has
its own page: [the particle system](particles.md).

## ccThEffect

```
ccThEffect (0x001c2e60), once:
  ccEffectElementManager::m_instance = new ccEffectElementManager (if none)
  effc = new ccEffectCtrl(0)
each frame (after its Breath):
  ccEffectCtrl::Main(effc)                    0x001c38a0
  ccEffectElementManager::Main(m_instance)    gcmn 0x004e8da0
ccEffectCtrl::Main:
  WORLD_MAN::SetActiveLayer(3)                the effect layer: +0x4c8, priority 20
  for i in 0..500 with effWork[i].status != 0:
    active__7ccLayer = effWork[i].layer, or SetActiveLayer(3) when it has none
    ccEffect::Main(&effWork[i])
  SetActiveLayer(3)
  for i in 0..100 with effWork2[i].status != 0: ccEffect2::Main
```

`WORLD_MAN::SetActiveLayer(n)` (main 0x001a20b0) picks from `WORLD_MAN`'s
layers by a jump table (0x00355790): 0 `sysLayer` (0), 1 +0x4bc (0), 2
+0x4c0 (-10), 3 +0x4c8 (20), 4 +0x4c4 (-20), 5 +0x4cc (10, the characters),
6 +0x4d0 (30); the numbers in brackets are the priorities `WORLD_MAN::GO`
gives them.

A slot started during the pass (a ring started by `effTransfer`'s case, say)
runs in the same pass when its index is higher, which it usually is: every
`eff*` function takes the first slot whose status is 0.

### ccEffectCtrl (0x2c0 bytes)

```
+0x000  ccEffect *eff          effWork (0x003e4a60), 500 x 0xc0
+0x004  ccEffect2 *eff2        effWork2 (0x003fc160), 100 x 0x14
+0x008  int strFlag            0 in the field; 1 for the in-engine streams
+0x00c  void *adrs[173]        each effect id's object, looked up once
```

The constructor (0x001c3030) with `sw` 0 looks up every row of a table whose
type is not 6 (`GetCCSAdrs(ccs)`, `GetChunkAdrsF(stream, name)`) into
`adrs[id]`: `effectTbl` (173 rows) when `game` +0x14 (the area) is not 0,
`effectTbl2` (4 rows) when it is - in a Root Town. `effectTbl2`'s only
object is row 3, `CMP_x032` (the arrival's ring), so in a town every other
id's `adrs` keeps what the heap held. Outside a town it also fetches
`CLT_x036` / `CLT_x036c1` (`WoodClt1`, `WoodClt2`) and `x703`'s
`ANM_x703nut0` / `ANM_x703atc0` (`effTCDrillNut`, `effTCDrillAct`). Then
every slot's status and object are cleared, the 20 attribute-guard entries
(`effAttributeGuardEntry`) zeroed, a layer of priority 60 made for
`effSBL`, and the smoke particles' force-field pointers (`ccpfSmoke1` ...)
set.

### ccEffect (0xc0 bytes)

```
+0x00  float pos[4]          where it is drawn
+0x10  float offset[4]
+0x20  float rot[4]          radians
+0x30  float speed[4]
+0x40  float scale[4]
+0x50  float posT[4]         usually its target's position when made
+0x60  bits: distSW 0, dispSW 1, pauseSW 2, endFlag 3, zyxFlag 4,
       level 5-8 (signed)
+0x64  int param
+0x68  int flags
+0x6c  short id              -26..172
+0x6e  short status          0 free
+0x70  short lifeTime        -1: until its own code ends it
+0x72  short age
+0x74  short cnt             frames it has run
+0x76  u16 texAnmPat         its ccEff's pattern
+0x78  u16 rotSpeed[4]       16-bit angle steps
+0x80  float velocity
+0x84  float transparency
+0x88  ccChar *targetPtr
+0x8c  void *effPtr          its ccClump, ccAnm or ccEff
+0x90  float (*posPtr)[4]    read into pos each frame when set
+0x94  float (*rotPtr)[4]    read into rot each frame when set
+0x98  ccEffect *linkPtr     pos and rot copied from it each frame when set
+0x9c  ccLayer *layer        drawn on it instead of the effect layer
+0xa0  FreeTemp temp[4]      float / int / short[2] / char[4], each id's own
+0xb0  u32 sn                effectSerialNum (gp 0x00378adc) when made
```

**`InitEffect(id, sw)`** (0x001c3af0): offset, rot and speed zero, scale
`oneVector` (0x0033f230: **(1, 1, 1, 0)**); dispSW, distSW and zyxFlag set,
pauseSW, endFlag and level clear; param, flags, age, cnt, texAnmPat,
rotSpeed and velocity 0; status 1, lifeTime -1, transparency 1; the next
serial number; targetPtr, effPtr, posPtr, rotPtr, linkPtr, layer and temp 0.
`pos` and `posT` are left as they were. For an id of 0 or more it makes the
object `effectTbl[id].type` names, from `adrs[id]` (from `effectStrTbl` and
`effcStr` when `sw` is set, the streams' case), by a jump table (0x00374c60):

| type | object |
| ---: | --- |
| 0 | `ccClump` (0xa0): `ccCoord` constructor, `Init(adrs[id])`, `SetFogSw(0)` |
| 1 | `ccAnm` (0x110): `SetAnm(adrs[id], 0)`, `SetFogSw(0)` |
| 2, 3, 4, 5 | `ccEff` (0x70): `Init(adrs[id], 1)`, then PRIM's fog bit (+0x62 bit 5) cleared |
| 6 | nothing |

**`ccNewEffect(id)`** (0x001c3a60) and every `eff*` function find the first
slot with status 0 and call `InitEffect(id, 0)` on it; with none free they
do nothing.

**`ccEffect::Main`** (0x001c3e10, 33452 bytes):

```
the object by effectTbl[id].type (jump table 0x00374fe0): ccAnm (1),
  ccEff (2-5) or ccClump (0)
endFlag: status 0; the object destroyed; effPtr 0; return
s = 48 bytes of ccSys +0x25c's scratch stack; s[0] = 1
posPtr: pos = *posPtr;  rotPtr: rot = *rotPtr
linkPtr: pos = link->pos, rot = link->rot
the first switch on id + 15 (188 entries, jump table 0x00374cf0): the
  effect's motion; each case ends at the draw, two of id 168's at the end
the draw (0x001c724c):
  not paused, a ccAnm:
    pos = ccTransPosFW2LW(pos)                  (in place)
    SetMatrix_PosRotZYXScale(pos, rot, scale) (zyxFlag) or ..XYZScale
    +0xac set: _AnimateForward(frameSpd +0x9c); at its end endFlag (id
      170 also takes its target off the attribute-guard list)
  not paused:
    eye = cameraGetPos(camID); p = ccTransPosFW2LW(pos)
    dispSW = ccCheckCameraDeg(pos, 12288)
    distSW: d = |eye - p|; s[1] = d
      d < 7000: s[2] = clamp((5000 - d) / 2000, 0, 1);
                dispSW and s[2] == 0: dispSW = 0
      else dispSW = 0
    no distSW: s[2] = 1
    a ccAnm, dispSW:  its +0x88 = transparency * s[2]; ccAnm::Draw
    a ccEff: dispSW: its pos = ccTransPosFW2LW(pos), its transparency =
             transparency * s[2], Draw(texAnmPat); then the pattern steps
    a ccClump, dispSW: SetMatrix_..(ccTransPosFW2LW(pos), rot, scale);
             SetTransparency(transparency * s[2]); Draw
  not paused, a ccEff: the pattern steps again (0x001c76f4)
the second dispatch on id (a compare chain at 0x001c77f8): children,
  timing, sounds; ids it does not name go straight on
the end (0x001cc000): cnt++; lifeTime != -1: age++, and endFlag when
  lifeTime < the age before it; the scratch stack released
```

The first switch gives 161 of its 188 ids a case (27 go straight to the
draw); the chain names 124 ids, sharing 34 cases. Some cases have no maker:
of the 88 calls to `InitEffect` and `ccNewEffect` in each volume's main and
overlays (the same calls and ids in all four), none makes id 0, 113 or
-11..-8, and only `InitEffect` writes +0x6c. Id 0 (`ANM_x300`, the fire
meteor's animation) shares the meteors' cases ([FallSystem](#fallsystem));
113 (`CMP_x042`, 80's model) has its own ([a physical skill's
blow](#a-physical-skills-blow-effphysicalskillhitshockwave)); -11..-8 have
no object and no first-switch case, and their chain case (0x001c9b34) is
`sw $zero, 0($zero)`: the game's "cannot happen" store to address 0, then
the end.

The pattern step, by `effectTbl[id].type`: 3 `texAnmPat++`, endFlag once it
reaches `patNum`; 4 `++`, back to 0 at `patNum`; 5 `++`, held at `patNum -
1`. Not paused, it steps twice a frame (after the draw, and again at
0x001c76f4); paused, not at all. With lifeTime n an effect's code runs on
n + 2 frames (the first the frame it is made on, when it is made ahead of
its slot in the pass), and the next frame's `Main` frees it.

`ccCheckCameraDeg(pos, deg)` (main 0x001da710): `pos`, `activeCamPtr`'s eye
and the point it looks at, each through `ccTransPosW2P` (the player's
`W2PPos`); the headings from the eye to each by `atan2f`, `RAD2DEG`;
`deg + (to pos - to view)` as a 16-bit angle, true when strictly between 0
and 2 deg. 12288 is 67.5 degrees either side.

`ccTransPosFW2LW(out, in)` (gcmn 0x0059b9c0) is `P2WPos(W2PPos(in))` about
`plw`: `W2PPos` takes x and y less the player's, wrapped into the map's span
about its centre (z kept, w 1); `P2WPos` adds the player's position back
(all four lanes, then w 1) and wraps x and y the other way when the player
is on one side of the map's centre and the point past the other. In a town
(bounds +-24000) it only rounds: `(p - player) + player`.

**The fades** (`transparency`): `FadeIn(time, tt)` (0x001cc0c0) `t / time`
(t = `cnt`, or `tt` when `tt` >= 0) until `time < t`, then 1.
`FadeOut(time, tt)` (0x001cc120) 1 while `cnt < tt - time`, then
`(tt - cnt) / time` not below 0. `FadeInOut(in, out, life)` (0x001cc190)
`cnt / in` while `cnt <= in`, 1 while `cnt < life - out`, then
`(life - cnt) / out` not below 0.

## The effect files and tables

`effectCCSTbl` (main 0x002fd4f0, `FILEDATA` rows of 0x2c: name, offset, size,
flag) names the files the effects draw from, in `DATA.BIN`: `DRAIN.CCS`,
`PARTICLE.CCS`, `X104.CCS`, `X204.CCS`, `X304.CCS`, `X404.CCS`, `X504.CCS`,
`X604.CCS`, `X703.CCS`, `X704.CCS`, `X705.CCS`, `X706.CCS`, then an empty row.

`effectTbl` (main 0x0033f240, 173 rows of `ccEffectTbl`: `char *ccs`,
`char *anm`, `int type`) gives each effect id its object: 153 rows in
`particle` (clumps `CMP_`, animations `ANM_`, sprites `EFF_`), 9 in `drain`
(ids 12-14 `ANM_xdhdref0`-`2`, 19-24 `ANM_xdhpros0` ... `ANM_xdhprol1`), 9 in
the `x?04` / `x70?` files (ids 57-63 `ANM_x104` ... `ANM_x704`, 164
`ANM_x706`, 165 `ANM_x705`), and ids 4 and 168 with none. 171 rows resolve
an object outside a town. `effectTbl2` (0x0033fa60) is the town's: rows 0-2
none, row 3 `particle` / `CMP_x032` / 0. `effectStrTbl` (0x0033fa90) is the
streams' (`EFF_x007`, `CMP_x037`, `CMP_x032`).

### The Eff chunk (0x0e00)

`ccStream::Decode_Eff` (main 0x0014cca0) makes a `ccEffChunk` (0x38 bytes,
with `ccEffUVPat pat[patNum]` after it) of each; little-endian, the size field
in words as for every chunk:

```
u32     object         the EFF_ object
u32     texture        its TEX_ object (ccEffChunk +0x0c texIndex)
u16     flag           +0x18; flag & 7 is the blend type
s16     zoffs          +0x1a
u16     unknown_0c     not read (112 in all 49 of PARTICLE.CCS's)
u16     patNum         +0x1c
f32     x0, y0, x1, y1 +0x20: the quad's corners
u16     w, h           +0x30 wh = w | h << 16: a pattern's UV size
patNum x { u16 u, u16 v, u16 transparency (4096 opaque), u16 pad }
```

UVs are in 1/4096ths of the texture (ST = (u + du) / 4096 with Q 1): 4096
is the whole texture whatever its size. `PARTICLE.CCS` has 49, `EFF_x000`
first: 46 patterns each the whole texture (w = h = 4096), corners (-5, -5,
5, 5), flag 9.

`ccEff::Init(chunk, fogSw)` (main 0x0013ba20) copies the corners, `patNum`,
`pat`, `wh`, `zoffs` and flag; puts it at the origin, scale 1 and 1, turn 0,
colour 0x808080, transparency 1; the texture's `ccTex` (+0x38, from
texIndex +0x2c) and palette (+0x3c) when the texture has them; PRIM
`fogSw << 5 | 0x54` (a textured, blended triangle strip), TEST 0x50000 (z
test GEQUAL); then `SetBlendType(flag & 7)` (0x0013bc00): ALPHA
`alphaBlendTbl[type]` (main 0x00348580); for type 0 TEST's alpha test is
ATE, ATST 5 (GEQUAL), AFAIL 1 and flag bit 0x20 clears, for any other ATE,
ATST 0 (NEVER), AFAIL 1 (FB_ONLY: colour written, depth not) and the bit
sets.

## Drawing

What an effect's `Main` sends - a sprite, a clump or an animation - goes to
the GS as the game sends it. The port records it (`DrawRec`) during the
step and draws the records into the frame's layers afterwards.

### Drawing a sprite: ccEff::Draw and mc_DrawEff

`ccEff::Draw(pat)` (main 0x0013bcb0; `Draw(pos, pat)` 0x0013bc90 copies
pos x, y, z first) builds one 27-qword packet, sorts it into the active
layer, and VU1's `mc_DrawEff` (micro 0x595, byte 0x2ca8) turns it into one
textured triangle strip facing the camera.

```
a = ftoi0(transparency * (float)pat[pat].transparency) >> 5   (s32, arithmetic)
a < 0: return (nothing sent, the scratch stack untouched); a > 255: 255
m = unit; m[0][0] = scaleX; m[1][1] = scaleY
rotate != 0 (c.eq.s: -0 counts as 0): m = RotMatrixZ(m, rotate)
m = view.fview * m; m[3].xyz += pos.xyz; m = view.world_screen * m
m[3][3] < view +0x1dc (bboxClipMin.w), or not <= +0x1ec (bboxClipMax.w): return
p = active layer's packetCtrl.GetWork(432); none: return
texture or palette with a ccBltData whose flag lacks bit 1:
  ccBltData::MakePacketLoadDataSub chains its upload before the packet
ccDLSort::Add(layer.sort, m[3][2] / m[3][3], p, p + 16)
```

`a >= 4096` would take an unsorted path (appended to the layer's list);
`a` never exceeds 255, so every sprite goes to the sorted group.

#### The packet

| qword | contents |
| --- | --- |
| 0 | DMA NEXT qwc 0 to qword 1 (the head the uploads are put before) |
| 1 | DMA NEXT qwc 25 (the tail; `Add` links it); VIF STMOD 0, STCYCL 4,4 |
| 2 | STMASK 0x50505050, FLUSHA, UNPACK V4-32 num 18 to VU 0 |
| 3-6 | VU 0-3: `m` |
| 7 | VU 4: not written, not read |
| 8 | VU 5: ccDrawEnv fMin (+0xc8), fMax (+0xcc), fogB (+0xd4), fogA (+0xd0) |
| 9 | VU 6: GIFtag PACKED nloop 1, nreg 10 x A+D (the qword at 0x002fb110) |
| 10-19 | VU 7-16, A+D: ALPHA_1, TEST_1, TEXFLUSH, TEX0_1, TEX1_1, MIPTBP1_1, CLAMP_1, RGBAQ, FOGCOL, ZBUF_1 |
| 20 | VU 17: GIFtag PACKED eop nloop 1 nreg 9, PRE with PRIM = `ccEff.prim`; regs FOG, (ST, XYZ2) x 4 (0x002fb120, low word ORed with prim << 47) |
| 21-22 | STMOD 1 (offset: add the row), STROW (u, v, 0, 0x3f800000), NOP, UNPACK V2-16 usn masked num 4 to VU 32 |
| 23 | its data: (0,0) (w,0) (0,h) (w,h), `w`/`h` from `wh` |
| 24-26 | NOP, UNPACK V2-32 masked num 4 to VU 36: (x0,y0) (x1,y0) (x0,y1) (x1,y1); NOP, MSCAL 0x595 |

The mask takes x and y from the data (plus the row) and z, w from the row,
so VU 32-35 are `(u + du, v + dv, 0, 1.0 bits)` and VU 36-39 the corners
`(x + u, y + v, 0, 1.0)` - the offset mode adds `u` and `v` (integers) to the
corners' float bits too, moving them a few units in the last place.

The registers:

- ALPHA_1 `ccEff.alpha` (`alphaBlendTbl[type]`, `SetBlendType` indexes it
  unmasked: the table has 9 entries).
- TEST_1: `ccEff.test`; unless flag bit 0x20 is set (blend type 0),
  ORed with `((texChunk ? aref : 0) * a >> 7) << 4` (aref the texture's
  +0x42; nothing clears the field first and a value over 255 would run into
  AFAIL; every effect texture's aref is 112, so at most 223). With ZTE set
  (as `Init` leaves it) ZBUF_1 is `ccSys` +0xbc8; with ZTE clear
  (`SetRenderState(0, 0)`), TEST's ZTE/ZTST become 1/1 (ALWAYS) and ZBUF_1
  gets ZMSK.
- TEX0_1 `tex.tex0 | clut.tex0` (the texture's PSM, TW, TH, TCC 1, TFX 0
  MODULATE from `ccTexChunk::SetBuffAdrs` 0x00137090; the palette's CBP /
  CPSM / CLD); TEX1_1 `tex.tex1 (MXL = mipmap) | ccDrawEnv.tex1`; MIPTBP1_1
  the texture's (0: no effect texture has mip levels); CLAMP_1 5 (clamp) for
  a texture flag with 0x10, else 0. No texture: all four 0.
- RGBAQ: `color & 0xffffff | a << 24`, Q 1.0 - the Q the whole strip uses.
- FOGCOL `ccDrawEnv.fogColor` (+0xd8).

#### mc_DrawEff (VU1 0x595-0x5d1)

```
fog = clamp(fogB + fogA * m[3][3], fMin, fMax)     mini then max; the centre's w
per corner p (VU 36 + i, UV VU 32 + i), i = 0..3:
  q = ((m0 * p.x + m1 * p.y) + m2 * p.z) + m3 * p.w   (mulax, madday, maddaz, maddw)
  v = q.xyz * (1 / q.w)                               (div Q, then mulq)
  inside = 0 < v.x < 4095 and 0 < v.y < 4095          (sub.xy 0 - v, v - 4095;
                                                       fmand of both sign pairs)
  a corner outside: branch to the end - no XGKICK, the whole sprite not drawn
  ST = itof12(u + du, v + dv)          (Q stays 1.0: 1/4096ths of the texture)
  XYZ2 = ftoi4(v.x, v.y, v.z), word 3 = 0 (drawing kick)
VU 18.w = ftoi4(fog)                   (FOG's F: bits 4-11 of that word)
XGKICK VU 6
```

The FMANDs read the MAC flags four pairs after the SUBs they test (the FMAC
pipeline's length), with no stall between. A model that gives an FMAND the
flags of the upper instruction in its own pair (tools/vu.py's `Vu` does)
rejects every sprite.

`u`, `v`, `w` and `h` are therefore in 1/4096ths of the texture's size (for
`EFF_x000`, w = h = 4096: the whole texture); 1/16 texel only for a 256-texel
texture.

#### What Draw reads besides the ccEff

- The active layer's `ccView` (+0x2c; every field layer shares `sysLayer`'s):
  `fview` (+0x10), `world_screen` (+0xd0), `bboxClipMin.w` (+0x1dc) and
  `bboxClipMax.w` (+0x1ec), which `ccLayer::Init` (0x00108260) sets to 8 and
  2^20. `ccView::SetView(cam, mat)` (0x001052c0; `cameraSet` passes `mat` 0)
  makes `fview` from `world_view` (+0x50): `sceVu0InversMatrix`
  (0x001107b0: the 3x3 transposed, w lanes 0, translation `0 - R^T t` from a
  mulax/madday/maddz chain, w `t.w`) times `RotMatrixX(-pi)`, then its
  translation column set to (0, 0, 0, 1). It turns the sprite's local x/y
  plane to face the camera with y up the screen.
- `ccDrawEnv::active`: `tex1` (+0xa8), fMin/fMax/fogA/fogB (+0xc8-0xd4),
  fogColor (+0xd8). `Reset` (0x001054d0): `SetFog(0.1, 16777215, 0, 0, 0)`
  (fMin = fMax = 2.55 * 100, 254.99998 after the EE's truncating multiply:
  FOG 254 everywhere), `SetMipmapMode(1, 5)` and `SetMipmap(0xff18, 0)`
  (tex1 0x00000f18_00000160: MMAG linear, MMIN 5, K 0xf18).
  `SetFog(near, far, nearRate, farRate, colour)` (0x00105820): fMin =
  2.55 (100 - farRate), fMax = 2.55 (100 - nearRate), fogA = (fMin - fMax) /
  (far - near), fogB = fMax - near (fMin - fMax) / (far - near), colour's low
  24 bits. Mac Anu's (`ROOTTOWN01::ROOTTOWN01`, gcmn 0x00421630) is
  `SetFog(1000, 7500, 0, 85, 0x144870)`.
- `ccSys` +0xbc8: ZBUF_1 (the Z buffer's block address >> 5 | the frame
  buffer type << 24 from `ccSystem`'s screen set-up; ZMSK 0).

The effects' own sprites have PRIM.FGE off (`ccEffect::InitEffect` clears
bit 0x20 of +0x62 after `Init(chunk, 1)`), so their FOG is sent but unused.

#### The skills' other effects

What the battle starts around a skill besides the attack spells: the
start every skill shows on its caster, a physical skill's blow, heals and
cures, a box opening and a trap removed, the stat changes and the
resistant shield. Ported in `skillstart.rs`, `shockwave.rs`, `heal.rs`,
`ability.rs` and `shield.rs`; who calls them is under [piney-battle's
events](#from-piney-battles-events). `->` below is a ring's linear
growth over its life, `from + cnt ((to - from) / lifeTime)`, as each case
computes it.

### The start: effSkillStart

```text
effSkillStart(ch, sid, a, b)        (main 0x001d35f0) = effSkillStart(ch,
                                    ccGetSkillParam(sid), a, b)
effSkillStart(ch, sp, a, b)         (main 0x001d3650; the enemies call it
                                    with their own rows)
  e = sp->type & 0xfc: soil 4, water 8, fire 16, wind 32, thunder 64,
    dark 128 (places 0-5), anything else 6
  k = ccSkillCheckType(sp) (gcmn 0x00573be0): 0 for skillTbl[1], else
    _ccSkillCheckType(type) (0x00573c30): physical (bit 0) 0 with any of
    bits 10-12 (every physical skill of skillTbl), else -1; magic (bit 1)
    3 with bit 18 (the Repths), 2 with bit 16, -2 with bit 17, else 1
  a generator of particleGeneratorTbl[r + e]: r 164 (b: 178) when k,
    else 157 (171); on ch's pos, offset (0, 0, b ? height : 0, 0)
  effSkillExecRing(ch, (0, 0, height, 0), type, b)
  the controller -12 (b: -13): life 7, param type, flags k, offset (0,
    0, height, 0), target ch, posT its pos
  a == 0: effSkillExecForceRing2(ch, type, b), effSkillExecForceRing(..)
  ccSeOn3D(104, ch's pos) when ch's type & 0x0700000f (a PC or member),
    else ccSeOn3D(159) when & 0xe0 (an enemy or boss); the controller
effSkillStartEffect(ch, attr, b)    (main 0x001d39d0) the same with r 164
                                    (178), param attr, flags 1, the force
                                    rings always
-12, -13 (second chain 0x001cac4c): the target off the lists: endFlag; at
  age == lifeTime (count 7) effSkillExecSummonsCircle(target, &offset,
  param, id == -13) when flags, else effSkillExecCircle
```

| effect | starter | object, life | CLUT by place 0-6 (particleCcsAdrs; - none) | first switch |
| --- | --- | --- | --- | --- |
| 82 | `effSkillExecRing(ch, off, type, b)` (0x001d2d80) | CMP_x038, 20; offset off | 197 made 200, 198, 199, -, 200, 201, 203 | 0x001c4ee0: pos.z += offset.z; x, y, z 0 -> 2 (b: 2 -> 0); FadeInOut(2, 15, life) |
| 83 | `effSkillExecForceRing(ch, type, b)` (0x001d2f50) | CMP_x033, 30 | 175 made 181, -, 176, 178, 177, 179, 180 | 0x001c4f78: x, y 0.5 -> 2.5 (b: back); FadeInOut(5, 15, life) |
| 84 | `effSkillExecForceRing2(ch, type, b)` (0x001d3100) | CMP_x030, 30 | 161 made 167, -, 162, 164, 163, 165, 166 | 0x001c4ffc: x, y 1 -> 3, z 0.2 -> 1.2 (b: back); FadeInOut(5, 15, life) |
| 85 | `effSkillExecSummonsCircle(ch, off, type, b)` (0x001d32b0) | EFF_x047 sprite, 25; offset off | its `ccTex` +0x3c: 230, -, 225, 227, 226, 228, 229 | 0x001c50e0: pos.z += offset.z; the sprite's scale 0 -> 2 (b: 2 -> 0); FadeInOut(2, 15, life) (b: (12, 2, life)) |
| 86 | `effSkillExecCircle(..)` (0x001d3450) | EFF_x003 sprite, 25 | 130, 126, 127, -, 128, 129, 131 | as 85 |

Every ring follows ch's pos (`posPtr`) with `param` b; a clump ring is
`Duplicate`d and `ChangeClut`ed. The tables are gp's (main 0x00377f48,
0x00377f58, 0x00377f68, 0x00377f88 and the fades' times 0x00377f98),
loaded onto `Main`'s stack. So a physical skill (Saber Dance) shows its
sparks, the exec ring rising at the feet, the two force rings (unless
`a`), and at count 7 the circle 86; a spell the circle 85.

### A physical skill's blow: effPhysicalSkillHitShockWave

`ccSkillCheckNote` (gcmn 0x00573a10) calls it at `nowSkillPtr`'s target
position (+0x60) for each 0x8002 note of the skill's animation, with the
skill's `type & 0xfc`.

```text
effPhysicalSkillHitShockWave(p, attr)   (main 0x001d2790), attr exactly:
  soil 4     effSmokeRock(p, down, 30, 0, 8, -1)   CLUTs 167, 219, 212
  water 8    effSmokeIce(p, down, 30, 0.5, 8)      -, -, -
  fire 16    effSmokeSparks(p, down, 30, 2, 8)     162, 218, 211
  wind 32    effSmokeLeaf(p, down, 30, 1, 16)      164, 220, 213
  thunder 64 effSmokeElectric(p, down, 30, 1, 16)  163, 219, 212
  dark 128   effSmokeSmoke(p, down, 30, 2, 8)      165, 221, 214
  else       nothing, no radiate                   166, 222, 215
  (down: -90 degrees about x); an element: effRadiateSomething(p, down,
  30, attr, 8); checkCameraShakeRange(p): cameraShake(0, 2, 10, 0);
  ccSeOn3D(35, p)
  79 (CMP_x030, 161 made the first CLUT) life 15; two 80 (CMP_x042, 217
  made the second) life 15, param 0, 1; two 81 (CMP_x041, 210 made the
  third) life 20, param 0, 1; all at p, level 1; the last 81 answered
79 (0x001c4b74)  x, y 0.5 -> 2.8, z 0.5 -> 1.5; turning 1092 a frame
80 (0x001c4c4c)  x, y 1 -> 2.8 (param 1: 3.6), z 1 -> 2.1 (2.3); turning
                 -2184 (2184); param 1 rises 12.6 a frame
81 (0x001c4da4)  x, y 1 -> 3 (2), z 1 -> 5.3 (8.9); turning -1092 (1092)
                 each FadeInOut(5, 5, life)
113 (0x001c5a80) CMP_x042 like 80, made by nothing: x, y 2.6; z
                 2.5 + cnt (2.2 - 2.5) / 5 while cnt < 5, 2.2 while cnt <
                 life - 15, then 0.5 + (2.2 - 0.5) (life - cnt) / 15;
                 FadeInOut(5, 0, life): at cnt = life the fade-out's 0 / 0
                 is the EE's largest number (0x7fffffff), after it 0
```

The tables: 80's main 0x00377ef0 (turns 0x00377f10), 81's 0x00377f18
(turns 0x00377f40). The smoke and radiate pieces are the spells' (the
convergence's burst: [ConvergenceSystem](#convergencesystem)).

### Heals, cures, a box, a trap

```text
effHeal(ch, n)          (main 0x001ccd20) 130 (CMP_x060) life 30 on ch's
                        pos; the controller -19 life 30, level n (its 4
                        bits, signed), on ch's pos; with -19
                        ccParticleHeal(ch, 1) and ccSeOn3D(75, ch's pos);
                        answers 0
effHealSkill(ch, sid)   (0x001cccf0) effHeal(ch, sid - 149): Repth 1, Ol
                        Repth 2, Pha Repth 3, La 4, Ola 5, Phal 6
130 (first switch 0x001c5b6c): z 1 -> 2.3 over its life less 10, then
  2.3 - 0.23 (10 - (life - cnt)); x, y 0.2 -> 1.2 over 10 frames, 1.2,
  and from 10 before the end 4 + (1.2 - 4)(life - cnt) / 10;
  FadeInOut(5, 5, life)
-19 (second chain 0x001cb354): every fourth count flags + 1; while the
  old flags are below the level a generator 211 on ch's pos (so n
  bursts), else endFlag
effCure(ch)             (0x001cef50) -2  } life 120, target ch, posT its
effSanity(ch)           (0x001cf0d0) -3  } pos, temp[0] half its height;
effResurrect(ch)        (0x001cedd0) -1  } a generator 107, 108 or 61 on
                                           ch's pos 100 over its head
-1, -2, -3 (second chain 0x001c9348): the target off the lists: endFlag;
  count 12: A and B, 30: D, 55: C, on the target's pos temp[0] up; by
  -id - 1 (main 0x00340170 A, 0x00340180 B, 0x00340190 C, 0x003401a0 D):
  resurrect 95, 97, 96, 98; cure 99, 101, 100, 102; sanity 103, 105,
  104, 106
effOpenBox(pos)         (0x001ced40) a generator 94 at pos; answers 1
effRemoveTrap(pos, a, b) (0x001d0980) a -1: 101, b -1: 118; the
                        controller -6 at pos, life 5, param b; a generator
                        118 at pos, pTexMod a
-6 (second chain 0x001c98a8): at count == lifeTime a generator 119 at
  its pos, pTexMod param
```

### A Grunty growing up

`ccPGuso`'s growing-up acts ([town02.md](town02.md#the-grunties)) call
these (piney-effect's `pg`):

```text
effEvolvePG(ch)         (main 0x001d0f00) the first free slot of the 500
                        (+0x6e 0): InitEffect(-7, 0), lifeTime 120, target
                        ch, posT ch's pos; answers the slot (0 with none)
-7 (second chain 0x001c9920): count 12 generators 126 and 127, count 30
  generator 128, each on ch's pos (syncPos, syncPosType 0) with the
  offset (0, 0, half ch's base->height); it does not look at whether ch
  is listed, and has no object of its own
effGrowPG(ch)           (0x001d0fd0) a generator 129 at ch's pos with z
                        replaced by half its base->height; answers 1
```

### The dungeon's objects: crystals, breakables, traps, the statue

`ccGimBox` and `ccGimIdol` ([the battle](battle.md)) start these:

```text
effVirusCrystal(pos)    (main 0x001cf420) the controller -4 at pos, life
                        60; a generator 109 at pos
-4 (second chain 0x001c96f8): at count 13 and 20 a generator 109 at its
  pos (the 20 is a register the chain's compares left holding it)
effWoodFragment(p, r, v, s, n, x)   (0x001cf540; effEggshellFragment
  0x001cf8b0, effPotFragment 0x001cfc20, effBoneFragment 0x001cff90 the
  same but the ids) n pieces, each:
    rn = rand() >> 3
    d = Rot(r) Rot(0, (rn << 8) & 0xfc00, rn % 12288 + 4096) (0, -1, 0, 1)
    id base + (rn & 15) % 3: wood 25, eggshell 138, pot 142, bone 146
    speed d v (100 - (rn >> 8) % 50) / 100, velocity v
    rotSpeed x (rn >> 4) & 0x3c00, z (rn >> 8) & 0x3c00; rot x and z on
      by three times those
    pos p + |speed| s (100 - (rn >> 16) % 50) / 100, life 90
    x not 0: the clump duplicated (Duplicate(0x3800)) with CLT_x036
      swapped for CLT_x036c1 (WoodClt1, WoodClt2: ccEffectCtrl's
      constructor reads them outside a town from effectTbl row 25's file)
  the last piece; the pieces tumble and bounce as the debris do
effCrushBarrel(pos, x)  (0x001d0300; effCrushEgg, effCrushPot,
  effCrushCorpse with their fragments) the fragments (pos, (-pi/2, 0,
  0), 18, 50, 15, x); a generator 110 at pos; answers 1. A copy of pos
  50 up is made and not used.
effOpenTrapBox(pos, kind, trap)  (0x001d0780) the controller -5 at pos,
                        life 5, param 1 for trap 0; kind not 0 (a wooden
                        box or barrel): the wood fragments (pos, (-pi/2,
                        0, 0), 18, 50, 15, kind); a generator 110 + trap
                        at pos; ccSeOn3D(39, pos)
-5 (second chain 0x001c9770) every frame: rn = rand() >> 3; q = pos +
  Rot(0, (rn << 8) & 0xfc00, rn % 12288 + 4096)(0, -75 (100 - (rn >> 16)
  % 20) / 100, 0, 1), w 1; ccParticleExplode(q, speed, 1.8, param)
effStatueOfGod(pos, sw) (0x001d0e60) a generator 125 at pos, its syncSW
                        sw: the idol's effsw (+0x1e0), so it runs until the
                        idol has been open 100 frames; answers 1
effFountain(pos, sw)    (0x001d0ae0) generators 120 and 121 at pos, their
                        syncSW sw: the spring's effsw (ccGimEtc +0x1e0), so
                        the mist runs until ctrlFountain's state 10
effDungeonEntrance(pos, sw, n)
                        (0x001d0c90) a generator 238 at pos, syncSW the
                        entrance's effsw, pTexMod
                        effDungeonEntranceColor[n] (0x003401f0: -1, 0x66,
                        0x67, 0x68; stored when not -1); a field's
                        entrance swirls pass n 2
effBossRoomEntrance(pos, sw)
                        (0x001d0be0) a generator 122 at pos, syncSW the
                        warning's effsw, which ccGimEtc never clears for
                        row 19
```

`ccGimBox::breakObject` passes x 0 (barrels and wooden boxes the wood,
pots, bodies the bones, eggs), `invokeTrap` kind 0 for a treasure box and
1 for a wooden box or barrel with trap 0, 3 or 4. The only CLUT swap is
a trapped wooden box's.

### The ice rocks

The bosses' ice throws rocks 15-18 (`CMP_x202a`-`d`):

```text
effIceRock(p, r, v, s, n)   (main 0x001ceaa0) n rocks, each: rn = rand()
  >> 3; id 15 + (rn & 3); thrown from p as effSmokeRock throws its rocks
  (the direction turned by r, speed v (100 - (rn >> 8) % 50) / 100), the
  rise (speed.z) cut to 0.6; life 90, scale s, velocity v; rotSpeed x
  (rn >> 4) & 0xf00, z (rn >> 8) & 0xf00; the last rock
first switch 0x001c4034: the debris' tumble (ConvergenceSystem)
second chain 0x001c839c: the upheaval rocks' landing (0x001c9b44, the
  same code): 0.75 a bounce, z by -0.75; FadeOut(10, lifeTime)
```

| caller | throws |
| --- | --- |
| `ccBossEffIceBreak::Draw` (gcmn 0x0046cc10), from Skeith's magic (`ccBoss01::OnMagicAtk`, [the boss](boss.md)) and Fidchell's `ccBoss04::OnIceBreak` | twelve, 60 Draws after its smoke is killed, from 100 above pos turned about z by 0, pi/2, -pi, -pi/2: (v, s, n) (50, 0.2, 10), (25, 1, 5), (10, 5, 1) each way, 64 rocks |
| `ccBossEffIceMissile::Draw` (gcmn 0x00462d00), from Innis's `ccBoss02::IceAttack` ([Innis](boss-innis.md)) | two at its impact, turned -pi/2 about x: (50, 1, 20), (100, 0.5, 20), 40 rocks |

The callers are the same in all four volumes (Infection's names, carried),
and so is Mutation's case.

### Smoke and dust

`effSmoke` makes one particle on effect.cpp's static `ccpgSmoke`. gcmn's
enemy dust is built on it, and so are the Grunties', the dogs', the
riding Grunty's, the symbols', the fields' steam and puffs, and the later
volumes' bosses' (`ccBoss04`, `06`, `08_b`, `ccBossEffLightBallShock`,
`ccBossEffRockTower`: not ported).

```text
effSmoke(pos, v, s, life, t, in, out)  (main 0x001ce300) rn = rand() >> 3;
  ccParticleSetup(t, pos, (int)(life - 0.5 life (rn % 101) / 100), 0,
  ccpgSmoke); velocity v, speed |v|, size and scale x, y s, fadeInD in,
  fadeOutD out, rotate[3] (rn >> 3) & 0xfff0; answers 1 (0 with no slot)
ccEnemyEffDust(p, n, s, life, t)  (gcmn 0x0043a3e0; 0x0043a3b0 with life
  30, t 109) n puffs: M = Rz(ccRandF(pi)); effSmoke(p + M (10 s, 0, 0),
  M (s / 2, 0, 0) + (0, 0, s / 4), s, life, t, 512, 32)
ccEnemyEffDustRing(p, s, r, n, life, t)  (gcmn 0x0043a590) n under 4:
  ccEnemyEffDust(p, n, s, life, t); else a = ccRandF(pi), n puffs
  effSmoke(p + Rz(a) (r, 0, 0), Rz(a) (s, 0, 0) + (0, 0, s / 4), s, life,
  t, 512, 32), a on by 2 pi / n, less 2 pi once past pi
ccEnemyEffDustRing(ch, ofs, s, r, n, life, t)  (gcmn 0x0043a7a0) the ring
  at FW2LW(ch's pos) + Rot(ch's dirc) ofs
```

Who raises dust:

- **An enemy's feet.** `ccEnemyDustCtrl::ctrl` runs from its race's
  `exclusive()` and on notes 1 and 2 (`piney_world::combat::dust`). Each
  `ccEnemyDustInfo` row names an animation (2 and 3 match either), a
  window of frames and how often to fire. With `every` 0 it fires only on
  a note. With first and last frames equal it fires every frame. Otherwise
  it fires once per `every` frames. It raises a ring at the row's offset,
  in the enemy's `eneSmoke` texture (`ccCheckDustColor`: 116 on stone,
  else 117).

  ```text
  ccEnemyDustInfo (0x20 bytes): +0 anm (2 and 3 match either), +1 n, +2
    life, +3 every, +4 first frame, +6 last frame (shorts), +8 s, +0xc r,
    +0x10 ofs
  ctrl(obj, flag) (gcmn 0x0043aa90), with dispSW, each row whose anm is
    the enemy's anmNum and whose frames hold its frameNum:
      every 0: only on a note (flag)
      first == last: every frame
      else once a bucket: k = first + (frame - first) / every * every,
        skipped while it is the node's last, which it becomes
    ccEnemyEffDustRing at FW2LW(pos) + Rot(dirc) ofs, (s, r, n, life,
    eneSmoke)
  ```
- **A gold goblin running.** `ccEnemyEffDust(FW2LW(pos), 1, size, 6,
  eneSmoke)`.
- **An idol opening** (from frame 100). `ccEnemyEffDustRing(idol, (0,
  800, -530), 4, 100, 8, 30, 132)`.
- **Kite and the party running.** `ccEffPawSmoke(ch, speed)` (main
  0x001ce640) makes one puff on effect.cpp's other static, `ccpgPawSmoke`:
  - it rises at the lower of the character's `OBJ_t0 l foot` and `OBJ_t0 r
    foot` nodes;
  - it flies back along the heading at 0.1 x 0.0375 x speed x (10 - rand()
    % 5);
  - it lives 10 less up to 5 frames, at size 1.5, with fades 1024 and 64;
  - water and grass (0xb0c000, 0xc0d000, 0x60b0d0, 0x70c0e0, 0x8080f0)
    raise none, the grey and dark grounds texture 4, the rest 133.

  The port poses the feet from the actor as it stands when the effects
  start.

### Stat changes: effAbilityUp, effAbilityDown

`ccConditionEffect`'s constructor ([the particle system](particles.md))
calls them for conditions 0-18 (down) and 20-34 (up), keeps the effect
and its serial number, and ends it with the condition.

```text
checkEffAbilityColor(num) (main 0x001d78c0): 13, 29 -> 0; 14, 21, 30 ->
  1; 15, 31 -> 2; 16, 20, 32 -> 3; 17, 33 -> 4; 0, 1, 4, 6, 18, 34 -> 5;
  else 6: the column of the texture tables (main 0x0033fdf0 UpS1 ...
  0x0033feb0 DownS3, eight ints each, read by their low halves)
effAbilityUp(ch, num)   (0x001d7380) the controller -20 (target ch, posT
                        its pos, param num); with it generators 212 (UpS1)
                        and 213 50 up (UpS2) on ch's pos; the controller
                        -21 (life -1, the same fields), answered
effAbilityDown(ch, num) (0x001d7630) generators 217 (DownS1), 218
                        (DownS2), 219 (DownS2) on ch's pos; the controller
                        -22 (life -1), answered
-20 (0x001cb418): ch off the lists: endFlag; count 4: 214 (UpS3); 10:
  215 (UpS4); from 11 endFlag
-21 (0x001cb5cc): ch off the lists: endFlag; (count - 30) % 45 == 0: 212
  and 213 50 up; == 20: 214
-22 (0x001cb83c): ch off the lists: endFlag; (count - 30) % 30 == 0 (so
  count 0 too): 220 (DownS1), 221 (DownS2), 219 50 up (DownS3)
```

Every generator follows ch's pos (`syncPosType` 0) with the pTexMod
named; `%` is C's.

### The resistant shield

`ccEnemy::affectEnemy` and the bosses' `Affect` call
`effResistantShield(ch, magic, -1)` (gcmn 0x00501ab0) when a blow meets
an immunity (Exdefense); it makes a `ccResistantShieldElement` (gcmn
effect2.cpp, 0x1a0 bytes) on the element manager.

```text
effResistantShield(ch, magic, n)
  base->type & 0x60 (an enemy): s by n (n < 0: ccCheckObjectSize(ch)):
    1 -> 7, 3 -> 4, 4 -> 2, else 0; scale (s/2, s/2, s/2, 1)
  & 0x80 (a boss): s by n itself; s > 0: (s, s, s, 1), else (width,
    width, height/2, 1) * 0.01 (all four lanes)
  anyone else: nothing
effResistantShield(ch, magic, scale)       (gcmn 0x00501cb0)
  ch off the lists or down (dead not 0, 1), or its affectPerson (+0x98)
    on the lists and down: nothing
  the element: ccEffectElement's defaults; m_anmShield a ccAnm of
    particle's ANM_x070 (magic 0 or less) or ANM_x069; m_targetChar ch;
    the manager's first empty slot (none: deleted, nothing more)
  m_offset (0, -width/2, height/2, 1), m_scale scale, m_dirc (0, 0,
    ccGetDirc(ch->posP, affectPerson->posP), 1); ccSeOn3D(99, ch's pos)
Main (0x00501970)
  the target off the lists or down: m_delFlag (nothing drawn)
  m_pos = ccTransPosP2W(target->posP + RotMatrix(m_dirc) m_offset)
  _AnimateForward; SetMatrix_PosRotZYXScale(m_pos, m_dirc, m_scale);
  ccAnm::Draw at its transparency 1 (the ccCoord constructor's) on the
  effect layer; the animation's end: m_delFlag
```

`ccGetDirc(a, b)` (main 0x001d9ce0) is `atan2f(b.y - a.y, b.x - a.x) +
pi/2`, less 2 pi above pi, plus 2 pi below -pi. `ccTransPosP2W` (gcmn
0x0059b980) is `plw->P2WPos`.

## The port

`crates/piney-effect/src/sprite.rs`:

- `Eff::packet(assets, pat, camera) -> Option<Packet>`: what one
  `Draw(pat)` sends, bit for bit - None when nothing is sorted (transparency
  below 0, the centre outside the w range, or `pat` past the chunk's
  patterns); `Packet { key, matrix, gs }` with `gs` None when `mc_DrawEff`
  kicks nothing; `GsSprite` holds ALPHA, TEST, TEX1, CLAMP, the texture
  (`TexInfo`: TEX_ and CLT_ objects, flag, PSM, mipmap, aref, TW, TH),
  RGBAQ, FOGCOL, ZBUF, PRIM, FOG's F and the four (S, T, X, Y, Z).
- `Eff::render(assets, layers, layer, pat, camera)`: the packet as a
  `piney_draw::Prim` in `layer`'s sorted group at the key: a strip (PRIM
  bits 0-2), flat (IIP), blend `Blend::from_reg(ALPHA)` when ABE, the alpha
  test from TEST (ATE, ATST, AREF, AFAIL), depth from ZTST and ZBUF.ZMSK,
  texture `TexRef::Ccs { file, texture, clut }` MODULATE with alpha, linear
  when TEX1.MMAG, clamped when CLAMP is 5; vertices in frame-buffer pixels
  (`(X - XYOFFSET) / 16`), Z as sent, texels `S * 2^TW`, `T * 2^TH`, RGBA
  from RGBAQ. `Prim` has no fog; FOG is dropped.
- `fview(world_view)`, `DrawEnv` (in `draw::Camera::env`: the view's w range,
  the fog - `DrawEnv::set_fog` is `SetFog` - TEX1 and ZBUF; its `Default` is
  the field's view with a `Reset` draw environment), `eff_textures` (read
  into `Assets::eff_tex` at start).

### Drawing clumps and animations

`crates/piney-effect/src/nodes.rs` gives what an effect's `ccClump` or
`ccAnm` hands `ccObj::Draw` (0x0013f220), and `draw.rs` draws those through
piney-world's model draw.

```
ccClump::Draw (0x0013f470): each node of objTbl in order
  Obj (ccstag 0x100): ccObj::Draw(node, 1.0)
  EffObj (0xe00): its ccEff's pos = its lwMatrix's translation; Draw(its +0x106)
ccAnm::Draw (0x001524d0): (its own ccCam: a view for the draw); ambient
  from the Anime chunk unless 0x80000000; each ccAnmIndex entry in order
  whose flag has bit 2: Obj: ccObj::Draw(obj, ccAnm.localtp (+0x88));
  EffObj: ccEffObj::DrawNoAnm
ccObj::Draw(obj, tp):
  no parent: lwMatrix = matrix; else ccCoord::_SetLWMatrix (0x00138380):
    from the topmost coordinate of the chain whose matCalcSW is set down,
    lw = parent's lw * matrix (VU0), the root's lw its matrix
  tp *= flag bit 0 (Obj2 succession bit 0) ? _GetTransparency (0x00138490:
    the localtp chain) : worldtp
  tp <= 1/128 (0x3c000000): nothing; else the model, and the shadow model
    when its switch is on
```

- `ccClump::Init` (0x0013c5e0) gives every node unit `matrix` and
  `lwMatrix` and parents it to the node its Obj names (the chunk's parent
  table), else to the clump; nothing in the effects touches the nodes, so a
  node draws at the clump's matrix times the unit matrix once per level
  (the same numbers; only a zero's sign can change).
- `ccClump::SetTransparency(t)` (0x0013d7d0) sets each Obj node's localtp
  and, without succession bit 0, its worldtp. Every `Obj2.succession` in the
  effect files is 0: every node draws at `t`.
- An object whose Model chunk is a five-word dummy (no meshes: `MDL_x100`,
  `MDL_x502` and the like) gets no model from `ccObj::Init`: nothing drawn.
- An animation draws each object a track names, ExtObj copies each their
  own instance (`drain`'s `ANM_xdhdref0` draws `MDL_ene_pl_10` six times),
  at the ccAnm's matrix times its parents' poses and its own, at
  `ccAnm.localtp` times its own transparency, which the effects' animations
  key (the `drain` spheres and bars fade object by object). A ccAnm drawn
  before its first `_AnimateForward` shows its objects unposed; the effects
  always step first.
- No clump `effectTbl` names has an Eff or ExtObj node (of its 110 clump
  rows, 104 name one-node clumps and 6 clumps with children); 49 of
  `PARTICLE.CCS`'s clumps are one `EFF_` each, used by nothing in
  `effectTbl`.

`nodes::clump(assets, obj, matrix, alpha)` and `nodes::anm(assets, obj,
play, matrix, alpha)` return `ObjDraw { obj, model, world, alpha }` in the
game's order; a clump's `world` is the game's bits, an animation's the VU0
product order over `piney_data::anim`'s poses.

## The arrival

Kite's act 13 (the arrival in a town or a field) calls `effTransfer(this)`
when its count reaches 30 ([the field game](field-game.md#the-arrival)), or
`effWarpTransfer(this)` at count 0 when `WORLD_MAN` +0x168 is set (its fades
then run from 20 to 40 instead of 50 to 70); act 12 (leaving) calls
`effTransfer` at count 1 and takes his body off the hit list
(`ccPlayer::AnimCtrl`, gcmn 0x0059a7f4, 0x0059a8ec, 0x0059a900). The
others who come and go by it: the walking PCs (`ccRtownPC::normalMode`
0x005085ec, 0x00508688, `eventMode` 0x00508a60), a merchant's `sysopeAct`
(`ccMerchan::sysopeAct` 0x005062f0, 0x00506430) and the party members
(`ccFellow::Action` 0x0041df20, 0x0041e074; `effWarpTransfer` 0x0041e060).

```
effTransfer(ch) (0x001ce020)     effect 4 (no object): targetPtr ch, posT its
                                 pos, lifeTime 12, temp[0] = 20 + its height
                                 (Kite: 180); ccSeOn3D(76, its pos)
  second dispatch (0x001c9194), by cnt:
    0, 3, 6    effTransferRing(ch, temp[0])
    10         new ccParticleGenerator(&particleGeneratorTbl[82], 0, 0, 0, 0):
               syncPosType 0, syncPos &ch->pos, offset (0, 0, temp[0], 0);
               startParticleGenerator
effTransferRing(ch, h) (0x001cdef0)  effect 3 (CMP_x032, a clump): lifeTime 40,
                                 scale (0, 0, 0, 1), offset (0, 0, h, 1),
                                 rot (0, 0, DEG2RAD((short)(rand() & 0x3f00)), 1),
                                 targetPtr ch, posT its pos
  first switch (0x001c4248):
    cnt < 10: scale = oneVector * (0.1 + 0.9 cnt / 10)   (w too: 0)
    cnt == 10: scale = oneVector
    FadeInOut(10, 5, lifeTime)
    pos = ch->pos; pos.z += offset.z
  second dispatch (0x001c90f0):
    cnt == 10: speed.z = (20 - offset.z) / (lifeTime - 10 - 5)
    cnt < lifeTime - 5: offset.z += speed.z
    rot.z = DEG2RAD(RAD2DEG(rot.z) + 2048)
effWarpTransfer(ch) (0x001ce120) effect -25 (no object): targetPtr ch, posT its
                                 pos, lifeTime 2, temp[0] = 20 + its height;
                                 ccSeOn3D(76, its pos)
  second dispatch (0x001c92a4): at cnt 0 the same generator as effTransfer's
                                 at 10; no rings
```

Three rings appear at the head three frames apart, grow over 10 frames,
spin 11.25 degrees a frame, sink to 20 above the feet over their next 25
frames following him as he moves, and fade out over the last 5. The arrival
draws three `rand()`s, all on the frames the rings are made; every slot is
free again after 49 frames.

In a town the port runs its own `ccEffectCtrl(0)` for these
(`piney-game`'s `town_fx::TownFx`): after the town's tasks it starts the
frame's transfers - Kite's (`World::take_kite_transfers`), the members'
(`ccFellow::Action`'s, `World::take_member_transfers`), the walking PCs'
(`PcEvent::Transfer`) - then steps the effects and particles on the town's
`rand()` and draws them, the characters followed through
`World::fx_chars`. So the Chaos Gate's leave shows each member's rings and
photons, and so does an arrival. A merchant's `sysopeAct` transfer is not
started yet.

## Hits and the numbers over characters

What the battle shows where a blow lands: the hit spark, ring and photons,
the protect gauge's break, the attribute guard and critical, the words
(CRITICAL, DYING, NO DAMAGE), and the damage and recovery numbers. Ported in
`crates/piney-effect`: `hit.rs` (the effects), `flyfont.rs` (the rising
numbers and the `flyFont` sprite they and the stacked numbers draw with),
`damupr.rs` (the stacked numbers). Every function below is checked bit-exact against the game's own code in
eemu ([Checks](#checks)), except where [Unknown](#unknown) says otherwise.

### Entry points

| function | VA | what | port |
| --- | --- | --- | --- |
| `ccHitMarkDisp(ch, attacker)` | gcmn 0x005714e0 | spark, ring, photons on `ch` facing `attacker` | `Effects::hit_mark` |
| `effHitMark(pos)` | main 0x001cc3e0 | `ccParticleHitMark(pos)` | `hit::eff_hit_mark` |
| `ccParticleHitMark(pos)` | main 0x001bc420 | a generator of `particleGeneratorTbl[1]` at pos | `particle::cc_particle_hit_mark` |
| `effHitRing(pos, rot)` | main 0x001cc410 | effect 131 | `hit::eff_hit_ring` |
| `effHitPhoton(pos, rot)` | main 0x001cc550 | controller -23 | `hit::eff_hit_photon` |
| `effProtect(ch, broken, kind)` | main 0x001cf250 | effects 19-24 | `Effects::protect` |
| `ccParticleAttributeGuard(ch, att)` | main 0x001bcc40 | `effAttributeGuard` (0x001cc890) once per character | `Effects::attribute_guard` |
| `ccParticleAttributeCritical(ch)` | main 0x001bca90 | `effAttributeCritical` (0x001ccb40), sparks, shake | `Effects::attribute_critical` |
| `ccParticleCritical(ch)` | main 0x001bc540 | sparks, word 151, shake | `Effects::critical` |
| `ccParticleDying(ch)` | main 0x001bc6d0 | sparks, word 150 | `Effects::dying` |
| `ccParticleNoDamage(ch)` | main 0x001bc8c0 | sparks, word 152 | `Effects::no_damage` |
| `ccInitFlyFont()` | gcmn 0x0051a8c0 | every `flyFontCtrl` timer 0 (from `ccThGameCtrl`) | `FlyFonts::init` |
| `ccEntryFlyFontNum(col, num, pos, sx, sy, ofsX)` | gcmn 0x0051a900 | a rising number (bosses 03 and 08 only) | `Effects::fly_font_num` |
| `ccCtrlFlyFont(still)` | gcmn 0x0051aa10 | the rising numbers' frame | `FlyFonts::ctrl` |
| `ccEntryFlyFontNew(kind, v, pos, ch, sx, sy)` | gcmn 0x0051afc0 | a stacked number or MISS | `Effects::fly_font` |
| `ccEntryFlyFontNewExp(kind, v, pos, ch)` | gcmn 0x0051aea0 | number + EXP | `Effects::fly_font_exp` |
| `ccEntryFlyFontNewLevelDown(kind, pos, ch)` | gcmn 0x0051af20 | LEVEL DOWN | `Effects::fly_font_level_down` |
| `ccEntryFlyFontNewMiss(pos, ch)` | gcmn 0x0051af70 | MISS | `Effects::fly_font_miss` |
| `ccDamUprStr::AddStr` / `CtrlAll` / `DrawAll` / `ResetAll` | gcmn 0x0051bb80 / 0x0051b780 / 0x0051bb20 / 0x0051b710 | the stacked numbers | `DamUprStr::add_str` / `ctrl_all` / `draw_all` / `reset_all` |
| `ccUprollStr::AddStr` / `Ctrl` / `Draw` | gcmn 0x0051b140 / 0x0051b530 / 0x0051b320 | one character's lines | `Uproll::add_str` / `ctrl` / `draw` |

`ccMenuCtrl::Disp` (gcmn, 0x0052170c-0x00521754) runs the numbers each
frame, after the enemy bars' labels and the protect marks: it makes the
menu's `flyFont` (+0xb4) the global `font` (0x00378954), calls
`ccCtrlFlyFont(still)` (`still` is `ccMenuCtrl` +0x18), then unless
`still` `ccDamUprStr::CtrlAll()` and, unless `compulsionGameOver`
(0x00378c74), `DrawAll()`. Port: `Effects::fly_fonts(host, still,
game_over)`. `ccUseItemRequest`'s party-annihilation wait loop (gcmn
0x0057bbc8, 0x0057bc84) calls `CtrlAll` alone each frame (the lines keep
rolling, undrawn): `DamUprStr::ctrl_all`.

### The hit mark

```text
ccHitMarkDisp(ch, attacker)          both must pass ccCheckTarget (gcmn 0x00519920)
  p  = ch.pos,       z + 0.5 ch.base.height
  q  = attacker.pos, z + 0.5 attacker.base.height
  at = normalize(q - p) * ch.base.width + p        (sceVu0Normalize: w 0, so at.w 1)
  effHitMark(at)
  ch->hitFlip (+0x84) = !hitFlip; side = DEG2RAD(hitFlip ? 4096 : -4096)
  ground = atan2f(a.y - c.y, a.x - c.x)              (the positions, z 0)
  effHitRing(at, (0, side, turn, 0))
     turn = ch == attacker ? ch.dirc.z : DEG2RAD(RAD2DEG(ground) + 16384)
  effHitPhoton(at, (DEG2RAD(-4096), side, turn', 0))
     turn' = ch == attacker ? DEG2RAD(RAD2DEG(ch.dirc.z) - 16384 if hitFlip else + 16384)
                            : hitFlip ? ground : DEG2RAD(RAD2DEG(ground) - 32768)
```

`hitFlip` is read and written nowhere else (no other byte access at +0x84
in main or GCMN.PRG; `ccChar`'s constructor leaves it); the port keeps it
per character in `Hits::flip`, starting clear.

Effect 131 (`effHitRing`: CMP_x037, in PARTICLE.CCS): lifeTime 15 (drawn
17 frames), pos and rot as given, zyxFlag 0 (XYZ turn order); its clump
`Duplicate(0x3800)`d and `ChangeClut(particleCcsAdrs[191],
particleCcsAdrs[190])`, i.e. CLT_x037 drawn as CLT_x037c1 (the port:
`Effect::clut_swap`, carried to `DrawRec::Clump::clut`). First-switch case
0x001c5ce4:

```text
scale.x = scale.z = 0.5 + cnt * ((2.5 - 0.5) / lifeTime)
scale.y = 0.4 + cnt * ((2.0 - 0.4) / lifeTime)
FadeOut(8, lifeTime)
```

Controller -23 (`effHitPhoton`, no object), second-chain case 0x001cba88:
on counts 0 and 1, five times:

```text
p = ccParticleSetup(102, &pos, 12, 0, 0)          (main 0x001c2940; none: next)
r = rand() >> 3
tilt = DEG2RAD(r & 0x7c0)                          (up to 10.9 degrees)
v = (sinf(tilt), -cosf(tilt), 0, 0)
v = RotMatrixY(DEG2RAD((short)((r >> 3) & 0xff00))) * v
p.dirc (+0x50) = normalize(v), w 1
p.dirc = RotMatrixZ(rot.z) * RotMatrixX(rot.x) * p.dirc, w 1
p.speed (+0x84) = 30 - 21 * ((rand() >> 3) % 101) / 100
p.velocity (+0x70) = p.dirc * speed (all four lanes)
p.fadeInD (+0x92) = 1024, p.fadeOutD (+0x94) = 113
p.gene (+0x04) = &hitPhotonDummyG (main .bss 0x003ff1f0)
```

and on count 2 endFlag. `ccParticleSetup`'s `Setup` (main 0x001bf0f0) may
itself draw `rand()` (0x001bf418, 0x001bf76c): the particle system's
([the particle system](particles.md)).

### Protect, attribute guard

`effProtect(ch, broken, kind)`: `kind` -1 is taken from
`ccCheckObjectSize(ch)` (gcmn 0x0042e120: 1 -> 2, 3 -> 1, 4 -> 0; any other
size leaves the effect id uninitialised, whatever `s0` held: the port
starts nothing). `broken` 0 starts effect 19 + kind (drain's ANM_xdhpros0,
m0, l0), whose second-chain case (0x001ca628) plays sound 79 at count 30;
any other `broken` starts 22 + kind (ANM_xdhpros1, m1, l1; no case) and
plays sound 80 at once. Both: pos = ch.pos, target ch, posT ch.pos,
posPtr &ch.pos; the animation plays to its end (the core's).

`ccParticleAttributeGuard(ch, attacker)`: both pass `ccCheckTarget` and `ch`
is not on `effAttributeGuardEntry[20]` (main .bss 0x003feeb0;
`effCheckAttributeGuardEntry` 0x001d9510) -> `effAttributeGuard`, and when
it made an effect `effAddAttributeGuardEntry(ch)` (0x001d9560, the first
empty entry). Effect 170 (ANM_x071): target ch, posT ch.pos, pos ch.pos z +
0.5 height, rot (0, 0, turn, 0) with the ring's `turn`, scale oneVector *
3.5 / 2.0 / 1.0 for sizes 1 / 3 / 4 (any other: the caller's `f20`; the
port 1), sound 99 at ch.pos. When its animation ends `ccEffect::Main`
(0x001c730c) takes ch off the list (`effDelAttributeGuardEntry`).

### The words

`ccParticleAttributeCritical(ch)`: `CheckCharAttribute(ch, 1)` (gcmn
0x005701d0, a host input) 0-7 picks, by jump tables, the word's CLUT row
(0x00375000: 237, 237, 237, 238-242 = CLT_x061, c1-c5) and the sparks'
texture row (0x003741b0: 209, 209, 209, 205, 44, 207, 206, 208); any other
value as 0. Then `effAttributeCritical(ch)`: effect 169 (EFF_x061), target
ch, posT ch.pos, offset (0, 0, 0.5 height, 0), lifeTime 50, velocity 10,
temp[0] 0, the ccEff's `ccTex` CLUT (+0x3c) = `particleCcsAdrs[row]`,
`SetRenderState(0, 0)` (TEST_1's ZTE off), layer `effSBL`. Then a generator
of `particleGeneratorTbl[59]`, syncPos &effect.posT, syncPosType 0, offset
(0, 0, 0.5 height, 0), pTexMod (+0x2c) the texture row,
`ccParticleCtrlAddGenerator` (the same as `startParticleGenerator`); and
`cameraShake(0, 2, 2, 0)` (main 0x00162cd0) when
`checkCameraShakeRange(ch.pos)` (0x00162f10: in the camera's 67.5-degree
cone, `ccCheckCameraDeg(pos, 12288)`, and nearer the active camera's eye
than 2000).

`ccParticleCritical`, `ccParticleDying`, `ccParticleNoDamage` start
generators on ch.pos (syncPosType 0) and a word:

| function | generators (row: z offset) | word | shake |
| --- | --- | --- | --- |
| Critical | 59: 0.5 h | 151 (EFF_x056) | as above |
| Dying | 60: 15 + h; 206: 0.5 h | 150 (EFF_x055) | no |
| NoDamage | 207: 15 + h; 208: 15 + h | 152 (EFF_x057) | no |

The word: `ccNewEffect(id)`, offset = ch.pos with z + 0.5 h (a point, not
an offset), lifeTime 50, velocity 10, temp[0] 0, `SetRenderState(0, 0)`,
layer `effSBL`; no target. `effSBL` (main .sbss 0x00378ad8) is the layer
`ccEffectCtrl`'s constructor makes at priority 60 with sysLayer's view.

First-switch cases 0x001c5f0c (169) and 0x001c5d90 (150-154):

```text
cnt < 3: the ccEff's scaleX, scaleY 0 (drawn, invisible)
else:    scaleX, scaleY 2.5; s = cnt - 3: FadeIn(3, s) while s <= 3, then FadeOut(5, lifeTime)
  169: if ccCheckTarget(target) posT = target.pos; point = posT + offset, w 1
  150-154: point = offset
  pos = ccTransPosW2CZW(point, panelCamDist)   (main 0x001633c0: normalize(point - activeCam.eye)
                                                * 1000 + eye, w 1; panelCamDist 0x00377eec)
  old = (short)temp[0]; temp[0] += 0x10000 / 5; new = (short)temp[0]
  pos.z -= velocity * sinf(DEG2RAD(new))
  the signs of old and new differ: velocity *= 0.7
  velocity < 0.5: velocity = 0
```

So the word sits 1000 in front of the eye on the line to the character
(drawn over everything, depth test off, on its own layer) and bounces
five frames a period, each bounce 0.7 of the last.

### The rising numbers: `flyFontCtrl`

`flyFontCtrl[16]` (gcmn 0x0072ebd0), 0x40 bytes each:

| offset | field | |
| --- | --- | --- |
| +0x00 | color | `ccSpriteColorTable` row |
| +0x04 | timer | 40 when entered, 0 free |
| +0x08 | num | not set for a miss |
| +0x10 | pos[4] | z + 140 when entered |
| +0x20 | str[16] | "" (draw num) or "MISS" (`ffMissStr`, main 0x00378228) |
| +0x30 | ofsX | pixels |
| +0x34, +0x38 | sx, sy | scale |

`ccEntryFlyFontNum` fills the first entry with timer 0 (none: nothing).
`ccCtrlFlyFont(still)`, each live entry in slot order:

```text
unless still: timer--
pos += (0, 0, 0, 0); pos.w = 1
(X, Y, Z) = sceVu0RotTransPers(sysLayer's view +0xd0 (world to screen), pos)
Z not in (0, 0x0fffffff): nothing drawn
x = fptosi(float(X) - 28672.0) / 16 + ofsX
y = (Y - 32768 + 3584) / 16 + (-3 (40 - timer)) / 2      (rises 1.5 px a frame)
x < -63 or x >= 576: nothing drawn
font: SetType(2); sx, sy, cx, cy *= the entry's sx, sy; r, g, b from the row;
      dx, dy = x, y; alpha timer 128 / 20 below 20, else 128; ctrl |= 0x10
str "": MakeSignedNum(1 / 2 / 3 / 4 digits by num < 10 / 100 / 1000, num)
else:   MakePacketStr(str)
```

Divisions round toward zero. `MakeSignedNum(clm, n)` (main 0x0015cde0) is
`sdec2str(min(clm, 126), n)` (0x0015cfc0: a sign column then `clm`
digits, leading zeros but the last blank, the sign - '-' or ' ' - just
before the first digit shown; a number with more digits keeps its low
ones, the sign column first) drawn by `MakePacketStr`.

### The stacked numbers: `ccDamUprStr`

A node (0x2d8, `new`) per character, on the list at `root` (main .sbss
0x00378c80; new nodes at the head; `CtrlAll` and `DrawAll` walk from the
head): +0x00 the list links, +0x08 ch, +0x0c a `ccUprollStr`:

| offset | field |
| --- | --- |
| +0x000 | `ccUprollStrLine line[16]` (0x2c each) |
| +0x2c0, +0x2c2 | lineNum, lineTop |
| +0x2c4, +0x2c6 | x, y (pixels) |
| +0x2c8 | dispSw |

`ccUprollStrLine`: +0x00 str[16], +0x10 color, +0x14 alphaCnt, +0x18 alpha,
+0x1a lx, +0x1c ly, +0x1e addly (1/16 px), +0x20 ftype, +0x24 sx, +0x28 sy.
The constructor (0x0051b0b0): every line empty, color 2, sx sy 1, the rest
0; dispSw 1.

The battle's calls (all font type 3, the big digits: byte 0x20 + n is
cell n of ` 0123456789MISEXPLVDOWN-`; `Int2StrFF(buf, v, 5)`, gcmn
0x0051add0, writes digit d as 0x21 + d, no leading zeros, 0x37 for a minus,
dividing by its table of powers of ten at 0x00650db0):

| call | string | colour | scale |
| --- | --- | --- | --- |
| `ccEntryFlyFontNew(kind, -1, ...)` | `ffstrMISS` "+,--" (0x006dfed8) | 0 | 1 |
| `ccEntryFlyFontNew(kind, v, ...)` | `Int2StrFF(v, 5)` | kind | 1.6 from 1000, 1.3 from 100, else 1 |
| `ccEntryFlyFontNewExp(kind, v, ...)` | `Int2StrFF(v, 5)` + `ffstrEXP` "./0" (0x006dfee0) | kind | 1 |
| `ccEntryFlyFontNewLevelDown(kind, ...)` | `ffstrLEVELDOWN` "1.2.1 3456" (0x006dfec8) | kind | 1 |
| `ccEntryFlyFontNewMiss(...)` | "+,--" | 0 | 1 |

The `pos`, `sx` and `sy` arguments are not read. Each goes to
`ccDamUprStr::AddStr(str, col, 3, ch, s, s)`: the node whose ch is `ch`
(null included), else a new one at the head; its `ccUprollStr::AddStr`;
dispSw 0 until `CtrlAll` places it.

```text
ccUprollStr::AddStr(str, col, ftype, sx, sy)
  old = lineTop; lineTop = lineTop < 15 ? lineTop + 1 : 0; lineNum = min(lineNum + 1, 16)
  new line: str (15 bytes at most), alphaCnt 24, alpha 128, color col, ftype
  the global font's SetType(ftype & 15) for the cell: w = fptosi(sx float(su << 4)),
    h = fptosi(sy float(sv << 4))
  lx = -(len w) / 2, ly 0, sx, sy; old line's addly = h + 16 - its ly if its ly is less;
  new line's addly 288
ccUprollStr::Ctrl()                         each line from lineTop back (wrapping)
  addly != 0: step = max(addly >> 2, 1); ly += step; addly -= step
  sum += ly; alphaCnt--
  alpha = alphaCnt < -8: 0 | alphaCnt < 0: (alphaCnt + 8) 128 / 8 | else 128  (the last two count as seen)
  sum >= 2561: stop (this line's alpha not stored)
  sum >= 1281: alpha -= (sum - 1280) 8 / 80; below 0: 0, else seen
  lineNum = the last line seen + 1
CtrlAll(), each node:
  no ch: nothing
  ccCheckTarget(ch) fails: Ctrl if lineNum, else the node is deleted
  ch->base->type & 2 and camID 1 and activeCamPtr +0x5c 1 (the party view):
    x = checkPartyMenberNum(base->id) 170 + 60, y = 360 - ((ccConditionIconNum(ch) + 4) / 5) 32, dispSw 1
  else point = ch.pos, and for a non-member z + 0.9 height and lift 16
    ccCalcTagPosChar(ch, (0, 0, 160), 1) != 1: dispSw 0
    else (X, Y, Z) = RotTransPers(point); converted as the rising numbers are when Z is in
      range (else the raw fixed point); x clamped to 26..486, y to 90..436, + lift; dispSw 1
  Ctrl
ccUprollStr::Draw(), when dispSw and lineNum, from lineTop back:
  SetType(ftype & 15); sx, sy, cx, cy *= the line's; ctrl |= 0x10; r, g, b of its colour
  y16 -= ly; dy = (y16 >> 4) - fptosi(font.sy) (y16 starts y << 4); dx = ((x << 4) + lx) >> 4
  alpha the line's; MakePacketStr(str)
```

`>> 4` here is an arithmetic shift (floor); the other divisions round
toward zero.

### Drawing the numbers

`flyFont` is `ccMenuCtrl` +0xb4: `new ccSprite`, `SetPrim(160, 0)` (a TF
SPRITE per cell, 160 packets), `CopyTex(fontTex)` (`xasc00::TEX_xasc00`,
256 high), the menu's layer (+0x58, priority 242, `SetFrame(0, 0, 512,
384, 256, 192, 1, 6/7)`: x 16 + 28672.5, y 16 + 29184.5 in 12.4). The
enemy bars' "ENEMY" labels are queued on it first in `Disp`; the numbers
follow; `Disp` later `SendPacket`s the whole queue as one group at the
front of the layer. The packet count and the 160 cap are shared with the
labels. `ccMenuCtrl::Disp` (gcmn 0x0052170c-0x00521754), after the labels
and the protect marks: `font = flyFont`; `ccCtrlFlyFont(menu +0x18 != 0)`;
unless menu +0x18, `ccDamUprStr::CtrlAll()` and, unless
`compulsionGameOver`, `ccDamUprStr::DrawAll()`.

`ccSprite::MakePacketStr(str, 0)` (main 0x0015aed0): the start is
`ApplyLayerScreenMatrix(dx + cx, dy + cy)` truncated; the cell is
`fptosi(sx 16)` by `fptosi(sy 16)`; each byte 0x21-0x7f draws cell `c -
0x20` of the grid (u = (cell % wi) su 16 + wu, v = 256 16 - wv - (cell /
wi) sv 16 - 1, the corner (u + su 16, v - (sv 16 - 1))), with ctrl 0x10
first a shadow (16, 16, 16, a) one unit right and down; any other byte
only moves on by the cell width; a quad wholly outside (0x7000..0x9000,
0x7200..0x8e00) is dropped; nothing past packetMax. dx moves on by sx a
byte. The colour is RGBAQ's words r, g, b and alpha times `transp` (1).
The port records each quad (`flyfont::Quad`, the packet's RGBAQ, UV and
XYZ2 words) and turns it into a primitive with
`piney_desktop::sprite::sprite_prim` (`flyfont::quad_prim`, `flyfont::send`).

Both fly-font draws scale `flyFont`'s cx and cy along with sx and sy
(they stay 0: nothing sets them), set ctrl 0x10 and never clear it, and
`ccUprollStr::AddStr` calls `SetType` on the global `font` whenever the
battle adds a number, so `flyFont`'s grid, colour, alpha and shadow bit
are left as the numbers set them for whatever draws with it next.

### From piney-battle's events

The runtime turns `piney_battle::event::Event`s (`on`, `by` resolved to
the characters' `CharRef`s) into the calls below (piney-game's `fx::event`,
for `Show::Rule` and the rules inside a Kite's, member's or enemy's
output). Until worklog 194 only the first ten rows were carried out: the
heals', cures', revivals' and the resistant shield's effects were ported
and tested in piney-effect but never started in play. `DispCondition`
(worklog 195) goes through the combat, which picks the condition
([battle](battle.md)) and keeps each character's effect number;
piney-game holds the effects.

| event | game call | port |
| --- | --- | --- |
| `HitMark { on, by }` | `ccHitMarkDisp(on, by)` | `fx.hit_mark(host, on, by)` |
| `FlyFont { on, kind, value }` | `ccEntryFlyFontNew(kind, value, pos, on, 1, 1)` | `fx.fly_font(Some(on), kind, value)` |
| `Protect { on, broken, kind }` | `effProtect(on, broken, kind)` | `fx.protect(host, on, broken, kind)` |
| `AttributeGuard { on, by }` | `ccParticleAttributeGuard(on, by)` | `fx.attribute_guard(host, on, by)` |
| `AttributeCriticalParticle(on)` | `ccParticleAttributeCritical(on)` | `fx.attribute_critical(host, on)` |
| `Critical(on)` | `ccParticleCritical(on)` | `fx.critical(host, on)` |
| `Dying(on)` | `ccParticleDying(on)` | `fx.dying(host, on)` |
| `NoDamage(on)` | `ccParticleNoDamage(on)` | `fx.no_damage(host, on)` |
| `DrainCtrl { from, to, kind, a, b }` | `effDrainCtrl(from, to, kind, a, b)` | `fx.drain_ctrl(host, from, to, kind, a, b)` |
| `AfterDrain(on)` | `effAfterDrain(on, 0)` | `fx.after_drain(host, on, 0)` |
| `HealSkill { on, sid }` | `effHealSkill(on, sid)` | `fx.heal_skill(host, on, sid)` |
| `Cure(on)`, `Sanity(on)`, `Resurrect(on)` | `effCure(on)`, `effSanity(on)`, `effResurrect(on)` | `fx.cure(host, on)`, `fx.sanity(host, on)`, `fx.resurrect(host, on)` |
| `ResistantShield { on, magic }` | `effResistantShield(on, magic, -1)` | `fx.resistant_shield(host, on, magic, -1)` |
| `DispCondition(on)` | `ccChar::DispConditionEffect` (gcmn 0x0056f950): the battle picks the condition number, then `killConditionEffect` / `deleteConditionEffect` of the old and `setConditionEffect(on)` | `fx.kill_condition_effect(host, ep)`, `fx.delete_condition_effect(host, ep)`, `fx.condition_effect(host, on, num)` |

piney-battle's other outputs that start effects (all started in play
since worklog 195; before, only the drain menu's, the items' heal, the
traps' and the boxes' were):

| output | game call | port |
| --- | --- | --- |
| `skill::Request::skill_start` `(who, a, b)` (`_ccSkillRequest`: `(target, stype == 2, 1)`, an item's `(me, 1, 0)`) | `effSkillStart(who, sid, a, b)` | `fx.skill_start(host, who, sid, a, b)` |
| `skill::ModifyEvents::started` (`ccSkillModifyCondition`) | `effSkillStart(ch, sid, stype == 2, 1)` | `fx.skill_start(host, ch, sid, stype == 2, 1)` |
| `kite::Out::SkillStart { sid, item }` (`ccPlayer::AnimCtrl`) | `effSkillStart(this, sid, item, 0)` | `fx.skill_start(host, kite, sid, item, 0)` |
| `fellow::Out::SkillStart { sid, flag }` (`ccFellow::Action`) | `effSkillStart(this, sid, flag, 0)` | `fx.skill_start(host, member, sid, flag, 0)` |
| `enemy_ai::Out::SkillStart(param)` (`ccEnemy::startSkill`) | `effSkillStart(this, skillParam, 0, 0)` | `fx.skill_start_param(host, enemy, type, param is skillTbl[1], 0, 0)`, `type` the row's +0x2c |
| `flow::MainOut::shock_waves` (`ccSkillCheckNote`, each 0x8002 note) | `effPhysicalSkillHitShockWave(tPos, type & 0xfc)` | `fx.shock_wave(host, t_pos, attr)` once a note |
| `drain::SideStart::Effect(ch)` (`DataDrainMenu` step 10) | `effSkillStartEffect(ch, 0, 1)` | `fx.skill_start_effect(host, ch, 0, 1)` |
| `drain::SideStart::Heal(ch)`, `item::Step::EffHeal(on)` (`ccUseItemRequest`) | `effHeal(ch, 1)` | `fx.heal(host, ch, 1)` |
| `drain::SideStart::Miss(ch)` | `ccEntryFlyFontNewMiss(ch's pos, ch)` | `fx.fly_font_miss(ch)` |
| `drain::SideStart::Exp { who, value }` | `ccEntryFlyFontNewExp(23, value, pos, who)` | `fx.fly_font_exp(who, 23, value)` |
| `drain::SideStart::AfterDrain(ch)` | `effAfterDrain(ch, 0)` | `fx.after_drain(host, ch, 0)` |
| step 11's level lost (`Show::DrainLevelDown`) | `ccEntryFlyFontNewLevelDown(23, pos, plw)` | `fx.fly_font_level_down(kite, 23)` |
| `item::Step::RemoveTrap(pos)` | `effRemoveTrap(pos, -1, -1)` | `fx.remove_trap(host, pos, -1, -1)` |
| `entry::Out::RemoveTrap { pos }` (`entryEnemyObject`: a treasure box) | `effRemoveTrap(pos, 103, 121)` | `fx.remove_trap(host, pos, 103, 121)` |
| `kite::Out::OpenBox { pos }`, `fellow::Out::OpenBox { pos }` (each twice) | `effOpenBox(pos)` | `fx.open_box(host, pos)` |

The runtime answers the new host inputs for these: `char_pos_p` (`posP`,
+0x50: `W2PPos` of `pos` unless given) and `affect_person` (+0x98, who
last struck the character) for the shield, and `char_type` for the
start's sound and the shield's size.

A level up (`CheckLevelUp` raising the level, piney-battle's `exp`) is
`fx.level_up(host, ch)`; a new form after a Data Drain
(`entryDrainEnemy`) `fx.after_drain(host, ch, -1)`; an attack spell's
request `fx.spell_request` ([the spells](#the-spells)).

Experience, level-down and drain misses come from the drain and
experience code (`ccEntryFlyFontNewExp`, `...LevelDown`, `...Miss`:
`fx.fly_font_exp`, `fly_font_level_down`, `fly_font_miss`). Each frame the
menu's `Disp` calls `fx.fly_fonts(host, still, game_over)` after the
labels are queued (with `fx.font.packet_num` set to the labels' count),
then sends `fx.font.take()` with its `flyFont`.

Kinds seen: 2 damage (row 2, red; -1 MISS in row 0), 3 SP lost (row 3), 5
SP gained (row 5), 20 HP gained (row 20), 19 and 23 experience, 23 level
down.

The host (`piney_effect::Host`) answers, beyond the core's: `check_target`
(`ccCheckTarget`), `char_type` (`base->type`), `party_slot`
(`checkPartyMenberNum(base->id)`), `condition_icon_num`
(`ccConditionIconNum`), `camera_id` / `camera_type` (`camID`,
`activeCamPtr` +0x5c), `object_size` (`ccCheckObjectSize`),
`char_attribute` (`CheckCharAttribute`); `Camera::world_screen` is
sysLayer's view's world-to-screen matrix. New events: `CameraShake([0, 2,
2, 0])`.

## Level up

`ccPlayer::Main` (gcmn 0x00598404) and `ccFellow::Main` (0x0041b718)
call `effLevelUp(ch)` (main 0x001cdde0) when `ccChar::CheckLevelUp`
(gcmn 0x0056d430) raised the level, unless the character's `actNum`
(`ccSpcChar` +0xee) is 14.

```text
effLevelUp(ch)   effect 2 (PARTICLE's EFF_x035, a sprite of kind 2)
                 target ch, posT its pos, lifeTime 80, velocity 10,
                 temp[0] 0, temp[1] 20 + its height (+0x18 of its row);
                 SetRenderState(0, 0): TEST's ZTE off, no depth test;
                 layer effSBL (priority 60)
first switch     cnt < 50: the sprite's scale 0 (nothing shows)
(0x001c60cc)     then scale 3; FadeIn(3, cnt - 50) up to cnt 53, then
                 FadeOut(5, lifeTime); while ch is listed posT = its pos;
                 pos = ccTransPosW2CZW(posT + (0, 0, temp[1]), 1000); the
                 bounce below
second chain     ch off the lists (ccCheckTarget) or condition.dead not 0 or
(0x001c8d98)     5: endFlag, nothing else; by cnt:
                   0, 15, 25   particleGeneratorTbl[222], [223], [224] on
                               ch's pos, offset (0, 0, temp[1], 0)
                   40          [225] and [226] on ch's pos (offset left as
                               the constructor's, (0, 0, 0, 1))
                   50          ccSeOn3D(84, posT)
```

The bounce (the words', [above](#the-words)): `temp[0]`'s low 16
bits are an angle stepped by 0x10000 / 5 (72 degrees) a frame as a 32-bit
add; `pos.z -= velocity sinf(DEG2RAD(angle))`; each time the angle's sign
turns, velocity is multiplied by 0.7 (0x3f333333), and below 0.5 it is 0.

`ccTransPosW2CZW(out, pos, dist)` (main 0x001633c0): the point `dist`
from `activeCamPtr`'s eye on the line to `pos`, w 1 (`panelCamDist`, main
0x00377eec, is 1000 and never written). So the word hangs 1000 in front of
the camera over the character and is drawn over everything (no depth test,
layer 60).

`effSBL` (main 0x00378ad8) is a `ccLayer` that `ccEffectCtrl::ccEffectCtrl`
makes at priority 60 with `sysLayer`'s view (0x001c33dc).

## Deaths

### The dying blow

`ccChar::CalcBattleDamage` (gcmn 0x0056d910, at 0x0056eb14): an attacker
with the dying condition whose `rand() % 100` roll is within it raises the
damage to 3/4 of the target's HP and calls `ccParticleDying(target)`
(main 0x001bc6d0): `particleGeneratorTbl[60]` on its pos with offset
(0, 0, 15 + height, 0) and `[206]` with (0, 0, height / 2, 0), both
through `ccParticleCtrlAddGenerator` (0x001c2a20, which
`startParticleGenerator` also calls); then `ccNewEffect(150)`, the word. The words' table under [the words](#the-words) has the
same.

### A foe's death

Nothing in the effect system marks an ordinary foe's death. `ccEnemy::
interruptThink` (gcmn 0x004353f0): HP 0 starts act 8 (dying: the race's
dying animation; `condition.dead` 2 and the conditions cleared at its first
frame); at its count 91 act 9 (dead): off the command lists, the kill's
exp (`ccExpDistributor`), the enemy book (`saveData` +0x68b7 by row,
+0x69f0 where), and `fadeFlag` 2 with `fadeCnt` 30; at act 9's count 31
`condition.dead` 3, and the next `ccEnemy::main` (0x00432cd0) hands the
enemy to `ccEntryCtrl::entryEnemyObject` and returns 1 (deleted).
`ccEntryObj::routine` (0x0042fa60) fades it each frame: `setTransparency`
less 1 / 30 until 0, `transparency` the same (ported as
`piney_world::merchant::EntryObj::routine`). `animEnemy` (0x00434560) and
`dispEnemy` (0x004348b0) only play and draw the body; the races' dust
(`ccEnemyDustCtrl::ctrl` 0x0043aa90, `ccEnemyEffDustRing`) follows the
animation frames of any act by the race's `ccEnemyDustInfo` tables. Bosses
have their own death effects (`ccBoss::BeginDeadEffect` 0x0045ee90,
`ccBossEffDead`), not covered here.

## Around a Data Drain

```text
who calls                               what
CalcBattleDamage, the HP drain          effDrainCtrl(attacker, target, 0, 5, 5)
  condition (gcmn 0x0056ebcc)           (after EntryAffect(9, half the damage))
CalcBattleDamage, the SP drain          effDrainCtrl(attacker, target, 1, 5, 5)
  condition (0x0056ecbc)
Influence (Kite, gcmn 0x0059b1cc) and   effAfterDrain(ch, 0), then
  ccFellow::Influence (0x0041c4d4) on   effDrainCtrl(drainer, ch, 0, 5, 3) and
  affect 13 (a boss drains a member)    effDrainCtrl(drainer, ch, 1, 5, 3)
ccEnemy::entryDrainEnemy (0x00435a94)   effAfterDrain(new form, -1)
  after ccThDrainEnemy's change
DataDrainMenu (0x005344b0..0x005346e0)  effAfterDrain(Kite, 0) with the
  side effects 23-27 (exp lost)         lost exp's number
ccRtownPC::eventMode (0x00508d4c)       effAfterDrain(pc, 0), sound 79
CalcBattleDamage, CalcReal, boss 5      effProtect(ch, sw, size): the protect
                                        gauge ([protect](#protect-attribute-guard))
```

An enemy's own affect 13 (`ccEnemyInfluence`) starts nothing: the drain of
a foe is the movie, `ccThDrainEnemy` (gcmn 0x00432000) and then
`effAfterDrain` on the new form.

### effDrainCtrl and the orbs

```text
effDrainCtrl(ap, bp, type, time, num)   (main 0x001d79f0)
  effect -24 (no object): param type, lifeTime (int)(float)time, flags num,
    target ap, posT ap's pos, temp[0] bp (a ccChar *), temp[1]
    (float)num / (float)time
  effect 154 (type 0) or 153 (type 1): the word over bp; the function
    returns it. Another type: the word's id is a register never set.
-24's second chain (0x001cbcac), every frame:
  k = (int)(temp[2] + temp[1]) - (int)temp[2]; temp[2] += temp[1]
  k > 0: effDrain(ap, bp, type, k) if both are listed; flags -= k either way
  flags <= 0: endFlag
effDrain(ap, bp, type, num)             (main 0x001d7c40), num times:
  effect 132 (type 0: EFF_x000, palette particleCcsAdrs[104] CLT_x000c3)
      or 133 (type 1: EFF_x002, palette [118] CLT_x002), scale 3 (the
      ccEff's +0x3c tex.clutChunk is the palette)
  lifeTime 240, target ap, posT ap's pos; pos = b = bp's pos, z + height/2
  a = ap's pos, z + height / 2; ang = normal2Angle(b - a) (main 0x00155210:
      (-atan2f(z, x x + y y), 0, atan2f(x, -y), 1))
  r = rand() >> 3
  d = (sinf(t), -cosf(t), 0, 0), t = DEG2RAD((short)(r & 0x1f00))
  d = RotY(DEG2RAD((short)((r << 8) & 0xff00))) d; d.w = 1
  d = RotZ(ang.z) RotX(DEG2RAD(-8192)) d; d.w = 1; rot = normalize(d)
  velocity = 33 - 3.3 (r % 101) / 100; speed = rot velocity
  temp[0] (short) 2048, temp[1] 1; texAnmPat = sn % patNum
  particleGeneratorTbl[227 + type]: syncPos the orb's pos, syncSW its
      temp[1] (the orb clears it as it ends, and a generator whose syncSW
      reads 0 dies: the trail stops)
effects 132/133's first switch (0x001c6280):
  age >= lifeTime - 1 or ap off the lists: endFlag, temp[1] 0
  count >= 31: temp[0] (short) += 48, at most 16384
  count < 3: pos += speed, w 1, pos = ccTransPosFW2LW(pos)
  else: t = FW2LW(ap's middle); p = FW2LW(pos); d = t - p
    |d| <= ap's width (+0x1c): endFlag, temp[1] 0
    d = normalize(d); axis = d x rot; c = d . rot; s = sqrtf(2 (1 + c))
    s == 0: q = (0, 0, 1, 1) sin(h), w cos(h), h = 0.5 DEG2RAD(32767)
    else:   q = (axis / s, 0.5 s)
    a = RAD2DEG(2 acosf(q.w)) as unsigned 16 bits, at most temp[0]
    q = (q.xyz / sinf(acosf(q.w)), 1) sin(h), w cos(h), h = 0.5 DEG2RAD(a)
    M = the rotation of q: n = x x + y y + z z + w w through the FPU's
        accumulator (mula.s, madd.s, adda.s), f = 2 / n (0 when n <= 0),
        the usual 1 - f(yy + zz), f(xy + wz) ... into a unit matrix
    rot = normalize(M rot); speed = rot velocity; pos += speed, w 1, FW2LW
```

The orb's sin and cos (of `h`) are double precision: `fptodp`, newlib's
`sin` (main 0x001274c8) and `cos` (0x00126ee0) with `__kernel_sin`
(0x001259b0), `__kernel_cos` (0x00124b88) and `__ieee754_rem_pio2`
(0x00123260) on the soft-float `dpadd` (0x0012a1b0), `dpsub`, `dpmul`
(0x0012a270), then `dptofp` (0x0012a990; `fptodp` 0x00129cc8). These round
to nearest like IEEE doubles, so `f64` arithmetic in the same order gives the
same bits. `acosf` is `__ieee754_acosf` (0x00123b38) on the FPU. So each orb leaves
the victim in a random direction, tilted, and turns toward the drainer by
at most 11.25 degrees a frame, then from its count 31 by 0.26 degrees more
each frame, up to 90.

### effAfterDrain

```text
effAfterDrain(target, size)    (main 0x001cd380)
  size -1: ccCheckObjectSize(target) (gcmn 0x0042e120, the entry's table
    row +0x70 for an enemy): 1 -> 2, 3 -> 1, 4 -> 0; every enemy row is 1,
    3 or 4 (another value leaves the id unset)
  effect 12 + size (DRAIN's ANM_xdhdref0/1/2) at the target's pos, target
  and posT it; no code of its own: ccEffect::Main steps the animation and
  ends the effect with it
```

`effProtect`, the gauge's break with the same file's animations, is under
[protect](#protect-attribute-guard).

## The magic portal

`ccMagicCircle` (gcmn gmcircle.cpp, 0x39f0 bytes over `ccGimmick`),
`gimmickTbl[15]` ("Magic Portal", file `XMAGCIR.CCS`, palette
`CLT_x031c2`). It is an entry object, not a `ccEffect`: `ccThEntryCtrl`
runs it (its `routine`, then `main`) and it draws on the effect layer
itself.

```text
ccEntryCtrl::entryMagicCircle(ep) (gcmn 0x004313f0), from entryObjectCheck
  ep.pos z = ccLandHitCheck(pos, 0x20000002) + 250; gimmickTbl[15]'s entry
  (0x0061e7a8) with ep, its func ccEntryGimCircle (0x00455820): new
  ccMagicCircle; initObject; onto the circle list; g_entCtrl +0x1c += 1
ccMagicCircle::ccMagicCircle (0x00455900)
  pos, dirc from ep; posP W2P(pos); actNum, actCnt 0; gimId ep.id and its
  base parameters; clump CMP_xmagcir0 (fog off), ccEntryChangeCLUT
  (0x0042e670: ccClump::ChangeClut(CLT_x031c2, MAT_clut)); ccAnm with
  ANM_xmagcir1 (the idle loop); partFlag 0; destFunc ccDestMagicCircle;
  deleteCmnd(1)
ccMagicCircle::main (0x00455b60)
  actNum 0 and freezeFlag (routine: Kite beyond 10000): the 128 sparks'
    ccEffs deleted, partFlag 0; return 0
  partFlag 0: each spark status 0 and a new ccEff (Init(EFF_xmagpat1, 0));
    partFlag 1, partTop 0
  posP = W2P(pos); pos = P2W(posP)
  act 0  ccCheckTarget(plw) and plDist <= 3000: ccSeOn3D 216 and 217 at
         pos; act 1
  act 1  after 32 frames: SetAnm(ANM_xmagcir2), act 2, destFlag
  act 2  after 22 frames: ccSeOn3D(215), ccEntryCtrl::entryCircleObject;
         act 3
  act 3  when the animation's last step ended (anmFlag): act 4
  act 4  after 66 frames: entRoot set -> saveData +0x6864 += 1 (to 10000);
         g_entCtrl +0x1c == 1 -> ccStartThread(ccThDfComp, 33, 4096) and in
         a field (game +0x14 == 1) +0x6866 += 1, in a dungeon (2) +0x6866
         when GetFieldType() == 4 and game +0x28 == 0, else +0x6868
         (each to 10000); return 1 (the entry control deletes it)
  createPart (0x00455710): m = unit with the pos as translation; act 0:
    setPart(1, m, 3); 1: setPart(2, m, actCnt / 4 + 1); 2: setPart(4, m,
    8); 3: setPart(3, m, 2) while actCnt < 16
  setPart(type, m, n) (0x00455510), n times: the first spark with status 0
    from partTop round (partTop unchanged), else the one at partTop
    (partTop += 1, wrapping at 128): status type, anmFlag 0, radCnt 0,
    cnt 0, effAnmPat 0, transparency 1, mat m, mcActCnt actCnt; its ccEff
    at (0, 0, 0, 1), unturned, colour 0x808080
  every spark with a status: ccMcPart::main
  act 0 and plDist >= 7000: return 0
  anmFlag = _AnimateForward(frameSpd)
  dispSW (routine: within 7000) and ccCheckCameraDeg(pos, 12288):
    t = ccGetCameraTransparency(pos, 0, 0, 7000, 600) setTransparency
    (0 at 144 from the active camera's eye rising to 1 at 225, 1 to 7000,
    to 0 over 600 more); SetActiveLayer(3) (priority 20); the anm at
    SetMatrix_PosRotZYX(pos, dirc), transparency t, Draw; then each live
    spark: t = min(1, max(0, t * its transparency)) - the product runs
    on from spark to spark - and ccEff::Draw(effAnmPat); SetActiveLayer(0)
ccMcPart::main (0x004548e0)
  eff.pos = ccTransPosFW2LW(eff.pos) (from Mutation on, unchanged without a
  player); at cnt 0: eff.pos = mat's
  translation (w 1), mat = RotZ RotY RotX of three DEG2RAD((short)ccRand())
  (x drawn first) and by status:
    1  life 120, scale 5; rnd = |ccRandF(0.2)|
       then radCnt += 838 (16-bit angle); speed = 6 sinf(radCnt); eff.pos +=
       mat (speed, 0, 0, 1) + (0, 0, 6 sinf(DEG2RAD((short)(1383 cnt))), 0);
       scale -0.04 while above 1
    2  life 10, scale 5; mat's random turn m; eff.pos = translation +
       m (400 - 6 mcActCnt + ccRandF(20), 0, 0, 1); mat = m
       then radCnt += 1638; v = (-18 sinf(radCnt) (20 x 0.9), 0, 0, 1);
       speed = 0.072 cosf(radCnt) wrapped into -pi..pi; mat = RotZ RotY RotX
       (speed each) mat; eff.pos += mat v; scale -0.28 while above 1
    3  life 30, scale 5, speed 30
       then radCnt += 582; speed -= sinf(radCnt); eff.pos += mat (speed, 0,
       0, 0) + (0, 0, -4, 0); scale -0.06 while above 2
    4  life 120, scale 5
       then radCnt += 758; speed = 8 sinf(radCnt); eff.pos += mat (speed,
       0, ccRandF(1.2), 1); scale -0.03 while above 1
  fade (0x00454840): every fourth frame (cnt & 3 == 0) the colour's r, g, b
    bytes down by one to 0; the alpha byte up by one every frame, wrapping
    from 255 to 0 (ignored by the draw)
  cnt += 1, anmFlag once life < the old cnt; effAnmPat += 1, 0 at patNum;
  anmFlag: status 0
```

`ccRand` is `genrand` (main 0x001d9620, the Mersenne Twister of
`piney_world::mt`); `ccRandF(x)` (0x001d9a90) is `x * (int)genrand() /
2^31` in double precision. The spark's `ccEff::Init(chunk, 0)` leaves fog
off. The additions `eff.pos += mat (.., 1)` also add 1 to w, which the next
frame's `ccTransPosFW2LW` resets.

**What the player sees.** `ccAnm::Draw` draws each of the portal's
objects at its own transparency (the Anime's alpha track, `localtp`) times
the anm's, and none at or below 1/128. The idle loop (`ANM_xmagcir1`, 91
frames) keeps the sphere (`OBJ_xmagcir1`) and the two rings at 0.9, the
ball (`OBJ_xmagball`) at scale 0. The opening (`ANM_xmagcir2`, 111 frames,
once) flares the ball to 0.98 at frame 30 and out by 60, grows the rings to
twice their size and shrinks them to nothing by 100, and fades the sphere
from 0.9 to 0 between frames 60 and 110; `OBJ_xmagcir0`, at 1 throughout,
has no mesh. So the monsters come out at the opening's frame 22 (act 2's
end), the portal is last drawn at frame 108, about 86 frames later, and
act 4's 66 frames pass with nothing of it drawn but the last sparks before
the entry control deletes it. The port had the circle's objects all at the
portal's own transparency, which left the sphere and rings up, full, until
the deletion: in a field that is about when the battle mode ends.
`portal::MagicCircle::render` now draws through `nodes::anm`, checked
against the game's `ccAnm::Draw` below.

## The attack spells

A spell is a `ccSkill` (gcmn skill.cpp, 0xb0 bytes) on `SkillEntryTop`;
`ccThSkill` (priority 82, after `ccThEffect` at 80) runs each one's
`ccSkill::Main` (gcmn 0x005731d0) and deletes it when Main answers non-zero.
For a magic skill Main jumps through the table at gcmn 0x006e1210 (on
id - 150) to the spell's element system, which starts the effects and makes
the damage calls:

| system | VA (gcmn) | skill ids | what it does |
| --- | --- | --- | --- |
| `FallSystem` | 0x00577a30 | 193-196 soil, 225-228 fire, 257-260 thunder, 273-276 dark | meteors (`effMeteoFireBall2`), a `ccSkillDamage2` as each lands; level 3+ `ccFallElementGenerate` |
| `TornadoSystem` | 0x00577540 | 197-200 soil, 209-212 water, 229-232 fire, 241-244 wind, 261-264 thunder | smoke, rings, nine `ccSkillDamage`s; level 3+ `ccSkillTornadeElementsGenerate` |
| `ConvergenceSystem` | 0x00578060 | 213-216 water, 233-236 fire, 245-248 wind, 265-268 thunder, 277-280 dark | pieces fly in to the target, one burst and `ccSkillDamage`; level 3+ `ccConvergenceElementGenerate` |
| `UpheavalSystem` | 0x005787c0 | 201-204 soil, 217-220 water, 249-252 wind, 281-284 dark | pillars burst up round the target, `ccSkillDamage2`; level 3+ `ccUpheavalElementGenerate` |
| `SummonsSystem` | 0x00579000 | 205-208 soil, 221-224 water, 237-240 fire, 253-256 wind, 269-272 thunder, 285-288 dark, 289-294 | a ring and a creature over the caster, `ccSkillDamage2`; level 3+ `ccSummonsElementGenerate` |

The level (1-4) is the id's place in its element's four (`id - base`, set
at count 0); Summons with no element: 291-294 give 1-4, 289 and 290 1.

### ccSkill, as the systems use it

```
+0x00 int ID              +0x24 skillType (row +0x2c)
+0x08 ccSkillParam *param +0x28 tType (the target's parameter row +0x08)
+0x0c bits 0-3 type (0 own skill, 1 item, 2 item through the caster),
      bits 4-5 endFlag (1: over; Main then answers 1)
+0x0d bit 0 holdFlag      +0x30 cPos, +0x40 cDirc, +0x50 cHeight (caster)
+0x10 int level           +0x54 short acFlag (the damage calls' `Rs`)
+0x14 short count         +0x60 tPos (target)
+0x16 trigger, +0x18 step, +0x1a effNum (shorts)
+0x1c int atkCnt, +0x20 int tempCnt
+0x70 ccChar *creator, +0x74 ccChar *target
+0x84 ccEffect *effPtr[8], +0xa4 ccEffectElement *m_effElm
```

`count` is what Main passes before its own increment. "The end" below is
`endFlag = 1` (usually with `holdFlag` cleared); "released" is the caster
of its own skill (type 0 or 2) freed to act again: its `skillID` (+0x7c)
and `skillStatus` (+0x7e) cleared.

From Mutation on the `ccSkill` is 0xc0 bytes (`_ccSkillRequest`, MUT gcmn
0x00598000, news 192; OUT's and QUA's too). A vector at +0x70 moves
`creator` and all after it 0x10 on: creator +0x80, target +0x84, effPtr
+0x94, `m_effElm` +0xb4. The request copies the target's position into
+0x70 beside `tPos` when the target is on the lists (MUT 0x005981d4).
`Main` (MUT 0x00598980, Infection's up to the offsets) refreshes only
`tPos`, so +0x70 keeps where the target stood at the request (the port's
`Spell::t_pos_req`). The systems aim at it, the elements still at `tPos`
([from Mutation on](#from-mutation-on)).

Every system starts on a target on the command lists (`ccCheckTarget`)
with `effMagicAttackSign(tp, atr)` (main 0x001d1090): two generators on
the target's pos by element (soil 136/140, water 130/137, fire 131/139,
wind 133/138, thunder 132/140, dark 134/141; the first 20 up). No target:
the end at once, the caster released.

### FallSystem

```
count 0   the sign (fire also startParticleEffect(target, 38)); caster
          released; tempCnt = FallSystemLevelTbl[level-1] (main 0x00378260:
          1, 2, 3, 4 meteors)
from 20   while step < that: every fsDelay[level-1] frames (main
          0x00378268: 0, 20, 17, 15; level 1's 0 is a divide by zero, which
          on the EE leaves the count: a meteor at 20 only): a meteor into
          effPtr[effNum++], step on, ccSeOn3D(57, tPos)
          its place: a random point within the target's width (x1.6 under
          200), 500 over its head, held at 890 when area 2 or a town field
from 21   each effPtr whose effect has ended: ccSkillDamage2(creator,
          target, tPos, tType, param, &acFlag, ID), tempCnt - 1; none left
          and all thrown: the end
```

`effMeteoFireBall2(p, t, atr)` (main 0x001cd4e0): the meteor (soil 87,
fire 6 with its flame 5 linked, thunder -14, dark 88), life 300, falling
from 20 a frame by 3.2 more each frame to the land under it
(`ccLandHitCheck2(p, -(500 + z), 0x20000000)`, else the target's z), its
smoke generators (`et[4][3]`, main 0x003401d0) following it until it lands
(their `syncSW` is the meteor's `flags`). The landing: `effSkillBreakSE`,
a shake (0, 1, 10, 0) in the camera's range, `effSmokeRock` (soil, fire:
9 rocks at 45) or `effDarkSmoke` (dark), `effFlareRing(p + 20 up, n)`,
fire's sparks (generators 91, 93). The thunder meteor -14 instead drops a
`ccThunderBoltElement` (`skillThunderEff`, main 0x0033fcc0) at its count 20.
Id 0 (`ANM_x300`, an animation of the fire meteor) shares the meteors'
cases of both passes (first switch 0x001c41a0, second chain 0x001c8900,
turning as 6 and 87 do), but nothing makes it.

Levels 3 and 4: `ccFallElementGenerate` (gcmn 0x004fef30) makes
`ccFallElement` (ctor 0x004e9610, Main 0x004e9b30: 8 or 15 meteors - the
fire, rock, dark or ice draw elements - spiralling down onto the target
10 frames apart, each bursting as `effExplode3` and `effRadiateSomething2`,
the hits 5 frames after 0, 10, 20 (and 30)) or for thunder `ccThunderFallElement`
(ctor 0x004ea890, Main 0x004eae10: 3 or 8 `ccThunderElement` bolts from
2000 up, 5 frames apart, down to the land met by `ccHitCheckLM2`); the
system ends once the element is deleted.

### TornadoSystem

```
count 0     level = id - (196, 208, 228, 240, 260); level 3+: the tornado
            element (below)
20          effSkillTornadeSmoke(tPos, 0, atr, level): generator 196, 40 up
30          ccSeOn3D(sndcode[level-1], tPos) (gcmn 0x00651920), the rings
            (effSkillTornadeRingsPos), the caster released
35 .. 75    every 5: ccSkillDamage(creator, target, param, &acFlag, ID) on
            a target on the lists and not down, else ccSkillDamage(creator,
            tPos, tType, param, ID) round tPos
40          in the camera's range: noise 20, cameraShake(0, 2, 20, 2)
after 75    the end
```

The rings: one 89, two 90, four 91 (life 60, `param` level - 1),
re-coloured per element (`Duplicate`, `ChangeClut` of the CLUT
`particleCcsAnmTbl` names; soil 181/223/216, fire 176/218/211, wind
178/220/213, thunder 177/219/212 over 175/217/210; water keeps its own),
their growth, rise and turn from the tables at main 0x0033ff50 (89),
0x0033ff90 (90, by flags), 0x00340030 (91) and the turns at gp 0x00377fa8
and 0x00377fb0. From level 2: the tornado's generator 191 + level with the
`skillTornadeSmokePFF1-3` force fields (main 0x0033fb40, 0x0033fbc0,
0x0033fc40, 0x20 a level), and the flying objects (effect -16 spawning
92-101, `effSkillTornadeObjPos`) or for thunder the bolts (effect -17,
`tornadeThunderTbl` main 0x0033fdb0: (count, bolts) lists; each bolt a
`ccEffect2` with a `ccThunderBoltElement`, `skillThunderEff2` main
0x0033fd00).

```text
rings 89 / 90 / 91 (first switch 0x001c5198 / 0x001c5320 / 0x001c5568):
  pos = posT (temp[0] set) or pos, plus offset; w 1
  90, 91: z up by min(cnt, 30) rise / 30
  rot.z on by turn / 30 a frame (89: 16384; 90, 91: +-16384 by flags)
  scale x, y from a to b and z from c to d over 30 counts (and on past
    30: the counts are not clamped)
  FadeInOut(5, 25, lifeTime)
effSkillTornadeThunderPos(pos, n)  (main 0x001d5b20) effect -17, no object:
  life 30, level n, posT pos, param 4 (n - 1) + rand() % 4; ccSeOn3D(68, pos)
-17 (second chain 0x001cae60): tornadeThunderTbl[param] is a list of
  (count, bolts); when cnt reaches the flags-th pair's count, that many
  bolts at P2W(W2P(posT) + offset), and flags on
  each bolt: a ccEffect2 (id 1) with new ccThunderBoltElement(p, 1, 250,
  abs(ccRand() & 7) + 1, skillThunderEff2)
ccThunderBoltElement(vec, time, ran, num, dat):
  Time, BottomRange = ran, Num; from dat the angles and scales (fptosi),
  mode, EFF_SW (flg, flg2), RndPoint (built-in ones for no dat); CenterPos
  vec; CMP_x012 of particle.ccs, duplicated and re-coloured by mode
  (CLT_x012c1 for mode 1); SetBreakPoint: each of the Num strokes
  abs(ccRand() & 7) + 5 segments; UnitPoint: each stroke's start
  BottomRange - ccRandF(BottomRange / 10) out at ccRandF(pi) about z
ccEffect2::Main (0x001cc290), ids 0 and 1: the bolt's Draw each frame
  until its EndFlg, then the bolt deleted and the slot freed a frame after
Draw (0x00501470): for each stroke a scale 2 + |ccRandF(2)|, then for each
  segment a turn ccRandF(pi) about z and a tilt DefaultAngle +
  |ccRandF(RandAngle)| about x: the clump at the stroke's point scaled
  (scale, 0.222, scale) along the segment, the next point 200 on; after
  Time frames m_delFlag and EndFlg
```

Levels 3 and 4: `ccTornadeElement` (gcmn 0x004ff0d0; Main 0x004ff3f0,
`_Level3` 0x004ff4a0, `_Level4` 0x004ff8b0) makes three (four) tornados
300 from the target a third of a turn apart, on the counts of
`START_1`/`START_2`/`sndcode` (gcmn 0x005ed760.., 0x005ed790..), and makes
the damage calls itself every 5 frames; the system ends once the element
is deleted:

```text
START_1[k]   (level 3: k 0-2; level 4: only k 3) the smoke at tPos +
             m_offsets[k], and a ring (mode 195) at tPos + m_offsets[m_index]
START_2[k]   (k 0-2; level 4 0-3) sndcode by level (level 4: the fourth's
             sound for the last) at P2W(W2P(tPos) + m_offsets[m_index]), and
             the rings (effSkillTornadeRingsPos with the offset, level 2;
             level 4's last level 4), m_index on
35 .. 75     every 5 from START_2[1] + 5 (level 4 START_2[3] + 5):
             ccSkillDamage on the target, else round the skill's tPos,
             while the skill runs; after: m_delFlag
START_2[0] + 10  in range of the camera, noise 20 and a shake (level 3
             (2, 2, 20, 2), level 4 (0, 2, 20, 2))
```

### ConvergenceSystem

```
count 0   effPtr[0] cleared; level = id - (212, 232, 244, 264, 276)
20        tPos = the target's pos; effPtr[0] = effSkillChargeObject(target,
          type, level); ccSeOn3D(63, tPos)
40        atkCnt 1; the controller's temp[0] = 1 (go); the caster released
45        ccSeOn3D(64, tPos)
then      once the controller's temp[0] >= 2 (a piece is in), once: at the
          target's middle effRadiateSomething(p, (-90 deg, 0, 0), 30, atr,
          radiate[level-1]) and the element's effSmoke* (water Ice v 30 s
          0.5, fire Sparks 2, wind Leaf 1, thunder Electric 1, dark Smoke
          2; smoke[level-1] pieces), ccSkillDamage, in range noise 10 and
          cameraShake(0, 2, 10, 0), water ccSeOn3DNote(66, tPos, 52) else
          effSkillBreakSE; tempCnt = wait[level-1] + 1; the end once
          temp[0] >= tempCnt
```

Tables (gcmn): radiate 0x00651930 (6, 8, 10, 12), smoke 0x00651940 (6, 8,
10, 12), wait 0x00651950 (8, 16, 24, 32); pieces by level main 0x003401c0
(8, 16, 24, 32).

The controller -18 (`effSkillChargeObject`, main 0x001d5da0; second chain
0x001cb170), life 200 on the target: each frame temp[2] (a float) grows by
max(1, n / 20) and a piece is made for each whole step (temp[1] counts
them; temp[3] set when all n are made); then it ends once temp[0] > n; two
frames before its life ends temp[0] = 99 (everything goes).

A piece (`effSkillChargeObj(tp, f, type, &temp[0], cnt / 2)`, main
0x001d5eb0; ids water 102-105, fire 106, wind 107-110, thunder 111, dark
112): at f = 100 + 15 cnt (+-10) from the target's middle at a random
bearing and tilt, 50 up; life 60, velocity -40, speed.z `effIV` (main
0x00377fb8, -4) slowed by `effSR` (0x00377ee8, 0.8) from count 7, still
from 20 (first switch 0x001c58f0). Once the controller says go and its own
wait has run out it turns to the target's middle and flies, velocity + 10
a frame; within |velocity| of the middle (or the target gone, or a frame
before its life ends) it adds 1 to the controller's temp[0] and ends
(second chain 0x001cafdc). The pieces keep a pointer to the controller's
temp[0].

`effRadiateSomething(p, r, v, atr, n)` (main 0x001d4cd0): n pieces life 25
thrown out like `effSmokeRock`'s rocks: soil 134-137, water 114-117 (half
size), fire 118, wind 119, thunder 120 (one random pattern for all; CLUT
151, 153, 145 into the `ccEff`'s `ccTex`), dark 121. `effSmokeIce/Sparks/
Leaf/Electric/Smoke(p, r, v, s, n)` (main 0x001d6410, 0x001d6750,
0x001d6a70, 0x001d6db0, 0x001d7060): n pieces (122-124, 126, 127, 128,
129) life 60, speed and size shares of v and s, each with generator 85
following it (pTexMod 110, 151, 1, none, 31); the leaves' sprites turned at
random and fogged. All of 114-129 and 134-137 tumble and bounce with the
rocks (first switch 0x001c4034, second chain 0x001c7dd0):

```text
first switch (0x001c4034; ids -15, 9-11, 15-18, 25-28, 42-55, 66-78,
114-129, 134-149, 155-163), every frame:
  rot.x on by rotSpeed[0], rot.z by rotSpeed[2]; pos on by speed (four
  lanes); below z -500 (flags not yet 3): flags 3, cnt and age 80; w 1;
  until flags bit 0, speed.z down by 2
second chain (0x001c7dd0; ids -15, 9-11, 25-28, 114-129, 138-149):
  falling faster than 3, one frame in four (cnt & 3 == sn & 3): the land
  under it (ccLandHitCheck2 down by 20 more than the fall); on a hit (or
  already bouncing) z onto it and a bounce; otherwise, bouncing (flags & 3
  == 1), a bounce every other frame
  a bounce: speed x, y by 0.8, z by -0.5; x, y below 4 and z below 8 (bit
  0 then) to 0; the spins by 0.8, below 16 to 0; all still: flags bit 1,
  cnt and age life - 10
  FadeOut(10, lifeTime)
```

Levels 3 and 4: `ccConvergenceElement` (ctor 0x004eb250, Main 0x004ebc70,
`Shock3`, `Shock4` 0x004ecad0): 8 (level 3) or 24 (level 4; the table at
gcmn 0x005ed570) pieces of the element's kind (ice, fire ball, plasma ball, dark
bat, branch) growing out round the target and flying in, 5 frames apart by
threes; level 4 then gathers them into one blast (`effExplode3` growing,
lightning for thunder).

### UpheavalSystem

```
count 0   level = id - (200, 216, 248, 280)
20        (level 3+: the element) the ground generator (soil, wind 144;
          water 145; dark 146) at tPos, offset 40 up
35-41     every 2: pillar 1-4 (level 2 also 55-61: 5-8), soil 29 + n % 4,
          water 33 + n % 4, wind 37 + n % 4, dark 41; life 80, 150 (1-4) or
          200 (5-8) out at pillarDegTbl[n-1] (gcmn 0x00651960), turned
          (deg + 49152)
35        the caster released
49 (and 69 for level 2)  ccSkillDamage2 round tPos, ccSeOn3DNote(56,
          tPos, 67), in range cameraShake(0, 2, 10, 0)
the end   level 1 at 71, level 2 at 91
```

A pillar (first switch 0x001c4468): height (offset.w) -500 rising at 45,
-1.1 a frame to count 14, then back by half to 17, still to 27, then down at
10; placed at offset plus its turn applied to (0, 0, height); FadeInOut(6,
55). At count 14 it bursts: `effUpheavalFragment` (main 0x001d12a0: 6
rocks 42-45, ice 46-49, a leaf 50, dark 51) and `effUpheavalFlash` (main
0x001d1740: 6 flashes 52-55), thrown up at 40; the flashes' and some
fragments' `ccEff`s have their pattern count cut to 0-3 and their CLUT
changed (`ccEff::ChangeClut(new, old)`, main 0x0013bb20: only while it is
`old`). 42-49 bounce (0.75 a bounce; second chains 0x001c9b44, 0x001ca0a8),
50-55 FadeOut(5) (0x001ca60c).

Levels 3 and 4: `ccUpheavalElementGenerate` (gcmn 0x005007e0) makes the
element's manager (`upheaval::UpheavalMngr`), which raises its pieces in
waves 2 frames apart and hits (`ccSkillDamage2` while the spell is on
`SkillEntryTop`, `ccSeOn3DNote`, a shake) as each wave comes up:

| manager | Main | waves | pieces |
| --- | --- | --- | --- |
| soil `ccSoilUpheavalMngrElement` | 0x004f0d30 (levels 3 / 4 0x004f0db0 / 0x004f1000) | 35, 55, 75 (95), hits 49, 69, 89 (109), over at 111 (131) | `ccSoilUpheavalElement` (CMP_x101a-d, 250-500 out, rising from -1170 and bursting at 14 with fragments, flashes, `effRadiateSomething2` and a ring 202) |
| water `ccIceUpheavalMngrElement` | 0x004ee950 | 32 spikes (level 4: 128 round a great spike that grows over 15 frames with a flash and a mist generator), raised 11 (43; 42 from Outbreak on, OUT gcmn 0x005067e0) at a time; `Delete` (0x004ee710) throws 32 pieces of ice and 5 rings 161 | `ccIceElement` 6 / 8, held by the manager |
| wind `ccTreeUpheavalMngrElement` | 0x004efd60 | as soil with `ccTreeUpheavalElement` (CMP_x401a-d_1, 450-700 out); level 4 first raises CMP_x407_1 with its roots ANM_x403 and starts the leaves, lowered at 131 | |
| dark `ccDarkUpheavalMngrElement` | 0x004edce0 | 20, 40, 60 (80), hits 80, 100, 120 (140), over at 126 (146); at the model 500 under the target (`ccHitCheckLM2`) | `ccDarkHandElement` (ANM_x605, 150-400 out) |

The pieces' Draws: the rock and the tree (gcmn 0x004fc7c0 / 0x004fcee0) at
`PosRotZYXScale`, the height from -1170 rising at 100 slowing by 1.1 a frame
to count 14, back by half to 17, still to 27, then sinking at 10, fading in
by 1/8 a frame; at 14 the burst (`effUpheavalFragment` 2 and
`effUpheavalFlash` 6 at 40, `effRadiateSomething2` 4 at 45; the rock also a
ring 202 under it); out by 0.1 a frame from 28, then `m_delFlag`. The hand
(0x004f8de0): from -500 at 45 (all three speeds), the same slowing and
bounce, `ccSeOn3D(56)` at 17, still to 60, then sinking; in by 1/6, out by
0.1 from 14, `m_delFlag` once gone after 61.

### SummonsSystem (and 289-294)

```
damage at 52, end at 62 (289: 150, 190; 290: 115, 125)
count 0   ccSeOn3D(62, tPos) and the sign; level as above
30        ccSeOn3D(62, tPos); 290: effSBLockon at the target's middle;
          effSummonsRing(cPos, atr, level, type) and
          effSummonsElement(cPos, cHeight, cDirc.z, atr, type) (type 1 for
          289, 2 for 290); 289: effTCDrillMissile(cPos, tPos, by size)
289, 290  ccSkillDamage2 at the damage; the end at the end (the caster
          released when on the lists)
others    at the damage ccSeOn3D(35, cPos) and soil 56, water
          ccSeOn3DNote(66, cPos, 48), dark 69; ccSkillDamage2; in range of
          cPos a shake (level 1 (0, 2, 20, 0), else (2, 2, 30, 2)) and
          noise 20; the end at the end
```

The ring 56 (`effSummonsRing`, main 0x001d1aa0; cases 0x001c468c /
0x001ca650): life 40, CLUT 161 made 167/162/164/163/165/166 by element,
generator 150-156 at cPos; 2.6 wide, 0.5 to 2.2 high, turning; at 12
`effSummonsShockWave` (main 0x001d1f10: three waves 64 and one 65,
re-coloured, growing by the tables at main 0x0033ff10-0x0033ff40 and the
level), at 22 (type 0) `effSmmonsFragment` (main 0x001d22b0: 6 + 2 level
pieces 66-78 thrown up, bouncing like 42-49). The creature 57-63 (164, 165
for 289, 290; `effSummonsElement` main 0x001d1d30, case 0x001c4738): an
`ANM_x?04` life 65, rising from half the caster's height to 50 above over
10 frames and turning in to its heading over 20.

289's drills (`effTCDrillMissile`, main 0x001d8410; effect 168, first
switch 0x001c6950, its ten states by `flags` through the table at main
0x00374c80): five, a fifth of a turn apart round the target from the
caster's bearing, 300, 400 or 500 out by the target's size
(`ccCheckObjectSize` 1: 2, 3: 1, else 0), each on the land there
(`ccLandHitCheck`, or 300 down) 96.5 under it. The game builds each place
in the spell's own `cPos`, which keeps the last (its z gathers `tPos.z`
each time round). Each waits 1-21 frames (by its serial number), leaps out
of the ground (`effSmmonsFragment` 3 soil, `ccSeOn3D(39)`, a shake,
generator 241) and drops back, turns, circles the target once over 40
frames riding the land, turns back, waits 3-43 frames, then leaps at the
target and bursts (`effSkillBreakSE`, noise 30, a shake, `effSmokeRock` 3,
`effFlareRing`, generators 239 and 240). It draws one of two `ccAnm`s of
X703.CCS it owns (temp[0] ANM_x703nut0, temp[1] ANM_x703atc0, by
`texAnmPat`); after the burst it deletes them and stays, drawing nothing.

290's lock-on (`effSBLockon` main 0x001d8190): the controller -26 (life
80, second chain 0x001cbd5c): at 0 the mark 166 (`effSBLockonMark`), at
30 the count 167 (`effSBStackNum`) and a range ring 172
(`effStackRangeRing`), every 5 frames the count on and another ring (up to
8), at 80 the blast (`effSkillBreakSE`, noise 50, cameraShake(2, 1, 50, 2),
`effSmokeRock` 12 rocks, `effHugeFlareRing` 171, generators 242, 243). The
mark and count are sprites on `effSBL` (priority 60) with no depth test.

Levels 3 and 4 (not 289, 290): `ccSummonsElementGenerate` (gcmn
0x005006b0) makes `ccSummonsSystemElement` (ctor 0x004ed180, Main
0x004ed840): the magic circle `ccMagicCircleElement` (CMP_x031 recoloured
by element, 10 over the target, growing to 5 and fading in over 15 frames;
60 frames after, out over 30, then deleted), `effEnergyGrow`'s rising
sparks (`ccEnergyGrowElement`, EFF_x005, 80 a level, life 60), and the
summoned element, run from count 30 under the system element until it is
deleted:

| element | ctor / Main | what it does |
| --- | --- | --- |
| fire `ccFireSummonElement` | 0x004f1570 / 0x004f1b00 | 1, 2, 4, 16 `ccExplodeElement`s 100-600 round the point 280 over the target, 2.5 frames apart, each with a ring 162 and as it goes off `ccElementShock` and a shake; the first hits |
| water `ccWaterSummonsElement` | 0x004f1c70 / 0x004f2320 | level 3 a level-4 water `ccFallElement`, the hit once it is deleted; level 4 16 `ccBubbleElement`s bursting into ice and rings |
| thunder `ccThunderSummonsElement` | 0x004f25f0 / 0x004f30a0 | six `ccThunderElement`s from 2000 up (the first scaled 5); level 4 first two small bolts (in the manager) every 8 frames for 60 frames with explosions and rings 195; `effThunderShock` (0x005002a0) on the sixth |
| dark `ccDarkSummonsElement` | 0x004f3510 / 0x004f3f80 | a `ccDarkBallElement` 580 up; level 3 32 `ccDarkBatElement`s out and back along cubic splines; level 4 four light balls (`ccLightBallElement` generators) along splines (`CalcSplinePoint` 0x004f3d80), bursting on the fourth leg |
| soil `ccSoilSummonsElement` | 0x004f4d10 / 0x004f5390 | 16 or 32 `ccRockElement`s 1000-1500 out flying in faster and faster, bursting at the target (level 4 with explosions) |
| wind `ccTreeSummonsElement` | 0x004f5850 / 0x004f6420 | the great tree CMP_x401b_1 and roots ANM_x403 growing, the leaves falling; level 4 32 `ccNeedleElement`s along splines |
| none `ccGoblinSummonsElement` (293, 294) | 0x004f7020 / 0x004f79e0 | level 3 eight falling `ccStarElement`s; level 4 stars along splines from 680 over the target |

### From Mutation on

MUT's systems are Infection's with two changes, and OUT's and QUA's
(recompiled) behave as MUT's. First, what Infection's aim at `tPos`
(+0x60, the target's place this frame) MUT's aim at +0x70 (its place at
the request). Second, the caster is released later:

| system | MUT gcmn | aimed at +0x70 | the caster released (Infection) |
| --- | --- | --- | --- |
| `FallSystem` | 0x0059d1e0 | the meteor's height, sound 57, the hits' `ccSkillDamage2` | at count 0 only when the target is gone; else at 120, or at the end or a lost target before 120 (count 0) |
| `TornadoSystem` | 0x0059ccf0 | the smoke, the rings and their sound, the hits round the place, the shake's range | at the end, after 75 (30) |
| `ConvergenceSystem` | 0x0059d930 | count 20 writes the target's pos to +0x70, not `tPos`; sounds 63 and 64, the shake's range, note 66 | at 93, or at the end before 93 (40) |
| `UpheavalSystem` | 0x0059e170 | the ground generator, the pillars, `ccSkillDamage2`, note 56, the shake's range | at 93, or at the end before 93 (35) |
| `SummonsSystem` | 0x0059ea10 | sounds 62, 290's lock-on, 289's drills, `ccSkillDamage2` | at 60 (level 3 up, 289 and 290: 100) when on the lists (the end) |

The convergence's burst, its smoke and `effSkillBreakSE` stay at the
target's middle. The pieces changed in main:
- `effSkillChargeObject` (MUT main 0x001eb650) sets the controller's
  `posT` to the target's middle (Infection: its feet).
- The controller passes `&posT` to `effSkillChargeObj` (MUT 0x001eb7a0)
  as a new second argument. The pieces start round it, not round the
  target's middle.
- A piece flies to its own `posT`, the target's feet when it was made,
  not to the target's middle (MUT 0x001db248, 0x001e08f8).

Four effects and elements changed too:
- `effTCDrillMissile` (MUT main 0x001edcf0) builds each drill's place
  from `tPos` in a local: the spell's `cPos` is left alone and the height
  no longer gathers. Each drill lives 600 frames (Infection: the row's
  -1).
- The drill 168 sets `endFlag` after its burst (MUT main 0x001dcae8).
- `ccTreeSummonsElement`'s roots: Infection's loop never moves past
  `m_rootsTbl[0]`. MUT's (gcmn 0x00511108) places all eight 100 round
  the tree, through `ccTransPosW2P` and `ccTransPosP2W`.
- `ccThunderSummonsElement::Main` (MUT gcmn 0x0050e0e8) plays the hit's
  four sounds once even when the shake is out of the camera's range.

The other elements differ only in the skill's offsets.

### The element manager and the elements

`ccEffectElementManager` (gcmn: ctor 0x004e8c40, Main 0x004e8da0;
`m_instance` main 0x00378c04): 1024 slots, run by `ccThEffect` after
`ccEffectCtrl::Main` on the effect layer: an element with `m_delFlag`
deleted, any other's virtual Main run, in slot order. Generators put a new
element in the first empty slot (none: deleted, the generator answers 0).
`ccAnimateObject` (0x130: the fade, the scale animation, the motion towards
a point) and `ccEffectElement` (0x190: the skill it serves, its target,
level, flags and counters) are the bases; every element's constructor
inlines both (vectors (0, 0, 0, 1), `m_scale` (1, 1, 1, 1), `m_transparency`
1, flags clear, `m_level` 1, `m_life` -1) and leaves `m_fadeSpd` and
`m_scaleSpd` as the heap had them (zero in the port);
ported: `ccRingElement` (ctor 0x004fbbe0, `SetModel` 0x004fb3f0, Draw
0x004fbf00; `effSummonRingElement` 0x004ffe20), `ccTornadeElement`, the
fall, convergence, upheaval and summons elements above, the drawn elements
(`drawelm`: fire, rock, dark, ice, thunder, fire ball, plasma ball, dark
bat, branch, explosion `effExplode3` 0x005000b0, magic circle, bubble,
needle, star, dark ball, light ball), and `ccThunderBoltElement` (gcmn
0x00500dc0, Draw 0x00501470) inside a `ccEffect2` (main: `InitEffect`
0x001cc240, `Main` 0x001cc290; 100 slots at 0x003fc160, run after the
500). An element a summons element holds (a bubble, a star, a bat) is not
in the manager: its owner runs it.

The drawn elements (effect2.cpp) run `ccDrawElement::Main` (gcmn
0x004ea670: `m_life` counts down to `m_delFlag`, -1 never, then the virtual
Draw). The models the spells move about:

| element | ctor / Draw | draws |
| --- | --- | --- |
| `ccFireElement` | 0x004f96f0 / 0x004f99e0 | `ANM_x300` (looping) and particle.ccs's `CMP_x100`, one matrix: scale, the cap's turn about y (`m_kasaRot`, on 0.1047 a frame), a quarter about z, the heading, a quarter back, at `m_pos` |
| `ccRockElement` | 0x004fa2c0 / 0x004fa540 | one of `CMP_x102a-d` (`ccRand() & 3`) at `PosRotZYXScale(pos, dirc, scale)` |
| `ccDarkElement` | 0x004fa5b0 / 0x004fa850 | `CMP_x604`, its heading three `ccRandF(pi)`; drawn as the rock, then heading y and z on 0.2094 a frame |
| `ccIceElement(n)` | 0x004faf90 / 0x004fb260 | one of nine ice clumps (`abs(ccRand()) & 7`); scale and heading, `m_pos` the translation |
| `ccThunderElement` | 0x004fa940 / 0x004fabc0 | `CMP_x012` along a chain of 20 points from `m_sp` to `m_ep`, the 18 between redrawn each frame at random round the line; one more link a frame to 19, then `m_endFlag` and a fade by 1/15 a frame |
| `ccRingElement(VP, VR, VS, Mode)` | 0x004fbbe0 / 0x004fbf00 | `SetModel(Mode)`: a ring clump of particle.ccs, fog off, duplicated and re-coloured when the mode names a CLUT; `Transparency` 1, `Tpoint` 0.05, `Spoint` 0.5 in all four lanes; with `EnyFlg` (which `effSummonRingElement` sets; with no empty slot its ring is never run) the scale grows by `Spoint` and `Transparency` falls by `Tpoint` each Draw, `m_delFlag` below 0; the matrix from Pos, Rot (x, y, z) and Scale |
| `ccExplodeElement(attr)` | 0x004fd8d0 / 0x004fdcf0 | `ANM_x305` with its four objects' palettes swapped for the element's (`CLT_x048`-`51` + suffix), fading and scaling as set, moving by `m_speed` in the player's frame; `m_delFlag` when the animation ends |

## The port

`crates/piney-effect`:

| module | what |
| --- | --- |
| `effect.rs` | `ccEffect`, `ccEffectCtrl` (`EffectCtrl`): `init_effect`, `new_effect`, `main` / `main_one` with the draw, the per-id dispatch to each kind's module, the fades, `check_camera_deg` |
| `files.rs` | `Assets`: the effect files (`SceneFile`s), `effectTbl`, `effectTbl2`, `alphaBlendTbl`, the sprites' textures and the particle tables, read from the disc and the executable at start; `add_file` for another scene file (the portal's) |
| `eff.rs` | the Eff chunk, `ccEff::Init`, `SetBlendType` |
| `sprite.rs` | `ccEff::Draw` and `mc_DrawEff`: `Eff::packet`, `Eff::render`, `fview`, `DrawEnv` |
| `nodes.rs`, `draw.rs` | what `ccClump::Draw` and `ccAnm::Draw` hand `ccObj::Draw`; `DrawRec` (a sprite, clump or animation with its matrix, transparency, palette swap and layer) and `Camera`; rendering through piney-world's model draw into piney-desktop's `Layers` |
| `space.rs`, `vu.rs`, `dmath.rs` | the map wrap; `libvu0`'s matrix routines and `ccCoord::SetMatrix_PosRot*Scale`; `acosf` and newlib's double `sin` / `cos` - bit for bit |
| `arrival.rs` | `effTransfer`, `effTransferRing`, `effWarpTransfer`: effects 3, 4, -25 |
| `hit.rs` | the hit mark, protect, attribute guard and critical, the words: effects 19-24 (their second chain), 131, 150-154, 169, 170, -23 |
| `flyfont.rs`, `damupr.rs` | the rising and the stacked numbers, the `flyFont` sprite |
| `levelup.rs` | `effLevelUp`: effect 2 |
| `drain.rs` | `effDrainCtrl`, `effDrain`, `effAfterDrain`: effects -24, 132, 133, 12-14 |
| `portal.rs` | `ccMagicCircle` and `ccMcPart` |
| `skillstart.rs` | `effSkillStart` (both), `effSkillStartEffect`, the exec and force rings and the circles: effects -12, -13, 82-86 |
| `shockwave.rs` | `effPhysicalSkillHitShockWave`: effects 79-81 (the pieces are `debris.rs`') |
| `heal.rs` | `effHeal`, `effHealSkill`, `effCure`, `effSanity`, `effResurrect`, `effOpenBox`, `effRemoveTrap`: effects 130, -19, -1, -2, -3, -6 |
| `ability.rs` | `effAbilityUp`, `effAbilityDown`, `checkEffAbilityColor`: effects -20, -21, -22; `eff_ability` is what `setConditionEffect` takes |
| `shield.rs` | `effResistantShield` (both) and `ccResistantShieldElement` (an element of the manager) |
| `spell.rs` | the attack spells: the `ccSkill` mirror `Spell`, the systems' dispatch; with `tornado.rs`, `thunder.rs`, `fall.rs`, `convergence.rs`, `upheaval.rs`, `summons.rs` (289's drill), `summoned.rs`, `debris.rs` (the tumbling pieces), `element.rs` (the element manager), `drawelm.rs` (the drawn elements), `ring.rs`, `pfx.rs` (the spells' generators) |
| `particle.rs`, `particle/` | `ccThParticle` ([the particle system](particles.md)) |
| `host.rs` | `host::Simple`: a `Host` from values the runtime has at hand; `Camera::from_field`, `CharState::player` |

### Driving it

The runtime owns an `Effects` (`Effects::new(archive, exe)` once the field is
entered; `ctrl.town` true in a Root Town) and each field frame, in the
tasks' order:

1. the starters, as the tasks before `ccThEffect` call them: `transfer` /
   `warp_transfer` for piney-world's `Request::Transfer` (Kite's arrival,
   act 13) - in the same frame as the request, so before
   `Effects::step`; `hit_mark`, `protect`, `attribute_guard`,
   `attribute_critical`, `critical`, `dying`, `no_damage`, `fly_font` for
   piney-battle's events ([the table above](#from-piney-battles-events));
   `level_up`, `drain_ctrl`, `after_drain`, `fly_font_exp`,
   `fly_font_level_down`, `fly_font_miss` where the game calls them; the
   skills' ([the skills' other effects](#the-skills-other-effects)):
   `skill_start`, `skill_start_param`, `skill_start_effect`, `shock_wave`,
   `heal`, `heal_skill`, `cure`, `sanity`, `resurrect`, `open_box`,
   `remove_trap`, `ability_up`, `ability_down`, `condition_effect` (and
   `kill_condition_effect`, `delete_condition_effect`),
   `resistant_shield`, in the frame and order the battle's code makes
   the calls: before `Effects::step` for the tasks that run before
   `ccThEffect` (the menu's items and Data Drain at 34, Kite and the party
   at 48 (`ccThSpc`), the enemies at 64 (`ccThEntryCtrl`)), after it for
   `ccSkill::Main`'s (`ccThSkill`, 82: the heals and cures of its
   systems, the shock waves of its animation's notes);
2. `Effects::step(host)`: `ccThEffect` (80) - the 500 effects, the 100
   `ccEffect2`s, then the spells' element manager;
3. the attack spells with `ccThSkill` (82): `Effects::spell_request` when a
   spell is requested, then each frame `Spell::sync` and
   `Effects::spell_system` for each running spell ([the spells](#the-spells));
4. `Effects::step_particles(host)`: `ccThParticle` (98);
5. `Effects::draw(&mut ctx.layers, &camera)`: every draw of the steps
   into the frame's layers (the effect layer is 20, `effSBL` 60; an effect
   with its own layer uses that), before `Ctx::finish`;
6. the numbers from the menu task's `Disp` (`Effects::fly_fonts(host,
   still, game_over)` after the enemy labels are queued on `flyFont`, then
   `flyfont::send` of `fx.font.take()` on the menu layer, 242);
7. the magic portals from `ccThEntryCtrl` (64): `MagicCircle::main` after
   the entry's `routine`, `MagicCircle::render` (piney-battle's `entry.rs`
   ports the same `ccMagicCircle` and `ccMcPart` for the entry control,
   with `CircleDraw` / `PartDraw` events for what they draw; both are checked
   against the game, so a runtime keeps one of the two);
8. `Effects::take_events()`: `Sound3d` (`ccSeOn3D`, whose volume and pan
   are [the sound page's](sound.md) rules), `Sound3dNote`, `CameraShake`,
   `Noise`, `Flash`, and the spells' `SkillDamage`, `SkillDamageAt`,
   `SkillDamage2` and `SkillRelease` (also handed to `Host::raise` the
   moment the game makes each call).

`Host` is what the effects read of the world: `rand()` (the game's one
newlib LCG: the world's walking PCs and the effects draw from the same
sequence) and `genrand()` (the Mersenne Twister behind `ccRand`), the
player's position and the map's bounds (the wrap), the camera
(`Camera::from_field(world.camera())`: `tcam`'s eye and view point,
`world_view`, `world_screen`; `Camera::env` the draw environment's fog), and
characters by a `CharRef` the runtime chooses: their position, heading,
height and width, `check_target` (`ccCheckTarget`), `char_dead`
(`condition.dead`), `char_type`, `object_size` (`ccCheckObjectSize`),
`char_attribute` (`CheckCharAttribute`), `party_slot`,
`condition_icon_num`, `camera_id` / `camera_type`, and `char_vec` /
`char_int` for the vectors and switches particle generators follow; for the
spells also `area`, `land_hit_check2` (`ccLandHitCheck2`),
`land_hit_check` (`ccLandHitCheck`), `hit_check_lm2` (`ccHitCheckLM2`) and
`raise`; for the resistant shield `char_pos_p` (`posP`) and
`affect_person` (`affectPerson`).
`host::Simple` builds one from those values each frame.

### The spells

The spells' modules: spell.rs (the `ccSkill` mirror `Spell`, the
systems' dispatch, the shared helpers), tornado.rs, thunder.rs, fall.rs,
convergence.rs, upheaval.rs, summons.rs (with 289's drill), summoned.rs
(the summons' level 3-4 elements), debris.rs (the tumbling pieces and their
spawners), element.rs (the manager, `ccAnimateObject`, `ccEffectElement`),
drawelm.rs (the drawn elements), ring.rs, pfx.rs (every particle generator
the spells start).

A runtime running `ccSkill` (piney-battle's `SkillRun`):

1. on `_ccSkillRequest` of an attack spell, `Effects::spell_request(host,
   key, sid, stype, creator, target)`;
2. each frame, after `Effects::step` (ccThEffect), for each run in
   `SkillEntryTop`'s order: what `ccSkill::Main` does before the system
   (positions into cPos/cDirc/tPos, the checks that end it), then
   `Spell::sync` (count = the value before Main's increment) and
   `Effects::spell_system(host, key)`; then take back `status` (endFlag),
   `hold`, `level` and the `SkillRelease` events;
3. when ccThSkill deletes the run, `Effects::spell_remove(key)`.

The damage calls are events, raised at the moment the game makes the call
(`Host::raise` sees each at once: the game's damage code and
`cameraShake(.., 2)` draw `rand()` between the effects' own draws):
`SkillDamage` (gcmn 0x00573e60), `SkillDamageAt` (0x005743c0),
`SkillDamage2` (0x005746d0), plus `Sound3d`, `Sound3dNote`, `CameraShake`,
`Noise`, `SkillRelease` and `Flash` (`scFadeDef->EntryFlash`, main
0x00160240: the ice upheaval's flash). The host provides `check_target`,
`char_dead`, `char_type`, `object_size`, `genrand`, `area` (game +0x14,
+0x28, `WORLD_MAN` field type), `land_hit_check2` (`ccLandHitCheck2`, gcmn
0x00572100; 9900.0 for no hit), `land_hit_check` (`ccLandHitCheck`, gcmn
0x00571e00; `pos.z` for none) and `hit_check_lm2` (`ccHitCheckLM2`, main
0x00153900; -1.0 for none).

A generator that follows an element's own position (the dark ball's, a
light ball's, a star's; the game points its `syncPos` into the element) is
moved by the element itself after each of its steps, as the particle step
would read it.

## The streams' effects

The in-engine streams' effect tasks start hit marks and transfers of their
own. These run in a second effect system, which `ccThEffectStr` (priority
80) steps while a stream plays, and a second particle system.

```text
ccEffectCtrl(1) (effcStr)     50 slots (effStrWork 0x003fc930); effectStrTbl's
                              three objects of particle: 0 EFF_x007 (type 3),
                              1 CMP_x037, 2 CMP_x032; the hit marks' palettes
                              CLT_x007c1, CLT_x037, CLT_x037c1; and
                              hitPhotonDummyF1/F2 on hitPhotonDummyStrG
ccParticleCtrl(1)             particlesStr's 150 (0x003ddf60): +0x08 and +0x10
                              chosen by the constructor's argument; each
                              particle runs ccParticle::MainStr
particleThreadStrFlag         set while a stream plays: every generator
                              started (ccParticleCtrlAddGenerator) and every
                              particle made (ccParticleCtrlGenerateParticle)
                              goes to the second system
effHitMarkStr(pos, rot, lay)  effect 0 at pos (its eff drawn through
                              CLT_x007c1); effect 1 at pos turned by rot (XYZ),
                              lifeTime 15, its clump duplicated with CLT_x037
                              drawn as CLT_x037c1; effect -1 at pos turned by
                              rot; each on the first free slot, layer set
effTransferStr(pos, h, lay)   effect -2, lifeTime 50, offset.z h
ccEffectCtrl::MainStr         each live slot in order: active__7ccLayer set to
                              the effect's layer when it has one (never put
                              back per slot, so an effect with none draws on
                              the last one set), then ccEffect::MainStr
ccEffect::MainStr             as Main, with its own cases and a draw with no
                              ccTransPosFW2LW and no camera cone or distance:
  1   the hit ring (the field's 131)
  2   the transfer ring (the field's 3)
  -1  counts 0 and 1: five photons (ccParticleSetup(102, pos, 12)), their
      strFlag 1, generator hitPhotonDummyStrG, the effect's layer
  -2  counts 0, 3, 6: a ring (effect 2, lifeTime 40, following this pos,
      offset.z 20 + this offset.z, rot z DEG2RAD(rand() & 0x3f00)); count 10:
      particleGeneratorTbl[82] at pos + offset.z, strFlag 1
ccParticle::MainStr           the fades and force fields as Main, pos +=
                              velocity, then the draw with no camera; while
                              strEffectStopFlag is set a particle with a
                              strFlag stands still
```

`hitPhotonDummyF1` and `F2` make the stream's photons four times the size
and slow them, as they do the field's.

**The port.** `piney_effect::strfx` holds the starters and `MainStr`'s
cases. `EffectCtrl::stream()` and `Particles::stream()` build the two
systems, and `StreamEffects` wraps them. `piney-stream` starts them from
its effect tasks (`Task::take_marks`, `take_transfers`) and draws them
after the scene. `piney-game` gives them to the event and Data Drain
streams, with main's tables and GCMN.PRG's.

## Checks

Each harness runs the game's own code in eemu beside an example of the
port and compares, frame by frame, every live effect slot's whole state,
every draw in order, every event and `rand()`'s state:

- `tools/test_effect_rs.py` (`examples/effect_probe.rs`, on `EffectMachine`:
  main and gcmn with `ccEffectCtrl` built by its own constructor over the
  effect files, `rand()` newlib's LCG seeded as the probe seeds it, the
  draws, sounds and generator starts recorded by hooks):
  - `ArrivalAgainstGame`: `effTransfer` on a character, then
    `ccEffectCtrl::Main` for 80 frames: Kite at Mac Anu's start with the
    camera behind him; a character walking with the camera far enough to
    fade the rings, over three seeds; the camera turned away (nothing
    drawn); a field (`effectTbl`); `effWarpTransfer` in a town and a field.
    0 mismatches.
  - `CoreAgainstGame`: `ccEffect::Main`'s own work through the ids with no
    code of their own - 1 and 8 (clumps), 12-14 and 22-24 (`drain`'s
    animations), 170 (`ANM_x071`) - and id 1 with its `effectTbl` row
    patched (in the game's memory and the port's table alike) to each
    sprite kind, 2-5, over `PARTICLE.CCS`'s `EFF_` objects. In a field, 6-16
    effects at a time with random positions, turns, scales, transparencies,
    lifetimes and bits, some following a character's position or another
    effect's (posPtr), turned by a character's heading or another effect's
    (rotPtr), linked to another effect, with a target, or on a layer of
    their own; characters walking, the camera moving and sometimes turned
    away or 6000 off; effects paused and resumed at random and more started
    as the frames go. 40 cases of 70 frames over the clumps and animations
    (7,979 draws; 15,248 paused and 6,394 hidden slot-frames) and 24 cases
    of 50 frames over the sprites (3,341 draws): 0 mismatches.
- `tools/test_effect_draw_rs.py` (`examples/draw_probe.rs`):
  - `SpritesAgainstGame`: the game's `ccEff::Draw` in eemu with a ccView set
    up as the field's (`sysLayer`'s view, `WORLD_MAN::GO`'s `SetFrame`,
    `SetView` from `ccCam::SetMatrix_PosTarget(eye, target)`), a ccDrawEnv
    from its constructor and `SetFog` (Reset's, Mac Anu's, random), every
    `EFF_` object's `ccEffChunk` with its texture and palette made by
    `ccTexChunk::Init` / `ccClutChunk::Init` from the file, the ccEff by
    `Init(chunk, fogSw)`; the packet through a VIF1 model into
    `tools/vu.py`'s `Vu` with the MAC flags delayed four pairs, and
    XGKICK's output decoded as the GS takes it. Random cameras, fog, `EFF_`
    objects, positions (round the target, off to a side, at or behind the
    eye, beyond 2^20), scales (negative and 0 too), turns (0, -0, large),
    colours, transparencies (negative, over 2), patterns, blend types 0-7,
    render states, PRIM with and without FGE. Compared bit for bit: the
    view's `fview`, the fog constants, whether a packet is sorted and why
    not, its key and matrix, whether VU1 kicks, ALPHA, TEST, TEX1, CLAMP,
    TEX0's format bits, MIPTBP1, RGBAQ, FOGCOL, ZBUF, PRIM, FOG, each
    corner's S, T, X, Y, Z and drawing kick, and the port's rendered `Prim`
    against the one the GS draws. 3 x 10,000 cases (17,269 drawn, 5,382
    rejected by VU1, 4,927 behind the near plane, 1,820 beyond the far one,
    602 hidden; 750 views, 750 fog set-ups; the default run is 6,000): 0
    mismatches. Changing the corners' row offset, the fog's w, the far test
    or the divide-by-multiply in the port fails it.
  - `ModelsAgainstGame`: every effect file decoded by the game's
    `ccStream::DecodeSetup`; for each of the 113 `CMP_` objects whose nodes
    are all Obj, 10 random matrices and transparencies, `ccClump::Draw`;
    for each of the 21 `ANM_` objects `effectTbl` names, 30 random step
    counts and speeds, `ccAnm::Draw`; `ccModel::Draw` recording the object,
    model, lwMatrix and transparency. 1,130 clump draws (864 models):
    models, order, transparency and lwMatrix bit for bit (up to a zero's
    sign); 630 animation draws (2,751 models): models, order and
    transparency bit for bit, lwMatrix within 2.9e-6 relative. 0 mismatches.
- `tools/test_effect_particle_rs.py`: [the particle system](particles.md#checks),
  14,359 frames, 0 mismatches.
- `tools/test_effect_hit_rs.py` (`examples/hit_probe.rs`, on the particle
  harness's machine: the real particle system runs in the game side,
  `ccParticleSetup`'s `rand()` draws included; `ccCheckTarget`,
  `ccCheckObjectSize`, `CheckCharAttribute`, `checkPartyMenberNum`,
  `ccConditionIconNum` answered, `cameraShake` recorded, `flyFont` a real
  `ccFont` so `MakePacketStr` writes real packets):
  - hits: 40 random cases, 480 `ccHitMarkDisp` calls (random positions,
    heights 40-400, widths 5-120, headings, a character on itself, dead
    characters), 2,169 frames: 9,200 slot-frames, 7,157 ring draws with
    their palette swap, 460 hit marks, 4,600 photons, 172,035 particle
    states and 131,470 particle draws: 0 mismatches;
  - protect, guard, words: 30 cases, 119 `effProtect`, 136
    `ccParticleAttributeGuard` (44 refused), 95 `ccParticleAttributeCritical`,
    25 `ccParticleCritical`, 29 `ccParticleDying`, 29 `ccParticleNoDamage`,
    6,003 frames (15,172 slot-frames, 4,728 animation draws, 7,896 word
    draws, 282 sounds and shakes, the generators with their pTexMod, the
    guard list, the ccEff's TEST and palette): 0 mismatches;
  - numbers: 60 cases, 5,400 `Disp` frames (still and game over at random,
    the party view, cameras moving), 815 `ccEntryFlyFontNum`, 4,339
    `ccEntryFlyFontNew`, 671 `...Exp`, 491 `...LevelDown`, 585 `...Miss`
    (values 0-9,999, -1, negative, 10,000-199,999; null and dying
    characters): every `flyFontCtrl` entry (32,369 entry-frames), every
    `ccDamUprStr` node and line (84,894 node-frames, 404,061 line-frames),
    `flyFont`'s fields after each call, and the 356,939 SPRITEs
    `MakePacketStr` wrote, word for word (the 160 cap hit in 502 frames): 0
    mismatches;
  - each of 12 one-constant mutations of the port fails the checks.
- `tools/test_effect_misc_rs.py` (`examples/misc_probe.rs`, in a field so
  every effect row resolves; `ccCheckTarget`, `ccCheckObjectSize`,
  `condition.dead`, the words' layer and the orbs' palettes supplied):

  | check | cases | frames | compared |
  | --- | ---: | ---: | --- |
  | math | 3,411 `acosf`, 3,010 `sin`, 3,010 `cos` | - | results to the bit |
  | level up | 60 (1-3 level ups, moving, off the lists, down) | 5,726 | 2,949 draws, 538 generators, 97 sounds |
  | dying blow | 40 | 2,853 | 3,449 draws, 160 generators |
  | `effDrainCtrl` | 40 (1-3 drains, types 0 and 1, 1-11 orbs over 1-11 frames) | 3,335 | 18,432 draws, 493 generators |
  | many orbs (`effDrain` x 30) | 15 | 1,059 | 17,619 draws, 450 generators |
  | the orb's half turn | 10 | 482 | 472 draws |
  | `effProtect`, `effAfterDrain` (sizes 0-2 and -1) | 40 | 2,825 | 3,864 draws, 74 sounds |
  | portal | 40 (every act; frozen, far, turned away, faded) | 10,280 | 485,619 draws, the portal's and all 128 sparks' state, `ccRand`'s index; 39 opened to the end |

  All 0 mismatches. The generators started are compared by their row,
  what they follow, their switch and offset (their particles are the
  particle harness's). The dying blow, the words and `effProtect` run
  through `hit.rs` here too (this harness was written against a second port
  of them): both harnesses agree with the game.
- `tools/test_effect_spell_rs.py` (`examples/spell_probe.rs`): the game's
  own ccSkill (its constructor, `_ccSkillRequest`'s set-up, `ccSkill::Main`
  and the systems), ccEffectCtrl::Main and ccEffectElementManager::Main in
  eemu beside the port, random casts (caster, target and bystanders at random
  places, sizes, headings, types and object sizes; the target sometimes off
  the lists, down, or moving; sometimes no caster; random ground under a flat
  land for `ccLandHitCheck2` and `ccLandHitCheck`, no models for
  `ccHitCheckLM2`; random fields), frame by frame until everything
  has ended: every live effect slot (every field, the `ccEff`'s turn, PRIM
  and CLUT), every `ccEffect2` and element (every DWARF member), the
  `ccSkill`'s fields, the draws in order (clumps with their CLUT swaps, anms,
  sprites), the events with their arguments, every generator started (row,
  force fields, what it follows, every field set), the casters' skill state,
  and both generators' positions (`rand()` and `genrand()`).

  - every attack spell (102 ids: all four levels of the five systems and
    289-294), 3 casts each on the final build: 37057 frames (tornado 5614,
    fall 4925, convergence 6360, upheaval 6876, summons 13282), 0
    mismatches;
  - per volume (`PINEY_VOLUME`; the `ccSkill` laid out as the disc's):
    20 casts of each id, 2,040 casts a volume, 0 mismatches on Infection
    (247,286 frames) and Mutation (244,762);
  - deeper runs along the way: levels 1 and 2 and 290-292 (51 ids, 6 casts
    each) 32511 frames; tornado levels 3 and 4 (10 ids, 8 casts) 7929; fall
    and convergence levels 3 and 4 (18 ids, 6 casts) 11632; upheaval levels
    3 and 4 (8 ids, 8 casts) 9573; all 0 mismatches;
  - unit tests (spell.rs, fall.rs): the id ranges, a level 1 tornado's nine
    hits, the end frames of upheaval and summons, convergence and fall's
    single damage, and every level 3-4 system's end frame and hits.
  
  Masked or pinned (neither side can know): a convergence piece's pos.w until
  it moves (the stack's leftover plus 1) and a summoned creature's offset.z
  and w (never set); locals the game reads before it writes them - the tree
  upheaval piece's throw, the ice upheaval's `Delete` throws' x and w, the
  none summons' rings' x and w - which the check zeroes as the function
  starts (a hook at the instruction after the prologue) and the port takes
  as 0; and `ccGoblinSummonsElement::_EntryElement`'s search past its table
  (it counts on from the element it last set up), which the check answers
  busy as the port does. A generator pointed into an element's own vector (a
  ball's, a star's) is compared as following nothing: the port's element
  moves it instead.
- `tools/test_effect_skill_rs.py` (`examples/spell_probe.rs`'s starter
  requests, on the spell harness's `SpellMachine`: ccEffectCtrl::Main and
  ccEffectElementManager::Main each frame, a `ccAnm`'s transparency left
  1 by its constructor, the characters' `posP` and `affectPerson` set):
  random fields, characters of every type (PC, enemy, boss) moving,
  leaving the lists and going down, the camera moving. Each group runs
  its sweep first - every value worth a start once, a few starts a
  frame: all 304 skills (by id and by `ccSkillParam`, `a` and `b` in
  turn) and `effSkillStartEffect` on every element and some mixed bits;
  the shock wave on every element; heals of levels -8 to 16, the heal
  skills 140-170 and 295, each cure, boxes, traps with default and odd
  textures; both stat changes for conditions 0-40; the virus crystal,
  every crush with x 0 and 1, the trap box for kinds 0 and 1 and traps 0,
  3 and 4, and the statue with its switch turned off some frames on; the
  shield on two
  enemies, a boss and an enemy-and-boss for magic -1 to 2 and n -1 to 4 -
  then random cases of 1-4 starts in the first 40 frames; then frames
  until everything has ended (the stat changes' controllers ended by
  their endFlag at a set frame, as `~ccConditionEffect` does). Each start's
  answer, events, generators (every field their starters set) and both
  random generators' states, and each frame's live slots, elements (every
  DWARF member), draws, events, generators, `rand()` and `genrand()`:

  | group | starts | cases | frames | compared |
  | --- | ---: | ---: | ---: | --- |
  | start: `effSkillStart` by id and by `ccSkillParam`, `effSkillStartEffect` | 512 | 91 | 5,371 | 41,123 draws, 52,493 slot-frames (ids -13, -12, 82-86), 512 generators, 433 sounds |
  | shock: `effPhysicalSkillHitShockWave` | 227 | 82 | 6,604 | 88,648 draws, 114,336 slot-frames (the waves and every piece), 1,344 generators, 375 events |
  | heal: `effHeal`, `effHealSkill`, `effCure`, `effSanity`, `effResurrect`, `effOpenBox`, `effRemoveTrap` | 262 | 83 | 7,537 | 3,041 draws, 11,815 slot-frames, 761 generators, 119 sounds |
  | ability: `effAbilityUp`, `effAbilityDown` | 288 | 84 | 14,129 | 25,121 slot-frames, 3,054 generators with their textures |
  | shield: `effResistantShield` | 262 | 84 | 2,964 | 3,096 element-frames, 3,090 animation draws, 209 sounds |
  | ids: `effIceRock` over a flat land at a random height; 0, 113, -11..-8 by `ccNewEffect`, set up as a maker would | 169 | 64 | 5,604 | 19,453 draws, 31,403 slot-frames (ids -15, -11..-8, 0, 7, 9-11, 15-18, 113), 610 generators, 64 events |

  All 0 mismatches (36,605 frames; the default run is the sweeps and
  20-30 random cases a group). Each of 18 one-constant mutations of the
  port (a life, a CLUT row, a generator row, a sound, a fade time, the
  rise, a turn, a piece count, a scale's end, a trigger count, a modulus,
  a lift, a colour column, the shield's hundredth, its pi/2, its
  animation's choice) fails the checks.
  The ids group's sweep is an IceBreak's twelve throws at once, the meteor
  0 for each element (and one not falling), 113 with lives 0-40 and -1
  (ended at random) and each of -11..-8. Each of 8 mutations (the ice
  rocks' bounce as the debris', their case dropped, 0 dropped from either
  pass, 113's case dropped, its start, its fade-out time, its hold) fails
  it; with the ice rocks' case dropped, as the port had it, the rocks part
  from the game at the first landing check. Run on Mutation's disc
  (`PINEY_VOLUME=mutation`) it agrees too: 34 cases, 2,786 frames, 10,079
  draws, 15,968 slot-frames. On Outbreak's and Quarantine's it parts at the first draw
  the distance fades: their `Main` has `sqrt.s` where Infection's calls
  `sqrtf`, an ulp or two apart, for every effect.
- `tools/test_effect_draw_rs.py`'s `ModelsAgainstGame` also draws the
  shield's two animations (ANM_x069, ANM_x070) and the magic portal's
  file after the effect files (`XMAGCIR.CCS`: its clump, the idle loop
  and the opening stepped 1-179 frames at 256, into act 4's held end): 1,140
  clump draws (896 models), 750 animation draws (2,872 models), 70 of them
  the portal's (107 models), 0 mismatches in models, order, transparency
  and lwMatrix.
- `cargo test -p piney-effect`: the core's, the arrival's, the sprite's,
  the nodes', the hits', the particles', the level up's, the drain's, the
  portal's (it wakes, opens and goes; the opening fades it out, the last
  object drawn at frame 108) and the skills' own tests (a skill start's rings and circle, a
  fire blow's waves, a heal's bursts, a cure's generators, a stat up's
  repeats, the shield's facing and end) against the disc (skipped without
  it).
- `tools/test_effect_str_rs.py` (new): the game's `ccEffectCtrl(1)` and
  second particle system, built by their constructors with
  `particleThreadStrFlag` set, against `hit_probe`'s `strreset`,
  `strhit` and `strtransfer`. 30 random runs of eight starts (hit marks at
  random turns and layers, transfers at random heights), with frames until
  everything ends, compare every live slot, draw, generator, particle and
  rand's state each frame. All match: 5313 frames, 15093 effect draws and
  234972 particle draws.

## Unknown

- `unknown_0c` of the Eff chunk (112 in every chunk seen), which
  `Decode_Eff` reads past.
- What `ccEffect::level` means to the ids that use it, beyond each case's
  own code.
- `ccEffObj` nodes in a clump (`DrawNoAnm`, the node's pattern at +0x106):
  no effect clump or animation has one; not ported.
- What `ccBltData` flag bit 1 means exactly (resident in VRAM, so no upload
  is chained); the port leaves uploads to the renderer. `Prim` has no fog,
  so a sprite's FOG is dropped (the effects' own sprites never fog).
- `Draw`'s unsorted path for `a >= 4096`, which no `a` reaches; a TEST
  reference over 255 spilling into AFAIL (ported as the OR, never reached:
  aref 112); the shadow model switch for effect objects (never reached).
- Effects the game calls that are not ported (Skeith's `BeginDeadEffect`
  and `ccBossEffDead` are, in piney-battle's `boss.rs` and
  piney-effect's): `effVirusCrystal` from `ccRtownPC::eventMode` (the
  mode volume 2's event 114 gives), and the later volumes' bosses'
  `effSmoke`. Every other caller of `effSmoke` in Infection is ported
  (`WORLD::DrawSteam` and `DrawEffect` in `field_ambient`, `ccGimSymbol`,
  `ccDog`, `ccPGuso`, `ccPucciguso`, the enemies' dust and broken parts).
- `effResistantShield` with no `affectPerson` reads the heading's target
  through a null pointer: whatever the machine holds at 0x50 (the
  kernel's memory on a PS2, zero in eemu); the port takes (0, 0, 0, 0). `effAbilityUp` and
  `effAbilityDown` with no memory for a generator (a failed `new`) would
  write through a null pointer; the port's generators never fail.
- What the effects' generator rows (157-184 the start's sparks, 211-221
  the stat changes', 61, 94-108, 118, 119) look like on screen, beyond the
  shots.
- `CheckCharAttribute(ch, 1)`, `ccCheckObjectSize`, `ccConditionIconNum`,
  `condition.dead` and the command lists are host inputs, not ported here.
- `ccCheckObjectSize` sizes other than 1, 3, 4, and drain types other than
  0 or 1: the game goes on with an id it never set (`effProtect(-1)`,
  `effAfterDrain(-1)`, `effDrainCtrl`) or scales by the caller's `f20`
  (`effAttributeGuard`); the port starts nothing, or scales by 1.
- `hitFlip`'s first value: `ccChar`'s constructor does not set it (the port
  starts it clear).
- Whether any menu clears `flyFont`'s ctrl bit 0x10: until then the enemy
  labels drawn with it are shadowed once a number has been.
- What each particle generator row (60, 206, 222-228 ...) looks like on
  screen, beyond the calls named here.
- The portal counters in the save (+0x6864, +0x6866, +0x6868) beyond
  "portals opened" (what reads them); `ccEntryCtrl::entryCircleObject`
  (gcmn 0x00430360), what comes out of a portal. (`ccThDfComp`, which the
  last portal starts, is the "ALL FIELD / ALL DUNGEON PORTALS OPEN"
  banner: [field UI](field-ui.md#all-portals-open-ccthdfcomp).)
- `ccEntryChangeCLUT`'s swap is ported as "the palette `MAT_clut`'s texture
  uses", which also swaps any other texture on that palette; the game swaps
  per material. The portal's and the palette swaps' effect on the picture
  are not checked on screen.
- `SetFogSw` on the pieces' clumps and sprites, `SetShadowSw` on the
  drill's and the tree's models, and the drill's `ccDrawEnv` settings
  (+0xa4 128, +0xa0 450.0 while it draws) are not modelled (no fog, shadow
  or draw-environment state in the draw records).
- The explosions' palette swaps (`ccExplodeElement`'s objects,
  `GetSubstAdrsF`, `ccModel::ChangeClut`) are kept on the element but not
  in its `Anm` draw record.
- `ccEnergyGrowElement`'s `m_genRnd` (a random scale of the sparks a
  frame, `ccRandF(1, rnd)`) is 0 at its one caller and not ported.
- `ccThunderBoltElement`'s MissileSmoke/BurstSmoke generator paths (its
  data's `EFF_SW` bits) are not reached by any data the spells use and are
  not ported.
- The wind charge pieces' `rand()` written into their clump's `lwMatrix`
  (rebuilt before any draw) is drawn and dropped.
