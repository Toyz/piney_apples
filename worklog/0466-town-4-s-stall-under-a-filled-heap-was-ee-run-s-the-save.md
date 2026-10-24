---
number: 466
title: Town 4's stall under a filled heap was ee_run's: the save went in without LoadGame, so Kite's name pointer was the port's stand-in inside the heap
date: 2026-10-24
area: tooling
files: crates/piney-eemu/examples/ee_run.rs docs/engine/heap.md plans/quarantine-story.md
started: 2026-10-24T15:37:55-06:00
took: 5m
---

# 466. Town 4's stall under a filled heap was ee_run's: the save went in without LoadGame, so Kite's name pointer was the port's stand-in inside the heap

## The stall

[[439]] left a side finding. With the heap filled (`EE_FILL=a5a5a5a5`),
town 4 stalls at about 1,440 vblanks. Unfilled, it loads. The guess was
that some town-4 code reads heap memory it never wrote and expects 0.

**Where it stops.** `ee_run` was given a wider stack dump at a spin: 256
words, each with its offset. The run is the save from before 313,
`EE_GO=900` and town 4.
- After the load, thread 21 (`ccThMenu`) spins in
  `ccGetExtendedCode` under `ccKanjiStrlen`.
- The chain above it: `ccThMenu`, `ccMenuCtrl::Disp`, then
  `ccMenuCtrl::FacePanelDisp` (gcmn 0x0042ebac).
- `FacePanelDisp` measures the panel's name: `ccKanjiStrlen(*GetSpcParam(id))`,
  the `name` pointer at `spcParam[id]` +0.
- `EE_PRINT` at that call shows id 0 (Kite) and the pointer 0x01810000,
  inside the heap's hole (0x00670330-0x01ff3ff0). Unfilled, the bytes
  there are 0, and the name reads empty. Filled, the length runs over
  25 MB of 0xa5.

**Why.**
- 0x01810000 is `piney_demo::newgame::SAVE_VA`, the port's stand-in for
  `saveData`'s address. The port stores it as character 0's name pointer
  in the saves it writes.
- The game never keeps a saved pointer. After a load, `ccThDemo` calls
  `ccSaveData::LoadGame()` ([title.md](../docs/engine/title.md)), which
  points every character's `name` and `ccsname` again (character 0's at
  the save's own `plName`).
- `ee_run`'s `EE_SAVE` wrote the bytes over `saveData` and changed the
  scene, without `LoadGame`.

So no town-4 code reads unwritten heap. The harness left the pointer.

**The fix.** `EE_SAVE` now calls `LoadGame__10ccSaveDataFv` on
`saveData` after the write.
- Its calls are `GetSpcParam`, `SetDefaultWord`, `setCameraCtrlType`,
  `SetDisplayOffset` and `SetSoundEnv`. None allocates, so the heap
  histories of [[436]]-[[439]] are unchanged.
- The name pointer is now `saveData` (0x00980ad0).

**Checked.**
- With `EE_FILL=a5a5a5a5`, town 4 now runs to 2,400 vblanks with the save
  at 900. With members 1 and 2 invited at 1,200 and 1,260, it runs to
  2,400, filled or not. With the save at 1,440 and the members at 1,740
  and 1,800, filled, it runs to 3,000.
- One run did fail: inviting member 1 at 1,560, 120 vblanks after a save
  at 1,440, faults in `_ccFree` (thread 21) both filled and unfilled.
  Town 4 is still loading then (its loader threads 16-19 are up), so the
  invite comes mid-load, which play cannot do. A later invite runs.
- piney-eemu's tests, clippy and fmt pass.

**Still unknown:** nothing.
