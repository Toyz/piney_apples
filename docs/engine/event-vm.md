---
title: The event interpreter
status: partial
volumes: INF
covers: INF SLUS_202.67:0x001b5a60 ccThEvent, 0x001b5230 ccStartThEvent, 0x001b52c0 ccEnableThEvent, 0x001b5360 ccDisableThEvent, 0x001b5380 ccStartEvent, 0x001b55f0 ccStartEventConvert, 0x001b5ef0 eventSub, 0x001b6160 ccEventFlagSet, 0x001a7400 ccEvent::CheckOpen, 0x001a7120 CheckCurrentOpen, 0x001a6ec0 SetCurrentOpen, 0x001a8d20 ccEvent::Execute, 0x001a6ca0 ccEvent::Init, 0x001b32f0 CheckOperate, 0x001b33d0 AddOperate, 0x001b34a0 DelOperate, 0x001b27b0 DispInfo, 0x00177730 ccSaveData::AddItem, 0x00177af0 DelItem, 0x00177590 ccAddSkill, 0x00177eb0 AddFriendship, 0x00178160 SetGateList, 0x00178480 SetAreaBan, 0x00178570 ClearAreaBan, 0x00178af0 NewMail, 0x00178b80 ReadNewMail, 0x00307140 addItemCategoryTbl, 0x00307180 the friendship caps, 0x00168960 ccSetupGameCtrl (its event passes), 0x00167380 ccGame::ChangeScene, 0x001671e0 ccGame::ChangeRequest, 0x0015a200 ccSleepNoSleepThread, 0x00160400 ccScFade::EntryFade, 0x00160490 ContinueFade, 0x0015fb80 SendPacket, 0x00315120 eventAreaInfo, 0x001a29a0 WORLD_MAN::GetWordParamFromEvCode, 0x00311790-0x00314c10 word_a1-word_c4, 0x001b1c00 fade, 0x001b1c60 fade_more, 0x001b04b8 area, 0x001b01d8 scene, 0x001af440 member_add_msg, 0x001af980 gate_add_msg, 0x001aff34 desktop_item, 0x0019d3b0 WORLD_MAN::GetEventAreaInfo, 0x0015f860 ccKanjiStrSeparate, 0x00377e5c serverStr, 0x00377e0c getItemMenuStr, 0x00378150 bookItemAddMsg, 0x00387840 spcNameList, 0x00168320 ccSetupDesktop (its passes, its return on a mode change, SetFrameRate); INF SLUS_202.67:0x001b0c1c ccEvent::Execute's piros_colour, 0x00160240 ccScFade::EntryFlash; INF gcmn.prg:0x0056b1c0 ccChar::Draw
worklog: 18, 24, 40
---

# The event interpreter

How the game runs its event scripts, as the port (`crates/piney-event`)
reproduces it: the scheduler task `ccThEvent`, the walk `eventSub`, the
replay `ccEventFlagSet`, `CheckOpen` and `Execute`, and what each
instruction does to the save and to the event manager. The script format
and the opcode table are on [the events page](events.md). Every statement
here is Infection's; the tests that hold it are listed under
[Checks](#checks).

## The port

```
crates/piney-event
  ir        the instruction set: Script { open, blocks: [Block { tags, conds, ops }] },
            Op (169 kinds), Cond (40), Tag (10), Cmp; Message tables separate
  text      the readable form of the IR; print and parse round-trip exactly
  official  the game's scripts: tables found through the code, decode and encode;
            events(): a disc's, parsed from its port data (PINEY/EVENTS.EVS)
  vm        EventMng (ccEvent), eventSub, ccEventFlagSet, ccThEvent (Vm::frame)
  host      the Host trait: every effect outside the interpreter, as named methods
  state     the save members scripts use, over piney_data::save::SaveData
```

Every `Host` method but `save` has a default that does nothing, so a host
ports only what its mode needs. A default that runs is a gap, not a
choice. It calls `host::unported(name)`, which prints the method's name
once to stderr, and `host::take_unported()` returns the names since the
last call. A playthrough test can then assert that its host left nothing
to the defaults.

The IR has no opcode numbers and no offsets: `official` maps them. A
script set of our own is written in the text form and loaded into the same
`Library` the official adapter fills (`tests/clean_room.rs`).

### The text form

```
event 1 label="WELCOME"
open
  if event_done event=0
  if game_status status=2
block 0
  set phase phase=0 comp=eq
  message msg=0
  add_operate num=1 except=1
  mail mail=10
block 1
  set phase phase=4 comp=ge
  if operate num=-1 except=1
  message msg=1
  repeatable
block 2
  if mail_got mail=10
  del_operate num=1 except=1
  end_event grp=-1
end
messages
  0 mode=0 name="Guide" "Welcome to the test desktop."
  1 mode=0 "Read your mail first."
end
```

`open` holds the open conditions; each `block` holds `set` lines
(precondition settings), then `if` lines (conditions), then instructions.
Fields are `name=value` with the operand struct's names; comparisons are
`eq`, `ge`, `le`. Strings are the game's bytes (ASCII, Shift-JIS as
characters, `\xNN` for anything else); `#` starts a comment outside quotes.
`parody` follows `messages` for Parody Mode's table.

### Finding the tables

On all four volumes, without symbols (`official::locate`):

