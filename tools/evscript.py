#!/usr/bin/env python3
"""Event scripts: the `short eventTbl<name>[]` arrays that ccEvent::CheckOpen
and ccEvent::Execute run.

    tools/evscript.py list   ELF                         every script, its event number and name
    tools/evscript.py dis    ELF NAME|EVENT [--parody] [--japanese]
                                                         one script, disassembled
    tools/evscript.py dump   ELF [--out DIR]             every script, to DIR/<name>.txt
    tools/evscript.py survey ELF                         walk them all: clean ends, counts
    tools/evscript.py ops    ELF                         the opcode tables, with case addresses
    tools/evscript.py check  ELF                         lengths and block ends against the
                                                         game's own code, run in tools/eemu.py

Works on all four volumes (the stripped ones through the names piney-gen
syms carries).
Nothing is taken from Infection's addresses: the three switches are found in
the code of Execute, CheckOpen and SetCurrentOpen (`sltiu $at, rX, COUNT`, the
default branch, the lui/addiu of the jump table), every case's operand count
is measured by running the executable's own case at lv 0 in tools/eemu.py,
and the scripts, messages and voice come from that executable's tables. The
names below are Infection's cases (with the codes later volumes add); a code
whose measured length the tables do not document is shown as op_N.

Event N's script is eventTbl[N / 50][N % 50].tbl (ten ccEvTbl groups M1 S1
M2 S2 M3 S3 M4 S4, then eventTblM for 400-499), with a name in .str. A
script's extent runs to the next script or table (every array is 16-byte
aligned, the padding zero); in Infection that equals the symbol sizes. A
script is a run of shorts:

    open conditions ... 0            CheckOpen, ev -1: the event is live when all hold
    then up to 62 blocks, each:
        (-2 TAG operands)*           SetCurrentOpen: persistent block preconditions
        conditions ... 0             CheckOpen: this block runs when all hold
        instructions ... 0           Execute
    -1                               end of the event

Every opcode and condition has a fixed operand length and there are no jumps:
the block is the unit of control flow. The game walks a block it is not
running with lv 0 (every case advances eot before looking at lv), and sets
the block's bit in saveData.eventFlag[N] when it runs one (unless op 4). It
skips an event whose flag word has bit 63 (closed, by end_event) or bit 62
(done); ccStartThEvent (INF 0x001b5230) turns 63 into 62 for events 0-449
when the event task starts again, and bit 62 is what `event_done` tests.

The event task ccThEvent (INF 0x001b5a60) makes passes over the live events.
ccEnableThEvent (INF 0x001b52c0), called by each mode's setup, sets
eventMng.enablePhase to 0 before the mode's files load, 2 once they have, and
4 when play starts; the task makes one pass at 0 and one at 2 (moving them
to 1 and 3) and from 4 on a pass every frame. `set phase == 0` blocks
register what an area holds, `== 2` ones play arrival scenes, `>= 4` ones
run during play.

Messages are evMsgTbl[N / 50][N % 50][msg] (evMsgTblp in Parody Mode); a
message array runs to the next one, less trailing all-zero records. Voice is
what the game's ccEvVoiceRequest(N, msg) and evVoicePlay hand to sewordCmd,
run in tools/eemu.py.
"""

import argparse
import bisect
import collections
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mips  # noqa: E402
from image import Program  # noqa: E402
from text import render  # noqa: E402

# The three switches, by name; each executable's own code is read for them.
EXECUTE = "Execute__7ccEventFRPsiii"          # (short *&eot, int grp, int ev, int lv)
CHECKOPEN = "CheckOpen__7ccEventFRPsiii"      # same signature
SETCURRENT = "SetCurrentOpen__7ccEventFPs"    # (short *tbl) -> tbl past the setting
BLOCKS = 62                   # eventSub / ccEventFlagSet loop bound
END, CURRENT = -1, -2
GROUPS = ("M1", "S1", "M2", "S2", "M3", "S3", "M4", "S4")   # eventTbl[0..7]; 8, 9 are ML

