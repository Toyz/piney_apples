---
title: Dungeon generation
status: partial
volumes: all
covers: INF gcmn.prg:0x005c1440 DUNGEON::SetClutList, 0x005c17b0 DUNGEON::ChangeClut; INF SLUS_202.67:0x0013aad0 ccModel::ChangeClut, 0x00152d00 ccAnm::ChangeClut; INF gcmn.prg:0x005c12a0 DUNGEON::Generate, 0x005c0110 MakeFloor, 0x005b6840 FOOT::FOOT, 0x005b7330 FOOT::Move, 0x005b78c0 FOOT::CheckDirection, 0x005ba1d0 DUNGEON::MakeRoom, 0x005bb340 SetAllGim, 0x005be020 MakeRealMap, 0x005bcbb0 DUNGEON::MakeRoom(ROOMDATA *), 0x005b7c00 DUNGEON::DUNGEON, 0x005c1ca0 SetRoom, 0x005c3a50 ClearRoom, 0x005c1980 DeleteRoom, 0x005c7c30 SetDoor, 0x005cd3d0 MoveDoor, 0x005c9e10 GotoNextRoom, 0x005ce930 DUNGEON::Draw, 0x005cf030 DUNGEON::GetHeight, 0x005cf040 GetStartPosition, 0x005c4560 SetLight, 0x0042e010 ccCheckActiveObject, 0x00571da0 initHitCheck, 0x005c8820 DUNGEON::OpenDoor, 0x005c8f50 CloseDoor, 0x005c8800 CloseDoor2, 0x005cf080 GetRoom2DPos, 0x005b6720 GetEditDungeonPtr; INF SLUS_202.67:0x001382f0 ccCoord::SetMatrix_PosRotZYX, 0x00110a30 sceVu0RotMatrixZ, 0x00139330 ccDirectLight::CheckRange, 0x001392d0 ccDirectLight::Init, 0x001389b0 ccCreateLight, 0x00105820 ccDrawEnv::SetFog, 0x0019f4b0 WORLD_MAN::SetDungeonTexClut, 0x0019cf50 SetDungeonTypeFromField, 0x0019c460 fieldrand, 0x0019f8e0 WORLD_MAN::GO, 0x0019dda0 WORLD_MAN::Enter, 0x001a1190 WORLD_MAN::SetCharPosition, 0x001a10c0 WORLD_MAN::GetHeight, 0x001a4070 WORLD_MAN::SetPrevRoom, 0x00151f30 ccAnm::HitEnable, 0x001520f0 ccAnm::SetHitMatrix, 0x00153760 ccModelHit::HitEnable, 0x001107b0 sceVu0InversMatrix, 0x0014cb80 ccStream::Decode_ExtObj, 0x001b3620 ccEvent::SetEventPoint, 0x001b3590 ccEvent::SetEventPos, 0x001a22b0 WORLD_MAN::Get2DMapPtr, 0x001b218c open_door, 0x001b21cc close_door, 0x0019dca0 WORLD_MAN::RoomSelect, 0x001b0948 room, 0x001b097c room_point; INF gcmn.prg:0x005c95c0 DUNGEON::RoomSelect, 0x005b61e0 DUNGEON::GetBanRoom; INF SLUS_202.67:0x0013cff0 ccClump::HitEnable, 0x0013d240 ccClump::SetHitMatrix, 0x001d9dd0 ccGetDist, 0x00178400 ccSaveData::CheckAreaBan; INF gcmn.prg:0x005c3f30 DUNGEON::SetWater, 0x005c70c0 SetObject, 0x005c6e80 EntryObject, 0x005c3ad0 SetAnmObject, 0x005ce1c0 DrawWater, 0x005cdee0 DrawEff, 0x005ce3d0 DUNGEON::DrawBG, 0x00503cf0 SNOW::SNOW(ccStream *, float *), 0x00503e80 SNOW::Move, 0x005042f0 SNOW::Draw, 0x00502e20 calcPos3; INF SLUS_202.67:0x00151ce0 ccAnm::GetSubstAdrs, 0x00101ad0 ccMatchIndex, 0x00101e20 ccSubstSearchResult::SetTbl, 0x00138b60 ccSetColor, 0x00139060 ccLightGrp::AddGrp, 0x0013ba20 ccEff::Init, 0x0014bce0 ccStream::Decode_Model, 0x0013b5c0 ccObj::Init
worklog: 19, 25, 30, 34, 114, 147, 156, 161
---

# Dungeon generation

A random dungeon is grown room by room from `dungeonSeed`; a story dungeon is
laid out from a hand-made table. `tools/dungeon.py gen` reproduces both
exactly (300 random dungeons and all 88 hand-made ones checked against the
game code run in `tools/eemu.py`, 0 mismatches).

The port carries the generator in Rust (`piney_data::dungeon`, worklog 30):
`piney-gen` (`placement::dungeon`) reads the tables below out of each
volume's executable into the build (`dungeon::tables_of`), and
`tools/test_dungeon_rs.py` compares it with `dungeon.py` field by field.

## Inputs

From the area generator ([area keywords](area-words.md)): `dungeonSeed[n]`,
`levelMax`/`roomMax` (`dungeonData[dungeonSize]`), and the dungeon type from
`WORLD_MAN::SetDungeonTypeFromField`:

| field type | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| dungeon type | 2 | 1 | 3 | 3 | 8 | 1 | 1 | 0 | 3 | 2 | 0 |

Field type 4 also sets `dungeonType[1] = 2`. Story areas 120, 66, 91, 16, 18,
21 get types 3, 5, 7, 7, 4, 6; other story areas use the table but switch to
the "E" variant (2→6, 1→5, 3→7, 0→4, 8→9) when `EVENTAREA_INFO.flag == 3` or
`saveData+0x6772` is set. Type `t` loads CCS file `DungeonName[t]`: sd1, sd2,
sd3, sd5, sd6, sd7, sd8, sda, sd4, sd9 (the "a" versions when
`WORLD_MAN.texType == 1`).

`WORLD_MAN::GO` sets `seed = dungeonSeed[game.dungeon]`, `randcnt = 0` before
`DUNGEON::Generate`. `fieldrand(n)` is the area generator's RNG, returning
`seed % n`.

## Random layout

```
floors = levelMax (4 if the first keyword is word 131; types 8, 9: 1)
for each floor:
  floorRoomNum = fieldrand(5) + roomMax;  >= 15 becomes 12
  MakeFloor:
    realmap[80][80] = {d 0, next 15, here 15}
    room 0: fieldrand(1000) discarded; size = ChooseRoomSize();
            centred on (40, 40); one south exit of size ChooseRoomSize()
    grow in passes over live rooms (rooms added in a pass are visited):
      drop exits whose new room would leave 0..79 or overlap a room
      for each exit: new room n centred on the door; the old room stops
        being live; exitNum = max(1, fieldrand(4000) / 1000); exits chosen
        by fieldrand(400) >> 2 until exitNum distinct unused sides (not the
        way it came), each sized by ChooseRoomSize (medium on the last floor)
      stop at floorRoomNum rooms
    restart the floor (RNG not rewound) after 2 * floorRoomNum passes, or
      if fewer than 2 rooms have exactly one exit
  down stairs (every floor but the last): the first pass-through of rooms
    finding a room with one connection, fieldrand(100) >= 91, not room 0;
    later qualifiers in the same pass win. Up stairs are room 0.
  MakeRoom for each room: pick a model, place gimmicks
```

`ChooseRoomSize`: `r = fieldrand(100)`; volume 1 on servers 0-1: small below
20, large below 50, else medium; otherwise small below 20, large below 40,
else medium. Volume 2+ on servers 2+: a large room 0 becomes medium when
`fieldrand(100) >= 31`. Sizes in cells: small 4, medium 8, large 16; each map
cell is 750 units.

## Rooms and gimmicks

`MakeRoom` takes `direc` = connected sides (bits 1/2/4/8) | 0x10 up | 0x20
down, finds the row with that exit mask in the size's `ROOM_INFO` table for
the dungeon type (`{char **roomobj; u32 num; u8 exit; float r}`), and picks
model `roomobj[fieldrand(num)]` with rotation `r`. A medium one-exit room
without stairs becomes the Gott statue room once - on the last floor for
types 0-7, on any floor for 8/9.

`SetAllGim` then walks the model's dummy objects: each item-box dummy
(`OBJ_0ppi*`) draws `fieldrand(100)` and is kept at 20 or more; each
`OBJ_0ps0*`..`OBJ_0ps3*` draws `fieldrand(100)` and is kept at 31 or more;
magic circles (`OBJ_0ppm*`) and fountains draw nothing. So a floor's layout
depends on how many dummies the earlier floors' room models contain.

## Story dungeons

`MakeRealMap` paints each `ROOMDATA` row (0x4c bytes) of a floor with its
index and writes door cells from its `dirc` bits; 0x10 up, 0x20 down. No RNG.
`EditDungeon` (88 rows) selects the table by event area number and dungeon
index. The constructor first sets every cell of all ten floors to
`{d 0, next 15, here 15}`.

