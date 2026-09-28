# piney-eemu

`tools/eemu.py`, the Emotion Engine interpreter the Python harnesses run the
game's own code in, ported to Rust instruction for instruction, and the
`eemu_rs` Python module a harness can use in eemu's place.

- `src/cpu.rs`: the interpreter. eemu's `Machine` (the EE core, the FPU with
  the EE's float rules, the C library stand-ins), plus `tools/test_anim.py`'s
  VuMachine (VU0 macro mode, lqc2 / sqc2, ldl / ldr, its MMI instructions),
  `tools/test_stream_rs.py`'s Vu0Machine (vi0-15, VU0 data memory, cfc2 /
  ctc2, viadd / viaddi, vlqi / vsqi / vlqd / vsqd) and `tools/test_toppage_rs.py`'s
  divide-by-zero rule, each behind a `Features` switch. It holds state the
  way the Python does and stops on exactly what the Python stops on.
- `src/fpu.rs`: `piney_data::field::ee`'s bit-exact EE arithmetic plus
  `rsqrt.s`. `src/hle.rs`: eemu's HLE string and memory functions.
- `src/machine.rs`: a machine over memory it owns, for Rust callers.
- `src/python.rs` (feature `python`): the `eemu_rs` module.

## Building the module

    cargo build --release -p piney-eemu --features python
    cp target/release/libpiney_eemu.so tools/eemu_rs.so

`tools/.gitignore` keeps the copy out of git. `python3 tools/test_eemu_rs.py`
builds and copies it before it tests. The module imports `tools/eemu.py` (its
`Stop`, `ANNUL`, `HLE` and the helpers it re-exports) and `tools/mips.py`, so
it is imported from `tools/`, like eemu. The `python` feature is off by
default, so the workspace builds and tests without PyO3.

## Opting a harness in

`eemu_rs` exports everything `eemu` does (`Stop` and `ANNUL` are eemu's own
objects, so `except eemu.Stop` and `is eemu.ANNUL` keep working), with its
own `Machine`, `run_ctors`, and test_anim's `machine_class`. So a harness
switches by naming eemu_rs where it names eemu's machine:

| The harness has | Change it to |
| --- | --- |
| `from eemu import Machine, ...` | `from eemu_rs import Machine, ...` |
| `import eemu` ... `eemu.Machine(p)` | `import eemu_rs as eemu` |
| `from test_anim import machine_class` | `from eemu_rs import machine_class` |
| test_morph_rs / test_stream_rs: `import test_anim` ... `test_anim.machine_class()` | `import eemu_rs as test_anim` |
| test_anim's own `machine_class()` | `return __import__("eemu_rs").VuMachine` as its first line switches every harness that takes its VuMachine from there (world, morph, demo's light, stream) |

Python subclasses keep working: test_stream_rs's Vu0Machine (a `cop2`
override on top of `machine_class()`) and test_toppage_rs's `ee_machine` (an
`exec` override) run unchanged on eemu_rs, their overrides called for exactly
the instructions eemu would call them for. For full speed they have native
forms: `eemu_rs.Vu0Machine(p)` and `eemu_rs.Machine(p, ee_div=True)`.

Proven: every eemu-using unittest suite in `tools/` that runs at 2222dc9
passes with eemu's `Machine` and test_anim's `machine_class` pointed at
eemu_rs (which is what the switch above does) and nothing else changed.
test_stream_rs has no unittest suite (it writes fixtures); its code runs in
lockstep, both ways, in `tools/test_eemu_rs.py`.

