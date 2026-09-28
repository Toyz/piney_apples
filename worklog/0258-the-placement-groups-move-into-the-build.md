---
number: 258
title: The placement groups move into the build
date: 2026-09-27
area: build, tooling
files: crates/piney-gen/src/placement, crates/piney-data/src/area/mod.rs, crates/piney-data/src/statics/mod.rs, crates/piney-data/src/sound/mod.rs, crates/piney-data/src/field/mod.rs, crates/piney-data/src/dungeon/mod.rs, crates/piney-audio/src/setbl.rs, tools/portdata.py, plans/build-data.md
---

# 258. The placement groups move into the build

Step 3 of `plans/build-data.md` is done. The last tables whose values were
in the repository now come from the build:
- the area generator's words and event areas;
- the towns' statics;
- the sound tables and setbl.cpp's rows;
- the fields;
- the dungeons.

Before, the Python generators wrote these as Rust into `piney-data`, one
file per volume, as 2.68 MB of generated Rust:

| module | generated Rust |
| --- | --- |
| `dungeon` | 1,810,673 bytes |
| `sound` | 356,162 |
| `field` | 209,126 |
| `area` | 166,304 |
| `statics` | 134,920 |
| `piney-audio`'s `setbl.rs` | 29,764 |

Now each is a file per volume, `PINEY/TABLES/<group>.bin`, 385-398 KB per
volume in all. The largest is `dungeon.bin`: 289,416 bytes for INF,
301,966 for MUT and 301,958 for OUT and QUA. `pack::DATA_VERSION` is 8.

## The Rust generators

The five generators are now Rust, in `piney_gen::placement`: `area`,
`statics`, `sound`, `field` and `dungeon`. Each is a function from a
volume to its file's bytes, listed in `placement::GROUPS`. `piney-gen gen`
writes their output to `work/data/<volume>/TABLES/`, and `piney-build`'s
`port_files` adds it to each disc. `--group <name>` runs one of them;
`field` and `dungeon` take under 0.3 s each for all four volumes.

These Python tools were deleted, per the owner's rule that a Python tool
goes once Rust replaces it:
- `tools/area_tables.py`, `statics.py` (with `test_statics.py`),
  `sound_tables.py`, `field_tables.py` and `dungeon_tables.py`;
- the `test_generated_file_is_current` checks in `test_field_rs.py` and
  `test_dungeon_rs.py`.

The ports read the code exactly as the Python did. They find the same
things:
- the `li`/branch pairs;
- the registers that move between compilers (`this` in `$s0` or `$s2`, the
  `object` percentage in `$f20` or `$f1`, the hill counters);
- the jump tables on the dungeon type;
- the fog blocks of the `DUNGEON` constructor.

Two parts of the Python ran game code in `tools/eemu.py`. Both now run in
`piney-eemu`, with the machine `sinit.rs` already set up (every loaded
segment, the overlay and the C library stand-ins):
- `FIELD_MESH::SetMESH2` and `FIELD::SetSmallMESH` (INF gcmn.prg:0x005af780,
  0x005ade80), over a FIELD whose every height and colour names its own
  cell;
- `WORLD_MAN::SetDungeonTexClut` for servers 0-4 and field types 0-11.

Some values are models or measurements, not reads. The Python generators
held them as constants, and the Rust ones carry the same constants:
- `WORLD::Init`'s draw counts;
- `tools/dungeon.py`'s model of `SetDungeonTypeFromField`;
- the expected `WORLD::Generate` constants, which the generator checks
  the code against.

`tools/test_field.py` and `tools/test_dungeon.py` still check those
models against the game.

## The file shapes

Each module's types stay hand-written. Each gets a `Load` impl in field
order, in the store's format from [[256]]. Where the game's tables share an
array, the file holds it once and the users name it by index:
- the fields' `FOBJECT_INFO_TABLE` rows (a lake field's `LakeObjTABLE`
  entry is its key rows);
- the dungeons' `ROOM_INFO` tables;
- the dungeons' `dungeonFog*` tables;
- the story dungeons' `ROOMDATA` and `GIMMICKDATA` arrays.

`tables_of(volume)` keeps its reference in a `OnceLock` per volume.
`INF` became a `LazyLock<&'static Tables>`. Every user of `field::INF`,
`dungeon::INF` and `sound::INF` compiled unchanged through deref coercion,
except one in `se3d.rs`, which took a `*`.

`setbl` is made only for Infection and Mutation, since Outbreak and
Quarantine have no `spc0SeData`. A group maker returns `None` for a volume
without the table, and the build skips it. `piney-audio` reads
Infection's rows, as setbl.cpp's port always did. So a build without
Infection has no setbl for the port to read. Nothing else in the port
depends on which discs are in a build.

## Python that read the generated Rust

`tools/iopemu.py`'s `wave_bank` and `tools/test_sound3d_rs.py`'s `Setbl`
had parsed the generated Rust. They now read the port's files through
`tools/portdata.py`: `sound(volume)` and `setbl(volume)` decode
`work/data/<volume>/TABLES/sound.bin` and `setbl.bin`, the same format
the Rust `Load` impls read. `tools/sound_ee.py`'s `FIELD_VOICE`, which it
had imported from `sound_tables.py`, is now inline.

## Checked

Each move was checked the same way. A `dump_<group>` example prints
`{:#?}` of every volume's tables. It ran in a worktree at 70f4632, which
still has the compiled-in values, and again after the move. The two
outputs were compared byte for byte:

| group | lines of Debug, all four volumes |
| --- | --- |
| `dungeon` | 402,743 |
| `field` | 25,984 |

`area`, `statics`, `sound` and `setbl` were also identical in their
commits (e39f305, 20cf8fa, 20d36af).

The whole workspace's tests pass (694), reading the tables from
`work/data`, and clippy is clean.

**Still unknown:** The generator still reads its inputs (the executables,
the overlays, the carried symbols, Infection's DWARF) from `work/`, so a
build needs a machine with the discs extracted. Committing the symbols and
layouts as facts, and reading the executable from the disc, is the
remainder of step 2. The default build at `~/.local/share/piney/game` is
at data version 3 and has to be remade.
