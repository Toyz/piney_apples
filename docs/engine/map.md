---
title: The minimap - town, field and dungeon maps, the map button and ShowMap
status: partial
volumes: INF, MUT
covers: INF SLUS_202.67:0x001a4430 ccThFieldDisp, 0x001a3fd0 WORLD_MAN::ChangeMapMode, 0x001a3b70 WORLD_MAN::SetMapAlpha, 0x001a3b80 WORLD_MAN::GetMapAlpha, 0x001a3ee0 WORLD_MAN::ShowMap, 0x0015a960 ccSprite::SetPrim, 0x0015ae80 ccSprite::MakePacket, 0x0015aed0 ccSprite::MakePacketStr, 0x0015c4b0 ccMask::ccMask, 0x0015c690 ccMask::MakePacketS, 0x0015c5e0 ccMask::SendPacketS, 0x00102190 ccRotate, 0x00104ae0 ccView::SetFrame, 0x001538d0 ccHitCheckLM, 0x00129d08 fptoui; INF gcmn.prg:0x00517800 ccThGameCtrl, 0x00421470 ROOTTOWN01::ROOTTOWN01, 0x00422d00 ROOTTOWN01::DrawMap, 0x005d4ad0 RT01ICONPOS, 0x004240c0 ROOTTOWN02::ROOTTOWN02, 0x00425760 ROOTTOWN02::DrawMap, 0x005d4e90 RT02ICONPOS, 0x005a4cd0 WORLD::Init, 0x005a6da0 WORLD::Generate, 0x005a97b0 WORLD::Draw, 0x005a3940 WORLD::DrawMiniMap, 0x005a3170 WORLD::DrawHeightOnMiniMap, 0x005a1b60 WORLD::DrawKeyObjectOnMiniMap, 0x005a1ff0 WORLD::DrawSubObjectOnMiniMap, 0x005a2510 WORLD::DrawLakeOnMiniMap, 0x005a2840 WORLD::DrawCircleOnMiniMap, 0x005a2c60 WORLD::DrawDungeonOnMiniMap, 0x005ad990 WORLD::ShowMap, 0x0042e0c0 ccCheckFountain, 0x00658680 KeyIconTBL, 0x006586b0 SubIconTBL, 0x006586e0 BaseIconTBL, 0x00658710 TreeIconTBL, 0x006584b0 FP_DUNGEON, 0x006571f0 fieldminimap, 0x005b7c00 DUNGEON::DUNGEON, 0x005ce930 DUNGEON::Draw, 0x005cd850 DUNGEON::MakeMiniMap, 0x005cc160 DUNGEON::DrawMap, 0x005cf260 DUNGEON::ShowMap, 0x005c7c30 DUNGEON::SetDoor, 0x0042e010 ccCheckActiveObject, 0x00695550 levelstr; INF SLUS_202.67:0x003782f8 mapmsg
---

# The minimap - town, field and dungeon maps, the map button and ShowMap

The map at the top right of the field's screen: a Root Town's plan with
its shop signs (Mac Anu's and Dun Loireag's are ported), a field's height map with its objects and the dungeon's red
arrow, a dungeon's floor as the party has seen it. `ccThFieldDisp` (main
0x001a4430, priority 96) draws it each frame inside the area's own draw;
`ccThGameCtrl` (gcmn 0x00517800) switches it with the map button; the field
UI fades it; the events' `map_on` and `show_map` bring it back and fill it
in. The port is `piney-world`'s `map/`.

```text
a town      ROOTTOWN01::Draw -> DrawMap (vtable +0x1c)     gcmn 0x00422d00
            ROOTTOWN02::Draw -> DrawMap                    gcmn 0x00425760
a field     WORLD::Draw -> DrawMiniMap, unless inBattle     gcmn 0x005a3940
a dungeon   DUNGEON::Draw -> MakeMiniMap(here), then        gcmn 0x005cd850
            DrawMap, unless inBattle                        gcmn 0x005cc160
```

`ccGame.inBattle` (+0x58) keeps the field's and dungeon's map away during a
battle. Every map draws on `mapLayer` (`WORLD_MAN` +0x490), a layer of
priority 50 with a view of its own the area's constructor frames, and some
on the font layer (240, the whole screen).

