---
number: 439
title: Quarantine's ccBoss08 keeps its first form's m_exit from town 4's room: on the story's way the manager lands where town 4's scene freed its objects and buffers, the party and the title wait choose 0 or a pixel word, and straight into the field it reads RAM no game code writes
date: 2026-10-24
area: engine, tooling, battle, test
files: crates/piney-eemu/examples/ee_run.rs, crates/piney-game/src/start.rs, crates/piney-game/src/session/tests/blur.rs, crates/piney-world/src/combat/boss.rs, docs/engine/heap.md, docs/engine/boss.md, GAPS.md, UNKNOWNS.md
resolves: 432
started: 2026-10-23T23:42:22-06:00
took: 1h 16m
---

# 439. Quarantine's ccBoss08 keeps its first form's m_exit from town 4's room: on the story's way the manager lands where town 4's scene freed its objects and buffers, the party and the title wait choose 0 or a pixel word, and straight into the field it reads RAM no game code writes

Worklog 432 found that one blur besides Gorre's keeps a heap word for its
`m_exit`: Quarantine's `ccBoss08`, until its second form's constructor
writes 0. Worklog 436 settled Gorre's by running Outbreak whole in
piney-eemu. This entry runs Quarantine the same way, to `ccBoss08`'s
first frame.

## The fight and the blur

`ccBoss08` is `bossFunc` code 7, fought in event 313:

- Block 14 runs in town 4 and ends with `area 8` and `scene 1 4 8`.
- In field 8, block 15's `entry type=7 code=7` starts the fight's tasks
  (`ccBossEntryStart`: the boss's and the effect manager's).
- Block 16's `stream 105` plays its 1,100 frames before the first form
  moves.
- Block 19, on a second visit (status 2), enters code 7 again.

`ccThBoss08` (QUA gcmn 0x003aca30):

- With `saveData` +0x652f set, `ccThBoss08c_Main`.
- Otherwise `new ccBoss08` (the first form, which makes its
  `ccBoss08Seed`s). Once that has exited: a flash, its destructor, then
  `ccThBoss08b_Main` (`ccBoss08_b`, which makes `ccBoss08Root`).
- `ccBoss08_c` makes `ccBoss08KillerEye` and the `ccBoss08Eye`s.

The manager outlives the forms; it is `ccThBossEffect`'s
`operator new(0x1038)` (QUA gcmn 0x003657b4). Worklog 432's
register-tracked scan was run on every function of the eight classes.
Writes through +0x148:

- `ccBoss08_b`'s constructor (enabled, `m_exit` 0);
- its `OnThinkBodyPress` and `OnThinkRootDrain` (the callback);
- `ccBoss08_c`'s constructor (enabled, 0, 0x48808080, scale 1.01).

None in `ccBoss08`, `ccBoss08Seed` or `ccThBoss08`. So through the first
form `m_exit` is the word under the manager.

## Running Quarantine

`examples/ee_run.rs` had three of Outbreak's addresses written in. It
now finds each from the program:

- `_start`'s `$gp`: the `lui`/`addiu` on `$a0` before `move $gp, $a0`
  (OUT 0x0038ddf0, QUA 0x00280770; both printed and checked).
- `_ccMalloc`'s `jr ra`: +0x2ac, main 0x0010098c on both.
- The manager's `new`: `ccThBossEffect` + 0x1c (OUT 0x00472eac, QUA
  0x003657bc).
- The stream thread, by its name.

New steering:

- `EE_AREA=n`, or `118,n` in `EE_THEN`, is the event instruction
  `area n` for a story area: `WORLD_MAN.eventAreaNumber` (+0x120),
  docs/engine/area-words.md.
- `EE_FILL=word` fills the heap's one hole, past its record, at the
  first `_ccMalloc`.

piney-game:

- `start::state_at(iso, n)` is `build`'s story replay, for any event of
  the story.
- `session::tests::blur::event_313_save_bytes` writes the save before
  313's fight: blocks 0-14 played, `eventStatus[0]` 1.

The heap model placed all 56,537 handouts of the first run where the game
did.

## Where it lands

| history | the manager | under its +0x2c | `m_exit` |
| --- | --- | --- | --- |
| town 4, members 1 and 2, save at 900 frames | 0x01465c30 | an object of a town-4 clump (`ccClump::Init`'s `new(0xb0)` for an Obj chunk, made at the load, freed at the change) | 0 |
| town 4, member 1, 900 | 0x013a7890 | a 0x10000 `ccBltData` buffer of town 4 | 0xffea7682 |
| town 4, members 1 and 2, 1,440 | 0x014663a0 | the same kind of buffer | 0xbebeb6d4 |
| town 0, members 1 and 2, 900 | 0x014667c0 | a model's DMA colour buffer (`Decode_Mmat01`) | 0x80657077 |
| field 8 at once, Kite alone, 900 | 0x012c7480 | nothing ever handed out | RAM as the boot left it |
| the same, 240 and 1,440 | 0x013c2e90, 0x012c7350 | nothing | the same |

- **The story's way.** Block 14 always runs in town 4 first. So the
  manager lands in the room town 4's scene freed at the change, as
  Gorre's lands in its cutscene's hole. Unlike Gorre's, the cutscene here
  (stream 105) plays after the change and its blocks lie below. The
  party and the frame the save was loaded on pick which owner's word is
  read: a clump object's 0 (the blur shows), or a blit buffer's pixels (no
  blur).
- **Straight into the field** (not the story's way): nothing at or above
  the manager was ever handed out. With `EE_FILL=a5a5a5a5` all three
  runs read 0xa5a5a5a5: no code of the game writes that word between
  `InitFirst` and the manager. The game never reloads itself (no
  reference to `LoadExecPS2` or `ExecPS2`). So the word is RAM as the
  console's boot left it, which the BIOS's loader decides.

A side finding: with the heap filled, town 4 stalls at about 1,440
vblanks, and without the fill it loads. Some code there reads heap memory
it never wrote and expects 0. It is not followed here.

## The port

The port has no `ccBoss08`. `start_boss` takes no code 5-7, so events
307, 310 and 313 go on without their fights. GAPS.md now lists what each
lacks:

- the first form, about 20 KB of code;
- the second, about 28 KB;
- the third, about 31 KB;
- `ccBoss06` and `ccBoss07`.

Porting them is Quarantine's next story work. It is not this entry's.

What the blur needs from the first form is only the word: the first form
neither writes nor reads it. That is ported:

- `piney_world::combat::boss::BOSS08_HEAP_EXIT` = 0, the 900-frame
  history's word on the story's way. The fight shows the manager's own
  blur from its first frame until `ccBoss08_b`'s constructor.
- `boss08s_first_form_shows_the_managers_own_blur` runs `Blur::main` on
  it for 600 frames: a reflex every frame, 0x40808080, turn 0, the scale
  falling from 1.01 to 1.0 and held.

docs/engine/heap.md has the section "ccBoss08's m_exit" and boss.md the
summary.

**Still unknown:** the RAM a console leaves at the manager's +0x2c when
event 313's fight is entered straight from power-on with no town loaded
first. No code of the game writes it (the fill), and the game never
reloads itself, so only the PS2 BIOS's loader decides it, which is not
on the disc or in the repo. The story's own way never reaches that case.
