---
number: 517
title: cargo-deny's advisories on: zstd 0.14.3, and winit without the font parser
date: 2026-10-29
area: build, tooling
files: deny.toml, .github/workflows/licences.yml, Cargo.lock, crates/piney-data/Cargo.toml, crates/piney-build/Cargo.toml, crates/piney-game/Cargo.toml, crates/piney-viewer/Cargo.toml, plans/rust-cleanup.md
started: 2026-10-29T23:09:34-06:00
took: 15m
---

# 517. cargo-deny's advisories on: zstd 0.14.3, and winit without the font parser

Section 1's last item of `plans/rust-cleanup.md`: the crates cargo-deny's
advisory check flagged, updated or dropped, and the check turned on.

## What was flagged

`cargo deny check advisories` (cargo-deny 0.20.2) found four:

| crate | advisory | pulled by |
| --- | --- | --- |
| zstd 0.13.3 | RUSTSEC-2026-0391 (a reused context leaks an abandoned stream's data) | piney-data, piney-build |
| zstd-safe 7.3.0 | RUSTSEC-2026-0398, -0403 | zstd |
| zstd-sys 2.1.0 | RUSTSEC-2026-0410 | zstd |
| ttf-parser 0.25.1 | RUSTSEC-2026-0192, unmaintained | winit, through sctk-adwaita and ab_glyph |

## zstd

`zstd = "0.14"` in piney-data and piney-build: zstd 0.14.3, zstd-safe
8.2.0, zstd-sys 2.1.2 (the same libzstd 1.5.7). The port calls only
`bulk::compress` and `bulk::decompress`, which kept their signatures.

The built discs keep their chunks in zstd (`chunks.pak`), so a build
made before must read the same after. A scratch reader
(`/mnt/data/claude/scratch/zstdcheck`, not in the repo) read every
fifth file of all four `.disc`s of `dist/` through `PackDisc`, checked
each against the manifest's blake3, and hashed every byte read:

- before (zstd 0.13.3): 101 files, 3,379,139,122 bytes, 22,402 zstd
  chunks, 0 mismatches, digest `39b52e60...72f1324`;
- after (zstd 0.14.3): the same counts, 0 mismatches, the same digest.

## ttf-parser

winit 0.30 (the newest release; 0.31 is a beta) takes sctk-adwaita
0.10, whose title text is drawn with ab_glyph over ttf-parser. There is
no maintained ttf-parser to move to, and sctk-adwaita draws nothing
else with it. winit's `wayland-csd-adwaita-notitle` feature keeps the
client-side frame (its buttons and border) without the title text, so
piney-game and piney-viewer take winit with that in place of the
default `wayland-csd-adwaita`; the rest of the defaults (`rwh_06`,
`x11`, `wayland`, `wayland-dlopen`) are as before. ab_glyph,
ab_glyph_rasterizer, owned_ttf_parser and ttf-parser leave the graph.

The frame is only drawn by a Wayland compositor that asks the client to
decorate (GNOME); KDE, X11 and full screen draw their own or none. There
the window's frame has no title text now. `wayland-csd-adwaita-crossfont`
would keep it, but through FreeType, which the licence allowlist keeps
out.

## The check

`deny.toml` gets `[advisories]`: vulnerable, unmaintained (`all`),
unsound (`all`) and yanked crates fail, with no ignores. CI's job runs
`cargo deny check licenses advisories`. Locally, both pass.

## Checks

- piney-data, piney-build, piney-viewer tests and piney-game's suite
  (339 passed), release.
- clippy with all features, fmt.

**Still unknown:** nothing