## The map button and the modes

`WORLD_MAN` keeps a mode per kind of area, `townMapMode` (+0x128),
`fieldMapMode` (+0x12c) and `dungeonMapMode` (+0x130). `ccThGameCtrl` reads
the three from `saveData.mapMode[3]` (+0x842d, signed bytes) when it starts
(0x00517854), so they live in the save.

Each frame, once the party is not wiped out, `ccCheckGtHackAnm()` is 0 and
`menuClrWait` (2 after an event's `menu_clear`, counted down a frame at a
time) is out, the map button is the task's first test (0x00517d84), before
`ccPlayerMenuCheck` and whatever menu is open:

```text
ccSys.pad push & saveData.assignPADmap (+0x840c, select)
    and ccEvent::CheckOperate(13, 0)
  WORLD_MAN::ChangeMapMode      by WORLD_MAN.flag (+8): 0 townMapMode + 1,
    (main 0x001a3fd0)           2 -> 0; 1 fieldMapMode + 1, 3 -> 0; 2
                                dungeonMapMode + 1, 2 -> 0
  saveData.mapMode[game.area]   = the mode of game.area (+0x14)
```

| area | mode 0 | mode 1 | mode 2 |
| --- | --- | --- | --- |
| town | the map | none | - |
| field | the default map (4 px a cell, objects) | the whole map (2 px a cell) | none |
| dungeon | the map | none | - |

## The fade

The field UI fades `ccMenu.mapStatus` (+0x10) into `mapAlpha` (+0x48, by
15 to 128) and hands `WORLD_MAN::SetMapAlpha(mapAlpha / 128)` (main
0x001a3b70: `mapAlpha` +0x148) every frame (`docs/engine/field-ui.md`); each
map sets every sprite's `transp` from `WORLD_MAN::GetMapAlpha` before it
draws. `menu_ban` sets `mapStatus` 0 (the map fades out), `menu_clear` and
`map_on` 1, the shop and Grunty menus 3 while they are open.
`ccThMenu` (34) runs before `ccThFieldDisp` (96), so a frame's map has the
frame's own alpha.

## The sprites

A map's sprites are `ccMask`s (`ccMask::ccMask(m, ms)`, main 0x0015c4b0): a
TF sprite of `m` packets (`SetPrim(m, 0)`) holding at +0xd0 a second sprite
of `ms` packets, a TRF one (`SetPrim(ms, 1)`: textured, rotated, flat), when
`ms` is not 0. `SetPrim`'s types (0x0015a960, its jump table at
0x0034b300):

| type | GIF tag | ctrl | prim |
| --- | --- | --- | --- |
| 0 TF | `ccSpriteGIFtagTF` | 0x01 | SPRITE, a qword TEX0 RGBAQ, then UV XYZ2 UV XYZ2 |
| 1 TRF | `ccSpriteGIFtagTRF` | 0x0b | TRISTRIP of 4, UV XYZ2 four times |
| 2 TG, 3 TRG | ...TG, ...TRG | 0x0d, 0x0f | Gouraud |
| 4 F, 5 RF, 6 G, 7 RG | ... | 0x00, 0x0a, 0x0c, 0x0e | untextured |

`ccSprite::MakePacket(code, t)` (0x0015ae80) is `MakePacketStr` of `{code,
t ? 0xff : 0}`: with `t` 0 a code of 0 draws nothing (every map passes `t`
1). `ccMask::MakePacketS(code, t)` (0x0015c690) copies the mask's layer,
TEX0/TEX1, `wi`, `wu` and `wv` cut to whole texels, `su`, `sv` from
`fptosi(sy)` (not `sv`), `sx`, `sy`, `dx`, `dy`, the colour, `transp`,
`rot`, `cx`, `cy` and ctrl 0x10, 0x20, 0x40 into the second sprite and
calls its `MakePacketStr`. `SendPacketS` (0x0015c5e0) sends the second
sprite, then the mask; a send puts the queue at the front of the layer, so
the last sent is drawn first.

`MakePacketStr` (0x0015aed0) for a cell, in 12.4 (`layer_screen` of the
sprite's layer's view):

