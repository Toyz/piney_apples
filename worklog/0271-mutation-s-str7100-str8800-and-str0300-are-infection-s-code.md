---
number: 271
title: Mutation's str7100, str8800 and str0300 are Infection's code
date: 2026-09-28
area: video, volumes, decomp
files: docs/engine/stream.md
supersedes: 267
---

# 271. Mutation's str7100, str8800 and str0300 are Infection's code

[[267]] left `str7100`, `str8800` and `str0300` as "changed on Mutation"
because no Infection name reached their rows in `StreamDemoFuncTbl`. They
have not changed. The port's tasks for them, taken from Infection, are
Mutation's too.

Their rows in the table (MUT main 0x00367210: rows 0, 2 and 11) point to:

| scene | INF main | MUT main | instructions |
| --- | --- | --- | --- |
| `str7100` | 0x00187ad0 | 0x0018c280 | 702, and 2 more nops at the end in MUT |
| `str8800` | 0x00185170 | 0x00188e30 | 815, plus 1 nop |
| `str0300` | 0x00189e10 | 0x0018e5c0 | 871, plus 1 nop |

The MUT ends are where the next carried name starts. Both listings were
reduced to mnemonic and operands, with branch labels and addresses
normalized. What was left differs only in the `lui`/`addiu` pairs that
build three data addresses. Every call target is the same by name. The
three addresses are:

- the cue jump table (INF 0x0034e310 / 0x0034e280 / 0x0034e3a0, MUT
  0x00365d20 / 0x00365c90 / 0x00365db0). Every entry is moved by exactly
  the function's own shift (0x47b0, 0x3cc0 and 0x47b0), so it is the same
  switch.
- `eventExNoteTbl` (INF 0x00387f80, MUT 0x0039af80, 192 bytes) and
  `eventExNoteParamTbl` (INF 0x00388040, MUT 0x0039b040, 32 bytes). They
  are byte for byte the same.

So `piney-gen syms` missed these three names for another reason (the jump tables
are a guess), not because the code changed. docs/engine/stream.md
("Mutation's other effect tasks") now says so, and GAPS.md and
plans/mutation-story.md drop the item.

## Also: the desktop's late mail in [[270]]

The mail that stood the whole run on the desktop, 88, is sent by the side
event `ML001`, block 5. The block needs mail 87 answered with the first
reply (`mail_5 mail=87`) and PC 15's friendship at 125 or more; then
`mail 88`. It runs while the desktop is up. docs/engine/desktop.md (Mail)
has the inbox built once, by `AddMailList` (INF desktop.prg 0x00409800)
from `AddAllList`, and the event's mail opcodes only write `mailList` and
`mailOrderList`. So the game lists 88 only at the next opening too, and the
port matches it.

**Still unknown:**
- Why `piney-gen syms` did not carry `Func_str7100`, `Func_str8800` and
  `Func_str0300` to Mutation (a guess: their jump tables' relocations);
  they could be added to the MUT sidecar.
- Outbreak's and Quarantine's versions are not compared.
