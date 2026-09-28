---
title: Area keywords and the area generator
status: partial
volumes: all
covers: INF SLUS_202.67:0x0019e5c0 WORLD_MAN::SimGenerateCode, 0x0019e500 CopyWordParam, 0x001a2340 GetWordParamPtr, 0x001a25d0 GetWordParamPtr(int), 0x001a3370 GetWordParamID, 0x001a29a0 GetWordParamFromEvCode, 0x001a4290 ccCheckEventAreaNum, 0x001a4220 ccGetEventAreaInfo, 0x0019d3b0 WORLD_MAN::GetEventAreaInfo, 0x0019ceb0 WORLD_MAN::IsProtectArea, 0x001b04b8 ccEvAreaCodeAdd (event instruction 118), 0x001b7020 ccRegisterDifficultyEnemy; INF gcmn.prg:0x0042ed70 ccRegisterEnemyList; word_a1..word_c4, eventAreaInfo, dungeonData, ccEnemyListInfo; MUT SLUS_205.62:0x001b9330 ccGetEventAreaInfo, 0x001b1f10 WORLD_MAN::GetEventAreaInfo; OUT SLUS_205.63:0x001af500 ccGetEventAreaInfo
worklog: 11, 13, 21, 68, 78, 181, 220, 259
---

# Area keywords and the area generator

