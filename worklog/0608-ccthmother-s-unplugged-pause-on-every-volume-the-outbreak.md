---
number: 608
title: ccThMother's unplugged pause on every volume, the Outbreak icon static proven unwritten over the whole disc, exact floats in the field UI harnesses
date: 2026-11-16
area: ui, engine, volumes, tooling, test
files: tools/refscan.py, crates/piney-demo/src/pause.rs, crates/piney-demo/examples/pause_probe.rs, tools/test_mother_pause_rs.py, crates/piney-game/src/session.rs, crates/piney-game/src/mode.rs, crates/piney-event/src/vm/exec.rs, crates/piney-desktop/src/savesys.rs, crates/piney-fieldui/src/menus/drain.rs, crates/piney-gen/src/manifest.rs, tools/test_fieldui_rs.py, crates/piney-fieldui/examples/fieldui_probe.rs
started: 2026-11-16T09:46:27-07:00
took: 4h 39m
---

# 608. ccThMother's unplugged pause on every volume, the Outbreak icon static proven unwritten over the whole disc, exact floats in the field UI harnesses

## What the review asked

The review of [[607]] asked for three things before the merge. First, prove
over the whole disc that nothing writes the static that Outbreak's and
Quarantine's `Disp` reads for the party panels' condition icons. The
harness agreeing with the port proved nothing, because its memory never set
that static either. Second, compare the field UI harnesses' floats exactly
instead of within 1e-3. Third, port `ccThMother`'s pause for an unplugged
controller on every volume, with a test against the game.

## The condition icons' static