| Suite | Tests | eemu | eemu_rs |
| --- | ---: | ---: | ---: |
| test_world_rs | 5 | 1275.3 s | 18.5 s |
| test_field | 8 | 163.9 s | 10.1 s |
| test_desktop_name_rs | 3 | 145.5 s | 6.2 s |
| test_dungeon | 9 | 86.6 s | 2.4 s |
| test_font | 13 | 69.4 s | 1.5 s |
| test_toppage_rs (its exec-override subclass) | 6 | 25.6 s | 5.1 s |
| test_save | 9 | 23.5 s | 1.5 s |
| test_anim | 4 | 20.4 s | 3.3 s |
| test_desktop_data_rs | 6 | 18.6 s | 0.9 s |
| test_field_rs | 7 | 18.4 s | 18.5 s |
| test_areas | 7 | 16.9 s | 9.5 s |
| test_desktop_rs | 20 | 14.9 s | 6.4 s |
| test_battle | 14 | 6.7 s | 1.2 s |
| test_desktop_menu_rs | 5 | 6.5 s | 1.0 s |
| test_event_vm (its exec-override subclass) | 3 | 5.4 s | 0.7 s |
| test_evscript | 13 | 3.9 s | 0.4 s |
| test_dungeon_rs (run_range drives exec) | 7 | 3.9 s | 3.7 s |
| test_morph_rs | 1 | 2.9 s | 2.1 s |
| all | 150 | 1908 s | 93 s |

The times include each suite's own work (cargo, the probes, the Python
models it compares against); test_field_rs and test_dungeon_rs barely use
the interpreter. test_demo_rs cannot run at 2222dc9 under either: its
demo_probe does not compile (`piney_gs::convert::mmat` takes an argument
more than `demo_probe.rs:796` passes); its game side runs in lockstep in
`tools/test_eemu_rs.py`. `tools/iopemu.py`'s `Iop`, which builds itself
without a program, is a tool rather than a harness and stays on eemu.

## The Python API

As `eemu.Machine`: `Machine(program)`, `mem` (a 32 MB `bytearray`),
`pristine`, `hooks`, `p`, `r`, `f`, `hi`, `lo`, `hi1`, `lo1`, `fcr31`, `acc`,
`steps`, `load`, `store`, `get`, `set`, `set32`, `call`, `exec`, `cop1_s`,
`branch`, `bad`, `changed`; the VuMachine's `vf`, `vacc`, `q`, `vset`, `ea`,
`cop2`, `mmi`; the Vu0Machine's `vi` and `vumem`. The module adds:

