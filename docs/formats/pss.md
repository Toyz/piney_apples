---
title: PSS movies
status: partial
volumes: all
covers: INF PSS/LOGO_B.PSS, INF PSS/LOGO_C.PSS, INF PSS/LOGO_H.PSS, INF PSS/OPENING.PSS; INF SLUS_202.67:0x00116c68 sceMpegDemuxPssRing, 0x001169e0 _type2id, 0x002f75f8 _strmap, 0x00117ac0 sceMpegGetPicture, 0x00118010 _decodeOrSkipFrame, 0x00113ab0 _decMB0, 0x00116010 _doCSC, 0x00116128 _ch3dmaCSC, 0x00119218 sceIpuInit, 0x0010ad20 ccSystem::SetScreenMode, 0x0010ad40 ccSystem::SetScreenModeMain, 0x00104a60 ccView::GetScreenClip, 0x00179590 ccSetInputVol; INF demo.prg:0x00409df0 ccDecodeMpeg, 0x00409fa0 defMain, 0x00409fe0 readMpeg, 0x0040a2b0 isAudioOK, 0x0040a340 initAll, 0x0040a680 termAll, 0x0040a940 setImageTag, 0x0040ad40 vblankHandler, 0x0040af60 handler_endimage, 0x0040afa0 startDisplay, 0x0040b000 endDisplay, 0x0040b140 pcmCallback, 0x0040b560 strFileOpen, 0x0040cd80 videoDecMain, 0x0040cdf0 decBs0, 0x0040d2c0 audioDecCreate, 0x0040d3e0 audioDecStart, 0x0040d460 audioDecReset, 0x0040d510 audioDecBeginPut, 0x0040d5e0 audioDecEndPut, 0x0040d670 audioDecIsPreset, 0x0040d690 audioDecSendToIOP, 0x0040db60 changeInputVolume; INF SLUS_202.67:0x001159c8 _markOutput, 0x00115370 _outputFrame, 0x00115b80 _dispRefImage, 0x00117888 sceMpegCreate, 0x00117bf8 sceMpegReset; MUT PSS/OPENING2.PSS; OUT PSS/OPENING3.PSS; QUA PSS/OPENING4.PSS
worklog: 49, 245
---

# PSS movies

Sony's PSS: an MPEG-2 program stream with one MPEG-2 video stream and, in
`OPENING.PSS`, one audio stream of SPU2-format PCM in private stream 1.
Only the title plays them, through `ccDecodeMpeg` (demo.prg 0x00409df0,
[the title screen](../engine/title.md)), which is Sony's `libmpeg` sample
player built into DEMO.PRG (`mpeg.cpp`). The desktop's Audio screen movies
are not PSS: they are in-engine streams (`Audio_control::SimplePlayStream`,
desktop.prg 0x00407100, starts `ccThExecuteStream`).

| file | bytes | packs | pictures | at 30 / s | audio |
| --- | ---: | ---: | ---: | ---: | --- |
| `PSS/LOGO_B.PSS` | 3,227,652 | 197 | 146 | 4.87 s | none |
| `PSS/LOGO_C.PSS` | 3,997,700 | 244 | 180 | 6.00 s | none |
| `PSS/LOGO_H.PSS` | 3,244,036 | 198 | 151 | 5.03 s | none |
| `PSS/OPENING.PSS` | 77,299,716 | 4,718 | 2,771 | 92.37 s | 92.33 s, 48 kHz stereo |

The logos are Bandai, CyberConnect2 and Project .hack, in the order
`LogoMain` plays them before the opening.

## Layout

The container is big-endian, as MPEG is; the audio samples inside are
little-endian.

