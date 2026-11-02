---
number: 559
title: Issue #81: the disc's loading time held twice too long; --cdrom and a disc_speed setting
date: 2026-11-02
area: engine, disc, ui
files: crates/piney-game/src/dvd.rs, crates/piney-game/src/session.rs, crates/piney-game/src/settings.rs, crates/piney-game/src/main.rs, docs/engine/field-walk.md, README.md
started: 2026-11-02T03:07:54-07:00
took: 3m
---

# 559. Issue #81: the disc's loading time held twice too long; --cdrom and a disc_speed setting

Issue #81 asked for an optional loading delay, default off. The maintainer remembered a `--cdrom` flag for it. The flag is `--dvd [SPEED]`, and it worked, but its wait was twice the disc's time in the field.

`Dvd::frames` turned the estimated seconds into counts at 60 a second, and `Session::step` took one off per game frame. The field runs at `FRAME_RATE` 2, two vertical blanks a frame (`ccSetupNewGame` 0x001688d0), so every hold lasted twice as long. The estimate is now `Dvd::vblanks` at 59.94 a second, and each step takes `Mode::frame_rate` off. Area 15 from Mac Anu at 3x: 84 blanks, 42 frames (`session::area15::a_warp_waits_for_the_disc_under_the_loading_display`, which fails with one off a frame: 43 blanks left a frame before the end).

The timing follows what the game's loader leaves to the drive. `ccCdInit` sets the read mode to trycount 0, spindle 1, pattern 0 (INF main 0x001596e0-0x001596f4). `ccCdvd::StInit` streams through 64 sectors in 4 banks (`sceCdStInit(64, 4, iop_buf)`, 0x00159c94). `ccCdStRead` (0x00159a40) calls `sceCdStRead` in mode 1, blocking, and only breathes frames while the drive is busy. Nothing in the loader paces the reads by frames, so the drive's own rate is the only limit. The estimate stays a seek (0.12 s) per run of new members lying end to end, plus their bytes at SPEED times DVD 1x (1,385,000 bytes a second). The inflate is not counted.

Two ways to turn it on, both off by default:
- `--cdrom [SPEED]`, the same as `--dvd [SPEED]`.
- `disc_speed = SPEED` in `settings.toml`, read for a windowed run. The flag is set first and wins (`dvd::set` keeps the first). `Settings::store` used to rewrite the file from the save's options alone, which would have dropped the key. It now keeps the port's keys' lines as the player wrote them, and writes `disc_speed = 0` when the file has none, so the option can be found. Test: `settings::tests::the_disc_speed_survives_a_store`.

**Still unknown:** nothing.
