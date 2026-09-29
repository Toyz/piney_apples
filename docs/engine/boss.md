---
title: Bosses - ccBoss and Skeith
status: partial
volumes: INF
covers: INF gcmn.prg:0x0045b2a0 ccBossEntryStart, 0x0047b340 ccThBoss01, 0x0045bcf0 ccBoss::ccBoss, 0x0047b410 ccBoss01::ccBoss01, 0x0045c270 ccBoss::Main, 0x0047bd50 ccBoss01::Main, 0x0045c600 ccBoss::Move, 0x0045c730 ccBoss::Affect, 0x0047bdf0 ccBoss01::Affect, 0x0045f090 bossAffectFunc, 0x0045d310 ccBoss::ChangeAction, 0x0045d210 ccBoss::ChangeNextPattern, 0x0045cad0 ccBoss::ExecPatternIndex, 0x0047c500 ccBoss01::ExecPatternIndex, 0x0047c1e0 ccBoss01::Think, 0x0047bf80 ccBoss01::Action, 0x0047ca50 OnThinkNeutral, 0x0047cba0 OnThinkDamage, 0x0047cc20 OnThinkEpitaphNeutral, 0x0047cdc0 OnThinkEpitaphWave, 0x0047d000 OnThinkChase, 0x0047d2f0 OnThinkEscape, 0x0047dac0 OnThinkReturn, 0x0047dbd0 OnThinkWander, 0x0047ddf0 OnThinkDash, 0x0047e030 OnCrossAtk, 0x0047e1a0 OnWaveAtk, 0x0047e410 OnDataDrainAtk, 0x0047e6a0 OnMagicAtk, 0x0047c390 ccBoss01::DrawCross, 0x0045e070 ccBoss::SelectTarget, 0x0045ec60 ccBoss::CalcTargetInfo, 0x0045df40 ccBoss::EraseCmndTarget, 0x0045dfc0 ccBoss::EntryCmndTarget, 0x0045ee90 ccBoss::BeginDeadEffect, 0x0045f0e0 _ccBossSkillDamage (char), 0x0045f470 _ccBossSkillDamage (pos), 0x00461750 ccBossEffManager::Draw, 0x00461a60 ccBossEffManager::IsEnabledEffect, 0x00478820 ccBossEffWaveShockCreate, 0x0046b3f0 ccBossEffWaveShock::Draw, 0x00478b10 ccBossEffMagicSquareCreate, 0x004793d0 ccBossEffLightCreate, 0x004797e0 ccBossEffForceGeneratorCreate, 0x0046dd90 ccBossEffBrightMagicSquare::ccBossEffBrightMagicSquare, 0x0046e320 ccBossEffBrightMagicSquare::Draw, 0x00479900 ccBossEffAutoSamonRingCreate, 0x0046ed70 ccBossEffAutoSamonRing::Draw, 0x00462660 ccEffSamonRing::Draw, 0x00479240 ccBossEffIceBreakCreate, 0x0046cc10 ccBossEffIceBreak::Draw, 0x004795d0 ccBossEffDeadCreate, 0x0046a440 ccBossEffDead::Draw, 0x00466a30 ccBossEffLight::ccBossEffLight, 0x00466df0 ccBossEffLight::Draw, 0x0046c950 ccBossEffIceBreak::ccBossEffIceBreak, 0x00462430 ccEffSamonRing::ccEffSamonRing, 0x00461bd0 ccEffSamonRing::SetModel, 0x00462750 ccEffSamonRing::BurstScale, 0x00462780 ccEffSamonRing::BurstTransparency, 0x00462790 ccEffSamonRing::SetPosRot, 0x00579ad0 ccLattice::ccLattice, 0x00579b70 ccLattice::Init, 0x00579c50 ccLattice::ClearCnt, 0x00579cb0 ccLattice::MakePacket, 0x0057a0e0 ccLattice::SendPacket, 0x0057a300 ccLattice::SetPos, 0x0057a3a0 ccLattice::NextVertex, 0x0057a470 ccLattice::Disp, 0x006519b0 latticeAttributeColorTable, 0x005eb430 boss01NormalActTbl, 0x005eb490 boss01SuperActTbl, 0x005eb530 boss01EpitaphActTbl, 0x005eb5b0 Boss01AnmTbl, 0x00696ce0 BossSkillTbl; INF SLUS_202.67:0x00519920 ccCheckTarget, 0x00519630 ccEntryCmnd, 0x0056cb50 ccChar::ClearCondition, 0x0059d080 checkPartyAnnihilation, 0x00106850 ccBufferReverce::ccBufferReverce, 0x00106890 ccBufferReverce::MakePacket, 0x00160240 ccScFade::EntryFlash; INF gcmn.prg:0x004618b0 ccBossEffManager::OnCinemaMode, 0x00461940 ccBossEffManager::OffCinemaMode, 0x0046a7e0 ccBossEffCinemaFade::ccBossEffCinemaFade, 0x0046a8c0 SetupSkillName, 0x0046a9b0 CinemaOn, 0x0046aa70 CinemaOff, 0x0046aa80 Init, 0x0046ac20 Clear, 0x0046ad10 ccBossEffCinemaFade::Draw, 0x005eb040 _g_cinemaSkillName, 0x006aa950 Draw's mode table
worklog: 109, 112, 117
---

# Bosses - ccBoss and Skeith

