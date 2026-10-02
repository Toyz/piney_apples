---
title: The memory card save
status: solid
volumes: all
covers: INF SLUS_202.67:0x00306be0 mcDirName, 0x00306d40 mcFname, 0x00306c00 iconBinTbl, 0x001661d0 ccMcard::DataWrite, 0x001664b0 ccMcard::SaveSys, 0x00171c20 ccSaveSys::MainProccess, 0x001716e0 ccSaveSys::CheckRightInfo, 0x00174320 ccSaveData::ccSaveData, 0x001743d0 ccSaveData::Init, 0x0033eb90 timeIdolRankDefStr, 0x00180ed0 ccSound::ccSound, 0x00167940 ccThMother; INF DATA/ICON.BIN; MUT SLUS_205.62:0x00171b50 LoadInfoPrevReq, 0x00171c00 LoadDataPrevReq, 0x001767e0 ConvGame, 0x001cac50 ccStartEventConvert, 0x0017af20 the save extension, 0x00175740 ccSaveData::Init, 0x00176270 ccSaveData::NewGame, 0x00177be0 ccSaveData::InitTradeItem, 0x0017a010 the trade count setter, 0x0017a9d0 GetItemList, 0x0017aa20 GetSpcTradeList, 0x0017aba0 GetSkillList, 0x0017ac30 the talkNum setter, 0x0017aca0 the partyTime setter, 0x0017adf0 the spcPresent setter, 0x0017aec0 GetSpcParam, 0x0017b050 the extension tail clear; INF SLUS_202.67:0x00178030 ccSaveData::CheckEventEntry, 0x001780d0 ClearEventEntry; INF gcmn.prg:0x00430200 ccEntryCtrl::deleteEventEntry, 0x0040ffb0 BOOK::GetBookItem, 0x0040ddd0 BOOK::GetBook01Item, 0x005a1930 ccSpcSetOperation, 0x0053af2c ccMenuCtrl::PartyInMenu's call check
worklog: 16, 24, 246, 338
---

# The memory card save

A save slot is the game's `ccSaveData` (34,096 bytes), and from Mutation on
a 0x854-byte extension after it, plus a 336-byte index of all twelve slots.
`tools/save.py` runs each volume's own save and load code in `tools/eemu.py`
against a memory card kept in Python, so everything below was produced by
the game's code on all four volumes unless marked *read*.

## Files

```
<dir>/icon.sys            964 bytes, "PS2D" header (ccMcard::MakeDir)
<dir>/<3 icon files>      from DATA/ICON.BIN via iconBinTbl
<dir>/<dir>               ccSaveSys.info[12], 336 bytes (ccMcard::SaveSys)
<dir>/dhdata01 .. 12      one slot each: INF 0x8530 bytes, MUT/OUT/QUA 0x8d84

<dir> = mcDirName[volumeNum - 1]
```

| | INF | MUT | OUT | QUA |
| --- | --- | --- | --- | --- |
| directory | `BASLUS-20267DOTHACK` | `BASLUS-20562DOTHACK` | `BASLUS-20563DOTHACK` | `BASLUS-20564DOTHACK` |
| `mcDirName[1..3]` | `00001`, `00002`, `00003` (placeholders) | `20562`, `20563`, `20564` | same | same |
| icons | `icon10/11/12.ico` | `icon20/21/22.icn` | `icon30/31/32.icn` | `icon40/41/42.icn` |
| icon bytes | 89102, 89166, 89102 | 88138, 88202, 88138 | 80794, 80858, 80794 | 80890, 80954, 80890 |
| slot file | 0x8530 (34,096) | 0x8d84 (36,228) | 0x8d84 | 0x8d84 |