```
file           N packs of 16,384 bytes, then 00 00 01 B9 (the last 4 bytes)
pack           00 00 01 BA, then 10 bytes (MPEG-2 pack header, no stuffing):
                 SCR 3000 more per pack (90 kHz); mux_rate 12,800 (x 50 B/s)
                 in the logos, 16,640 in OPENING (3,840 in its last packs)
               then 4 PES packets: three of 4,096 bytes and one of 4,082
               (the pack header's 14 bytes taken out); the first pack also
               holds the system header, the last packs padding
system header  00 00 01 BB, length 9: rate_bound, one stream entry (0xE0,
               P-STD buffer 1,542 x 1024 bytes); the audio is not listed
PES packet     00 00 01 id, u16 length, then the MPEG-2 PES header
                 +0  0x83               '10', original, copyright
                 +1  0x00 no stamps, 0x80 PTS, 0xC0 PTS and DTS; 0xC1 with
                     a PES extension (the first video packet only)
                 +2  header_data_length: 10 (13 with the extension), the
                     stamps then 0xFF stuffing
               then the payload
  id 0xE0      video: the MPEG-2 elementary stream, cut anywhere
  id 0xBD      private stream 1: 4-byte PSS sub-stream header, then data
                 +0  0xFF
                 +1  kind: 0xA0 PCM (sceMpegStrPCM), 0xA1 PS-ADPCM
                 +2  0x00
                 +3  channel: 0
  id 0xBE      padding, in the last packs
```

The sub-stream header is how `libmpeg` routes the audio:
`sceMpegAddStrCallback` keys each stream with `_type2id` (0x001169e0) from
`_strmap` (0x002f75f8), 40-bit id and mask pairs: video `E0 ch` over the
stream id, PCM `BD FF A0 00 ch`, ADPCM `BD FF A1 00 ch`, data `BD FF 90 00
ch`, all five bytes compared. `sceMpegDemuxPssRing` (0x00116c68) walks the
packs and hands each matching payload to its callback. The game registers
video channel 0 and, with audio, PCM channel 0 (`initAll`, demo.prg
0x0040a340).

