---
number: 230
title: The members' item uses carried out
date: 2026-09-27
area: battle
files: crates/piney-world/src/combat/stage.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/area.rs, crates/piney-world/src/town_party.rs, crates/piney-game/src/session.rs
---

# 230. The members' item uses carried out

Reported from play: party members could not use items. Their AI decided
to use one and took it from the member's list, but the use itself never
happened.

## The cause

The AI decides in piney-battle (`party_ai`). There are two paths:
- `ccAI::UseItem`, reached from the heal, cure and revive plans and
  CHAT's orders;
- the ocarina command.

Both call the runtime with `Call::UseItemRequest`. The field's runtime
(`combat::Stage`) sent that call to its catch-all arm, whose comment said
the item requests were "not reached in the opening's fights". Because the
AI takes the item from the list before the call, the item was spent and
nothing happened.

## The fix

- **The stage.** `Stage` queues each call as a `combat::MemberItem`
  (user, target, code, argument) in `Combat::member_items`. Every stage
  borrows the one queue.
- **The world.** `FieldWorld::take_member_items` runs each use's rules
  (`Combat::use_item`, `piney_battle::item::use_item_request`: affects and
  the item's skill as steps). It hands back the steps with the user and
  the target.
- **The area.** The area plays those steps the way the menu plays Kite's:
  - the world's steps through `FieldWorld::item_step` (affects, the
    item's skill, the heal effect, the dungeon exit's no-death);
  - sounds as `Se` and `SeNote` events;
  - a menu the use opens (the ocarina's gate-out, 86) through
    `FieldUi::open_menu`;
  - `Frames` as frames to wait (Infection's full-heal items wait ten
    frames before the SP).

  A member's use has no menu open, so the menu's own presentation steps
  do not occur.
- **Towns.** A town's queue is cleared after its frame, since members do
  not use items there.

The new test `a_member_uses_an_item` queues Orca's use of recovery row 0
on Kite in event 3's field, as the stage would. The item's heal (skill
150) then runs. The battle, world and game tests still pass.

**Still unknown:** The use is carried out after the AI's frame, not
inside its call, so an AI that reads the result in the same frame reads
it one frame late. No test drives a member's own decision to use an item
end to end (a heal plan through the AI's messages, in a fight). CHAT's
"First Aid!" out of battle on Infection plans once and drops the order
(`chat_command_heal_plz`), as the code has it.
