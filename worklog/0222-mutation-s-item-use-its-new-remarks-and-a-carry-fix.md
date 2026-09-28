---
number: 222
title: Mutation's item use, its new remarks and a carry fix
date: 2026-09-27
area: volumes
files: crates/piney-gen/src/syms.rs, crates/piney-gen/src/manifest.rs, crates/piney-battle/src/party_chat.rs, crates/piney-battle/src/party_ai.rs, crates/piney-battle/src/damage.rs, crates/piney-battle/src/item.rs, crates/piney-fieldui/src/items.rs, crates/piney-world/src/field_world.rs, tools/volume.py, tools/test_battle_items_rs.py
---

# 222. Mutation's item use, its new remarks and a carry fix

`tools/test_battle_items_rs.py` now passes on Mutation, 5 of 5. It
still passes on Infection. Getting there turned up new rules, six new
chat tables and a wrong stretch of the carry.

## Six remark tables

After `ChatMessageWalkingTalk`, Mutation adds six `ccAI` functions. Each
says a member's line from a table of its own: 19 rows, one per
`MessageIndex()`. No symbol names them, so the port names them after
their tables (`party_chat::Remark`):

| remark | said by | when |
|---|---|---|
| `PhysicalTolerance`, `MagicTolerance` | `ChatMessageAttack` | a hit did nothing because the foe holds that Exdefense |
| `UseItem`, `UseLastItem` | `ccAI::UseItem` | after a use (`#a` the item's name), the second when it was the last one |
| `OnlyBuff`, `OnlyDebuff` | `ChatCommandBuffPlz`, `ChatCommandDeBuffPlz` | the member has no other buff or debuff |

They follow the usual pattern: in the party (`partyFlag` 1), not under
manual control, a line in the row, then `ChatMessageModify` and
`chatRequest`.

On Quarantine the tables sit elsewhere and have more rows. So piney-gen
finds each one through its code, with `callee_table`: the named caller's
n-th unnamed callee, and the first `lui`/`addiu` pair in it. Each table
is empty on Infection.

`tools/volume.py` finds the functions the same way (`callee`,
`remarks()`). The party AI and items harnesses hook them under the
port's names.

## Rules

- **`ChatMessageAttack`.** On a zero-damage hit it asks a new function
  (0x005c24b0), ported as `damage::exdefense_held`. That function
  returns the Exdefense kinds the foe still holds that the skill runs
  into. A physical or magic kind gives its remark. An element gives
  `ChatMessageAttributeGuard`, which Infection decided with
  `CheckCharAttribute`.
- **`ccAI::UseItem`.**
  - The member must pass `CheckAction(7)`.
  - Held, it may still use an item. With a boss in (`CheckBossEntry(1)`)
    it returns -1 instead.
  - After the use it says its remark.
  - `item::ai_use_item` emits the remark as `Step::UseItemRemark`.
- **`ccUseItemRequest`.**
  - The both-bars recovery items (21, 22) no longer close the menu and
    wait ten frames between the HP and the SP affects.
  - Books cap maxHP at 9999 and maxSP at 999.
  - The gate-out ocarina clears `worldman->warpFlag` (`Step::WarpFlagOff`).
  - The Grunty flute sets `ccSnd` +0x139 (`Step::VoicesOff`). While that
    flag is set, `ccWordsPlay` and `ccVoiceRequest` return at once.
    `ccThPucciguso` clears it when the ride ends, and so does
    `ccFileListLoad`. `FieldWorld::voices_off` holds it: the ride's exit
    clears it, and so does a new field world. The skill words are not
    queued while it is set.
- **`ccCheckSkillUseful`.** With `eventStatus[40]` set (the bracelet
  off, the same flag that hides its gauge), the Data Drain skills (2-5)
  find no target. Both copies take `drain_off`: the battle crate's
  `skill_useful` and fieldui's `check_skill_useful`.

## The carry

Mutation's main `.sbss` has a word inserted before `cmndPcRoot`. The
syms layout pass still laid about 40 globals there (`rtpcChatNum` to
`cmndTargetPriNum`) by Infection's spacing, 4 bytes short. It even put
`cmndTarget` and `cmndTargetPrev` at one address. gcmn's code builds
them 4 bytes further on, and the carry had both rows.

`symbol_named` found the main row first, so harnesses wrote
`cmndSortRoot` where the game never reads it. That was the last
`skill_useful` mismatch.

The layout pass now ends a span at the first global that an overlay's
code puts elsewhere. The rows up to that point keep their places,
including the boss vtables lower in the span. The sidecars and carries
of Mutation, Outbreak and Quarantine were regenerated. The generated
tables did not change.

## The harnesses

- **ccSpcChar's vtable.** The items harness gives the AI's member the
  vtable at +0x1f4, so Mutation's `CheckAction(7)` runs.
- **Characters 18-20.** Their item lists go where Mutation's
  `GetItemList` reads them: the save's second block
  (`volume.item_list_at`). The probe reads them the same way
  (`by_id::item_list`).
- **Epitaph pointers.** The harness reports them as Infection's
  addresses through the carry (`volume.inf_of`).

**Still unknown:** The callers of the other remarks are unported:
`ChatCommandExecute` for the last item, and `ChatCommandBuffPlz`,
`DeBuffPlz` and `ChatCommandFulfilCheck` for the only-one lines. The
party AI harness fails five checks on Mutation: `RequestChatCmd`,
`CheckNeedHealing`, `Reconnoiter`, `SelectAttackSkill` and
`CheckHealParty`. The new `ccAI` fields (+0x24, +0xa0) and the `ccSkill`
vector are still unread.
