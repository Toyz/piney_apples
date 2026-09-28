---
number: 96
title: The gate hack: GtHackMenu and ccHackMenu, the virus cores into a protected area
date: 2026-09-25
area: ui, test
files: crates/piney-fieldui/src/menus/hack.rs, crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/render.rs, crates/piney-fieldui/src/spr.rs, crates/piney-fieldui/src/tables.rs, crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/src/menus/mod.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-desktop/src/anm.rs, crates/piney-demo/src/fade.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/gate_hack.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md
---

# 96. The gate hack: GtHackMenu and ccHackMenu, the virus cores into a protected area

Six of Infection's story areas are protected: 16, 19, 22, 23, 25 and 27
(`protect[1] != 0`). The gate's Warp sends such an area, until its
`protectArea` bit is set, to menu 62, the gate hack. The port closed menu
62 at once (`Request::Unported`), so the story could not get into those
areas. No one had it, so I took it.

## What the game does

The details are on [the field UI page](../docs/engine/field-ui.md#the-gate-hack-gthackmenu-menu-62).
- **The warning.** Proccess 0 and 1 of `GtHackMenu` (gcmn 0x005661e0) show
  "PROTECTED / You cannot warp to this area." and four red flashes, while
  the town's music fades (`ccSndGateHack(0)`).
- **The menu's own frames.** Proccess 2 runs them. The screen breaks up
  (noise), goes black, and the layers flip again under a sleeping world.
  Then comes `ccHackMenu` on `XDHHACK.CCS`, with the camera, the background
  (`back0`, then `back1` looping), the ring of crystals, the frame, the
  completion, the 26 cores' counts and the slot's core.
- **The minigame (`Select`).** Left and right turn the ring; up and down
  put the slot's core in and take it out; OK passes only with every slot
  full.
- **Success.** The cores are taken and `protectArea`'s bit is set.
  `setupMode` 1 and `gtHackFlag` make the arrival hack in, `areaCount` goes
  up, and `SetGenerateCode` sends the party to the area.
- **Cancel.** The music comes back and the town wakes after 40 frames of
  noise.
- **The area's `protect` row** holds four (core, count) pairs. Area 19
  wants two of core 12 (M).

## The port

- **`crates/piney-fieldui/src/menus/hack.rs`.** The menu's frames are
  `Resume` steps, resumed from `After::Hack` as the OPTION pages are. The
  screen, `HackMenu`, keeps `ccHackMenu`'s fields under their names.
- **`piney_desktop::anm::Anm`** gains an owner's writes to its objects:
  - `set_disp` for `dispSW`;
  - `set_local` for the matrix with `matCalcSW`;
  - `object_named` and `objects_prefixed` for `GetSubstAdrsF` and
    `GetSubstAdrs("x*")`;
  - `local_of`, and `draw_on` for a layer and view of the caller's.
- **`ScFade::entry_flash2`**, for the red flashes.
- **The draws.** `Draw::Hack` carries the animations to `render.rs`, which
  draws them on layer 127 through `ANM_xdhcamer`'s camera. The texts are
  `Draw::Send` on `Obj::HackMask` (`TEX_xdhroma1`, layer 128) and on
  `Obj::SysFont` (the global `font`, on the font layer).
- **New requests:** `GateHackSound`, `HackNoise`, `HackScreen` and
  `GateHacked`. The town acts on two:
  - `HackScreen` stops drawing the world and the minimap;
  - `GateHackSound` 0 and 1 fade sequences 0 and 2 out and back over 20
    frames.

## One mistake the check caught

The cancel's `EntryFade(20, $a2, 0)` never sets `$a2` itself. It still
holds the 0x80000000 that was just or'd into `bgColor`, so the fade runs
from black to clear. I had read it as 0 to 0; the cancel scenario's first
fade frame differed.

## Checked

- **`tools/test_fieldui_rs.py`.** It runs the game's `ccThMenu` natively
  through menu 62, and 62 is no longer on its unported list.
  - **How `ccHackMenu`'s animations are faked.** Each `_AnimateForward`
    ends by its animation's real length, so `back0` ends on its 55th step.
    `GetSubstAdrsF` hands out blocks whose `dispSW` the menu writes.
  - **What each draw logs.** Every `ccAnm::Draw` logs its animation. The
    crystal draw also logs its 48 switches and the ring's turn, taken
    from `sceVu0RotMatrixZ`'s argument as bits.
  - **Other hooks.** The gate hack's `EntryFlash` goes on the menu's fader
    as the game's does, and its `SetNoiz`, sounds, thread and chat calls
    are logged.
  - **`test_gate_hack`, success (600 frames).** Two M go in, a third is
    refused, the ring turns both ways (an empty slot's down is refused),
    and OK runs the completion and the fade into `SetGenerateCode`. The
    cores are gone and `protectArea` is set.
  - **`test_gate_hack`, cancel (520 frames).** One M only: OK is refused,
    then cancel, the noise, and the shut.
  - **Result.** Every frame's state, packets, kanji, message, events and
    watched save bytes match. The six field UI suites (129 tests) pass.
- **`session::gate_hack::gate_hack_opens_area_19`.** From `story:19` (area
  19 closed again, in the Word List, three M in Key Items), a scripted
  player walks to Mac Anu's gate, chooses area 19 from the Word List, puts
  two M in and presses OK. The party arrives in field 19 with the bit set
  and one M left.
- **Shots.** `gate_hack_shots` (ignored) writes the PROTECTED window, the
  intro, the crystals, one core in, and COMPLETE to
  /mnt/data/claude/scratch/gatehack. The screen reads as the game's.
- **The rest.** The workspace's tests, clippy, fmt and the docs check
  pass.

**Still unknown:**
- **The hacked arrival is not ported.** `gtHackFlag` and `setupMode` 1
  drive `ccPlayer::GateHackingOut` (gcmn 0x0059bc00),
  `GateHackOutFileList` and `ccGateHackOutCcsName`, and the fellows' and
  the AI's `ccCheckGtHack`. `ccRequestLoadStreamGateHack` (main
  0x00199f00) is called from `ccThExecuteStream`. The party arrives as from
  any warp. This is field-player code.
- **The noise is not drawn.** `ccNoiz` is logged and asked for, not
  drawn.
- **The cancel's music.** `ccSndGateHackCtrl` restarts sequences 0 and 2
  at volume 0 (`ccSqPlayVol`) before fading them in. The port fades them
  back from where they are. `ccSnd+0x64` (how many sequences are faded)
  is read as "both".
- **The 3D is not checked against the game's pixels.** The harness
  compares the animations' calls, switches and turn, not what they draw.
  The shots are all there is.
- **After success, proccess stays 2.** The next frame starts the fall
  again until the area changes. The port does the same; how many such
  frames the game shows before its scene change is not measured.
- **`gate_hack_anim`** (instruction 157) waits on `ghoFlag`
  (`ccCheckGtHackAnm`), which the arrival sets. The hosts do not model it.
