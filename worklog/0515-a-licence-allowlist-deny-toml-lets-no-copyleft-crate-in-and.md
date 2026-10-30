---
number: 515
title: A licence allowlist: deny.toml lets no copyleft crate in, and CI checks it with a pinned cargo-deny
date: 2026-10-29
area: build, tooling
files: deny.toml, .github/workflows/licences.yml, plans/rust-cleanup.md
started: 2026-10-29T21:26:26-06:00
took: 1m
---

# 515. A licence allowlist: deny.toml lets no copyleft crate in, and CI checks it with a pinned cargo-deny

The port is MIT OR Apache-2.0 on purpose. `deny.toml` at the root now
says which licences a crate in the graph may carry:

- MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception;
- BSD-2-Clause, BSD-3-Clause, ISC, Zlib;
- Unicode-3.0, Unicode-DFS-2016, Unlicense, CC0-1.0, BSL-1.0;
- MPL-2.0, whose copyleft is per file.

No GPL, LGPL or AGPL. cargo-deny reads each licence as an SPDX
expression, so an OR licence passes when one of its options is
allowed. `r-efi` ("MIT OR Apache-2.0 OR LGPL-2.1-or-later") passes by
MIT, with LGPL not on the list. `all-features` is on, so optional
dependencies count too.

**CI.** `.github/workflows/licences.yml` runs on every push and pull
request. It installs cargo-deny 0.20.2 through
`taiki-e/install-action` (a prebuilt, pinned binary) and runs
`cargo deny check licenses`.

**Local run** (cargo-deny 0.20.2 was already in `~/.cargo/bin`):
`licenses ok`. Of 313 third-party crates, the common licences are
"MIT OR Apache-2.0" (139), MIT (72), "Apache-2.0 OR MIT" (16) and
"MIT/Apache-2.0" (13). To show the list bites, I dropped
BSD-3-Clause from a copy of the config: cargo-deny then rejected the
crates under it.

**What else cargo-deny flags, not checked yet:**

| check | finding |
| --- | --- |
| advisories, vulnerability | zstd 0.13.3: RUSTSEC-2026-0391 |
| advisories, vulnerability | zstd-safe 7.3.0: RUSTSEC-2026-0398, -0403 |
| advisories, vulnerability | zstd-sys 2.1.0: RUSTSEC-2026-0410 |
| advisories, unmaintained | ttf-parser 0.25.1: RUSTSEC-2026-0192 |
| bans | 15 duplicate versions (bitflags, syn, windows-sys, rustix, thiserror and others) |
| sources | nothing |

No Rust code changed, so piney-game's suite was not run for this entry.

**Still unknown:** nothing