```text
TF    (x, y) = layer_screen(dx + cx, dy + cy); (w, h) = FTOI0(sx ls00, sy ls11)
      culled when x > 0x9000, x + w < 0x7000, y > 0x8e00 or y + h < 0x7200
      a SPRITE (x, y) - (x + w, y + h)
TRF   M = RotZ(-rot) (sceVu0UnitMatrix, sceVu0RotMatrixZ); o = M (cx, cy) +
      (dx, dy); ex = M (sx, 0); ey = M (0, sy)
      corners o, o + ey, o + ex, o + ex + ey, each through layer_screen;
      the strip dropped when all four are off the screen (the test reads a
      width and height the unrotated path would have left in the scratch
      area; any corner on the screen passes)
UV    u0 = (code % wi) su16 + wu, v0 = th16 - wv - (code / wi) sv16 - 1 (th
      the texture's 1 << TEX0.TH rows); the far corner u0 + su16, v0 - (sv16
      - 1); ctrl 0x20 swaps the u's less one, 0x40 the v's
RGBAQ the colour words' low bytes, A = __fixunssfdi(alpha * transp)
```

Textures are stored bottom-up, so `v0` near the texture's height is its
first row in memory.

## Mac Anu: ROOTTOWN01::DrawMap

The constructor (gcmn 0x00421470) makes `mapLayer` (`ccLayer::Init(50, 0)`,
`SetFrame(340, 36, 160, 160, 80, 80, 1, 1)`: 160 x 160 at (340, 36)) and
three masks, and sets `mapw` 140, `maph` 256, `stepw` 1.55, `steph` 1.6
(+0x7c .. +0x88):

```text
+0x10 map[0]  ccMask(128, 32) on mapLayer   town01 (town01d)::TEX_sr1map1
+0x14 map[1]  ccMask(128, 32) on fontLayer  the same texture
+0x18 map[2]  ccMask(6, 6) on mapLayer      xallow0::TEX_xallow0
```

`DrawMap` (0x00422d00):

```text
townMapMode 1: nothing
the pulse     ccRotate(r, 0.3134) (r$1560, one step a frame, wrapping at
              pi); alpha = min(fptosi(64 cosf r) + 80, 128)
transp        the three masks' = GetMapAlpha()
the scroll    cy = (maph - 160) / 2 (48); x = pos.x / 100 * stepw, y =
              -pos.y / 100 * steph; addy = fptosi(y) held within -cy .. cy,
              the rest of y left to the arrow (y - cy or y + cy at a stop,
              else 0); top = cy + addy
map[0]        140 x 160 texels from row top (wv = top << 4) at (0, 0), alpha
              128
map[2]        the arrow, MakePacketS: 16 x 16 at (68 + x, 4 + (80 + y)),
              centre (-8, -10), rot = the player's heading (dirc.z), alpha
              the pulse
map[1]        for each of RT01ICONPOS's six signs (gcmn 0x005d4ad0, ICONPOS:
              x, y, the label's cell u, v, where the balloon and label were
              last drawn, alpha):
              top < y: the balloon, 54 x 59 at u 164, at (340 + x, 36 + y -
                cy - addy - 59) right of the middle (mapw / 2 < x), else at
                (340 + x + 6 - 54, ...) mirrored (ctrl 0x20); its label, 25 x
                24 at (u, v), 15 right and 14 down of it (6 more left of the
                middle); the places kept; alpha + 4 to 128
              else, alpha not 0 and drawn once: both again where they were
                last drawn, alpha - 4 to 0; otherwise alpha 0
map[0]        MakePacketS twice: the shades over the map's top and bottom,
              24 x 140 at u 140, at (71, 12) turned -pi/2 and (71, 148)
              turned pi/2, centre (-12, -70); alpha 128, the top one (top
              128) / 10 when top < 10, the bottom one (cy - addy) 128 / 10
              when addy >= 39
send          map[2], map[0], map[1] (SendPacketS each)
```

`RT01ICONPOS` is a global that starts each sign at alpha 128 and never
unset (-1) places, and the game keeps it from visit to visit.

## Dun Loireag: ROOTTOWN02::DrawMap

