---
number: 459
title: The battle banner stays off through a boss's fight: the HUD reads eventMng.bossEntry, which the port fed as -1
date: 2026-10-24
area: ui
files: crates/piney-game/src/area.rs crates/piney-fieldui/src/disp.rs crates/piney-game/src/session/tests/corbenik.rs docs/engine/field-ui.md
started: 2026-10-24T12:51:17-06:00
took: 11m
---

# 459. The battle banner stays off through a boss's fight: the HUD reads eventMng.bossEntry, which the port fed as -1

## "BATTLE MODE ON" over field 8's sky

Worklog 457's shots of Corbenik's fight showed "BATTLE MODE ON" tiled
across the top of the screen from start to end.

**Where the game draws it.**
- It is not the sky. `ccMenuCtrl::Disp`'s banner (INF gcmn 0x00521754;
  the port's `piney_fieldui::disp::banner`) draws it on the menu's
  `eneLife` sprite.
- With `game.inBattle` 1 and `inBattleCnt` 0 it scrolls three tiles at
  y 16 (the band while a fight waits). From count 16 to 60 it draws the
  shrinking announcement.
- It gates all of this at its top: `game` +0x58 set, the menu's forbid
  (+0xfe) 0, `eventMng` +0x04 (an event running) 0, and `eventMng`
  +0x08 (`bossEntry`) negative.
- `ccEntryEventMng` sets `bossEntry` to the code when an `entry 7`
  starts a boss (INF 0x001b6794). So from a boss's entry on, the banner is
  never drawn.

**The port.** `AreaMode::hud_world` gave the HUD `boss_entry: -1`
whatever the field held. It now gives the code of the boss the field
entered (`FieldWorld::boss_entry`, kept until the area changes), so the
band and the announcement stay off through every boss's fight, not only
Corbenik's. The earlier bosses' shots (Tarvos's announcement at its
fight's start) had the same mistake.

**Checked.** `session::tests::corbenik::the_banner_stays_off_through_the_fight`
sees the HUD's `bossEntry` 7 once Corbenik is in. The piney-game suite
passes.

`ccEvent::Init` (INF 0x001a6e40, from `ccStartThEvent` in each area's
set-up) sets `bossEntry` back to -1. The port's boss state lasts exactly
as long: the field's combat is made anew with each area.

**Still unknown:**
- Nothing left on the banner. Its draw is not compared with game frames;
  its gate is from the code.
