---
number: 103
title: The party in a field: members join and leave the running fights
date: 2026-09-25
area: world, battle, test
files: crates/piney-world/src/field_world.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/event11.rs, crates/piney-world/examples/world_probe.rs, tools/test_party_rs.py, docs/engine/field-walk.md
---

# 103. The party in a field: members join and leave the running fights

Outside the towns, `party_add` and `party_remove` fell to the event host's
default, which does nothing. So did the field UI's `AddMember`,
`DelMember` and `TransferOut` requests. Event 11 hit this first: at the
end of the church its tail runs `party_remove 15`, and BlackRose stayed in
the party.

## What the game does

The same functions run as in Mac Anu (field-game.md):

- **`ccParty::AddMember`** (gcmn 0x0059ce80): the first slot whose
  `memberID` is -1 gets `inviteSpc(id)` (0x005a08e0).
  - For a registered id, `inviteSpc` sets the registry's `partyFlag` to 1.
  - If the character is built, its own flag (+0xe0 bits 14-16) changes
    too: -2 turns `recallFlag` on, -1 clears recall and becomes 1, and 0
    becomes 1.
  - It returns the character pointer. When that is null, the slot's
    `memberChar` is 0, `memberID` stays -1 and AddMember returns -1.
- **`ccParty::DelMember`** (0x0059cf60) acts only on slots above 0 whose
  `memberID` is not -1. It sets `memberChar` to 0 and calls `disbandSpc`
  (0x005a0f50): the registry's flag becomes 0, and the character becomes
  -2 with recall off if it was recalled, else -1. Then `memberID` is -1
  and `num` goes down by 1.
- **`expulsionSpc`** (0x005a1070) is just
  `DelMember(CheckMemberID(registry[listNum].id))`.

## The port

party.rs's `Spcs` already had all of these for the town.

**Party changes.** `FieldWorld` now has `party_add`, `party_remove` and
`transfer_out`. They run these functions over the fights' characters
(`with_party`, the `BattleChars` adapter). After a change,
`Combat::set_party` rebuilds the fights' `ccPartyManager` from `memberID`.
A member who leaves drops out of the fights, the panels and the AI's
party. One who joins is back in.

**Who calls them.**
- `area_host.rs` routes cases 77 and 78 to them. `party_add` also sets the
  new slot's menu face, as the town's host does.
- `area.rs` answers `AddMember`, `DelMember(slot)` (as `party_remove
  -slot`) and `TransferOut` for a party handle.

**A dismissed leaver.** When the fellow task lets a leaver go (`partyFlag`
-2 and act 14 set `exitFlag`), the game's `ccThFellow02` calls
`expulsionSpc` and deletes the character. The port now does the same
after each combat frame (`fellows_gone`):
- `Spcs::expulsion` with the character's flag, which also frees the
  registry slot unless it is still a member;
- the character dropped from the fights' members and cast.

## Checked

- **Against the game.** `tools/test_party_rs.py` runs `AddMember`,
  `DelMember` and `expulsionSpc` natively in eemu, with `inviteSpc`,
  `disbandSpc` and `CheckMemberID` beneath them. It uses 600 random
  registries and parties and the characters' whole +0xe0 words, with one
  to five calls each. The world_probe's new `party` request runs the same
  calls on `Spcs`.
  - Compared: each return, the registry's ids and flags, `memberChar`,
    `memberID`, `num`, and each character's `partyFlag` and `recallFlag`.
  - None differ. The adds return slots 1 and 2 and -1, and 235 of the
    runs change the registry's flags.
- **The session.** `event_11_church_ends_with_blackrose_out` plays event
  11 from the join through the church. It checks:
  - BlackRose is in the party on the holy ground;
  - `party_remove 15` leaves her registry flag 0 and her character's flag
    -1, with the party without her;
  - the party the field hands back to the session is without her;
  - `take_unported()` is empty.
- The workspace's tests, clippy, fmt and the docs check also pass.

**Still unknown:**
- **Adding a stranger.** When an unregistered id is added and the registry
  has room, `inviteSpc` builds the character at the Chaos Gate
  (`ccGetChgatePosDirc`, `inviteOffsetTbl`). That is not ported: the add
  does nothing. No event script adds anyone (no `party_add` at all), so
  only the menus could reach it.
- **The town doesn't take the party back.** Mac Anu still starts from its
  own registry, not the party the field returns (field-walk.md's
  Unknown).
- **Only the rules are compared.** The fellow task's own frames are not
  run against the game in a field: when a dismissed leaver's exit happens,
  and the walk away before it.
