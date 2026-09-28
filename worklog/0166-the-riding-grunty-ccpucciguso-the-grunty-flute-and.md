---
number: 166
title: "The riding Grunty: ccPucciguso, the Grunty Flute and pgRideFlag"
date: 2026-09-26
area: world, battle, audio, ui, test
files: crates/piney-battle/src/ride.rs, crates/piney-battle/examples/ride_probe.rs, crates/piney-world/src/combat/ride.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/field_world/ride.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/area/ride.rs, crates/piney-game/src/area.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/main.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/session/tests/ride.rs, crates/piney-fieldui/src/menus/useitem.rs, crates/piney-fieldui/src/menus/keyitem.rs, crates/piney-fieldui/src/world.rs, crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, tools/test_ride_rs.py, tools/sound_ee.py, docs/engine/grunty-ride.md
---

# 166. The riding Grunty: ccPucciguso, the Grunty Flute and pgRideFlag

A GAPS item, and the piece worklog 0162 left for next. A grown Grunty
from Dun Loireag's pens earns the Grunty Flute (key item 49). Blown in a
field, it calls the Grunty for Kite to ride. In the port, Key Items asked
`ccPgAdultCheck` (always no answer: the runtime gave none) and pushed a
`UseItemArg` nothing carried out. Now the whole ride plays:

- the call;
- the fade, and the party asleep;
- riding with the stick: the camera, the acts, the footsteps, the dust;
- the dismount, with the members set down about Kite;
- the ride's music.

The page is `docs/engine/grunty-ride.md`.

## What the game does

- **The source.** `pgrider.cpp` (gcmn 0x00510880-0x005130ac, 17
  functions) holds all of it but `ccPgAdultCheck`, which ends
  `pgbreed.cpp`.
- **The call.** `ccPgAdultCheck(game.server, pen)` reads
  `growth[server].type[pen]`. It answers kind 0 for pen 0 and
  `2 server + pen - 2` for pens 1 and 2. `game.server` is the gate
  town's server (Mac Anu 0, Dun Loireag 1), so the flute finds Dun
  Loireag's Grunties only in Theta's fields. The session test's first
  attempt, from Mac Anu, found none.
- **The start.** `ccPuccigusoStart` runs on the menu task, which
  `ccUseItemRequest` holds through the whole ride:
  - it bans the menu and sets the flags;
  - `ccPgBgmInit` stops the voice and fades the sequences out;
  - it fades to black;
  - `ccSpcSleep` sleeps every registered character's task (Kite's
    `ccThPlayer` too), and each in the party loses its body and its
    condition effect;
  - it loads the kind's file and starts `ccThPucciguso` (priority 49);
  - it fades back in and plays `BGM.BIN` track 0.
- **The task.** `ccThPucciguso` runs `Main` 30 times, then looks for the
  cancel button at the top of each frame after the next.
- **`Main`.** It is `ccPlayer`'s frame reworked:
  - the move is eased (0.08 of the way a frame while leaning, 0.125
    back), which is why it glides to a stop;
  - it pushes out of bodies and walls, but a blocked move shrinks to half
    the way past the radius instead of stopping;
  - it writes `plw.pw`'s place and heading, so Kite goes where it goes;
  - Kite's riding clump is a second `ctu1body` `CMP_trall`, drawn by
    `ccChar::Draw`; the Grunty is drawn by `DrawPG`, the same test.
- **The Grunty's notes.** They play the dogs' sounds
  (`ccSeSetParamInu`). `PawSmoke` puts dust at a leg's node on the
  ground's attribute: none on five, type 4 on eleven, else 133. With
  several legs asked, each calls `rand()` but only the last is used.
