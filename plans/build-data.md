# The port's data in the build, not the repository

Status: steps 1 (the event scripts), 2 (every generator group's values, read from the discs) and 3 (the placement groups) done; step 4 (clean-up) is left. Written 2026-09-27.

Everything the port takes from the discs moves out of the repository and
into what `piney-build` makes from the player's own discs. The repository
keeps code: the engine, the generators and readers, the type definitions,
the format docs, and facts about the binaries (addresses, struct layouts,
disc hashes). It keeps no game content: no scripts, text, tables of values
or placement data.

This replaces "generated into piney-data" from `plans/volumes.md`. The rule
that the port never reads the boot executable at play time stands: the
build reads it, once, and play reads what the build wrote.

## What moves

| now | size | made by |
| --- | --- | --- |
| `piney-data/src/events/*.evs` | 2.6 MB | `piney-event` `gen_events` |
| `piney-data/src/tables/*.rs` values | 12 MB | `piney-gen` |
| `piney-data/src/{dungeon,area,field,sound,statics}` | 3 MB | `tools/*_tables.py`, `tools/statics.py` (all moved; the tools deleted) |
| `piney-audio/src/setbl.rs` | 32 KB | generated |

What stays in code: the constants a reimplemented function uses as part of
its logic (a 5-entry offset table, a distance, a colour), written where the
function is and documented with their address. Bulk tables, text and
placement move; a constant that reads as code stays code.

## Where it goes

Each volume's data is a set of files under `PINEY/`, in that volume's disc
of the build (its `.disc` manifest). They are chunked into `chunks.pak`
like the disc's own files, so what the four volumes share is kept once,
and read through `Iso::read_path` as any disc file is.

- `PINEY/VERSION`: `piney_data::pack::DATA_VERSION`. The game refuses (or
  remakes) data of another version; it is bumped whenever a producer's
  output changes.
- `PINEY/EVENTS.EVS`: the event scripts and messages in their text form.
- `PINEY/TABLES/<group>.bin`: each generator group's values in the
  generated types' own shape (`piney_data::store::Load`: numbers
  little-endian, text and slices counted, `Option` tagged, structs field
  by field), read once a run through `tables::<group>::of(volume)`; the
  entries alike on every volume keep their statics, read on first use.
- `PINEY/TABLES/{area,dungeon,field,sound,statics,setbl}.bin`: the
  placement groups, in `piney-data`'s hand-written types, each with its
  own `Load`; where the game's tables share an array (object rows, room
  tables, fog tables, story dungeons' rows) it is written once and named
  by index. `setbl` exists only for Infection and Mutation (Outbreak and
  Quarantine have no `spc0SeData`); the port reads Infection's.

A single disc image (no build) gets the same files once, in the port's
folder (`data/<image>-<size>/`), which `Iso` serves as `PINEY/...` for that
image. The game makes them at start, before it shuts off reads of the
executable, if they are missing or of another version.

Tests and tools open raw images with no port data and are not under
`deny_executable`: they extract in-process (the readers fall back to the
executable), so `work/*.iso` is still all a checkout needs.

## Overrides (later)

A folder in the port's home laid out like `PINEY/`, whose files win over
the build's: a translation, a fix to a line, a changed table. Text gets
stable ids so an override can name one line rather than replace a file.

## Steps

1. **The store and the events.** `pack::DATA_VERSION`, `Iso`'s image data
   folder, `piney-build` as a library (`port_files`, the image data), the
   build writing `PINEY/`, `official::events` reading `PINEY/EVENTS.EVS`
   (or the executable, for tools), the game making an image's data at
   start. `piney-data/src/events` removed.
2. **The tables.** Done: `piney-gen` is a library too; every group's
   values are written as data (`piney_gen::data`) beside its types and
   accessors, the build adds them, `piney_data::store` reads them (the
   tools' and checks' copy in `work/data/<volume>/TABLES/`, which
   `piney-gen gen` writes). 12 MB of Rust became 388 KB of types.
   The generator reads the discs themselves (`piney_gen::source`, named
   by `piney_build::use_disc`): each volume's executable, overlays and
   `DATA.BIN` from its own disc, and the later volumes' carried names and
   carry made in memory from it and Infection's disc (about 17 s for
   Mutation, 35 s each for Outbreak and Quarantine), the tables' layouts
   from Infection's DWARF on its disc. Nothing new is committed: a build
   of a later volume needs Infection's disc beside it (the owner's
   choice, over committing the names, carry and layouts as facts). The
   tools, with no disc named, still read `work/`.
3. **The placement groups.** Done: `tools/area_tables.py`, `statics.py`,
   `sound_tables.py`, `field_tables.py` and `dungeon_tables.py` ported
   into `piney_gen::placement` and deleted, with setbl.cpp's rows; the
   code they ran in `tools/eemu.py` (the field mesh tables, the dungeons'
   texType and clutType) runs in `piney-eemu`. Each group's Debug dump
   was identical on all four volumes to the compiled-in values it
   replaced.
4. **Clean-up.** README and `cairns.toml`'s claim made true; the history
   rewritten to drop the committed data (never pushed; back up first).
