---
number: 261
title: Root Towns as a trait
date: 2026-09-27
area: world, build
files: crates/piney-world/src/town.rs, crates/piney-world/src/town01.rs, crates/piney-world/src/town02.rs, crates/piney-world/src/town03.rs, crates/piney-world/src/town04.rs, crates/piney-world/src/town05.rs
---

# 261. Root Towns as a trait

The five Root Towns are now five types behind one trait. Before this, the
port had grown one town at a time from Mac Anu:

- `Town` held the shared fields and an `enum Class` with a variant per
  town.
- `Town::select` and `Town::draw` matched on the variant and called free
  functions in `town02.rs` .. `town05.rs`. Each of those took the whole
  `Town` and got its own state back with `let Class::X(d) = &t.class else
  { return }`, so a check at run time stood in for a type the compiler
  could have known.
- There were five `Option` accessors (`mac_anu()`, `dun_loireag()`, ...).
- One shared `Piece` enum carried every town's variants: the airship, the
  glows, the flares. Every town's draw had to ignore the others' variants.
- Mac Anu's own code sat in `town.rs` beside the shared code.

The owner pointed out that this was not how Rust is written. The shape now
follows the game's own classes, where each `ROOTTOWN0N` overrides a virtual
`Draw`:

| type | what it is |
| --- | --- |
| `town::Base` | What every constructor builds from its `Spec` (number, tables, light animation, fog, clear colour): the file, the static models and objects, the lights, the fog, the hits, the events. The shared helpers are its methods: `rows(pass)`, `step_objects(pass, eye)`, `row_draw`, `object_draw`, `clump_draw`. |
| `town::RootTown` | The trait: `draw(&mut self, base, layers, to_screen, view)`, and `water_time` and `water0` for the towns with water. |
| `town01::MacAnu` .. `town05::LiaFail` | One per class, each in its own module. Each has its own `Piece` enum and an inherent `select(&mut self, base, view)`, which is `Draw`'s choice of pieces, the thing the eemu harnesses compare. |
| `town::Town` | `{ pub base: Base, class: Box<dyn RootTown> }`, with `open(no)`, `draw`, and the generic `class::<T>()` and `parts_mut::<T>()` (downcasts through the trait's `Any` supertrait) in place of the five accessors. |

Two other shapes were weighed and turned down:
- **Keep the enum and add a trait on each variant** (`enum_dispatch`
  style). The set of towns is closed, so an enum would be fair. But the
  per-town piece types and the split borrow of base and class read more
  plainly as trait objects, and the game's own shape is virtual dispatch.
- **Keep the shared fields directly on `Town`.** The class's methods need
  `&mut` of the shared part and of themselves together. With both in one
  struct, `self.class.draw(self)` cannot borrow. The other way round,
  taking the class out and putting it back, is a hack. So the shared part
  became its own `Base`. That costs about 45 outside reads going through
  `.base` (`town.base.hits`, `town.base.file`).

Nothing drawn changed:
- The eemu comparisons of the game's own `Draw` against the port still
  match: `tools/test_world_rs.py` (`test_town_draw`, `test_gate`,
  `test_merchants`), `tools/test_town02_rs.py` and
  `tools/test_town03_rs.py`.
- Each town's own tests pass (`town01::tests::gate_plaza_pieces`,
  `town02::tests::first_draw`, `town03::tests::the_airship_flies_its_first_leg`,
  `town04::tests::first_draw`, `town05::tests::a_frame`), and so does the
  whole workspace (705).

**Still unknown:** nothing.
