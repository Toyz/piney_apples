---
title: Bosses - Tarvos
status: partial
volumes: QUA, INF
covers: QUA gcmn.prg:0x003a81e0-0x003ac9f0 (boss07.cpp), 0x003a8300 ccBoss07::ccBoss07, 0x003a8700 ExecPatternIndex, 0x003a8e00 SetPatternMode, 0x003a8eb0 OnMagicCamera, 0x003a9040 SetDefMode, 0x003a91e0 Main, 0x003a9260 Affect, 0x003a96b0 PreDrawAnm, 0x003a9750 Think, 0x003a9920 OnThinkNeutral, 0x003a9a80 OnThinkDrain, 0x003a9bc0 OnThinkDead, 0x003a9cb0 OnThinkSacrifice, 0x003aa060 OnThinkWave, 0x003aa2a0 OnThinkSkill, 0x003aa7b0 OnThinkDashCenter, 0x003aa9b0 OnThinkMagic, 0x003aaea0 OnThinkPhantom, 0x003ab0d0 OnThinkDataDrainAtk, 0x003ab360 OnThinkSpikeShot, 0x003ab560 _SpikeShotCB, 0x003ab620 OnThinkWander, 0x003ab7b0 OnThinkTransfer, 0x003abf20 OnThinkChase, 0x003ac210 OnThinkEscape, 0x003ac4d0 OnThinkReturn, 0x003ac5f0 OnThinkEpitaph, 0x003ac750 OnThinkEpitaphWave, 0x003ac9e0 OnThinkPoison, 0x003738f0 ccBossEffSpikeRain::ccBossEffSpikeRain, 0x00373d60 ccBossEffSpikeRain::Draw, 0x0037a4b0 ccBossEffPhantom::ccBossEffPhantom, 0x0037a980 InitRadiateParam, 0x0037ac50 ChangePtclGenerator, 0x0037add0 ccBossEffPhantom::Draw, 0x0037b7b0 ccBossEffEruptionCreate, 0x0037d180 ccBossEffSpikeRainCreate, 0x0037d760 ccBossEffExplode2Create, 0x0037db00 ccBossEffTransEffectCreate, 0x0037dca0 ccBossEffPhantomCreate, 0x005ec420 boss0xAnmTbl, 0x005ec490 boss07NormalActTbl, 0x005ec510 boss07SuperActTbl, 0x005ec5a0 boss07EpitaphActTbl, 0x005ec5f8 @1178, 0x005ec618 @1517, 0x003781e0 @1568
worklog: 447
---

# Bosses - Tarvos

Tarvos (`ccBoss07`, boss07.cpp) is Quarantine's second phase boss:
`bossTbl` row 6, `bossFunc` code 6, file `x71`, body `CMP_trall` (the
drained form `CMP_trall2`, its clips `ANM_ex7x*`). The names and layouts
are Infection's DWARF (Infection's gcmn carries the class, never
entered); the behaviour is Quarantine's code, over the `ccBoss` base
Outbreak's and Quarantine's share ([Macha](boss-macha.md)).

## Where it comes from

Event 310 (M410), field 7's arena after area 102's dungeon (Cruel
Vindictive Scars): block 6's `area 7` and `scene 1 4 7`, block 7's
`entry type=7 code=6` makes it.

## The class

```text
ccBoss07 : ccBoss (0x293b0)
+0x29350 anmw (ANM_xx11wave)    +0x29354 m_effAura (never set)
+0x29358 m_effSpike             +0x29360 m_effDead
+0x29364 m_effPhantom           +0x29368 m_effStart
+0x2936c m_skillID              +0x29370 m_stageEff
+0x29394 m_patternMode          +0x29398 m_defMode
+0x2939c m_blendRate            +0x293a0 m_blendColor (0x00404080)
```

The constructor zeroes eight more words (+0x29374-+0x29390, one -1) that
nothing reads.

## The constructor

