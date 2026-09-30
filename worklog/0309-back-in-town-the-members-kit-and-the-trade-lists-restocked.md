---
number: 309
title: Back in town: the members' kit and the trade lists restocked, their faces
date: 2026-09-30
area: world, save, ui
files: crates/piney-battle/src/restock.rs, crates/piney-battle/src/lib.rs, crates/piney-world/src/area.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/town_return.rs, BUGS.md
---

# 309. Back in town: the members' kit and the trade lists restocked, their faces

Three player reports (GitHub issues 3, 4 and 9), all about coming back to
a Root Town from a field in Infection.

## The rule: two steps of `ccSetupGameCtrl`

INF main 0x00168ca0-0x00168f74, after `ccStartThEvent` and before the
file lists, branches on `ccGame.area` (+0x14) and whether the scene is new
(`area`, `town`, `field`, `dungeon` against their previous values, the
port's `Scene::changed`):

| scene set up | new scene and | steps |
|---|---|---|
| town (0) | `areaPrev` (+0x18) not 0 | `SetSpcItemTown`, then `SetTradeItemTown` |
| town (0) | `areaPrev` 0 (a town, or Log in) | `SetTradeItemTown` |
| field (1), dungeon (2) | `areaPrev` 0 | `SetTradeItemTown` |

Nothing when the scene is the same (a dungeon's next floor, an event's
`scene -2`). Mutation (0x00168ae0), Outbreak (0x00167d90) and Quarantine
branch the same way.

`ccSaveData::SetSpcItemTown` (INF main 0x00176260): for members 1-17
(1-20 from Mutation on) whose bits are set in both `partyMemberFlag`
(+0x2220) and `partyMemberCall` (+0x2224). Past 20 stacks, the member
sells, as many times as he has stacks over 20, the cheapest stack he may
(`ccGetItemPrice`, the last of equal prices; never category 10 rows 0-5
and 18-23, 13/1, or member 1's 14/10): `num * price / 2` into his own
`spcParam.base.gold` (+0x14), capped at 9,999,999, and `DelItem` of the
stack. Then each of his ten `spcDefaultItemList` rows (gcmn 0x00647d80)
tops up to its count by `AddItem`, `(count - held) * price` off his gold,
held at 0. Kite's gold is never touched: the reporter's "no GP" is Kite's.

`SetTradeItemTown` (INF main 0x001767a0): the members' lists (save
+0xe3c, 16 slots) and the NPCs' (+0x127c, 48 lists, first 15 slots) get
back each `spcDefTradeList` / `npcDefTradeList` row whose `sw` word (the
row's bytes 4-7, the reporter's "4th byte") is set. Every member's first
row in Infection is the Speed Charm (11/56) with `sw` 1. Then each NPC, if
`rand() & 1`, draws a category `rand() % 10` and offers in its 16th slot
a piece near Kite's: his weapon (+0xd0) looked up in `feTbl[0]`, or his
armour piece (+0xc8 + 2k) in `feTbl[6 + k]`, among the first
`f_limitTbl[10 + server]` rows (`[15 + server]` for category 2), moved by
`rand() % 4 - 1`, held to 0..limit, `feTbl[category]`'s row there, count
1, unless the first 15 slots have it. Mutation, Outbreak and Quarantine:
the same code over 20 member lists and 54 NPC lists (the last 3 and 6 in
the save's extension); only the member count and the table addresses
differ in the listings.

## What the port did

Neither step existed: a traded Speed Charm stayed gone (issue 9, NPCs too)
and a member's used-up potions never came back (issue 4).

## The fix

`piney_battle::restock`: `item_price`, `set_spc_item_town`,
`set_trade_item_town` over the save and the volume's tables (no new
table: `spc_default_items`, `spc_trade`, `npc_trade`, `fe_tbl`, `f_limit`
were all in the build). `Scene::restock` (piney-world) decides the steps as
above; the session applies them to the save before building each scene's
mode, and at Log in. newlib's `rand()` is a session-held `Rand` (the modes
keep their own, so their sequences are as before).

Checked against the game: a scratch harness ran `InitTradeItem`, then
`SetSpcItemTown` and `SetTradeItemTown` natively in eemu on eight made-up
saves (every address known, lists of 0-40 random stacks, gold up to near
the cap, Kite's pieces from the `feTbl` lists, 40 trades taken, servers 0
and 1, `rand` from a seeded generator), and the Rust over the same saves
and draws: byte-exact on all eight, the draw count too. Items past a
table's rows were left out: the game reads past the table there
(category 13 rows 3-7 price -1), the port prices them 0.

## Issue 3: the faces

Already fixed by `WorldMode::set_spcs` (the BlackRose face fix in BUGS.md,
committed 2026-09-28, the day the repository went up): before it the town set only Kite's
menu face, so a member's face was empty on the status page and panels
after any way back from a field, which is the report. Every return (a Gate
Out, the Sprite Ocarina, an event's `scene`, the console's `town`) goes
through `Session::enter_world`, which calls it; the Party menu's own add
(`PartyInMenu` step 31) sets the face in town. The game's rule for the
record: `ccCheckMenuFaceNameParty` (gcmn 0x0056a930) is called only by
`ccMenuCtrl`'s constructor (gcmn 0x0051c7fc), once per scene, for slots
0-2 from `ccPartyManager.memberID`; `SetMenuFace` (0x00526940) only by the
event's `party_add`.

## Tests

`session/tests/town_return.rs`, each failing with the step (or the face
loop) switched off: `faces_come_back_from_a_field` (Orca called by the
Party menu, to a field and back), `the_kit_comes_back_in_town` (Orca's
kit used up, his gold 5000; back in Mac Anu the kit is whole and the cost
off his gold, Kite's as it was), `trades_come_back_each_scene` (Orca's
Speed Charm and an NPC's trade taken; back in both in Mac Anu and again
in the field after). `restock.rs`'s own: the sale past 20, an unknown
member left alone, an NPC's restock and pick, every volume's kit.
`area.rs`: `restocks_by_the_scene_left`.

**Still unknown:** whether the issue 3 reporter's build predates the face
fix (the report has only a picture; no way back to a town found without
it). newlib's `rand()` is one sequence through the whole game, so the
NPCs' picks in the port are the game's kind of draw, not its values. The
native check ran on Infection only; Mutation's and Outbreak's listings
match but the eemu harness is Infection's.
