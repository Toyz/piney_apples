---
number: 181
title: "One core for the four volumes: detection, the first generated tables, and a mail from Helba"
date: 2026-09-26
area: volumes
files: crates/piney-data/src/volume.rs, crates/piney-data/src/area/mod.rs, crates/piney-data/src/events/mod.rs, crates/piney-data/src/dungeon/mod.rs, crates/piney-data/src/statics/mod.rs, crates/piney-event/examples/gen_events.rs, crates/piney-event/src/extras.rs, crates/piney-desktop/src/extras.rs, tools/xfer.py, tools/area_tables.py, tools/rustgen.py, tools/dungeon_tables.py, plans/volumes.md
---

# 181. One core for the four volumes: detection, the first generated tables, and a mail from Helba

The aim, now in `plans/volumes.md`: one build boots any of the four discs.
At run time the port reads only the disc's data files (DATA.BIN, the PRG
overlays, the streams and the voices), never the boot executable. What it
takes from the executable is generated into piney-data, per volume. The
repository now also keeps the game's text (the scripts and messages),
which it did not before; it stays private.

**Which disc.** `piney_data::volume::Volume`, found from `DATA/GCMN.PRG`:
its length picks the volume and an FNV-1a hash confirms it. `Iso::volume`
reads it once. The game names a later volume's disc and stops, since only
Infection plays yet.

**Generated so far, all four volumes:**

- **The area words** (`area/{inf,mut_,out,qua}.rs`, `tools/area_tables.py`):
  the twelve word tables, `dungeonData`, `eventAreaInfo`, `volumeNum`, and
  from Mutation on the substitute records for areas 71 and 47. Quarantine
  renames slot b's word 100 from `Vengeful` to `Vindictive`.
  `AreaTables::of(volume)`; the check against the executable reads it at
  Infection's addresses independently of the generator.
- **The event scripts and messages** (`events/*.evs`, the IR's text form,
  by `examples/gen_events.rs`; 192 to 206 events a volume, 0.7 MB each).
  `official::events` parses them. `generated_scripts_are_the_executables`
  checks every event of every volume. Every disc carries every volume's
  events.
- **The dungeons and the towns' statics** (`dungeon`, `statics`, the
  existing generators). The statics are the same on every disc apart from
  their addresses; the dungeons differ as the Rust types already allowed
  (Mutation's plain fixed types, its random-path save flag, area 125's 15
  floors). Outbreak's hacked-dungeon fog values differ too.

**Finding the tables in the stripped executables.** Infection's names reach
the others through `tools/xfer.py`. It could not see four kinds of table,
so it gained these passes:

- *Order of the addresses built:* a body edited around a table.
- *Pointer:* a table only another table points at, e.g. `sqDataField` →
  `typeA`. A unanimous vote also corrected `data` names, 7 in Mutation
  and 55 in Outbreak (`typeAplayType` sat 0x20 off).
- *Content:* a table in recompiled code, its numbers equal and its
  pointers to the same strings (Outbreak's `EA_MODELTABLE`s).
- *Ref:* functions named by the tables and strings they build, with the
  names already known on both sides left out. This finds Outbreak's
  `EVENTAREA` and `ROOTTOWN` constructors, which give the statics' scene
  sets.

Other generator fixes:

- `SetClutList`'s count register is `$s2` in Outbreak and Quarantine.
- A local global's relocation names its section.
- The pointer pass keys names by section; overlays share one address
  window.

**Not done.** The field tables: Mutation's `DrawBG` adds `BgMatName2` for
hacked and crisis fields, which the port's `field::Tables` has no place for
yet. The sound tables: each volume has its own event voice tables
(Infection's `evVoiceDataVol1*` have no counterpart in Mutation). About 25
other readers still open the executable (the inventory is in the plan's
terms: the battle's, the desktop's, the field UI's, the world's).

**Helba's mail.** The port's own content, asked for: on a new game, after
CC Corporation's "Version Update", Helba writes "Old Worlds", a cold note on
the old emulated way of playing (plugins, speedhacks, black screens at the
Chaos Gate) and a press-F for the GS plugin. `piney_event::extras` patches
event 1 after the scripts load, so `official::events` stays the game's. The
text is in `piney_desktop::extras`, in mail slot 511, the save's last.
Shot in the mailer: her photo, the yellow highlights, every line within
34 characters. The desktop test now reads four mails.

**Still unknown:** Whether the `ref` and `content` passes name anything
wrongly was only checked on the tables they were built for; the statics of
all three later volumes equal Infection's row for row, which is the only
full check. How the later volumes pick their event voice tables was not
read.
