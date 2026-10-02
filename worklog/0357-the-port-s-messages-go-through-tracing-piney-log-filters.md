---
number: 357
title: "The port's messages go through tracing: PINEY_LOG filters them, and the console shows warnings and errors"
date: 2026-10-02
area: build, tooling
files: Cargo.toml, crates/piney-game/src/logging.rs, crates/piney-game/src/main.rs, crates/piney-viewer/src/main.rs, README.md
---

# 357. The port's messages go through tracing: PINEY_LOG filters them, and the console shows warnings and errors

Not the game's. The user asked for a better logger. The port's
diagnostics were 125 bare `eprintln!`s outside the tests, in nine crates.
They had no level, no source and no way to quiet them. A Data Drain trace
sat behind its own variable (`DBG_DRAIN`). Warnings never reached anyone
playing without a terminal.

## Now

The `tracing` crate (0.1.44) logs everywhere, and `tracing-subscriber`
(0.3.23, `env-filter`) prints it. Both are workspace dependencies. Each
site took a level by what it reports:

| level | what |
| --- | --- |
| error | the run cannot go on: bad arguments, no GPU, an unreadable disc or build, a failed `--shot` |
| warn | a part is missing and play goes on without it: a map, effects, a stream or movie counted as played, a set-up pass that did not settle, an event-host method not ported, a texture not found |
| info | a replay's progress, the pad log's path, the viewer's model count |
| debug | the desktop's and top page's script trace, a set-up pass's frame count, a mode change the session did not expect |
| trace | Kite's skill and act around a field frame (was `DBG_DRAIN`; now `tracing::enabled!`) |

`piney-game` installs the subscriber first thing in `main`
(`logging::init`). It prints compact lines to stderr, coloured only on a
terminal. `PINEY_LOG` is read as an `EnvFilter`. The default,
`warn,piney=info`, keeps the port's crates at info (targets are matched by
prefix) and wgpu and naga, whose `log` records now arrive through
`tracing-log`, at warn. The viewer sets up the same filter.

A third layer, `ToConsole`, keeps every warning and error as one line
(`WARN: message field=value`). The window's frame loop hands those lines to
the console's `say`, so they show in the console (F1) without a terminal.
`warnings_reach_the_console` checks it under a scoped subscriber (info is
left out, fields are kept).

`piney-gen` and `piney-build` keep `eprintln!`: their output is a command
line tool's answer, not a log. Tests keep theirs too.

**Still unknown:** nothing.
