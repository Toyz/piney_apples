---
title: Event scripts
status: partial
volumes: all
covers: MUT SLUS_205.62:0x001bdf00 ccEvent::Execute, OUT SLUS_205.63:0x001b4250, QUA SLUS_205.64:0x001bba60; INF SLUS_202.67:0x001a8d20 ccEvent::Execute, 0x001a7400 ccEvent::CheckOpen, 0x001a6ec0 ccEvent::SetCurrentOpen, 0x001b5ef0 eventSub, 0x001b6160 ccEventFlagSet, 0x00317e30 eventTbl, evMsgTbl, evMsgTblp
worklog: 18, 24, 40
---

# Event scripts

The story's scripts: `short` arrays in main (`evtbl.cpp`), one per event,
interpreted by `ccEvent::Execute`. `tools/evscript.py` lists, disassembles,
surveys and checks them.

## Format

All values are little-endian `short`s. No header, no jumps; every opcode is
fixed-length.

```
open conditions ... 0          CheckOpen(eot, N, -1, lv): the event is live when all hold
block, up to 62:
  (-2 tag operands)*           SetCurrentOpen: preconditions for the rest of the event
  conditions ... 0             CheckOpen(eot, N, block, lv)
  instructions ... 0           Execute(eot, N, block, lv)
-1                             end
```

A condition list passes when every condition passes; every operand is always
consumed. When `ev >= 0` and `lv > 0` the current preconditions
(`CheckCurrentOpen`) are one more condition.

`lv`: 0 walk without effect; 1 bookkeeping only (`ccEventFlagSet`, used to
fast-forward earlier volumes); 2 play.

## Events and flags

```
script(N)   = eventTbl[N / 50][N % 50].tbl         eventTbl 0x00317e30, ccEvTbl {short *tbl; char *str}
messages(N) = evMsgTbl[N / 50][N % 50][msg]       {emode, name, str}; evMsgTblp in Parody Mode
flags(N)    = saveData.eventFlag[N]               u64 at saveData+0x54f8+8N
```

| group | events | scripts |
| --- | --- | ---: |
| M1 | 1-31 | 26 |
| S1 | 50-62 | 11 |
| M2 | 101-116 | 16 |
| S2 | 150-168 | 18 |
| M3 | 201-219 | 19 |
| S3 | 250-271 | 21 |
| M4 | 301-315 | 15 |
| S4 | 350-360 | 11 |
| ML | 401-455 | 55 |

The ML events are the members' mail events: each opens once a story event
is done, on the desktop (`game_status` 2), and its blocks send the next
mail as the member's friendship grows, the reply (`mail_5`, `mail_6`)
choosing the branch. Infection's disc carries all 55, and 12 open on
Infection's events alone: Black Rose's 401 (after 14) and 405 (23),
Mistral's 407 (18) and 410 (31), Piros's 417 (15) and 421 (31), and the
side members' 422, 426 (Sanjuro, after 58), 427, 431 (Gardenia, 59), 432
and 436 (Natsume, 61). The rest wait on later volumes' events. The walk
reaches them after S1 to S4 each pass (event-vm.md: the gate), so an
event still playing earlier in the pass holds them back.
`black_rose_writes_when_friendly` (piney-game) runs 401 on the desktop:
mail 85 at friendship 50, none at 49.

Flag bits: bit `b` < 62 - block `b` has run (set when a block ends at `lv` >
0, unless opcode 4 ran in it); bit 62 - done; bit 63 - closed.
`ccStartThEvent` turns 63 into 62 for events 0-449 when the event task
restarts. An event with 62 or 63 set is not walked.

Phases (`eventMng.enablePhase`, set through `ccEnableThEvent` by each mode's
setup): 0 before the mode's files load, 2 after, 4 during play. The event task
makes one pass at 0, one at 2, then one per frame from 4; after the first
play pass the phase reads 5. `phase` preconditions pick the pass. The walk,
the task and every instruction's effect are on
[the event interpreter page](event-vm.md).

## Operations

`add_operate`, `del_operate` and the `operate` condition name the
player's operations by number. The code that asks `ccEvent::CheckOperate`
(`this` eventMng, then the number and a flag) fixes the numbers:

| num | operation | asked by |
| ---: | --- | --- |
| 0-5 | the desktop's icons: The World, Mail, News, Wallpaper, Audio, Data (`dsel`) | `Desktop_control::SelectMode` |
| 6-8 | the top page's commands: Log in, BBS, Log out | `ccThToppageCtrl::_Normal` |
| 9 | the action button on the command target (talk; the target becomes `operateTarget`) | `ccThGameCtrl` |
| 10 | the chat menu (square by default) | `ccThGameCtrl` |
| 11 | START on the desktop; PERSONAL in The World (triangle) | `ccThDtMenu`; `ccThGameCtrl` |
| 12 | the options menu (START in The World) | `ccThGameCtrl` |
| 13 | the map (`WORLD_MAN::ChangeMapMode`) | `ccThGameCtrl` |
| 14 | leaving the area: System's Gate Out, the Sprite Ocarina (the chat menu's order and the item), greyed or refused while it is intercepted (flag 1: not recorded in `operateSet`) | `ChangeMenu`, `SystemMenu`, `ChatMenu`, `ItemMenu`, their `Disp`; `battle_ready` adds it |
| 15 | a gate hack | `ccHackMenu::Select` |
| 16 | the Chaos Gate's Warp | `GateMenu`, `GtTownMenu` |
| 17 | using a skill from the SKILL menu | `SkillMenu` |
| 19-24, 25-27, 28 | the second lock of 0-5, 6-8 and the desktop's START (11): `add_operate_sf`, `del_operate_sf` | as the first |

