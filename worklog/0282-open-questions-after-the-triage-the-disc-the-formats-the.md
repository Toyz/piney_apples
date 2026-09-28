---
number: 282
title: Open questions after the triage: the disc, the formats, the engine core and the tools
date: 2026-09-28
area: disc, format, decomp, engine, tooling, build
files: UNKNOWNS.md
resolves: 2, 4, 5, 6, 10, 15, 62, 63, 82, 181, 198, 204, 211, 212, 213, 219, 232, 233, 241, 242, 247, 255, 256, 257, 258, 259, 262, 271, 276
---

# 282. Open questions after the triage: the disc, the formats, the engine core and the tools

This entry closes the open questions of 29 entries about the disc, the file
formats, the executable, the engine's modes and tasks, the pad, the build
and the project's own tools. The triage of 2026-09-28 (UNKNOWNS.md) split
every entry's open paragraph into single questions and checked each one
against the later log, the docs and the code. The answered ones are listed
below with their evidence. What is still open is restated at the end, with
the entry that asked it, the play questions first. Open questions of these
entries that belong to another subsystem are restated in that subsystem's
entry: [[286]], [[288]], [[289]] and [[291]].

## Answered

- [[2]]: relocation type 123 — answered by [[243]] (the micro address >> 3
  in the 15-bit immediate) / docs/engine/executable.md
- [[2]]: what the `mc_*` symbols resolve to — answered by [[243]] (VU entry
  points, 21 referenced)
- [[2]]: how much of the DWARF survives — answered by [[6]] (all of it;
  `tools/dwarf1.py`)
- [[2]]: what the `MWo3` header's overlay id is used for — answered by
  [[242]] (nothing reads it) / docs/formats/prg.md
- [[4]]: the VU1 microcode is not disassembled — answered by [[17]]
  (`tools/vu.py`, `tools/test_vu.py`)
- [[5]]: what fills `directCCSTbl` and which files use category 19 —
  answered by [[242]] (`ccAddFileList`; no volume adds one) /
  docs/formats/data-bin.md "Category 19"
- [[6]]: the contents of the `.mwcats` sections — answered by [[243]]
  (per-function size records) / docs/engine/executable.md
- [[10]]: the rigid ITOF12, the ST scale and culling inferred from the EE
  side — answered by [[17]] (the microcode settles them) and [[244]] (ST
  scale)
- [[10]]: the 15 extra words per mmat in the size of `mtype & 4` models —
  answered by [[244]] (an overstated size nothing reads) /
  docs/formats/ccs-model.md
- [[10]]: the model flag bits above the blend type, and `zoffs` — answered
  by [[244]] (bit 3 dropped; `zoffs` never read)
- [[10]]: Anime sub-kinds other than 0x0102 — answered by [[32]], [[244]]
  (all eleven kinds) / docs/formats/ccs.md "Anime sub-kinds"
- [[10]]: the texture and CLUT flag bits and the CLUT's `unknown_1` —
  answered by [[244]] (`unknown_1` is `cpsm`) / docs/formats/ccs.md "Texture
  and CLUT flags"
- [[15]]: `ccEvent::Execute`, the event script interpreter — answered by
  [[18]], [[40]] / docs/engine/event-vm.md
- [[82]]: `WORLD_MAN::SetEventData` — answered by [[87]]
- [[82]]: Mac Anu's gate-address window empty — answered by [[90]]
  (`FieldUi::announce_lines`)
- [[82]]: streams 4-6's subtitles — answered by [[85]]
- [[82]]: event 10's sound 8 — answered by [[176]] (the sound instruction's
  eleven cases)
- [[181]]: how the later volumes pick their event voice tables — answered by
  [[226]] (each disc's own `ccEvVoiceRequest`, decoded by groups)
- [[204]]: OUT's and QUA's `ccParticle::Setup` switch — answered by [[205]]
  (decoded by register)
- [[211]]: whether the carry's Rust port gives the same cache byte for byte
  — answered by [[212]] (`piney-gen carry --check`)
- [[211]]: the six trade and shop failures — answered by [[214]]
- [[212]]: whether the symbol transfer (`xfer.py transfer`, `.syms`) ports
  exactly — answered by [[213]]
- [[213]]: the five older Python generators remain — answered by [[256]],
  [[257]], [[258]] (every table group in the build; the Python generators
  are gone from tools/)
- [[219]]: Carmina Gade's `DrawMap` and Mutation's changed town maps —
  answered by [[225]]
- [[256]]: how many of the 337 shared statics' users change when their
  groups move — answered by [[257]] (every table group moved into the build)
- [[257]]: the generator still reads its inputs from `work/` — answered by
  [[259]] (the generator reads the discs)
- [[258]]: the generator reads the executables, overlays, symbols and DWARF
  from `work/` — answered by [[259]]

**Still unknown:**
- (play) [[259]]: a later volume's image played without Infection's disc
  falls back to `work/`, and `crate::die` ends the process — a player who
  owns only a later disc.
- [[4]]: the EE encodings the game never uses (the debug and
  performance-counter moves, `rsqrt.s`, `max.s`/`min.s`, `vrnext`/`vrget`,
  the `pmfhl` variants) are decoded from the manuals only.
- [[5]]: whether any requested name can miss its category table (the walk
  would pass the nameless terminator); the game relies on never asking.
- [[5]]: the two `s16` fields (`+0x24`, `+0x26`) of a `sceneFileList` entry.
- [[6]]: the four-byte member missing from `ccChunkIndex`, and the 18 other
  classes' unexplained gaps.
- [[242]]: what category 19 (a loose scene file outside the archive) was
  for; nothing in the code says.
- [[15]]: what modes 1 and 0x1000 do (docs/engine/overview.md Unknown).
- [[15]]: the full list of tasks each mode starts and their priorities; the
  port orders each ported mode's tasks itself (docs/engine/overview.md
  Unknown).
