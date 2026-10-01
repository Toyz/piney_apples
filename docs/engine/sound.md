---
title: Sound - effects, music and the IOP sound driver
status: partial
volumes: INF
covers: INF SLUS_202.67:0x00179c10 ccSeOn, 0x00179cb0 ccSeOnNote, 0x00179d90 ccSeOn3D, 0x0017a140 ccSeOn3DLoop, 0x0017a620 ccSeOffLoop, 0x001794b0 ccSetMainVol, 0x001794f0 ccSetBgmVol, 0x00179540 ccSetSeVol, 0x00179630 ccSndChangeOption, 0x00181010 ccSoundMain, 0x0017bac0 waterTest, 0x0017c3c0 bgmChurch, 0x0017c6d0 bgmBreed, 0x0017ad70 ccPortVolSet, 0x001798f0 ccSqPlay, 0x00179aa0 ccSqStop, 0x00179b50 ccSqFade, 0x00181250 ccSound::ccFade, 0x001834e0 ccSndChangeData, 0x0017b020 ccSndBgmCtrl, 0x001821d0 ccSndSQLoad, 0x00183380 ccSound::bgmChange, 0x0017de80 ccSndEvRequest, 0x00168960 ccSetupGameCtrl, 0x00167580 ccGame::CheckSceneReplace, 0x00307bc0 sqDataField, 0x00307bf0 playTypeTbl, 0x00309f60 sqVolTblField, 0x00307c20 sqDataDungeon, 0x00307d28 dungeonPlayType, 0x0030a100 sqVolTblDungeon, 0x00307f00 sqDataEvent, 0x00308a90 eventPlayType, 0x0030a3d0 sqVolTblEvent, 0x00307d40 sqDataTown, 0x00309f90 sqVolTblTown, 0x00182050 ccSndCommSeLoad, 0x00181ee0 spuInit, 0x001816f0 ccSound::sdCommand, 0x00182dc0 ccSoundRpc, 0x0017e6e0 wavPlay, 0x00308db0 seData, 0x00181440 ccSound::ccSceneFade, 0x001811f0 ccSound::gameInterrupt, 0x0017ae10 ccAllSoundOff, 0x00183080 initBeforeLoad, 0x0017e810 ccEvVoiceRequest, 0x0017eeb0 ccVoiceRequest, 0x0017e290 ccWordsPlay, 0x0017e350 skillVoicePlay, 0x0017eca0 evVoicePlay, 0x0017ee40 ccEvVoiceStop, 0x001800e0 ccVoicePgFood, 0x00179f50 ccSeOn3DNote, 0x0017a370 calcVel, 0x0017a4c0 calcPan, 0x0017a6b0 ccSeOnPCStep, 0x0017a7f0 seHitAttr, 0x0017aa20 ccSeSetParamSPC, 0x0017aaf0 ccSeSetParamPC, 0x0017abd0 ccSeSetParamEnemy, 0x0017ac70 ccSeSetParamInu, 0x001830c0 initAfterLoad, 0x00183fb0 ccSound::strSeEnd, 0x0017bf20 tobjSeLoopStart, 0x0017c0d0 tobjSeLoop, 0x003789dc looptest; INF gcmn.prg:0x00572860 _ccSkillRequest (its ccWordsPlay), 0x005a17e0 ccSpcShoutOperationName, 0x00493370 ccBoss04::OnThinkPrediction, 0x00639560 spc0SeData, 0x00639d40 spcSeTbl, 0x0063a850 enemySeTbl, 0x0063a8a0 inuSeData; INF SLUS_202.67:0x0017d190 ccSndStreamCtrl, 0x0017caa0 ccSndStreamSE, 0x0017cb20 ccSndStreamBGM, 0x001799b0 ccSqPlayVol, 0x00183f40 ccSound::strSeInit, 0x00183fb0 ccSound::strSeEnd, 0x00180b30 ccSndMoviePlayer, 0x0030b950 strSndTbl; INF desktop.prg:0x004072c0 Audio_control::ChangeWeve, 0x0042b6f0 Wave; INF MODULES/SNDBASE.IRX:0x0a58 ccSoundFunc, 0x031c ccSoundFunc2, 0x07e4 bgmChange, 0x3564 setModuleContext, 0x3860 ATick, 0x27d4 ccSetSq, 0x257c ccSetHdSynth; INF MODULES/MODMIDI.IRX; INF MODULES/MODHSYN.IRX; INF MODULES/SEWORDS.IRX:0x0e24 BgmSetVolumeDirect
worklog: 42
---

# Sound - effects, music and the IOP sound driver

The EE decides what to play and sends it to the IOP; there SNDBASE.IRX runs
Sony's CSL modules - MODMIDI.IRX, a MIDI sequencer, and MODHSYN.IRX, a
"hardware synthesizer" that turns MIDI messages into SPU2 voices - on a
4,167 us timer. Sound effects are MIDI notes sent straight to the
synthesizer; the music is MIDI sequences (`.sq`) played over sample banks
(`.hd` + `.bd`, [the sound banks](../formats/snddata.md)). Only the voice
lines and the two tracks of `VOICE/BGM.BIN` are streamed PCM, by
SEWORDS.IRX ([voice](../formats/voice.md)).

The executable's sound tables are read for each volume into the build
(`piney_data::sound::tables_of`, piney-gen's `placement::sound`), and the
audio takes the disc's own. The voice tables (the events', the field's,
the food's, the skill words' and the file names, Japanese and English)
are piney-gen's `voice` group (`piney_data::tables::voice`), read from
each volume's own code.

The port is `crates/piney-audio`: `driver` (the EE rules below), `se3d`
(the positioned sound effects and the animation notes' sounds, with
setbl.cpp's tables in `setbl`, read by piney-gen's `placement::sound`),
`midi` (MODMIDI), `hsyn` (MODHSYN), `seword` (SEWORDS's channel 0, the
voice), `spu` and `reverb` (the SPU2).

How each part is checked:

- The EE rules: the game's own `ccSeOn`, `ccSeOnNote`, `ccSndChangeData`
  with `ccSndBgmCtrl`, the volume setters with `ccSndChangeOption` and
  `sdCommand`, the jukebox change with its fade frames, and `ccSndSQLoad`
  with `ccSqPlay`, `ccSqStop`, `ccSqFade`, `ccSetMainVol` and
  `ccAllSoundOff` over the sound task's frames (`ccFade`, `ccSceneFade`),
  run in `tools/eemu.py` by `tools/sound_ee.py`, for every sound effect,
  every jukebox row and a set of volume, jukebox and title cases; the Rust
  driver sends the same (`crates/piney-audio/tests/driver.rs`). The voice
  requests likewise (`tools/sound_ee.py voice-fixture`,
  `crates/piney-audio/tests/voice.rs`).
