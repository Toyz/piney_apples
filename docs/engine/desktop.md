---
title: The ALTIMIT desktop
status: partial
volumes: INF
covers: INF SLUS_202.67:0x00168320 ccSetupDesktop, 0x001524d0 ccAnm::Draw, 0x0013f220 ccObj::Draw, 0x0013eab0 ccModel::Draw, 0x001052c0 ccView::SetView, 0x00104ae0 ccView::SetFrame, 0x00104e40 ccView::SetAspect, 0x001386b0 ccCam::SetMatrix_PosRotXYZ, 0x0014e950 ccStream::DecodeF_Obj, 0x0014ece0 ccStream::DecodeF_Camera, 0x0015aed0 ccSprite::MakePacketStr, 0x0015abb0 ccSprite::SendPacket, 0x0015c0c0 ccSprite::SetTag, 0x00108570 ccLayer::AddAll, 0x0015fb80 ccScFade::SendPacket, 0x001b32f0 ccEvent::CheckOperate, 0x00178af0 ccSaveData::NewMail, 0x00178b80 ReadNewMail, 0x00178ac0 CheckMail; INF desktop.prg:0x00400910 ccThDesktop, 0x004015c0 Desktop_control::SetStream, 0x004014d0 SetWall, 0x00400d10 PlayOpening, 0x00400e40 SelectMode, 0x00401020 ChooseMode, 0x00402690 DrawLogo, 0x00402300 DrawBack, 0x004020c0 NewIconDraw, 0x00408350 MailList_control::MainMailer, 0x00409800 AddMailList, 0x0040ca10 MoveLine, 0x0040a3f0 WriteTitle, 0x0040a150 WriteMail, 0x0040b1a0 ResWindow_Control, 0x0040bbf0 PreviewRes, 0x0040d1d0 Web_control::MainWebNews, 0x0040d5c0 AddHtmlList, 0x0040dc10 MoveCur, 0x0040ded0 WebMove, 0x0040e0f0 WriteList, 0x0040e5d0 SetLength, 0x0040e7e0 SetWebLength, 0x0040ec10 DeleteWeb, 0x00403c40 Acces_control::MainAcces_control, 0x004044f0 AddWallList, 0x00404710 WallList, 0x00404c70 ChangeWall, 0x00404f20 SetLength, 0x004050c0 WriteList, 0x004053d0 ListMove, 0x00405700 Audio_control::MainAudio_control, 0x00406450 AddWaveList, 0x004065b0 AddStrList, 0x00406770 SelectMode, 0x00406f90 StartStream, 0x004072c0 ChangeWeve, 0x00407730 SetLength, 0x00407860 WriteStrList, 0x00407db0 WriteList, 0x00408050 ListMove, 0x0015f8e0 dec2sjis, 0x00171c20 ccSaveSys::MainProccess, 0x00173f90 NextProccess, 0x001716e0 CheckRightInfo, 0x00174300 GetMessage, 0x003753e0 __sinit_sdmng.cpp, 0x001bba30 ccFacePanel::Draw, 0x0016ac00 ccThDtMenu, 0x0016a7a0 ccDtMenu::OpenMenu, 0x0016aeb0 SystemMenu, 0x0016e930 ResetMenu, 0x0016ee20 FadeMenu, 0x001a4730 ccMessage::Disp, 0x001a5960 Check, 0x001a54a0 Change, 0x001a6000 ChangeInfo, 0x001b27b0 ccEvent::DispInfo, 0x0016a8e0 ccDtMenu::CheckMenuType, 0x001b7910 ccMenuWindow::DispSelectCursol, 0x001baa10 DispPageCursol, 0x00414310 NameEntry_Control::Main, 0x004195b0 Init, 0x00415830 AllTlans, 0x00418960 JudgmentProcess, 0x00419390 CutInBlock, 0x0041a4e0 CurRepeat, 0x0040ede0 SaveData_control::MainSaveData_control, 0x0040eef0 SetData, 0x004106e0 SlotSelect, 0x00410a00 SlotStateData, 0x00410c50 Slot_control, 0x00411720 SaveData, 0x0040fa50 ReCheckData, 0x00411ae0 ListMove, 0x00410f80 SetSavePar, 0x00411d80 InfoWindow, 0x00410430 DispButton, 0x0040fc30 DiaWindow, 0x00412040 ResWindow, 0x004101b0 TimeAlphaCurDraw, 0x0040fb00 DrawBack, 0x0041bac0 desktopFileList, 0x0042b1b0 WallTbl, 0x0041c120 HtmlTbl, INF desktop.prg:0x004125c0 ccThStaffRoll, 0x004126d0 ccThStaffRollCtrl::ccThStaffRollCtrl, 0x00412a80 Main, 0x00412b70 ChangePage, 0x00412d10 BuildPage, 0x00412e50 _Random, 0x00413210 _Fix, 0x00413410 _End, 0x00413690 _BGOnly, 0x00413990 ccThStaffRollLine::Draw, 0x00413b10 Init, 0x00413bb0 SetLink, 0x00413d50 StopChar, 0x00413e50 Build, 0x00414020 DrawFix, 0x00414100 Except, 0x00414210 DrawBackShadow, 0x00412410 ccTransCode2Name, 0x0042c500 g_srDataGrp; INF SLUS_202.67:0x00105080 ccView::SetLayerCenter, 0x00104aa0 ccView::ApplyLayerScreenMatrix
worklog: 37, 41, 45
---

# The ALTIMIT desktop

The player's desktop computer (mode 3): an animated 3D scene out of one CCSF
file, `xddesk01`, seen through a fixed camera, with a column of six icons
(The World, Mailer, News, Accessory, Audio, Data), a page title for the
selected icon, and 2D text and cursors over it. It is the `DESKTOP.PRG`
[overlay](../formats/prg.md); its content tables are on
[the desktop text page](text.md). `crates/piney-desktop` is the port,
checked against the game's code by `tools/test_desktop_rs.py`, `tools/test_desktop_data_rs.py`, `tools/test_desktop_menu_rs.py` and `tools/test_desktop_name_rs.py`; the event scripts' announcements on it by `tools/test_desktop_announce_rs.py`.

## Setup and the task

