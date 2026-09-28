---
number: 39
title: Fields in the viewer
date: 2026-09-23
area: world, render, test
files: crates/piney-viewer/src/mesh.rs, crates/piney-viewer/src/main.rs
---

# 39. Fields in the viewer

With the draw rules of [[38]] in `Field::scene`, the viewer shows fields:
`piney-viewer --field SEED[,TYPE[,WEATHER]]`.

`mesh::build_field` assembles what the scene lists:
- **Ground and cover.** Each visible chip's ground tile, and each cover, is a
  copy of its template model with the listed vertices' z and colour
  rewritten, placed by its translation.
- **Objects.** Each object's clump or animation stands at the game's z, and
  animated objects play ([[35]]).
- **Object colours.** Before anything is emitted, the rigid models of the
  type's object clumps are lit by `object_colours`, as many times as
  `lit_clumps` lists each clump. This is what `WORLD::Init` does to the
  loaded models.
- **Water.** On lake fields, the `field_eff` water surface is drawn.
- **Fog.** The fog row of the type and weather becomes the viewer's fog
  environment (F toggles it).

The whole 80 x 80 map is drawn at once. The game draws the 12 x 12 chips
around the player on a torus.

The background (sky, clouds, mountains) is built too, but the viewer leaves
it out. The game centres it on the player. Placed at the map's middle and
seen from the viewer's camera above the field, the sky dome covers
everything. `build_field` still takes a flag for it, and the test builds it
for every type.

In the window, Left/Right (L1/R1) change the weather, A (Triangle) the
field type, skipping type 4, which has no field, and M (Square) the seed,
by the game's own RNG.

## Checked

Shots of three fields look right:
- seed 12345, type 0: volcanic ground, `field_a`;
- seed 777, type 2: rolling grassland with a lake, `field_c`;
- seed 777, type 5, weather 3: snow, `field_g`.

Objects stand on the ground rather than floating or sinking, which the
bilinear guess of [[33]] did not promise.

`every_field_type_builds` generates and assembles a field of each of the ten
types with its background and water. Every field has more than 10,000
triangles, all vertices finite, all indices in range, and every object's z
within the ground's range.

**Still unknown:**
- **Viewer approximations.** The draw window and fade, the water's scroll and
  run-time textures, the sky's scroll, and a player-centred background are
  not reproduced.
- **Fog.** The fog here uses the viewer's depth mapping, not a traced GS fog.
