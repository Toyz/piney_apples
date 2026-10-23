---
number: 432
title: Every boss fight draws its blur: ccBossBlur's m_exit is written 0 by every boss's constructor but Gorre's, whose word is the heap's residue of the play since power-on, and the blur is ported and checked against the game's Main in every boss harness
date: 2026-10-22
area: battle, render, test, tooling
files: crates/piney-battle/src/boss/blur.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/innis.rs, crates/piney-battle/src/boss/magus.rs, crates/piney-battle/src/boss/magus/leaf.rs, crates/piney-battle/src/boss/fidchell.rs, crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/kyvia.rs, crates/piney-battle/examples/battle_probe/boss.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/blur.rs, tools/test_battle_boss_rs.py, docs/engine/boss.md
resolves: 117, 128
started: 2026-10-22T19:46:52-06:00
took: 1h 1m
---

# 432. Every boss fight draws its blur: ccBossBlur's m_exit is written 0 by every boss's constructor but Gorre's, whose word is the heap's residue of the play since power-on, and the blur is ported and checked against the game's Main in every boss harness

Worklog 117 left `ccBossBlur` unported. Its reason: `m_exit` (+0x28 of
the blur) is never written, and the blur's memory comes from
`_ccMalloc`, which does not clear, so whether Skeith's fight blurs
"depends on the heap". Worklogs 422 and 266, and the Innis and Magus
pages, said the same, and no fight drew it. Reading every writer of the
blur showed that only Gorre's fight depends on the heap.

## Who writes the blur

`ccBossBlur` is the effect manager's +4 to +0x34 (`ccBossEffManager`'s
constructor, INF gcmn 0x004614b0). Each boss writes it through `ccBoss`
+0x148, which `ccBoss::ccBoss` points at it (0x0045bfb4). The earlier
reading searched for `_g_bossEffManager` and missed every write through
+0x148. A register-tracked scan of every function on all four volumes for
stores through a word loaded from +0x148 found:

| boss | constructor's writes | later |
| --- | --- | --- |
| Skeith `ccBoss01` (INF 0x0047bc40) | +0x24 1, `m_exit` 0, +0x2c `ccBoss03BlurParamCB` | |
| Innis `ccBoss02` (MUT 0x004949a4) | 1, 0, colour `DefaultARGB`, scale 1.01, turn 0.02 | colour in `Action`, `MagicSquare`, `SkillAttack`, `SwitchActionPattern` |
| Magus `ccBoss03` (MUT 0x0049e614) | 1, 0, colour 0x48808080, scale 1.01 | `OnThinkRise` 0x60808080; a leaf's `OnThinkFire` 0x48808080 |
| Fidchell `ccBoss04` (OUT 0x004a3afc) | 1, 0, `ccBoss03BlurParamCB` | |
| Gorre `ccBoss05` | none | none (only `OnCinemaMode`, `OffCinemaMode`) |
| Kyvia 1-4 | 1, 0, scale 1.0, `DefaultARGB` | scale and colour by act |
| `ccBoss06`, `07`, `08_b`, `08_c` | 1, 0 and callbacks | |

`m_exit` 0 is written over the heap's word in every fight but Gorre's and
Quarantine's `ccBoss08`'s first form. Nothing reads the blur but
`ccBossKyvia01::CheckDiscMove`'s second stage, which Mutation's one-stage
disc never reaches, and Quarantine's `ccBoss06`.

## What the blur does

`ccBossBlur::Main` (0x00461200) runs at the end of the manager's `Draw`,
before the boss's task:

- with `m_exit` not 0, nothing;
- enabled with a callback, the callback sets scale, turn and colour;
- disabled, the scale eases to 1.0 by a thirtieth of the gap from its
  start (+0x10) a frame, held at 1.0 when it passes. The turn eases to 0
  from its start (+0x18) the same way. Both arrived set `m_exit` 1; from a
  turn of 0 the turn never arrives, so it never exits.
- `SetReflex(scale, turn, colour)`, then `MakePacket` on the manager's
  layer (`ccLayer::Init(200, sysLayer's view)`): the last frame's picture
  over the view at that scale and turn, blended at the colour's alpha.

`ccBoss03BlurParamCB` (INF 0x004610e0; OUT 0x00472b90):

- `blurRad` += pi/30, less 2 pi past pi;
- scale `1 + 0.01 sin(blurRad)`, turn `pi sin(blurRad) / 1800`, colour
  0x48808080.

`blurRad` is a function static: Infection and Mutation set it 0 at the
first call (`init$1068`), Outbreak and Quarantine start from the .bss's
0. No fight resets it, so it runs on from fight to fight until power-off.

The manager's task (`ccThBossEffect`) is started by `ccBossEntryStart`
(0x0045b2a0) without keeping its handle. It outlives the boss's task and
draws the blur until the scene ends.

## Gorre's m_exit

The manager is `operator new(0x1038)`, which is `_ccMalloc(0, 0x1038, 0,
0)` (main 0x001006c0). The size rounds to 0x1050.

- 0x1050 is past 4096, so `ccHeapFree::SearchFree` (0x00100500) walks
  the large free list. `AddTbl` (0x001005a0) inserts it in address
  order, so the block taken is the lowest-addressed free hole of 0x1050
  or more.
