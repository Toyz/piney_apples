---
title: SNDDATA.BIN sound banks and PS-ADPCM
status: partial
volumes: INF
covers: INF DATA/SNDDATA.BIN, INF SLUS_202.67:0x00307410 commseTbl, SQ_LOAD tables 0x00307420-0x00308b10, 0x001821d0 ccSndSQLoad, 0x00182050 ccSndCommSeLoad; INF MODULES/SNDBASE.IRX:0x1e44 ccSQDataLoadCD, 0x156c ccCommDataLoadCD; INF MODULES/MODHSYN.IRX:0x047c sceHSyn_Load
worklog: 14, 42, 243
---

# SNDDATA.BIN sound banks and PS-ADPCM

79 sound banks in Sony's SDK formats - `.hd` header, up to three `.sq`
sequences, `.bd` PS-ADPCM sample data - concatenated with no directory. The
index is in the executable.

## Layout

All little-endian.

```
bank                         79 of them, tiling the file from 0 to 0x27c8800
  .hd       hdSize bytes
  .sq       sqSize1 bytes
  .sq       sqSize2 bytes    may be 0
  .sq       sqSize3 bytes    may be 0
  padding   0xff to a 2048 boundary
  .bd       bdSize bytes
  padding   0xff to a 2048 boundary
```

## The index

```
commseTbl           u32[3] at 0x00307410    the common sound-effect bank
  +0x00  ofs        0
  +0x04  hdSize     0x6260
  +0x08  bdSize     0x12a560

SQ_LOAD             24 bytes, all s32; a row of -1 is unused
  +0x00  ofs        byte offset of the bank in SNDDATA.BIN
  +0x04  hdSize
  +0x08  sqSize1
  +0x0c  sqSize2
  +0x10  sqSize3
  +0x14  bdSize
```

`SQ_LOAD` tables: `typeA`, `typeC`, `typeE`, `typeG`, `typeI`, `typeK`,
`typeM`, `typeO` (10 rows each, 0x00307420 - 0x00307ab0) and `piroshi`
(0x00307ba0), reached per field type through `sqDataField` (0x00307bc0);
`sqDataDungeon` (0x00307c20, 11), `sqDataTown` (0x00307d40, 10),
`sqDataTitle` (0x00307e30, 1), `sqDataDesktop` (0x00307e50, 6),
`sqDataToppage` (0x00307ee0, 1), `sqDataEvent` (0x00307f00, 123),
`sqDataStream` (0x00308b10, 28). 122 rows name the 79 banks.

The IOP (`ccSQDataLoadCD`, `SNDBASE.IRX:0x1e44`) reads `.hd` and the `.sq`s
from sector `lsn + (ofs + 2047) >> 11` and the `.bd` from
`lsn + (ofs + hdSize + sqSize + 2047) >> 11`, where `lsn` is its own
`sceCdSearchFile` of `\DATA\SNDDATA.BIN;1` and `sqSize` the three summed.
The `.bd` is streamed to SPU RAM at `commseTbl[2] + 0x5020`; the common bank's
goes to 0x5010.

## .hd chunks

Each chunk: `char[4] "SCEI"` and a 4-character type, both stored
byte-reversed (`IECS` `sreV` = `SCEI` `Vers`), then `u32 size`. Offsets in
`Head` count from the start of the `.hd`; offsets in the index tables count
from their chunk. 0xffffffff is an unused slot.

```
Vers   16 bytes; version 2.0 in all 79
Head   +0x0c hdSize, +0x10 bdSize, +0x14 Prog, +0x18 Sset, +0x1c Smpl,
       +0x20 Vagi, +0x24 seTimbre (-1 in all 79)
Vagi   u32 maxIndex, u32 offset[maxIndex + 1], then per VAG:
         u32 bdOffset, u16 rate, u8 loop, u8 0xff
Smpl   42-byte records: u16 vag, velocity range, base note +0x0b, pan +0x0d,
       volume +0x10, 0xff +0x11, ADSR1 +0x12, ADSR2 +0x14, then key-follow
       and LFO fields
Sset   u8 velCurve, u8 velLow, u8 velHigh, u8 n, u16 sample[n]
Prog   36-byte header (u32 split offset, u8 nSplit, u8 splitSize, volume,
       pan, ...), then nSplit 20-byte splits: u16 sampleSet, keyLow,
       keyCrossfade, keyHigh, ...
```

Field names past those checked (offsets above) follow Sony's SDK and are not
confirmed against code; the synthesizer module is stripped.

## .sq

```
Vers        16 bytes; its size field (+0x08) is where MODMIDI.IRX looks for Sequ
Sequ        +0x0c file size, +0x10 Song (0xffffffff in all 150), +0x14 Midi,
            +0x18 SE sequence, +0x1c SE song - offsets from the file start
Midi        +0x0c max index, +0x10 u32 offsets[] from the chunk: one block in all
block       u32 data offset (6 in all 150), u16 ticks per quarter note (480 in all)
            then events: [varlen delta] status data...
```