The `DUNGEON` constructor takes the table when `isEventArea` (+0x14,
`WORLD_MAN.eventAreaNumber`, cleared for dungeon 0 of field type 4) and the
dungeon index find an `EditDungeon` row; it copies the rows into `roomdata`
(the unused ones get floor 10). `Generate` runs `MakeRealMap` and then, as
for a random dungeon, ten `fieldrand(5)`s and `MakeFloor`s. `MakeFloor`'s
story path (0x005c0148) sets `UpRoom[f]` to 0 and `DownRoom[f]` to the row
with the down stairs (not on floor 9, which has none to look for), makes
nothing for types 8-9, and passes each row of the floor (floor 9's too:
area 108's dungeon has rooms there) to `MakeRoom(ROOMDATA *, ROOM_INFO *, 23)` (0x005bcbb0) with the
"E" table of the type's family, whatever the area's own variant: types 0
and 4 `?room_infoE0`, 1 and 5 `E1`, 2 and 6 `E2`, 3 and 7 `E4`, by the row's
size (small, medium, large).

`MakeRoom(ROOMDATA *)` takes every table row whose exit mask is the row's
`dirc` and, by `room_type` (+0x10):

| type | model |
| --- | --- |
| 0-5 | `roomobj[type]` |
| 6-11 | `roomobj[0]` |
| 12, 13 (up, down stairs) | `roomobj[0]` |
| 14 | the Gott statue room, `symroom[type]`; `symFlag` 1, `symFloor`, `symBlock` |
| 15, 27, 36 on | none (an empty anm) |
| 16-24 | none: the centre is stored and the room deleted |
| 25-35 but 27 | event rooms with scene files of their own (`se1_3`, `se1_4`, `se1_7_2`, `se2_2`, `se3_1`, `se3_3`, `se3_5_2`, `se4_2`, `se4_6`): [Event rooms](#event-rooms) |

A row of type 16 or more is a large room whatever its size. The centre is
`750 * (x + cells / 2)` each way, the turn the table row's; the startpos
follow as for a random room (below).

## Assembly

**One room at a time.** `DUNGEON::Draw` (0x005ce930) draws only the room
the player is in, `room[floor][here]`. `GotoNextRoom` (0x005c9e10) reads the
map cell at (x/750, y/750), deletes the current room, moves the player
1,125 units through the door and builds the next one with `SetRoom`.

**Where a room goes.** `SetRoom`'s random path (0x005c373c) places the
room's `ANM_` animation under `ccCoord::SetMatrix_PosRotZYX` with position
(pos.x, pos.y, 0) and rotation (0, 0, rotate):

```
room = T(pos) * Rz(rotate)          rotate in radians, +x turns toward +y
```

`sceVu0RotMatrixZ` (main 0x00110a30) builds the basis `(c, s, 0)`,
`(-s, c, 0)`, libvu0 keeping a matrix as its basis vectors. Large and
medium rows mean the same `rotate`. Every room model marks each of
its exits with one object on the middle of that side, 600 units inside the
edge, with its +y pointing out:
- a door dummy, `OBJ_0pae0_*` (`SetDoor`'s door name), or
- a gate wall, `OBJ_w_0g10_*` (the door frame's model).

Turned by Rz(rotate), those markers give exactly each row's exits, in all
2,569 (table, row, model) cases of the ten types. Opposite doors of two
neighbouring rooms face each other on one line, 1,200 apart, centred on
their shared edge.

**Doors.** `SetDoor` (0x005c7c30) plays one door animation at each door
dummy, at `T(pos) * Rz(rotate) * dummy`, chosen by type (jump table
0x006f9c60):

| types | door |
| --- | --- |
| 0, 4 | `ANM_sd1ae0_a` |
| 1, 5 | `ANM_sd2ae0_a` |
| 2, 6 | `ANM_sd3ae0_a` |
| 3, 7 | `ANM_sd5ae0_a` |
| 8, 9 | `ANM_sd4ae0_a` |

A room with nothing alive in it (`ccCheckActiveObject`, gcmn 0x0042e010)
gets its doors at their last frame, open. Otherwise they stay at frame 0,
closed, and `MoveDoor` (0x005cd3d0) opens them once the room is cleared.

Door pieces reach about 800 units along their +y (the lakes' 1,306), so
the two halves of a doorway overlap. The far end fades to almost black.
Gott statue rooms (`symroom`) are built straight through their doorway,
with no door dummy.

**Fog and ambient.** The `DUNGEON` constructor (0x005b802c-0x005b84c0)
starts from `dungeonFog` and picks a row from one of 13 tables (gcmn
0x00695580-0x00695c40), each of 4 rows of 9 floats:

```
fog colour RGB, near, far, fog percent at far, ambient RGB (0-255)
```

The row is chosen by `WORLD_MAN`'s clutType (+0x140) and texType (+0x144),
which `WORLD_MAN::SetDungeonTexClut` sets by server and field type:
- the forest table, row `GetBG()`, for types 8 and 9;
- the "hacked" table, row type − 4, for types 4-7;
- the plain table, row = type, for types 0-3.

The fog colour is packed into `DUNGEON.fog`, which is also the frame's clear
colour. `ccDrawEnv::SetFog` (0x00105820) makes the GS fog value run from 255
at near to 2.55 · (100 − percent) at far. `SetRoom` sets the fog and
`SetAmbient(row / 255)` again for every room.

**The ambient does not light the rooms.** Every model in every dungeon file
is unlit. `ccModel::Draw` sends `mtype & 7 == 0` models to the unlit
programs, which write the vertex colour as it is, and those colours average
44-70/128. So dungeons are as dark as their baked colours. The ambient
lights only lit models: players, monsters and items.

`piney_data::dungeon::place` reproduces the room matrix, the exits, the
doors and the fog and ambient rows. `placement::dungeon` extracts the
door names, the 13 fog tables and the rule, and the clutType/texType grid
(run in eemu), and `tools/test_dungeon_rs.py` checks them against the game
code.

## Entering and walking a dungeon

Infection's code only (the other volumes are not read for this section).
The port is `crates/piney-world/src/dungeon_area.rs`;
`tools/test_dungeon_rt.py` runs the game's own code beside it
([Checked](#checked)).

**`WORLD_MAN::GO(2)`** (main 0x001a0b08):

```
minx = miny = 0; maxx = maxy = 60000; dungeonback (+0x164) = 0
hackFlag = EVENTAREA_INFO.flag of eventAreaNumber (3 in the crisis)
dungeon[game.dungeon] (+0x438) null:
    seed = dungeonSeed[d]; randcnt = 0
    new DUNGEON (0xd3910 bytes) (d); Generate
    GetStartPosition(&position)          position (+0x20) = startpos[0][0]
else if prevFlag (+0xf4):
    ClearRoom; SetRoom(prevFloor +0xfc, prevBlock +0xf8); prevFlag = 0
field type 4, game.dungeonPrev 1, game.block == lastRoom (+0x100), game.dungeon 0:
    position = dungeon[0].startpos[1][0]; ClearRoom; SetRoom(0, lastRoom)
    (the seed kept round it); randcnt = 0; dungeonback = 1
the shadow packets
```

`Generate` ends with `level = 0` and `SetRoom(prevFloor, prevBlock)` when
`prevFlag` is set, else `SetRoom(0, 0)`. So a dungeon is made once, on the
way in from its field; every door and stairs after that is a scene change
that finds it kept, with the room `GotoNextRoom` already built.

A lake (field type 4) has no field and two dungeons: dungeon 0 the lake,
dungeon 1 below it (`dungeonType[1] = 2`). `dungeon[n]` keeps each, so the
lake's stairs down make the second and the second's floor-0 stairs up find
the lake as it was left; `GO(2)`'s last branch then builds `lastRoom`
with the party at its stairs down. The port keeps them the same way
(`piney_world::dungeon_area::Dungeons`, the slots and `lastRoom`, carried
in `Kept`; `DungeonArea::come_back`). Until worklog 191 it kept one
dungeon, so the lake itself was taken for the second.
`ccSetupGameCtrl`'s `initHitCheck` (gcmn 0x00571da0) empties the hit lists
only when `areaPrev` is -1, `area` is 0, or a field is entered from a town,
so the room's hits stay registered across those changes.

**The start.** `MakeRoom` (random, 0x005ba1d0, and story) poses the room's
anm (`SetAnm`, one `_AnimateForward`, `SetMatrix_PosRotZYX((x, y, 0), (0, 0,
rotate))`) and, for a room with the up stairs, copies the world position
(`_SetLWMatrix`) of its `OBJ_0ppp` dummy into `startpos[0][f]`, for the down
stairs into `startpos[1][f]`, with w the heading the other way from the
room's turn (0 to pi, pi to 0, pi/2 to -pi/2, -pi/2 to pi/2); the lake types
take the room's centre. `WORLD_MAN::SetCharPosition`'s area-2 case (main
0x001a1c2c) puts the leader at `position`'s x, y, z facing its w, and the
second, third and fourth at (300, -150, 0), (-300, -150, 0) and (0, -300,
0) turned by that heading (`sceVu0RotMatrix`) and added.

**`DUNGEON::SetRoom(f, i)`** (0x005c1ca0) makes `room[f][i]`:

```
stillOpenDoor = CloseStart = 0; the direct light out of the group
statue room (symFlag or lakeFlag, (f, i) == (symFloor, symBlock)):
    SetFog, SetAmbient; SetAnm(the type's statue room, @4950 0x00696aa0)
story rows of types 25-35: their own scene files and lights (@5452)
else: SetFog(near, far, 0, max, DUNGEON.fog); SetAmbient(ambient / 255)
      SetAnm(animIdx's roomobj)
      (the lakes' and clutType 3 / 4's palette swaps: below)
_AnimateForward(frameSpd)
SetMatrix_PosRotZYX((pos.x, pos.y, 0), (0, 0, rotate))
HitEnable(1); SetHitMatrix
SetWater, SetLight, SetObject, SetDoor, SetAnmObject ("The dressing");
the lakes' fireflies ("The lakes' sky and fireflies")
```

**The palette swaps.** The constructor calls `DUNGEON::SetClutList(sfx)`
(gcmn 0x005c1440) for a dungeon of type 0-3 under clutType 3 ("c1") or 4
("c2"), and for the lake types by `GetBG` 1-3 ("c1"-"c3"). It keeps a list
of `CLT_` names per type (types 0, 1, 2, 3, then 8 and 9: 52, 62, 37, 42
and 40 names) with each name's twin, the name plus the suffix, both looked
up in the dungeon's file (+0x14c and +0x24c). `SetRoom` then calls
`ccAnm::ChangeClut(twin, name)` for each pair on the room's anm until a
name is missing. `ccModel::ChangeClut(new, old)` (main 0x0013aad0) writes
`new` into every material whose palette is `old`, in the shared model,
so the swap holds for every later draw of the file, the doors' included.
Theta's dungeons (server 1) of field types 2-4, 7 and 9 take clutType 3;
servers 2-4 swap more.

`placement::dungeon` extracts the lists (`Tables::clut_lists`,
`Tables::clut_list`). `DungeonArea::clut_swaps` holds the file's pairs,
and every draw of the dungeon's own file uses them.
`theta_dungeons_swap_their_palettes` builds a type-3 dungeon on servers 0,
1 and 3: none, 42 pairs to "c1", and 42 to "c2".

Every object the room's Anime chunk places is its own `ccObj` - an ExtObj
copy too: `ccStream::Decode_ExtObj` (main 0x0014cb80) gives each its
index (renamed `EXT_...`), model and `ccModelHit`, so a wall piece placed
twenty times collides twenty times. `ccAnm::HitEnable(1)` (0x00151f30) puts
each object's model's hit on the list's tail (once: `ccModelHit::HitEnable`
0x00153760 keeps a hit already on it) with `type` 1, so a query is turned
into the piece's space by `im` as well as moved; `ccAnm::SetHitMatrix`
(0x001520f0) gives each `rm` = the object's `lwMatrix` (its parent chain up
to the anm's matrix, `ccCoord::_SetLWMatrix`) and `im` = `sceVu0InversMatrix`
of it (main 0x001107b0: the rotation transposed, the translation
`-(r0 t.x + r1 t.y + r2 t.z)` in VU0 multiply-adds, w kept). An object's
local matrix is `ccAnm::SetAnmCtrlWork`'s: the rotation times the scale
(`sceVu0MulMatrix`), then the position added. The list is the anime's
index order, then each door's.

**Doors.** `SetDoor` (0x005c7c30) finds the room anm's `OBJ_0pae0_*`
dummies (`lockNum`), deletes the old door anms and makes one per dummy (at
most four): its matrix is the dummy's own matrix turned by
`sceVu0RotMatrix((0, 0, rotate))` and moved by `sceVu0TransMatrix((x, y,
0))`. When `ccCheckActiveObject(f, i)` (gcmn 0x0042e010) finds no entity of
that room in either entity list, `doorFlag` is set and the door runs
`frameMax` `_AnimateForward`s, to its last frame: open. Otherwise it takes
one step and stays shut. Then `HitEnable(1)` and `SetHitMatrix`: the frame
(`OBJ_w_9g10_`) and the leaf (`OBJ_w_9e20_`) collide where the door
stands. `MoveDoor` (0x005cd3d0), from `Draw` each frame, sets `doorAnm`
from `ccCheckActiveObject()`; for each door (unless `stillOpenDoor`) with
the room empty it plays the opening sound once (`ccSeOn3D`, one of 45-48
and 51 by type), steps the anm, re-enables and re-places its hits (not for
types 3, 7, 8, 9, which take the leaf's hit off when the door has opened),
and draws it; an open door just holds its last frame.

**Ground.** `DUNGEON::GetHeight` (0x005cf030) is 0, and
`WORLD_MAN::GetHeight` (main 0x001a10c0) answers it in area 2.
`ccLandHitCheck` in a dungeon is the segment from 105 above to 1000 below
against the room's floors, or the point's own z (no height map): the
floor tiles without a hit are simply walked on at the height the player
has.

**`DUNGEON::GotoNextRoom(nxt, now)`** (0x005c9e10), which
`WORLD_MAN::Enter` calls with `&WORLD_MAN.position` when the player lands
on ground with attribute bit 0x80000 (a doorway or the stairs):

```
(d, next, here) = realmap[level][fptosi(now.x / 750)][fptosi(now.y / 750)]
WORLD_MAN::SetPrevRoom(pos[level][here])    prevBlock, prevFloor = game's; prevPos
room[level][here].HitDisable(); DeleteRoom(level, here); specialRoom = -1
next 15, UpRoom[level] == here:     mapHideFlag = 1; ccMenu+0x10 = 3
    level > 0: nxt = startpos[1][level - 1]; SetRoom(level - 1, DownRoom[level - 1])
    roomEnterFlag = 1; return 15
next 15, DownRoom[level] == here:   the same flags; lakes: return -1
    nxt = startpos[0][level + 1]; SetRoom(level + 1, UpRoom[level + 1])
    roomEnterFlag = 1; return -1
isEventArea 108, 73, 47, 66, 46, 27, 91, 16, 101, 77, 71, 48, 25, 23:
    their own branches (Event rooms, below)
warpFlag = 0; a story row whose side names this next and has a side flag:
    nxt = warpPoint[flag - 1] (+0x170, 32 bytes), next = its room byte (+0x10)
    (the east and west sides read each other's flag); warpFlag = 1
else nxt = now, then by d: south y + 1125, w pi; north y - 1125, w 0;
    east x + 1125, w pi/2; west x - 1125, w -pi/2
SetRoom(level, next); roomEnterFlag = 1; return next
```

**`WORLD_MAN::RoomSelect(floor, block)`** (main 0x0019dca0) is how the
events' `room` and `room_point` move the party. It runs these steps in
order:

1. `ccSys.bgColor` (+0x18) is set to 0, and the GS words at +0xce0 and
   +0xdd0 to 0x3f800000_80000000.
2. On `dungeon[game.dungeon]` it calls `ClearRoom`, `SetRoom(floor,
   block)` and `DUNGEON::RoomSelect(&position, floor, block)`.
3. `ChangeScene(-2, -2, -2, -2, floor, block)`.

`DUNGEON::RoomSelect` (gcmn 0x005c95c0):

```
roomEnterFlag (+0x4c) = 1; mapHideFlag (+0x42c) = 2; ccMenu+0x10 (map status) = 3
position = (pos[floor][block], 0, 1); level = floor
isEventArea 71 or 77: a story room of type 30 or 31 stands the party at its
    OBJ_user_point (specialRoom 0) - later volumes' areas
else the room's anm: the first object matching "OBJ_w_0g10_*" (a gate
    wall), else "OBJ_0pae0_*" (a door dummy); its world matrix
    (_SetLWMatrix) applied to (0, -200, 0, 1) is the position (w 1.0);
    neither: the room's centre stays
```

A `Rz(rotate[floor][block])` matrix is built on the way and not used.
`mapHideFlag` 2 paints the map again without opening it (DrawMap skips its
`mapStatus = 1`), so the map stays closed until something opens it. The
port: `DungeonArea::room_select`, `FieldWorld::room_select` (which asks
for the change of scene the way a door does), and the area host's `room`.
It leaves out `bgColor` (the change's fade covers those frames).

**`WORLD_MAN::Enter`** in area 2 (main 0x0019e018) then asks for the scene:

| answer | Enter |
| --- | --- |
| -100 | nothing |
| a room b | `ChangeScene(-2, -2, -2, -2, -2, b)` |
| 15, `game.floor` > 0 | `level -= 1`; `ChangeScene(2, -2, -2, -2, level, DownRoom[level])` |
| 15 on floor 0 | `ChangeArea(1, eventAreaNumber)` (field type 4: `ChangeScene(2, -2, -2, 0, 0, lastRoom)`) |
| -1 | `level += 1`; `ChangeScene(2, -2, -2, -2, level, 0)` (field type 4, dungeon 0: `lastRoom = game.block`, `ChangeScene(2, -2, -2, 1, 0, 0)`) |
| -255 | the event areas' way out, `ChangeArea(1, n)`: 66 to field 67, 108 to 9, 47 to 9 (volume 2) or 10, 73 to 4, 46 to 2, 27 to 1 (its call falls into 46's, but `ChangeRequest(6, 7)` has put the task to sleep) |

So the room behind a door is built before the fade out, and the scene
change only re-runs `ccSetupGameCtrl` (the player at `position`).

**The entry control across rooms.** A room change is a scene change, so
`ccThEntryCtrlDelete` keeps the dungeon's circles, boxes and idols in
`g_entryList` and the next room's `ccThEntryCtrl` brings them back
(`restoreEntry`; [battle](battle.md#the-entry-control)), each switched on
only in its own room (`entry.floor`, `entry.block`). `WORLD_MAN::EntryGimmick`
places them once per dungeon (`entryFlag`). The port rebuilds its battle
state for each room, so the kept objects come back at new scene indices
with their bodies and animation players (`EntryCtrl::keep`,
`restore_kept`; `DungeonArea.kept_entries`, `kept_actors`,
`gimmicks_placed`); a box's or an idol's rays follow its new index.

**`DUNGEON::Draw`** (0x005ce930): `here` is the realmap cell under the
player (`plw`); `DrawBG(here)` (lakes only), `DrawWater`, `DrawEff` (fire,
snow, the lights' glows, fireflies, leaves); the placed clumps and
animated objects; `MoveDoor(here)`; the minimap; then, if
`room[level][here]` is built, the clear colour and GS fog colour from
`DUNGEON.fog` when they changed, `SetPathFindingMap` once after a room
change, and the room's anm drawn as it stands (not stepped); the block
object; `DrawMap` outside battle. All on `objLayer`.

`here` is the cell's own byte (`lbu $s3, 0x432`, 0x005ceaec), and a door
cell belongs to its room, so there is no fallback to the room that was
built: from the frame `WORLD_MAN::Enter` fires on a doorway until the
player crosses onto the new room's cells, `room[level][here]` is the room
`GotoNextRoom` deleted, and neither room nor doors are drawn (`MoveDoor`
walks the doors only when `room[level][here]` is built). The scene's fade
out is running meanwhile; checked in eemu (below).

**Light.** The constructor puts one distant light in `cc3d`'s group
(0x005b8c10): grey (0.7, 0.7, 0.7), turned by `SetMatrix_RotZYX((-0.8, 0,
-0.6))`, also `WORLD_MAN::SetLightDirection`'s. `SetRoom`'s `SetAmbient`
gives lit models the row's ambient / 255. `SetLight` (0x005c4560) adds an
omni light for each of the room's glows ("The dressing"). The rooms are
unlit; a lit model (Kite) takes the ambient and, as `ccChar::Draw` picks
lights, those omni lights near him and the distant light (the port's
`chara::light_matrix` over the group).

## The dressing

After the room's anm and its hits, `SetRoom` stands the room's pieces:
`SetWater`, `SetLight`, `SetObject`, `SetDoor` (above), `SetAnmObject`,
and in the lakes the fireflies. Each finds its dummies with
`ccAnm::GetSubstAdrs(pattern)` (main 0x00151ce0) on `room[f][i]`:

- The search walks the anm's index in order. An ExtObj copy answers to its
  target's name (`ccMatchIndex`, main 0x00101ad0, follows the 0x0a00
  entries).
- A pattern ending in `*` matches a prefix (`ccMatchStr`), any other the
  whole name. `ccSubstSearchResult::SetTbl` (0x00101e20) skips an object
  already found.
- A piece's matrix is the dummy's own (+0x40) turned about z by
  `rotate[f][i]` (`sceVu0RotMatrix`) and moved by `(pos.x, pos.y, 0)`
  (`sceVu0TransMatrix`), as the doors' are. The water is not turned.

**`SetWater(f, i)`** (0x005c3f30):

- Deletes `water[0..3]` (+0x364).
- At the first `OBJ_0paf0_` (the whole name): three `ccAnm`s of
  `waterAnmName[type]` (@5657: `ANM_sd1af0_a`, `sd2`, `sd3`, `sd5` for types
  0-3 and 4-7, `sd4` for the lakes). Each has `SetFogSw(0)` and its own
  `OBJ_f_8s40_` (`ccObj::Duplicate`, 0x2008 for water 0, 0x2000 for 1 and
  2). Water 0's texture is the frame-buffer copy (`ccModel::ChangeTex` of
  `texChunk`, +0x408). The matrix, one step on. No hits.
- At each `OBJ_o_magma*`: `fire[k]` (+0x54, the 60 not checked), a
  `SNOW(ccs, pos)` (0x00503cf0) at the dummy's place in its `lwMatrix` (as
  the room's `SetHitMatrix` left it), z -100. With clutType 3
  `DUNGEON::ChangeClut(eff, "c1")`, with 4 `"c2"`.
- `SNOW(ccs, pos)`: type 3, range 15, speed 1; `cnt = fieldrand(10) + 3`,
  `life = fieldrand(50) + 10`; `EFF_o_spark_s0_` (`Init(chunk, 1)`, scale
  0.5, the fog bit cleared); then `calcPos3` (0x00502e20).
- `calcPos3(base, pos, dir, range, speed)`: two `fieldrand(300)` for a
  place that `sceVu0CopyVector(pos, base)` then overwrites (the spark
  starts at its base, w 1). The step is `((fieldrand(15) - 7.5) / 10,
  (fieldrand(15) - 7.5) / 10, (fieldrand(3) + 2) * speed, 1)`.
- `DUNGEON::ChangeClut(eff, add)` (0x005c17b0): nothing in the lakes and
  types 2-7. Type 0's `CLT_sd100o11` and type 1's `CLT_sd200o19` (gp
  0x003783a0): `ccEff::ChangeClut(name + add, name)`, which changes the
  palette only while it is `name`.

**`SetLight(f, i)`** (0x005c4560):

- Deletes `roomlight[0..32]` (+0x374, `DelGrp` for each light).
- Searches `OBJ_o_light_m0*`, `OBJ_o_light_s0*` and, in types 2 and 6 only,
  `OBJ_o_altar_l0*`. For each dummy, while fewer than 32 are made (checked
  once a dummy, so a case of several could run past 32), the case of `type
  & 3` (jump tables @6868 and @6869; none in the lakes):

| dummy | type & 3 | glows at | light | second glow |
| --- | --- | --- | --- | --- |
| m0 | 0 | `EFF_0lel0_` (0, -2.35, 310) | 0xe6fafa | |
| m0 | 1 | `EFF_0lel0_` (0, -0.35, 310) | 0xe6fafa | |
| m0 | 2 | `EFF_0lec0_` (0, 175, 215) | 0x96b4fa | `EFF_0lec1_` |
| m0 | 3 | `EFF_0leb0_` (0, 64, 416) | 0x6ec3e1 | |
| s0 | 0 | `EFF_0let0_` (0, 0, 90) | 0x96b4fa | `EFF_0let1_` |
| s0 | 1 | `EFF_0lec0_` (45, 0, 205), (-45, 0, 205) | 0x96b4fa | `EFF_0lec1_` |
| s0 | 2 | `EFF_0lec0_` (0, 10, 215) | 0x96b4fa | `EFF_0lec1_` |
| s0 | 3 | `EFF_0leb0_` (0, 0, 400), (100, 0, 275), (-100, 0, 200) | 0xe6fafa | |
| altar | 2 (types 2, 6) | `EFF_0lec0_` (200, 310, 215), (-200, 310, 215) | 0x96b4fa | `EFF_0lec1_` |

- Each glow is a `ROOMLIGHT` (0x40). Its `ccEff` is `Init(chunk, 1)` with
  the fog bit cleared. Its place is the dummy's `lwMatrix` applied to the
  vector (w 0), plus the dummy's place. Its pattern is
  `fieldrand(patNum)`.
- Its `ccOmniLight` (type 4, priority 1) has `ccSetColor(rgb, 1)` (main
  0x00138b60: each byte times 1/255), `farDownStart` 100, `farDownEnd`
  250, and stands at the dummy's place. `AddGrp(cc3d + 0x80)` (main
  0x00139060) puts it after the group's lights of its priority or more.
- The second glow (fogged) is at the same place, its pattern drawn after.

**`SetObject(f, i)`** (0x005c70c0):

- Lakes (types 8 and 9), in `room[level][i]` (the floor the party is on):
  a clump at each `OBJ_0ps0*` (`CMP_o_statue_l2_`), `OBJ_0ps1*`
  (`statue_l3`), `OBJ_0ps2*` (`flower_l2`), `OBJ_0ps3*` (`flower_l3`), then
  each `OBJ_0ps4*` picked by `fieldrand(4)` (`statue_l0`, `statue_l1`,
  `flower_l0`, `flower_l1`). The files' rooms with such dummies are
  `ANM_sd4l1n31`-`l4n31`; none has an `OBJ_0ps4*`.
- `EntryObject(dummy, f, i, name, num)` (0x005c6e80), for `num` under 10:
  `object[num]` (+0xd3588) a `ccClump` at the dummy's matrix turned and
  moved, `HitEnable(1)`, every node's hits at that matrix
  (`SetHitMatrix`). With `GetBG` not 0 the lakes' palettes (clut0, clut1).
- Types 3 and 7: deletes `anmobj[0..10]` (+0xd35b0); at each `OBJ_0paw0*`,
  then `OBJ_0paw1*` (ten in all at most), a `ccAnm` of `ANM_sd5aw0_a` or
  `ANM_sd5aw1_a`, one step on, `HitEnable(1)`, `SetHitMatrix`.
- The walls' matrix goes to `anmobj[s0]`, `s0` the dummy's index in its own
  search. So a paw1 wall moves the paw0 wall of its index, after that
  one's hits are placed, and keeps its own unit matrix (its hits at the
  world's origin), unless the room has no paw0 walls.

**`SetAnmObject(f, i)`** (0x005c3ad0): types 0-3 only. Deletes
`anmobj2[0..10]` (+0xd35d8); at each of the type's dummies (`OBJ_0pag0_*`,
`OBJ_0pac0_*`, `OBJ_0par0_*`, `OBJ_0pab0_*`, @5566) a `ccAnm` of
`ANM_sd1ag0_a`, `sd2ag0`, `sd3ag0`, `sd5ag0` (@5571; ten at most), one step
on, the matrix, `HitEnable(1)`, `SetHitMatrix`.

**The hit list.** After the room's: the clumps' and walls' (`SetObject`),
the doors' and the ban block's (`SetDoor`), the animated objects'
(`SetAnmObject`). A model chunk with no mmats makes no model
(`ccStream::Decode_Model`, main 0x0014bce0, leaves its data word 0), so its
object has none (`ccObj::Init`, 0x0013b5c0) and its Hit chunk never joins:
sd3's moving floor (`ANM_sd3ag0_a`) carries `MDL_o_move_l0_`, a hit with
nothing to draw.

**Each frame** `DUNGEON::Draw` does, before the room:

- `DrawWater` (0x005ce1c0): the three waters one step on;
  `ccAnm::SetUV(vftoi12(u))` into water 1's U and water 2's V; `u` on by
  0.005, back to 0 at 1 (`u$8862`, a static of the whole game);
  `waterUVModifi` of water 0's `OBJ_f_8s40_`; waters 1 and 2 on `objLayer`,
  water 0 on `refLayer`, then `MakePacketDrawBuffTrans(texChunk)`.
- `DrawEff` (0x005cdee0), on the EFF layer: each `fire[k]`'s `SNOW::Move`
  (0x00503e80) and `SNOW::Draw` (0x005042f0).
- `SNOW::Move` for type 3: the step added (no wrap about the player);
  `cnt` and `life` down; at `cnt` 0 a new `fieldrand(10) + 3` and x, y
  drifts; at `life` 0 `calcPos3` from the base and `life =
  fieldrand(60) + 20`. `SNOW::Draw`: transparency `life / 100` under 32,
  else 1; pattern 0.
- Then each room light: its glows drawn at their patterns; the second's
  pattern on by one; the light's intensity `eff.transparency *
  pat[patNum].transparency / 4096`; the first's pattern on (both wrap at
  their `patNum`). Then the lakes' fireflies, by night or hacked (below),
  and type 32's tree and leaves.
- For k < 10: `object[k]` drawn, `anmobj[k]` and `anmobj2[k]` stepped and
  drawn.

**The port** (`dungeon_area::Dressing`,
`crates/piney-world/src/dungeon_area/dress.rs`):

- `set_room_with` runs `set_water`, `set_light`, `set_object`, `set_door`
  and `set_anm_object` with `DungeonArea.rng` as `fieldrand`; the pieces
  join the hit list (`joined`) in the game's order, and `put_doors` keeps
  them in place.
- `RoomFile`'s models leave out those without mmats (no hits for them).
- `draw` draws the dressing whether or not the player's cell is the room's
  and steps it while awake. `DrawEff`'s sprites go to the effects as the
  fields' ambient ones do (`field_ambient::Sprite`, the palette change in
  `clut`).
- The glows' omni lights join `lights` and flicker with their glows.
- Water 0 samples the frame drawn so far, not the copy made after it.
- `u` lives as long as the dungeon.
- Not ported: type 32's tree and leaves; `EntryObject`'s palettes for the
  lakes' statues and flowers with `GetBG` not 0.

## The lakes' sky and fireflies

The lakes (types 8 and 9, field type 4's dungeons) are open to the sky.
Their constructor makes the sky's clumps, `DrawBG` draws them about the
room each frame before everything else, and by night `SetRoom` lets
fireflies loose in the room:

```text
DUNGEON()     type 8 (0x005b8e84) and type 9 (0x005b9500): +0x34c =
              GetCCSAdrs("field_eff"); by GetBG (WORLD_MAN.bgnum, 0-3,
              nothing past 3) new ccClumps at +0x350..+0x360, each
              Init(GetChunkAdrsF(name)) and SetFogSw(0):
                type 8: CMP_o_bac_l{n}_, bac_m{n}, clo_l{n}, clo_m{n}
                type 9: bac_l{n}, bac_m{n}, ero_l{n}, ero_m{n}, ero_s{n}
SetRoom(f, i) (end, 0x005c3830) types 8 and 9, GetTime() == 2 or
              ishack (+0x40, WORLD_MAN.hackFlag +0xf0) == 3: the five
              FIREFLYs at +0xd38a8 deleted, five new ones:
                new FIREFLY (its pattern fieldrand(100))
                base = room[f][i]'s centre + (fieldrand(1000),
                       fieldrand(1000)), z 0; SetBasePosition (z +
                       fieldrand(200) + 100)
                hacked: Init(field_eff, 600) (EFF_sfzdigi0 / 1)
                else:   Init(field_eff, EFF_sfpfir_1, EFF_sfpfir_2, 300)
DrawBG(here)  (0x005ce3d0) types 8 and 9:
                mat = GetSubstAdrsF(MAT_sfp7bac{n + 1}) (type 8)
                      or MAT_sfp9dat{n + 1}_3 (type 9)
                mat +0x16 = vftoi12(v) - the Material chunk's +0x16
                v += 0.003; past 1, v -= 1       (v$8917, 0 at first)
                pos = room[level][here]'s centre, z 0
                scale 0.5, 1 or 1.5 by the room's size (+0x306d0)
                each clump: ccLayer::active = WORLD_MAN +0x494 + 4k
                  (-100, -90, -80, -70, -60); SetMatrix_PosRotZYXScale(
                  pos, 0, scale); ccClump::Draw
DrawEff       (0x005cdee0) after the glows, types 8 and 9 by night or
              hacked: each FIREFLY's Move, then its Draw
```

- The clumps' models are domes about the room: `MDL_o_bac_l0_` is 12,208
  across each way and runs from z -332 to 6,412; `bac_m` 7,538, `clo_l`
  11,500, `clo_m` 9,912. They are drawn unfogged on the lowest layers, so
  the room's pieces cover them and they show above its walls and through
  its doorways. The three rotations `DrawBG` passes (`@8923`, `rot2`,
  `rot3`) are all zero.
- `sd4` holds bgnum 0-3's clumps and materials; `sd9` holds bgnum 0's
  alone. Type 9 is the hacked lake (field type 4 with `hackFlag` 3 and
  story area 14's words); a type 9 of another bgnum would read chunks that
  are not there.
- `v` is one static for the whole game: it runs on from where the last
  lake left it.
- By night is `GetTime` 2: bgnum 2 in field type 4 (the table in
  [field.md](field.md#weather-and-ambient-pictures)). The fireflies stand
  in the quarter of the room on the +x, +y side of its centre. They fly as
  the field's `FIREFLY`s do (field.md): about the player, through
  `ccTransPosW2P` over `WORLD_MAN`'s bounds (60,000 in a dungeon), put
  back within 2,500 of him when more than 3,000 away (z 0 in field type
  4), their sparks `EFF_sfpfir_1` (`EFF_sfzfir1` in field type 9).

The port (`crates/piney-world/src/dungeon_area/lake.rs`):

- `Lake::new` finds the clumps and the material; `DungeonArea::draw` calls
  `draw_bg` (`bg_scroll`, `bg_world`) before the dressing, and
  `draw_fireflies` after `DrawEff`'s sprites, which go to the effects with
  them (`field_eff` is loaded on first use).
- `field_firefly::FieldFirefly::lake` makes each firefly;
  `field_ambient::Env.bounds` carries the dungeon's bounds to `Move` and
  its sparks.
- `v` lives as long as the dungeon.

## Event rooms

Infection's code only. A story row of `room_type` 16 or more is a room
`MakeRoom(ROOMDATA *)`, `SetRoom` and `GotoNextRoom` treat apart. The port
is `piney_data::dungeon::special` (what each type and area builds) and
`dungeon_area.rs`. Infection's story dungeons have five:

| area, dungeon | row | type | what it is |
| --- | --- | --- | --- |
| 23, `D0081` | floor 2, room 2 | 25 | Aura's shrine (`se1_3`); event 25's block 16 plays there and bans it |
| 25, `D0101` | floor 2, room 2 | 25 | the shrine of area 25 (`se1_4`) |
| 16, `D0241` | floor 1, room 3 | 26 | `se1_7_2` |
| 91, `D0671` | floor 3, room 2 | 35 | `se3_5_2` |
| 27, `D0121` | floor 4, room 3 | 16 | no room: the door out to Skeith's arena (field 1) |

**`MakeRoom`** (jump table at 0x006f8760, types 12-35) stores the centre
`750 * (x + 8)` each way for all of them (16 cells). Types 16-24 and 34
then delete the room and return: there is nothing to build. Types 25-35
but 27 load their scene file (`ccStream::GetCCSAdrs`) into `DUNGEON.spccs`
(+0xd3888) and play its Anime chunk. The last such row that `MakeFloor`
reaches leaves its file there. The rooms by type:

- 25: `se1_3` / `ANM_se1_3_1a` in area 23 (`WORLD_MAN.eventAreaNumber`), `se1_4` / `ANM_se1_4_1a` in 25, nothing elsewhere.
- 26: `se1_7_2` / `ANM_se1_7sh2a`.
- 28: `se1_4` / `ANM_se1_4_1a`.
- 29: `se2_2`.
- 30: `se3_1`.
- 31: `se3_3`.
- 32: `se4_2`.
- 33: `se4_6`, which Infection's disc does not have.
- 35: `se3_5_2` / `ANM_se3_5_2a`.
- 15, 27, and 36 on: an empty anm.

**`SetRoom`** finds the first row of type 16 or more at (f, i). Types
25-35 but 27 and 34 then take their own case (jump table @5452,
0x006f9890) in place of the fog row's:

```
SetLightEnv(1); SetAnm(the type's anm of spccs); _AnimateForward
light (+0x370) = GetSubstAdrsF(the type's LGT_): SetMatrix_PosRotZYX(it,
    (750 (row.x + 4), 750 (row.y + 4), 0), a rotation left uninitialised)
GetAmbient (the chunk's packed +36 bytes, each / 255.0); AddGrp(light); SetAmbient
SetFog(32767, 65536, 0, 100, 0)
rotate[f][i] = 0 (26: pi)
then the common tail: _AnimateForward, SetMatrix((pos, 0), (0, 0, rotate)),
HitEnable(1), SetHitMatrix, SetWater, SetLight, SetObject, SetDoor, SetAnmObject
```

The lights: `LGT_se1_3lig1` for 25 and 28, `LGT_se2_2lig1`,
`LGT_se3_1lig1`, `LGT_se3_3lig1`, `LGT_se4_2lig1`, `LGT_se4_6lig1` for
29-33. Types 26 and 35 read the ambient and set neither it nor a light. Type
32 also makes `ANM_se4_2lea` at the centre and twenty `LEAF` objects
(+0xd38bc, +0xd38c0); the port does not. The next `SetRoom` takes the light
out of the group (`DelGrp`). The port draws the event room's lit models
(`SetLightEnv(1)`) with that group: the light as the chunk's record at the
frame, its place replaced by the row's.

Each of these lights is a direct light (animation.md): white, intensity
1, the record's place (0, 0, 2500) replaced by `(750 (x + 4), 750 (y +
4), 0)`. That is a quarter of the way into the 16-cell room from its
corner, not its centre `750 (x + 8)`. `ccDirectLight::CheckRange` (main
0x00139330) lights a point in a round beam down the light's -z: the point
taken into the light's space (its matrix inverted), nothing behind it
(`-z` below 0) or past the beam's end (`+0x148`, when not 0); full to the
beam's start (`+0x144`), then falling linearly to its end; full within the
first radius (`+0x14c`, 498 here), falling linearly to 0 at the second
(`+0x150`, 500; `se2_2`'s 800 and 850); nothing at 1/128 or below. Its
direction is `lightVector` (0, 0, -1) through the light's matrix, its
priority 0 (`ccCreateLight`). The rooms' own models are unlit (mtype 0),
so only the characters feel it, and only where they stand within the
beam. `tools/test_lights_rs.py` checks the port's `chara::light_matrix`
with direct lights among the others against the game's `SetLightMatrix`.

**`GotoNextRoom`** sets `specialRoom` (`WORLD_MAN` +0x160) to -1 when it
leaves a room (the constructor also sets it to -1, at 0x005b8e3c). Before
the warps and the doorway, it takes a branch by `isEventArea` when the room
behind the door is a row of the branch's types:

| areas | types | `CheckAreaBan` area | then |
| --- | --- | --- | --- |
| 108, 73, 47, 66, 46, 27 | any, 16 on | the same | 66: `SetAreaBan(66, game.dungeon, level, next)`; return -255 |
| 23 | 25 | 23 | stand at the room's `OBJ_user_point`; `specialRoom` 0 |
| 25 | 28, 25 | 25 | the same |
| 48, 71, 77, 101 | 29, 30, 31, 32 | the same | the same |
| 16 | 26 | 108 | a temporary anm of `ANM_se1_7sh2a` made and deleted; the doorway |
| 91 | 35 | 108 | stand at spccs' `DMY_marker01` (decoded x, y, z, 1) plus (centre, 0, 1): w 2 |

`CheckAreaBan(area, game.dungeon, level, next)` reads the save's 32
entries of four signed bytes (+0x6548). For a banned room, `GotoNextRoom`
builds the room it left again (`SetRoom(level, here)`) and returns -100,
and `Enter` does nothing. The user point comes from a temporary anm of the
room's Anime chunk at `T(centre)`, stepped once: `OBJ_user_point`'s world
translation, w 1. `SetRoom(level, next)` and `roomEnterFlag` 1 follow, and
the answer is the room. `DUNGEON::RoomSelect` has the same stand for areas
71 and 77 (types 30, 31). A roomless row (16-24, 34) that no branch takes
cannot be walked into: `SetRoom` would read through its null anm. Of the
dungeons checked, only area 67's has one behind a door (floor 2, room 3,
type 34), and 67 has no branch.

`-255` goes to `WORLD_MAN::Enter`'s `ChangeArea(1, n)`
([above](#entering-and-walking-a-dungeon)). Area 27's is field 1, whose
arena `FieldWorld` builds.

**The ban block.** `SetDoor`'s tail (0x005c8370) deletes the block
(`~ccClump`, +0xd3880). With `game.field` not 0 it then asks
`GetBanRoom(info)` (0x005b61e0). That call takes the first story row whose
room `CheckAreaBan(game.field, game.dungeon, floor, index)` finds banned,
and returns its floor and index, and `next` = the last of its
`next0`-`next3` that is not 20. None found: floor -1. When that `next` is
the room being set, on its floor, a `ccClump` of the dungeon file's
`CMP_o_block_m0_` stands in the doorway toward the banned room:

1. The candidates are the room's gate walls (`GetSubstAdrs("OBJ_w_0g10_*")`).
   With none, its door dummies (`OBJ_0pae0_*`).
2. Each candidate's place is its local matrix turned by
   `sceVu0RotMatrix((0, 0, rotate))` and moved by
   `sceVu0TransMatrix((pos[f][i], 0))`.
3. `ccGetDist` (a ground distance) is measured from the banned room's
   centre to each of the first four places. A missing one counts as
   400,000.
4. The nearer of 0 and 1 is set against the nearer of 2 and 3. A tie goes
   to the later.
5. The block's local matrix becomes that place. `ccClump::HitEnable(1)`
   puts each node's hit on the list's tail (type 1), and
   `ccClump::SetHitMatrix` gives them all that matrix and its inverse.

`DUNGEON::Draw` draws the block after the room, and `DeleteRoom` deletes
it. Event 25's `area_ban 23 0 2 2` bans the shrine, so floor 2's room 3
gets the block in its doorway to room 2. `GetBanRoom` reads `game.field`,
so area 16's and 91's rooms (banned under 108) get none. The port reads
the save's bans when it builds a room: `DungeonArea.bans`, copied in by
`new_banned`, by `GotoNextRoom` from its save, and by the field world
before `RoomSelect` and `MoveDoor`.

**`specialRoom`** 0 hides the minimap: `DrawMap` returns while it is not
-1. It also makes `ccSndSQLoad` on a dungeon's room change load the story
area's bank (`sqDataEvent`): `piney_audio::setup_context`, fed by
`FieldWorld::special_room`.

## The doors and the events

What the entry control and the event scripts do to a room's doors, what
a story dungeon hands the event manager, and the 2D map the party's path
finding reads. All in gcmn but `SetEventData` and the `WORLD_MAN` getters
(main).

**`DUNGEON`'s door words.** +0x1c `CloseStart`, +0x20 `lockNum` (the
room's door dummies), +0x24 `lockOff` (the last opening step's
`_AnimateForward` answer), +0x28 `doorAnm`, +0x2c `doorFlag` (the doors
stand open), +0x48 `stillOpenDoor`. `SetRoom` sets `stillOpenDoor` and
`CloseStart` to 0.

```
SetDoor(f, i)     0x005c7c30  lockNum .. doorFlag = 0; the old door anms
                              deleted; a door anm per OBJ_0pae0_ dummy (at
                              most four), each: ccCheckActiveObject(f, i)
                              true (no enemy or magic circle of the entry
                              control's lists belongs to the room, on or
                              off): doorFlag = 1, frameMax _AnimateForwards
                              (open); else one (shut); HitEnable(1),
                              SetHitMatrix
OpenDoor(f, b)    0x005c8820  the same, every door run its whole length,
                              and doorFlag = stillOpenDoor = 1 (set per
                              door: none, neither)
CloseDoor(f, b)   0x005c8f50  the same, every door one step (shut); no flag
CloseDoor2(f, b)  0x005c8800  stillOpenDoor = 0; CloseStart = door[0]'s
                              frameMax (door[0] null: read through null)
```

`OpenDoor` and `CloseDoor2` are the event instructions `open_door` (case
158) and `close_door` (159), on `game.floor`, `game.block`.
`ccEntryEventMng` (main 0x001b62e0) calls `CloseDoor(game.floor,
game.block)` in a dungeon for each event entry it makes (the `entry_mc`
portals, the `entry` enemies of types 5 and 6), so a room an event fills
starts shut.

**When.** `ccEntryEventMng` runs in `ccThEntryCtrl`'s first slice, before
its first `ccTscb::Breath` (gcmn 0x00431b00, after `restoreEntry`,
`initEntryCCS` and `WORLD_MAN::EntryGimmick`); `ccSetupGameCtrl` starts
that task (priority 64) in the slice that ends the set-up (`GO`,
`ccGetStartPositions`, `rebootSpcManager`, `ccEnableThEvent(4)`,
0x00169400-0x001694d4), and the loop's first frame comes after the breath.
The doors are therefore shut before the event's first pass that plays:
event 4's trap room (block 6) runs `open_door`, one frame, `close_door`,
and the leaves come down from open over the animation's 46 frames
(6.4 a frame, 294 in all, from 290 to about the shut -9) as the camera
watches, with no jump at `SetDoor`. The port runs the set-up on the
frame of `Phase::Play(0)` (`FieldWorld::entry_setup`), a frame before the
event's first play pass; when it ran in the entry control's first loop
frame after that pass, the doors were made shut again after `open_door`
and `close_door` sank the leaves from shut into the floor, to jump back up
at `SetDoor`.

`MoveDoor(here)` (0x005cd3d0), from `Draw` each frame. Its sounds come
from two jump tables by `DUNGEON.type` (+0x10), @8659 for the opening and
@8664 for the closing, which agree: 45, 46, 47, 51 for types 0-3 and again
for 4-7, 48 for 8 and 9. The port plays them as the field's weather sounds
are played (`DungeonArea::door_se`, `FieldWorld`'s ambient calls);
`the_doors_sound_as_they_open` hears area 26's (type 3: 51) on the walk to
event 29's room.

```
doorAnm = ccCheckActiveObject()         no enemy, no magic circle switched on
room[level][here] not built: return
each door (nothing but its draw while stillOpenDoor):
  doorAnm:  not doorFlag: its opening sound (ccSeOn3D, by type) at the door
            lockOff = _AnimateForward(frameSpd); HitEnable(1)
            types other than 3, 7, 8, 9: SetHitMatrix
            lockOff and type 3, 7, 8 or 9: the leaf's (OBJ_w_9e20_) hit off
  else:     CloseStart == the door's frameMax: its closing sound
            CloseStart: the leaf's matrix = the unit matrix translated to
            its translation less 6.4 in z (it sinks); HitEnable(1),
            SetHitMatrix
doorAnm: doorFlag = 1
CloseStart == 1: SetDoor(level, here)   the doors made again (open or shut)
CloseStart: CloseStart - 1
```

So a door the room's entities hold shut opens a step a frame from the
first frame the entry control has nothing switched on, and `close_door`
lowers each leaf into the floor over the animation's length, then
rebuilds the doors as `SetDoor` finds the room. `ccAnm::HitEnable(1)`
appends only the objects not on the list, so a leaf taken off and put
back moves to the list's tail.

**`WORLD_MAN::SetEventData()`** (main 0x001a3c40), from `ccSetupGameCtrl`
between `ccStartThEvent` and `ccEnableThEvent(0)`: in a dungeon
(`game.area` 2) of a story area with an `EditDungeon` entry
(`GetEditDungeonPtr(eventAreaNumber)`, 0x005b6720: the area's first
entry, whichever dungeon the party is in):

```
each ROOMDATA row with eventFlag (+0x20)   ccEvent::SetEventPoint(floor, index, eventFlag)
each GIMMICKDATA row, in order:
  type 2    ccEvent::SetEventPos(floor, index, kind, dirc, (x, y, 0))
            dirc by direc: 0 -> 0, 1 -> pi, 2 -> pi/2, 3 -> -pi/2, else the last
            row's ($f20, the caller's register before the first)
  type 3, kind 7-26   warpPoint[kind - 7] = (x, y, 0), its room word = index
```

`SetEventPoint` (main 0x001b3620) and `SetEventPos` (0x001b3590) fill the
first of the 16 slots whose number is negative, nothing when none is. An
event's `set in_point n` is true in the room of point n: story area 14's
`D0001` makes points 4 (floor 0 room 0), 1 (room 1), 6 (room 2), 2 (room
3), 5 (floor 1 room 4) and 3 (floor 1 room 1), where TEACH-D's blocks 4,
2, 3, 5-6, 7 and 8 play.

**The 2D map.** `WORLD_MAN::Get2DMapPtr()` (main 0x001a22b0) is the
current floor's `u8[256][256]` at `DUNGEON` +0x33538 + level << 16, what
`MakeMiniMap` fills ([battle](battle.md#party-navigation)).
`Get2DMapInfo(info)` (0x001a22f0), in a dungeon only, is
`GetRoom2DPos(info)` (0x005cf080) of room `game.block` on the current
floor: the size 80 when an `EditDungeon` row of type 16 or more has this
floor and index, else 10, 20 or 40 by `minimap[level][block].size`; the
corner the room's centre / 300 (`WORLD_MAN::Get2DPos`, `fptosi`) less
half the size. A new `ccNavi` in a dungeon takes it
(`SetDungeonMapInfo`); `Draw` runs `SetPathFindingMap` once the room is
built while `roomEnterFlag` (+0x4c, set by the constructor and
`GotoNextRoom`) is set, and clears it.

## Checked

`tools/test_dungeon_rt.py` decodes the dungeon's scene file with the
game's own `ccStream::DecodeSetup` in eemu, fills `DUNGEON`, `WORLD_MAN` and
`ccGame` as the constructor and `GO(2)` leave them, and runs the game's
`Generate`, `GetStartPosition`, `SetRoom`, `ccLandHitCheck`, `GetHeight`,
`GotoNextRoom` and `Draw` beside `crates/piney-world/examples/dungeon_probe.rs`, over
24 dungeons: story area 14's (the one `--mode dungeon:14` enters), nine
other story areas' (types 0-3 and, with flag 3, 5-7) and 14 random ones
(types 0-4, servers 0 and 2):

- 672 rooms: model, turn and centre, each floor's `UpRoom`, `DownRoom`
  and both startpos, and `SetFog` / `SetAmbient`'s arguments;
- the hit list of each of the 672 `SetRoom`s and of the room each
  `GotoNextRoom` built: 297,909 hit models identical in owner, order, type,
  `rm` and `im`, and 1,428 door leaves posed by a keyed rotation within
  1e-5 (rotation) and 0.05 (translation), at most 32 ULPs (the port's
  keyed rotations are interpolated in double precision);
- each `SetRoom`'s dressing, to the bit: the water (its anm and matrix, 6),
  the sparks (base, place, step, `life`, `cnt`, the `ccEff`'s place and
  transparency, and its palette: 738, 291 of them changed), the room
  lights (the glows' names, places and patterns, the omni light's place,
  colour and intensity: 1,183), the walls (40) and animated objects (134)
  by anm and matrix, and `fieldrand`'s seed and count after it. A story
  dungeon's `Generate` draws for its gimmicks (`SetAllGim`), which the
  port does not, so the port's `fieldrand` is set to the game's after it
  (9 dungeons);
- 26,880 `ccLandHitCheck`s in game.area 2 (8,807 on a floor): height,
  count, nearest contact, distance, attribute and
  `checkHitResultAttlibute` (undefined without a result with ground bits);
- 2,016 `GetHeight`s;
- 2,384 `GotoNextRoom`s from door cells and 128 from stairs rooms: the
  answer and `WORLD_MAN.position`;
- 9,536 `DUNGEON::Draw`s before and after each door's `GotoNextRoom`, at
  the spot the player left, the spot he arrives at and random points (the
  map, minimap, layers and door sounds stubbed): the cell's room, and
  whether `ccAnm::Draw` was given the room and how many doors.

Beside them, `KiteInDungeon` runs `cameraMain` and `ccPlayer::Main` over a
room the real `SetRoom` registered (`tools/test_world_rs.py`'s set-up, area
2, the `DUNGEON` behind `AddCenter` and `GetHeight`): 3,437 frames from the
start and other rooms, walking into walls and through two doors (three
`WORLD_MAN::Enter`s), every scheme and the eye view. 0 mismatches in all.
No entity is in any room (open doors).

`test_lake_dressing` runs four lake dungeons (type 8, `sd4`), which
`test_dungeons` leaves out: `Generate`'s rooms and `fieldrand`, then every
room's `SetRoom` (34): the hit list (3,081 hit models identical) and the
dressing, 18 statues and flowers and 3 waters. `test_draw_eff` runs
`DrawEff` 70 frames in 60 rooms with sparks or glows of the random
dungeons (`ccEff::Draw` recorded): every frame's glows and sparks (name,
place, pattern, transparency), each room light's intensity and
`fieldrand`, 4,200 frames. 0 mismatches. `piney-game`'s
`delta_and_theta_rooms_are_dressed` walks Kite through 28 rooms of two Δ
and two Θ random dungeons (types 0-3): each room's omni lights beside the
distant light, one a glow, and the glows' patterns running on.

`test_lake_night` runs six lakes by night (bgnum 2) or hacked (`ishack`
3), four of type 8 and two of type 9 (`sd9`, by day), with `field_eff`
decoded beside the dungeon's file: in 51 rooms `SetRoom`'s five
`FIREFLY`s (base, pattern, `fieldrand` after them), 960 frames of
`DrawEff` with them (the player at a room's centre, then 4,000 off so that
they are put back about him), and 480 frames of `DrawBG` over the clumps
the constructor makes (the halfword left in the material's substitute,
each `ccClump::Draw`'s matrix). 0 mismatches. `piney-game`'s
`a_lake_by_night_has_its_sky_and_fireflies` enters a Δ lake of bgnum 2:
the sky's four models are the frame's first, its scroll runs and the
five fireflies fly.

`test_doors` runs the doors over the same dungeons, six rooms of each:
`SetRoom` with `ccCheckActiveObject(f, i)` answering either way, then
scripts of `MoveDoor` frames (the entry control empty or not, the room
clear or not), `OpenDoor`, `CloseDoor` and `CloseDoor2` (the closing
played out busy, then cleared). After every step the door words, the
doors and the whole hit list (owner, order, type, `rm`, `im`), and each
`MoveDoor`'s sounds (how many, where): 75 rooms with doors, 12,846
`MoveDoor`s (6,309 opening, 3,405 while a door closes), 44 `OpenDoor`s,
35 `CloseDoor`s, 72 `CloseDoor2`s, 332 door sounds; 1,590,176 hit models
identical and 4,594 posed by a keyed rotation within 1e-5 and 0.05 (the
largest difference 8,192 ULPs). `test_dungeons` also compares
`GetRoom2DPos` for all 672 rooms. `EventDataAgainstGame` runs
`SetEventData` for every area with an `EditDungeon` entry, ten of them in
a field, and four without, `$f20` random: 102 cases, 204
`SetEventPoint`s, 254 `SetEventPos`es (recorded in order) and 10 warp
points; and the real `SetEventPoint` and `SetEventPos` on 300 random slot
states. 0 mismatches.

`test_room_select` runs `ClearRoom`, `SetRoom` and `DUNGEON::RoomSelect`
over the same dungeons, eight rooms of each and room 0 of floor 0: 208
rooms, 202 of them placing the party in front of a way in and 6 at the
centre. It compares the position, `level`, `roomEnterFlag` and the room's
whole hit list (18,969 hit models identical, 60 within the keyed-rotation
tolerance), and checks that the game's `mapHideFlag` is 2 and its map
status 3. 0 mismatches. `piney-game`'s `story_4_item_add_and_room` runs
`room 0 4` through the area host in story 4's dungeon: the scene changes
to room 4 and Kite arrives where `RoomSelect` put him.

`test_special_rooms` runs the story dungeons with rows of type 16 or more
(areas 23, 25, 16, 27, 91, 46, 47, 48's dungeon 1, 66, 67, 71, 73, 77, 101
and 108), each in a game of its own with its `spccs` decoded:

- every floor's rooms as `MakeRoom` leaves them, the event rooms' centre
  and turn, `DUNGEON.spccs`, and `GetStartPosition`;
- `SetRoom` of the 8 event rooms (types 25, 26, 29, 30, 31, 32, 35): the
  Anime chunk, the hit list, `SetFog` and `SetAmbient`;
- `GotoNextRoom` from up to three doors into each row, the room free and
  then banned (`SetAreaBan` on the branch's area). That is 56 walks: 29
  answering -100, 11 -255, 16 into the room. Each compares the answer,
  `specialRoom`, `roomEnterFlag`, `WORLD_MAN.position` and the hit list of
  the room built (25 of those with the ban block), plus area 66's bans;
- `WORLD_MAN::Enter` through the same doors (56): the first `ChangeArea`
  or `ChangeScene` it asks for;
- `RoomSelect` into the 8 event rooms.

The totals are 1,616 hit models identical and 24 within the keyed-rotation
tolerance (at most 5 ULPs), with 0 mismatches. The walks skip area 67's
roomless row, which the game cannot build. `piney-game`'s
`event_25_opens_in_area_23s_shrine` puts `--mode story:25` in area 23's
dungeon, with event 25's earlier blocks marked played, and walks Kite down
to floor 2 and into the shrine. There it checks the arrival at
`OBJ_user_point`, `specialRoom` 0, `se1_3`, the story bank asked for, block
16, its ban (23, 0, 2, 2) in the save, and the scene back to Dun Loireag.

## Structures

```
MAP_INFO    {u8 d, next, here}, realmap at DUNGEON+0x430 + floor * 0x4b00, [x][y]
FOOT        0x60: x, y, direc +8, live +0xc, size +0x10, exitSize[4] +0x11,
            exitNum +0x18, nextX +0x1c, nextY +0x2c, pos +0x40,
            roomNumber +0x50, oldDirec +0x54
DUNGEON     0xd3910: code +0, level +4 (the floor the party is on), type +0x10,
            isEventArea +0x14, floorRoomNum +0x18, CloseStart +0x1c,
            lockNum +0x20, lockOff +0x24, doorAnm +0x28, doorFlag +0x2c,
            symFlag +0x30, symFloor +0x34, symBlock +0x38, lakeFlag +0x44,
            stillOpenDoor +0x48, roomEnterFlag +0x4c, fog +0x50,
            fire[60] +0x54 (SNOW *), fogParam +0x144, fogIndex +0x148,
            clut0[64] +0x14c, clut1[64] +0x24c, water[3] +0x364,
            roomlight[32] +0x374, door[4] +0x3f4, clutAnm +0x404,
            texChunk +0x408 (water 0's frame-buffer copy), gimPos +0x424,
            mapHideFlag +0x42c, realmap +0x430, room[10][15] +0x2f230,
            rotate +0x2f488, pos +0x2f6e0, animIdx +0x30040 ({s8 k, s8 r,
            u8 size, pad, ROOM_INFO *} per room), startpos[2][10][4]
            +0x30590, minimap +0x306d0, roomdata +0x30894, edit +0x3351c,
            center +0x33520, envLight[2] +0x33530, UpRoom +0xd3538,
            DownRoom +0xd3560, light +0x370 (an event room's LGT_),
            object[10] +0xd3588 (ccClump *), anmobj[10] +0xd35b0,
            anmobj2[10] +0xd35d8, block +0xd3880 (the ban block's
            ccClump), ccs +0xd3884, spccs +0xd3888, firefly[5] +0xd38a8,
            leaf anm +0xd38bc, LEAF[20] +0xd38c0
ROOMLIGHT   0x40: patNum +0, patNum2 +4, effPos +0x10, effPos2 +0x20,
            eff +0x30, eff2 +0x34, light +0x38 (ccOmniLight), use +0x3c
SNOW        0x50 (type 3): life +0, type +4, cnt +8, range +0xc, speed
            +0x10, eff +0x14, basePos +0x20, pos +0x30, direc +0x40
saveData    areaBan[32][4] +0x6548 (area, dungeon, floor, block; free: -1)
WORLD_MAN   (the dungeon's part) position +0x20, prevFlag +0xf4, prevBlock
            +0xf8, prevFloor +0xfc, lastRoom +0x100, prevPos +0x110,
            eventAreaNumber +0x120, specialRoom +0x160, dungeonback +0x164,
            warpFlag +0x168, warpPoint[21] +0x170 (0x20 each: the point, its
            room byte at +0x10), minx..maxy +0x420, dungeon[3] +0x438
```

## Other volumes

Each volume's own `DUNGEON::Generate` in eemu against `tools/dungeon.py`, per
volume MUT, OUT, QUA:
- 120 random dungeons (414, 395 and 414 floors; 4163, 4081 and 4150 rooms);
- `MakeRealMap` on all 90 `EditDungeon` entries;
- `SetDungeonTypeFromField` on 12,288 inputs.

0 mismatches. `dungeon.py` reads the room tables (through `MakeFloor`'s jump
tables), `symroom`, the `EditDungeon` count and the type rule from the
executable. The room tables, `symroom`, `DungeonName`/`DungeonName2` and the
dummy-object counts of all 18 dungeon CCS files are identical on all four.

From Mutation on:
- **`DUNGEON` shrinks** from 0xd3910 to 0xce0 bytes (`WORLD_MAN::GO`'s `new`,
  MUT `0x001b5c34`). The per-floor arrays become `new[]` allocations whose
  pointers sit in `DUNGEON`: realmap +0x434, animIdx +0x7bc, minimap +0x7c0,
  roomdata +0x7c4, smallmap +0x7c8, rotate +0x7cc, pos +0x7d0, startpos
  +0x930 (now [floors][2][4]). `room[15][15]` is at +0x438, UpRoom +0x8b8,
  DownRoom +0x8f4, edit +0x934. Fields below +0x44 and gimPos are unchanged.
- **The floor count is a field**, `DUNGEON+0x430`: 10, or 15 for event area
  125. Infection has the constant 10. `MakeFloor`'s story path uses
  count − 1 (*read*, not modelled).
- **`EditDungeon` has 90 entries** (was 88). New: area 118 (34 rooms, 4
  floors) and 119 (43 rooms, 4 floors). 30 of the shared entries change,
  17 of them in room count; area 125 goes from 10 to 15 floors.
- **`SetDungeonTypeFromField`** (MUT `0x001b1a20`, OUT `0x001a77c0`, QUA
  `0x001aefd0`):
  - It uses the area-71/47 substitute records
    ([area keywords](area-words.md#other-volumes)) and searches 127 records.
  - It no longer reads `saveData+0x6772`. Instead, bit 62 of the u64 at
    `saveData+0x5ec8` sends a story area down the random path, where the
    fixed areas get plain types: 120→3, 66→1, 91→3, 16→3, 18→0, 21→2.
- The same bit sets `DUNGEON.ishack` (+0x40) to 2 (*read*, not modelled).

`ChooseRoomSize`, `FOOT`, `MakeRoom`, `SetAllGim` and `CheckEntryItemBox`
are unchanged.

## The breakables

`DUNGEON::EntryBreakObject()` (gcmn 0x005bff10), which
`WORLD_MAN::EntryGimmick` runs each time the party enters a room, places
the room's breakable objects. Its guard places none in the lake types (8,
9), in `game.field` 14 (the tutorial's dungeon), or in a story room with
an event. Otherwise it calls `EntryBreakObjectMain(pattern, id)`
(0x005bfc40) for `OBJ_0pr2*` (id 0), then `OBJ_0pr4*` .. `OBJ_0pr7*` with
the four rows its jump table (@4166) gives the dungeon's type:

```text
type 0, 4   8 10  7 12        type 2, 6   10  9 11 12
type 1, 5   8  9  7 12        type 3, 7   13 14 11 12
```

`EntryBreakObjectMain` walks the room's anm objects matching the pattern
(`ccAnm::GetSubstAdrs`, in the anm's order). For each it takes the
object's world translation and makes an entry with that position. The
entry is a gimmick (type 1) on area 2, the dungeon, `level` and
`game.block`, with `entRoot` 2 and `land` 0. Its row is the id given, or
for id 0 the four rows' `fieldrand(4)`th. It draws `fieldrand(4)` for
every object either way. Leaving the room deletes them: gimmicks with
`entRoot` 2 are not kept. The next entry makes them again.

The port: `DungeonArea::breakables_here` gives the dummies after the
guard, `combat::DungeonEntries` carries them, and `Combat::start_entries`
makes the entries after the idols with the dungeon's random numbers.
`piney-game`'s `the_rooms_have_their_breakables` walks area 26's dungeon
to event 29's room. Three rooms get two breakables each, and the event
room gets none.

## Unknown

- The palette swaps are drawn over the whole dungeon file from the start
  (the game's `ChangeClut` in the first `SetRoom` writes the shared
  models); what the door functions' own `ChangeClut` calls add is not
  checked, nor is a picture of a swapped dungeon compared with the game's.
- Type 32's leaves (`ANM_se4_2lea`, `LEAF`), a later volume's room, are not
  ported. How `ccLight` combines an event room's `LGT_` record with the
  place `SetRoom` gives it is not checked (the port moves the omni light's
  position), and neither is the draw of the ban block's nodes (the port
  draws the clump's models at its matrix).

- `EntryObject`'s palettes for the lakes' statues and flowers with
  `GetBG` not 0.
- A story dungeon's `fieldrand` after `Generate` (its gimmicks' draws).
- What `ccObj::Duplicate(0x2008)` and `(0x2000)` copy of the water's
  object: the port takes each water's `OBJ_f_8s40_` materials as its own
  (water 1 scrolled in U, water 2 in V). The pictures of the water are not
  compared with the game's.
- Whether a room's dummies can make more than 32 room lights (the case of
  several runs past `roomlight[32]` into `door[]`); none of the rooms
  checked comes near.
- The warps (`warpPoint`, filled by `WORLD_MAN::EntryGimmick`). A lake's
  way between its two dungeons is ported from the code (`Enter`, `GO(2)`,
  `GoField`) and run in `a_lake_keeps_both_its_dungeons` and
  `a_type_4_area_goes_down_into_its_second_dungeon_and_back`, not against
  the game's own run.
- A keyed rotation's last bits (the doors that swing): the port's
  interpolation (`piney_data::anim`) is in double precision.

- Why the interpreter's `DecodeSetup` reads a bad pointer (0x7c7c7b84)
  when `sd9` is decoded a second time after `sd4` (`test_lake_night` runs
  `sd9` last).
- What the save bit at `saveData+0x5ec8` bit 62 and `ishack` mean.
- Story-dungeon room models (`MakeRoom(ROOMDATA *)`).
- Why types 8/9 pick a down-stairs room on their one floor.