# Execute opcodes: number -> (name, operand struct, operand fields, runs, effect).
# Every operand is a short. `runs` says at which lv the case acts: "play" only
# when the block really runs (lv 2), "flag" also when ccEventFlagSet replays a
# volume's flags (lv 1), "split" does something different at lv 1, "replay"
# only at lv 1. At lv 0 every case only advances eot. The effects are
# Infection's cases; "(MUT+ ...)" notes what later volumes' cases do
# differently, read from their code (the numbering and lengths are the same
# except where VARIANTS says otherwise).
OPS = {
    1: ("end_event", "ccEvFlagGrp", "grp", "flag",
        "eventFlag[grp] bit 63 (closed); grp < 0 is this event, and -2 also sets bit 62 (done) at"
        " once"),
    2: ("set_block", "ccEvFlagLocal", "num", "flag",
        "set bit num of this event's flag word; num < 0 sets bits 0..ev-1; num >= 30 halts (store"
        " to 0)"),
    3: ("clear_block", "ccEvFlagLocal", "num", "flag", "clear bit num of this event's flag word"),
    4: ("repeatable", "", "", "flag",
        "this block does not set its own bit when it ends, so it can run again"),
    5: ("wait", "ccEvWait", "count", "play", "wait count + 1 frames (ccBreathThread)"),
    6: ("message", "ccEvMessage", "msg", "play",
        "speech window (ccMessage::Change): name and three lines, voice; emode 3 asks a question "
        "and stores msg/answer in eventMng.msgNum/msgSelect"),
    7: ("clear_answer", "", "", "play", "eventMng.msgNum = msgSelect = 0"),
    8: ("info", "ccEvMessage", "msg", "play",
        "information window (ccMessage::ChangeInfo), then closed (MUT+: the wait also checks "
        "dtMenu)"),
    9: ("info_now", "ccEvMessage", "msg", "play",
        "as info, window shown without its opening fade (windowStatus, windowAlpha 0), then 10 frames "
        "rather than 5 before the wait"),
    10: ("stream", "ccEvStream", "num", "play",
         "play cutscene stream num (ccEventStream(num, 1)); in the field the area file list is "
         "dropped and reloaded"),
    11: ("entry", "ccEvEntry", "type code marker param", "play",
         "ccEvent::AddEntry: register a character to appear (16 slots; halts when full)"),
    12: ("entry_mc", "ccEvEntryMc", "type code marker param", "play",
         "ccEvent::AddEntryMc (16 slots; halts when full)"),
    13: ("remove", "ccEvTarget", "type code", "play",
         "type 2: party character code out of the party and the SPC manager; 5/6: entry object "
         "code deleted; -1: all objects"),
    14: ("add_operate", "ccEvOperate", "num except", "play",
         "intercept player operation num (eventMng.operate bit; num -1: all of 0-17; except 1: "
         "all of them but num)"),
    15: ("del_operate", "ccEvOperate", "num except", "play",
         "stop intercepting operation num (DelOperate, same encoding)"),
    16: ("add_target", "ccEvTarget", "type code", "play",
         "talking to the character (operation 9) goes to the event (eventMng.target[])"),
    17: ("del_target", "ccEvTarget", "type code", "play", "undo add_target"),
    18: ("menu_ban", "", "", "play",
         "cutscene on (MenuBan): menus forbidden and hidden, party under manual AI, camera id "
         "saved"),
    19: ("menu_clear", "", "", "play", "cutscene off (MenuClr): undoes menu_ban"),
    20: ("add_area_code", "ccEvAreaCode", "code except", "play", "ccEvent::AddAreaCode"),
    21: ("del_area_code", "ccEvAreaCode", "code except", "play", "ccEvent::DelAreaCode"),
    22: ("cam_look_pos", "ccEvCamSetVpPos", "x y z", "play",
         "camera look-at = (x, y, z) * 10, at once"),
    23: ("cam_look_char", "ccEvCamSetVpChar", "type code height", "play",
         "look-at = the character's position + height * 10, at once"),
    24: ("cam_follow_char", "ccEvCamSetVpChar", "type code height", "play",
         "look-at tracks the character (vpCtrl 1)"),
    25: ("cam_look_marker", "ccEvCamSetVpMarker", "marker", "play",
         "look-at = the marker's position"),
    26: ("cam_look_pos_half", "ccEvCamSetVpPos", "x y z", "play",
         "look-at moves halfway towards (x, y, z) * 10"),
    27: ("cam_look_char_half", "ccEvCamSetVpChar", "type code height", "play",
         "look-at moves halfway towards the character"),
    28: ("cam_look_marker_half", "ccEvCamSetVpMarker", "marker", "play",
         "look-at moves halfway towards the marker"),
    29: ("cam_pan_pos", "ccEvCamChgVpPos", "x y z rate", "play",
         "look-at moves to (x, y, z) * 10 at rate"),
    30: ("cam_pan_char", "ccEvCamChgVpChar", "type code height rate", "play",
         "look-at moves to the character at rate"),
    31: ("cam_pan_follow", "ccEvCamChgVpChar", "type code height rate", "play",
         "look-at moves to and tracks the character at rate"),
    32: ("cam_pan_marker", "ccEvCamChgVpMarker", "marker rate", "play",
         "look-at moves to the marker at rate"),
    33: ("cam_orbit", "ccEvCamSetCp", "rotx roty dist", "play",
         "camera angles and distance * 10, at once (cpCtrl 2)"),
    34: ("cam_orbit_move", "ccEvCamChgCp", "rotx roty dist rate", "play",
         "camera angles and distance, moving at rate (cpCtrl 2)"),
    35: ("cam_orbit_turn", "ccEvCamChgCp", "rotx roty dist rate", "play",
         "as cam_orbit_move with the rate in degrees (cpCtrl 3)"),
    36: ("cam_mode4", "", "", "play", "cpCtrl = 4: the camera point stays where it is, the look-at goes on"),
    37: ("teach_camera1", "", "", "play",
         "tutorial: this event's message 7 (55 with the other camCtrlType), event 3's voice; "
         "waits for 150 frames of camera input"),
    38: ("teach_camera2", "", "", "play", "tutorial: messages 9 / 56, as teach_camera1"),
    39: ("teach_camera3", "", "", "play",
         "tutorial: messages 11 / 57; waits for a button, then 45 frames"),
    40: ("camz_set", "ccEvCamzSet", "vx vy vz cx cy cz", "play",
         "two-point camera: look-at and position * 10, at once"),
    41: ("camz_move", "ccEvCamzChg", "vx vy vz cx cy cz vprate cprate", "play",
         "two-point camera moving at the rates"),
    42: ("camz_speed", "ccEvCamzType", "vstype cstype", "play",
         "speed curve types of the two points"),
    43: ("camz_point", "ccEvCamzInpSet", "num vx vy vz cx cy cz", "play",
         "interpolation point num (num clamped to 16 in the script itself)"),
    44: ("camz_path", "ccEvCamzInpChg", "num vprate cprate alpha", "play",
         "run the camera through num points (num clamped to 16 in place)"),
    45: ("radiator", "ccEvRadiator", "rtype type code x y z roty rotz", "play",
         "entryObject(ep, 1) of gimmick row 19 at (10x, 10y, 10z) turned (0, DEG2RAD roty, DEG2RAD "
         "rotz), param[2] rtype, param[3] the character type code names; rtype 0: rays at its right "
         "hand until its AI leaves manual mode"),
    46: ("boss_smoke", "ccEvBossSmoke", "floor block x y z", "play",
         "gimmick entry type 1, id 19 in a dungeon room"),
    47: ("delete_gimmick19", "", "", "play", "delete every id-19 gimmick (deleteGimmick)"),
    48: ("camera", "ccEvCamera", "type code height rotx roty dist", "play",
         "look at the character with angles and distance, at once"),
    49: ("camera_end", "", "", "play",
         "back to the normal camera (changeCamera(1)); stop the event camera task"),
    50: ("camera_end_reset", "", "", "play",
         "changeCamera(eventMng.normalCamID), cameraSoftReset, stop the camera task"),
    51: ("npc_act", "ccEvPcAct", "npc param", "play",
         "town NPC action (codes 29 and 158 are merchants)"),
    52: ("npc_walk_pos", "ccEvPcMovePos", "npc x y z", "play", "NPC walks to (x, y, z) * 10"),
    53: ("npc_walk_dir", "ccEvPcMoveRD", "npc rot dist", "play",
         "NPC walks dist * 10 towards rot degrees"),
    54: ("npc_walk_marker", "ccEvPcMoveMarker", "npc marker", "play", "NPC walks to the marker"),
    55: ("npc_walk_char", "ccEvPcMoveChar", "npc type code rot dist", "play",
         "NPC walks to a point rot/dist from a character"),
    56: ("npc_put_marker", "ccEvPcPos", "npc marker", "play", "NPC placed at the marker"),
    57: ("npc_put", "ccEvPcPosDirect", "npc x y z", "play", "NPC placed at (x, y, z) * 10"),
    58: ("npc_turn", "ccEvPcRot", "npc dirc chg", "play", "NPC turns to dirc degrees"),
    59: ("npc_face", "ccEvPcRotChar", "npc type code chg", "play",
         "NPC turns to face a character"),
    60: ("pc_act", "ccEvPcAct", "pc act", "play",
         "party character under manual AI with remote command act (9-way switch)"),
    61: ("pc_walk_pos", "ccEvPcMovePos", "pc x y z", "play",
         "party character walks to (x, y, z) * 10 (remote command 1)"),
    62: ("pc_walk_dir", "ccEvPcMoveRD", "pc rot dist", "play",
         "party character walks towards rot degrees"),
    63: ("pc_walk_marker", "ccEvPcMoveMarker", "pc marker", "play",
         "party character walks to the marker"),
    64: ("pc_walk_char", "ccEvPcMoveChar", "pc type code rot dist", "play",
         "party character walks to a point rot/dist from a character"),
    65: ("pc_mode", "ccEvPcAct", "pc param", "play",
         "ccSpcManager entry word +8 = param (pc -3: whole party, -1/-2: party slot)"),
    66: ("pc_command", "ccEvPcAct", "pc on", "play",
         "ccEntryCmnd / ccDeleteCmnd for the party character"),
    67: ("pc_put_marker", "ccEvPcPos", "pc marker", "play",
         "party character placed at the marker, facing"),
    68: ("party_put_marker", "ccEvPcPos", "pc marker", "play",
         "the companions (party slots 1, 2) placed at the marker (inference)"),
    69: ("party_put", "ccEvPcPosDirect", "pc x y z", "play",
         "the companions placed at (x, y, z) * 10 (inference)"),
    70: ("pc_put", "ccEvPcPosDirect", "pc x y z", "play",
         "party character placed at (x, y, z) * 10"),
    71: ("pc_turn", "ccEvPcRot", "pc dirc chg", "play", "party character turns to dirc degrees"),
    72: ("pc_face", "ccEvPcRotChar", "pc type code chg", "play",
         "party character turns to face a character"),
    73: ("pc_use_skill", "ccEvCondition", "pc skill", "play",
         "ccItemSkillCompel(pc, pc, skill, 0)"),
    74: ("enemy_put", "ccEvPcPos", "enemy posnum", "play",
         "enemy placed at event position posnum (see set_pos)"),
    75: ("affect_on", "ccEvAffect", "type code bit", "play", "character's affectMask |= 1 << bit"),
    76: ("affect_off", "ccEvAffect", "type code bit", "play",
         "character's affectMask &= ~(1 << bit)"),
    77: ("party_add", "ccEvMember", "pc", "play", "ccParty::AddMember, and the menu face"),
    78: ("party_remove", "ccEvMember", "pc", "play",
         "ccParty::DelMember (pc -3: everyone, -1/-2: that slot)"),
    79: ("member_add", "ccEvMember", "pc", "flag", "partyMemberFlag and partyMemberExp bit pc"),
    80: ("member_add_msg", "ccEvMember", "pc", "split",
         "as member_add; when playing, SE 74 and an info box with the name (MUT+: the name "
         "through the 21-character spcParam accessor)"),
    81: ("call_on", "ccEvMember", "pc", "flag",
         "partyMemberCall bit pc (and the stored copy while calls are locked)"),
    82: ("call_on_later", "ccEvMember", "pc", "flag",
         "partyMemberCall bit pc, or only the stored copy while locked"),
    83: ("call_off", "ccEvMember", "pc", "flag",
         "clear partyMemberCall bit pc (and in the stored copy while locked)"),
    84: ("call_lock", "", "", "flag",
         "partyMemberCallStore = partyMemberCall; set bit 31, clear bits 0-17 (MUT+: 0-20)"),
    85: ("call_unlock", "", "", "flag", "partyMemberCall = partyMemberCallStore; store = 0"),
    86: ("exp_on", "ccEvMember", "pc", "flag", "partyMemberExp bit pc"),
    87: ("exp_off", "ccEvMember", "pc", "flag", "clear partyMemberExp bit pc"),
    88: ("save_party", "", "", "play", "partyMemberSave = the current party's bits"),
    89: ("gate_add", "ccEvAreaCodeAdd", "area", "flag",
         "story area into its server's Chaos Gate list (SetGateList), its words into wordList"),
    90: ("gate_add_msg", "ccEvAreaCodeAdd", "area", "split",
         "as gate_add; when playing, an info box with the address"),
    91: ("gate_mark", "ccEvAreaCodeAdd", "area", "flag", "gateListMark[server] bit area"),
    92: ("gate_unmark", "ccEvAreaCodeAdd", "area", "flag", "clear gateListMark[server] bit area"),
    93: ("item_add", "ccEvAddItem", "pc category id num", "split",
         "ccSaveData::AddItem; when playing, for a companion (pc not 0) and a category below 10 "
         "whose character GetSpc finds, through the menu (AddSpcItem) instead"),
    94: ("item_add_menu", "ccEvAddItem", "pc category id num", "split",
         "when playing, the item-get menu (29) first; then AddItem"),
    95: ("item_del", "ccEvAddItem", "pc category id num", "flag", "ccSaveData::DelItem"),
    96: ("gold_add", "ccEvAddGold", "pc num", "flag",
         "spcParam[pc].base.gold += num, clamped to 0..9,999,999 (MUT+: pc 18-20 in the "
         "save extension)"),
    97: ("noise2", "", "", "play", "ccMenu.interNoiz = 2 (interference effect)"),
    98: ("noise1", "", "", "play", "ccMenu.interNoiz = 1"),
    99: ("noise3", "", "", "play", "ccMenu.interNoiz = 3 (INF; see VARIANTS for MUT+)"),
    100: ("scene", "ccEvScene", "area town field dungeon floor block", "play",
          "ccGame::ChangeScene"),
    101: ("mode", "ccEvChangeReq", "num", "play",
          "ccGame::ChangeRequest(num, 7); num 5 (field game): ChangeRequest(5, 8) and "
          "ChangeArea(0, lastTown) (MUT+: always ChangeRequest(num, 7))"),
    102: ("wallpaper_add", "ccEvDtList", "num", "flag",
          "desktop wallpaper num (dtWallpaperList bit)"),
    103: ("bgm_add", "ccEvDtList", "num", "flag", "desktop BGM num (dtBgmList bit)"),
    104: ("bbs_post", "ccEvBbs", "thread post", "split",
          "playing: post appears (bbsList 1) if absent; replaying: marked 3"),
    105: ("bbs_remove", "ccEvBbs", "thread post", "flag", "bbsList[thread][post] = 0"),
    106: ("bbs_post7", "ccEvBbs", "thread post", "split",
          "playing: bbsList 7 if absent; replaying: 3"),
    107: ("mail", "ccEvMail", "mail", "split",
          "playing: ccSaveData::NewMail if never received; replaying: ReadNewMail"),
    108: ("mail_vol", "ccEvMail", "mail", "split",
          "as mail, only while saveData.clearFlag <= this event's volume index"),
    109: ("mail_member", "ccEvMailMemSave", "mail pc", "split",
          "as mail, only if pc is in partyMemberSave"),
    110: ("mail_remove", "ccEvMail", "mail", "flag", "mailList[mail] = 0"),
    111: ("news_add", "ccEvNews", "news", "flag", "webnewsList[news] = 1 if 0"),
    112: ("news_remove", "ccEvNews", "news", "flag", "webnewsList[news] = 0"),
    113: ("frame_rate", "ccEvFrameRate", "rate", "play", "ccSystem::SetFrameRate(rate)"),
    114: ("overlay", "ccEvOverlay", "num", "play",
          "ccLoadOverlay: 0 DEMO, 1 DESKTOP, 2 TOPPAGE, 3 GCMN"),
    115: ("crisis", "ccEvNum", "num", "flag", "saveData.crisis = num != 0"),
    116: ("set_point", "ccEvSetPoint", "floor block evnum", "play", "ccEvent::SetEventPoint"),
    117: ("set_pos", "ccEvSetPos", "floor block posnum dirc x y z", "play",
          "ccEvent::SetEventPos: event position posnum = (x, y, z) * 10, dirc in degrees"),
    118: ("area", "ccEvAreaCodeAdd", "area", "play",
          "areas 1-13: leave the field (WORLD_MAN::Quit) and set eventAreaNumber; others: "
          "SimGenerateCode from its words (MUT+: Quit only while game+0x14 is set; area 126 "
          "only sets eventAreaNumber)"),
    119: ("clear_operate", "", "", "play", "eventMng.operateSet = -1, operateTarget = 0"),
    120: ("clear_area_code", "", "", "play", "eventMng.areaCodeSet[0..2] = -1"),
    121: ("map_on", "", "", "play", "ccMenu.mapStatus = 1"),
    122: ("show_map", "", "", "play", "SE 97, then WORLD_MAN::ShowMap until it returns true"),
    123: ("remove_trap", "", "", "play",
          "SE 228, effRemoveTrap at the command target, 20 frames, affect 12 on it from the player"),
    124: ("virus_core", "ccEvVirus", "area", "replay",
          "regenerate the area, protectArea bit area, delete its protect items (category 15) from"
          " Kite"),
    125: ("town_move", "ccEvTown", "town", "flag", "townMoveFlag bit town"),
    126: ("data_drain", "", "", "flag",
          "saveData.plcol = 1; AddSkill(0, 2): Kite gets skill 2 (Data Drain)"),
    127: ("prev_room", "", "", "play", "WORLD_MAN::GoPrevRoom"),
    128: ("area_ban", "ccEvRoomBan", "field dungeon floor block", "flag",
          "ccSaveData::SetAreaBan"),
    129: ("area_unban", "ccEvRoomBan", "field dungeon floor block", "flag",
          "ccSaveData::ClearAreaBan"),
    130: ("room", "ccEvRoom", "floor block", "play", "WORLD_MAN::RoomSelect(floor, block)"),
    131: ("room_point", "ccEvPointNum", "num", "play", "RoomSelect to event point num's room"),
    132: ("hold", "ccEvTarget", "type code", "play", "start the ccThEvHold task on the character"),
    133: ("hold_end", "", "", "play", "stop the ccThEvHold task"),
    135: ("pc_tint", "ccEvSpcCol", "pc r g b rate", "play",
          "party character colour (affectColor rgb, rate, fixed)"),
    136: ("pc_tint_off", "ccEvMember", "pc", "play", "party character colour off"),
    137: ("piros_colour", "ccEvPiroCol", "code", "play",
          "if Piros (8) is present: a flash, SE 74 and info sequence chosen by eventStatus[1] "
          "(14-way switch)"),
    138: ("status_set", "ccEvExecStatus", "index num", "flag",
          "saveData.eventStatus[index] = num"),
    139: ("status_add", "ccEvExecStatus", "index num", "flag", "eventStatus[index] += num"),
    140: ("status_sub", "ccEvExecStatus", "index num", "flag", "eventStatus[index] -= num"),
    141: ("protect", "ccEvAreaCodeAdd", "area", "flag", "saveData.protectArea bit area"),
    142: ("talk_num", "ccEvTalkNum", "pc num", "flag",
          "saveData.talkNum[pc] = num (0 for pc 1 in Parody Mode; MUT+: pc 18-20 in the save "
          "extension)"),
    143: ("regist_npc16", "", "", "play", "eventMng.registNpcNum = 16"),
    144: ("marker_pos", "ccEvMarker2Pos", "marker posnum", "play",
          "event position posnum = the marker's position (SetEventPos)"),
    145: ("fade", "ccEvFade", "count alpha", "play",
          "screen fade (ccScFade::EntryFade) over count frames to alpha (MUT+: ccScFade::Init "
          "first)"),
    146: ("fade_more", "ccEvFade", "count alpha", "play",
          "ccScFade::ContinueFade on that fade (MUT+: a new EntryFade when the stored fade is "
          "not 0-3)"),
    147: ("staff_roll", "", "", "play",
          "run the desktop staff roll to its end, 30 frames, 30 fps, then (not Parody) desktop menu 8 "
          "(OUT: also wallpapers 57-66 with an info box each; QUA: 77-86)"),
    148: ("clear_count", "", "", "play", "saveData.clearFlag += 1 (not in Parody Mode)"),
    149: ("friendship", "ccEvExecFriendship", "pc num", "flag",
          "ccSaveData::AddFriendship(pc, num)"),
    150: ("menu", "ccEvMenuReq", "num", "play", "open menu num and wait for it to close"),
    151: ("condition_fx_on", "", "", "play", "ccSpcConditionEffectON"),
    152: ("condition_fx_off", "", "", "play", "ccSpcConditionEffectOFF"),
    153: ("player_skill", "", "", "play",
          "the player uses a skill on the command target (ccSkillCheck, ccSkillRequest)"),
    154: ("target_forbid", "ccEvNum", "num", "play", "ccMenu.targetForbid = num != 0"),
    155: ("last_town", "ccEvTown", "town", "flag", "saveData.lastTown = town (low byte)"),
    156: ("name_entry", "", "", "play", "the desktop's NameEntry_Control until done"),
    157: ("gate_hack_anim", "", "", "play", "menus off, wait while ccCheckGtHackAnm"),
    158: ("open_door", "", "", "play", "DUNGEON::OpenDoor in the current room"),
    159: ("close_door", "", "", "play", "DUNGEON::CloseDoor2 in the current room"),
    160: ("sound", "ccEvSoundCtrl", "cmd p0 p1 p2", "play", "ccSndEvRequest(cmd, p0, p1, p2)"),
    161: ("trans_off", "ccEvTarget", "type code", "play", "character's transDist = 0"),
    162: ("trans_on", "ccEvTarget", "type code", "play", "character's transDist = 1"),
    163: ("battle_ready", "", "", "play", "game.inBattleDist = -1.0; AddOperate(14, 0, 0)"),
    164: ("pc_run_pos", "ccEvPcMovePos", "pc x y z", "play",
          "party character runs to (x, y, z) * 10 (remote command 2)"),
    165: ("add_operate_sf", "ccEvOperate", "num except", "play",
          "as add_operate over operations 19-28 (AddOperate sf 1)"),
    166: ("del_operate_sf", "ccEvOperate", "num except", "play",
          "as del_operate over operations 19-28"),
    167: ("desktop_item", "ccEvDtItem", "type id", "split",
          "wallpaper (0), BGM (1) or movie (2) id for the desktop; when playing, SE 74 and an "
          "info box (MUT+: nothing for a movie in Parody Mode)"),
    # Added by later volumes (the jump table grows: 169 cases in MUT, 170 in OUT
    # and QUA; OUT's 169 is the default, a no-op like 134).
    168: ("grunty_mail", "", "", "play",
          "MUT+: if mail 324 was never received and some town 1-4 has all three Grunty types "
          "adult (ccPgAdultCheck(town, 0..2) >= 0), ccSaveData::NewMail(324)"),
    169: ("ending_kanji", "", "", "play",
          "QUA: game+0x7c = 1; reads \\DATA\\KFED.BIN and \\DATA\\KFAED.BIN, adds the ending.ccs "
          "file list, runs a new gcmn task (gcmn 0x004f0a30 starts thread 0x004f0950) until it "
          "ends, frees both files"),
}

