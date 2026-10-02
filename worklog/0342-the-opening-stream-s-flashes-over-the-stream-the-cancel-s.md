---
number: 342
title: The opening stream's flashes over the stream: the cancel's and the end's
date: 2026-10-01
area: ui, video
files: crates/piney-demo/src/lib.rs, crates/piney-game/src/session.rs, crates/piney-game/src/stream.rs, docs/engine/title.md
---

# 342. The opening stream's flashes over the stream: the cancel's and the end's

The title's intro stream has two white flashes, and the port got both
wrong. A cancel push's flash was not modelled. The end flash was entered
only once the stream had finished, then advanced 20 frames at once. So the
stream's last 20 frames never faded to white, and the menu came in partway
through the fade-out.

## The game

`PlayOpeningStream` (INF demo.prg:0x004066b0) starts `ccThExecuteStream`
at priority 33 with the stream number in its param +0x14. The count is
410 for volume 1's plain stream and 450 with `m_ParoFLG`. INF's switch on
`m_NowVol` gives volumes 2-4 450 and 490. It waits for `ccGetStreamAdrs()` and then the stream's
+0x17e bit 3. Then, each frame (0x00406838), before the stream's own task
(the same priority, started later):

```
pad push & saveData.assignPADcancel (+0x8410) and s1   EntryFlash(50, 0x80ffffff), s1 = 0
the stream task's +0x14 == -1                           return
ccGetStreamFrame() == count - 20 and s1                 EntryFlash3(20, 20, 5, 0x80ffffff), s1 = 0
ccBreathThread(1)
```

The flashes go to `scFadeDef` over (0, 0, 512, 384). A cancel does not end
the stream; the stream's own skip rule does that. It only flashes, and it
spends `s1`, so the end flash never follows it. MUT (demo.prg 0x00419720),
OUT (0x004153e0) and QUA (0x00307d40) run the same loop. OUT and QUA keep
the count and the flag in swapped registers and load the rectangle's 512.0
and 384.0 from constants (OUT 0x0041db58, 0x0041db50).

## The port

`Demo` keeps the count from its `Request::Stream` as `stream_flash`, the
port's `s1`. While `PlayOpeningStream` holds the title, the session calls
`Demo::stream_tick(pad, frame)` before each of the stream's steps, from its
first step on, with `StreamPlayer::frame()`, the frame the last step drew.
After the step, `Demo::stream_fade` draws `scFadeDef` over the stream's
picture. `Phase::Stream` now just goes on to the title. The demo's own
frame keeps sending the fade, so the flash runs on over the menu.
`advance_one` is gone.

`the_opening_stream_flashes`: played through, the flash starts on the step
that draws the stream's frame 391 (the check saw 390). It lies over the
stream's last 20 frames (391-410). The stream then shows frame 0 twice as
it ends, and the menu comes out of the white, 47 frames lit in all. With
cancel pushed on the 100th step, white starts on that step and lasts 50
frames, the stream ends two steps later, and no second flash comes.

**Still unknown:** how many frames the stream takes to set its +0x17e bit 3
before the loop looks at the pad; the port starts on the stream's first
step.
