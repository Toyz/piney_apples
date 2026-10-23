---
number: 428
title: Magus's flashes fade in, hold and out and its grow's sound loops until it ends: EntryFlash2 and EntryFlash3 on the fader, ccSeOn3DLoop's id kept for ccSeOffLoop, and acts 30 and 23 shown unreachable from every writer of m_bDropping and m_bDrop
date: 2026-10-22
area: battle, audio, render, test, tooling
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/magus.rs, crates/piney-battle/examples/battle_probe/magus.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/area.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/main.rs, crates/piney-audio/src/lib.rs, crates/piney-game/src/session/tests/magus_fx.rs, crates/piney-game/tests/field_host_fixture.txt, tools/test_field_host.py, tools/test_battle_magus_rs.py, docs/engine/boss-magus.md, docs/engine/sound.md, docs/engine/event-vm.md
started: 2026-10-22T18:03:34-06:00
took: 22m
---

# 428. Magus's flashes fade in, hold and out and its grow's sound loops until it ends: EntryFlash2 and EntryFlash3 on the fader, ccSeOn3DLoop's id kept for ccSeOffLoop, and acts 30 and 23 shown unreachable from every writer of m_bDropping and m_bDrop

Worklog 274 left three open points in Magus's fight, and boss-magus.md
listed them:

- `EntryFlash2` and `EntryFlash3` were drawn as one flash;
- the grow's looping sound effect played once;
- act 30 and a leaf's act 23 were never reached by the harness.

## The flashes

`ccScFade::EntryFlash2(t0, t1, col)` (MUT 0x001603d0, INF 0x001602d0)
and `EntryFlash3(t0, t1, t2, col)` (0x00160460, INF 0x00160360) take the
first free element:

- `EntryFlash2`: status 5; `tcnt` t0, the out count (+6) t1; from `col`
  with alpha 0 (`col & 0xffffff`) to `col`. The hold's count (+8) is
  left as it was.
- `EntryFlash3`: status 9; the hold (+8) t2 as well.

`SendPacket`'s counting (already ported in piney-game's `ScFade::count`)
chains them:

- 9 goes to 5, held at the colour for t2;
- 5 goes to 3, fading out over t1;
- 3 ends.

Magus's rise calls `EntryFlash2(60, 60, 0x80e0ffff)` (0x004a41dc) and the
laser `EntryFlash3(80, 80, 20, 0x80e0ffff)` (0x0049ef58), both on the
whole screen. The port made both one `EntryFlash` (status 3) over the
summed time: a flash out with no fade in and no hold. Infection's and
Mutation's three entry functions are the same code.

## The grow's loop

`OnThinkLeafGrow` keeps `ccSeOn3DLoop(230, pos)`'s id (+0x5271c, -1 from
the constructor):

- at its end, `ccSeOffLoop(230, id)` unless the id is -1, and the id
  set -1;
- the drain's 13 (`Affect` 0x0049f76c) ends it too, but keeps the id.

The driver's loop functions were already ported and checked (`se3d.rs`,
tools/test_sound3d_rs.py's `loop`). Nothing called them: the area played
SE 230 once as an ordinary 3D sound, and its end was dropped.

## Acts 30 and 23

They are never reached in the game. Every writer of the two fields that
decide it, from a scan of all of gcmn:

- Act 30 is set only by `OnThinkNeutral` (0x004a1130). It needs
  `m_bDropping` (+0x526f4) 0 (0x004a0fa4), the laser reserved, and a leaf
  with `m_bDrop` (+0x297e4) set (0x004a1010).
- A leaf's act 23 is set only by `Order(28)` (0x004a6c38). `Order` is
  called only through the leaves' vtable (+0x84), at eight sites with
  constants. Order 28 comes only from act 30's `OnThinkDestroyLeaf`
  (0x004a3a48).
- `m_bDropping` is written by:
  - the constructor (0, 0x0049e29c);
  - `OnThinkLeafDrop`, on every frame of act 20 before anything else (1,
    0x004a3148);
  - the grow's end (0, 0x004a3904).
- `m_bDrop` is written by:
  - a leaf's constructor (0, 0x004a4c60);
  - `Init` (1, 0x004a5a84), from `Order(24)` and `Order(28)` only;
  - the grow's end (0 for all twelve, 0x004a38d8, just before its
    `m_bDropping` 0);
  - the fall's end (0, 0x004a460c).
- `Order(24)` comes only from `OnThinkLeafDrop` (0x004a3414), after its
  `m_bDropping` 1.

So a leaf is down only while `m_bDropping` is 1, and the neutral takes
the drop before it looks at the laser. The case that looks reachable is
all twelve down with the gauge high: the laser is reserved, but Magus
circles in act 20 (none left to drop, back to act 0, and the drop again)
until the leaves burst or die. Their regrow's grow clears both fields,
and the laser then finds no leaf down.

## The port

- piney-battle:
  - `Out::FlashFade { t_in, t_out, hold, colour }` (`hold` None for
    `EntryFlash2`);
  - `Out::SeLoop { se, op: LoopOp }`, with `LoopOp::On(pos)` and
    `Off { forget }` (the grow's end forgets the id, the drain keeps it).
- piney-game:
  - `ScFade::flash2` and `flash3`;
  - the area turns the loop into `Event::SeLoopOn` and `SeLoopOff`, which
    `handle` sends to `Audio::kept_loop_on` and `kept_loop_off`
    (piney-audio). These keep the id as `ccBoss03` does.

## Checks

- `tools/test_field_host.py` now also runs the game's `EntryFlash`,
  `EntryFlash2` and `EntryFlash3`. That is 41 more random faders, plus
  Magus's two flashes run out, with `SendPacket`'s counting, against
  `ScFade`: 1,863 calls agree (`the_fader_counts_as_the_game_does`).
  Mutation's fixture gives the same fader lines.
- `tools/test_battle_magus_rs.py`:
  - Some cases now force a while of `m_bDropping` 0 with the laser
    reserved. This is a state the game never reaches, so that act 30 and
    the leaves' `Order(28)` run.
  - A leaf's stand-in clip now has its chunk at `anmChunk` (+0x94), whose
    `frameMax` `Order(28)` divides by. The harness had put it at +0xa8,
    and the game divided by 0.
  - `bulk 120 10000` (308,056 frames) and the two tests agree. Acts 30
    and the leaves' 23 are reached.
- `session::tests::magus_fx::magus_flashes_and_its_grows_loop` sends
  every frame's events through `crate::handle` to a headless `Audio`:
  - the flash phases seen are (5, 60) and (3, 60) for the rise, and
    (9, 80), (5, 20) and (3, 80) for the laser;
  - over two grows SE 230 holds a loop slot on 592 of 596 grow frames
    (from the grow's second frame), and on none outside them.
  - Before, the flashes showed only status 3 and no slot was taken.
- piney-battle and piney-audio tests, piney-game's suite (four threads),
  clippy, fmt.

Shots (`magus_flash_shots`, /mnt/data/claude/scratch/a428):

- `magus-00352-act25-status5-t20.png`: the laser's flash held.
- `magus-01200-act27-status5-t60.png`: the rise's flash coming in.

**Still unknown:** nothing. The blur (ccBossBlur) is drawn for no boss (worklog 117).
