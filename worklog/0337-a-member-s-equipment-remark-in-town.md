---
number: 337
title: A member's equipment remark in town
date: 2026-10-01
area: ui, world
files: crates/piney-world/src/party.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/event22.rs, docs/engine/field-ui.md
---

# 337. A member's equipment remark in town

[[67]] left the CHAT menu's `ChangeEquipReport` dropped in town. In a
field, `FieldWorld::equip_report` has the member answer the settings it was
left (`ccSpcChar::ChangeEquipReport(n)`, gcmn 0x0059f4a0):
- `n` 0: `ChatMessageEquipOK`;
- `n` 7: `ChatMessageEquipNOT(1)`;
- `n` 1-10 otherwise: `ChatMessageEquipNOT(0)`;
- Kite, or any other `n`: nothing.

The town's `WorldMode` listed the request among those it drops.

The town's party has the same AI. `World::equip_report(code, n)` now runs
`chat_line` on the member through `with_ai`, as `chat_cmd` runs its
orders. The line is queued like any line raised outside the frame. The
town's frame (`Combat::town_frame`, `drain_chats` at its start) sends it
the next frame, and `WorldMode` puts it up over the member, as the field
does. The menu's `R::ChangeEquipReport` goes there.

The test is `a_member_reports_his_equipment_in_town` (piney-game,
event22). In Mac Anu with Piros in the party, Kite's report and an `n` of
11 put up nothing. Piros's report with 0 puts up his balloon on the next
frame.

**Still unknown:** the line's text was not compared with the game's in
town. It comes from the same tables as the field's.