`SetBaseParam(ccGetBossParam(6))`, `boss0xAnmTbl`; 500 behind Kite,
Kite's heading; `OffExit`, `OnDraw`, `OnBodyHit`, `OnCheatHP`;
`ChangeAction(0)`; `ccEntryCmnd`. After-images (30), the stage effect,
`InitCenterPos`, boss effects (32), `InitBossCamera(200, 1000)`; the blur
enabled under `ccBoss03BlurParamCB`. `SetPatternMode(0)`,
`SetDefMode(0)`.

## The guard

`SetDefMode(d)`: 0 and 1 point `param` at `bossTbl` row 42 or 43 (the
physical and the magic guard) and copy its stats into the boss's; nothing
else of the row is taken. 2 is `SetBaseParam(row 6)` whole. A blow
(`affect` 1 or 3) of the kind the guard holds off (`ccSkillCheckType` of
its skill 0 under guard 0, 1 under guard 1) shows `effResistantShield`,
its scale times 0.7 and its offset 80 up. The transfer swaps the guards.

## The patterns

`SetPatternMode(m)` picks `@1201[m]`: 0 `boss07NormalActTbl`, 1
`boss07SuperActTbl` (at half the gauge, from `Affect`), 2
`boss07EpitaphActTbl` (drained). `ExecPatternIndex` has its own words:

| word | act |
| --- | --- |
| 1 | Wave (4), the Epitaph's wave (13) once drained |
| 2 t | wait `t` frames (30 for -1) in Neutral or the Epitaph |
| 9 ty sid | skill `sid` at a target of type `ty` (-1 a living member by `ccRand`) |
| 10 ty | Skill (22): `@1178` drawn and dropped, `OnThinkSkill` casts one of `@1517` |
| 11 sid | skill `sid` on itself |
| 14 | Spike Shot (3) |
| 15 | Sacrifice (5) |
| 17 | Transfer (21) |
| 19 | Magic (23) |
| 20 | Data Drain attack (25) |
| 21 | Phantom (6) |

As Macha's, every operand of its own words is read from `patTbl` at the
running index, whatever table was passed. The base's words bring the
chase (18) and the escape (16); no table reaches the dash (7), the
wander (20) or the return (17).

## The acts

`Think`: 0 Neutral, 1-2 damage, 3 Spike Shot, 4 Wave, 5 Sacrifice, 6
Phantom, 7 Dash Center, 11 Drain, 12 Epitaph, 13 Epitaph Wave, 14 Dead,
16 Escape, 17 Return, 18 Chase, 20 Wander, 21 Transfer, 22 Skill, 23
Magic, 25 Data Drain attack, 26 Poison (returns at once); any other
`ChangeAction(0, 0, 1)`.

- **Spike Shot.** SE 245 (note 57); at 30 frames the spike rain (below)
  at the target, held while it falls.
- **Wave.** `ANM_xx11wave` at 512, SEs 223 and 224 at the body's frames 6
  and 35, a WaveShock at 40 and boss skill 23 fifteen frames on.
- **Sacrifice.** `SelectTarget(2)` (a leftover register's), the magic
  camera, the dark stage; stream 103 on the member at 30 frames; once the
  menu closes `EntryAffect(this, 1, 9999)` on it.
- **Phantom.** At Kite (else the nearest), `ccBossEffPhantomCreate(pos,
  centerDirc)`, the cinema 25, the party held; once the phantom is off,
  boss skill 25 from Tarvos's place.
- **Skill.** One of `@1517` (157, 158, 160) by `ccRand`, the skill start at
  60 frames, the skill once it ends; camera 2 after.
- **Magic.** `@1568[1]` (284), the spell at 75 frames, the camera shaken
  100-160 with `ccNoiz::SetNoizBs(20)` every 20.
- **Transfer.** Off the targets, SE 244 at the clip's 80, 70, 65 and 55,
  stream 102, the trans effect; the fog (`SetFogBlend(m_blendRate,
  0x00404080)` in `PreDrawAnm`) thickening to 70; a random place within
  2000 of the centre, `ANM_ex71out0`, the fog thinning; then the other
  guard. The body is not posed while the stream plays (act 21, step 2).
- **Drain.** HP 7000, the guard dropped (`SetDefMode(2)`), the Epitaph's
  table; the gauge left at -1.

## Its effects

- **`ccBossEffSpikeRain`.** 32 spikes (`xeffect`'s `CMP_ex71body2` at a
  quarter) 200 from a point 500 over the target's head, each on a random
  bearing (`ccRandF(pi)` twice), spike `k` waiting `k * 60 / 32` frames,
  then flying 50 a frame at the target's middle; within 50 (the VU0
  length) it hits (`_SpikeShotCB`: boss skill 24; every eighth sounds 171
  and makes a `ccBossEffEruption`, created off) and fades a fifteenth a
  frame.
