---
number: 527
title: Boss split, step five: Part<T: PartClass> and Parts<T> for every part's take and put
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss/part.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/, docs/engine/boss.md, plans/rust-cleanup.md
started: 2026-10-30T11:51:01-06:00
took: 10m
---

# 527. Boss split, step five: Part<T: PartClass> and Parts<T> for every part's take and put

The fifth step of section 2's boss split (`plans/rust-cleanup.md`; the
second item, after 523-526). A boss's parts are bosses of their own
characters, which the master's frame takes out and puts back. Each kind
had its own copy of the take and the put. Now there is one, generic over
the part's class. The 288 pinned probe answers are byte for byte the
same.

## Part and PartClass

`boss/part.rs`:

- **`PartClass`**: a part's members as its `Class` variant holds them
  (`wrap`, `unwrap`).
  - A small `macro_rules!` implements it for the eight kinds: the Kyvia
    core and gomora, the Magus leaf, the Gorre brother, and Corbenik's
    seed, root, eye and killer eye.
- **`Part<T>`** (moved out of `kyvia.rs`):
  - `take(cx, me) -> Option<Part<T>>`: the part out of its character. A
    slot with no boss, or with another class, is left as it was.
  - `put(self, cx)`: the part back into its character.
- **The stand-in, where code indexes its parts:**
  - `take_or_stand_in` gives a default part at `me`, and whether there
    was one to take.
  - `put_back(self, cx, found)` never writes a stand-in over what the
    character holds.
  - This is the old copies' placeholder rule, kept.
- **`Parts<T>`**: a master's parts by index for one frame, with their
  found flags.
  - `Parts::take(cx, &mes)` takes them all in order; `put` puts them all
    back and empties the list.
  - It derefs to `[Part<T>]`, so indexing and iterating read as they did.

## The copies gone

- Kyvia's `Tree::take`/`put` (the core, then its gomoras) and Magus's
  `Leaves::take`/`put`. They are `Part::take` and `put`; a missing part is
  skipped, as before.
- Magus's `LeafPart`, a second `Part` by another name: it is
  `Part<Leaf>`.
- Gorre's `take_brother`/`put_brother`. Gorre keeps its two brothers as
  an array between frames, so it uses `take_or_stand_in` and `put_back`;
  a brother's lone affect does too.
- Corbenik's seeds' `take`/`put`, `take_seeds`/`put_seeds`,
  `take_root`/`put_root`, `take_roots`/`put_roots`, `take_eye`/`put_eye`,
  `take_killer`/`put_killer` and the third form's
  `take_parts`/`put_parts`.
  - `First::seeds`, `Second::roots`, `Third::eyes` and `Third::killers`
    are now `Parts<_>`.
  - The found arrays the frames passed from the take to the put live in
    `Parts`.

The diff is about 220 lines added and 320 removed.

## Checks

- The pin: 288 probe calls replayed, every answer byte for byte the same.
- The tests of piney-battle, piney-world, piney-effect and piney-event,
  and piney-game's full suite (its boss session tests), release.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 152 before, 152 after.

**Still unknown:** nothing
