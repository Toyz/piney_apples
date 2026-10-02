---
number: 329
title: "The field's set-up passes checked: ccEnableThEvent and the game+4 exit"
date: 2026-10-01
area: script, engine
files: crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, docs/engine/event-vm.md
---

# 329. The field's set-up passes checked: ccEnableThEvent and the game+4 exit

[[69]] took the town's and the field's side of the event passes from the
desktop's, unchecked. All addresses below are INF SLUS_202.67.

`ccEnableThEvent(n)` (0x001b52c0) stores `n` in `eventMng.phase` (+0x0c):
- for 0, it breathes until the phase reads 1 or drops below 0;
- for 2, it waits the same way for 3;
- for 4, it returns at once.

`Vm::enable_settled` already had these exact exits.

`ccSetupGameCtrl` (0x00168960) runs towns, fields and dungeons through one
path and calls it three times:
- **`(0)`** at 0x00168c98, after `ccStartThEvent` and
  `WORLD_MAN::SetEventData`;
- **`(2)`** at 0x00168f94, after the area's file list and
  `ccFileExistCheck(0)`;
- **`(4)`** at 0x001694d4, after the load, `GO`, `ccGetStartPositions`
  and `rebootSpcManager`.

That is the port's order. `AreaMode`'s and `WorldMode`'s `Setup` make the
passes at 0 and 2 and enable 4 at F0.

One step was missing. After each call the set-up reads `game+4` and
returns if a mode change is waiting (0x00168ca8, 0x00168fa4, 0x001694e4).
`ChangeRequest` disables the task, so a pass that asked for another mode
ends the set-up there. The port went on: a pass at 0 that disabled the
task counted as made, and `enable(2)` turned the task back on in a VM
that the next mode inherits. Both `Setup::Pass` arms now stop
(`Setup::Left`) when the pass leaves the task disabled.

No town or field script reaches it. The scripts that ask for a mode or a
scene in a pass at 0 or 2 run on the desktop (`game_status 2`):
- event 4's block 9 (`mode 3`), on all four volumes;
- event 314's block 2 (Quarantine's ending).

The desktop's set-up has its own handling. The piney-game suite passes
(204).

**Still unknown:** no test drives a town or field pass into a mode
change, since no script does it. The load's frames between the passes at
2 and 4 are still taken as none (docs/engine/event-vm.md Unknown).