`ROOTTOWN02`'s constructor (gcmn 0x004240c0) makes the same layer and
three masks, from `town02` (`town02d`)`::TEX_sr2map1`, with `mapw` 140,
`maph` 256, `stepw` 1.28 and `steph` 1.2921; but its arrow's mask (+0x18)
is on the font layer, not `mapLayer`. `DrawMap` (0x00425760) is Mac Anu's
but for three things:

```text
the pulse   r$1488 (its own static)
map[2]      the arrow in screen pixels: at (68 + (340 + x), (116 + y) -
            14); (ax, ay) = fptosi of each
map[1]      a sign in view (top < y) draws its balloon and label at alpha
            64, not its own, while the arrow's point is inside the
            balloon's box: tmp.x <= ax <= tmp.x + 54 and tmp.y + 10 <= ay
            <= tmp.y + 49 (tmp: where the balloon is drawn); its own alpha
            still steps by 4
signs       RT02ICONPOS (gcmn 0x005d4e90), six of them
```

The sends, the scroll, the shades and the fading out are Mac Anu's.

## From Mutation: Dun Loireag and Carmina Gade

From Mutation on (MUT gcmn: `ROOTTOWN02` 0x00437e70 / `DrawMap`
0x00439740, `ROOTTOWN03` 0x0043b980 / 0x0043cfb0) the town class keeps
its map part 4 bytes further on (`mapw` +0x80 .. `steph` +0x8c). Mac
Anu's map is otherwise Infection's. Dun Loireag's and Carmina Gade's
(`game.town` 2) change:

```text
ROOTTOWN0n()  two layers of the class's own: +0x1b4 of priority 40 with
              mapLayer's frame, +0x1b8 of priority 45 over the screen
              SetFrame(0, 0, 512, 384, 256, 192, 1, 1)
              map[0] ccMask(128, 32) on +0x1b4
              map[3] ccMask(128, 32) on +0x1b8, the same texture
              map[2] (the arrow) on mapLayer
              Dun Loireag: town02(d), stepw 1.28, steph 1.2921
              Carmina Gade: town03(d)::TEX_sr3map1, RT03ICONPOS, stepw
              1.02 (0x3f828f5c), steph 1.013 (0x3f81a9fc)
DrawMap()     Carmina Gade: (x, y) move by (16, 46) before the scroll
              the arrow at (68 + x, (80 + y) - 14), Carmina Gade's at
              (70 + x, (80 + y) - 14); Dun Loireag still dims a sign
              under the arrow's point in screen pixels
              while the flag race runs in the town (0x005ff790: the
              area 0 and the race's state 0x0038bd44 set), each of its
              three racers (0x00774a90) with its place inside the map's
              view: map[3], 24 x 24 at (3712, 144 + 24 i), at 12 + ((420
              + px) - 24), y - 12; alpha 128 fade (+0x1ec, 0x005fd100),
              12 fade per pixel within 11 of the view's top or bottom
              and then no sign draws (each still fades in or out)
              Carmina Gade's sign 3 draws its balloon mirrored and
              flipped (ctrl 0x60), and left of the middle its label 6
              lower; no sign dims
              SendPacketS: map[2], map[3], map[0], map[1]
```

## A field: WORLD::DrawMiniMap

`WORLD::Init` (gcmn 0x005a4cd0) makes `mapLayer` framed at (340, 28)
(`SetFrame(340, 28, 160, 160, 80, 80, 1, 1)`) and:

```text
+0x6134 minimap   ccMask(512, 64) on mapLayer: fieldccs[type] (isHacked 2,
                  else fieldccs2)::fieldminimap[type] - TEX_sfamap .. TEX_sfpmap
                  (gcmn 0x006571f0), 256 x 256 PSMT8
+0x6138 minimap2  ccMask(16, 16) on mapLayer: xallow0::TEX_xallow0
+0x6174 kanji     ccInitKanji(16, 0) (kt 0, 64 rows, 16 packets), ctrl 0x10,
                  on mapLayer
```

`WORLD::Generate` (0x005a6da0) starts `water` (+0x30) at 0 and ends by
painting the height map into the texture's texels (0x005a7c3c - 0x005a7d1c):

