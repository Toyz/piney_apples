---
title: The World's top page and bulletin board
status: partial
volumes: INF
covers: INF SLUS_202.67:0x00168570 ccSetupToppage, 0x00169aa0 ccSetFileListToppage, 0x00307ee0 sqDataToppage, 0x0030a3a0 sqVolTblToppage, 0x0017b9c0 ccSndBgmCtrl (board case), 0x001602d0 ccScFade::EntryFlash2, 0x00177fa0 ccSaveData::CheckWriteBbs, 0x001671e0 ccGame::ChangeRequest, 0x00167320 InitScene, 0x001674a0 ChangeArea, 0x00167380 ChangeScene, 0x00306dc0 @1489, 0x0034bbf8 volumeNum, 0x00378e1c g_TP_pushFlag, 0x00378e20 g_TP_pushCntr, 0x0015a960 ccSprite::SetPrim; INF toppage.prg:0x00400880 ccAddRequestFileListToppage, 0x004008e0 ccThToppage, 0x004009c0 ccThToppageCtrl::ccThToppageCtrl, 0x00400c60 Init, 0x00400da0 _ChangeMode, 0x00400ed0 _SetCommand, 0x00400f50 Main, 0x00401010 _Enter, 0x00401130 _Normal, 0x00401380 _Exit, 0x004014f0 _BBS, 0x004016a0 ccIsKeyRepeat, 0x00401730 ccStrRangeCopy, 0x00401830 ccThBBSCtrl::ccThBBSCtrl, 0x00401b60 ccThBBSCtrl::Init, 0x00401f60 Release, 0x004020e0 _CreateReadTable, 0x00402220 CheckThreadData, 0x00402260 CheckNewMessage, 0x00402320 DrawBBS, 0x004023d0 InitWritingMsgPage, 0x00402530 DrawWritingMsgPage, 0x004029b0 ExitWritingMsgPage, 0x00402a90 InitThreadPage, 0x00402b30 DrawThreadPage, 0x00403050 InitMessagePage, 0x00403060 DrawMessagePage, 0x004038d0 SelectMessage, 0x00403b30 ShowMessage, 0x00404050 ccBBSThreadObj::Init, 0x004040a0 CreateViewTbl, 0x004041f0 BbsCheck, 0x00403f70 ~ccBBSThreadObj, 0x00404310 ccBBSMsgObj::Init, 0x00404490 _SetupTimeIdolMessage, 0x00404610 GetLineLengthNoColor, 0x00404720 GetLineText, 0x00404770 SetSaveState, 0x00404880 ccScrollBar::Init, 0x004048a0 SetRange, 0x00404900 SetPos, 0x00404980 toppageFileList, 0x00404990 @1088
worklog: 58
---

# The World's top page and bulletin board

Mode 4: the log-in screen of The World, a menu of three commands (LOG IN,
BOARD, QUIT) over an animated scene out of one CCSF file, `xdttopen0`, and
the bulletin board behind BOARD. It is the `TOPPAGE.PRG`
[overlay](../formats/prg.md), two source files: toppage.cpp
(0x00400880-0x00401728, 14 functions) and bbs.cpp
(0x00401730-0x0040495c, 35), with the board's tables (bbsmsg.cpp,
bbsmsgP.cpp; [the desktop text page](text.md)). The desktop's The World icon
asks for it with `ChangeRequest(4, 7)` ([the desktop](desktop.md)).
`crates/piney-toppage` is the port, checked against the game's code by
`tools/test_toppage_rs.py`.

## Setup

`ccSetupToppage` (0x00168570):