- The header goes at the hole's start, nothing is cleared, and the
  manager's constructor never writes +0x2c. `m_exit` is the word 0x3c
  into that hole: what its last owner wrote there.
- `_start` (0x00100008) clears only the .bss (0x00378880 to `end`,
  0x00730600). `ccHeap::InitFirst` (0x00100370), called once from
  `main`, takes the region past it. Nothing else sets `heapTop`,
  `heapNow` or the free lists: the heap runs from power-on with no reset.
- `ccBossEntryStart` allocates the two tasks' stacks (2048, and 4096 in
  the large list) just before.

Which hole is lowest, and what lay 0x3c into it, follows from every
allocation and free since power-on. That history depends on the player:

- the party's members: their files are decoded into the heap on entering
  each area, and their sizes differ (Outbreak's `cbu1body` 284,672 bytes,
  `cha2body` 307,200);
- the areas visited and their order;
- the enemies met, the menus and items used.

No code in the repo can give those, so there is no single value. The port
takes a word that is not 0 (`combat::boss::HEAP_EXIT`): no blur in
Gorre's fight, one of the game's two outcomes.

## The port

- piney-battle:
  - `boss::blur` is `ccBossBlur`: `Blur::new(exit)` (the manager's
    constructor), `Blur::set` (`Set`: `Enable`, `Scale`, `Turn`, `Abgr`,
    `Func`, `Ease`), `Blur::main` with `BlurFunc::Boss03` and its
    `Statics`.
  - Every boss's writes are `Out::Blur(Set)`, in the game's order: Skeith,
    Innis, Magus and its leaves, Fidchell, Kyvia.
  - Innis's and Magus's own copies of the colour and Kyvia's
    `kyvia::Blur` are gone.
  - `Boss::take_blur` takes the writes an affect left pending.
- piney-world:
  - `BossRun::blur` is made with the constructor's writes. Its `main`
    runs at the end of the manager's pass (`Combat::blur_pass`), after
    the pending writes, as long as the manager's task does: past the
    boss's exit.
  - The field draws the reflex on layer 200 with
    `piney_desktop::noiz::Sampling::reflex`, the `ccBufferSampling` the
    streams already use.
  - `Combat::blur_statics` holds the callback's statics.
- piney-game: the session carries the statics from area to area.

## Checks

- Each boss harness now builds the manager with the game's own
  constructor at its address, over a random heap word at `m_exit` and
  random callback statics. Its pass runs `ccBossBlur::Main` natively
  (`SetReflex` recorded). Every frame it compares the blur's members,
  `blurRad` and the frame's `SetReflex` with battle_probe's `ProbeBlur`.
  The cases, every frame of each agreeing:
  - Skeith 60 on Infection and 40 on Outbreak;
  - Innis and Magus (Mutation) 60 each;
  - Fidchell 60 (179,158 frames), Gorre 60 (199,700);
  - Kyvia: Mutation 60 (76,299 frames), Outbreak's second fight 30
    (93,172) and its first 20 (27,380).
- `test_battle_boss_rs.py`'s `test_blur_main` runs `Main` on 200 blurs in
  any state, the callback with and without its statics, on all four
  volumes. Every frame agrees, through every branch of the ease and the
  exit.
- The Fidchell harness's stand-in for `ccItemSkillRequest` left out
  `_ccSkillRequest`'s `targetChar` (gcmn 0x00572b80), and its target
  index did not know the boss's own address. A case where Fidchell casts
  on itself parted at both, with or without this work. Both are fixed,
  and the Gorre and Magus stand-ins write `targetChar` too.
- `test_battle_boss_rs.py` on Outbreak (its first run there) parted
  twice, with or without this work:
  - frame 1's centre distance by one ulp: `ccBoss`'s distances took
    Infection's `ccGetDist`, and Outbreak's square root is the FPU's
    truncating `sqrt.s` (OUT main 0x001e6ec0). Every boss's distances
    now go through one `boss::dist` on the volume's: Kyvia's on
    Outbreak changed too; Fidchell's and Gorre's own copies went.
  - `SwitchLayer` has no name in Outbreak's build, so the hook never
    took. The harness finds it as `OnMagicAtk`'s unnamed callee
    (0x00417db0), as the Gorre harness does.
  Its tests now pass on all four volumes.
- `session::tests::blur::every_fight_shows_its_blur` plays 1,500 frames
  of each fight. Every frame that draws the field draws the feedback
  strip:
  - Skeith and Fidchell: 0x48808080 at 59 scales and 1,273 turns;
  - Innis: 1.01 turned 0.02, in `DefaultARGB` and `ExARGB`;
  - Magus: 1.01 in both its colours (a stream plays over 664 of its
    frames);
  - Kyvia: both fights at their scales and colours;
  - Gorre: none.
- piney-battle, piney-world and piney-game's suite (four threads),
  clippy, fmt.

Shots (`blur_shots`, /mnt/data/claude/scratch/a432; every frame rendered,
so the feedback reads the frame before):

- `blur-innis-0450.png`: Innis's ghost turned beside it.
- `blur-skeith-0600.png`, `blur-fidchell-*.png`: the callback's slight
  swing.
- `blur-kyvia-*.png`, `blur-magus-*.png`.

**Still unknown:** Gorre's `m_exit`, and so whether its fight blurs: the
word the heap leaves there follows from the player's play since power-on
(party, areas, enemies, menus), which no code in the repo can give (see
above).
