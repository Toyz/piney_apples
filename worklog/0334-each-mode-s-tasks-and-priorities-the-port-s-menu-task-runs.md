---
number: 334
title: "Each mode's tasks and priorities; the port's menu task runs after the world's"
date: 2026-10-01
area: engine, decomp
files: docs/engine/overview.md, GAPS.md
---

# 334. Each mode's tasks and priorities; the port's menu task runs after the world's

[[15]] left open which tasks each mode starts and in what order. [[101]]
noted that a request the menus make reaches the sound task a frame late in
the port.

Every `ccStartThread(fn, priority, stack)` call in the set-ups, read with
its arguments (INF SLUS_202.67):

| started by | tasks, by priority |
| --- | --- |
| `ccThMother` | `ccThEvent` 32, `ccSoundMain` 112 (0x001679ac) |
| `ccSetupDemo` | `ccThLoadOverlay` 17, `ccThDemo` 33, `ccThDtMenu` 33 |
| `ccSetupDesktop` | `ccThLoadOverlay` 17, `ccThDtMenu` 33, `ccThDesktop` 33 |
| `ccSetupToppage` | `ccThLoadOverlay` 17, `ccThDtMenu` 33, `ccThToppage` 33 |
| `ccSetupNewGame` | `ccThLoadOverlay` 17 |
| `ccSetupGameCtrl` | `ccThGameCtrl` 33, `ccThMenu` 34, `ccThCamera` 40, `ccThSpc` 48, `ccThEntryCtrl` 64, `ccThExecuteStream` 65, `ccThEffect` 80, `ccThSkill` 82, `ccThFieldDisp` 96, `ccThParticle` 98 |

The kernel runs the woken tasks lowest number first, each to its next
`Breath`. So a frame of The World runs:
1. the event task;
2. the game control;
3. the menus;
4. the camera;
5. the party (`ccThSpc`);
6. the entries (enemies, gimmicks, NPCs);
7. the movie stream, the effects, the skills, the display and the
   particles;
8. the sound task, last.

A menu's request is acted on by the characters and heard in the same
frame.

## The port

`AreaMode::step` (area.rs) runs, in order:
1. the event task;
2. the map button (game control);
3. the ride's slot;
4. `world.step_into`, which is every world task;
5. the talks, the shows and the menu's rules;
6. the menu task (`ui.step_into`) and its requests.

`WorldMode::step` has the same shape. The menu therefore runs after the
characters, where the game runs it before them. A menu's request (a skill,
an item, an order) reaches the world a frame late, and so does the sound
it leads to. That frame is the cause of [[101]]'s late words, and it
affects every menu-to-world request, not only sound.

Moving the menu task to its place changes the frame on which a talk opens
its menu, on which the menu's rules see the battle's shows, on which Data
Drain's movie and the item steps are answered, and on which the gate hack
starts. Session tests pin all of these to frames. That is a pass of its
own, listed in GAPS.md under "Missing in play".

**Still unknown:** how the kernel orders tasks of equal priority (the
title's, the desktop's and the top page's two at 33), taken as their start
order. `ccSystem::Ctrl`'s other branches ([[15]]) are unchanged.
