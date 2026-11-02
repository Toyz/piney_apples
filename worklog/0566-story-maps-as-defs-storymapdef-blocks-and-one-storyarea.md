---
number: 566
title: Story maps as defs: StoryMapDef, blocks and one StoryArea, every EVENTAREA made of systems
date: 2026-11-02
area: world
files: crates/piney-data/src/classes.rs, crates/piney-game/src/area.rs, crates/piney-game/src/main.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/area43.rs, crates/piney-game/src/session/tests/ending.rs, crates/piney-game/src/session/tests/kyvia_stage.rs, crates/piney-game/src/session/tests/mirror_world.rs, crates/piney-gen/src/classes.rs, crates/piney-world/examples/ambient_probe.rs, crates/piney-world/examples/dun_loireag_probe.rs, crates/piney-world/examples/story_map_probe.rs, crates/piney-world/src/field/ambient.rs, crates/piney-world/src/field/area.rs, crates/piney-world/src/field/world.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/sky/firefly.rs, crates/piney-world/src/sky/lensflare.rs, crates/piney-world/src/story_map/abyss.rs, crates/piney-world/src/story_map/boss_arena.rs, crates/piney-world/src/story_map/disc.rs, crates/piney-world/src/story_map/ending.rs, crates/piney-world/src/story_map/giant.rs, crates/piney-world/src/story_map/holy_ground.rs, crates/piney-world/src/story_map/mod.rs, crates/piney-world/src/story_map/sky.rs, crates/piney-world/src/story_map/truths_core.rs, crates/piney-world/src/systems/anim.rs, crates/piney-world/src/systems/bob.rs, crates/piney-world/src/systems/clouds.rs, crates/piney-world/src/systems/fireflies.rs, crates/piney-world/src/systems/flare.rs, crates/piney-world/src/systems/mod.rs, crates/piney-world/src/systems/scrolls.rs, crates/piney-world/src/systems/sky.rs, crates/piney-world/src/systems/statics.rs, crates/piney-world/src/systems/water.rs, crates/piney-world/src/town/carmina_gadelica.rs, crates/piney-world/src/town/dun_loireag.rs, crates/piney-world/src/town/fort_ouph.rs, crates/piney-world/src/town/lia_fail.rs, crates/piney-world/src/town/mac_anu.rs, crates/piney-world/src/town/mod.rs, crates/piney-world/src/town/relic_city.rs, docs/engine/evarea.md, docs/engine/events.md, docs/engine/field-walk.md, docs/engine/statics.md, plans/mutation-story.md, plans/rust-cleanup.md
started: 2026-11-02T13:34:13-07:00
took: 52m
---

# 566. Story maps as defs: StoryMapDef, blocks and one StoryArea, every EVENTAREA made of systems

Section 2's map systems item (`plans/rust-cleanup.md`): the story maps
were eight types behind a `StoryMap` trait, each with its own `select`
and `Piece` enum, its own copies of the clouds, the lens flare and the
scrolls, and the world reached into them by downcast (`story::<Arena>`,
`DiscArea::next_stage`, `EventArea` kept by type). Now each `EVENTAREA`
class is a `StoryMapDef` and one `StoryArea` holds whichever `GO(1)`
made, as `TownDef` and `Town` (worklog 560). Nothing drawn changed.

## The def

`story_map::StoryMapDef`, one `static` a map, each `BASE_STORY_MAP` with
its differences: `class` (`StoryMapGameClass`, generated from `GameClass`
beside the bosses' and towns'), the `fields` and `volumes` `GO(1)` makes
it for (`StoryMapDef::of` is its switch), its stream, whose tables it
reads, the stage its class's names are written for (`EVENTAREAB0`'s
`se1_5`, read as each stage's), the clear colour, how the party stands
(`Spread`), its doors (`[Door; 2]`: from block 0 or -1, from any other),
its `blocks`, the constructor's own start after the first block, a save
byte that starts it at another block, its own systems, and whether
`WORLD_MAN::Quit` keeps it or the boss centres on its start.

