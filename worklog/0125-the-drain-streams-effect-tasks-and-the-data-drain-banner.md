---
number: 125
title: "The drain streams' effect tasks and the Data Drain banner"
date: 2026-09-25
area: render, battle, test
files: crates/piney-stream/src/effect.rs, crates/piney-stream/src/lib.rs, crates/piney-stream/tests/effects.rs, crates/piney-stream/examples/stream_shot.rs, crates/piney-game/src/stream.rs, tools/test_stream_rs.py, docs/engine/stream.md, docs/engine/field-ui.md
---

# 125. The drain streams' effect tasks and the Data Drain banner

Data Drain's movies played without their glitches. Neither Kite's drains
(streams 108-111) nor Skeith's (18) had their noise, inversion or
flashes. Skeith draining a member (stream 20) also had no letterbox and
no "Data Drain" title. Stream 17's task was also missing.

- **`Func_str8000`** runs the drain movies. It is `Func_str0300` with two
  cues changed: 999 falls back to `frameEnd`, and 75 has no case of its
  own. Skeith's `str9102` runs it too.
- **`Func_str9000`** runs the member drained.
  - It letterboxes the view with `SetFrame(0, 30, 512, 324, 256, 162, 1,
    0.84375)`. [[121]]'s `Frame` carries it.
  - It makes a banner when the game is running: a `ccMask` on a layer of
    its own (101), textured with `xeffect`'s `TEX_detadrain`. Every pass
    it draws the 256 x 20 title at (128, 10).
  - The texture is `DATA.BIN`'s, so the player passes it in:
    `Options::skill_names`, which `piney-game`'s stream player sets.
- **`Func_str9001`** runs stream 17, each cue through `Func_str9001sub`.

## The harness

The banner's sprite made the effects harness find three gaps:

- `ccSprite::MakePacketStr` returns at once while `fontTex` is 0, which
  the game has long set by this point, so the harness now sets it.
- `ccSprite::SetTex` looks the texture up in a CCSF file the harness
  does not have. It is hooked to give a marker TEX0 (256 x 128, PSMT4),
  which the GS model names `ccs`, as the Rust side names `TexRef::Ccs`.
- The sprite's GIF packet writes TEX0 in PACKED mode (register 6), which
  the decoder now reads, with CLAMP.

Every earlier fixture regenerates unchanged.

## Checked

`crates/piney-stream/tests/effects.rs` runs these streams against the
game's tasks. Every pass matches by hash, and so does `rand`:

| stream | passes | primitives |
| --- | --- | --- |
| 109 (a small enemy's drain) | 353 | 642 |
| 18 (Skeith's drain) | 353 | 642 |
| 20 | 353 | 884 (352 of them the banner) |
| 17 | 1,103 | 1,635 |

- `drain_movie_shots` shows the drain movie inverted and broken into
  bands at frame 161.
- `stream_shot --stream 20 --banner` shows the "Data Drain" title in the
  top bar of the letterbox.
- The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **Stream 20's fog**, and the members drawn into it in `stream_shot`.
  `stream_shot` has no party members; the field's `StreamMenu` draws them.
- **Unported tasks.** `Func_str7100` (the gate hack's movie),
  `Func_str8800` (streams 112-119) and stream 15's `Func_str0580` and
  `Func_str0581` are not ported.
