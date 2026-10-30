---
number: 520
title: dead_code denied: the last 13 exemptions test-only, used or deleted
date: 2026-10-30
area: build, tooling
files: Cargo.toml, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/stream.rs, crates/piney-game/src/session.rs, crates/piney-game/src/mode.rs, crates/piney-fieldui/src/book/pages.rs, crates/piney-desktop/src/dtmenu.rs, plans/rust-cleanup.md
started: 2026-10-30T08:57:27-06:00
took: 9m
---

# 520. dead_code denied: the last 13 exemptions test-only, used or deleted

Section 1's `dead_code` item of `plans/rust-cleanup.md`. Worklog 518 left
13 exemptions; now none are left, and `[workspace.lints.rust]` denies
`dead_code`.

## The 13

- **piney-game's test-only accessors** are now `#[cfg(test)]`, so the
  game's build no longer carries them. They are `WorldMode::calls` and
  `vm`, `AreaMode::map_state`, `calls`, `streaming` and `stream_scene`,
  `StreamPlayer::entry_name`, and `Session::new` (the game starts its
  sessions with `after_logos` and `in_world`; only the tests use `new`).
- **`WorldMode::world` and `AreaMode::world`** were marked "for tests"
  but the session reads them (`start_before`, the town's file, the
  spcs, `gt_hack`). They lose the allow and keep their place.
- **`mode::Event::WorldMan` and its arm in the session** are deleted.
  Nothing raised the event. The session handles the `area` instruction
  itself, through `area::ev_area_world_man`, so the event was a second,
  unused road to the same place.
- **`fieldui/book/pages.rs`'s `Table`** carried the allow and the note
  "Books 4-8's pages, which read T40 on, are not ported yet (worklog
  317)". They have been ported since: every variant is used by
  `CheckItemGetNN`. Both the allow and the note are gone.
- **`dtmenu.rs`'s `json_lines`** had `cfg_attr(not(feature = "trace"),
  allow(dead_code))`. Its callers' closures name it in every build, so
  it is used whether or not the closure runs. The attribute was stale.

## Checks

- `cargo clippy --workspace --all-targets`, with and without
  `--all-features`, and `cargo check -p piney-desktop -p piney-fieldui
  -p piney-game` on their own features: no dead code.
- The tests of piney-fieldui and piney-desktop, and piney-game's suite,
  release.
- fmt.

**Still unknown:** nothing