# A code whose case takes a different number of operands in some volume:
# (code, operand count) -> row. The executable's measured count picks the row.
VARIANTS = {
    (99, 1): ("noise", "", "level", "play",
              "MUT+: ccMenu.interNoiz = level + 2 for level 1-3, otherwise 0 (off)"),
}
# 134's jump table entry is the loop head itself: a two-byte no-op.
NOP = 134

# CheckOpen conditions: tag -> (name, operand struct, fields, test). comp 0 is
# ==, 1 is >=, 2 is <= (the game's value against the operand).
CONDS = {
    1: ("event_done", "ccEvFlagGrp", "event",
        "eventFlag[event] bit 62 (end_event -2, or bit 63 promoted by ccStartThEvent)"),
    2: ("block_done", "ccEvFlagLocal", "num", "bit num of this event's flag word"),
    3: ("phase", "ccEvPhase", "phase comp", "eventMng.enablePhase against phase"),
    4: ("game_status", "ccEvGstatus", "status", "game.status == status (the mode)"),
    5: ("in_point", "ccEvPointNum", "num", "game.floor/block are event point num's"),
    6: ("scene", "ccEvScene", "area town field dungeon floor block", "all six of game's equal"),
    7: ("in_town", "ccEvTown", "town", "game.area 0 and game.town == town"),
    8: ("in_field", "ccEvField", "town field", "game.area 1, town and field"),
    9: ("in_dungeon", "ccEvDungeon", "town field dungeon", "game.area 2, town, field and dungeon"),
    10: ("status", "ccEvOpenStatus", "index num comp", "saveData.eventStatus[index] against num"),
    11: ("status_range", "ccEvOpenStatus", "index lo hi", "lo <= eventStatus[index] <= hi"),
    12: ("gate_words", "ccEvAreaCode", "area except",
         "the words at the gate (areaCodeSet) are the area's on its server; except 1 negates"),
    13: ("talked_to", "ccEvTarget", "type code",
         "operateSet == 9 (talk) and the character talked to (operateTarget) is type/code"),
    14: ("operate", "ccEvOperate", "num except",
         "the operation the player tried (operateSet) is num (-1: one not intercepted); except 1 "
         "negates"),
    15: ("near_marker", "ccEvMarker", "marker bounds comp",
         "player-to-marker distance against bounds * 10"),
    16: ("answer", "ccEvSelect", "msg select",
         "the last question was message msg and the answer was select"),
    17: ("bbs_read", "ccEvBbs", "thread post", "bbsList[thread][post] == 3"),
    18: ("mail_got", "ccEvMail", "mail", "mailList[mail] is 4, 5 or 6"),
    19: ("mail_4", "ccEvMail", "mail", "mailList[mail] == 4"),
    20: ("mail_5", "ccEvMail", "mail", "mailList[mail] == 5"),
    21: ("mail_6", "ccEvMail", "mail", "mailList[mail] == 6"),
    22: ("news_read", "ccEvNews", "news", "webnewsList[news] == 3"),
    23: ("in_party", "ccEvMember", "pc", "pc is in the party (pc -1: the party has 2 or more)"),
    24: ("not_in_party", "ccEvMember", "pc", "pc is not in the party (pc -1: fewer than 2)"),
    25: ("in_party_of_2", "ccEvMember", "pc", "the party has exactly 2 and pc is one"),
    26: ("party_other", "ccEvMember", "pc",
         "counts party slots 1-2 holding someone else (can count twice)"),
    27: ("callable", "ccEvMember", "pc", "partyMemberCall bit pc"),
    28: ("present", "ccEvTarget", "type code",
         "type 2: in the SPC manager; 5/6: in the entry list; 20: in the gimmick list; 7: the "
         "boss (bossEntry) whose task param is still 0"),
    29: ("absent", "ccEvTarget", "type code",
         "the negation of present (7: the boss task param is non-zero)"),
    30: ("no_active", "", "", "ccCheckActiveObject() returns non-zero and no gimmick is active"),
    31: ("no_entries", "", "", "g_entCtrl.enNum == 0 and mcNum == 0"),
    32: ("no_menu", "", "", "ccMenuCtrl::CheckMenuType() == -1"),
    33: ("has_item", "ccEvItem", "pc category id num comp",
         "item (category, id) count in pc's list against num; category 15 is impItemList (MUT+: "
         "pc 18-20 in the save extension)"),
    34: ("friendship", "ccEvOpenFriendship", "pc num comp",
         "spcParam[pc].friendship against num (MUT+: pc 18-20 in the save extension)"),
    35: ("pad", "ccEvInput", "mask", "ccSys.pad[0].push & mask"),
    36: ("enemy_pp", "ccEvTarget", "type enemy",
         "GetEnemy(enemy) param PPcount != 0 (type unused)"),
    37: ("member", "ccEvMember", "pc", "partyMemberFlag bit pc"),
    38: ("member_saved", "ccEvMember", "pc", "partyMemberSave bit pc"),
    39: ("volume", "ccEvNumComp", "num comp", "volumeNum against num"),
    40: ("in_room", "ccEvRoom", "floor block", "game.floor and game.block"),
}
COMPARE = {0: "==", 1: ">=", 2: "<="}

