# Rust clean-up

A read-only audit of the whole workspace (2026-10-29, tree at 2f4ffdf) for
idiomatic Rust: duplication lifted into traits, tables and shared modules,
owned state instead of hidden globals, no `unsafe`, errors instead of panics,
no lint exemptions; and, after the clean-up, the frame-time suspects. Every
step keeps behaviour bit-exact: the suite, the harnesses against the game's
code, and a byte compare of the generated tables guard each one. Work it in
order; tick items off here as they land, with the worklog entry.

The lifts are not cosmetic. Each one is there to make the code faster, to
make the next boss, map, town, menu or volume one impl or one table row,
and to leave clean seams for the render and sound split (plans/modern.md:
a core crate per system, Classic and Modern backends). So no lift is
skipped, shrunk or worked around because the code has two shapes today or
because the full version is more work: unifying the shapes is the point. If
a lift truly cannot work, say exactly why and ask the maintainer.

## 0. Real bugs the audit turned up

- [x] `field::INF` / `dungeon::INF` / `sound::INF` statics: `piney-world`
      `field_area.rs:277` and `field_ambient.rs:360` read Infection's field
      tables on every volume (Outbreak and Quarantine take `sqrt.s`, not
      newlib `sqrtf`). Delete the statics so every caller names its volume
      (16 sites, the viewer too); check an Outbreak field against the game.
      (worklog 508: the fields were already the game's; the later volumes'
      thunder was not, and is now.)