The bosses are not enemies: `ccBoss` is its own `ccChar` with a pattern
table, a set of actions and a manager of effects. Infection has one,
Skeith (`ccBoss01`, `bossTbl` row 0), fought in event 30 (the `EVENTAREAB0`
arena). This page is the rules: what a frame of Skeith does, and what the
Data Drain and Kite's hits do to it. It also covers its pictures: the
effects, the cross's trail and the reversed layer, and its camera. The
arena is [The story maps](evarea.md).

## Where it comes from

Event 30's block 20 runs `entry type=7 code=0`. `ccEntryEventMng` hands
type 7 to `ccBossEntryStart(0)`, which starts two tasks at priority 66:
`ccThBossEffect` (the effect manager) and `bossFunc[0]` = `ccThBoss01`.
The manager's task runs first in the frame. `ccThBoss01` news a `ccBoss01`
and loops `boss->Main()`. Once `CheckExit()` is true it sets its task
parameter's `+0x14` to 1, which is what the event's `absent type 7`
reads.

## The constructor

`ccBoss::ccBoss` sets the boss off (exit on, not drawn, no body hit, no
cheat HP). It sets `affectFunc = bossAffectFunc`, `waitPatNum = -2`,
`patNum = 13` and loads `xeffect`. `ccBoss01::ccBoss01` then:

1. `SetBaseParam(ccGetBossParam(0))`, `actAnmTbl = Boss01AnmTbl`.
2. Exit off, drawn, body hit on, cheat HP on.
3. Stands 500 units below Kite (`pos = plw.pos`, `pos.y -= 500`), facing
   his way.
4. Loads `x11` (`CMP_trall`) and makes the `ccAnm`. `ChangeAction(0, 0, 4)`
   sets `ANM_ex11nut0`.
5. `ccEntryCmnd(this)` puts it on the enemies' command list.
6. `SetPatternTbl(Normal)` and `ExecPatternIndex(patTbl, 0)`. The index it
   returns is dropped: `patIndex` stays 0.
7. `InitAfterImage(anm, 50)`, `InitBossEffect(64)`,
   `InitBossCamera(200, 1000)`, `InitCenterPos()` (the arena's
   `DMY_center01`), `InitStageEffect()`.

Both `ccAnimateObject`s (`m_animate`, `m_animateDead`) start at
transparency 1.0 and scale 1.0.

## A frame: `ccBoss::Main`, `ccBoss01::Main`

```
ccBoss::Main:
  if exit: return
  CalcReal(0); CalcTargetInfo()
  centerPosP = W2P(centerPos); centerDist, centerDirc from posP
  if stop: stopCount++ (at 61: stop = 0, stopCount = 0); moveSpd = 0
  else stopCount = 0
  Think(); Action()
  if lockPlayer:
    every party member on a command list gets EntryAffect(this, 5, 32767) (held)
    reserved menu forbid (1) and no menu open: ccMenu.forbid = 1
      (and forbidChatExcept if reserved)
  elif reserved unforbid (-1) and no menu open: ccMenu.forbid = forbidChatExcept = 0
  Move()
  boss camera, stage effect, PreDrawAnm / Draw / DrawAfterImages when drawn
  condition.hold = 0
ccBoss01::Main:
  ccBoss::Main(); condition.hold = 0; DrawCross()
  if (lockPlayer or eraseTarget) and not the Epitaph and PPcount > 0: PPcount++
```

`Think` and `Action` read `anmStatus`, which `PreDrawAnm` set the frame
before: `anm._AnimateForward(anm.speed)` has reached the clip's end.
Skeith's `PreDrawAnm` also sets the wave's `ccAnm` speed to 512 in acts 4
and 13.

`Move` steps `posP` by `moveSpd` along `moveDirc` (`x += sin`,
`y -= cos`) plus `moveVector`, then resolves the body hit.

## Actions and patterns

`ChangeAction(n, forbid, anm)` gates on `actForbid`:

| `forbid` | effect |
| --- | --- |
| 0 | only if `actForbid == 0` |
| 1 | `actForbid = 0`, always |
| 2, 3 | `actForbid = forbid`, always |

On success it sets `actNum = n` and zeroes `actProccess` and `actCount`.
If `anm` is set and `actAnmTbl[n]` names one, it sets that animation.
Act 12 sets `epitaph`.

`ChangeNextPattern` wraps `patIndex` at the table's -1. It zeroes the
movement and changes to 12 (the Epitaph) or 0 with forbid 3 or 1. Then
`patIndex = ExecPatternIndex(patTbl, patIndex)`.

A pattern is a word and its operands. Skeith's `ExecPatternIndex`
handles:

| word | operands | does |
| --- | --- | --- |
| 1 | | the wave: act 13 in the Epitaph, else 4 |
| 2 | `t` | wait `t` frames (-1: 30 unless the party is gone; -2: until the skill ends) |
| 9 | `type`, `skill` | a skill on the target `SelectTarget(type)` picks |
| 10 | `type` | one of skills 157, 158, 160 (`ccRand() % 3`) on the target |
| 11 | `skill` | a skill on itself |
| 14 | | the cross: act 3 |
| 15 | | the Data Drain: act 5 |
| 16 | | the magic: act 6 |

For words 9 and 10 an invalid or dead target falls back to
`SelectTarget(3)`. If that fails too, it waits 30 frames and the word is
tried again. A skill is `ccItemSkillRequest`, then a -2 wait.

The base `ccBoss::ExecPatternIndex` handles every other word:

| word | operands | does |
| --- | --- | --- |
| 3 | | escape (act 16) |
| 4 | | chase (act 18) |
| 5 | | return (act 17) |
| 6 | `d` | dash `d` units across the centre (act 7) |
| 7 | | wander (act 8) |
| 8 | | neutral |
| 12 | `skill` | a skill on each living party member |

The tables (words, -1 ends):

```
Normal  0x005eb430: 2 30 4 14 4 14 2 60 3 2 30 4 1 2 60 6 1000 2 30 16 6 500
Super   0x005eb490: 2 30 15 2 60 4 14 4 14 2 60 6 500 6 1000 2 30 16 2 120 4 1
                    2 30 6 1000 2 30 16 2 30 3 2 120 4 14 3 2 100
Epitaph 0x005eb530: 3 2 30 11 150 2 30 7 4 1 2 30 6 1500 3 2 30 10 3 7 4 1 2 30
```

Word 5 (return) is in none of them, so act 17 never happens to Skeith.
Acts 9, 10 and 19 write to address 0, and none is reachable.

## The acts

| act | name | anim | what it does |
| --- | --- | --- | --- |
| 0 | neutral | `ex11nut0` | face the target; count the wait down; then the next pattern |
| 1, 2 | damage | `ex11dmg0/1` | as neutral; at the clip's end SE 195, back to 0 |
| 3 | cross | `ex11atc0` | hold the target every frame; at frame 40 skill 0 on it, release, SE 171 |
| 4 | wave | `ex11atc1` | at count 73 the wave's `ANM_xx11wave`, a flash, WaveShock; 25 frames on, skill 1 at its position |
| 5 | Data Drain | | lock the player, cinema; at 45 open menu 74 on the target's slot; when it closes, affect 13 on the target |
| 6 | magic | `ex11mag0` | see below |
| 7 | dash | | fade to half, off the command list, run to the dash point (120 frames at most) |
| 8 | wander | | move away from the target at speed 45, 60 frames or out of the arena |
| 11 | Epitaph | | drained: cheat HP off, the Epitaph table, act 12 |
| 12 | Epitaph neutral | `ex1xnut0` | as neutral; `patNum` 8 also waits for the target within 900 (300 frames at most) |
| 13 | Epitaph wave | `ex1xatc0` | the wave from frame 29 of the clip, skill 1 at count 30 |
| 14 | dead | `ex1xdea0` | off the command list, the dead effect, a 30-frame fade; then exit |
| 16 | escape | | zigzag at speed 35 for 90 frames, off the command list |
| 18 | chase | | fade to half, speed 150, after the target until within 400 (90 frames) |

"The arena" is 1500 units of `centerPosP` (`IsValidArea`).

The magic (act 6) is timed by its effects:

1. **Process 0.** Lock the player, chat excepted. Switch the arena's
   layer. Begin the stage effect (0x64000000). Leave the command list.
   Play `ANM_ex11mag0` and make the MagicSquare. SE 225, cinema 2.
2. **Process 1, count 30.** A ForceGenerator 1000 units above each living
   member. SE 226.
3. **Process 2.** SE 227 at count 130. Once the last ForceGenerator is
   gone, an AutoSamonRing at the target.
4. **Process 3.** Once the ring is gone, an IceBreak at each living member
   (or the target). Stop drawing, end the stage effect, SE 228.
5. **Process 4.** Switch the layer at count 15. While the IceBreak lives,
   draw the reversed layer. Then:
   1. draw again and rejoin the command list;
   2. unlock the player;
   3. skill 2 at the target's position;
   4. SEs 66 (note 52), 65 and 56;
   5. cinema off, the next pattern.

## Hits and the drain: `Affect`

Nothing reaches `bossAffectFunc` unless `EntryAffect`'s checks pass:

- the boss is on a command list (`ccCheckTarget`);
- an enemy's pending affect is not 13;
- the type is not 5 or 6 (5 holds, 6 does nothing);
- the type is not masked.

`bossAffectFunc` calls the vtable's `Affect` (slot 0x24).

```
ccBoss01::Affect:
  if type != 21 and (actForbid == 2 or lockPlayer): return
  ccBoss::Affect()
  if type == 21: HP = maxHP = 4500
  elif type is 1 or 3, patMode == 0, PP >= maxPP / 2:
    the Super table from its start; patMode = 1
ccBoss::Affect (the same gate first):
  5: stop = 1          6: stop = 0
  21: HP = maxHP = maxHP / 10; PP = -1; PPcount = 0
  13: ChangeAction(11); affectType = 0
  7, 9: fly font 20, HP += p0 (to maxHP)
  1, 3: nothing once checkPartyAnnihilation() or compulsionGameOver
        fly font 2 (p0); p0 < 0 stops here; the hit mark
        cheatHP: HP -= p0 / 10, never below maxHP / 2; else HP -= p0
        HP <= 0: ccClearSpcCondition, ClearCondition, ChangeAction(14, 2, 4)
        else for type 1: ChangeAction(1 or 2 by ccSys.count's low bit, 0, 4)
```

`ClearCondition` here is the plain `ccChar::ClearCondition()`: the
conditions, not `PP` or the buffs.

So Skeith cannot be killed before the drain. The drain's target menu sends
13 (the Epitaph), and the drain itself sends 21 (4500 HP, cheat HP off
from act 11 on).

## Targets

`SelectTarget(type, radius)`:

- **Type 0:** the nearest person (`ccSearchNearPerson(this, 3, 0,
  radius)`).
