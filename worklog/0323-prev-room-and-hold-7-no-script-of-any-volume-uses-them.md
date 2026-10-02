---
number: 323
title: "prev_room and hold 7: no script of any volume uses them"
date: 2026-10-01
area: script, volumes
files: crates/piney-event/src/host.rs
---

# 323. prev_room and hold 7: no script of any volume uses them

Two event instructions have stayed unported since Infection's story was done:
- `prev_room` (`WORLD_MAN::GoPrevRoom`, INF main 0x001a40b0), which [[87]]
  and [[111]] left at the host's default, "no volume-1 script needs it";
- `hold` with type 7 (the boss), which [[109]] and [[117]] left out, "no
  Infection script uses it".

Both notes were Infection's alone. Every script of all four volumes was
dumped with `tools/evscript.py dump` (main, parody and side tables):
- **`prev_room`:** no script of Infection, Mutation, Outbreak or Quarantine
  has it.
- **`hold` (op 0x84):**

  | type | INF | MUT | OUT | QUA |
  | --- | ---: | ---: | ---: | ---: |
  | 5 | 25 | 25 | 27 | 27 |
  | 6 | 10 | 10 | 10 | 10 |
  | 32 | 1 | 1 | 1 | 1 |

  Type 7 never.

So neither is reached by the games' own scripts. They stay as they are: `prev_room` reported as unported if a
script ever calls it, and `hold 7` falling to the host's default. The
ledger (UNKNOWNS.md) lists both as dead.

**Still unknown:** nothing about these two. A script loaded from somewhere
other than the four executables (none is known) could still reach them.
