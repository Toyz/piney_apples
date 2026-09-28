---
number: 33
title: The field generator ported to Rust, heights bit for bit
date: 2026-09-23
area: world, build, test
files: tools/field_tables.py, tools/test_field_rs.py, crates/piney-data/src/field, crates/piney-data/tests/field.rs, crates/piney-data/examples/field_snapshot.rs, docs/engine/field.md
---

# 33. The field generator ported to Rust, heights bit for bit

[[19]] reproduced `WORLD::Generate` in `tools/field.py`; [[25]] checked it
on every volume. This entry carries it into the port as `piney_data::field`,
the same way [[30]] carried the dungeon generator. A helper agent did the
port; I re-ran its checks.

## The tables

`tools/field_tables.py rust ELF OUT` writes
`crates/piney-data/src/field/inf.rs` (21 KB, exempt from rustfmt, with
`--check`). It holds:
- **The object tables.** The six tables, each row array written once: 55
  arrays, 168 rows.
- **Models.** The field CCS file per type (`field_a` ... `field_p`; `WORLD`'s
  constructor sets `isHacked = 2`, which picks that set over `field_a1`),
  and the ground and cover model names.
- **Draw counts.** `WORLD::Init`'s count per field type and weather, taken
  from `field.py`, which measured them in eemu.
- **Constants from `Generate`'s code.** Each is checked against `field.py`:
  - the story areas without an entrance;
  - the lake types and the tree-row type;
  - the object counts 20/30/35/35, the per-type divisions and the
    0.8/0.9/1.0 factors;
  - the hill counts and sizes.
- **Also:** the lake water animation (`field_eff` / `ANM_sfwat1_1a`), and
  which square root the executable's `ccGetDist` uses (newlib's `sqrtf` in
  Infection).

## Typed rows

The user asked why a row's `type` was an `i32`. The rows now carry:
- `ObjectClass { Key, Sub, Base, Enter }` for `type`;
- a `Flat` newtype for `flat`, with the `LEVEL`, `LAKE` and `SLOPE`
  bits;
- `rotflag` as a `u8`, because nothing gives it a meaning.

Tabulating them corrected the old comment: lake rows are 1 (sub), not 0.
Tree rows are 2 (base), and three of `BaseObjTBL_I`'s rows are 1.

The agent then looked for readers.
- **`type` and `rotflag`: none.** The rows are reached only through the six
  tables. Only `WORLD::Init` and `Generate`'s setters use those, and they
  read the names, the size and `flat`.
- **Rotation.** `FOBJECT` and `FOBJECT2` keep no pointer to their row, and
  their constructors zero the rotation. So no field object is rotated,
  whatever `rotflag` says.
- **`flat`.** Every row sets exactly one bit (118 / 5 / 45 rows for 0x2 /
  0x4 / 0x8). The code tests them as bits, so the type keeps them as bits.

## The port

`generate(&INF, &Params)` returns a `Field`, and `ee` does the float work.
- **Params:** the seed, field type, weather, ground, object setting, story
  area and protect flag.
- **Field:** the height map as the game's float bits, the four chip grids,
  hills, objects, cover tiles, entrance, start, the draw count and the
  final RNG.
- **`ee`:** the FPU operations on bit patterns (truncation, no denormals, no
  infinities, `madd` rounding its product first). It never uses the host's
  IEEE arithmetic.

For a viewer, `vertex(x, y)` gives the ground in world units, and
`placements(&INF)` gives each object's CCS file, animation or clump, and
position.

## Checked

`tools/test_field_rs.py` builds `examples/field_snapshot` and compares it
with `field.py` field by field. The height maps are compared bit for bit.
- **Story areas.** 104: every typeable story area with a field. That is 112,
  less 7 of type 4, less event 67, which shares 66's words. 10,500 objects
  and 155,113 cover tiles, with 0 mismatches.
- **Random fields.** 220, covering every field type, weather, ground and
  object setting; 48 carry a story area number and 17 have no entrance.
  0 mismatches.
- **The FPU.** 36,000 EE operations against `eemu.py`'s, on random and edge
  operands, all equal.
- **The comparison bites.** With `ee`'s add swapped for ordinary `f32`
  addition, all 220 random fields differ. The EE model is needed, and the
  test notices when it is gone.

`tests/field.rs` pins one story area and three random fields to numbers
from `field.py`, including a hash of the height map. It also checks on the
disc that all 359 object, ground and cover names exist in their field's CCS
file.

On integration:
- `test_field_rs.py`: 4 tests OK.
- `field_tables.py rust --check`: current.
- `cargo test -p piney-data --lib --test field`: 17 and 5 passed.
- clippy: clean.

**Still unknown:**
- **Object heights.** The game takes z from `WORLD::GetHeight`, a collision
  query that is not ported. `placements` uses 0 for levelled objects and
  otherwise estimates z bilinearly from the height map.
- **Drawing.** How the ground mesh is built and where the cover is drawn.
- **After the start position.** Anything rolled then, plus the weather
  effects and `EntryGimmick`'s circles and treasure.
- **Other volumes and the area generator.** The other volumes' tables, and
  the keyword-to-seed step, which is not ported.
- **Endless loops.** Where the game would loop forever, the port gives up
  after a cap with `NoSite`. No case has reached it.