The index is `<dir>/<dir>`. `MakeDir` writes all twelve slot files; `mcFname`
also has `dhdata13` and `BISLPS-00000HUCKER`, which nothing uses (INF's
relocations reference only `mcFname+4`, in `DataWrite` `0x00166278` and
`DataRead` `0x001667e4`). `ICON.BIN` is byte-identical on every disc; volume
v uses `iconBinTbl` entries 3(v−1) to 3(v−1)+2. `icon.sys` carries the title
from `mcTitleName` (e.g. `．ｈａｃｋ　ＩＮＦＥＣＴＩＯＮ`), with the line
break at byte 10.

Infection writes `saveData` itself (`DataWrite` at `0x00173e4c`). Mutation on
fill a `ccMalloc`'d 0x8d84-byte buffer member by member and write that, so
padding bytes come from the heap there (save sites MUT
`0x00174338`/`0x0017515c`, OUT `0x00173ac4`/`0x001748d8`, QUA
`0x00173954`/`0x00174768`).

### Infection slots of 0x8d84 bytes

A card exported from PCSX2 can hold Infection slot files of 0x8d84 bytes,
the later volumes' size, though Infection's own `MakeDir` makes them 0x8530
(`li 0x8530` at `0x001660d4`). In the one card seen, slots 1 and 12 hold a
cleared game (level 34, `clearFlag` 1). The index record's `sum` covers the
whole 0x8d84 bytes (0x599, against 0xc31b over the first 0x8530). The first
0x8530 bytes are the same `ccSaveData`. Infection's own load sums 0x8530
bytes and would refuse such a slot. The port takes either: the sum over the
volume's size, or, for a longer file, over 0x8d84. What wrote these files
is not known.

### PS2 card images

`piney_data::ps2card` reads a PCSX2 card image (`Mcd001.ps2`):
- 512-byte pages, each followed by 16 bytes of ECC when the image is a
  power-of-two count of 528-byte pages (8,650,752 bytes for 8 MB);
- 2 pages to a cluster;
- the superblock at cluster 0 (`page_len` +0x28, `pages_per_cluster`
  +0x2a, `alloc_offset` +0x34, `rootdir_cluster` +0x3c, `ifc_list[32]`
  +0x50);
- the FAT through the indirect clusters (bit 31 in use, 0x7fffffff a
  chain's end), relative to `alloc_offset`;
- 512-byte directory entries: mode +0x00 (0x8000 exists, 0x20 a directory,
  0x10 a file), length +0x04, first cluster +0x10, name +0x40.

`piney_desktop::card::import` copies the four volumes' save directories
(`mcDirName`) from such an image, from a PCSX2 folder card or from one
exported save directory. PCSX2's own `_pcsx2_*` files are left out, and a
save directory it replaces is moved to a `backup-<secs>` beside the card.

## Slot index

```
ccSaveDataInfo      28 bytes, 12 of them = 336
  +0x00  s8        status          0 empty, 1 used
  +0x01  s8        level           0..99, spcParam[0]'s
  +0x02  s8        clearFlag       0..4
  +0x03  s8        parodyFlag      0..1
  +0x04  char[18]  name            spcParam[0]'s
  +0x16  u16       sum             16-bit sum of every byte of the slot file
  +0x18  s32       playtime        1/60 s, below 0x0CDFE5C4 (999:59:59)
```

An empty record must be all zeros. A load recomputes `sum` over the slot file
and rejects a mismatch. `CheckRightInfo` is the same code on all four volumes:
level below 100, `clearFlag` below 5, `parodyFlag` below 2, play time below
0x0CDFE5C5.

Saving over a used slot asks for confirmation (result 0x801f), and is allowed
only when that slot's `clearFlag` is below `volumeNum` or equals the current
one; otherwise it is refused with message 4138, 4140, 4142 or 4144 by volume
(INF `0x00173c08`, MUT `0x00174200`).

## ccSaveData

0x8530 bytes on every volume: the bound of the constructor's clear loop (INF
`0x00174320`, MUT `0x00175640`, OUT `0x00174da0`, QUA `0x00174c30`). The
members match Infection's DWARF on every volume up to `+0x8432`, by the load
copy loops and by `eventFlag` (+0x54f8) being used the same way. Offsets and
sizes in hex:

```
  +0x000     0x18  char plName[24]
  +0x018     0x18  char plRealName[24]
  +0x030    0xb40  ccItemList itemList[18][40]
  +0xb70    0x18c  ccItemList plItemList[99]
  +0xcfc    0x140  char impItemList[320]
  +0xe3c    0x440  ccItemList spcTradeList[17][16]
  +0x127c   0xc00  ccItemList npcTradeList[48][16]
  +0x1e7c    0x48  char tpcTradeListSW[24][3]
  +0x1ec4   0x2d0  short skillList[18][20]
  +0x2194    0x78  GROWTH_PARAM growth[5]
  +0x220c    0x12  char talkNum[18]
  +0x2220     0x4  int partyMemberFlag
  +0x2224     0x4  int partyMemberCall
  +0x2228     0x4  int partyMemberCallStore
  +0x222c     0x4  int partyMemberExp
  +0x2230     0x4  int partyMemberSave
  +0x2234     0x2  short townMoveFlag
  +0x2236     0x1  char dtWallpaper
  +0x2237     0x1  char dtBgm
  +0x2238     0xc  int dtWallpaperList[3]
  +0x2244     0xc  int dtBgmList[3]
  +0x2250    0x14  int dtStrList[5]
  +0x2264   0x200  char mailList[512]
  +0x2464   0x400  short mailOrderList[512]
  +0x2864    0x80  char webnewsList[128]
  +0x28e4  0x1800  char bbsList[128][48]
  +0x40e4   0xf00  int eventEntry[160][6]
  +0x4fe4    0x64  int gateList[5][5]
  +0x5048    0x64  int gateListMark[5][5]
  +0x50ac   0x190  int gateRecord[5][20]
  +0x523c    0x3c  int wordList[15]
  +0x5278   0x280  short gateOrderList[5][64]
  +0x54f8  0x1000  long eventFlag[512]
  +0x64f8    0x50  char eventStatus[80]
  +0x6548    0x80  char areaBan[32][4]
  +0x65c8    0x14  int protectArea[5]
  +0x65dc   0x190  int fountainRecord[100]
  +0x676c     0x2  short fountainNum
  +0x676e     0x2  short erosion
  +0x6770     0x1  char newGameFlag
  +0x6771     0x1  char plcol
  +0x6772     0x1  char crisis
  +0x6773     0x1  char tactics
  +0x6774     0x1  char timeIdolRank
  +0x6775    0xc8  char timeIdolRankStr[5][2][20]
  +0x683d    0x20  char hyProccess[8][4]
  +0x685d     0x1  unsigned char hyItem
  +0x685e     0x2  short drainCount
  +0x6860     0x2  short drainEvolution
  +0x6862     0x2  short areaCount
  +0x6864     0x2  short circleCount
  +0x6866     0x2  short circleCompleteFieldCount
  +0x6868     0x2  short circleCompleteDungeonCount
  +0x686a    0x4d  char pcTradeCount[77]
  +0x68b7   0x139  char enemyKillCount[313]
  +0x69f0   0x9c8  short enemyKillArea[313][4]
  +0x73b8    0x44  int partyTime[17]
  +0x73fc    0x44  int spcPresent[17]
  +0x7440     0x2  short itemBoxCount
  +0x7442     0x2  short itemObjCount
  +0x7444     0x2  short symbolCount
  +0x7446    0x28  short pucciFoodCount[20]
  +0x746e     0x2  short itemIdolCount
  +0x7470     0x4  short fountainCount[2]
  +0x7474    0x12  short inuCount[9]
  +0x7488   0xf78  ccSpcParam spcParam[18]
  +0x8400     0x4  int playTime
  +0x8404     0x2  short assignPADaction
  +0x8406     0x2  short assignPADpersonal
  +0x8408     0x2  short assignPADchat
  +0x840a     0x2  short assignPADoption
  +0x840c     0x2  short assignPADmap
  +0x840e     0x2  short assignPADok
  +0x8410     0x2  short assignPADcancel
  +0x8412     0x2  short assignPADcamIn
  +0x8414     0x2  short assignPADcamOut
  +0x8416     0x2  short assignPADcamReset
  +0x8418     0x2  short assignPADcamMode
  +0x841a     0x2  short screenX
  +0x841c     0x2  short screenY
  +0x841e     0x2  short mainVol
  +0x8420     0x2  short seVol
  +0x8422     0x2  short bgmVol
  +0x8424     0x2  short output
  +0x8426     0x1  char lastTown
  +0x8427     0x1  char drainDemo
  +0x8428     0x1  char vibration
  +0x8429     0x1  char camType
  +0x842a     0x1  char clearFlag
  +0x842b     0x1  char parodyFlag
  +0x842c     0x1  char voice
  +0x842d     0x3  char mapMode[3]
  +0x8430     0x1  char strWinMode
  +0x8431     0x1  char cameraMode
  +0x8432    0xfa  char reserved[250]         INF
                                           MUT on (*read*, from the copy loops):
  +0x8432    0x30    48 bytes, copied as shorts; NewGame zeroes +0x8432/+0x8434
  +0x8462     0xc    12 bytes, copied byte by byte
  +0x846e    0xbe    reserved
```