- **Type 1:** the farthest living member.
- **Types 2-14:** a pick among living members:
  - 2 and 3: most and least HP;
  - 4-11: most and least of a personality field (`+72`, `+74`, `+80`,
    `+82`);
  - 12: no conditions;
  - 13: most conditions.

  The pick is dropped if it is farther than `radius`.

If the only PC on the list is dead, the boss keeps facing its way and
there is no target. `CalcTargetInfo` copies the target's positions and
works out `targetDirc` and `targetDist`.

## Skill damage

`ccBossSkillDamage(boss, X, i)` uses `BossSkillTbl[i]` (0x38 bytes each).
It puts the boss back on the command list for the call. `sk + 32` is a
range:

- **Above 0:** every living member within range of the centre (less its
  width). With flag 0x8000, those other than the target take half. The
  centre is the target, or with flag 0x2000 the boss itself.
- **Below 0:** every living member.
- **0:** the target alone.

Damage is `CalcBattleDamage(boss, m, sk, 1.0, -1)`, sent as affect 1.

## Effects

Each `ccBossEff*Create` news an effect and stores it in the first free of
`_g_bossEffManager`'s 1024 slots. It returns the slot, and
`ccGetBossEffAdrs(id)` gives the object. Byte 0 is `m_bEnabled`.

The manager's `Draw` runs each effect's `Draw` (vtable slot 8). It deletes
effects that are no longer enabled, the frame after they clear the flag.
Skeith waits on `m_bEnabled`, so each effect's life is part of its timing:

| effect | Skeith's call | `Draw`s until disabled |
| --- | --- | --- |
| WaveShock | `(pos, dirc, 1.0)` | 45: its `ccAnm` to the end of `ANM_ex31lhit` (46 frames) |
| MagicSquare | `(pos, 0)` | 91: a `ccBossEffLight(pos, 0, 80, 10)`. Returns -1, not the slot |
| ForceGenerator | `(p, (pi/2,0,0,1), 10, 200, 200, 8, 100, 8)` | `life + 10 * ((num - 1) / 2) + 3` = 133. Photon k waits `(k / 2) * 10` |
| AutoSamonRing | `(targetPos, (0,0,targetDirc,0), (0, 1/3, 10), 35)` | 20: its `ccEffSamonRing` fades out, whatever the model (35) |
| IceBreak | `(pos, 2.0)` | 123: a flash, 61 frames, 61 more |
| Dead | `(pos, pos, 0)` | 120 |

The ForceGenerator formula was checked for num 1-8 and several lives.

### The effects' pictures

`ccBossEffManager::Draw` (0x00461750) is the task `ccThBossEffect`
(priority 66), which runs before the boss's own task:

```text
SetActiveLayer(3)                      effLayer (priority 20)
each of the 1024 slots in order:
  m_bEnabled 0: delete (vtable +0xc, 1), the slot freed
  else: Draw (vtable +0x8)
SetActiveLayer(5); ccBossBlur::Main; the cinema's Draw
```

Each effect as Skeith makes it:

- **WaveShock** (`Create` 0x00478820, `Draw` 0x0046b3f0). The `Create`
  plays SE 36, 68 and 56 (note 60) at pos. It makes an Eruption:
  `g_eruptionGenerator` at pos with `pTexMod` 158, in a slot left
  disabled, so the next pass deletes it. The WaveShock itself holds
  `ANM_ex31lhit` of `xeffect`. Each `Draw` steps it and draws it at
  `SetMatrix_PosRotZYXScale(pos, vf0, 5 scale)`. At the clip's end it
  clears `m_bEnabled`. The dirc is not used.
- **MagicSquare(pos, 0)** (0x00478b10) plays SE 228 and starts
  `MagicSquareGenerator` rows 0, 1 and 7 at pos with `distSW` off. It
  then makes a `ccBossEffLight(pos, 0, 80, 10)` and returns -1. (Squares
  1 and 2, and the light's modes 1 and 2, are Fidchell's:
  [boss-fidchell.md](boss-fidchell.md).)
- **Light** (constructor 0x00466a30, `Draw` 0x00466df0) is an omni light:
  `ccLight(4, 1)`, `ccOmniLight::Init` (intensity 1, no fall-off). Each
  Draw places it at pos plus (0, 200, 300) turned about x by -pi/2, then
  about z by the camera's heading (`cameraGetRot(camID)`'s z). It waits 10 Draws,
  then joins the light group (`AddGrp`) for 80 more. Its blue climbs by 10
  a Draw to 250, holds 255 for 15 Draws, then falls by 10. The colour is
  `ccSetColor(b << 16 | g << 8 | g)`, with g 1 while b is 2 or more. The
  characters are lit by it with the arena's lights. The value goes to an
  unsigned through `fptoui` on Infection and Mutation (0 for a negative
  b) but an inline `cvt.w.s` on Outbreak and Quarantine: there a b below
  0 keeps its high bits, which `ccSetColor` reads as an alpha byte and so
  as an HSV colour (a grey).
- **ForceGenerator** (0x004797e0) makes a `ccBossEffBrightMagicSquare`
  (0x0046dd90, `Draw` 0x0046e320) of num photons, each an `EFF_x000` of
  `particle` with `CLT_x000c5` (clt 8). Photon k waits `(k / 2) * 10`
  Draws, then starts its `g_energyOutGenerator`, which follows the photon
  (`syncPos`). It circles its centre at pi/16 a Draw, its radius going
  from r0 to r1 over life, and falls by speed (10) a Draw. It is drawn at
  scale 10 until its life runs out; then its generator is killed.