# SetCurrentOpen tags, after a -2: tag -> (name, struct, fields, meaning). They
# stay set for the rest of the event (eventSub clears them once, when the
# open conditions pass) and are checked before each block's own conditions.
CURRENTS = {
    0: ("none", "", "", "nothing"),
    1: ("none", "", "", "nothing"),
    2: ("block_done", "ccEvFlagLocal", "num", "this event's flag bit num"),
    3: ("phase", "ccEvPhase", "phase comp", "eventMng.enablePhase against phase"),
    4: ("game_status", "ccEvGstatus", "status",
        "game.status; resets the scene to any unless status 5"),
    5: ("in_point", "ccEvPointNum", "num", "the room of event point num"),
    6: ("scene", "ccEvScene", "area town field dungeon floor block", "the scene (-1 = any)"),
    7: ("in_town", "ccEvTown", "town", "area 0, the town"),
    8: ("in_field", "ccEvField", "town field", "area 1"),
    9: ("in_dungeon", "ccEvDungeon", "town field dungeon", "area 2"),
    10: ("status", "ccEvOpenStatus", "index num comp", "eventStatus[index] against num"),
    11: ("status_range", "ccEvOpenStatus", "index lo hi", "lo <= eventStatus[index] <= hi"),
}


MSG_OPS = {6: "msg", 8: "msg", 9: "msg"}
TEACH_MSGS = {37: (7, 55), 38: (9, 56), 39: (11, 57)}   # both camCtrlType variants
PC_SPECIAL = {-1: "slot1", -2: "slot2", -3: "party"}
CHAR_RECORD = 92              # charTbl entry size; ConvGame copies base parameters from it
PARODY_FLAG = 0x842b          # ccSaveData.parodyFlag, the same offset in all four volumes
VOICE_FLAG = 0x842c           # ccSaveData.voice: 0 Japanese, otherwise English


