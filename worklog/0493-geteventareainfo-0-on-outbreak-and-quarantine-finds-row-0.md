---
number: 493
title: GetEventAreaInfo(0) on Outbreak and Quarantine finds row 0 as the port does: the carried names of its two overloads were crossed
date: 2026-10-25
area: tooling
files: crates/piney-gen/src/syms.rs, crates/piney-data/examples/area_probe.rs, tools/test_area_rs.py, tools/test_battle_flow_rs.py, tools/volume.py, docs/disc/volumes.md, docs/engine/area-words.md
resolves: 291.11
started: 2026-10-25T20:01:05-06:00
took: 19m
---

# 493. GetEventAreaInfo(0) on Outbreak and Quarantine finds row 0 as the port does: the carried names of its two overloads were crossed

Volume question [[220]] (291.11): on Outbreak and Quarantine
`GetEventAreaInfo(0)` was said to return NULL where the port finds row 0.

**The cause: crossed names, not the game.** The area harness
(tools/test_area_rs.py) calls the game's `GetEventAreaInfo__9WORLD_MANFi`
by name. On the stripped volumes that name comes from the sidecar `piney-gen
syms` carries, and on Outbreak and Quarantine it was on the wrong one of
the two overloads:
- OUT main 0x001a7ce0 (QUA 0x001af4f0), filed as `(int)` by the ref pass.
  It opens with `lw $a1, 0x120($a0)`: it is `GetEventAreaInfo()`, which
  searches for `eventAreaNumber`.
- OUT main 0x001a7dc0 (QUA 0x001af5d0), filed as `()` by the order pass.
  It compares `$a1` as passed: it is `(int)`.
- The callers agree. `ccEvent::Execute` and `CheckAreaCode` call
  0x001a7dc0 with a code. `AreaItem`, `ccHackMenu` and
  `ccRegisterDifficultyEnemy` call 0x001a7ce0 with none, as Infection's
  and Mutation's call `()`.
The protect loop runs first and leaves `eventAreaNumber` at 130, so "(int)
of 0" searched for 130 and found nothing. The game's real `(int)` finds
row 0, as the port does.

**The fix (piney-gen syms, a last pass `overload`).** It looks for two
functions of a section whose names agree up to their arguments, one of
them taking none (`...Fv`). The body that reads its first argument's
register (`$a1` for a member, `$a0` else) before writing it takes the
other name. `gpr_use` and `live_in` read the registers straight down.
The pass renames exactly this pair on Outbreak and Quarantine and nothing
on Mutation. A first try that compared code shapes swapped Mutation's
correct pair, so it was dropped.

**The harness, now on all four volumes.**
- With the names right, two more later-volume cases showed. Both are the
  port's behaviour, already in the docs:
  - area 47's substitute record from volume 3 on: `info` now answers
    `{"sub": code}` for a substitute, read in the game by the record's
    code;
  - Quarantine's instruction 118 with 126 (QUA main 0x001c3204) sets
    `eventAreaNumber` alone.
- tools/test_area_rs.py passes on Infection, Mutation, Outbreak and
  Quarantine (6 tests each; `GetEventAreaInfo` and `IsProtectArea` for
  -1 to 130 with 0 mismatches).
- tools/test_battle_flow_rs.py hooked the callee through `volume.called`,
  a workaround for this swap. It now hooks `GetEventAreaInfo__9WORLD_MANFv`
  by name, and `called` is gone. It passes on Infection, Outbreak and
  Quarantine.
- test_gate_out_rs and test_desktop_announce_rs fail on Outbreak with
  the old sidecar as with the new one, and on Mutation too, whose sidecar
  did not change. They are Infection harnesses.

`piney-gen syms --check` is current on all three volumes, and `gen
--check` is unchanged. piney-gen has new unit tests (`overload_base`, and
`live_in` on Outbreak's two openings).

Docs: docs/disc/volumes.md (the `overload` row and paragraph),
docs/engine/area-words.md, UNKNOWNS.md.

**Still unknown:** nothing
