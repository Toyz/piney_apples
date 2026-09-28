---
title: Carmina Gade, Fort Ouph and Lia Fail - ROOTTOWN03, 04 and 05
status: partial
volumes: MUT, OUT
covers: MUT gcmn.prg:0x0043b980 ROOTTOWN03::ROOTTOWN03, 0x0043e400 ROOTTOWN03::Draw, 0x0043cb30 DrawBG, 0x0043ce60 DrawObj2, 0x0043cdc0 DrawObj, 0x0043cf00 DrawFloor, 0x00388f60 ROOTTOWN03 vtable, 0x0043af10 AIRSHIP::AIRSHIP, 0x0043b0c0 AIRSHIP::Move; OUT gcmn.prg:0x00439c50 ROOTTOWN04::ROOTTOWN04, 0x0043c430 ROOTTOWN04::Draw, 0x0043aa40 DrawBG, 0x0043af40 DrawObj2, 0x0043ae50 DrawObj, 0x0043b030 DrawFloor, 0x003842d0 ROOTTOWN04 vtable, 0x0043c5e0 ROOTTOWN05::ROOTTOWN05, 0x0043e8e0 ROOTTOWN05::Draw, 0x0043d200 DrawBG, 0x0043d410 DrawObj2, 0x0043d370 DrawObj, 0x0043d4b0 DrawFloor, 0x00384300 ROOTTOWN05 vtable
worklog: 97, 198, 218, 219, 260
---

# Carmina Gade, Fort Ouph and Lia Fail - ROOTTOWN03, 04 and 05

