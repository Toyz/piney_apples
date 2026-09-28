---
title: Bosses - Magus
status: partial
volumes: MUT
covers: MUT gcmn.prg:0x006192c0 bossFunc (code 2), 0x0049da60 ccThBoss03, 0x0049db40 ccBoss03::ccBoss03, 0x0049f580 ccBoss03::Main, 0x0049f400 Move (Mutation's), 0x0049e9c0 CheckEpitaph, 0x0049e7f0 the dropped leaves' hiding, 0x0049e9e0 CalcCamera, 0x0049f080 SetupLaserShock, 0x0049f6a0 Affect, 0x0049fa50 Think, 0x0049fc20 Slave, 0x0049fd30 ExecPatternIndex, 0x004a0110 ChangeAction, 0x004a02c0 SetupLeafObjPtr, 0x004a05b0 _GetLeafIndex, 0x004a08c0 PreDrawAnm, 0x004a0c20 OnExitLeaf, 0x004a0d50 OnThinkNeutral, 0x004a1310 OnThinkEpitaph, 0x004a1470 OnThinkDataDrain, 0x004a1530 OnThinkChase, 0x004a17d0 OnThinkEpitaphWave, 0x004a1a50 OnThinkDataDrainAtk, 0x004a1c20 act 9 (Mutation's), 0x004a1f80 OnThinkNeedle, 0x004a2200 OnThinkDie, 0x004a2300 OnThinkReturn, 0x004a2420 OnThinkStop, 0x004a2540 OnThinkWander, 0x004a27a0 OnThinkDash, 0x004a2990 OnThinkEscape, 0x004a2c40 OnThinkWave, 0x004a2ec0 act 8 (Mutation's), 0x004a3100 OnThinkLeafDrop, 0x004a36b0 OnThinkLeafCountDown, 0x004a37c0 OnThinkLeafGrow, 0x004a39a0 OnThinkDestroyLeaf, 0x004a3ae0 OnThinkLaser, 0x004a3fd0 OnThinkRise, 0x004a4460 OnThinkFall, 0x004a46a0 ccBoss03Leaf::ccBoss03Leaf, 0x004a4e00 Move, 0x004a4f30 Main, 0x004a5080 PreDrawAnm, 0x004a52b0 Affect, 0x004a54b0 Think, 0x004a5680 DeadEffect, 0x004a57a0 OnThinkDie, 0x004a5920 Init, 0x004a5ab0 OnThinkDrop, 0x004a5db0 OnThinkFire, 0x004a5ec0 OnThinkCountDown, 0x004a6560 OnThinkDestroy, 0x004a65a0 OnExit, 0x004a6620 PrepareExplode, 0x004a6780 Order, 0x004a6e00 ChangeAction, 0x004a6e80 CheckBlink, 0x004a6ee0 SetBlinkTime, 0x004a7090 ccBoss03LeafExplode::Init, 0x004a7100 ccBoss03LeafExplode::Draw, 0x004a7570 ccBoss03LaserShock::Init, 0x004a7630 ccBoss03LaserShock::Draw, 0x00481180 ccBossEffNeedle::ccBossEffNeedle, 0x004814d0 ccBossEffNeedle::Draw, 0x0048e430 ccBossEffNeedleCreate, 0x0048d5d0 ccBossEffThunderCreate, 0x0061a010 boss03EpitaphActTbl, 0x0061a080 Boss03AnmTbl, 0x0061a130 Boss03SlaveAnmTbl, 0x0061a108 @1538, 0x0061a118 @2301
worklog: 274
---

# Bosses - Magus

Magus (`ccBoss03`, boss03.cpp) is `bossFunc` code 2, fought in Mutation's
event 115 in the arena of field 3 (`EVENTAREAB0`). It is a `ccBoss` (the
base is [Skeith's page](boss.md)) with twelve leaves (`ccBoss03Leaf`),
each a `ccBoss` of its own that falls from the body, lands on the command
lists and bursts after a countdown unless the party kills it first. The
names and layouts are Infection's DWARF; the rules are Mutation's code
(70 functions, 0x0049da20-0x004a7974), checked frame by frame against the
port.

## Where it comes from

Event 115's block 22 (`in_field town=2 field=3`) runs `entry type=7
code=2` and `battle_ready`; block 24 waits on `absent type=7 code=2`.
`ccBossEntryStart(2)` starts `ccThBoss03` (0x0049da60): `new
ccBoss03` (0x52a70 bytes in Mutation), its `Main` (vtable +0x1c) each
frame until `CheckExit` (+0x4c), then the parameter's +0x14.

```text
class          size     own members from  bossTbl row
ccBoss03       0x52a70  0x29350           2 (Magus)
ccBoss03Leaf   0x29810  0x29350           11 (a leaf)
```

Mutation's `ccBoss03` is 16 bytes longer than Infection's: four words at
+0x52720 (below), and everything from `m_blurRad` on 16 bytes later
(`m_blurRad` +0x52730, `m_blurScale` +0x52734, `m_laserShock[12]`
+0x52740, `m_bNoLeaf` +0x52a40, `m_camCount` +0x52a44, `m_vecPrevLaser`
+0x52a50, `m_vecPrevDirc` +0x52a60).

| offset | meaning |
| --- | --- |
| +0x52720 | stuck: set when the leaf drop is held 90 frames against something or strays; the leaves' countdowns wait on it; act 8 clears it |
| +0x52724 | the frames in a row `Move` pushed the body out of something (`CollisionDetection`) |
| +0x52728 | act 9's stage effect (-1 none) |
| +0x5272c | regrow: every leaf fell and went; the next neutral grows them back |

Mutation also adds `Move` (0x0049f400), a 16-byte `CheckEpitaph`
(`m_bEpitaph`), act 8 (0x004a2ec0), act 9 (0x004a1c20), and a global at
0x0038bb5c holding the boss for the draw callback 0x0049e7c0.

## The constructor

`ccBoss03::ccBoss03` (0x0049db40): `SetBaseParam(row 2)`; `actAnmTbl`
`Boss03AnmTbl`; exit off, drawn, body hit and cheat HP on; at Kite's place
500 back on y, his heading. `m_camID` 1, `m_laserFlash` -1, `m_dmgCount`
100, `m_leafGrowSE` -1, +0x52728 -1, `m_atkWait` `|ccRand() & 31|`. Twelve
leaves (`new[]`, `SetMaster(this, k)`), `ccEntryCmnd(this)`,
`ChangeAction(0, 3, 4)`. The after-images (`m_eai`: 20 frames, alpha 100,
transparency 1, -0.05 a draw), `InitCenterPos` (`DMY_center01`),
`InitStageEffect`, the blur (the manager's +4) on at `m_abgr` 0x48808080
and scale 1.01, `m_blurScale` 1.005, `SetPatternTbl(boss03EpitaphActTbl)`,
and `m_bReserveLeafDrop` 1. It makes no boss camera: `CalcCamera` turns
camera 3 itself.

A leaf (0x004a46a0): `SetBaseParam(row 11)`, cheat HP off,
`Boss03SlaveAnmTbl`, a body hit of 300 by 300 never switched on, its clump
`CMP_ex31leaf`, `ChangeAction(0, 3, 4)`; exited and on no list.

## A frame

`ccBoss03::Main` (0x0049f580): in acts 0, 12, 21, 22 and 29 `m_atkWait`
counts down to 0 (nothing reads it); then `ccBoss::Main` (Think, the lock,
Magus's `Move`, `PreDrawAnm`, and `Slave`: each leaf not exited `OnDraw`
and its `Main`), `CalcCamera`, the holds cleared on Magus and every leaf,
and while the player is locked a broken gauge's count held.

`Think` (0x0049fa50) by act:

| act | function | act | function |
| --- | --- | --- | --- |
| 0 | `OnThinkNeutral` | 18 | `OnThinkChase` |
| 1, 2 | `OnThinkDmg0` (nothing) | 20 | `OnThinkLeafDrop` |
| 4 | `OnThinkWave` | 21 | `OnThinkWander` |
| 5 | `OnThinkDataDrainAtk` | 22 | `OnThinkStop` |
| 7 | `OnThinkDash` | 23 | `OnThinkLeafCountDown` |
| 8 | back toward the centre (Mutation) | 25 | `OnThinkLaser` |
| 9 | skill 250 under camera 3 (Mutation) | 26 | `OnThinkLeafGrow` |
| 11 | `OnThinkDataDrain` | 27 | `OnThinkRise` |
| 12 | `OnThinkEpitaph` | 28 | `OnThinkFall` |
| 13 | `OnThinkEpitaphWave` | 29 | `OnThinkReturn` |
| 14 | `OnThinkDie` | 30 | `OnThinkDestroyLeaf` |
| 16 | `OnThinkEscape` | 31 | `OnThinkNeedle` |

Most acts clear `moveVector` with `vf0`, so its w is 1 and `Move`'s
`sceVu0AddVector` raises the place's w each frame; the game's
`ccTransPosW2P` and `P2W` (with a player) set w back to 1. Where they are
copies (a harness), w grows, and the rise's camera (below) with it.

`ChangeAction` (0x004a0110) is the base's forbid rules, but sets no clip
for the laser (25), finds the body's leaves again when not drained
(`SetupLeafObjPtr`), and notes `m_bIsNeutralAnm` (1 in act 0, 0 in any
other but 12). It never sets the base's `epitaph`.

## Before the drain: the neutral's choice

`OnThinkNeutral` (0x004a0d50) stands facing the target and picks, in
order:

```text
no target (SelectTarget 0): if m_bPrepareExplodeAll, every leaf Order(27); stay
m_dmgCount == 0            -> 31 (the needle), m_dmgCount = (ccRand() & 31) + 50
regrow (+0x5272c)          -> 26 (grow), regrow cleared
m_bPrepareExplodeAll       -> 27 (rise), cleared
m_bDropping                -> 20 (drop)
m_breserveLaser            -> any leaf down: 30 (destroy the rest)
                              else m_bHighDrive and m_laserCount > 0: 5 (drain a member), count 0
                              else 25 (laser), count + 1; either way reserve cleared, leaf drop reserved
m_bReserveLeafDrop         -> all twelve down: 26 (grow), reserving the laser if PP > maxPP / 4
                              (else the drop again); else 20, reserve cleared
otherwise                  -> 20, the drop reserved
```

`m_dmgCount` is 100 from the constructor and nothing else in gcmn writes
it (a scan of every access to +0x52700), so the needle never comes in the
game.

## The leaf drop

`OnThinkLeafDrop` (0x004a3100) circles the centre at 10 (`SpdUp`):
beyond 1500 of it, heading `centerDirc + pi/2 - 0.1745`; within 1000,
`centerDirc + pi/2 + pi/2 (1 - dist/1500)`; between, `centerDirc +
pi/2`. Pushed 90 frames in a row, or beyond 1500 and outside
`IsValidArea`, it sets stuck and goes to act 8. Held (`stop` or a hold),
it stands, and after 61 held frames waves (act 4). Every 60 frames the
first leaf not yet down falls (`OffExit`, `Order(24)`): leaf 0 and 6 then
bring the wave, leaf 5 a skill of `@2301` (157, 158, 160, by `ccRand() %
3`, `ccItemSkillCompel(this, target, sid, 0)`) on a `SelectTarget(3)`
target and a stop (act 22). With none left: `m_bNoLeaf`, the laser or the
drop reserved by the gauge (as the neutral), act 0.

`IsValidArea` is the base's (0x00473800): the world place's `|x|` under
`3000 + center.x - 500` and `|y|` under `3000 + center.y - 500`.

Act 8 (0x004a2ec0) takes Magus off the targets and its body hit, heads for
the centre at 100 a frame with an after-image every fourth frame, until
1500 from where it began; then back on, stuck cleared, act 0 (12 once
drained). Its speed is left at 100 until the next neutral.

## A leaf's life

```text
order  what it does
24     fall: act 19, Init (at the body's leaf, z 0, the body's heading, ANM_ex31atc0/1 by
       slave parity, HP full, m_bDrop 1), SE 169 note 50, the marker's smoke made
25     the countdown again (450 frames), act 22
26     burst: m_bExplode, PrepareExplode, act 21 (xeffect's ANM_ex31exp1), SEs 35 note 53, 56
27     go (its charge killed, OnExit)
28     fade off the body: Init, m_transSpd 1 / the clip's frames, act 23
29, 30  the grow's charge particles on, off
```

Landing (`OnThinkDrop`, 0x004a5ab0, at the fall clip's end): the
countdown starts (450, blink every 30), act 22, 137.6 aside
(`(137.6 cos d, 137.6 sin d)`, negative for even slaves), `ANM_ex31lea0/1`,
`ccEnemyEffDust(pos, 30, 4.0)`, SE 56 note 72, and `ccEntryCmnd`: the
party can now target it.

`OnThinkCountDown` (0x004a5ec0) waits while Magus is stuck or the party is
down or `compulsionGameOver`. Each frame the blink quickens
(`SetBlinkTime(450`-scaled`)`, an after-image each flip), `m_expAlpha` and
`m_expScale` move by the blink, `m_expBaseAlpha` rises 1/9 to 50. At 0:
`PrepareExplode` and, unless Magus already rises, its `m_fPrevRise`, and
Magus rises (act 27, `m_bExplosion`) unless it waves or is held, when all
leaves burst at its next neutral (`m_bPrepareExplodeAll`).

`OnThinkRise` (0x004a3fd0): camera 3, locked, cinema 10, blur 0x60808080,
up at 250 (slowing past 2000) to 5000, 15 frames, then a flash
(`EntryFlash2(60, 60)`), boss skill 10 at the target with its `atk` set to
`300 * n / 7`, and every 5 frames the next leaf down and not yet bursting
bursts (order 26). `n` (`m_explodeLeafNum`) was meant to count those
leaves, but the loop never steps its pointer: it tests leaf 0 twelve times,
so `n` is 12 (`atk` 514) when leaf 0 is down, not exited and not bursting,
else 0. `OnThinkFire` (0x004a5db0):
the last to go brings Magus down (act 28) and its blur back to 0x48808080.
`OnThinkFall` (0x004a4460) drops at 50 to its old height, waits 30, then
camera 1, unlock, and if all twelve were down, all back and the regrow.

A leaf's affect (0x004a52b0) takes a hit's HP with no party checks and no
hit mark; at 0 it dies (act 14: off the lists, SE 104 note 36, its smoke,
an auto samon ring of model 195 at (0.5, 1, 0, 40) 50 up, a 30-frame fade,
`OnExit`). `OnExit` (0x004a65a0) calls Magus's `OnExitLeaf` unless it is
drained: once every leaf is down and gone (not while rising or falling),
the regrow, and out of act 30.

`OnThinkLeafGrow` (0x004a37c0) plays `ANM_ex31grow` at 51 (a fifth of the
speed), the charges on at its second frame with a looping SE 230; at its
end every leaf back on the body, its drop cleared, the SE off, act 0.

## The laser, the waves, the needle

`OnThinkLaser` (0x004a3ae0): locked (the chat left open: its argument is
`Think`'s jump table address left in a1), cinema 9, camera 3 (`m_camID`
3), the place and heading kept; up at 50 to 1500, then at the neutral
clip's frame 0 `ANM_ex31atc2`, `SetupLaserShock` (the twelve shocks,
`OBJ_ex31les00`-`55`, each on the leaf of the other parity), twelve
`ccBossEffThunderCreate`, SE 68. 40 frames on, the clip plays; at its end
Magus is put back, and on the ground boss skill 9 at the target, camera 1,
cinema off. SEs by the clip's frame: 222 SE 40; 202, 187, 172, 157, 142,
127 SE 56 note 70; 190, 175, 165, 150, 135, 120 SE 61 note 72.

`PreDrawAnm` (0x004a08c0) does not step the clip in the laser's second
step. In its third, from frame 100 the shocks 0 to `min(((f - 100) / 15)
2, 11)` draw: each whose beam meets the ground (`ccHitCheckLM` from the
shock 2000 down, turned by its leaf and Magus's heading) fades 0.01, and
every sixth frame shocks 0, 3, 6 and 9 burst (`ccParticleExplode` twice,
`effSmokeRock`) drawing one `ccRandF(pi)`.

`CalcCamera` (0x0049e9e0), with `m_camID` 3: in the rise the eye turned
0.2618 about y and `dirc + 5.7596` about z round Magus, 1500 up; in the
fall 2000 behind the target's heading, 100 up; in the laser's first step
Magus faces the centre, the eye 1000 back and 50 up; in its third a
`ccRand()` every frame, until frame 135 a count to 15, at 135 Magus over
Kite (`cmndPcRoot`, the old place in `m_vecPrevLaser`), `EntryFlash3(80,
80, 20)`, and from 150 the view shaken by `ccRandF()`.

`OnThinkWave` (0x004a2c40): SE 223, SE 224 at 30, at 35 the flash, the
WaveShock and `ANM_xx11wave` at 512; boss skill 12 25 frames on; a camera
shake (`cameraShake(2, 2, 3, 2)`) each frame in range. The drained wave
(0x004a17d0) is the same with skill 12 at 60 and the shock at 36.

`OnThinkNeedle` (0x004a1f80): 21 frames on, `ccBossEffNeedleCreate` 465
ahead and 200 up (128 needles, three `ccRandF(pi)` each), boss skill 11
there two frames later, act 0 at 35. The needles grow (alpha +0.2333 to 1,
SE 231), hold 15 frames and fade (-1/15).

## After the drain

`Affect` (0x0049f6a0): 13 is act 11 and the grow's SE off; 21 sets HP and
max HP to 4500, the gauge -1, the clip's speed 256; a hit is the base's
(cheat HP holds half), then unless drained or already, a gauge at half
`maxPP` or more reserves the laser and `m_bHighDrive`. The rest is the
base's.

`OnThinkDataDrain` (0x004a1470): cheat HP off, `m_bEpitaph`, act 12, every
leaf goes (order 27). The patterns then come from `boss03EpitaphActTbl`:

```text
boss03EpitaphActTbl 0x0061a010 (MUT): 3 | 2 30 | 10 3 | 2 30 | 4 | 2 30 | 1 | 2 30 | 3 | 2 30 |
  14 | 6 1500 | 2 30 | 4 | 2 30 | 1 | 2 30 | -1
  (Infection's has 9 3 250 where Mutation's has 14)
@1538 0x0061a108: 157 158 160
Boss03AnmTbl 0x0061a080: 0 ANM_ex31nut0, 4 ANM_ex31atc3, 12 ANM_ex3xnut0, 13 ANM_ex3xatc0,
  14 ANM_ex3xdea0, 25 ANM_ex31atc2, 26 ANM_ex31grow (32 rows, the rest none)
Boss03SlaveAnmTbl 0x0061a130: 0 ANM_ex31atc0, 19 ANM_ex31atc0 (24 rows)
```

`ExecPatternIndex` (0x0049fd30): 2 waits in act 12 whatever the drain
(`m_stopTime` the next word); 1 waves (13 once drained, else 4); 14 is act
9; 9, 10 and 11 cast (`ccItemSkillCompel(this, target, sid, 1)`, 10's from
`@1538` by `ccRand() % 3`) and wait for the skill (`ExecPattern(2, -2)`),
reading 9's and 10's target type from `patTbl` itself; the rest are the
base's (3 escape, 4 chase, 6 dash). Act 9 (0x004a1c20): the stage darkened
(`BeginStageEffect(0x64000000, 15, 15, 3000)`), locked, off the targets,
camera 3 1000 off the target, skill 250 at 15 (`ccItemSkillRequest` with
its flag left unset in a3; the harness measures 1, cast through Magus),
the stage back at 90, camera 1 and the next pattern 15 later.

`OnThinkEscape` (0x004a2990) turns `moveDirc` each frame toward `dirc.x`
(0), not the swinging `actDirc.x` it sets. `OnThinkDie` (0x004a2200):
`BeginDeadEffect(2500, 2500)`, and once the dead effect ends, exit.

## In the port

`piney_battle::boss::magus` (the body) and `magus::leaf`; the tables
(`magus_epitaph`, `magus_anims`, `magus_leaf_anims`, `magus_skills`,
`magus_drop_skills`) are the build's combat group. Each leaf is a
character with a `Boss` of `Class::MagusLeaf`, taken out for the body's
frame. The body's leaf places (`OBJ_ex31leafNN`'s world matrix) and the
shocks' ground hits are the runtime's (`Magus::leaf_pos`, `shock_hit`):
`piney_world::combat::boss` poses x31's body and reads the leaves; every
beam is taken to meet the ground. Camera 3 moves are `Out::CameraChange`,
`CameraPos`, `CameraView`; the shake `Out::CameraShake` (the runtime
checks its range and draws its `rand()`).

A boss's affects from the menus and the other characters' frames wait in
`Boss::queued` for its own frame. The game's `Affect` runs at once and
leaves a drain's 13 as type 0 unless held (`lockPlayer`, forbid 2); the
queue does the same to the type, or the drain menu's 21 (sent while the
world is paused, before the boss's frame) meets `EntryAffect`'s check on a
drained foe and is lost, and Magus keeps its 30000 HP in the Epitaph.

## Checks

`tools/test_battle_magus_rs.py` (`PINEY_VOLUME=mutation`) builds the
game's `ccBoss03` with its twelve leaves and runs its `Main` natively
(with the effect manager's pass) against battle_probe's `magus`, frame by
frame. A case has one to three members, 600 to 7000 frames, the leaves'
places on the body, hits on Magus and on the landed leaves (some to the
death), the gauge set, the drain's 13 and 21 with or without heavy hits
after, Magus's own skills ending now and then, a while of being pushed
out of something, shocks' beams missing on every third, fifth or seventh
call, and the menu's type changing. Compared each frame: Magus's acts,
movement, place, target, HP and gauge, flags, clips and own members
(Mutation's layout, the four new words included); the twelve shocks; each
leaf's acts, place, HP, lists, clip and own members (countdown, blink,
burst, fade); the effects' slots; the party's HP and hold; the calls; the
menu; `rand()` and `ccRand`'s index. 120 cases (308,056 frames) agree
(`bulk 120 10000`); Magus's acts 0, 4, 5, 7, 8, 9, 12, 13, 14, 16, 18,
20, 22, 25, 26, 27 and 28 and the leaves' 0, 14, 19, 21 and 22 are
reached. `test_effect_lives` runs the WaveShock's, the dead effect's and a
leaf's ring's `Create` and `Draw` natively: 45, 120 and 20 draws, the
port's lives.

Not reached: act 30 (and a leaf's 23) needs the laser reserved while
leaves are down and the drop over, which the neutral's order never gives
(the drop is chosen first while `m_bDropping` holds, and only the grow
clears it); act 31 needs `m_dmgCount` 0; acts 21, 23 and 29 come from no
pattern of Magus's table.

`boss::magus::tests` (piney-battle) hold the constructor, a leaf's fall
and landing, a leaf killed, a burst's rise and fall, the drain to death
and the laser. `event_115_ends_with_magus` (piney-game's session tests)
starts in field 13 with blocks 0-20 played and runs block 21 to field 3,
Magus, the drain and death, to the event's end; `magus_drained_through_the_menus`
drains it through the menus (menu 66) and checks the 21's 4500 HP.
`magus_under_the_autopilot` (ignored) runs the fight under the story
pilot: the survey's event 115 (`PINEY_SURVEY_GOD`, 200,000 frames) ends
done, Magus drained at frame 165,723 and dead by 172,000.

## Unknown

- The pictures: the leaves' markers, charges and bursts, the laser's
  shocks and thunder, the needles, the dead leaves' smoke are named, not
  drawn; the body's dropped leaves are not hidden.
- The shocks' beams are taken to meet the ground; the game's line test
  (`ccHitCheckLM` on the land) is not ported.
- `EntryFlash2` and `EntryFlash3` on `scFadeDef` are drawn as one flash;
  the grow's looping SE plays once.
