---
number: 196
title: "A book given to a party member in town raises its stat; the third member remarks on a present"
date: 2026-09-26
area: ui
files: crates/piney-battle/src/item.rs, crates/piney-world/src/party.rs, crates/piney-game/src/world.rs, docs/engine/field-ui.md
---

# 196. A book given to a party member in town raises its stat; the third member remarks on a present

The audit of 0193-0195 also flagged two of the talk menus' requests
(`piney_fieldui::talk::TalkReq`) that nothing carried out. Both come from
the trade (49) and present (50) pages. The town mode's `TalkReq` match
let them fall to `_ => {}`.

**`SpcUseItem { spc, code }`.** `AddSpcItem` (gcmn 0x00527950) has a
member given a book (category 12) read it at once, `num` times:
`ccUseItemRequest(spc, spc, code, 0)`. A book raises one of the record's
stats by its row (piney-battle's `BOOKS`), writing through `ccChar` +4,
which is the save's `spcParam[id]`. The menu showed "<name> used <book>."
and took the books from the bag, but the stat never rose. A book given
in town was lost.

`piney_battle::item::book_on_record` runs `book` on the record, and the
town does that on the save's record for each request.
`a_book_on_the_record`: book 0 raises physical attack by 10, capped at
999, and leaves the rest alone. The member is not the player, so the
game shows no message of the stat.

**`PresentOther(member)`.** Read `ccSpcMessagePresentOtherFellow`
(0x005a1a70):

- It needs three in the party (`getPartyMenberNum`).
- It finds the receiver's slot (`checkPartyMenberNum`).
- For the other slot that is neither 0 nor the receiver's, it calls
  `ccAISysMsgSendP(0x10011, -1, id, id, 0, 30, -1, receiver)`, where `id`
  is that member's own sys-msg id.

piney-battle's AI already answered message 17 with
`ChatMessagePresentOtherFellow(receiver)` outside a fight. Nothing sent
it. `World::present_other` sends it on the town party's bus.

**Still unknown:** The present's remark was not run in a town with
three members; no test forms such a party from outside piney-world. The
town still leaves the Screen page's display offset and the pad's
vibration undone.
