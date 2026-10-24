---
number: 463
title: PhaJuk Zot's damage is Quarantine's own: its ccSkillDamage on the party as the game makes it agrees with the port, and four blows kill the member aimed at at any level
date: 2026-10-24
area: battle, test
files: tools/test_battle_rs.py crates/piney-game/src/session/tests/corbenik.rs docs/engine/boss-corbenik.md
started: 2026-10-24T13:57:37-06:00
took: 6m
---

# 463. PhaJuk Zot's damage is Quarantine's own: its ccSkillDamage on the party as the game makes it agrees with the port, and four blows kill the member aimed at at any level

## Checking [[461]]'s numbers in the game's code

[[461]] read the magic's damage off `tools/battle.py`, which is the
repo's own model. This entry runs it in Quarantine's code instead.

**The blows.** `tools/test_effect_spell_rs.py ids 5 252` runs skill 252's
`UpheavalSystem` natively with `PINEY_VOLUME=quarantine`: 930 frames
agree. Run alone (`look 252`), the game's spell makes exactly four
`ccSkillDamage2` calls, at counts 91, 111, 131 and 151, 20 frames
apart.

**The damage.** A new check in `tools/test_battle_rs.py`, `boss_on_party`,
runs `ccSkillDamage` with a boss's own row (unmodified) on a party
member as the game makes him: `ccSetLevelParam` at his level, his
charTbl gear, then `CalcReal`. Sometimes it adds Ap Vorma (+50 mDef) and
an element's Ap (+100). On Quarantine a third of the cases are
Corbenik's PhaJuk Zot (bossTbl 7, skill 252) on the 313 party at its
start levels or at 99. The other cases are random bosses and attack
spells.
- 300 cases on Quarantine, Infection and Outbreak: no mismatch.
- `damage`, `skill_damage` and `area_damage` (the splash's half) on
  Quarantine, 300 each: no mismatch.

200 rolls each, the game's own numbers for a blow on the member aimed at:

| member | start level | blow | at 99 | blow | HP at 99 |
| --- | --- | --- | --- | --- | --- |
| Kite | 70 | 480-786 | 99 | 330-633 | 1,827 |
| BlackRose | 70 | 1,026-1,299 | 99 | 690-1,000 | 2,030 |
| Balmung | 75 | 693-1,005 | 99 | 498-843 | 2,030 |

The 5-7% of blows that miss are counted apart, not in these ranges.
With Ap Vorma and Ap Juka the means fall 10-15% (Balmung at 99: 770 to
692). There is no cap and no level scaling past `CalcBattleDamage`'s
own: the game's numbers are the port's.

**What a player has at that point** (`PINEY_CORBENIK_TRACE` now prints
the party's skills and Kite's items at the fight's start):
- Kite: Pha Repth, Rip Synk, Dek Vorv, Dek Raio and attack spells.
- BlackRose: Repth, Rip Teyn, Ap Vakz, Dek Juka.
- Balmung: Pha Repth, Rip Maen (the revive), Ap Raio.
- Kite carries five each of Resurrect, Recovery Drink, Restorative,
  Mage's Soul and Antidote, one Emperor's Soul, two Speed Charms and one
  Meteor Strike.

No one knows Ap Vorma or Ap Juka. Even with both, four blows exceed the
target's HP at 99. A player's answer is to keep apart from the member
aimed at, since the splash is half and only within 300. Then the dead
one is revived at once (Rip Maen or Resurrect), with Pha Repth and the
Recovery Drink in between. The other way is levels and gear from play.
So the no-aid fight needs the pilot to revive and spread out, and the
grinding pilot. It does not need a battle-rule fix, and Macha's and
Tarvos's fights are not too hard for this reason. Their spells run on
the same `ccSkillDamage`, now checked with bosses' rows too.

**Still unknown:** whether a pilot that spreads out, revives and grinds
beats Corbenik with no aid, carried by [[461]]'s open question.