```text
texel (79 - x, 255 - y) = (char) fptoui(FIELD::GetHeight(x, y) * 0.1240234375) + 128
```

for every height cell (x, y) of the 80 x 80 map (`FIELD.map[x][y]`, +0xaf02c;
`fptoui` gives 0 for a negative), an index into the texture's own palette.
The rest of the texture holds the icons. On the map x runs right to left.

`DrawMiniMap` (0x005a3940), statics `r$3072` and `alpharate$3075` (0.7):

```text
fieldMapMode 2: nothing
transp        minimap's, minimap2's and the kanji's = GetMapAlpha()
the label     the kanji at (10, 6), ccSpriteColorTable[7]: Disp("Overall
              Map") in mode 1, else "Default Map" (mapmsg, main 0x003782f8)
the pulse     as the town's
the place     px = fptosi(x / 300), py = fptosi(y / 300); s0 = 80 - px;
              subx = 2 s0, suby = 2 (py - 80); xpos = ypos = (-80, 80); if
              xpos[0] - subx >= 0 then xpos[1] = -240, else if it is below
              -159 then xpos[0] = 240 (ypos by suby alike): where the height
              map's two halves fall, the field wrapping
Height        (DrawHeightOnMiniMap 0x005a3170) alpha fptosi(112 x 0.7) (78):
              mode 1: all 80 x 80 texels drawn 160 x 160 at (px - 80, 80 - py)
                and at the eight places 160 away round it
              mode 0: its four 40 x 40 quarters (u, v 0 or 40) drawn 160 x 160
                at (xpos[i] - subx, ypos[j] - suby)
mode 0        DrawKeyObjectOnMiniMap (0x005a1b60): fobj2[0 .. KeyObjNum]'s
              key (type 1) and sub (type 2) objects; DrawSubObjectOnMiniMap
              (0x005a1ff0): fobj[x][y]'s base (3) and tree (4) objects, x
              the outer loop; each its row of KeyIconTBL, SubIconTBL,
              BaseIconTBL or TreeIconTBL[type] (FIELDMAPICON: x, y, w, h,
              ofsx, ofsy) at its height cell: (xb - subx - ofsx, yb - suby
              - ofsy), xb = xpos[0] + 4 (79 - mx) (xpos[1] + 4 (79 - mx - 40)
              past 40), yb likewise from my; alpha fptosi(64 x 0.7) (44)
dungeon       DrawDungeonOnMiniMap (0x005a2c60) unless eventAreaNumber is 28,
              39-42, 53-57, 78-82 or 109-113: FP_DUNGEON's cell (u 96, v 12,
              18 x 18, hung from (9, 9)) at the entrance's cell (dx[0],
              dy[0]), then the red arrow (u 81, v 65, 12 x 15) 3 right and 8
              up of it; mode 1: nine copies at (160 - 2 dx[0] + px - 80 - 9,
              2 dy[0] + 80 - py - 9) and the arrow at (-6, -17) from it
lake          DrawLakeOnMiniMap (0x005a2510) when ccCheckFountain() (the
              entry control's gimmicks hold id 20): u 185, v 13, 18 x 18 at
              fobj2[water]'s cell less 9 (mode 1: nine copies from its wp)
portals       DrawCircleOnMiniMap (0x005a2840) once WORLD::ShowMap has set
              mapFlag (+0x6170): the entry control's magic circles (mcHead)
              at their entParam x, y (+0x138, +0x13c): 16 x 16 at u 80 less
              8 (mode 1: 4 x 4 at u 88, v 8, nine copies); alpha 128, or an
              opened circle's (+0xe0 bit 5) +0xec less 1 a packet to 0
the arrow     minimap2, MakePacketS with ctrl 0x40: 16 x 16 at v 80, at (80,
              80), centre (-8, -4), rot the heading, alpha the pulse
the frame     minimap's cell (96, 96), 160 x 160, at (0, 0), alpha 112
send          minimap2, minimap (SendPacketS)
```

## A dungeon: MakeMiniMap and DUNGEON::DrawMap

The constructor (gcmn 0x005b7c00) frames `mapLayer` at (340, 28) as the
field does and makes, over `DungeonName[type]` (or `DungeonName2` for
texType 1):

