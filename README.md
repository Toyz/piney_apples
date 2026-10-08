# piney_apples

A native port of the engine behind the four .hack games for the
PlayStation 2 (CyberConnect2 / Bandai, 2002-2003): .hack//Infection,
.hack//Mutation, .hack//Outbreak and .hack//Quarantine. It is written in Rust
from a reverse engineering of the games, and it plays your own discs.

**Status: work in progress.**

| part | disc | where it stands |
| --- | --- | --- |
| .hack//Infection | `SLUS_202.67` | Playable from power-on to the ending: the whole story, the staff roll and the save after it |
| .hack//Mutation | `SLUS_205.62` | The whole story, all 16 events, plays through under the test autopilot from the new game to the ending; not yet played through by hand |
| .hack//Outbreak | `SLUS_205.63` | The whole story, all 19 events, plays through under the test autopilot from the new game to the ending and staff roll; not yet played through by hand |
| .hack//Quarantine | `SLUS_205.64` | Boots: the title, the opening cut scenes with their sound, the desktop, the Root Towns; its story comes after Outbreak's |

What is still missing is listed in [GAPS.md](GAPS.md), and the known bugs in
[BUGS.md](BUGS.md).

## Not an emulator

An emulator runs the PlayStation 2's code: it imitates the console's
processors and the game runs as it did on the hardware. This project runs
none of that code.

- **What runs** is the engine rewritten in Rust, part by part: the game
  loop and its tasks, the event-script interpreter, the battle rules and the
  AI, the Chaos Gate's area-word generator, the fields, dungeons and Root
  Towns, the menus, the ALTIMIT desktop, the in-engine cut scenes, the
  sound driver and synthesizer, and the movie decoder. The picture is drawn
  with wgpu from the port's own draw lists, and the sound is mixed by the
  port.
- **How we know it matches.** Each part is checked against the game's own
  code: the original functions are run one instruction at a time in a
  small interpreter written for this project, and the port has to give the
  same results. More than 850 tests hold it to that.
- **What it reads** is your discs, once. `piney-build` reads them into a
  build and generates the tables the engine needs from the discs' own
  executables. During play nothing from the PlayStation 2 runs, and no BIOS
  is needed.

## What owning the engine gives you

The games' systems become ordinary source code, so you can read them, change
them and build on them. You do not have to patch a binary.

**Today:**

- **One program for all four parts.** One build holds your discs and one
  memory card serves all four parts. It opens on the four-part selector
  that Outbreak's disc carries but never uses.
- **Every Root Town from any disc.** The console's `town N` reaches Mac
  Anu, Dun Loireag, Carmina Gadelica, Fort Ouph and Lia Fail from any part.