The buttons are the save's `assignPAD` rows (the defaults shown). The
same number means different things on the desktop and in The World (11).
18 is never asked.

`emode & 0xff == 3` makes a message a question; the choice goes to
`eventMng.msgNum`/`msgSelect`, tested by the `answer` condition.

## Opcodes, conditions and tags

`bytes` includes the opcode. Operands are `short`s, in the order of the named
operand struct. `[flag]` runs at `lv` >= 1, `[split]` differs between 1 and 2,
`[replay]` runs only at 1, anything else only at 2. Printed by
`tools/evscript.py ops`:

```
# Execute: jump table 0x00355fd0, 168 entries
   0  0x001a8d80   -
   1  0x001a8dc0   4  end_event              ccEvFlagGrp          grp                                      eventFlag[grp] bit 63 (closed); grp < 0 is this event, and -2 also sets bit 62 (done) at once [flag]
   2  0x001a8e38   4  set_block              ccEvFlagLocal        num                                      set bit num of this event's flag word; num < 0 sets bits 0..ev-1; num >= 30 halts (store to 0) [flag]
   3  0x001a8ef0   4  clear_block            ccEvFlagLocal        num                                      clear bit num of this event's flag word [flag]
   4  0x001a8f34   2  repeatable             -                    -                                        this block does not set its own bit when it ends, so it can run again [flag]
   5  0x001a8f4c   4  wait                   ccEvWait             count                                    wait count + 1 frames (ccBreathThread) [play]
   6  0x001a8fa8   4  message                ccEvMessage          msg                                      speech window (ccMessage::Change): name and three lines, voice; emode 3 asks a question and stores msg/answer in eventMng.msgNum/msgSelect [play]
   7  0x001a9568   2  clear_answer           -                    -                                        eventMng.msgNum = msgSelect = 0 [play]
   8  0x001a9584   4  info                   ccEvMessage          msg                                      information window (ccMessage::ChangeInfo), then closed [play]
   9  0x001a9b0c   4  info_now               ccEvMessage          msg                                      as info, window shown without its opening fade (windowStatus, windowAlpha 0), then 10 frames rather than 5 before the wait [play]
  10  0x001aa0ac   4  stream                 ccEvStream           num                                      play cutscene stream num (ccEventStream(num, 1)); in the field the area file list is dropped and reloaded [play]
  11  0x001aa20c  10  entry                  ccEvEntry            type code marker param                   ccEvent::AddEntry: register a character to appear (16 slots; halts when full) [play]
  12  0x001aa250  10  entry_mc               ccEvEntryMc          type code marker param                   ccEvent::AddEntryMc (16 slots; halts when full) [play]
  13  0x001aa290   6  remove                 ccEvTarget           type code                                type 2: party character code out of the party and the SPC manager; 5/6: entry object code deleted; -1: all objects [play]
  14  0x001aa3f8   6  add_operate            ccEvOperate          num except                               intercept player operation num (eventMng.operate bit; num -1: all of 0-17; except 1: all of them but num) [play]
  15  0x001aa430   6  del_operate            ccEvOperate          num except                               stop intercepting operation num (DelOperate, same encoding) [play]
  16  0x001aa4d8   6  add_target             ccEvTarget           type code                                talking to the character (operation 9) goes to the event (eventMng.target[]) [play]
  17  0x001aa504   6  del_target             ccEvTarget           type code                                undo add_target [play]
  18  0x001aa534   2  menu_ban               -                    -                                        cutscene on (MenuBan): menus forbidden and hidden, party under manual AI, camera id saved [play]
  19  0x001aa558   2  menu_clear             -                    -                                        cutscene off (MenuClr): undoes menu_ban [play]
  20  0x001aa578   6  add_area_code          ccEvAreaCode         code except                              ccEvent::AddAreaCode [play]
  21  0x001aa5a4   6  del_area_code          ccEvAreaCode         code except                              ccEvent::DelAreaCode [play]
  22  0x001aab54   8  cam_look_pos           ccEvCamSetVpPos      x y z                                    camera look-at = (x, y, z) * 10, at once [play]
  23  0x001aabf8   8  cam_look_char          ccEvCamSetVpChar     type code height                         look-at = the character's position + height * 10, at once [play]
  24  0x001aad2c   8  cam_follow_char        ccEvCamSetVpChar     type code height                         look-at tracks the character (vpCtrl 1) [play]
  25  0x001aae9c   4  cam_look_marker        ccEvCamSetVpMarker   marker                                   look-at = the marker's position [play]
  26  0x001aafc8   8  cam_look_pos_half      ccEvCamSetVpPos      x y z                                    look-at moves halfway towards (x, y, z) * 10 [play]
  27  0x001ab0cc   8  cam_look_char_half     ccEvCamSetVpChar     type code height                         look-at moves halfway towards the character [play]
  28  0x001ab264   4  cam_look_marker_half   ccEvCamSetVpMarker   marker                                   look-at moves halfway towards the marker [play]
  29  0x001ab3ec  10  cam_pan_pos            ccEvCamChgVpPos      x y z rate                               look-at moves to (x, y, z) * 10 at rate [play]
  30  0x001ab4a0  10  cam_pan_char           ccEvCamChgVpChar     type code height rate                    look-at moves to the character at rate [play]
  31  0x001ab5e4  10  cam_pan_follow         ccEvCamChgVpChar     type code height rate                    look-at moves to and tracks the character at rate [play]
  32  0x001ab680   6  cam_pan_marker         ccEvCamChgVpMarker   marker rate                              look-at moves to the marker at rate [play]
  33  0x001ab7b8   8  cam_orbit              ccEvCamSetCp         rotx roty dist                           camera angles and distance * 10, at once (cpCtrl 2) [play]
  34  0x001ab840  10  cam_orbit_move         ccEvCamChgCp         rotx roty dist rate                      camera angles and distance, moving at rate (cpCtrl 2) [play]
  35  0x001ab8d8  10  cam_orbit_turn         ccEvCamChgCp         rotx roty dist rate                      as cam_orbit_move with the rate in degrees (cpCtrl 3) [play]
  36  0x001ab96c   2  cam_mode4              -                    -                                        cpCtrl = 4: the camera point stays where it is, the look-at goes on [play]
  37  0x001ab9b4   2  teach_camera1          -                    -                                        tutorial: this event's message 7 (55 with the other camCtrlType), event 3's voice; waits for 150 frames of camera input [play]
  38  0x001abbe8   2  teach_camera2          -                    -                                        tutorial: messages 9 / 56, as teach_camera1 [play]
  39  0x001abe18   2  teach_camera3          -                    -                                        tutorial: messages 11 / 57; waits for a button, then 45 frames [play]
  40  0x001ac1c8  14  camz_set               ccEvCamzSet          vx vy vz cx cy cz                        two-point camera: look-at and position * 10, at once [play]
  41  0x001ac348  18  camz_move              ccEvCamzChg          vx vy vz cx cy cz vprate cprate          two-point camera moving at the rates [play]
  42  0x001ac544   6  camz_speed             ccEvCamzType         vstype cstype                            speed curve types of the two points [play]
  43  0x001ac578  16  camz_point             ccEvCamzInpSet       num vx vy vz cx cy cz                    interpolation point num (num clamped to 16 in the script itself) [play]
  44  0x001ac6b0  10  camz_path              ccEvCamzInpChg       num vprate cprate alpha                  run the camera through num points (num clamped to 16 in place) [play]
  45  0x001ac7c8  18  radiator               ccEvRadiator         rtype type code x y z roty rotz          gimmick entry (ccEntryParam type 1, id 19, param[0] rtype) at a character's position plus the offset, rotated in degrees [play]
  46  0x001ac9bc  12  boss_smoke             ccEvBossSmoke        floor block x y z                        gimmick entry type 1, id 19 in a dungeon room [play]
  47  0x001acad4   2  delete_gimmick19       -                    -                                        delete every id-19 gimmick (deleteGimmick) [play]
  48  0x001ac028  14  camera                 ccEvCamera           type code height rotx roty dist          look at the character with angles and distance, at once [play]
  49  0x001acb3c   2  camera_end             -                    -                                        back to the normal camera (changeCamera(1)); stop the event camera task [play]
  50  0x001acb7c   2  camera_end_reset       -                    -                                        changeCamera(eventMng.normalCamID), cameraSoftReset, stop the camera task [play]
  51  0x001acbc4   6  npc_act                ccEvPcAct            npc param                                town NPC action (codes 29 and 158 are merchants) [play]
  52  0x001acc50  10  npc_walk_pos           ccEvPcMovePos        npc x y z                                NPC walks to (x, y, z) * 10 [play]
  53  0x001acd18   8  npc_walk_dir           ccEvPcMoveRD         npc rot dist                             NPC walks dist * 10 towards rot degrees [play]
  54  0x001ace00   6  npc_walk_marker        ccEvPcMoveMarker     npc marker                               NPC walks to the marker [play]
  55  0x001acf50  12  npc_walk_char          ccEvPcMoveChar       npc type code rot dist                   NPC walks to a point rot/dist from a character [play]
  56  0x001ae3cc   6  npc_put_marker         ccEvPcPos            npc marker                               NPC placed at the marker [play]
  57  0x001ae524  10  npc_put                ccEvPcPosDirect      npc x y z                                NPC placed at (x, y, z) * 10 [play]
  58  0x001ae5cc   8  npc_turn               ccEvPcRot            npc dirc chg                             NPC turns to dirc degrees [play]
  59  0x001ae678  10  npc_face               ccEvPcRotChar        npc type code chg                        NPC turns to face a character [play]
  60  0x001ad0c0   6  pc_act                 ccEvPcAct            pc act                                   party character under manual AI with remote command act (9-way switch) [play]
  61  0x001ad8ac  10  pc_walk_pos            ccEvPcMovePos        pc x y z                                 party character walks to (x, y, z) * 10 (remote command 1) [play]
  62  0x001adab0   8  pc_walk_dir            ccEvPcMoveRD         pc rot dist                              party character walks towards rot degrees [play]
  63  0x001adbf4   6  pc_walk_marker         ccEvPcMoveMarker     pc marker                                party character walks to the marker [play]
  64  0x001adda4  12  pc_walk_char           ccEvPcMoveChar       pc type code rot dist                    party character walks to a point rot/dist from a character [play]
  65  0x001adf74   6  pc_mode                ccEvPcAct            pc param                                 ccSpcManager entry word +8 = param (pc -3: whole party, -1/-2: party slot) [play]
  66  0x001ae164   6  pc_command             ccEvPcAct            pc on                                    ccEntryCmnd / ccDeleteCmnd for the party character [play]
  67  0x001ae7d8   6  pc_put_marker          ccEvPcPos            pc marker                                party character placed at the marker, facing [play]
  68  0x001ae9e8   6  party_put_marker       ccEvPcPos            pc marker                                the companions (party slots 1, 2) placed at the marker (inference) [play]
  69  0x001aec5c  10  party_put              ccEvPcPosDirect      pc x y z                                 the companions placed at (x, y, z) * 10 (inference) [play]
  70  0x001aeb84  10  pc_put                 ccEvPcPosDirect      pc x y z                                 party character placed at (x, y, z) * 10 [play]
  71  0x001aed5c   8  pc_turn                ccEvPcRot            pc dirc chg                              party character turns to dirc degrees [play]
  72  0x001aee30  10  pc_face                ccEvPcRotChar        pc type code chg                         party character turns to face a character [play]
  73  0x001aefd4   6  pc_use_skill           ccEvCondition        pc skill                                 ccItemSkillCompel(pc, pc, skill, 0) [play]
  74  0x001af028   6  enemy_put              ccEvPcPos            enemy posnum                             enemy placed at event position posnum (see set_pos) [play]
  75  0x001af0e4   8  affect_on              ccEvAffect           type code bit                            character's affectMask |= 1 << bit [play]
  76  0x001af1a8   8  affect_off             ccEvAffect           type code bit                            character's affectMask &= ~(1 << bit) [play]
  77  0x001af26c   4  party_add              ccEvMember           pc                                       ccParty::AddMember, and the menu face [play]
  78  0x001af2c4   4  party_remove           ccEvMember           pc                                       ccParty::DelMember (pc -3: everyone, -1/-2: that slot) [play]
  79  0x001af3ec   4  member_add             ccEvMember           pc                                       partyMemberFlag and partyMemberExp bit pc [flag]
  80  0x001af440   4  member_add_msg         ccEvMember           pc                                       as member_add; when playing, SE 74 and an info box with the name [split]
  81  0x001af53c   4  call_on                ccEvMember           pc                                       partyMemberCall bit pc (and the stored copy while calls are locked) [flag]
  82  0x001af5a4   4  call_on_later          ccEvMember           pc                                       partyMemberCall bit pc, or only the stored copy while locked [flag]
  83  0x001af618   4  call_off               ccEvMember           pc                                       clear partyMemberCall bit pc (and in the stored copy while locked) [flag]
  84  0x001af684   2  call_lock              -                    -                                        partyMemberCallStore = partyMemberCall; set bit 31, clear bits 0-17 [flag]
  85  0x001af6ec   2  call_unlock            -                    -                                        partyMemberCall = partyMemberCallStore; store = 0 [flag]
  86  0x001af710   4  exp_on                 ccEvMember           pc                                       partyMemberExp bit pc [flag]
  87  0x001af748   4  exp_off                ccEvMember           pc                                       clear partyMemberExp bit pc [flag]
  88  0x001af784   2  save_party             -                    -                                        partyMemberSave = the current party's bits [play]
  89  0x001af7fc   4  gate_add               ccEvAreaCodeAdd      area                                     story area into its server's Chaos Gate list (SetGateList), its words into wordList [flag]
  90  0x001af980   4  gate_add_msg           ccEvAreaCodeAdd      area                                     as gate_add; when playing, an info box with the address [split]
  91  0x001afbe0   4  gate_mark              ccEvAreaCodeAdd      area                                     gateListMark[server] bit area [flag]
  92  0x001afc70   4  gate_unmark            ccEvAreaCodeAdd      area                                     clear gateListMark[server] bit area [flag]
  93  0x001afd04  10  item_add               ccEvAddItem          pc category id num                       ccSaveData::AddItem; when playing, for a companion (pc not 0) and a category below 10 whose character GetSpc finds, through the menu (AddSpcItem) instead [split]
  94  0x001afdac  10  item_add_menu          ccEvAddItem          pc category id num                       when playing, the item-get menu (29) first; then AddItem [split]
  95  0x001afe74  10  item_del               ccEvAddItem          pc category id num                       ccSaveData::DelItem [flag]
  96  0x001afeb0   6  gold_add               ccEvAddGold          pc num                                   spcParam[pc].base.gold += num, clamped to 0..9,999,999 [flag]
  97  0x001b0174   2  noise2                 -                    -                                        ccMenu.interNoiz = 2 (interference effect) [play]
  98  0x001b0198   2  noise1                 -                    -                                        ccMenu.interNoiz = 1 [play]
  99  0x001b01b8   2  noise3                 -                    -                                        ccMenu.interNoiz = 3 [play]
 100  0x001b01d8  14  scene                  ccEvScene            area town field dungeon floor block      ccGame::ChangeScene [play]
 101  0x001b021c   4  mode                   ccEvChangeReq        num                                      ccGame::ChangeRequest(num, 7); num 5 (field game): ChangeRequest(5, 8) and ChangeArea(0, lastTown) [play]
 102  0x001aa5d4   4  wallpaper_add          ccEvDtList           num                                      desktop wallpaper num (dtWallpaperList bit) [flag]
 103  0x001aa694   4  bgm_add                ccEvDtList           num                                      desktop BGM num (dtBgmList bit) [flag]
 104  0x001aa754   6  bbs_post               ccEvBbs              thread post                              playing: post appears (bbsList 1) if absent; replaying: marked 3 [split]
 105  0x001aa7e8   6  bbs_remove             ccEvBbs              thread post                              bbsList[thread][post] = 0 [flag]
 106  0x001aa828   6  bbs_post7              ccEvBbs              thread post                              playing: bbsList 7 if absent; replaying: 3 [split]
 107  0x001aa8b8   4  mail                   ccEvMail             mail                                     playing: ccSaveData::NewMail if never received; replaying: ReadNewMail [split]
 108  0x001aa918   4  mail_vol               ccEvMail             mail                                     as mail, only while saveData.clearFlag <= this event's volume index [split]
 109  0x001aaa14   6  mail_member            ccEvMailMemSave      mail pc                                  as mail, only if pc is in partyMemberSave [split]
 110  0x001aaab4   4  mail_remove            ccEvMail             mail                                     mailList[mail] = 0 [flag]
 111  0x001aaae4   4  news_add               ccEvNews             news                                     webnewsList[news] = 1 if 0 [flag]
 112  0x001aab28   4  news_remove            ccEvNews             news                                     webnewsList[news] = 0 [flag]
 113  0x001b02e8   4  frame_rate             ccEvFrameRate        rate                                     ccSystem::SetFrameRate(rate) [play]
 114  0x001b0318   4  overlay                ccEvOverlay          num                                      ccLoadOverlay: 0 DEMO, 1 DESKTOP, 2 TOPPAGE, 3 GCMN [play]
 115  0x001b02a0   4  crisis                 ccEvNum              num                                      saveData.crisis = num != 0 [flag]
 116  0x001b03d4   8  set_point              ccEvSetPoint         floor block evnum                        ccEvent::SetEventPoint [play]
 117  0x001b0410  16  set_pos                ccEvSetPos           floor block posnum dirc x y z            ccEvent::SetEventPos: event position posnum = (x, y, z) * 10, dirc in degrees [play]
 118  0x001b04b8   4  area                   ccEvAreaCodeAdd      area                                     areas 1-13: leave the field (WORLD_MAN::Quit) and set eventAreaNumber; others: SimGenerateCode from its words [play]
 119  0x001b0598   2  clear_operate          -                    -                                        eventMng.operateSet = -1, operateTarget = 0 [play]
 120  0x001b05b8   2  clear_area_code        -                    -                                        eventMng.areaCodeSet[0..2] = -1 [play]
 121  0x001b05dc   2  map_on                 -                    -                                        ccMenu.mapStatus = 1 [play]
 122  0x001b0600   2  show_map               -                    -                                        SE 97, then WORLD_MAN::ShowMap until it returns true [play]
 123  0x001b064c   2  remove_trap            -                    -                                        SE 228, effRemoveTrap at the command target, 20 frames, affect 12 on it from the player [play]
 124  0x001b06c8   4  virus_core             ccEvVirus            area                                     regenerate the area, protectArea bit area, delete its protect items (category 15) from Kite [replay]
 125  0x001b084c   4  town_move              ccEvTown             town                                     townMoveFlag bit town [flag]
 126  0x001b0888   2  data_drain             -                    -                                        saveData.plcol = 1; AddSkill(0, 2): Kite gets skill 2 (Data Drain) [flag]
 127  0x001b08b8   2  prev_room              -                    -                                        WORLD_MAN::GoPrevRoom [play]
 128  0x001b08d8  10  area_ban               ccEvRoomBan          field dungeon floor block                ccSaveData::SetAreaBan [flag]
 129  0x001b0910  10  area_unban             ccEvRoomBan          field dungeon floor block                ccSaveData::ClearAreaBan [flag]
 130  0x001b0948   6  room                   ccEvRoom             floor block                              WORLD_MAN::RoomSelect(floor, block) [play]
 131  0x001b097c   4  room_point             ccEvPointNum         num                                      RoomSelect to event point num's room [play]
 132  0x001b09fc   6  hold                   ccEvTarget           type code                                start the ccThEvHold task on the character [play]
 133  0x001b0a68   2  hold_end               -                    -                                        stop the ccThEvHold task [play]
 134  0x001a8d80   -
 135  0x001b0a94  12  pc_tint                ccEvSpcCol           pc r g b rate                            party character colour (affectColor rgb, rate, fixed) [play]
 136  0x001b0b74   4  pc_tint_off            ccEvMember           pc                                       party character colour off [play]
 137  0x001b0c1c   4  piros_colour           ccEvPiroCol          code                                     if Piros (8) is present: a flash, SE 74 and info sequence chosen by eventStatus[1] (14-way switch) [play]
 138  0x001b1918   6  status_set             ccEvExecStatus       index num                                saveData.eventStatus[index] = num [flag]
 139  0x001b1948   6  status_add             ccEvExecStatus       index num                                eventStatus[index] += num [flag]
 140  0x001b1980   6  status_sub             ccEvExecStatus       index num                                eventStatus[index] -= num [flag]
 141  0x001b19b8   4  protect                ccEvAreaCodeAdd      area                                     saveData.protectArea bit area [flag]
 142  0x001b1a24   6  talk_num               ccEvTalkNum          pc num                                   saveData.talkNum[pc] = num (0 for pc 1 in Parody Mode) [flag]
 143  0x001b1a84   2  regist_npc16           -                    -                                        eventMng.registNpcNum = 16 [play]
 144  0x001b1aa4   6  marker_pos             ccEvMarker2Pos       marker posnum                            event position posnum = the marker's position (SetEventPos) [play]
 145  0x001b1c00   6  fade                   ccEvFade             count alpha                              screen fade (ccScFade::EntryFade) over count frames to alpha [play]
 146  0x001b1c60   6  fade_more              ccEvFade             count alpha                              ccScFade::ContinueFade on that fade [play]
 147  0x001b1ca4   2  staff_roll             -                    -                                        run the desktop staff roll to its end, 30 frames, 30 fps, then (not Parody) desktop menu 8 (OUT: also wallpapers 57-66 with an info box each; QUA: 77-86) [play]
 148  0x001b1dd4   2  clear_count            -                    -                                        saveData.clearFlag += 1 (not in Parody Mode) [play]
 149  0x001b1e20   6  friendship             ccEvExecFriendship   pc num                                   ccSaveData::AddFriendship(pc, num) [flag]
 150  0x001b1e50   4  menu                   ccEvMenuReq          num                                      open menu num and wait for it to close [play]
 151  0x001b1ec8   2  condition_fx_on        -                    -                                        ccSpcConditionEffectON [play]
 152  0x001b1ee4   2  condition_fx_off       -                    -                                        ccSpcConditionEffectOFF [play]
 153  0x001b1f04   2  player_skill           -                    -                                        the player uses a skill on the command target (ccSkillCheck, ccSkillRequest) [play]
 154  0x001b1ff8   4  target_forbid          ccEvNum              num                                      ccMenu.targetForbid = num != 0 [play]
 155  0x001b2040   4  last_town              ccEvTown             town                                     saveData.lastTown = town (low byte) [flag]
 156  0x001b2070   2  name_entry             -                    -                                        the desktop's NameEntry_Control until done [play]
 157  0x001b20fc   2  gate_hack_anim         -                    -                                        menus off, wait while ccCheckGtHackAnm [play]
 158  0x001b218c   2  open_door              -                    -                                        DUNGEON::OpenDoor in the current room [play]
 159  0x001b21cc   2  close_door             -                    -                                        DUNGEON::CloseDoor2 in the current room [play]
 160  0x001b220c  10  sound                  ccEvSoundCtrl        cmd p0 p1 p2                             ccSndEvRequest(cmd, p0, p1, p2) [play]
 161  0x001b2248   6  trans_off              ccEvTarget           type code                                character's transDist = 0 [play]
 162  0x001b22f4   6  trans_on               ccEvTarget           type code                                character's transDist = 1 [play]
 163  0x001b23a8   2  battle_ready           -                    -                                        game.inBattleDist = -1.0; AddOperate(14, 0, 0) [play]
 164  0x001ad9b0  10  pc_run_pos             ccEvPcMovePos        pc x y z                                 party character runs to (x, y, z) * 10 (remote command 2) [play]
 165  0x001aa468   6  add_operate_sf         ccEvOperate          num except                               as add_operate over operations 19-28 (AddOperate sf 1) [play]
 166  0x001aa4a0   6  del_operate_sf         ccEvOperate          num except                               as del_operate over operations 19-28 [play]
 167  0x001aff34   6  desktop_item           ccEvDtItem           type id                                  wallpaper (0), BGM (1) or movie (2) id for the desktop; when playing, SE 74 and an info box [split]

# CheckOpen: jump table 0x00355d60, 41 entries
   0  0x001a7478   -
   1  0x001a74b8   4  event_done             ccEvFlagGrp          event                                    eventFlag[event] bit 62 (end_event -2, or bit 63 promoted by ccStartThEvent)
   2  0x001a750c   4  block_done             ccEvFlagLocal        num                                      bit num of this event's flag word
   3  0x001a7568   6  phase                  ccEvPhase            phase comp                               eventMng.enablePhase against phase
   4  0x001a7614   4  game_status            ccEvGstatus          status                                   game.status == status (the mode)
   5  0x001a7658   4  in_point               ccEvPointNum         num                                      game.floor/block are event point num's
   6  0x001a76f4  14  scene                  ccEvScene            area town field dungeon floor block      all six of game's equal
   7  0x001a7788   4  in_town                ccEvTown             town                                     game.area 0 and game.town == town
   8  0x001a77d4   6  in_field               ccEvField            town field                               game.area 1, town and field
   9  0x001a7838   8  in_dungeon             ccEvDungeon          town field dungeon                       game.area 2, town, field and dungeon
  10  0x001a78f8   8  status                 ccEvOpenStatus       index num comp                           saveData.eventStatus[index] against num
  11  0x001a79ac   8  status_range           ccEvOpenStatus       index lo hi                              lo <= eventStatus[index] <= hi
  12  0x001a7a0c   6  gate_words             ccEvAreaCode         area except                              the words at the gate (areaCodeSet) are the area's on its server; except 1 negates
  13  0x001a7b3c   6  talked_to              ccEvTarget           type code                                operateSet == 9 (talk) and the character talked to (operateTarget) is type/code
  14  0x001a7bc4   6  operate                ccEvOperate          num except                               the operation the player tried (operateSet) is num (-1: one not intercepted); except 1 negates
  15  0x001a7ca8   8  near_marker            ccEvMarker           marker bounds comp                       player-to-marker distance against bounds * 10
  16  0x001a7e84   6  answer                 ccEvSelect           msg select                               the last question was message msg and the answer was select
  17  0x001a7ed4   6  bbs_read               ccEvBbs              thread post                              bbsList[thread][post] == 3
  18  0x001a7f34   4  mail_got               ccEvMail             mail                                     mailList[mail] is 4, 5 or 6
  19  0x001a7f9c   4  mail_4                 ccEvMail             mail                                     mailList[mail] == 4
  20  0x001a7fe8   4  mail_5                 ccEvMail             mail                                     mailList[mail] == 5
  21  0x001a8030   4  mail_6                 ccEvMail             mail                                     mailList[mail] == 6
  22  0x001a8078   4  news_read              ccEvNews             news                                     webnewsList[news] == 3
  23  0x001a80c0   4  in_party               ccEvMember           pc                                       pc is in the party (pc -1: the party has 2 or more)
  24  0x001a8138   4  not_in_party           ccEvMember           pc                                       pc is not in the party (pc -1: fewer than 2)
  25  0x001a81b0   4  in_party_of_2          ccEvMember           pc                                       the party has exactly 2 and pc is one
  26  0x001a8210   4  party_other            ccEvMember           pc                                       counts party slots 1-2 holding someone else (can count twice)
  27  0x001a8294   4  callable               ccEvMember           pc                                       partyMemberCall bit pc
  28  0x001a82e4   6  present                ccEvTarget           type code                                type 2: in the SPC manager; 5/6: in the entry list; 20: in the gimmick list; 7: the boss (bossEntry) whose task param is still 0
  29  0x001a84bc   6  absent                 ccEvTarget           type code                                the negation of present (7: the boss task param is non-zero)
  30  0x001a8614   2  no_active              -                    -                                        ccCheckActiveObject() returns non-zero and no gimmick is active
  31  0x001a86dc   2  no_entries             -                    -                                        g_entCtrl.enNum == 0 and mcNum == 0
  32  0x001a871c   2  no_menu                -                    -                                        ccMenuCtrl::CheckMenuType() == -1
  33  0x001a8758  12  has_item               ccEvItem             pc category id num comp                  item (category, id) count in pc's list against num; category 15 is impItemList
  34  0x001a8a04   8  friendship             ccEvOpenFriendship   pc num comp                              spcParam[pc].friendship against num
  35  0x001a8acc   4  pad                    ccEvInput            mask                                     ccSys.pad[0].push & mask
  36  0x001a8b14   6  enemy_pp               ccEvTarget           type enemy                               GetEnemy(enemy) param PPcount != 0 (type unused)
  37  0x001a8b6c   4  member                 ccEvMember           pc                                       partyMemberFlag bit pc
  38  0x001a8bbc   4  member_saved           ccEvMember           pc                                       partyMemberSave bit pc
  39  0x001a8c0c   6  volume                 ccEvNumComp          num comp                                 volumeNum against num
  40  0x001a78a8   6  in_room                ccEvRoom             floor block                              game.floor and game.block

# SetCurrentOpen: jump table 0x00355d30, 12 entries
   0  0x001a7110   2  none                   -                    -                                        nothing
   1  0x001a7110   2  none                   -                    -                                        nothing
   2  0x001a703c   4  block_done             ccEvFlagLocal        num                                      this event's flag bit num
   3  0x001a6ef4   6  phase                  ccEvPhase            phase comp                               eventMng.enablePhase against phase
   4  0x001a6f14   4  game_status            ccEvGstatus          status                                   game.status; resets the scene to any unless status 5
   5  0x001a7054   4  in_point               ccEvPointNum         num                                      the room of event point num
   6  0x001a6f58  14  scene                  ccEvScene            area town field dungeon floor block      the scene (-1 = any)
   7  0x001a6f98   4  in_town                ccEvTown             town                                     area 0, the town
   8  0x001a6fc8   6  in_field               ccEvField            town field                               area 1
   9  0x001a7000   8  in_dungeon             ccEvDungeon          town field dungeon                       area 2
  10  0x001a70bc   8  status                 ccEvOpenStatus       index num comp                           eventStatus[index] against num
  11  0x001a70e8   8  status_range           ccEvOpenStatus       index lo hi                              lo <= eventStatus[index] <= hi
```