A field area is named by three keywords typed at a Chaos Gate. The three word
IDs seed a random area; each word may pin some of its attributes; a fixed
story area may pin more. `WORLD_MAN::SimGenerateCode` (`0x0019e5c0`) does all
of it, and `tools/areas.py gen` reproduces it exactly (checked against the
game code run in `tools/eemu.py`, 0 mismatches over 412 triples). The port
is `crates/piney-data/src/area.rs` ([The port](#the-port)). The
algorithm and the tables' layout are the same on all four volumes; the
differences are under [Other volumes](#other-volumes).

## Tables

All little-endian, in main's `.data`.

```
WORDPARAM           0x30 bytes     word_a1..word_c4, 0x00311790 - 0x003150bf
  +0x00  char *  word
  +0x04  s32     ID               unique across slots
  +0x08  s32     pri              higher wins
  +0x0c  s32     fieldType        255 = unset
  +0x10  s32     dungeonSize      0 = unset
  +0x14  s32     weather          255 = unset
  +0x18  s32     ground           255 = unset
  +0x1c  s32     object           255 = unset
  +0x20  s32     areaLevel        255 = unset
  +0x24  s32     enemyOfs         255 = unset
  +0x28  s32     itemOfs          255 = unset
  +0x2c  s32     circleOfs        255 = unset

EVENTAREA_INFO      0x54 bytes     eventAreaInfo 0x00315120, eventAreaInfoNum entries (126)
  +0x00  s32     code             the event area number
  +0x04  char *  wordA            NULL for records with no address
  +0x08  char *  wordB
  +0x0c  char *  wordC
  +0x10  s32     server           0 Δ, 1 Θ, 2 Λ, 3 Σ, 4 Ω
  +0x14  s32     circle           -> circleOfs, 255 = keep
  +0x18  s32     enemy            -> enemyOfs,  255 = keep
  +0x1c  s32     item             -> itemOfs,   255 = keep
  +0x20  s32     model
  +0x24  s32     flag
  +0x28  s32     type             -> fieldType, 255 = keep
  +0x2c  s32     bgnum            -> weather,   255 = keep
  +0x30  s32     dungeonNum
  +0x34  s32[8]  protect

DUNGEON_INFO        8 bytes        dungeonData[11], 0x003150c0
  +0x00  s32     levelMax
  +0x04  s32     roomMax
```

`word_<slot><group>`: slot `a`, `b`, `c` is the word's position in the
address; group 1-4 splits each slot. Word counts: a 29/25/22/24, b
28/25/24/26, c 28/26/23/25 - 305 in all. `dungeonData` is (2,5) (2,5) (2,7)
(3,7) (3,9) (4,7) (4,9) (4,11) (5,7) (5,9) (5,11).

## The algorithm

```
code = A.ID * 1000000 + B.ID * 1000 + C.ID          also stored in wordparam.ID
seed = code
rand():  randcnt += 1
         seed = ((seed * 109) mod 2^32 + 1021) mod 2^32 mod 0xfffffffe
         return seed

fieldSeed = rand(); dungeonSeed[0..2] = rand() x3

sort [A, B, C] by pri ascending (insertion sort, stable)

base.fieldType   = rand() % 11
base.weather     = clamp(base.fieldType, rand() % 10)
base.dungeonSize = rand() % 10 + 1
base.ground      = rand() % 3
base.object      = rand() % 3
base.areaLevel   = rand() % 5 + 1
base.enemyOfs    = rand() % 20 - 9
base.itemOfs     = rand() % 20 - 9
base.circleOfs   = rand() % 3

m = copy(sorted[0]); merge(m, sorted[1]); merge(m, sorted[2]); merge(base, m)
    merge(dst, src): each field of src that is set overwrites dst's

if a story area on the current server has words (A, B, C):
    its circle, enemy, item, type, bgnum overwrite base where not 255
    area 71 and save flag (saveData+0x5bc0 bit 62): enemyOfs = itemOfs = 119
    area 47 and volumeNum >= 3: enemyOfs = 78, itemOfs = 122
        (INF inline; from MUT on, substitute records instead - see below)

base.weather = clamp(base.fieldType, base.weather)
if base.dungeonSize >= 11: base.dungeonSize = 1
levelMax, roomMax = dungeonData[base.dungeonSize]
timeSym = (A.ID == 131)
```

`clamp(type, w)`: types 0-4 keep `w` < 4, types 7-10 keep `w` < 8, types 5-6
turn 8 into 4 and 9 into 5 and keep `w` < 4; otherwise 0.

The results land in `WORLD_MAN`: `A, B, C` at +0x14c..+0x154, the merged
`WORDPARAM` through the pointer at +0x158 (its `ID` +0x04 becomes `code`,
+0x0c..+0x2c the nine attributes; `word` and `pri` are not written),
`fieldSeed` +0x44, `dungeonSeed` +0x48, `eventAreaNumber` +0x120,
`fieldtype` +0x10, `bgnum` (the weather) +0xc, `timeSym` +0x134,
`dungeonLevelNum` +0x138, `dungeonRoomNum` +0x13c. `dungeonType[0]` (+0x34)
is set to 0 and `SetDungeonTypeFromField` (0x0019cf50,
[dungeon generation](dungeon.md)) called before the dungeon counts are set;
it reads the story area's `flag` and `saveData.crisis` (+0x6772), and
writes `dungeonType[1]` only for field type 4. The RNG is the globals
`seed` (0x00377d20), left at the thirteenth draw, and `randcnt`
(0x00378a80), which goes up by 13 - `SimGenerateCode` does not reset it;
`WORLD_MAN::GO` does before a generator runs.

## Enemies

`ccRegisterDifficultyEnemy` (`0x001b7020`, fields and dungeons only) and
`ccRegisterEnemyList` (`gcmn.prg:0x0042ed70`):

```
type = 6                                  if ccGame.field > 0 (a story area)
     = FIELD_ATTRIB[fieldType][weather]   otherwise, WORLD_MAN::GetFieldAttrb
rank = 25 * (areaLevel - 1) + 10 + enemyOfs, at least 0
     = the story area's `enemy`           in a story area that sets it
rank += ccGame.floor + 1                  in a dungeon
list = ccEnemyListInfo[server][type]      gcmn 0x005da200, 5 x 7 {int *list; int num}
register list[rank], list[rank+1], list[rank+2]   (ccRegisterEnemyRange = 3),
         staying on the last entry past the end
```

`FIELD_ATTRIB`, from running `GetFieldAttrb` over every input:

| fieldType | weather 0-3 | 4-5 | 6-9 |
| ---: | ---: | ---: | ---: |
| 0-3 | 2 | 2 | 2 |
| 4 | 3 | 3 | 4 |
| 5, 6 | 1 | 1 | 1 |
| 7 | 0 | 0 | 0 |
| 8 | 5 | 5 | 4 |
| 9 | 3 | 1 | 4 |
| 10 | 3 | 5 | 4 |

The lists (`enemyList00` .. `enemyList46`, 130 entries each) hold indexes into
[`enemyTbl`](../content/game-data.md). Each registered row also registers
its drained form, and a middle boss (row type 0x40) its base form's drained
form as well ([battle](battle.md), "Spawning").

`itemOfs` feeds `AreaItem` (gcmn 0x00544c00) the same way: the index into
the area's item list is `25 * (areaLevel - 1) + 10 + itemOfs` (the story
area's `item` in a story area), plus `floor + 1` and `rand() % 5`, at most
129. The lists by kind, server and element are in
[field UI](field-ui.md).

## Lookups

- `GetWordParamPtr(part, slot, id)` (0x001a2340): the word of `slot` with
  that ID, `word_<slot>1` to `word_<slot>4` in order; NULL for none.
  `GetWordParamPtr(id)` (0x001a25d0): any slot, `word_a1`, `word_b1`,
  `word_c1`, `word_a2` ... in order. `GetWordParamID(text)` (0x001a3370):
  the ID of the first word, in that order, whose text `strcmp`s equal; -1
  for none.
- `ccCheckEventAreaNum(a, b, c)` (0x001a4290): over `eventAreaInfoNum`
  rows, the first with `wordA` not NULL, `server` equal to `game.server`
  (+0x1c) and the three texts equal to those of words `a`, `b`, `c` (by
  `GetWordParamPtr(id)`); its code, else 0.
- `ccGetEventAreaInfo(n)` (0x001a4220): the first of `eventAreaInfoNum`
  rows with code `n`. `WORLD_MAN::GetEventAreaInfo(n)` (0x0019d3b0) and
  `GetEventAreaInfo()` (0x0019d350, with `eventAreaNumber`) search a
  constant 126 rows instead - the same in Infection, whose
  `eventAreaInfoNum` is 126.
- `WORLD_MAN::IsProtectArea()` (0x0019ceb0): 0 when `eventAreaNumber` is 0
  or has no row among the first 126, else `protect[1] != 0`. 28 of the
  numbers -1 to 130 are protected. Field generation reads it
  ([field](field.md)).
- `GetWordParamFromEvCode(n, part)` (0x001a29a0): the row with code `n`
  (first 126), then the word of slot `part` (groups 1 to 4) whose text
  `strcmp`s equal to the row's `wordA`, `wordB` or `wordC`; NULL when none
  does (area 94's `Howling`). A code with no row, or a row with no address,
  is read through NULL.

## Event instruction 118, `area`

`ccEvAreaCodeAdd` (`ccEvent::Execute` at 0x001b04b8, playing only) takes
one signed 16-bit operand `n` and switches on it unsigned through the jump
table at 0x00355f00:

- `n` 1-13: when `game.area` (+0x14) is not 0, `WORLD_MAN::Quit()`; then
  `eventAreaNumber = n`. Nothing is generated. Areas 0-12 have no address;
  area 13 has one (Λ), which this path never uses.
- anything else (0, 14 and up, negative): `SimGenerateCode(A, B, C)` with
  the IDs of `GetWordParamFromEvCode(n, 0)`, `(n, 1)` and `(n, 2)`.

The instruction sets nothing else: not `game.server`, not `game.field`.
`SimGenerateCode` matches the words against the story areas on the
current `game.server`, so on the story area's own server `eventAreaNumber`
becomes `n` and the area's overrides apply; on another it is 0 and the
area is the random one the same words make there.

## The port

`crates/piney-data/src/area/`: `AreaTables::of(volume)` holds the
twelve word tables, `dungeonData`, `eventAreaInfo` (`eventAreaInfoNum`
rows), the substitute records (Mutation on) and `volumeNum` of each
volume, generated from its disc into the build by `piney-gen` (the
keywords' text byte for byte; the port does not read the executable at
run time). The four volumes' tables differ: Mutation on adds the
substitutes for areas 71 and 47, and Quarantine renames slot b's word 100
from `Vengeful` to `Vindictive`. `sim_generate_code(tables, a, b,
c, server, flag71)` returns every field `SimGenerateCode` writes
(`AreaCode`, with `dungeon_type(tables, crisis)` for
`SetDungeonTypeFromField`); `ev_area(tables, n, server, flag71)` is
instruction 118 and `story_area_code(tables, n, flag71)` the same on the
area's own server; the lookups above are `word`, `word_by_id`,
`word_by_text`, `word_id`, `event_area_info`, `cc_event_area_info`,
`check_event_area_num`, `word_from_event` and `is_protect_area`.

Checked by `tools/test_area_rs.py`: the game's own functions run in
`tools/eemu.py` on a scratch `WORLD_MAN` against the `area_probe` example.
`SimGenerateCode`, 1,080 cases - the 112 story areas whose words are all
keywords, each typed on all five servers and once more with the crisis
byte set; areas 47 and 71 with the save flag and `volumeNum` 3 and 4; 400
random triples on random servers, about a fifth with the save flag and a
third with the crisis byte - comparing `A`, `B`, `C`, `fieldSeed`,
`dungeonSeed[3]`, `bgnum`, `fieldtype`, `eventAreaNumber`, `timeSym`, the
two dungeon counts, `dungeonType[0..1]`, the merged `WORDPARAM`'s `ID` and
nine attributes, `seed` and `randcnt`: 0 mismatches. `IsProtectArea` and
`GetEventAreaInfo(n)` for every `n` from -1 to 130, `GetWordParamFromEvCode`
for every code and part (378), instruction 118's generation for every code
outside 1-13 on every server (555 areas), and `GetWordParamID` for every
keyword and two non-words: 0 mismatches. The harness runs on any volume's
disc through `PINEY_VOLUME`.

## Story areas

113 records carry an address: Δ 22, Θ 12, Λ 28, Σ 34, Ω 17. Area 14 is
`Bursting Passed Over Aqua Field` on Δ, code 13026. `tools/areas.py events`
lists them.

`model` 1 (areas 1-13, 15, 16, 43, 66, 67, 91) makes `ccSetupGameCtrl`
load the area's event music bank for its field instead of the field
type's ([sound](sound.md)). `flag` 3 gives its dungeons the E types
([dungeon generation](dungeon.md)).

## Notes

The match against story areas compares the word strings, not IDs, and
requires the current server to match.

One story area (94, Σ, `Howling Hot-blooded 500 Lohan`) uses a first word that
is not a keyword in any table on the Infection disc. From Mutation on it is
`Barking`, which is.

## Other volumes

Every volume has the same 305 keywords by ID and the same 113 story areas
with an address, and `tools/areas.py` reads each executable's own tables
(the stripped ones through the names `piney-gen syms` carries). Each volume's own
`SimGenerateCode` in eemu against `areas.generate`, over every story area,
events 71 and 47 under their special conditions (the save flag set, and
`volumeNum` forced to 3 and 4), and 100 random triples: INF 218 cases, MUT,
OUT and QUA 219 each, 0 mismatches. `volumeNum` is 1, 2, 3, 4.

**Substitute records.** From Mutation on, the two special cases are no longer
inline. `ccGetEventAreaInfo` and `WORLD_MAN::GetEventAreaInfo` return a
different `EVENTAREA_INFO` under the same conditions:

| area | condition | MUT record | OUT record | enemy | item |
| ---: | --- | --- | --- | ---: | ---: |
| 71 | saveData+0x5bc0 bit 62 | 0x0032ea00 | 0x00324e20 | 119 | 119 |
| 47 | `volumeNum >= 3` | 0x0032ea60 | 0x00324e80 | 42 | 122 |

Each is a copy of the table's record: area 71's with `enemy` and `item`
changed, area 47's with only `item`. So area 47 keeps `enemyOfs` 42 in
Outbreak and Quarantine, where Infection's inline code says 78 - code Infection never runs, its
`volumeNum` being 1. `ccRegisterDifficultyEnemy` reads a story area's `enemy`
through `WORLD_MAN::GetEventAreaInfo`, so from Mutation on the rank of these
two areas comes from the substitute too (read from the code; not run).
`areas.py` finds the substitutes by reading `ccGetEventAreaInfo`.

**Table changes.**

- MUT: keyword ID 304 (b, group 4) `Beginning` becomes `Darkside`, and area
  125's address with it. Area 94's `Howling` becomes `Barking`. Four
  addresses lose a trailing space. In Σ and Ω areas, 19 unset (255) `type`
  and 19 unset `bgnum` values get a value, and one `type` changes (area 76,
  9 to 7); `enemy` changes in 18 areas and `item` in 16, all but one on Σ or
  Ω; `protect` changes in 7, `dungeonNum` in 5 (109-113, 1 to 0). A record for area 126, with no
  address, is added (127 in all).
- OUT: `Evil Eyed` becomes `Evil-eyed`, `Sun Colored` `Sun-colored`,
  `Grave Stone` `Gravestone`.
- QUA: `Vengeful` becomes `Vindictive`.

## Unknown

- What the values of `fieldType`, `weather`, `ground`, `object`, `circleOfs`
  produce in the game.
- `EVENTAREA_INFO.model` beyond the music bank, `.protect` beyond
  `protect[1]` and the event VM's item pairs, `.dungeonNum`.