def fields(spec):
    return spec.split() if spec else []


def script_name(event):
    """Infection's source name for event N's array - eventTblM101 for event 1,
    eventTblS100 for 50, eventTblML006 for 406 - used for every volume."""
    g = event // 50
    if g < len(GROUPS):
        return f"eventTbl{GROUPS[g]}{event % 50:02d}"
    return f"eventTblML{event - 400:03d}"


# ---------------------------------------------------------------------------
# The game's switches, and what each code of one executable is.

class Switch:
    """One dispatch switch, read from its function's code: `sltiu $at, rX,
    COUNT`, the branch to DEFAULT when out of range, the table's lui/addiu,
    `jr`. A code whose case is DEFAULT, or that is COUNT or more, has no case:
    the game steps over it alone."""

    def __init__(self, prog, name, limit=64):
        sym = prog.symbol_named(name)
        if sym is None:
            raise SystemExit(f"{prog.path}: no {name} (a stripped volume needs the sidecar "
                             "piney-gen syms writes)")
        self.name = name
        self.fn = sym.value
        self.count = self.default = self.table = None
        hi = {}
        for i in range(limit):
            va = self.fn + 4 * i
            ins = mips.decode(prog.u32(va), va)
            w = ins.word
            if self.count is None:
                if w >> 26 == 0x0b and ins.rt == 1:          # sltiu $at, rX, COUNT
                    self.count = w & 0xffff
                continue
            if self.default is None and ins.cond:
                self.default = ins.target
            elif w >> 26 == 0x0f:
                hi[ins.rt] = (w & 0xffff) << 16
            elif w >> 26 == 0x09 and ins.rs in hi:
                self.table = (hi[ins.rs] + ((w & 0xffff) ^ 0x8000) - 0x8000) & 0xffffffff
            elif ins.mnemonic == "jr" and self.table is not None:
                break
        else:
            raise SystemExit(f"{prog.path}: no jump table in the first {limit} instructions "
                             f"of {name}")
        self.cases = [prog.u32(self.table + 4 * i) for i in range(self.count)]

    def has(self, code):
        return 0 <= code < self.count and self.cases[code] != self.default


class Table:
    """One switch of one executable: `nargs[code]` is the operand count (shorts)
    of every code the game gives a case (measured by running it), and `row`
    names it from the documented tables when one fits that count."""

    def __init__(self, title, prefix, switch, doc, nargs, variants=None):
        self.title = title
        self.prefix = prefix
        self.switch = switch
        self.doc = doc
        self.nargs = nargs
        self.variants = variants or {}

    @staticmethod
    def _row(r):
        if len(r) == 5:
            return r[0], r[1], fields(r[2]), r[3], r[4]
        return r[0], r[1], fields(r[2]), "", r[3]

    def documented(self, code):
        n = self.nargs[code]
        doc = self.doc.get(code)
        if doc is not None and len(fields(doc[2])) == n:
            return self._row(doc)
        v = self.variants.get((code, n))
        return self._row(v) if v is not None else None

    def row(self, code):
        """(name, struct, [fields], runs, effect) for a code with a case."""
        r = self.documented(code)
        if r is not None:
            return r
        n = self.nargs[code]
        return (f"{self.prefix}_{code}", "", [f"a{i}" for i in range(n)], "",
                f"not documented: the game's case takes {n} operands")

    def args(self, code):
        """Operand shorts after the code, or None when the game has no case."""
        return self.nargs.get(code)


# ---------------------------------------------------------------------------
# Parsing: a script is walked exactly as eventSub walks it.

class Insn:
    __slots__ = ("off", "kind", "code", "args")

    def __init__(self, off, kind, code, args):
        self.off = off        # byte offset of the opcode short in the array
        self.kind = kind      # cond, current, op, nop, or unknown-cond/-current/-op
        self.code = code
        self.args = args


class Block:
    def __init__(self):
        self.currents = []
        self.conds = []
        self.ops = []
        self.cond_end = self.end = None   # byte offsets of the terminating 0s


class Script:
    def __init__(self, name, va, shorts):
        self.name = name
        self.va = va
        self.shorts = shorts
        self.header = []
        self.header_end = None
        self.blocks = []
        self.end = None           # byte offset of the -1
        self.errors = []

    @property
    def clean(self):
        """Walked without error to a -1 that is the array's last short."""
        return not self.errors and self.end == 2 * (len(self.shorts) - 1)


def parse(name, va, shorts, spec):
    """`spec` has the executable's three Tables as .ops, .conds, .currents."""
    s = Script(name, va, shorts)
    n = len(shorts)
    pos = [0]

    def take(k):
        i = pos[0]
        if i + k > n:
            raise IndexError(f"runs past the end of the array at +0x{2 * i:x}")
        pos[0] = i + k
        return shorts[i:i + k]

    def conds(out):
        while True:
            off = 2 * pos[0]
            (tag,) = take(1)
            if tag == 0:
                return off
            k = spec.conds.args(tag)
            if k is not None:
                out.append(Insn(off, "cond", tag, take(k)))
            else:
                out.append(Insn(off, "unknown-cond", tag, ()))   # the game skips it

    try:
        s.header_end = conds(s.header)
        for _ in range(BLOCKS):
            if shorts[pos[0]] == END:
                s.end = 2 * pos[0]
                pos[0] += 1
                break
            b = Block()
            while shorts[pos[0]] == CURRENT:
                off = 2 * pos[0]
                tag = take(2)[1]
                k = spec.currents.args(tag)
                if k is not None:
                    b.currents.append(Insn(off, "current", tag, take(k)))
                else:
                    # SetCurrentOpen jumps nowhere for a tag past its table and returns tbl + 4.
                    b.currents.append(Insn(off, "unknown-current", tag, ()))
            b.cond_end = conds(b.conds)
            while True:
                off = 2 * pos[0]
                (op,) = take(1)
                if op == 0:
                    b.end = off
                    break
                k = spec.ops.args(op)
                if k is not None:
                    b.ops.append(Insn(off, "op", op, take(k)))
                elif 0 < op < spec.ops.switch.count:
                    b.ops.append(Insn(off, "nop", op, ()))      # a table entry with no case
                else:
                    b.ops.append(Insn(off, "unknown-op", op, ()))
            s.blocks.append(b)
        if s.end is None:
            s.errors.append(f"no -1 after {BLOCKS} blocks")
        elif pos[0] != n:
            s.errors.append(f"-1 at +0x{s.end:x} but the array has {n} shorts")
    except IndexError as e:
        s.errors.append(str(e))
    return s


# ---------------------------------------------------------------------------
# Running the game's code: CheckOpen, SetCurrentOpen and Execute in
# tools/eemu.py with lv 0, which is how the game steps over blocks it does not
# run; ccEvVoiceRequest and evVoicePlay for the voice.

SCRATCH = 0x01800000     # ccEvent (0x7e0 bytes in Infection), eot pointer, script copy
EOT = SCRATCH + 0x1100
CODE = SCRATCH + 0x2000
SAVE = 0x01900000        # a scratch ccSaveData (0x8530 bytes)
SND = 0x01920000         # a scratch ccSnd


