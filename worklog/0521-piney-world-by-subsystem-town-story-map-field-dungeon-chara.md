---
number: 521
title: piney-world by subsystem: town, story_map, field, dungeon, chara, camera, event, grunty and sky folders, and the story maps and towns as registries
date: 2026-10-30
area: world, build
files: crates/piney-world/src/, crates/piney-world/examples/, plans/rust-cleanup.md
started: 2026-10-30T09:12:00-06:00
took: 57m
---

# 521. piney-world by subsystem: town, story_map, field, dungeon, chara, camera, event, grunty and sky folders, and the story maps and towns as registries

Section 2's module-tree item of `plans/rust-cleanup.md`, for piney-world.
Its 60 files at the root become folders by subsystem, and the two
hand-written dispatch matches, `story_map::build` and `Town::open_with`,
become tables of definitions.

## The tree

Every file moved with `git mv`. The root keeps the shared core: `lib`,
`ee`, `mt`, `hit`, `draw`, `area`, `map/`, `combat/`.

| folder | what moved into it |
| --- | --- |
| `town/` | `town.rs` as `mod.rs`; `mac_anu` (town01), `dun_loireag` (02), `carmina_gadelica` (03), `fort_ouph` (04), `lia_fail` (05), `relic_city` (06); `pc` (rtownpc), `merchant`, `party` (town_party), `ride` (town_ride), `dog` |
| `story_map/` | `story_map.rs` as `mod.rs`; `holy_ground` (evarea, EVENTAREA02), `giant` (07), `boss_arena` (B0), `disc` (B8), `ending`, `sky` (04-06), `truths_core` (01, area 13, "Pulsating Truth's Core"), `abyss` (03, area 43, "Soaring Sky Bounded Abyss") |
| `field/` | a new `mod.rs`; `area` (field_area), `ambient`, `firefly` (FIREFLY), `npcs`, `world` (field_world, with `world/ride.rs`) |
| `dungeon/` | `dungeon_area.rs` as `mod.rs`, with `dress`, `lake`, `leaves` |
| `chara/` | a new `mod.rs`; `char`, `kite` (chara.rs: Kite drawn), `body`, `pose`, `motion`, `arms`, `player`, `party`, `ai`, `navi`, `foe`, `npc` |
| `camera/` | `camera.rs` as `mod.rs`; `evcam`, `bosscam`, `cinema` |
| `event/` | `event.rs` as `mod.rs`; `entry`, `talk`, `gate`, `gameover` |
| `grunty/` | `grunty.rs` as `mod.rs`; `race` |
| `sky/` | a new `mod.rs`; `cloud`, `lensflare`, `lattice`, `firefly` (FIREFLY2) |

Names:

- **The game's classes.** Each module keeps its class in its doc, and
  its `pub mod` line carries `#[doc(alias = "...")]` (`ROOTTOWN01`,
  `EVENTAREAB0`, `ccRtownPC`, `ccBossCam` and so on), so a search by the
  game's name finds it.
- **The sixth town** is `relic_city`, named for its loading card.
  `townNameTbl`'s sixth row is Lia Fail's words again ("Relic City",
  "Lia Fail"), and `lia_fail` was taken.
- **Area 13's map** is `truths_core`, not `core`, which would shadow
  the `core` crate in paths.
- **Three probes** follow their modules: `story_map_probe` (was
  evarea_probe), `dun_loireag_probe` (town02_probe) and
  `carmina_gadelica_probe` (town03_probe). Their harnesses, the docs and
  the source paths in docs, tools and `UNKNOWNS.md` were updated.

The paths were rewritten by a script: `crate::old` in piney-world and
`piney_world::old` everywhere. Bare uses after a `use` of a renamed
module were rewritten too. lib.rs imports `event::{entry, gate, talk}`
and the rest by name, so its bodies keep their short paths.

## The registries

**Story maps.** `story_map::MapEnv` is `GO(1)`'s one constructor input,
which the field world fills once. It holds what the old `At` held (the
volume, `game.field`, `areaPrev`, `fieldPrev`, the server, the save,
`eventStartPos`, `fieldrand`, the scrolls and statics), plus the
archive, `DEF_SE`, the kept map and `ccRand`.

A `MapDef { class, claims, make }` row stands for each class, and
`static MAPS` lists them in the order the old match tried them. `build`
takes the first row that claims the field; a model of 0 is a generated
field, as before.

Each class's quirks moved into its own module's `DEF`:

- Quarantine's last-stage arena is in `boss_arena::DEF`.
- Area 15's kept map is in `holy_ground::DEF`. It hands back
  `Made::Kept`, which does not take gcmn's scrolls, where every other
  map hands back `Made::New`, which does.
- Quarantine-only field 126 is in `ending::DEF`'s `claims`.
- `ccRand` for the disc's rocks is in `disc::DEF`.

The `impl StoryMap` blocks for `EventArea`, `Arena` and `Giant` moved
out of `story_map/mod.rs` into their modules, so a new map is one
module and one row.

**Towns.** `town::TownEnv { archive, volume, crisis, rng }` is the one
input. Every town's `new` now takes it; Lia Fail's took `rng` as a third
argument before. A `TownDef { no, class, volumes, make }` row is each
module's `DEF`, `static TOWNS` lists the six, and `Town::open_with` finds
the one for `game.town` on the disc. As before, town 5 exists only on
Quarantine, and an unknown town is a `NotFound`.

## Checks

- The workspace's tests, release, including piney-game's suite.
- clippy with all features, fmt.
- On Infection, every harness whose probes are piney-world's: evcam,
  evchar, evarea (story maps), dust_ctrl, cinema, merchcam, dungeon_rt,
  map, field_rt, camera_shake, bosscam, grunty, field_ambient, gamectrl,
  town02, town03, world, foe, race, party, lights and gate_out.
- The registry-dependent ones on Quarantine too: evarea, town03, world,
  map, field_rt and dungeon_rt pass. `test_race_rs` passes on Mutation;
  it skips on Infection, which has no race.
- `test_town02_rs` passes on Infection only.
  - On Mutation it fails its own set-up check of ROOTTOWN02's vtable at
    `+0x1AC`: the object's layout is Infection's in the harness.
  - On Quarantine the carry names neither that vtable nor the
    `u$1777` static.
  - That is the harness, not this change: the probe passes wherever the
    harness can run.
- Docs and tools that named `piney_world::<old module>` or
  `crates/piney-world/src/<old file>` now name the new ones (worklog
  entries keep theirs).

**Still unknown:** test_town02_rs runs on Infection only (its ROOTTOWN02 layout and statics are Infection's); making it run on the later volumes is open