On Outbreak, `Disp` reads the icons' y with `lui $at, 0x72; lw $v0,
0x0c90($at)` (gcmn 0x00539ad4) and adds the panel's shake. Quarantine does
the same at 0x0042c3e8, from 0x00619690. Infection and Mutation add a
literal instead (`addiu $v0, $s2, 320`, INF gcmn 0x00520a9c, MUT
0x0053ec3c), so no static of theirs matches it.

The word sits in a run of 4-byte statics, each in its own 8 bytes, starting
at 0x00720c38: 512.0, 5400, 450, 300, 300, 5400, 450, 900, 900, 32, 32, 0.
The run comes after the jump tables `@1486` and `@2983` and before the
"PERSONAL..." literals. It is in GCMN's `.data` (0x005fd580 to 0x0073d800),
which `mwLoadOverlay` fills from the PRG.

`tools/refscan.py` lists every way a volume's code or data can reach an
address range:

- Code: main and the four overlays' text, every word decoded. From each
  `lui`, the address registers are followed for 128 instructions through
  `addiu`/`ori`/`daddiu` and moves. It lists every load, store or address
  built near the range, every address handed to a call in `$a0`-`$a3`
  (memset, memcpy, a file read), and every indexed access in the form the
  compiler emits (`lui; addu index; lw %lo(...)`) from a base up to 8 KB
  below. It checks `$gp`-relative accesses too, which cannot reach this
  range.
- Data: every aligned word of every file on the image, with `DATA.BIN`'s
  and the STREAM archives' members inflated, that equals an address near
  the range.

The scan covered 1440 files and members on Outbreak (around 0x00720c90) and
1519 on Quarantine (around 0x00619690). It found:

- Loads only. `Disp`'s load is the only one of the word itself.
- Two indexed loads. These are the jump tables, bounded by `sltiu` at 6 and
  21 entries, ending at 0x00720c34.
- No store, no address taken and no call argument.
- No word of the executable or an overlay that points there. The 114
  (Outbreak) and 9 (Quarantine) data words that do are in audio, movies
  and the archives' compressed bytes, by chance.

The first version of the scan missed the `lui; addu; lw %lo` form. It only
flagged an `addu` on a register that already held an address near the
range. With that form added, the two jump tables appeared and nothing else
did. So the value is 0 from boot to power-off. The docs carry the proof
(field-ui.md, "The condition icons' static").

## Exact floats in the field UI harnesses

This is commit a9eebc2. The game's side now logs these floats as their
bits: a sprite's position, size, centre and turn, a message text's place,
and the minimap's alpha. `fieldui_probe` prints the port's values as bits,
and `close()` is plain equality. All seven field UI harnesses pass on all
four volumes with it; nothing needed a tolerance.

## ccThMother's pause

`ccThMother` (main, INF 0x00167d28-0x00168018; MUT 0x00167ea8, OUT
0x00167130, QUA 0x00166fc0) is the same code on every disc, except for the
START line's text: Infection's reads "Retun".

`piney_demo::pause` is its state machine:

- the flips and sleeps at the start;
- the fade;
- the two texts on `fontSys`, as `SetType` leaves it;
- START's `DeleteFade` and `ccWakeAllThread`;
- the frame after START.

The texts are read from each executable as `pause_texts`: the two
`MakePacketStr` arguments in the function's body.

`tools/test_mother_pause_rs.py` runs each volume's loop in eemu, entering
the loop head through a trampoline. Frame by frame it compares the calls,
the texts (floats as bits), `game.fade`'s elements, which tasks run, the
still layer's flip flag and the event task's mask. The scenarios are:
unplugged and back, the pause held off by `pauseFlag`, and six random runs
of 60 frames. OK on all four volumes.

### When the test runs

The set-ups run inside the mother (`ccSetupDesktop`, `ccSetupToppage`,
`ccSetupGameCtrl` at 0x00167cc0-0x00167cf0). So the pause can start only
once a set-up has returned:

- On the desktop and the top page, that is the frame of
  `ccEnableThEvent(4)`.
- In The World, the set-up first fades in on `game.fade`
  (0x00169538-0x0016957c, ten frames), then calls `ccSndBgmCtrl`.

The modes report this as `Mode::in_play`.

The port tests at the start of a frame, on the state the last frame left.
The game can begin the pause in the very frame a set-up returns. The port
begins it one frame later, after that frame's tasks have run. The docs
state this.

### The flag's writers

`game.pauseFlag` is one word that several places write. A scan of every
store to `game+0x7c` through the `game` pointer, on all four volumes, found
these writers:

- `staff_roll` sets 1 at its start and 0 after the roll and
  `ccBreathThread(30)`. Quarantine has no roll, so it sets and clears the
  flag in the same frame.
- Quarantine's `ending_kanji` sets 1 and never clears it.
- `ccThGameCtrl` sets 1 at the game over.
- From Outbreak, `ccSaveSys`'s requests write it. `SaveSys` already
  modelled this; it now reports the writes.
- On Mutation, `MainProccess` sets 1 and 0 around its card calls, within a
  call that takes no frames in the port.
- From Outbreak, Data Drain sets 1 at step 0, clears it after the rules,
  and clears it again at step 20.

The session keeps the flag. After each frame it takes the mode's last
write, and `ccGame::Init` clears the flag at the soft reset.

The field UI harnesses now compare `game+0x7c` every frame. With Data
Drain's write turned off in the port, `test_data_drain` and
`test_drain_movie` fail on Outbreak. As ported they pass, as do the
Recorder's save pages.

### Held frames

On a held frame no mode steps. The session draws the last frame it showed,
then `game.fade`'s elements and the texts over it. The play time still
counts, because `ccAddPlayTime` is part of main's loop, not a task.

Session tests:

- The desktop holds for 30 unplugged frames, drawing the fade and five
  cells.
- With the controller back, the START line's cells are drawn too.
- START's own frame is still held; the desktop runs after it, and the
  pause's fade is deleted.
- With `pauseFlag` set, nothing pauses.
- Mac Anu's set-up is not yet a point where the pause can start. Once it
  has returned, the town holds with Up pressed until START.

Unit tests:

- the texts on each volume;
- the state machine;
- `staff_roll`'s writes on each volume: frames 0 and 32 when the roll is
  seen done on its third check, frames 0 and 0 on Quarantine;
- `ending_kanji`'s write.

## Checks

At b0f33ab:

- Targeted harnesses on all four volumes:
  - OK: `test_mother_pause_rs`, `test_fieldui_rs` (5 skipped, as in 607),
    `_shop_rs`, `_personal_rs`, `_talk_rs`, `_trade_rs`, `_option_rs`,
    `test_chat_msg_rs`, and `test_desktop_savemenu_rs` on Infection,
    Mutation and Outbreak.
  - `test_desktop_savemenu_rs` fails 2 on Quarantine, as in 607.
  - `test_desktop_data_rs` fails 3 random-pad scenarios on Quarantine. It
    fails the same way on main (2d2b796), and this branch did not touch it.
- The sweep: 308 runs (the 304 of [[607]] plus `test_mother_pause_rs`). 23
  runs fail here, against 31 on main.
  - Newly passing against main: the field UI's eight runs and
    `test_desktop_savemenu_rs` on Outbreak.
  - Quarantine's `test_desktop_savemenu_rs` goes from 3 failures to 2.
  - Against 607's sweep, `test_effect_draw_rs` is back to OK on all four.
  - Nothing else changed.
- Battle pins: 291 calls, 0 differ (probe at b0f33ab).
- Golden shots against today's base from 2d2b796: 4,550 files, 22 differ,
  all `macha_shots` (the condition icons of [[607]]). Against 607's
  `fieldui-tip` shots, 0 differ.
- Tests:
  - Crate tests: 45 ok.
  - piney-demo and piney-input: ok.
  - The piney-game suite: 349 passed, 0 failed, 153 ignored.
- `gen --check`, and `syms --check` and `carry --check` on all three later
  volumes: current.
- clippy, fmt, `cairns check` and `docs.py check`: clean.

**Still unknown:**
- Why Quarantine's Data screen differs in `test_desktop_data_rs`'s random
  pads 2-4. The port plays sound 4 and shows another message code (0x800b
  against the game's 0x100a) where the game shows a card message. This
  predates this work: main fails the same way.
- Why `test_desktop_savemenu_rs` still fails twice on Quarantine. The
  overwrite's menu does not close.
- How a player would trigger the pause. `Pad::unplugged` is set only by
  tests and harnesses; the front end always reports the keyboard as a
  controller. Whether a disconnected gamepad should count as no controller
  is a design question for the player app, not the game's.