`ccSetupDesktop` (0x00168320) loads `cdrom0:\DATA\DESKTOP.PRG`, clears the
background colour to black, builds the file list (`cmnFileList`,
`gcmnFileList`, `strcmnFileList`, then `ccAddRequestFileListDesktop`:
`desktopFileList` = `xddesk01.CCS`, plus `WallTbl[dtWallpaper].Wall`, the
wallpaper's own file `XDDWALnn.CCS`), starts the music
`ccSndChangeData(&Wave[saveData.dtBgm], -1)`, sets `frameRate` 1 (60 frames
a second), `ccGame.enableReset` 1, and starts two tasks: `ccThDtMenu` (the
system menu over the desktop, not described here) and `ccThDesktop`. In
order, with the event passes ([Setup](#setup)):

```
ccGame.enableReset = 0; status = 2; fontOnFlip; ccDeleteAllThread; clear; Breath(2)
ccStartThEvent; ccEnableThEvent(0)            game +0x04 set (a change asked): return
load DESKTOP.PRG; ccSetFileListDesktop; ccFileExistCheck; ccAllSoundOff
ccEnableThEvent(2)                            game +0x04 set: return
ccSndChangeData(Wave[dtBgm]); ccLoadResourceFL
ccSystem::SetFrameRate(1)                     0x001684dc
enableReset = 1; ccThDtMenu, ccThDesktop; ccEnableThEvent(4)
                                              game +0x04 set: return
ccScFade::Init; ccSnd +0x17 = 1; ccSndBgmCtrl
```

`ccThDesktop` (0x00400910) runs, one `Breath` a frame:

```
DESKTOP_FLG = 1; ccsc = GetCCSAdrs("xddesk01")
this = new Desktop_control; SetStream(ccsc); SetWall(); breathe
PlayOpening()              ccSeOn(0); per frame: frame 60 of the opening
                           plays ccSeOn(1); SetView(Back[0].cam); forward and
                           draw Opening, draw ScrBack; until the opening
                           ends or ok|cancel is pushed
AddAllList()               the mail list (below); News, wallpaper, music lists
loop: breathe; SelectMode(); ChooseMode()
```

## Desktop_control

`SetStream` (0x004015c0) sets every animation from `xddesk01` (members by
offset; each is a `ccAnm`):

| member | animations | stepped once at setup |
| --- | --- | --- |
| `Back[5]` +0x114 | `ANM_xddcamer`, `xddback0`, `xddtunne`, `xddwave0`, `xddrainb` | 0, 1 |
| `Icon[12]` +0x118 | `ANM_xddroll1..6`, then `ANM_xddsele1..6` (selected) | |
| `PageTitle_IN`, `_OUT` +0x120 +0x124 | `ANM_xddr_in1`, `ANM_xddl_ou1` | |
| `Title` +0x10c | `ANM_xddfade1` | |
| `Logo` +0x128 | `ANM_xddlogo1` | |
| `Opening` +0x12c | `ANM_xddesk01` | |
| `Mailview[2]` +0x130 | `ANM_xddpage2`, `ANM_xddphot0` | |
| `Webnews` `Acces` `Audio` +0x134.. | `ANM_xddpage3_1` (by `volumeNum` 1), `xddpage4`, `xddpage5` | |
| `Data[4]` +0x140 | `ANM_xddpage6`, `xddsel60..62` | |
| `NewIcon` +0x144, `BackBar` +0x108 | `ANM_xddnew0`, `ANM_xddback1` | both |
| `ScrBack` +0x110 (`SetWall`) | `WallTbl[dtWallpaper].AnmName` in its own file | yes |

and two layers, 126 (`backlayer`, made `ccLayer::active`: every `ccAnm`
draws here) and 127 (`kanjilayer`), and `mask`, a `ccMask(64, 0)` on the
kanji layer with `xddesk01::TEX_xddcurs1`.

The page-title name arrays are `PageTitleAnm_R_O` +0x4c (`ANM_xddr_ou1..6`),
`_L_O` +0x64 (`xddl_ou`), `_R_I` +0x7c (`xddr_in`), `_L_I` +0x94
(`xddl_in`), `TitleLogo` +0xac (`xddfade`), `LogoAnm` +0xc4 (`xddlogo`).

### SelectMode (0x00400e40)

Nothing happens unless `check` (+0x19c, DrawLogo's last result) is 1.
Pad bits are `ccPad.push` (see `crates/piney-input`).

```
if App == -1:
  push & (UP|RIGHT):  se 2; DrawFlg = 0; LRflg = 0; dsel -= 1; Lock = 1;
                      check = 0; dsel < 0 -> 7
  else push & (DOWN|LEFT): se 2; DrawFlg = 0; LRflg = 1; dsel += 1; Lock = 1;
                      check = 0; dsel >= 6 -> 6; flg = 0
  else StartFlg and push & assignPADok and not push & START
       and CheckOperate(dsel) and CheckOperate(dsel + 19):
                      DrawFlg = 0; NewMail = 0; App = dsel;
                      dsel 0: ccGame.enableReset = 0
push & SELECT: DrawFlg toggles 0/1 (1 hides the back and page titles)
```

`dsel` 6 and 7 are the wraps: 6 turns the icon ring on to icon 0, 7 back to
icon 5. `ccEvent::CheckOperate(n, 0)` (0x001b32f0) fails when bit `n` of
`ccEvent.operate` (+0x770, u64) is set, and records the first `n` asked in
`ccEvent.operateSet` (+0x778) while that is negative: the events lock icons
and wait on the player trying them.

### ChooseMode (0x00401020)

`App` 0-5 enters a loop that owns the frames until it returns:

| App | icon | on entry | each frame | on exit |
| ---: | --- | --- | --- | --- |
| 0 | The World | enableReset 0, se 8, `ccScFade::EntryFade(20, 0, 0x80000000, 0, 0, 512, 384)`, `ccSoundFadeOut` | while `CheckFade`: DrawLogo, DrawBack | `ccGame::ChangeRequest(4, 7)` ([TOPPAGE](toppage.md)) |
| 1 | Mailer | DrawFlg 0, `Mailer->SetBack`, se 4 | `MainMailer`; `Mailer->DrawBack`; `DrawMailer` | se 7 |
| 2 | News | DrawFlg 0, `weber->SetData`, se 4 | `MainWebNews`; `weber->DrawBack`; `DrawWebnews` | se 7 |
| 3 | Accessory | `acces->ControlDataSet(kanjilayer, ScrBack, &WallTbl[dtWallpaper])`, se 4 | `MainAcces_control`; `DrawAccess`; `acces->DrawBack` | se 7 |
| 4 | Audio | `audio->ControlDataSet`, se 4 | `MainAudio_control`; `audio->DrawBack`; `DrawAudio` | se 7 |
| 5 | Data | enableReset 0, se 4, `ccSaveSys::StartReq(3)`, `save->SetData` | `MainSaveData_control`; `save->DrawBack`; `DrawData` | se 7, enableReset 1, `EndReq` |

A mode leaves when its `Main*` returns -1; that frame then draws
`check = DrawLogo(); DrawBack()` and sets `App = -1`. Every mode's `Draw*` runs
`Desktop_control::DrawBack`, so the new-mail notice keeps drawing (on the
kanji layer, in whatever frame the mode left it) while a mode is open. Any other `App` does
that every frame.

The fader (`fade.cpp`) draws its element each frame on the font layer (240):
a flat TRISTRIP over the frame, colour per channel
`c0 + trunc((c1 - c0) * cnt / tcnt)` with `cnt` the frames drawn (held at
`tcnt`), blend `(Cs - Cd) As + Cd`. The World's fade is black with alpha
0, 6, 12, 19, 25 ... 121, then 128.

### DrawLogo (0x00402690)

A jump table on `dsel` (0x0042e610). Each case: if `Lock`, set `Logo` to
its logo; step and draw `Logo`; then by `LRflg` (1 = moved on, 0 = moved
back), if `Lock`, set the page title coming in, the one going out and the
fade, and clear `Lock`; step the three; if `DrawFlg` is 1 return 1; else
draw IN then OUT (case 0 turning back draws OUT first; case 1 turning on
draws OUT only once `StartFlg` is set, so the first title at start-up comes
in alone). When all three steps report the end, `StartFlg = 1` and it
returns 1; otherwise it sets their `frameSpd` to 768 (three frames a step)
and returns 0.

| dsel | logo | moved on: IN, OUT, fade | moved back: IN, OUT, fade |
| ---: | --- | --- | --- |
| 0 | 1 | r_in1, -, fade1 | l_in1, r_ou2, fade1 |
| 1 | 2 | r_in2, l_ou1, fade2 | l_in2, r_ou3, fade2 |
| 2 | 3 | r_in3, l_ou2, fade3 | l_in3, r_ou4, fade3 |
| 3 | 4 | r_in4, l_ou3, fade4 | l_in4, r_ou5, fade4 |
| 4 | 5 | r_in5, l_ou4, fade5 | l_in5, r_ou6, fade5 |
| 5 | 6 | r_in6, l_ou5, fade6 | l_in5, r_ou6, fade6 (sic) |
| 6 | 1 | r_in1, l_ou6, fade1; then dsel = 0 | - |
| 7 | 6 | - | l_in6, r_ou1, fade6; then dsel = 5 |

(`r_in1` is `ANM_xddr_in1` and so on.) Case 5 moving back names case 4's
page titles; it is reached only from `dsel` 6, which the wrap never leaves
there.

### DrawBack (0x00402300)

```
SetView(active->view, Back[0].cam)
ScrBack.Draw(); if acces+0x68: acces->PhantomDrawBack()
NewIconDraw()
if DrawFlg != 1: Back[1].Draw(); for i in 2..5: step Back[i], draw it,
                 set it again when it ends; Title.Draw()
else:            BackBar.Draw()
for i in 0..6: step Icon[i] and Icon[i+6]; draw Icon[i+6] (selected) for
               the selected icon (6 -> 0, 7 -> 5), else Icon[i]
the new-mail notice (below)
```

`Back[0]` (the camera) and `Back[1]` are never stepped after setup.

`NewIconDraw` (0x004020c0) draws the NEW mark (`ANM_xddnew0`) at
`IconPos[0]` (-13500, 8300, 90000) over the mailer while `NewMailNum > 0`
and at `IconPos[1]` (-13500, 5300, 90000) over News while `weber+0x48 > 0`.
It rests `TmpCou` = 126 frames, then spins about x by `4 pi / 40` a frame
for two turns (`IconY` counts to 4 pi; `tmpIrot` wraps at pi). Resting, it
flashes: `v = 2 TmpCou` for `TmpCou <= 64`, else `256 - 2 TmpCou`; spinning,
`v = 0`. It draws under `ccDrawEnv::SetFogBlend(90, (255 - v, 255 - v,
255))`: a constant fog coefficient F = 25 toward that colour.

The new-mail notice: when `AddMailList` returned 1 (`NewMail`), se 3, and
each DrawBack: while `infoFlg < 200` the alpha rises by 5 to 128; after
`infoFlg > 200` it falls by 5 and at 0 clears `NewMail`;
`infoFlg = Mailer->InfoWindow()`, which draws "You have new mail." in an
8 x 8-tile window at (160, 50) on layer 128 and counts its frames (never
reset, so the notice shows once per visit).

## The 3D draw

Everything `ccAnm::Draw` (0x001524d0) draws goes to layer 126 through its
view. An animation that holds a camera (`F_Camera`) copies the view and
sets it; the desktop's camera animation `ANM_xddcamer` is only a camera.

**Camera.** An `F_Camera` record (0x0502, frame section of the Anime chunk)
is `u32 object, u32 flags`, then one f32 per clear flag bit 1-8: position
xyz, rotation xyz in degrees, one unused value, the full horizontal field of
view (`ccCam.viewRange`, 45 after `ccCam::Init`). It applies when a step
crosses its frame; cameras are not interpolated. The view matrix is the
inverse of `T(pos) Rz Ry Rx Rx(pi)`: view space is +x right, +y down, +z
forward. `ANM_xddcamer` holds pos (0, 0, 292609.3125), no rotation, fov
9.999999, so view = (x, -y, 292609.3 - z).

**Projection** (`ccView::SetView` 0x001052c0, Sony's view-screen matrix):

```
X = scrz * ax * v.x / v.z + scx         scrz = W / (2 tan(fov * pi / 360))
Y = scrz * ay * aspect * v.y / v.z + scy   aspect = 1.3333334 * H / W = 7/6
Z = az / v.z + cz    az = far near (zmax - zmin) / (far - near)
                     cz = (zmin far - zmax near) / (far - near)
```

W, H = 512, 448. The desktop's layer 126 keeps `ccLayer::Init`'s view:
`SetFrame(0, 0, 512, 384, 256, 192, 1, 1)`, so `scx` = `scy` = 2048,
`ax` = `ay` = 1, `zmin` 1, `zmax` 2^28, near 8, far 2^20. X and Y are GS
primitive coordinates; the frame buffer pixel is (X - 1792, Y - 1824)
(XYOFFSET is (0x7000, 0x7200) in 12.4, no half-pixel offset). With fov 10,
`scrz` = 2926.09: at z = 0, 100 world units a pixel.

**Objects.** A ccAnm draws the Obj entries of its index (the objects its
records name, ExtObj copies resolved, each its own instance) whose model is
set and whose `dispSW` is 3. `lwMatrix` is the chain of the animation's own
objects down to the object, under the ccAnm's own matrix (identity except
for the NEW mark). Object poses come from the controller records
([animation](animation.md)) and from `F_Obj` records (0x0101: u32 object,
u32 flag, f32 pos[3], rot[3] in degrees, scale[3], transparency clamped to
0..1, u32 dispSW; `dispSW = (rec & 1) | 2`), which apply when a step
crosses their frame; the opening `ANM_xddesk01` is mostly F_Obj records.
Transparency is the object's own (`worldtp`; no Obj2 in `xddesk01` has
`succession` bit 0, which would chain it to the parent), times the ccAnm's
`localtp` (1). An object at or below 1/128 is not drawn.

**Mmats** (`ccModel::Draw` 0x0013eab0): `t = tp * material.transparency`;
a mmat with `t < 127/128`, or whose texture flag has bit 8, is translucent
and keeps `t`; the rest are opaque with `t = 1`. Every model in `xddesk01`
is rigid and unlit (mtype 0): vertex alpha is `trunc(A_vertex * t)`.

**GS state** (`ccSetMaterialPacket` 0x0013e6c0): ALPHA `(Cs - Cd) As + Cd`
(blend type 0); TEST ZTE, ZTST GEQUAL, ATE, ATST GEQUAL,
AREF = `trunc(texture aref * t)`, AFAIL FB_ONLY (the alpha test only gates
the Z write); ZBUF PSMZ32, Z written; TEX0 MODULATE with TCC; TEX1 bilinear
(`ccDrawEnv::Reset`); CLAMP when the texture flag has 0x10, else REPEAT;
PRIM 0x7c (TRISTRIP, IIP, TME, FGE, ABE, perspective STQ). FGE is on with
F = 255 (no visible fog) except for the NEW mark. A triangle with a vertex
outside GS primitive space is dropped, not clipped.

**Texture offsets.** A 0x0202 material record sets `ccMaterial.u/v`
(1/4096 units, minus the crop); the model packet's STROW is
`(u >> 4) & 0xff`, added to each vertex's S and T in 1/256 of the texture.

## Draw order

- Layers go to the GS in ascending priority (`ccLayer::Add` 0x00108930,
  `AddAll` 0x00108570): 126 (3D), 127 (kanji), 128, 129 (mail reply
  windows), 240 (font layer, the fader).
- Within a layer, opaque model chains and `ccSprite::SendPacket` packets are
  **prepended** (0x0013f180, 0x0015ac88): the last sent is drawn first.
- Then the layer's sorted group (`ccDLSort`): one node per model's
  translucent mmats. The key (`ccModel::Draw` 0x0013eb38) is the screen Z of
  the model's Bbox centre when it has one; no desktop or wallpaper file does,
  and then the key is `M[2][3] / M[3][3]` of the model's world-screen matrix
  M, read from the w row (offsets 44 and 60) rather than the translation
  column. `ccDLSort::Add` (0x001081d0) builds a binary tree, an equal key
  going left, read in order (0x00108100): ascending key, the later of equal
  keys first.
- A model whose `tp` is below 1/128 is not drawn (0x0013eaf8), nor a mmat
  whose `tp * transparency` is (0x0013ecdc).
- Inside a model, mmats go in reverse order.
- Each layer's list starts with its view's SCISSOR_1.

## The 2D draw

**Layer views.** `ccView::SetFrame(x, y, w, h, cx, cy, ax, ay)` works in a
512 x 384 logical screen: `x' = x W/512`, `y' = y H/384`; the scissor is
`[trunc(x' + .5), trunc(x' + .5 + w') - 1] x [trunc(y' + .5), trunc(y' + .5
+ h') - 1]`. `SetAspect` builds `layer_screen`:
`sx = 16 W ax / 512`, `sy = 16 H ay / 384`,
`ox = ((4096 - W + 2 SCAX0) 16 + 1) / 2`, `oy` likewise. A logical point
(x, y) lands at GS `(trunc(x sx + ox), trunc(y sy + oy))`. The default view
has `sy` = 18.666666 (y stretched 7/6); the desktop's panels pass
`ay = 6/7`, which makes it 16.

**Sprites** (`ccSprite`, `SetPrim(m, 0)`: SPRITE, TME, ABE, FST; every
desktop `ccMask` is `ccMask(m, 0)`, a plain sprite). `MakePacket(code, 1)`
draws cell `code` at `(dx + cx, dy + cy)`, `sx` x `sy` logical units, UV
`u0 = col su 16 + wu`, `v0 = TH 16 - wv - row sv 16 - 1` (texels x 16;
rows are stored bottom-up), and moves dx on by sx. `SendPacket` sends the
queued cells with `SetTag`'s state: TEST ZTST ALWAYS, ZMSK, CLAMP REPEAT,
TEX1 nearest, ALPHA `alphaBlendTbl[alphaBlendType]` (0x00348580).
`TEX_xddcurs1` (64 x 64) holds the desktop's cells:

| cell | wu, wv (1/16 texel) | size |
| --- | --- | --- |
| big cursor | 208, 0 | 14 x 11 |
| small cursor | 0, 0 | 13 x 10 |
| unread mail | 0, 208 | 13 x 10 |
| read mail | 192, 208 | 13 x 10 |
| scroll bar | 0, 368 | 7 x 7, stretched |
| window tiles | 0 / 128 / 256, 496 / 624 / 752 | 8 x 8 |

**Text** is `ccKanji` ([font](font.md)): every desktop kanji is
`Init(3, 16)`: a 128 x 128 texture, 16 packets, kt 0. `Disp` rasterises
and sends at once; in VRAM every kanji shares one scratch page, re-uploaded
before its own sprites, so each is its own texture in effect. The two float
arguments of `Disp` are never read.

## Mail

`MailList_control` (mailer.cpp). The inbox is built once, by `AddMailList`
(0x00409800) from `AddAllList` after the opening:

```
for each m in mailOrderList, stopping at the first -1:
  s = CheckMail(m)                       (1 becomes 2)
  s 0: skipped; s 1 or 2: NewMailNum++ (s 1 makes the result 1)
  s 4, 5, 6: the table's reFlg = 0, mailFlg = 1
  listed at the front (newest first)
```

`mailList[n]` (+0x2264) is 0 absent, 1 delivered and not yet seen, 2
unread, 4 read, 5 replied with `oneRes`, 6 with `twoRes`. The event
opcodes 107-109 deliver with `ccSaveData::NewMail(n)` (state 1) or
`ReadNewMail(n)` (4), each only if `n` is not already in `mailOrderList`
(+0x2464), which they append to; opcode 110 sets `mailList[n] = 0`. A new
game has no mail; event 1 delivers mails 4, 5 and 320.

`MainMailer` (0x00408350) by `MailMode`: 0 enter (an empty list waits for
cancel; otherwise se 5, the sender's photo, and the list in the same
frame), 1 the list, 2 reading a mail that takes no reply, 3 a mail that
waits for one (`reFlg` 1), 4 leave (returns -1).

**The list** (`MailList`, `WriteTitle`) on the kanji layer framed
`SetFrame(167, 70, 318, 250, 159, 125, 1, 6/7)`: 14 rows from `MinMax` at
y = 48 + 17 r, title at x 139, sender at x 20, the selected row prefixed
`#G`; the unread icon pulses (`IconScale` 1.0-1.3 by 0.012 a frame, drawn
at (8, 48 + 9 (s - 1) + 17 (r + 1)) with centre (-6 s, -17 s)), the read
icon at (0, 48 + 17 r); "You have N new mail(s)." / "No new mail." at
(20, 3); the big cursor at (0, 53 + 17 row), alpha
`48 + tri(5 count mod 160)` (48 to 128 and back every 32 frames);
the scroll bar at x 311, length `min(224, 224 * 14 / n)`, y
`52 + (224 - len) / (n - 14) * MinMax`, or its end on the last mail.
Up and down move the cursor with `CurRepeat` (a push moves at once; held,
the `ccPad.repeat` bits count to 30, then every 9), wrapping; the list
scrolls down past row 9 and up above row 4. OK (once released since
opening) opens the mail, se 4; cancel leaves.

**Reading** (`PreviewMail`, `WriteMail`): body lines at (8, 48 + 17 r),
title at (140, 24) and sender at (22, 24) without shadows, the icon at
(0, 27); up/down scroll. Cancel: se 7, then four frames of only the scroll
bar, then `mailList[n] = 4` unless 5 or 6, `mailFlg = 1`, back to the list.

**Replying** (`ReplyMail`, `SubReplyMail`, `ResWindow_Control`,
`PreviewRes`): OK or cancel on the mail opens the chooser (layer 129 text,
layer 128 window `reLaysize` (90, 270, 224 x 80), fading in by 8 a frame):
the two reply titles, "Don't reply", "Choose reply e-mail.", small cursor.
A reply shows its text (body at (20, 48 + 17 r), title (139, 22), sender
(20, 22), the player's photo `CMP_xddphot01`); OK asks "Send e-mail?" YES /
NO; YES sets `mailList[n]` to 5 or 6 and shows "Reply sent." at (200, 150)
until OK, then `reFlg = 0`, `mailFlg = 1`. "Don't reply" leaves the mail
unread and waiting.

The sender's photo is the clump `CMP_xddphot{fromNO + 1}` placed at
`OBJ_xddphoto` in the mailer's own `ANM_xddphot0` and drawn with that
object's transparency.

**The port's own mail.** Not the game's: on Infection the port patches
event 1 (`piney_event::extras`) to send one more mail between Yasuhiko's
"Registered yet?" (4) and CC Corporation's "Thank You" (5) and "Version
Update" (320). The mailer lists the newest first, so it shows straight
below CC Corporation's two. It is Helba's "Old Worlds" (her photo,
`fromNO` 17), with its text in `piney_desktop::extras`. It takes
mail number 511, the save's last slot (Infection's mails are 0-325), and
the mailer's table is filled to 512 with empty rows. The game reads a
save's mail order list as indexes into its own 326-row table, so a card
holding this mail is for the port.

## News

`Web_control` (webnews.cpp). The headlines are listed once, by
`AddHtmlList` (0x0040d5c0) from `AddAllList`: every `HtmlTbl` entry (69,
then a "NULL" title) whose `webnewsList[i]` (+0x2864) is non-zero, in table
order. 0 is not posted, 1 posted (event opcode 111 `news_add`; 112
`news_remove` sets 0), 3 read; a 3 sets the table's `flg` to 3, anything
else counts in `NewWebNum`, the second NEW mark. Infection's scripts post
each of 0-68 once.

`MainWebNews` (0x0040d1d0) by `Webmode`: 0 enter (an empty list waits for
cancel; otherwise `WebDataSet`, nothing drawn), 1 the list, 2 loading, 3 a
page, 4 leave: one frame of only the scroll bar, then `ResetWebNews` and
-1. The cursor starts at the top on every entry.

**The list** (`WebList`, `WriteList`) on the kanji layer framed
`SetFrame(116, 140, 364, 180, 182, 90, 1, 6/7)`: 12 rows from `MinMax`, row
r's title at (44, 5 + 16.5 r) without a shadow, `#G` on the selected one;
the "NEW!" cell (0, 55) of `TEX_xddcurs1`, 25 x 9, at (12, 7 + 16.5 r)
while `flg` is 0; the big cursor at (0, trunc(5 + 16.5 row) + 3), blinking
as the mailer's. The scroll bar (`SetLength`) at x 356, 7 wide, length
`min(181, 181 * 12 / n)`, y `12 + (181 - len) / (n - 12) * MinMax`, or its
end on the last headline. Up and down move as the mailer's list with
`CurRepeat` (30, then 9; se 6), wrapping; it scrolls down past row 8 and up
above row 3; a cancel push also resets the repeat count. OK: se 4,
`enableReset` 0, `Webmode` 2; cancel leaves (se 7 from ChooseMode).

**A page** (`setPage`, `LoadHtml`): the first loading frame starts the
task "PAGE LOAD" on the headline's file list (`xddn_NNN.CCS`); the next
frame that finds it loaded gives a one-packet mask the texture
`TEX_xddn_NNN` (512 x 512, 256 colours), sets `Webmode` 3 and
`enableReset` 1, and draws the page in the same frame as the list. The
port loads at once, so the page shows two frames after OK. `PreviwWeb`
cuts texel rows `Hi .. Hi + 206`, 353 wide, drawn at the frame's origin
1:1. `WebMove` scrolls on held up / down (the direct bits): `Hi` moves 6 on
held frames 1, 4, 6, 8 ..., clamped to `0 .. hight - 206` (0 when the page
is no taller than 206). The page's bar (`SetWebLength`) at x 357, 5 wide,
length `180 * 206 / (hight - hight mod 6)` (180 for a short page), y
`12 + (180 - len) / trunc((hight - 206) / 6) * trunc(Hi / 6)`, or its end.
Cancel: se 7, `PreMode` 3; that frame still draws the page, the next draws
nothing, the third (`DeleteWeb`) marks the headline read
(`webnewsList[No] = 3`, `flg` 1, `NewWebNum - 1` if it was new) and returns
to the list, which draws again on the fourth. While `PreMode` is 3,
`DrawWebnews` draws the page animation twice.

`SetData` also builds a `ccAnm` of `ANM_xddwebn0` that nothing draws.

## Accessory

`Acces_control` (acces.cpp): the wallpapers. The list is built once, by
`AddWallList` (0x004044f0) from `AddAllList`: `WallTbl[k]` for each bit k of
`dtWallpaperList` (+0x2238, bit k in word k >> 5) set, k 0-48 in table
order, then `ORIGINAL_WALL_1..3` (49, 50, 51) always. Event opcode 167
(`desktop_item 0 id`) and `BOOK::BookAddItem` set the bits; Infection's
only script use is event 271, bit 49. `CheckPhantomWall` looks for an
`AnmName` starting `ANM_xddwal49_a`, which no Infection `WallTbl` entry
has, so the phantom overlays never draw.

`MainAcces_control` (0x00403c40) by `Accesmode`: 0 enter (an empty list
waits for cancel; otherwise `SetData`, nothing drawn), 1 the list, 3 leave:
one frame with nothing drawn, then `DelData`, `ResetData` (the cursor back
to the top) and -1.

**The list** (`WallList`, `WriteList`) on the kanji layer framed
`SetFrame(180, 130, 192, 178, 96, 89, 1, 6/7)` (left so after leaving): 9
rows at (32, 3 + 17 r) with the drop shadow, `#G` on the selected one,
whose comment's two lines (artist, caption) show at (18, 165) and
(18, 186); the big cursor at (20, 10 + 17 row). `ListMove` is the mailer's
with 9 rows, scrolling down past row 6 and up above row 2. The bar and a
rule go on a layer of their own (`ScrLayer`, priority 129, the same frame
with `ay` 1): the bar at x 187, 5 wide, `min(156, 156 * 9 / n)` long, y
`12 + (156 - len) / (n - 9) * MinMax`, or its end when `MinMax` is at the
bottom; the rule 171 x 2 at (12, 135). Frame order: the list, then
`DrawAccess` (the desktop and `ANM_xddpage4`), then the two masks.

**Choosing** (`ChangeWall` 0x00404c70): OK plays se 4 (also while a change
runs) and, unless one is running, sets `enableReset` 0 and starts it. On
the wallpaper already up it ends at once (`enableReset` 1). Otherwise the
task "WALL LOAD" loads `WallTbl[k].Wall`; the frame that finds it loaded
sets `ScrBack` to the new file's `AnmName` with one step and writes
`dtWallpaper = No`; the next frees the old file, and `enableReset` is 1
again. The port loads at once, so `dtWallpaper` changes the frame after
OK. While a change runs the cursor and cancel are ignored.

## Audio

`Audio_control` (audio.cpp): a menu, Sound Mode (the desktop music) and
Movie Mode (the movies), and a list for each. The lists are built once
from `AddAllList`: `AddWaveList` (0x00406450) takes `Wave[i]` for each bit
i < `MAX_WAVE_NUM` (50) of `dtBgmList` (+0x2244) set, then `Wave[50]`
("Original 01") always; `AddStrList` (0x004065b0) takes `Stream[i]`
(0x0041b640, 89 movies) for each bit of `dtStrList` (+0x2250), with its
volume: 1 below 18, 2 below 42, 3 below 63, else 4 (0x0041bbc0). A movie
plays only when `saveData.clearFlag` (+0x842a, read as the screen opens)
is at least its volume.

`MainAudio_control` (0x00405700) by `Accesmode`: 0 enter (`SetData`,
nothing drawn), 2 the menu, 1 the music list, 4 the movie list, 5 a movie,
6 leave (one frame with nothing drawn, then -1). The kanji layer is framed
`SetFrame(130, 131, 305, 153, 152.5, 76.5, 1, 6/7)` on entry (left so);
the scroll bar goes on its own layer, priority 129, framed
`SetFrame(130, 131, 155, 153, 77.5, 76.5, 1, 1)`.

**The menu** (`SelectMode` 0x00406770): "Sound Mode" at (34, 60), "Movie
Mode" at (34, 95), `#G` on the one under the cursor (`AccsMode`, kept
between visits), the cursor at (21, 65) or (21, 100), and two lines of
`STR_STR_LOCK` at (165, 140) and (165, 157): "Change desktop music." for
Sound; for Movie "Watch movies you've obtained." when the clear flag
reaches `volumeNum` (1), else "Cannot be used in Parody Mode." in parody
mode, else "Clear the game for Vol.1 movies." (`dec2sjis`, which writes
ASCII digits). Up and down (se 6) wrap; OK opens the list (se 4, the
cursor at the top) except Movie in parody mode (se 20); cancel leaves.

**The lists** (`WriteList` 0x00407db0, `WriteStrList` 0x00407860): 10 rows
at (32, 5 + 17 r) with the drop shadow, `#G` on the selected one, whose
comment's two lines show at (165, 140) and (165, 157); a movie the save
cannot play is drawn in `ccSpriteColorTable[0]` (72, 72, 72) and, when
selected, shows "???" / "Clear the game."; the cursor at
(20, 11 + 17 row). `ListMove` (0x00408050) is the mailer's with 10 rows,
scrolling down past row 7 and up above row 2, and does nothing but clear
the repeat while `BgmRead` runs. The bar (`SetLength` 0x00407730) at x 147,
5 wide, `min(138, 138 * 10 / n)` long, y `8 + (138 - len) / (n - 10) *
MinMax`, or its end on the last entry. Cancel goes back to the menu (se 7).

**Choosing music** (`ChangeWeve` 0x004072c0): OK plays se 4; on a piece
not playing it starts the task "LOAD_BGM", `ccSndChangeData(&Wave[n],
NowNum)` (the old piece, where the startup call passes -1), sets `NowNum`
and `dtBgm` (+0x2237) to `n` and `enableReset` 0; the frame that finds the
task done sets `enableReset` 1 and ignores input. The port's load is
immediate, so that is the next frame. Nothing marks the piece playing.

**A movie** (`StartStream` 0x00406f90, mode 5): OK on a playable movie
starts `EntryFlash3(30, 1, 1, 0x80000000, ...)` (taken here as a fade to
black over 30 frames) and `enableReset` 0; 30 frames later
`ccSndMoviePlayer(0, NowNum)`, `SetFrameRate(2)`, every task asleep while
`SimplePlayStream(StrNum)` plays, then the reverse and
`EntryFlash(30, 0x80000000, ...)`, a fade from black; after 30 more frames
the list takes input again and `enableReset` is 1. OK on a locked movie
does nothing. No save data is written.

## Data

`SaveData_control` (savedata.cpp) with the memory card task `ccSaveSys`
(sdmng.cpp, in main): saving the game ([the save](../formats/save.md)).
`Desktop_control`'s constructor builds the screen once; `ccSaveSys` is the
game's one, built at boot, so its `port` and `fileNum` (the card slot and
file used last, by the title screen's load too) carry in: the card slot
cursor starts on `port` (`ResetData` 0x0040fab0) and OK on a card slot puts
the list cursor on `fileNum + 1`.

ChooseMode's entry calls `StartReq(3)` (0x00171810): the task
`ccThSaveSys` (priority 20: `Breath(1)`, then `MainProccess`, each frame),
`result` 4, `proccess` 0, `operate` 3; leaving calls `EndReq`. The screen
and the task talk through `result` (+0x2a8): the low 12 bits index
`saveSysMsg` (0x00307040, 64 pointers `__sinit_sdmng.cpp` fills; 0-4, 14
and 15 are null), 0x1000 a message to acknowledge (the button shown),
0x2000 an error to acknowledge (the task stops polling), 0x8000 a YES / NO
question, 0x10000 an operation running; plain 0 is back to the card slots,
1 done, 2 read the index again, 4 working. `proccess` (+0x2ac) is the
task's next step: 2 the card slots (`SlotSelectReq`, result 13 "Select
MEMORY CARD slot."), 3 read the index (`LoadInfoReq(pn)`, after `InitInfo`,
which clears every record but its name), 10 the list (`SaveSelectReq`, 25
"Select a place to save."), 11 check the slot (`SaveDataReq(fn)`), 12 and
13 write, 14 format, 15 make the directory, 1 idle.

`MainProccess` (0x00171c20) does nothing while `proccess` is 0 or 2, an
error is up, or the result's message is 0 or 1. Otherwise it asks
`ccMcard::CheckPort(port, 0)`: no card gives 0x2005 / 0x2006 by port.
Then it waits (`proccess` 1) unless a message or question is up, the
message is 32, 33, 36 or 37 (formatting, creating), or the card is ready;
otherwise not a PS2 card gives 0x2007, unformatted 0x100a, full (no
directory, under 685 KB free) 0x2008, no directory 0x1032. Then by
`proccess`:

| proccess | does | result |
| --- | --- | --- |
| 3 | `ReadSys` the index `<dir>/<dir>` into 336 bytes of 0xff, `CheckRightInfo`, copy it in if good | 1, or 0x1032 when it cannot be read |
| 11 | the slot's record | empty 0x801e "Create new data?"; used 0x801f "Overwrite data?", or 0x102a (clear data lost) when its clear flag reaches `volumeNum` and is not this save's |
| 12, 13 | the slot's record from `saveData` (status 1, `spcParam[0]`'s level, the name `strcpy`'d from `spcParam[0].base.name`, which `LoadGame` points at `plName`, play time, clear and parody flags, the 16-bit sum of the 0x8530 bytes), `SaveSys` (the index), then `DataWrite` (the slot file); `gameCntStop` holds the play-time clock meanwhile | 0x101c "Data saved." or 0x201d |
| 14 | `Format` | 0x1022 or 0x2023 |
| 15 | `MakeDir`, `InitInfo`, `SaveSys` | 0x1026 or 0x2027 (a failed `SaveSys` there is tested for 5, which it never returns) |

`NextProccess(sel)` (0x00173f90, jump table 0x0034cf90) answers the
message up. Acknowledging 5-7, 9, 24, 29, 35 or 39 goes back to the card
slots; 8 "not enough space" leads to 9 "685KB is needed"; 10 "not
formatted" to 11 "Format ...?", whose YES formats (0x10020 + port); 34
"Formatted" and 38 "Save data created." read the index again; 50 "no
saved data" leads to 51 "Create new save data?", whose YES makes the
directory (0x10024 + port); 30 and 31 YES write, NO gives 1; 42 leads to
43, whose YES writes; any other (28 "Data saved.") gives 1 with
`proccess` kept.

`MainSaveData_control` (0x0040ede0) by `SaveMode` (+0x14): 0 enter, 1 the
card slots, 2 waiting on the card, 4 and 5 card slot 1's and 2's list, 6
saving, 7 leave (the next frame `ResetData`, `DelData` and -1; ChooseMode
plays se 7), 8 reading the index again after a save. Every state shows the
result's message, if any (`InfoWindow` 0x00411d80): up to four lines, to
the first empty one, at (177, 240 + 16 n) on layer 129; while it waits for
a push, the button (`DispButton` 0x00410430, cell (448, 0) 16 x 16 of
`TEX_xddcurs1`) at (297, 256 + 16 lines), pulsing about its centre: `c` =
`count mod 48` reflected at 24, `b = trunc(16 + 112 c / 24)`, scale
`0.9 (0.9f + (128 - b) / 600)`, the product in double precision.

**The card slots** (`SlotSelect` 0x004106e0): the big cursor at (112, 139)
or (112, 175); up and down (se 6, `CurRepeat` 30 then 9) wrap; OK (se 4)
reads that card's index; cancel leaves.

**Waiting** (`SlotStateData` 0x00410a00) follows the result: 0 back to the
card slots, 2 read again, 1 the list. A message waits for OK or cancel
(se 4, `NextProccess(0)`); "Formatted" and "Save data created." play
se 74 once. A question shows the YES / NO window.

**The list** (`Slot_control` 0x00410c50), while the result is 4 or 25:
`ListMove` (0x00411ae0: up and down wrap over 1-12, se 6) and the slot
panel (`SetSavePar` 0x00410f80, blank for the frame after each move) at
x 177 on layer 129: "DataNN" at y 153, then at 169 "Vol.N Data Flag"
when the clear flag reaches `volumeNum` or " Parody Mode" in parody mode,
"Lv. LL" at 185, the name at 201, "Time HHH:MM:SS" at 217 (`dec2sjis`: NN
and LL zero-filled, HHH space-filled; H = t / 216000, M = t mod 216000 /
3600, S = t mod 3600 / 60), in `ccSpriteColorTable[6]` (yellow) for clear
data, [18] (red) in parody mode, else [7]; an empty slot shows "Unused" at
(177, 169). The cursor at (114, trunc(3 + 11.3 + 125 + 11.3 listNO)).
Messages here take OK only. OK on a slot (se 4) asks `SaveDataReq`;
cancel (se 7) goes back to the card slots.

**Saving** (`SaveData` 0x00411720) shows the question with the slot's
panel, or the message; "Data saved." plays se 74. Its OK keeps the record
as it was and the message, and `ReCheckData` (0x0040fa50) shows them while
the index is read again; then the list comes back, its cursor kept.

**The question** (`DiaWindow` 0x0040fc30): "YES" at (254, 276) and "NO" at
(254, 292) on layer 128, `#G` on the one chosen (NO first), the small
cursor at (239, 278) or (239, 294), over a window of 20 x 7 tiles of 8 at
(204, 260) on the kanji layer; all fade in together, alpha 0 to 128 by 8 a
frame. Up and down (se 6) toggle; OK (se 4) answers, cancel (se 7) answers
NO.

The pages (`DrawBack` 0x0040fb00): `ANM_xddsel60` for the card slots,
`xddsel61` / `62` for slot 1's / 2's list, each stepped once in `SetData`
and drawn still; then `DrawData` draws the desktop, the logo and
`ANM_xddpage6`. The big cursor blinks as the mailer's, held at 128 while a
message, question or operation is up. The kanji layer is framed
`SetFrame(0, 0, 512, 448, 256, 224, 1, 1)` (and left so), layer 128 the
same, layer 129 with `ay` 6/7.

The port's card calls (`card.rs`: a `MemoryCard` trait the runtime
installs with `Desktop::set_card`, and `FilesCard`, a directory per card
slot in Infection's layout without the icons) complete at once, so the
frames a console spends on "Saving....", "Formatting...." and "Creating
save data." (with a `ccMenuWindow` page cursor under the text) do not
happen. `tools/test_desktop_data_rs.py` runs the game's `SaveData_control`
and `ccSaveSys` frame by frame against the port over scripted cards.

## Setup

While `ccSetupDesktop` (0x00168320) runs the event passes at phases 0 and
2, before the desktop's tasks exist, the screen is the black clear
(`ccSys +0x18` 0; no loading display, no fade). On a new game event 1
draws on it: its setup lines, then name entry. A setup `message` (the
event's code at 0x001a930c) makes its own layer 242 (framed as the menu
layer), a fresh `ccMessage`, and `Change`s it (the reopen branch: both
statuses 1 from alpha -24, `waitCnt` 10, at (39, 328)); after each of its
breaths the event itself calls `Disp`, and from its tenth frame `Check`,
so in a frame `Disp` comes before `Check` (on the desktop the other way).
There is no dim. When `Check` answers (it closes the window itself for a
plain line, sound 18), `Disp` goes on for 15 frames (the fade out takes 4),
two frames draw nothing, and the event deletes the window; the next line
starts after 14 black frames. `info` and `info_now` at setup are the same
with `ChangeInfo` and 8 frames after the answer. The docs' order of setup
is corrected here: the screen is cleared before phase 0, and the files load
after phase 2.

Until its `SetFrameRate(1)` the setup runs at the frame rate the mode
before left: 1 from the title or the board, 2 (30 frames a second) from
The World. Event 4's block 9, in the pass at phase 2 after its `mode 3`
from the dungeon, plays streams 4, 5 and 6 so, and its own `mode 3` at the
end makes `ccSetupDesktop` return with the rate still 2; the next setup
(event 10's lines on the setup screen at phase 0) runs at 2 as well, until
its own `SetFrameRate(1)`. `crates/piney-game`'s `DesktopMode` carries the
rate the session hands it (`carry_frame_rate`), sets 1 where the game does
and stops at a pass that asks for another mode, building no desktop.

## Name entry

`NameEntry_Control` (NameEntry.cpp, desktop.prg 0x00414310-0x0041b5b0) is
the new game's name entry. Event 1 opens it in the setup pass at phase 0
(`name_entry`, `ccEvent::Execute` case 156, 0x001b2070): `new` (0x268
bytes) and `Init` (0x004195b0), then each frame `ccBreathThread(1)` and
`Main` (0x00414310) until `Main` returns non-zero, then
`ccBreathThread(2)` and the destructor. `Main` returns 1 at once while
`newGameFlag` (+0x6770) is set. Otherwise, each frame:

```
ChangeACT            ReserveACT -> MainAct; cursor, CurAct, ALL_Lock 0
MainAct's function   (jump table 0x00461020)
MoveCur; NameSelSet  input; TempName = the name with its next place blinking _ / space (every 31 frames)
unless ALL_Lock: JudgmentProcess, SetBufBlock, SelName
AllTlans             the fade and the alphas
DrawCur; KanjiDraw; DrawWindow; return 0
```

The US build keeps `NowMode` at 2, the English grid, and reaches:

| MainAct | does |
| --- | --- |
| 0, 1 | pages 0 and 1 (`InfoMsg`); a decide (se 18) fades the page out, `InfoCount` + 1, and the next page fades in; after page 1, the grid (`m_Max` (14, 5)) |
| 2, 3 | `SetHira`, `PlayHira`: two frames of the kana grid, empty here, then `ReserveACT` 6 |
| 6, 7 | `SetEigo`, the grid |
| 14 | `SetNameTwo`, after the user name's Enter: `TrueSurName`, then the character name (`NameAct` 2, `StrMaxNum` 14) |
| 16, 17 | `SetNameThree`, `PlayNameThree`: the dialog; YES goes on, NO or cancel goes back to Enter |
| 18, 19 | pages 2-4 |
| 20 | `SetSaveData`: `strcpy(plName, EntryGameName)`, `strcpy(plRealName, EntryName)`; return 1 |

`MoveCur` reads the pad only while `Play_Lock` and `Move_Lock` are not 1
and the fade is still. Keys go through `CurRepeat` (0x0041a4e0): a push
counts at once; a held key (the repeat bit) counts when a counter shared by
every key reaches `RepeatCount`, 30 and then 9; an unpush puts it back to
30. On the grid, directions move (se 6) and X wraps over 0-14; off the top
or bottom goes to the menu-row item under the cursor's block (Default, One
Back, Enter); a decide enters the character under the cursor (se 4),
cancel deletes (se 7), START jumps to Enter. On the menu row, left and
right wrap (se 6, clearing `Err`), up or down goes back to the column the
cursor left from or the middle of the item's block. In the dialog, up is
YES and down NO (se 17); a decide is se 18, cancel se 19.

The grid is `EigoBlock` (0x0042c7e0): 18 rows of seven two-byte cells,
three blocks of five across and six rows down; row `NO = X / 5 + 3 Y`, cell
`X mod 5`. `CutInBlock` (0x00419390) shows the row as `prefix #G cell #W
rest` and copies the cell into `m_cbuf`. `SelName`: a decide puts the cell
at `NameNum` (at `StrMaxNum` the cursor goes to Enter); cancel puts a `_`
back; Default sets the user name to 18 `_`, or the character name to
"Kite" padded to 14 with `NameNum` 4. The user name holds up to 18
characters, the character name 14; `NameNum` is
`StrMaxNum - strlen(strstr(name, "_"))`. `JudgmentProcess` (0x00418960):
Enter with nothing entered gives `Err` 1; `CheckName` (0x00418db0)
lowercases, drops spaces and looks at 19 characters: nothing left is `Err`
1, a character name equal to one of the 21 at 0x0042c6c0 is `Err` 3; an
accepted user name fades the grid in again, an accepted character name
fades out to the dialog; One Back on the user name is `Err` 2, on the
character name it goes back to the user name. `AllTlans` (0x00415830)
steps the fade by `Tlans` (the alpha 10 a frame); the name frame's
`winTrans` pulses 5 a frame while the screen is in.

Everything draws on layer 131 (`ccLayer::Init(131, NULL)`, the default
view): `ccKanji` `Init(3, 16)` texts in `ccSpriteColorTable[7]` with the
shadow (`Info` and `NameInfo` kt 2; the grid rows, names and `RealName` kt
3; the rest kt 0) and two `ccMenuWindow`s on the window texture; in a
frame `DrawCur` (the cursor), `KanjiDraw`, then `DrawWindow` (the face
panel, the name frame, the windows).

- **Pages**: `DispSquare(25, 2)` at (71, 156), the "next" arrow at
  (260, 210), the page's lines centred about x 256 at y 170 / 190 (one line
  at 180).
