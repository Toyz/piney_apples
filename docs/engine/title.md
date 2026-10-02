---
title: The title screen
status: partial
volumes: INF
covers: INF SLUS_202.67:0x00168160 ccSetupDemo, 0x00174d70 ccSaveData::NewGame, 0x00175500 SetDefaultWord, 0x00176610 InitTradeItem, 0x00167700 ccResetPlayTime, 0x00345f10 spcDefTradeList, 0x00346790 npcDefTradeList, 0x00347f90 tpcTradeList, 0x0015fb80 ccScFade::SendPacket, 0x00160240 EntryFlash, 0x00160360 EntryFlash3, 0x00160400 EntryFade, 0x00171a60 ccSaveSys::BootCheckReq, 0x00171a80 BootCheckProccess, 0x00173f90 NextProccess, 0x00166c50 ccMcard::CheckPort, 0x0016a8e0 ccDtMenu::CheckMenuType, 0x0016a7a0 ccDtMenu::OpenMenu, 0x0016a150 ccDtMenu::Disp, 0x0016aeb0 ccDtMenu::SystemMenu, 0x0016ac00 ccThDtMenu, 0x00105900 ccDrawEnv::SetLightMatrix, 0x00139830 ccOmniLight::CheckRange, 0x00110d50 sceVu0NormalLightMatrix, 0x0013e350 ccSetMatrixPacket, 0x00379b10 sysLayer, 0x00307040 saveSysMsg, 0x001821d0 ccSndSQLoad, 0x001798f0 ccSqPlay, 0x00179aa0 ccSqStop, 0x00179b50 ccSqFade, 0x001794b0 ccSetMainVol, 0x00181440 ccSound::ccSceneFade, 0x00182fc0 sqStatusGet, 0x00307e30 sqDataTitle, 0x0030a290 sqVolTblTitle, 0x00175110 ccSaveData::LoadGame, 0x00171990 LoadSelectReq, 0x001719b0 LoadDataReq, 0x00171c20 MainProccess, 0x00171620 ccSaveSys::ccSaveSys, 0x00171810 StartReq; INF demo.prg:0x00403af0 NextDataLoad_Control::NextDataLoad_Control, 0x00406f40 ccOpening_Control::PlayNextData, 0x00409470 ccOpening_Control::SetNextData, 0x00400900 ccThDemo, 0x0040dc80 charTbl, 0x004043b0 ccOpening_Control::Main, 0x00404540 LogoMain, 0x00404710 MOVEcount, 0x004047e0 AllAnimate, 0x00405420 AllTransparency, 0x004059d0 AllDraw, 0x00405d20 ChangeMainAct, 0x00405e80 SetVolCcs, 0x00405f50 Init, 0x004066b0 PlayOpeningStream, 0x00406950 PlayBootMemCard, 0x00406b50 PlayNeutral, 0x00406bf0 PlayNewGame, 0x00406c20 PlayParodyGame, 0x00406c60 PlayDataLoad, 0x00406de0 PlayOption, 0x004070c0 MoveCurNut, 0x004072a0 SwitchCur, 0x00407640 EndDataLoad, 0x00407670 SetStream, 0x004076f0 SetBootMemCard, 0x00407950 SetNeutral, 0x00408240 SetNewGame, 0x004082c0 SetDataLoad, 0x00408ba0 SetOption, 0x00409d40 SetParodyGame, 0x00403c00 BootMem_Control::Main_Control, 0x00403d40 BootCheck, 0x00400bc0 Data_Control::Main, 0x00403710 CurRepeat, 0x00401e80 InfoMessage, 0x004020f0 YesNoDialogue, 0x00401c40 TimeAlphaCurDraw, 0x004037f0 DataLoad_Control::Main_Control, 0x00400e20 Slot_Select, 0x00401170 SlotStateData, 0x00401390 Data_Select, 0x00401750 LoadData, 0x00401990 DispButton, 0x00402360 WriteDataList, 0x00402570 SetLoadPar, 0x00402e40 Data_Control::Init, 0x00409df0 ccDecodeMpeg, 0x0040dc40 DemoFileList; MUT DATA/DEMO.PRG:0x00419630 the max cursor setter
worklog: 43, 45, 342
---

# The title screen

