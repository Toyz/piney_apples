---
number: 130
title: "The field's weather: rain, snow, fireflies and the rest of WORLD::Draw"
date: 2026-09-25
area: world, render, test
files: crates/piney-world/src/field_ambient.rs, crates/piney-world/src/field_firefly.rs, crates/piney-world/src/firefly.rs, crates/piney-world/src/field_area.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-world/examples/ambient_probe.rs, crates/piney-data/src/anim.rs, crates/piney-data/src/field/render.rs, crates/piney-data/src/field/mod.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/fx/ambient.rs, crates/piney-game/src/area.rs, crates/piney-game/src/main.rs, tools/test_field_ambient_rs.py, docs/engine/field.md, docs/engine/field-walk.md
---

# 130. The field's weather: rain, snow, fireflies and the rest of WORLD::Draw

The port's fields drew their ground, objects, lake and sky, and nothing
else `WORLD::Draw` does: no rain, no snow, no fireflies at night, no steam
over type 0's lava, none of the servers' travellers crossing the sky. This
entry ports all of it, checked against the game frame by frame, and draws
it in the field.

## What the game draws

`WORLD::Init` and `Generate` make the pieces and `Draw` / `DrawEffect`
step and draw them every frame, all from `fieldrand` except type 7's
fires, which turn with `ccRand`. [field.md](../docs/engine/field.md),
"Weather and ambient pictures", has the whole listing. In short:

- **Snow.** 100 or 200 `SNOW`s falling round `WORLD_MAN`'s centre (types 5
  and 6); type 0's 16 embers rise from the ground.
- **Rain.** 50 drops with their splashes. In a storm there are also bolts
  on the horizon, and a flash of the screen with thunder every 900-1,499
  frames.
- **Smoke.** Type 7's fires smoke on even frames. Type 0's vents steam in
  bursts of eight.
- **The 2D smoke.** Types 2, 3, 5 and 6 drift big translucent puffs of
  `TEX_sfsmo1` across the screen for a few seconds now and then.
- **The heat haze.** Types 0-3 draw a model ahead of the player whose
  texture is the picture drawn so far, so the horizon shimmers.
- **Fireflies.** Five `FIREFLY`s at night, or glyph trails when hacked.
  Type 9 has fifteen `FIREFLY2`s instead.
- **TOBJ.** One field in seven gets the server's traveller.
- **BIRD.** A hawk over the dungeon entrance by day in type 10.
- **The lens flare.**

## The game's slips

Each of these is kept, because the random draws depend on them.

- **Rain placement.** `DrawRain` sets `WORLD` +0x1c where it meant the
  drop's own "placed" flag, so all 50 drops are placed again every frame
  (100 draws).
- **Snow height.** A flake's height asks `GetHeight(x, y != -500)`.
- **Snow at Init.** The flakes made in `Init` ask for heights before the
  map exists, which is the heap's garbage in the game (zeros in eemu and
  here). They also fall round a centre `GO` never set.
- **BIRD's far test.** It adds `d.y` twice where it meant `d.y` squared.
- **The 2D smoke's transparency.** It is an int, so the fade out takes a
  single frame.
- **The bolt's offset.** A negative offset is converted as unsigned.
- **Type 9's `FIREFLY2`.** It takes its height at the offset instead of the
  place.

## The port

- **`piney_world::field_ambient`.** Makes the pieces with the field. Each
  frame it hands back the pictures as a list of `Op`s in the game's order:
  sprites, smoke puffs, masks, models, sounds and the flash.
- **`field_firefly`.** `FIREFLY` (its splines are `FRAC2`'s natural
  spline).
- **`firefly`.** Gains type 9's `FIREFLY2` path.
- **`FieldArea`.** Carries `fieldrand` on from `Generate`, keeps the
  drawing's function statics across fields, and draws the haze, TOBJ, BIRD
  and the 2D smoke.
- **`AreaFx`.** A new `FieldFx::field_ambient` draws the `ccEff`s, each
  made once as its `Init` left it. The puffs queue for `effSmoke`, and the
  sounds and flash go out with the area's events.

Three things came to light while wiring it up.

- **The haze's model.** `OBJ_sfzair00` is an ExtObj. Its model is found
  through the target, and the posed object is picked by that model, not by
  name.
- **TOBJ's "fog" is a shadow.** The two `ccDrawEnv` fields TOBJ sets are
  read by `ccShadowModel::Draw` (main 0x001412b0) as the shadow's length and
  transparency. The fog lives elsewhere (+0xb8..+0xd8, `SetFog`).
- **Where the fires come from.** They are set by `SetSpecialObj` in
  `EntryGimmick`, which the port's fields don't have yet. The runtime
  finds them as the field is made: each chip object's `OBJ_roc1fire` and
  `OBJ_roc3fire` node in its animation, plus the object's `wp`. The
  nodes are matched by name, because the file holds two objects of each
  name. The smoke rises from the braziers' tops in the shot.

`piney-game --mode field:14/TYPE,ROW[,HACK[,SEED]]` puts another field's
type, weather row, hack flag and seed into story area 14's `WORLD_MAN`, for
looking at any weather.

## Checked

- **`tools/test_field_ambient_rs.py` (new).** It runs the game's `Init`,
  `Generate` and `Draw` in eemu against the `ambient_probe` example, over
  every field type and background row, hacked and not, on a random server
  and seed, with the player and camera walking. It compares `fieldrand`'s
  counts, everything `Init` and `Generate` made, and every frame's
  pictures in order along with the `fieldrand` and `ccRand` counts. It
  ran 124 cases of 1,000 frames with 0 mismatches. Every snow, rain,
  storm, flare, firefly and ember sprite showed up, as did the masks,
  puffs, sounds, flash, bird, haze and TOBJ.
- **Shots.** Rain, a storm, heavy and light snow, night with fireflies,
  type 9 at night, the bird, type 7's fires, type 0's steam and haze,
  hacked, and the 2D smoke.
- The workspace's tests, clippy, fmt, and `test_evarea_rs` for the shared
  `FIREFLY2` all pass.

**Still unknown:**
- **`EntryGimmick` in a field.** It isn't ported. In the game its draws
  (`SetFood`, `SetMagicCircle`, `SetSpecialObj`) come after the weather's
  first frames, so from then on the port's weather draws are offset from
  the game's by them.
- **The depth shades.** `WORLD_MAN`'s two `ccBufferSampling` depth shades
  need the z buffer as a texture, which the renderer lacks.
- **TOBJ's shadow and sounds.** Neither is drawn or played.
- **The smoke puffs.** They start a frame late, at the next effect task.
- **Balloon and Ω's runners.** The balloon's cloth scroll and Ω's runners
  aren't seen in a shot, since the story's server is Δ.
- **The lens flare.** It is left above the picture by the follow camera,
  and the eye view fails its camera test at the player's own place. No
  shot shows it.
