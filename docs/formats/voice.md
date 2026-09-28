---
title: Voice, streamed music and cutscene PCM
status: partial
volumes: INF
covers: INF VOICE/*.BIN, INF VOICE_E/*.BIN, INF VOICE/BGM.BIN, INF STREAM/*.BIN Pcm chunks; INF SLUS_202.67:0x0017e810 ccEvVoiceRequest, 0x0017ec30 ccMesVoicePlay, 0x0017eca0 evVoicePlay, 0x0017ee40 ccEvVoiceStop, 0x00183d00 sewordCmd, 0x0033cff0 evVoiceDataVol1M, 0x0033d0c0 evVoiceDataVol1S, 0x0033e670 evVoiceDataVol1ME, 0x0033e740 evVoiceDataVol1SE, 0x0017e6e0 wavPlay; INF MODULES/SEWORDS.IRX:0x0290 bgmFunc, 0x2c74 wordPlay, 0x15e0 BgmPreLoad, 0x1b20 BgmStart, 0x1fb4 _BgmPlay, 0x0af0 _BgmRaw2SpuMono, 0x29dc loopEnd, 0x33e8 bgmPlay; INF SLUS_202.67:0x0017e350 skillVoicePlay, 0x0014ddb0 ccStream::Decode_Pcm; IOPRP243.IMG CDVDMAN:0x1414 cdrom read, 0x1378 its stream read, 0x62d8 the blocking stream copy, 0x65b0 the stream dispatcher; IOPRP243.IMG IOMAN:0x053c read
worklog: 14, 245
---

# Voice, streamed music and cutscene PCM

Three kinds of raw PCM, all signed 16-bit little-endian at 48 kHz, played by
the IOP module `SEWORDS.IRX` straight into SPU2 streaming voices.

## Voice lines: VOICE/ and VOICE_E/

```
file        headerless mono PCM; lines at 2048-byte boundaries, 0xff between
VOICE_DATA  8 bytes, in tables in main and gcmn
  +0x00  s32  ofs     byte offset of the line in its file; -1 = no line
  +0x04  s32  siz     bytes
```

`VOICE/` is Japanese, `VOICE_E/` English; the game uses `VOICE_E` when
`ccSaveData.voice` (+0x842c) is set. `evVoicePlay` (`0x0017eca0`) sends the
file name, `ofs` and `siz` with IOP command 0x80e0; `wordPlay`
(`SEWORDS.IRX:0x2c74`) seeks to `ofs` and plays mono.

| files | line table |
| --- | --- |
| `SPC00` - `SPC17` | per character, `voiceData[nn]` / `spcVoiceData[nn]` (0x003071e0, 0x00307230), in the order Kite, Mia, Orka, Merlaw, Suna, Usagi, Val, Gettyou, Piro, Waiz, Elku, Natume, Reichel, Garu, Ryouko, Black, Mist, Heruba |
| `EVVOL1`, `EVVOL1S` | event dialogue: `evVoiceDataVol1M` for event numbers below 50, `evVoiceDataVol1S` above |
| `PGUSO`, `INU`, `FOUNTAIN`, `PARTY`, `SPCTALK`, `PRESENT`, `FOOD` | by negative voice ids through `ccVoiceRequest` (`0x0017eeb0`) |

File names come from `evVoiceFile` (0x00307330) and `evVoiceFileE`
(0x00307380). Five names there - `EVVOL1P`, `EVVOL2`-`EVVOL4`, `BOSSTALK` -
have no file on this disc.

54 files, 8,208 lines: 4,117 Japanese (7,261.7 s), 4,091 English (6,840.7 s).

### Event lines

Every message window (`ccMessage::Change`, `ChangeInfo`, `Open`) calls
`ccEvVoiceRequest(event, msg)` (0x0017e810) as it opens, unless `event`
is -1; the camera tutorial (`teach_camera1`-`3`) asks with event 3.

```
event           table (Japanese / English)                     voiceFile   file
0-49            evVoiceDataVol1M  / evVoiceDataVol1ME [event]      6       EVVOL1   / EVVOL1_E
50-99           evVoiceDataVol1S  / evVoiceDataVol1SE [event-50]   7       EVVOL1S  / EVVOL1SE
100-349         the volume 2-4 tables                            9-17      EVVOL2 ... (not on this disc)
below -1        ccVoiceRequest (0x0017eeb0), the field's voices
-1, 350 on      nothing
```

- Each table is 51 pointers: NULL (no voice for the event) or a
  `VOICE_DATA` array indexed by `msg` (`evm1NNTbl`, `evs1NNTbl`, and
  `...TblE`). `ofs` -1 is no line. `msg` is not bounds-checked.
- `saveData.voice` (+0x842c) picks the English table at the request and
  the English file name (`evVoiceFileE`) when the line is sent.
- In Parody Mode (`saveData.parodyFlag`, +0x842b) events 0-49 have no
  voice; events 50-99 keep theirs.
- The row found is kept in one global, `vdRequest` (0x003789e0), with
  `voiceFile` (0x003789e4). `ccMesVoicePlay` (0x0017ec30) puts 0x80e0 in
  the first free of two slots (`ccSnd +0x140`); `ccEvVoiceStop`
  (0x0017ee40) puts 0x120. With both slots taken the call is lost.
- Once a frame the sound task's `evVoicePlay` (0x0017eca0) sends each
  taken slot in order: 0x80e0 as `vBank` {`vdRequest->ofs`,
  `vdRequest->siz`, 0, 0x6fff6fff, file name} through `sewordCmd`, 0x120
  alone. Both slots send the same `vdRequest`, the last one asked for.

Infection's four volume 1 tables hold 1,292 rows: 1,152 lines and 140 of
-1, one row per message of their event (event 22 has one row more than
messages). Every line is 2048-aligned and at least 40,756 bytes (0.42 s).
Infection's volume 2-4 tables are empty externs or name files its disc
does not have, so no event from 100 on is heard.

### Streaming: SEWORDS.IRX channel 0

Voice lines and `BGM.BIN` share channel 0, core 0's sound-data input: the
module refuses a new file (`wordPlay`, `bgmPlay`) while one is open, so a
line asked for while the last still plays is not heard. At start the EE
sends 0x130 (2, a DVD) and 0x140 (1, ring-buffer reads).

```
wordPlay(0, vBank)          SEWORDS 0x2c74
  refuse if gFd[0] >= 0; open, lseek(ofs)
  BgmInit(0, 0xc000)         buffer: SPU 2 x 16 KiB, then raw 2 x 8 KiB
  BgmSetMode(0, 0x10)        mono: gSPacketSize 16384, gRPacketSize 8192
  BgmPreLoad                 read 16 KiB into raw; convert both SPU halves;
                             if more than 16 KiB remain, read 16 KiB more
  BgmSetVolume(vol)          BVOLL = vol >> 16, BVOLR = vol & 0xffff
  BgmStart                   sceSdBlockTrans(0, loop, SPU buffer, 32 KiB)

SPU block                   1024 bytes: 256 left then 256 right samples;
                            mono copies each 512 raw bytes to both

on each transfer interrupt (half h played; _BgmPlay 0x1fb4):
  convert raw packet h into SPU half h
  end flagged:  let 2 interrupts pass, stop at the third:
                _BgmStop, BVOL 0, loopEnd (close, gFd[0] = -1)
  else if more than 8 KiB remain: read 8 KiB into raw h
  else:         read the rest up to the next 2 KiB sector, zero the
                packet's remainder, flag the end
```

A packet read at one interrupt starts to play at the third after it, and
the third interrupt after the end is flagged stops the transfer - just as
the last packet would start. A line plays `4096 * (ceil(siz / 8192) - 1)`
samples at 48 kHz; its last 1 to 4,096 samples are never heard (quiet in
all but 16 of the 1,152 event lines: the median peak of the cut part is
98). A line of
16 to 32 KiB would play its first two packets twice (the preload leaves
the raw buffer as it was); none is that short.

**A line under 16 KiB.** After the preload `size - pos` is negative, so
at the first interrupt `_BgmPlay` takes the last-packet branch and asks
`read(fd, raw half, size - 16384)`, rounded to a sector: -16,384 for a
line of size 0. By the code (not run on a console), that read never ends:
- `IOMAN`'s `read` (0x053c) hands the size to the device unchanged.
- SEWORDS opens its files with mode 0x40000000 while ring-buffer reads are
  on (0x140, which the EE sends at start), and `CDVDMAN`'s `open` then
  marks the file for stream reads (flag 8). The `cdrom` device's `read`
  (0x1414) sends those to a stream read of `size >> 11` sectors (0x1378);
  a normal read would return -22 for a negative size (0xde0).
- The stream read (0x62d8) loops, blocking, until the bytes copied equal
  the size asked. Its copy (the dispatcher's mode 2, 0x65b0) compares
  sizes unsigned: -16,384 is 0xffffc000, so it copies every sector the
  stream has buffered, 2 KiB at a time, on past the 8 KiB raw half. The
  total can never equal -16,384, so it waits for more sectors and copies
  those too.

Channel 0 would stop for good, and the IOP memory after SEWORDS's buffer
would be overwritten with the disc's following sectors.

Only one kind of row asks for such a line: every skill-word table
(`voiceKiteTbl` ... `voiceHerubaTbl`, both languages, all four discs)
ends with a row `{ofs 0, siz 0}`, and `skillVoicePlay` (0x0017e350)
checks only `ofs == -1` before sending a row. The row is reached when a
skill's id less the character's base is that last index. For Kite
(row 172 of 173) that is skill 295, `Para Repth` (bit 0 of its `+0x2c`
clear: base 123), the skill of the item Recovery Drink (`itemTblR` 23,
restores 800 HP). Kite using one in a field or dungeon would ask for it,
through `_ccSkillRequest` (stype 2) and `ccWordsPlay`. Only Quarantine
hands Recovery Drinks out: five in Kite's new-game items, and a trade.
Quarantine's `skillVoicePlay`, `ccWordsPlay` (one more check, a byte at
`ccSnd +0x139`) and `SEWORDS.IRX` do the same.

The port's `seword::Word` returns nothing for a read of 0 or fewer
bytes, so such a line plays its preloaded 16 KiB and ends. `evVoicePlay`'s 0x6fff
is the only volume: no option scales it, only core 1's master volume
(`saveData.mainVol`). Nothing lowers the music while a line plays, and
the EE never asks whether one does (`voiceStFlag` is only ever cleared).

0x120 (`ccEvVoiceStop`) is `BgmStop`, `BgmClose`, `BgmQuit` and BVOL 0 on
channel 0: it cuts a line, or a `BGM.BIN` track, at once. The game sends
it when the confirm button is pressed in a message window (while the text
types and to close it), when the tutorial's prompt closes, from the event
op `sound` 7 (`ccSndEvRequest`), and from `ccAllSoundOff` at the start of
every mode's set-up.

## Streamed music: VOICE/BGM.BIN

Interleaved stereo (L, R, L, R ...). `bgmWavTbl` (`INF gcmn.prg:0x006317d0`)
gives each track's byte range; `wavPlay` (`0x0017e6e0`) sends it with command
0x80f0.

| track | offset | bytes | length | loops | played from |
| ---: | ---: | ---: | ---: | --- | --- |
| 0 | 0 | 16,941,056 | 88.23 s | yes | `ccPuccigusoStart` (gcmn) |
| 1 | 16,941,056 | 46,073,124 | 239.96 s | no | `ccThStaffRoll` (desktop) |

The last 732 bytes of the file are 0xff. All other music is sequenced, in
[the sound banks](snddata.md).

## Cutscene audio: CCSF Pcm chunks

In the `STREAM/` [CCSF files](ccs.md):

```
0x2200 Pcm (setup)     CCSTRM_PCM, 16 bytes, then dataNum * dataSize words
  +0x00  u32  myID
  +0x04  u8   type          0
  +0x05  u8   bitNum        16
  +0x06  u8   trackType     1
  +0x08  u32  dataNum       blocks that follow
  +0x0c  u32  dataSize      words per block, 256

0x2201 F_Pcm (per frame)   CCSTRM_FSET_PCM, 12 bytes, then blocks
  +0x00  u32  myID
  +0x04  u16  dataNum       2 to 7 per frame
  +0x06  u16  padding       0x0012 in every chunk
  +0x08  u32  dataSize      256

block        1024 bytes: 256 left samples, then 256 right samples
```

`ccPcmSound` passes the blocks to the IOP untouched. `Decode_Pcm`
(0x0014ddb0) reads the 16-byte `CCSTRM_PCM` onto its stack and uses only
`dataNum` and `dataSize`; `type`, `bitNum` and `trackType` are read by no
code. 132 cutscenes carry
3,000.2 s. A frame carries 1,600 samples (48,000 / 30), except `str6100`
at 800.

## Notes

`OPENING.PSS` carries its audio in an MPEG private stream with an `SShd`
header: format 1, 48,000 Hz, 2 channels, interleave 0x200.

`crates/piney-data` holds the tables (`tables::voice`, generated by
piney-gen from each volume's code, with the file names as paths on the
disc) and reads the lines (`sound::voice`); `crates/piney-audio` runs the
requests (`driver`) and channel 0 (`seword`). Checked: the requests
against the game's own `ccEvVoiceRequest`, `ccEvVoiceStop` and
`evVoicePlay` run in `tools/eemu.py` for every row in both languages and
modes (`tools/sound_ee.py voice-fixture`); the stream against SEWORDS.IRX
itself run in `tools/eemu.py` over 115 lines, event 1's six among them
(`tools/iopemu.py voice-fixture`); the decoded lines against
`tools/sound.py` (`tools/test_sound_rs.py`).

## Unknown

- The 48 kHz rate is the SPU2's fixed streaming rate plus measurements
  (pitch, spectrum, frame timing); no code states it.
- The transfer interrupt's timing is libsd's documented behaviour (at the
  end of each half, `sceSdBlockTransStatus` bit 24 naming the half
  transferring); the cut tail rests on it and was not heard on a console.
- How long the disc takes before a line's first sample.
- Whether a Recovery Drink used by Kite on Quarantine does stop channel 0
  as the code says (above); not run on a console or an emulator.