`pc` operands are `charTbl` indexes (0 Kite ... 17 Helba; -1/-2 party slots,
-3 the party). Target `type` is tested against the character's base type: 2
party, 3/4 NPC, 5/6 enemy, 20 gimmick (conditions 28, 29). Coordinates are
multiplied by 10. Condition comparisons: 0 `==`, 1 game value `>=` operand,
2 game value `<=` operand.

## Other volumes

`tools/evscript.py` reads every table from the executable it is given: it
finds the three switches from their code (`sltiu`, default branch, jump
table, `jr`), measures each case's operand count by running the case in
eemu, and takes script and message extents from the gaps between arrays. On
Infection this reproduces the symbol table's names and sizes exactly. `check`
walks every script with each volume's own `CheckOpen`, `SetCurrentOpen` and
`Execute`: 0 mismatches on all four.

| | Execute | jump table | entries | cases | CheckOpen | SetCurrentOpen |
| --- | --- | --- | ---: | ---: | --- | --- |
| INF | 0x001a8d20 | 0x00355fd0 | 168 | 166 | 0x001a7400 | 0x001a6ec0 |
| MUT | 0x001bdf00 | 0x00370070 | 169 | 167 | 0x001bc5e0 | 0x001bc0a0 |
| OUT | 0x001b4250 | 0x00369a20 | 170 | 167 | 0x001b2940 | 0x001b23c0 |
| QUA | 0x001bba60 | 0x0025cff0 | 170 | 168 | 0x001ba150 | 0x001b9bd0 |