- The positioned sound effects: `ccSeOn3D`, `ccSeOn3DNote`, `calcVel`,
  `calcPan`, runs of `ccSeOn3DLoop` and `ccSeOffLoop`, runs of TOBJ's
  `tobjSeLoopStart` and `tobjSeLoop` among other loops, and the four
  `ccSeSetParam*` with `ccSeOnPCStep` and `seHitAttr`, run in
  `tools/eemu.py` (gcmn.prg loaded) by `tools/test_sound3d_rs.py` against
  `crates/piney-audio/src/se3d.rs`: every row of `seData`, random cameras
  (none, types 0, 1 and 2, a view point straight below the eye) and points
  near and far, beside and behind, on the eye's x line and at the eye;
  every character id, enemy category and ground, params past their tables'
  ends - 50,000 cases of each of the nine checks, the bytes sent (and the
  slots, and the functions' returns) equal, 0 mismatches. What they send,
  played by MODHSYN.IRX in `tools/iopemu.py se3d-fixture` and by the port's
  synthesizer (`crates/piney-audio/tests/hsyn.rs`): every sound effect so
  sent, pans 0-126 with and without the bend, loops' note ons and offs by
  id, an override waiting for the next note on - 350 cases, every register
  write of 985 ticks equal.
- The areas: `ccSndSQLoad` for every row the town (10), a field (every
  field type with every weather `SimGenerateCode` leaves it, 64, and
  Piros's row, 3), a dungeon (every type, 11, and the same dungeon again)
  and the event banks (every row, 123, from a field and from a dungeon)
  can pick, each followed by `ccSndBgmCtrl`; the hold of `sound 10` on a
  field and a dungeon; battles starting and ending (`bgmChange`) on
  fields and dungeons of every play type, on a Grunty and in an event
  bank - 346 scenarios in the same fixture, with the sound task's frames
  running: 0 mismatches. SNDBASE's side of `bgmChange` is read from its
  code, and `crates/piney-audio/tests/render.rs` plays story area 14's
  field music and a battle through it.
- The sequencer and the synthesizer: models (`tools/midi.py`,
  `tools/hsyn.py`) checked against MODMIDI.IRX and MODHSYN.IRX themselves,
  run in `tools/eemu.py` by `tools/iopemu.py`, then the Rust ports checked
  against the models (`tests/midi.rs`) and against the modules' own
  register writes (`tests/hsyn.rs`).
- SNDBASE.IRX's wiring (ports, attributes, order of loads) is read from
  its code, not run.
- SEWORDS.IRX's channel 0 streaming a voice line: the module run in
  `tools/eemu.py` by `tools/iopemu.py voice-fixture`, the stream model
  (`crates/piney-audio/src/seword.rs`) playing the same samples.

## The IOP side (SNDBASE.IRX)

```
timer 4167 us -> ATick (0x3860):
  copy the EE's MIDI bytes (msinBf_T -> msinBf_R) if the EE sent any
  sequencer 0 or 1 flagged playing but stopped by itself -> resetSqData
  sceMidi_ATick(midiCtx)        sequencers 0-2 -> stream buffers 0-2
  sceHSyn_ATick(synthCtx)       ports 0-3 -> SPU2 registers
```

`setModuleContext` (0x3564) wires the modules together:

| synthesizer port | input | bank | attributes (`ccSetPortAttr`) |
| ---: | --- | --- | --- |
| 0 | MIDI bytes from the EE (`sceMSIn_PutMsg`) | the common SE bank (`commseTbl`), `.bd` at SPU 0x5010 | priority 0x40, 16 voices |
| 1 | stream buffer 0 (sequencer 0) | the loaded music bank, `.bd` at `commseTbl[2] + 0x5020` | priority 0x10, 32 voices |
| 2 | stream buffer 1 (sequencer 1) | the same bank | priority 0x10, 32 voices |
| 3 | stream buffer 2 (sequencer 2) | the same bank | priority 0x10, 32 voices |

`sceHSyn_Init` and `sceMidi_Init` both get 4167. `ccSetSq(addr, i)`
(0x27d4) loads a `.sq` into sequencer `i`, routes all 16 channels and
sysex to stream buffer `i` (`outPort[ch] = excOutPort = 1 << i`), selects
MIDI block 0 and locates to tick 0.

RPC commands (`ccSoundFunc`, `ccSoundFunc2`; the low nibble is the port or
sequencer): 0x00 play, 0x20 stop (`sceMidi_MidiPlaySwitch`), 0x30 locate,
0x40 `ccSetSq`, 0x110 locate then play, 0xa0 port attributes, 0xb0 port
volume (`sceHSyn_SetVolume`), 0xd0 all sound off, 0xe0 all notes off,
0x100 output mode, 0x120 stop all sequencers (`sqAllStop`), 0x140 all
sound off (`allSoundOff`, 0x2e5c: on each of the four ports all notes off,
all sound off and volume 0, then `sqAllStop` - the music stops too),
0x9050 give a port a bank (`sceHSyn_Load(ctx, port, spuAddr, hdAddr)`),
0x9210 / 0x9310 load the SE bank / a music bank from the disc.

A music bank load (`ccSQDataLoadCD`, 0x1e44) resets the controllers of
all four ports, stops the sequencers, reads `.hd` + `.sq`s into IOP
memory, streams the `.bd` into SPU RAM, then on every port: reset
controllers, all notes off, all sound off, volume 0 - sound effects too.

## Sound effects

`seData` (0x00308db0) is 237 `SETBL` rows of 8 bytes:

```
SETBL
  +0x00  s8   progNo       program of the SE bank
  +0x01  s8   port         0 in every row
  +0x02  s8   ch           MIDI channel: 0, 2 or 15
  +0x03  s8   note
  +0x04  s8   velocity
  +0x05  s8   dummy        127 in every row
  +0x06  s16  decay        distance scale for the 3D forms
```

`ccSeOn(n)` sends two messages to port 0: `Cc progNo` (program change on
channel `ch`), then `9c note velocity` (note on). `ccSeOnNote(n, note)`
sends the same with `note` clamped to 0..127. Nothing else chooses the
voice, the pitch or the volume: the synthesizer does (below), from the SE
bank's program.

### Positioned sound effects

`ccSeOn3D(n, pos)` (0x00179d90) plays row `n` at a point of the world
(`float pos[4]`: x and y on the ground, z up) as heard from the active
camera, `activeCamPtr` (0x0037896c, a `CAMERA`: `pos` +0x00 the eye,
`view` +0x10 the point it looks at, `type` +0x5c). The desktop uses none
of this. All the arithmetic is EE single precision
([the field](field.md), "EE floating point"), the library functions the
game's own:

- `calcVel(pos, se)` (0x0017a370): 127 with no camera (`activeCamPtr` not
  above 0). Else `d = sqrtf(dx*dx + dy*dy + dz*dz)` from the eye (`mul.s`,
  `mul.s`, `adda.s`, `madd.s`) and `v = (int)(velocity * (max(decay * 5500
  / 256 - d, 0) / 10) / 100)` (`fptosi`); -1 when `v` is 0 or less, else at
  most 127. So `v` is `velocity * (reach - d) / 1000` with the reach
  `decay * 5500 / 256` - 1,375 to 5,500 for the table's decays of 64 to
  256: at or above the row's velocity out to 1,000 short of the reach,
  falling to nothing over the last 1,000.
- `calcPan(pos)` (0x0017a4c0) returns `pan << 16 | cdeg`: `cdeg` is
  `RAD2DEG(atan2f(dy, dx))` of the point from the eye less the same of the
  view point, as 16-bit angles (`RAD2DEG`, 0x001dab50: `(short)(int)(32768
  (pi + r) / pi - 32768)`, counter-clockwise from +x), and pan `(int)(63 -
  63 sinf(DEG2RAD(cdeg)))` (`DEG2RAD`, 0x001dabb0: `pi s / 32768`): 0 a
  quarter turn counter-clockwise from the view (which the synthesizer puts
  hard left), 126 a quarter turn clockwise (hard right), 63 ahead and
  straight behind. With no camera 63 and 0; likewise in the eye view
  (type 1) when the point's own angle is exactly 0 (dy 0, dx not
  negative).
- The velocity is `min(calcVel, se.velocity)` - the row's own velocity
  out to 1,000 short of the reach - also stored in `ccSound.testVelocity`
  (+0x148), which nothing reads. Below 0 nothing is sent; else, on the
  row's port (0):

```
C<ch> prog                       program change (sceMSIn_PutMsg)
F9 01 <ch> pan 00                the pan of the channel's next note on (sceMSIn_PutHsMsg)
F9 02 <ch> 78 3F                 its bend, 8184 - only when 16385 <= cdeg < 49152 (behind)
FD 10 <ch> note 00 velocity 00   note on, id 0
```

`ccSeOn3DNote(n, pos, note)` (0x00179f50) is the same with `note` for the
row's, sends nothing at all for a note below 0 (where `ccSeOnNote`
clamps), and leaves `testVelocity` alone.

`ccSeOn3DLoop(n, pos)` (0x0017a140) returns -1 and sends nothing when
`calcVel` is -1 - here the velocity is not capped by the row's - or when
the eight slots of `ccSound.loopID` (+0x65, `char[8]`) are all taken
(none is -1); else the first slot holding -1 takes its own index `i`, the
messages go out with `FD 10 <ch> note i velocity 00`, and it returns `i`.
`ccSeOffLoop(n, id)` (0x0017a620) sends `FD 10 <ch> note id 00 00`, the
synthesizer's note off of that note and id, and sets `loopID[id]` to -1
without checking `id`. The slots are -1 from the constructor and after
every `ccSndSQLoad` (`initAfterLoad`, 0x001830c0, on each of its three
ends). One effect loops, SE 230 (`ccBoss03::OnThinkLeafGrow`, which keeps
the id and ends it unless it is -1); `ccSound::strSeEnd` (0x00183fb0) ends
the loops listed in `loopNum` (+0xf4).

