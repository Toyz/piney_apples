---
number: 518
title: Clean-up lift: the Table trait, one table cache indexed by generated IDs, and no accessors or LazyLock statics
date: 2026-10-30
area: build, tooling
files: crates/piney-data/src/store.rs, crates/piney-gen/src/render.rs, crates/piney-data/src/tables/, crates/piney-data/src/area/mod.rs, crates/piney-data/src/dungeon/mod.rs, crates/piney-data/src/field/mod.rs, crates/piney-data/src/sound/mod.rs, crates/piney-data/src/statics/mod.rs, crates/piney-audio/src/setbl.rs, plans/rust-cleanup.md
started: 2026-10-29T23:25:55-06:00
took: 1h 9m
---

# 518. Clean-up lift: the Table trait, one table cache indexed by generated IDs, and no accessors or LazyLock statics

Section 2's first lift of `plans/rust-cleanup.md`: every table group is a
`store::Table`, read through one cache, and the generated modules lose
their accessor methods, their `LazyLock` statics and their per-module
caches.

## The trait and the cache

`piney_data::store::Table: Load + Send + Sync + 'static`:

- `const ID: usize`, the group's place in the store;
- `of(v)`, volume `v`'s values, read once a run;
- `shared()`, the values every volume has alike: those of the first
  volume with a disc registered, else of the first whose tools' copy is
  in `work/`, else Infection's, fixed at the first call (as each
  module's own `shared()` did).

The cache is one array, `[[OnceLock<&'static (dyn Any + Send + Sync)>;
5]; GROUPS]`: a slot per volume and one for the shared values, indexed
by `T::ID`. There is no lock and no hash, so a read once made is one
atomic load and a downcast, as the per-module `OnceLock`s were. A
`HashMap` under a lock was drafted first and dropped for this: the
fieldui and the world each call `of()` about a hundred times a frame.

The IDs come from one list. `piney-gen` writes it into
`tables/mod.rs`: `id::TITLE` .. `id::RACE` for the built groups in the
manifest's order, then the placement groups (`AREA`, `DUNGEON`, `FIELD`,
`SETBL`, `SOUND`, `STATICS`), with `GROUPS` and `NAMES` (each group's
file, `<name>.bin`, by its ID). A test checks that the 29 types' IDs are
the numbers 0 to 28, each once.

## The generated modules

Each built group's module now has:

- `impl crate::store::Table for G { const ID: usize = super::id::G; }`;
- a one-line `of(v)` and a one-line `shared()`, both through the trait.
  `shared()` is public, so the 23 `#[allow(dead_code)]` on it are gone;
- the struct's fields, read directly. The 1,005 accessor methods
  (`.kite_pos()`) are gone, and so are the 598 `LazyLock` statics
  (`combat::PLAYER_ANIMS`), which callers now read as
  `combat::shared().player_anims`. A field alike on every volume says so
  in its doc ("Alike on every volume."), which was what a static marked.

The two groups still rendered as Rust literals, `registry` and
`stfroll`, are not read from the build and keep their 12 accessors
until the plan's piney-gen item puts them there.

The hand-written groups implement the trait: `field`, `dungeon` and
`sound` (`tables_of` is one line), `statics` (`Statics`, now
`pub(crate)` for the ID test), `area` (`AreaTables::of` is the trait's,
so its callers import `Table`) and piney-audio's `setbl`.

## The callers

The callers were rewritten from the compiler's own diagnostics: a
script over `cargo check --message-format=json` dropped the `()` where
rustc said "field, not a method", turned `m::STATIC` into
`m::shared().static`, dropped a `*` where rustc suggested it, and
imported `Table` where `AreaTables::of` needed it. That took eight
rounds. The rest was done by hand: the `SPRITE_COLOR_TABLE` imports
(`kanji::shared().sprite_color_table`), voice's test, and three `*`s in
array literals.

The diff is 1,954 lines added and 6,836 removed: the tables 806 and
5,809, the rest 1,148 and 1,027.

## dead_code

There were 36 allows: the 23 generated ones and 13 by hand. One of the
13 is a `cfg_attr` in `dtmenu.rs`, and the other 12 are piney-game's
test-only accessors and `book/pages.rs`. A grep counting 37 also took in
the generator's template line (`render.rs`) that emitted the generated
ones. The plan's "26 generated" was 23. 13 remain, for section 1's
dead_code item.

## Checks

- `piney-gen gen --check`: every table's `.bin` and every generated
  `.rs` is current.
- The whole workspace's tests, release: 1,090 passed, none failed;
  piney-game's suite 339.
- clippy with all features, fmt.
- The harnesses whose probes changed, on Infection and Outbreak:
  `test_area_rs`, `test_race_rs`, `test_map_rs`, `test_evarea_rs`,
  `test_effect_spell_rs` and `test_effect_skill_rs` pass on both;
  `test_evcam_rs`, `test_evchar_rs` and `test_demo_rs` pass on
  Infection. On Outbreak those three fail the same tests at f294376,
  before this lift, as after it: evcam 5 failures, evchar errors in the
  harness's own set-up (a symbol it looks up is `None`), demo 3 failures
  and 2 errors (among them `LogoMain` playing Infection's `OPENING.PSS`
  where Outbreak's code plays `OPENING3.PSS`). They are not from this
  change.

**Still unknown:** whether the Outbreak failures of test_evcam_rs, test_evchar_rs and test_demo_rs (the same before this lift) are the port's or the harnesses' (Infection-only set-ups); demo's LogoMain picking OPENING.PSS on Outbreak looks like the port's
