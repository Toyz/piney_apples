---
number: 464
title: Event 314 reaches field 126 and its end: Quarantine's area 126 sets the number alone, the desktop's scene sets the field up, and its map is an empty EVENTAREA that puts the panels out
date: 2026-10-24
area: world, script
files: crates/piney-data/src/area/mod.rs crates/piney-world/src/evarea_end.rs crates/piney-world/src/story_map.rs crates/piney-world/src/field_world.rs crates/piney-world/src/area.rs crates/piney-game/src/desktop.rs crates/piney-game/src/session.rs crates/piney-game/src/area.rs crates/piney-game/src/area_host.rs crates/piney-game/src/session/tests/ending.rs docs/engine/evarea.md docs/engine/area-words.md docs/engine/events.md
started: 2026-10-24T14:03:46-06:00
took: 19m
---

# 464. Event 314 reaches field 126 and its end: Quarantine's area 126 sets the number alone, the desktop's scene sets the field up, and its map is an empty EVENTAREA that puts the panels out

## Event 314's way into field 126

The survey of [[441]] stopped event 314 at blocks 0x7. Its block 2
runs on the desktop: four streams, then `mode 5`, `area 126` and
`scene 1 0 126`. The port's desktop ran none of `area` or `scene`, and
area 126 had no words, so the pilot took the gate to field 122 instead.

**What the code does.**
- **`area 126`.** Quarantine's instruction 118 tests `n == 126` before its
  switch (QUA main 0x001c3204). It sets `eventAreaNumber = 126` and does
  nothing else: no `Quit`, no `SimGenerateCode`. Mutation and Outbreak
  have no such case. Their general path would read row 126's words
  through NULL, since the row has no address.
- **`mode 5`, then `scene`.** `ChangeScene`'s `ChangeRequest(6, 7)` is
  the frame's last request, so `ccSetupGameCtrl` sets up field 126 straight
  from the desktop.
- **The map.** `GO(1)` (QUA main 0x001b302c) makes, for
  `eventAreaNumber` 126, a class with no symbol: gcmn 0x00305120, 0x1c4
  bytes, vtable 0x002766b0.
  - Its constructor runs the base's and clears `modelArray`, `anmArray`,
    `bg` and the light count. It loads nothing, sets no fog and no
    `bgColor`, and never calls `SetStartPos`.
  - Its `Draw` (0x00305260) only sets `ccMenu` +0x0c (`panelStatus`) to 3.
  - So the field is black, the colour `ccSetupDesktop` cleared to. The
    party stands at `eventStartPos` as the last story map left it.
- **Kite hidden.** Block 3's `pc_act 0 6` is remote command 6: act 14,
  transparency and cloak 0.

**The port.**
- `piney_data::area::ev_area` gives `EvArea::Number(126)` on Quarantine
  only (`ENDING_AREA`). A unit test checks Infection still gives
  `Missing`.
- `piney_world::evarea_end::EndArea` is the map. `StoryMap::frame`
  pushes `StoryRequest::PanelStatus(3)`, and the area sets
  `panel_status`.
- `WorldMan::event_start` (+0x70 `eventStartPos`) is carried from map to
  map. The area's `leave` takes it from the story map's start, and a new
  area's words keep it (`keeping_start`).
- The desktop's `Bridge` records `area` and `change_scene`, and pushes
  the scene's `ChangeRequest(6, 7)`. The session's
  `(NEW_GAME | GAME_CTRL, Desktop)` arm, when a scene is recorded,
  applies it through `story_scene`, the old story-change code moved out.
  It then sets the field up.
- A session started on the desktop has no `WORLD_MAN` yet. The game's is
  `WORLD_MAN::Init`'s random area, of which a story map reads only the
  number, so the port takes the default.
- The area host had no `set_frame_rate`. Field 126's `frame_rate 1` now
  sets the area's rate, as the town's already did.

**Checked.**
- `session::tests::ending::event_314_plays_through_field_126` starts at
  314's story start. It reaches field 126 with `eventAreaNumber` 126 and
  the empty map, and plays to the event's `end_event` under the pilot.
- The survey of 314 now ends done. Left unported: `ending_kanji`.
- The piney-data, piney-world and piney-game suites pass.

**Still unknown:** `ending_kanji` (`STFROLL_VOL4`, the staff roll with its
text, logos, particles and `ending` animations) is not ported. Its frames
in field 126 pass as an unported call. It is the next piece.