A field's TOBJ (field.md) hums with `seData[44]` (program 31, channel 15,
note 60, velocity 64, decay 128) through two functions of its own.
`tobjSeLoopStart(pos)` (0x0017bf20), from `TOBJ::Init` for Δ's airship
and Λ's whale, does nothing when `ccSnd +0x133` is set or the eight slots
are taken; else the first free slot takes its index, which goes to
`looptest` (0x003789dc), and it sends the program change, `FD 10 15 60 i
127 00` and `FD 00 15 60 i 00 00`: the loop starts silent. `+0x133` is set.
`tobjSeLoop(pos, rate)` (0x0017c0d0), from `TOBJ::Draw` while its
transparency `rate` is not 0 (Ω's sleigh calls it too, with no hum
started), does nothing unless `+0x133` is set; else its volume is
`calcVel`'s with a reach of `decay * 15000 / 256` (7,500, not 5,500),
clamped to 0-127 and not capped by the row, times `rate`, and it sends
`FD 01 15 60 i pan 00`, behind the camera `FD 02 15 60 i 120 63`, then `FD
00 15 60 i vol 00`. `+0x133` is cleared by the constructor, by
`initBeforeLoad` (so every `ccSndSQLoad`) and by the desktop's
`ccSndChangeData`; nothing ends the note but the load's sound off. The
port: `se3d::tobj_se_loop_start`, `se3d::tobj_se_loop`, sent by the field
as `Op::SeLoopStart` (once, after the area's `ccSndSQLoad`) and
`Op::SeLoop`.

What the synthesizer does with these ("Controllers and messages" below):
an override is one-shot, spent by the channel's next note on that starts a
voice - a plain `9n` one too, if the 3D note on started none. The bend of
8184 is 8 below the centre, which every split's bend range of 256
truncates to 0: behind the camera sounds as in front (the module's writes
are the same with and without it, 36 pairs measured).

### The animation notes' sounds: ccSeSetParam*

A character's animation notes 1 and 2 call one of four functions with the
note's `param` and the character (`ccChar`); each picks a row of
setbl.cpp's tables (gcmn.prg) and ends in `ccSeOn3D` or `ccSeOn3DNote` at
the character's `pos` (+0x40). `param` is not decoded: it is the row.

```
SE_NT                      8 bytes, little-endian
  +0x00  s32  code         the seData row; -1 plays nothing
  +0x04  s8   note         0: ccSeOn3D(code); else ccSeOn3DNote(code, note)
  +0x05  s8   velocity     read by nothing
  +0x06  s16  dummy
```

| function | called by | rows | the footstep |
| --- | --- | --- | --- |
| `ccSeSetParamSPC(param, ch)` 0x0017aa20 | `ccPlayer::CheckNote` (Kite), `ccFellow::CheckNote`, `ccSkillCheckNote` | `spcSeTbl[base->id]` (0x00639d40: 19 pointers, to `spc0SeData` ... `spc17SeData`, 13 rows each; 18 NULL) | param 0 |
| `ccSeSetParamEnemy(param, ch, category)` 0x0017abd0 | `ccEnemyCheckNote` | `enemySeTbl[category]` (0x0063a850: 19 pointers, `cateWarrior`, `cateMagic`, `cateGob`, `cateLizard`, `cateUndead`, `cateGolem`, `cateDemon`, `catePlant`, `cateGhost`, `cateCrust`, `cateDog`, `cateFish`, `cateInsect`, `cateKnife`, `cateEarth`, `cateBird`, `cateSpace`, `cateEtc1`, `cateEtc2`; 13 to 25 rows) | none |
| `ccSeSetParamInu(param, ch)` 0x0017ac70 | `inuCheckNote`, `ccPuccigusoCheckNote` | `inuSeData` (0x0063a8a0, 6 rows) | params 0 and 1 |
| `ccSeSetParamPC(param, ch, ccstype)` 0x0017aaf0 | `rtpcCheckNote` | none | param 0; any other plays nothing |

`base->id` is `ccCharBaseParam.id` (+0xc) and `category` the enemy race's
`seCategory`. A NULL table or a row of code -1 plays nothing (checked
before the footstep); otherwise the row plays, or on the footstep's param
the ground. A param past its array's end reads on into what follows in
setbl.cpp's .data (gcmn 0x00639560-0x0063a8d0): the arrays are 16-byte
aligned, so after one of an odd number of rows (every `spcN`, and 12 of
the 19 enemy tables) comes a row of zeros, which plays `ccSeOn3D(0)`;
then the next array, and after the last `spcN` and `cateEtc2` the pointer
tables.

Across the volumes the readers (`ccSeSetParam*`, `ccSeOnPCStep`,
`seHitAttr`) differ only in code layout, and the data differs. MUT's
(gcmn 0x00669dc0) is INF's. OUT's (0x006687b0) and QUA's (0x0055fb30)
change row 11 of each party table and the enemy tables, and their
`spcSeTbl` points ids 18-20 at tables of their own (21 pointers and 3
NULL, to the 16-byte boundary).

The footstep (`ccSeOnPCStep`, 0x0017a6b0, for SPC) is the row
`seHitAttr(ch->hitAttribute)` (`ccChar` +0x80; `seHitAttr` 0x0017a7f0)
gives for `hitAttribute & 0x00f0f0f0`:

| ground | row |
| --- | ---: |
| 0x008080f0 | 50 |
| 0x00b06010 | 49 |
| 0x00c0d000 | 55 |
| 0x00304050, 0x00405060, 0x20f0, 0x2040, 0x3060, 0x5030, 0x6040, 0x4080 | 24 |
| 0x0090b0c0 | 27 |
| 0x0060b0d0 | 26 |
| 0x0070c0e0 | 25 |
| 0x00c0c0c0, 0x00e0e0e0 | 28 |
| 0x00d0d0d0, 0x00606060 | 22 |
| 0x00404040, 0x00505050 | 23 |
| 0xc000 | 21 |
| anything else | 202 |

played with `ccSeOn3DNote` at note 67 for row 202, else at a note by who
steps - SPC by `base->id`: 60 for 0 (Kite), 62 for 1, 58 for 2, 3 and 8,
64 for 6, 61 for 10, 63 for 15, 61 for the others; PC by `ccstype`: 60
for 0, 65 for 1, 58 for 2, 61 otherwise; Inu 60.

## Music

A bank has up to three sequences. `ccSndSQLoad` (per game context) and
`ccSndChangeData` (the jukebox) load one row of an `SQ_LOAD` table:

- the bank into ports 1-3; sequence 0 at the end of the `.hd` into
  sequencer 0; sequence 1 (only if `sqSize2` is non-zero) at `hdSize +
  sqSize1`, sequence 2 (only if `sqSize3`) after both. So a bank whose
  `sqSize1` is 0 (15 rows; jukebox row 19 is one) plays its second file as
  sequence 0 and again as sequence 1.
- `sqNum` = 1, 2 if `sqSize2`, 3 if `sqSize3`.
- the row's three `SQTBL`s from the matching volume table: `{int
  midiPort, int hdPort, u16 vol}` - sequence `i` plays on sequencer `i`
  into port `i + 1` in every row; `vol` is 0x100 for full.

`ccSqPlay(i)` (0x001798f0): if `i < sqNum` and it is not playing, set port
`hdPort` to `vol` (through `ccPortVolSet`) and send 0x110 (locate 0,
play). `ccSqStop(i)` sends 0x20.

### The desktop and its jukebox

`Wave` (desktop.prg 0x0042b6f0) has 51 rows `{char *title, int category,
int fieldType, int bgNum, char *comment, int NO}`, `NO` equal to the
index, then a terminating row with category -1. `saveData.dtBgm`
(+0x2237) holds a `NO`.

`ccSndChangeData(wave, old)` (0x001834e0):

1. Fade out what plays: if `old` is 47, 27 or 7, `ccSqFade(1, 0, 10, 3)`,
   wait 10 frames, port 2 volume 0, `ccSqStop(1)`; if `old` is -1, all
   sound off; otherwise the same on sequence 0 and port 1.
2. Load the row: category 1 `sqDataDesktop`, 2 `sqDataTown`, 3
   `sqDataField[fieldType]`, 4 `sqDataDungeon`, 5 `sqDataEvent`, 6
   `sqDataStream`, 7 `sqDataTitle` (jump table 0x0034e130; 0 loads
   nothing), row `bgNum`, and the `SQTBL`s of the matching volume table
   (0x0034e110). Port 0 back to `seVol`; ports 1-3 to their `vol`.
3. Unless `old` is -1: `ccSqPlay(1)` if `wave->NO` is 47, 27 or 7, else
   `ccSqPlay(0)`.

Three jukebox rows share a bank with another row and play its second
sequence: 7 ("BGM 08", event 15), 27 ("BGM 28", town 1) and 47 ("BGM 48",
event 8).

The desktop starting (`ccSetupDesktop`, 0x00168320) calls
`ccSndChangeData(&Wave[dtBgm], -1)` - load only - then `ccSndBgmCtrl`,
whose desktop case (`sqLoadParam` 1, 0x0017b7d4) plays sequence 1 when
`dtBgm` is 27 or 7 and sequence 0 otherwise. Row 47 is missing there, so
a desktop started with `dtBgm` 47 plays row 46's music, which the jukebox
itself never does.

The jukebox (`Audio_control::ChangeWeve`, desktop.prg 0x004072c0) plays SE
4 on a selection, starts a thread (`BgmRead`) that calls
`ccSndChangeData(&Wave[no], current)`, and stores `no` in `dtBgm`.

### Fades

`ccSqFade(sq, per, t, sw)` (0x00179b50) sets `fade[midiPort]`: from the
port's current volume `<< 8` to `(vol * per) & 0x1ff00` in `t` frames
(`rate = (now - end) / t`); `sw` bit 1 stops the sequence at the end.
`ccSound::ccFade` (0x00181250) runs once a frame from `ccSoundRpc`: fade 0
drives port 1, fade 1 (with two or more sequences) port 2; each frame
`volume -= rate`, clamped to 0..0x10000, and the port is written only on
odd `time` for port 1 and even `time` for port 2.

## The sound task

`ccSoundRpc` (0x00182dc0) runs once a frame:

```
if !bgmStopFlag (+0x61) && gameStart (+0x17):  bgmChange; ccFade
if !gameStart:                                 ccSceneFade
if +0x63:                                      ccSndGateHackCtrl
if gameStart && game mode 1 or 2:              skillVoicePlay
if +0x135:                                     wavPlay        (BGM.BIN)
evVoicePlay                                                   (the voice slots)
ccSndChangeOption; sdCommand
if +0x134:                                     ccSndCmd(0x140) all sound off; clear
```

`+0x63` is the gate hack's hold (`ccSndGateHack(n)`, 0x00180780, from
`GtHackMenu`; OUT's `GtNewMenu`). 0, as the menu opens: state 1, and
sequence 0 and, with `sqNum` 3, sequence 2 faded inline from the port's
volume to 0 over 20 frames with switch 3 (stopped at the end). 1, a
cancel: state 2. 2: state 0. `ccSndGateHackCtrl` (0x00180910) runs
`ccSceneFade` in states 1 and 3, after `ccFade`, so in a town fade 0 steps
twice a frame (stopped at frame 11) and fade 2 once (frame 21). State 2
is `ccSqPlayVol(0, 0)`, a switch-1 fade up to the table volume over 20,
the same for sequence 2, then state 3. The same in all four volumes.

`gameStart` is 0 from `ccSound`'s constructor and after every mode change
(`ccGame::ChangeRequest` -> `ccSound::gameInterrupt`, 0x001811f0, unless
`+0x105` is 2 and `ccGame::CheckSceneReplace()` is false); `ccSetupDesktop` (0x00168550, before `ccSndBgmCtrl`),
`ccSetupToppage` and `ccSetupGameCtrl` set it. The title never has it.

`ccAllSoundOff` (0x0017ae10), which each mode's set-up calls early
(`ccSetupDemo`, `ccSetupNewGame`, `ccSetupDesktop`, `ccSetupToppage`,
`ccSetupGameCtrl`, `ccThMother`), clears fades 0 and 1 (`sw`, +0x94 and
+0xa8), sets +0x134 and calls `ccEvVoiceStop` - all of it skipped when
`+0x105` is 2. The +0x134 command (0x140) stops the sequencers, so the
last mode's music ends there even before the next bank loads.
`ccSetupDesktop` (0x00168320) calls it after the event task's phase-0
pass and `DESKTOP.PRG`'s load, before the phase-2 pass: event 4's block 9
plays streams 4-6 (Skeith) in that pass, with the dungeon's music
already stopped, and only then does `ccSndChangeData` load the desktop's
bank.

## A mode's bank: ccSndSQLoad

`ccSndSQLoad(n)` (0x001821d0) loads the bank of a context: 0 the board
(`sqDataToppage`), 1 `sqDataDesktop`, 2 the town, 3 the field, 4 the
dungeon, 5 an event, 6 a stream, 7 the title (`sqDataTitle`) - jump tables
0x0034e0e0 (rows) and 0x0034e0c0 (volumes); any other `n` returns -1
once SNDBASE has been told it (step 2). 0, 1 and 7 take row 0 of their tables; the others read the world.
Each case also sets three `ccSnd` bytes: +0x104 the play type (sent to
SNDBASE, and what `ccSndBgmCtrl` and `bgmChange` go by), +0x5f, set where
`bgmChange` may switch to the battle arrangement (sequence 1), and +0x105
(0 unless said below).

| n | rows (24 bytes) | volumes (36 bytes) | row | play type | +0x5f |
| ---: | --- | --- | --- | --- | ---: |
| 2 | `sqDataTown` 0x00307d40 | `sqVolTblTown` 0x00309f90 | `GetTownType()`, + 5 while `saveData.crisis` | 0 | 0 |
| 3 | `sqDataField[ft]` 0x00307bc0 | `sqVolTblField[ft]` 0x00309f60 | `GetBG()` | `playTypeTbl[ft][bg]` 0x00307bf0 | 1 |
| 3, Piros in the party | `sqDataField[11]` (`piroshi`) | `sqVolTblField[11]` | 0 | `playTypeTbl[11][0]` | 1 |
| 4 | `sqDataDungeon` 0x00307c20 | `sqVolTblDungeon` 0x0030a100 | `GetDungeonType()` | `dungeonPlayType` 0x00307d28 | 1 |
| 5 | `sqDataEvent` 0x00307f00 | `sqVolTblEvent` 0x0030a3d0 | `game.field` | `eventPlayType` 0x00308a90 | see below |
| 0, 1, 7 | as above | | 0 | 0 | 0 |

`ft` is `WORLD_MAN::GetFieldType()` (`fieldtype`, +0x10) and `bg`
`GetBG()` (`bgnum`, +0xc): the field type and weather `SimGenerateCode`
made ([area keywords](area-words.md)). `sqDataField`, `sqVolTblField` and
`playTypeTbl` hold twelve pointers, by field type, to ten-row tables
(`typeA` ... `typeO`, with eight play-type bytes each) and, twelfth,
Piros's one-row table. "Piros in the party" is `checkPartyMenberNum(8)`
(gcmn 0x0059d100) not -1: character 8 in one of `ccPartyManager`'s three
member slots, whatever the field. `GetDungeonType()` is
`dungeonType[game.dungeon]`, `GetTownType()` `game.town`; the server
plays no part.

**The town** (case 2): ten rows, five towns and their crisis versions;
+0x105 becomes 1 in Mac Anu (town 0), 3 in the others, which also clear
+0x108..+0x114.

**The event bank** (case 5): the play type is `eventPlayType[game.field]`,
except that 4 means play type 0 with +0x5f set (areas 1-12 and 18). Area
15 sets +0x105 to 2 (`ccAllSoundOff` then does nothing, and
`gameInterrupt` keeps `gameStart` unless the scene changes) and clears
+0x108..+0x114. Area 48 forces play type 0; otherwise `game.areaPrev` 2
(the player came from a dungeon) forces 1. 25 of the 123 rows name a bank:
areas 1-13, 15, 16, 18, 23, 25, 48, 66, 67, 71, 77, 91 and 101.

1. `initBeforeLoad` (0x00183080): fades 0 and 1 off, `bgmStopFlag` (+0x61)
   set, +0x134 set, +0x133 and +0x137 cleared; the task's next frame
   sends all sound off.
2. `ccSndCmd3(0x200, n)`; +0x105 = 0; the case; `ccSndCmd3(0x300, play
   type)`: SNDBASE's `area` (only printed) and `playtype`.
3. A row of -1 (or a NULL field table) loads nothing: 60 frames with
   `sqNum` 0, port 0 back to `seVol`, `initAfterLoad`, return -1.
   `bgmStopFlag` stays set - only a load that finds its bank (or
   `ccSndChangeData`) clears it - so the fades and `bgmChange` stay off. Story area 43's field (`model` 1, no event bank) is one.
4. The bank (`0x9310`), waiting for the IOP's reply; sequence 0 to
   sequencer 0 (`0x9051`, `0xa1` 0x2010, `0x40`), then sequences 1 and 2
   where `sqSize2` and `sqSize3` are not 0; `sqNum` 1 to 3.
5. The three `SQTBL`s; port 0 back to `seVol`, ports 1-3 to their `vol`,
   except port 2 to 0 in the field (case 3), where sequence 1 is the
   battle arrangement `bgmChange` fades in; `sqStatus` -1; `initAfterLoad`
   (+0x65..+0x6c = -1); `bgmStopFlag` and +0x60 cleared; +0xf0 = `n`.

Nothing plays until `ccSqPlay` or `ccSndBgmCtrl`. The title
(`ccSetupDemo`, 0x00168160): `ccAllSoundOff`, `ccSndSQLoad(7)`; `ccThDemo`
(demo.prg) `ccSqStop(0)`, then `ccSetMainVol(saveData.mainVol)` and
`ccSqPlay(0)`; New Game and Load leave with `ccSqFade(0, 0, 8, 3)`.

### The areas: ccSetupGameCtrl

`ccSetupGameCtrl` (0x00168960), when `game.request[0]` is 0, picks the load
by `game.area`. "The scene changed" is `ccGame::CheckSceneReplace()`
(0x00167580): `area`, `town`, `field` or `dungeon` differs from `areaPrev`,
`townPrev`, `fieldPrev` or `dungeonPrev` (+0x14/+0x18, +0x20/+0x38,
+0x24/+0x3c, +0x28/+0x40); floors and blocks are not compared.

- Town (0): `ccSndSQLoad(2)` when the scene changed.
- Field (1): when the scene changed, `ccSndSQLoad(5)` if `game.field` is
  not 0 and `WORLD_MAN::GetEventAreaInfo(game.field)->model` is 1 (areas
  1-13, 15, 16, 43, 66, 67, 91), else `ccSndSQLoad(3)`.
- Dungeon (2): `ccSndSQLoad(4)` when the scene changed; otherwise
  `ccSndSQLoad(5)` when `WORLD_MAN.specialRoom` (+0x160) is 0 or more.

`ccSnd.gameStart` is set just before the fade in (0x001694ec) and
`ccSndBgmCtrl` called once it has ended (0x0016957c), in a town as in a
field or dungeon. (The port's town sent neither until worklog 185: its
music was started as `ccSqPlay(2)` in the crisis or `ccSqPlay(0)`
otherwise, which missed sequence 0 under Mac Anu's crisis music and
Dun Loireag's sequence 2, and with `gameStart` never set the town ran
`ccSceneFade` and no scene sounds.)

### ccSndBgmCtrl

`ccSndBgmCtrl` (0x0017b020) switches on +0xf0, the last loaded context
(jump table 0x0034d350). Each start is `ccSqPlay` inlined: only a
sequence below `sqNum` that is not playing, its port set to its `vol`,
0x110 sent, `sqStatus` 1. At the end +0x137 is set, which lets `bgmChange`
run.

- 2, the town (0x0017b064): types 0 and 2 start sequence 2 when
  `saveData.crisis` is 1, types 1 and 3 whenever the bank has three
  sequences; then every type starts sequence 0 (type 0 also sets +0x105 =
  1 and +0x132 = 0). So Mac Anu in the crisis plays its sequence 2 over
  sequence 0.
- 3 (field) and 5 (event bank), 0x0017b444: if +0x138 is set, clear it
  and start nothing; else by play type: 0 sequence 0; 1 sequences 0 and
  2; 2 sequence 2; 3 or anything else nothing.
- 4, the dungeon (0x0017b430): nothing at all, +0x137 included, unless the
  scene changed; then as the field.
- 1, the desktop (0x0017b7d4): the hold as the field's, then sequence 1
  for `dtBgm` 27 and 7, else 0.
- 0, and anything unsigned past 7: sequence 0. 6 and 7: nothing.

+0x138 is set by the event instruction `sound 10`
(`ccSndEvRequest(10, ...)`, case 10 at 0x0017e260) and cleared by the
constructor: an event holds the music of the area it leads into, once.

### Battle music: bgmChange

`ccSound::bgmChange` (0x00183380) runs in the sound task before `ccFade`
while `bgmStopFlag` is clear and `gameStart` set. It acts only with +0x137
and +0x5f set, `sqNum` 2 or more, and `pgRideFlag` (0x00378cdc) not 1 (on
a Grunty):

- +0x60 clear and `game.inBattle` (+0x58) set: `ccSndCmd3(0x130, 0)`,
  +0x60 = 1, `sqStatus[1]` = 1; unless the play type is 2,
  `ccSqFade(0, 0, 30, 3)`; then `ccSqFade(1, 256, 30, 1)`.
- +0x60 set and `inBattle` clear: `ccSndCmd3(0x130, 0)`, +0x60 = 0,
  `sqStatus[0]` = 1, `ccSqFade(1, 0, 30, 3)`; unless the play type is 2,
  `ccSqFade(0, 256, 30, 1)`.

SNDBASE's `bgmChange` (0x7e4, command 0x130), by its `playtype`: 0 or 1,
when sequencer 0 plays and 1 does not, locates sequencer 1 to sequencer
0's position (`midiEnv[0]` +0x8, the tick) and plays it; when only 1
plays, 0 to 1's position; 2, sequencer 1 from the start unless it plays;
otherwise nothing. "Plays" is `gIsInPlay_port0..2`, each sequencer's
status bit 1 as ATick saw it at the start of the last tick. So a battle's
music is the bank's sequence 1 picking up at the field music's tick,
crossfaded over 30 frames (port 2 starts at 0 in a field, at its `vol` in
a dungeon).

In the port the field's and the dungeon's frame hands `game.inBattle` to the
sound each frame (piney-game's `Event::InBattle`, then
`Audio::set_battle`): the value `ccThGameCtrl` keeps with `SetInBattle`
([the field's battle state](field-walk.md#the-battle-state)), 1 from the
first frame an enemy on the lists is within 2200 of a party member, 2 when
none is, 0 sixty frames later - so the field music crossfades to the
battle arrangement as a goblin comes out of a portal and back after it
falls (`Audio::set_in_battle`). `pgRideFlag` is the riding Grunty's
([grunty-ride.md](grunty-ride.md#the-music)): `Event::PgBgm` gives it to
`Audio::pg_bgm` with the ride's `ccPgBgmInit` and `ccPgBgmEnd(n)` (the
driver's `pg_bgm_init`, `pg_bgm_end`, checked by `tools/sound_ee.py`'s
`pginit` and `pgend` steps).

### ccSceneFade

While `gameStart` is 0, `ccSound::ccSceneFade` (0x00181440) runs the fades
instead of `ccFade`: fade `k` drives port `k + 1` (fades 1 and 2 only with
`sqNum` 2 and 3), written every frame. At `time` 0 the port takes the end
volume and, with `sw` bit 1 and `rate` above 0, `ccSqStop(k)`; otherwise
`volume -= rate`, clamped to 0..0x10000, port = `volume >> 8`, `time -= 1`.
So the title's `ccSqFade(0, 0, 8, 3)` writes port 1 at 224, 192, ... 0,
then 0 again and stops sequence 0: nine frames.

### The event instruction: ccSndEvRequest

The event instruction `sound cmd p0 p1 p2` calls `ccSndEvRequest`
(0x0017de80). Its switch (jump table 0x0034d380) has eleven cases; any
other `cmd` does nothing.

| cmd | what | as |
| --- | --- | --- |
| 0 | play sequence `p0` of the loaded bank: nothing unless `p0 < sqNum` and its status is not 1 or 3; its port takes the table volume (scaled as `ccPortVolSet` scales it), `ccSndCmd3(0x110 + midi port)`, status 1 | `ccSqPlay(p0)` |
| 2 | fade sequence `p0` from its port's volume to `p1` 256ths of its table volume over `p2` frames, `sw` 3 (stop at the end); `p2` 0 divides by zero | `ccSqFade(p0, p1, p2, 3)` |
| 4 | sound effect `p0` of `seData`: program change, then note on with the row's note, or, when `p1` is not -1, `p1`'s low byte sign-extended and floored at 0 (`p2` unused) | `ccSeOn(p0)`, `ccSeOnNote(p0, note)` |
| 7 | the voice stopped | `ccEvVoiceStop` |
| 8 | port `p0`'s volume `p1`: port 0 takes `seVol` whatever `p1` is, ports 1-3 `p1 * bgmVol >> 8`, `ccSndCmd(0xb0 + port)` | `ccPortVolSet(p0, p1)` |
| 10 | `ccSnd` +0x138 = 1: the next `ccSndBgmCtrl` starts nothing | |
| 1, 3, 5, 6, 9 | nothing | |

Infection's scripts use 0, 2, 4 (a sound effect, most often 74, the
announcement's chime), 7, 8 (`8 0 256`, the SE port back to full), 9 and
10. The port maps a request to the session's sound events
(`piney_game::mode::sound_request`) in all three hosts; ports past 3 are
ignored.

### The scenes' own sounds

`ccSoundMain` (0x00181010), the sound thread's loop, runs one more thing a
frame while `gameStart` (+0x17) is set, by +0x105 (`piney_audio::scene`,
fed each frame by the town and area modes):

- **1, Mac Anu: `waterTest`** (0x0017bac0), with a town built and
  `GetTownType()` 0: the canals, `seData` row 42.
  - The first frame starts it silent in a free `loopID` slot, as TOBJ's
    hum starts (0x0017bf20's messages), sets +0x132 and keeps the slot in
    `looptest`.
  - Every later frame sends `FD 01 ch note id 60 00` (the pan) and `FD 00
    ch note id vol 00`.
  - The volume is the camera's ground distance from y 0 (its x and z
    zeroed): full within 1700, then `(decay * 3000 / 256 - (d - 1700)) /
    10 * velocity / 100`, 0..127.
  - Outside the 1700 it is at least 48 west of x -3000, and exactly 48
    south of y -6300 while x is negative. It is never above the row's
    velocity.
  - +0x132 is cleared by the constructor and by `ccSndBgmCtrl` in Mac
    Anu (town type 0).
- **2, area 15's event bank: `bgmChurch`** (0x0017c3c0).
  - The first frame plays sequence 0 silent (`ccSqPlayVol(0, 0)`), sets
    +0x108, takes `game.block` into +0x10c and sends play type 0
    (`ccSndCmd3(0x300, 0)`).
  - Entering the church (block 1): port 2 at 256 under `bgmVol`, SNDBASE's
    `bgmChange`, `sqStatus[1]` = 1, and sequence 0 fading out over 30
    frames and stopping.
  - Leaving it: `bgmChange`, `sqStatus[0]` = 1, sequence 1 fading out.
  - Outside, port 1's volume is set from the camera's ground distance to
    y 3500 (x and z zeroed): `max(3500 - d, 0) / 35 * 2.56`, under
    `bgmVol`, sent when it changes. 2.56 in the EE's floats makes the full
    volume 255.
- **3, a town other than Mac Anu: `bgmBreed`** (0x0017c6d0), Dun Loireag's
  Grunty breeder.
  - Once (+0x114), the town scene's `DMY_merchant6` position goes to
    +0x120.
  - Kite coming within 1000 of it on the ground: sequence 0 fades out and
    stops, sequence 1 starts silent and fades in to its table volume
    (+0x108 = 1).
  - Going beyond 1200: the reverse (+0x108 = 0).

`ccSndSQLoad` clears +0x108.

`ccSndChangeOption` leaves these scenes' ports alone. In The World
(`game.status` 5), a changed `bgmVol` sets:

| where | ports set |
| --- | --- |
| a field or dungeon with +0x105 2 | port 2, or ports 1 and 2 inside the church (+0x10c 1) |
| a town with +0x105 3 | ports 1 and 3, or 2 and 3 while the breeder's tune plays (+0x108) |
| anywhere else | all three |

The two cases the field's code tests on +0x104 do the same.

## Streams

`ccRequestLoadStream` ([streams](stream.md#the-call)) calls
`ccSndStreamCtrl(num, sd, 0)` (0x0017d190) before the stream plays and
`(num, sd, 1)` after it has returned, on the bank that is loaded - the
area's; `ccStreamInit`'s `strSeInit(num)` (0x00183f40) sets `ccSnd +0xe4`
and `+0xe8` from `strSndTbl[num]` (0x0030b950: 134 x `{strse, strbgm}`)
and frees the four SE slots at +0xf4; `strSeEnd` (0x00183fb0) turns off
(`ccSeOffLoop`) any of those still on. Nothing is done while `ccSnd +0x62`
is set: `ccSndMoviePlayer(0, bgm)` (0x00180b30), the desktop Audio screen's
movie, stops the jukebox's sequence and sets it; `(1, bgm)` plays the
sequence again and clears it.

`ccSndStreamCtrl` reads `sd->size`, the stream header's: bit 0 before,
bit 1 after (not with `game.status` 7, the stream viewer). Its fades are
`ccSqFade` inlined, 30 frames each; `ccSqPlayVol(sq, vol)` (0x001799b0) is
`ccSqPlay` at `min(vol, SQTBL.vol)`:

| stream | before (bit 0) | after (bit 1) |
| --- | --- | --- |
| 3 | - | sequence 0 from its start at volume 0 (`ccSqPlayVol(0, 0)`; nothing if it plays), fading in to its table volume |
| 8, 58 | `ccSnd +0x105` = 0; port 2 at 256 of `bgmVol`; SNDBASE's `bgmChange` (0x130); `sqStatus[1]` = 1; sequence 0 fades out and stops | 8: sequences 0 and 1 stop |
| 12 | - | sequence 1 plays (unless it does) |
| 13 | - | `ccSnd +0x105` = 0; the first taken `loopID` slot (+0x65): `FD 10 ch note id 00 00` (`seData[42]`'s channel and note) through `sceMSIn_PutHsMsg`, then the slot the id indexes is freed |
| 25, 26 | sequence 0 fades out and plays on | as 3 |
| 56 | the battle switch off (+0x5f = 0), `bgmChange`, sequence 0 out and stopped, sequence 1 in to its table volume | - |
| 57 | sequence 1 out and stopped | - |
| 107 | - | in `game.field` 16: sequences 0 and 2 play |
| 112-119 | sequence 0 to half its table volume | sequence 0 back to its table volume |

Stream 9 has an after case that does nothing.

Mutation, Outbreak and Quarantine have their own switch (MUT 0x0017f870,
OUT 0x0017f2c0, QUA 0x0017f220; Quarantine's is Outbreak's). Before:
118-125 sequence 0 to half; 25, 26 sequence 0 out, playing on; 57, 132
sequence 1 out, stopped; 87, 134 sequence 0 out, stopped; 126, 131 sequence
0 out, stopped (Outbreak on: sequence 1 instead while the battle music plays,
`ccSnd +0x60`); 56 the battle switch; 133 the battle switch and
`sqStatus[1]` 1; 8, 58, 127-130 the hand-over (8's before). After: 118-125
sequence 0 back; 113 (the Chaos Gate, Infection's 107) in area 16 sequences 0
and 2 again; 3, 25, 26, 134 sequence 0 in; 87, 131 sequence 1 in; 133 the
battle switch on, sequence 1 stopped and sequence 0 in; 132 (Outbreak on) the
battle switch on and sequence 0 in; 8 sequences 0 and 1 stop; 12 sequence 1
plays; 13 as Infection's.

A stream's note of event 4 goes to `ccSndStreamSE` (0x0017caa0), which,
with +0x62 clear and +0xe8 set, calls `ccSndStreamBGM(param)` (0x0017cb20);
events 1-3 do nothing, and `strse` (+0xe4) is not read there.
`ccSndStreamBGM` ignores `param` and acts on the record the cursor (+0xe8)
is at:

```
STRBGM      0x0c bytes, strSndTbl[num].strbgm
  +0x00 s16 sq      below 0: passed over; with cmd 5 the end, where the cursor stays
  +0x02 s16 sq2     -1 none
  +0x04 s16 time    frames
  +0x06 u16 vol     256ths of the table volume; for cmd 1 a port volume, 0xffff none
  +0x08 s16 unknown_8   not read
  +0x0a s16 cmd
