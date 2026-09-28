---
number: 108
title: Back in the town with the field's party: the registry carried, the members at their StartPos
date: 2026-09-25
area: world, test
files: crates/piney-world/src/party.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/event22.rs, crates/piney-game/src/session/event11.rs, docs/engine/field-game.md, docs/engine/field-walk.md
resolves: 103
---

# 108. Back in the town with the field's party: the registry carried, the members at their StartPos

Worklog 103 left one gap. The session carried `ccSpcManager` and
`ccPartyManager` from the town into a field and from area to area, but
not back into a town: Mac Anu always started from a new game's registry.
In the game both are globals, so the town's set-up builds the party the
field left.

## Where a returning member stands

**`ccGetStartPositions(1, 2, 3)`** (gcmn 0x0059ff50) runs before
`rebootSpcManager`:
- `WORLD_MAN::SetCharPosition` fills `StartPos[0..3]` by party slot. In a
  town that is the start, then the start + (200, 100), + (-200, 100) and
  + (0, 300), all facing Kite's way.
- Then each registered slot 1-4 gets its `pos` and `dirc` (+0x20, +0x30):
  a party member's from its party slot's scalars (+0x04-+0x10), anyone
  else the origin facing 0.
- The loop reads the scalars and writes the vectors, so one slot's write
  cannot spoil another's read.

**`ccFellow::Initialize`** then stands the character at
`StartPos[listNum]`, its registry slot.

**Before.** The port's town built every fellow at the origin. That was
right only for characters registered outside the party, whom their
entry's marker then moves.

## The port

- **`World::set_spcs`** takes the area's `Spcs`, with every `bootParam` 0
  as the old area's `ccThSpcDelete` leaves them.
- **The session** gives it the `Spcs` right after `WorldMode::enter` on a
  change of scene into a town. Logging in still starts a new game's
  registry, as `ccSetupNewGame` does.
- **The town's `build_spc`** places each fellow at its `StartPos`
  (`start_of`, using the field's `party_starts` for the town's offsets).

## Checked

- **`the_town_takes_the_party_back`:**
  - It starts in area 31 with Piros in the party and Gates Out to Mac Anu.
  - The town's party is [0, 8, -1], with Piros at (200, 5700, 600).
  - Event 22's town blocks are kept out by setting `eventStatus[0]`: they
    are written for a party without Piros, and their `member_add` would
    add him a second time.
- **`event_11_church_ends_with_blackrose_out`** now checks Mac Anu's own
  party again. Before, that check passed only because the town ignored
  the field.

**Still unknown:**
- **No following in town.** The party in town does not follow Kite:
  `ccAI::ActInTown` is ported in `piney-battle` (`ai_move`), but the
  town's fellows run `piney-world`'s own frame, which has only the manual
  paths. A member not under the events' control stands where it arrived.
- **Positions not checked against the game.** The town branch of
  `SetCharPosition` was read, not run. Its member offsets are the same
  numbers as the field's `party_starts`, which `test_field_rt.py` checks
  against the game for the fields.