```text
+0xd388c map    ccMask(2, 0)   TEX_xdsmap01: the backdrop
+0xd3890 map1   ccMask(50, 5)  TEX_xdsmap01: the dots and signs
+0xd3894 map2   ccMask(2, 0)   TEX_xdsmap02: the rooms seen
+0xd3898 map3   ccMask(50, 5)  xallow0::TEX_xallow0
+0xd389c kanji  ccInitKanji(16, 0), ctrl 0x10, on fontLayer
```

`smallmap[10][256][256]` (+0x33538) holds a byte a 300-unit square of each
floor (200 x 200 of it used): 0 nothing, 1 floor, 2 a door, 3 stairs, 4 a
ground of kind 0xc0c0. `minimap[10][15]` (+0x306d0, `MINIMAP_INFO`: size,
direc, flag) has each room's size (`MakeRoom` sets it from the room's
footprint, 0x005ba8fc, or `ROOMDATA.size`, 0x005bd800) and whether it is
seen.

`DUNGEON::Draw` (0x005ce930) calls `MakeMiniMap(here)` (0x005cd850) for the
room of the player's cell each frame:

```text
here >= 15 or seen: 0
seen = 1
half, n       by the room's size: 1800 and 12, 3300 and 22, 6300 and 42; a
              story room (edit's ROOMDATA) of type 16 or more: 6300 and 82
the rays      from (x0, y0, 1500) to (x0, y0, -500), x0 = 150 + (pos.x -
              half), y0 = 15 + (150 + (pos.y - half)), n x n of them 300
              apart; square (fptosi((x - 150) / 300), fptosi((y - 165) /
              300)); within 200 x 200, ccHitCheckLM(from, to, 0xffffffff)
              over the room's collision; a hit on a square still 0 marks it
              by the nearest result's attribute: & 0x20000 3, & 0x80000 2,
              & 0xf0f0f0 0xc0c0 4, 0x303030 left 0, else 1. The hit point
              becomes the ray's end, which walks on from there
1
```

and, for 1, paints `TEX_xdsmap02` (`DUNGEON.ccs`, +0xd3884): square (x, y)
of the floor is texel (211 - x, 243 - y), 1 -> 255, 2 -> 254, 3 -> 253, 4
-> 251 (other squares untouched).

`DrawMap` (0x005cc160), statics `r$8238` and `alpharate$8241` (0.7):

```text
dungeonMapMode 1, or specialRoom (WORLD_MAN +0x160) not -1: nothing
mapHideFlag (+0x42c)  the texture painted again from the squares, the others
              to index 1; unless it was 2 or eventMng +0x78c is set,
              ccMenu.mapStatus = 1; mapHideFlag 0
transp        the five sprites' = GetMapAlpha(); the pulse as the town's
the place     s1 = fptosi(2 (200 - fptosi(x / 300)) - 80), s5 = fptosi(2
              fptosi(y / 300) - 80): 2 px a square, x mirrored
portals       mcHead's on this floor in a seen room: map1 4 x 4 at u 239, v 1
              at (sx - s1, sy - s5), sx = 2 (200 - x / 300) - 1 (2 less when
              x % 300 is over 200), sy = 2 (y / 300) (2 less when y % 300 is
              200 or less), integer division of fptosi(entParam.pos)
gimmicks      gimHead's likewise at u 243, ids 0-5 always, 17 and 38-44 while
              their param[2] is set
the stairs    UpRoom's (u 233, v 45, 14 x 10, centre (-6, -5)) at
              startpos[0], DownRoom's (u 225, v 19, 29 x 10, centre (-15,
              -5)) at startpos[1], each when its room is seen, alpha the
              pulse; the up one not in types 8 and 9
a fountain    types 8 and 9: the first gimmick of id 20 in a seen room (its
              floor not tested): u 225, v 70, 18 x 18
map3          the arrow as the field's, ctrl 0x40
map, map2     224 x 224 texels drawn 448 x 448 at (-24 - s1, -24 - s5): the
              backdrop at alpha 44, the rooms seen at 128
send          map3, map1 (SendPacketS); map2, map (SendPacket)
the floor     types 8 and 9 none; else the kanji at (350, 34),
              ccSpriteColorTable[7]: levelstr[level] ("B 1" .. "B 10")
```