- **Grid**: `DispSquare(15, 4)` at (224, 10), `(27, 7)` at (56, 131),
  `(27, 2)` at (56, 301); row `s` at (88 + 120 (s mod 3), 141 + 20 (s / 3));
  the menu row at (124, 270); the prompt or the error at (77, 315) and
  (77, 335); the labels at (234, 24) and (234, 69) with `#G` on the one
  being entered, the names at (234, 44) and (234, 84); the name frame (16 x
  16 cells from (0, 3456)) at (228, 20) 200 wide or (228, 65) 160 wide; the
  face panel at (50, 14); the cursor's wing only
  (`DispSelectCursol(..., 1)`), a frame behind the highlight.
- **Dialog**: `DispSquare(17, 2)` at (136, 186), `(17, 5)` at (136, 36),
  `(6, 2)` at (205, 260); "Your Name" and the user name at (150, 50) and
  (150, 70), "Character Name" and the character name at (150, 100) and
  (150, 120), "Registering name." / "Proceed?" at (150, 200) and
  (150, 220), YES / NO at (230, 270) and (230, 290) with the full cursor.

`ccFacePanel::Draw(x, y, alpha)` (main 0x001bba30) draws the target frame
(`DispTarget(8, 1)`), the HP and SP boxes with bars `maxHP * 56 / maxHP`
wide (always full), "---/---" in the system font on the font layer (240),
and the face `xwin_f00::TEX_xwin_f18` (96 x 96) at (x, y).