- **AutoSamonRing** (0x00479900, `Draw` 0x0046ed70, `ccEffSamonRing::Draw`
  0x00462660): `SetModel(35)` is `particle`'s `CMP_x033`. It is drawn
  bursting: the scale grows from 0 by 1/3 a Draw and the transparency
  falls from 1 by 0.05. It is gone once that is below 0.
- **IceBreak** (0x00479240, constructor 0x0046c950, `Draw` 0x0046cc10).
  The constructor draws `ccRand()` once. It starts `g_IceSmokeGenerator`
  at (pos.x, pos.y, 180), offset 100 up. The first Draw flashes the screen
  (60 frames, 0x80e0ffff). The crystal, `particle`'s `CMP_x201d`, is then
  drawn at scale 2. After 60 Draws the smoke is killed. After 60 more come
  twelve `effIceRock`s: three sizes four ways from 100 above pos, ids 15-18
  by `rand`. Then a 30-frame flash, and it is disabled.
- **Dead** (0x004795d0, `Draw` 0x0046a440) runs four stages of
  `BossDeadGenerator` rows, raising its z each time. The first stage (z
  +220) comes at once, with SE 56. The second (+20, `ccSeOnNote(104,
  36)`) comes 35 Draws later, the third 60 after that, the fourth 20
  after that. It is disabled once the last generator has ended.

No Skeith effect reads `ccIBossEff::cmn_bossCam`.

**The port.** `piney_effect::boss` holds the effects: `Effects::boss_create`
and `Effects::boss_step`, the manager's pass. Their draws go first in the
effects' draw. The field's effect tasks gain two calls around the boss's
frame (`FxTasks::boss_effects`, `boss_shows`):

1. The manager's pass comes before the boss's task, as the game's task
   order has it.
2. The boss's `Out::Effect`s are made after it.

The Creates draw from the battle's `rand` and `ccRand` as the game's do.
The sounds go to the area mode as notes, positioned or not. The flashes go
to `scFadeDef`. The light joins the characters' light list. In the game the
Creates run inside the boss's `Main`; the port makes them right after it,
before any other draw from `rand` that frame.

**Checked.** `tools/test_boss_effect_rs.py` runs the game's
`ccBossEff*Create` and the manager's `Draw` loop in eemu against
`boss_probe`, frame by frame until every slot is free. It covers each of
the six with two cases each, and `OnMagicAtk`'s scripted order of
MagicSquare, ForceGenerator, AutoSamonRing and IceBreak with a WaveShock
and the dead effect alongside. It compares:

- every draw in order (the animation's step, the clump's transparency,
  each sprite's place, scale, turn, colour, transparency, CLUT and
  pattern, the matrix, the layer);
- the sounds, the flashes;
- the generators started (rows and force fields by address, `distSW`,
  `syncPosType`, `pTexMod`, pos, offset, the photon followed) and their
  `killFlag`;
- the light (in the group, place, colour), each slot's `m_bEnabled`;
- the effect slots the ice rocks fill, `rand`'s state and the `genrand`
  count.

They match, on Infection's disc and on Outbreak's (where Fidchell's spells
and magic squares are run too). `genrand` (behind `ccRand` and `ccRandF`)
is a 32-bit LCG on both sides. The particle system itself is not run
there (its own tests cover it): no particle is ever free.

### The cross's trail: `DrawCross` and `ccLattice`

`ccBoss01::DrawCross` (0x0047c390) runs at the end of each
`ccBoss01::Main`:

```text
act 3 (the cross):
  once: DMY_xdummy_w01, DMY_xdummy_w02 (the sword's ends) and x11's
        OBJ_ex11swd (GetObjAdrsF)
  each through the sword object's last world matrix (sceVu0ApplyMatrix)
  lattice.NextVertex(); SetPos(w01, 0); SetPos(w02, 1)
every frame: lattice.Disp()
```

The lattice is `new ccLattice(2, 7)` (0x00579ad0): 16 rows of two
vertices, colour type 7 (magenta, `latticeAttributeColorTable`
0x006519b0).

- `NextVertex` (0x0057a3a0) moves the head one row back, the tail with it
  when they meet, and gives the new row life 6.
- `Disp` (0x0057a470) makes a strip from the head row to the tail row
  (`MakePacket` 0x00579cb0). A row's alpha is `life * 48 / 5` under life
  5, else 48.
- `SendPacket` (0x0057a0e0) sends it only once 16 rows have been made:
  TEST 0x73001 (depth GREATER, no write), ALPHA 0x44 (the usual mix),
  PRIM 0x4c (a gouraud strip), sorted on effLayer by the vertices' mean
  world z.
- Each row's life then drops by 1; a row at 0 pulls the tail back.

A vertex behind the eye, or more than 640 x 576 pixels off the display's
corner, gets ADC and draws no triangle.

