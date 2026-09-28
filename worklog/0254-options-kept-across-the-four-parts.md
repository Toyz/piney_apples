---
number: 254
title: Options kept across the four parts
date: 2026-09-27
area: save
files: crates/piney-game/src/settings.rs, crates/piney-game/src/session.rs, crates/piney-game/src/main.rs, crates/piney-game/src/launcher.rs, crates/piney-demo/src/lib.rs, README.md
---

# 254. Options kept across the four parts

Asked for from play: settings should carry across the volumes instead of
being "a janky mess from options". Each volume keeps its options in its
own save's option block, which the title's Option page and the START
menu's write. A part started fresh therefore came up at a new save's
volumes and voice, and a save made in another part brought its own.

## The option block

These are the fields the Option pages write (`ccSaveData`, from
`dtmenu`'s and the field menus' writes):

| field | offset | size |
| --- | --- | --- |
| `mainVol`, `seVol`, `bgmVol`, `output` | +0x841e, +0x8420, +0x8422, +0x8424 | short |
| `screenX`, `screenY` | +0x841a, +0x841c | short |
| `drainDemo`, `vibration`, `camType` | +0x8427, +0x8428, +0x8429 | byte |
| `voice`, `mapMode` | +0x842c, +0x842d | byte |
| `strWinMode`, `cameraMode` | +0x8430, +0x8431 | byte |

`cameraMode` (L2's field camera) and `mapMode` are the field's own
switches, not a page's, but they are the player's preference too. The
button assignment (`assignPADok` and `assignPADcancel`) is not an option
the menus offer, and it is not kept.

## What the port does

`crate::settings` keeps one copy in `settings.toml` in the port's folder:
one `name = number` a line, each the save's own value.
1. **On boot.** The title's new save takes the file's values
   (`boot_title`), over its own for the names the file has.
2. **On Load.** The title's Load, once it has read a slot, puts the kept
   options over the loaded save's, before `SetSoundEnv` sends the volumes.
   `Demo::save_mut` gives the port that access.
3. **Each frame** (`Session::keep_settings`), the session reads the
   current save's options. On the first frame of a new mode it puts the
   kept ones into that mode's save, since a set-up may have made its own.
   Otherwise, a difference means a menu changed them, and the file takes
   them.
4. **The launcher** starts the title's music at the kept main volume.

Only a window keeps the file. `Options::settings` is None for a headless
run (`--shot`, `--webp`), and the tests make sessions without it, so
neither touches a player's options.

`options_round_trip` checks the file both ways.
`options_kept_across_the_parts` checks a session end to end:
- Infection's title comes up with the file's volume (100) and voice (1);
- the save's volume set to 180, as a menu would, is in the file on the
  next frame, with the voice kept.

**Still unknown:** Whether a player wants a save's own options to win when
loading a save from a real card made elsewhere. The port always prefers
the kept ones.
