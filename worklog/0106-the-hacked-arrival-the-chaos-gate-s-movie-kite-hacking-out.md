---
number: 106
title: The hacked arrival: the Chaos Gate's movie, Kite hacking out, and the party hidden until he has
date: 2026-09-25
area: world, video, test
files: crates/piney-world/src/combat/gate_out.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/combat/cast.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/body.rs, crates/piney-battle/src/kite.rs, crates/piney-stream/src/table.rs, crates/piney-stream/src/lib.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/session.rs, crates/piney-game/src/stream.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/gate_hack.rs, crates/piney-world/examples/world_probe.rs, tools/test_gate_out_rs.py, tools/test_stream_rs.py, docs/engine/field-walk.md, docs/engine/stream.md
---

# 106. The hacked arrival: the Chaos Gate's movie, Kite hacking out, and the party hidden until he has

The gate hack's menu (menu 62) ends with two writes before the warp:
`ccGame.setupMode = 1` and `ccSetGtHack()`. Until now the port dropped
both, and the party arrived in the protected area as from any warp. This
entry follows the two flags to everything that reads them.

## Who reads the flags

**`setupMode`** (`ccGame` +0x50) is read by one place.
- `ccSetupGameCtrl` starts stream 107 after the sound bank loads, and waits
  for the reader to pause.
- Then it loads the field's files, calls `ResetPause`, and waits for the
  stream to end.
- Only then does it start the field's tasks.
- A scan of every store to `game+0x50` finds two: `GtHackMenu`'s 1 and
  this function's 0 at its end.

**`gtHackFlag`** (main 0x00378a98) is read through `ccCheckGtHack` by six
call sites:
- **`ccAddRequestFileListSpc`**: maps the field to `gateHackingOutID`
  (19 -> 0 ... 91 -> 18, else -1) and adds `x<name>.ccs` to the file list.
- **`ccPlayer::ccPlayer`**: `ghoFlag`, act 24, and the body on the
  collision list, instead of act 13.
- **`ccFellow::Initialize`** (twice, once with its result unused): the
  member hidden and off the command list, `dispWait` 65, standing.
- **`ccAI::ccAI`**: `arrivalChatCnt` -1 instead of 150.
- **`ChatMessageSender`**: calls `ChatMessageEnteredField` on both
  branches, so the check changes nothing.

**`ccClearGtHack`**, from `ccStartThEvent`, keeps the flag only in a field
(or a dungeon of type 8 or 9) entered from a town.

**`gate_hack_anim`** waits on `ccCheckGtHackAnm`, which is `ghoFlag`. It
is set by the constructor and cleared by `GateHackingOut`'s last step.

## The movie

**Its own file lists.** `ccRequestLoadStreamGateHack(107, town, field)`
plays stream 107 through `RequestStrPlayGH`, which builds its lists from
the `str7000` tables instead of `searchPreLoad`:
- Mac Anu's departure (`str7100`);
- the stream's own `str7200` and `str7300`;
- the arrival scene `str7000Out` picks for the field (`str7404` for area
  19).

Three setup files are read whole first.

**The loop.** `str7300` has flag 0xc1: the reader pauses before it, the
player cannot skip it, and it loops. `ResetPause` ends it through
`WaitEnd`'s skip path.

**The port.** `piney-stream`'s `table::gate_hack_files` and
`Stream::gate_hack` build the same lists. The port loads nothing while the
loop plays, so it calls `ResetPause` after one pass. A first try waited for
`frameNow == frameEnd`, but that frame never comes: a scene played from
memory rewinds from `frameEnd - 1`.

## GateHackingOut

**The camera file.** `x7404cam.ccs` holds one animation,
`ANM_str7404cam0`, and three markers:
- `OBJ_marker_cam1`: the eye;
- `OBJ_marker_target0`: the point looked at;
- `OBJ_marker_man0`: where Kite stands.

**The steps.**
- Step 0 creates the anm, switches to `ecam` and hides the blades (their
  hand nodes' `dispSW` 0).
- Step 1 plays the anm and places the camera and Kite from the markers
  every frame. When the anm ends it goes to step 7 and shows the blades.
- Step 7 fades the blades in by 0.1 a frame (`ghoCamArmsT`). Once Kite's
  act 24 clip has ended into act 23, it goes to step 8.
- Step 8 hands the last eye and target to `tcam` through
  `cameraParamChange`, stands Kite (act 2) and clears `ghoFlag`.
- Steps 2-6 are never reached from 1, but they are ported and checked.

**How it is split.** `combat::gate_out` takes the anm through a small
trait (`GhoCam`). The field plays the real file through the desktop's
`ccAnm`; the probe gives scripted markers.

**Two members found by the port.** Kite's `ccPlayer` needed +0x250 (the
kept eye), now `Player::pos_cam`. The blades' transparency needed a way
through `Body` (`draw_char_arms`).

## Checked

- **`tools/test_gate_out_rs.py`**. None differ in any of the three:
  - `ccClearGtHack`: 240 cases.
  - `ccAddRequestFileListSpc`: 2,376 cases, the ID and the file listed.
  - `GateHackingOut`: 400 runs of up to 60 frames. The camera functions
    and `cameraParamChange` are the game's; `ghoCam` is scripted. All nine
    steps are reached, and every field of the player and both cameras is
    compared bit for bit each frame.
  - The first run had the marker parent's scratch memory overlapping the
    third marker, which made all three positions equal. Runs that start
    past step 0 also needed `ghoCam` seeded as step 0 leaves it.
- **`tools/test_stream_rs.py gatehack`**: `RequestStrPlayGH` natively over
  towns, fields, the crisis and both languages, 768 runs. The new fixture
  and `gate_hack_stream_matches_request_str_play_gh` compare the lists:
  none differ.
- **`gate_hack_arrives_hacking_out`** (story:19, with Mia and Elk put in
  the party as the area starts):
  - the movie plays its four scenes;
  - the flag is kept;
  - Kite goes 24 -> 23 -> 2 under `ecam`, the blades hidden, then fading,
    then shown;
  - the members appear on their 65th frame.
- **`gate_hack_arrival_shots`** takes pictures of the movie, the landing,
  the fading blades and after.

**Still unknown:**
- **Load time.** How long the game holds the loop depends on
  `ccLoadResourceFL`'s time, which is not measured.
- **The camera file's own animation.** `ghoCam`'s real markers come from
  the desktop's anm player, which is not compared with the game's.
- **The arrival chat.** The party's arrival chat lines (`ccChatMsg`) are
  not ported, so `arrivalChatCnt` changes nothing visible yet.
