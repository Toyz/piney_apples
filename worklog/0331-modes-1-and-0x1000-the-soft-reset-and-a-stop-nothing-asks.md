---
number: 331
title: "Modes 1 and 0x1000: the soft reset, and a stop nothing asks for"
date: 2026-10-01
area: engine, decomp
files: docs/engine/overview.md
---

# 331. Modes 1 and 0x1000: the soft reset, and a stop nothing asks for

[[15]] left modes 1 and 0x1000 open. Both are handled inside `ccThMother`
itself, which shifts `ccGame.request[]` down and dispatches the first
entry (INF SLUS_202.67:0x00167c30).

**Mode 1** (0x00167ca0) branches back to the start-up path at 0x00167b18,
in this order:
1. `ccAllSoundOff`;
2. `game.status` = 0;
3. the layers flipped back (`fontOnFlip`, `OnFlipExcept`);
4. every task deleted (`ccDeleteAllThread`);
5. `ccSystem` and `ccDrawEnv` reset, two breaths, the default font;
6. `scFadeDef->Init`, `SetFrameRate(1)`, the gcmn file list;
7. `ccGame::Init`, `ccSaveData::Init(1)`, `ccEvent::Init`,
   `ccStartEvent(1, 0)`;
8. `ChangeRequest(2, 7)`, the title.

It is the soft reset. All twelve `ChangeRequest` calls in main and the
four overlays were listed with their first argument, and three ask for 1,
all as `ChangeRequest(1, 7)`:
- `ccDtMenu::ResetMenu` (0x0016ed90; `a1` set at 0x0016ed44), the
  desktop's Reset;
- `ccMenuCtrl::ResetMenu` (gcmn 0x00540688), The World's Reset;
- `ccOpenGameOverMenu` (gcmn 0x0056a7d4), the game over.

The port already treats 1 as `RESET`. The session boots the title again,
and `GameOverMode` asks for it.

**Mode 0x1000** (0x00167d00) flips the layers back and deletes every
task. Nothing else happens: no set-up, no next request. None of the twelve
calls asks for it. The scripts' `mode` takes its number from the script,
and every script uses 3 or 5 ([[323]]'s dumps). It is unreachable on
Infection, and the port has no case for it.

**Still unknown:** whether the later volumes ask for 0x1000 anywhere (their
`ChangeRequest` calls were not listed). The rest of [[15]]'s list (each
mode's tasks and their priorities, `ccSystem::Ctrl`'s other branches) is
unchanged.
