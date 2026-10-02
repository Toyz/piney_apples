---
number: 330
title: "scene does not sleep the event task: the pass runs on in its frame, disabled"
date: 2026-10-01
area: script, engine, decomp
files: crates/piney-event/src/vm/exec.rs, crates/piney-event/src/host.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/world.rs, tools/test_event_vm.py, crates/piney-event/tests/vm_fixture.txt, docs/engine/event-vm.md, docs/engine/field-walk.md
---

# 330. scene does not sleep the event task: the pass runs on in its frame, disabled

[[74]] read from the thread code that the event task, put to sleep inside
`scene`, wakes on the next frame. It left that unchecked against the game.
It does not sleep at all. All addresses are INF SLUS_202.67.

## Why

`ChangeScene` (0x00167380) ends in `ChangeRequest(6, 7)` (0x001671e0).
That calls `ccDisableThEvent`, then `ccSleepNoSleepThread(1, 1)`
(0x0015a200), which does two things:
- it raises the sleep count of every thread whose flags (+0x10) lack bit
  0;
- it puts the calling thread to sleep (flag 0x20, `SleepThread`) only if
  its flags lack bit 1 (`andi 0x2` at 0x0015a27c).

`ccThEvent` sets both bits on itself as it starts (`flags |= 3`,
0x001b5a8c). So `ChangeRequest` returns into `Execute`, whose `scene` case
(0x001b01d8) goes straight back to the loop head. Nothing there or in
`eventSub` (0x001b5ef0, all 62 blocks) tests the phase. The rest of the
block and the rest of the pass run in the same frame, with
`eventMng.phase` at -1. Blocks with a phase condition then fail
`CheckOpen`; blocks with none still run.

`mode` was already modelled this way (`ChangeRequest(num, 7)`, then the
next instruction). Every instruction that follows a `scene` in any
volume's scripts is bookkeeping, with no wait: `end_event`,
`gate_unmark`, `mail`, `bbs_post`, `news_add`, `menu_clear`, `set_block`.

## The port

The interpreter had `scene` wait (`Wait::ChangeRequest`) until the next
mode's host answered no. The rest of the block then ran a frame later, in
the next area. It also left the phase at 4 until the frame's end, when the
host disabled it.

Now `scene` calls `Host::change_scene` and disables the VM, and the pass
goes on. `Wait::ChangeRequest` and the hosts' answers to it are gone.

Two tests show the difference:
- **`event_2_plays_to_the_scene_change`** (world.rs) now ends with event
  2 closed (`CLOSED | 0b111`), no block playing, and the play pass done,
  all in frame 2295, the `scene`'s frame.
- **`event_21_elk_walks_up_in_mac_anu`** first failed with only the
  disable missing. Event 21's block 1 ends in `scene -2 ...` (the same
  town again). With the phase still 4, block 4 (`phase >= 4`) opened in
  the old town and walked Elk in the new town's set-up, before his entry
  ("not done"). With the disable, block 4 waits for the new town's play,
  as in the game.

## The checks' harness

`tools/test_event_vm.py` stubbed `ChangeScene` out entirely, so the game
side never disabled the task. It is now hooked to do what its
`ChangeRequest` does to the event manager (phase -1), without the scene.
`vm_fixture.txt` is regenerated from it.

**Still unknown:** nothing about the frame. The rest of [[74]]'s list
(the battle pieces, `pc_command`'s walks, Kite held, the party back in
Mac Anu) is unchanged.