class GameWalker:
    def __init__(self, program, execute, checkopen, setcurrent):
        import eemu
        self.m = eemu.Machine(program)
        self.execute = execute
        self.checkopen = checkopen
        self.setcurrent = setcurrent

    def call(self, fn, eot, grp=0, ev=0, lv=0):
        """fn(this, &eot, grp, ev, lv) with eot at `eot`; returns (v0, new eot)."""
        m = self.m
        m.store(EOT, 4, eot)
        v0 = m.call(fn, (SCRATCH, EOT, grp & 0xffffffff, ev & 0xffffffff, lv & 0xffffffff))
        return v0, m.load(EOT, 4)

    def set_current(self, eot):
        return self.m.call(self.setcurrent, (SCRATCH, eot))

    def load(self, shorts):
        raw = struct.pack(f"<{len(shorts)}h", *shorts)
        self.m.mem[CODE:CODE + len(raw)] = raw
        return CODE

    def measure(self, fn, code, lv=0):
        """Bytes one opcode's case advances eot past the opcode, at lv."""
        va = self.load([code] + [0] * 32)
        _, eot = self.call(fn, va, lv=lv)
        return eot - va - 4    # the opcode, its operands, then the 0 that ends the run

    def measure_current(self, tag):
        va = self.load([CURRENT, tag] + [0] * 16)
        return self.set_current(va) - va - 4

    def walk(self, shorts):
        """eventSub's loop over a whole script at lv 0: the byte offset where each
        CheckOpen and Execute run stops, and the offset of the -1."""
        base = self.load(list(shorts))
        ends = []
        _, eot = self.call(self.checkopen, base, ev=-1)
        ends.append(eot - base - 2)
        for _ in range(BLOCKS):
            if self.m.load(eot, 2, True) == END:
                return ends, eot - base
            while self.m.load(eot, 2, True) == CURRENT:
                eot = self.set_current(eot)
            _, eot = self.call(self.checkopen, eot)
            ends.append(eot - base - 2)
            _, eot = self.call(self.execute, eot)
            ends.append(eot - base - 2)
        return ends, None


# ---------------------------------------------------------------------------
# The game's tables.

