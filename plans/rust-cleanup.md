# Rust clean-up

A read-only audit of the whole workspace (2026-10-29, tree at 2f4ffdf) for
idiomatic Rust: duplication lifted into traits, tables and shared modules,
owned state instead of hidden globals, no `unsafe`, errors instead of panics,
no lint exemptions; and, after the clean-up, the frame-time suspects. Every
step keeps behaviour bit-exact: the suite, the harnesses against the game's
code, and a byte compare of the generated tables guard each one. Work it in
order; tick items off here as they land, with the worklog entry.

## 0. Real bugs the audit turned up

- [ ] `field::INF` / `dungeon::INF` / `sound::INF` statics: `piney-world`
      `field_area.rs:277` and `field_ambient.rs:360` read Infection's field
      tables on every volume (Outbreak and Quarantine take `sqrt.s`, not
      newlib `sqrtf`). Delete the statics so every caller names its volume
      (16 sites, the viewer too); check an Outbreak field against the game.
- [ ] `piney-battle` `geom.rs` `get_dirc_chg` divides by `mode & 0xffff`
      with no zero guard (the world's copy has one); gimmick data feeds it.
- [ ] Boss `affect`s do plain `i16` `hp ± p0` (panics on overflow in debug);
      others use `wrapping_*`. One `hit_hp` computing in `i32` as the game's
      `int` promotion does, checked against the code.
- [ ] `piney-game` `fx.rs:201` sprite cache key leaves out the file
      (`town_fx.rs:210` has it): two maps' same-named effects collide.
- [ ] `piney-data` `store::group` races: two threads can load and leak one
      group twice and get different `&'static` pointers.
- [ ] `piney-gen` `die()` calls `process::exit` from library code that the
      game reaches through `piney_build::image_data` (43 sites): `Result`.
- [ ] `piney-eemu` thread-local `BUS` is never uninstalled: a second `Sys` on
      a thread silently routes through a dead one. Put the bus in `Mem`.
- [ ] Process-wide state shared between tests: `field_ambient` `static KEPT:
      Mutex` (carry it in `SaveState` like the scroll statics) and `iso`
      `DENY_EXECUTABLE` (an option on the `Iso`).
- [ ] `piney-event` `PARSED` caches an `Err` forever and clones every event
      per call: `OnceLock<Arc<[Event]>>`, retry on error.

## 1. Guardrails

- [ ] `[workspace.lints]` (rust: `unsafe_code = "deny"`,
      `unsafe_op_in_unsafe_fn`, `rust_2018_idioms`, `unreachable_pub`;
      clippy: `all`, `undocumented_unsafe_blocks`, `exit`, `print_stdout`,
      `dbg_macro`, `todo`, `unimplemented`, `redundant_clone`,
      `implicit_clone`, `needless_pass_by_value`, `manual_let_else`,
      `match_same_arms`), `[lints] workspace = true` in every crate. Cast
      lints stay off workspace-wide (about 1,900 deliberate PS2 casts); turn
      `cast_possible_truncation`, `unwrap_used`, `expect_used` on per module
      in the disc and file readers and writers.
- [ ] `unsafe`: `eemu/python.rs:369` goes (reuse the `Buffer`); 138 and 145
      stay, sound and documented, under a module-level allow.
      `#![forbid(unsafe_code)]` everywhere else.
- [ ] `#[allow(dead_code)]` (36): 26 are the generated unused `shared()`
      (go with the `Table` trait); the test-only accessors in `piney-game`
      (`area.rs`, `world.rs`, `stream.rs`) become `#[cfg(test)]`; the rest
      used or deleted. Then deny it.
- [ ] `#[allow(clippy::too_many_arguments)]` (152): parameter and context
      structs (below), then a threshold and deny.

## 2. Lifts

- [ ] `Table` trait (`const NAME`, default `of(v)` / `shared()`) and one
      keyed cache with a per-key `OnceLock` in `store.rs`; piney-gen emits
      `impl Table` and a one-line `of`; drop the 1,017 field accessors and
      the 598 `LazyLock` statics. The hand-written tables (`dungeon`,
      `field`, `sound`, `area`, `statics`, `audio/setbl`) implement it too.
- [ ] Bosses: `Boss { base: BossBase, class: Class }` and a `BossClass`
      trait (from `corbenik::Host`) with `BaseActs` data (neutral forbid,
      the dash's w, act ids); the base `Main` as ordered building blocks
      (`prologue`, `tick_stop`, `lock_party`, `forward_and_draw`,
      `pp_count_tick`) each class calls in its own order; one generic
      `base_exec_pattern_index`, `change_next_pattern`, `exec_pattern`. Ends
      the 10 class take/put pairs and the 12 copied frames.
- [ ] `Part<T: PartClass>` / `Parts<T>` for gomoras, leaves, brothers,
      roots, seeds, eyes (keep the placeholder semantics where code indexes).
- [ ] Boss effects: `enum Fx` in place of 10 `Option<Box>` fields, one
      `Effects::pass(PassMode)` (snapshot vs live length, set vs clear-only
      kept per boss), `BossBase::make_fx`; effect side a `Mirrored` trait,
      `Source<A>`, one `sync` with an explicit order.
- [ ] Story maps on the towns' base: `StaticSet` (placed models and
      objects, one loader), `MapCore` with defaulted getters (drop the 8
      `as_any` impls: `StoryMap: Any` upcasts), `party_starts(StartSpread)`,
      `event_hits()`, `light_group`, one `ground_dist`. Keep each map's
      `statics::tables()` vs `statics::of(volume)` choice and its quirks.
- [ ] Towns: `step_scrolls`, `Sky` (clouds and lens flare), `WaterSpec`
      with the two real wrap rules as data, `Base::passes`, statics slots.
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
