---
number: 360
title: "Field 28's held battle flag is event 50's game of tag; Infection's story without god, and what the pilot lacked there"
date: 2026-10-02
area: test, battle, script
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-battle/src/enemy_ai.rs
---

# 360. Field 28's held battle flag is event 50's game of tag; Infection's story without god, and what the pilot lacked there

[[358]] and [[359]] left two questions open: why `in_battle` held at 1 in
field 28 with no fight, and how the story plays without the survey's
aids. Both are answered here; neither found a fault in the port.

## Field 28: Stehoney's tag

The hold was reproduced by letting the pilot skip the closed-event rule
for events 13 and 15 (a switch since removed). It came back at frame
38,400: Kite alone at field 28's start, one foe 1,928 away. That foe is
enemy row 131, put there by an event (`ent_root` 0), standing (act 0)
with Kite as its target.

INF event 50 (`SB0020 GOB1-1`) is a side event. Once board thread 9's
post is read it gate-marks area 28. In field 28 its block 5 sets the
place and makes enemy 131 (`entry 5 131`). Block 7 opens a game of tag,
and block 8 ends it once the foe is absent, with a reward (item 6/60).
The tagged foe keeps clear of Kite. `ccThGameCtrl` (gcmn
0x00517c50-0x00517d2c, `battle_now`) puts the game in battle while an
enemy is within `inBattleDist` (2,200) of a member, so the flag holds as
long as Stehoney does. That is the game's own rule. The pilot only went
there because it took the last marked area; the closed-event rule of
[[358]] keeps it away. Kite alone, sent straight into field 28 from story
start 16 and left idle for 20,000 frames, meets no foe at all; that start's
flags for event 50 were not looked at.

On the way, `enemy_ai::Out::InBattleDist`'s comment had the gold goblin's
-2.0 backwards. The same function (0x00517c98) sets battle mode when the
distance is exactly -2.0, so a goblin shaking off a hold forces battle
mode for a frame. The comment now says so; the code was right.

## The story without god

`PINEY_SURVEY_AIDS` picks the aids (`Aids` in `survey.rs`):
- **unset** keeps them all: god, the infection held at 0, the hacks'
  cores, the levels for the bosses;
- **`0`** turns them all off;
- **`levels`** turns off god but levels the party to the foes it meets
  and stocks Kite in town, as a player grinds and shops (below).

A run without god ends at the first game over, which takes the game to
the title. The run then fails, printing the place and the last 20
seconds of the fight (`FIGHT` lines). `PINEY_TRACE_BOSS` prints a boss
fight a line a second.

**No aids.** The game is over at frame 40,564, in area 17's dungeon
(event 14). Kite (63 HP) and BlackRose (70 HP) are both still level 1;
they fall to a level-4 Chicken Hand. The pilot walks straight to each
goal and fights only what holds the walk, so nothing levels it.

**`levels`.** Each member below the strongest foe standing (bosses
excepted: Skeith's row says 99) plus three gets the experience to that
level (`levels_for_foes`, the game's own level-ups). In town, Kite's
Healing Potions and Mage's Souls are topped up to five each (`supplies`).
The run ends events 3, 4 and 10-28, then the game is over at frame
242,740 in area 26's dungeon (event 29). Three Armor Shoguns (level 24,
1,010 HP) and a Grand Mage (level 23) wipe Kite (level 75: the
boss-level aid's lone-dungeon level, 75, had lifted him) with BlackRose and Mia (27-28).

Earlier settings told the same story:
- Matching the foes' level exactly, the party dies to event 14's Data
  Bug at level 5.
- Levelled by the lowest member (Kite overshooting to 99), it reached
  event 30 and fell to Skeith, whose own Data Drain (act 5, `affect 13`)
  fells members with HP left.

## What the pilot lacked without god

God's endless SP had hidden three gaps:
- **A refused skill.** With too little SP (or Data Drain barred) the
  Skills menu opens its info window (process 2-3), which takes OK, not
  cancel. The pilot gave cancel for ever (80,000 frames in event 14's
  dungeon).
- **Data Drain's cost.** It cast Data Drain whatever its SP. It now
  drains only with the SP for it, else a Mage's Soul on Kite first.
- **No plain blow.** With no skill it could pay for, it stood still while
  the foes hit it. In the last 20 seconds of the area-26 wipe, no foe
  lost a point. It now gives the action button's blow when the command
  target is an enemy. With that, one Armor Shogun falls and another
  drops to 200 of 1,010 HP, but the room still wins.

## Where that leaves the measurement

With the aids on, the story runs to its end ([[359]]). Without them, the
first walls are the pilot's, not the port's: it does not grind, plan its
SP, flee or use magic against armour. The damage rules it meets were
checked against the game's code earlier (the battle harnesses), and
nothing here contradicts them. Area 26's monster room is where a better
pilot would have to start.

**Still unknown:**
- Whether a player at those levels clears area 26's room (three Armor
  Shoguns and a Grand Mage) in the game; no capture of that fight.
- How Skeith's own drain picks its targets in a full fight without god;
  only seen at the end of one wiped run.
