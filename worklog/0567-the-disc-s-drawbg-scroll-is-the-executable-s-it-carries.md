---
number: 567
title: The disc's DrawBG scroll is the executable's: it carries across visits from its first-use 1.0
date: 2026-11-02
area: world
files: crates/piney-desktop/src/save.rs, crates/piney-world/src/story_map/boss_arena.rs, crates/piney-world/src/story_map/disc.rs, crates/piney-world/src/systems/scrolls.rs, crates/piney-world/src/town/mod.rs, docs/engine/evarea.md, docs/engine/root-towns.md
started: 2026-11-02T16:18:24-07:00
took: 2m
---

# 567. The disc's DrawBG scroll is the executable's: it carries across visits from its first-use 1.0

Found while worklog 566 made the disc's scroll a `Scrolls` system: the
port started `EVENTAREAB8::DrawBG`'s scroll at 1.0 with each map it made,
so each visit to a disc field (9, 10, 11) began its cloud layer anew.

## What the code does

`DrawBG__11EVENTAREAB8Fv` (MUT gcmn 0x0041e420) opens with a first-use
guard on two statics of the executable's small data, not gcmn's:

```text
lb   v0, -28660(gp)      # 0x0038ba7c, the flag
bnez v0, skip
lui  v0, 0x3f80          # 1.0
sw   v0, -28664(gp)      # 0x0038ba78, v
li   v0, 1
sb   v0, -28660(gp)
```

then, outside field 12, `v` less 0.003 (0x3b449ba6), back to 1.0 below
0, `vftoi12(v)` less the crop into `MAT_se1_6clo1`'s V. Both are main's
(gp-relative), 0 at power-on and set by no load (root-towns.md, "The
scroll statics"): the first `DrawBG` of the session makes `v` 1.0, and
every later one goes on from where the last left it, whichever disc
field and however many visits. `Draw` (0x0041ed30) calls `DrawBG` only
outside field 12.

## The port

- `SaveState::scroll_statics` gains the two: `town::scroll_statics::DISC_BG`
  (26) and `DISC_BG_FIRST` (27), `SCROLL_STATICS` 28.
- `ScrollsDef::first` (`First { value, flag }`): a static with a first-use
  guard. The first step sets the lanes to `value`; the flag goes with the
  lanes in the statics (`Scrolls::used`). `ScrollsDef::from` is gone: every
  other scroll starts at the statics' 0.
- The disc's `SCROLLS` carry `DISC_BG` with its flag.
- `the_discs_scroll_is_the_executables`: field 9's first step gives 1.0
  less the step, its statics carry it and the flag, and field 10 goes on
  from there.

## Checks

- Golden shots, every PNG byte for byte, the towns' set and the story
  maps', fields', dungeons' and breaths' (`kyvia_stage_shots` among them,
  one visit from power-on: its first `DrawBG` sets 1.0 as before).
- Against the game on all four volumes, the harnesses of worklog 566.
- The crates' tests, piney-game's suite (release); `piney-gen gen
  --check`, clippy with all features, fmt, `cairns check`, `tools/docs.py
  check`.

**Still unknown:** nothing.