## ShowMap: the Fairy's Orb

The events' `show_map` (instruction 122: sound 97, then `WORLD_MAN::ShowMap`
until it answers 1) is the orb's reveal. `WORLD_MAN::ShowMap` (main
0x001a3ee0) by `WORLD_MAN.flag`:

```text
0 town     1 (nothing)
1 field    eventAreaNumber not 0 and its EVENTAREA_INFO (the 126 at main
           0x00315120) has model 1: 1 (a story map of its own). Else
           WORLD::ShowMap (gcmn 0x005ad990: mapFlag = 1, the portals on the
           map) and 1
2 dungeon  DUNGEON::ShowMap (0x005cf260), one room a call:
             the room of the player's cell deleted (DeleteRoom)
             the first room of the floor built (animIdx set) and not seen:
               SetRoom, MakeMiniMap, the texture painted, DeleteRoom; 0
             none left: SetRoom(floor, here); 1
```

So in a dungeon the room the party stands in is gone from the screen for a
frame a room while the orb scans the floor. Each `SetRoom` runs `SetDoor`
(0x005c7c30), which opens the doors only when `ccCheckActiveObject(f, i)`
finds no foe and no magic portal of that room: an orb used in a fight
builds the room again with its doors shut, as they were.

## The port

`piney-world`'s `map/`:

- `sprite.rs`: `Spr` (a `ccSprite` / `ccMask` as the map drives it) and
  `Packet` (its fields at each `MakePacketStr`, floats as bits); `corners`
  is `MakePacketStr`'s TF and TRF geometry, `prims` the GS primitives.
- `town.rs` (`TownMap`, Mac Anu's or Dun Loireag's by `TownMap::open`),
  `field.rs` (`FieldMap`, with `paint_heights`),
  `dungeon.rs` (`DungeonMap`, `floors_of`, `show_map_step`): the three
  draws, each returning what it sent (`Out`: sends and texts) in order.
- `mod.rs`: `MapModes` and `button` (`ChangeMapMode` and the save),
  `MapState` (the modes, `mapAlpha`, the last frame), `town_frame`,
  `setup_area`, `area_frame`, `show_map`.

The field's and dungeon's painted textures go to the frame as PSMT8
uploads with their files' palettes. A `FieldArea` and a `DungeonArea` hold
their map (`map`); the dungeon's lives as long as the dungeon, room to room.
The `DungeonArea` slots carry the room's size, and `delete_room` is
`DeleteRoom`.

`piney-game` runs it as the game orders the tasks: the map button between
the event task and the world's tasks (in `world.rs` and `area.rs`, from the
tasks' first frame, not while every task sleeps); `Request::MapAlpha` into
`MapState`; the map drawn after the field UI's frame (the town from the
tasks' second frame, as `ROOTTOWN01::Draw`), again as it was while every
task sleeps; `DrawMap`'s `mapStatus` back to the UI (`map_on`). The area
host's `show_map` and the orb's steps call `FieldWorld::show_map`, which
gives `map::show_map` the combat's `ccCheckActiveObject` (`room_clear`) for
the rooms' doors; the town host's answers 1 as `WORLD_MAN::ShowMap` does
there.

## Checks

`tools/test_map_rs.py` runs the game's own code in eemu beside
`map_probe`:

- `ROOTTOWN01::DrawMap` over three masks the real `ccMask::ccMask` builds on
  layers the real `ccLayer::Init` and `ccView::SetFrame` make, the real
  `RT01ICONPOS` carried frame to frame: 95 frames from the arrival (the
  alpha fading in) over the whole town, both stops of the scroll, the map
  off and on, random places, headings and alphas - every `MakePacketStr`
  call's sprite, sub-sprite, cell, transp, centre, rotation, place, size,
  grid, colour and ctrl (1,456), every send in order, and `RT01ICONPOS`
  after each frame, none differing;
- `ROOTTOWN02::DrawMap` the same way, its arrow's mask on the font layer,
  `RT02ICONPOS` carried: 191 frames from Dun Loireag's start over the town
  and around each sign with the arrow on its balloon - 2,992 packets, 168
  of them dimmed to 64, none differing;