```
game.enableReset = 0; game.status = 3
fontOnFlip; game.layer->OnFlipExcept; ccDeleteAllThread
ccSys.bgColor = 0 (black); cc3d->Reset; ccBreathThread(2)
font = fontDef; scFadeDef->Init
ccStartThEvent; ccEnableThEvent(0)                 the event pass, phase 0
unless a request is pending (game.request[0]):
  task ccThLoadOverlay("cdrom0:\DATA\TOPPAGE.PRG") until it has loaded
  ccSetFileListToppage: cmnFileList, gcmnFileList, strcmnFileList,
    then toppageFileList (0x00404980): XDTTOPEN0.CCS, category 4
  ccFileExistCheck(0)                              the files loaded
  ccEnableThEvent(2)                               phase 2
  unless a request is pending:
    ccAllSoundOff; ccSndSQLoad(0)                  the board's bank, sqDataToppage
    ccLoadResourceFL; SetFrameRate(1); game.enableReset = 1
    start ccThDtMenu (priority 33, stack 0x1000), ccThToppage (33, 0x800)
    ccEnableThEvent(4)                             phase 4
    unless a request is pending:
      scFadeDef->Init; ccSnd.gameStart (+0x17) = 1; ccSndBgmCtrl
```

`ccSndBgmCtrl`'s board case (`sqLoadParam` 0, 0x0017b9c0) plays sequence 0
of the loaded bank when it is not already playing, as `ccSqPlay(0)` does
([sound](sound.md)). The event scripts run beside the page as on the
desktop, their blocks for `game_status` 3 ([events](events.md)); the
system menu task is the desktop's `ccThDtMenu`
([the desktop](desktop.md#the-menu-task)): every status test in it is
against 1, so on the board it behaves as on the desktop.

`ccThToppage` (0x004008e0): the task's name "TOPPAGE", `ccLayer::active =
sysLayer` (priority 0, the view `SetFrame(0, 0, 512, 384, 256, 192, 1, 1)`),
`new ccThToppageCtrl`, the delete callback `ccThToppageDelete`, the clear
colour black; then `Breath(1); Main()` each frame while `m_exit` (+0x2c) is
0. Nothing sets `m_exit`.

## ccThToppageCtrl

```
ccThToppageCtrl     0x44 bytes
  +0x00  ccThBBSCtrl *m_bc
  +0x04  ccAnm *m_anmEnter      ANM_xdttopst   the log-in, 151 frames, once
  +0x08  ccAnm *m_anmNeutral    @1088[volumeNum - 1]: ANM_xdttop1a on Infection
  +0x0c  ccAnm *m_anmLogin      ANM_xdtlog1a   the command highlighted, 121 frames, looping
  +0x10  ccAnm *m_anmBBS        ANM_xdtbbs1a
  +0x14  ccAnm *m_anmQuit       ANM_xdtqui1a
  +0x18  ccAnm *m_anmCamera     ANM_xdtcame0   stepped once
  +0x1c  ccAnm *m_anmCommand    one of Login, BBS, BBSNew, Quit
  +0x20  ccAnm *m_anmBBSNew     ANM_xdtbbs2a   BOARD with a post new
  +0x24  ccAnm *m_anmNew        ANM_xdtnew1a   the NEW mark
  +0x28  int m_mode             0 enter, 1 menu, 2 leaving, 3 board
  +0x2c  int m_exit
  +0x30  int m_start            unused
  +0x34  int m_cmd              0 LOG IN, 1 BOARD, 2 QUIT
  +0x38  int m_actProccess      zeroed, unused
  +0x3c  int m_actCount
  +0x40  int m_bBBSNew
```

`@1088` (0x00404990) names `ANM_xdttop1a` .. `ANM_xdttop4a`; `_ChangeMode`
copies it to the stack at `sp + 48` and reads `sp + 44 + 4 volumeNum`, so
volume 1 takes the first.

The constructor (0x004009c0): the nine `ccAnm`s (`m_anmCommand` null),
`m_bc = new ccThBBSCtrl` and `m_bc->Init(this)` (the board built at once,
below), `m_exit = 0`, `Init`, `_ChangeMode(0)`, `_SetCommand(0)`. `Init`
(0x00400c60) sets every animation of `xdttopen0` (neutral as
`ANM_xdttop1a`) and steps the camera once.

`_ChangeMode(mode)` (0x00400da0): `m_mode = mode`, `m_actProccess` and
`m_actCount` 0, `m_anmNeutral` set again to `@1088[volumeNum - 1]` (so the
idle animation restarts on every change); mode 1 takes `m_bBBSNew =
CheckNewMessage()`; mode 2 calls `ccSoundFadeOut` unless `m_cmd` is BOARD,
then `EntryFlash2(30, 1, 0x80000000, 0, 0, 512, 384)`. `_SetCommand(cmd)`
and `_Normal` pick `m_anmCommand`: Login, BBSNew or BBS by `m_bBBSNew`,
Quit.

`Main` (0x00400f50): `SetView(ccLayer::active->view, m_anmCamera->cam, 0)`,
then by `m_mode`:

| mode | function | each frame |
| ---: | --- | --- |
| 0 | `_Enter` (0x00401010) | step and draw `m_anmEnter`; at its frame 1 sound 9, at 60 sound 10; otherwise, when it has ended or cancel is pushed, `EntryFlash(30, 0x80e0ffff, 0, 0, 512, 384)` and mode 1 |
| 1 | `_Normal` (0x00401130) | the menu, below |
| 2 | `_Exit` (0x00401380) | step and draw neutral, the command and (with `m_bBBSNew`) NEW; `m_actCount++`; once it was 30 or more: LOG IN `ChangeRequest(5, 8)` and `ChangeArea(0, saveData.lastTown)`; QUIT `ChangeRequest(3, 7)`; BOARD mode 3 and `m_bc->Init(this)` |
| 3 | `_BBS` (0x004014f0) | if the board is built, `DrawBBS`; once it sets `m_exit`: every animation set again (as `Init`, the camera stepped), mode 1, `EntryFlash(30, 0x80e0ffff, ...)` |

**The menu** (`_Normal`): step and draw neutral; up and down through
`ccIsKeyRepeat(key, 6)`, sound 6 each, wrapping over 0-2; `m_anmCommand`
by the command; then OK (`assignPADok`, no menu test) with
`CheckOperate(cmd + 6)` and `CheckOperate(cmd + 25)` both passing: sound 13
for LOG IN, else 4, `_ChangeMode(2)`, and for LOG IN and QUIT
`game.enableReset = 0`. Then step and draw the command's animation, and NEW
with `m_bBBSNew`. The events lock the commands as operations 6-8 and 25-27
([the event VM](event-vm.md)); `CheckOperate` records the first asked in
`operateSet`.

`ccIsKeyRepeat(key, n)` (0x004016a0) uses two main-executable globals: a
push returns 1 and sets `g_TP_pushFlag` (0x00378e1c); a release clears it
and returns 0; a repeat bit counts `g_TP_pushCntr` (0x00378e20), shared by
every key and every visit, and returns 1 when it reaches `n`, starting it
again. After ccPad's 15-frame delay a held key moves every 6 frames.

The fader: `EntryFlash(30, c)` is status 3 from `c` to clear over 31
frames; `EntryFlash2(t0, t1, c)` (0x001602d0) is status 5: clear to `c`
over `t0 + 1` frames, then `c` to clear over `t1 + 1`
([the title's fader](title.md#the-fader)). The command's black fade is
full on `_Exit`'s last frame.

## The hand-off

LOG IN: `ChangeRequest(5, 8)` queues mode 5 (`ccSetupNewGame`, GCMN.PRG)
and runs `InitScene` (0x00167320: area, server, town, field, dungeon,
floor, block and their `Prev`s all -1). `ChangeArea(0, n)` (0x001674a0)
with `n = saveData.lastTown` (+0x8426, `lb`) is `ChangeScene(0, n, -1, -1,
-1, -1)` (0x00167380):

```
townPrev = town;       town = n; saveData.lastTown = n      (for n >= -1)
serverPrev = server;   if town >= 0: server = @1489[town]
fieldPrev, dungeonPrev, floorPrev, blockPrev = the old values; all four -1
areaPrev = area;       area = 0                             (a town)
inBattle = inBattleCnt = 0; inBattleDist = 2200.0
ChangeRequest(6, 7)                                         ccSetupGameCtrl next
```

`@1489` (0x00306dc0) is `{0, 1, 2, 3, 4, 0, 1, 2}`. `sf` 8 does nothing
more; `sf` 7 turns the font and layer flips off (`fontOffFlip`,
`OffFlipExcept`) and calls `ccSleepNoSleepThread(1, 1)`, which by its name
puts the tasks to sleep, so the page's last picture holds.
The top page writes no other save field: the player's name, party and the
rest go on as the save has them; the town is the last one the event
scripts set (`last_town`, opcode 155).

QUIT: `ChangeRequest(3, 7)`, back to the desktop.

The Title Screen item of the system menu asks `ChangeRequest(1, 7)`.

## The board

```
ccThBBSCtrl         0x850 bytes
  +0x000  ccThreadPage m_ThreadPage      0x180
  +0x180  ccMsgPage m_MsgPage            0x380
  +0x500  ccWriteMsgPage m_WriteMsgPage  0x118
  +0x618  int m_iDrawState               0 threads, 1 a thread, 2 writing, 3 leave
  +0x61c  ccBBSThreadObj *m_ThreadTbl[128]
  +0x81c  ccThToppageCtrl *m_lpToppage
  +0x820  ccLayer *m_lpLayer             priority 127
  +0x824  ccMask *m_maskBbs              96 packets, TEX_xdtbbsp1
  +0x828  ccScrollMark                   blinkFlag, blinkInterval 30, count, setMask
  +0x838  int m_thNum
  +0x83c  int m_initialized
  +0x840  int m_exit

ccThreadPage        0x180: ccAnm m_Anm (ANM_xdtbbsa1); ccScrollBar m_sb (+0x110);
                    ccKanji *m_kanjiTbl[18] (+0x130); index (+0x178); startLine (+0x17c)
ccMsgPage           0x380: ccScrollBar m_sbLow, m_sbHigh; ccAnm m_Anm (+0x40, ANM_xdtbbsa0);
                    ccKanji m_kNoMsg (+0x150), m_kThread (+0x238); ccKanji *m_kTitleTbl[7]
                    (+0x320), *m_kMsgTbl[10] (+0x33c); index, thStart, thExtra,
                    iDrawState, msgStart, msgLast (+0x364 ..)
ccWriteMsgPage      0x118: thObj, msgObj, ccKanji kNewMsg, thIndex, msgIndex, frameCnt,
                    lineIndex, strCnt, noColLength, bDrawEnd, startLine, bWrite
ccScrollBar         0x20: sPos, sLength, bPos, bLength, lPos, lmin, lmax, nPage
ccBBSThreadObj      0x210: m_logicalindex, m_thList, ccBBSMsgObj *m_msgTbl[128],
                    m_maxVisibles (+0x208), m_read (+0x20c)
ccBBSMsgObj         0x480: m_timeRankName[5][61], m_timeRankTime[5][61], m_lpParent
                    (+0x264), m_msgList, m_MaxMsgTbl, m_read (+0x270),
                    char *m_sentenceTbl[128] (+0x274), m_lpTransName (+0x474),
                    m_logicalindex (+0x478), m_selectColorF (+0x47c, 7, unused)
```

### The save

`saveData.bbsList[128][48]` (+0x28e4, [the save](../formats/save.md)) holds a
state a post, byte `48 t + p` for post `p` of thread `t`: 0 not posted, 1
new (`bbs_post`), 3 read, 7 the player's own post waiting to be written out
(`bbs_post7`). The board reads the table of the save's mode
(`CheckThreadData`, 0x00402220: `bbsThreadTblP` when `parodyFlag`), 63
threads, 341 posts, at most 26 a thread and 39 lines a post (both tables).

`Init(tc)` (0x00401b60): `Release`; both cursors 0; the scroll bars
`m_sb.Init(62, 329, 18)`, `m_sbHigh.Init(62, 133, 7)`, `m_sbLow.Init(225,
165, 10)`; a new layer `ccLayer::Init(127, NULL)` with `SetFrame(0, 0, 512,
448, 256, 224, 1, 6/7)` (one unit a frame-buffer pixel); `new ccMask(96,
0)` on it with `SetTex("xdttopen0", "TEX_xdtbbsp1", 1)`; 18 + 7 + 10 new
`ccKanji` (`Init(3, 16)`) on it, the 10 of the post lines then
`SetPrim(32, 0)`: 32 packets and ctrl 1, which drops `Init`'s drop shadow;
`m_kThread` and `m_kNoMsg` put on it; both page animations set and stepped
once (neither moves again); `m_iDrawState` 0; `_CreateReadTable`;
`InitWritingMsgPage`; the scroll mark on the mask with interval 30 and
count 0 (the flag kept); `m_exit` 0, `m_initialized` 1. `Release`
(0x00401f60) deletes the mask, the layer, the thread objects and the page
kanji. The page cursors `startLine`, `thStart`, `thExtra`, `msgStart` are
not reset by `Init`.

`_CreateReadTable` (0x004020e0): a `ccBBSThreadObj` for each of the 63
threads with any post in a non-zero state, in table order.
`ccBBSThreadObj::Init` (0x00404050) runs `CreateViewTbl` (0x004040a0): a
`ccBBSMsgObj` for each post in a non-zero state, 7 taken as 1; then
`m_read = BbsCheck()` (0x004041f0): 1 when one of its posts is 1 in the
save, else 3. `~ccBBSThreadObj` (0x00403f70) writes 3 back for each post
whose `m_read` is 3.

`ccBBSMsgObj::Init` (0x00404310): `m_sentenceTbl` is the title, then
`"Author: "` and the poster (a `new[]` string), then a null (an empty line),
then `GetLineText(i)` (0x00404720, the string after the `i`th NUL of the
message) for each of `maxLines` lines. Post 1 of thread 29 is the Time
Idol ranking: `_SetupTimeIdolMessage` (0x00404490) puts over lines 7 + 5 i
and 8 + 5 i `"Time:   "` and `"Player: "` followed by rank `i`'s strings
from `saveData.timeIdolRankStr` (+0x6775: rank `i`'s player at `+40 i`,
time at `+40 i + 20`, `strcat` to their NULs). `SetSaveState(s)`
(0x00404770) sets `m_read` and writes 0, 1 or 3 into the save.

`CheckNewMessage` (0x00402260): whether any post of the 63 threads is 1;
the menu's BOARD then shows `ANM_xdtbbs2a` and NEW.

### The pages

`DrawBBS` (0x00402320) draws the page of `m_iDrawState`, then
`m_maskBbs->SendPacket()`; state 3 sets `m_exit` and sends nothing. OK and
cancel on the board are taken only while `CheckMenuType()` is -1; OK is
`assignPADok`, cancel `assignPADcancel`. Text is the thread and post
strings as `ccKanji::Disp(s, -1)` ([text rendering](font.md)), colour
`ccSpriteColorTable[7]` (128, 128, 128), the row under the cursor [4]
(16, 128, 16): the row's kanji takes [4] before its `Disp` and [7] after.

The mask's cells in `TEX_xdtbbsp1` (128 x 128, 4-bit; `wu`, `wv` in 1/16
texels from the texture's bottom as the [desktop's sprites](desktop.md#the-2d-draw)):

| cell | wu, wv | size |
| --- | --- | --- |
| NEW | 0, 0 | 32 x 16 |
| the tree's top | 0, 256 | 16 x 16 |
| a branch; the last branch | 0, 512; 0, 768 | 16 x 16 |
| a post | 256, 512 | 16 x 16 |
| the "more below" arrow | 0, 1024 | 16 x 16 |
| a scroll bar's box | 256, 256 | 8 x 5, drawn 5 x bLength |
| the date `dateindex` | 512, 256 dateindex | 64 x 16 |

`dateindex` is 0-5 in the plain table, always 0 in the parody one.

**The thread list** (`DrawThreadPage` 0x00402b30): the page animation; no
threads: cancel (sound 7) leaves. Otherwise 18 rows from `startLine`
(pulled to keep `index` on the page), row `r` at y `65 + 18 r`: NEW at
(50, y) when `BbsCheck()` is 1, the title at (100, y). The bar:
`m_sb.SetRange(0, m_thNum)`, `SetPos(startLine)`, the box at (472, 62 +
bPos). With threads below the page, the arrow at (240, 390) while the blink
flag is up (`m_count` counts each such frame; past 30 the flag flips and the
count restarts). OK (sound 4) opens the thread; cancel (sound 7) state 3;
up and down (`ccIsKeyRepeat`, sound 6) wrap.

**A thread** (`DrawMessagePage` 0x00403060): the page animation; the bar
(`m_sbHigh.SetRange(0, m_maxVisibles + 1)`, `SetPos(thExtra ? thStart + 1 :
0)`, box at (472, 62 + bPos)) from last frame's scroll; then `thStart` and
`thExtra` pulled to keep `index` on the page (6 rows, 7 once `thExtra` is 1:
the thread's title row has scrolled away). With the title (`thExtra` 0):
the thread's title at (80, 65), the tree's top at (80, 81), NEW at (48, 65)
when `BbsCheck()` is 1; else the tree's top at (80, 65). Posts from y 97
(81 without the title), 16 apart: the title at (112, y), NEW at (48, y)
when the save has it 1, the branch at (80, y) (the last branch on the last
post), the post cell at (96, y), the date at (`8 strlen(title) + 126`, y).
Then reading (`iDrawState` 1) `ShowMessage`, or choosing: with posts below,
the arrow at (240, 195); `SelectMessage` (0x004038d0): up and down (sound
6); above the first post `thExtra` falls (the title comes back) and at -1
the cursor wraps to the last post; past the last it wraps to the first; OK
(sound 4) reads; cancel (sound 7) leaves the thread: every page cursor 0,
then `InitWritingMsgPage` when a post was written this visit, else the
thread list with every thread's `m_read` taken again. A thread with no
post draws `m_kNoMsg` at (151, 100) instead; `_CreateReadTable` never
builds one.

**Reading** (`ShowMessage` 0x00403b30): `msgStart` held to `0 ..
maxLines - 7`, `msgLast = min(msgStart + 10, maxLines + 3)`; the bar
(`m_sbLow.SetRange(0, maxLines + 3)`, `SetPos(msgStart)`, box at (472, 225 +
bPos)); with lines below, the arrow at (240, 395); on the first page the
date at (`8 strlen(title) + 54`, 216); lines `msgStart .. msgLast` at
(48, 216 + 18 i), a null line skipped but counted. Up and down scroll (no
sound). Cancel (sound 7): `msgStart`, `msgLast`, `iDrawState` 0 and
`SetSaveState(3)`: the post is read in the save at once.

**The player's own post** (`InitWritingMsgPage` 0x004023d0,
`DrawWritingMsgPage` 0x00402530, `ExitWritingMsgPage` 0x004029b0): the page
resets (`lineIndex` 3, `noColLength` -1, `bWrite` 0) and
`ccSaveData::CheckWriteBbs` (0x00177fa0) finds the first post in state 7
(threads 0-127 by posts 0-47); none: the thread list. Found: its thread and
post objects, `noColLength = GetLineLengthNoColor(0)` (0x00404610: the
line's bytes, a `#W` `#Y` `#B` `#G` `#R` pair counting none), `bWrite` 1,
state 2. Each frame: the posts' page animation; `kNewMsg` (no layer of its
own, so `sysLayer`) at (100, 100) with the overlay's label (Shift-JIS,
which the font prints as underscores), drawn before the scene's opaque
models on that layer; `startLine` follows `lineIndex` down (one line a frame
once it is 9 lines on, until the end); the date and the bar as reading; the
lines above `lineIndex` in full at (40, 216 + 18 i), then line
`lineIndex` as `ccStrRangeCopy(line, 0, strCnt)` (0x00401730: the first
`2 strCnt` bytes, colour pairs copied whole and not counted). Every 6
frames `strCnt` grows by one; past `noColLength / 2` the next line starts;
past the last, `bDrawEnd`. OK shows the rest (`lineIndex` to the end); once
all is shown OK runs `ExitWritingMsgPage`: the post becomes 3 in the save
and its thread opens with the cursor on it (`bWrite` stays 1, so leaving
that thread looks for the next post in state 7).

