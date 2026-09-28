---
number: 124
title: "Streams 11 to 14 and 16's effect tasks"
date: 2026-09-25
area: render, test
files: crates/piney-stream/src/effect.rs, crates/piney-stream/src/lib.rs, crates/piney-stream/tests/effects.rs, tools/test_stream_rs.py, docs/engine/stream.md
---

# 124. Streams 11 to 14 and 16's effect tasks

This follows [[121]] and [[122]] with five more tasks:

- `Func_str0301` (stream 11), `Func_str0305` (stream 12) and
  `Func_str0350` (stream 13);
- `Func_str0570` (stream 14) and `Func_str0610` (stream 16).

All of them keep the same effect objects as the others. The cues are
listed in `docs/engine/stream.md`.

- **Fades.** The fades go to and from black and white, some over the
  frames left and some over a fixed count. `Ctrl::fade_left` now takes
  the colours, and when no frames are left it falls back to `frameEnd`,
  as these tasks do.
- **Feedbacks.** Some are plain. Others are set up with a fade ready
  (`fadeTime` set, `execTime` 0): they fade only when a cue x6 comes.
- **Stream 11's hit mark** uses `hitRot0301`.
- **Transfers.** Streams 12, 14 and 16 raise them, and stream 16's cue
  510 gives a taller character (`charHeightTbl[17]`, 210). A transfer
  now carries its own height (`effect::Transfer`), and the request
  passes it on.
- **Stream 14's own frame hooks.** Before its cues on each pass: at
  scene frame 1455 it starts a fade from white, and at 1470 it deletes
  the fade.

## Checked

`crates/piney-stream/tests/effects.rs` runs each of these streams against
the game's task in eemu. Every pass's primitives match by hash, and so
do `rand` and each mark's and transfer's object and bits:

| stream | passes | primitives | also |
| --- | --- | --- | --- |
| 11 | 3,463 | 32 | a hit mark |
| 12 | 1,303 | 158 | a transfer |
| 13 | 2,803 | 40 | |
| 14 | 1,828 | 3,836 | a transfer |
| 16 | 3,598 | 2,782 | transfers 160 and 210 high |

The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **Stream 15.** `Func_str0580` and `Func_str0581` make their own
  particle parts (`ccEffPart0580`, `ccStrPartGrp`) and event objects.
  They are not ported.
- **The `str7100`, `str8000`, `str8800` and `str9xxx` tasks** are not
  ported either.
- **What the marks and transfers look like** is not ported: nothing runs
  `ccThEffectStr`.
- **The fog** of streams 13 and 16 is not drawn.