```

| cmd | does |
| ---: | --- |
| 0 | `ccSqPlay(sq)` |
| 1 | `ccSqStop(sq)`; port `sq + 1` to `vol` (`* bgmVol >> 8`) unless 0xffff; `ccSqStop(sq2)` unless -1 |
| 2 | `ccSqPlayVol(sq, 0)` (sq below 3), then `ccSqFade(sq, vol, time, 1)` |
| 3 | `ccSqFade(sq, vol, time, 3)` |
| 4 | `ccSqFade(sq, 0, time, 3)`; `ccSqPlayVol(sq2, 0)` and `ccSqFade(sq2, 256, time, 1)` |
| other | nothing |

and the cursor moves to the next record. Three streams have a table:

| stream | records | its notes of event 4 |
| ---: | --- | --- |
| 3 | stop 0, port 1 to 0; play 0 at 0 (a 1-frame fade to 0); end | frame 385 only: the dungeon's music stops |
| 8 | passed over; stop 1, port 2 to 0; end | frames 2 (passes), 366 (stops sequence 1), 1520 (the end) |
| 58 | fade 1 out over 120 frames and stop; end | |

So in the arc: stream 3 (event 4's dungeon) stops the dungeon's music at
frame 385 and, after the stream, starts it again from its beginning,
fading in. In area 15's church (its event bank: play type 3, nothing
playing) stream 8's before asks SNDBASE's `bgmChange`, which does nothing
for play type 3, and marks sequence 1 playing; its note at 366 sends the
stop; stream 12's after then starts sequence 1, the church's music.

**The port**: `crates/piney-audio/src/stream.rs` (`stream_ctrl`,
`stream_bgm`, `sq_play_vol`; `Audio::stream_music`, `Audio::stream_bgm`),
stream 13 on the driver's `loop_id`; the cursor is
`piney_stream::table::BgmCursor`, which walks the table and hands on the
record to act on (`Request::StreamBgm`); `piney-game`'s
`StreamPlayer::event` turns them into `Event::StreamMusic` and
`Event::StreamBgm`.

**Checked**: `tools/test_stream_rs.py music` runs `ccSndStreamCtrl`,
`ccSndStreamSE` / `ccSndStreamBGM` and `strSeInit` in `tools/eemu.py`
between `tools/sound_ee.py`'s steps (banks loaded as the game loads them,
`ccSndBgmCtrl`, the sound task's frames) and writes
`crates/piney-audio/tests/stream_fixture.txt`;
`crates/piney-audio/tests/stream.rs` runs the port through the same steps.
1,399 scenarios, 0 mismatches: stream 3 in each of four dungeon banks with
its three notes, streams 8-12 in the church, 13 in Mac Anu with looping
effects on, 58's table, every stream before and after over five banks
with their music playing or not, `game.status` 7, the movie player,
stream 107 in areas 16 and 15, and a record of every command.

## Volumes

`ccSound` starts with `mainVol`, `bgmVol`, `seVol` = 256
(`ccSound::ccSound`, 0x00180ed0); `ccSaveData::SetSoundEnv` (0x00178600)
loads `saveData.mainVol` (+0x841e), `seVol` (+0x8420), `bgmVol` (+0x8422)
and `output` (+0x8424). The desktop's sound menu shows them 0-32 (`v *
32 / 256`) and steps by 8.

- `ccSetMainVol(v)`: SPU2 core 1 master volume (`MVOLL`/`MVOLR`) =
  `(v << 14) - v >> 8`, 16383 at 256 (`sdCommand` case 1).
- `ccSetSeVol(v)` and `ccSetBgmVol(v)` clamp to 0..256 and flag a change;
  `ccSndChangeOption` (0x00179630), once a frame, applies it through
  `ccPortVolSet(port, vol)` (0x0017ad70): port 0 gets `seVol` whatever
  `vol` says; ports 1-3 get `vol * bgmVol >> 8`, `vol` being the port's
  `SQTBL.vol` (in The World, only the ports the scene's own music leaves:
  [the scenes' own sounds](#the-scenes-own-sounds)).
- The synthesizer's port volume (`sceHSyn_SetVolume`) is 0..256.
- `BGM.BIN` (`wavPlay`, 0x0017e6e0): `bgmParam`'s left and right 0x7fff,
  each `* bgmVol >> 8`, go to SEWORDS, whose `BgmSetVolumeDirect`
  (0x0e24) writes them to the core's sound-data input volume
  (`BVOLL`/`BVOLR`).
- Voice lines (`evVoicePlay`): 0x6fff left and right, fixed; only the
  master volume reaches them. Nothing lowers the music while one plays.

## Voice

A message window's voice is `ccEvVoiceRequest(event, msg)` as the window
opens; the line goes out through one of two slots on the sound task's
next frame (`evVoicePlay`) and SEWORDS streams it into core 0's
sound-data input, mixed dry with the voices of core 0 and passed to core
1 (`MMIX`). `ccEvVoiceStop` cuts it. Which line, the slots and the
stream: [voice](../formats/voice.md).

Events below -1 go to `ccVoiceRequest` (0x0017eeb0), the field's voices,
with no Parody Mode test. Each group has a Japanese and an English table
in gcmn.prg (8-byte `VOICE_DATA` rows) and a `voiceFile`:

| groups | voices | file |
| --- | --- | --- |
| -2 to -12 | the Grunties (`chibigusoTbl` .. `woodTbl`) | 0 `PGUSO` |
| -13 to -16 | the dogs (`inu0VoiceTbl` .. `inu3VoiceTbl`) | 5 `INU` |
| -20 | the fountain | 1 `FOUNTAIN` |
| -30 | party members joining and leaving (the menus' greetings) | 2 `PARTY` |
| -31 | member talk | 3 `SPCTALK` |
| -32 | presents | 4 `PRESENT` |
| -40 | Fidchell | 19 `BOSSTALK` |

A group without a case (-17 to -19, -21 to -29 and the rest) plays
nothing.

**Mutation's changes.**
- **`MIAE.BIN`.** `evVoiceFileE` gains a 21st file, `MIAE.BIN` (20).
  With the English voices, the party (-30), talk (-31) and present (-32)
  groups test `talkNum[1]` through a new getter (MUT 0x0017abf0; 18-20 in
  the extension). While it is set, their messages 3-5, 2-3 and 5-9 play
  rows 0-2, 3-4 and 5-9 of one table from `MIAE.BIN`. The case sets
  `voiceFile` before it tests the row, where Infection's sets it after.
- **The party tables.** The Japanese party, talk and present tables grow
  from 54, 36 and 90 rows to 63, 42 and 105. The English ones keep
  Infection's size, so an English message past them reads the next
  table's rows. The port plays nothing there.
- **Loading.** `ccVoiceRequest` and `ccWordsPlay` return at once while
  `ccSnd +0x139` is set: `ccFileListLoad` sets it from its first file to
  its last. The port loads between frames, so it has no such window.
- **Fidchell.** `BOSSTALK.BIN` is on neither Infection's disc nor
  Mutation's.

`piney-gen` reads the cases from the code: the dispatch's `li`/`beq`
pairs, then in each case the `saveData.voice` test, the table builds, the
`voiceFile` stores and, for an alternative, its bounds, getter and
addend. A table runs to the next table either function builds, or to the
next symbol.

`ccVoicePgFood(food)` (0x001800e0), a Grunty food calling out as Kite
comes near (`ccGimFood::main`), makes the same request itself: out of
battle (`game.inBattle` 0), row `food` of `voiceFoodTbl` (gcmn
0x006317f0; `voiceFoodTblE` 0x00631870 with the English voices, 16 rows),
`voiceFile` 18 `FOOD`, a slot claimed. The port's tables carry it as
`voice::FOOD` (the same on every disc); the runtime asks for it with the event number -100
(`piney_data::sound::voice::FOOD_GROUP`, no `ccVoiceRequest` group), which
`Audio::voice` sends to `Driver::food_voice_request`.

A skill's name is `ccWordsPlay(sid, ch)` (0x0017e290). It is queued in
`ccSound` +0xd0 (four (character, skill) pairs, +0xec the count, the
fourth overwritten while full), unless an event is running (`eventMng`
+0x78c), the caster is neither Kite nor a party member (`base->type &
5`), or the id is 304 or more. The sound task plays it with
`skillVoicePlay` (0x0017e350) before `evVoicePlay`, when `gameStart` is
set and `game.area` is 1 or 2:
- **Which line.** The character's file comes from `spcVoiceData` and its
  rows from `voiceData` (`voiceKiteTbl` ..). The row is the skill id less a
  base, which depends on the character and on bit 0 of
  `ccGetSkillParam(sid)+0x2c` (the jump table at 0x0034d830). For example,
  Kite subtracts 6, or 123 when the bit is clear; characters 9, 10, 16
  and 17 use row 0, or subtract 150.
- **Sending it.** The line goes as `evVoicePlay` sends one, and the queue
  empties.
- **A quirk.** A character without a file, or a row without a line,
  returns before the queue empties, so the same word is tried again the
  next frame.

**Who calls it.** `ccWordsPlay` has one caller, `_ccSkillRequest` (gcmn
0x00572b98): a character's own skill (stype 0: the menus' `ccSkillRequest`,
the party AI's, an enemy's) or an item used through its user (stype 2),
id 6 or more (not the normal attacks 1-5), after the caster's `skillID` and
`skillStatus` are set; an item's effect on its target alone (stype 1)
names nothing. The enemies' calls reach the queue's own checks and are
dropped there (`base->type & 5`). In the port piney-battle's request says
it (`skill::Request::words`); piney-world shows it as `Show::Words { who,
sid }` beside the skill's start wherever a request is made (the menus and
the action button, the party AI, the enemies, the items' skills, the
traps); piney-game turns it into `Event::SkillWords` with the caster's
`base->type`, its `charTbl` row (`base` +0x0c), bit 0 of the skill row's
+0x2c and `puppetShow` (`eventMng` +0x78c), and `Audio::words_play` queues
it. A request the menus make reaches the sound task a frame later than the
game's (the world's shows are taken before the menu task runs).

**The fights' other voices.** No battle code of the ordinary fights calls
`ccEvVoiceRequest` or `ccVoiceRequest`: their callers are the message
window (the events' lines), the gate's tutorial (`GtNewMenuT`), the
Grunties' growth (`ccPGuso::evoAct*`), the Grunty foods
(`ccVoicePgFood`) and one boss, Skeith's prediction
(`ccBoss04::OnThinkPrediction`, gcmn 0x004937bc: group -40, Fidchell, row
by the prediction). The Kyvia bosses' "voices" (`DeadKyviaVoice`,
`kyviaCore::CoreVoice`) are sound effects (`ccSeOn3DNote`). Kite's call of
the party's strategy (`ccSpcShoutOperationName`, gcmn 0x005a17e0) speaks
no line either: it opens a chat balloon over Kite (`ccChatMsg::OpenChat`,
`chatActionStr` piece 5, the strategy's `chatMenuStr` piece, piece 6)
while a field or dungeon's battle is on (`game.inBattle` 1, the party of
two or more, no event, `spcOpnCnt` 20).

The language is `saveData.voice` (+0x842c, 1 for English, `VOICE_E/`),
which the Options screen sets and the driver reads each time a line
plays. The port's driver keeps a copy, so every mode hands it the save's
value as it starts (`Event::VoiceOptions`): the desktop, the town, and the
fields and dungeons (a story start enters one directly). Streams choose
their voice track from the same value. `piney-game --voice en|jp` sets the
language for a whole run, over the save's.

## How the music loops

Every jukebox track is a sequence; 147 of the 150 sequences loop forever
through their NRPN loop markers (the jump replays the same messages at
the same positions), and the other 3 - the title's and two cutscene banks'
- end and stay silent. Of `BGM.BIN`'s two streamed tracks, track 0
(`bgmParam` mode 2) loops and track 1 (the staff roll, mode 0) plays once.

## The SPU2 set-up

`spuInit` (0x00181ee0), per core: effect area ending at 0x1fffff (core 0)
and 0x1dffff (core 1), `sceSdSetEffectAttr` mode 5 (hall) with the area
cleared (0x105), effect return volume `EVOL` 0x3fff, master volume 0x3fff;
then `MMIX` core 0 = 0x0fc0 (sound-data input dry; voices dry and wet),
core 1 = 0x0fcc (core 0's output dry; voices dry and wet), and effects
enabled on both cores. `sdCommand` cases 4 and 5 turn the reverb off and
back to hall later (not on the desktop). A voice reaches the reverb when
its sample's `spu_attr` asks (bits 2 and 3; 307 of 1,402 samples).

## The sequencer (MODMIDI.IRX)

Module offsets. Checked: `tools/midi.py` matched the module run in
`tools/eemu.py` byte for byte, tick for tick, on all 150 sequences through
their first loop jump, and on hand-made files for the paths the disc never
takes; `crates/piney-audio/tests/midi.rs` checks the Rust port against it.

```
Midi block                 at Midi + offsets[n]
  u32  data offset         6 on all 150
  u16  division            ticks per quarter note, 480 on all 150
  [if data offset >= 12, u16 1, u16 table bytes: (status, data1) pairs
   for the An compressed form - used by no file]
