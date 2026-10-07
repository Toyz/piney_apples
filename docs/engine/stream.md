---
title: In-engine streams
status: partial
volumes: INF, MUT
covers: INF SLUS_202.67:0x0013af10 ccMorpher::Modify, 0x00169690 ccSetFileListTown, 0x001697a0 ccSetFileListField, 0x001698b0 ccSetFileListDungeon, 0x00169a20 ccSetFileListDesktop, 0x0013a440 ccModel::Init, 0x00348580 alphaBlendTbl, 0x001695a0 ccThExecuteStream, 0x00198da0 ccRequestLoadStream, 0x00198ee0 ccStreamInit, 0x00198fc0 ccStreamLoadPlay::RequestStrPlay, 0x00199690 searchPreLoad, 0x00198330 ReadThread, 0x00198830 DecodeThread, 0x00197f40 WaitEnd, 0x00198250 CheckPause, 0x00198220 ResetPause, 0x0019aaf0 ccGetStreamFrame, 0x0019ab80 ccGetStreamAdrs, 0x0030ef90 streamTbl, 0x003102f0 streamTblE, 0x0030b950 strSndTbl, 0x00159620 ccCdInit, 0x00143890 ccStream::Init, 0x001443b0 DecodeSetup, 0x00149e10 DecodeIndexSection, 0x001476c0 CompleteIndexChunkAdrs, 0x00147a60 CompleteChunk, 0x0014c200 Decode_Clump, 0x0014cb80 Decode_ExtObj, 0x0014c8f0 Decode_Obj2, 0x0014c5d0 Decode_Layer, 0x0014d440 Decode_Light, 0x0014ddb0 Decode_Pcm, 0x00148640 InitScene, 0x0013c5e0 ccClump::Init, 0x0013b5c0 ccObj::Init, 0x00149450 PlayScene, 0x001494c0 PlaySceneMain, 0x00149730 EndScene, 0x00149790 ResetScene, 0x00155d10 ccRingBufferTh::OpenData, 0x00155dd0 OpenBufferData, 0x00155e60 OpenBuffer, 0x0014ded0 DecodeFrameSection, 0x0014e1b0 DecodeFrameChunk, 0x0014e950 DecodeF_Obj, 0x0014ece0 DecodeF_Camera, 0x001385a0 ccCam::SetMatrix_PosRotXYZDebug, 0x0014eef0 DecodeF_Ambient, 0x0014ef80 DecodeF_DistantLight, 0x0014f6a0 DecodeF_OmniLight, 0x0014e5b0 DecodeF_Material, 0x0014e6e0 DecodeF_Morpher, 0x0014f830 DecodeF_Note, 0x0014e4a0 DecodeF_Pcm, 0x00148400 ccStreamDrawLayerList::Draw, 0x0013f220 ccObj::Draw, 0x00138380 ccCoord::_SetLWMatrix, 0x00138490 _GetTransparency, 0x00138120 SetMatrix_PosRotZYXScale, 0x00105900 ccDrawEnv::SetLightMatrix, 0x00139830 ccOmniLight::CheckRange, 0x00139170 ccDistantLight::CheckRange, 0x001389b0 ccCreateLight, 0x00139060 ccLightGrp::AddGrp, 0x001092f0 ccPcmSound::Open, 0x00109640 PlayThread, 0x0017d190 ccSndStreamCtrl, 0x0017caa0 ccSndStreamSE, 0x00184340 ccSetStreamDemoNote, 0x001979c0 ccSetStreamDemoThread, 0x0034f420 StreamDemoFuncTbl, 0x001885d0 Func_str0001, 0x0034e370 fogt, 0x0030bd80 fogOffObj, 0x00144630 ccStream::GetSubstAdrsF, 0x00101ad0 ccMatchIndex, 0x0013b970 ccObj::SetRenderState, 0x00105890 ccDrawEnv::SetFog, 0x00106170 ccRasterNoize::Init, 0x001061e0 ccRasterNoize::SetYH, 0x00106280 ccRasterNoize::SetY, 0x001062d0 ccRasterNoize::SetTex, 0x00106320 ccRasterNoize::SetNoize, 0x001063d0 ccRasterNoize::MakePacket, 0x00108a10 ccMakePacketDrawBuffTrans, 0x001089c0 iFuncDrawBuffTrans, 0x001038d0 ccGetBuffAdrs, 0x00106b20 ccBufferSampling::Init, 0x00106bb0 ccBufferSampling::SetReflex, 0x00106dd0 ccBufferSampling::MakePacket, 0x00106d20 iFuncBuffSampling0, 0x00106890 ccBufferReverce::MakePacket, 0x0015fb80 ccScFade::SendPacket, 0x00160400 ccScFade::EntryFade, 0x00160510 ccScFade::DeleteFade, 0x0034b380 ccFadeGIFtag, 0x00133a38 rand, 0x00133a20 srand, 0x0010a5f0 ccSystem::Ctrl, 0x0010ac40 ccSystem::SwapDoubleBuffer, 0x0010ad40 ccSystem::SetScreenModeMain, 0x0015a5c0 ccThControl, 0x00159d40 ccTscb::GoThread, 0x001b5670 ccEventStream, 0x003112f0 evStrMsgTbl, 0x00311510 evStrMsgTblp, 0x00184440 ccGetStreamDemoMsg, 0x0015f860 ccKanjiStrSeparate, 0x0015c210 ccSprite::Trans, 0x00189e10 Func_str0300, 0x0034e3a0 Func_str0300's cue table, 0x001950b0 Func_str0120, 0x0034f350 Func_str0120's cue table, 0x0034f320 hitRot0120a, 0x001cc620 effHitMarkStr, 0x0014f830 DecodeF_Note, 0x00106890 ccBufferReverce::MakePacket, 0x00199f00 ccRequestLoadStreamGateHack, 0x0019a0e0 ccStreamLoadPlay::RequestStrPlayGH, 0x0030e1a0 str7000TblPre, 0x0030e1d0 str7000TblTown, 0x0030e2b0 str7000TblTownC, 0x0030e390 str7000TblAfter, 0x0030f1b0 str7000Out, 0x001919f0 Func_str0090, 0x00194410 Func_str0110, 0x001961c0 Func_str0130, 0x00104ae0 ccView::SetFrame, 0x00189320 Func_str0150, 0x0018abb0 Func_str0240, 0x0018b700 Func_str0250, 0x001ce220 effTransferStr, 0x0034e3d0 hitRot0240, 0x0034e450 hitRot0250, 0x00377cf8 fogObj0150, 0x00106bb0 ccBufferSampling::SetReflex, 0x001923b0 Func_str0301, 0x001938d0 Func_str0305, 0x00192e80 Func_str0350, 0x0018c780 Func_str0570, 0x00196c00 Func_str0610, 0x0034f310 hitRot0301, 0x0034e460 Func_str0570's cue table, 0x00377d10 fogOffObj (str0350), 0x00377d18 fogObj0610, 0x0034e150 charHeightTbl, 0x00184460 Func_str8000, 0x0034e250 Func_str8000's cue table, 0x00186af0 Func_str9000, 0x0034e2e0 Func_str9000's cue table, 0x0034e200 bossSkillNameTbl, 0x00186210 Func_str9001, 0x00185e30 Func_str9001sub, 0x0034e2b0 Func_str9001sub's cue table, 0x0015c2f0 ccSprite::SetTex, 0x0015aed0 ccSprite::MakePacketStr, 0x00187ad0 Func_str7100, 0x0034e310 Func_str7100's cue table, 0x00185170 Func_str8800, 0x0034e280 Func_str8800's cue table, 0x0018e340 Func_str0580, 0x001904a0 Func_str0581, 0x0018d7e0 ccObjPart0580::Ctrl, 0x0018d8a0 ccObjPartCreate0580::Ctrl, 0x0018dee0 ccEffPart0580::Ctrl, 0x0018dfc0 ccEffPart0580::Create, 0x00184220 ccStrPart::~ccStrPart, 0x00184320 ccStrPart::Ctrl, 0x00184150 ccEventObj::SetObj, 0x0034eb60 eventObjTbl_0580, 0x0034f1f0 eventObjTbl_0581, 0x0030bda0 p0580, 0x0030be00 p0580_1, 0x0030be60 p0580_2, 0x0030bf20 p0580_3, 0x0030bec0 p0580_4, 0x00377d00 p0580_0, 0x00377d08 str0581Fog, 0x0030bf60 fogOffObj (str0581), 0x0034e490 rockScaleTbl, 0x00102190 ccRotate, 0x00106bf0 ccBufferSampling::SetShade, 0x0013f3b0 ccObj::CheckBoundingBox; INF demo.prg:0x004066b0 PlayOpeningStream; INF desktop.prg:0x00407100 Audio_control::SimplePlayStream; INF MODULES/SEWORDS.IRX:0x0800 strPcmFunc; MUT SLUS_205.62:0x0019c140 Func_str0710, 0x00187960 the text block constructor, 0x00187ac0 its fade, 0x00187bd0 SetText, 0x00366e30 eventObjTbl_0710, 0x00366ea0 its cue jump table, 0x00367210 StreamDemoFuncTbl, 0x0018c280 Func_str7100, 0x00188e30 Func_str8800, 0x0018e5c0 Func_str0300
worklog: 52, 131, 136, 376, 388, 389
---

# In-engine streams

