---
number: 564
title: One sprite request: EffSprite and one ccEff cache for towns, story maps, fields, dungeons and breaths
date: 2026-11-02
area: world
files: crates/piney-game/src/fx.rs, crates/piney-game/src/fx/ambient.rs, crates/piney-game/src/town_fx.rs, crates/piney-world/examples/ambient_probe.rs, crates/piney-world/examples/dungeon_probe.rs, crates/piney-world/src/combat/breath.rs, crates/piney-world/src/dungeon/dress.rs, crates/piney-world/src/field/ambient.rs, crates/piney-world/src/field/area.rs, crates/piney-world/src/field/firefly.rs, crates/piney-world/src/field/world.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/story_map/abyss.rs, crates/piney-world/src/story_map/boss_arena.rs, crates/piney-world/src/story_map/disc.rs, crates/piney-world/src/story_map/ending.rs, crates/piney-world/src/story_map/giant.rs, crates/piney-world/src/story_map/holy_ground.rs, crates/piney-world/src/story_map/mod.rs, crates/piney-world/src/story_map/sky.rs, crates/piney-world/src/story_map/truths_core.rs, crates/piney-world/src/systems/clouds.rs, crates/piney-world/src/systems/flare.rs, crates/piney-world/src/systems/mod.rs, crates/piney-world/src/town/lia_fail.rs, crates/piney-world/src/town/mod.rs, docs/engine/dungeon.md, docs/engine/evarea.md, docs/engine/town02.md, crates/piney-game/src/fx/sprites.rs, crates/piney-world/src/sprite.rs, plans/rust-cleanup.md
---

# 564. One sprite request: EffSprite and one ccEff cache for towns, story maps, fields, dungeons and breaths

The systems audit's "sprite and effect-request system" (`plans/rust-cleanup.md`
section 2). Four types asked the effects for a `ccEff::Draw(pos, pattern)`:
`TownSprite` (the towns' flares, clouds and glows), `StorySprite` (the story
maps'), `breath::Flame` (an enemy's flames, turned into the next) and
`field::ambient::Sprite` (the weather, fireflies, a dungeon's sparks and
glows, a symbol's fire). Three caches made the `ccEff`s: `TownFx::effs`, and
`AreaFx`'s `story` and `ambient`. Nothing drawn changed.

## The request

`piney_world::sprite::EffSprite`: the file, the `ccEff`'s name, the place,
pattern and layer, the scale (two lanes), turn, transparency, the colour
where the code sets it, and an `EffState`: what the sprite's owner did to the
`ccEff` after `Init(chunk, 1)`, each from the game's own code:

| state | what the code did | whose |
| --- | --- | --- |
| `EffState::INIT` | nothing: fogged, depth-tested | Lia Fail's glows, an arena's `FIREFLY2`s, the field's fire roamers |
| `EffState::FLARE` | `SetRenderState(CCRS_ZENABLE, 0)` and PRIM's fog bit cleared | `LENSFLARE`'s and `CLOUD`'s constructors, in towns, story maps and fields |
| `EffState::NO_FOG` | PRIM's fog bit cleared | `EVENTAREA01`'s glows, the symbols' fires, a breath's flames (`Init(chunk, 0)`), a dungeon's lights |
| `EffState::fogged(fog)` | the fog bit as the drawing left it | the weather's `Eff`s, the fields' fireflies |
| `clut: Clut::Set(new)` | `eff->tex.clutChunk = chunk` | a breath |
| `clut: Clut::Change { new, old }` | `ccEff::ChangeClut(new, old)` | a dungeon's sparks (`DUNGEON::ChangeClut`) |

The flags that differed by caller type (`depth`, `flare` and `fog`, `fog`
and `ztest`) are these. A sprite's layer is the field's draw layer; the
ambient code turns its `SetActiveLayer` number into it as it makes the
sprite (`field::area::ambient_layer`; `active_layer` back for the
probe). sysLayer (0) and objLayer (1) are one layer here, so the way back
gives 0: the snow under the hack is the one ambient sprite on either
(`test_field_ambient_rs` caught the first try, which gave 1).

## One cache

`piney_game::fx::sprites::EffSprites`: the `ccEff` of each (file, name,
state), made once as `Init(chunk, 1)` and the state leave it, cloned for
each draw, then the place, scale, turn, transparency and colour set and
`ccEff::Draw` run. `TownFx::draw` and `AreaFx` (a story map's sprites
through `FieldFx::sprites`, the ambient ops' sprites) both draw through it.
The world's `Request::Sprite`, `StoryMap::draw`, `ambient::Op::Sprite` and
`Breaths::flames` carry `EffSprite`; `TownSprite`, `StorySprite`,
`breath::Flame` and `ambient::Sprite` are gone, and so are the three
caches' keys (one of them, worklog 509, once without the file).

## Checks

- Golden shots, every PNG byte for byte: the towns' set (worklog 560, 561)
  and the story maps', fields', dungeons' and breaths' (worklog 562,
  the walk's two re-recorded by worklog 563).
- Against the game on all four volumes, as for worklog 562 (among
  them
  `test_field_ambient_rs`, whose probe prints the sprites' layers and
  names, and `test_dungeon_rt`, whose probe prints the sparks' palette).
- The crates' tests, piney-game's suite (release); `piney-gen gen
  --check`, clippy with all features, fmt, `cairns check`, `tools/docs.py
  check`.

**Still unknown:** nothing.
