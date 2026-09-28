---
number: 128
title: "Skeith's cinema: the bars and the skill's name"
date: 2026-09-25
area: battle, render, test
files: crates/piney-world/src/cinema.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-world/examples/cinema_probe.rs, tools/test_cinema_rs.py, docs/engine/boss.md
---

# 128. Skeith's cinema: the bars and the skill's name

In the game, when Skeith casts its magic or starts its Data Drain, black
bars slide over the top and bottom of the picture. For the magic, the
skill's name ("Judgement") fades in within the top bar. The rules already
raised `Out::Cinema` ([[113]]), but nothing drew it; [[117]] listed the
cinema as waiting.

## ccBossEffCinemaFade

The cinema is a state machine with six modes, stepped once per frame by
`ccBossEffManager::Draw` before the boss's task.

- **Slide in.** The bars start 30 logical lines off the picture. Each
  frame the remaining distance is divided by 1.5, and it becomes 0 once
  it is under 1.
- **Hold.** The name's transparency rises by 1/15 a frame.
- **Slide out.** The same slide in reverse, with the name fading out.
- **The bars.** Each frame each bar is a one-frame `ccScFade::EntryFlash`,
  an element deleted when done: black at alpha 0x80, 512 x 30, on the
  cinema's layer (241).
- **The name** is a `ccMask` over the top bar, cell 0 of its texture.
  The texture comes from `_g_cinemaSkillName` (gcmn 0x005eb040); row 2
  is Skeith's `x11` `TEX_ske_skl`.

`piney_world::cinema::Cinema` is the port. `BossRun` steps it at the start
of the boss's frame and takes `Out::Cinema` after `Main`. `BossLook` finds
the name's texture when the boss's files load, and the field world draws
the cinema on layer 241.

## One thing the game leaves to chance

`Init` never sets `tempDat`, which holds where the bars were last drawn.
The first `Draw` after the cinema is switched on uses it. On a fresh
(zeroed) heap that puts both bars at y 0 for one frame. The harness has
exactly that, so the port starts from 0 as well. What the PS2's heap
really leaves there is not known.

## Checked

- **`tools/test_cinema_rs.py` (new).** It builds the game's
  `ccBossEffCinemaFade` by its constructor, with a manager holding it,
  over the gcmn overlay. It runs eight runs of 300-600 frames: the cinema
  switched on and off at random with numbers 2, -1, 0, 5 and 70, `Draw`
  every frame, and `cinema_probe` alongside. Every frame matches: the
  name drawn or not (transparency bits, `wv`), every bar's y bits (and
  its constant frames, colour and size), and which ons set a name
  (`SetTex("x11", "TEX_ske_skl")`).
- **`cinema::tests::slides_in_and_out`.**
- **`skeith_act_shots`** (the magic) shows the top bar with "Judgement"
  over the arena.
- The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **`hold 7` and `ccBossBlur`** are not ported.
- **Other names.** Only row 2's name is looked up. The other bosses' rows
  and the `x01`-`x04` files (`game`+0x24 9-12) are for later volumes.
