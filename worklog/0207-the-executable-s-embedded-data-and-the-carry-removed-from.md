---
number: 207
title: "The executable's embedded data and the carry removed from the port"
date: 2026-09-26
area: volumes
files: crates/piney-data/src/exe.rs, crates/piney-data/src/main_data, tools/carry.py, tools/sinit_tables.py, plans/volumes.md, GAPS.md
---

# 207. The executable's embedded data and the carry removed from the port

Phase 1 (worklog 182) had taken Infection off the executable the short
way. `tools/main_data.py` copied each volume's main section (from the font
`ef12x20` to the end of main) and three function bodies into piney-data,
550 KB a volume in a format of its own. `Image::main(volume)` stood where
the executable had.

From worklog 199 on, a *carry* moved each of Infection's addresses into
the later volumes (`Image::at`), also in a format of its own. The readers
kept walking executable memory by Infection's addresses.

The owner's rule is that the port never reads the executable at run time.
This kept the letter of it (the file was not opened) but not the intent.
It ran the later volumes as Infection with its addresses moved, and
worklogs 199-206 had spent their effort widening it. The owner also asked
for generated Rust only, with no data files of the port's own format.

Removed:

- `Image::main`, `Image::at`, `Image::carried` and the image's volume;
- `crates/piney-data/src/main_data` (the four `.bin` files and three
  `.carry` files);
- all 367 `at` calls in the readers, which now name bare addresses.

A caller that built the main image builds an empty one, so its reads
fail. The workspace builds and clippy is clean.

The carry's passes stay in the tools as `tools/carry.py`, a finding aid:
it caches its rows under `work/analysis/carry/` for a generator to find a
volume's globals, and nothing of it reaches the port.
`tools/sinit_tables.py` uses it that way for the mail links.

## What now fails

Every system that read the main section, on Infection too. Of the
workspace's tests, 440 pass and 219 fail:

- the desktop's, the demo's, the top page's and the field UI's suites;
- 109 of the session's play tests;
- the effects, the streams and their subtitles;
- the world's areas.

The list is saved as the rebuild's checklist.

## The rebuild

`plans/volumes.md` ("Where the data lives", "The generator", "The
shortcut removed") has the plan:

- Each system's data is generated as typed Rust tables per volume, from
  each volume's own executable.
- A table two or more volumes hold alike is written once (`shared.rs`).
- Readers take the tables' rows, not addresses.

The order follows the boot: the title and the save, the desktop and the
top page, then the field UI, the world, the battle, the effects and the
streams.

**Still unknown:** Nothing new. The rebuild restores the systems one at a
time, each checked by the tests that fail now.