- [x] `piney-battle` `geom.rs` `get_dirc_chg` divides by `mode & 0xffff`
      with no zero guard (the world's copy has one); gimmick data feeds it.
      (worklog 508: one guarded copy.)
- [x] Boss `affect`s do plain `i16` `hp ± p0` (panics on overflow in debug);
      others use `wrapping_*`. One `hit_hp` computing in `i32` as the game's
      `int` promotion does, checked against the code. (worklog 508: the
      code keeps the sum in a `short`, so `i16` wrapping it is.)
- [x] `piney-game` `fx.rs:201` sprite cache key leaves out the file
      (`town_fx.rs:210` has it): two maps' same-named effects collide.
      (worklog 509; latent: no two cached names collide today.)
- [x] `piney-data` `store::group` races: two threads can load and leak one
      group twice and get different `&'static` pointers. (worklog 509)
- [x] `piney-gen` `die()` calls `process::exit` from library code that the
      game reaches through `piney_build::image_data` (43 sites): `Result`.
      (worklog 510: faults and searches `Result`; the manifest's own
      mistakes `panic!`.)
- [x] `piney-eemu` thread-local `BUS` is never uninstalled: a second `Sys` on
      a thread silently routes through a dead one. Put the bus in `Mem`.
      (worklog 509)
- [x] Process-wide state shared between tests: `field_ambient` `static KEPT:
      Mutex` (carry it in `SaveState` like the scroll statics) and `iso`
      `DENY_EXECUTABLE` (an option on the `Iso`). (worklog 509)
- [x] `piney-event` `PARSED` caches an `Err` forever and clones every event
      per call: `OnceLock<Arc<[Event]>>`, retry on error. (worklog 509)

## 1. Guardrails

- [x] `[workspace.lints]` (rust: `unsafe_code = "deny"`,
      `unsafe_op_in_unsafe_fn`, `rust_2018_idioms` but for
      `elided_lifetimes_in_paths`; clippy: `all`,
      `undocumented_unsafe_blocks`, `exit`, `dbg_macro`, `todo`,
      `unimplemented`, `redundant_clone`, `implicit_clone`,
      `needless_pass_by_value`, `manual_let_else`), `[lints] workspace =
      true` in every crate. Cast lints stay off workspace-wide (about 1,900
      deliberate PS2 casts); `cast_possible_truncation`, `unwrap_used`,
      `expect_used` are on in the disc and file readers and writers (`iso`,
      `archive`, `pack`, `ps2card`). `print_stdout` is denied in each
      library's `lib.rs` (programs and probes print by design; tests may,
      `clippy.toml`). Left out: `unreachable_pub`, the maintainer's call
      (a `pub` in a private module is crate-only already, the binaries have
      no API, and its `pub(crate)` rewrite was noise); `match_same_arms`
      (an arm per case of the game's switch, with its own note, is the
      port's record of it: merging 105 pairs orphaned 29 notes);
      `elided_lifetimes_in_paths`, the maintainer's call (2,539 `<'_>` of
      noise: the compiler checks lifetimes whether written or not).
      (worklog 516)
- [x] `unsafe`: `eemu/python.rs:369` goes (reuse the `Buffer`); 138 and 145
      stay, sound and documented, under an allow on `impl Buffer` alone.
      `#![forbid(unsafe_code)]` in every other crate root. (worklog 516)
- [x] `#[allow(dead_code)]` (36): 26 are the generated unused `shared()`
      (go with the `Table` trait); the test-only accessors in `piney-game`
      (`area.rs`, `world.rs`, `stream.rs`) become `#[cfg(test)]`; the rest
      used or deleted. Then deny it. (worklog 518: the generated 23 are
      gone with the generator's template line, which a grep counted as a
      37th; 13 remain, one a `cfg_attr` in `dtmenu.rs`. Worklog 520: the
      13 gone, `dead_code` denied workspace-wide.)
- [ ] `#[allow(clippy::too_many_arguments)]` (152): parameter and context
      structs (below), then a threshold and deny.
- [x] A licence allowlist (`deny.toml`) so no copyleft crate can enter: MIT,
      Apache-2.0 (and WITH LLVM-exception), BSD-2/3-Clause, ISC, Zlib,
      Unicode-3.0 / Unicode-DFS-2016, Unlicense, CC0-1.0, BSL-1.0, MPL-2.0;
      an OR licence passes by an allowed option. CI runs `cargo deny check
      licenses` (`.github/workflows/licences.yml`). (worklog 515)
- [x] cargo-deny's advisories, by updating the crates: zstd 0.13.3
      (RUSTSEC-2026-0391), zstd-safe 7.3.0 (RUSTSEC-2026-0398, -0403),
      zstd-sys 2.1.0 (RUSTSEC-2026-0410); ttf-parser 0.25.1 is unmaintained
      (RUSTSEC-2026-0192). Then turn `[advisories]` on in `deny.toml` and CI.
      (worklog 517: zstd 0.14.3; winit's `wayland-csd-adwaita-notitle`
      drops ttf-parser, which only drew the title text of the Wayland
      client-side frame.)

## 2. Lifts

The shape the world's code converges on (the maintainer's direction,
2026-10-30): **defs and systems.** A town, story map, field or dungeon is a
small core plus the systems it has, not one struct that does everything
inline. Each system follows one contract:

```rust
trait MapSystem {
    fn step(&mut self, cx: &mut StepCx);               // per frame, the game's arithmetic
    fn draw(&self, cx: &DrawCx, out: &mut Vec<Piece>); // emits requests, never draws
    fn take_statics(&mut self, _: &Statics) {}         // what carries across visits
    fn put_statics(&self, _: &mut Statics) {}
}
```

- Systems: water, sky (with the crisis sky), clouds, lens flare,
  fireflies, static objects and models, scrolls, light groups, the minimap;
  in fields rain, thunder, steam, the lake; in dungeons the dress, lake,
  leaves, portals and gimmicks; for characters body, arms, motion, AI and
  hits; the bosses already go this way (parts, `Fx`, the wave, `Shape`).
- Each map keeps the game's order explicit: its definition lists the order
  its `Draw` ran its parts in (`[Objects(Other), Sky, Water, Models, Map]`),
  so bit-exactness never depends on a scheduler. Not a dynamic ECS: nothing
  needs queries, and the order must stay visible.
- Systems emit draw and sound requests, never draw: Classic and Modern
  (plans/modern.md) consume the same requests, so a system is written once.
- A new town or map is its definition (systems and order); a new kind of
  thing is one new system.

### Defs and systems

