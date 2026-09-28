---
number: 42
title: The game's sound in Rust: its driver, sequencer and synthesizer checked against the IOP modules
date: 2026-09-23
area: audio, iop, test, build
files: crates/piney-audio, crates/piney-data/src/sound, tools/iopemu.py, tools/irx.py, tools/midi.py, tools/hsyn.py, tools/sound_ee.py, tools/sound_tables.py, tools/test_sound_rs.py, docs/engine/sound.md
---

# 42. The game's sound in Rust: its driver, sequencer and synthesizer checked against the IOP modules

[[14]] decoded every sound on the disc to WAV. A game has to play them the
way the game does: which sample a sound effect number sounds, at what pitch
and volume; how the desktop's music is chosen, played and looped. A helper
agent worked this out and ported it in its own git worktree, and I merged
it and re-ran its checks. The reference is [the sound page](../docs/engine/sound.md).

## The music is not streamed

The task assumed the desktop's music streamed. It does not. Every jukebox
piece is a MIDI sequence (`.sq`) played over a sample bank by the IOP's
own modules:
- **MODMIDI.IRX** is the sequencer.
- **MODHSYN.IRX** is the hardware synthesizer that drives the SPU2's
  voices.
- **SNDBASE.IRX** ticks both every 4,167 microseconds.

Only `VOICE/BGM.BIN`'s two tracks are streamed PCM: one loops in gcmn, the
other is the staff roll.

## Reaching the IOP

The IOP's modules had never been read. The agent wrote:
- `tools/irx.py`, an IRX disassembler that names a stripped module's
  exports from other modules' import stubs;
- `tools/iopemu.py`, which runs IOP modules in `eemu.py` with libsd's
  register writes recorded instead of emulated.

It then modelled MODMIDI (`tools/midi.py`) and MODHSYN (`tools/hsyn.py`),
each against its real module, run.

## The port

- **`piney_data::sound`** decodes the banks (`.hd` / `.bd`), PS-ADPCM and
  the sequences. `tools/sound_tables.py` generates the executable's
  tables: the bank index, `seData` and the jukebox.
- **`piney-audio`** is in four layers:
  - the EE driver (`ccSeOn`, `ccSndChangeData`, the jukebox fade and the
    volume options);
  - MODMIDI's sequencer;
  - MODHSYN's synthesizer;
  - an SPU2 model (interpolation, ADSR, pitch counter, the hall reverb)
    mixing at 48 kHz.

  Output goes to cpal, falling back to silence, or headless to WAV.
- **`piney-game`** now plays what the desktop asks for:
  - the start-up music (`desktop_bgm`);
  - the jukebox's changes (`bgm`);
  - every sound effect.

  `audio.frame()` runs once per game frame; `--mute` turns it off. A new
  game starts at `ccSound`'s volumes of 256.

## Checked

**Decoding**, bit for bit against `sound.py`:
- 79 banks' every header field;
- 1,013 VAGs: PCM, loop points and clip counts;
- 150 sequences' offsets;
- BGM.BIN.

**The EE rules**, run in eemu. The Rust driver sends the identical command
stream for:
- `ccSeOn` for all 237 sound effects;
- `ccSndChangeData` for all 51 desktop rows;
- the volume setters;
- ten jukebox changes with their fade frames.

**The sequencer.** The model matches MODMIDI byte for byte on all 150
sequences through their first loop. The Rust matches the model over 40,000
ticks each.

**The synthesizer.** The Rust reproduces MODHSYN's own register writes:
- every sound effect, at several port volumes;
- 12 jukebox pieces over 12,000 ticks.

**Renders.** Sound effect 4 lasts its sample's length. Every sound effect
but seven sounds, and for those seven MODHSYN starts no voice either. The
music renders are non-silent, unclipped and DC-free.

Two game bugs are kept:
- a desktop started with `dtBgm` 47 plays row 46's music;
- the jukebox's odd and even frame port writes.

On integration:
- `cargo test -p piney-audio` passes;
- `cargo test -p piney-data --test sound` passes;
- `test_sound_rs.py` is OK;
- clippy and fmt are clean.

**Still unknown:**
- **The hardware.** The SPU2 itself (envelope timing, ENVX/ENDX,
  interpolation, reverb) comes from PlayStation documentation, not the
  hardware. In the harness ENVX/ENDX read 0, so voice freeing and stealing
  are checked only under that model.
- **Other contexts.** The field, town, dungeon and event music contexts and
  battle-music switching are not ported. Their tables are generated.
- **Other sound.** Voice lines, cutscene PCM, the 3D sound effect API and
  `sdCommand`'s reverb switches are not ported.
