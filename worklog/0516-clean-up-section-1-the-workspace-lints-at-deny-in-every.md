---
number: 516
title: Clean-up section 1: the workspace lints at deny in every crate, what they flagged fixed, the readers' own lints, and unsafe kept to eemu's Python buffers
date: 2026-10-29
area: build, tooling, test
files: Cargo.toml, clippy.toml, crates/piney-gen/src/main.rs, crates/piney-data/src/pack.rs, crates/piney-data/src/iso.rs, crates/piney-eemu/src/python.rs, plans/rust-cleanup.md
started: 2026-10-29T21:27:31-06:00
took: 1h 33m
---

# 516. Clean-up section 1: the workspace lints at deny in every crate, what they flagged fixed, the readers' own lints, and unsafe kept to eemu's Python buffers

Section 1 of `plans/rust-cleanup.md`, the first two items: the
workspace lints, now at deny in every crate, with what they flagged
fixed; and `unsafe` confined to eemu's two Python buffer views.
`[lints] workspace = true` was already in every manifest (1f8313f, at
warn). This commit brings the code fixes and raises the levels.

## The lints

`[workspace.lints]`:

- **rust**: `unsafe_code` and `unsafe_op_in_unsafe_fn` deny,
  `rust_2018_idioms` deny but for `elided_lifetimes_in_paths`, which is
  allowed;
- **clippy**: `all` deny, plus `undocumented_unsafe_blocks`, `exit`,
  `dbg_macro`, `todo`, `unimplemented`, `redundant_clone`,
  `implicit_clone`, `needless_pass_by_value` and `manual_let_else`;
- `too_many_arguments` stays at warn until section 3's parameter
  structs. Its 152 allows and the 36 `dead_code` allows are untouched.

`cargo clippy --workspace --all-targets --all-features` is clean.

**What they flagged, and what became of it** (counts at warn, on the
default features):

| lint | hits | fix |
| --- | --- | --- |
| `elided_lifetimes_in_paths` | 2,500 (+19 with `--all-features`) | allowed instead, below |
| `print_stdout` | 626 | below |
| `match_same_arms` | 105 | left out, below |
| `needless_pass_by_value` | 39 | below |
| `redundant_clone` / `implicit_clone` | 21 / 1 | clippy's fix |
| `exit` | 1 | `piney-gen`'s `main` returns an `ExitCode` from `run()` |
| `manual_let_else` | 1 | `official::locate`'s event table |

**`print_stdout`.**
- Tests may print (`clippy.toml`: `allow-print-in-tests`).
- Programs and probes print by design, so the workspace allows it and
  each library's `lib.rs` denies it.
- The library prints that remained went to stderr: piney-gen's carry
  and syms reports (piney-build runs them inside the game's launcher)
  and `LogHost`'s echo.

**`needless_pass_by_value`.**
- Most parameters now borrow: slices for `Vec`s, `&str` for `String`s,
  `&Arc<Archive>` and so on. The call sites take `&`.
- `Smoke` (`debris.rs`) is a small plain struct, now `Copy`.
- `Cmp::test` takes `Copy` operands.
- `entry::routine` takes `&mut RoutineRef`.
- pyo3's four `__traverse__` must take `PyVisit` by value, so each
  carries an allow with that reason.

**Left out.**
- `elided_lifetimes_in_paths` (the maintainer's call). Writing `<'_>` at
  every elided lifetime took 2,539 edits in 298 files, and it was noise:
  the compiler checks the lifetimes whether or not they are written. The
  lint is allowed, and every line has its lifetimes as before. The
  workspace counts 356 `'_`, the same as before this commit.
- `unreachable_pub` (the maintainer's call). A `pub` in a private module
  is crate-only already, and the binaries have no API to guard. Its
  `pub(crate)` rewrite touched about 268 files for nothing. Every
  item's visibility is as HEAD had it: 405 `pub(crate)`, 22,577 `pub`.
- `match_same_arms`. Clippy's merges of the 105 pairs left 29 notes
  above arms they no longer describe. In the port, each arm stands for
  one case of the game's switch, with its note: keeping them apart is
  the record.

## The readers' lints

`cast_possible_truncation`, `unwrap_used` and `expect_used` are denied
in the disc and file readers and writers: `iso`, `archive`, `pack` and
`ps2card`. Tests may unwrap, by `clippy.toml`. The fixes:

- **The `.disc` writer.** `Manifest::encode` returns `Result`; a count
  its u16/u32 field cannot hold is an error (`fits`). piney-build
  passes the error on.
- **The `.disc` reader.** `Reader::array::<N>` takes the place of
  `try_into().unwrap()`, and a chunk's offsets convert checked.
- **Poisoned locks.** The manifest cache's lock takes a poisoned guard
  rather than panic.
- **Lengths.** `SECTOR_LEN` and `PAK_HEADER_LEN` are lengths in their
  own right; `read_at` saturates an entry's remaining length.

## `unsafe`

The `PyBytes` copy of fresh RAM (`python.rs:369`) no longer needs
`unsafe`: it reads the RAM `Buffer` adopted just before it.

The two views of a pinned bytearray (`Buffer::bytes`, `bytes_mut`) stay.
They are sound and documented, under `#[allow(unsafe_code)]` on
`impl Buffer` alone. Every other crate root has
`#![forbid(unsafe_code)]`.

## Checks

`took` is the first clock (1h 14m) plus the clock for undoing the
`<'_>` (19m).

- The tests of every crate, and piney-game's suite (339).
- `piney-gen gen --check`: tables and built bytes unchanged.
- clippy with all features, and fmt.
- Harnesses whose probes changed signatures: `test_eemu_rs` (which
  builds the Python module), `test_effect_spell_rs`, `test_demo_rs`,
  `test_battle_boss_rs`, `test_battle_corbenik_b_rs` and
  `test_battle_kyvia_rs`.

**Still unknown:** nothing