`crates/piney-desktop/src/name_entry.rs` is the port;
`tools/test_desktop_name_rs.py` checks it against the game in eemu: the
logic and names every frame over random pads, and every draw call over a
scripted pass through the screens.

## The menu task

`ccDtMenu` and its task `ccThDtMenu` (main executable 0x00169b00 -
0x00171250) own the menu layer, priority 242, framed
`SetFrame(0, 0, 512, 384, 256, 192, 1, 6/7)` (one unit a frame-buffer
pixel), over everything the desktop draws. `ccSetupDesktop` starts the task
(priority 33, 0x001684f0) just before `ccThDesktop`; with the event task
(32) first, a frame runs event, menu, desktop. The task makes `dtMenu` and
with it `ccMsg`, the event windows' `ccMessage`.

Each frame (0x0016ac00): a request in `openReqNum` (+0x06) opens that menu
(`OpenMenu` 0x0016a7a0); `CheckMenuType` (0x0016a8e0) is `menu` when
`menu == menuNext`, else 12, and while it is 12 and the window is shut the
task steps `menu` to `menuNext`; then the menu's handler (jump table
0x0034c010), then `Disp`. With no menu open the handler is the START test:
`enableReset`, not the title, START pushed, `forbid` (+0x16) clear, and
`CheckOperate(11)` and `(28)` (so `operateSet` becomes 11) open menu 0.

