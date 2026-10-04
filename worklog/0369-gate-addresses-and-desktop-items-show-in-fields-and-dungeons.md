---
number: 369
title: Gate addresses and desktop items show in fields and dungeons
date: 2026-10-03
area: script, ui
files: crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/session/tests/survey.rs
---

# 369. Gate addresses and desktop items show in fields and dungeons

Issue #39: in event 26, Bob gives story area 24's keywords (a Δ server
area) in a field (`gate_add_msg 24`, INF event 26 +0x0180), and the window
that follows came up empty. The keywords were still given.

`gate_add_msg`, `member_add_msg` and `desktop_item` compose their lines in
`ccEvent::Execute` and hand them to `DispInfo` (INF
SLUS_202.67:0x001b27b0), whatever the place
([event-vm.md](../docs/engine/event-vm.md#announcements)). The port
composes them in `story::Announcements`. The desktop and the town host
used it. The field and dungeon host (`AreaHost::announce`) went to
`FieldUi::announce`, which words a member's address only. For a gate
address or a desktop item it gave four empty lines, so `ChangeInfo`
opened a window with nothing in it.

`AreaHost::announce` now does as the town's host does: a member's address
through the field UI, the rest through `Announcements`. The area mode
loads them as the town mode does (`desktop::announcements`).

`bob_s_keywords_show_in_the_field` (piney-game) runs the story pilot from
start 26 until the field logs `announce GateAddress { area: 24 }`. At that
point the info window's first two lines have glyphs. Without the fix
their counts are `[0, 0, 0, 0]`. The game suite (226) passes.

**Still unknown:** nothing.
