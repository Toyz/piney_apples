---
number: 211
title: The table generator in Rust
date: 2026-09-27
area: volumes
files: crates/piney-gen, crates/piney-data/src/tables, crates/piney-data/src/lib.rs, plans/volumes.md, README.md
---

# 211. The table generator in Rust

The generator from 0208 was a Python tool (`tools/static.py`) that wrote
Rust. Its manifest was Python and its layouts were Python classes that
printed Rust. That is Rust written from Python. It is now a Rust tool,
`piney-gen`. The port never runs it.

## What moved

| Python | Rust (`crates/piney-gen/src`) |
| --- | --- |
| `static.py`: the layouts, the emitter, finding an entry, rendering | `layout.rs`, `locate.rs`, `render.rs` |
| `static_manifest.py` | `manifest.rs` |
| `Type(...)` from Infection's DWARF | `dtype.rs` on its own DWARF 1 reader, `dwarf.rs` |
| `image.py`'s program image, the `.syms` sidecar | `elf.rs`, `program.rs` |
| `sinit_tables.py`: `saveSysMsg`, the mail tables' replies | `sinit.rs`, run in `piney-eemu` |
| `talk_index.py`: the talk records by address | `talk.rs` |
| the Shift-JIS text and its literals | `text.rs` |

The manifest is typed Rust: `e("skills", 0x0061_F4A0, array(skill(), 304),
GCMN, "...")`. Its custom finders (`voice_groups`, `item_icons`, the mails,
`by_volume`) are plain functions. The Python manifest's entries were
carried over mechanically, docs and all, so nothing was retyped.

## Parity

The first full run of `piney-gen gen --check` matched every committed
file byte for byte: all 16 groups, `mod.rs`, `types.rs` and `sjis.rs`. The
only change since is the header naming `piney-gen`. A run takes 9 seconds;
the Python took about 8 minutes.

`piney-data`'s `sinit.rs`, the older output of `sinit_tables.py`, had no
readers left once the tables took over (0208, 0209). It is gone.

## The talk index

The `talk` group (committed just before this entry) holds every talk
record array and pointer table reachable from the roots the game keeps:
`npcTbl`'s `base.msg`, `spcMsgTbl`, the present tables, the breeder's and
the Grunty's records and `errorData`. Each is listed with its address in
its own volume, since the save stores those pointers. Infection's DWARF
gives each object's kind and count; the later volumes' come through the
carry, and an object nothing declares is told by its shape. The port does
not read the index yet.

## Still in Python

`tools/carry.py` and the parts of `tools/xfer.py` it uses make the carry
cache, `work/analysis/carry/*.json`, which `piney-gen` reads. The symbol
transfer that writes the `.syms` sidecars is Python too. Both move next.

Five older generators also write Rust from Python: `statics.py`,
`area_tables.py`, `dungeon_tables.py`, `field_tables.py` and
`sound_tables.py`. They were built for Infection before the per-volume
tables existed. The world's rebuild will need them per volume, and they go
into `piney-gen` then.

**Still unknown:** Whether the carry's port gives the same cache byte for
byte, and so the same tables. Python's `difflib` matching, which
`xfer.aligned` uses, has to be reproduced exactly, or its differences
shown harmless by `piney-gen gen --check`. The six trade and shop harness
failures from 0210 remain until the talk pages read the `talk` group.
