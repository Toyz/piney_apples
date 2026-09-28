---
number: 99
title: Piros's colour: the event instruction piros_colour and a character's affect tint in town
date: 2026-09-25
area: script, render, test
files: crates/piney-game/src/piros.rs, crates/piney-game/src/piros_fixture.txt, crates/piney-game/src/field_host.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/event22.rs, crates/piney-event/src/host.rs, crates/piney-event/src/vm/exec.rs, crates/piney-world/src/char.rs, crates/piney-world/src/body.rs, crates/piney-world/src/lib.rs, tools/test_piros_rs.py, docs/engine/event-vm.md
---

# 99. Piros's colour: the event instruction piros_colour and a character's affect tint in town

[[92]] and [[93]] left `piros_colour` (137) unported: event 22 (PIRO02)
runs it in Mac Anu, Dun Loireag, field 31 and dungeon 31, where Piros
changes colour after Mia's potion. It was parked until Dun Loireag
landed ([[97]]).

## What the game does

The details are on [the interpreter's page](../docs/engine/event-vm.md#piros_colour-137).
- **When it runs.** The case (main 0x001b0c1c) runs only while playing,
  and only with Piros (`charTbl` 8) in `ccSpcManager`'s registry.
- **What it does.** It switches on `eventStatus[1]`, with the
  instruction's operand used for status 9. It flashes `scFadeDef`, plays
  sound 74, shows an information line, and sets Piros's affect tint
  (`affectColorFix`, `affectColorRate` and `affectColor`), with breaths
  between.
- **The pulse (7, 11-13):** 13 ramps of `fptosi(65 k / n)`, a frame each.
- **Status 9 is the finale.** Operands 1-4 ramp a colour down and back
  up, and 5 clears the tint with a long white flash.
- **The line** is the first line of the event's message numbered by the
  save's +0x6510.
- **How it draws.** `ccChar::Draw` (gcmn 0x0056b1c0) with the fix set
  blends the character by `SetFogBlend(rate, colour)` each frame without
  decay. The port's `char_blend` had skipped the fix, since no enemy
  sets it.

## The port

- **The sequence.** `crates/piney-game/src/piros.rs`'s `Sequence` is the
  case as a list of (breath, actions). `start` gives the instruction's
  own frame, and `tick` one frame of the interpreter's `busy` poll.
- **The line.** `Host::piros_colour` now takes it. The interpreter reads
  it from the event's messages (the Parody table when on), as the case
  does.
- **The town host.** It runs the sequence with a second fader
  (`scFadeDef`, on the font layer), sound 74, `announce_lines`, and
  `World::set_affect_colour`. It also gains `begin` and `end`, which this
  wait needs nothing from.
- **`piney_world::char::Char`** gains `AffectColour`, with
  `ccChar::Draw`'s blend (the fix held, else the flash counting down).
  `fade` works it out and `draw` passes it through the new
  `Body::draw_char_fog`.

## Checked

- **`tools/test_piros_rs.py`** runs the game's case in eemu.
  - It covers statuses 0-14 with messages 0-3, status 9 with operands
    1-5, and a run with no Piros: 66 records, now committed as the
    fixture.
  - It records every flash (count, colour and rectangle), sound, line
    (length and hash), and breath, each with the tint after it.
  - `piros::tests::piros_colour_is_the_games` replays all 66 with no
    difference.
  - It first found that 11-13 end with the tint back at 65, where 7 ends
    at 0.
- **`event_22_lines_are_the_games`.** Event 22's messages 0-3, read as
  the interpreter reads them, give the four lines' lengths and hashes the
  game passed to `DispInfo`.
- **`session::event22::event_22_turns_piros_orange`.** From `story:22`
  through the board and Log in to Mac Anu, block 4 plays. Piros ends at
  fix 1, rate 65, 0x002080ff, and no instruction on the way falls to a
  host default.
- **Shots.** `event_22_shot` (ignored) writes the orange flash, and Piros
  orange at "Aaaargh!!!", to /mnt/data/claude/scratch/event22.
- **The rest.** The workspace's tests, clippy, fmt and the docs check
  pass.

**Still unknown:**
- **The field and dungeon side is not done.** `area_host` has no
  `piros_colour`, and `field_world`'s `draw_cast` draws the party with
  `draw_char`, not `draw_char_fog`. Event 22's blocks in field 31 and
  dungeon 31 still report it unported. That code is the combat agent's
  lane: it needs a `Sequence` in `area_host` and `a.ch.affect` set on
  Piros's actor.
- **The pixels are not compared.** The tint's look is the port's fog
  blend (`FogBlend::fog`); it is compared with neither the game's pixels
  nor a capture.
- **Where `scFadeDef` draws.** It is taken as the font layer, the same as
  the title's `ccScFade`; its own layer word (+0x94) is not read.
- **Status 9 with an operand past 5** pulses in a colour the game's stack
  last held; the port does nothing. No script uses it.
