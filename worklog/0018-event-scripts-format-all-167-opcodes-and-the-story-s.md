---
number: 18
title: Event scripts: format, all 167 opcodes, and the story's control flow
date: 2026-09-22
area: script, decomp, tooling
files: tools/evscript.py, tools/test_evscript.py, docs/engine/events.md
supersedes: 9
---

# 18. Event scripts: format, all 167 opcodes, and the story's control flow

The story runs on event scripts: arrays of `short` compiled into main as C
data, interpreted by `ccEvent::Execute` (`INF SLUS_202.67:0x001a8d20`,
38,708 bytes, `evmng.cpp` lines 2641-6972) and gated by `ccEvent::CheckOpen`
(`0x001a7400`). All 167 opcodes, all 40 open conditions and all 12
precondition tags are decoded, `tools/evscript.py` disassembles every script,
and every one of the 192 scripts walks to its terminator with no desync. The
reference is [the events page](../docs/engine/events.md).

## The format

There is no header and there are no jumps: every opcode has a fixed length,
and control flow is entirely by guarded blocks.

```
open conditions ... 0          the event is live when all of them hold
block, up to 62 of them:
  (-2 tag operands)*           persistent preconditions (SetCurrentOpen)
  conditions ... 0             this block runs when all of them hold
  instructions ... 0           the block's body
-1                             end of the event
```

`eventSub` (`0x001b5ef0`) and `ccEventFlagSet` (`0x001b6160`) hold the walk
loop. Each event has a 64-bit word in `saveData.eventFlag` (+0x54f8, [[16]]):
a block that finishes sets its own bit - unless it ran opcode 4,
`repeatable` - and is skipped from then on; bit 63 closes the event, bit 62
marks it done, and `event_done` tests bit 62. `ccStartThEvent` promotes 63 to
62 for events 0-449 when the event task restarts, so the next story event
opens after the next restart - a mode change, for example (inferred from the
code). The interpreter's `lv` argument decides what runs: 0 walks a block
without effect, 2 plays it, and 1 - used by `ccEventFlagSet` to fast-forward
earlier volumes - applies only the bookkeeping. Of 166 real opcodes, 120 run
only when playing, 35 also at `lv` 1, 10 do different things at each, and one
(`virus_core`) only at 1.

Event `N`'s script is `eventTbl[N / 50][N % 50].tbl` (`eventTbl`,
`0x00317e30`, `{short *tbl; char *str}`, `str` a label like
`"MG0001 OPENING"`): groups M1, S1, M2, S2, M3, S3, M4, S4, then ML for
400-499. 192 events map one-to-one onto 192 arrays, 117,366 bytes. Messages
come from `evMsgTbl[N / 50][N % 50]` - `evMsgTblp` in Parody Mode - and voice
from the event voice tables of [[14]]. The main-story message tables for
volumes 2-4 are null on this disc: 933 of the 1,512 message opcodes in
volumes 2-4 have no text, while all 605 in volume 1 do. Like the mail, the
bosses and the story areas ([[9]], [[11]], [[12]]), the scripts for the later
volumes are here; their dialogue is not.

## Parody Mode is a different script

[[9]] read the parody tables as the Japanese original of the same text - the
mail and news entries it compared agreed line for line. The event messages do
not. In event 2, where the English has Orca greet Kite and Kite hesitate,
the parody table has Kite greet Orca by his real-world name and Orca yell
at him for it. Parody Mode is a rewritten comedy script. It
shipped on the US disc in Japanese, untranslated, and at least the event
dialogue differs in content. [[9]]'s statement that the parody entries it
read were ordinary translations stands for those mail entries; its wider
reading of what Parody Mode is does not.

## How it was verified

Statically, every path from every opcode's case back to the loop head was
followed through the nested jump tables, tracking every register and stack
slot derived from the script pointer: each case advances it by exactly one
constant and none stores a computed pointer, which is what rules out
variable-length opcodes and jumps. The DWARF locals of `Execute` and
`CheckOpen`, in source-line order, match the cases one to one; that is where
the operand struct names come from.

Dynamically, in `tools/eemu.py` ([[9]]): the real `Execute`, `CheckOpen` and
`SetCurrentOpen`, run at `lv` 0 on a single opcode, give the table's length
for all 167 opcodes, 40 conditions and 12 tags; a Python copy of the walk
loop calling the real functions stops at exactly the parser's block ends for
all 192 scripts (`evscript.py check`: 0 mismatches, and it does catch a
deliberately broken table entry); 29 bookkeeping opcodes run at `lv` 1 change
`saveData` exactly as described; and conditions 1, 10, 11 and 39 give the
documented comparisons. Opcodes 2 and 3 and the block-bit epilogue use
`dsllv`, which eemu does not interpret, and were not run.

Survey: 1,935 blocks, 13,515 instructions, 2,059 conditions, 1,918
preconditions. 145 of 167 opcodes are used; the commonest are `message`
(1,993), `wait` (1,596), `camz_set` (829), `menu_ban` (571), `pc_face` (547).
`tools/test_evscript.py` has 7 tests, including event 1's disassembly and the
full game-code check.

## The opening, as the script tells it

Event 1, `MG0001 OPENING`, opens when event 0 is done and the game is in mode
2. Its first block, at phase 0, sets 30 frames per second, loads gcmn, plays
cutscene stream 2, returns to 60 frames per second, plays stream 106, loads
the desktop overlay, shows two messages (the registration done, and Kite
thinking of Yasuhiko waiting), runs name entry,
intercepts every desktop operation except one, and delivers mails 4, 5 and
320, seven bulletin board posts and five news items. Until mails 4 and 5 have
been read, any other operation answers with Kite's line that the mail comes
first - which
suggests operation 1 is the mailer. Then the board fills with 57 posts and the
event closes.

**Still unknown:** the numbering of cutscene streams, markers, menus, sound
commands, player operations and NPC/PC action codes; the mail and BBS state
values; the camera angle units; whether the ML events (400-455), which wait on
later-volume events, can open in Infection at all; how far the parody script
diverges beyond the lines compared here.
