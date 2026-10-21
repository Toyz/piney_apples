---
number: 418
title: Lios's shutdown empties Carmina Gadelica alone: regist_npc16's 16 reaches the walking PCs' draw through ccRegisterEventMng's count, so the town after Innis has no walkers while Mac Anu keeps its own (#70)
date: 2026-10-21
area: world, script
files: crates/piney-event/src/vm/mod.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/shutdown.rs, crates/piney-game/src/session.rs, crates/piney-world/tests/world.rs, crates/piney-world/examples/world_probe.rs, tools/test_world_rs.py, docs/engine/field-game.md
started: 2026-10-21T11:06:56-06:00
took: 14m
---

# 418. Lios's shutdown empties Carmina Gadelica alone: regist_npc16's 16 reaches the walking PCs' draw through ccRegisterEventMng's count, so the town after Innis has no walkers while Mac Anu keeps its own (#70)

Issue #70 (build 3e87cd5, Mutation): after Innis, Lios says he forced a
system shutdown, yet the reporter's Mac Anu at night still had walking
PCs. The reporter was not sure it was a bug. The game's rule: the
shutdown empties Carmina Gadelica alone, so Mac Anu walking is right; the
port, though, never emptied Carmina Gadelica.

## What the game does

- Event 107 (M207, Innis's event). Block 22 ends the dungeon with
  `scene` to town 2. Block 23 (in town 2, phase 0, status 3 or more)
  enters the Administrator (`entry 4 29`) and runs `regist_npc16` (op
  143, `eventMng.registNpcNum = 16`). Block 24 is the tour of the empty
  town, then Lios: message 8 "Where's everyone else?", 9 the shutdown.
  Block 26 bars the gate and the town's ways out (`add_operate` 7, 6:
  "currently unavailable due to maintenance"); logging out ends it.
- Event 108 (M208), block 6 (town 2, phase 0, status 1: from mail 38
  until block 23 sets 2 after field 9's fight): Mistral (`entry 2 16`),
  six PCs at markers 7-12 (rows 122-127), `regist_npc16`.
- No other event on any disc uses the op. Mac Anu and Dun Loireag have
  their walkers throughout, as in the issue's shot.
- `ccSetupGameCtrl` runs the phase 0 pass, then `ccSetFileListTown`'s
  `ccRegisterEventMng(0)` (MUT main 0x001cc480, INF 0x001b6d70), then the
  phase 2 pass. It adds one to `registNpcNum` (+0x1c) per type 3/4 entry
  (`1 << type & 0x18`, 0x001cc640), then `ccSpcManager.registryNum`
  (+0xdc, 0x001cc6f4), and hands the sum to `ccRegisterRandomNpc`
  (0x001ccdf0): `16 - n - fewer` walkers, none once `n` is 16 or more.
  Nothing in the walkers' own code looks at the story: no flag in
  `ccRegisterRandomNpc`, `ccEntryRandomNpc` (0x001cc400) or
  `ccSetRtownPC` (0x00521cf0) but the area.

## The cause

The port's VM set its `regist_npc_num` and nothing read it. The town
counted the entries itself (`World::entry_npc`, from both passes) and
reserved `1 + members`. So Carmina Gadelica after Innis drew 7 walkers
(16 - 3 - 6 for server 2) and on event 108's visit one beside the six.

## Fix

- `EventMng::register_npcs` (piney-event): `ccRegisterEventMng`'s count,
  kept in the manager.
- `WorldMode`'s set-up calls it after the phase 0 pass and hands it to
  `World::set_regist_npc_num`.
- `World::place_entries` reserves it with `Spcs::registry_num`, the
  game's own field, in place of `1 + members`.
  `entry_npc` no longer counts.

No town block on any volume makes an entry at phase 2 (scanned in all
four volumes' scripts), so the registry the draw reads is the one the
count would have read.

## Check

- `session::tests::shutdown`, on Mutation's own starts:
  - `carmina_gadelica_is_empty_after_innis`: event 107, blocks 0-22 run,
    status 3, Kite and BlackRose in town 2. `registNpcNum` is 17 (16 and
    the Administrator), the Administrator stands, no PC.
  - `carmina_gadelica_has_only_the_events_pcs_after_the_shutdown`: event
    108 at status 1, the PCs are rows 122-127.
  - `mac_anu_walks_after_the_shutdown`: the same in Mac Anu, 14 walkers.
  - Before the fix the first two fail (`registNpcNum` 16; a walker, row
    61, beside the six).
- `TownPcsAgainstGame.test_reserved` in tools/test_world_rs.py runs the
  game's `ccRegisterEventMng(0)` in eemu (its callees answered) for 63
  cases: `registNpcNum` 0 or 16, entries of every type, registry 1-3.
  The count it hands `ccRegisterRandomNpc` matches the probe's
  `pcreserve` on all four volumes. `test_select` now also draws with 17
  and 19 reserved.

Shots (`shutdown_shots`, the tour, frame 400): before,
/mnt/data/claude/scratch/i70/before/carmina-400.png (a walker at the
bridge's right edge); after, /mnt/data/claude/scratch/i70/after/carmina-400.png
(empty; 0 PCs on all six shots).

**Still unknown:** nothing