events: [varlen delta] event
  delta absent when the previous channel event had bit 7 set in a data
  byte (masked off on output); running status; 8n has NO velocity byte
```

- Timing (0x958, 0x1580): `ticklen = ((tempo << 7) / division << 8) /
  reltempo` in 1/128 us (`reltempo` 256); each ATick adds `4167 << 7` to
  an accumulator and plays every event due, advancing one MIDI tick per
  `ticklen` it holds; a tempo change applies mid-tick. Default tempo
  500,000.
- Loops (0x664): NRPN. `CC 99 0, CC 6 id` records a loop start; `CC 99 1,
  CC 6 id, CC 38 n` jumps back to it - forever when `n` is 0 (all 147
  looping sequences), else `n` more times. Tempo is not restored. The
  loop CCs are still sent to the synthesizer.
- End: `FF 2F` stops the sequencer (3 sequences: the title's and two of
  `sqDataStream`'s) and sends `CC 64 0`, `CC 123 0` on every channel used;
  it never loops to the start. SNDBASE then rewinds it without playing.
- Output (0xdb4): raw MIDI, running status expanded, 2 bytes for 8n, Cn,
  Dn. CC 7 goes out as `min(127, vol * chvol * master >> 14)` (the scales
  are 128 unless set - the game never does). CC 32 would re-route a
  channel's port. Metas and F7 are swallowed; F0 is sent.
- Play (0x18d4) resends each channel's stored program, CC 0, 1, 2, 5,
  10, 11, 64, 65 and bend (not CC 7, and not a bend stored with a no-delta
  bit); stop sends the all-notes-off pair.

## The synthesizer (MODHSYN.IRX)

Module offsets; LIBSD offsets marked. Checked: `tools/hsyn.py`, a model of
the module, made the same libsd calls as the module run in
`tools/iopemu.py` for every sound effect, 14,260 note ons over eight banks,
the controllers, LFOs, portamento and stealing;
`crates/piney-audio/tests/hsyn.rs` checks the Rust port against the
module's own writes (every sound effect over three ticks, and twelve
jukebox rows over 12,000 ticks each, sequenced by the MODMIDI model). In
the harness libsd is a recorder, so ENVX and ENDX read 0: the envelope's
real timing, which decides when voices are freed, is the SPU2's.

### A tick (`sceHSyn_ATick`, 0x182c)

1. Each port's input, in port order (0x173c), is parsed (0x15cc) and
   emptied. No running status: bytes without bit 7 are skipped. 8n takes
   one data byte (no velocity), 9n An Bn En two, Cn Dn one; 9n with
   velocity 0 is a note off; An and Dn are ignored; F0 is skipped to F7;
   F9 takes 4 more bytes and FD 6; other Fx are ignored; a message cut short
   by the end of the buffer is dropped.
2. 0x37a4: KOFF written per core; each core's waiting voices, oldest
   first, key on once `ENVX < 1000` (0x3f50 writes VOLL, VOLR, PITCH, SSA,
   ADSR1, ADSR2, NON, VMIXL, VMIXR, VMIXEL, VMIXER); KON written per core.
   A note on reaches the SPU2 in the tick it arrives; a note on and off in
   the same tick never sounds.
3. 0x4dc0, over the sounding voices, core 0 then 1, oldest first: free a
   voice whose envelope has ended (released and ENVX 0; or held with ENDX
   set and ENVX 0, after a grace of 3 ticks); else step portamento and the
   pitch LFO, write PITCH if its fine sum changed, step the amp LFO, write
   VOLL/VOLR if they changed.

### Note on (0x65bc)

Program change (0x7dc) keeps the program of the bank slot CC 0 selects
(slot 0 is the only one loaded; CC 0 is never sent); none past the table's
end, at an empty slot, or with no splits. A note on sounds **every**
sample of **every** split whose key range (`key_low & 0x7f` ..
`key_high & 0x7f`) holds the note and whose sample set's velocity range
holds the velocity, among the set's samples those whose own velocity range
(masked with 0x7f) holds it - layered. A sample whose VAG index is past the
`Vagi` table is a noise voice (none on the disc); one whose VAG offset is
0xffffffff is skipped. The voice starts at `spuAddr + Vagi.offset`; LSAX is
never written - loops come from the ADPCM flags.

7 of the 237 sound effects select no sample at all and are silent in the
game: 11, 12, 14, 37, 54, 83 and 211.

### Pitch (0x43e8, 0x45f0)

```
fine = lfo_part + bend + portamento                       1/128 semitone
  bend = (bend >= 0 ? split.bend_high : split.bend_low) * bend14 / 8192
         (truncating; bend14 = (msb << 7 | lsb) - 8192; 256 in every bank)
