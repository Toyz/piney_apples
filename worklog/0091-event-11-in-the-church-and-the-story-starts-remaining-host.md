---
number: 91
title: Event 11 in the church and the story starts' remaining host gaps: the town's near_marker distance, the desktop's overlay and pass
date: 2026-09-25
area: script, test
files: crates/piney-world/src/event.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/session/event11.rs
---

# 91. Event 11 in the church and the story starts' remaining host gaps: the town's near_marker distance, the desktop's overlay and pass

[[90]] brought event 11 to the holy ground with BlackRose. This entry
follows it into the church, and closes the host defaults the other story
starts still ran ([[86]]).

## Event 11 in the church

An ignored test, `event_11_church_log`, plays on from the arrival. Kite
walks north through block 0's door (x 0..190 from y 3575, [[84]]) into
the nave, and every window is closed as it opens, including those of the
church's set-up pass. Event 11 then plays:
1. **Streams 8 and 9.**
2. **The Book of Twilight's four windows** (messages 24-27, `InfoNow`).
3. **Streams 10, 11 and 12.** Stream 10 is the first Data Drain.
4. **BlackRose's talk** (messages 28-32), with its camera moves.
5. **The way back** to Mac Anu.

**The one host gap on the way.** The field host's `party_remove`, where
the event takes BlackRose out of the party, still runs the default, so in
the port she stays. That method belongs with the field's party work (the
combat agent's `area_host`, with the field menu's Add and Remove).

## The other starts' defaults

- **`near_marker` in a town.** The game computes
  `ccGetDist(ccTransPosW2P(marker), plw->posP)` (`ccEvent::CheckOpen`,
  main 0x001a7d9c):
  - `ccGetDist` measures the distance on the ground;
  - `ccTransPosW2P` in a town is `char::w2p`, relative to Kite and wrapped;
  - the player's own `posP` is `w2p` of his position.

  `World::player_distance` does the same, with piney-battle's `get_dist`
  (0x001d9dd0, checked with the enemies' movement). `story:13` used it.
- **The desktop's `overlay`.** The game loads an overlay's code. The port
  holds every overlay's code at once, so there is nothing to load.
- **The desktop's `play_pass_done`.** It sets a flag of `ccMenu`, the
  field menu, which does not run on the desktop.
- **The desktop's `clear_gate_hack`.** As in the town ([[90]]): nothing in
  the port sets `gtHackFlag` yet.

## Checked

- **The story starts.** With X pressed every 25 frames, story starts 10,
  12, 13 and 14 now run without a host default.
- **The rest.** The workspace's tests, clippy, fmt, the docs check and
  test_world_rs pass.

**Still unknown:**
- **The distance is not checked in eemu.** `player_distance` in the town
  reuses checked pieces (`get_dist`, `w2p`), but the whole condition has
  not been run against the game.
- **The church's streams lack subtitles and music.** The field host plays
  them through `StreamPlayer::start`, which has neither. The combat agent
  is switching it to `StreamPlayer::event`.
- **Not checked against the game.** The church's messages, cameras and
  timing are only seen to run.
- **Still defaults in the fields.** `party_remove`, `game_over` and
  `clear_gate_hack`.