The 40 conditions and 12 precondition tags have the same lengths everywhere.

**New and changed opcodes:**
- **99 `noise`**: no operand in INF (sets `ccMenu.interNoiz = 3`). From MUT
  on it takes 1: `interNoiz = level + 2` for levels 1-3, else 0.
- **168**, MUT on, no operand: while playing, if `mailList[324]` is 0 and
  any of towns 1-4 passes `ccPgAdultCheck(town, type) >= 0` for all three
  Grunty types, it sends mail 324. It is used once, in event 101 (M201).
- **169**, a case only in QUA (`0x001c52e8`); OUT's table entry is the
  default.
  - It sets `game+0x7c = 1` and reads `\DATA\KFED.BIN` and
    `\DATA\KFAED.BIN` into two `$gp` globals (`0x0027956c`, `0x00279570`).
  - It adds the `ending.ccs` file list and runs gcmn `0x004f0a30`, which
    starts a thread and waits for it.
  - It then frees both buffers. No other code reads the two globals.
  - It is used in event 314 "Ending", block 3; OUT's copy of that script
    already contains it and skips it.

**Same length, changed behaviour, from MUT on:**
- 101 `mode` always calls `ChangeRequest(num, 7)`.
- 118 `area` quits `WORLD_MAN` only while `game+0x14` is set.
- 145 `fade` calls `ccScFade::Init` first; 146 starts a new fade when the
  stored one is not 0-3.