The events are MIDI with Sony's changes: note off (8n) carries no velocity
byte; bit 7 set in any data byte means the next event has no delta (it is
masked off); running status. Loops are NRPN controllers (CC 99 0 / CC 6 id
marks a start, CC 99 1 / CC 6 id / CC 38 n jumps back, n = 0 forever); FF 51
sets the tempo, FF 2F ends the sequence. How the sequencer plays them is in
[the sound engine](../engine/sound.md). 150 `.sq` files in 78 banks: 147
loop forever (one loop each), 3 end; 13 change tempo.

### The parts the game leaves out

Every bank leaves out three parts of Sony's format:
- **`Head.seTimbre`** is -1 in all 79. `sceHSyn_Load` (`MODHSYN.IRX:0x047c`)
  reads it only from a `Vers` of 2 or later, which all of these are
  (`Vers` +0x0e is 2), and takes no SE timbres when it is -1.
- **`Sequ`'s Song, SE sequence and SE song** (+0x10, +0x18, +0x1c) are -1
  in all 150 `.sq` files.
- Nothing asks for them either. `SNDBASE.IRX` is the only module that
  imports `MODMIDI.IRX`, and it calls only `sceMidi_Init`, `ATick`,
  `Load`, `SelectMidi`, `MidiPlaySwitch` and `MidiSetLocation`. It plays
  the `Midi` blocks and never selects a Song. None of the ten modules the
  game loads ([the disc](../disc/layout.md)) plays SE sequences; the
  sound effects are notes on the synthesizer's ports
  ([the sound engine](../engine/sound.md)).

Which file a sequencer plays is decided by the loading code, not by the
file list: sequence 0 starts right after the `.hd`, sequence 1 after
`sqSize1`, sequence 2 after `sqSize1 + sqSize2`, and sequences 1 and 2 exist
only when their own size is non-zero. The 15 rows (11 banks) with `sqSize1`
0 - rows 2, 6 and 7 of typeE, typeI, typeK and typeM, and `sqDataDungeon`
rows 3, 7 and 10 - therefore play their second file as both sequence 0 and
sequence 1.

## PS-ADPCM

```
frame       16 bytes, 28 samples
  u8        shift | filter << 4
  u8        flags            bit 0 end, bit 1 repeat, bit 2 loop start
  u8[14]    nibbles, low nibble first, signed

s = (nibble << 12 >> shift) + ((s1 * F0[filter] + s2 * F1[filter] + 32) >> 6)
    clamped to 16 bits
F0 = 0, 60, 115, 98, 122      F1 = 0, 0, -52, -55, -60
```

The rounding is the SPU2's, which nothing on the disc shows. The decoder
follows PCSX2 (`XA_decode_block`: the two products summed, then `+ 32 >>
6`). DuckStation's PS1 SPU (`Voice::DecodeBlock`) shifts each product by
6 on its own, with no rounding. ffmpeg truncates the sum, and over these
banks differs from the rounded sum by up to 112 LSB.

A VAG runs from its `Vagi` offset to the next VAG's. Its first frame is 16
zero bytes. A looped VAG ends with a frame whose flags are 0x03, and loops
from the last frame with bit 2; exactly those have `Vagi.loop` = 1. A
one-shot's end frame is followed by one unplayed `00 07 77 ... 77` frame.

## Counts

1,013 VAGs, 1,402 samples, 1,398 sample sets, 926 programs, 1,398 splits;
314 looped VAGs. Rates: 223 at 11,025 Hz, 140 at 22,050, 341 at 44,100, 1 at
33,074, the rest within a few Hz of those. Over 2,534,127 frames no shift
exceeds 12 and no filter exceeds 4.

## Notes

`typeI[2]` (bank 26) has `bdSize` 0x5cbf0 in its table row - `typeI[0]`'s
value - where its `Head` says 0x4ec30, which is what the file layout agrees
with.

The desktop jukebox (`Wave[52]`, `INF desktop.prg:0x0042b6f0`, `WaveData`:
title, category, fieldType, bgNum, comment, number) names 51 tracks; a jump
table at `0x0034e130` maps its category to the `SQ_LOAD` table (1 desktop,
2 town, 3 field, 4 dungeon, 5 event, 6 stream, 7 title), and one at
0x0034e110 to the matching volume table (`sqVolTbl*`: per row three
`SQTBL {int midiPort, int hdPort, u16 vol}`). Rows 7, 27 and 47 play the
bank's second sequence, every other row its first; see
[the sound engine](../engine/sound.md).

`crates/piney-data` reads all of this (`piney_data::sound`): the banks
through the tables `piney-gen` reads from the executable into the build,
the `.hd` and `.sq` chunks and PS-ADPCM, checked against `tools/sound.py`,
`tools/scei.py` and `tools/adpcm.py` bank by bank and sample by sample
(`tools/test_sound_rs.py`).

## Unknown

- The SPU2's own ADPCM rounding (above): the emulators disagree, and it
  needs a measurement on the console.