- `VuMachine(program)`, `Vu0Machine(program)`, `machine_class()` (the
  VuMachine class, as test_anim's), and keywords on all three:
  `Machine(program, vu=False, vi=False, ee_div=False)`; `m.features`.
- For lockstep checking: `m.trace = fn` calls `fn(pc, word, next_pc,
  next_npc)` after every step (word None for a hook), `m.writes` lists the
  RAM (address, length) the step wrote, and `m.state()` returns every
  register as one tuple.
- A long run can be interrupted with Ctrl-C.

## What differs from eemu.Machine

- RAM is a real bytearray, but one the machine holds a buffer export on (the
  interpreter reads and writes its storage directly), so it cannot change
  size: a slice assignment that would grow or shrink it raises
  `BufferError`, where eemu would silently resize its RAM. That includes the
  HLE functions: a `memcpy` / `memset` / `strcpy` running past the end of
  RAM raises `BufferError` in eemu_rs; in eemu it grows the bytearray.
  `m.mem = x` takes only another 32 MB bytearray.
- The register files are fixed-width live views, not lists. `list(m.r)`
  copies; a reference kept across `call` (which in eemu makes a new list)
  sees the new values. Values are masked when stored: GPRs and HI / LO to
  128 bits, FPU registers, ACC, FCR31, Q and the VU0 floats to 32, vi to 16,
  `steps` to 64. eemu keeps whatever int it is given, so a negative or
  oversized value a harness stores itself can behave differently.
- `hooks` is a mapping (`eemu_rs.Hooks`) with the dict methods harnesses use
  (`[]`, `get`, `pop`, `update`, `setdefault`, `del`, `in`, `len`,
  iteration, `items` / `keys` / `values`, `clear`, `copy`), not a `dict`.
  `m.hooks = {...}` copies into a new one. Only int keys can match a pc.
  eemu's own HLE functions installed as hooks run natively; `m.hooks[a]`
  still returns eemu's function.
- Construction is in `__new__`, so a subclass passes the program to the
  constructor. A class that builds its own state without a program
  (`tools/iopemu.py`'s `Iop`) cannot sit on eemu_rs.
- Of a Python subclass's overrides, `exec`, `cop2` and `mmi` are honoured;
  overriding `load`, `store`, `set` and the like does not reach the
  interpreter (eemu's `exec` calls them through `self`). No harness does.
- An `exec` override is a Python call per instruction, so that path runs
  near eemu's speed; `cop2` / `mmi` overrides cost only on those
  instructions.
- `call` takes `p.gp` of None as 0 (eemu would store None in `$gp`).

## Parity

`tools/test_eemu_rs.py` runs eemu and eemu_rs in lockstep and must find no
difference:

- Lockstep over the harnesses' own code (their game side, no cargo probes).
  The Rust machine runs natively and traces every step; the Python one takes
  the same step; the pc, every register (GPRs, FPU, HI / LO of both
  pipelines, FCR31, ACC, the step count; on the VU machines vf, ACC, Q, vi
  and VU0 data memory) and every byte either machine wrote must agree, each
  call must return the same and leave the same 32 MB, and a stop (eemu.Stop
  or a hook's own exception) must come at the same step with the same
  message. Hooks run once, against both; the Python side replays their
  results. Every workload runs to its end:

  | Workload | Steps compared | Calls | Machine |
  | --- | ---: | ---: | --- |
  | test_anim: controllers, playback, ccMorpher::Modify | 3,186,242 | 167 | VuMachine |
  | test_world_rs: Decode_Hit, land checks, 44 frames of Kite | 7,396,245 | 103 | VuMachine |
  | test_stream_rs: skips, 536 table lookups, stream 106 drawn | 4,039,272 | 728 | its Vu0Machine subclass (cop2 override) |
  | the same | 4,039,272 | 728 | eemu_rs.Vu0Machine |
  | test_morph_rs: 120 random morphers | 96,333 | 120 | VuMachine |
  | test_demo_rs: MoveCurNut, the fader, LoadGame, the icons' light | 127,204 | 761 | Machine, VuMachine |
  | test_desktop_rs: 1,500 SelectModes, 512 DrawLogos | 82,628 | 2,012 | Machine |
  | test_toppage_rs: the top page and board, 160 frames | 134,387 | 161 | its ee_machine subclass (exec override) |
  | the same | 134,387 | 161 | Machine(ee_div=True) |
  | test_dungeon_rs: run_range driving exec itself, SetDungeonTexClut | 105,456 | 78 | Machine |

  19.3 million steps in all, no difference.
- Fuzzing, one instruction at a time from random states full of EE float
  edge cases, on all six flavours: 120,000 words; the test checks that
  every instruction each flavour interprets ran (125 of eemu's, 108 the
  VuMachine adds, 10 the Vu0Machine adds) and that everything else stopped
  alike. 900 random programs run whole (branches, likely branches, jal,
  hooks, HLE), and 1,500 HLE calls on random strings.
- The Python surface: mem, load / store sizes and signs, hooks, register
  views, stops and their messages, nested calls from hooks, subclasses, and
  that machines in reference cycles are collected.

Mutation-checked: five deliberately wrong builds (addiu zero-extending, a
likely branch not annulling, a pmaddh lane swap, a vopmsub lane, a one-bit
rsqrt parity slip) are each caught.

## Speed

`python3 tools/test_eemu_rs.py bench` runs the same workloads on each alone
and compares the results:

| Workload | eemu | eemu_rs | |
| --- | ---: | ---: | ---: |
| world: 200 frames of Kite walking (cameraMain, ccPlayer::Main) | 40.52 s | 0.16 s | 247x |
| world: Field() set-up (mostly DATA.BIN and animations in Python) | 3.74 s | 1.79 s | 2.1x |
| anim: controllers and 40 playback steps, 2 animations | 12.11 s | 0.79 s | 15x |
| stream: stream 106, 3 frames, test_stream_rs's cop2 override | 5.34 s | 0.06 s | 92x |
| stream: the same on eemu_rs.Vu0Machine | 5.37 s | 0.05 s | 103x |
| raw: libm's sinf in a loop | 0.66 M instr/s | 74.3 M instr/s | 112x |

Every row gave the same results on both. Hook-heavy code (anim: a Python
malloc and ring-buffer read every few hundred instructions) gains least;
long stretches of game code gain most.