The first video and audio packets carry PTS 3,874 in the logos (0.043 s) and
4,119 in OPENING (0.046 s, both streams). Nothing in the game's player uses
the stamps to pace anything (see [the player](#the-player)).

| file | video packets | audio packets | padding |
| --- | ---: | ---: | ---: |
| LOGO_B | 786 | 0 | 2 |
| LOGO_C | 975 | 0 | 2 |
| LOGO_H | 791 | 0 | 2 |
| OPENING | 14,513 | 4,356 | 3 |

## The audio

The audio payloads, joined, start with a 40-byte header (`SpuStreamHeader`
and `SpuStreamBody` in the DWARF), little-endian:

```
+0x00  char[4]  id         "SShd"
+0x04  s32      size       0x18
+0x08  s32      type       1
+0x0c  s32      rate       48000
+0x10  s32      ch         2
+0x14  s32      interSize  0x200
+0x18  s32      loopStart  -1
+0x1c  s32      loopEnd    -1
+0x20  char[4]  id         "SSbd"
+0x24  s32      size       bytes of sound that follow: 17,727,488
+0x28  ...      the sound
```

The sound is signed 16-bit PCM in 1,024-byte sets: 256 left samples, then
256 right samples (`interSize` bytes a channel). That is the SPU2's own
streaming layout, and the game never converts it: `pcmCallback` (0x0040b140)
skips the 4-byte sub-stream header and copies the rest into `audioDec`'s EE
ring (0xC000 bytes); the first 40 bytes go to the header (`audioDecBeginPut`
0x0040d510, `audioDecEndPut` 0x0040d5e0), which no code reads afterwards -
not `type`, not `rate`. `audioDecSendToIOP` (0x0040d690) moves 1,024-byte
multiples by SIF DMA into a 0x6000-byte IOP buffer, and `audioDecStart`
(0x0040d3e0) sets core 0's sound data input volume to 0x7FFF
(`changeInputVolume` 0x0040db60 -> `ccSetInputVol` 0x00179590: BVOLL and
BVOLR) and starts `sceSdBlockTrans(0, SD_TRANS_MODE_WRITE_FROM |
SD_BLOCK_LOOP, buffer, 0x6000, ...)` through `sceSdRemote` (0x80E0): the
SPU2 plays the buffer as core 0's input at its fixed 48 kHz. The
input then mixes on through core 0 and core 1 like `BGM.BIN`'s streams
([sound](../engine/sound.md)). `audioDecReset` (0x0040d460) sets the input
volume to 0 and stops the transfer.

17,727,488 bytes are 17,312 sets, 4,431,872 samples a channel: 92.33 s.

## The video

Every picture of every file is the same kind of MPEG-2:

| | |
| --- | --- |
| size | 640 x 448, aspect_ratio_information 1 (square samples) |
| frame_rate_code | 5: 30 frames a second (not 29.97) |
| bit_rate, vbv_buffer_size | 12,800 (5.12 Mbit/s), 96 |
| profile_and_level | 0x48: Main profile, Main level |
| progressive_sequence, chroma_format | 1, 1 (4:2:0) |
| intra matrix | loaded: 8 at DC, 32 everywhere else |
| non-intra matrix | the default (all 16) |
| extensions | sequence display (video_format 2, NTSC, with colour description); no quant matrix extension, no scalable extension |
| GOPs | 9, 10, 9 and 160; the first closed (decode order `I P B B P B B ...`), the rest open (`I B B P B B ...`, the two B pictures shown before their I); mostly 18 pictures, shorter at scene cuts |
| pictures | LOGO_B 9 I, 41 P, 96 B; LOGO_C 10, 51, 119; LOGO_H 9, 42, 100; OPENING 160, 783, 1,828 |

and every picture coding extension has picture_structure 3 (frame
pictures; no field pictures anywhere), progressive_frame 1,
frame_pred_frame_dct 1 (every prediction a frame prediction, every DCT a
frame DCT), concealment_motion_vectors 0, q_scale_type 1 (the non-linear
quantiser scale), intra_vlc_format 1 (table B.15 for intra blocks),
alternate_scan 0, top_field_first 1, repeat_first_field 0.
intra_dc_precision is 0 (8 bits) in the logos and 1 (9 bits) in OPENING.
The f_codes reach 4 (vectors to +-64 samples).

## The player

`ccDecodeMpeg(name, audio)` (0x00409df0):

```
sceDevctl("cdrom0:", 0x4382, {16}, 1)
ccSys->SetScreenMode(640, 448, 2)             applied next frame
if changeScreen_hf >= 0: ccBreathThread(1)
sceGsSyncVCallback(vblankHandler); isWithAudio = audio & 1
initAll(name): buffers; videoDec (sceMpegCreate); audioDec (+0xC000 EE,
  0x6000 IOP); the stream callbacks; voBuf of 2 pictures; the threads
  defMain (readMpeg) and videoDecMain (decBs0); strFileOpen until it opens
retFlag = 0; thEndFlag = 0
while !thEndFlag: ccBreathThread(1)           the game's tasks keep breathing
termAll(); ccBreathThread(1)
SetScreenMode(512, 448, 0)
return retFlag
```

`strFileOpen` (0x0040b560) opens `\PSS\` + the name upper-cased, streamed
with `sceCdStStart`.

**Reading** (`readMpeg` 0x00409fe0, on `defMain`): read the file into a
0x50000-byte ring, demultiplex what is there, send audio to the IOP; the
first time `voBuf` is full (two pictures) and, with audio,
`audioDecIsPreset` (0x0040d670: the IOP buffer's 0x6000 bytes sent), call
`startDisplay(1)` and `audioDecStart` together: picture and sound begin on
the same vblank. At the end of the file it flushes the decoder, waits for
it, calls `endDisplay`, `audioDecReset` and sets `thEndFlag`.

**Skip**: each pass of the reading loop, if `ccSys.pad[0].push` (+0x2d0)
has any of `saveData.assignPADok`, `saveData.assignPADcancel` (+0x840e,
+0x8410) or 0x800 (START) and `videoDec->mpeg.frameCount >= 11`, it calls
`videoDecAbort` and sets `retFlag` -1; the stream then ends as above.
`frameCount` is the index of the last picture the decoder produced
(`_decodeOrSkipFrame` 0x00118010 stores it before counting the picture), so
the twelfth picture must have been decoded; the decoder runs two pictures
ahead of the display.

In libmpeg's own terms, `_decodeOrSkipFrame` stores `frameCount` as its
picture counter (private +0x118, one per picture decoded, counted after the
store) less a base at private +0xac. `sceMpegCreate` and `sceMpegReset` zero
both. `_markOutput` (0x001159c8), reached from `_dispRefImage` when a
reference picture is shown, sets the base to the counter the first time. A
stream opens on an I picture, which is shown when the second picture is
decoded, so the base is 1: from then on `frameCount` is pictures decoded
less 2, the display-order index of the last picture out.

**Decoding** (`decBs0` 0x0040cdf0, on `videoDecMain`):
`sceMpegGetPicture(mpeg, voBuf slot, 1350)` decodes the next picture in
display order into RGBA32 through the IPU (below); the first time, it
builds both display packets of every `voBuf` slot (`setImageTag`, field 0
and 1); then marks the slot ready (status 2).

**Showing** (`vblankHandler` 0x0040ad40, every vblank): with a picture
ready and the counting started, on an even field (GS CSR bit 13 clear) and
status 2 it swaps the double buffer and sends the slot's field-0 packet
(status 1); on the odd field and status 1 it swaps and sends the field-1
packet (status 0); when that transfer ends, `handler_endimage` frees the
slot. So each picture is shown for exactly two vblanks, 29.97 pictures a
second on NTSC, whatever the stream's rate and time stamps say; a picture
the decoder is late with leaves the last one up.

**The draw** (`setImageTag` 0x0040a940): field 0's packet uploads the
picture - one 16 x 16 block per IPU macroblock, BITBLTBUF to
`ccSys.buffEndAdrs` (+0xbbc, past the frame and Z buffers), PSMCT32, buffer
width 768; the IPU's blocks come in column order, top to bottom then left
to right - and both packets then draw it:

```
TEXFLUSH; TEX1_1 0x60 (MMAG = MMIN = 1: bilinear)
TEX0_1   buffEndAdrs, TBW 12, PSMCT32, TW = TH = 10, TCC 0, TFX DECAL
TEST_1   ZTE, ZTST ALWAYS; ZBUF_1 ZMSK; RGBAQ 0x80808080
PRIM     SPRITE, TME, FST (no ABE)
UV (8, 8), XYZ2 (clip x0, y0); UV (16 w + 8, 16 h + 8), XYZ2 (clip x1, y1)
```

The corners are the active layer's view clip (`ccView::GetScreenClip`
0x00104a60). `SetScreenModeMain` (0x0010ad40) re-runs every view's
`SetFrame` on a mode change, so with the 640-wide mode the title's view
covers the whole 640 x 448 screen; mode 2 is interlaced field mode (a
640 x 224 buffer a field, `sceGsResetGraph(0, 1, 2, 1)`, and
`sceGsSetHalfOffset`), so each field draws the whole picture at half
height. On the TV the picture fills the
screen the game's own 512-wide frames fill: no letterbox, no crop. The
game's CLAMP register is not set here.

**The colour conversion** is the IPU's CSC command. libmpeg sends it with
OFM 0 (RGBA32) and DTE 0 (no dither) (`_doCSC` 0x00116010 writes
`0x70000000 | mbc`; `_ch3dmaCSC` 0x00116128 likewise), and `sceIpuInit`
(0x00119218) sends SETTH with both thresholds 0, so alpha is 0x80
throughout. The IPU's conversion is BT.601 studio range in fixed point
(the EE User's Manual's IPU chapter, as PCSX2's `yuv2rgb_reference`
transcribes it):

```
lum = (0x95  * max(0, Y - 16)) >> 6     1.1640625
rcr = (0xcc  * (Cr - 128)) >> 6         1.59375
gcr = (-0x68 * (Cr - 128)) >> 6        -0.8125
gcb = (-0x32 * (Cb - 128)) >> 6        -0.390625
bcb = (0x102 * (Cb - 128)) >> 6         2.015625
R = clamp((lum + rcr + 1) >> 1)         >> floors; clamp to 0..255
G = clamp((lum + gcr + gcb + 1) >> 1)
B = clamp((lum + bcb + 1) >> 1)
A = 0x80
```

Chroma is not interpolated: the four luma samples of each 2 x 2 square share
one Cb and one Cr. The sequence display extension's colour description is
ignored.

The IDCT is the IPU's too (libmpeg's `_decMB0` 0x00113ab0 sends BDEC, which
decodes, inverse quantises and transforms each block in hardware; motion
compensation is done by the EE). Its arithmetic is not documented to the
bit.

## Other volumes

The logos are the same bytes on all four discs. Each later disc has its own
opening, which its `DEMO.PRG` names (`opening2.pss`, `opening3.pss`,
`opening4.pss`), made the same way as Infection's:

| file | bytes | packs | mux_rate | pictures (I, P, B) | video bit_rate, vbv | intra_dc_precision | audio |
| --- | ---: | ---: | --- | --- | --- | ---: | ---: |
| INF `OPENING` | 77,299,716 | 4,718 | 16,640 | 160, 783, 1,828 | 12,800, 96 | 1 | 92.33 s |
| MUT `OPENING2` | 111,067,140 | 6,779 | 24,320 (20,480 in 113 packs) | 156, 764, 1,813 | 20,480, 112 | 0 | 89.30 s |
| OUT `OPENING3` | 73,433,092 | 4,482 | 16,640 | 153, 739, 1,738 | 12,800, 96 | 0 | 87.66 s |
| QUA `OPENING4` | 75,743,236 | 4,623 | 16,640 | 153, 756, 1,806 | 12,800, 96 | 1 | 90.50 s |

All are 640 x 448 at frame_rate_code 5, Main profile at Main level,
progressive, frame pictures only, with the same picture coding extension
apart from `intra_dc_precision`. Every audio stream is one PCM sub-stream
(`FF A0 00 00`) with the same `SShd` (type 1, 48,000 Hz, 2 channels,
`interSize` 0x200), and its first PTS equals the video's (3,764 in
Mutation's, 4,116 in Outbreak's, 4,119 in Quarantine's). Mutation's is the
one at a higher rate: 8.19 Mbit/s against 5.12.

The players are Infection's:
- **Mutation's** is the same code, apart from one padding `nop` after
  `endDisplay`.
- **Outbreak's and Quarantine's** are each other's, and Infection's code
  built by a different compiler. The functions differ in register choice,
  instruction order and stack layout (a `& 0x0fffffff` becomes a shift
  pair), but compared function by function they make the same calls with
  the same constants and field offsets.

## The port

`piney_data::pss` demultiplexes the files and reads the audio;
`crates/piney-mpeg` decodes the video with its own MPEG-2 decoder (what the
table above uses; field pictures, field and dual-prime prediction and
concealment vectors are refused), converts with the CSC above and plays as
described. Its IDCT is integer, in the arithmetic of ffmpeg's "simple" IDCT,
and passes IEEE 1180-1990 (`idct::tests::ieee1180`).

Checked (`crates/piney-mpeg/tests/ffmpeg.rs`, against ffmpeg's `mpeg2video`
as an outside reference): all 3,248 pictures of the four files are ffmpeg's
`-f rawvideo -pix_fmt yuv420p` output byte for byte, in the same number and
order. So are the later openings' 2,733, 2,630 and 2,715
(`later_openings_match_ffmpeg_exactly`). Against ffmpeg with another IEEE 1180 IDCT (`-idct int`) the worst
picture is 57.96 dB PSNR (OPENING; the logos 67.50 dB and better), drift over
each group of pictures included; a guess at the size of difference the
IPU's own IDCT makes, which is not measured. The audio is checked against the set layout read
independently from the packets.

## Notes

`SShd.type` is 1 in all four openings, and no code reads it. The other
values belong to Sony's format and appear on no disc.

## Unknown

- The IPU's IDCT arithmetic, and so how far the console's pictures are from
  ours (by the measurement above, well under a level on average).
- What the SPU2 plays after the last audio set: the IOP buffer loops
  (`SD_BLOCK_LOOP`) until `audioDecReset` stops it when the video ends,
  0.1 s later at the console's rate; whether that tail is silence or a
  replay of the buffer is not checked.
- How many frames the console takes between `ccDecodeMpeg`'s call and the
  first picture (the disc read filling `voBuf` and the audio preload).