The Root Towns the later volumes add, each built by `WORLD_MAN::GO(0)` for
its `game.town` and drawn by `ccThFieldDisp` each frame, as Mac Anu
([the field game](field-game.md#what-the-town-draws)) and Dun Loireag
([Dun Loireag](town02.md)) are. Infection's executable carries earlier
`ROOTTOWN03`-`05` classes that no disc reaches; these are the later
volumes'. The ports are `crates/piney-world/src/town03.rs`, `town04.rs`
and `town05.rs`.

## Carmina Gade (MUT)

`ROOTTOWN03` (MUT gcmn town03.cpp), the third Root Town (`game.town` 2),
new in Mutation.

```text
ROOTTOWN03()  the map's layers: WORLD_MAN +0x490 (50), +0x1b4 (40) over
  0x0043b980  (340, 36) 160 x 160, +0x1b8 (45) the screen; town03
              (town03d in crisis); the AIRSHIP (town03Ship); SetFog(3000,
              15000, 0, 85, 0x0014140c) and ccSys.bgColor the same; 34
              STATICMODELs (RT_MODELTABLE03), no STATICOBJECTs; the
              clumps CMP_sr3bac1, _bac2, _mou1, _mou2 (+0x1c8..+0x1d4),
              in crisis CMP_sr3dat1_1-3 (+0x74..+0x7c), each SetFogSw(0);
              the map's four sprites (TEX_sr3map1 three times, xallow0);
              the lights from ANM_sr3bac1a (town03Light: LGT_sr3lig1,
              LGT_omni01-11); BLT_bg, _obj, _obj2, _floor; three ccAnms
              of ANM_sr3wat1a, SetFogSw(0), OBJ_sr3wat00 duplicated
              (8200, 8192, 8192), rooted at the origin and stepped once;
              a 128 x 128 ccTexChunk (+0x1f4) swapped into water 0's
              model
Draw()        cameraGetPos; on effLayer waterUVModifi2 of water 0 (reach
  0x0043e400  32000); on objLayer water 0 stepped and drawn, then
              MakePacketDrawBuffTrans into its texture; the airship's
              Move and its ccAnm's Draw; DrawBG; on obj2Layer DrawObj2,
              DrawObj, DrawFloor, DrawWithOutFog of rows 29-33 and of the
              type-0 rows below 29 (24-28), DrawMap; u$1875 += 0.005
              (back to 0 at 1, read by nothing)
DrawBG()      v$1474 += (0.001, 0.002), each back to 0 past 1; the four
  0x0043cb30  clumps at the origin on bgLayer[0]-[3] (-100 .. -70); in
              crisis MAT_sr3dat1_2's U and _3's U and V from vftoi12 of
              the second, CMP_sr3dat1_1-3 on bgLayer[4]-[6] (-60, -50,
              -45)
DrawObj2, DrawObj  0x0043ce60, 0x0043cdc0: the rows of type 3, 2
DrawFloor     0x0043cf00: rows 0-7
```

Water 1 and 2 are made and stepped once but never drawn. The BLT chunks
(`ccBltGrpChunk::LockBlt`, `MakePacketLoadData`) load the textures into
the GS, which the port's renderer does not need.

The airship (`AIRSHIP`, 0x0043af10, `Move` 0x0043b0c0) flies a round of
six legs between the dummies `DMY_marker69`-`72`: 69 to 70, 70 to 71,
71 to 72, then back. A leg takes 2000 frames (its `t` += 0.0005), then
the ship turns for 300 frames (`dirc.z` by pi/300 or pi/600 a frame, to
the leg's heading), then waits 300 frames. It puffs smoke from its two
chimneys while it flies and while it waits, and sounds (SE 257) as each
puff comes while it waits. The puffs draw from `fieldrand`.

## Fort Ouph (OUT)

`ROOTTOWN04` (OUT gcmn town04.cpp), the fourth Root Town (`game.town` 3,
the server Sigma), new in Outbreak. Quarantine's is the same.

```text
ROOTTOWN04()  the map's layers; town04 (town04d in crisis); SetFog(1500,
  0x00439c50  10000, 0, 30, 0x00c8f0fb) and ccSys.bgColor the same; 16
              STATICMODELs (RT_MODELTABLE04) and 1 STATICOBJECT
              (RT_OBJTABLE04); the clumps CMP_sr4bac1 (+0x228),
              CMP_sr4clo1_1, _1_2, CMP_sr4clo2 (+0x22c..+0x234),
              CMP_sr4sun1 (+0x238), each SetFogSw(0); in crisis
              CMP_sr4dat1_1-3 (+0x74..+0x7c); the map's sprites
              (TEX_sr4map1 three times, xallow0); the lights from
              ANM_sr4bac1a (town04Light: LGT_sr4lig1, LGT_sr4omn1-5);
              BLT_bg, _obj, _obj2, _floor; town_z (+0x1c0), 25 CLOUDs
              (+0x1c4) and its LENSFLARE (+0x1bc)
Draw()        the clouds' Init on the first call (+0x04); cameraGetPos;
  0x0043c430  on objLayer DrawBG, DrawObj2, DrawObj, DrawFloor, DrawMap;
              on effLayer LENSFLARE::Draw(town04, DMY_sr4lig1point, 0),
              then each cloud's Move and Draw
DrawBG()      v$ (gp 0x00386ed0) += 0.01, v2 (0x00386ed4) += 0.002, each
  0x0043aa40  back to 0 past 1; MAT_sr4clo1's and _2's U offsets vftoi12
              of the first less their crops; the sky on bgLayer[0]
              (-100), the sun at DMY_sr4lig1point on bgLayer[1] (-90); in
              crisis MAT_sr4dat1_2's U and _3's U and V from the second,
              CMP_sr4dat1_1-3 on bgLayer[2]-[4] (-80, -70, -60); the three
              cloud clumps on bgLayer[5] (-50)
DrawObj2, DrawObj, DrawFloor  0x0043af40, 0x0043ae50, 0x0043b030: the
              rows of type 3, 2, 1 (and the static object of types 3
              and 2)
```

Unlike Dun Loireag it has no water, draws no type-0 rows, puts its
cloud layers on `bgLayer[5]` rather than effLayer and scrolls them in U.
The BLT chunks only load textures into the GS. The clouds and the lens
flare are Dun Loireag's (`CLOUD`, `LENSFLARE`, `town_z`).

## Lia Fail (OUT)

`ROOTTOWN05` (OUT gcmn town05.cpp), the fifth Root Town (`game.town` 4,
the server Omega), new in Outbreak. Quarantine's is the same.

```text
ROOTTOWN05()  the map's layers; town05 (no crisis file); SetFog(1500,
  0x0043c5e0  10000, 0, 75, 0x00f0c080) and ccSys.bgColor the same; 7
              STATICMODELs (RT_MODELTABLE05), no STATICOBJECTs; the sky
              CMP_sr5bac1 (+0x4c4), SetFogSw(0); the map's sprites
              (TEX_sr5map1 three times, xallow0); the lights from
              ANM_sr5bac1a (town05Light: LGT_fdirect01, LGT_omni01-19);
              BLT_bg, _obj, _obj2, _floor; town_z and six ccEffs of its
              lens flare (EFF_sflenz_1-6, never drawn); 19 ccEffs of
              EFF_se1_1ef1 (+0x1b4), each with a pattern fieldrand(patNum)
              (+0x330), a wait fieldrand(100) + 30 (+0x37c) and a flicker
              fieldrand(15) + 1 (+0x3c8), at DMY_omnpoint01-19 (+0x200)
Draw()        DrawBG; on objLayer DrawObj2, DrawObj, DrawFloor, DrawMap,
  0x0043e8e0  the type-0 rows; on effLayer each omni point's glow: while
              it waits, its pattern drawn and stepped (back to 0 at
              patNum); then it flickers, a pattern fieldrand(patNum) a
              frame for its count, and waits again (both drawn anew)
DrawBG()      three scrolls stepped (0.01, 0.02, 0.001, each back to 0
  0x0043d200  past 1) that nothing reads; the sky at the origin on
              bgLayer[0] (-100)
DrawObj2, DrawObj, DrawFloor  0x0043d410, 0x0043d370, 0x0043d4b0: the
              rows of type 3, 2, 1
```

The BLT chunks only load textures into the GS.

## Unknown

- The BLT chunks' GS uploads (`ccBltGrpChunk::LockBlt`,
  `MakePacketLoadData`) are not described; the port's renderer does not
  need them.
