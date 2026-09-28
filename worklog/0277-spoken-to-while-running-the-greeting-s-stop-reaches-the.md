---
number: 277
title: Spoken to while running: the greeting's stop reaches the member; the field's talk target and the town's faces
date: 2026-09-28
area: ui, world, test
files: crates/piney-world/src/combat/mod.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs
---

# 277. Spoken to while running: the greeting's stop reaches the member; the field's talk target and the town's faces

Three play-test reports about Infection's party members, each found and
held by a test.

## A member runs off while spoken to

The report: talking to a town NPC or a party member, and after a few
seconds they run off. A pad log (`--pad-log`, with the card saved beside
it) replayed headless (`--replay`) reproduced the run exactly. BlackRose,
on her own route through Mac Anu, is spoken to at frame 754. The menu's
`EntryAffect` 14 reaches her, and her AI's `talkFlag` is set, so `Brains`
returns early. Yet she stays in act 5 (running) with `moveFlag` and
`runFlag` set (flags 0x2a). She runs about 780 on her old heading until a
wall stops her, with her menu still open.

The cause is where `ccAI::Greeting` writes. It clears `moveFlag` and
`runFlag` in the members' records (`Crew::spc`). The town's frame runs the
menus' lines (`Combat::drain_chats`) before the members' frames. Each
member's frame begins with `fellow::sync_in`, which copies the
character's flags into those records. The greeting's clears were never
written to the character, so they were read back over. `drain_chats` now
writes each member's record to its character (`fellow::sync_out`) after
the lines, as a member's own frame does after it runs.

The fix applies to fields too, which use the same `drain_chats`.
`piros_stops_when_spoken_to_while_running` holds it: Piros running
through Mac Anu on his own, spoken to, stands where he is. The test fails
without the write.

A walking PC (`ccRtownPC`) was not seen to run off.
`a_walker_stands_through_its_talk` keeps one standing, facing Kite,
through a talk left open ten seconds. The report's "any NPC" may have
been the party members.

## A member spoken to in a field

Talk said nothing, Trade found nothing, and Gift said "received" with no
name, kept the item with Kite, and at the holy ground opened a garbled
window. The talk menus read the one spoken to from the UI's `TalkTarget`
(`cmndTargetPrev->base`). The town sets it when a member's menu (21), an
administrator (23), a breeder (27), a dog (44) or a Grunty (45, 46) is
asked for. The field did not, so `base_of` answered nothing, and the
menus fell back to id 0 with no name.

`AreaMode::talk` now sets it the same way:
- a member is `(1 << 24 | id, Speaker::Spc(id))`;
- an event NPC's stand-in is `(2 << 24 | scene index)`, with its
  `npcTbl` row from `Combat::npcs`.

`event_11_blackrose_spoken_to_on_the_holy_ground` holds it.

## The party panel's faces in town

After a Gate Out mid-event, BlackRose's panel had no face. The town's
constructor set only slot 0 (Kite). `ccCheckMenuFaceNameParty` (gcmn
0x0056a930) is not a stored face: it reads `memberID[slot]` whenever it
is asked. The town now sets every slot's face from the party the last
area left (`WorldMode::set_spcs`). `event_11_gate_out_keeps_blackrose`
holds it, along with her staying in the party and the registry.

**Still unknown:**
- Whether a walking PC can run off in some case the tests do not cover.
  The play test named "any NPC", but only a member's run was caught in a
  log.
- BlackRose's own lines at the holy ground (the "optional event" of the
  report) were not checked once her talk target was right.