Mode 2: the company logos, the in-engine intro, then the title menu (New
Game, Load, Option) over an animated scene out of one CCSF file, `title1`.
It is the `DEMO.PRG` [overlay](../formats/prg.md). Everything on the menu is
a `ccAnm` of `title1`, drawn through one fixed camera; the only 2D is the
screen fader, the memory-card question and the load screen's text.
`crates/piney-demo` is the port, checked against the game's code by
`tools/test_demo_rs.py` (see [Checks](#checks)).

## Setup

`ccSetupDemo` (0x00168160): `ccAllSoundOff`, `ccGame.enableReset` 0,
background colour 0 (black), `ccDrawEnv::Reset(cc3d)` (ambient
`ccDefAmbientColor` 0.35, 0.35, 0.35), `frameRate` 1 (60 frames a second),
`ccSndSQLoad(7)` (the title's sequence bank), the overlay
`cdrom0:\DATA\DEMO.PRG` loaded on `ccThLoadOverlay`, then
`saveData->NewGame(1)`, the file list (`ccAddRequestFileListDemo`:
`DemoFileList[volumeNum - 1]` = `TITLE1.CCS`, and `DemoFileWINDOWList` =
`XWINDO10.CCS`), and two tasks at priority 33: `ccThDemo` and the system
menu `ccThDtMenu`.

## The task

`ccThDemo` (0x00400900), one `Breath` a frame:

```
ccSqStop(0); o = new ccOpening_Control; o->SetVolCcs()
r = 0; while !r: breathe; r = PlayBootMemCard()
if ResetOpFlg == 0:  r = 0; ResetOpFlg = 1; dtMenu->dispFlag = 0     first boot
else:                r = 1; dtMenu->dispFlag = 1;                    soft reset
                     ccSetMainVol(saveData->mainVol); ccSqPlay(0)
loop:
  while !r: r = LogoMain(); breathe; if r: dtMenu->dispFlag = 1
  PlayOpeningStream(); r = 0
  while !r: breathe; r = Main()
  r 2: saveData->NewGame(0)                           leave
  r 3: saveData->LoadGame()                           leave
  r 4: ccStartEventConvert(); saveData->ConvGame()    leave (volumes 2-4)
  r 5: ccSqStop(0); r = 0; dispFlag = 0; LogoAct = 3  the attract loop
ccSqFade(0, 0, 8, 3); Breath(2); game->ChangeRequest(3, 7); breathe forever
```

`ChangeRequest(3, 7)` starts the desktop (`ccSetupDesktop`, see
[the overview](overview.md)) for every way out: new game and load alike.

## The logos

`LogoMain` (0x00404540) by `m_LogoAct` (+0x1e4):

| LogoAct | plays | then |
| ---: | --- | --- |
| 0 | `ccDecodeMpeg("logo_b.pss", 0)` | 1, or 4 if skipped |
| 1 | `logo_c.pss` | 2, or 4 |
| 2 | `logo_h.pss` | 3, or 4 |
| 3 | `opening.pss`, with audio | 4 |
| 4 | `ccSetMainVol(saveData->mainVol)`, `ccSqPlay(0)` (the title music) | LogoAct 0, return 1 |

The files are `PSS/LOGO_B.PSS` and so on (`strFileOpen` prefixes `\PSS\`).
`ccDecodeMpeg` (0x00409df0) plays a movie to its end inside the call and
returns `retFlag`: -1 when a push of ok, cancel or START came after the
decoder had passed 10 frames (`readMpeg`), else 0. Every time `LogoAct`
becomes 4, `LogoMain` breathes 60 frames (nothing drawn) before it returns;
the next call starts the music.

## The opening stream

`PlayOpeningStream` (0x004066b0) starts `ccThExecuteStream` with a stream
number and waits for it: volume 1 plays stream 0 (`streamTbl[0]`: `title1`,
then `title1_st1`), or 1 with `m_ParoFLG` (`title1_st2`), and counts it to
frame 410 (450 with `m_ParoFLG`). It waits for the stream to start
(`ccGetStreamAdrs()`, then its +0x17e bit 3), then each frame, before the
stream's own task (both at 33, the stream started later):

```
cancel pushed (assignPADcancel) and s1:  EntryFlash(50, 0x80ffffff); s1 = 0
the stream task's param +0x14 == -1:     return
ccGetStreamFrame() == count - 20 and s1: EntryFlash3(20, 20, 5, 0x80ffffff); s1 = 0
Breath(1)
```

So one flash at most. Played through, white comes in over the stream's
last 20 frames (in over 21, held 6, out over 21) and the menu's first
frames come out of it. A cancel only flashes: the stream ends on its own
skip rule, under 50 frames of white, and the end flash never comes.

The port runs this loop over the stream's frames (`Demo::stream_tick`
before the stream's step from its first frame on, `Demo::stream_fade`
drawing `scFadeDef` over the stream's picture);
`the_opening_stream_flashes` checks both cases.

## The memory-card check

`PlayBootMemCard` (0x00406950) by `m_BootAct` (+0x200):

```
0  SetBootMemCard(); BootAct 1
1  r = BootMem_Control::Main_Control()
     r 1: BootAct 3, ErrFlg 1      no question: nothing drawn
     r 2: BootAct 2, ErrFlg 2      the question answered
     r 0: ErrFlg 0                 the question is up
2  step A_BACK[3]; when it ends, BootAct 3
3  delete A_BOOT; return 1
ErrFlg 0 or 2: step A_BACK[0..3], draw A_BACK[0..4] and A_BOOT
```

`SetBootMemCard` (0x004076f0) makes `sysLayer` active, sets `A_BACK` to
`ANM_xdt_ne01`, `ne00`, `ne09`, `ou00` and a new `A_BOOT` to `ANM_xdt_ou00`
under `SetMatrix_PosRotXYZScale(0, 0, (1, 0.5, 1))`, and steps `A_BACK[3]`
and `A_BOOT` 30 times: the screen-out, half-way and squashed to a band,
behind the question.

`BootMem_Control` (DataControl.cpp; a `Data_Control`, 0x8c bytes, then
+0x8c state and +0x90 result, -1 after `Init`):

```
Main_Control (0x00403c00), by state:
  0  saveSys->BootCheckReq()           ccSaveSys result 4, proccess 16
  1  mask |= 2|8|1; BootCheck(); state 2
  2  mask |= 2|8|1; BootCheck(); Data_Control::Main()
  3  return +0x90 == -1 ? 1 : 2
  return +0x90

BootCheck (0x00403d40):
  s = saveSys->result; s == 1: state += 1
  s & 0x8000: +0x90 = 0; mask &= ~(8|1)
      cancel push: m_dialog = 0; NextProccess(0)
      else ok push: NextProccess(m_dialog)
      YesNoDialogue(m_dialog, 1)
  InfoMessage(s, 1)
```

`ccSaveSys::BootCheckProccess` (0x00171a80), on the `ccThSaveSys` task,
breathes, then calls `ccMcard::CheckPort(0, 0)`, breathes, `CheckPort(1, 0)`,
breathes, and round again until the result is 1. A port whose `CheckPort`
is not 0 (no card), 1 (not a PS2 card) or 3 (no room) ends it with result 1;
an unformatted card (2) passes. Otherwise the result is 0x8029 when either
port was 3, else 0x8028, and it keeps checking, so a card put in ends the
question. `NextProccess(sel)` (0x00173f90) for 0x28 and 0x29: `sel` non-zero
sets result 1; 0 does nothing.

`Data_Control::Main` (0x00400bc0): with mask bit 2, up adds 1 to `m_dialog`
and down subtracts 1 through `CurRepeat` (0x00403710: push; release resets
the wait to 30; held, the repeat bits count to 30, then every 7), wrapping 2
to 0 and -1 to 1; se 6 unless mask 8. Unless mask 1, ok sets `m_SW` 1 (se 4
unless mask 4) and cancel -1 (se 7). Then mask 0. 1 is YES, 0 NO.

The questions: 0x8028, four lines, no card inserted, 685KB needed, start
anyway?; 0x8029, not enough space on the card, likewise.
`saveSysMsg` (0x00307040) is filled by `__sinit_sdmng.cpp`; entry 0x28 is
0x0034ce30, 0x29 is 0x0034ceb0. Drawn on `BootMem_Control`'s layer 130
(`SetFrame(0, 0, 512, 384, 256, 192, 1, 6/7)`), every `ccKanji`
`Init(3, 16)` in `ccSpriteColorTable[7]`:

| what | where |
| --- | --- |
| message line i (`InfoMessage`, up to 5, to the first empty) | (85, 127 + 17 i) |
| "YES" (`STR_YES`) | (85, 127 + 17 * 5) |
| "NO" (`STR_NO`) | (85, 127 + 17 * 6) |
| the chosen one | prefixed "#G" (demo.prg 0x0040eba8) |
| cursor, `TEX_xgtcur00` cell (0, 0) 13 x 10 | (73, the chosen row + 3), alpha `48 + tri(4 count mod 160)` |

`Info_X` 85, `Info_Y` = (int)(17 * 7.5) = 127 and `m_hi` 17 come from
`BootMem_Control::Init` (0x00403e60).

## ccOpening_Control

0x250 bytes (DWARF). The members the port keeps, with `Init`'s
(0x00405f50) values:

| offset | member | Init |
| --- | --- | --- |
| +0x004 | `m_Mask`: bit 0 skips `AllAnimate`, bit 1 `AllTransparency`, one frame | 0 |
| +0x008 | `m_NowVol` | `volumeNum` (1) |
| +0x00a .. +0x00e | `m_Pushflg`, `m_Pushcnt`, `m_RepeatCount` | 0, 0, 30 |
| +0x014 | `m_DemoCount`: idle frames on the menu | |
| +0x018 | `m_FlashCount` | |
| +0x01c | `m_Alpha` | 0.05 (0x3d4ccccd) |
| +0x020 .. +0x028 | `m_Transparency` (items), `_ico`, `_win` | 1, 1, 1 |
| +0x02c .. +0x0a4 | name arrays `m_ico_D`, `m_ico`, `m_menu_D`, `m_menu`, `m_menu_Sel`, `m_back[4]` (+0x90), `m_window` (+0xa4) | |
| +0x0b8, +0x0cc | `m_coord[5]`, `m_APP_coord[5]`: dummies from `GetSubstAdrsF` | |
| +0x0e0, +0x130 | `m_ico_Dvec[5]`, `m_app_Dvec[5]`: their positions | |
| +0x190 | `m_Rot_ICO` | 0 |
| +0x1b0 | `m_O_light`: a `ccOmniLight` | (-8500, 0, 6500), white |
| +0x1c0 | `m_Camera`: `ANM_xdtcam00` | |
| +0x1c4 .. +0x1d8 | `m_A_ICO[5]`, `m_A_ICO_D[5]`, `m_A_APP[5]`, `m_A_APP_D[5]`, `m_A_WIN`, `m_A_BACK[4]` | |
| +0x1dc | `m_A_BOOT` | 0 |
| +0x1e0 | `m_Cur`: `ccMask(10, 0)` with `TEX_xgtcur00`; nothing draws it | |
| +0x1e4 .. +0x204 | `m_LogoAct`, `m_MainAct`, `m_NutAct`, `m_DatAct`, `m_OptAct`, `m_BootAct`, `m_ErrFlg` | 0, 2, 0 ... |
| +0x20c | `m_ParoFLG` | 0 |
| +0x20d | `m_NutLock` | 1 |
| +0x210 | `m_Nut_CurNo`: the cursor | 1 (Load) |
| +0x214, +0x218, +0x224 | `m_Ne_SW`, `m_Dat_SW`, `m_OptSW` | |
| +0x220 | `m_LoadSW` | 0 |
| +0x228 | `m_Trans_Wait` | 0 |
| +0x22c | `m_Max_CurNO` | 3 |
| +0x234 | `m_StartMode` | 2 |

`SetVolCcs` (0x00405e80) picks `title1` .. `title4` by volume and calls
`SetStream` (0x00407670): `m_Camera->SetAnm("ANM_xdtcam00")`, one step,
`SetView(ccLayer::active->view, m_Camera->cam)`. The active layer is
`sysLayer` (0x00379b10; priority 0, `InitCCSys`'s view `SetFrame(0, 0,
512, 384, 256, 192, 1, 1)`), and the camera is never stepped again. Its
`F_Camera` is the desktop's: position (0, 0, 292609.3), fov 10.

## Main

`Main` (0x004043b0) runs the action `m_MainAct` names (jump table
0x0040eeb0), then, unless it returned, `AllAnimate` (unless mask 1),
`AllTransparency` (unless mask 2), `AllDraw`, `MOVEcount`, and clears the
mask.

| MainAct | does |
| ---: | --- |
| 2 | `SetNeutral` |
| 3 | `PlayNeutral`: the menu |
| 4, 5 | `SetNewGame`, `PlayNewGame` |
| 6, 7 | `SetDataLoad`, `PlayDataLoad` |
| 8, 9 | `SetOption(0)`, `PlayOption` |
| 10, 11 | `SetNextData(0)`, `PlayNextData` (volumes 2-4) |
| 12, 13 | `SetParodyGame`, `PlayParodyGame` |
| 14 | nothing (the fade to black) |
| 15 | `m_Nut_CurNo = 1; SetNeutral(); return m_StartMode` |

`ChangeMainAct(mode)` (0x00405d20), volume 1 (0x0040ef90): 0 -> 4, 1 -> 6,
2 -> 8, 3 and 4 -> 12, 5 -> 2, 6 -> 15; volumes 2-4 have 3 -> 10. It sets
`m_FlashFlg`, which nothing reads.

`MOVEcount` (0x00404710): in MainAct 3 and 14 `m_DemoCount` counts; at
2380 MainAct 14 and `EntryFlash3(20, 1, 1, 0x80000000)` (black in over 21
frames); at 2400 MainAct 15 and `m_StartMode` 5. `SetNeutral` and every
cursor move reset the count.

## The menu

`SetNeutral` (0x00407950), volume 1, sets the names:

| item | `m_menu` | `m_menu_Sel` | `m_ico` | `m_ico_D` | dummy |
| ---: | --- | --- | --- | --- | --- |
| 0 New Game | `ANM_xdt_ne02` | `ch00` | `ic00` | `dn00` | `OBJ_dam_ico_00_` |
| 1 Load | `ne03` | `ch01` | `ic01` | `dn01` | `OBJ_dam_ico_10_` |
| 2 Option | `ne04` | `ch02` | `ic02` | `dn02` | `OBJ_dam_ico_20_` |
| 3 Parody | `ne05` | `ch03` | `ic03` | `dn03` | `OBJ_dam_ico_30_` |

and `m_back` `ANM_xdt_ne01` (the logo), `ne00` (the backdrop), `ne09` (the
digits, hexagons and copyright), `ou00` (the screen-out). Then: MainAct 3;
the localtp of `A_ICO[0]`, `A_ICO[2]` = `m_Transparency_ico` and of
`A_APP[0]`, `A_APP[2]` = `m_Transparency` (sic, not 1); if `m_NutLock`,
`A_BACK[1]`, `A_BACK[2]` and each `A_ICO[i]` set and `m_NutLock` 0; `A_BACK[0]` and
`A_BACK[3]` set; each item's `A_APP[i]` at the origin and set to `m_menu_Sel`
for the cursor's item, else `m_menu`; each `A_ICO_D[i]` set and stepped once,
its dummy found, `A_ICO[i]` moved to it; then the cursor's item jumps to
frame 30 (`_AnimateFrame(7680)`; cursor 3 and 4 name `A_APP[4]`, which
volume 1 never sets).

`MoveCurNut` (0x004070c0), each of up (-1) and down (+1): a push moves
(and sets `m_Pushflg`); else a release sets `m_RepeatCount` 30 (and clears
`m_Pushflg`); else while `ccPad.repeat` has it, `m_Pushcnt` counts and at
`m_RepeatCount` moves, `m_RepeatCount` 9, `m_Pushcnt` 2. `m_Pushcnt` is
shared by both directions and never reset on a push. A move resets
`m_DemoCount`. The cursor is clamped to 0 .. `m_Max_CurNO - 1` (a clamp
cancels the move); a move calls `SwitchCur` and plays se 2.

`SwitchCur(cur)` (0x004072a0), volume 1: 0: `A_APP[0]` selected, `[1]` at
rest; 1: `[0]` rest, `[1]` selected, `[2]` rest; 2: `[1]` rest, `[2]`
selected, `[3]` rest with `m_ParoFLG`; 3, 4: `[2]` rest, `[3]` selected.

`PlayNeutral` (0x00406b50): `m_NutAct` 0: `MoveCurNut`; ok push: se 4,
`m_NutAct` 1. 1: `ChangeMainAct(m_Nut_CurNo)`, `m_NutAct` 0.

### Volumes 2-4

Fifteen of the control's functions branch on `m_NowVol`. Infection's
DEMO.PRG holds every volume's branches and names, and Infection's archive
holds `title2`-`title4`. Mutation's copies of these functions are
Infection's byte for byte; Outbreak's and Quarantine's are recompiled
copies (`tools/voldiff.py ported`). So the later titles run the same
code with `m_NowVol` 2-4, from `title<n>` (`SetVolCcs`, and the cursor's
texture in `Init`).

The menu has five items:

| item | `m_menu` | `m_menu_Sel` | `m_ico` | `m_ico_D` | dummy |
| ---: | --- | --- | --- | --- | --- |
| 0 New Game | `ne02` | `ch00` | `ic00` | `dn00` | `OBJ_dam_ico_00_` |
| 1 Load | `ne03` | `ch01` | `ic01` | `dn01` | `OBJ_dam_ico_10_` |
| 2 Option | `ne04` | `ch02` | `ic02` | `dn02` | `OBJ_dam_ico_20_` |
| 3 the previous volume's save ("CONVERT") | `ne06` | `ch04` | `ic04` | `dn04` | `OBJ_dam_ico_40_` |
| 4 Parody | `ne15` | `ch13` | `ic03` | `dn13` | `OBJ_dam_ico_30_` |

The items' dummies swap 30 and 40 the same way (`OBJ_dam_men_40_` for
item 3).

- **The backdrop.** `m_back[0..1]` are `ne11`/`ne10`, `ne21`/`ne20` and
  `ne31`/`ne30` by volume.
- **The panels.** They carry a fifth dummy each: opening `do03`/`do12`
  and `do07`/`do16`; closing `dc03`/`dc12` and `dc07`/`dc16`. The
  backdrop behind them is `op10`, `op20`, `op30` (opening) and `cl10`,
  `cl20`, `cl30` (closing).
- **The panels' menus:**
  - Load: `de05 de01 de12 de14 de06`;
  - Option: `de05 de11 de02 de14 de06`;
  - the previous volume's save: `de05 de11 de12 de04 de06`, with Load's
    window.
- **The item count** is four (five with `m_ParoFLG`). Infection's
  `ccThDemo` never raises `m_Max_CurNO` from `Init`'s 3. Mutation's calls
  a new function after `Init` (MUT 0x00419630, from 0x00413a3c), which
  sets it to 4 on volumes 2-4. Nothing sets `m_ParoFLG`, so Parody stays
  hidden there too.
- **`SwitchCur`.** Each item's neighbours rest and it is selected. Item 4
  rests after item 3 only with Parody.
- **`SetNeutral`'s jump** goes to the cursor's own item.
- **`ChangeMainAct`'s table** (0x0040ef70): 3 goes to 10
  (`SetNextData`), 4 to 12 (Parody).
- **`SetParodyGame`** sets `A_APP[4]` to `de13`.
- **`AllAnimate`, `AllTransparency` and `AllDraw`** take items 3 and 4
  as the count has them. `AllTransparency` leaves item 3 alone while it
  is the open panel.
- **`SetNextData(mode)`** (0x00409470) is `SetDataLoad` with the mode as
  an argument. It clears `m_OptSW`, as `SetOption` does.
- **`PlayNextData`** (0x00406f40) runs the `NextDataLoad_Control`
  (+0x240: a `Data_Control` with `m_PrevFlg` 1, set after its `Init`).
  A save read marks `m_LoadSW` at once. The flash's 20 frames end with
  `m_NextSW` 0, so the window's closing animations run before it leaves
  with `m_StartMode` 4 (`ccStartEventConvert`, `ConvGame`).
- **`PlayOpeningStream`** plays streams 21/22, 50/51 or 75/76 (plain /
  Parody), counted to frame 450 or 490.

`test_demo_rs.py` runs its set, switch, cursor, count, transparency,
animation, draw, `Main` (with actions 10 and 11), `Play*` (with
`PlayNextData`) and boot checks once for each volume. It sets the
control's `m_NowVol` in eemu and gives the probe `vol N`, which loads
`title<N>`. All pass on the four volumes.

## New Game and Parody

`SetNewGame` (0x00408240): MainAct 5, `A_APP[0]` = `ANM_xdt_de00`.
`AllAnimate` in MainAct 4, 5 and 13 steps `A_BACK[3]` (`ANM_xdt_ou00`, 61
frames, play-once) into `m_Ne_SW`; `AllDraw` draws it. `PlayNewGame`
(0x00406bf0): once `m_Ne_SW`, MainAct 15, `m_NewGameSW` 0, `m_StartMode` 2.

`SetParodyGame` (0x00409d40): MainAct 13, `A_APP[3]` = `ANM_xdt_de03`.
`PlayParodyGame` (0x00406c20): as New Game with `m_NewGameSW` 4, and every
frame `saveData->parodyFlag = 1`. The item is unreachable in Infection:
`m_ParoFLG` is cleared by `Init` and written nowhere else in DEMO.PRG
(the same holds for Mutation's DEMO.PRG, MUT 0x00418f80), and `m_Max_CurNO`
3 keeps the cursor off item 3 even with it set.

## Load and Option

`SetDataLoad` (0x004082c0) by `m_DatAct` and `SetOption(mode)` (0x00408ba0)
set `m_Trans_Wait` 0 (and `m_OptSW` 0), the names, then: `A_WIN` to the
window; `A_APP[i]` to the panel's `m_menu`; `A_BACK[0]`, `[3]`; each
`A_ICO_D[i]` set, stepped, its dummy `OBJ_dam_ico_n0_` found and `A_ICO[i]`
moved there; each `A_APP_D[i]` set, stepped, `OBJ_dam_men_n0_` found and
`A_APP[i]` moved there. Any other `m_DatAct` or mode keeps the names.

| | `m_ico_D` | `m_menu_D` | `m_window` | `m_back[0]`, `[3]` |
| --- | --- | --- | --- | --- |
| open (DatAct 0, MainAct 7; mode 0, MainAct 9) | `do08 do00 do01 do02` | `do09 do04 do05 do06` | `op01` / `op02` | `op00`, `op03` |
| close (DatAct 1; mode 1) | `dc08 dc00 dc01 dc02` | `dc09 dc04 dc05 dc06` | `cl01` / `cl02` | `cl00`, `cl03` |

`m_menu` is `de05 de01 de12 de06` for Load and `de05 de11 de02 de06` for
Option.

`PlayDataLoad` (0x00406c60) by `m_DatAct`:

```
0  once m_Dat_SW (the window has opened): r = DataLoad_Control::Main_Control()
     r -1 (cancelled): DatAct 1
     r 1 (loaded): DatAct 2; m_FlashCount 0; EntryFlash3(20, 1, 1, 0x80000000)
1  SetDataLoad() (closing); DatAct 3; m_Dat_SW 0; m_LoadSW 0
2  when m_FlashCount++ == 20: DatAct 3; m_Dat_SW 1; m_LoadSW 1
3  once m_Dat_SW == 1: DatAct 0; EndDataLoad(); ChangeMainAct(LoadSW ? 6 : 5)
```

`EndDataLoad` (0x00407640): both transparencies 1, mask bit 2, `m_StartMode`
3. So a load leaves through MainAct 15 with 3; a cancel returns to the menu
once the close (`cl03`, 61 frames) has played.

### The load screen

`DataLoad_Control` (DataControl.cpp; a `Data_Control`, 0x8c bytes, then
`m_DataAct`) is built once by `ccOpening_Control::Init`; its constructor
calls `Data_Control::Init` (0x00402e40), and then `Init` calls
`saveSys->StartReq(1)` (operate 1, the title's load). The screen talks to the
one `ccSaveSys` the game builds at boot, the same one the boot check and
the desktop's Data screen use. The `ccThSaveSys` task (priority 20) runs
`MainProccess` a frame at a time. Neither ever resets `m_SelInitFlg`,
`m_CurNo` and the like between visits.

```
Init: m_CurNo = saveSys->port, m_DataNO = fileNum, Max 1, Min 0, repeat 30,
      m_SlotProc 0, m_SelInitFlg 1; SlotSelectReq()          result 13, proccess 2
Main_Control (0x004037f0): Data_Control::Main(), then by m_SlotProc
  0  Slot_Select()        return -1 if m_SW is -1 (cancel)
  1  SlotStateData()
  2  WriteDataList(); Data_Select()
  3  WriteDataList(); LoadData()   return 1 once m_LoadFlg
  return 0
```

Each step reads `saveSys->result` (s) once at its start. The branches they
share:

| s | what the step does |
| --- | --- |
| 0 | back to the card slots: SlotProc 0, dialog 0, CurNo = port, Max 1, `SlotSelectReq` |
| 2 (read the index again) | SlotProc 1, dialog 0, `LoadInfoReq(SlotNO)` (SlotStateData also sets CurNo 0) |
| & 0x3000 (acknowledge) | mask 2; an ok (or cancel, but only ok in Data_Select) push: `NextProccess(0)` |
| & 0x8000 (yes / no) | mask 2; cancel: dialog 0, `NextProccess(0)`; else ok: `NextProccess(dialog)`; `YesNoDialogue(dialog, 0)` |
| any | `InfoMessage(s, 0)` |

The pushes are read from `ccSys` +0x2d0 against the save's `assignPADok` and
`assignPADcancel`, not through `CurRepeat`. The steps:

- `Slot_Select` (0x00400e20). The first time it calls `SlotSelectReq`. It
  draws "MEMORY CARD" and "slot 1" / "slot 2", the chosen pair "#G", and the
  big cursor. On ok (`m_SW` 1): SlotProc 1, SlotNO = CurNo, CurNo 0, Max 11,
  `LoadInfoReq(SlotNO)`.
- `SlotStateData` (0x00401170). On s 1: SlotProc 2, dialog 0,
  `LoadSelectReq` (result 16, proccess 5). While an acknowledged message
  is up, message 23 plays se 74 if `m_tempPN` is set, then clears it; the
  acknowledgement sets it.
- `Data_Select` (0x00401390). On s 1 it re-reads the index. Messages 5 and
  6 (the card taken out) acknowledged: SlotProc 1. On s 1 or 16: DataNO =
  CurNo and the record is `saveSys->info[DataNO]`. Cancel goes back to the
  card slots. Ok freezes the cursor (mask 0x10); if the record is used:
  SlotProc 3 and `LoadDataReq(DataNO)` (result 4, proccess 6). While s is
  not 1 and SlotProc is not 0: `SetLoadPar`.
- `LoadData` (0x00401750) keeps the cursor frozen. On s 1: SlotProc 2
  (dialog non-zero: CurNo 0, dialog 0). The acknowledgement branch plays
  the jingle as `SlotStateData` does. Acknowledging message 23 ("Data
  loaded.") sets `m_LoadFlg`. With the question up, and on s 4, it draws
  `SetLoadPar`.

`ccSaveSys` (sdmng.cpp, the desktop's port in
`piney_desktop::savesys`) for the load:

```
LoadSelectReq (0x00171990)   proccess 5, result 16          (5 waits)
LoadDataReq(fn) (0x001719b0) proccess 6, result 4, fileNum fn; operate 2 -> 1
MainProccess 6 (0x001723dc)  proccess 1, result 0x8011       "Load this data?"
NextProccess 0x11: yes       proccess 8, result 4;  no: result 1
MainProccess 8 (0x0017240c)  proccess 1; game->gameCntStop 1 (then 0);
    result 0x10013 / 0x10014 while DataRead(port, 0, fileNum, buf, 0x8530) runs
    DataRead fails (5): 0x2018
    16-bit sum of the 0x8530 bytes != info[fileNum].sum: 0x203f
    else saveData <- buf, member by member: every byte but 0x221e-0x2220,
    0x7486-0x7488, 0x852c-0x8530 (padding; they keep the old save's); 0x2017
NextProccess 0x17: result 1, proccess kept; 0x18, 0x3f: result 0, proccess 1
```

MainProccess's other steps are the desktop's (`docs/engine/desktop.md`):
- with operate 1, a card without the save directory, or with no room,
  gives 0x103b "There is no .hack//INFECTION saved data";
- an unformatted card gives 0x103a;
- a card taken out gives 0x2005 or 0x2006.
Proccess 4, 7 and 9 load the previous volume's data (`m_PrevFlg`,
`NextDataLoad_Control`, the `Prev` requests), which Infection never asks
for.

What it draws, on its own layer 129 (`ccLayer::Init(129, NULL)`, framed
`SetFrame(0, 0, 512, 384, 256, 192, 1, 6/7)`). `m_hi` is 17; every
`ccKanji` is `Init(3, 16)` in `ccSpriteColorTable[7]`:

| what | where |
| --- | --- |
| "MEMORY CARD" (`STR_MEMORYCARD`), slot i | (75 - 7, 119 + 51 i - 17) |
| "slot 1" / "slot 2" (`STR_SLOT1`, `STR_SLOT2`), the chosen pair "#G" | (75, 119 + 51 i) |
| the slots' cursor | (75 - 12, 119 + 51 CurNo + 3) |
| "Data" (`STR_DATA`) + `dec2sjis(i + 1, 2, 2)`, i < 12, the cursor's "#G" (`WriteDataList` 0x00402360, not while messages 5 and 6 are up) | (75, 102 + 17 i) |
| the list's cursor | (75 - 14, 102 + 17 CurNo + 3) |
| `saveSysMsg[s & 0xfff]` line i (`InfoMessage`, up to 4 and the first empty) | (165, 229 + 17 i) |
| YES / NO (`YesNoDialogue(dialog, 0)`), the chosen "#G" | (165, 229 + 17 * 3) / (165, 229 + 17 * 4) |
| the question's cursor | (165 - 12, the chosen row + 3) |
| the button, under a message to acknowledge (`DispButton` 0x00401990) | ((int)(165 + 7.5 * 17), 229 + 17 (lines + 1)) |
| the panel (`SetLoadPar` 0x00402570), 4 lines | (165, 102 + 17 k) |

The cursors and the button are `m_Cur`, a `ccMask` on `TEX_xgtcur00`:
- `TimeAlphaCurDraw(x, y, mode)` (0x00401c40): mode 1 is cell (0, 0) 13 x
  10, the question's; mode 0 is cell (208, 0) 14 x 11, the lists'. Alpha
  is `48 + tri(4 count mod 160)`, held at 128 in mode 0 while
  `saveSys->result` has 0x8000 or 0x3000.
- `DispButton`: cell (448, 0) 16 x 16, scaled about its centre by
  `0.9 (0.9 + (128 - (16 + 112 tri(count mod 48) / 24)) / 600)`.

`InfoMessage` quiets `Main`'s sounds (mask 8|4) for messages 19-22, 32, 33,
36 and 37, and quiets the moves (mask 8) under a message to acknowledge.

`SetLoadPar(info, no)` shows the record's panel. Its first line is "Data"
and the number, then:
- a record whose clear flag is at least `volumeNum`: two spaces, "Vol."
  (demo.prg 0x0040ebd8), the clear flag and `STR_CLEAR`, the whole panel in
  `ccSpriteColorTable[6]` (yellow);
- a parody record: two full-width spaces and `STR_PARODY`, the panel in
  `[18]`;
- otherwise nothing more, in `[7]`.
A used record then shows that line, "Lv." (`STR_LV`) with the level
(`dec2sjis(level, 2, 2)`), the name and "Time" (`STR_ALLTIME`) with
`H:MM:SS` of the play time: 216000 frames an hour, 3600 a minute, 60 a
second, with the hours as `dec2sjis(h, 3, 1)`. An empty record shows only
`STR_NODETA` on the second line, in `[7]`.

`m_tempPN` is never initialised: the object is `new`ed and `ccMalloc` does
not clear. So the jingle for "Data loaded." plays only after the player
has acknowledged some other message on this visit, or if the heap held a
non-zero word there. The port starts it at 0.

After `Main_Control` returns 1, `ccThDemo` calls
`ccSaveData::LoadGame()` (0x00175110) on the loaded save:
- for each of the 18 characters (`spcParam[i]` at +0x7488 + 0xdc i), `name`
  points at `plName` (the save itself) for 0 and `spcNameList[i - 1]`
  otherwise; `ccsname` points at `ccsNameList[i]`, into which the
  `charTbl` row's name is copied (up to 32 bytes and its NUL; an empty
  name when the row has none); `velocity` (+0xd4) comes from the row's
  +0x54;
- `SetDefaultWord()`;
- `setCameraCtrlType(camType)`, `ccPad::actuaterSw = vibration`,
  `ccSys->SetDisplayOffset(screenX, screenY)` and `SetSoundEnv()`, outside
  the save.
Then `ccSqFade(0, 0, 8, 3)`, `Breath(2)` and `ChangeRequest(3, 7)`, as for
New Game. No event call: the event manager keeps what `ccThMother`
started at boot.

`PlayOption` (0x00406de0) by `m_OptAct`:

```
0  once m_OptSW (window open): OptAct 1
1  dtMenu->openReqNum = 1; OptAct 2         the system menu's settings
2  CheckMenuType() == 1 and a cancel push: OptAct 3
3  CheckMenuType() == -1: OptAct 4; SetOption(1)
4  once m_OptSW (closed): OptAct 0; ChangeMainAct(5); transparencies 1; mask 2
```

### The system menu

`ccSetupDemo` starts `ccThDtMenu` right after `ccThDemo` (both priority 33),
so the menu runs after the title each frame. `ccGame.enableReset` is 0, so
START opens nothing. `ccThDemo` sets `dtMenu->dispFlag` (+0x18) to 0 while
the logos play and to 1 from the title on; the task calls `Disp`, which
also steps the window's alpha, only while it is 1.

`PlayOption`'s `openReqNum = 1` opens list 1, the title's OPTION list:
Controller, Vibrate, Adjust Screen, Sound, Voiceover, Movie Text, each
opening the same option menu as on the desktop. With `ccGame.status` 1:
- nothing sleeps and nothing freezes: the title's own frame goes on under
  the menu;
- no dim and no sound 16;
- decide and back sound 4 and 7;
- the list has no window of its own. It sits on the title's panel
  (`op02`); `AllTransparency` fades that panel out while an option menu
  other than the list is up. Its rows are at (193, 164 + 24 i), and the
  chosen one is `ccSpriteColorTable[20]` with no select cursor, as in
  every option menu's page. See [the desktop's menu task](desktop.md#the-menu-task).
Back out of the list, cancel both closes the menu and moves `PlayOption`
on. The title sees `CheckMenuType() == 1` with a cancel push (OptAct 3),
then -1 once the window's alpha is down (OptAct 4).

The options write the save the title hands on (`vibration`, `voice`,
`strWinMode`, `screenX`/`screenY`, the volumes and output, `camType`) and
ask for the display offset, the sound settings, the vibration and the
camera scheme, as on the desktop.

`AllAnimate` in MainAct 7 and 9 steps every `A_ICO_D[i]` and `A_APP_D[i]`
and keeps `A_APP[i]` on its dummy; at DatAct / OptAct 0 it steps `A_WIN`
into `m_Dat_SW` / `m_OptSW` and steps `A_BACK[3]`; at DatAct 3 / OptAct 4
it steps `A_WIN` and `A_BACK[3]` into the switch.

`AllTransparency` (0x00405420):

```
MainAct 7:  DatAct 0: Transp -= Alpha (to 0); Transp_ico = Transp
            else: if Trans_Wait++ >= 30 and DatAct 3: both += Alpha (to 1)
            A_ICO[0], [2] = Transp_ico; A_APP[0], [2] = Transp
MainAct 9:  OptAct 0: as above
            OptAct 2: Transp_win += 2 Alpha (to 1) while CheckMenuType() == 1, else -= (to 0)
            else: if Trans_Wait++ >= 30 and OptAct 4: both += Alpha; Transp_win = 1
            A_ICO[0], [1], A_APP[0], [1] as above; A_WIN = Transp_win
m_ParoFLG:  A_ICO[3] = Transp_ico; A_APP[3] = Transp
```

The item chosen keeps its localtp: the others fade out while the panel is
up (21 frames at 0.05 a frame, EE single precision).

## Every frame

`AllAnimate` (0x004047e0): steps `A_BACK[0..3]`, `A_APP[0..3]`,
`A_ICO[0..3]` (and `[3]` with `m_ParoFLG`), the action's extra (above), then
`m_Rot_ICO.y -= 0.03` (0x3cf5c28f), wrapped by 2 pi above pi and below -pi,
and `A_ICO[i]->SetMatrix_PosRotXYZ(the dummy's position, m_Rot_ICO)`: the
icons turn about y at their dummies.

`AllDraw` (0x004059d0): `ccLayer::active = sysLayer`; draws `A_BACK[0..3]`,
`A_APP[0..3]`, `A_ICO[0..3]` (and `[3]`s with `m_ParoFLG`), then `A_BACK[3]`
in MainAct 4-13 and 15, and `A_WIN` in 7, 9 and 11.

## The fader

`ccScFade` (fade.cpp; `scFadeDef`, drawn on the font layer 240) has four
elements (`status`, `cnt`, `tcnt`, `tcnt1`, `tcnt2`, `col0`, `col1`).
`SendPacket` (0x0015fb80) draws each element with status bit 1 as a
flat quad over the screen, colour per channel
`c0 + trunc((c1 - c0) * cnt / tcnt)` (64-bit), then `cnt += 1`; past `tcnt`:
bit 2 ends it; else bit 4: bits `|2 &~4`, `cnt` 0, `tcnt = tcnt1`,
`c0 = c1`, `c1` alpha 0; else bit 8: bits `|4 &~8`, `cnt` 0,
`tcnt = tcnt2`, `c0 = c1`; else it holds (`cnt = tcnt`). `EntryFlash(t, c)`
(0x00160240) is status 3 from `c` to alpha 0; `EntryFlash3(t0, t1, t2, c)`
(0x00160360) status 9 from alpha 0 to `c` (in over t0, hold t2, out t1);
`EntryFade(t, c0, c1)` (0x00160400) status 1. Each takes the first free
element.

## The 3D draw

As [the desktop's](desktop.md#the-3d-draw), on `sysLayer` (priority 0,
under the dialogs' 130 and the fader's 240). Of `title1`'s 97 models the
ten icon parts (`MDL_xdt_ico_*`) are lit (mtype bit 0); the rest are
unlit. `Init` makes a `ccOmniLight` (type 4, priority 1,
`ccSetColor(0xffffff, 1.0)`: white, intensity 1, no fall-off) at
(-8500, 0, 6500) and adds it to `cc3d`'s light group; `cc3d` is
`active__9ccDrawEnv` whenever the title draws (`InitCCSys` sets it,
`RequestStrPlay` puts it back after the opening stream). For a lit model
`ccModel::Draw` calls `ccDrawEnv::SetLightMatrix` (0x00105900) with the
object's `lwMatrix` translation; `ccOmniLight::CheckRange` (0x00139830)
gives `normalize(pos - light)` and the colour; the light sorts into slot 2
(slots 0 and 1 keep unit rows and black); `sceVu0NormalLightMatrix`
negates, normalises and transposes; `ccSetMatrixPacket` (0x0013e350)
multiplies it by the `lwMatrix` (VU memory 16-19, one light per lane) and
copies the colours and ambient (12-15); VU1's `mc_SetMatrix` renormalises
each light's model-space direction and doubles the light colours, not the
ambient. `mc_DrawTriL` then writes per vertex
`min(128, sum 2 c_i max(0, l_i . N) + 128 ambient)` with `N` the stored s8
normal, whose length is 64: for the title's white light,
`min(128, 128 (0.35 + max(0, l . n)))` with `n` the unit normal. Nothing
else goes in: no vertex colour (a lit model's colours are dropped at
load), alpha `128 t`; the GS modulates the texture by it. A face turned
from the light is 0.35 of its texture. No title animation carries an
ambient (`ccAnmChunk::ambient` stays 0x80000000, so `ccAnm::Draw` leaves
the env's) or a light, and `title1` has no light chunk.

Checked (eemu): the port's `Lights` equal VU1's registers after
`mc_SetMatrix`, and every icon vertex's RGBA from `mc_DrawTriL` equals
`min(128, 128 (ambient + sum c_i/2 max(0, dir_i . n)))` of the port's
`Lights` (`c_i` the doubled colours `Lights` carries).

No title animation has material (texture offset) or `F_Obj` records; all
are controller tracks. `ANM_xdt_ne09` (601 frames), the icons' `ic00..03`
(121) and the selected items' `ch00..03` (61) loop.

## The seam to the desktop

| | the save | request |
| --- | --- | --- |
| boot (`ccThMother`) | `saveData->Init(1)` | `ChangeRequest(2, 7)` |
| `ccSetupDemo` | `saveData->NewGame(1)` | |
| New Game | `saveData->NewGame(0)` | `ChangeRequest(3, 7)` |
| Load | `saveData->LoadGame()` (the slot `Data_Control` read) | `ChangeRequest(3, 7)` |

### ccSaveData::NewGame

`ccSaveData::NewGame(sw)` (0x00174d70), in the port
`piney_demo::newgame::new_game`:

```
for i in 0..18:                                    charTbl: demo.prg 0x0040dc80, 92 bytes a row
  spcParam[i] (+0x7488 + 0xdc i) <- row i:
    +0x00..+0x0c  name, ccsname, type          +0x0c..+0x12  id, level, exp   (not +0x12)
    +0x14..+0x24  gold, height, width, msg     +0x24..+0x28  maxHP, maxSP
    +0x28..+0x48  elm
    row +0x48..+0x54 -> +0xc8 equipment; +0x54 -> +0xd4 velocity;
    +0x58 -> +0xd8 job; +0x5a -> +0xda friendship
  name: to plName (+0x00) for i 0, else spcNameList[i - 1] (0x00387840,
        char[17][24]); up to 20 bytes, stopping after a NUL; a null name
        writes one NUL. spcParam[i].name = that buffer's address.
  ccsname: to ccsNameList[i] (0x00387600, char[18][32]), up to 32 bytes;
        spcParam[i].ccsname = its address.
  sw 0, i 0, parodyFlag (+0x842b): spcParam[0].level = 20
ccResetPlayTime: ccGame's four play-time words, playTime (+0x8400) = 0
SetDefaultWord: wordList (+0x523c) ORed, by volume; volume 1:
                +0x5248 |= 0xfff00000, +0x524c |= 0x001dfffb
InitTradeItem:  spcTradeList[17][16] (+0xe3c) <- spcDefTradeList (0x00345f10),
                npcTradeList[48][16] (+0x127c) <- npcDefTradeList (0x00346790):
                id, category, num of each 8-byte entry;
                tpcTradeListSW[24][3] (+0x1e7c): [j][k] = 1 when
                tpcTradeList[j][0][0].category (0x00347f90 + 48 j + 2) is not
                negative, else 0 - the loop never indexes by k
setCameraCtrlType(camType), ccPad::actuaterSw = vibration,
ccSys->SetDisplayOffset(screenX, screenY), SetSoundEnv(): outside the save
```

So the save holds three kinds of EE address: each character's `name`
(character 0's is the save's own address) and `ccsname`, and `msg` copied
from `charTbl` (gcmn.prg addresses: `spcMsg1` 0x006377f0 on). The port takes the
save's address as an argument. Nothing `NewGame` writes is one of the
desktop's members (mail, news, wallpaper, music, +0x2236 .. +0x28e4). The
parody flag is `saveData->parodyFlag` (+0x842b), set only by
`PlayParodyGame`. After `NewGame(0)` Kite has maxHP 63 and maxSP 13.

`piney_demo::Demo` applies `NewGame(1)` when it is built (as
`ccSetupDemo` does before the task starts) and `NewGame(0)` in the step
whose `Main` returns 2, so `Demo::save` is the save the desktop starts on.

## The music

One bank, one sequence, one port: the title plays sequence 0 of
`sqDataTitle[0]` on sequencer 0 into synthesizer port 1, and everything
else it does to it is a volume. The rules are [the sound engine's](sound.md)
(Music, Fades, ccSceneFade, Volumes); this is what the title asks of them.

Marks: [code] read from the code at the address given; [eemu] the game's
own functions run in `tools/eemu.py` through `tools/sound_ee.py`'s harness
(`ccSndCmd` / `ccSndCmd3` caught); [data] read off the disc
(`tools/sound.py`, `tools/midi.py`, a byte scan); [inferred] not shown
directly. SNDBASE addresses are module offsets in `MODULES/SNDBASE.IRX`.

### The bank: ccSndSQLoad(7)

| | address | value |
| --- | --- | --- |
| `sqDataTitle[0]` (`SQ_LOAD`) | 0x00307e30 | ofs 0x131000, hdSize 0x560, sqSize1 0x780, sqSize2 0, sqSize3 0, bdSize 0x8df30 |
| `sqVolTblTitle[0]` (`SQTBL[3]`) | 0x0030a290 | {midiPort 0, hdPort 1, vol 0x100}, {1, 2, 0}, {2, 3, 0} |

Case 7 of the row jump table (0x0034e0e0 -> 0x001825f0) and of the volume
one (0x0034e0c0 -> 0x00182adc) [code]. It is bank 1 of
[SNDDATA.BIN](../formats/snddata.md): a 1,376-byte `.hd` and one 1,920-byte
`.sq` at 0x131000, the 581,424-byte `.bd` at 0x132000; 8 VAGs (0-2 looped,
3-7 one-shot), 11 samples, 7 programs; no jukebox row plays it [data].

`ccSndSQLoad(7)` (0x001821d0), in order [code]:

1. `initBeforeLoad` (0x00183080): `fade[0]`, `fade[1]` `fadeSw` 0;
   `bgmStopFlag` (+0x61) 1; `soundOffFlag` (+0x134) 1; `tobjFlag` (+0x133)
   and `sqReadyFlag` (+0x137) 0.
2. Breathe a frame at a time until `soundOffFlag` is 0, i.e. the sound
   task has sent `ccSndCmd(0x140, 0)`: at least one frame.
3. `ccSndCmd3(0x200, 7)`; `stageParam` (+0x105) 0; case 7: `battleField`
   (+0x5f) 0, `playtype` (+0x104) 0; `ccSndCmd3(0x300, 0)`.
4. `fadeSw` of fades 1 and 0 cleared again; `sceSifFreeIopHeap(iopSQAddrOld)`
   (+0x10) if set; `sceSifAllocIopHeap(0x560 + 0x780 = 0xce0)` (-1 back,
   nothing loaded, if the address is negative).
5. `dataLoad` (0x00387d80, `CCSND_LOAD`) = {iopAddr the block, ofs 0x131000,
   hdSize 0x560, bdSize 0x8df30, sqSize 0x780, spuAddr `commseTbl[2]` +
   0x5020 = 0x12f580, `\DATA\SNDDATA.BIN;1`}; `ccSndCmd(0x9310, &dataLoad)`;
   `loadFlag` (+0x15) 1; breathe until `sbuff[0]` (0x00387bc0, the reply)
   is 0x8df30; `loadFlag` 0.
6. `sndAddr` (0x00378a00) = {hdAddr iopAddr, spuAddr 0x12f580};
   `ccSndCmd(0x9051, &sndAddr)`, `ccSndCmd(0xa1, 0x2010)`,
   `ccSndCmd(0x40, iopAddr + 0x560)`; `sqNum` (+0x64) 1. No 0x9052, 0x9053.
7. `sqtbl[0..2]` (+0x70) = `sqVolTblTitle[0]`; `hdSynPortVol[1..3]`
   (0x00378a20) = 0x100, 0, 0; `ccPortVolSet(0..3)`: `ccSndCmd(0xb0, seVol)`,
   `(0xb1, 0x100 * bgmVol >> 8)`, `(0xb2, 0)`, `(0xb3, 0)`, each sent value
   left in `hdSynPortVol`.
8. `sqStatus[0..2]` (0x00387dc0) -1; `iopSQAddrOld` = iopAddr;
   `initAfterLoad` (`loopID[8]` -1); `bgmChangeFlag` (+0x60) and
   `bgmStopFlag` 0; `sqLoadParam` (+0xf0) 7; returns 0.

Nothing plays. With the volumes at 256 what goes out is: load 0x131000;
sequence 0 at +0x560; ports 0, 1, 2, 3 at 256, 256, 0, 0 [eemu].

Around it, `ccSetupDemo` (0x00168160) [code]: `ccAllSoundOff` (0x0016816c;
fades 0 and 1 off, `soundOffFlag` 1, `ccEvVoiceStop`; all skipped when
`stageParam` is 2), `ccBreathThread(2)` (the sound task sends 0x140),
`ccSndSQLoad(7)` (a second 0x140, then the load), ..., `NewGame(1)`, whose
`SetSoundEnv` calls `ccSetMainVol`, `ccSetBgmVol`, `ccSetSeVol` and
`ccSetOutputMode` with the save's values, and last `ccSndBgmCtrl()`
(0x001682f8), whose case 7 (jump table 0x0034d350) only sets
`sqReadyFlag`. The save's volumes are `ccSnd`'s: the `ccSaveData`
constructor's `Init(0)` copies them (0x00174618-0x00174664; `Init(1)` at
boot skips the copy) [code], so those setters change nothing unless the
player has changed an option [inferred].

### The calls on the EE

`sqStatus[sq]`: -1 loaded, 0 stopped, 1 play sent. `sqStatusGet`
(0x00182fc0), which would ask the IOP (0x8090) and move 1 to 3 (playing)
and 3 to 0 (ended), runs only while `sqStFlag` (0x00378a30) is set, and
nothing sets it: `ccSoundRpc` is its only user and writes 0 [code; xrefs
with every overlay and `--scan`]. So the EE never learns that a sequence
ended by itself: after the title's music ends, `sqStatus[0]` is still 1.

`ccSqPlay(sq)` (0x001798f0) [code]:

```
if sq >= sqNum or sqStatus[sq] is 1 or 3: return
t = sqtbl[sq]
hdSynPortVol[t.hdPort] = t.vol
ccPortVolSet(t.hdPort, t.vol)          ccSndCmd(0xb0 | hdPort, vol * bgmVol >> 8)
ccSndCmd3(0x110 + t.midiPort, 0)        locate to tick 0 and play
sqStatus[sq] = 1
```

For the title: `ccSndCmd(0xb1, 0x100 * bgmVol >> 8)`, `ccSndCmd3(0x110, 0)`,
both sent at once from the calling task.

`ccSqStop(sq)` (0x00179aa0) [code]:

```
if sq >= sqNum or sqStatus[sq] is not 1 or 3: return     -1 and 0 send nothing
ccSndCmd3(0x20 + sqtbl[sq].midiPort, 0)
fade[midiPort].fadeSw = 0
sqStatus[sq] = 0
```

The port volume stays.

`ccSqFade(sq, per, t, sw)` (0x00179b50) sends nothing; it sets the fade
the sound task runs [code]:

```
if sq >= sqNum: return
f = fade[sqtbl[sq].midiPort]                 SQ_FADE, ccSnd +0x94 + 20 midiPort
f.fadeSw = sw; f.time = t
now = hdSynPortVol[sqtbl[sq].hdPort] << 8     the last value sent, bgmVol in it
end = (sqtbl[sq].vol * per) & 0x1ff00
f.rate = (now - end) / t                     signed, truncating; t 0 not checked
f.volume = now; f.endVol = end >> 8
```

No status check: it fades a sequence that has ended or never played.
`SQ_FADE.per` (+2) is never written.

The sound task (`ccSoundRpc`, 0x00182dc0, once a frame) runs
`ccSound::ccSceneFade` (0x00181440), not `ccFade`: `gameStart` is 0 in
the title [code]:

```
fade k = 0, then 1 if sqNum >= 2, then 2 if sqNum >= 3; port p = k + 1
  fadeSw 0: skip
  time 0:   hdSynPortVol[p] = endVol; ccPortVolSet(p, endVol)
            fadeSw & 2 and rate > 0: ccSqStop(k)
            fadeSw = 0
  else:     volume = clamp(volume - rate, 0, 0x10000)
            hdSynPortVol[p] = volume >> 8; ccPortVolSet(p, volume >> 8)
            time -= 1
```

`ccPortVolSet` scales by `bgmVol` again, so a fade starts from an already
scaled value and scales each step once more: at `bgmVol` 128, port 1 is
128 after `ccSqPlay`, then 56, 48, ..., 8, 0, 0 [eemu]. At 256,
`ccSqFade(0, 0, 8, 3)` gives 224, 192, 160, 128, 96, 64, 32, 0, then 0
again with `ccSndCmd3(0x20, 0)` and `sqStatus[0]` 0: nine frames [eemu].
In the same frame `ccSndChangeOption` runs after the fade, so a `bgmVol`
change mid-fade puts port 1 back to full for that frame [code].

`ccSetMainVol(v)` (0x001794b0) [code]:

```
if v == mainVol: return                        ccSnd +0x00
mainVol = v; sdRemote[0] = 1; sdData[0] = ((v << 14) - v) >> 8      no clamp
```

On the sound task's next run `sdCommand` (0x001816f0) slot 0, case 1:
`sceSdRemote(1, 0x8010, 0x0981, value)` and `(.., 0x0a81, ..)`, SPU2 core 1
`MVOLL` and `MVOLR`; for 0 it also reads `MVOLXL` / `MVOLXR` (0x8020,
0x1181 / 0x1281) and writes 0 again to a side still sounding; then the slot
is cleared [code]. 256 -> 16383, 128 -> 8191, 64 -> 4095, 0 -> 0 [eemu].
Core 1's master is after everything: music, effects and core 0's input
(movies, voices).

`ccSndCmd` (0x00181a60) and `ccSndCmd3` (0x00183190) are RPCs to
SNDBASE's servers 0x12346 (`ccSoundFunc`, 0x0a58) and 0x12347
(`ccSoundFunc2`, 0x031c) (`rpcInit` 0x00181d60, `rpcInit2` 0x00183100;
SNDBASE `ccSoundLoop` 0x30ec, `ccSoundLoop2` 0x31b8) [code]. Both return
after the IOP has run the command (mode 0, or no-wait and a poll); both
drop it, returning -1, while `loadFlag` or their busy byte (`rpcCmd`
+0x130, `rpcCmd3` +0x131) is set. A command with bit 0x1000 sends 64 bytes
from the pointer, others the one word; bit 0x8000 asks for a 64-byte reply.

### On the IOP

| EE | SNDBASE | does |
| --- | --- | --- |
| `ccSndCmd(0x140, 0)` | `allSoundOff` 0x2e5c | ports 0-3: all notes off, all sound off, volume 0; then `sqAllStop` (0x2db4): each sequencer with `sqPlayFlag` set switched off, flag cleared |
| `ccSndCmd3(0x200, 7)`, `(0x300, 0)` | `ccSoundFunc2` | `area` 7, `playtype` 0, stored and printed; read only by `bgmChange` (0x07e4), which runs on 0x130 |
| `ccSndCmd(0x9310, &dataLoad)` | `ccSQDataLoadCD` 0x1e44 | `.hd` + `.sq` to iopAddr, `.bd` to SPU 0x12f580 (see [the banks](../formats/snddata.md)); every port reset controllers, all notes off, all sound off, volume 0; replies the `.bd` bytes moved, -1 on a transfer error |
| `ccSndCmd(0x9051, &sndAddr)` | `ccSetHdSynth` 0x257c | port 1 gets the bank (`sceHSyn_Load`) |
| `ccSndCmd(0xa1, 0x2010)` | `ccSetPortAttr` 0x2730 | port 1: priority 0x10, 32 voices |
| `ccSndCmd(0x40, addr)` | `ccSetSq` 0x27d4 | the `.sq` into sequencer 0, every channel to stream buffer 0, block 0, tick 0 |
| `ccSndCmd(0xb0 + p, v)` | `ccSetPortVolume` 0x2794 | `sceHSyn_SetVolume(p, v & 0xffff)` |
| `ccSndCmd3(0x110 + n, 0)` | `ccSoundFunc2` | `sceMidi_MidiSetLocation(n, 0)`, then `sceMidi_MidiPlaySwitch(n, 1)` even if the locate failed; `sqPlayFlag[n]` 1 if the switch worked; replies the play bits |
| `ccSndCmd3(0x20 + n, 0)` | `ccSoundFunc2` | `sceMidi_MidiPlaySwitch(n, 0)` (MODMIDI sends the all-notes-off pair); `sqPlayFlag[n]` 0 if it worked |

[code SNDBASE] Each `ATick` (0x3860), sequencer 0 or 1 with `sqPlayFlag`
set whose MODMIDI play bit has dropped goes through `resetSqData`
(0x3b98): block 0, tick 0, flag cleared - rewound, not playing. Nothing
in the title sends 0x130 (the title's streams are none of
`ccSndStreamCtrl`'s), so `area` and `playtype` do nothing here [inferred].

The sequence: seven channels, program n on channel n, 85 note-ons and no
loop markers; first note at ATick 1, last note on at ATick 9791 (40.80 s),
`FF 2F` at ATick 10943 (45.60 s at 4,167 us), where MODMIDI stops and
sends CC 64 0 and CC 123 0 on channels 0-6 [data, `tools/midi.py --disc
--bank 1`]. It plays once.

### The title's calls

Every music call in DEMO.PRG (all its `jal`s, 0x00400800-0x0040dc40); the
rest are `ccSeOn` and the movie player's own `sceSdRemote` /
`ccSetInputVol`. `SetVolCcs` (0x00405e80) picks the disc volume's CCS file
and makes no sound call [code].

| where | call | when | sent |
| --- | --- | --- | --- |
| `ccThDemo` 0x00400950 | `ccSqStop(0)` | task start | nothing: `sqStatus[0]` is -1 |
| `ccThDemo` 0x00400a04, 0x00400a10 | `ccSetMainVol(saveData->mainVol)`, `ccSqPlay(0)` | `ResetOpFlg` set (every entry after the first; only `ccThDemo` writes it): after the card check, before the opening stream | master if changed (next sound frame); `0xb1`, `0x110` |
| `LogoMain` 0x00404694, 0x004046a0 | the same | LogoAct 4, 60 frames after it was set: after the logos and `opening.pss` on the first boot, after `opening.pss` in each attract | the same |
| `ccThDemo` 0x00400b44 | `ccSqStop(0)` | `Main` returned 5 (the attract) | `0x20`; `sqStatus[0]` 0 |
| `ccThDemo` 0x00400b78 | `ccSqFade(0, 0, 8, 3)` | `Main` returned 2 (New Game), 3 (Load) or 4 (volumes 2-4) | port 1 steps from the sound task, then `0x20` |

```
ccSetupDemo      ccAllSoundOff; Breath(2): 0x140; ccSndSQLoad(7): 0x140, the load,
                 port 1 at 256 * bgmVol >> 8, nothing playing
ccThDemo         ccSqStop(0): nothing; the card check
first boot       logo_b, logo_c, logo_h, opening.pss; 60 frames;
                 ccSetMainVol(mainVol), ccSqPlay(0): the music from tick 0
later entries    ccSetMainVol(mainVol), ccSqPlay(0) at once, no logos
opening stream   no music call
menu             the music ends 45.6 s after ccSqPlay; silence after
attract          2400 idle frames: ccSqStop(0); opening.pss; 60 frames;
                 ccSetMainVol, ccSqPlay(0): the music again from the start
New Game, Load   NewGame(0) / LoadGame() (each ends in SetSoundEnv);
                 ccSqFade(0, 0, 8, 3); Breath(2); ChangeRequest(3, 7)
ccSetupDesktop   ccDeleteAllThread (the sound task has flag bit 0 and stays),
                 ccBreathThread(1), DESKTOP.PRG loaded, ccAllSoundOff (0x0016847c):
                 fadeSw 0 and 0x140 on the next frame; ccSndChangeData(&Wave[dtBgm], -1)
```

- The opening stream: `ccRequestLoadStream(n)` (0x00198da0) calls
  `ccSndStreamCtrl(n, .., 0)` and `(.., 1)` (0x0017d190), which act on
  streams 3, 8, 9, 12, 13, 25, 26, 56-58, 107 and 112-119 only [code]; the
  title's 0 (1 with `m_ParoFLG`) passes. `title1_st1` and `title1_st2` in
  STR1 and STR1E hold no `F_Note` (0xcccc0108) or `F_Pcm` (0xcccc2201) tag
  [data, byte scan].
- The movies: `ccDecodeMpeg(.., 1)` sends `opening.pss`'s audio into core
  0's sound-data input (`sceSdRemote` block transfer; `ccSetInputVol`,
  0x00179590, BVOLL/BVOLR 0x7fff at start, 0 at reset) and touches no
  synthesizer port and no master volume [code]. The music never plays
  under a movie: it starts after `opening.pss`, and the attract stops it
  first [code].
- The end of the music: `ccSqPlay` comes at least 410 stream frames and
  2400 menu frames before the attract's `ccSqStop` (about 46.9 s at 59.94
  Hz if the stream advances a frame per frame), so the music has always
  ended by then and 0x20 finds sequencer 0 rewound and stopped [inferred].
  It ends roughly 39 s into the menu on the same assumption.
- Load's volumes: `ccSaveData::LoadGame` (0x00175110) ends in
  `SetSoundEnv` (0x00175284; `NewGame` at 0x001750f0), called by
  `ccThDemo` just before the fade [code]. A loaded save whose `bgmVol`
  differs flags a change; on the next sound frame `ccSndChangeOption` runs
  after `ccSceneFade`, so port 1 goes back to full at the new `bgmVol` for
  that frame, and the fade then continues from its own volume under the new
  scale; a different `mainVol` changes the master [inferred from the order
  in `ccSoundRpc`]. `NewGame(0)` keeps the save's volumes, so it changes
  nothing [inferred].
- Leaving: the fade needs nine sound-task frames. The game gives it
  `Breath(2)`, then `ccSetupDesktop`'s `ccBreathThread(1)` and the
  432,128-byte DESKTOP.PRG load before its `ccAllSoundOff` clears `fadeSw`
  [code]; whether it always reaches 0 is not measured. The 0x140 and the
  desktop bank's load silence port 1 either way.
- Option: the title makes no call. The system menu's Sound row
  (`ccDtMenu::SoundMenu`, `SetSoundEnv` at 0x0016e034; [the system
  menu](#the-system-menu), list 1's fourth item [eemu]) applies each
  change: `ccSetBgmVol` flags a change and on the next frame
  `ccSndChangeOption` (0x00179630; `game->mode` is not 5) sets port 1 to
  `0x100 * bgmVol >> 8` and ports 2, 3 to 0, playing or not; a new
  `mainVol` reaches the master through `sdCommand` [code].

### For the audio crate

`crates/piney-audio`'s `driver` already has each of these
(`Driver::sq_load(SqContext::Title)`, `sq_play`, `sq_stop`, `sq_fade`,
`scene_fades`, `set_main_volume`, `task`), checked against the fixture;
the list is what any implementation must keep, with the gaps found.

- `ccSndSQLoad(7)`: stop fades 0 and 1; let one sound frame pass that
  ends in all sound off; load `sqDataTitle[0]` (`.bd` to SPU `0x5020 +
  commseTbl[2]` = 0x12f580), which itself stops the sequencers and puts
  every port's volume to 0; port 1 gets the bank with attributes 0x2010;
  sequencer 0 the `.sq` at tick 0, not playing; `sqNum` 1; `sqtbl` =
  {0, 1, 256}, {1, 2, 0}, {2, 3, 0}; ports 0-3 to `seVol`,
  `256 * bgmVol >> 8`, 0, 0, the sent values kept per port; `sqStatus`
  all -1 (not 0: `ccThDemo`'s first `ccSqStop(0)` must send nothing).
  `area` / `playtype` (0x200, 0x300) can be left out.
- `ccSqPlay(0)`: nothing unless `sq < sqNum` and the status is not 1 or
  3; port 1 to `vol * bgmVol >> 8` (keep that value for the fades); locate
  sequencer 0 to tick 0 and switch it on; status 1. Never set the status
  back when the sequence ends by itself, so a repeated `ccSqPlay(0)` stays
  silent until a `ccSqStop(0)`. On the IOP side the ATick rewinds an ended
  sequencer (`resetSqData`) without playing it. Small gap: SNDBASE
  switches the sequencer on even when the locate fails and flags it only
  when the switch works; `midi::Sequencer::play` switches on only after a
  successful locate, and `Engine::apply` sets `play_flag` either way - no
  title data takes that path.
- `ccSqStop(0)`: nothing unless the status is 1 or 3; switch sequencer 0
  off (the all-notes-off pair); `fade[0]` off; status 0; the port volume
  untouched.
- `ccSqFade(0, 0, 8, 3)`: no command at the call; the fields as above,
  starting from the last value sent to port 1; then, each frame while
  `gameStart` is 0, port 1 written every frame from the fade's own 16.16
  volume through `ccPortVolSet` (so `bgmVol` applies twice), and at time 0
  the end volume and a stop. Guard `t` 0 (the game divides by it).
- `ccSetMainVol(v)`: nothing if `v` equals the current main volume; else
  on the next sound frame both sides of core 1's master to
  `((v << 14) - v) >> 8`. The game does not clamp.
- Load: the loaded save's `mainVol`, `bgmVol`, `seVol` must reach the
  driver (`set_volumes`) in the same frame as the leaving `ccSqFade`, as
  `LoadGame`'s `SetSoundEnv` does, before the fade's first sound frame.
- All sound off (0x140): SNDBASE also sets every port's volume to 0 and
  stops the playing sequencers; `Engine::apply` only cuts the voices. In
  the title a bank load always follows, which does the rest, so nothing
  heard differs there [inferred].
- The runtime must run the sound task once per game frame through mode
  changes. `crates/piney-game` enters the desktop in the frame
  `ChangeRequest(3, 7)` is made, and `DesktopMode::enter`'s `AllSoundOff`
  reaches the audio on the next frame, so the port's leaving fade gets
  about three steps (224, 192, 160) before it is cut; the game also gives
  it `ccSetupDesktop`'s frames up to its `ccAllSoundOff` [inferred from
  `crates/piney-game/src/session.rs` and `desktop.rs`, not run; the game's
  count not measured].

## Checks

`tools/test_demo_rs.py` runs each function in eemu with the calls into the
main executable hooked and compares the port on the same states:
`MoveCurNut` over random pad sequences, `SwitchCur`, `PlayNeutral`,
`ChangeMainAct`, `MOVEcount`, `SendPacket` after each `Entry*`,
`AllTransparency` (bit-exact), `AllAnimate` (the icons' turn bit-exact),
`AllDraw`, the `Set*` and `Play*` functions, `Main` over every volume 1
action, `LogoMain`, `PlayBootMemCard` with `BootMem_Control`,
`Data_Control::Main` and `NextProccess` native, the question's text
addresses, `ccSaveData::NewGame` (the whole 0x8530-byte save and the copied
names, after (1) then (0) from the boot's save - ccSaveData's constructor
and `Init(1)`, `tools/test_save_init_rs.py` - with and without the
parody flag, and from random bytes), `ccThDemo` itself with
`PlayBootMemCard` and `Main` scripted, the load screen
(`DataLoad_Control::Main_Control` with every step, `ccSaveSys` built,
started and stepped by its own code over a card kept as files, frame by
frame: the state, every text and cursor drawn, the sounds, and the whole
save at the end), `ccSaveData::LoadGame` on random saves, and the icons'
light: the port's title run from boot to the menu and, for every lit object
drawn at two frames, its `lwMatrix` through the game's light setup,
`SetLightMatrix` and `ccSetMatrixPacket` (eemu with VU0 macro mode), then
VU1's `mc_SetMatrix`, `mc_SetObjParam` and `mc02_Start00`/`01` +
`mc_DrawTriL` (`tools/vu.py`'s `Vu`) over the model's vertices, against the
port's `Lights` and the colours `piney_gs::convert::mmat` gives. The system
menu in the title's mode is checked by `tools/test_desktop_menu_rs.py`.
`crates/piney-demo/tests/demo.rs` drives the port on the disc from boot to
each way out.

The music: `tools/sound_ee.py fixture` runs `ccSndSQLoad(7)`, `ccSqStop`,
`ccSetMainVol`, `ccSqPlay` and `ccSqFade(0, 0, 8, 3)` with the sound
task's frames in eemu, and `crates/piney-audio/tests/driver.rs` holds the
driver to it (the `seq load 7...` lines). Run the same way for this
section: the attract order (a second `ccSqPlay(0)` sends nothing;
`ccSqStop(0)` then `ccSqPlay(0)` sends `vol 1 256; play 0` again),
`sqStatus` after each step, the fade's fields (`fadeSw` 3, rate 8192, time
8, volume 0x10000, endVol 0), a fade at `bgmVol` 128, a `bgmVol` change
while playing (ports 1-3: 64, 0, 0) and `ccSetMainVol(0)` then 256 (master
0, then 16383).

## Unknown

- How many frames `ccThSaveSys` takes to answer the boot check (the
  `sceMcSync` waits in `CheckPort`); the port says 2.
- How many frames the stream takes to set +0x17e bit 3 before
  `PlayOpeningStream` looks at the pad: the port starts from the stream's
  first step.
- `m_tempPN`'s first value (the heap's): whether "Data loaded." plays its
  jingle on a first acknowledgement.
- How many frames `DataRead` and the index read take on a real card; the
  port's card answers at once, so "Loading...." (0x10013 / 0x10014) is
  never seen.
- Volumes 2-4: the `NextDataLoad_Control`'s `m_PrevFlg` branches of
  `Data_Control` (reading the previous volume's card) and
  `ccSaveData::ConvGame`.
- What `sf` 7 of `ChangeRequest` means.
- What the logo PSS files show and how long each is.
- How many frames `ccSetupDesktop` takes to reach `ccAllSoundOff`, so
  whether the leaving fade ends at 0.
- How long the IOP takes over the 0x9310 load (the EE task waits in the
  RPC).
- Where the sound task runs in a frame relative to `ccThDemo`, so the
  frame a new master volume lands on.