A `BlockDef` is what `ChangeBlock` makes (or the constructor, for a class
of one): the file its names are in (the giant's `se1_7_2`), its light
animation and table (`LightsDef`), `SetFog`, its start (`Start`: kept, a
point, a marker with the class's own lanes over it, by `game.areaPrev`),
its systems and its `Draw`'s steps, awake and through a frame the tasks
sleep. The block's systems come first in the map's list, then the
map's own, which outlive the block (area 15's and the giant's
`EA_moveTex`, the giant's bob, the arenas' fireflies and scroll).

| map | defs | own systems |
| --- | --- | --- |
| `EVENTAREA01` | `AREA_13` | `Flicker` (the glows) |
| `EVENTAREA02` | `HOLY_GROUND`, two blocks | none |
| `EVENTAREA03` | `AREA_43` | `FlyOver` |
| `EVENTAREA04`-`06` | `MIRROR`, `MIRROR_ARENA`, `WHALE` | none |
| `EVENTAREA07` | `GIANT`, two blocks | none |
| `EVENTAREAB0` | `ARENA_1` .. `ARENA_8` (8 has the last stage's block) | `Reflection`, `Bobbing`, `LastStage` |
| `EVENTAREAB8` | `DISC_9` .. `DISC_12` | `Disc`, `Rocks`, `Bodies` |
| the ending's | `ENDING` | `Panels` |

## The differences as data

- **Clouds** (`CloudsDef::made`): `Made::Build`, the constructor's or
  `ChangeBlock`'s about the origin (the giant's, the disc's, never drawn),
  or `Made::FirstDraw` about the player (the towns', the sky maps').
- **Lens flare** (`FlareDef::draw`): `LENSFLARE::Draw` with the view's test,
  or `EVENTAREA02::DrawLensFlare` (`FlareDraw::Own`), the same points
  with only the puppet show's.
- **Scrolls** (`ScrollsDef`): lanes, each a step, a `Wrap` (`Under` for
  the disc's, down and back to 1), whether the materials take a lane
  before or after it steps (`Shown::Before`: `EVENTAREAB0::DrawBG`),
  where they live (`Carry::Statics`, `Carry::MoveTex`: a `MOVETEX` row
  two lanes in gcmn's data), and the materials (`MatScroll`, U and V each
  by a lane). The offsets they write are the file's materials', which
  every model of the file draws with (`DrawCx::rows`); the towns' sky
  materials moved into their `ScrollsDef` and draw the same.
- **Models** (`ModelsDef`): the layer of each pass (`PassLayers`),
  whether the rows take the area's fog, the rows `DrawObj2` draws by
  `DrawWithOutFog` (area 15's third: no fog, within its clip of the
  player) and the rows `ChangeBlock` puts at the bob (`ride`).
  `STATICMODEL` enables each row's Hit chunk as the block is placed.
- **Sky** clumps at the origin, a dummy or the bob (`At::Bob`), with fog
  or not; `systems::bob::Bob` is `EVENTAREA07::DrawBG`'s bob.
- `systems::anim::Anim` is a `ccAnm` a class steps and draws at a root of
  its own (area 15's `revAnm`); `systems::fireflies::Fireflies2` the
  arenas' `FIREFLY2`s.

## Frames the tasks sleep through

The game's `Draw` does not run in them; the port draws one from the
block's sleeping steps on a copy of the map, which is dropped. The
classes drew it two ways, kept as `StoryMapDef::asleep`: `Stepped`, the
copy's pieces as the steps leave it (the sky maps, area 13 without its
objects, the disc's clumps, objects and models as they stand), and
`Standing`, the pieces the copy chose drawn from the map as it stood
(areas 15 and 16, the arenas: their scrolls, animations, bob and clouds
as the last awake frame left them; the flares' and fireflies' pieces
carry their own draws, `Piece::Fly`).

## What the world sees

`Place::Story(Box<StoryArea>)`. The battle's calls on
`WORLD_MAN.eventmap` are `Signal`s (`SwitchLayer`, `StageClump`,
`DiscMove`, `BodyHit`) to whichever system answers; what Kyvia reads of
the disc is the map's `Report`; the boss's centre is
`StoryArea::boss_centre`; area 15 is kept by its def. Gone: the
`StoryMap` trait, `MapDef`, `Made`, `sky::Class` and its `Kind`, each
map's struct, `Piece`, `select` and `draw_pieces`, `MoveTex`,
`clump_models`' second copy, `Names`, `disc_view`,
`FieldWorld::arena_switch_layer` (now `switch_layer`) and the downcasts.
No world or game code names a story map; `FlareCamera` moved to
`sky::lensflare`, `BOUNDS` and `layer` to `story_map`.

## Checks

- Golden shots, every PNG byte for byte: the towns' set and the story
  maps', fields', dungeons' and breaths' (worklog 562, 563). The first
  run caught two slips, both fixed: area 15's church drew its row 2 of
  `DrawObj` without fog (the rule is `DrawObj2`'s alone;
  `event_11_church_shots`, and `test_evarea_rs` likewise), and the arenas
  drew a sleeping frame from the stepped copy (`macha_shots`).
- Before the change, over every story map's clumps, rocks, bodies and
  the last stage on all four volumes, the towns' `clump_models` gives the
  holy ground's copy's models; `EA_MODELTABLEB8`'s floor has no Hit chunk,
  so the disc's list is as it was.
- Against the game on all four volumes, the harnesses of worklog 562
  (`test_evarea_rs`: area 15's and the arenas' pieces and places through
  the rewritten `story_map_probe`).
- The crates' tests, piney-game's suite (release); `piney-gen gen
  --check`, clippy with all features, fmt, `cairns check`, `tools/docs.py
  check`.

**Still unknown:** nothing. Found on the way, for the next entry: the disc's `DrawBG` scroll (MUT main gp 0x0038ba78, its first-use flag at 0x0038ba7c, set to 1.0 by the first `DrawBG`) is a static of the executable, so it carries across visits as the other scroll statics do; the port starts it at 1.0 with each map.
