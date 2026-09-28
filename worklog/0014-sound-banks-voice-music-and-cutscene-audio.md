---
number: 14
title: Sound banks, voice, music and cutscene audio
date: 2026-09-22
area: audio, iop, format, tooling
files: tools/adpcm.py, tools/scei.py, tools/sound.py, tools/wav.py, tools/test_sound.py, docs/formats/snddata.md, docs/formats/voice.md
supersedes: 5
---

# 14. Sound banks, voice, music and cutscene audio

Every kind of audio on the Infection disc is decoded and exported:
1,245 WAVs under `work/infection/audio/`. There are four paths, each found by
reading both the EE code and the IOP modules that do the playing. Two of the
IOP modules, `MODULES/SNDBASE.IRX` and `MODULES/SEWORDS.IRX`, shipped with
symbols and DWARF like the EE executable ([[2]]), from
`C:\Usr\Rpg\Iop\sndbase\` and `C:\Usr\Rpg\Iop\sewords\`; `tools/dwarf1.py`
reads them as they are. IRX addresses below are module-relative.

## Sound banks: DATA/SNDDATA.BIN

79 banks in Sony's SDK sound-bank formats - a `.hd` header, up to three `.sq`
sequences, a `.bd` of PS-ADPCM samples - packed back to back and padded with
0xff to 2048 bytes. Like `DATA.BIN` ([[5]]) the archive has no directory; the
executable indexes it. `commseTbl` (`INF SLUS_202.67:0x00307410`) places the
common sound-effect bank, and `SQ_LOAD` rows (24 bytes: offset, `.hd` size,
three `.sq` sizes, `.bd` size) in tables per context - field types, dungeons,
towns, title, desktop, bulletin board, 123 events, 28 streams - place the
rest; 122 rows name the 79 banks. `ccSndSQLoad` (`0x001821d0`) sends them to
the IOP, where `ccSQDataLoadCD` (`SNDBASE.IRX:0x1e44`) finds the file with its
own `sceCdSearchFile`, reads `.hd`+`.sq` to IOP memory and streams `.bd` to
SPU RAM from sector `lsn + (ofs + hdSize + sqSize + 2047) >> 11`.

Checked over all 79: they tile the file exactly, from 0 to 0x27c8800; all
163,344 padding bytes are 0xff; every `.hd` size matches its table row, and
78 of 79 `.bd` sizes do. The exception is `typeI[2]` (bank 26): the table
repeats `typeI[0]`'s `.bd` size, 0x5cbf0, where the bank's own header says
0x4ec30 - and the next bank starts where 0x4ec30 says it should. A data bug
in the shipped table; the IOP would read 0xdfc0 bytes too many, which is
presumably harmless.

Inside, the `.hd` chunks (`SCEI` + type, stored byte-reversed as `IECS`...)
hold 1,013 VAGs, 1,402 samples, 1,398 sample sets and 926 programs, with
every cross-reference resolving. Sample rates: 223 at 11,025 Hz, 140 at
22,050, 341 at 44,100, the rest near those. 150 `.sq` files each hold one
MIDI block. The desktop's jukebox (`Wave[52]`, `INF desktop.prg:0x0042b6f0`)
names 51 of the banks' tracks - "BGM 01" to "BGM 50" and "Original 01" -
through a category jump table at `0x0034e130`; the music is sequenced MIDI
over ADPCM samples, not streamed.

PS-ADPCM is decoded by `tools/adpcm.py`: 16-byte frames of 28 samples, shift
and filter in byte 0, flags in byte 1, nibbles low first. It was checked
against ffmpeg's `adpcm_psx` over 81 VAGs (4,375,000 samples): bit-exact once
ffmpeg's arithmetic is substituted (truncating division, unclipped history);
with the psx-spx `+32` rounding kept, the largest difference is 112 LSB. The
`Vagi` loop byte is 1 exactly when the ADPCM ends in a frame with flags 0x03 -
314 looped VAGs; the 699 one-shots are each followed by one unplayed
`00 07 77...` frame.

## Voice: VOICE/ and VOICE_E/

Headerless mono signed 16-bit little-endian PCM at 48 kHz. `VOICE_DATA`
tables (`{int ofs; int siz;}`, -1 for no line) in main and gcmn - the
per-character tables `voiceKiteTbl` ... `voiceHerubaTbl` among them - give
each line's byte range; `evVoicePlay` (`0x0017eca0`) sends the file name,
offset and size to the IOP with command 0x80e0, and `wordPlay`
(`SEWORDS.IRX:0x2c74`) seeks to it and sets the stream to mono
(`BgmSetMode(ch, 0x10)`). English files are used when `ccSaveData.voice`
(+0x842c) is set. 54 files hold 8,208 lines: 4,117 Japanese (7,261.7 s) and
4,091 English (6,840.7 s). Every line is 2048-aligned, and every byte between
and after lines is 0xff, so the tables cover the files completely. Five files
the tables name - `EVVOL1P`, `EVVOL2`-`EVVOL4`, `BOSSTALK` - are not on this
disc.

Mono was measured, not assumed: lag-1 autocorrelation beats lag-2 (0.980
against 0.925 over the 172 lines of `SPC00_E`), where interleaved stereo shows
the reverse. At 48 kHz the voices' fundamental frequencies sit in the human
range (medians 274 Hz and 302 Hz for Kite's and BlackRose's battle shouts,
176 Hz for event dialogue). Almost nothing is above 16 kHz, which suggests
32 or 24 kHz recordings resampled to 48 kHz; an inference.

## Music streams: VOICE/BGM.BIN

Two stereo tracks, interleaved 16-bit LE at 48 kHz, from `bgmWavTbl`
(`INF gcmn.prg:0x006317d0`) via `wavPlay` (`0x0017e6e0`) and command 0x80f0.
Track 0 is 88.23 s and loops, played from `ccPuccigusoStart` (gcmn) - the
Grunty race, going by the name; track 1 is 239.96 s and plays once, from
`ccThStaffRoll` (desktop) - the credits. `bgm_r2s.s` in SEWORDS splits the LRLR
words into the SPU's block layout.

## Cutscene audio: Pcm chunks in STREAM/

The CCSF `Pcm` (0x2200) and `F_Pcm` (0x2201) chunks of [[10]] carry stereo
16-bit PCM in 1024-byte blocks of 256 left samples then 256 right samples;
`ccPcmSound` ring-buffers them to the IOP untouched. Shown by the seam test:
inside a block, the jump from sample 255 to 256 averages 5,405 against 519
elsewhere, while across a block boundary it is continuous. 132 cutscenes carry
3,000.2 s of audio; at 48 kHz a frame holds almost exactly 1,600 samples, so
the cutscenes run at 30 frames per second (inferred; not traced in the
player).

`OPENING.PSS` carries its audio in a private stream with an `SShd` header:
48 kHz, 2 channels, interleave 0x200.

## A correction to [[5]]

[[5]] gave `ccCdInit__Fv` as `0x00159720`. The function starts at
`0x00159620` (624 bytes); `0x00159720` is inside it, where it looks up
`\DATA\DATA.BIN;1` (the relocation there reads `ccCdInit__Fv+0x10c`). A scan
of main and all four overlays for loads from `ccCd+0x30`/`+0x34` found no
reader of the `SNDDATA.BIN` lookup it stores there - the IOP does its own
`sceCdSearchFile` - but the scan is heuristic, not a proof.

## Tools

`tools/sound.py` reads the game's indexes (`banks`, `bank`, `export-banks`,
`voices`, `export-voices`, `bgm`, `stream`, `streams`), on top of
`tools/scei.py` (the `.hd`/`.sq` formats), `tools/adpcm.py` and `tools/wav.py`.
`tools/test_sound.py` has 20 tests.

**Still unknown:** the 48 kHz rate for the PCM paths rests on the SPU2's fixed
input rate and the measurements, not on anything the code states; the SPU2's
exact ADPCM rounding; the `.sq` MIDI content and which sequence of a bank the
jukebox plays; the sound-effect maps (`seData`, the SE tables, `strSndTbl`);
what `ccSaveData.parodyFlag` changes in `ccEvVoiceRequest`; the meaning of the
`Pcm` chunk's `type`, `bitNum` and `trackType`.