- **The dismount.** `ccPuccigusoExit`:
  - fades out with `Main` still running;
  - puts each member but Kite 150 out at `puccigusoAngleTbl`'s angle
    (135 degrees either side behind);
  - wakes the party (`ccSPC::Wakeup`: act 0, flags, `ccEntryCmnd`) and
    sends 0x1000e on the AI bus, whose `ChatMessageQuitPucciguso` gives
    Orca's "This is a weird Grunty.";
  - fades back in, lifts the ban and brings the field's music back
    (`ccPgBgmEnd(0)`).
  With `pgDIN` set (a dungeon's way in ridden into) it only stops the
  music (`ccPgBgmEnd(1)`) and deletes itself.

## The port

- **The rules.** `piney_battle::ride` holds the object's functions over a
  `RideWorld`, plus `exit_place` and `adult_check`.
- **The frame.** `piney_world::combat::ride` holds the flags, the
  party's sleep and the object in `Combat::ride`. `ride::frame` runs in
  `Combat::frame` where `ccThPucciguso` does, and makes the object on its
  first frame. While the party sleeps, `kite::main` and the members'
  frames are skipped, so nothing draws them. The flag reaches the
  enemies' `selectTarget` and the party's AI.
- **The field.** `field_world/ride.rs` does the start's and the
  dismount's slices that touch the field. piney-game's `area/ride.rs`
  runs the sequence on `scFadeDef`, the menu and the music
  (`Event::PgBgm`).
- **The menu's side.** `WaitParty`, `Pucciguso` and `WaitRide` are menu
  waits now. Only the chat balloons draw during them, since
  `ccSprite::Trans` only loads textures.
- **The item's argument.** It now reaches `use_item_request`, so the
  flute calls the chosen kind; before, the port always passed 0.
- **Timing.** The port runs the menu task after the field's tasks, and
  the game runs it before. So the start's slices run before the menu's
  frame, and the ride's task before the field's. The menu sees
  `pgRideFlag` as the frame began. The ride's first `Main` comes one
  (black) frame late.

## Checks

- **`tools/test_ride_rs.py`.** Runs the game's code against the
  `ride_probe` example:
  - the tables;
  - the constructor for every kind;
  - `Main` for 1-80 frames on random states, compared after every frame:
    every member, `pgR`/`pgDIN`, `plw`, the camera's reset, `rand()`,
    and every call in order;
  - `ControlMove`, `AnimCtrl`, `DrawPG`, the note function and `PawSmoke`
    alone;
  - `PadLeverPower`, and `ccPgAdultCheck` over every server and pen;
  - `ccPuccigusoExit` and `ccPuccigusoStart` natively with their fades
    done at once: the members' places, and the order of calls the port's
    sequence follows.
  All 12 checks pass, with 400 cases each in bulk.
- **What the harness caught.** The partyFlag bits are 14-16, not 16-18
  (a harness slip). The Grunty's clip names depend on the kind's digit,
  which the constructor writes into a shared table. `PawSmoke` with no
  leg uses garbage, which the game never asks for.
- **The music.** `tools/sound_ee.py` runs `ccPgBgmInit` and
  `ccPgBgmEnd(0/1)` over fields of each play type: 9 new driver fixture
  lines (the fades out and back in through `ccSqPlayVol`) and 3 voice
  lines (the stop). The driver matches.
- **`tools/test_fieldui_personal_rs.py`.** The flute's frames are
  unchanged; all 17 tests pass.
- **Session.** `the_grunty_flute_rides_and_dismounts` plays a Theta field
  with the flute and a pen-1 Grunty:
  - PERSONAL, Key Items, the flute: the ban, `ccPgBgmInit`;
  - the fade: the party asleep, Kite's own model gone, the Grunty where
    he stood;
  - the music, and the kind's clips;
  - the stick: it runs (act 5), carrying Kite over 600 in 90 frames and
    across the map's edge;
  - let go: it stands;
  - Circle: `Main` through the fade, then the members 150 from Kite;
    then the flags clear and `ccPgBgmEnd(0)`; the menu shuts unbanned and
    Kite walks.
- **Shots.** `ride_shots` shows the Grunty with Kite on it, standing and
  running with its shadow, and after the dismount Orca's balloon "This
  is a weird Grunty.".
- **The rest.** The workspace's tests pass.

**Still unknown:** The ride's task runs a frame later than the game's, and so does the first look at the cancel button. The port sleeps and wakes the whole party, where `ccSPC::Wakeup` wakes only those in it (in Infection's fields the registry is the party). `plw +0x10` (the camera's eye) and `ccDamUprStr::CtrlAll` in the menu's waits are not kept. The legs' places come from the port's last pose, not from the node matrices the game's last draw left. A dungeon's way in ridden into (`pgDIN`) is not played through in a session.