**Scroll bars** (`ccScrollBar`, 0x00404880-0x0040495c): `SetRange(lmin,
lmax)`: `bLength = sLength` when `range = lmax - lmin` is 0 or less or
below `nPage`, else `nPage sLength / range`; `SetPos(l)`: `bPos = 0` for
`range <= 0`, else `(sLength - bLength) l / (range - nPage)`. A list exactly
a page long divides 0 by 0; there is no check, and the EE's `div` leaves -1
in LO (PCSX2's model of the hardware), so the box draws one pixel up. The
tables have such lists: a thread of 6 posts (thread 11) and 31 posts of 7
lines.

## Draw order

`ccLayer::active` is `sysLayer` (priority 0): every `ccAnm` draws there,
and `kNewMsg`. The board's layer is 127, the fader's font layer 240, the
system menu's 242. Within a layer the desktop's rules hold
([draw order](desktop.md#draw-order)): the mask's packets, sent last, are
drawn under the text sent before them.

## Sounds and music

`ccSeOn` (the common bank): 9 and 10 at the log-in's frames 1 and 60, 6 a
cursor move, 13 LOG IN, 4 BOARD, QUIT and on the board a thread or post
opened, 7 back on the board. The system menu's own sounds are the
desktop's (16 open, 17 move, 18 OK, 19 back). The music is sequence 0 of
`sqDataToppage` (0x00307ee0; `sqVolTblToppage` 0x0030a3a0, port 1 full);
LOG IN and QUIT fade it out (`ccSoundFadeOut`: every sequence over 8
frames, then stopped); the board keeps it.

## Checks

`tools/test_toppage_rs.py` runs the constructor and `Main` of TOPPAGE.PRG
in eemu beside the port's task, frame by frame, with the main executable's
calls hooked (the animations answered with the port's own playback): the
calls in order (animations, text with position, colour, ctrl and bytes,
every mask cell bit for bit, the board's layer and texture, the fader),
the sounds and requests, and the whole state (both controls' fields, the
three scroll bars, the writing page, every thread and post object, the
save's `bbsList`, the key-repeat globals, `enableReset`, `operateSet`, and
after LOG IN the scene `ChangeArea` leaves), over scripted runs (a fresh
save, event 1's posts, written posts, the Time Idol post, the longest
thread, a parody save with every thread, locked commands, the system menu
open under the board) and random pad runs.

## Unknown

- Whether the task's first `Main` runs in the frame the task starts; the
  port draws nothing for one frame.
- Whether `kNewMsg`'s label is ever visible past the board's scene.
- How `ccScFade` draws while the system menu holds the layers' flip.
- When `~ccThToppageCtrl` runs after a hand-off (its `~ccThBBSCtrl` writes
  back the posts read); the port writes them back at the hand-off.
- The EE's `div` by zero result is taken from PCSX2, not measured.
- Which events fill `timeIdolRankStr`.
