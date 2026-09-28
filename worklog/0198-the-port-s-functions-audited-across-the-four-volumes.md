---
number: 198
title: "The port's functions audited across the four volumes: voldiff.py ported"
date: 2026-09-26
area: volumes
files: tools/voldiff.py, plans/volumes.md, GAPS.md
---

# 198. The port's functions audited across the four volumes: voldiff.py ported

Infection's own gaps are closed as far as its code goes. The rerun of
0186's audit (the functions neither a crate nor a page names) leaves
three groups:

- Mutation-on code: `BreakMirror` is `ccBoss02Slave`'s, `HellSpecialExpress`,
  the bosses' effects, `ROOTTOWN03`, `EVENTAREAB8`;
- what GAPS already lists: the Ryu Books, `ccEnemyL` type 3, the type-32
  rooms' leaves;
- ported functions the pages name another way, such as
  `ccAddBattleAbility` in the level-up or `FIELD::GetColor` in the meshes.

So the work moves to plans/volumes.md's phase 3: which of the ported
functions the later volumes changed.

## The audit

`tools/voldiff.py ported` harvests every function the port names:

- the docs' `covers`;
- the crates' citations (`gcmn 0x0059ddd0`, `main 0x...`) that start a
  function.

That is 2,138 functions. For each, it finds the function in MUT, OUT and
QUA by the name `xfer.py` carried, and sorts it:

- **same:** the body, addresses masked (`xfer.normalise`), is
  Infection's;
- **recompiled:** the opcode sequence is, or the callees in order and the
  constants are;
- **changed** or **missing**.

The first pass counted only exact opcode sequences. Outbreak and
Quarantine then came out 1,097 and 1,085 "changed", and a diff of the
near misses showed what their compiler does:

- the stack laid out anew;
- delay slots filled differently;
- `sqrtf`, `fptosi` and `fptoui` inlined;
- zero extension by a load instead of `andi 0xff` or `0xffff`;
- constants loaded and `and`ed instead of `andi`;
- structure fields moved (a `ccChar` field read at +172 is at +180 in
  `ccAI::FollowTargetDirc`).

So "recompiled" also takes:

- the same callees in order, with the inlined helpers left out and a
  callee the carry left unnamed matching any;
- the same constants: the immediates of `slti`, `sltiu`, `andi`, `ori`
  and `xori`, and of an `addiu` or `daddiu` from `$zero`, whatever the
  instruction.

An `addiu` from another register is left out, since it adds a field
offset as often as a number. So are the zero-extension masks.

| volume | same | recompiled | changed | missing |
| --- | ---: | ---: | ---: | ---: |
| MUT | 1,539 | 292 | 296 | 11 |
| OUT | 218 | 1,298 | 561 | 61 |
| QUA | 218 | 1,275 | 565 | 80 |

Mutation's least alike are what the pages expected:

- the save's accessors (`ccSaveData::ChangeEquipment`, `GetItemNum`,
  the trade lists), for 21 characters and the save's extension;
- `ccGetCharParam`, `ccSkillCheck` and the party AI's healing;
- `ccVoiceRequest`, the sound's known change;
- `DUNGEON::MakeRealMap` and the event area lookups;
- `ccEvent::Execute`, for the new instructions.

The missing ones are stream functions Mutation dropped
(`Func_str0001`, `str0300`, ...), `ccPlayerStart`, `ccSpcSleep`, the trade
counters and the top page's file list.

**Still unknown:** The "recompiled" rule is a heuristic. A change that
keeps every callee and constant, such as a different field or a swapped
comparison, passes as recompiled. Some of Outbreak's and Quarantine's
"changed" menus call nothing Infection's do, which points at names the
carry put on the wrong function. That needs checking in `xfer.py` before
those functions are read. None of the 296, 561 and 565 changed functions
has been read yet.
