---
number: 489
title: The later volumes' event cases settled: the rewritten ones are Infection's behaviour, area 126 and the NULL dtMenu test as the port has them, and nothing fills the BSS message groups
date: 2026-10-25
area: script
files: docs/engine/events.md, docs/engine/event-vm.md, UNKNOWNS.md
resolves: 291.1
started: 2026-10-25T19:03:52-06:00
took: 7m
---

# 489. The later volumes' event cases settled: the rewritten ones are Infection's behaviour, area 126 and the NULL dtMenu test as the port has them, and nothing fills the BSS message groups

Play question [[24]]/[[40]] (291.1): the later volumes' changed event
cases (Outbreak's 6, 8, 9, 60, 71, 72, 150), their event task's order,
and what fills Outbreak's and Quarantine's BSS message groups. The code
settles each. No code changed.

**The rewritten cases.** The events page already traces them. OUT and
QUA compile 6, 8, 9, 60, 71, 72 and 150 through small helpers in main
(the `ccMenu` request, the `ccAI` turn fields at their new offsets, a bit
clear, `ChangeInfo` with two fields zeroed, a zeroing constructor). They
call the same gcmn functions as Infection's inline code, so the
behaviour is the same. The real changes from Mutation on (`mode`, `fade`,
`fade_more`, `desktop_item`, `call_lock`, characters 18-20) are the
VM's per volume already. The task's walk is `story_stages` by `volumeNum`,
tested by `each_volume_walks_its_own_main_story`.

**The two the port was said not to model:**
- **`area 126`** (MUT 0x001c5874). For 1-13 the case quits `WORLD_MAN`
  while `game` +0x14 is set, then stores `eventAreaNumber` (+0x120). For
  126 it only stores it. The port's story scene sets the number for
  1-13 and 126 alike (`ev_area_number`) and models no quit, which is
  126's own path.
- **The NULL `dtMenu` test** of `info` and `info_now` (MUT 0x001be908):
  with no `dtMenu` the case leaves `Execute` with 0. `eventSub` then goes
  on to the next block from the pointer left mid-block. The test is
  never met:
  - `dtMenu` is cleared only by `ccThDtMenuDelete` (MUT 0x00169c94), as
    a mode's tasks go, and stored by `ccThDtMenu`'s first run
    (0x0016adc8).
  - `ccSetupDesktop` (INF 0x00168320) starts that task before
    `ccEnableThEvent(4)`. The event task, waiting out phase 3, meets
    phase 4 and goes round to its loop's `Breath(1)` before the first
    pass at 4. So the menu task has run first, whatever the set-up
    thread's priority.
  - Before phase 4, the cases take the set-up path (`enablePhase` < 4).
  So the port's desktop, with its menu from the start, is the game's.
  Event 103's block 1 meets it on Mutation's story path: its `info` 13
  and 14 ("You can now go to Λ Server") play on the first desktop pass
  after event 102's mail 33, with the menu task already run.

**The BSS message groups.** On OUT `evMsgTblM1`, `M2` and `M4` (and
their Parody tables) are BSS; on QUA `M1`, `M2` and `M3`. A scan of
every code word (`disasm.py xrefs --scan`) finds only loads of them, and
no store:
- `Execute`'s message cases, through `evMsgTbl`/`evMsgTblp`;
- the tutorial menus (`PartyMenuT`, `PersonalMenuT`,
  `PartyDisbandMenu`, `PartyInMenuT`, `GateMenuT`, `GtNewMenuT`,
  `PersonalMenuTS`), for M1's events 2-4.
Nothing fills them. A volume's pass walks only its own main story, so
in play nothing reads them.

Docs: events.md (the groups; the Unknown line gone), event-vm.md.

**Still unknown:** nothing