key  = note + prog.transpose + split.transpose
fine += prog.detune + sample.detune + split.detune
if fine < 0: floor-normalise into 0..127, carrying into key
p     = sceSdNote2Pitch(sample.base_note, 0, key, fine)     (LIBSD 0x3050)
ratio = floor(Vagi.rate * 65536 / 48000)                     (0x3ec8)
PITCH = min((p * ratio) >> 16, 0x3fff)
```

`sceSdNote2Pitch(center, cfine, note, fine)`: `semi = note + (fine +
cfine) / 128 - center`, split into octave and semitone (C division);
`p = SEMI[semitone] * FINE[fine % 128] >> 16` from 12 and 128 ratios
(LIBSD .data 0x4860, 0x4878; 0x8000 = 1.0), then for octaves below +2 a
rounded right shift; there is no scaling up (0x1000 at the base note).
No key follow of pitch: the split's and sample's pitch-follow fields only
scale the pitch LFO's depth.

### Volume and pan (0x3b94, 0x484c, 0x4644, 0x4a40, 0x4b14)

```
vel    = the Sset's velocity curve (0x8fe4, low nibble): 0 v, 1 128 - v,
         2 max(1, v*v/127), 3 128 - that, 4 128 - sq(128 - v), 5 sq(128 - v),
         6-9 as 0-3 (the high nibble picks a velocity map; the game has none)