The party's weapons use the same lattice
([battle.md, the party's weapon trails](battle.md#the-partys-weapon-trails)).
Job 4's has three columns. `Disp` then sends a strip for columns 0-1 and
another for 1-2 in the one packet, each starting with ADC. A middle
vertex's z goes into the sum once, and the key is the sum over twice the
pairs sent. `ClearCnt` (0x00579c50) empties the ribbon when +0x72c is
set (`ClearArmsEffect`). `ArmsEffect` also writes the colour type (+4),
which `MakePacket` reads each time.

`MakePacket` transforms each vertex once a `Disp` (a flag per vertex,
cleared in `SendPacket`) and adds its world z to a sum; `SendPacket`
divides the sum by twice the rows for the key, and also clears the
first-row flag.

**The port.** `piney_world::lattice::Lattice` is `ccLattice`. The boss's
field run (`BossRun`) keeps one: in the cross it takes the two dummies
through `OBJ_ex11swd`'s world matrix at the actor's drawn pose, and
`FieldWorld` sends the strip each frame (`trail_packet`) as the triangles
it draws.

**Checked.** `test_boss_effect_rs.py`'s `test_skeiths_trail` drives the
game's `ccLattice(2, 7)` in eemu as `DrawCross` does: runs of cross
frames (`NextVertex`, both `SetPos`, `Disp`) between runs of `Disp`
alone: 605 frames, 443 of them sending a strip. It compares the port's
lattice after every call:
head, tail, each row's life and vertices, rows made, full, the first-row
flag. After every `Disp` it also compares the strip `SendPacket` sent
(each vertex's RGBA and ADC as `MakePacket` wrote it, or none) and the
sort key, to the bit. They match. `ccTransPosFW2LW` and
`sceVu0RotTransPers` are stand-ins that keep every vertex on the screen,
so the off-screen ADC is not covered.

### The reversed layer: `ccBufferReverce`

`OnMagicAtk`'s last part keeps a `ccBufferReverce` (main 0x00106850,
`MakePacket` 0x00106890) drawn on the boss's own layer, `Init(1, sysLayer's
view)`: priority 1, over the map and under the characters. It is one
sprite over the view (0, 0)-(512, 384), white (0x80ffffff). The blend is
`(Cs - Cd) * FIX + 0` with FIX 128, so the picture under it comes out
inverted. TEST 0x1001 fails every pixel's alpha test into the frame
buffer only; no depth test. The port draws it (`reverse_packet`) at
priority 1 while the rules report `Out::Reverse`.

## The camera: `ccBossCam`

`InitBossCamera(z, y)` (0x0045e750) news a `ccBossCam` (0x140 bytes) with
`PView` the boss's `pos` and `transfer` (0, y, z, 1), sets `bossCamSW`, and
hands it to the effects (`ccIBossEff::cmn_bossCam`). Skeith's is
`InitBossCamera(200, 1000)`. The constructor (0x00460ca0):

- `tempChar` is the player (`plw+0x20`), `MoveTransfer` is `transfer`,
  `MaxRenge` 1000, `RemitRotMax` and `RemitRot` 1280, `LimitRot` 1.0,
  `RenXrotFlg` and `InitLock` 1, everything else 0;
- `changeCamera(2)`: `bcam` becomes the active camera, of type 3;
- `CamPos` is the player's place.

`ccBoss::Main` runs `CamMain` (0x0045fbe0) after `Move` while `bossCam`
and `bossCamSW` hold:

```
if not lock:
  CamView = boss pos
  SetTransfer()                         the pad, or a reset easing
  CamPos = player pos; d = ccGetDirc(CamPos, boss pos)
  memDircZ = RAD2DEG(d); deg = RAD2DEG(CamRot.z)
  CamRot.z = CamZRotDeg2Dad(memDircZ, deg)
  t = max(0, (transfer.y + MaxRenge - MoveTransfer.y) / MaxRenge)
  x = Xrot (BasisRot + t LimitRot)      0 for Skeith: Xrot stays 0
  CamPos += Rz(ZrotCont) Rz(CamRot.z) MoveTransfer
  CamView = CamPos + Rz(ZrotCont) Rz(CamRot.z) Rx(x) (0, -1000, 0, 1)
camera 2's pos and view = CamPos, CamView (+ Quakevector when QuakeFlg)
Quakevector = (0, 0, 0, 1); QuakeFlg = 0
```

So the eye stands `MoveTransfer.y` behind the player on the line from the
boss and 200 up, looking level 1000 ahead: the boss is always in the
middle of the picture.

**The turn** (`CamZRotDeg2Dad` 0x00460ea0), in 16-bit angles:

1. The difference is `(short)(target - cur)`.
2. Past `RemitRotMax` it is held to it and `count` goes up.
3. After 20 such frames it is doubled.
4. At 113-127 it is +-128 (and `count` 2); under 112 it is 0 (and `count`
   0).
5. The heading moves a quarter of it. The first call (`InitLock`), or a
   difference of 0, takes the target outright.
6. Within 128 of the target (as ints, with no wrap) it is the target,
   `count` 0 and `RemitRotMax` back to `RemitRot`.

**The pad** (`Pad_Control` 0x004602a0, from `SetTransfer` while `ResetFlg`
is 0) moves `MoveTransfer.y` between `transfer.y` and `transfer.y +
MaxRenge` by `checkCameraDistModValue`:

| `cameraControlType` | zoom | `ResetFlg` 1 (ease in) | `ResetFlg` 2 (ease out) |
| --- | --- | --- | --- |
| 0, 1 (A) | right stick within pi/4 of angle 0 up to the far end, within pi/4 of pi down to the near end (power 2 or more) | R2 | L2, if not R2 |
| 2, 3 (B) | R2 up to the far end, R1 down to the near end, the same value both | L1 | L2 |

A reset moves a quarter of what is left each frame and snaps to the end
within 8, then `ResetFlg` goes back to 0.

**The quake.** `if (bossCam) bossCam->QuakeCam((35, 35, 35, 1))` in the
wave's attack and the Epitaph's wave: `Quakevector` is three `ccRandF(35)`,
which draw from `ccRand` whether or not the camera still runs.

**The dead camera** (`BeginDeadEffect`):

1. While `bossCamSW`, camera 2's eye, view and rotation
   (`cameraGetRot(r, 2)`) go to camera 1 and `changeCamera(1)` (as
   `OffBossCamera` 0x0045ede0).
2. `bossCamSW` = 0.
3. `changeCamera(3)`: camera 3 at `P2W(posP + Rz(dirc.z - pi/2)(2000, 0,
   2000))` looking at the boss.

`~ccBossCam` goes back to camera 1. The boss is deleted only with its
task, when the mode changes, so camera 3 holds after the fight.

`SetMode`, `SetFreeCamPosView` and `SetRotXLimit` are other bosses'.
Without them `ExLock`, `ZrotControlFlg`, `Xrot` and `ZrotCont` stay 0, and
`ResetFlg` is never 3 or 4. `lock` is only cleared (`OnThinkRandDrive`).

## The cinema: `ccBossEffCinemaFade`

The manager's cinema (+0x00, gcmn 0x0046a7e0) puts black bars over the
top and bottom of the picture while a skill runs, with the skill's name in
the top one. The boss asks for it with `OnCinemaMode(n)` and
`OffCinemaMode`: Skeith's magic asks for cinema 2, and its Data Drain for
-1 (bars only).

- **The constructor** (`Clear`, `Init`) makes the cinema's layer
  (`ccLayer::Init(241, sysLayer's view)`, with the default `SetFrame`),
  two `ccScFade`s on it, and a `ccMask(128, 0)` for the name.
  - `CinemaHight` is 30 and the bottom bar rests at y 354.
  - `NowCinemaData`'s tops are 30 and -30.
  - `tempDat` is left as the heap had it.
- **`OnCinemaMode(n)`** (0x004618b0) does nothing while `CineMode` is 1-3,
  or while the party is wiped out or a game over forced. Otherwise
  `CinemaOn(n)`:
  - with `0 <= n < 61` and a file and texture in `_g_cinemaSkillName[n]`
    (gcmn 0x005eb040, rows of file, texture, row), `SetupSkillName` gives
    the mask that texture, sets its transparency to 0 and shows it. (With
    `game`+0x24 9-12 the file is `x01`-`x04` instead.)
  - `CineMode` becomes 2.
  - Row 2 is Skeith's: `x11`'s `TEX_ske_skl`, row 0 ("Judgement").
- **With a skill** (Outbreak on, 0x004733b0): the same checks, then a
  row by `game.field` and the skill id (OUT 0x00472550); see
  [boss-fidchell.md](boss-fidchell.md).
- **`OffCinemaMode`** calls `CinemaOff` (`CineMode` 4) unless `CineMode` is
  0, 4 or 5.
- **`Draw`** (0x0046ad10) runs each frame in `ccBossEffManager::Draw`,
  before the boss's task.
  - **The name** is drawn first while it is shown: cell 0, 256 x 20 texels
    at (128, 10), `wv` = row x 320, at the name's transparency.
  - **The mode** (table 0x006aa950):
    - 0 stops showing the name;
    - 1 holds the bars at rest (top 0, bottom 354) and raises the name's
      transparency by 1/15 to 1;
    - 2 and 4 draw the bars where `tempDat` has them and step to 3 and 5;
    - 3 slides them in: `NowCinemaData`'s two tops are each divided by
      1.5 a frame (0 once under 1 in size). The bars go at `0 - NowTop`
      and `354 - NowBot`. Once both were 0: mode 1, the tops back to 30
      and -30.
    - 5 slides them out: the name's transparency falls by 1/15, and the
      bars go at `0 - (30 - NowTop)` and `354 + (30 + NowBot)`. Then
      mode 0.
  - **A bar** is `ccScFade::EntryFlash(1, 0x80000000, 0, y, 512, 30)`
    (0x00160240): an element from black at alpha 0x80 to the same colour
    at alpha 0 over one frame, deleted when done. It shows as an opaque
    black band, drawn by both faders' `SendPacket`.

**The port**: `piney_world::cinema::Cinema`, in `BossRun` (`cinema`,
`cinema_sent`).
- **Order.** It steps at the start of the boss's frame (the manager's
  time) and takes `Out::Cinema` after `Main`.
- **The name** comes from `BossLook::skill_name` (x11's `TEX_ske_skl`,
  looked up when the boss's files load).
- **The draw.** The field draws the cinema on layer 241 with
  `piney_desktop::sprite` and `fade::draw_rect`.
- **`tempDat`** starts at 0, as a fresh heap block has it. So the first
  cinema's first frame draws both bars at the top.

`tools/test_cinema_rs.py` checks it (below).

## In the port

`piney_battle::boss` holds the rules:

- `Boss::main` dispatches on `Boss.class`: `Class::Skeith` is
  `ccBoss01::Main`, the effect manager's pass included; `Class::Innis` is
  Mutation's `ccBoss02` ([Innis](boss-innis.md)), `Class::Magus`
  ([Magus](boss-magus.md)), `Class::Kyvia` ([Kyvia](boss-kyvia.md)),
  `Class::Fidchell` Outbreak's `ccBoss04` ([Fidchell](boss-fidchell.md)).
  Each boss's tables come from the build through `BossData`
  (`tables::combat`).
- `Foe.boss` carries the boss's state.
- `AffectFunc::Boss` sends an affect on the boss to `boss::entry`
  synchronously, as `bossAffectFunc` does. The Cx a frame needs comes from
  `AffectCtx.boss`.
- What the rules ask of the rest of the game is returned as `Out`:
  - sounds, flashes and hit marks;
  - effects made, with their slot;
  - the menu's bar, cursor and stream menu, and the cinema;
  - the arena's layer, the stage effect and the reversed layer;
  - the dead camera, skill requests, `ccDeleteCmnd`.

`piney_data::anim::forward_clip` is `_AnimateForward` over a clip's
frame count. Skeith's `ccAnm`s use it with the lengths of `x11`'s and
`xeffect`'s clips.

**In the field** (`piney_world::combat::boss`):

1. An event's `entry 7 code` makes the boss at the entry control's
   set-up (`FieldWorld::start_boss(code)`, `Combat::start_boss`): code 0
   Skeith from `bossTbl` row 0, code 1 Innis from row 1 (2 Magus, 3
   Fidchell, 12 Kyvia on their pages), its centre the arena's
   `DMY_center01`. Other codes start nothing.
2. `ccBoss01::Main` runs each frame after the entry control, the task
   order's 66.
3. It draws as an actor of `x11`'s `CMP_trall` at the boss's clip and
   frame, heading and `setTransparency`. The frame is the one
   `PreDrawAnm`'s step posed (`Anm.posed`), and the after-images and the
   wave take theirs the same way.
4. The boss joins the enemies Kite and the HUD target.
5. Affects from Kite's and the members' frames, whose contexts carry no
   `BossEnv`, are queued on the boss and applied first in its task, the
   same frame.
6. Outputs:
   - `Out::Skill` runs the skill through `Combat::item_skill`;
   - hit marks and damage numbers go to the effects as rule events;
   - sounds, `scFadeDef` flashes and `SwitchLayer` go to the area mode;
   - the menu locks go to `ccMenu`'s forbid and cursor;
   - the member drain opens menu 74 ([`StreamMenu`](field-ui.md#a-member-drained-streammenu-menu-74));
   - the body hit goes on the collision list;
   - the stage fader, the after-images and the wave are drawn;
   - the quake and the dead camera go to the boss camera, and the dead
     effect's `ccSqFade` to the music;
   - the effects are made and drawn ("The effects' pictures"), the
     cross's trail and the reversed layer drawn;
   - the cinema ([above](#the-cinema-ccbosseffcinemafade)) is drawn;
     `ccBossBlur` is not (below), and no script of this volume holds the
     boss (`hold 7`).
7. The boss camera (`piney_world::bosscam::BossCam`) is made with the
   boss and runs after its `Main`.
8. The area host answers `present/absent 7` from the task's exit.

`tools/test_bosscam_rs.py` builds the game's `ccBossCam` in eemu over the
field's camera and player tasks. Each frame it runs `CamMain` with the
boss moving, now and then after a `QuakeCam`, over the four control
schemes and random pads, then `OffBossCamera`. It compares the camera's
fields, the cameras and the view matrix with the port's.

`tools/test_cinema_rs.py` builds the game's `ccBossEffCinemaFade` in eemu
by its constructor, with a manager holding it. It turns the cinema on
(numbers 2, -1, 0, 5 and 70) and off at random frames over eight runs of
300-600 frames, and runs `Draw` every frame against `cinema_probe`. It
compares every frame's name (transparency bits, `wv`; the cell, place and
size constant), each bar's `EntryFlash` (y bits; frames, colour, x, width
and height constant), and which ons set a name (`SetTex`'s file and
texture: `x11`, `TEX_ske_skl`).

`tools/test_battle_boss_rs.py` runs the game's `ccBoss01` in eemu against
the port frame by frame over random cases. A case is:

- a party of one to three;
- Kite's hits and gauge;
- the drain, and hits enough to kill after it;
- menus opening.

It compares the acts, patterns, waits, movement, target, HP, gauge, flags,
both animations, the effects, the party's hold, the calls, the menu state,
and both random generators. Its effects are stand-ins with the port's
lives. `test_effect_lives` runs each real `Create` and `Draw` and checks
those lives against them.

## Not described yet

- **`ccBossBlur`** (the manager's +4, `Main` gcmn 0x00461200) is a
  `ccBufferSampling` on the manager's layer (200, sysLayer's view).
  - The constructor sets scale 1.01, end scale 1.01, turn 0, colour
    0x40808080, no function and not enabled.
  - It never sets `m_exit` (+0x28 of the blur), and `operator new`
    (`_ccMalloc`) does not clear memory.
  - Each frame that `m_exit` is 0, `Main` eases the scale to its end
    (1/30 of the gap a frame, held at 1.0 from above) and the turn to
    its end. It then draws the feedback at alpha 0x40.
  - It sets `m_exit` only once both have arrived, and with the end scale
    above 1 the scale stops at 1.0 without arriving, so a zero `m_exit`
    would blur every frame of the fight. Only Kyvia's
    `MagicalBall` (later volumes) gives it a function.
  - Whether Skeith's fight blurs depends on what the heap left there, so
    it is not ported.
- **`hold 7`**, the event instruction that holds the boss while
  `eventMng.bossTscb`+0x14 is not 0: no script of this volume uses it.
- What the heap leaves in the cinema's `tempDat` before its first slide.
- `test_battle_boss_rs.py`'s effects are stand-ins with the port's lives,
  and they draw nothing from `rand`. In the field the real effects and
  their particle generators draw from the battle's `rand`, as the game's
  do, but no test runs the rules and the effects together against the
  game.
- `ccBossEffMagicSquareCreate` for n other than 0 is not Skeith's and not
  ported.