- 167 plays no movie in Parody Mode.
- 84 `call_lock` clears bits 0-20 (was 0-17).
- 96, 142, 80 and conditions 33, 34 reach characters 18-20 through the save
  extension ([save](../formats/save.md#the-extension-mutation-on)).
- 8, 9 also check `dtMenu` while waiting.

OUT's 147 `staff_roll` grants wallpapers 57-66 with an info box each, and
QUA's 77-86. OUT reimplements 6, 8, 9, 60, 71, 72 and 150 through unnamed
functions, which have not been read. Otherwise OUT and QUA differ only in
compilation. Several structure offsets moved without a change in behaviour.

**Scripts:**

| | scripts | blocks | instructions | conditions | preconditions |
| --- | ---: | ---: | ---: | ---: | ---: |
| MUT | 196 | 2035 | 14253 | 2128 | 2164 |
| OUT | 199 | 2062 | 14645 | 2123 | 2220 |
| QUA | 206 | 2139 | 14764 | 2249 | 2227 |

- MUT adds events 272-275 ("SEARCH ..."), relabels 267-271, and changes 73
  scripts.
- OUT adds 316 "ADD SPC", 317 "ITEM COMPLETE" and 361 "Vol3-SIGN06", and
  changes 31.
- QUA adds 456-462 (ML-HELBA, TSUKASA, SUBARU, SORA, MIA, ELK, ORCA); its
  199 shared scripts are byte-identical to OUT's.
- The first story event of each volume (101, 201, 301) opens on
  `event_done 100·(v−1)`, the flag a carried-over save gets.

Each volume has message text only for its own main story (INF M1, MUT M2,
OUT M3, QUA M4) and the side events. OUT's and QUA's other M groups point
into BSS. Voice comes from each volume's own files.

## The port

`crates/piney-event` runs these scripts; [the event interpreter
page](event-vm.md) has the details. In short:

- **Our own instruction set** (`ir`): a script is open conditions, then
  blocks of settings, conditions and instructions, each a typed variant with
  the operand struct's field names (169 instructions: Infection's 166, plus
  Mutation's `noise` with a level, `grunty_mail` and Quarantine's
  `ending_kanji`; 40 conditions; the 10 settings 2-11). No opcode numbers or
  offsets; message tables separate. `text` prints and parses it, exactly;
  a clean-room script set is written in that form.
- **The official adapter** (`official`) reads the scripts and both message
  tables from a disc's boot executable, finding the tables through the code
  on all four volumes, and converts to and from the IR. Every script of
  every volume decodes and encodes back to identical shorts (INF 192, MUT
  196, OUT 199, QUA 206). The port does not read them at play time:
  `piney-build` writes each disc's events in the text form into its port
  data (`PINEY/EVENTS.EVS`, `official::events_text`), and
  `official::events` parses that (`plans/build-data.md`); a disc without
  port data (a tool's or a check's image) is read from its executable.
  `the_builds_scripts_are_the_executables` checks the text form event for
  event on every volume.
- **The interpreter** (`vm`) is Infection's `eventSub`, `ccEventFlagSet`,
  `CheckOpen`, `Execute` and the task `ccThEvent`, one frame per
  `Vm::frame`; every effect outside it goes through the `Host` trait, and
  the saved state is the game's `ccSaveData`. `tools/test_event_vm.py` checks
  it against the game's own code in `tools/eemu.py`: the replay of every
  script, every condition, every block played, and a new game reaching the
  desktop frame by frame.

## Unknown

- What fills OUT's and QUA's BSS message groups; what OUT's rewritten cases
  do; what gcmn `0x004f0a30`'s thread does with the KFED buffers.
- Nothing about the numbering the instructions take is unknown any more.
  The streams are `streamTbl`'s ([stream](stream.md#the-tables)); the
  markers `markerEvTbl`'s in a Root Town and the event positions elsewhere
  ([field game](field-game.md)); the menus `ccMenuCtrl`'s
  ([field UI](field-ui.md)); the operations above; the sound commands
  `ccSndEvRequest`'s ([sound](sound.md#the-event-instruction-ccsndevrequest));
  `pc_act`'s and `npc_act`'s codes `ccEvent::Execute`'s and
  `ccRtownPC::eventMode`'s ([field game](field-game.md)). Only `npc_act`
  -3 and -5 (row 139's drain, the Administrator's -5) are not ported, and
  only volume 2's event 114 uses them.
- None left about the boards, the windows or the camera:
  - `bbsList` holds 0 not posted, 1 new, 3 read, and 7 the player's own
    post waiting to be written out ([top page](toppage.md)). Mail states
    4-6 are the mailer's: read, and replied with one or the other answer
    ([desktop](desktop.md)).
  - `emode`'s low byte picks how the window ends: 0 fades it out, 1
    chains to the next record, 2 leaves it up, 3 asks. 0x100 shows the
    text at once, untyped. 0x200 is a typing-speed test that gives one
    frame either way ([desktop](desktop.md)).
  - The camera's angles are `DEG2RAD` shorts, 65536 to a turn: `rotx`
    the pitch, `roty` the heading. The distances are tenths. `cam_mode4`
    (`cpCtrl` 4) leaves the camera point where it is while the look-at
    goes on moving (`piney_world::evcam`).