- **A console** (F1 or `` ` ``) into the running game. It has god mode, items
  and gold, warps to any town, and any character into the party whatever
  the story says. It also sets event flags and the bracelet's infection,
  and restarts the game at any story event (`help` lists them all).
- **A scriptable, repeatable game.** The same input gives the same game.
  - `--pad-log FILE` records your pad from power-on, or type `pad_log` in
    the console at any time to write the run so far and go on recording;
    `--replay FILE` plays either back.
  - `--press` scripts button presses and `--console` scripts console commands.
  - `--mode story:N` starts at story event N.
  - `--shot` and `--webp` capture frames and video without a window.

  The test suite plays the story this way and checks what the game draws
  and plays.
- **A viewer** (`piney-viewer`) for every scene on the discs: models,
  animations, towns as they are assembled, and any dungeon or field from
  its seed.
- **Unused content, reachable.** Outbreak's unused cut scenes play with
  `--mode loose:STREAM/STRT.BIN:N`. The earlier builds of the later towns
  left in Infection's executable are documented.
- **Documentation for everyone.**
  - [docs/](docs/README.md) is the reference for the formats and the engine.
  - [WORKLOG.md](WORKLOG.md) records how each piece was worked out.
  - `piney-build --export-symbols DIR` writes the function and data names of
    all four executables for other reverse engineers.

**Possible because of it** (not done yet):

- Changing the rules in code: the battle, the AI, items and skills, the
  area words, the story's events. The event scripts already have a
  readable text form of their own.
- Fixing the originals' bugs, or keeping them. Either way it is a choice
  made in code.
- Drawing beyond the PS2, such as other resolutions and aspect ratios,
  since the picture is the port's own.
- Running on any platform that Rust, wgpu and cpal reach.

## Getting started

The game is built from source on your own computer, then made from your own
discs. It takes four steps the first time; after that, updating is two
commands.

### What you need

- **Your own disc images** (`.iso`) of the North American releases:
  Infection `SLUS_202.67`, Mutation `SLUS_205.62`, Outbreak `SLUS_205.63`,
  Quarantine `SLUS_205.64`. One disc is enough to play that part. Other
  regions are not supported.
- **A graphics card with Vulkan, Metal or DirectX 12**: most from the last
  ten years, with current drivers.
- **Disk space**: about 2 GB per disc for the game, and a few GB more for
  compiling.

### 1. Install the tools (once)

You need Git and Rust (the current stable version).

- **Windows**: install [Git for Windows](https://git-scm.com/download/win)
  and run `rustup-init.exe` from [rustup.rs](https://rustup.rs). When it asks,
  let it install the Visual Studio C++ build tools. Use the commands below
  in PowerShell or the "Developer PowerShell".
- **Linux**: install Git, a C compiler, `pkg-config`, and the ALSA (sound)
  and udev (gamepads) headers, then Rust from [rustup.rs](https://rustup.rs):
  - Debian, Ubuntu, Mint: `sudo apt install git build-essential pkg-config libasound2-dev libudev-dev`
  - Fedora: `sudo dnf install git gcc pkgconf-pkg-config alsa-lib-devel systemd-devel`
  - Arch: `sudo pacman -S git base-devel alsa-lib systemd-libs`
  - then: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **macOS**: run `xcode-select --install`, then install Rust from
  [rustup.rs](https://rustup.rs).

### 2. Download and compile (once)

```sh
git clone https://github.com/Toyz/piney_apples.git
cd piney_apples
cargo build --release
```

The first compile takes a while. The programs end up in `target/release/`
(`.exe` on Windows).

### 3. Make the game from your discs (once)

Give `piney-build` your disc images, or a folder holding them:

```sh
# Linux and macOS
target/release/piney-build ~/isos/infection.iso ~/isos/mutation.iso

# Windows (PowerShell)
target\release\piney-build.exe "D:\isos\infection.iso" "D:\isos\mutation.iso"
```

It tells the discs apart by their contents, checks every file it reads,
and prints where the game went:
- Linux: `~/.local/share/piney/game`
- Windows: `%APPDATA%\piney\game`
- macOS: `~/Library/Application Support/piney/game`

Got another disc later? Run it again with just that disc: the ones already
in are kept. Your disc images are not needed after this step.

### 4. Play

```sh
target/release/piney-game           # Windows: target\release\piney-game.exe
```

With more than one part made, it opens on the four-part selector;
`--volume 1` (2, 3, 4) starts a part directly. You can also play one image
without step 3: `piney-game --iso path/to/infection.iso`.

On a console the game softens the picture for an interlaced TV, mixing
each line with the one above. The port shows it sharp, as PCSX2 does by
default. `--deflicker` (or `deflicker on` in the console) shows it as the
console did.

The picture can be drawn sharper than the PS2's own resolution:
`--render-scale N` (or `render_scale N` in the console) draws at N times
it, 1 to 8. `--no-vsync` (`vsync off`) stops waiting for the display's
refresh, and `--fps-cap N` (`fps_cap N`, 0 for none) limits the pictures a
second. The game itself runs at its own rate whatever these are.

`--hud-scale N` (`hud_scale N` in the console, 0.5 to 1) draws the HUD
smaller: the party panels toward the bottom left corner, the map toward the
top right, the target's window toward the top left. Render scale does not
do this: it adds detail, while the picture, HUD included, always fills the
window as the PS2's did.

### Updating

```sh
git pull
cargo build --release
```

If the game then says `no port data of version N ... run piney-build
again`, run `target/release/piney-build` with **no arguments**. It remakes
the game from the discs already in it; your saves are not touched.

### If something goes wrong

- **It will not start, or says no graphics adapter**: update your graphics
  drivers. On Linux, make sure the Vulkan driver for your card is
  installed (`mesa-vulkan-drivers` on Debian and Ubuntu, `vulkan-radeon` or
  `vulkan-intel` on Arch).
- **No sound**: the game plays on your system's default output device. Check
  it there, and that the in-game Option menu's volumes are up.
- **A gamepad does nothing**: plug it in before starting the game, then press
  a button on it (the game reads the pad last used).
- **Messages**: the game's warnings and errors go to the terminal it was
  started from, and also to the console (F1). `PINEY_LOG` picks how much
  is shown: `PINEY_LOG=debug piney-game` for everything, or one part, e.g.
  `PINEY_LOG=piney_world=debug,warn`. The default is `warn,piney=info`.
- **Anything else**: see [Reporting a bug](#reporting-a-bug) below.

### Controls

| PS2 pad | keyboard | gamepad |
| --- | --- | --- |
| D-pad | arrow keys | D-pad |
| Cross / Circle | Z / X | south / east button |
| Square / Triangle | A / S | west / north button |
| L1 / R1 | Q / W | shoulders |
| L2 / R2 | 1 / 2 | triggers |
| L3 / R3 | C / V | stick clicks |
| Start / Select | Enter / Backspace | Options / Create (or Start / Back) |
| left stick | (none) | left stick |
| right stick (camera) | I / J / K / L | right stick |

Any gamepad that [gilrs](https://gitlab.com/gilrs-project/gilrs) knows
works, with its buttons where a DualShock 2 has them. With more than one
connected, the one that last sent input is read. Escape asks to quit, and
F1 or `` ` `` opens the console. Its history scrolls with Page Up / Page
Down, the mouse wheel or the scroll bar on its right; Tab completes a
command.

### Reporting a bug

Bugs go to [the issues page](https://github.com/Toyz/piney_apples/issues):
say which part (Infection, Mutation...), where you were, what you did, and
what you expected. Give the build too: it is in brackets after "Ported by
helba" on the disc menu, the first line `piney-game` prints, and the
console's `version` (a 7-digit commit such as `fc82e63`; "unknown" for a
tree downloaded as a ZIP, so clone with git if you can). A screenshot
helps; a recording helps most:

Open the console (F1 or `` ` ``) and type `pad_log`. The game has been
keeping every frame's pad since power-on, so this writes the whole run so
far and keeps recording. It prints where the file went: `padlogs/` in the
build's folder, next to a `.card` folder with your memory card as it was
at power-on. Play until the bug shows, then type `pad_log stop`. Attach
the log and the `.card` folder (zipped) to the issue. `piney-game --replay
FILE` plays the run back exactly, console commands included.

### Saves and settings

Saves go to one memory card in the port's folder, shared by all four
parts: `memcard/slot1` beside `game` (`%APPDATA%\piney\memcard\slot1` on
Windows, `~/.local/share/piney/memcard/slot1` on Linux). Each part keeps
its own save folder there (`BASLUS-20267DOTHACK` is Infection's,
`BASLUS-20562DOTHACK` Mutation's, `BASLUS-20563DOTHACK` Outbreak's,
`BASLUS-20564DOTHACK` Quarantine's). The options (volumes, voice language,
screen position, vibration, the message window) are kept in
`settings.toml` in the port's folder. Whatever any part's Option menu
sets, every part starts with.

A part's clear data carries into the next as on the PS2: save after the
ending's staff roll, then choose CONVERT on the next part's title and the
cleared save. Leave the earlier part's folder where it is; the next part
reads it there, and needs no save of its own first. Copying slot files
into another part's folder does not convert them.

Saves from PCSX2 can be brought over. Point the port at PCSX2's memory
card file (`Mcd001.ps2` in PCSX2's `memcards` folder), at a folder memory
card, or at one exported save folder such as `BASLUS-20267DOTHACK`:

```
piney-game --import-card path/to/Mcd001.ps2
```

or type `import_card path/to/Mcd001.ps2` in the console (F1 or `` ` ``).
The four parts' save folders are copied onto the port's card. A save it
replaces is kept in a `backup-...` folder beside the card. Only the North
American discs' saves are recognised.

A single slot file (`dhdata01` to `dhdata12`, or a folder of them) can be
imported the same way. A slot shows as used only through its entry in its
folder's index file, so copying the file in by hand leaves it "unused";
the import writes that entry for it. A later part's slot file needs the
part named (`--volume 2`, or the console of a running game).

This repository does not include the games. Everything the port plays
comes from your discs.

## For developers

The port is a Cargo workspace under [`crates/`](crates/):

| crate | what |
| --- | --- |
| `piney-data` | the disc: ISO 9660, the `DATA.BIN` gzip archive, CCSF scene files - textures, palettes, models, clumps, animation playback - and the engine's generators (towns' placement tables, dungeons, fields) |
| `piney-viewer` | every `DATA.BIN` scene textured, posed and playing, towns as their placement tables assemble them, generated dungeons and fields, with mouse, keyboard or a gamepad; `--shot` renders one to PNG without a window |
| `piney-input` | the pad as `ccPad::Read` reads it: held, pressed, released, repeat, the stick as a direction |
| `piney-draw` | one frame as the game hands it to the GS: primitives, model draws, run-time textures, blend and test state |
| `piney-gs` | draws those frames as the GS does, on wgpu, in the game's 512 x 448 frame buffer |
| `piney-game` | the game from power-on: the title with its movies and intro stream, New Game, the desktop with the game's own event scripts, the Root Towns, fields and dungeons, their cut scenes, voices and sound; modes on the game's own frame clock, pad or keyboard in, GS frames out |
| `piney-world` | The World: the Root Towns, fields and dungeons as their tasks run them, the camera, the characters, the party and its AI |
| `piney-battle` | the battle's rules: stats, hits, skills, items, conditions, the enemies' and members' AI, Data Drain |
| `piney-effect` | the effects: skills, spells, transfers, the particles |
| `piney-fieldui` | the field's menus and HUD |
| `piney-audio` | the game's sound: sound effects and the sequenced music through ports of the IOP's MIDI sequencer and hardware synthesizer, an SPU2 model mixing at 48 kHz, `BGM.BIN`'s streams; cpal output or headless WAV |
| `piney-desktop` | the ALTIMIT desktop (`DESKTOP.PRG`) as a state machine: the opening, the main screen, the mailer, News, Accessory, Audio and Data (saving to a memory card), the event scripts' message windows and the START menu |
| `piney-demo` | the title screen (`DEMO.PRG`) as a state machine: the memory-card check, the logo movies and intro stream, the menu (New Game, Load, Option), and New Game's hand-off to the desktop |
| `piney-toppage` | THE WORLD's top page: the board, the news and the log in |
| `piney-mpeg` | the PSS movies: our own MPEG-2 decoder and the IPU's colour conversion, every picture exact against ffmpeg, and the movies' SPU2-format sound |
| `piney-event` | the event scripts: our own instruction set and its text form, the scripts and messages read from the disc and converted without loss, and the interpreter checked against the game's own code |
| `piney-stream` | the in-engine cut scenes (`ccRequestLoadStream`): a stream's files read from `STREAM/*.BIN`, its scenes built and played frame by frame as the game's `InitScene` / `DecodeFrameSection` do them, pad in, draw list and PCM out |
| `piney-build` | one game from your discs: the disc images' files cut into chunks, each kept once and zstd-compressed, into a build `piney-game` finds by itself; the port's data generated from the discs into it |
| `piney-gen` | the table generator: each volume's engine tables read out of its executable through Infection's DWARF types, into the build; the carry that finds Infection's globals in the later volumes; and the symbol transfer that names the stripped executables |
| `piney-eemu` | the EE interpreter the checks run the game's own code in |