The game's real-time cutscenes: CCSF scenes streamed from the
`STREAM/*.BIN` archives and played frame by frame through the ordinary 3D
draw, with their own PCM audio. `ccRequestLoadStream(num)` plays stream
`num` to its end inside the call. `crates/piney-stream` is the port,
checked against the game's code by `tools/test_stream_rs.py` (see
[Checks](#checks)).

## Who plays a stream

| caller | stream | around it |
| --- | --- | --- |
| `PlayOpeningStream` (demo.prg 0x004066b0) | 0 (1 with `m_ParoFLG`; 21/22, 50/51, 75/76 on volumes 2-4) | [the title](title.md#the-opening-stream): the flash at frame `410 - 20`; frame rate 1 |
| `ccEventStream(num, 1)` (0x001b5670), the event `stream` instruction | any | background colour 0 for the call; the [subtitles](#subtitles); the event task waits in the call; in a set-up's pass at phase 0 or 2 (event 30's 14 and 17) before the area's load and its [loading display](field-walk.md#the-loading-display) |
| `Audio_control::SimplePlayStream` (desktop.prg 0x00407100) | the Audio screen's movie list (`Stream[i].StrNum`) | [the desktop](desktop.md#audio): `SetFrameRate(2)`, every task asleep, background colour 0; the effects and the desktop's [common files](#the-streams-common-files) as anywhere |
| `ccSetupGameCtrl` (0x00168960) | 107 (the Chaos Gate) | plays while the field loads; `ResetPause` lets it go on |

Each starts the task `ccThExecuteStream` (0x001695a0, priority 33) with the
number and waits for its parameter to become -1; stream 107 goes through
`ccRequestLoadStreamGateHack` instead ([below](#the-gate-hacks-movie)). The frame rate is the caller's: the
title's 1, event 1 sets 2 before stream 2 and 1 before stream 106, the Audio
screen 2. The PCM agrees: 1,600 samples a frame (48 kHz at 30 frames a
second), 800 in `str6100`.

## The call

```
ccRequestLoadStream(num)            0x00198da0
  ccPcmSound::Open                  the PCM task; IOP 0x200 (channel 0)
  slp = ccStreamInit(num)           sd = (saveData.voice ? streamTblE : streamTbl)[num]
                                    strSeInit(num); new ccStreamLoadPlay; StInit
  ccSndStreamCtrl(num, sd, 0)       the music, before
  slp->RequestStrPlay(0)            below; returns when every file has played
  ccSndStreamCtrl(num, sd, 1)       the music, after
  strSeEnd; ccPcmSound::Close       IOP 0x240, 0x210
  Breath(1); free everything
```

`ccStreamInit` with `saveData.parodyFlag` set skips both tables and leaves
`sd` whatever the register held: no parody stream is meant to play.

## The tables

`streamTbl` (0x0030ef90) and `streamTblE` (0x003102f0) are 134
`STREAMDATA *` each; each list is 20-byte records ended by one without a
name. Record 0 is the header, the rest the files:

```
STREAMDATA  0x14 bytes
  +0x00 char *name     header: the stream; file: the member's stem
  +0x04 int   ofs      header: bytes into the archive; file: bytes after that
  +0x08 int   size     header: music bits; file: the gzip member's length
  +0x0c short type     header: the archive; file: see below
  +0x0e short flag     header: 1 on the title streams (unread); file: see below
  +0x10 u32   gzip     file: the inflated length
```

The header's `type` indexes `ccCd`'s file records (`ccCdInit` 0x00159620):
0 `STREAM/STRCMN.BIN`, 1 `STREAM/STR1.BIN`; with `saveData.voice` the reader
takes the record 0xb4 bytes on, `STRCMNE.BIN` and `STR1E.BIN` (the same files
with English voices; their tables' offsets differ). Types 2 to 5 are the
later volumes' archives: streams 21-105 and 120-132 do not play on this
disc. 35 do: 0-20, 47, 106-119, 133 (19, 47 and 133 are `strdummy`).

| file `type` | meaning |
| ---: | --- |
| 0 | a setup file (the models and textures: `strNNNNe`) |
| 1 | a scene another follows: its last frame stays up until the next is ready |
| -1 | the last scene |
| -2 | a scene read whole first, played from memory (`str7300`, which loops) |

| file `flag` | meaning |
| ---: | --- |
| 0x01 | the reader pauses before it until `ResetPause` (the Chaos Gate) |
| 0x02, 0x04 | a group of alternatives: 2 starts it, the parameter of `RequestStrPlay` picks one (always 0) |
| 0x10 | read whole before the scenes (`ccLoadStreamOnMem`) |
| 0x40 | START skips (else cancel) |
| 0x80 | no skip |

`RequestStrPlay` (0x00198fc0) splits the list: `searchPreLoad` (0x00199690)
reads every `type` -2 file, then every `flag` 16 file, whole into memory;
every other record (the -2 ones too) goes into the `ccsTbl`, the scenes the
decoder plays in order. `ReadThread` (0x00198330) reads file `e` at sector
`lsn(archive) + ceil(header.ofs / 2048) + ceil(e.ofs / 2048)`, `e.size`
bytes; with `gzip` set it inflates on the `Gzip` task.

## The streams' common files

A scene's `#` objects resolve against every loaded `ccStream`, so also
against the place's resident files. Three file lists hold stream files,
and each place's `ccSetFileList*` adds its own:

| place | `strcmnFileList` `STR8000E` | `datadrainFileList` `STR8001E` | `happyakuyujunFileList` `STR8800E` |
| --- | --- | --- | --- |
| town (`ccSetFileListTown` 0x00169690) | yes | | yes |
| field, dungeon (0x001697a0, 0x001698b0) | yes | yes | |
| desktop (`ccSetFileListDesktop` 0x00169a20) | yes | yes | yes |
| title, top page (0x001699c0, 0x00169aa0) | yes | | |

On Infection only these streams name objects outside their own files:
stream 18 (`str9102`, Kite's drain of Skeith: Kite's body and the
backdrop of `str8000e`, the bracelet's rings of `str8001e`), the drain
movies 108-111 (the same two), the Ryu Books' covers 112-119
(`str8801`-`str8808`: `str8000e`, and the backdrop, sparks and lightning
of `str8800e`). The gate hack's (107) names the same models, but the
rows it reads whole (`str7000e` and the others) define them. So stream 18
needs a field or the desktop, and a book's cover a town or the desktop.

The port: `piney_game::stream::FileList` (`Town`, `Field`, `Desktop`)
gives `Stream::with_resident` the list's `DATA.BIN` files for the
scripts' streams, the menus' movies and the Audio screen's;
`every_hash_object_resolves_where_its_stream_plays` checks every stream
each place plays.

## A scene

`DecodeThread` (0x00198830), per `ccsTbl` entry: a new `ccStream` and
`DecodeSetup` (the [CCSF](../formats/ccs.md) header, index and setup
section), or `GetCCSAdrs` of the file read whole for type -2; then
`InitScene(sysLayer, 0)`, `PlayScene`, `ccSetStreamDemoThread`; it waits
for the scene's state bit 0x20 (done). A scene of a type other than 0 with
another entry after it plays with `PlayScene` flag 4: at its end it keeps
drawing its last frame; the next file is set up meanwhile, then `EndScene`
takes it down.

**Names across files.** `DecodeIndexSection` leaves every index entry
unresolved; `CompleteIndexChunkAdrs` (0x001476c0) matches this file's `#`
entries against every other loaded `ccStream` (`ccscRoot`, newest first)
by path and name, and the older files' `#` entries against this one. So a
scene names the objects of its setup file (`str0001` has 208 `#` OBJ and
164 `#` MDL entries, all in `str0001e`).

**Objects** (`InitScene` 0x00148640). Only the scene file's own Clump chunks
draw, in object-index order. Each node is an Obj or ExtObj of the scene
file; `ccGetExternalIndex` (0x00101a50) follows ExtObj targets (across
files) to the Obj that gives the model; the node's own Obj2 (0x2000: `u32
obj, flags, modifier, layer, slayer`) gives flag bit 0 (the transparency is
the parent's times its own), bit 1 (`partFlag`: a record at the origin
hides it), the `MPH_` morpher and the layer. Its parent is the node its own
parent field names (`CompleteChunk` builds `parentTbl`), else the clump's
coordinate (the identity). Nodes whose model is a shadow model (mtype bit
3) are taken out of the clump. `InitScene` sets every node's `localtp` and
`worldtp` to 0: nothing shows until an `F_Obj` record places it. The first
Camera chunk in object order is the scene's `ccCam` (the others are never
moved); every Light chunk becomes a `ccLight` (`ccCreateLight` 0x001389b0:
type 1 distant, 2 direct, 3 spot, 4 omni; priority -1 for a distant light,
0 for the others) in a fresh `ccDrawEnv`'s group. `InitScene` adds them in
the order `lightChunkRoot` holds them, the file's reversed, and
`ccLightGrp::AddGrp` (0x00139060) keeps the group in descending priority,
equal priorities in the order added: the distant lights come after all the
others. 37 of the 69 stream files with lights have a distant light that
the reversed file order puts before an omni one (`str0580`'s
`LGT_se1_5lig1` before its twenty `LGT_se1_5omn*`).

**Layers.** `Decode_Layer` (0x0014c5d0): `u16 n`, pad, `n x (u8 kind, pad,
u32 obj)`; kind 0 a draw layer, 1 a shadow layer, object 0 the default one.
Numbers are handed out in order: the default draw layer first when no entry
names it, the default shadow layer next when none names it, then each entry.
A shadow layer with a Shadow chunk gets a `ccShadowPacket`, which its
nodes' shadow models cast into as `F_Shadow` lights them ([the shadow
volumes](shadow.md#streams)).
Each `LYR_` object is a `ccLayer` of that priority sharing `sysLayer`'s view;
a node goes to its Obj2's layer or the default (`sysLayer`, priority 0,
without a Layer chunk). `str0001`: sky 1, moon 2, clouds 3, floor 4, relic
5, default 6, filter 7, text 8.

**The draw list.** `ccStreamDrawLayerList` (+0xac): one entry per layer, a
new layer's entry put first, each node put first in its layer's list; so
`Draw` walks layers in reverse order of first use and nodes in reverse
clump order.

## Frames

`PlaySceneMain` (0x001494c0), the scene's task:

```
if state & 0x80: ccPcmSound::StartPlay
loop:
  if !(state & 0x10): DecodeFrameSection      the next frame's records
  Breath(1)
  if ctrl & 0x10: break                       EndScene asked
  notes: funcNoteProcess(note) each           ccSetStreamDemoNote
  SetMatrix_PosRotXYZDebug(cptr, camPos, camRot); SetView(layer view, cptr)
  drawEnv.ambient = ambient; drawLayerList->Draw()
  if !(state & 0x10): continue
  flag 1: ResetScene; flag 4: continue (draw the last frame again)
  Breath(2); break
DelSceneObject; state |= 0x20
```

The draw's camera is thus `SetMatrix_PosRotXYZDebug` of `DecodeF_Camera`'s
`camPos` and `camRot` with the stream's matrix (+0xc0, the unit matrix
from `Init`), not the `SetMatrix_PosRotXYZ` the record itself made; both
in VU0's arithmetic as on the desktop ([desktop](desktop.md), "The 3D
draw"). `Scene::view_camera` is that camera.

`DecodeFrameSection` (0x0014ded0): `frameCnt += frameSpd` (256), the whole
frames pass; `frameNow` moves on by them, held at `frameEnd` (the Frame
chunk's count - 1). `DecodeFrameChunk(nowf, decf)` (0x0014e1b0) reads chunks
until a Top past `decf`. The first pass reads frames 0 and 1; each later one
frame. The pass that reads the end Top (-1 or -2) sets state 0x14. So a
scene draws frames 1 to `frameEnd`, one a game frame, then two frames of
nothing.

Only a file read whole into memory (type -2) has a `frameOffsetTbl`:
`InitScene` allocates it when the reader's ring buffer is in memory mode
(`OpenBufferData` sets mode 1, `OpenData` 2; the stream's own ring,
`SetStreamBuffer` -> `OpenBuffer`, is mode 0). With one, a Top records its
read pointer there, a frame read once is sought rather than re-read, and the
-2 end Top rewinds the scene (`ResetScene`), which is how `str7300` loops.
Without one (every streamed scene) `ResetScene` does nothing, and each pass
goes on from the record after the Top that ended the last: that frame's
records are read with `nowf` still the pass's first frame, so an `F_Note`
there carries the frame before the one it sits in (`str0001`'s BGM note at
frame 5 says 4).
`ccGetStreamFrame` (0x0019aaf0) is `frameNow` of the scene playing.

| record | payload | does |
| --- | --- | --- |
| 0x0101 F_Obj | `u32 obj, flag, f32 pos[3], rot[3] (degrees), scale[3], tp, u32 dispSW` | the node's matrix `SetMatrix_PosRotZYXScale(pos, pi deg / 180, scale)`: `T Rx Ry Rz S`; `localtp` = tp clamped to 0..1, and `worldtp` too unless the node inherits; `dispSW = (rec & 1) | 2`, without the 2 for a `partFlag` node at (0, 0, 0) |
| 0x0502 F_Camera | `u32 obj, flag`, then a float per clear bit 1-8: pos xyz, rot xyz (degrees), unused, fov | the first camera only; flag bit 0: nothing follows |
| 0x0601 F_Ambient | `u32 rgb` | ambient = channel / 255 |
| 0x0608 F_OmniLight | `u32 obj, flag, f32 pos[3], u32 rgb, f32 intensity, far start, far end` | |
| 0x0602 F_DistantLight | `u32 obj, flag, f32 rot[3] (degrees), u32 rgb [, f32 intensity if flag & 0x20]` | the light's matrix `Rx Ry Rz` |
| 0x0201 F_Material | `u32 mat, flag [, f32 u, v unless flag & 2]` | `s16(4096 u)`, `s16(4096 v)` on every model material of the same name |
| 0x1901 F_Morpher | `u32 morpher, u16 n, pad, n x (u32 target, f32 weight)` | the morph targets |
| 0x0108 F_Note | `u32 obj, event, param` | a note, stamped with `nowf` |
| 0x2201 F_Pcm | `u32 id, u16 blocks, pad, u32 words` then the blocks | PCM into `ccPcmSound`'s ring |

## The draw

As [the desktop's 3D draw](desktop.md#the-3d-draw): `ccObj::Draw(1.0)`
takes `lwMatrix` from `_SetLWMatrix` (0x00138380, the parent's times its
own, VU0 multiply-adds) and the transparency from `worldtp` (or
`_GetTransparency`, 0x00138490, own times parent's); it draws the model when
`dispSW` is 3 and the transparency is above 1/128. Bone and skin mmats take
the matrices of the node's clump (`ccObj::Init` hands `ccModel::Init` the
clump's node table). Layers go to the GS by priority. The view is
`sysLayer`'s (`SetFrame(0, 0, 512, 384, 256, 192, 1, 1)`, `divZ` 1000: VU1
clips triangles nearer than that instead of dropping them).

**Light** (`ccModel::Draw` for mtype bit 0 calls `ccDrawEnv::SetLightMatrix`
0x00105900 with the object's position): each light in group order (above:
the omni lights before the distant ones) is asked `CheckRange`; the first
three that answer fill slots 2, 1, 0. An empty slot's normal comes out as
(-1, 0, 0), its colour 0. Omni (0x00139830): nothing at intensity 1/128 or below; `d =
pos - light`; full intensity when `farDownEnd` is 0 or `|d| <= start`,
`intensity (end - |d|) / (end - start)` up to `end`, nothing past it; normal
`d / |d|`. Distant (0x00139170): the light's matrix times `(0, 0, -1)`,
colour times intensity. `sceVu0NormalLightMatrix` negates and normalises;
VU1 doubles the colours ([render](render.md)); the ambient is the scene's.

**Morph** (an Obj2's modifier, an `MPH_` morpher, whose targets and weights
the `F_Morpher` records set): `ccModel::Draw` calls the modifier's
`ccMorpher::Modify` (0x0013af10) for each rigid mmat before its packets.
- **Rescale.** A target is first brought to the base model's vertex scale
  (`trunc(v * target / base)`, once, in place).
- **Weights.** Each weight goes to 1/4096 fixed point (`vftoi12`).
- **The blend.** Per vertex and axis, in the EE's 16-bit SIMD: the target's
  difference from the base (`psubh`, wrapping), times the weight, summed in
  32 bits (`pmaddh`), shifted down 12 (`psraw`) and added back with
  saturation (`paddsh`).

Stream 2's four ribbons (`MPH_ribbon_a00`-`d00`) morph between two targets
each (`MDL_ribbon_*39`, `*40`): straight bands at their base shape,
tendrils winding around the figure as drawn.

**Blend** (`ccModel::Init` 0x0013a440): a model's ALPHA is
`alphaBlendTbl[flag & 3]` from its header, and a blended model (type 1-3)
tests NEVER with FB_ONLY, writing no Z, and is always sorted. In `str0001e`,
`MDL_ngon02` (the figure's runes) is additive.

**A Z-only eraser.** In stream 5's drain (about frames 2684-2700) a gold
ring, `str0120e` object 0x20c, fades in on layer 3 with Z written. At
transparency 0.02 its vertex alpha and its AREF are both 2, so it passes
the alpha test everywhere and writes Z while drawing almost nothing. The
figure on layer 4 comes after it and loses every pixel behind the band,
which sweeps down and erases it before it turns to particles
([render](render.md#what-vu1-does-per-vertex) on why every pixel
passes).

**The renderer's files.** A stream's models are in its own archive, which
`piney-gs` looks members up in first (`Gs::set_overlay`). It reads a file
the first time a draw names it. Stream 5's `str0120` (31 MB, 83 MB
inflated) is first drawn at frame 1834, the book's bubble, and reading it
then held that frame 135 ms. The renderer now reads the overlay's files on
a thread from the moment it is set, and a draw takes the file read there
when it is ready.

## Audio

`Decode_Pcm` (0x0014ddb0, the setup section's Pcm chunk: `u32 id, u8 type,
bits, track, pad, u32 blocks, u32 words`) puts the first blocks in the ring
and sets state 0x80; `PlaySceneMain` starts the PCM (`StartPlay`, IOP 0x230);
each frame's `F_Pcm` blocks follow (0x220 per block); `EndScene` (a skip, or
the next scene) stops it (`EndPlay`, 0x240); a scene that ends by itself
leaves the queue to play out until `ccPcmSound::Close`. A block is 1,024
bytes: 256 left samples then 256 right, 16-bit, 48 kHz ([voice](../formats/voice.md)).
SEWORDS.IRX's `strPcmFunc` (0x0800) runs them on channel 0, the channel
voice lines and `BGM.BIN` use.

`ccSndStreamCtrl` (0x0017d190) changes the loaded bank's music around some
streams: before (header `size` bit 0) for 8, 25, 26, 56, 57, 58 and
112-119; after (bit 1, not with `game.status` 7) for 3, 8, 12, 13, 25, 26,
107 and 112-119. Streams 0, 2 and 106 change nothing. What each does, and
the notes' `ccSndStreamBGM`: [sound](sound.md#streams).

## Notes, subtitles and effects

`ccSetStreamDemoThread` (0x001979c0) makes `ccSetStreamDemoNote` the note
handler (and sets the view's `divZ` to 1000, `eventMsg` to -2):

| event | does |
| ---: | --- |
| 0x8001 | `eventMsg = param`: `ccEventStream` shows line `param` of `evStrMsgTbl[num]` under the stream ([subtitles](#subtitles)) |
| 0x8011 | `eventMsg = -1`: the line closes |
| 0x8010 | a cue queued for the stream's effect task (up to 8) |
| 4 | `ccSndStreamBGM(param)` (through `ccSndStreamSE`) when `strSndTbl[num]` has a BGM table (streams 3, 8, 58); events 1-3 do nothing |

A note (`DecodeF_Note` 0x0014f830, 24 bytes) is `{+0x00 next, +0x04 event,
+0x08 param, +0x0c frame, +0x10 the index entry's object for the record's
first word (a `ccObj` for an `OBJ_`), +0x14 u16 the entry's type}`; a
queued cue keeps the object (stream 5's hit marks stand there).

A 0x8010 note is copied into `eventExNoteTbl` (24 bytes each) and its
param into `eventExNoteParamTbl` while `eventExTscb` is set and
`eventExNum` is below 8; others are dropped.

The effect tasks (`StreamDemoFuncTbl` 0x0034f420, 8-byte records: name,
function; the scene's name with a trailing `p` dropped, `str7Nxx` for N 0-4
read as `str7N00`, `str88xx` as `str8800`, `str8Nxx` for N 1-3 as
`str8000`): `str7100`, `str8000`, `str8800`, `str0001`, `str0090`,
`str0110`, `str0120`, `str0130`, `str0150`, `str0240`, `str0250`,
`str0300`, `str0301`, `str0305`, `str0350`, `str0570`, `str0580`,
`str0581`, `str0610`, `str9101`, `str9104`, `str9102`. A found one starts
as `ccStartThread(fn, 96, 2048)` with `param[0]` the `ccStream`, and becomes
`eventExTscb`; the next `ccSetStreamDemoThread` sets the old one's
`param[1]` (+0x18) to 1, which ends it at its next loop check, and
`eventExTscb2`'s to 2 (`Func_str0580` outlives its scene until then). Ported:
`str0001` (below), `str0120` and `str0300` ([streams 5 and
10](#streams-5-and-10s-effect-tasks)), `str0090`, `str0110`, `str0130`,
`str0150`, `str0240`, `str0250`, `str0301`, `str0305`, `str0350`,
`str0570` and `str0610` ([streams 3, 4, 6 to 9 and 11 to
16](#streams-3-4-6-to-9-and-11-to-16s-effect-tasks)); stream 15's
`str0580` and `str0581` ([stream 15](#stream-15s-effect-tasks));
`str8000`, `str9102`, `str9104`, `str9101`, `str7100` and `str8800` ([the
drain streams and the rest](#the-drain-streams-effect-tasks)): every one.

The hit marks of streams 5, 8, 9 and 11 and the transfers of streams 7,
12, 14, 15 and 16 (`effHitMarkStr`, `effTransferStr`: `ccEffect`s on
`strEffLayer`) are drawn by the streams' own effects
([the streams' effects](effects.md#the-streams-effects)). No stream of
Infection's has a Particle node (0x0d00).

Subtitle tables exist for streams 3-5 and 7-16.

## Subtitles

`evStrMsgTbl` (0x003112f0) and, in Parody Mode (`saveData.parodyFlag`
+0x842b), `evStrMsgTblp` (0x00311510) hold a pointer per stream (NULL for
none) to 12-byte records, indexed by the notes' params:

```
EVSTRMSG    0x0c bytes
  +0x00 u32   flag    0x800: shown with Movie Text off too
  +0x04 char *name    the speaker ("Orca", "#0" the player), NULL for none
  +0x08 char *text    three NUL-separated lines; ccKanjiStrSeparate(text, 0..2)
```

A note past its table reads the records after it (the next stream's
table); the parody tables of streams 4, 7, 11, 13 and 16 have one record
fewer than the notes ask for. `strMsg0090` (stream 3) record 0: the name
"Orca", two lines of his and an empty third. Three records have flag
0x800, all without a name: stream 11's record 0 (empty) and stream 16's
records 6 (a line that names Kite by `#0`) and 20 (empty).

With a table, `ccEventStream` makes, once the decoder's scene is set up
(`ccGetStreamAdrs` 0x0019ab80 and its state bit 8): a layer (`ccLayer::Init(242,
0)`, `SetFrame(0, 0, 512, 384, 256, 192, 1, 6/7)`: the menu layer's
priority and frame), the menu window (`ccInitMenuWindow`) and a fresh
`ccMessage`, which replaces the global `ccMsg`. Then, until the stream's
task has set its parameter to -1, one pass a frame:

```
m = ccGetStreamDemoMsg()          0x00184440: eventMsg, which is then -2
m == -1: ccMsg->Close()
m >= 0:  rec = table + 12 m
         if saveData.strWinMode (+0x8430, Movie Text) or rec.flag & 0x800:
           ccMsg->Change({emode 0x200, lines 0-2 of rec.text}, rec.name, -1, -1)
ccLayer::active = the layer; ccMsg->Disp(); menuWin->Trans()   (the texture upload)
Breath(1)
```

When the parameter is -1 it calls `Close` and leaves the loop without a
`Disp`: the window vanishes, it does not fade. Then `Breath(3)` and the
window, sprite and layer are deleted (`ccMsg` is left pointing at the freed
message). `Check` is never called, so the window takes no pad and shows no
page cursor. The window is the desktop's speech window
([desktop](desktop.md)): `Change` reopens it unless a speech window is up
(fading in by 24 a frame), restarts the text, which types a glyph a frame
(emode 0x200 is not 0x100); the name line sits in the header. Movie Text is
on in a new save ([the save](../formats/save.md)).

**When.** The tasks run by priority each frame: the scene's (24, its notes
set `eventMsg`), the decoder (27, whose `ccSetStreamDemoThread` for a
following scene sets `eventMsg` to -2 again), the event task (32, the pass
above), the stream's own task (33, whose `WaitEnd` writes -1 on a skip).
So a line shows on the frame its note is in; a skip's -1 is read the next
frame, the stream's one blank frame after a skip.

**The port.** `piney_stream::subtitle::Subtitles` (the table read from
the executable, the pass on `piney_desktop::message::MsgWindow`) and
`piney_stream::event::EventStream` (the stream, then the pass on
`Stream::demo_msg`, drawn on layer 242 over the stream's frame: every layer
a stream draws on is below it). `piney-game`'s `StreamPlayer::event`
plays the event instruction's streams through it, with the music; the
title's and the Audio screen's streams have neither.

## Stream 2's effect task

`Func_str0001` (0x001885d0, strdemo.cpp), for `str0001`. Its state is a
`ccStrEffectCtrl` (0x20): the stream, its own layer, a
`ccRasterNoizeSDM_sc` (8 bands), a `ccScFade_sc`, a `ccBufferSampling_sc`,
a `ccBufferReverce_sc`, a `ccBufferShade_sc` (4 samplers) and a
`BossSkillName` pointer (0 here).

**When it runs.** `ccTscb::GoThread` (0x00159d40) creates and starts the
kernel thread at once; `ccThControl` (0x0015a5c0, priority 1) wakes every
task each frame and they run by priority, so the scene's task (priority
`pri + 1` = 24) runs before it every frame. Its first pass is in the frame
the decoder sets the scene up, before the scene's first drawn frame; each
later pass sees the notes the scene took that frame and `frameNow` already
on the next frame (the scene reads it before it breathes). The loop runs
while `param[1]` is 0 and the scene's state has 0x40 (`InitScene` sets it,
`DelSceneObject` clears it): after a natural end it runs in the first of
the two gap frames and not the second; after a skip it draws on the skip
frame and not after. On leaving it switches the feedback off, breathes once
and frees everything.

**At start.**
- `GetSubstAdrsF(name, 0)` (0x00144630) searches the scene file's own index,
  skipping `#` entries; `ccMatchIndex` (0x00101ad0) follows an ExtObj entry
  to its final target and compares that name. Not found with 0: a write to
  address 0.
- `OBJ_se1_6flo1` (the floor, `EXT_se1_6flo1`'s target): its model's TEST
  becomes `(test & ~0x300f) | 0x1001`, NEVER with FB_ONLY
  (`ccObj::SetRenderState(CCRS_ZWRITEENABLE, 0)`): colour always, no Z.
- `fogOffObj` (0x0030bd80): `OBJ_se1_6bac1`, `OBJ_se1_6clo1_1`,
  `OBJ_se1_6clo1_2`, `OBJ_se1_6moo1` get `fogSW` (+0xa0 bit 3, set by
  `ccObj::Init`, which `ccObj::Draw` puts in PRIM.FGE) cleared.
- `ccDrawEnv::SetFog(7500, 20000, 0)` (0x00105890, `fogt` 0x0034e370) on the
  scene's draw environment: fMin 0, fMax 255, fogA = -255 / (far - near),
  fogB = 255 - near fogA, FOGCOL black; VU1 fogs each vertex
  `clamp(fogB + fogA w, fMin, fMax)` ([render](render.md)). So everything
  else fades to black from view depth 7,500 to 20,000.
- A layer of its own: `ccLayer::Init(100, sysLayer's view)`.
- Eight `ccRasterNoize::Init` on it: `h = vftoi0(ls11 * 24) >> 4` = 27
  rows (the EE's truncating multiply), `SetNoize(32)`, and `SetTex(896, 0)`:
  `ccGetBuffAdrs(fbuffType, 896, 0)` = block 14336 (the 64-pixel column 14
  of VRAM, pages 448 up, spare).
- `ccBufferSampling_sc.layer = GetSubstAdrsF("LYR_jm")` (priority 8, the
  epitaph's text), `SetReflex(1.015, 0, 0x58808080)`, on.
- `ccScFade`: `EntryFade(30, 0x80000000, 0, view x, y, w, h)`, black to
  clear over 30 frames, sent every frame.

**Cues** (`eventExNoteTbl` read from the last queued down):

| cue | does |
| ---: | --- |
| 5, 6, 15 | `SetReflex(1.015, 0, c)` with c 0x58808080, 0x50808080, 0x60808080; on, counters 0 |
| 11 | per band i: `y = rand % 50 + 48 i`, `h = rand % 11 + 2`, `moveY = rand % 32 - 16`, `time = rand % 15 + 5` (in that order), `offsY = 0`, `posY = y`, `SetYH(y, h)`; noise on |
| 12 | noise off |
| 999 | `DeleteFade`; `EntryFade(frameEnd - frameNow or 1, 0x00ffffff, 0x80ffffff)`: to white |
| other | nothing (`str0001` also has 800 and 16) |

`str0001`'s cues: 5 at frame 1, 800 at 80, 6 at 400, 11 at 1960, 12 at
1970, 15 at 1980, 16 at 2000, 999 at 2020 (frameEnd 2040, so the white
takes 19 frames).

**Each pass**, in this order:
1. The feedback: on (flag 1), `MakePacket(LYR_jm)`, and unless the scene
   is held (`ctrl` bit 0) `execTime` counts down to flag 2; flag 2 (never
   reached here) counts `fadeTime` down and sets alpha
   `fadeAlpha0 + (fadeTime fadeAlpha1 >> 12)` clamped to 0-255.
2. The fade: `ccScFade::SendPacket` when its flag is set.
3. The noise, when on: unless held, per band `SetNoize(20)`,
   `offsY += moveY`, `SetY(posY + (offsY >> 4))`, and when `--time <= 0`
   the band restarts as cue 11 does; then every band's `MakePacket`.
4. `ccBufferReverce::MakePacket` when its flag is set (never here), then
   the `ccBufferShade_sc` samplers (none here).

`ccRasterNoize` (0x54 bytes: layer, `short y, h`, colour, tbp0, tbw, tpsm,
`short ampli[32]`; the `_SD` form adds `moveY, offsY, posY, time`):
- `SetYH(y, h)` / `SetY(y)`: `vftoi0(ls11 * v) >> 4`, logical lines to
  frame rows; h at most 32.
- `SetNoize(max)`: `m = vftoi0(ls00 * max)` (1/16 pixel), then per row
  `ampli[i] = rand % 2m - m`: +-20 pixels a frame, +-32 at start.

**The frame buffers.** `SetScreenModeMain` lays out `fbuffAdrs` = {0, 3584}
(blocks; 512 x 448 PSMCT32, `fbuffType` 0), the Z buffer at 7168 (PSMZ32)
and `zbuf_1 = (7168 >> 5) | 48 << 24`; `db.disp[0]` shows buffer 1 and
`draw0` draws buffer 0, `disp[1]` / `draw1` the other way. `ccSystem::Ctrl`
swaps to `page ^ 1` (`sceGsSwapDBuff`) before it sends the lists the tasks
built, so a list built on page P draws into `fbuffAdrs[P ^ 1]` while
`fbuffAdrs[P]` - the last frame, finished - is shown. The effects' DMA-time
callbacks (`iFuncDrawBuffTrans` 0x001089c0, `iFuncBuffSampling0`
0x00106d20, run by a `ccDmaFuncTag`: a tag of ID 7 with ADDR 1, then a
qwc-0 tag holding the function and its argument) patch TEX0 with the page
then current and arrive at the same buffers.

**The packets.** Every list is prepended to its layer (`ccDrawPacketCtrl`'s
use group: a new packet's DMA ADDR is the old head), so a layer draws its
packets last sent first; layers go by ascending priority. In frame-buffer
pixels, off XYOFFSET (0x7000, 0x7200):

- **Feedback** (`ccBufferSampling::MakePacket` without a texture of its
  own, `texuse` 0), on `LYR_jm`, before the scene's own objects there (the
  scene's task sent those first): one flat TRISTRIP (PRIM 0x154: TME, ABE,
  FST) of four corners, the view's `bboxClipMin`/`Max` (1792, 1824) -
  (2304, 2272) scaled by 1.015 about the centre, `vftoi0` then less 8
  (half a pixel): (-4.375, -3.875) to (515.3125, 450.8125). UV the scissor
  box's corners, (0, 0) to (512, 448). TEX0: `fbuffAdrs[(page + 1 + 1) &
  1]` = the last frame, TBW 8, PSMCT32, 512 x 512, TCC 0, MODULATE; TEX1
  bilinear; CLAMP_1 5 (clamp both); ALPHA `alphaBlendTbl[0]` = 0x44,
  `(Cs - Cd) As + Cd` with As the colour's alpha (0x58, 0x50 or 0x60);
  RGBAQ the colour (0x80 grey); TEST 0x51001 (alpha NEVER with FB_ONLY, Z
  GEQUAL), ZBUF masked, FBA 1. Z is `view_screen (0, 0, z, 1)`'s z / w for
  z 0: w is 0, the EE's division gives the largest float and `vftoi4`
  saturates, 0x7fffffff, above any model's Z. So the last frame, 1.5%
  larger, is laid over layers 0-7 at alpha 0.6875 (0.625, 0.75): a zoom
  trail that feeds on itself.
- **Raster noise** (`ccRasterNoize::MakePacket`), on layer 100, band 7
  first: `ccMakePacketDrawBuffTrans(tbp0, 8, psm, 512, 32, rect, layer)`
  (0x00108a10) copies 512 x 32 from row `y` of the draw buffer into the
  spare VRAM: a context-2 SPRITE (PRIM 0x316) with FRAME_2 there, SCISSOR_2
  511 x 31, XYOFFSET_2 0, TEST_2 ALWAYS, ZBUF_2 masked, TEX0_2 the draw
  buffer (512 x 512, REPEAT, point), RGBAQ 0x80 grey, UV (0, y) - (512, y +
  32) onto (0, 0) - (512, 32). Then per row i of h one SPRITE (PRIM 0x116:
  TME, FST, no blend) from (`ampli[i]` / 16, y + i) to (512 + `ampli[i]` /
  16, y + i + 1), Z 0xffffffff, UV (0, y + i) - (512, y + i + 1), TEX0 the
  copy with TW 512, TH 32, REPEAT, point, TCC 0, RGBAQ 0x80 grey; TEST
  ALWAYS, ZBUF masked. Rows wrap at 32, so row y + i shows copy row
  (y + i) mod 32, frame row y + ((y + i) mod 32). A band can start below
  row 416 (y reaches 449) or above row 0: its copy then takes rows past the
  draw buffer (the other buffer's top, or the Z buffer), shown where the
  wrapped rows fall inside the frame.
- **Fade** (`ccScFade::SendPacket`): its element layer is 0, so it sends
  on `fontLayer` (240): the `SetTag` state (TEST ALWAYS, ZBUF masked,
  ALPHA 0x44, PABE 0, FBA 0) and per element `ccFadeGIFtag` (0x0034b380:
  flat TRISTRIP with ABE, RGBAQ + XYZ2 four times) at (0, 0), (0, 447.9375),
  (512, 0), (512, 447.9375), colour `c0 + (c1 - c0) cnt / tcnt` per byte
  (64-bit, truncating); then `cnt` goes on, held at `tcnt`.
- **Never here**: `ccBufferReverce::MakePacket` (0x00106890) would send an
  untextured SPRITE over the view's box with ALPHA 0x80000000a4, `(Cs - Cd)
  FIX + 0`, white: the frame inverted.

**rand** (0x00133a38) is newlib's: `_impure_ptr->_rand_next = next *
6364136223846793005 + 1` (64-bit), returning bits 32-62; it starts at 1 and
only the staff roll (`ccThStaffRollCtrl`, desktop.prg) calls `srand`
(0x00133a20). The task takes 216 numbers at start (8 x 27), h per band
each noise frame and 4 per band restart. The same state serves every task
that draws random numbers, so the numbers the noise gets depend on
everything before it.

**The port** (`piney_stream::effect`): the task in `Stream::step` after the
scene's draw, its first pass folded into the stream's start; the band, the
feedback's draw and the inversion from `piney_desktop::noiz`, which the
field menu's noise ([`ccNoiz`](field-ui.md#the-noise-ccnoiz-0xe8)) shares;
`Stream::with_resident` for the CCSF files a place keeps in memory (Data
Drain's streams resolve their models against them, [the drain
movie](field-ui.md#the-drain-movie)) and `Stream::step_with` for another
task drawing into the scene (`ccThDrainEnemy`);
`piney_draw::TexRef::FrameBuffer` (a rectangle of the frame as drawn so far,
copied once per command, texels past the frame 0) and `PreviousFrame` carry
the frame buffer reads; the floor's Z write is off. The fog is
[the scene's](#the-tasks-fog). Not drawn: the VRAM past the draw buffer a
band's copy can reach (0 in the port).

## Streams 5 and 10's effect tasks

`Func_str0120` (0x001950b0, stream 5: Skeith drains Orca) and
`Func_str0300` (0x00189e10, stream 10: Kite's first Data Drain) keep the
same `ccStrEffectCtrl` as `Func_str0001` and run the same pass each frame
(the feedback, the fade, the noise, the inversion, the buffer shades - none
- and +0x1c, a `ccMask` - none), with these differences:

- **At start** nothing is on: no feedback, no fade, no fog; the eight
  noise bands are made (216 numbers from `rand`). The feedback's layer is
  the task's own (100), so it draws over every scene layer.
  `Func_str0120` also gives `strEffLayer` (the effect objects' layer,
  priority 20, `RequestStrPlay`'s) the scene's view.
- **The inversion** (`ccBufferReverce::MakePacket` 0x00106890, on the
  task's layer after the noise): one sprite, PRIM 0x146 (untextured, ABE,
  FST), from `bboxClipMin` to `bboxClipMax` (the whole frame), Z
  0xffffffff, RGBAQ 0x80ffffff; ALPHA 0x80000000a4, `(Cs - Cd) 0x80 >> 7 +
  0`, white less the frame; TEST 0x1001 (alpha NEVER with FB_ONLY, no Z
  test); ZBUF masked.
- **Cues.** The special cases first, then the cue's last digit
  (`divu` by 10; jump tables 0x0034e3a0 and 0x0034f350):

| cue | `Func_str0300` | `Func_str0120` |
| --- | --- | --- |
| 999 | fade to white over `frameEnd - frameNow` frames (1 at the end) | fade to white over 9 frames |
| 899 | (last digit 9: nothing) | black at once: `EntryFade(1, 0x80000000, 0x80000000)` |
| 800 | (0: nothing) | from white over 10 frames: `EntryFade(10, 0x80ffffff, 0x00ffffff)` |
| 75 | `SetReflex(1.015, 0, 0x40808080)` | (last digit 5) |
| 55 | `SetReflex(1.125, 0, 0x48808080)` | (last digit 5) |
| 5 | (last digit 5) | `SetReflex(1.04, 0, 0x66808080)` for 24 frames, then its alpha falls to 0 over 32 |
| 610-614 | (last digit) | a hit mark (below), then `SetReflex(1.02, 0, 0x60808080)` for 2 frames, falling over 10 |
| 601-603 | (last digit) | a hit mark |
| 12 | (last digit 2) | the scene view's `divZ` (+0x25c) to 500; noise off |
| x1 | noise on (each band as `str0001`'s cue 11) | the same |
| x2 | noise off | the same |
| x3 / x4 | inversion on / off | the same |
| x5 | `SetReflex(1.015, 0, 0x50808080)` | `SetReflex(1.03, 0, 0x30808080)` |
| x6 | feedback off (`flag`, `execTime`, `fadeTime` 0) | the same |
| x7 / x8 | a one-frame white flash: `EntryFade(1, 0x00ffffff, 0x80ffffff)`, then `(1, 0x80ffffff, 0x00ffffff)` | (last digit 7, 8: nothing) |

A timed feedback sets `execTime` and `fadeTime` and `fadeAlpha1 =
(colour's alpha as a signed byte << 12) / fadeTime` (C division), with
`fadeAlpha0` 0: flag 1 draws for `execTime` frames, then flag 2 draws with
alpha `fadeAlpha1 fadeTime >> 12` as `fadeTime` counts down. The fades'
flag stays on after they end: the last colour is drawn every frame.

**Hit marks** (`Func_str0120`): `effHitMarkStr(pos, rot, strEffLayer)`
(0x001cc620), `pos` the cue's object's `lwMatrix` translation
(`_SetLWMatrix`, or its local matrix without a parent, copied to one of
eight `vecWork` slots), `rot` `pi * deg / 180` (EE floats) of
`hitRot0120a[n]` (0x0034f320, three entries: (0, 30, 180), (0, 30, 0),
(0, 30, 180) degrees), cues 601-603 and 610-614 each counting their own
`n` from 0. The five cues 610-614 read two entries past the table (the jump
table's words, denormals the EE takes as 0). Stream 5's eight marks stand
at objects 472 and 470. Cues 604 and 605 are not marks: their last digits
switch the inversion off and a feedback on. Stream 5's cues, in order: 5,
6, 601, 602, 610, 603, 611, 612, 613, 604, 605, 614, 15, 16, 25, 26, 35,
36, 45, 46, 55, 56, 65, 66, 75, 76, 85, 86, 1, 2, 95, 11, 999, 899, 800,
96, 12; stream 10's: 61, 62, 65, 63, 64, 66, 75, 71, 76, 72, 81, 82, then
1-6, 11-16, 21-26, 31-36 and 41-46 (each x1, x3, x5, x2, x4, x6), 51, 55,
7, 8, 52, 56, 999.

**The port** (`piney_stream::effect`: `Ctrl`, `Str0120`, `Str0300`,
`Task`): as `Func_str0001`'s; a hit mark comes out as a request with the
object's world position and the rotation (`Request::HitMark`), and the
`divZ` change as `Request::DivZ`. The marks are `ccEffect`s the stream's
`ccThEffectStr` would run (drawn by the stream's effects when the player
gives them); the `divZ` goes to every model the scene draws
(`Scene::div_z`, `ModelDraw::div_z`), where piney-gs clips by it.

## Streams 3, 4, 6 to 9 and 11 to 16's effect tasks

`Func_str0090` (0x001919f0, stream 3), `Func_str0110` (0x00194410,
stream 4), `Func_str0130` (0x001961c0, stream 6), `Func_str0150`
(0x00189320, stream 7), `Func_str0240` (0x0018abb0, stream 8),
`Func_str0250` (0x0018b700, stream 9), `Func_str0301` (0x001923b0,
stream 11), `Func_str0305` (0x001938d0, stream 12), `Func_str0350`
(0x00192e80, stream 13), `Func_str0570` (0x0018c780, stream 14) and
`Func_str0610` (0x00196c00, stream 16) keep the same
`ccStrEffectCtrl` and pass as streams 5 and 10's, their feedback on the
task's layer (100); the noise bands' texture is set to (896, 0) after
`Init`. Nothing is on at the start.

- **`Func_str0090`** sets the scene's fog (`SetFog(200, 7000,
  0x00141e00)`, every object with `fogSW`).
  Cues 5 and 15: `SetReflex(1.001, 0, 0x6c808080)`; 6 and 16: the
  feedback's flag to 2 (fading) with no fade time, so it draws at alpha 0
  from then on. Stream 3's cues: 5, 6, 15, 16.
- **`Func_str0110`** sets the same fog. Cue 25: `SetReflex(1.02, 0,
  0x40808080)`; 16: flag 2; 15: 1.04, alpha 0x66, for 44 frames, fading
  over 32; 5: 1.015, alpha 0x40, for 10, fading over 15. Then by the last
  digit: 1 noise on, 2 off, 3 the fog off (`SetFog(0, ...)`) and the
  inversion on, 4 the inversion off, 6 flag 2. Stream 4's cues: 1-6, 15,
  16, 25, 11, 12, 26.
- **`Func_str0130`** first letterboxes the scene's view:
  `ccView::SetFrame(0, 48, 512, 288, 256, 192, 0.75, 0.75)` on sysLayer's
  view, which every scene layer and the task's own share (its end sets
  the default back). `SetFrame` (0x00104ae0) scales x by W / 512 and y by
  H / 384 (EE floats, truncating); `scx`, `scy` = the frame's corner plus
  (2048 - W / 2, 2048 - H / 2) plus the centre (here (2048, 2104) less an
  ulp; the default's `scy` is itself 0x44ffffff); `bboxClipMin` / `Max`
  the corners rounded (1792, 1880)-(2304, 2216); the scissor the frame's
  rows 56-391; then `SetAspect(0.75, 0.75)`, so `layer_screen` scales by
  12 and 14. The bands' `Init` runs after it: 21-row bands, 168 numbers
  from `rand`. Its only cue, 899, fades to black over `frameEnd -
  frameNow` frames (`frameEnd` when that is 0) with `EntryFade(t, 0,
  0x80000000, view.x, view.y, view.w, view.h)`: the fade's rectangle is
  the frame's. `ccScFade::SendPacket` draws on the font layer (its `layer`
  is null) through that layer's default view: the corner
  `ApplyLayerScreenMatrix(x, y)`, the size `(w ls00, h ls11)` truncated,
  a strip over rows 56-391.

- **`Func_str0150`** gives `strEffLayer` the scene's view, sets the fog
  (`SetFog(1000, 7500, 0, 85, 0x00144870)`) and takes `fogObj0150`'s one
  object (`OBJ_sfp7bac2`) out of it (`fogSW` bit 3 cleared). Cue 899: a
  fade to black over the frames left, as `Func_str0130`'s. Cue 500:
  `effTransferStr(pos, charHeightTbl[0], strEffLayer)` (0x001ce220), `pos`
  the note's object's `lwMatrix` translation (through `vecWork`, as a hit
  mark's): stream effect controller -2 (the arrival's rings) at a point
  for a character 160 high, on the first free `effcStr` slot of 50. The
  task leaves only when the next scene's `ccSetStreamDemoThread` sets
  `param[1]`; after `ccDeleteThread` it breathes forever.
- **`Func_str0240`** looks up `OBJ_trall` (unused), gives `strEffLayer`
  the scene's view. Cue 899 as above; cues 600-605, 610 and 611 each a
  hit mark at the note's object, one counter over them all taking the
  next row of `hitRot0240` (0x0034e3d0: (0, 30, 180) and (0, 30, 0) in
  turn, seven rows). Stream 8's eighth mark reads the next data, the
  string "OBJ_trall", as degrees.
- **`Func_str0250`** gives `strEffLayer` the scene's view. Cues: 601 a
  hit mark turned by `hitRot0250` (0, 0, 0); 989 a fade to white over the
  frames left (`frameEnd` when none are); 900 a fade from white over 10
  frames; 940 that fade and the feedback off; 910, 920 and 930 that fade
  and `SetReflex(1.0, 0.00697, 0x50808080)`, the feedback turned; 25
  `SetReflex(1.015, 0, 0x40808080)`; 15 `SetReflex(1.03, 0, 0x40808080)`
  and 5 `SetReflex(1.015, 0, 0x50808080)` with `fadeTime` 15 ready (no
  `execTime`: they fade when a cue x6 comes). The rest by the last digit:
  1 noise on, 2 off, 3 and 4 the inversion, 6 the feedback fading.
- **`Func_str0301`**: cue 899 a fade to black over the frames left; 601 a
  hit mark turned by `hitRot0301` (0x0034f310: (0, 30, 180)).
- **`Func_str0305`**: 899 as above; 500 a transfer (160 high); 5
  `SetReflex(1.001, 0, 0x40808080)` with a 20-frame fade ready; 6 the
  feedback fading.
- **`Func_str0350`**: `Func_str0150`'s fog and object; cue 899 as above.
- **`Func_str0570`**: before its cues each pass, at scene frame 1455 a fade
  from white over 10 frames, at 1470 the fade gone (`DeleteFade`, flag
  off). Cues: 500 a transfer; 999 a fade to white over the frames left;
  55 `SetReflex(1.015, 0, 0x40808080)` with a 32-frame fade ready, 45 the
  same without; 25 `SetReflex(1.001, 0, 0x61808080)` with a 64-frame fade
  ready, 15 without; the rest by the last digit (jump table 0x0034e460):
  1 noise on, 2 off, 3 and 4 the inversion, 5 `SetReflex(1.015, 0,
  0x50808080)`, 6 fading, 7 and 8 a one-frame flash to and from white
  (as `Func_str0300`'s).
- **`Func_str0610`**: `Func_str0150`'s fog and object. Cues: 889 a fade
  to black over the frames left; 800 from black over 20 frames; 899 to
  black over 30; 6 fading; 5 `SetReflex(1.01, 0, 0x40808080)` with a
  10-frame fade ready; 510 a transfer for a character 210 high
  (`charHeightTbl[17]`), 500 one 160 high (`[0]`).
- **The turn.** `SetReflex(scale, roll, colour)` keeps `roll` at +0x0c;
  `MakePacket` builds its matrix as a unit with the scaled sizes, turns it
  (`sceVu0RotMatrixZ(m, m, roll)`), then sets the translation, so the
  picture turns about the view's centre.

**The port**: `Str0090`, `Str0110`, `Str0130`, `Str0150`, `Str0240`,
`Str0250`, `Str0301`, `Str0305`, `Str0350`, `Str0570`, `Str0610`. A transfer comes out as `Request::Transfer` (not drawn), the
hit marks as `Request::HitMark`. `Ctrl::framed` keeps the
frame (`piney_desktop::view::Frame`), and the bands, the feedback and
the inversion take their scissor, clip box and scales from it
(`piney_desktop::noiz::EffView`); the fade draws over its rectangle
(`piney_desktop::fade::draw_rect`). The scene's `frame` gives its
models the letterbox's projection (`camera::frame_projection`) and
scissor.

## Stream 15's effect tasks

Stream 15 is the ending (event 31's `stream 15`, before the staff roll):
`str0580`, then `str0581`, which follows it. `Func_str0580` (0x0018e340)
and `Func_str0581` (0x001904a0) keep the same `ccStrEffectCtrl` and pass
as the tasks above, on the task's layer (100). Each also has a table of
cues that start parts at the scene's objects, and a group of parts: puffs
of smoke and, in `Func_str0580`, rocks thrown up from the ground.

- **The effect file.** Both read `str0580e` (`GetCCSAdrsNE`; when the
  stream has none, `GetCCSAdrs` of `str0580ep`) and its `EFF_x001`, the
  puff (`ccEff::Init(chunk, 1)`). `Func_str0580` also takes the
  `OBJ_se1_6ro*` chunks (`GetChunkAdrs`), the rocks. There must be four:
  with any other count it writes to address 0.
- **The view.** Each sets the scene view's `divZ` (+0x25c): 2000 for
  `Func_str0580`, 3000 for `Func_str0581`. `Func_str0581` also gives
  `strEffLayer` the scene's view and sets a fog ([the tasks'
  fog](#the-tasks-fog)).
- **The event tables.** `eventObjTbl_0580` (0x0034eb60) and
  `eventObjTbl_0581` (0x0034f1f0) are 16-byte entries: cue, object name,
  type, parameters. A cue of 100000 ends each. The type is one of:
  - 0 a burst of puffs (`ccEffPartParam0580`);
  - 1 a rock thrower (`ccObjPartCreate0580`'s 0x34 bytes);
  - 2 both. The parameter is then a pair: `p0580_0` (0x00377d00) holds
    `p0580_2` and `p0580_3`.
- **The walker** (`ccEventObj`, 12 bytes: the stream, the object, the
  entry last found) starts on its table's first entry (cue 0,
  `EXT_t0`) and that entry's object (`GetSubstAdrsF(name, 1)`).
- **`SetObj(cue)`** (`ccEventObj::SetObj`, 0x00184150) looks from the
  entry last found. It goes forward for a larger cue and back for a
  smaller one. Its loop tests are the wrong way round, so it goes on only
  past entries on the far side of the cue. It finds a cue only when the
  cue is the next entry (or the one before). A miss returns 0 and leaves
  the walker where it was.
- **Stream 15's cues** meet that bug:
  - `str0580` gives 500 to 601 in order, a few frames apart. 500 to 538
    each find the next entry.
  - At scene frame 1456 it gives 539, 540 and 541 together. The task takes
    a frame's cues last first: 541 and 540 miss, and 539 is found.
  - Every cue after that misses, so the table's entries from 540 on never
    play. That includes the lone puffs of cues 582 and 584-586. 40
    entries play in all: 200 puffs and 138 rocks.
  - `str0581`'s 602, 603, 604 and 607 each find the next entry.

**The parts.** `ccStrPartGrp` (16 bytes) keeps two lists of `ccStrPart`s
(+0 and +4 the links, +8 the vtable; the destructor 0x00184220 unlinks
one): creators and parts. Each pass, before the `Ctrl`'s packets, the
group runs its creators, newest first, then its parts, newest first. A
creator or a part whose `Ctrl` gives 1 is deleted (`ccStrPart::Ctrl`,
0x00184320, always gives 1).

- **A burst** (`ccEffPart0580::Create`, 0x0018dfc0) makes `count` puffs,
  plus `rand() % countRand` when `countRand` is not 0. Each puff gets:
  - an angle of `rand() angle / 2^31 - 180` degrees;
  - a place `rand() radius / 2^31` out along that angle from the object's
    `lwMatrix` translation, raised by `z + rand() zRand / 2^31`;
  - a pattern `rand() & 3`;
  - the burst's speed, acceleration, scale and growth, transparency and
    fall, and colour.
- **A puff** (`ccEffPart0580`, 0xd0; `Ctrl` 0x0018dee0) each pass moves by
  its speed, the speed by the acceleration, grows and fades. Below
  transparency 0.01 it is done. Otherwise it is drawn: `ccEff::Draw(pos,
  pattern)` of `EFF_x001`, with its scale (+0x20, +0x24), colour (+0x2c)
  and transparency (+0x34) set first.
- **A thrower** (`ccObjPartCreate0580`, 0x44; `Ctrl` 0x0018d8a0) keeps two
  counters in 1/4096. Each pass it sets `a = (a & 0xfff) + rate` and `b =
  (b & 0xfff) + rateRand`. It throws `a >> 12` rocks, plus `rand() % (b >>
  12)` when that is not 0. After `life` passes it is done.
- **A rock** (`ccObjPart0580`, 0x120; `Ctrl` 0x0018d7e0) is made with:
  - an angle in [-180, 180) degrees;
  - a start up to `radius` out along it from the object;
  - a speed and acceleration across along it (up to `vx`, `ax`);
  - a speed up of `vz` plus up to `vzRand`, and an acceleration up `az`;
  - random turns about x and y;
  - turn rates about y (`dry` plus up to `dryRand` degrees) and z
    (likewise);
  - a model, `rand() % 3`: `OBJ_se1_6roa`, `rob` or `roc`. The fourth,
    `OBJ_se1_6rod`, is never used. The scale is `rockScaleTbl`'s
    (0x0034e490: 0.2, 0.4, 1.0).
- **A rock's pass.** It moves, then turns (`ccRotate` 0x00102190 brings
  an angle past pi back by a turn). It is placed (`SetMatrix_PosRotXYZScale`)
  and drawn (`ccObj::Draw(1.0)`).
- **No rock is ever deleted.** `ccObj::CheckBoundingBox` (0x0013f3b0)
  gives 1 for a model with no Bbox, and the rocks have none. Each falls
  until the task ends.

The parameters (0x0030bda0 on; speeds and accelerations in units a pass,
z up):

| | used by | puffs | spread | speed, acceleration | scale | transparency | colour |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `p0580` | `eventObjTbl_0580`'s type 0 (582, 584-586: never reached) | 1 (+ `rand() % 1`) | 100 | (0, 0, 12), z -0.01 | 6, +0.5 | 1, -0.05 | 0x80606060 |
| `p0580_2` | its type 2 (cues 0-601) | 5 | 800, z less up to 100 | (0, 0, 25), z -0.01 | 6, +0.5 | 1, -0.04 | 0x80606060 |
| `p0580_1` | `eventObjTbl_0581`'s 602-604 (`EXT_t0`) | 3 | 70 | (10, 0, 4), z -0.05 | 2, +0.1 | 1, -0.05 | 0x80606060 |
| `p0580_4` | its 607 (`OBJ_marker01`) | 20 | 300, z less up to 100 | (7, -7, 0.5), none | (2, 1), +0.1 | 1, -0.01 | 0x40606060 |

Every burst spreads over the full 360 degrees. `p0580_3`, the type-2
entries' thrower:

- 3 rocks, plus `rand() % 2`, for one pass;
- out to 800;
- across up to 15;
- up 80 plus up to 20, falling 6 a pass faster each pass;
- turning 1-11 degrees a pass about y and 2-22 about z.

**`Func_str0580`.** Each pass, while `param[1]` is 0 and the scene is up:

- **The frames.** At scene frames 1500, 1575, 1630, 1664, 1682, 1694,
  1706, 1718, 1730 and 1741 it turns the feedback on:
  `SetReflex(1.0, 0, 0x4c808080)`, then its offset's y (+0x24) 0.16:
  each frame the last frame's picture is drawn 0.01 of the view's height
  (4.5 pixels) lower, so it trails downwards. At 1565, 1592, 1640, 1670,
  1685, 1697, 1709, 1721, 1733 and 1744 it sets the feedback fading (flag
  2).
- **The cues**, last first:
  - 500-610: `SetObj(cue)`. When it is found, the entry's parts start at
    its object.
  - 999: a fade to white over 30 frames.
  - The rest by the last digit: 1 noise on, 2 off, 3 and 4 the inversion
    on and off, 5 the frames' feedback, 6 the feedback off, 7 and 8 the
    one-frame flashes to and from white.
- **The draws.** The parts on `sysLayer`, then the `Ctrl`'s pass.

**After its loop** it outlives its scene:

1. It makes itself `eventExTscb2`, turns the feedback off and passes once.
2. It deletes the parts.
3. It passes (the `Ctrl` alone) until `param[1]` is 2.
4. It passes three more times and ends.

The next scene's `ccSetStreamDemoThread` (0x001979c0) sets `eventExTscb`'s
`param[1]` to 1 and `eventExTscb2`'s to 2. This task was started first,
so each frame it passes before `Func_str0581`.

**`Func_str0581`.** Each pass, while `param[1]` is 0 and the scene is up:

- It sets the fog again.
- **The frames.** At scene frame 1050 it turns the feedback on
  (`SetReflex(1.005, 0, 0x58808080)`); at 1155 it turns it off.
- **The cues**, last first:
  - 600-610: `SetObj(cue)`. A type-0 entry that is found starts its burst:
    602-604 at `EXT_t0`, 607 at `OBJ_marker01`.
  - 65: `SetReflex(1.005, 0, 0x40808080)` with a 15-frame fade ready.
  - 55: the shades. Sampler `i` of two gets `SetShade(2 << i, 1000, (4 -
    i) << 28 | 0x808080)`: 2- and 4-texel blocks, alpha 0x40 and 0x30.
    Their offsets (+0x20, +0x24) are -0.0041 and -0.0082. The feedback
    is set to `SetReflex(1.01, 0, 0x50808080)` at depth 1000 (+0x08).
  - 56: the shades off (none drawn) and the feedback off.
  - 899: a fade to black over 30 frames. 901-904: a fade from white over
    5. 900: from white over 45.
  - 699: `effTransferStr` at the note's object, `charHeightTbl[17]` (210)
    high.
  - The rest by the last digit, as `Func_str0580`'s, but 5 is
    `SetReflex(1.02, 0, 0x40808080)` and 6 the feedback fading.
- **The draws.** The parts on `sysLayer`, then the `Ctrl`'s pass.

After its loop it turns the feedback off and ends without a pass.

**The shades** (`ccBufferShade_sc`) hold four `ccBufferSampling`s, the
number drawn (+0xe8), and an alpha fade that neither task sets. Each pass,
after the inversion, the first `count` samplers' packets go on the task's
layer, the last first.

`ccBufferSampling::SetShade(t, z, colour)` (0x00106bf0) sets:

- CLAMP_1 to REGION_REPEAT both ways, with mask `1024 - t` and fix `t / 2`,
  so every `t` x `t` block of texels reads one texel;
- scale 1, no turn, centre and offset 0;
- the depth `z` and the colour;
- page 0: the picture being drawn, not the last frame's.

`ccBufferSampling::MakePacket` puts the strip at the view's Z for its depth
(+0x08). A depth of 0 (the reflex's) is a division by zero: the far plane.
The Z test is GEQUAL, so a strip at depth 1000 covers only what is at
least 1000 away: the figures in front keep sharp. The centre (+0x18,
+0x1c) moves the strip in half view sizes, less. The offset (+0x20,
+0x24) is added in 1/16 pixel, unscaled: it moves the strip in
sixteenths of the view's size.

**The port**:

- **The tasks.** `piney_stream::ending`'s `Str0580` and `Str0581`
  (`Task::Str0580`, `Task::Str0581`), with the walker `EventObj`, the
  parts and their draws (`PartDraw`). The objects are the scene's
  (`effect::SceneWorld`: `find_entry`, the node's world matrix).
- **The parts' draws.** `Stream` draws them on `sysLayer`. A puff is a
  `piney_effect::eff::Eff` of `str0580e`'s `EFF_x001`: when the player
  gives the effects, their assets read `str0580e` from the stream's
  archive. A rock is its model from the loaded `str0580e`
  (`draw::draw_rigid`, with the scene's lights and fog).
- **The shades** are `Ctrl::shades` (`piney_desktop::noiz::Sampling::shade`).
  `piney_draw::Wrap::Region` carries REGION_REPEAT. The software
  rasterizer reads it, and `piney-gs`'s shader fetches the texel `(u &
  MINU) | MAXU`, `(v & MINV) | MAXV` itself (`Params::REGION`,
  `region_repeat_reads_in_blocks`).
- **`Func_str0580` after its scene** is `Stream`'s tail. It runs before
  the next scene's task each frame, and the switch sets its `param[1]` to
  2 at once.
- **Requests.** The `divZ` goes out as `Request::DivZ`. Cue 699's
  transfer goes out as `Request::Transfer`, and the stream's own effects
  draw it as they draw the other tasks'.

## Stream 15's opening

`str0580`'s first 420 frames, as the game's own decode and draw run them
(the scene check below):

| frames | drawn |
| --- | --- |
| 1-60 | the sky (`OBJ_se1_5bac1`, the clouds `clo1` and `clo2`), the ground (`se1_5fl0`-`fl4`), the towers (`ob0`-`ob3`), Kite, the stone figure (`ex1xbode01`), `ex1xwtr1` (a flat disc 1000 across) |
| 61-69 | all of `se1_5` fading out, 0.1 a frame (`fl1`-`fl4` also take `fl0`'s: 0.81, 0.64, ...) |
| 70-120 | only Kite, the figure and the disc, on black |
| 121-140 | the sky and clouds fading back in, 0.05 a frame; the ground and towers stay at 0 for the rest of the scene |
| 141-315 | the sky, the clouds, Kite, the figure, the disc |
| 316-345 | the `se1_5` sky fading out, 1/30 a frame |
| 316-404 | the `se1_6` set (its floor `flo1`, sky `bac1`, clouds, moon `moo1`) fading in, 1/90 a frame |

- **The black.** The background is colour 0: `ccEventStream` sets it for
  the call.
- **The lower half** from frame 121 to about 330. The camera looks down at
  the figures from above, and there is no ground under them.
  - What lies there is the bottom of the sky. `MDL_se1_5bac1` is a closed
    shell: radius 23712, from z -4928 (its lowest vertex, the pole) to
    13624.
  - `TEX_se1_5bg1` runs from v 0 at the bottom pole to 1 at the top. Below
    the horizon ring (z -288, v 0.25) its rows go from brown to black.
  - The screen goes from the brown horizon to black at the bottom.
- **Nothing is clipped.**
  - At frame 145 every sky triangle on screen lies wholly inside the GS
    primitive space, so VU1 draws all of them.
  - The scene's fifteen effect objects (`EXT_ex1xsmk*`, `ccEffObj`s) keep
    pattern 0xffff and draw nothing.
  - The files hold no particles.
- **The `divZ`** (2000 from `Func_str0580`'s start) changes nothing here.
  In frames 1-65 one or two ground triangles on screen are not wholly
  inside. Their last vertex is nearer than 1000, so they are clipped at
  either value.

**The port** draws the same. Frame by frame through frame 420, these match
the game's: the draw list, each object's transparency and matrix, the
camera and the lights at the first object drawn. The lights needed a fix:
the distant light goes after the omni ones ([a scene](#a-scene)). The
black and the missing ground are the scene's own; the effect tasks draw
nothing of their own before frame 1225.

## Stream 24's effect task (MUT)

Stream 24 is Mutation's opening (event 101's `stream 24`), scene
`str0710`. `StreamDemoFuncTbl` (MUT main 0x00367210) names
`Func_str0710` (MUT main 0x0019c140-0x0019df6c) for it, a row Infection
does not have. It keeps the `ccStrEffectCtrl` and pass of the tasks
above, on the task's layer (100), with stream 15's walker and puffs
(`Func_str0581`'s, over its own table), and two blocks of text lines.

- **The effect file.** `str0710e` (else `str0710ep`) and its `EFF_x001`.
- **The table.** `eventObjTbl_0710` (MUT main 0x00366e30): cue 0 on
  `EXT_t0` (type 2, never asked for), 602, 603 and 604 on `EXT_t0` and
  607 on `OBJ_marker01` (each type 0, a burst), then 100000. `SetObj`
  and `ccEffPart0580::Create` are Infection's code (same instructions).
- **A block of lines** (0x298 bytes, new in Mutation): ten slots of 64
  bytes (+0x00 a `ccKanji`, `Init(3, 8)`; +0x08 the length; +0x0c the
  text; +0x38 x; +0x3c y), +0x280 the alpha, +0x284 its step a frame,
  +0x288 where the step stops, +0x28c the lines in use, +0x290 20 and
  +0x294 24 (the width per byte and the line height). The constructor
  (0x00187960) makes each slot's `ccKanji`; the task puts them on a layer
  of priority 1000 with `sysLayer`'s view, colour `ccSpriteColorTable[2]`,
  alpha 0.
- **`SetText(s)`** (0x00187bd0) empties the old lines, then splits `s` at
  `\n` into at most ten lines of at most 40 bytes, line `i` at (0, 24 i).
- **The fade** (0x00187ac0), each frame: nothing while the step is 0;
  else the alpha moves by the step and is held at its stop (the step then
  0), and every line's sprite alpha becomes `fptosi(128 alpha)`. While
  the alpha is not 0 each line is drawn: `ccKanji::Disp(text, -1, 1, 1)`
  at (x, y).

The pass: at scene frame 465 the shades go off (count 0); at 1050
`SetReflex(1.005, 0, 0x58808080)`; at 1155 the feedback and the shades
off. Then the cues, last queued first:

```text
602-610   SetObj(cue); a type-0 entry bursts puffs at its object
700, 710  block 1 (2): SetText of its text (Parody Mode's when
          saveData.parodyFlag), each line at x 256 - 20 n / 2 / 2, the
          block at y 24 i + 175 - 24 lines / 2 / 2; alpha 0 rising 0.05
          a frame to 1
709, 719  block 1 (2): alpha 1 falling 0.05 a frame to 0
600, 601  EntryFade(20, 0, 0x40000000), (20, 0x40000000, 0x80000000)
800       EntryFade(30, 0x80000000, 0)
999       EntryFade(1, 0x80000000, 0x80000000)
900       EntryFade(16, 0x80ffffff, 0x00ffffff)
901-904   EntryFade(10, 0x80ffffff, 0x00ffffff)
65        SetReflex(1.005, 0, 0x40808080), a fade over 15 ready
56        the shades and the feedback off
55        two shades SetShade(0, 896, 0, 7 - i, 7 - i, 250, 0x40808080);
          SetReflex(1.01, 0, 0x50808080), its depth 1000
else      the last digit (jump table MUT main 0x00366ea0): 1 noise on,
          2 off, 3 inversion on, 4 off, 5 SetReflex(1.02, 0, 0x40808080),
          6 the feedback fading, 7 and 8 a one-frame white flash
```

Then the parts, the two blocks' fades and draws, and the `Ctrl`'s
packets (noise at `SetNoize(20)`). The task starts with
`EntryFade(1, 0x80000000, 0x80000000)`. When the loop ends it turns the
feedback off, waits a frame and deletes itself.

## Mutation's other effect tasks

The rest of Mutation's new rows keep the same `ccStrEffectCtrl` and pass,
with no walker or parts but `Func_str1070`'s (below the table). Each is started for its
scene as the tasks above; "black over the frames left" is
`EntryFade(frameEnd - frameNow, 0, 0x80000000)` (1 frame when none are
left), and a transfer is `effTransferStr` at the note's object with a row
of Mutation's `charHeightTbl` (main 0x00365b40: 0 160, 6 180, 15 165, 17
210). A digit is the cue's last one through the task's jump table.

| task (MUT main) | set-up | frames | cues |
| --- | --- | --- | --- |
| `str0770` (0x0019df80) | fog (0, 4000, 0, 50, 0x008c5000); shades at 310, 620 | | 899 black |
| `str0780` (0x0019e9e0) | the same fog; shades at 1000, 3000 | 287, 2540 shades 310, 620; 2469 800, 1600 | 899 black |
| `str0820` (0x0019f5d0) | white for a frame | | 500, 510 transfers (160, 165); 900 white out over 20; 999 to white over the frames left; 5 grey out over 10 and `SetReflex(1.01, 0, 0x34808080)`; 15, 25 `SetReflex(1.01, 0, 0x50808080)` with a fade over 20 ready; 6, 16, 26 feedback fading |
| `str0821` (0x001a03a0) | white for a frame | 2 white out over 20 | 501, 502 transfers (210, 165); 900 white out over 20; 899 black |
| `str0880`, `str0885` (0x001a1050) | fog (0, 4000, 0, 60, 0x0078145a) | | 601-604 a hit mark, each the next of four rotations (0x00366ed0: (0, 30, 180), (0, 30, 0), twice); digits (0x00366f10) 1-4 noise and inversion, 5 `SetReflex(1.015, 0, 0x50808080)`, 6 off |
| `str0932` (0x001a1cb0) | fog (200, 4000, 0, 60, 0x0019140a) | 1700 `SetFog(0, 0, 0, 0, 0)` | 899 black; digits 1, 2 noise, 5 `SetReflex(1.02, 0, 0x48808080)`, 6 off |
| `str1040` (0x001a2880) | two text blocks | shades (below) | 700-730 a text; 709-739 its fade out; 800, 801, 809, 819, 829 fades |
| `str1041` (0x001a4930) | black for a frame | 2 black out over 30 | 899 black; 800 nothing; digits (0x00366f30) 1 noise and `SetReflex(1.02, 0, 0x40808080)`, 2-4, 5 `SetReflex(1.015, 0, 0x48808080)`, 6 off |
| `str1050` (0x001a5640) | | | 9, 7 transfers (180, 210); 899 black; digits (0x00366f50) 1-4, 5 `SetReflex(1.01, 0, 0x48808080)`, 6 off |
| `str1090` (0x001a8570) | | | 500, 501 transfers (160, 165); 900-950 white out over 6; 809-859 to 0x60 black over 40; 869 black at once; digits 5 `SetReflex(1.01, 0, 0x30808080)`, 6 off |
| `str9204`-`str9206` (0x001a92a0) | `Func_str9000`'s letterbox and fog | | the cue itself (0x00367030): 1-4, 5 `SetReflex(1.01, 0, 0x58808080)` with a fade over 60 ready, 6 fading |

The shades are two `SetShade(0, 896, 0, 8 - i, 8 - i, z_i, colour)`, the
second's depth a multiple of the first's.

`SetFog` with `near` equal to `far` divides 0 by 0; the EE gives the
largest float, so the fog term is held at its near value: `str0932`'s
frame 1700 clears the fog.

`Func_str1040` (stream 31) keeps two text blocks as the opening's, and
which one the next text goes to (+0x12, 0 first). Cues 700, 710, 720 and
730 `SetLines` (0x00187cf0: `last + 1` NUL-separated lines, here 4) the
current block from stream 31's subtitle records 16 to 19 (MUT main
0x00327640 and Parody Mode's 0x00327ec0, at `evStrMsgTbl[31]` + 192), then
place and fade it in as cue 700 does; 709, 719, 729 and 739 fade it out
and make the other block current. The blocks fade and draw after the
`Ctrl`'s packets. Its shades by scene frame:

```text
340             310, 620, colour 0x00808080; their alpha from 0 to 96 over
                20 frames (+0xec 20, +0xf0 (0 - 96) << 16 / 20, +0xee 96)
1331, 1752      1000, 1600, 0x40808080
1676            150, 300, 0x50808080
2400            335, 670, 0x50808080
2566            750, 1500, 0x50808080
3170            1500, 3000, 0x40808080
4146, 4306, 4725  1500, 1500, 0x50808080
621, 1586, 2106, 2961, 3411, 4416  none drawn
```

A new set leaves a fade in progress running over it. Fades: 809 to black
over 20, 800 from black over 20, 819 to 0x40 black, 801 from it, 829 black
over the frames left.

`Func_str1070` (0x001a6af0, stream 34) keeps a part group as stream 15's
tasks do, with a part class of its own: objects of `str1070e` (the chunks
named at MUT main 0x00321e30, `OBJ_xpart00`-`05`) circling the scene's
origin as they rise and spin. At set scene frames it empties the group
and starts creators, each on a block of MUT main 0x00321c00 (0x50 bytes):

```text
+0x00 f[14]      base and random pairs: radius, angle (deg), turn (deg a
                 frame), height, rise a frame, spin about y, spin about z
                 (deg a frame)
+0x38 rate       parts a frame in 1/4096; +0x3c its random part; +0x40 life
260  blocks 0 and 1 (16 at once; one every other frame for 300)
411  blocks 2 and 3;  505 blocks 4 and 5;  636 4 and 5;  781 block 6
```

A creator (`ccPartCreate`, 0x44; `Ctrl` 0x001a6510) steps its counters
as the ending's rock thrower does and makes each part (0x120) drawing
`rand()` for, in order: the radius, the angle (less 180, back by a turn
past 180), the turn, the height, the rise, turns about x and y in [-180,
180), the two spins, and its model (`rand() % 10` into MUT main
0x00366f70: a chunk and a scale). A part's `Ctrl` (0x001a63e0) turns its
angle (`ccRotate`), rises, spins about y and z, and is placed at `(r sin
a, r cos a, 0)` plus its place (`SetMatrix_PosRotXYZScale`) and drawn;
above 200 and out of view (`CheckBoundingBox`) it is gone. The shades:
261 two at 500 and 1000 (7, 6); 505 one at 3100; 636 two at 2500 and
5000 (8, 7); 781 two at 8500 and 11050; 932 none. Cues: 899 black over
the frames left; 1-4 noise and inversion; 5, 15, 25 `SetReflex(1.01, 0,
0x40808080)` with a fade over 10 ready; 6, 16, 26 fading. Outbreak's
`Func_str1070` is other code, not read.
`str9201` and `str9301` run `Func_str9101`'s (0x00189ed0).

`str7100`, `str8800` and `str0300` carry no Infection name on Mutation,
but their code is Infection's: `Func_str7100` at MUT main 0x0018c280,
`Func_str8800` at 0x00188e30 and `Func_str0300` at 0x0018e5c0 match
Infection's instruction for instruction, with the same calls and the same
`eventExNoteTbl` / `eventExNoteParamTbl` bytes. The only differences are
the data addresses and the cue jump tables (0x00365d20, 0x00365c90,
0x00365db0), whose every entry is moved by the function's own shift.

## The drain streams' effect tasks

- **`Func_str8000`** (0x00184460) runs the drain movies (`str8000`-`str8300`,
  streams 108-111) and Skeith's (`str9102`, stream 18). It is
  `Func_str0300` again, with these differences: 999's fade falls back to
  `frameEnd`, 75 has no case of its own, and 55 is the wide feedback
  (1.125, alpha 0x48). The last digits go through jump table 0x0034e250:
  - 1 and 2 the noise on and off;
  - 3 and 4 the inversion on and off;
  - 5 `SetReflex(1.015, 0, 0x50808080)`;
  - 6 the feedback off;
  - 7 and 8 the one-frame flashes.
- **`Func_str9000`** (0x00186af0) runs a member drained (`str9104`, stream
  20).
  - **At the start** it letterboxes the scene's view: `SetFrame(0, 30, 512,
    324, 256, 162, 1, 0.84375)` (rows 35-412). It also sets a fog it does
    not draw (`SetFog(1500, 12000, 0, 70, 0x001e1e1e)`). Then the objects
    are made, the bands scaling through the letterbox.
  - **The banner.** Unless `game` is missing or its status is 2 or 7, it
    makes a banner: a layer of its own (`Init(101, 0)`, the default view)
    and a `ccMask(128, 0)` on it, textured from `bossSkillNameTbl`
    (`xeffect`'s `TEX_detadrain`, row 0). Each pass, after the shades, the
    mask draws cell 0: 256 x 20 texels at (128, 10), `wv` = row x 320,
    transparency 1. That is the "Data Drain" title in the top bar.
  - **Cues.** 999 a fade to white over the frames left; 910 from white over
    12 frames; 989 to white over 4. The last digits go through jump table
    0x0034e2e0: 5 is `SetReflex(1.01, 0, 0x50808080)` with a 15-frame fade
    ready, 6 the feedback fading, and the rest as above.
- **`Func_str9001`** (0x00186210) runs stream 17 (`str9101`). Every cue
  goes through `Func_str9001sub` (0x00185e30): 999 a fade to white over
  the frames left. The last digits (0x0034e2b0) are as above, except that
  5 is `SetReflex(1.0, 0, 0x48808080)` with a 60-frame fade ready and 6
  the feedback fading.

- **`Func_str7100`** (0x00187ad0) runs the gate hack movie's town gate
  (`str71xx`). It has no fade at all. Each cue is taken by its last digit
  alone (jump table 0x0034e310, digits 0-6):
  - 1 and 2 the noise on and off;
  - 3 and 4 the inversion on and off;
  - 5 `SetReflex(1.015, 0, 0x50808080)`. It is set the same either way of
    the task's test of a state 2.
  - 6 the feedback off.
- **`Func_str8800`** (0x00185170) runs streams 112-119 (`str8801`-`str8808`).
  It is `Func_str8000`, except that its 5 is `SetReflex(1.005, 0,
  0x60808080)` and it has no 55 of its own (jump table 0x0034e280).

**The port**: `Str8000`, `Str9000`, `Str9001`, `Str7100` and `Str8800`.

- **The banner's texture** is `DATA.BIN`'s, not the stream's. The player
  gives it as `Options::skill_names` (`SkillNames`: the texture and palette
  objects and the height). `piney-game`'s `StreamPlayer` does, for the
  drain and event streams.
- **The banner** is a `piney_desktop::sprite::Sprite` mask on layer 101.
- **`stream_shot --banner`** gives it too.

## The tasks' fog

Several tasks set the scene's draw environment fog at their start:

| task | call | fog |
| --- | --- | --- |
| `Func_str0001` | `SetFog(7500, 20000, 0)` | black; not on `fogOffObj`'s four objects |
| `Func_str0090`, `Func_str0110` | `SetFog(200, 7000, 0x00141e00)` | green-blue; `Func_str0110`'s cue x3 sets it off |
| `Func_str0150`, `Func_str0350`, `Func_str0610` | `SetFog(1000, 7500, 0, 85, 0x00144870)` | not on `OBJ_sfp7bac2` |
| `Func_str9000` | `SetFog(1500, 12000, 0, 70, 0x001e1e1e)` | |
| `Func_str0581` | `SetFog(30000, 50000, 0)` (`str0581Fog`, 0x00377d08), again every pass | black; not on `fogOffObj`'s (0x0030bf60) `EXT_se1_6bac1`, `EXT_se1_6clo1_1`, `EXT_se1_6clo1_2` |

`ccDrawEnv::SetFog(near, far, nearRate, farRate, colour)` (0x00105820)
keeps `fMax = 2.55 (100 - nearRate)` and `fMin = 2.55 (100 - farRate)`.
It keeps `fogA` and `fogB` as the line from `fMax` at `near` to `fMin` at
`far`. The three-argument form (0x00105890) is rates 0 and 100: clear to
all fog. VU1 fogs each vertex of an object whose `fogSW` has bit 3 (as
`ccObj::Init` leaves it) with `F = fogB + fogA w`, held between `fMin`
and `fMax` at view depth `w`, towards `FOGCOL` (the colour's low three
bytes, R low).

**The port**: `Scene::fog` (`SceneFog`) and `Scene::fog_off`. Each model
carries the terms as `piney_draw::DepthFog`, and the renderer gives each
vertex its `F` by its depth as VU1 does (render.md).

## Skipping

`WaitEnd` (0x00197f40), each frame while a scene plays: in mode 2 (the
title) with `DESKTOP_FLG` 1, a cancel push skips; otherwise, unless the
reader is paused (`CheckPause`), the playing file's flag decides: 0x80 never,
0x40 a START push, else a cancel push (`saveData.assignPADcancel`). A skip
sets `canselFlag`, `ExitScene`, `eventMsg` -1 and `strEffectStopFlag`; the
decoder calls `EndScene` and the scene's task leaves at its next wake
without drawing; the decoder then goes on to the next file. So a skip ends
the scene playing, not the stream. (Mode 7, a stream viewer, skips on
circle and cycles `frameSpd` on triangle.)

## The gate hack's movie

`ccSetupGameCtrl` with `ccGame.setupMode` set (only `GtHackMenu`'s OK sets
it) starts `ccThExecuteStream` with 107 after `ccSndSQLoad`, breathes until
`strPtr` exists and `CheckPause` says the reader has reached the paused
record, then loads the field's files (`ccLoadResourceFL`), calls
`ResetPause` and waits for the stream's task to return, before it starts
the field's tasks. `ccThExecuteStream` plays 107 through
`ccRequestLoadStreamGateHack(107, game.town, game.field)` (0x00199f00):
`ccStreamInit`'s work by hand (`ccPcmSound::Open`, `strSeInit`, the
`ccStreamLoadPlay`), `ccSndStreamCtrl(107, sd, 0)`,
`RequestStrPlayGH(town, field)` (0x0019a0e0), `ccSndStreamCtrl(107, sd, 1)`,
and the teardown. `RequestStrPlayGH` builds the file lists from tables of
its own instead of `searchPreLoad`:

```
town < 0 -> 0; crisis: saveData +0x6772; E tables with English voices
read whole (ccLoadStreamOnMem), in order:
  str7000TblPre[0] (str7000e)
  str7000TblTown[town] (TownC in the crisis), its first record (str7100e)
  str7000TblAfter[a], its first record (str7404e), a = str7000Out[field]
      ({field, a} shorts ending at a negative field; 0 when not listed)
  stream 107's type -2 records (str7300), then its flag 16 records
the ccsTbl:
  the town row's second record (str7100: Mac Anu's gate, START skips)
  every record of stream 107 (str7200; str7300, on memory, flag 0xc1: the
      reader pauses before it, no skip, and it loops)
  the arrival row's second record (str7404: the party at field 19)
```

The loop ends when `ResetPause` lets `WaitEnd` out of its wait on the
pause: its skip path, `ExitScene` on the loop. `str7000Out` lists fields
19, 22, 23, 25, 27, 16, 44, 45, 46, 48, 50, 52, 66, 71, 73, 74, 76, 77, 91,
100-106 and 108, arrivals 0-26 in that order (`str7404` to `str7482`,
`str7447` three times); the first 19 are the areas `GateHackOutCcsName`
has a camera for.

The tables (INF main): `str7000TblPre` 0x0030e1a0, `str7000TblTown`
0x0030e1d0, `str7000TblTownC` 0x0030e2b0, `str7000TblAfter` 0x0030e390;
the `E` tables with English voices 0x0030f2b0, 0x0030f2e0, 0x0030f3c0,
0x0030f4a0. Outbreak and Quarantine have the `E` tables alone. The rows'
files are in the archive stream 107's header names, at its offset.

The port (`piney_stream::Stream::gate_hack`, `table::gate_hack_files`)
builds the same lists and plays them as any stream; the area's set-up
([field walk](field-walk.md#the-hacked-arrival)) holds until it returns
and calls `ResetPause` once the loop has played through once.

## The streams the new game and title play

| stream | files | frames | rate | what |
| ---: | --- | ---: | ---: | --- |
| 0 | `title1_st1` (STR1) | 410 | 60 | the title's intro, assembling the menu scene; skipped with cancel |
| 2 | `str0001e`, `str0001` (STR1) | 2040 | 30 | "Epitaph of the Twilight": the relic among clouds, the text; 68 s of PCM; START skips |
| 106 | `str6100` (STRCMN) | 397 | 60 | the ALTIMIT boot screen; 6.6 s of PCM; cancel skips |

## Checks

`tools/test_stream_rs.py fixture` runs the game's code in `tools/eemu.py`
and writes `crates/piney-stream/tests/stream_fixture.txt`;
`crates/piney-stream/tests/stream.rs` compares the port:

| what | how much | compared |
| --- | --- | --- |
| `ccStreamInit` + `RequestStrPlay` (`searchPreLoad` native) | 134 streams x 2 languages | the header, the `ccsTbl`, the files read whole, in order |
| `WaitEnd` | 180 cases: mode, `DESKTOP_FLG`, flags, pushes | `canselFlag` |
| `DecodeSetup` + `InitScene` on the files | streams 0, 106, 2, 15 (`str0580`), 6 and 18 (`str9102` over the desktop's `str8000e`, `str8001e` and `str8800e`) | every node in `Draw`'s order, its layer and parent |
| `DecodeFrameSection`, `SetView`, `ccStreamDrawLayerList::Draw` (`ccModel::Draw` hooked) per frame | 410 + 397 frames, stream 2's first 200, stream 15's first 420, stream 6's first 60 and stream 18's 350 | the frame, the end, the notes; each object drawn (up to 273 a frame in stream 18): name, transparency and `lwMatrix` bit for bit; `world_screen` bit for bit (`SetMatrix_PosRotXYZDebug` and `SetView` in VU0's and the EE's arithmetic, `piney_desktop::camera`) and `SetLightMatrix` (worst 5e-7) |

`tools/test_stream_rs.py gatehack` writes
`crates/piney-stream/tests/gate_hack_fixture.txt`: `RequestStrPlayGH` run
natively on stream 107's `ccStreamLoadPlay` (`ccStreamInit(107)`), the
threads, the buffers, `WaitEnd` and the teardown hooked, for towns -1 to 4,
the fields `str7000Out` lists and eight it does not, in and out of the
crisis, in both languages (768 runs): the `ccsTbl` and the files read
whole, in order. `stream.rs`'s `gate_hack_stream_matches_request_str_play_gh`
compares `table::gate_hack_files`: none differ.

`tools/test_stream_rs.py effects` writes
`crates/piney-stream/tests/str0001_fixture.txt` and
`crates/piney-stream/tests/str0001.rs` compares:

| what | how much | compared |
| --- | --- | --- |
| `Func_str0001` run natively, one pass a frame (`Breath` hooked), with `ccRasterNoize`, `ccBufferSampling`, `ccScFade`, `ccMakePacketDrawBuffTrans`, the layer lists and `rand` native; its cues from `str0001`'s F_Note records, `frameNow` = min(step + 1, frameEnd), state 0x40 until the second gap frame; every layer's DMA list decoded (VIF DIRECT, GIF PACKED, A+D) through a GS register model | all 2043 passes of stream 2: 4,164 primitives, 10 frames of noise | the steps the port raises each cue on; per step every primitive in GS order: layer, kind, shading, texture (last frame, or the copied rectangle and its size), filter, wrap, TFX, TCC, ALPHA, alpha and Z tests, Z write, scissor, and each vertex's XYZ, UV and RGBA in GS units, bit for bit; `rand`'s state after each pass |

`tools/test_stream_rs.py effects str0300` (and `str0120`, `str0090`,
`str0110`, `str0130`, `str0150`, `str0240`, `str0250`, `str0301`,
`str0305`, `str0350`, `str0570`, `str0610`, `str8100`, `str9102`, `str9104`,
`str9101`, `str7100`, `str8801`) write `str0300_fixture.txt` (and the others') the
same way, each pass's
lines as a count and an FNV-1a 64 of them (`STREAM_FULL=1` writes the
lines), plus what a pass starts: `effHitMarkStr` hooked (each cue's object
a `ccObj` standing at its index on x, so the mark names it) and the view's
`divZ`; the task's end is its `ccDeleteThread` (hooked to stop the run),
its packets' work an arena of their own emptied each pass; `fontTex` is
set (`ccSprite::MakePacketStr` draws nothing without it) and
`ccSprite::SetTex` gives a marker texture the GS model names `ccs`;
`crates/piney-stream/tests/effects.rs` plays streams 10, 5, 3, 4, 6 to 9,
11 to 14, 16, 17, 18, 20, 109 and 112 for real (and `Func_str7100` alone)
and compares:

| what | how much | compared |
| --- | --- | --- |
| `Func_str0300` | all 1,257 passes of stream 10: 1,449 primitives (noise, feedbacks of scale 1.015 and 1.125, the inversion, the flashes) | as above, by hash; `rand` after each pass |
| `Func_str0120` | all 3,103 passes of stream 5: 3,193 primitives, 8 hit marks, the `divZ` change | as above; each mark's object and rotation bits, on its step; `divZ` |
| `Func_str0090` | all 1,203 passes of stream 3: 637 primitives (the feedback, then at alpha 0) | as above |
| `Func_str0110` | all 1,496 passes of stream 4: 1,474 primitives (noise, feedbacks timed and not, the inversion) | as above |
| `Func_str0130` | all 1,043 passes of stream 6: 67 primitives (the letterboxed fade) | as above, `rand` after the letterboxed bands' `Init` |
| `Func_str0150` | all 2,203 passes of stream 7: 32 primitives, the transfer | as above; the transfer's object, height and layer (`effTransferStr` hooked) |
| `Func_str0240` | all 1,583 passes of stream 8: 82 primitives, 8 hit marks | as above; each mark's object and rotation bits |
| `Func_str0250` | all 3,563 passes of stream 9: 4,586 primitives (noise, feedbacks plain, timed and turned, the inversion, the white fades), a hit mark | as above |
| `Func_str0301` | all 3,463 passes of stream 11: 32 primitives, a hit mark | as above |
| `Func_str0305` | all 1,303 passes of stream 12: 158 primitives, a transfer | as above |
| `Func_str0350` | all 2,803 passes of stream 13: 40 primitives | as above |
| `Func_str0570` | all 1,828 passes of stream 14: 3,836 primitives (noise, feedbacks, the inversion, the frame-1455 fade), a transfer | as above |
| `Func_str0610` | all 3,598 passes of stream 16: 2,782 primitives, transfers 160 and 210 high | as above |
| `Func_str8000` | all 353 passes of streams 109 (a small enemy's drain) and 18 (Skeith's): 642 primitives each | as above |
| `Func_str9000` | all 353 passes of stream 20: 884 primitives, 352 of them the banner | as above; the banner's texture a marker (`SetTex` hooked: 256 x 128, PSMT4) |
| `Func_str9001` | all 1,103 passes of stream 17: 1,635 primitives | as above |
| `Func_str8800` | all 363 passes of stream 112: 427 primitives | as above |
| `Func_str7100` | all 227 passes over `str7100`'s cues: 240 primitives | as above, the task alone (it plays inside the gate hack's movie) |

`tools/test_stream_rs.py effects str0580` and `effects str0581` write
`str0580_fixture.txt` and `str0581_fixture.txt` the same way, with more
hooked:

- the tables' objects stand in by name: a `ccObj` at x the name's bytes
  summed times 10, y its length times 100, z 50;
- `GetChunkAdrs` gives four rocks, and `ccObj::Init` on a rock records its
  model;
- `ccEff::Draw` and `ccObj::Draw` record each part's draw: a puff's place,
  pattern, scale, transparency and colour, a rock's model and `lwMatrix`;
- each note's object, for the transfer.

`Func_str0580` gets `param[1]` 2 in the step after its scene's last frame,
and `Func_str0581` 1 with the scene gone after its gap frame.
`crates/piney-stream/tests/ending.rs` runs the port's tasks over the same
cues and stand-ins, and plays stream 15 for real:

| what | how much | compared |
| --- | --- | --- |
| `Func_str0580` | all 1,950 passes of `str0580` (and after it): 1,534 primitives (noise, the feedbacks with their offset, the white fade), 89,116 part draws (200 puffs, 138 rocks) | as above, by hash; `rand` after each pass; the part draws each pass, by hash; the `divZ` it sets |
| `Func_str0581` | all 3,218 passes of `str0581`: 9,177 primitives (the shades, the feedbacks timed and at depth, noise, the inversion, the fades), 2,131 part draws | as above; the transfer's object and height |
| stream 15 | `Stream` over the scene's own objects, to the end | `Func_str0580`'s primitives and `rand` to `str0580`'s last frame, its cues, the `divZ` of both tasks, the one transfer |

`tools/test_stream_rs.py subtitles` writes
`crates/piney-stream/tests/subtitle_fixture.txt`: `ccEventStream(num, 1)`
itself run in eemu (the stream's task, the thread calls and the draws
hooked; `ccMessage`'s constructor built by hand as
`tools/test_desktop_rs.py` builds it; the save the game's boot save with
the player Kite), its loop fed the scene files' notes through
`ccSetStreamDemoNote` on the steps the port draws their frames, the
decoder's -2 at a following scene's start, and WaitEnd's -1 after a skip;
per step the draws of `ccMessage::Disp` (each cell's `MakePacket`: code,
position, size, grid, alpha; each line's `ccKanji::Disp`: text, glyph
count, position, colour; the send), as a count and an FNV-1a 64 (`subtitles
--full NUM` prints them), and `Change`'s arguments.
`crates/piney-stream/tests/subtitles.rs` plays each stream for real
through `EventStream`:

| what | how much | compared |
| --- | --- | --- |
| the subtitles | 19 runs: streams 3-5 and 7-16 with Movie Text on; 3, 11 and 16 with it off; 3 in Parody Mode; skips in lines of 3 and 5 | every step's draws, bit for bit, on every step whose draws change; the step the call returns after; nothing drawn after |

## Effect nodes

A clump node whose object follows to an Eff chunk is a `ccEffObj`.
`ccAnm::SetAnm` makes one with `ccEffObj::Init` (main 0x0013c330):
`ccEff::Init(chunk, 1)`, pattern 0, and from the Eff chunk's +0x10 (the
node's Obj2 flags, which `Decode_Eff` copies) bit 0 the coordinate's
succession bit and bit 1 the "part" bit (+0x104). `InitScene` and
`ResetScene` end every one (`EndAnm`: `localtp` 0, the pattern 0xffff,
stopped). `DecodeF_Obj` (0x0014eb80) handles a record for one:

- **Starting.** A stopped node with the part bit starts (`StartAnm`,
  pattern 0xfffe) when the record's place is off the origin. Any other
  starts when its `localtp` was 0 and the record's transparency is not.
- **Animating.** Then `Animate(1)` (0x0013c510): a stopped node stays so,
  one just started takes pattern 0, else the pattern moves on 1; past the
  chunk's `patNum` it stops.
- **The rest.** The matrix becomes the place alone (a translation), and
  `localtp`, the Eff's transparency, its scale (the record's x and y) and
  its turn (the record's z, in radians) come from the record.

`ccStreamDrawLayerList::Draw` then draws each running node
(`DrawNoAnm`, 0x0013c490): the Eff at its `lwMatrix`'s translation with
its pattern. Four streams hold Eff nodes: str0001, str0570, str0580 and
str9104. Only str0001's move: its 181 `EFF_srzsmo*` (all `EFF_srzsmo1` of
`str0001e`) are the clouds the relic stands among, each moved by a record
every frame. str0580's 15 smokes keep still at the origin and draw
nothing (above).

**The port**: `Scene::effs` (`EffNode`), `f_obj_eff`, and
`Stream::draw_effect_nodes`. The last draws each running node through
piney-effect's `Eff` (the chunk read into the effects' assets), when the
player gives the stream its effects. The fixture (`tools/test_stream_rs.py
fixture`) records every `ccEff::Draw` of `DrawNoAnm` in eemu: the pattern,
place, scale, turn and transparency, as a sorted hash a step. Stream 2's
first 200 steps have up to 181 a step, starting, stopping and restarting,
and the port matches every step.

## Unknown

- VU1's clip at the view's `divZ` (1000; 500 after stream 5's cue 12; 2000
  and 3000 in stream 15) is modelled for the unlit rigid program's
  triangles (cut at the near plane when the last vertex is nearer than
  `divZ`, else dropped); no picture of a close-up was compared.
- The lights' priority order is checked against the game in streams 15
  and 6 only. In streams 3, 5, 7, 8, 10, 11 and 16 it also changes which
  three lights a model gets, and those pictures are not compared.
- What the game's picture of stream 15's opening looks like: the check is
  of the draw state. The dark under the figures is the sky texture's
  bottom as the port renders it.
- Stream 15's parts are checked as draws (a puff's place, pattern, scale,
  transparency and colour; a rock's model and matrix), not as packets:
  `ccEff::Draw` and `ccObj::Draw` are hooked. The port draws a puff through
  `piney_effect::eff::Eff` and a rock as the scene's models are drawn.
- Stream 15's switch: the port sets `str0581` up in the step after
  `str0580`'s last frame, so `Func_str0580` sees `param[1]` 2 at once. The
  game's loading between the scenes takes frames the port does not model.
  `str0581`'s note on its frame 0 (cue 900, a fade from white over 45) is
  never given by the check, which queues a step's cues after the pass
  before it; the port's stream gives it to the task's second pass.
- The fog's pixels are not checked: the check is of the packets. The
  GS's bilinear filter at the feedback's bottom edge (texture rows 447 and
  448, the next VRAM) and what a noise band's copy reads past the draw
  buffer are not modelled.
- The `rand` state when stream 2 starts on the PS2: every task drawing
  numbers before it moves it (and `ccParticleGenerator::Generate` can while
  the stream plays), so the noise matches the game only from a known
  state; the check starts both at the boot value 1.
- The task's timing is from the thread priorities and the code, not a run
  of the game: its first pass one frame before the scene's first, its last
  in the first gap frame. With a scene that another follows (flag 4) the
  old task's last frames are not modelled (except `Func_str0580`'s, which
  outlives its scene).
- How many frames the reading and decoding take before the first frame and
  between scenes (the port has none), and where `ccGetStreamFrame` falls
  within a frame against the caller's task.
- The pause (`flag` 1) and its release: the port's `ResetPause` comes
  once the Chaos Gate's loop has played through (the set-up loads nothing
  meanwhile), where the game's comes when `ccLoadResourceFL` returns.
- `strse` of `strSndTbl` (read into `ccSnd +0xe4` by `strSeInit`): no
  stream has one, and `ccSndStreamSE` does nothing for events 1-3.
- When `ccEventStream`'s loop sees the stream's end against the stream's
  last blank frame: the port draws the window on every frame the stream
  yields and none after (the window has closed by then in every stream).
- Direct and spot light records (no Infection stream has them).
