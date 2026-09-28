---
number: 274
title: Magus and its leaves
date: 2026-09-28
area: battle, test, volumes
files: crates/piney-battle/src/boss/magus.rs, crates/piney-battle/src/boss/magus/leaf.rs, crates/piney-battle/src/boss.rs, crates/piney-world/src/combat/boss.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs, crates/piney-gen/src/manifest.rs, crates/piney-battle/examples/battle_probe/magus.rs, tools/test_battle_magus_rs.py, crates/piney-game/src/session/tests/skeith.rs
---

# 274. Magus and its leaves

Mutation's event 115 now fights Magus in field 3 and runs to its end.
Block 22's `entry type=7 code=2` makes `ccBoss03` (`bossFunc` code 2, MUT
gcmn 0x006192c0) with its twelve `ccBoss03Leaf`s, and block 24's `absent
type=7 code=2` follows once the drained body dies and exits. The
reference is [docs/engine/boss-magus.md](../docs/engine/boss-magus.md).

## How it was taken apart

The 70 functions of boss03.cpp (0x0049da20-0x004a7974), the needle's and
the thunder's `Create`s, and the three tables were lifted to a
straight-line form and ported from it, each counter and float checked
against the listings (with the post-increment compare of [[272]] in
mind). The names and layouts are
Infection's DWARF. Mutation's `ccBoss03` is 16 bytes longer: four words at
+0x52720 (stuck, the pushed count, act 9's stage effect, the regrow), with
`m_blurRad` and all after it 16 bytes on. Mutation adds `Move`, act 8
(back to the centre when stuck) and act 9 (skill 250 under camera 3),
and its Epitaph table has 14 (act 9) where Infection's casts 9.

The game's slips are kept. `OnThinkRise` counts its bursting leaves with
a loop that never steps its pointer: leaf 0 twelve times, so skill 10's
`atk` is 514 or 0. Act 9's `ccItemSkillRequest` flag is whatever a3 held
(the harness measures 1). The laser's `LockPlayer` argument is `Think`'s
jump table address left in a1, so the chat stays open. `m_dmgCount` is
100 and nothing writes it, so the needle (act 31) never comes. The
escape turns toward `dirc.x`, 0. Most acts clear `moveVector` with `vf0`,
whose w of 1 `Move` adds to the place's w.

## The shape of the port

`piney_battle::boss::magus` is the body (`Class::Magus`) and
`magus::leaf` a leaf (`Class::MagusLeaf`); each leaf is a character of
its own (`AffectFunc::Boss`, row 11), taken out of it for the body's
frame as Kyvia's parts are. The five tables are the combat group's
`magus_*` entries (data version 13). Camera 3's moves, the camera shake,
the flashes, the SE notes and loops and Magus's pictures are new `Out`s;
`piney_world::combat::boss` poses x31's body for the leaves' places
(`OBJ_ex31leafNN`), and the field's effects draw a dying leaf's ring
(`AutoSamonRing`, model 195) and the dust of a landing.

## The drain's lost 21

Under the autopilot the drain brought the Epitaph but not its 4500 HP:
Magus kept 30000, its gauge ran on, and cheat HP was off. The target
menu's `EntryAffect(13)` waits in `Boss::queued` for the boss's frame
while the world is paused, so the boss's affect type stayed 13, and the
drain menu's 21 met `EntryAffect`'s check on a drained foe and was lost.
The game's `bossAffectFunc` runs `Affect` at once, and every class's
`Affect` takes a 13 (leaving the type 0) unless the boss is held. The
queue now does that to the type (`boss::entry`). Skeith and Innis go
through the same menus; their menu tests stop at the Epitaph and do not
look at the 21.

## Checks

`tools/test_battle_magus_rs.py` (`PINEY_VOLUME=mutation`) builds the
game's `ccBoss03` with its leaves and runs `Main` natively against
battle_probe's `magus`: acts, place, target, HP, gauge, clips, every own
member of the body, the twelve shocks and each leaf, the effects' slots,
the party, the calls, the menu and both generators, each frame. Cases mix
hits on the body and the landed leaves, the gauge, the drain, pushes,
missed beams and menu types. 120 cases (308,056 frames) agree; the body's
acts 0, 4, 5, 7, 8, 9, 12, 13, 14, 16, 18, 20, 22 and 25-28 and the
leaves' 0, 14, 19, 21 and 22 are reached. The effect lives (the wave's
45, the dead effect's 120, a leaf ring's 20) are the game's.

`boss::magus::tests` hold six cases; the session tests
`event_115_ends_with_magus` (block 21 to the event's end) and
`magus_drained_through_the_menus` (menu 66, now with the 4500 HP) pass.

## Under the autopilot

`PINEY_SURVEY_ONLY=115 PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=200000`
ends `story 115: done; 8321 places; last: desktop - icon 1 - events phase
4`. Field 13 at frame 145,250, the arena at 148,550, Kite's drain at
165,723, Magus dead by 172,000. The first run, before the owner's field
13 changes in the tree, stopped in field 13 (`blocks 0x7dc7`): the pilot
never talked to block 13's six NPCs.

The pilot's real attacks hurt Magus, and they are physical. Its CHAT
order is Skills!, since Magus's physical defence (2000) is under its
magic defence (5000); the members cast only for a resistance under 100,
and Magus's are 500 and 990. Kite's skills (page 0) go at the target
menu's pick, which is a landed leaf while one is listed (the pilot's
`focus`, the first part up), so he mostly kills leaves (29 in the
diagnostic), which does not hurt the body. BlackRose's hits reach the
body at a tenth (cheat HP): about 1100 in 28,000 frames, enough for its
gauge to break, then Kite's drain. After the drain (4500 HP, cheat off)
the party kills it in about 6300 frames. Without BlackRose (a start put
straight at block 21) nothing hurts the body.

**Still unknown:**
- The pictures are named, not drawn: the leaves' markers, charges and
  bursts, the laser's shocks and thunder, the needles, the dead leaves'
  smoke; the body's dropped leaves are not hidden.
- The shocks' beams are taken to meet the ground (`ccHitCheckLM` on the
  land is not ported); `EntryFlash2` and `EntryFlash3` are one flash; the
  grow's looping SE plays once.
- Acts 30 and 31 and a leaf's 23 are never reached, in the harness or in
  the game's own order; whether any path of the game reaches them.
- Whether Skeith's and Innis's drains through the menus lost their 21
  before this change (the same path; not checked).