```
cargo run --release -p piney-viewer -- town01
cargo run --release -p piney-viewer -- --dungeon 1234,0,5
cargo run --release -p piney-viewer -- --field 777,2
cargo run --release -p piney-game -- --mode story:11      # the game at story event 11
cargo run --release -p piney-audio --example render -- desktop 50 out.wav
cargo run --release -p piney-stream --example stream_shot -- --stream 2 --frame 600 --out stream.png
cargo test --workspace      # the checks; the disc tests need the images under work/
```

For the checks and the tools, put your disc images in `originals/`
and extract them under `work/` (both are ignored by git).
[docs/](docs/README.md) is the reference: what the formats are and how the
engine works, with an honesty status on every page.
[WORKLOG.md](WORKLOG.md) is how each piece was worked out.

### Tools

The reverse engineering started on a machine with no tools for it, so they
are written here as the work needs them, under [`tools/`](tools/), in Python
with nothing outside the standard library:

| tool | what it does |
| --- | --- |
| `tools/iso.py` | list, extract and cat files from an ISO 9660 DVD image; map an LBA back to a file |
| `tools/elf.py` | the EE executable: headers, sections, symbols, reads by VA |
| `tools/mips.py` | R5900 instruction decoder: base MIPS, EE extensions, MMI, FPU, VU0 macro mode |
| `tools/image.py` | the program image: main plus one overlay, symbols and relocations; a stripped volume's `.syms` sidecar |
| `tools/disasm.py` | disassemble a function or range, exact xrefs from the relocations |
| `tools/fdtbl.py` | the DATA.BIN index compiled into the executable; `check` verifies it against the archive |
| `tools/gzarc.py` | list, cat and extract the sector-aligned gzip archives (`DATA.BIN`, `STREAM/*.BIN`) |
| `tools/ccs.py` | CCSF scene files: header, name table, objects, chunk walk |
| `tools/dwarf1.py` | the DWARF 1 debug information: functions, types, globals, lines, per-unit headers |
| `tools/demangle.py` | CodeWarrior C++ symbol demangler, works like c++filt |
| `tools/ccstex.py` | textures and palettes out of CCSF files, as PNG |
| `tools/ccsmodel.py` | models out of CCSF files as Wavefront OBJ, optionally posed from an animation or a cutscene frame |
| `tools/areas.py` | the Chaos Gate keywords, the story areas, and the area generator reproduced exactly |
| `tools/tables.py` | any global table decoded through its DWARF type: enemies, skills, items, equipment, bosses |
| `tools/sound.py` | sound banks, voice lines, streamed music and cutscene audio, through the game's own indexes |
| `tools/scei.py` | Sony's SCEI .hd / .sq sound-bank formats |
| `tools/adpcm.py` | PS-ADPCM decoder |
| `tools/midi.py`, `tools/hsyn.py` | models of the IOP's MIDI sequencer and hardware synthesizer, checked against the modules |
| `tools/irx.py` | IOP module (.IRX) disassembler: imports, exports, functions |
| `tools/iopemu.py` | IOP modules run in `eemu.py`, the synthesizer's register writes recorded |
| `tools/sound_ee.py` | the game's EE sound code run in `eemu.py`, what it sends to the IOP recorded |
| `tools/wav.py` | WAV writer |
| `tools/vu.py` | VU0/VU1 microcode: labels, disassembly, and the game's model packets built in the interpreter |
| `tools/vif.py` | VIF code streams, DMA tags and GIFtags |
| `tools/evscript.py` | event scripts: list, disassemble with dialogue and voice inlined, survey, check against the game code |
| `tools/dungeon.py` | dungeon layouts from the area seed, reproduced exactly, as ASCII maps |
| `tools/battle.py` | the battle rules reproduced: effective stats, hit and damage, protect break, status, experience, Data Drain |
| `tools/field.py` | field terrain and objects from the area seed, with PNG height maps |
| `tools/anim.py` | Anime chunks evaluated at any time as the game plays them: poses, texture offsets, morphs |
| `tools/png.py` | minimal PNG writer |
| `tools/eemu.py` | a small EE interpreter: runs static initialisers, and game functions to check reimplementations against |
| `tools/font.py` | text rendering: glyph tables, measuring, and strings drawn to PNG as the game draws them |
| `tools/text.py` | the desktop's mail, bulletin board and news text, English or parody |
| `tools/portdata.py` | the port's data files (`work/data`, the build's `PINEY/TABLES`) read back for the other tools |
| `tools/save.py` | the memory-card save as each volume's own code writes it, and the carry-over into the next volume |
| `tools/xfer.py` | code compared across volumes: a body with its address fields masked, the addresses it forms, its opcode shape |
| `tools/docs.py` | regenerate and check the reference index |