class Events:
    """Switches, scripts, event numbers, messages and voice, read from the
    executable."""

    def __init__(self, elf_path):
        import eemu
        self.elf_path = elf_path
        self.p = p = Program(elf_path)
        self.execute = Switch(p, EXECUTE)
        self.checkopen = Switch(p, CHECKOPEN)
        self.setcurrent = Switch(p, SETCURRENT)
        self.walker = GameWalker(p, self.execute.fn, self.checkopen.fn, self.setcurrent.fn)
        try:
            self.ops, self.conds, self.currents = self.measure()
        except eemu.Stop as e:
            raise SystemExit(f"{elf_path}: measuring the cases stopped: {e}") from None
        self.tables = (self.ops, self.conds, self.currents)
        sym = p.symbol_named("volumeNum")
        self.volume = p.u32(sym.value) if sym else None

        tbl = p.symbol_named("eventTbl").value
        groups = [p.u32(tbl + 4 * g) for g in range(10)]
        entries = {}
        for g, base in enumerate(groups):
            for i in range(50 if base else 0):
                script, label = struct.unpack("<II", p.read(base + 8 * i, 8))
                if script:
                    entries[50 * g + i] = (script, label)
        # A script runs to the next script or group array, less the zero
        # padding that 16-byte alignment leaves (its last short is the -1).
        starts = sorted({tbl, *groups, *(v for v, _ in entries.values())} - {0})
        self.events = {}      # event number -> (script name, event name)
        self.bounds = {}      # script name -> (va, shorts)
        for event, (va, label) in sorted(entries.items()):
            i = bisect.bisect_right(starts, va)
            end = starts[i] if i < len(starts) else va + 0x10000
            shorts = list(struct.unpack(f"<{(end - va) // 2}h", p.read(va, end - va)))
            while shorts and shorts[-1] == 0:
                shorts.pop()
            name = script_name(event)
            self.events[event] = (name, self.cstr(label))
            self.bounds[name] = (va, len(shorts))
        self.by_name = {v[0]: k for k, v in self.events.items()}
        self.names = sorted(self.by_name, key=lambda n: self.bounds[n][0])
        self.chars = self.char_names()
        self._overlays = {}
        self._areas = None
        self._msg_starts = None
        self._voice = {}

    def measure(self):
        """Every case's operand count, from the executable's own code at lv 0."""
        g = self.walker
        ops = {c: g.measure(self.execute.fn, c) // 2
               for c in range(1, self.execute.count) if self.execute.has(c)}
        conds = {c: g.measure(self.checkopen.fn, c) // 2
                 for c in range(1, self.checkopen.count) if self.checkopen.has(c)}
        # Every SetCurrentOpen tag in range is a setting, the default ones empty.
        currents = {t: g.measure_current(t) // 2 for t in range(self.setcurrent.count)}
        return (Table("Execute", "op", self.execute, OPS, ops, VARIANTS),
                Table("CheckOpen", "cond", self.checkopen, CONDS, conds),
                Table("SetCurrentOpen", "tag", self.setcurrent, CURRENTS, currents))

    def cstr(self, va):
        return self.p.cstr(va, 512).decode("shift_jis", "replace") if va else ""

    def script(self, name):
        va, n = self.bounds[name]
        return parse(name, va, struct.unpack(f"<{n}h", self.p.read(va, 2 * n)), self)

    def resolve(self, what):
        """A script name or an event number -> script name."""
        if what in self.by_name:
            return what
        if not what.startswith("eventTbl") and "eventTbl" + what in self.by_name:
            return "eventTbl" + what
        try:
            return self.events[int(what, 0)][0]
        except (ValueError, KeyError):
            raise SystemExit(f"no script or event {what}") from None

    def char_names(self):
        """charTbl's names (demo.prg in INF and MUT, main from OUT on): the
        symbol when there is one, else the table ConvGame copies from, the
        first address its code builds."""
        try:
            d = Program(self.elf_path, "demo")
        except (OSError, ValueError, KeyError):
            return ()
        sym = d.symbol_named("charTbl")
        if sym is not None:
            base = sym.value
        else:
            f = d.symbol_named("ConvGame__10ccSaveDataFv")
            if f is None:
                return ()
            from xfer import addresses
            n = max(f.size, 64) // 4
            built = addresses(struct.unpack(f"<{n}I", d.read(f.value, 4 * n)), f.value, d.gp)
            if not built:
                return ()
            base = built[min(built)]
        names = []
        for i in range(64):
            ptr = d.u32(base + CHAR_RECORD * i)
            if not ptr or not d.mapped(ptr):
                break
            names.append(d.cstr(ptr).decode("latin-1"))
        return tuple(names)

    # messages ------------------------------------------------------------
    def message_count(self, va):
        """Records in the message array at va: to the next array or table, less
        trailing all-zero records (alignment padding)."""
        p = self.p
        if self._msg_starts is None:
            starts = set()
            for table in ("evMsgTbl", "evMsgTblp"):
                t = p.symbol_named(table).value
                starts.add(t)
                for g in range(10):
                    grp = p.u32(t + 4 * g)
                    if not grp:
                        continue
                    starts.add(grp)
                    starts.update(a for a in (p.u32(grp + 4 * i) for i in range(50)) if a)
            self._msg_starts = sorted(starts)
        i = bisect.bisect_right(self._msg_starts, va)
        end = self._msg_starts[i] if i < len(self._msg_starts) else va + 12 * 256
        n = (end - va) // 12
        while n and p.read(va + 12 * (n - 1), 12) == bytes(12):
            n -= 1
        return n

    def messages(self, event, parody=False):
        """[(emode, name, [lines])] of evMsgTbl[event / 50][event % 50]."""
        p = self.p
        tables = p.symbol_named("evMsgTblp" if parody else "evMsgTbl").value
        grp = p.u32(tables + 4 * (event // 50))
        if not grp:
            return []
        base = p.u32(grp + 4 * (event % 50))
        if not base:
            return []
        out = []
        for k in range(self.message_count(base)):
            emode, name, text = struct.unpack("<iII", p.read(base + 12 * k, 12))
            lines = [split_line(p, text, j) for j in range(3)] if text else []
            while lines and not lines[-1]:
                lines.pop()
            who = render(p.cstr(name, 128)) if name else ""
            out.append((emode, who, [render(x) for x in lines]))
        return out

    def voice(self, event, msg, english=True, parody=False):
        """(file, ofs, size) that ccEvVoiceRequest(event, msg) and evVoicePlay
        hand to sewordCmd, or None. A file the disc does not have is None too
        (Infection's code has rows for volume 2's side events, in EVVOL2S.BIN,
        which only Mutation's disc carries); that is judged from the ISO image
        beside the disc directory, when there is one."""
        key = (event, msg, english, parody)
        if key not in self._voice:
            v = self._run_voice(event, msg, english, parody)
            files = self.disc_files()
            if v is not None and files is not None and v[0].upper() not in files:
                v = None
            self._voice[key] = v
        return self._voice[key]

    def disc_files(self):
        """Upper-case paths of every file on the disc image work/<volume>/*.iso
        (beside the disc directory), or None when there is no image."""
        if not hasattr(self, "_disc_files"):
            self._disc_files = None
            import glob
            images = sorted(glob.glob(os.path.join(os.path.dirname(os.path.dirname(
                os.path.abspath(self.elf_path))), "*.iso")))
            if images:
                import iso
                img = iso.Iso(images[0])
                try:
                    self._disc_files = {e.path.upper() for e in img.walk() if not e.is_dir}
                finally:
                    img.f.close()
        return self._disc_files

    def _run_voice(self, event, msg, english, parody):
        import eemu
        p = self.p
        syms = [p.symbol_named(n) for n in ("ccEvVoiceRequest__Fii", "evVoicePlay__Fv",
                                            "sewordCmd__Fii", "saveData", "ccSnd")]
        if None in syms:
            return None
        request, play, seword, save, snd = (s.value for s in syms)
        m = self.walker.m
        m.store(save, 4, SAVE)
        m.store(snd, 4, SND)
        m.mem[SAVE:SAVE + 0x8530] = bytes(0x8530)
        m.mem[SAVE + PARODY_FLAG] = int(parody)
        m.mem[SAVE + VOICE_FLAG] = int(english)
        # A ccSnd of -1s: both voice slots free, so ccMesVoicePlay claims one
        # only when ccEvVoiceRequest found a line.
        m.mem[SND:SND + 0x400] = b"\xff" * 0x400
        got = []

        def hook(mm, cmd, bank, *_):
            got.append(bytes(mm.mem[bank:bank + 0x50]))
            return 0
        m.hooks[seword] = hook
        vd = p.symbol_named("vdRequest")
        try:
            m.call(request, (event, msg))
            row = m.load(vd.value, 4) if vd else None
            m.call(play, ())
        except eemu.Stop:
            return None
        finally:
            del m.hooks[seword]
        if not got:
            return None
        # Infection's volume 2-4 tables are empty externs, so its code indexes
        # whatever small data follows them; a row outside the file's data, or
        # an empty one, is no line.
        if row is not None and p.elf.file_offset(row) is None:
            return None
        ofs, size = struct.unpack_from("<ii", got[0])
        if size <= 0:
            return None
        fname = got[0][16:].split(b"\0", 1)[0].decode("latin-1")
        return fname.split(":", 1)[-1].lstrip("\\").replace("\\", "/"), ofs, size

    # names for operands ----------------------------------------------------
    def overlay(self, name):
        if name not in self._overlays:
            try:
                self._overlays[name] = Program(self.elf_path, name)
            except (OSError, ValueError):
                self._overlays[name] = None
        return self._overlays[name]

    def area(self, code):
        if self._areas is None:
            import areas
            self._areas = areas.Data(self.elf_path).events
        # The list is in eventAreaInfo's order, not by code (13 is followed
        # by 15, 16 comes later): look the code up.
        e = next((e for e in self._areas if e["code"] == code), None)
        if e is None:
            return None
        if e["has_words"]:
            return f"{e['wordA']} {e['wordB']} {e['wordC']}, server {e['server']}"
        return "no words"

    def table(self, overlay, name):
        """(program, symbol) of an overlay's data table, or None. A carried
        name can be wrong (Mutation's MailTbl lands in desktop.prg's code), so
        a table inside the overlay's text is refused."""
        d = self.overlay(overlay)
        sym = d.symbol_named(name) if d else None
        if sym is None:
            return None
        ov = d.overlay
        if ov.text <= sym.value < ov.text + ov.text_size:
            return None
        return d, sym

    @staticmethod
    def text_at(d, va):
        return render(d.cstr(va, 128)) if va and d.mapped(va) else None

    def mail_title(self, n):
        hit = self.table("desktop", "MailTbl")
        if hit is None or n < 0 or 0x48 * n >= hit[1].size:
            return None
        d, sym = hit
        return self.text_at(d, struct.unpack("<I", d.read(sym.value + 0x48 * n + 0xc, 4))[0])

    def news_title(self, n):
        hit = self.table("desktop", "HtmlTbl")
        if hit is None or n < 0:
            return None
        d, sym = hit
        for k in range(sym.size // 0x1c):
            no, _, title = struct.unpack("<iiI", d.read(sym.value + 0x1c * k, 12))
            if no == n and title:
                return self.text_at(d, title)
        return None

    def bbs_title(self, thread, post):
        hit = self.table("toppage", "bbsThreadTbl")
        if hit is None or thread < 0 or post < 0 or 12 * thread >= hit[1].size:
            return None
        t, sym = hit
        title, msgs, num = struct.unpack("<IIi", t.read(sym.value + 12 * thread, 12))
        if not title or post >= num or not t.mapped(msgs):
            return None
        ptitle = struct.unpack("<I", t.read(msgs + 0x14 * post + 4, 4))[0]
        a, b = self.text_at(t, title), self.text_at(t, ptitle)
        return f"{a} / {b}" if a and b else None


def split_line(p, text, k):
    """ccKanjiStrSeparate(text, k): skip k NUL-ended lines; a byte below 0x20
    or above 0x7f takes its follower with it."""
    va = text
    for _ in range(k):
        while True:
            c = p.read(va, 1)[0]
            if c == 0:
                va += 1
                break
            va += 1 if 0x20 <= c < 0x80 else 2
    return p.cstr(va, 256)


# ---------------------------------------------------------------------------
# Rendering.

def char_name(ev, v):
    if v in PC_SPECIAL:
        return PC_SPECIAL[v]
    return ev.chars[v] if 0 <= v < len(ev.chars) else None


def format_args(names, args):
    return " ".join(f"{n}={a}" for n, a in zip(names, args))


def describe_cond(ev, table, insn):
    """`name field=value ...`; a comparison shows as `... >= target`, the target
    being the field before comp (the game's value is on the left)."""
    name, _, names, _, _ = table.row(insn.code)
    args = list(insn.args)
    if names[:1] == ["index"]:
        index, x, y = args
        if name == "status_range":
            return f"{name:14} {x} <= eventStatus[{index}] <= {y}"
        return f"{name:14} eventStatus[{index}] {COMPARE.get(y, f'comp{y}')} {x}"
    if names and names[-1] == "comp":
        comp = COMPARE.get(args[-1], f"comp{args[-1]}")
        head = format_args(names[:-2], args[:-2])
        text = f"{name:14} {head + ' ' if head else ''}{comp} {args[-2]}"
    else:
        text = f"{name:14} {format_args(names, args)}".rstrip()
    a = dict(zip(names, args))
    if "pc" in a and char_name(ev, a["pc"]):
        text += f"  ; {char_name(ev, a['pc'])}"
    elif a.get("type") == 2 and "code" in a and char_name(ev, a["code"]):
        text += f"  ; {char_name(ev, a['code'])}"
    return text


class Renderer:
    def __init__(self, ev, event, parody=False, english=True):
        self.ev = ev
        self.event = event
        self.parody = parody
        self.english = english
        self.msgs = ev.messages(event, parody)

    def message(self, k, voice_event=None):
        out = []
        if 0 <= k < len(self.msgs):
            emode, name, lines = self.msgs[k]
            text = " / ".join(lines)
            who = f"{name}: " if name else ""
            # The game tests the low byte; 3 asks a question. The rest is not traced.
            q = " [question]" if emode & 0xff == 3 else f" [emode 0x{emode:x}]" if emode else ""
            out.append(f'{who}"{text}"{q}')
        else:
            out.append("(no text)")
        v = self.ev.voice(self.event if voice_event is None else voice_event, k,
                          self.english, self.parody)
        if v:
            out.append(f"[voice {v[0]} +0x{v[1]:x} {v[2] / 96000:.1f}s]")
        return " ".join(out)

    def op(self, insn):
        if insn.kind == "nop":
            return "nop"
        if insn.kind == "unknown-op":
            return f"op_{insn.code}  ; not an opcode, skipped by the game"
        ev = self.ev
        name, _, names, _, _ = ev.ops.row(insn.code)
        a = dict(zip(names, insn.args))
        text = f"{name:20} {format_args(names, insn.args)}".rstrip()
        notes = []
        if insn.code in MSG_OPS and "msg" in a:
            notes.append(self.message(a["msg"]))
        elif insn.code in TEACH_MSGS and not names:
            k0, k1 = TEACH_MSGS[insn.code]
            notes.append(self.message(k0, voice_event=3))
            notes.append("or " + self.message(k1, voice_event=3))
        if "pc" in a and char_name(ev, a["pc"]):
            notes.append(char_name(ev, a["pc"]))
        elif a.get("type") == 2 and "code" in a and char_name(ev, a["code"]):
            notes.append(char_name(ev, a["code"]))    # type 2 is a party character
        if "area" in a:
            w = ev.area(a["area"])
            if w:
                notes.append(w)
        if "mail" in a:
            t = ev.mail_title(a["mail"])
            if t:
                notes.append(f'"{t}"')
        if "news" in a:
            t = ev.news_title(a["news"])
            if t:
                notes.append(f'"{t}"')
        if "thread" in a:
            t = ev.bbs_title(a["thread"], a["post"])
            if t:
                notes.append(f'"{t}"')
        if insn.code == 1 and "grp" in a:
            g = a["grp"]
            notes.append("this event" if g == -1 else "this event, done" if g == -2 else
                         f"event {g} {ev.events.get(g, ('', ''))[1]}".rstrip())
        return text + ("  ; " + "  ".join(notes) if notes else "")


def disassemble(ev, name, parody=False, english=True):
    s = ev.script(name)
    event = ev.by_name[name]
    label = ev.events[event][1]
    r = Renderer(ev, event, parody, english)
    out = [f"# {name} @ 0x{s.va:08x}, {2 * len(s.shorts)} bytes: event {event} \"{label}\"",
           f"# {len(s.blocks)} blocks, {len(r.msgs)} messages" + ("" if s.clean else
                                                                 "  ** " + "; ".join(s.errors))]

    def line(insn, text):
        raw = " ".join(f"{v & 0xffff:04x}" for v in (insn.code, *insn.args))
        return f"  +{insn.off:04x}  {raw:30} {text}"

    out.append("open:")
    for c in s.header:
        out.append(line(c, cond_text(ev, c)))
    if not s.header:
        out.append("  (always)")
    for i, b in enumerate(s.blocks):
        out.append(f"block {i}:")
        for c in b.currents:
            if c.kind == "current":
                out.append(line(c, "set " + describe_cond(ev, ev.currents, c)))
            else:
                out.append(line(c, f"set tag {c.code}  ; not a tag, skipped"))
        for c in b.conds:
            out.append(line(c, "if " + cond_text(ev, c)))
        for o in b.ops:
            out.append(line(o, r.op(o)))
    if s.end is not None:
        out.append(f"  +{s.end:04x}  ffff{'':26} end")
    return "\n".join(out)


def cond_text(ev, c):
    if c.kind == "unknown-cond":
        return f"cond_{c.code}  ; not a condition, skipped"
    return describe_cond(ev, ev.conds, c)


# ---------------------------------------------------------------------------
# Checking against the game.

def expected_ends(s):
    ends = [s.header_end]
    for b in s.blocks:
        ends += [b.cond_end, b.end]
    return ends


def check(ev, out=sys.stdout):
    """Every case's measured operand count against the documented tables, and
    every script's block ends against the game code walking it."""
    bad = 0
    for t in ev.tables:
        for code in sorted(t.nargs):
            if t.documented(code) is None:
                bad += 1
                doc = t.doc.get(code)
                said = f"; the table says {len(fields(doc[2]))}" if doc else ""
                print(f"{t.title} {code}: the game's case takes {t.nargs[code]} operands{said}",
                      file=out)
    g = ev.walker
    scripts = 0
    for name in ev.names:
        s = ev.script(name)
        ends, end = g.walk(s.shorts)
        scripts += 1
        if ends != expected_ends(s) or end != s.end:
            bad += 1
            print(f"{name}: game stops at {ends} / {end}, "
                  f"parser at {expected_ends(s)} / {s.end}", file=out)
    print(f"{len(ev.ops.nargs)} Execute cases, {len(ev.conds.nargs)} conditions, "
          f"{len(ev.currents.nargs)} current tags and {scripts} scripts checked against the "
          f"game code: {bad} mismatches", file=out)
    return bad


# ---------------------------------------------------------------------------

def survey(ev):
    op_count = collections.Counter()
    cond_count = collections.Counter()
    cur_count = collections.Counter()
    unknown = collections.Counter()
    clean = blocks = insns = 0
    bad = []
    for name in ev.names:
        s = ev.script(name)
        clean += s.clean
        if not s.clean:
            bad.append((name, s.errors))
        blocks += len(s.blocks)
        for c in s.header:
            (cond_count if c.kind == "cond" else unknown)[("cond", c.code)] += 1
        for b in s.blocks:
            for c in b.currents:
                (cur_count if c.kind == "current" else unknown)[("current", c.code)] += 1
            for c in b.conds:
                (cond_count if c.kind == "cond" else unknown)[("cond", c.code)] += 1
            for o in b.ops:
                insns += 1
                (op_count if o.kind == "op" else unknown)[("op", o.code)] += 1
    return {"scripts": len(ev.names), "clean": clean, "blocks": blocks, "insns": insns,
            "ops": op_count, "conds": cond_count, "currents": cur_count,
            "unknown": unknown, "bad": bad}


def print_survey(ev, r):
    print(f"{r['scripts']} scripts; {r['clean']} walk cleanly to a -1 in their last short; "
          f"{len(r['bad'])} desync")
    for name, errors in r["bad"]:
        print(f"  {name}: {'; '.join(errors)}")
    print(f"{r['blocks']} blocks, {r['insns']} instructions, {sum(r['conds'].values())} "
          f"conditions, {sum(r['currents'].values())} current-open settings")
    print(f"unknown opcodes, tags or conditions: {sum(r['unknown'].values())}"
          + ("" if not r["unknown"] else f"  {dict(r['unknown'])}"))
    for title, table, key in (("Execute opcodes", ev.ops, "ops"),
                              ("CheckOpen conditions", ev.conds, "conds")):
        counts = r[key]
        print(f"\n{title}: {len(counts)} of {len(table.nargs)} cases used")
        for (_, code), n in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0])):
            print(f"  {code:3} {table.row(code)[0]:22} {n}")
        never = [c for c in sorted(table.nargs) if (key[:-1], c) not in counts]
        print(f"  never used: {', '.join(f'{c} {table.row(c)[0]}' for c in never)}")
    print("\nSetCurrentOpen tags:")
    for (_, code), n in sorted(r["currents"].items()):
        print(f"  {code:3} {ev.currents.row(code)[0]:22} {n}")


def print_ops(ev):
    for t in ev.tables:
        sw = t.switch
        print(f"# {t.title}: {sw.name} 0x{sw.fn:08x}, jump table 0x{sw.table:08x}, "
              f"{sw.count} entries, default 0x{sw.default:08x}")
        for code in range(sw.count):
            case = sw.cases[code]
            if code not in t.nargs:
                print(f"{code:4}  0x{case:08x}   -")
                continue
            name, struct_name, names, runs, effect = t.row(code)
            extra = f" [{runs}]" if runs else ""
            print(f"{code:4}  0x{case:08x}  {2 + 2 * t.nargs[code]:2}  {name:22} "
                  f"{struct_name or '-':20} {' '.join(names) or '-':40} {effect}{extra}")
        print()


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    for name in ("list", "survey", "ops", "check"):
        sub.add_parser(name).add_argument("elf")
    p = sub.add_parser("dis")
    p.add_argument("elf")
    p.add_argument("script", help="eventTblM101, M101 or an event number")
    p.add_argument("--parody", action="store_true", help="Parody Mode's messages")
    p.add_argument("--japanese", action="store_true", help="VOICE/ rather than VOICE_E/")
    p = sub.add_parser("dump")
    p.add_argument("elf")
    p.add_argument("--out", help="default: work/<volume>/events beside the disc directory")
    p.add_argument("--parody", action="store_true")
    args = parser.parse_args()

    ev = Events(args.elf)
    if args.cmd == "list":
        print(f"{'event':>5}  {'script':16} {'address':10} {'bytes':>5} {'blocks':>6}  name")
        for event in sorted(ev.events):
            name, label = ev.events[event]
            s = ev.script(name)
            flag = "" if s.clean else "  ** " + "; ".join(s.errors)
            print(f"{event:5}  {name:16} 0x{s.va:08x} {2 * len(s.shorts):5} "
                  f"{len(s.blocks):6}  {label}{flag}")
    elif args.cmd == "dis":
        print(disassemble(ev, ev.resolve(args.script), args.parody, not args.japanese))
    elif args.cmd == "dump":
        out = args.out or os.path.join(os.path.dirname(os.path.dirname(
            os.path.abspath(args.elf))), "events")
        os.makedirs(out, exist_ok=True)
        for name in ev.names:
            with open(os.path.join(out, name + ".txt"), "w", encoding="utf-8") as f:
                f.write(disassemble(ev, name, args.parody) + "\n")
        print(f"{len(ev.names)} scripts written to {out}")
    elif args.cmd == "survey":
        print_survey(ev, survey(ev))
    elif args.cmd == "ops":
        print_ops(ev)
    elif args.cmd == "check":
        return 1 if check(ev) else 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
