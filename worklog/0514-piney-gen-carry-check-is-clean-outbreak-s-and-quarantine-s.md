---
number: 514
title: piney-gen carry --check is clean: Outbreak's and Quarantine's carry caches made again, with GetEventAreaInfo's overloads uncrossed as worklog 493 left them
date: 2026-10-29
area: tooling, volumes
files: work/analysis/carry/out.json, work/analysis/carry/qua.json
started: 2026-10-29T21:24:28-06:00
took: 2m
---

# 514. piney-gen carry --check is clean: Outbreak's and Quarantine's carry caches made again, with GetEventAreaInfo's overloads uncrossed as worklog 493 left them

`piney-gen carry --check` said `work/analysis/carry/out.json` and
`qua.json` differ from what the carry makes, and it said the same at
HEAD (worklog 510). The caches date from 27 September.

**What differed.** Each volume had exactly two rows out of about
25,900, swapped between them: Infection's `WORLD_MAN::GetEventAreaInfo`
overloads, `()` at 0x0019d350 and `(int)` at 0x0019d3b0, 96 bytes
each.

| volume | `()` old | `()` new | `(int)` old | `(int)` new |
| --- | --- | --- | --- | --- |
| OUT | 0x001a7dc0 | 0x001a7ce0 | 0x001a7ce0 | 0x001a7dc0 |
| QUA | 0x001af5d0 | 0x001af4f0 | 0x001af4f0 | 0x001af5d0 |

The new rows agree with the carried names in the volumes' `.syms`
(OUT `GetEventAreaInfo__9WORLD_MANFv` at 0x001a7ce0, `...Fi` at
0x001a7dc0). Worklog 493
(7a5402a) uncrossed these two overloads' carried names after the
caches were written; the caches still crossed them. The game's build
never used these caches: with discs named, piney-build makes the carry
in memory. Only the tools read them (`tools/volume.py`'s `va()` and
piney-gen without a disc).

**The fix.** I made both caches again with `piney-gen carry --volume
OUT` and `--volume QUA`. They live under `work/`, which is not tracked,
so this commit holds only the entry. The old caches are kept in the
scratch area.

**Checks.**
- `piney-gen carry --check`: all three caches current.
- `piney-gen gen --check`: every table and every built group's bytes
  unchanged, all four volumes.
- `test_area_rs.py`, which runs `GetEventAreaInfo(int)`, passes on
  Outbreak and Quarantine.

No code changed, so piney-game's suite was not run again for this
entry.

**Still unknown:** nothing
