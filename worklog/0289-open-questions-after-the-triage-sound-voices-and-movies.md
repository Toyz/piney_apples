---
number: 289
title: Open questions after the triage: sound, voices and movies
date: 2026-09-28
area: audio, video
files: UNKNOWNS.md
resolves: 1, 14, 42, 46, 49, 52, 56, 60, 85, 89, 95, 101, 102, 150, 176, 184, 185, 188, 203, 226, 239, 240, 243, 245, 249, 267, 269, 278
---

# 289. Open questions after the triage: sound, voices and movies

This entry closes the open questions of 28 entries about sound and pictures
in motion: the sound banks, the driver and the music contexts, voices, the
PSS movies and the in-engine streams. The triage of 2026-09-28 (UNKNOWNS.md)
split every entry's open paragraph into single questions and checked each
one against the later log, the docs and the code. The answered ones are
listed below with their evidence. What is still open is restated at the end,
with the entry that asked it, the play questions first. Open questions of
these entries that belong to another subsystem are restated in that
subsystem's entry: [[282]], [[286]], [[288]], [[290]] and [[291]].

## Answered

- [[1]]: `SNDDATA.BIN`'s format — answered by [[14]] /
  docs/formats/snddata.md
- [[1]]: `ICON.BIN`'s format — answered by [[16]] (`iconBinTbl`, twelve
  icons) / docs/formats/save.md; noted in [[239]]
- [[1]]: the voice banks' format — answered by [[14]] /
  docs/formats/voice.md
- [[1]]: `BGM.BIN`'s format — answered by [[14]] / docs/formats/voice.md
  "Streamed music"
- [[1]]: the `STREAM/` files' format — answered by [[14]] (`Pcm` chunks),
  [[52]] / docs/formats/ccs.md, docs/formats/voice.md
- [[1]]: whether `OUTSIDE.BIN` is read by anything — answered by [[239]] (no
  string `OUTSIDE` in any volume's code)
- [[1]]: what `IOPRP243.IMG` replaces in the IOP's ROM modules — answered by
  [[239]] / docs/disc/layout.md "The IOP image" (15 ROMDIR modules)
- [[14]]: the `.sq` MIDI content, and which sequence of a bank the jukebox
  plays — answered by [[42]] (the sequencer) /
  `crates/piney-data/src/sound/mod.rs` `Tables::wave_sequence` (1 for rows
  47, 27 and 7)
- [[14]]: the sound-effect maps (`seData`, the SE tables, `strSndTbl`) —
  answered by [[42]], [[81]] / docs/engine/sound.md; [[243]] notes the
  question was stale
- [[14]]: what `parodyFlag` changes in `ccEvVoiceRequest` — answered by
  [[95]] / docs/formats/voice.md (events 0-49 have no voice in Parody Mode)
- [[14]]: the `Pcm` chunk's `type`, `bitNum` and `trackType` — answered by
  [[245]] (`Decode_Pcm` uses only `dataNum` and `dataSize`)
- [[42]]: the field, town, dungeon and event music contexts, and
  battle-music switching — answered by [[60]], [[185]] (towns), [[80]]
  (battle music; docs/engine/sound.md, `bgmChange`)
- [[42]]: voice lines, cutscene PCM, the 3D sound effect API — answered by
  [[46]], [[95]] (voices), [[52]] (stream PCM), [[81]] (`ccSeOn3D`)
- [[46]]: the logos, `OPENING.PSS` and the intro stream counted as played,
  and their audio — answered by [[49]], [[52]]
- [[46]]: the field's voices (`ccVoiceRequest`'s negative ids, per-character
  voices) — answered by [[95]], [[101]]
- [[46]]: the title's lit icons differing from the CPU GS model — answered
  by [[47]] (the directional term was doubled)
- [[49]]: the other volumes' movies — answered by [[245]]
  (`later_openings_match_ffmpeg_exactly`)
- [[52]]: the streams' effect objects and effect tasks (`StreamDemoFuncTbl`)
  — answered by [[54]], [[57]], [[121]], [[122]], [[124]], [[125]], [[126]],
  [[133]], [[156]], [[160]]