- `Execute` and `CheckOpen` are the only loops shaped `lw a,0(p); addiu
  b,a,2; sw b,0(p); lh c,0(a); beqz c; sltiu at,c,N`. N is 41 for
  CheckOpen and 168 (INF), 169 (MUT) or 170 (OUT, QUA) for Execute; OUT's
  entry 169 is the loop head, QUA's is a case. The jump table is the
  `lui`/`addiu` pair after the bound.
- `eventTbl` is the table built right after the divide by 50 (`lui 0x51eb`)
  in the functions that call CheckOpen (`eventSub`, `ccEventFlagSet`).
- `evMsgTblp` and `evMsgTbl` are the first two addresses Execute's message
  case (6) builds, the Parody Mode table first.

This finds INF 0x00317e30 / 0x00317e60 / 0x00317e90, MUT 0x0032ee80,
OUT 0x00325260, QUA 0x002192b0 (the `eventTbl`s), each matching the
symbol or the name `piney-gen syms` carried. A message array runs to the next array
or table of either message table, less trailing all-zero records.

## Levels

`Execute(eot, n, b, lv)` runs block `b` of event `n`. Level 0 only steps
over it. Level 1 applies the bookkeeping (flags, lists, items), level 2
plays it. Every case advances the script pointer before looking at `lv`.
The block's bit (`1 << b`, 64-bit) is set when it ends at level 1 or 2
unless `repeatable` ran; `eventMng.status` is 1 while it runs.

`CheckOpen(eot, n, ev, lv)` passes when every condition passes. At level 0
it tests nothing and passes. For a block (`ev` >= 0) at level 1 or 2 the
precondition settings (`CheckCurrentOpen`) are tested first as one more
condition. After the first failure the rest are stepped over unevaluated.

## Walking an event

```
eventSub(n)                                             0x001b5ef0
  if eventFlag[n] has bit 62 or 63: return
  script = eventTbl[n / 50][n % 50]; none: return
  if not CheckOpen(open, n, -1, 2): return
  currentOpen = unset
  for b in 0..62 while the next short is not -1:
    SetCurrentOpen for each -2 setting
    if eventFlag[n] bit b:           CheckOpen lv 0, Execute lv 0
    elif CheckOpen(conds, n, b, 2):  Execute lv 2
    else:                            Execute lv 0

ccEventFlagSet(n)                                       0x001b6160
  if eventFlag[n] has bit 62 or 63: return
  script; none: return
  CheckOpen(open, n, -1, 0)
  for each block: settings, CheckOpen lv 0, Execute lv 1
```

The done and closed bits are tested once, before the walk: an `end_event`
in one block does not stop the later blocks of the same pass. The
replay runs every block whatever its conditions and bits.

## Boot, modes and the task

```
ccThMother: ccSaveData::Init(1); ccEvent::Init; ccStartEvent(1, 0); ChangeRequest(2, 7)

ccStartEvent(vol, flag)                                 0x001b5380
  clearFlag += vol - 1
  vol >= 2: ccEventFlagSet 0-49;    flag 0: event 62 done,  else ccEventFlagSet 50-99
  vol >= 3: crisis = 1; 100-149;    flag 0: event 163 done, else 150-199
  vol >= 4: 200-249;                flag 0: event 263 done, else 250-299
  event 100 * (vol - 1) done        (event 0 on volume 1)

ccStartThEvent                                          0x001b5230
  registNpcNum = 0; ccEvent::Init (enablePhase -1); ccClearGtHack
  every event 0-449 with bit 63 also gets bit 62

ccEnableThEvent(p): enablePhase = p; for 0 and 2, wait (a frame at a time)
                    until it reads 1 or 3, or is negative
ccDisableThEvent:   enablePhase = -1   (ccGame::ChangeRequest calls it)
```

Each mode's setup (desktop, board, field) calls `ccStartThEvent`,
`ccEnableThEvent(0)` before loading its files, `(2)` after, and `(4)` when
play starts; the title (`ccSetupDemo`) enables nothing. `ccThEvent`
(0x001b5a60), one iteration per frame:

```
loop:
  operateSet = -1; operateTarget = 0; areaCodeSet[0..2] = -1
  Breath(1)                                           -- the frame boundary
  status 5, phase >= 4, compulsionGameOver: continue
  status 5, gameCntStop 0, phase >= 4: party slots 1 and 2: partyTime[pc] += frame rate
      (capped at 0x0cdfe5c4); every 7560, AddFriendship(pc, 1)
  phase < 0: continue
  actEvent = -1
  eventSub 0-49 (volumeNum 1 only), 50-99, 150-199, 250-299, 350-399
  unless Parody Mode: gate = 1;
      done(100) and not done(101), or done(200) and not done(203): gate = 0
      done(203) and not done(308): gate = 2
      gate: eventSub 400-499, skipping 407-410 when 2
  phase 0: phase = 1, wait while it is 1
  phase 2: phase = 3, wait while it is 3
  phase 4: (worldman +0xf0 == 3: ccMenu +0x104 = 1); phase = 5
```