- [[15]]: `ccSystem::Ctrl`'s other branches (screen-mode changes, the flags
  at `+0xbd4`/`+0xbd5`); only the vibration countdown was read ([[232]]).
- [[69]]: the load before F0 (`ccLoadResourceFL`) takes no frames in the
  port (docs/engine/event-vm.md Unknown).
- [[82]]: the load time between the mode change and the desktop's set-up
  (docs/engine/event-vm.md Unknown).
- [[62]]: a DualShock 2's real values at full diagonals: recalled, not
  measured.
- [[62]]: half-way diagonals read slightly stronger, so the analogue walk
  may feel different; not measured.
- [[232]]: the rumble has not been felt on a real pad (gilrs needs the pad's
  force-feedback node).
- [[232]]: the pad's connection states (`ccPad::Ctrl`'s other states,
  `scePadInfoAct`) are not modelled; a connected pad is a ready DualShock.
- [[63]]: piney-eemu: subclass overrides of load, store and set do not reach
  the interpreter; exec overrides run slowly.
- [[63]]: piney-eemu's RAM cannot grow past 32 MB; its registers are live
  views.
- [[63]]: iopemu's machine stays on `eemu.py`.
- [[82]]: a replayed story start lacks the earlier experience and items; its
  mails and posts are as `ccEventFlagSet` leaves them.
- [[82]]: `story:12` starts on the desktop where the game's first block
  would run in the top page's set-up.
- [[148]]: Kite starts event 29's dungeon session as a ghost (dead 4);
  possibly the story start's doing.
- [[149]]: a mid-dungeon story start leaves the room undrawn in its picture.
- [[86]]: the unported-default counts say which defaults ran, not how often.
- [[176]]: the host-call check plays blocks in isolation; a method reached
  only through earlier state may not show.
- [[125]]: `stream_shot` has no party members (the field's `StreamMenu`
  draws them).
- [[161]]: decoding sd9 a second time after sd4 makes the interpreter's
  `DecodeSetup` read a bad pointer (test order; docs/engine/dungeon.md
  Unknown).
- [[181]]: whether xfer's ref and content passes misname anything beyond the
  tables they were built for.
- [[182]]: a reader that swallows its error (`.ok()?`) would not show in
  smoke runs.
- [[198]]: voldiff's "recompiled" rule is a heuristic: a changed field or a
  swapped comparison passes.
- [[198]]: some OUT and QUA "changed" menus call nothing Infection's do,
  which points at misplaced carried names; to check in `xfer.py`.
- [[202]], [[204]], [[206]]: finders for the remaining carry misses:
  Mutation 5, Outbreak 23, Quarantine 28 after [[206]].
- [[204]]: readers that take an Infection constant through a shape the scan
  does not see; not surveyed.
- [[219]]: the other harnesses still name some Infection addresses of their
  own, moved to `va()` as each runs on a later volume.
- [[271]]: why `piney-gen syms` did not carry `Func_str7100`, `Func_str8800`
  and `Func_str0300` to Mutation; a guess: jump-table relocations.
- [[233]]: the game's log lines (stderr) do not reach the in-game console,
  and the console has no horizontal scroll.
- [[241]]: nothing records from the window, and there is no key to record
  live play.
- [[247]]: the window's switch from the launcher to a part is checked
  headless only.
- [[255]]: how long an image's first start takes on a slow drive.
- [[258]]: the default build at `~/.local/share/piney/game`, at data version
  3, has to be remade.
- [[259]]: whether `root()` (the repository path, compiled in) is reached by
  a build run on another machine.
- [[262]]: the dialogue scan misses lines under four words, and text inside
  the games' textures.
- [[262]]: the history's blobs are not scanned; the history will not be
  published as it is (plans/release.md).
- [[263]]: how the pilot should find a Data Bug's protect break for Data
  Drain.
- [[276]]: how the autopilot's run compares in time with a player's.