- [[52]]: the subtitles under a stream — answered by [[85]]
- [[52]]: the sequenced music around a stream (`ccSndStreamCtrl`) — answered
  by [[85]]; per volume by BUGS.md (`stream_ctrl` takes the volume)
- [[60]]: `ccSndBgmCtrl`'s branches for town types 1-4 — answered by [[185]]
  (types 1 and 3 start sequence 2 with three sequences; every town sequence
  0)
- [[60]]: the crisis flag `ccSnd +0x105` and the three-sequence condition —
  answered by [[185]]
- [[85]]: streams in The World (the town and field hosts) — answered by
  [[90]], [[91]]
- [[85]]: stream 5's eight hit marks (`effHitMarkStr`) — answered by
  [[122]], [[131]]
- [[85]]: the view's clip at `divZ` — answered by [[153]]
- [[85]]: the arc's other nine effect tasks — answered by [[121]], [[122]],
  [[124]], [[126]]
- [[95]]: the fights' calls (`Event::SkillWords`) not wired — answered by
  [[101]]
- [[95]]: Kite calling the party's strategy (`Show::Shout`) — answered by
  [[120]]
- [[101]]: Kite's strategy balloon and the party's chat lines — answered by
  [[120]], [[123]]
- [[102]]: the Data Drain movie — answered by [[105]]
- [[102]]: the side effect's visuals (`effSkillStartEffect`, the fly fonts,
  `effAfterDrain`) — answered by [[141]]
- [[102]]: the noise (`MenuNoise`) — answered by [[104]]
- [[102]]: the enemy's drained form after affect 13 — answered by [[229]]
- [[150]]: `TOBJ`'s shadow (`ccShadowModel`) — answered by [[160]]
- [[188]]: the voices across volumes (the `EvVoice` groups per volume) —
  answered by [[226]], [[227]]
