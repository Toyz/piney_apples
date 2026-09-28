---
title: The particle system - ccThParticle
status: partial
volumes: INF
covers: INF SLUS_202.67:0x001bc2f0 ccThParticle, 0x001bc3c0 ccThParticleDelete, 0x001bc420 ccParticleHitMark, 0x001bc4b0 ccParticleHeal, 0x001bccd0 ccParticleExplode, 0x001bcde0 ccParticleForceField::Calc, 0x001bd920 ccParticleGenerator::ccParticleGenerator, 0x001bdd00 ~ccParticleGenerator, 0x001bdda0 ccParticleGenerator::Generate, 0x001bed00 ccParticleGenerator::Main, 0x001bf070 ParticleKill, 0x001bf0b0 ParticleDelete, 0x001bf0f0 ccParticle::Setup, 0x001bf8d0 ccParticle::Main, 0x001c0d80 ccParticleCtrl::ccParticleCtrl, 0x001c0e10 InitParticleCtrl, 0x001c0f40 FreeParticleCtrl, 0x001c10d0 GenerateParticle, 0x001c11b0 DelGenerator, 0x001c1230 ccParticleCtrl::Main, 0x001c13f0 ccConditionEffect::ccConditionEffect, 0x001c2360 ~ccConditionEffect, 0x001c24a0 setConditionEffect, 0x001c24f0 killConditionEffect, 0x001c2520 deleteConditionEffect, 0x001c2560 startParticleEffect, 0x001c2770 startParticleEffect2, 0x001c2920 startParticleGenerator, 0x001c2940 ccParticleSetup, 0x001c2a00 ccParticleAdrs, 0x001c2a20 ccParticleCtrlAddGenerator, 0x001c2ab0 ccParticleCtrlParticleCountUp, 0x001c2af0 ccParticleCtrlGenerateParticle, 0x001c2b40 ccCheckParticleGenerator, 0x001c2c30 ccSetQuaternion, 0x001c2cf0 ccQuaternionToMatrix, 0x00155210 normal2Angle, 0x0013bb40 ccEff::SetRenderState, 0x00375830 __sinit_effect.cpp, 0x00340220 particleTbl, 0x003402f0 particleGeneratorTbl, 0x00343850 particleForceFieldTbl, 0x00344ad0 particleEffectTbl, 0x003739e0 particleCcsAnmTbl, 0x003e4680 particleCcsAdrs, 0x00388060 particles, 0x0033ef80 oneVector (particle.cpp's), 0x0033ef90 Polyhedron82Table, 0x0033fac0 ccpffpSmoke1-4, 0x0033fed0 hitPhotonDummyF1/F2, 0x003fefd0 ccpgSmoke, 0x003ff0e0 ccpgPawSmoke, 0x003ff1f0 hitPhotonDummyG, 0x003ff300 hitPhotonDummyStrG, 0x00374260 Setup's texture switch, 0x003741f0 Calc's switch, 0x00374240 Generate's rType switch, 0x003744d0 ccConditionEffect's effect switch, 0x001274c8 sin, 0x00126ee0 cos
---

# The particle system - ccThParticle

The field's second effect task (priority 98, after `ccThEffect`; see
[the field's effects](effects.md)). `crates/piney-effect`'s `particle`
module is the port, checked against the game's own code in eemu by
`tools/test_effect_particle_rs.py`.

The particle system (main particle.cpp) moves many small things at once:
sparks, dust, rising motes, the rings and debris of the spells. A
*generator* (`ccParticleGenerator`) makes *particles* (`ccParticle`) over
time; each particle is one sprite, clump or animation that flies, feels its
generator's *force fields* and fades out.

```
ccThParticle(data) (0x001bc2f0):
  pcp = new ccParticleCtrl(data->num)        num (+0x14) 0 in the field
  num 0: particleSystem = pcp (gp 0x00378aa0), task name "PARTICLE"
  else:  particleSystemStr = pcp (0x00378aa4), "PARTICLE(STR)",
         particleThreadStrFlag = 1 (the in-engine streams' second system)
  each frame: Breath(1), then ccParticleCtrl::Main(pcp)
ccParticleCtrl::Main (0x001c1230):
  for each generator g from gTop along next (+0x30):
    g->Main()
    g->endFlag: DelGenerator(g), delete its four force fields, delete g
  save ccLayer::active
  for i in 0..pMax, particles[i].texID != -1:
    ccLayer::active = the particle's layer, or WORLD_MAN +0x4c8 (the effect
                      layer, priority 20)
    strFlag (+0x02 low nibble) ? MainStr() : Main()
    texID now -1: its generator's pCnt--, pNum--
  restore ccLayer::active
```

A generator started during the generator pass (appended to the tail) runs in
the same pass; every particle made in it runs `Main` in the same frame. The
task runs after `ccThEffect` (priority 98 after 80).

## ccParticleCtrl (0x18 bytes)

```
+0x00  ccParticleGenerator *gTop, *gTail   the list, in the order started
+0x08  ccParticle *pWork     particles (0x00388060, 2000 x 0xb0), or
                             particlesStr (0x003ddf60, 150) when sw
+0x0c  short gNum, pNum      generators on the list, live particles
+0x10  short pMax            2000 (150)
+0x12  short strFlag         sw
+0x14  int particleSearch    toggles every GenerateParticle
```

`InitParticleCtrl(sw)` (0x001c0e10, from the constructor 0x001c0d80) looks
every row of `particleCcsAnmTbl` up once - `particleCcsAdrs[i] =
GetChunkAdrsF(GetCCSAdrs(ccs), anm, 0)` - empties the list, zeroes the
counts and sets every particle's texID to -1, leaving the rest of each slot
as it was (so a slot's `pos` and `temp` outlive its particle; `Setup` does
not clear them). `FreeParticleCtrl` (0x001c0f40, from the destructor) deletes
every generator and its force fields and every particle's object by style.

`GenerateParticle` (0x001c10d0) flips `particleSearch` (0 to 1, anything else
to 0) and answers the first slot with texID -1 from the front when it is now
1, from the back when 0; none: NULL (the caller makes nothing, and draws no
`rand()`). `ccParticleCtrlGenerateParticle`, `...AddGenerator` (append,
`next` 0, gNum++) and `...ParticleCountUp` (pNum++) (0x001c2af0, 0x001c2a20,
0x001c2ab0) pick `particleSystemStr` while `particleThreadStrFlag`, else
`particleSystem`; `startParticleGenerator(g)` (0x001c2920) is AddGenerator.
`ccCheckParticleGenerator(g)` (0x001c2b40): is `g` on either list.
`DelGenerator` (0x001c11b0) unlinks and gNum--.

## ccParticleGenerator (0x100 bytes)

```
+0x00  ccParticleGeneratorParam *param   a table row; NULL for the statics
+0x04  bits  0 distSW  1 gSyncFlag  2 pSyncFlag  3 pStopFlag  4 syncPosType
             5 syncPosType2  6-7 strFlag (signed)
+0x05  bit 0 pauseFlag
+0x06  short pCnt        live particles it made (Generate ++, their end --)
+0x08  short gAge, gLife, pLife, pNum (made), regularlyOfst
+0x14  void *syncRot, *syncPos, *syncPos2      what it follows
+0x20  int *syncSW       it stops generating when *syncSW is 0
+0x24  float gRate       pGenRate / 30: particles a frame
+0x28  float gRateCnt
+0x2c  short pTexMod     -1, or the texture id its particles use instead
+0x30  ccParticleGenerator *next
+0x34  ccParticleForceField *ff[4]       0x30 bytes each; only +0x00 param read
+0x44  ccLayer *layer    its particles' layer (NULL: the effect layer)
+0x50  float rot[4], pos[4], pos2[4], offset[4], offset2[4]
+0xa0  float velocity[4] never written or read here
+0xb0  float mat[4][4]   RotMatrix(rot), what directions are turned by
+0xf0  char killFlag     1 its particles follow its end, 2 killed, 3 deleted
+0xf1  char endFlag      ccParticleCtrl::Main deletes it
+0xf4  u32 sn            generatorSerialNum++ (gp 0x00378aa8)
```

The constructor `(param, f1, f2, f3, f4)` (0x001bd920): pCnt, gAge, pNum,
regularlyOfst 0; distSW 1; no sync pointers; gRate `pGenRate / 30.0` (0
without a param), gRateCnt `1 - gRate` when gRate is below 1, else 0; the
other bits 0; each `ff[k]` a new `ccParticleForceField` of `fk`, or of
`particleForceFieldTbl[param->ffNum[k]]` when `fk` is NULL and the row names
one (without a param: none); gSyncFlag and pSyncFlag the row's `gSync` and
`pSync`; gLife, pLife the row's (-999 without); layer 0; rot 0; pos, pos2,
offset, offset2 (0, 0, 0, 1); killFlag, endFlag 0; `sn`; pTexMod -1.

`Main` (0x001bed00):

```
strEffectStopFlag (0x00378ab8) and strFlag: nothing
gSyncFlag and syncRot: rot = *syncRot
gSyncFlag and syncPos: pos = offset + *syncPos (+0x10 with syncPosType); pos.w 1
gSyncFlag and syncPos2: pos2 = offset2 + *syncPos2 (the same); pos2.w 1
mat = sceVu0RotMatrix(unit, rot)               z, then y, then x
pauseFlag: return
killFlag < 2, (no syncSW or *syncSW), (gLife -1 or gAge < gLife):
  a = (int)gRateCnt; gRateCnt += gRate; b = (int)gRateCnt
  gType 0: b - a Generate()s    1: one    2: while pNum is 0, pGenRate of them
else: killFlag < 2: ParticleKill(); then pCnt <= 0: endFlag = 1
param->gLife != -1 and gAge >= 0: gAge++
```

`ParticleKill` / `ParticleDelete` (0x001bf070, 0x001bf0b0) set killFlag 2 /
3 when the generator is on a list. Particles of a generator whose killFlag
was set when they were made (`Setup` copies `killFlag != 0`) fade out at 2
and end at 3; others go on.

`Generate` (0x001bdda0) makes one particle; `r` is `rand() >> 3` each time it
is drawn:

```
p = GenerateParticle(); none: return
r = rand() >> 3
Setup(p, g, pTexMod or param->pTex, pPatRnd, distSW); pNum++ (ctrl), p2Num++,
g->pCnt++, g->pNum++
where, by rType (param bits 4-7, jump table 0x00374240), each (relative to g
  when the row's pSync, else plus g's pos, w 1):
  R = gRadius - (r % 101) (gRadius gRadiusRR) / 100
  0  the generator (0, 0, 0, 1)
  1  a circle: R (sin a, -cos a, 0) at a = (r << 8) & 0xff00, or with dType 3
     regularlyOfst + pDirc + (pNum << 16) / (int)pGenRate
  2  a sphere: z R sin b, then R cos b (sin a, -cos a) at b = r % 28672 -
     12288, a as 1
  3  Polyhedron82Table[(r >> 5) % 42] * R
  4  the line pos to pos2: its direction times temp = |line| (r & 0x3f00) /
     16128 (dType 3: |line| pNum / (1 + pGenRate))
  5  a ring about that line: normal2Angle of its direction turns (sin a, 0,
     -cos a, 1) of radius gRadius - gRadius (int)(gRadiusRR (r % 101)) / 100,
     plus the line's direction times t as in 4; synced also rotate[1] =
     rand() & 0xfc00, ofstR = 100 - that percentage, ofstD = 100 t / |line|
r = rand() >> 3
which way, by dType (param bits 8-11), dirc normalised, turned by g's mat:
  0, 5  the cone: z sin e, then cos e (sin d, -cos d) with
        e = pRange - pRangeRR / 2 + pRangeRR (r & 0xff) / 256,
        d = pDirc - pDircRR / 2 + pDircRR ((r >> 12) & 0xff) / 256;
        5 also rot = normal2Angle(dirc)
  1     out from the generator (rType 5: from the point on the line)
  2     in to it
  4     anywhere up: z sin e, cos e (sin d, -cos d), e = (r & 0x7f00) -
        16384, d = (r >> 5) & 0xff00
  3, 6+ none
  speed = pIV - (s % 101) (pIV pIVRR) / 100 (s r, or r >> 16 for 1 and 2);
  velocity = dirc speed
r = rand() >> 3: pRotRnd and a sprite: rotate[3] = r & 0xfff0
r = rand() >> 3: lifeTime = (int)(pLife - (r % 101) (pLife pLifeRR) / 100)
layer = g's; strFlag = g's
```

`normal2Angle(out, v)` (0x00155210) is `(-atan2f(v.z, v.x^2 + v.y^2), 0,
atan2f(v.x, -v.y), 1)` - the first angle's denominator is not rooted.

## ccParticle (0xb0 bytes)

```
+0x00  bits  0-2 style (signed; 0 ccEff, 1 ccClump, 2 ccAnm)  3 distSW
             4 dispSW  5 endFlag  6 killFlag  7 syncFlag
             8-11 fadeFlag (signed; 0 in, 1 shown, 2 out, 3 gone)
             12-15 anmType (signed)  16-19 strFlag
+0x04  ccParticleGenerator *gene
+0x08  void *pChar       its ccEff / ccClump / ccAnm
+0x0c  ccLayer *layer
+0x10  u32 sn            particleSerialNum++ (gp 0x00378aac)
+0x14  u16 anmPat        a sprite's pattern
+0x16  short texID       -1: the slot is free
+0x20  float pos[4], rot[4], offset[4], dirc[4], scale[4], velocity[4]
+0x80  float transparency, speed, size, temp
+0x90  short color       0-2048, times transparency / 2048 when drawn
+0x92  short fadeInD, fadeOutD, lifeTime, age, cnt
+0x9c  u16 rotate[4]     [1] the force fields' spin, [3] a sprite's turn
+0xa4  short ofstD, ofstR
```

`Setup(gene, pTex, patNum, distCheck)` (0x001bf0f0): gene; dispSW 1;
distSW `distCheck != 0`; strFlag 0; killFlag `gene->killFlag != 0` and
syncFlag gene's pSyncFlag (both 0 without a gene). The texture id picks a
`particleTbl` row and a CLUT through a switch (jump table 0x00374260 on
`pTex - 91`, 154 cases):

| texture ids | row | CLUT (`particleCcsAdrs[id]` swapped in) |
| --- | --- | --- |
| below 91, 91-100, 237 | the id | none |
| 101-108, 109-117, 118-124, 125-131 | 0, 1, 2, 3 | ids' own (`CLT_x000c*` ...) |
| 132-133, 134-140, 141-142, 143-147, 148-149, 150-153, 154-155, 156-158, 159-160 | 4, 5, 6, 7, 8, 9, 10, 11, 12 | ids' own |
| 161-167, 168-174, 175-181 | 32, 33, 35 | ids' own |
| 182-183, 184-185, 186-187, 188-189, 190-196, 197-203, 204-209 | 38, 39, 40, 41, 42, 43, 44 | ids' own |
| 210-216, 217-223, 224-230, 231-236, 238-242, 243-244 | 46, 47, 50, 57, 59, 84 | ids' own (231-236 against 143, 238-242 against 237) |

With `patNum` one `rand()`: `pTex += rand() % patNum` for clumps, anms and
sprites of anm 0 or 1, and the switch is taken again. Then anmPat 0, texID,
style and anmType from the row (`particleTbl[row]`: bits 0-3 style, 4-7 anm,
8-15 pat; `Setup` reads it by the id, so id 237 reads past the table's 101
rows), endFlag and fadeFlag 0. The object: style 0 `new ccEff`,
`Init(particleCcsAdrs[row], 1)`, PRIM's FGE (+0x62 bit 0x20) cleared, and
with a CLUT `ccTex.clutChunk` (+0x3c) = `particleCcsAdrs[id]`; style 1 `new
ccClump`, `Init`, `Duplicate(0x3800)`, `SetFogSw(0)`, with a CLUT
`ChangeClut(particleCcsAdrs[id], particleCcsAdrs[clut])`; style 2 `new
ccAnm`, `SetAnm(chunk, 0)`, `SetFogSw(0)`, the same `ChangeClut`. A row with
`pat` and anm 3 (or a sprite with anm 1-3) starts at `rand() % patNum` of the
sprite. Then `sn`; dirc, rot, offset, velocity 0; scale (1, 1, 1, 1),
transparency 1, speed 0, size 1, color 0; fadeInD/fadeOutD the row's
`pFadeIn`/`pFadeOut` (512/256 without a row); lifeTime 1; age, cnt,
rotate, ofstD, ofstR 0; layer 0; a sprite's transparency 0.

`Main` (0x001bf8d0):

```
killFlag: gene->killFlag 2: fadeFlag 3 ? age = lifeTime : fadeFlag = 2
          gene->killFlag 3: age = lifeTime, fadeFlag = 3
age >= lifeTime and fadeFlag 3: endFlag
endFlag: texID = -1, the object deleted, done
each of gene's force fields: Calc(this) answering 1: lifeTime -1, fadeFlag 2
pos += velocity; pos.w 1
eye = cameraGetPos(camID)
at = ccTransPosFW2LW(pos + offset (+ gene->pos when syncFlag))
  sprite: rotate DEG2RAD(rotate[3]), scaleX/Y scale.x/y, its pos at
  clump, anm: SetMatrix_PosRotZYXScale(at, rot, scale); an anm not ended
    steps (_AnimateForward(frameSpd)) and ends the particle with it
  dispSW = ccCheckCameraDeg(at, 12288)
  distSW: d = |eye - at|; d < 7000: f = (5000 - d) / 2000 clamped to 0-1,
    drawn when dispSW and f != 0 with alpha f (transparency (color / 2048));
    else dispSW 0
  else drawn when dispSW with alpha transparency (color / 2048)
  (ccEff::Draw(anmPat) / ccClump::SetTransparency + Draw / ccAnm +0x88 + Draw)
lifeTime != -1: age++; age >= lifeTime: lifeTime -1, fadeFlag 2
sprite, not ended, anmType 2 or 3: anmPat++; past patNum: 3 back to 0, 2 ends
fadeFlag 0: color += fadeInD, at 2048 fadeFlag 1
fadeFlag 2: color -= fadeOutD, at 0 color 1 and fadeFlag 3
cnt++
```

`MainStr` (0x001c0620) is the in-engine streams' version (not described).

## ccParticleForceField::Calc (0x001bcde0)

A force field is `{param, pos[4], scale[4]}` (0x30); only `param`
(`ccParticleForceFieldParam`, 0x20: +0x00 bit 0 calcType, +0x01 fieldType,
+0x02 forceType, +0x04 u16 rotate, +0x08 float radius, +0x10 float
force[4]) is read. fieldType above 3 does nothing. By forceType (jump table
0x003741f0, 17 cases):

(`dirc`, `pos`, ... are the particle's; `rotate` and `radius` the param's.)

| type | calcType 1 | calcType 0 |
| ---: | --- | --- |
| 0 | pos += dirc force.x | pos += force |
| 1 | speed += force.x, velocity += dirc force.x | velocity += force |
| 2 | rotate[1] += rotate; offset.xy = radius (cos, sin) rotate[1] | offset.xy = t + \|t\| (cos, sin)(rotate[1] + angle of t), t = the generator (or the origin when synced) - pos, t.z 0, after rotate[1] += rotate |
| 3 | clumps: rot.xyz += force.xyz (in 16-bit angles) | the same |
| 4 | pos += force | the same |
| 5 | nothing | |
| 6 | s = force.x + (force.y - force.x) min(cnt, force.z) / force.z; scale = size s (1, 1, 1, 0) times (force.w, 1, 1, 0) when force.w < 1 else (1, 1 / force.w, 1, 0); scale.w 1 | the same |
| 7, 8 | scale.x (7) or .y (8) = size s | the same |
| 9 | scale = force times the squash of 6; w 1 | the same |
| 10 | force.w 0: s from force.x to force.y and back over force.z frames (by cnt); scale = s (1, 1, 1); w 1 | the same |
| 11, 12 | rotate[3] += / = rotate | the same |
| 13 | rot = force; w 1 | the same |
| 14 | pos = normalise(gene pos2 - pos) temp; w 1 | the same |
| 15 | transparency over a cycle of (int) force.x + .y + .z + .w frames: cnt / force.x rising, 1, falling over force.z, 0 | the same |
| 16 | turned about the generator's line: ccSetQuaternion(dir, rotate[1] += rotate) on (ofstR gRadius / 100, -ofstD \|line\| / 100) turned by normal2Angle(dir) | nothing |

Then for forceType 1: force.x < 0 with speed <= 0 stops the particle (speed
0, velocity 0) and answers 1 (it fades out); fieldType 3 ends a particle
within twice its speed of the generator. Every other case answers 0.
`ccSetQuaternion` (0x001c2c30) works in doubles (`sin`/`cos` of half the
angle on libgcc's soft doubles); `ccQuaternionToMatrix` (0x001c2cf0) scales
by `2 / |q|^2`. The table's 148 rows use types 0-3, 6-12 and 14-16 (16 once,
row 139, for generator 236).

## The tables

```
particleGeneratorTbl  0x003402f0  244 x ccParticleGeneratorParam (0x38)
  +0x00  u16  bits 0-3 gType, 4-7 rType, 8-11 dType, 12 gSync, 13 pSync,
              14 pRotRnd
  +0x02  u16  bits 0-3 pPatRnd
  +0x04  short gLife          +0x08 float gRadius
  +0x0c  short pLife, pTex, pDirc, pRange
  +0x14  float pIV, pGenRate, gRadiusRR, pLifeRR, pIVRR
  +0x28  short pDircRR, pRangeRR, pFadeIn, pFadeOut
  +0x30  short ffNum[4]       rows of particleForceFieldTbl, -1 none
particleForceFieldTbl 0x00343850  148 x ccParticleForceFieldParam (0x20)
particleTbl           0x00340220  101 x u16 ccParticleParam
particleEffectTbl     0x00344ad0  49 x ccParticleEffectParam (0x20):
                                  short sync[4], geneNum[4]; float offset[4]
particleCcsAnmTbl     0x003739e0  245 x {char *ccs, *anm}: rows 0-100 the
                                  objects (PARTICLE.CCS, 48-49 DRAIN.CCS:
                                  CMP_x043, EFF_x044), 101-244 CLUTs
Polyhedron82Table     0x0033ef90  42 x float[4]
```

Of the 244 generator rows 182 are gType 0 and 62 gType 2 (none 1); rTypes
0-5 and dTypes 0-4 occur (not 5).

## effect.cpp's static generators

`__sinit_effect.cpp` (0x00375830) constructs four generators with no row -
`ccpgSmoke` (0x003fefd0), `ccpgPawSmoke` (0x003ff0e0), `hitPhotonDummyG`
(0x003ff1f0), `hitPhotonDummyStrG` (0x003ff300), serial numbers 0-3 - that
are never started: `ccEffectCtrl`'s constructor gives them force fields
(`ccpfSmoke1-3` = `ccpffpSmoke1-3`, 0x0033fac0 on; `ccpfSmoke1, 2, 4`; new
fields of `hitPhotonDummyF1/F2`, 0x0033fed0 and 0x0033fef0) and effect code
names them as a particle's generator (`effSmoke` passes `ccpgSmoke` to
`ccParticleSetup`, `ccEffPawSmoke` `ccpgPawSmoke`, the hit photon sets
`hitPhotonDummyG` into +0x04), so the particles take their force fields
(the smoke's: scale 0.5 growing to 20 over 900 frames, velocity.z -0.02 a
frame, rotate[3] -48 or -128 a frame; the photon's: a speed falling by 0.2 a
frame to a stop, scale 4 to 1 over 30 frames) and
the default fades. A particle of a generator that `ccParticleCtrl::Main` has
deleted keeps reading the freed generator.

## Starters

```
ccParticleHitMark(pos)        0x001bc420  generator 1 at pos
ccParticleHeal(ch, n)         0x001bc4b0  generator 2 on ch's pos (n unread)
ccParticleExplode(p, v, s, t) 0x001bccd0  ccParticleSetup(tex, p, -1, 0, 0),
                                          tex 10, 155, 44 for t 0-2, t + 202
                                          for 3-7 (past 7 the pos pointer);
                                          velocity v, speed fabs(|v|) in
                                          doubles, scale.x/y s
ccParticleSetup(t, p, life, pat, gp) 0x001c2940  GenerateParticle,
                                          Setup(gp, t, pat, 1), pos p,
                                          lifeTime life, pNum++, p2Num++
                                          (gp's pCnt is not raised)
startParticleEffect(ch, num)  0x001c2560  each geneNum of particleEffectTbl
                                          [num]: a syncing row follows ch's
                                          pos, offset z (+ height for sync 2,
                                          + height / 2 for 3); else placed at
                                          ch's pos, z + offset
startParticleEffect2(s, e, num, sw) 0x001c2770  the same rows between s
                                          and e (syncPos / syncPos2, or pos /
                                          pos2 read once), syncSW sw,
                                          killFlag 1 (ccSpcChar's arms,
                                          +0x160-0x19c and +0xe4)
```

`ccParticleCritical` (0x001bc540), `ccParticleDying` (0x001bc6d0),
`ccParticleNoDamage` (0x001bc8c0), `ccParticleAttributeCritical`
(0x001bca90) and `ccParticleAttributeGuard` (0x001bcc40) start effects as
well and are described with the hits ([the field's effects](effects.md)).

## ccConditionEffect (0x20 bytes)

```
+0x00  ccParticleGenerator *gene[5]
+0x14  ccEffect *eff; +0x18 u32 effSN; +0x1c int now
```

`ccChar::DispConditionEffect` (gcmn) calls `setConditionEffect(cp)`
(0x001c24a0: `new ccConditionEffect(cp, cp->+0x30)`). The constructor
(0x001c13f0) starts generators on the character's pos, each marked killFlag 1
(h its height):

| num | generators (row: texture, lift) |
| --- | --- |
| 0 | 197, h; 198, h/2 |
| 3 | 197: 17, h; 198: 144, h/2 |
| 5 | 199, h; 198: 145, h/2 |
| 1 | 198: 158, h/2; 198: 143, h/2 |
| 2, 4, 6, 22 | 200 then three of 201 (22: 202), texture 16 (4: 18, 6: 36), h; 198 as gene[4]: 145 (6: 147), h/2 |
| 13-18 / 29-34 | 31 / 37 and 204 / 203, h/2, textures 107/202, 0/198, 102/199, 104/43, 103/200, 105/201 |
| 20, 21 | 205: 119 / 118, h, as gene[4]; then as the default |
| -1 | 209 twice with force fields 105/107 and 106/108, h; 210, 50 + h |
| others | particleEffectTbl[num] as startParticleEffect (sync 2 only) |

then `effAbilityDown(cp, num)` for 0-18, `effAbilityUp` for 20-34 (a jump
table at 0x003744d0), keeping the effect and its serial number.
`killConditionEffect` / `deleteConditionEffect` (0x001c24f0, 0x001c2520) set
`now` 0 / -1 and delete it: the destructor (0x001c2360) kills (killFlag 2)
or deletes (3) each generator still on a list and ends the effect when its
serial number still matches.

## The port

`crates/piney-effect/src/particle.rs` and `particle/`: `Particles` is
`ccParticleCtrl` with particle.cpp's statics (the 2000 slots, the serial
numbers, `p2Num`, `strEffectStopFlag`); `Effects::step_particles` runs its
`Main` and records the draws (`DrawRec`s on each particle's layer, in the
order sent; `Effects::draw` renders the effects' then the particles').
Generators are named by their serial number (`GenRef`), what they follow by
`VecRef` (character pos/dirc, a vector `off` bytes into a character, an
effect's pos/rot/posT) and `IntRef` (an int `off` bytes into a character,
or an effect's `temp[k]`, which `effDrain`'s orbs clear to stop their
trails), read each frame.
The tables are read from the executable at start (`Assets::particle`),
`Setup`'s texture switch decoded from its code. `Effects::new` installs the
four static generators. The doubles of `ccSetQuaternion` are fdlibm's `sin`
and `cos` in host doubles (fp-bit rounds like them). A deleted generator a
particle still names is kept until none does.

## Checks

`tools/test_effect_particle_rs.py` runs the game's particle.cpp in eemu
beside the port (`examples/particle_probe.rs`): `ccParticleCtrl` built by its
own constructor, effect.cpp's static constructors run, every generator and
starter made by the game's functions, the real `ccParticleCtrl::Main` each
frame. After every request both sides answer every generator's fields, every
live particle's fields and its object's (the sprite's scale, turn, colour,
transparency, flag, PRIM, TEST, pos; the clump's or anm's matrix, CLUT swap,
animation time), the frame's draws in order with their layer, the effect
slots, the control's counts and serial numbers and `rand()`'s state:

- the arrival's generator (row 82 on a character, 180 up), towns and a field;
- every row of `particleGeneratorTbl` with random settings (following
  characters or not, positions, turns, layers, texture overrides, killFlag,
  paused, killed or deleted part way);
- rows and force fields patched in both tables to reach gType 1, dType 5,
  rTypes and dTypes past the switches, force types 4, 5, 13, 16, 17 and
  field types past 3;
- more particles than the 2000 slots;
- the starters, particles counted to the static and to started generators,
  and `ccConditionEffect` for conditions -1 to 48, killed or deleted;
- smoke and dust: `effSmoke` and gcmn's `ccEnemyEffDust` and both
  `ccEnemyEffDustRing`s (a character's, turned by its heading), with
  `genrand()` (their `ccRandF`) as MT19937 on both sides
  ([the effects](effects.md#smoke-and-dust));
- `ccEffPawSmoke` a step a frame, its feet nodes and ground stubbed: either
  foot lower, every kind of ground.

15,898 frames in all now; before the dust was added: 14,359 frames (19,768 compared states: 117,059 generator states, 1,261,884
particle states, 1,021,465 draws - 1,005,325 sprites, 842,800 of them with a
swapped CLUT, 15,583 clumps, 557 anms - and 940 effect slots), 0 mismatches.

## Unknown

- `ccParticle::MainStr` (0x001c0620): the streams' particles (strFlag); the
  port runs `Main` for them.
- forceType 10 with a force.w not 0 scales by whatever f20 held (no row has
  one); the port uses 1.
- A particle of a generator that was deleted reads freed memory; the port
  keeps the generator as it was, which matches until the heap reuses it.
- `ccParticleExplode` past type 7, texture id 237 (no row), and
  `Setup`'s behaviour for styles past 2 read beyond their tables.
- What each generator row is for (which spell or mark) beyond the callers
  named here.