`worldman +0xf0` is `WORLD_MAN.hackFlag`. `GO` sets it to 2 for every
area before it branches to the town, the field or the dungeon, and to 3
when the save's crisis byte (+0x6772) is set; a field or a dungeon then
takes its area's own (3 for a hacked one). So the first play pass turns
on the menu's noise (`interNoiz` 1, [field UI](field-ui.md#the-noise-ccnoiz-0xe8))
in a hacked area and anywhere in the crisis, towns included.

So a mode gets one pass at phase 0, one at 2, one at 4, then one per frame
at 5. A pass that waits (a message, a wait) keeps the task inside it; the
rest of the pass runs when the wait ends. `Vm::frame` is one iteration;
`Vm::enable` and `Vm::enable_settled` are the two halves of
`ccEnableThEvent`.

## Instructions that take frames

With `ccBreathThread(n)` as n frames. "Poll" is a check made in the same
frame and then once per frame.

| instruction | frames |
| --- | --- |
| `wait count` | count + 1 |
| `message` on the setup screen (phase below 4) | own window; 10; poll `Check`; 15; 2 |
| `message` on the desktop or board (mode 2, 3) | wait for `dtMenu` (type -1 or 7, request slot free); 5; poll; a plain line: `dtMenu +0x10 = 1`, 10; then 1 |
| `message` in the field | wait for `ccMenu` type -1 or 62; 5; poll; plain line or question: 10; then 1 |
| `info`, `info_now` | setup screen: 10; poll; 8 then 2 (with no record: 1). Desktop: wait for `dtMenu` type -1 or 7; field: for `ccMenu` type -1; then 5 (`info_now`: 10); poll; 10. The desktop closes the window only after a plain line |
| `member_add_msg`, `gate_add_msg`, `desktop_item` (DispInfo) | sound effect 74; as `info` (5 before the poll), the window always closed; on the desktop `dtMenu +0x06 = 7` when `dtMenu +0x00` is -1, and `dtMenu +0x10 = 1` after the close ([announcements](#announcements)) |
| `name_entry` | 1, then `Main` each frame until done; 2 |
| `menu num` | 2, then until `ccMenu` type is -1 |
| `item_add_menu` (playing) | 1, then until `ccMenu` type is -1 |
| `teach_camera1`, `2` | until 150 frames with camera input have passed |
| `teach_camera3` | until a button is pushed; 45 |
| `remove_trap` | 20 |
| `piros_colour` (with Piros loaded) | by `eventStatus[1]`: 0; 5 (1, 3, 5); a frame for each rate of the pulse, 104 (7, 11-13); 14 (9 with operand 1-4); see below |
| `staff_roll` | until the roll task ends; 30; frame rate 2; not Parody: desktop menu 8, 1, until it closes; 1 |
| `show_map`, `gate_hack_anim`, `player_skill` | until `ShowMap`, `ccCheckGtHackAnm`, the command target says so |
| `stream`, `overlay` | inside `ccEventStream`, `ccLoadOverlay` |

Answers are stored (`msgNum` = the message, `msgSelect` = `Check`'s value)
for a question (`emode` low byte 3) on the setup screen and in the field,
not on the desktop.

### `piros_colour` (137)

Event 22 (PIRO02) tints Piros with it after Mia's potion. Only while
playing, and only with Piros (`charTbl` row 8) in `ccSpcManager`'s
registry; it switches on `eventStatus[1]` (the save's +0x64f9):
- **0 and 10** clear the tint.
- **1, 3 and 5** flash `scFadeDef` (`EntryFlash(8, colour)`: 0x602080ff,
  0x608080ff, 0x6000ffff) and set the tint (fix 1, rate 65: 0x002080ff,
  0x008080ff, 0x0040c8ff). After 5 frames they set it again. 3 and 5
  first play sound 74 and show the information line.
- **2, 8, 4 and 6** set the tint at once.
- **7 and 11-13** flash white (0x60ffffff), then pulse. 7 first plays
  sound 74 and shows the line. The pulse is 13 ramps, ramp r over r + 2
  frames: the rate is `fptosi(65 k / (r + 1))`, k counting down on the
  even ramps and up on the odd ones. 11-13 end at rate 65 in their
  colour; 7 ends at 0.
- **9** goes by the operand:
  - 0: sound 74, the line and a white flash;
  - 1-4: a white flash, then a ramp of 65 k / 6 down and up (14 frames)
    in 0x002080ff, 0x008080ff, 0x00204070 or 0x000000ff;
  - 5: `EntryFlash(20, 0x80ffffff)` and the tint cleared.

The line is the first line (`ccKanjiStrSeparate(text, 0)`) of the event's
message whose number is the save's +0x6510. It comes from the Parody Mode
table when that mode is on, and is shown by `ccEvent::DispInfo`.

The tint is `affectColorFix`, `affectColorRate` and `affectColor`
(+0xa6, +0xaa, +0xac). With the fix set, `ccChar::Draw` blends the
character by `SetFogBlend(rate, colour)` every frame without letting it
decay.

In the port:
- The interpreter reads the line and calls `Host::piros_colour(code,
  line)`.
- `crates/piney-game/src/piros.rs`'s `Sequence` gives each frame's actions
  to the host's `busy`.
- The town host flashes its `scFadeDef` (on the font layer) and tints the
  town's Piros (`piney_world::char::AffectColour`, drawn through
  `Body::draw_char_fog`).
- The field and dungeon host (`area_host.rs`) does the same: the flashes on
  the area's `scFadeDef`, the tint on Piros's character in the fights
  (`FieldWorld::set_affect_colour`), whose `ccChar::Draw` takes the blend
  each frame it is drawn (`AffectColour::blend`, a flash counting down) and
  draws through `Body::draw_char_fog`. Event 22's blocks 16 and 19 hold
  the tint in area 31's field and dungeon while Piros is there;
  `piney-game`'s `event_22_tints_piros_in_area_31` plays both (fix 1, rate
  65, 0x002080ff, nothing left to a host default) and
  `event_22_dungeon_shot` (ignored) takes a picture of him in the
  dungeon.

## The event manager

`ccEvent` (0x7e0 bytes) as `vm::EventMng` keeps it; the camera and the task
pointers are the host's.

```
+0x000 actEvent          +0x00c enablePhase     +0x010 msgSelect      +0x014 msgNum
+0x018 entryNpcNum       +0x01c registNpcNum    +0x040 target[16]     (type, code)
+0x080 entry[16]         +0x100 entryMc[16]     (type, code, marker, param)
+0x180 areaCode[16]      (code, except)
+0x1c0 evPos[16]         floor, block, int num, float dirc, float pos[4] at +0x10
+0x3c0 evPoint[16]       floor, block, int num
+0x750 currentOpen       phase, comp, status, scene[6], flag, index, num, comp, rangeFlag
+0x770 operate           u64, bit per intercepted player operation
+0x778 operateSet        the first operation tried this frame, -1 none
+0x77a areaCodeSet[3]    words entered at the Chaos Gate
+0x780 operateTarget     the character talked to
```

`CheckOperate(num, flag)` (0x001b32f0), which the desktop and the field
menus call: `num` < 0 goes ahead; with `flag` 0 the first `num` of the
frame goes into `operateSet`; bit `num` of `operate` (a 32-bit shift,
sign-extended) intercepts it; for 9 (talk) the command target becomes
`operateTarget`, and a registered target intercepts too. `AddOperate` and
`DelOperate` act on bits 0-17, or 19-28 with `sf`; `num` -1 means all of
them, `except` 1 all but `num`.

## The save

The members, as `state::ScriptSave` reads and writes them over
`SaveData` (offsets in [the save page](../formats/save.md)): `eventFlag`,
`eventStatus`, `mailList` and `mailOrderList` (through `NewMail`,
`ReadNewMail`), `webnewsList`, `bbsList`, the `dt*List` bit lists,
`partyMember*`, `townMoveFlag`, `gateList`, `gateListMark`, `gateOrderList`,
`wordList`, `protectArea`, `areaBan`, `itemList`, `impItemList`,
`skillList`, `talkNum`, `spcParam[].base.gold` and `.friendship`,
`partyTime`, `plcol`, `crisis`, `lastTown`, `clearFlag`. Indexes keep the
game's flat arithmetic (`bbsList[t][p]` is byte `48 t + p`).

- `AddItem(pc, cat, id, num)`: Kite's category 15 adds to `impItemList[id]`
  (at most 99). Otherwise the list is emptied into a copy, the item added
  to its slot (or the first empty one; at most 99), and the copy put back
  category by category in `addItemCategoryTbl` order (10, 13, 11, 12, 14,
  0-9), lowest id first; an item whose id is at or past its category's
  limit (24, 3, 72, 34, 22, 82, 77, 97, 75, 74, 76, 69, 68, 67, 68), or of
  another category, is dropped.
- `DelItem`: the first match loses `num`, or its slot; category 15 floors at 0.
- `ccAddSkill`: skills 2-5 are one (2); a new skill takes the first free
  slot and the list is re-sorted, ids 304 and up dropped.
- `AddFriendship(pc, n)`: capped at 1000, 250, 500, 750, 1000 by
  `volumeNum` (250 on Infection), floored at 0.
- `SetGateList(server, area)`: the area's bit, then it is taken out of the
  server's order list (left as -1), the list shifts down one and the area
  goes first.

## The desktop

The events that run on the desktop and the board are the ones set to
`game_status` 2 or 3: 164 of Infection's 192 scripts have such blocks.
What they use there:

- instructions: `mail`, `mail_vol`, `mail_member`, `bbs_post`, `bbs_post7`,
  `news_add`, `add_operate`, `del_operate` (and `_sf`), `message`, `info`,
  `info_now`, `name_entry`, `staff_roll`, `stream`, `overlay`,
  `frame_rate`, `sound`, `mode`, `desktop_item`, and the bookkeeping ones
  (`end_event`, `call_*`, `member_add`, `gate_*`, `status_set`,
  `friendship`, `town_move`, `talk_num`, `crisis`, `last_town`,
  `clear_count`, `virus_core` at level 1). `wallpaper_add`, `bgm_add`,
  `mail_remove` and `news_remove` exist but no Infection script uses them.
- conditions: `operate`, `mail_got`, `mail_5`, `mail_6`, `bbs_read`,
  `news_read`, `status`, `friendship`, `event_done`.

A new game reaches the desktop like this (checked frame by frame, below):
the boot leaves event 0 done; the desktop's setup makes the pass at phase
0, where event 1 block 0 plays in frame 2: frame rate 2, overlay GCMN,
stream 2, frame rate 1, stream 106, overlay DESKTOP, messages 1 and 2
(27 frames each on the setup screen), name entry, then `add_operate 1
except 1` (every desktop operation but the mailer held back), `del_operate
11`, mails 4, 5 and 320 (state 1, in that order), board thread 62 posts 0-6,
news 0-3 and 37, and the call bits. The pass settles in frame 59; phases 2
and 4 add nothing. On the desktop, trying any other operation plays block
1 (message 0 on the desktop, repeatable); once mails 4 and 5 are read
(state 4 or more) block 2 releases the operations, posts 53 board messages
and closes event 1, which the next mode change makes done.

### Announcements

`member_add_msg`, `gate_add_msg` and `desktop_item` compose their lines in
`Execute` and hand them to `ccEvent::DispInfo(l0, l1, l2)` (0x001b27b0),
which is `ccMessage::ChangeInfo(l0, l1, l2, null, 0, -1)` in the place's
window (the setup screen's own, the desktop's or board's `ccMsg` over the
fade menu, the field's). The sound comes first; the bookkeeping after the
window closes.

```
member_add_msg pc    (case 80)   l0 getItemMenuStr[0]           "You now have "
                                 l1 "#Y" name getItemMenuStr[6] "'s member address!"
                                    name: the string spcParam[pc] +0x00 points at
gate_add_msg area    (case 90)   l0 "#B" server " " wordA " " wordB " " wordC
                                 l1 getItemMenuStr[7]           "#W is added to the Word List."
                                    GetEventAreaInfo(area) (0x0019d3b0): the first
                                    eventAreaInfo record of that code, else null;
                                    server: ccKanjiStrSeparate(serverStr, record.server)
desktop_item type id (case 167)  l0 bookWallPaperAdd, bookBgmAdd or bookMovieAdd by type
                                    (nothing for another type), dec2sjis(id, 3, 0),
                                    bookItemAddMsg
```

`getItemMenuStr` (.sdata 0x00377e0c) and `serverStr` (0x00377e5c, the five
servers' letters) are split by `ccKanjiStrSeparate` (0x0015f860: past `n`
NULs, a byte below 0x20 or from 0x80 taking two). A member's name pointer
is `NewGame`'s: `spcNameList` (main .bss 0x00387840, 24 bytes a member),
where it copies up to 20 bytes of `charTbl`'s name, or the save itself for
character 0. The four `book*` pointers (.sdata 0x00378150-0x0037815c) point
into GCMN.PRG's data (0x006a4640-0x006a4678), past DESKTOP.PRG's end.
`desktop_item`'s bookkeeping sets bit `id - 1` (0 for id 0 or less) of
`dtWallpaperList`, `dtBgmList` or `dtStrList` with no bound: an id past a
list's bits sets a bit in the list after it. `gate_add_msg`'s
`SetGateList` and word bits are `gate_add`'s.

`crates/piney-game`'s `story::Announcements` reads the texts from the
executable, DEMO.PRG (`charTbl`, for `spcNameList`) and GCMN.PRG; the
desktop's host (`desktop.rs`, the `Bridge`) opens them in the desktop's,
the board's or the setup screen's window and answers `story_area` from
`eventAreaInfo`, so `gate_add` and `gate_mark` on the board add the
area's words on its own server.

### Mode changes from the scripts

`mode 3` (`ChangeRequest(3, 7)`) is the desktop from anywhere; the session
leaves the mode it is in (The World, a field or dungeon, or the desktop
itself) and sets the desktop up on the same event task. `ccSetupDesktop`
(0x00168320) tests `game +0x04` (a change asked) after its waits for the
passes at 0 and 2 and returns if it is set, before loading, before
`SetFrameRate(1)` (0x001684dc) and before its tasks. So event 4's block 9
(the desktop, phase 2: streams 4, 5 and 6, `end_event -1`, `end_event 3`,
`mode 3` at +0x02d8) abandons that setup and the next one starts; its
`ccStartThEvent` makes events 3 and 4 done and event 10 opens in its pass
at phase 0. The frame rate is `ccSystem`'s and no setup sets it before
that `SetFrameRate(1)`: a setup entered from The World runs at 2 (30
frames a second), its passes' streams and windows with it, until the setup
that reaches the desktop.

## Starting later in the story

`piney-game --mode story:N` (`start.rs`) starts where event N of the
story opens (3, 4, 10-31: to the ending). It takes a new game's save
and the boot's task, then brings each event of the story before N forward
with the task's own `ccEventFlagSet` - every block at level 1, as
`ccStartEvent` brings a later volume's earlier events forward - in the
story's order (1, 2, 3, 4, 10, 11, 12, 13), with Log in's `ccSetupNewGame`
after event 1. The replay's host answers `story_area`, so the gate lists
and words are the game's. The mode's set-up makes its own `ccStartThEvent`
(the closed events done) and passes:

```
N   brought forward   where               first block
3   1 2               area 14's field     0, at phase 0
4   1 2 3             its dungeon         0, at phase 4
10  1 2 3 4           the desktop         0, at phase 0
11  ... 10            Mac Anu (Log in)    0, at phase 0
12  ... 11            the desktop         0, at phase 0 (no tag: the first set-up)
13  ... 12            Mac Anu (Log in)    0, at phase 0
14  ... 13            the top page        0, in its setup (no tag)
```

The field and dungeon starts carry the party event 2 made
(`ccPartyManager` is not in the save): Orca registered with `bootParam` 5
by his `entry`, Kite's 6 from `pc_mode -3 6`, the town's `SetParty`, Orca
added (`AddMember`), and for 4 event 3's `pc_mode -3 4`. The replay leaves mails read and board
posts at state 3, as `ccEventFlagSet` does, and no fight's experience.

## The World

In the field (`game.status` 5) the task runs beside the field's tasks, the
first of them each frame (priority 32). `crates/piney-game`'s
`field_host.rs` is the host, `world.rs`'s `WorldMode` runs it.

**The set-up.** `ccSetupGameCtrl` (0x00168960) calls `ccStartThEvent` and
`ccEnableThEvent(0)` after its fade out and two held black frames, then
`ccEnableThEvent(2)` once that pass is made, then (the load, the tasks
started, `WORLD_MAN::GO`, the start positions) `ccEnableThEvent(4)` just
before the fade in's first `Breath` (F0). The runtime holds the world on
its last black frame (`World::set_loading`) while the passes at 0 and 2
run, one event frame per game frame, and asks for phase 4 at the start of
F0; the first play pass is F1, before any other task. A new game (the
runtime's test):

```
frames 1-10  fade out; 11-12 black; the task idles (phase -1)
end of 12    ccStartThEvent, ccEnableThEvent(0)
13           the pass at 0: event 2 blocks 0 and 1 (entry, add_target,
             gate_add, gate_mark, pc_mode -3 6); ccEnableThEvent(2)
14, 15       the task's Breath; the pass at 2; the entries go to the world
             (ccEntryEventMng), the load ends
16 (F0)      ccEnableThEvent(4); the task's Breath
17 (F1)      the play pass: event 2 block 2 begins
```

The load (`ccLoadResourceFL`, a frame at a time in the game) takes no
frames here.

**What the instructions reach.** The windows are the field UI's `ccMsg`
(`FieldUi::message_open`, `message_check` with the frame's pad,
`message_close`, `announce` with `charTbl[pc]`'s name for
`member_add_msg`). While the set-up's passes run (phases 0 and 2, before
the menu task exists) they are the event's own window on the set-up's
black screen instead, as on the desktop's set-up (`FieldUi::set_setup`,
`piney_desktop::setup::SetupScreen`), and the runtime shows that screen
while the load holds. In Infection the one such window in a field is
event 11's church: block 24's "The book. / Open the book." before the
streams 10-12; `field_menu` is `ccMenuCtrl::CheckMenuType`; `menu`,
`menu_ban`, `target_forbid`, `map_on` and `noise` are `ccMenu`'s. The
voice of each line is asked by the window as it opens. The camera and
character instructions, `entry`, `remove`, `party_add` and
`party_remove` go to `piney-world` ([field game](field-game.md)).
`fade` and `fade_more` drive `ccMenu`'s fader (`ccMenu +0xb8`,
`EntryFade(fade, count, 0, alpha << 24, 0, 0, 512, 448)` keeping the
element's number in `eventMng.fadeNum` +0x788, `ContinueFade(fade,
fadeNum, count, alpha << 24)`), which `ccMenuCtrl::Disp` sends on the
menu layer every frame:

```
ccScFade      +0x00 off (nothing sent while set)   +0x94 layer
  element[4]  +0x04 + 36 i: status, cnt, tcnt, +0x06 back, +0x08 hold (s16),
              rectangle +0x0c-+0x18, col0 +0x1c, col1 +0x20
EntryFade     the first element with status 0: status 1, cnt 0, tcnt n, the colours
ContinueFade  status 1, cnt 0, tcnt n, col0 = col1, col1 = the new colour
SendPacket    each element with status bit 0 drawn at c0 + (c1 - c0) cnt / tcnt,
              then cnt + 1; past tcnt: bit 1 ends it; bit 2 becomes a fade to
              alpha 0 over `back` (status 2); bit 3 a hold over `hold` (status
              4); otherwise cnt stays at tcnt
```

`gate_add` and `gate_mark` read the story area (`Host::story_area`):
`eventAreaInfo` (0x00315120, 126 records of 0x54 bytes: code +0x00, the
three words' texts +0x04-+0x0c, server +0x10, the protect items +0x34) and
`WORLD_MAN::GetWordParamFromEvCode(code, k)` (0x001a29a0): the first
record of the code, then word `k`'s text among `word_a1`-`word_a4` (k 0),
`word_b*` or `word_c*` (groups 1 to 4, `strcmp`), whose `+4` is the id.
Area 14 (Bursting Passed Over Aqua Field) is server 0, words 0, 13, 26.

**Leaving.** `area` (0x001b04b8) with 1-13 quits the field
(`WORLD_MAN::Quit`) and sets `eventAreaNumber`; any other area has
`WORLD_MAN::SimGenerateCode` build it from its words. `scene` calls
`ccGame::ChangeScene(a, t, fd, d, f, b)` (0x00167380):

```
townPrev = town;     t >= -1: town = t, saveData.lastTown = t
serverPrev = server; town >= 0: server = {0,1,2,3,4,0,1,2}[town] (0x00306dc0)
fieldPrev, dungeonPrev, floorPrev, blockPrev = the old; each new one unless < -1
areaPrev = area;     a >= -1: area = a
inBattle = inBattleCnt = 0; inBattleDist = 2200
ChangeRequest(6, 7)
```

`ChangeRequest(num, sf)` (0x001671e0) queues the mode, `InitScene` unless
it is 6, interrupts the sound, calls `ccDisableThEvent` and, for `sf` 7,
freezes the layers and calls `ccSleepNoSleepThread(1, 1)` (0x0015a200),
which sleeps every task without the no-sleep flag and then the calling
one. The event task keeps its no-sleep flag (`ccDeleteAllThread` skips it
too), so its sleep count stays 0 and it wakes on the next frame, inside
the next mode's set-up: it runs the instruction after `scene` (event 2's
`end_event`, which closes event 2), and the set-up's `ccStartThEvent`
turns closed into done, so the next area's events see `event_done 2`
from their first pass. (Read from the thread code; the exact frame is not
run against the game.) The interpreter models the stop as
`Wait::ChangeRequest` after `Host::change_scene` (a host that does not
model the mode change answers no at once, as the checks' hooked
`ChangeScene` does); the field host says yes once `ChangeScene` is asked,
disables the phase and puts the world to sleep, and the next area's host
resumes the pass in its first frame ([Leaving the
town](field-walk.md)).

**Items for companions, the virus core, rooms, the party saved.** The
field and area hosts answer these the game's way:

- `item_add` (case 93, 0x001afd04). At level 1, or for Kite (pc 0), or for
  a category of 10 and up, the item goes into the save
  (`ccSaveData::AddItem`); M119's gift and every other volume-1 `item_add`
  to Kite take this path.
  - While playing, for a companion with a category below 10,
    `ccEvent::GetSpc(pc)` looks for the character built for that registry
    id. When it finds one, the item goes through `ccMenu->AddSpcItem(ch,
    cat, id, num, 0)` instead: it is worn if it is better, read if it is a
    book, and otherwise goes into the member's bag.
  - S108 gives Sanjuro the Kotetsu Sword this way, and S111 gives Natsume
    a blade, both in their dungeons.
  - The hosts' `add_spc_item` looks for the town party's member or the
    area's battle character (`FieldUi::add_spc_item`, with no
    `ccThEquipMenu`). When there is none, the host declines, and the
    interpreter puts the item in the save.
- `virus_core` (case 124, 0x001b06c8) runs only at level 1
  (`ccEventFlagSet`), so it never runs while playing.
  - At level 1 it calls `SimGenerateCode` with the area's three word IDs,
    sets the area's `protectArea` bit, and deletes its protect items
    (category 15) from Kite.
  - The start's replay host leaves `WORLD_MAN` alone, because a start's
    own set-up makes it again. The field and area hosts, which play never
    asks, set it as Area Information shows it.
- `room` and `room_point` (cases 130 and 131, playing only) call
  `WORLD_MAN::RoomSelect(floor, block)` ([the dungeon](dungeon.md)).
  - `room_point` uses the room of the first event point with that number.
  - The area host passes them to `FieldWorld::room_select`, which is a
    change of scene to that room, and sets `ccMenu`'s map status to 3.
- `save_party` (case 88, 0x001af784, playing only) sets `partyMemberSave`
  to `1 << memberID[s]` for each filled party slot. It needs only
  `Host::party`, which both hosts answer from the registry.
  - M130's block 20 runs it at phase 0 in the field, after the Skeith entry
    (`entry 7 0 0 0`, which goes to the entry control) and
    `battle_ready`.

**Event 2** plays so from a new game's arrival (the runtime's test, the
pad pressing CROSS every 24 frames while a window waits and walking the
tutorial menus): the camera work from F1, `pc_act 0 3` at frame 100, the
camera on Orca and his first line at 221, 12 lines, `member_add_msg`
(sound 74, the member-address window) at 685, 4 more lines, `menu 75` at
949 (PERSONAL, Party, Add: menus 75-77, messages 12-19; Party > Add's
`ccThPartyAdd` puts Orca in party slot 1), `menu_ban` and nine lines from
1268, the fade at 1741, `fade_more` at 1752, `menu 78` at 1783 (the gate's
lessons, menus 78 and 79, messages 30-56, ending with the party's leave
211 frames after the last OK), and `area 14` and `scene 1 0 14` at 2295.

**From a new game's save** (`piney-game --mode world`) the task is built
as a new game brings it to Log in: the boot's `ccStartEvent(1, 0)`; the
desktop's passes and play (event 1's opening) until mails 4 and 5 are read
and event 1 closes itself; the board's `ccStartThEvent` (event 1 done) and
passes; Log in's `ChangeRequest`. Every window and wait answers at once
there.

## Checks

`tools/test_event_vm.py` runs the game's own functions in `tools/eemu.py`
(adding `dsllv`, `dsrlv`, `dsrav`) and writes
`crates/piney-event/tests/vm_fixture.txt`, numbers only; `tests/vm.rs`
compares the port with it, reading the scripts from the disc:

| what | how much | compared |
| --- | --- | --- |
| `ccEventFlagSet(n)` | every script, boot save and 3 random saves (768) | every save byte; the event manager's registers |
| `ccStartEvent(vol, flag)` | volumes 1-4, flags 0 and 1 | every save byte |
| `ccStartThEvent` | 3 random saves | every save byte |
| `CheckOpen`, walked with `SetCurrentOpen` | every script, 8 random saves and worlds (1536) | header and every block |
| each condition alone | 2059 conditions, 8 worlds | the result |
| `eventSub(n)` at level 2 | every script, 3 worlds (576) | frames, every save byte |
| every block played at level 2 with `Execute` | 2616 blocks, 169,146 frames | frames per block, the save, the registers |
| a new game to the desktop and into play | 152 frames | the phase each frame, every hooked call and its frame, the save three times |

`tools/test_field_host.py` runs `ccScFade`'s `EntryFade`, `ContinueFade`
and `SendPacket` (its packet work refused, so only the counting runs: 592
calls from 41 random faders, every status bit and both chained counts,
and the events' own fade) and `ccGame::ChangeScene` (122 random
`ccGame` states, towns within the server table) in eemu and writes
`crates/piney-game/tests/field_host_fixture.txt`; `field_host.rs`'s tests
replay it: the whole fader after each call, every word of `ccGame` and
`lastTown`, 0 mismatches. `story.rs`'s test takes all 126 `eventAreaInfo`
records and their words from the generated area tables
(`AreaTables::of`) and compares them with the
ones `vm_fixture.txt` got from the game's `GetEventAreaInfo` and
`GetWordParamFromEvCode` (server, word ids, protect items): 0 mismatches.
`world.rs`'s `event_2_plays_to_the_scene_change` plays event 2 headless
from `--mode world`'s state to its scene change and compares every call
the host takes with its frame (69 calls), the voices, the save's gate,
word and member bits, and the party.

`tools/test_desktop_announce_rs.py` runs `Execute` in eemu on
one-instruction scripts with `ccGame.status` 2 and 3 and phase 5 -
`gate_add_msg` for all 126 `eventAreaInfo` codes, `member_add_msg` for
members 1-17 on the game's own new game (NewGame, which fills
`spcNameList`), `desktop_item` for types 0-3 and ids 1, 7, 50 and 123 with
GCMN.PRG in - with `DispInfo`, `GetEventAreaInfo`,
`GetWordParamFromEvCode`, `SetGateList` and the string code native, and
writes `crates/piney-game/tests/announce_fixture.txt` (numbers only: each
line's length and FNV-1a hash, the frames of the window, its polls, its
close and the end, `dtMenu +0x06` and `+0x10`, every save byte changed);
`desktop.rs`'s `announcements_match_the_game` replays the 159 cases
through the task, the `Bridge` and `Announcements`: 0 mismatches.
`session.rs`'s `story_starts_open_their_event` starts each story point and
sees block 0 of its event play, and none of the story before it;
`story_3_is_where_event_2_leaves_the_game` sets story:3 beside event 2
played from a new game's Log in (the town's tutorials walked): at the
field's arrival the scene, `WORLD_MAN`'s area and both party managers
agree, and 60 frames on the flags of events 0-4 and the gate, word and
member bits do;
`event_4_ends_on_the_desktop` takes event 4 from block 8's `mode 3` in the
dungeon through block 9's streams at rate 2, the abandoned setup and the
second one (event 10's block 0, still at rate 2) to the desktop at rate
1; `toppage.rs`'s `the_boards_keyword_announcements` reads posts 3/0, 9/0
and 10/1 at event 14's start and sees events 14, 50 and 55 add areas 17,
28 and 29 with their windows.

Calls outside the event and save code (windows, streams, overlays, menus,
gcmn) are hooked to finish at once, and the Rust test host answers the
same. Round trips (`tests/official.rs`): every script of all four volumes
decodes and encodes to identical shorts and prints and parses to the same
IR; every Infection event with its messages prints and parses back.

## Unknown

- The load between the passes at 2 and 4 (`ccLoadResourceFL`) takes
  frames in the game that depend on the disc; the runtime takes none, and
  one frame for `ccSetupGameCtrl`'s first `Breath` after starting the
  tasks. The set-up's side of the passes (when it calls
  `ccEnableThEvent`) is taken to be as the desktop's check has it, not
  checked in the field.
- `DispInfo`'s setup-screen branch (phase below 4: its own layer and
  `ccMessage`, 10 frames of `Disp` before the first `Check`) is not run
  against the game; no Infection script announces on the desktop or the
  board before play.
- What a desktop or board announcement shows for a desktop item: the
  `book*` strings are GCMN.PRG's, past DESKTOP.PRG's end, and whether
  GCMN's bytes are still there (or zeroed by DESKTOP.PRG's load) is not
  known; the port shows GCMN's. Infection's desktop scripts never reach
  `desktop_item`.
- How long `ccThMother` takes between a mode's `ChangeRequest(3, 7)` and
  the desktop's setup (DESKTOP.PRG's load); the session changes at once.
- `piros_colour` 9 with an operand past 5 pulses in the colour the stack
  last held (no script does it); the port does nothing.
- The volume 2-4 story events have no message arrays on Infection: a
  message there reads a record at address 12 × msg. The port treats it as
  missing; those events are not played in the checks.
- The later volumes' changed cases (the events page lists them): the port
  runs Infection's semantics for every dialect.