- [[203]]: each later volume's `ccEvVoiceRequest` groups — answered by
  [[226]] (each disc's own, decoded)
- [[203]]: Mutation's `ccVoiceRequest` cases — answered by [[227]]
  (Mutation's field voices, food and skill words)
- [[203]]: Outbreak's and Quarantine's `setbl` rows and their
  `ccSeSetParam*` — answered by [[278]] (`notes_are_the_volumes_own`)
- [[203]]: whether `ccSndStreamCtrl`'s stream numbers changed — answered by
  BUGS.md (`stream_ctrl` takes the volume;
  `the_later_volumes_music_around_the_streams`)
- [[226]]: Mutation's field, skill and food voices — answered by [[227]]
- [[226]]: Mutation's `BOSSTALK.BIN` — answered by [[227]], [[239]] (only
  Outbreak's disc carries it)
- [[239]]: what `STRT.BIN`'s scenes look like — answered by [[240]]
  (`STRT.BIN` played: the demo discs' screens)
- [[267]]: `Func_str1070` — answered by [[269]]
- [[267]]: str7100, str8800 and str0300 changed on Mutation — answered by
  [[271]] (they are Infection's code)

**Still unknown:**
- (play) [[58]]: `SoundFadeOut` stops every sequence at once
  (`crates/piney-game/src/main.rs` maps it to `bgm_stop`), where the game's
  `ccSoundFadeOut` fades over 8 frames — every mode change that fades the
  music.
- (play) [[96]]: the gate hack's cancel: `ccSndGateHackCtrl` restarts
  sequences 0 and 2 at volume 0 and fades them in; the port fades them from
  where they are (docs/engine/field-ui.md) — cancelling a gate hack.
- (play) [[89]]: changing Voice in The World's Options goes through
  `SoundEnv`, which carries the volumes only; the driver may not follow
  until the next mode — the voice language option in the field.
- (volumes) [[203]]: Outbreak's and Quarantine's `ccVoiceRequest` cases;
  [[227]] leaves their `spcVoiceData` to be checked.
- (volumes) [[278]]: who ids 18-20 are on Outbreak and Quarantine
  (`ccCharBaseParam.id` rows) and their tables' sounds; [[239]] names
  `charTbl` rows 18-20 (Tsukasa, Subaru, Sora), the sound tables are not
  followed.
- (volumes) [[267]], [[269]], [[271]]: Outbreak's and Quarantine's versions
  of the stream effect tasks Mutation changed (`Func_str1070`, str7100,
  str8800, str0300) are not compared.
- [[14]]: the 48 kHz rate of the PCM paths rests on the SPU2's fixed input
  rate and on measurements, not on anything the code states
  (docs/formats/voice.md Unknown).
- [[14]], [[243]]: the SPU2's exact ADPCM rounding; the port rounds as PCSX2
  (docs/formats/snddata.md Unknown).
- [[42]]: the SPU2 itself (envelope timing, `ENVX`/`ENDX`, interpolation,
  reverb) is taken from documentation, not hardware (docs/engine/sound.md
  Unknown).
- [[42]]: `sdCommand`'s reverb switches (cases 4 and 5): who asks for them
  (docs/engine/sound.md Unknown).
- [[56]]: what `ccSnd +98` (set by mode 0) does, and whether mode 1 clears
  it.
- [[56]]: the port-0 branch of the volume (the SE volume, sign-extended from
  24 bits) is not modelled on its own.
- [[184]]: what `ccSnd +0x110` is (docs/engine/sound.md Unknown).
- [[95]]: `vBank+0x08` is left as `evVoicePlay` last wrote it; the port
  sends none.
- [[50]]: the load-confirmation jingle's `m_tempPN` is never initialised
  (game UB); the port starts at 0.
- [[176]]: whether the SE port's volume is ever below full when
  `sound 8 0 256` runs.
- [[245]]: whether a Recovery Drink used by Kite on Quarantine stops voice
  channel 0 on a console; the port follows the code (docs/formats/voice.md
  Unknown).
- [[245]]: whether the other characters' skill-word bases reach their
  tables' last rows; only Kite's is traced.
- [[112]]: sound bank 5 is read from the session's `WORLD_MAN`, which for
  the arena is area 27's.
- [[101]]: a request the menus make reaches the sound task a frame after the
  game's (task order).
- [[49]]: the IPU's exact IDCT arithmetic; needs a console.
- [[49]]: what the SPU2 plays after the last audio block until
  `audioDecReset`.
- [[49]]: how many frames the console takes before a movie's first picture.
- [[52]], [[175]]: the frames the disc read and the decoding take are not
  modelled; the port reads ahead on a thread ([[175]]), where the game
  streams from disc as it plays.
- [[52]]: the intro's end flash, drawn after stream 0, not over it, is
  placed 20 frames in by inference (docs/engine/title.md Unknown).
- [[85]]: the exact frame `ccEventStream`'s loop sees the stream end
  (docs/engine/stream.md Unknown).
- [[133]]: how many frames the game loads between str0580 and str0581
  (docs/engine/stream.md Unknown).
- [[133]]: str0581's frame-0 note (cue 900), given to the task's second pass
  (docs/engine/stream.md Unknown).
- [[269]]: whether `OBJ_xpart00`-`05`'s models carry a bounding box (the
  game would drop parts the port keeps).
- [[105]]: streams 37, 44, 67, 69 and 100 (a character's drain, by id) are
  not played (docs/engine/field-ui.md "Not yet known or not ported").
- [[240]]: which demo disc each `STRT.BIN` scene belonged to, and how its
  menus' selection worked; not in the files.
- [[249]]: whether the four discs' `LOGO_B.PSS` and `LOGO_C.PSS` are the
  same file; the build's dedup would say.
- [[52]]: the Audio screen's movies have no test of their own; [[56]] covers
  their music only.
- [[267]]: no harness compares Mutation's changed stream tasks with the
  game's code frame by frame.
- [[89]]: `--voice` is in no playthrough test.
- [[60]], [[185]]: the music is not compared with the game playing, only the
  tables and the call order; Dun Loireag's sequence 2 is not listened to.
- [[184]]: the canals' and the church's volumes against the game playing
  (docs/engine/sound.md Unknown).
- [[150]]: the travellers' hum against the game.