`ccItemList`, `ccSpcParam`, `GROWTH_PARAM`: see
`tools/dwarf1.py type SLUS_202.67 <name>`.

## Starting values: ccSaveData::Init

`ccSaveData::Init(flag)` (INF `0x001743d0`) writes the record's starting
values. It is a run of member fills, not a clear: what it does not name
keeps its bytes. The game builds its one save in `ccThMother`
(`0x00167940`): `new ccSaveData`, whose constructor (`0x00174320`) zeroes
the 0x8530 bytes and calls `Init(0)`, then, after `ccGame::Init`,
`Init(1)`. Title, New Game, Load and the desktop all start from that save.
`flag` 0 also writes the options; 1 leaves them. In the game's order:

```
+0x0018  plRealName[0] = 0
+0x0b70  plItemList[99]              id -1, category -1, num 0
+0x0cfc  impItemList[320]            0
         for c in 0..18:
+0x0030    itemList[c][40]           id -1, category -1, num 0   (0xa0 a row)
+0x1ec4    skillList[c][20]          -1                          (0x28 a row)
+0x2220  partyMemberFlag 0, partyMemberCall 0x3fffe, partyMemberExp 0,
         partyMemberSave 0 (partyMemberCallStore is not written)
+0x2234  townMoveFlag 1
+0x8426  lastTown 0
+0x8404  assignPAD*: action 0x40 (cross), personal 0x10 (triangle), chat
         0x80 (square), option 0x800 (start), map 0x100 (select), ok 0x40,
         cancel 0x20 (circle), camIn 0x08 (R1), camOut 0x02 (R2), camReset
         0x04 (L1), camMode 0x01 (L2)
+0x6770  newGameFlag 0
flag 0:  camType 0, vibration 1, voice 1, screenX 0, screenY 0,
         mainVol, seVol, bgmVol, output = ccSnd->mainVol, seVol, bgmVol,
         outputMode (ccSound +0x0, +0x8, +0x4, +0xc; each word stored as a
         short), drainDemo 1
+0x8400  playTime 0, clearFlag 0, parodyFlag 0
+0x4fe4  gateList 0, gateListMark 0; gateOrderList -1; gateRecord -1;
         wordList 0
+0x54f8  eventFlag 0; eventStatus 0; eventEntry -1
+0x2236  dtWallpaper = ORIGINAL_WALL_1 (0x00378710, 49); dtWallpaperList 0;
         dtBgm = MAX_WAVE_NUM (0x00378754, 50); dtBgmList 0; dtStrList 0
+0x2264  mailList 0 and mailOrderList -1; webnewsList 0; bbsList 0
+0x676c  fountainNum 0; fountainRecord -1; areaBan -1 (every byte)
+0x65c8  protectArea[i] = 0 for i < 32: the bound is areaBan's loop's, so
         the 27 words after protectArea, fountainRecord[0..27], end 0
+0x676e  erosion 0, crisis 0, tactics 7, plcol 0
+0x2194  growth[5]: the ten shorts 0, foodNum -1
+0x220c  talkNum 0
+0x6774  timeIdolRank -1
+0x6775  timeIdolRankStr[r][k] = strcpy of timeIdolRankDefStr[2 r + k]
         (0x0033eb90: ten char *, each rank's name then its time; the bytes
         after each NUL are not written)
+0x683d  hyProccess 0, hyItem 0, drainCount .. circleCompleteDungeonCount 0
+0x686a  pcTradeCount -1 (every byte); enemyKillCount 0; enemyKillArea -1
+0x73b8  partyTime 0, spcPresent 0, itemBoxCount, itemObjCount,
         symbolCount 0, pucciFoodCount 0, itemIdolCount, fountainCount 0,
         inuCount 0
+0x842d  mapMode 0
flag 0:  strWinMode 1
+0x8431  cameraMode 3
```