## License

The code in this repository is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

The license covers this repository's own code and writing only. The games
themselves, their data, text, art, sound and code, belong to their owners
and are not covered by it. .hack is a trademark of its owners; this project
is not affiliated with or endorsed by CyberConnect2 or Bandai Namco.

### Use of AI

This project is built with AI coding agents: Claude (Anthropic), through
Claude Code. The split is:

**The maintainer (Toyz)**
- defined the system: a port, not an emulator; one core for all four
  volumes; the crates and how they fit; data made from your own discs,
  never shipped;
- defined the rules the work is held to: every behaviour bit-exact to the
  game and checked against its own code; idiomatic Rust; typed tables, no
  byte blobs; no shortcuts or cheats in the tests; unknowns closed, not
  listed;
- set the requirements: what each piece must do, and when it is done;
- understands every piece of the agents' work and can audit any of it: the
  reverse engineering, the code, the tests and the harnesses;
- decides what is worked on and in what order, and what ships;
- plays the port, with a pad, and finds and reports what is wrong;
- triages the players' bug reports on GitHub and hands them on;
- reviews the work and sends it back when it falls short of those rules;
- wrote [cairns](https://github.com/Toyz/cairns), the worklog tool below.

**The agents**
- do the reverse engineering: they read the game's code with the tools
  under `tools/` (which they also wrote) and work out what it does;
- write the port's code, its tests and the harnesses that run the game's
  own code beside it;
- write the reference docs and every worklog entry;
- reproduce reported bugs, fix them, and draft the replies on the issues.

Nothing here is taken on the model's word. Each ported piece is checked
against the game's own code: the original routines run in an EE interpreter
(`piney-eemu`, `tools/eemu.py`) beside the port, and their results are
compared frame by frame or bit for bit. Those comparisons are part of the
test suite. Where something cannot be checked that way, the docs and the
worklog say so.

Keeping that record is what [cairns](https://github.com/Toyz/cairns) is for.
Every unit of work is a numbered worklog entry: what was found, how, what was
checked against the game, and what is still unknown. The [worklog](WORKLOG.md)
is the full history of how the port was worked out, AI included. If you want
to know how something here came to be, or how far to trust it, start there.