- **`ccBossEffPhantom`.** A sprite (`particle`'s `EFF_x000`, `CLT_x000c7`)
  with fourteen lights, each a `g_phantomGen` generator following it: the
  first eight circle up and in, the sprite shrinks, a summon ring (195),
  then all fourteen fly out (`InitRadiateParam`) and fall
  (`ChangePtclGenerator`: row 1 on lights 0-6), each burst
  (`effExplode3`, grown 1 to 4 over 15) a quake; at three bursts a
  radiate and `ccBossEffExplode2Create`. The boss camera follows it.
- **`ccBossEffTransEffectCreate`, `ccBossEffExplode2Create`.** Particle
  generators only, no slot.

The phantom's constructor hands `SetFreeCamPosView` a view it never
writes (sp+144): `operator new`'s frame left `$s1` there, the target's
pointer (a float under 1e-37), its sign word and `$s1`'s upper half,
which no code of the game writes but by saving and restoring. The port
takes (0, 0, 0); the next `Draw` sets the view anew.

## In the port

- `piney_battle::boss::tarvos` holds the rules (`Class::Tarvos`), `eff`
  the spike rain and the phantom (`Fx`), which act on the boss as they
  draw (the hits, the ring). What they show is `Out::Tarvos(Pic)`; the
  spell's noise `Out::Noise`. The tables come through `TarvosData`
  (`piney-gen`'s `tarvos_*` entries).
- `BossEnv::cc` gives a boss's `Affect` the fight's `ccRand` (Tarvos's
  blows' notes).
- `piney_world::combat::boss` starts it for `entry 7 6` (`TARVOS`), its
  body `CMP_trall`, the drained `CMP_trall2`, the waves and the
  transfer's fog over the body.
- `piney_effect::tarvos` draws the spikes, the sprite, the lights'
  generators, the bursts, the transfer's and eruption's generators and
  the shield.

## Checks

`tools/test_battle_tarvos_rs.py` runs Quarantine's code natively against
`battle_probe` frame by frame: 300 random cases (913,177 frames) agree,
every act but the passing ones (1, 2, 11), the forced dash, chase,
escape, return, wander and poison, hit types 1 and 3 and blows by skill;
the spike rain and the phantom run natively, their slots compared. Its
unittests run the stand-ins' lives. `boss::tarvos::tests` hold the
constructor's guard, the shield, the super table, the drain, the rain's
eruptions and the phantom's bursts.

`session::tests::tarvos::tarvos_under_god` (ignored) plays the arena under
god: acts 0, 3-6, 12-14, 16, 18, 21-23 and 25 come, the drain and its
death, and event 310 ends at frame 43,610 with its music through 43,268
of them. `tarvos_fight_to_its_end` and
`event_310_from_its_start` (ignored) play it with no aid.

## Unknown

- The spikes', sprite's and lights' pictures are not checked against the
  game's draw frame by frame.
- The upper half of `$s1` at the phantom's making: what the kernel's
  `StartThread` put in the boss thread's first context (`ccTscb::GoThread`
  makes a kernel thread on a `ccMalloc`'d stack); taken as 0.