Init never writes `plName`, the trade lists (`spcTradeList`,
`npcTradeList`, `tpcTradeListSW`: `NewGame`'s `InitTradeItem` fills them),
`partyMemberCallStore`, `spcParam`, `reserved` or the padding.

`ccThMother` builds the save after the sound task has started and been
waited for (`ccSndSysWait`), so `Init(0)` copies `ccSound::ccSound`'s
(`0x00180ed0`) levels: volumes 256, output 1. The boot's save is therefore
the zeroed record with all of the above, both `flag` branches included.
An empty `itemList` entry is id -1: `ccSaveData::GetItemSlot` looks for one,
so a list left zero has no free slot.

Measured: `tools/test_save_init_rs.py` runs `Init(0)` and `Init(1)` in eemu
on zeroed, 0xff-filled and random records, with `ccSnd`'s levels the
constructor's or random, and the boot (ccSound's constructor, ccSaveData's
constructor over random bytes, `Init(1)`), and compares every byte with
`piney_data::save::SaveData::init` and `SaveData::boot`.

Not the game's: the port's slot load (`piney_desktop::savesys`, the
title's Load) repairs saves that the port wrote before it ran Init whole
(`SaveData::repair_port_save`). Those saves have only ok and cancel of the
eleven buttons and zero bytes where Init writes -1. If `assignPADaction` ..
`assignPADmap` are all zero with ok and cancel set, each zero assignment
gets Init's. A whole `itemList`, `plItemList`, `skillList`,
`mailOrderList`, `gateOrderList`, `gateRecord`, `fountainRecord` or
`areaBan` of zero bytes gets Init's fill. The game adds an entry to each of
these lists only once, so it can never leave one all zero. `pcTradeCount`,
`eventEntry`, `enemyKillArea` and `growth` are not repaired, because zero
is a state the game can reach in them.

## What the fields hold

The fields whose use the other pages describe:
- **`eventFlag[512]`**: a u64 per event. Bit `b` below 62 says block `b`
  has run, 62 done, 63 closed ([events](../engine/events.md#events-and-flags)).
- **`eventStatus[80]`**: counters the scripts set, add to, subtract from
  and test (`status_set` .. `status_sub`, the open condition `status`);
  what each index counts is the scripts' own.
- **The lists the scripts keep** ([event interpreter](../engine/event-vm.md#the-save)):
  `mailList` 0 none, 1 arrived, 2 seen, 4 read, 5 and 6 replied;
  `webnewsList` 0 none, 1 posted, 3 read; `bbsList` 0 none, 1 posted,
  3 read, 7 posted by `bbs_post7`; `gateList` and `gateListMark` a bit per
  story area per server, `gateOrderList` each server's areas newest first
  (-1 empty); `wordList` a bit per keyword; `protectArea` a bit per story
  area whose Virus Core is gone; `plcol` 1 once Kite has Data Drain;
  `partyTime` the frames characters 1-17 have been in the party. The
  scripts index them flat, as the game does: `bbsList[t][p]` is byte
  `48 t + p`, so a post past 47 lands in the next thread.
- **`partyMemberCall`**: a bit per member who answers a call. The
  Members menu refuses a member whose bit is clear (`PartyInMenu`, gcmn
  0x0053af2c), and the scripts turn bits on and off (`call_on`,
  `call_off`) or all of them at once (`call_lock` saves the word, sets
  bit 31 and clears bits 0-17; `call_unlock` puts it back). Init's
  0x3fffe is bits 1-17: every member answers.
- **`townMoveFlag`**: a bit per town the gate's Other Servers lists
  ([the field menus](../engine/field-ui.md)); Init's 1 is town 0 alone.
  The script op `town_move` adds one.
- **`tactics`**: the party's strategy as its CHAT order, 7 to 10:
  Operation Wonder Battle, Union Battle, Follow Me, Recover
  (`chatMenuStr` row `tactics + 1`). `ccSpcSetOperation` (gcmn
  0x005a1930) turns it into `partyStrategy` 0-3. Init's 7 is Wonder
  Battle.
- **`drainDemo`**: the Data Drain movie option (`DataDrainDemoMenu`):
  on, a drain plays its stream demo. Init turns it on.
- **`growth[5]`**: a Grunty per server: its food, type and growth
  ([Chaos Gate's Grunties](../engine/town02.md), the ride in
  [grunty-ride](../engine/grunty-ride.md)). Infection has the Grunty code
  and menus (`ccPGuso`, the Breeding and feeding menus).
- **`timeIdolRank`, `timeIdolRankStr`**: the Time Idol's best times, which
  `TimeIdolMenu` and `SetTimeIdolRank` write
  ([the field objects](../engine/field-ui.md)) and the board's Time Idol
  post shows ([the top page](../engine/toppage.md)).

Found here:
- **`eventEntry[160][6]`**: a bit per event-placed gimmick of each field,
  set while it is still there. `ccSaveData::CheckEventEntry(n)` (0x00178030)
  tests bit `n` of row `game.field - 1` (`n` below 192, the field 1 to 160);
  `ClearEventEntry` (0x001780d0) clears it. `ccEntryCtrl::deleteEventEntry`
  (gcmn 0x00430200) calls it when a box is opened, an object broken, a
  Virus Core taken or an idol used (`ccGimBox::boxMain`, `objectMain`,
  `virusMain`, `ccGimIdol::main`), for a gimmick whose `param[0]` names an
  entry. A field built later skips or changes the cleared ones
  ([battle](../engine/battle.md)). Init's -1 has every gimmick there.
- **`hyProccess[8][4]`**: how far each Ryu Book (key items 273-280) has
  paid out. `BOOK::GetBookItem` (gcmn 0x0040ffb0) runs the book's own
  `GetBook01Item` .. `GetBook08Item` for each level up to the one reached.
  Each gives its reward only when `hyProccess[book][part]` is below that
  level, and stores the level once `BookAddItem` has added the item.

Written but never read, on every volume that has them:
- **The blocks at +0x8432 and +0x8462** (Mutation on). Their only users
  are `Init`, which zeroes them, and `ccSaveSys::MainProccess`, which
  copies them in and out of a slot; no other instruction of any
  executable or overlay forms those offsets.
- **The extension's 256 bytes at +0x754.** Of the code that loads the
  extension's pointer, only the clear and `MainProccess`'s copy touch
  them.

## The extension (Mutation on)

Mutation adds three party members (Tsukasa, Subaru, Sora: `charTbl` rows
18-20). Their data does not fit `ccSaveData`, so the constructor allocates
`new(0x854)` (MUT `0x00175650`, OUT `0x00174db0`, QUA `0x00174c40`), keeps
the pointer in the `$gp` global after `saveData` (MUT `0x0038b95c`, OUT
`0x00386cf0`, QUA `0x002795f8`), and zeroes it (MUT `0x0017af20`). Accessors
(`GetSpcParam`, `GetItemList`, `GetSkillList`: MUT `0x0017aec0`, `0x0017a9d0`,
`0x0017aba0`) send ids 18-20 there and 0-17 to `ccSaveData`. The layout is the
same in MUT, OUT and QUA:

```
  +0x000    0x1e0  ccItemList itemList[3][40]     characters 18-20
  +0x1e0     0xc0  ccItemList spcTradeList[3][16] entries 17-19
  +0x2a0    0x180  ccItemList npcTradeList[6][16] entries 48-53
                                                  (NPC codes 181, 180, 182, 183, 121, 120)
  +0x420     0x78  short skillList[3][20]
  +0x498      0x3  char talkNum[3]
  +0x49c      0xc  int partyTime[3]
  +0x4a8      0xc  int spcPresent[3]
  +0x4b4    0x294  ccSpcParam spcParam[3]
  +0x748      0x3  char pcTradeCount of characters 18-20
  +0x74b      0x6  char pcTradeCount of NPCs 181, 180, 182, 183, 121, 120
  +0x754    0x100  ?, 16 x 4 words (cleared by MUT 0x0017b050), copied as 32 x 8 bytes
```

The slot file is `ccSaveData` followed by the extension.

**The accessors** (MUT addresses; OUT and QUA have the same code). Each
sends ids 0-17 to `ccSaveData` and 18-20 to the extension:

| MUT | what | `ccSaveData` | extension |
| --- | --- | --- | --- |
| 0x0017a9d0 | `GetItemList(id)` | +0x30 + 0xa0 id | +0x000 + 0xa0 (id - 18) |
| 0x0017aba0 | `GetSkillList(id)` | +0x1ec4 + 40 id | +0x420 + 40 (id - 18) |
| 0x0017aec0 | `GetSpcParam(id)` | +0x7488 + 0xdc id | +0x4b4 + 0xdc (id - 18) |
| 0x0017aa20 | `GetSpcTradeList(i)` (character i + 1's) | +0xe3c + 64 i | +0x1e0 + 64 (i - 17) |
| 0x0017ac30 | `talkNum[id] = v` | +0x220c + id | +0x498 + (id - 18) |
| 0x0017aca0 | `partyTime` | +0x73b4 + 4 id | +0x49c + 4 (id - 18) |
| 0x0017adf0 | `spcPresent` | +0x73f8 + 4 id | +0x4a8 + 4 (id - 18) |
| 0x0017a010 | `pcTradeCount(kind, code) = v` | see below | see below |

`partyTime` and `spcPresent` count from character 1, so id 0 writes the
word before each array: the last of `enemyKillArea`, and the last of
`partyTime`. The trade count setter takes the trader's kind:

- bit 4, a party character: code 1-17 at +0x6869 + code, 18-20 at the
  extension's +0x736 + code;
- bits 0x18, an NPC: codes 30-79 at +0x685d + code (so `ccSaveData`'s
  last 10 bytes of Infection's 77 go unused), and the six codes above in
  the extension's +0x74b.

The docs had guessed +0x74b to be sound settings. The setter shows it is
the six NPCs' trade counts, the same NPCs as the extension's trade lists
48-53.

**Init (MUT 0x00175740, OUT and QUA the same).** It runs Infection's
Init with five differences:

- The lists of 21 characters go through the accessors.
- `talkNum` goes through its setter.
- The trade counts go through their setter: characters 1-20, NPCs 30-79
  and the six. Infection's last 10 bytes of `pcTradeCount` are left alone.
- `partyTime` and `spcPresent` go through their setters for ids 0-19.
  Id 0 writes the word before each, and id 20 is not written.
- It clears four rows of three at +0x8432 (two shorts, 12 bytes a row) and
  +0x8462 (bytes), then the extension's tail.

The constructor zeroes the extension before `Init(0)`.
`tools/test_save_init_rs.py`'s `InitLaterVolumes` runs each later
volume's own Init in eemu on zeroed, 0xff and random records and
extensions, with flag 0 and 1. It compares every byte of both with
`piney_data::save`'s `init`: 0 differences on MUT, OUT and QUA.

**NewGame (MUT 0x00176270).** It copies the same `charTbl` fields for 21
rows, through `GetSpcParam`.

- `InitTradeItem` (0x00177be0) fills 20 characters' and 54 NPCs' trade
  lists from `spcDefTradeList` and `npcDefTradeList`. Both are longer than
  Infection's, although the carried symbol sizes are Infection's.
- A Parody Kite starts at level 20, 50, 70 or 90 by `volumeNum`.
- `SetDefaultWord` is the same code on every disc. It ORs more words the
  later the volume.
- `NewGame(0)` then gives Kite important items:
  - MUT: item 5;
  - OUT: also item 70, two each of 0-5, and five each of 26-41;
  - QUA: also item 285, news 19-22 set to 3, mails 56-60, 62 and 63
    cleared, two more each of 0-8, and five more of 26-41.

`NewGameLaterVolumes` compares the title's part of a new game (the boot,
`NewGame(1)`, the Parody flag, `NewGame(0)`) with the port's, every byte
of both parts: 0 differences on the three volumes, with and without
Parody.

## Carrying a save forward

1. **Title screen.** `Data_Control::Data_Select` (demo) offers "load previous"
   only when a previous-volume record has `clearFlag >= volumeNum - 1` (INF
   `0x004016c0`, MUT `0x004146e0`, OUT `0x00410158`, QUA `0x00302a88`).
2. **`LoadInfoPrevReq`** (INF `0x00171940`, MUT `0x00171b50`, OUT `0x001710e0`,
   QUA `0x00170f70`): `MainProccess` reads the index of volume
   `volumeNum - 1` (1 when that is ≤ 0 or ≥ 4) into `infoPrev` (+0x150) and
   checks it. MUT reads `BASLUS-20267DOTHACK`, OUT `20562`, QUA `20563`: each
   volume reads only the one before.
3. **`LoadDataPrevReq`** (MUT `0x00171c00`, OUT `0x00171190`, QUA
   `0x00171020`) reads that volume's slot file and checks its sum.
   - MUT reads Infection's 0x8530 bytes into `ccSaveData` at the same
     offsets, skipping only padding (+0x221e, +0x7486, +0x852c). The
     extension keeps its zeros.
   - OUT and QUA read 0x8d84 bytes: `ccSaveData`, then the extension, again
     at the same offsets. No conversion is needed.
4. **`ccThDemo`** (demo), when the opening returns 4, calls
   `ccStartEventConvert` and `ConvGame` (INF `0x00400b20`/`0x00400b2c`, MUT
   `0x00413b38`/`b44`, OUT `0x0040f560`/`56c`, QUA `0x00301e60`/`e6c`).
   - `ccStartEventConvert` (MUT `0x001cac50`, OUT `0x001c12f0`, QUA
     `0x001c8970`) sets bit 62 (done) of `eventFlag[100·(volumeNum−1)]`:
     event 100, 200 or 300. Each volume's first story event opens on exactly
     that flag.
   - `ConvGame` (MUT `0x001767e0`, OUT `0x00176020`, QUA `0x00175f80`) resets
     every character not in `partyMemberFlag` (1-17 in INF, 1-20 later) to
     its `charTbl` row, with 40 empty items and 20 empty skills. It then calls
     `LoadGame`, which rebuilds the names, CCS pointers, default words,
     camera, display offset and sound. `InitTradeItem` resets every trade
     list, including the extension's. Finally it sets `newGameFlag` to 2.

Infection has the same path: with `volumeNum - 1 = 0` replaced by 1, it
reads its own directory and marks event 0 done.

## Unknown

None.