xfade  = the velocity and key crossfades (0x4c68), 16.16
level  = ((prog.vol * split.vol * sample.vol * vel) >> 14) * xfade >> 16
level' = (cc7 * cc11 * level) >> 14      cc7, cc11: (v << 7) / 127, 128 at start
a      = level' * portvol / 256          portvol 0..256 (sceHSyn_SetVolume)
a      = a * (amp_lfo * depth / 128 + 16384) / 16384, at least 0
VOLL   = min(gl * a >> 7, 0x3fff); VOLR likewise with gr
```

Pan: `base = clamp(|prog.pan| - 128 + |split.pan| + |sample.pan| + kf,
0, 127)` (`kf` the programs' and split's key-follow pan, / 12); `a =
clamp(CC10 - 64, -63, 63) + base - 64`; for `a > 0` the left gain drops to
`(63 - a) * 128 / 63` (0 at 63), for `a < 0` the right; the near side
stays 128. A balance law with both sides at full in the middle.
`sceHSyn_SetOutputMode(0)` (mono) makes both 128. Program attribute bit 0
(only bank 0 program 119) wraps the pan and may invert the phase
(negative volumes).

### Envelope

ADSR1 and ADSR2 are the sample's (+0x12, +0x14), with key follow
(`(note - center) * kf / 12` added to Ar, Dr, Sl, Sr, Rr, clamped; every
kf on the disc is 0). The envelope runs in the SPU2. Note off (0x5cb4)
releases the held voices of that note (KOFF; with the sustain pedal down
they move to the sustained list, and go when it comes up); CC 123 is KOFF
alone; CC 120 and the port steal "cut" a voice: KOFF, ADSR2 0xc021, freed.

### Voices

- Core (0x6b3c): `spu_attr & 0x30` = 0x10 core 0, 0x20 core 1, else the
  core with more free voices (ties to core 0); no fallback. The free list
  hands out voice 23 first.
- The port's limit (`maxPolyphony`): at it, the port's lowest-priority
  released voice is cut (else a held one; the oldest on a tie; only at or
  below the new note's priority), or the note on stops.
- No free voice: reclaim ended voices (once a tick); then steal a sounding
  one - released before sustained before held, lowest priority, lowest
  ENVX, core 1 first - which gets a normal KOFF while the new note waits
  for its ENVX to fall below 1000; then steal a waiting one.
- Priority is the sample's plus the port's: 0x40 for the SE port, 0x10
  for music (SNDBASE clears the port's priority when it loads a bank, and
  the EE sends the attributes after).
- `spu_attr` bits 0-3 set VMIXL, VMIXR, VMIXEL, VMIXER for the voice (3 dry,
  15 dry and reverb on the disc).
- Exclusive groups (`sample.group`, 0x6f44) cut the channel's voices of the
  same group; no bank uses them.

### Controllers and messages

CC 0 bank, 1 pitch-LFO depth, 2 amp-LFO depth, 5 portamento time, 6 / 98 /
99 NRPN (MSB 2 sets the reverb mode, MSB 3 its depth, delay, feedback; the
sequences' loop markers use MSB 0 and 1, which do nothing here), 7, 10,
11, 64 sustain, 65 portamento, 120, 121, 123; bend; no RPN (bend range is
per split). A controller acts only when its value changes.

LFOs (0x93b4, 0x91ac) run when `sample.lfo_attr` asks (bits 0-2 pitch,
4-6 amp): saw, inverted saw, triangle, square, sample-and-hold noise,
sine; delay and fade in milliseconds, phase and cycle from the program.
Only bank 52 uses one (a triangle, under CC 1).

`F9 t ch a b` sets a one-shot override for the channel's next note on:
t 0 expression, t 1 pan, t 2 a 14-bit bend. `FD t ch|f note id b4 b5` is
the SE interface: t 0x10 is a note on with an id (`b4` the velocity) or,
with `b4` 0, a note off matching note and id; t 0, 1, 2 change the
expression, pan or bend of the sounding voices that match (id 127 any,
`f` = 0x10 any note).

## The SPU2 in the port

`crates/piney-audio/src/spu.rs` plays the voices as the PlayStation SPU is
documented to (psx-spx: the pitch counter, the 4-point interpolation table,
the ADSR generator, the fixed voice volumes), at 48 kHz, two cores mixed as
`MMIX` says, and `reverb.rs` runs the hall preset (LIBSD.IRX 0x42d8, the
same 32 register values as the PlayStation's) with its 39-tap resampling.
These are models of the hardware, not checked against a PS2.

## Unknown

- Which params the animations' notes 1 and 2 carry, and so whether any
  reads past its table: not surveyed. The port reads on as the game does
  to the end of setbl.cpp's data (gcmn 0x00639560-0x0063a8d0) and plays
  nothing beyond it, nor for rows naming no sound effect (the pointer
  tables' words), nor for an `id` or `category` past a pointer table and
  its padding, where the game would read other memory.
- `ccSeOffLoop` with an `id` outside 0-7 writes other `ccSound` bytes
  (-1 into +0x64 `sqNum` for -1); the port frees no slot then. No caller
  seen passes one.
- `sdCommand` cases 4 and 5 (reverb off / hall again) - who asks for them.
- `ccSndSQLoad(6)` (`sqDataStream`, by `ccGetStreamCode`) is not ported,
  and nothing in Infection asks for it: the only callers are
  `ccSetupDemo` (7), `ccSetupToppage` (0) and `ccSetupGameCtrl` (2 to 5),
  in main and every overlay alike.
- What `ccSnd` +0x110 does (+0x108, +0x10c, +0x114, +0x120, +0x132 and
  +0x133 are the scene sounds' and TOBJ's, above).
- Neither the canals' nor the church's volumes were compared with the
  game playing; they follow the code, run in unit tests.
- Event bank rows past 122 (`game.field` 123 and up) read past the end of
  `sqDataEvent`; no scenario gets there.
- Which `sceHSyn_SetOutputMode` value the options' stereo/mono setting
  sends (`ccSetOutputMode`, `sdCommand` case 3).
- The SPU2's own behaviour - envelope timing, ENVX/ENDX, interpolation,
  reverb - rests on the PlayStation SPU documentation, not on measurement.
- The synthesizer paths no data takes: noise voices, exclusive groups,
  velocity maps, user LFO waves, the SE-timbre port mode.
