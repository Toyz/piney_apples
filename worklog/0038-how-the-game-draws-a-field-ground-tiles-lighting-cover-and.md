---
number: 38
title: How the game draws a field: ground tiles, lighting, cover and object heights
date: 2026-09-23
area: world, render, test
files: tools/field.py, tools/test_field.py, tools/field_tables.py, tools/test_field_rs.py, crates/piney-data/src/field, docs/engine/field.md
---

# 38. How the game draws a field: ground tiles, lighting, cover and object heights

[[33]] ported the field generator, but a viewer could not show a field yet.
Two things were missing:
- how the height map becomes ground;
- where objects stand in z (the port estimated it bilinearly).

The user asked for fields in the viewer. I sent the field agent back to read
the draw code instead of guessing, after the town placement lesson of [[27]].
It extended `tools/field.py` and the Rust, and I re-ran its checks. What is
true now is on [the field page](../docs/engine/field.md#drawing).

## What it found

- **The ground is a template.**
  - Every chip draws one copy of the type's 24-vertex `BaseMeshName` model,
    translated only.
  - `SetMESH2` rewrites only each vertex's z and colour, from the height and
    lit colour of the cell under it.
  - Which vertex goes to which cell was read by running `SetMESH2`. All ten
    types' ground models put their vertices exactly on those cells.
  - Chips under levelled objects and the lake draw nothing.
- **The ground's colour is lighting, baked in.**
  - The game builds per-cell face normals and averages six per vertex. One of
    the six is the wrong neighbour, `quad[x+1][y-1]`; the port keeps the bug,
    and a deliberate fix of it fails the comparison.
  - Each vertex is lit with the area's background light, from `BGTBL` and the
    `bg_*` file's frame-0 `F_Ambient` and `F_DistantLight`.
  - The same pass lights the object models in memory, so a clump that appears
    twice in a table is lit twice.
- **Cover** is a 6-vertex template given the absolute heights of its cells,
  2.5 units above the ground.
- **Object heights are a collision query.** `WORLD::GetHeight` casts a
  segment into the cell's two triangles, made at the moment each object is
  placed. `SetCover` calls it for every cover, so the game's own `Generate`
  in eemu now spends most of its time there. The Python suite went from 95
  to 167 seconds.
- **Read from code, not run:**
  - the torus and draw window: 12 × 12 chips around the player, fading from
    6,700 to 7,200;
  - the lake's three water copies;
  - the background layers.

## Checked

`tools/test_field.py` runs the game's functions in eemu. All match, most bit
for bit:
- object z inside `Generate`: 5 Infection fields, plus 3 on each other
  volume;
- `GetHeight` at 178 points: on borders and on hidden chips;
- the whole map's normals and colours under one light: 6,400 of each;
- 20 ground tiles, 16 covers and 43 object vertex colours.

On integration:
- `test_field.py`: 8 tests OK (from the agent's run; its log is complete).
- `test_field_rs.py`: 7 tests, 0 mismatches. This covers object z over 220
  random fields and 104 story areas, and 4 fields drawn in full: 6,000 tiles,
  6,000 covers and 25,600 normals and colours.
- `field_tables.py --check`: current.
- `cargo test -p piney-data --lib --test field`: 20 and 9 passed.
- clippy: clean.

The agent's wait loops polled `pgrep -f` on a pattern that matched their own
command line, so they could never finish. I stopped them; the test they
waited for had passed.

**Still unknown:**
- **Fog.** How the fog table's near, far and percentage map to the GS fog.
- **Water.** The water's run-time texture and scroll.
- **Objects.** The objects' own draw code, and whether ExtObj-shared object
  models are lit once or per use.
- **Draw window.** Whether alpha is clamped past 7,200.
- **Hardware rounding.** Whether VU0 hardware rounds as the modelled
  truncation does.