**Standing rule** (the maintainer's, 2026-10-30): any step that touches a
kind of thing (bosses, towns, story maps, fields, dungeons, NPCs, menus,
desktop apps, effects) builds its `*Def` and its systems in that same
step. No intermediate shapes to be redone later (the class-per-boss stage
was that mistake).

The name (the maintainer's, 2026-10-30): a *def* says what a thing is (its
systems and their order), a *system* is one reusable behaviour. Component
composition from data, with no ECS scheduler and no queries, so it is not
called an ECS. In code: `*Def` types (`TownDef`, `FieldDef`, `BossDef`,
`MenuDef`, ...) and the systems under `piney-world/src/systems/` (`water.rs`,
`sky.rs`, `npcs.rs`, ...); other crates the same where they gain systems.

The maintainer's shape, 2026-10-30 ("this works for basically everything"):
a thing is a definition naming its systems, not a struct that does
everything. Three layers, for a town:

```rust
struct AreaCore { file, hits, lights, carry, cameras, rng, statics }  // every area (WORLD_MAN)
const TOWN_SYSTEMS: &[SystemKind] = &[Npcs, Merchants, Gate, Minimap, Party, StaticObjects, Scrolls];
pub const MAC_ANU: TownDef = TownDef {                                 // this town
    no: 0, class: "ROOTTOWN01", volumes: ALL, file: "town01", crisis_file: "town01d",
    systems: &[Water(MAC_ANU_WATER), Sky(MAC_ANU_SKY), CrisisSky(..), Ships],
    order: &[Objects(Other), Sky, CrisisSky, Obj2, Water, Models, Map],  // its Draw's order
};
```

Building one makes the core, adds its kind's systems, adds its own, and
runs them in its `order` each frame. A town file shrinks to its definition
and what is truly its own (Mac Anu's ships, Dun Loireag's ranch). The same
shape for everything that comes in kinds:

| thing | kind's shared systems | definition |
| --- | --- | --- |
| town | NPCs, merchants, gate, minimap, party, statics | `TownDef` |
| field | ambient (rain, thunder, steam), fireflies, lake, portals, NPCs | `FieldDef` (by field type) |
| dungeon | dress, lake, leaves, portals, gimmicks | `DungeonDef` (by dungeon type) |
| story map | whatever each has | `MapDef` |
| boss | base Main blocks, parts, `Fx`, wave, cameras | `BossDef` (started, worklog 529) |
| NPC | row, char, entry, acts, hit, fade, motion | `NpcDef` per game class |
| menu | cursor, list, message check, info, tail | `MenuDef` |
| desktop app | input repeat, window, cursor | `AppDef` |
| effect | generators, elements, draw requests | `EffectDef` |

Not an ECS: no scheduler, no queries; each definition states its order,
which keeps it bit-exact and readable. New towns, maps, bosses, menus, and
fragment's later, are definitions; a new kind of behaviour is one system.

### The systems audit (2026-10-30, at 1863a74)

Where the code does by hand, per kind, what a system should do once:

- [ ] **One world, one area system.** The game has one `WORLD_MAN` whose
      `GO(0/1/2)` makes a town, a story map, a field or a dungeon. The port
      has two worlds: `piney_world::World` (towns, `lib.rs`) and
      `FieldWorld` (fields, dungeons, story maps, `field/world.rs`, 4,311
      lines), carrying the same state twice (save, volume, kite, player,
      camera, phase, requests, targeting, talks, menu view, event targets,
      spcs and their entries, char files and names, asleep, ctrl_done,
      rebooted), and piney-game mirrors the split: `WorldMode` (`world.rs`)
      vs `AreaMode` (`area.rs`) share 24 method names, `FieldHost` vs
      `AreaHost` 51, `town_fx.rs` vs `fx.rs` two effect hosts. Make one
      `World` holding the shared core and one current `Area` (town, story
      map, field, dungeon behind the `AreaMap` trait, made by the registries
      as `WORLD_MAN::GO` does), one mode, one host, one effect host. The
      `AreaMap` and `EvHost` items below are steps toward this.
- [ ] **An NPC system.** Walking PCs (`town/pc.rs` `RtownPc`), merchants
      (`town/merchant.rs`), dogs (`town/dog.rs`), Grunties (`grunty/`),
      event NPCs (`event/entry.rs`, `Box<dyn entry::Npc>`) and field NPCs
      (`field/npcs.rs`) each carry the same parts by hand: an `NpcRow` and a
      `Char`, an `EntryObj`, the act state machine (`act_num`,
      `act_process`, `act_cnt`), the body hit (`body_hit_flag`,
      `body_hit_cnt`), `affect_type`, a fade (`transrate`), next and old
      positions; the town world keeps a vector and an event vector per kind
      (`merchants`/`merchant_events`, `dogs`/`dog_events`,
      `grunties`/`grunty_events`, `pcs`/`pc_events`, `npcs`). The heading
      helpers are written three times (`get_dirc`, `set_dirc`, `dirc_chg`,
      `check_camera_deg` in `town/pc.rs`, `town/merchant.rs`, `chara/ai.rs`,
      `combat/stage.rs`, besides battle's `geom`). One `Npc` core (row,
      char, entry, acts, hit, fade, motion) with a per-kind behaviour (the
      game's own class: `ccRtownPC`, the merchant, `ccDog`, `ccPGuso`, the
      event NPCs), one list, one event channel, the shared helpers once.
- [ ] **A camera system.** The game keeps `cameraList[camID]` with one
      active camera; the port has `camera/mod.rs`, `evcam`, `bosscam`,
      `cinema`, the merchant camera, and `CamView.active` threaded by hand
      (worklog 512). One camera list with the active id, each camera kind a
      member, every reader asking the list.
- [ ] **A sprite and effect-request system.** Four sprite types do one job
      (`TownSprite`, `StorySprite`, `breath::Flame`, `field::ambient::Sprite`)
      with two Eff caches (`town_fx.rs`, `fx.rs`, one of them keyed without
      the file, worklog 509); one `EffSprite` request and one cache.
- [ ] **A carry system.** What outlives a scene is scattered over
      `SaveState` and the session: the scroll statics, `move_tex`,
      `field_rand`, the field ambient's kept state, the kept dummies, the
      story maps' kept area-15 map, the dungeons kept. One `Carry` owned by
      the world, each system declaring what it carries (`take`/`put`
      statics in `MapSystem` are its first form).
- [ ] **A placement (entry) system.** Who stands where for an event is
      decided in several places (`event/entry.rs`, the town's
      `place_entries`, `field/npcs.rs` `FieldNpcs::build`, the merchants'
      `dummy_of`, the registry count `regist_npc_num`); one entry system
      that resolves markers and dummies for any area.
- [ ] **The map's own systems** (the contract above): water, sky, clouds,
      lens flare, fireflies (`FIREFLY` and `FIREFLY2` are two real game
      classes; keep both, as systems), static objects and models, scrolls,
      lights, the minimap (`map/` already one folder), in fields the
      ambient (rain, thunder, steam), in dungeons dress, lake, leaves.
- [ ] **Scene transitions.** The session's `change`, `leave`, `enter_world`
      and the modes' `change_area` / `scene_change` / `leave` repeat the
      same request handling (worklog 503 had the console's `town` skip it);
      one transition path every change goes through.

- [x] `Table` trait (`const NAME`, default `of(v)` / `shared()`) and one
      keyed cache with a per-key `OnceLock` in `store.rs`; piney-gen emits
      `impl Table` and a one-line `of`; drop the 1,017 field accessors and
      the 598 `LazyLock` statics. The hand-written tables (`dungeon`,
      `field`, `sound`, `area`, `statics`, `audio/setbl`) implement it too.
      (worklog 518: keyed by a generated `const ID`, the cache a fixed
      array of `OnceLock`s; `registry` and `stfroll`, still Rust literals,
      keep their 12 accessors until they are in the build.)
- [x] Bosses: `Boss { base: BossBase, class: Class }` and a `BossClass`
      trait (from `corbenik::Host`) with `BaseActs` data (neutral forbid,
      the dash's w, act ids); the base `Main` as ordered building blocks
      (`prologue`, `tick_stop`, `lock_party`, `forward_and_draw`,
      `pp_count_tick`) each class calls in its own order; one generic
      `base_exec_pattern_index`, `change_next_pattern`, `exec_pattern`. Ends
      the 10 class take/put pairs and the 12 copied frames. (worklog 523:
      `Boss { base, class }`, the 10 take/put pairs gone; 524: the
      `BossClass` dispatch and `Patterns` with `BaseActs` for
      `corbenik::Host`; 525: the building blocks, one generic
      `base_exec_pattern_index` and `base_change_next_pattern`; 526:
      Skeith's own members in `Class::Skeith`.)
- [x] `Part<T: PartClass>` / `Parts<T>` for gomoras, leaves, brothers,
      roots, seeds, eyes (keep the placeholder semantics where code indexes).
      (worklog 527)
- [x] Skeith is a boss like the others. He was the first boss ported, so the
      base grew up as his code and still is: `boss.rs` (2,997 lines) holds
      `SkeithData`, his acts and skill rows, `BossBase::skeith_main`,
      `Boss::new` as his constructor, `SkeithPatterns` in `class.rs` and a
      `skeith` field special-cased in `BossData`. Move him to
      `boss/skeith.rs` (or `boss/skeith/`) as a `Class` with its own
      `BossClass` / `Shape` / `Patterns` impls, his data in his module, his
      constructor `skeith::new`; `BossBase` keeps only `ccBoss`. First
      settle from the game's code (vtables, DWARF class layouts) which later
      bosses really derive from `ccBoss01` rather than `ccBoss` (Gorre falls
      back on Skeith's base patterns today): what `ccBoss01` gives them
      becomes a shared piece they compose, not Skeith's code left in the
      base. Checked by the pins and every boss harness against the game.
      (worklog 534: no boss derives from `ccBoss01`; `boss/skeith.rs` holds
      his data, `new`, `Main`, `Affect`, patterns and acts; `patMode` and
      `m_actDirc` are his and Gorre's own, the others' heading `actDirc`;
      a class with no `Affect` runs `ccBoss::Affect`, Gorre's fallback
      goes through his own vtable; `BattleData.skeith` gone, his sword trail
      behind `Shape::sword_trail`. Left for the next item: `BossData`'s
      field per boss, `Class`'s default, the world's `SKEITH_DEF` as the
      other rows' base and fallback look, `act`, `FAR`, `SKILL_*`.)
- [ ] **Bosses as defs and systems.** A boss is a `BossDef` naming its
      systems and their order, like a town:
      ```rust
      pub const INNIS: BossDef = BossDef {
          code: 2, class: "ccBoss02", volumes: MUT_ON, file: "x20",
          data: InnisData::load,              // its tables through the def, not a central BossData struct
          acts: BaseActs { .. },              // what really differs in the shared pattern logic
          systems: &[Patterns(INNIS_PATTERNS), Protect, Hp, Blur, BossCamera, Wave,   // shared boss systems
                     Images(4), MagicSquare, Missiles],                                // Innis's own
          order: &[Prologue, Patterns, Think, Action, Move, LockParty, ForwardAndDraw, Images, Effects],
      };
      ```
      - Shared boss systems written once (the base Main's blocks, the
        pattern runner, parts, Fx, protect/HP, blur, camera, wave,
        after-images); each boss's real uniqueness becomes its own system
        (Innis's magic square and missiles, Magus's leaves and laser,
        Kyvia's gomoras and the disc places, Gorre's brothers and Kerse,
        Corbenik's seeds, roots and eyes, Skeith's spells).
      - `order` states exactly the order each boss's Main ran things in
        (bit-exact); no scheduler.
      - `BossData`, the `Class` enum match (`each_class!`), the world's
        `BOSSES` table and per-boss arms fold into the defs; the world
        reads a boss only through its def and its systems' requests.
      - Shared helpers for systems live in `piney-battle/src/boss/systems/`.
      Checked by the pins (byte-identical), every boss harness against the
      game, the golden boss shots, the suite.
- [x] Bosses, world side: `piney-world` `combat/boss.rs` (start by code,
      `BossLook` images, part actors, clip-to-clump picks), `combat/corbenik.rs`
      and `field_world.rs` `draw_boss` match per boss; with the per-boss
      `Pic` variants on `Out` folded into shared ones (see the particle
      `GenSpec` below), give the class trait the hooks the world needs
      (`look`, `parts`, `pictures`) so a new boss adds one impl, not arms in
      four files. (worklog 529: a `BossDef` row per code, a `Shape` impl per
      class, one `Picture` list; `Out::Wave`, `Out::AfterImageAnm` and
      `Out::DrawAnm` shared; the effects' `Pic`s stay for `GenSpec`.)
- [ ] Bosses, the Python harnesses: `tools/test_battle_*_rs.py` (13 files)
      repeat set-up, stand-ins and per-frame compares; lift what they share
      into `test_battle_boss_rs.py`'s helpers (as `draw_case` and the forced
      cameras were), leaving each file its boss's own cases.
- [x] Boss effects: `enum Fx` in place of 10 `Option<Box>` fields, one
      `Effects::pass(PassMode)` (snapshot vs live length, set vs clear-only
      kept per boss), `BossBase::make_fx`; effect side a `Mirrored` trait,
      `Source<A>`, one `sync` with an explicit order. (worklog 528: the
      pass is `BossBase::effect_pass`; snapshot and live length read the
      same, so only set vs clear-only is kept; `Source` is one struct.)
- [x] Story maps on the towns' base: `StaticSet` (placed models and
      objects, one loader), `MapCore` with defaulted getters (drop the 8
      `as_any` impls: `StoryMap: Any` upcasts), `party_starts(StartSpread)`,
      `event_hits()`, `light_group`, one `ground_dist`. Keep each map's
      `statics::tables()` vs `statics::of(volume)` choice and its quirks.
      (worklog 531: the loader is `StaticModel::load` and
      `StaticObject::load` with `Placing`; `start_positions` defaulted
      from `start` and `spread`; `hits` and `lights` stay one-liners.
      Worklog 533: one `area::AreaCore { file, hits, lights, field_rand,
      start }` in every story map and in the towns' `Base`; `StoryMap`
      defaults the getters through `core()`, the towns' `fieldrand` is the
      core's, `RootTown::field_rand` gone.)
- [ ] Module trees and dispatch registries. The crates' `src/` roots are
      flooded (piney-world 60 files and 4 folders, piney-effect 51,
      piney-battle 39, piney-desktop 30, piney-game 28), and per-kind
      things are dispatched by hand-written `match` arms whose constructors
      each take different arguments, quirks inline (`Town::open_with`,
      `story_map::build`). Two parts, done together:
      - Group by subsystem, each folder a `mod.rs` with a small API. In
        piney-world: `town/` (`Town`, `RootTown`, registry; `mac_anu`,
        `dun_loireag`, `carmina_gadelica`, `fort_ouph`, `lia_fail`, the
        sixth town by its loading-card name; `pc` = rtownpc, `merchant`,
        `party`, `ride`, `dog`), `story_map/` (`StoryMap`, `MapCore`,
        registry; `holy_ground` = EVENTAREA02, `giant` = 07, `boss_arena` =
        B0, `disc` = B8, `ending`, `sky` = 04-06, areas 13 and 43 by their
        area-word names), `field/`, `dungeon/`, `chara/` (char, chara,
        body, pose, motion, arms, player, party, ai, navi, foe, npc),
        `camera/` (camera, evcam, bosscam, cinema), `event/` (event, entry,
        talk, gate, gameover), `grunty/` (grunty, race), the sky effects
        (cloud, lensflare, lattice, firefly); `ee`, `mt`, `hit`, `draw`,
        `area` stay at the root as the shared core. Each module keeps the
        game's class in its doc and `#[doc(alias = "EVENTAREAB0")]`.
      - Registries, not `match`: one constructor input (`MapEnv`: archive,
        volume, save, server, def_se, rng, cc, area_prev, field_prev, kept,
        event_start, crisis), and a table of definitions, e.g.
        `MapDef { class, claims: fn(field, &MapEnv) -> bool, make: fn(&mut
        MapEnv) -> Result<Box<dyn StoryMap>> }` with `static MAPS` and a
        `build` that finds the first claim; `TownDef { no, volumes, make }`
        likewise. Each kind's quirks live in its own `make` (Quarantine's
        field 8 last stage, area 15's kept map). A new map or town is one
        module and one table row. (worklog 521: piney-world's tree and
        both registries; area 13 is `truths_core`, the sixth town
        `relic_city`.)
      - The same for the other crates: piney-effect and piney-battle by
        system (the `BossClass` registry, effects by kind), piney-desktop
        (an app table), piney-fieldui (the `MenuDef` table), piney-game
        (modes and hosts). Probes and tests named after old files follow.
- [ ] Towns: `step_scrolls`, `Sky` (clouds and lens flare), `WaterSpec`
      with the two real wrap rules as data, `Base::passes`, statics slots.
      (worklog 532: `step_scrolls`, `Wrap` and `WaterSpec`; `Sky`,
      `Base::passes` and the statics slots to come.) Also one `town::Water`
      component: Mac Anu, Dun Loireag and Carmina Gadelica each build their
      water by hand in `new` (its file, `Play` of the water anim cloned three
      times and stepped once, `WaterZero::new`, `water_u` / `water_uv`
      stepped by `vftoi12` then `WATER.step`, `water_rows`, its own statics
      slot). A `WaterDef { file, anim, model, reach, spec, statics_slot }`
      constant per town and a `water: Water` member with `new`, `step`,
      `rows`, `take` / `put`; `Sky` the same way for the clouds and flares.
- [ ] `AreaMap` trait over field, dungeon and story map (the 3-arm match
      inlined 19 times in `field_world.rs`).
- [ ] `EvHost<W: HostWorld>` for the 29 identical `Host` methods in
      `field_host.rs` and `area_host.rs` (and the `SaveState` clones).
- [ ] fieldui: one `MenuDef` table for the two parallel id matches (84 runs
      twice: an explicit flag), `MenuCtrl::{cur, init_cursors, msg_check,
      info, back_to_list}`, `Ctx::pushed_key`, a `MenuTail` trait.
- [ ] A shared `cc` math module: `get_dist`, `get_dist3d`, `get_dirc`,
      `get_dirc_chg`, `rand_f`, named angle wraps (`wrap_hi_lo`,
      `wrap_lo_hi`, `wrap_hi`, `wrap_lo`: the order matters), `cam_matrix`,
      `orbit`, the PI constants (redefined in 39 files), one host-to-`Rng`
      adapter, one newlib `rand` (five copies), `GenSpec` / `KeptGens<H>`
      for particle generators, `UNIT` matrix (nine copies).
- [ ] One little-endian reader trait (about ten copies, each with
      `try_into().unwrap()`); `PerVolume<T>`, `VolumeInfo`, `VolumeRules`
      (no more `[OnceLock; 4]` and `v as usize`, `party_ai`'s 39 `later`
      checks); one PNG encoder (two byte-identical copies).
- [ ] piney-gen: a `Gen` context in place of 14 thread-locals and its own
      `DISCS`; `Vol` becomes `Volume`; a `record!` macro emitting `Load` and
      `Store` in one field order; drop the second (Rust literal) serialiser
      once `registry` and `stfroll` are in the build. Byte-compare output.
- [ ] battle_probe: a `BossProbe` trait and an owned `Probe` (nine
      near-identical files with their own thread-locals).
- [ ] Probe commands as types: the 44 `*_probe` examples match the line
      protocol's command strings in `fn handle(cmd: &str, ..)`, some twice
      (`evparty.rs` matches the same op names for their argument counts and
      again to run them). Parse a line once into a typed `enum` with its
      arguments (`FromStr`), match on that; one name-to-parser table shared
      by the probes. A typo then fails at parse, not by falling through.
- [ ] Asset names as handles: the CCS names (`"CMP_trall"`, `"DMY_marker01"`,
      `"ANM_..."`: 666 in piney-world, 180 in piney-effect, 168 in
      piney-demo, 127 in piney-battle) are the game's own keys but are
      scattered, repeated and looked up by string on draw paths. Named
      constants per file in one place (generated by piney-gen from the
      executables' string tables where they come from code), resolved once
      at load into typed ids (`ObjId`, `ClumpId`, `AnimId`), and the draw
      and step code holds the ids.
- [ ] `session.rs` (8,286 lines, 1,365 production): tests out to
      `session/tests/` with one harness (`stick_toward` has six copies);
      production into transition, title, console, loading, settings; a
      `ChangeRequest` enum; `Stage::leave`; a `ModeEnv`.
- [ ] Desktop apps: one `CursorRepeat` (five copies) and the shared draws;
      one trace `Recorder<T>` (five copies).

## 3. Ownership and state

- [ ] `Rc<RefCell<TownPcs>>` owned by `World`, passed `&mut`.
- [ ] `combat/breath.rs` leaking interner: sprite names `Rc<str>` (one
      `EffSprite` and `EffCache` for the four sprite types).
- [ ] `VOICE_ENGLISH` / `DVD` OnceLocks into `SessionOptions`.
- [ ] Audio's `kept_loop` belongs to Magus; `BossEnv.cc` required (no
      throwaway generator); `RefCell<&mut W>` in party motion and kite into
      a context struct.
- [ ] Pattern tables copied with `.to_vec()` per change: slices.
- [ ] Draw parameter structs (`Target`, `AnimDraw`, `BodyPose`,
      `ModelOpts`, `Draw::plain`), battle (`BattleCx`, `SkillCall`, `ItemCx`,
      `ProtectIn`), world (`SceneTarget`, `FieldCtx`, `MemberSpawn`), effect
      (`Spawn`, `SmokeParams`, `RingSpec`), gs (`MmatInput`, `Gpu`,
      `Target`), fieldui (`Grid`, `SpcItem`, `SelectBox`).

## 4. Errors and panics

- [ ] `Load::load` returns `Result`; `store::preload(v)` at disc open, so
      `of()` cannot fail after it; a different disc registered for a loaded
      volume is an error.
- [ ] One error enum per library crate (`piney-event` has three plus
      `String`; eemu's `sys` uses `Fault`; `GsError`).
- [ ] Runtime `expect`s and guarded `unwrap`s into `let ... else`
      (`crew.ais`, `foes[me].take()`, grunty, ai, combat, session, desktop).
- [ ] Clamp capacities read from `.disc` and ISO data; checked conversions
      in the writers.

## 5. Perf (after the clean-up; trace release builds at the slow scenes first)

- Every scene: the session clones the whole frame's draw list each frame
  (`session.rs:1280`); posing runs two or three times per animated thing,
  with a quadratic parent test (`pose.rs`, `body.rs`, `field_area.rs`);
  a `String` per textured draw (`piney-draw` `TexRef::Ccs`, `ModelDraw.file`).
- Renderer: frame-copy textures and four bind groups made per frame
  (`piney-gs` `lib.rs:811`); a deep clone of every UV-scrolled model
  (`lib.rs:695`); shadow buffers per group; scratch Vecs never reused; an
  overlay switch drops every decoded file and texture.
- Effects and battle: particles find their generator by linear scan
  (`particle.rs:195`); every sprite draw boxes a clone of its `Eff`;
  quadratic node depth (`effect/nodes.rs`); asset lookups by string in draw
  loops; `Scene::listed` (`Vec::contains`, 97 sites); `pc_list` / `ene_list`
  cloned per AI decision; `Scene::view` clones a `Char`.
- Streams: `Scene::world` recursive, not memoised; two clones per model draw.
- Audio: the cpal callback allocates under the engine mutex, which the game
  thread takes on every sound call.
- Data: `Archive::find` allocates about 2k strings per lookup (about 1M in a
  stream's warm-up); `Manifest::find` per disc read; each movie frame hashed
  in full and re-uploaded; the event-call log grows without bound.