- `WORLD::DrawMiniMap` and its six helpers over a `WORLD` laid out from the
  port's generation of story area 14's field and nine random fields of
  types 0-3 and 5-9 (fobj2[], fobj[40][40], water, dx[0]), 60 frames each
  in both maps and none, with a story area that hides the entrance, the
  fountain, the portals shown and an opened one fading: 45,746 packets,
  the texts, the portals' alphas; and `Generate`'s painting loop run
  natively over each field's heights in a `FIELD`: every texel;
- `DUNGEON::MakeMiniMap` over the rooms the real `DUNGEON::Generate` and
  `SetRoom` build in 24 dungeons (`tools/test_dungeon_rt.py`'s `Game` and
  cases: story area 14's, nine other story areas' and random ones of seven
  field types), rays through the real `ccHitCheckLM`: every room of each
  first floor (206), their sizes and every square of `smallmap`;
  `DUNGEON::ShowMap` call by call over each second floor (24) with the room
  under the player built, and the squares it leaves; and `DUNGEON::DrawMap`
  for 12 frames on each first floor (portals, gimmicks of every kind, the
  stairs, both modes, a special room, `mapHideFlag` with and without an
  event holding the map): 2,594 packets, `ccMenu.mapStatus`, and the
  repainted texture;
- `WORLD_MAN::ChangeMapMode` for every flag and mode (144 cases),
  `SetMapAlpha` / `GetMapAlpha`, and `WORLD_MAN::ShowMap` in a field for
  every story area's `eventAreaNumber` and none, and in a town;
- vertices: every frame of the three runs again from the same state with
  `ccSprite::MakePacketStr` run natively (its `__fixunssfdi` in Python), the
  GIF packets it writes read back - each SPRITE's and strip's XYZ2, UV and
  RGBAQ, the culled ones absent - beside the corners `sprite::corners`
  computes: all equal, the turning arrows and shades among them.

`piney-game`'s `event_3_shows_the_map` plays a new game through event 2
into the field and event 3's camera lesson: by Orca's "You see the Red
Down Arrow on it?" (line 21) the map is at full alpha with the entrance and
its red arrow on it, and line 25's `show_map` sets `mapFlag`.
`event_3_map_presses` prints that run's pad for `--press`.

## Unknown

- The entry control's lists come from the battle
  ([battle](battle.md#how-it-plugs-into-piney-world)): `FieldWorld::map_entries`
  hands the map its magic portals (`mcHead`) and gimmicks (`gimHead`, the
  fountain among them) and takes the portals' fading alphas back, so the
  Fairy's Orb's "yellow areas" appear once `show_map` sets `mapFlag`.
  Which gimmicks a field's or dungeon's entry control holds beyond the
  portals depends on the objects the port does not construct yet.
- `menuClrWait` (the two frames after `menu_clear`), `ccCheckGtHackAnm` and
  the party's annihilation, which hold the map button with the other
  buttons, are not modelled; nor is `ccGame.inBattle`.
- What sets `WORLD_MAN.specialRoom` (+0x160) beyond the rooms the port
  knows. `mapHideFlag` (+0x42c) is set by the constructor (0),
  `RoomSelect` (2) and both stairs in `GotoNextRoom` (1, 0x005ca074 and
  0x005ca120, with `ccMenu.mapStatus` 3), and cleared by `DrawMap`; the
  port sets it at the same places (`ccMenu.mapStatus` 3 is not modelled).
- The rotated cull reads a width and height from the scratch area that the
  rotated path never writes; the port takes 0, which only matters for a
  sprite whose corners are all off the screen.
- `RT01ICONPOS` and the pulses' angles are the game's globals and statics,
  kept across visits; the port keeps the town's for the town's mode and
  starts a new area's at 0. `ROOTTOWN03`-`05`'s `DrawMap` are not ported.
- `WORLD_MAN::ShowMap` in a field reads `EVENTAREA_INFO` by
  `eventAreaNumber` (`game.field`); the port's host takes the story area's
  `model` its `WORLD_MAN` was made with, which is the same area wherever the
  port makes one.