`OpenMenu(n)` puts every other task to sleep (`ccSleepAllThread`; the menu
and event tasks carry flag 2 and run on) and freezes every other layer's
picture (`still`, `OffFlipExcept`): the desktop stands still under the menu
until the menu wakes it. It plays sound 16 unless `n` is 7. A close sets
`menuNext` -1 and the window closing, wakes every task, draws, breathes,
and only then turns the flip back on; `CheckMenuType` reads 12 for the two
frames the window takes to go, and the desktop's input tests
(`CheckMenuType() == -1`) stay shut until it reads -1. The window's alpha
climbs by 12 and falls by 14. The dim under a menu is a `ccScFade` on the
menu layer, `EntryFade(6, 0, 0x60000000, 0, 0, 512, 448)`, undone by
`ContinueFade(6, 0)`.

**On the title screen** (`ccGame.status` 1, which `ccSetupDemo` sets; the
desktop's is 2) about twenty tests in the task change what it does:
- `OpenMenu` puts nothing to sleep, freezes nothing and plays no sound 16,
  so the title runs on under the menu;
- `SystemMenu` makes no dim;
- decide and back sound 4 and 7 (the title's) where the desktop's sound 18
  and 19;
- no select cursor is drawn. The chosen row is `ccSpriteColorTable[20]`
  instead, in the lists (`Disp`) and in every option menu's page. In the
  lists the rows are 24 apart, not 20. Sound's output choice keeps its
  colours.
The title's own list is list 1 (`disp` 5): no window, rows at
(193, 152 + 12 + 24 i). See [the title](title.md#the-system-menu). Checked
by `tools/test_desktop_menu_rs.py` (the title's OPTION list and each of its
options, with status 1).

**Menu 7, `FadeMenu`** (0x0016ee20), is the event message's: no window,
only the dim, until the event sets `externFlag` (+0x10) after a plain line;
then the dim goes and the menu closes. A run of lines shares one dim.

**Menu 0, `SystemMenu`** (0x0016aeb0), START's OPTION list: the items of
list 0 (`dtMenuElementData` 0x00306f70; Controller, Vibrate, Adjust Screen,
Sound, Voiceover, Movie Text, Title Screen), each list's name padded to 16
glyphs and extracted into one `ccKanji` (kt 0), a texture row an item, drawn
as 128 x 16 sprites at (193, 152 + 20 i) in `ccSpriteColorTable[7]` with the
window's alpha; the window `DispSquare(9, 7, "OPTION")` at (179, 136) from
`xwindow::TEX_xwindo00` (grid 1, 14 x 16 cells; the title in grid 0's
letters over its top edge); the cursor `DispSelectCursol`, a bar that fades
between rows and a trail of six wing cells easing after it. Up and down
(repeat bits) move with sound 17; cancel (sound 19) undims and closes; OK
(sound 18) opens the item's menu. **Menu 6, `ResetMenu`** (Title Screen)
asks "Quit and return to Title Screen." then "Data not saved will be lost.
Proceed?" in `ccMsg` info windows over an OK / Cancel dialog
(`DispSquare(6, 2)` at (200, 248)), Cancel first; OK twice asks
`ccGame::ChangeRequest(1, 7)`, the mother task's soft reset to the title.

**The option menus.** OK on an OPTION item closes the list (alpha down by
14), then the item's menu opens (up by 12); its `prev` is the OPTION list,
to which cancel returns with the selection kept, and the dim stays on
throughout. Each draws its own page (`ExceptionDisp` 0x0016a680, by `menu`,
so also while it fades) with the window's alpha, on the menu layer's
sprites in `Disp`'s send order. Sounds: 17 a move, 18 OK, 19 back.

- **Vibrate** (3, `VibrationMenu` 0x0016cdf0), **Voiceover** (10,
  `VoiceMenu` 0x001708a0), **Movie Text** (11, `StrwinMenu` 0x00170f40): a
  window `DispSquare(6, 2)` at (200, 248), rows ON / OFF (English /
  Japanese) at (214, 264 + 20 i), the one in force in
  `ccSpriteColorTable[6]`; an information line says which (`OpenInfo`, then
  `ChangeInfo` on each OK). OK writes `vibration` (+0x8428), `voice`
  (+0x842c, 1 English) or `strWinMode` (+0x8430) as row 0 ? 1 : 0; Vibrate
  also sets `ccPad::actuaterSw` and, switched on, buzzes once
  (`SetActuater(pad, 1, 160, 200)`).
- **Adjust Screen** (4, `ScreenMenu` 0x0016d4d0): up / down move the
  picture a line (-16..16), left / right 3 units (-48..48), each frame
  `ccSystem::SetDisplayOffset` (the GS DISPLAY registers) and `screenX` /
  `screenY` (+0x841a, +0x841c); a window at (200, 188) shows "X :" and
  "Y :" with the offsets (x / 3) in `menuFont`, and twelve marks round the
  frame and its centre pulse in `ccSpriteColorTable[10]`, alpha
  `16 + t 128 / 15` over a 60-frame triangle. Cancel leaves.
- **Sound** (5, `SoundMenu` 0x0016dc80): Main, BGM and SE Volume in 32
  steps (`value * 32 / 256`) on sliders (`DispSlideBar(8, now, 32)`), and
  Output Mono / Stereo; left and right change the row the cursor is on,
  and every change writes `mainVol`, `bgmVol`, `seVol` (+0x841e-0x8422,
  `step << 8 / 32`) and `output` (+0x8424) and calls
  `ccSaveData::SetSoundEnv`.
- **Controller** (2, `ControllerMenu` 0x0016b320): loads `XCONTROL.CCS`
  (the pad picture, drawn whole at (88, 168)), lists A-1, A-2, B-1, B-2 in
  `DispSquare(6, 4)` at (39, 56) with the one in force in yellow, and the
  scheme's help in an information window moved to (39, 444). OK sets
  `camType` (+0x8429) and `setCameraCtrlType` (the field camera's
  direction flags). It does not reassign buttons. Round the picture,
  labelled boxes as the field's Controller page draws them
  ([field UI](field-ui.md#controller-controllermenu-0x0053d0e0)): the
  buttons, START and SELECT, the two sticks and the shoulder buttons, the
  camera's labels and rotation arrows following the scheme.

A new game has vibration, voice and the movie text on, the volumes at 256,
and stereo (`ccSaveData::Init`).

`tools/test_desktop_menu_rs.py` runs the game's `ccThDtMenu` in eemu
beside the port over pad scripts through the OPTION list, every option
menu and Title Screen: the state, every packet, the sounds, the
information lines and the requests agree frame by frame, the Controller's
boxes and their labels (the setting kanji) included.

### The save menus after the staff roll

When [the staff roll](#the-staff-roll) is over, the `staff_roll`
instruction waits 30 frames and calls `SetFrameRate(2)`. Unless parody
mode is on, it then asks for menu 8 and waits until `CheckMenuType` reads
-1. Every other task has slept since the roll began, so the menus run over
black at 30 frames a second. At rate 2 the select cursor moves in bigger
steps: `DispSelectCursol` divides the bars' fade by `3 - rate`, the wing's
easing by `(10 - k)(3 - rate)`, and the lead wing's by `4.5 / rate`.

**Menu 8, `SaveSelMenu`** (0x0016ef90). The OK / Cancel dialog
(`DispSquare(6, 2)` at (200, 248)) sits under an information window.
Every frame, including the one it closes on, the menu calls `DispInfo`
with `clearFlagStr[volumeNum - 1]`: "You now have the Data Flag for" /
".hack//INFECTION." / "Save?".
- OK on OK (sound 18) opens menu 9, its `prev` 8. Its `InitCursol` gets
  the 1 still in `$a1`: the delay slot sets nothing.
- Cancel, or the cancel button (19), closes.

**Menu 9, `SaveMenu`** (0x0016f2c0) runs `ccSaveSys` from `StartReq` to
`EndReq`, the steps the [Data screen](#data) takes, but draws them in the
menu window. `StartReq` is given 12, the `$a1` that `ccThDtMenu`'s
`InitCursol` call left behind. `ccSaveSys` tests `operate` only for 1 and
2 (the title's loads), so this saves as the Data screen's 3 does. The menu
keeps its own state in list 9: +0x0a the selection, +0x0c the file, +0x10
the card slot (both start from `ccSaveSys`'s `fileNum` and `port`), and
+0x18 the last result seen. dtMenu's `waitCount` (+0x0c) holds input for 10
frames after each change.

| proccess | waits for | then |
| --- | --- | --- |
| 0 | - | `StartReq`, the window shut |
| 1 | the window shut, the wait over | `SlotSelectReq`: the card slots (`exceptionDisp` 1) |
| 2 | a push | OK: `LoadInfoReq(slot)`, 3; cancel: 100 |
| 3 | result 1 | 4 |
| 4 | the window shut, the wait over | `SaveSelectReq`: the files (`exceptionDisp` 2) |
| 5 | a push, while the result is 25 | OK: `SaveDataReq(file)`, 6; cancel: 1 |
| 6, 7 | result 1, the wait over | 4, the file cursor kept |
| 100, 101 | the window shut | `EndReq`, then the close |

Below 100, the menu then reads the result:
- 0 goes back to the card slots (`NextProccess(0)`), and 2 reads the index
  again.
- The first frame of "Data saved.", "Save data created." or "Formatted"
  (28, 38, 34) plays sound 74.
- A message (0x1000 or 0x2000) closes the window. After the wait, OK
  (sound 18) answers `NextProccess(0)`.
- A question (0x8000) opens the OK / Cancel dialog with Cancel chosen. OK
  answers `NextProccess(1 - selection)`, cancel `NextProccess(0)`.

With the wait over, the message goes to `ccMsg`:
- A message, a question or an operation goes to `DispInfo` (up to four
  lines, the empty ones null), with `ccMsg +0x24` 1 (the button) for a
  message and 3 for an operation.
- A plain line (13 "Select MEMORY CARD slot.", 25 "Select a place to
  save.") goes to `DispMsg` in the speech window while the menu window is
  up.

**`SaveMenuDisp`** (0x0016fe70) draws what `exceptionDisp` asks for, with
the window's alpha.
- **The card slots:** `DispSquare(11, 2)` at (179, 136), each row
  "MEMORY CARD" at (193, 152 + 20 i) and "slot 1" / "slot 2" at
  (289, 152 + 20 i), from `recordMenuStr`.
- **The files:** `DispSquare(24, 12)` at (74, 80), each file on a row at
  y = 96 + 20 i. The number and ":" are in `menuFont` (12 wide) at x 88.
  The name, padded to 16 glyphs, is in the name `ccKanji` (+0x30, eight
  rows each) at x 130. "LV" and the level are at x 256, and the play time
  "HHH:MM:SS" at x 312 ("999:59:59" from 0x0cdfe5c4). An empty file shows
  "Unused". The colours are the Data screen's: [6] for clear data, [18]
  for parody, else [7].

`tools/test_desktop_savemenu_rs.py` runs the game's `ccThDtMenu` and
`ccSaveSys` together in eemu over scripted cards and pads, beside
desktop_probe's `savemenu`. The run covers saving to new and used files,
the clear data warning, NO answers, both card slots, no card, not a PS2
card, unformatted (formatted and not), no directory, full, failing
writes, frame rates 1 and 2, and random pads. Every frame agrees: the
menu's and the task's state, the sounds, the `DispInfo` and `DispMsg`
lines, every window and kanji packet, and `menuFont`'s strings with their
colours. At the end, so do the files on the cards. piney-game's
`ending_saves_the_clear_data` plays story:31 through the staff roll and
saves to an empty card. It then reads the card back: the first file holds
the clear flag and the index records it.

## Messages

`ccMessage` (main executable 0x001a4580 - 0x001a60e0) on the menu layer.
The event's `message` asks for menu 7 when no menu is open, then
`Change(record, name, event, msg)`; `info` and `info_now` use
`ChangeInfo(l0, l1, l2, NULL, event, msg)` with empty lines as nulls
(`info_now` then zeroes the window so only the text shows). Both start the
voice (`ccEvVoiceRequest(event, msg)`). The event then polls `Check(0)`
once a frame; `ccDtMenu::Disp` calls `Disp` every frame.

**`Change`** (0x001a54a0) reopens the speech window (type 5) unless one is
up, from alpha -24; the text always restarts from -24, `waitCnt` 10.
`SetMsg` (0x001a51b0): line 0 the name (whole), lines 1-3 the record's,
each's glyph count by `ccKanjiStrlen` (`#0`, `#1` counted as the names; 73
or more drops the line); typing starts on line 1 with a name, else line 0.
**`Disp`** (0x001a4730): each status fades in by 24 to 128 or out by 28;
the speech types one glyph a frame (an empty or unnamed line costs a
frame); the window without a name is `DispSquare(29, 3)` at (39, 352),
with a name `DispSquareName(29, 3)` at (39, 328) (a header of the name grid
over a taller body); the name at (67, 334), line i at (67, 342 + 21 i) in
kt 2, `ccSpriteColorTable[7]` at the text's alpha, the typed prefix only.
The information window (type 6) is `DispSquare(29, n)` at
(39, 208 - 20 n), its bottom fixed at 236, lines centred at
`256 - width / 2`, y `220 - 20 n + 21 i`, shown at once. While the window
waits (`cursol` 1 or 2) the "next" arrow, cell 24 of grid 1, grows and
drops with an alpha climbing by 8 and falling by 4, under the text at
(256, 420) (info (256, 220)).

The scripts' announcements (`member_add_msg`, `gate_add_msg`,
`desktop_item`; [the event interpreter](event-vm.md#announcements)) come
through `ccEvent::DispInfo` (0x001b27b0): after sound 74 it waits until
`CheckMenuType` is -1 or 7, asks for menu 7 when `dtMenu +0x00` is -1, and
calls `ChangeInfo(l0, l1, l2, NULL, 0, -1)` with the lines `Execute`
composed (no voice); after 5 frames it polls `Check(0)`, then `Close`,
`dtMenu +0x10 = 1` (the fade menu closes) and 10 frames. A gate address is
two lines, "#B" with the server's letter and the area's three words, then
"#W is added to the Word List."; on the board this is how events 14, 50
and 55 give areas 17 and 19, 28 and 29.

**`Check`** (0x001a5960) reads nothing for its first 10 calls. The decide
button while typing shows everything (and stops the voice); once all is
shown it closes two calls later: emode 0 fades the window out and returns
1 (sound 18), emode 2 returns 1 and leaves the window up, emode 3 (a
question; answers are lines 2 and 3, moved with up and down, sound 17)
closes and returns the answer 1 or 2. Cancel is never read. Infection's
records are emode 0 and 2 only; the question's selection bar is not
ported.

## The seam to the event scripts

| `ccSaveData` | member | the desktop |
| --- | --- | --- |
| +0x0000, +0x0018 | `plName`, `plRealName` | `#0`, `#1` in text |
| +0x2236, +0x2237 | `dtWallpaper`, `dtBgm` | the wallpaper and music; a new game has 49 and 50 |
| +0x2238, +0x2244, +0x2250 | `dtWallpaperList[3]`, `dtBgmList[3]`, `dtStrList[5]` | unlocked wallpapers, music and movies |
| +0x2264, +0x2464 | `mailList[512]`, `mailOrderList[512]` | the inbox (above) |
| +0x2864 | `webnewsList[128]` | News (above) |
| +0x840e, +0x8410 | `assignPADok`, `assignPADcancel` | cross 0x40 and circle 0x20 after `Init` |
| +0x842b | `parodyFlag` | `MailTblp` and the other parody tables |

and `ccEvent.operate` / `operateSet` (+0x770, +0x778) above. The gate instructions (`gate_add`, `gate_add_msg`, `gate_mark`,
`gate_unmark`) read the story area's server and words from `eventAreaInfo`
(`Host::story_area`), on the board as in the field.

## The staff roll

The event instruction `staff_roll` (case 147, the ending's) starts
`ccThStaffRoll` (0x004125c0) on the desktop and waits for it; the menus are
forbidden until `staff_roll_done`. After the roll come the
[save menus](#the-save-menus-after-the-staff-roll).

```text
ccThStaffRoll        stfroll1 (the volume's, table 0x0042bbf0); ccThStaffRollCtrl;
                     ccBgmPlay(1) (VOICE/BGM.BIN track 1); Breath(2); then Main a
                     frame until done (+0x13a8); ccBgmStop
ccThStaffRollCtrl    layer 200, its view SetFrame(0, 0, 512, 384, 256, 192, s, s)
                     and SetLayerCenter(256, 192); srand(ccSys+0x358); 19 lines;
                     a ccMask for the page picture; status 3, page 0
ChangePage(p)        scale 1 on the first and last page, else 5; alpha 0; past the
                     last page: done; else the picture SetTex(stfroll1, tex, 1),
                     picture alpha 0, status 0 and BuildPage for a page with
                     credits, else 3
```

`g_srDataGrp` (0x0042c500) is 28 pages until a row of three zeros: a
credit list (null for a picture alone), its length, and the picture
(`TEX_xdsbac00`-`26`). A credit (`ccSRData`) is a column (-1 centred, -2
right-aligned), a line (-1 is 9, -2 is 18) and its text, where
`ccTransCode2Name` (0x00412410) puts the save's names for `#0` (+0) and
`#1` (+24); another `#x` becomes a 0 byte.

Each frame `Main` runs one phase:

| status | phase | what it does |
| --- | --- | --- |
| 0 | `_Random` | scale down by 5/66 to 1, text alpha up by 1/66; each line draws 44 random characters (`ccRand`, 33-126, `#` and `%` bumped by one) over the ones settled; from the second frame each line settles one more (`StopChar`, C `rand`); all settled and alpha 1: status 1 and `Except` |
| 1 | `_Fix` | the credits drawn whole, the random characters at the text alpha falling by 1/50, the back shadows and the picture at the picture alpha rising by 1/50; text 0 and picture 1: status 2 |
| 2 | `_End` | 110 frames held, then scale, text and picture down by 1/30 each; text below 0: the next page |
| 3 | `_BGOnly` | the picture up by 1/60, 91 frames held, down by 1/96; below 0: the next page |

- **Drawing a line.** A line draws at (-262, 20 × line − 192) in colour 7
  of `ccSpriteColorTable` with the `LargeFixed` font.
- **Settling.** A settled character shows the credit's letter. Where the
  line has no credit (a 0 or a space), it freezes whatever random
  character was on it.
- **Drawing a credit.** Each credit is its own sprite at x = column × 12 −
  262, with a back shadow one pixel over in colour 15 at half the picture
  alpha.
- **`Except` looks like a bug.** It blanks the random buffer under a credit
  only when the credit's column times 12 is between 1 and 41, meaning
  columns 1-3. The port keeps it as the game has it.
- **The picture.** `MakePacket(0, 1)` of a 512 x 384 sprite at
  (-256, -192); on the last page, a 20-row strip at (-256, -7).

- **The port.** piney-desktop's `staffroll.rs` is the controller, and
  `Desktop::start_staff_roll` the task.
- **The check.** `tools/test_staffroll_rs.py` runs the game's controller
  natively in eemu over the whole roll (7,231 frames), with seed 12345 and
  the names Kite and Tester. Every frame's state, line bytes and draws are
  kept in `crates/piney-desktop/tests/staffroll_fixture.txt`, which
  `tests/staffroll.rs` replays with 0 mismatches.
- **The view.** It also records the game's `ccView` at six scales:
  `SetFrame` keeps the centre `SetLayerCenter` set, and 36 points through
  `ApplyLayerScreenMatrix` match `LayerView::frame_centred`.

## Unknown

- The save menus' operations with the card busy ("Saving....", `ccMsg
  +0x24` 3 and its page cursor): the port's card calls complete before
  the menu's 10-frame wait is over, so none is shown.
- `ccScFade`'s `ContinueFade` and `CheckFade` to the exact frame.
- How long the card calls take on a console, the busy indicator
  (`ccMenuWindow::DispPageCursol`), and whether `ccThSaveSys` runs before
  the desktop task in a frame.
- What `ccScFade` status 9 (`EntryFlash3`) draws, and how
  `ccSndChangeData` treats the old piece (47, 27 and 7 are special).
- How long a wallpaper takes to load (`ccThWallLoad`).
- How long `ccLoadFLAddOne` takes to load a news page from the disc.
- `DispInfo`'s branch before play (phase below 4: its own layer and
  window on the setup screen) is not run against the game; no Infection
  script announces on the desktop or the board before play.
- Whether `xwindow` and `xwin_f00` are resident while name entry runs,
  and what layer 131 shows in the two frames after `Main` returns 1.
- Frame counter phase: `ccSys.count` runs from boot, so the cursor blink's
  phase depends on how long the game has run.
