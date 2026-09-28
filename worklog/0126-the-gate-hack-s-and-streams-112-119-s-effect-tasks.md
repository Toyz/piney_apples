---
number: 126
title: "The gate hack's and streams 112-119's effect tasks"
date: 2026-09-25
area: render, test
files: crates/piney-stream/src/effect.rs, crates/piney-stream/tests/effects.rs, tools/test_stream_rs.py, docs/engine/stream.md
---

# 126. The gate hack's and streams 112-119's effect tasks

This adds two more tasks after [[125]]:

- **`Func_str7100`: the gate hack movie's town gate.** It has no fades.
  The last digit of each cue switches the noise, the inversion and a
  feedback of 1.015. It tests for a state 2 on cue 5, but both branches
  do the same thing.
- **`Func_str8800`: streams 112-119.** This is `Func_str8000` with its 5
  at 1.005, alpha 0x60.

`str7100` plays inside the gate hack's movie and has no stream number of
its own. Its test therefore runs the task alone: the fixture's cues on
their steps, `rand` from its default. The harness does the same with the
game's task.

## Checked

Every pass matches the game's task by hash, and `rand` after it:

- stream 112 (`str8801`): 363 passes, 427 primitives;
- `str7100`'s cues: 227 passes, 240 primitives.

**Still unknown:**
- **Stream 15.** `Func_str0580` and `Func_str0581` make their own
  particle parts (`ccEffPart0580`, `ccStrPartGrp`, 8.5 KB and 5.5 KB of
  task). They are the only stream effect tasks left unported.
