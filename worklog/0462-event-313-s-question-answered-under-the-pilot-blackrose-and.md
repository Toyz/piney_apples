---
number: 462
title: Event 313's question answered under the pilot: BlackRose and another member called before her "Are you ready?", since her party_other is the way on and the pair the refusal
date: 2026-10-24
area: test, script
files: crates/piney-game/src/session.rs crates/piney-game/src/session/tests/corbenik.rs UNKNOWNS.md plans/quarantine-story.md
started: 2026-10-24T13:31:57-06:00
took: 18m
---

# 462. Event 313's question answered under the pilot: BlackRose and another member called before her "Are you ready?", since her party_other is the way on and the pair the refusal

## BlackRose's question

The survey of [[441]] stopped event 313 in Lia Fail at blocks 0x1:
BlackRose's "Are you ready?" was never answered so that the event went
on.

**What the script wants** (`tools/evscript.py dis` of event 313):
- Block 6 asks after a talk with BlackRose.
- Block 7 is a refusal when she is `not_in_party`: "Hey you! Are you
  going to ignore me...". This is what the pilot ran into every time.
- Block 8 is a refusal for a party of two, `in_party_of_2 15`: "Aren't
  you missing someone?".
- Blocks 9 and 10 need `in_party 15`, `party_other 15` (party slot 1 or
  2 holds someone other than BlackRose) and Yes. They ask again, then
  set `eventStatus[0]` 1 and change the scene.

**Why the pilot missed it.** `story_wants` read every `party_other` as
event 21's: Elk wants Kite alone with him, and anyone else along is
refused. So it asked for BlackRose alone (`Want::Only`). It also wanted
no member here at all, because the blocks with `in_party` were skipped as
unreachable while their answer was missing.

**The rule now.**
- An event takes `pc` as a pair (`Want::Only`) only where its
  `in_party_of_2 pc` block unbars the way (`del_area_code`, event 21).
  Elsewhere the pair is the refusal and `party_other` is the way on.
- A block short of nothing but an answer gives its party wants:
  `in_party` of a member the event refuses to go on without
  (`Want::Party`), and `party_other` as `Want::Others(pc)`, that member
  and someone else.
- `gate_goal` gathers that party before the talk that asks the question.
  It invites the member, then a companion (`companion`'s choice).

**Checked.**
- `session::tests::corbenik::event_313_answers_blackrose` starts at 313's
  story start. BlackRose and Balmung are called and both questions get
  Yes. At 2,914 frames block 14 is played, with its `area 8` and scene to
  field 8's arena. Without the change it stays at blocks 0x1 for 20,000
  frames.
- Event 204 also has a `party_other` (Balmung, beside its Kite-alone
  area). Its survey still ends done.
- Infection's event 21 surveys the same as before: blocks 0x7817 within
  40,000 frames, the same as without the change.
- The survey of 313 under god now reaches blocks 0x7401 (the arena).
- The piney-game suite passes.

**Still unknown:** nothing. The script's blocks decide each want above,
and the pilot's party satisfies them.
