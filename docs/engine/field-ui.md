---
title: The field UI - HUD, menus and the event windows
status: partial
volumes: INF
covers: INF gcmn.prg:0x005280d0 ccThMenu, 0x0051c140 ccMenuCtrl::ccMenuCtrl, 0x0051ced0 InitMenuList, 0x0051cfb0 Disp, 0x00522430 SetPanelBure, 0x00522460 ExceptionDisp, 0x00522910 ConditionIconDisp, 0x00522eb0 FacePanelDisp, 0x00525b40 OpenMenu, 0x00525ce0 OpenMenuNWD, 0x00525e20 CloseMenu, 0x00525f60 ChangeMenu, 0x005260d0 ChangeMenu(int), 0x00526150 CheckMenuType, 0x00526170 Select, 0x00526490 SelectScr, 0x00526940 SetMenuFace, 0x00526e60 SetProtect, 0x00526ed0 SetSkillList, 0x005272d0 SetItemList, 0x00528810 SystemMenu, 0x00528f20 ChatMenu, 0x00529fa0 ChatMenuDisp, 0x0052cae0 SkillMenu, 0x0052d180 SkillMenuDisp, 0x0052db30 ItemMenu, 0x0052e6c0 ItemMenuDisp, 0x00531af0 TargetMenu, 0x005670c0 PersonalMenuT, 0x00567410 PartyMenuT, 0x00567660 PartyInMenuT, 0x00567e50 GateMenuT, 0x00568350 GtNewMenuT, 0x00569480 PersonalMenuTS, 0x005697b0 SkillMenuT, 0x00569a00 TargetMenuT, 0x00569d30 ChatMenuT, 0x0056a300 ItemBoxMenuT, 0x00546450 ItemBoxMenu, 0x005466d0 TrapBoxMenu, 0x005477e0 ItemIdolMenu, 0x00535210 DataDrainSubMenu, 0x005a1a10 ccSpcMessageOpenTreasureBox, 0x00525ec0 CloseMenuDisp, 0x00525a40 StillOff, 0x0055cbe0 GateMenu, 0x0055d1e0 GtRandomMenu, 0x0055df10 GtNewMenu, 0x0055f430 GtNewMenuDisp, 0x0055f310 ccGetGtNewColor, 0x005624a0 GtListMenu, 0x00563010 GtListMenuDisp, 0x005634d0 GtRecordMenu, 0x00564080 GtRecordMenuDisp, 0x00564480 GtTownMenu, 0x00564df0 GtTownMenuDisp, 0x00544020 GetItemMenu, 0x00544c00 AreaItem, 0x00544e50 ReplaceItemMenu, 0x0053b6e0 PartyInMenuDisp, 0x00523a30 LevelDisp, 0x0053ba20 ccThPartyAdd, 0x0059ce80 ccParty::AddMember, 0x0059cfe0 ccParty::CheckMemberID, 0x0056a8f0 ccCheckMenuFaceName, 0x00570cc0 ccGetCharParam, 0x005199e0 ccCheckTargetTypeId, 0x0042f290 ccAnalyzeEnemyList, 0x00459570 chaosGateInfluence, 0x0057a6d0 ccCheckItemUseful, 0x0057a890 ccCheckSkillUseful, 0x00570d50 ccGetSkillParam, 0x00570d70 ccGetItemParam, 0x00570f60 ccGetEquipParam, 0x005198c0 ccChangeCmndTarget, 0x0056a9f0 ccGetNowMaxColor, 0x0052f980 ImportantItemMenu, 0x005305d0 ImportantItemMenuDisp, 0x005274f0 SetImportantItemList, 0x00530d60 ThrowItemMenu, 0x00531820 ThrowItemMenuDisp, 0x00535620 StatusMenu, 0x005362d0 StatusMenuDisp, 0x00545e70 ItemStatusMenu, 0x005462a0 ItemStatusMenuDisp, 0x00539430 EquipStatusMenu, 0x00539ca0 EquipStatusMenuDisp, 0x00523e50 ParameterDisp, 0x00524e20 SkillDisp, 0x005251f0 BeffDisp, 0x00525150 BeffDisp(ccCondition), 0x00536f70 EquipmentMenu, 0x00538130 EquipmentMenuDisp, 0x00538de0 ccEquipChangeMenuSub, 0x00538d80 ccThEquipMenu, 0x00570f60 ccGetEquipParam, 0x0053c8e0 GateoutMenu, 0x0053cd60 LogoutMenu, 0x0056a640 TransFieldMenu, 0x0056a7f0 AreaInfoMenu, 0x0053a370 PartyMenu, 0x0053a7a0 PartyInMenu, 0x0053ba90 PartyOutMenu, 0x0053c340 PartyDisbandMenu, 0x0059cf60 ccParty::DelMember, 0x00525a80 StillOn, 0x0053d0e0 ControllerMenu, 0x0053d500 ccThControllerMenu, 0x0053d550 ControllerMenuDisp, 0x0053ea80 VibrationMenu, 0x0053ed90 VibrationMenuDisp, 0x0053f060 ScreenMenu, 0x0053f250 ScreenMenuDisp, 0x0053f800 SoundMenu, 0x0053fbc0 SoundMenuDisp, 0x00540270 ResetMenu, 0x00540730 DataDrainDemoMenu, 0x00540a50 DataDrainDemoMenuDisp, 0x00540d20 VoiceMenu, 0x00541010 VoiceMenuDisp, 0x005412e0 StrwinMenu, 0x005415d0 StrwinMenuDisp, 0x00526730 SelectXY, 0x00517800 ccThGameCtrl (its menu buttons), 0x0061f4a0 skillTbl, 0x00651000 menuElementData, 0x006516d0 btInChatAct, 0x00651570 virusCol, 0x00548060 ccMenuCtrl::FountainMenu, 0x00548420 FountainMenu2, 0x00548a00 FountainMenu3, 0x00549ed0 FountainMenuDisp2, 0x0054a260 FountainMenuDisp3, 0x00526c60 SetFountainCamera, 0x0069a260 feTbl; INF SLUS_202.67:0x001a54a0 ccMessage::Change, 0x001a5d70 DispMsg, 0x001a5f50 OpenInfo, 0x00177af0 ccSaveData::DelItem, 0x00177730 ccSaveData::AddItem, 0x00177990 GetItemNum, 0x00177a20 GetItemSlot, 0x0019e5c0 WORLD_MAN::SimGenerateCode, 0x0019eea0 SetGenerateCode, 0x001a3830 GetFieldAttrb, 0x0019d350 GetEventAreaInfo, 0x001b3100 ccEvent::CheckAreaCode, 0x00178c10 ccGetItemName, 0x00178d10 ccGetItemComment, 0x00178d90 ccGetItemIcon, 0x001da710 ccCheckCameraDeg, 0x001b97f0 ccMenuWindow::DispSlideBar, 0x001611a0 setCameraCtrlType, 0x00178600 ccSaveData::SetSoundEnv, 0x00177010 ccSaveData::ChangeEquipment, 0x00177070 ccChangeEquipment, 0x00176ce0 ccCheckSkillCount, 0x00177590 ccAddSkill, 0x001776e0 ccDelSkill, 0x00176f50 ccSaveData::GetEquipmentNum, 0x001a5790 ccMessage::Open(ccEvMsgData), 0x00160490 ccScFade::ContinueFade, 0x00160510 ccScFade::DeleteFade, 0x0033ebc0 fountainMenuHelp, 0x00307010 f_limitTbl; INF gcmn.prg:0x005418a0 SpcMenu, 0x00541e90 PcMenu, 0x00542840 VenderMenu, 0x00542df0 RecorderMenu, 0x00543390 FairyshopMenu, 0x005269d0 SetMerchantCamera, 0x0054de30 TalkMenu, 0x005552f0 SellMenu, 0x00555ee0 SellMenuDisp, 0x00556480 BuyMenu, 0x005572e0 BuyMenuDisp, 0x00558260 RecordMenu, 0x00558db0 RecordMenuDisp, 0x00559960 ItemDepositMenu, 0x0055a6f0 ItemDepositMenuDisp, 0x0055ac40 ItemDrawMenu, 0x0055b9f0 ItemDrawMenuDisp, 0x00527740 SetPlItemList, 0x00619460 npcTbl, 0x00648230 itemShopItemList, 0x00648430 equipShopItemList, 0x00648650 magicShopItemList, 0x0054e720 TradeMenu, 0x0054f4b0 TradeSubMenu, 0x00551570 TradeMenuDisp, 0x00527950 AddSpcItem, 0x00527d80 AddSpcItemSub, 0x005a1a70 ccSpcMessagePresentOtherFellow, 0x00538d80 ccThEquipMenu, 0x00542510 NpcMenu, 0x00553cd0 PresentMenu, 0x00555020 PresentMenuDisp, 0x00543930 BreederMenu, 0x0054bdd0 BreedingMenu, 0x0054cd70 BreedingMenuDisp, 0x0054a680 NorainuMenu, 0x0054ac40 OtonainuMenu, 0x0054b1d0 InuMenu, 0x0054bb10 InuMenuDisp, 0x00631cb0 sysopeMsg, 0x00637e30 spcMsgTbl, 0x00638800 spcMsgPresent10, 0x00638890 spcMsgPresent11, 0x00631c90 breedTeachMsgTbl, 0x00631ca0 breedTeachMsgTbl2, 0x00638dc0 pgEvoMsg, 0x005ee450 foodTbl; INF SLUS_202.67:0x00178680 ccSaveData::CheckTradeCount, 0x00178700 AddTradeCount, 0x0017fff0 ccCheckVoiceGrp, 0x001b9120 ccMenuWindow::DispSquareW2, 0x00178eb0 ccGetItemPrice, 0x0033ec80 buyMenuHelp, 0x00377e40 sellMenuHelp, 0x0033eca0 itemDepositMenuHelp, 0x0033ecb0 itemDrawMenuHelp, 0x00377e48 recordMenuStr, 0x00347f90 tpcTradeList, 0x00377e84 tpcTalkStr, 0x00178fb0 ccGetItemTradeRate, 0x00177f40 ccSaveData::DelSpcTradeList, 0x00177f70 DelNpcTradeList, 0x00177010 ChangeEquipment, 0x00177070 ccChangeEquipment, 0x00176ce0 ccCheckSkillCount, 0x00177590 ccAddSkill, 0x001776e0 ccDelSkill, 0x001767a0 SetTradeItemTown, 0x003457f0 spcTradeRateTbl, 0x003459d0 npcTradeRateTbl, 0x0033ec40 tradeMenuHelp, 0x0033ec60 presentMenuHelp, 0x00377e38 tradeMenuStr, 0x00377e0c getItemMenuStr, 0x00176200 ccSaveData::SetSpcBaseMsg, 0x00177eb0 AddFriendship, 0x0015cdd0 ccFont::MakeSignedNum, 0x0015cfc0 sdec2str, 0x00160490 ccScFade::ContinueFade, 0x00160510 DeleteFade, 0x00307180 friendship caps, 0x00377e24 breederMenuStr, 0x00377e28 breedingMenuStr, 0x00377e30 breedingMenuHelp; INF gcmn.prg:0x005661e0 GtHackMenu, 0x00566cb0 ccThGtHackMenu, 0x00566d00 GtHackMenuDisp, 0x00564ff0 ccHackMenu::ccHackMenu, 0x00565510 ccHackMenu::Draw, 0x00565ed0 ccHackMenu::Select, 0x00651730 hackCrystalOn, 0x00651770 hackCrystalOnF, 0x006517b0 hackCrystalOff; INF SLUS_202.67:0x00180780 ccSndGateHack, 0x00180910 ccSndGateHackCtrl, 0x001b7840 ccSetGtHack, 0x001602d0 ccScFade::EntryFlash2, 0x00377e4c gtHackInfo, 0x00377e60 gtHackStr; INF gcmn.prg:0x00532ae0 DataDrainMenu, 0x00377ddc dataDrainWarn, 0x0033e910 dataDrainEvolutionStr, 0x00651430 dataDrainErosionTbl; INF SLUS_202.67:0x001bb080 ccNoiz::ccNoiz, 0x001bb2e0 ccNoiz::SetNoizRn, 0x001bb410 ccNoiz::SetNoizBs, 0x001bb480 ccNoiz::SetNoizBr, 0x001bb4b0 ccNoiz::SetNoiz, 0x001bb690 ccNoiz::Draw, 0x00106af0 ccBufferSampling::ccBufferSampling; INF gcmn.prg:0x00522220 Disp's interNoiz, 0x0042e120 ccCheckObjectSize, 0x00432000 ccThDrainEnemy; INF SLUS_202.67:0x00184460 Func_str8000, 0x00377b78 strcmnFileList, 0x00377b80 datadrainFileList; INF gcmn.prg:0x0052a950 ChatMenu1, 0x0052ba80 ChatMenu2, 0x0052c0b0 ChatMenu3, 0x0059f3c0 ccSpcChar::CheckChangeEquip, 0x00546fd0 ItemObjMenu, 0x00547360 TrapObjMenu, 0x00546aa0 SymbolMenu, 0x00546dd0 VirusMenu, 0x0054a420 FoodMenu, 0x00458280 ccGimFood::ccGimFood, 0x00458710 ccGimFood::main, 0x00458580 ccGimFood::actRolling, 0x00547a50 TimeIdolMenu, 0x00651700 timeIdolItem; INF SLUS_202.67:0x0033e8c0 chatMenuWarn, 0x0033eb80 timeIdolMenuHelp, 0x001789a0 ccSaveData::CheckTimeIdolRankIn, 0x00178870 ccSaveData::SetTimeIdolRank, 0x0015f8e0 dec2sjis, 0x0015fa30 sjis2dec, 0x00167740 ccAddPlayTime, 0x001671e0 ccGame::ChangeRequest; INF gcmn.prg:0x0059f9e0 ccSPC::ChangeEquip, 0x0059fb50 ccSPC::ChangeWeapon, 0x0059fdb0 ccSPC::ChangeWeaponOldNewCcs; INF gcmn.prg:0x004008c0 ccThDfComp, 0x00400980 ccDfComp::Main, 0x00400da0 ccDfComp::Init, 0x00400d50 ccDfComp::Clear
---

# The field UI - HUD, menus and the event windows

Everything drawn on the menu layer (242) while Kite is in The World: the
party panels, the target's window and cursor, the enemies' life bars, the
battle banner, the bracelet's gauge, the menus (PERSONAL, Skills, Items,
TARGET, CHAT, OPTION, the tutorials ...), and the event scripts' speech,
information and tutorial windows (`ccMessage`, `ccMsg`). One task,
`ccThMenu`, owns it all through one object, `ccMenuCtrl` (`ccMenu`), in
gcmn.prg's `menu.cpp`. `crates/piney-fieldui` is the port; it is checked
against the game's own task run in eemu by `tools/test_fieldui_rs.py`.

## The task

`ccThMenu` (0x005280d0) is started by `ccSetupGameCtrl` with priority 34,
after `ccThGameCtrl` (33). One pass between two `ccTscb::Breath` calls is a
frame (the field runs at `frameRate` 2):

```
ccMenu = new ccMenuCtrl        sprites, ccChat, ccMsg, InitMenuList (menuList[89])
breathe until ccPartyManager.memberChar[0] is set
loop: breathe
      openReqNum >= 0:  OpenMenu(n), or OpenMenuNWD(n & 0xfff) for n & 0x1000
      CheckMenuType() == 88 and menuStatus == 0:
          menu = menuNext; menuNext != -1: menuStatus = 1, reverseHead = 0
                           else: the panels back (unless forbid), cmndTargetFix = 0
          both select cursors reset
      the handler of CheckMenuType() + 1 (jump table 0x006e0300)
      Disp()
```

`CheckMenuType` is `menu` when `menu == menuNext`, else 88 (a menu opening
or closing). A handler that closes a menu (`CloseMenu`, or its inlined
copies) calls `Disp` and `ccBreathThread(1)` itself; the rest of it runs at
the start of the next frame, before anything else in the task.

### `ccMenuCtrl` (0x240 bytes), the fields the menus use

| offset | member | use |
| --- | --- | --- |
| +0x00 | `panelBure[3]`, `panelFlash[3]` | a party panel's shake and flash (`SetPanelBure`) |
| +0x06 | `menu`, `menuNext` | the menu open and the one it goes to (-1 none) |
| +0x0a | `menuStatus` | the window: 0 shut, 1 fading in (+24 a frame to 128), 2 open, 3 fading out (-28) |
| +0x0c | `panelStatus`, `bgStatus`, `mapStatus`, `drainStatus` | the same four states for the party panels, the dim (to 104 by 15), the minimap (to 128 by 15), the bracelet's gauge |
| +0x14 | `openReqNum` | a menu asked for (0x1000: without the dim) |
| +0x16 | `mode`, `still` | `still` 1: the other tasks sleep and their layers keep their picture |
| +0x1a | `proccess`, `waitCount` | the open menu's step and counter |
| +0x2c | `faceNum` | the member PARTY's Add is on |
| +0x30 | (a task) | the Controller page's loading task (`ccThControllerMenu`), Equipment's model change (`ccThEquipMenu`), Add's call (`ccThPartyAdd`) |
| +0x1e | `exceptionDisp` | the menu draws its own page (`ExceptionDisp`) |
| +0x20 | `battleCnt` | the battle banner's blink |
| +0x34 | `alpha`, `kanjiAlpha`, `targetAlpha`, `panelAlpha`, `bgAlpha`, `mapAlpha`, `cursolAlpha`, `drainAlpha` | the fades |
| +0xec | `reverseHead` | the list's rows greyed, a bit each |
| +0xf4 | `plAttack` | |
| +0xb8 | `menuFade` | a `ccScFade` over the menu layer (Gate Out's and Log Out's fade to black, a Ryu Book's) |
| +0xf8 | `firstTime` | set by the buttons that open a menu |
| +0xfe | `forbid`, `forbidChatExcept`, `targetForbid`, `interNoiz` | the events' `menu_ban` and friends |
| +0x106 | `chatMember`, `chatAction`, `chatSkill`, `equipSpcNum`, `chatEquipStatus[2]`, `chatEquip[2]` | CHAT's kept member settings (below) and the Equipment menu's member |
| +0x12a | `mailCnt` | frames to the new-mail sound (240) |
| +0x12c | `protect[12]`, `protectCnt[12]`, `protectChar[12]` | the protect-break marks |
| +0x18c | `temp[8]` | the pages' own: Status's member for its Items (31), Area Information's three keywords, Add's "the member was loaded"; the Sound page's volumes (int[4]) |
| +0x1f4 | `subTarget[16]` | an area skill's other targets |
| +0x234 | `dummyTarget` | a character the talk menus hold as the target |
| +0x238 | `itemNum` | the item (or Data Drain skill) a sub-menu is about |

`menuList[89]` (at 0x0072efe0, 0x20 bytes each) is filled by
`InitMenuList` from `menuElementData` (0x00651000: name, title, and a
`{disp, x, y, items[y]}` block per list): `prev`, `select`, `page`,
`pageNum`, `index`, `disp` (how `Disp` draws the window: 4 its own page, 6 a
target list, 7/8 a list with a title, 9 tabs, 11 a dialog), `x` (width in
cells), `y` (rows shown), `sx`, `sy`, `dy` (first row shown), `my` (rows in
all).

## Who opens what

`ccThGameCtrl` (0x00517800), with no menu open, no event holding the pad and
the player free to act:

| button (`saveData.assignPAD`) | operation checked | opens |
| --- | --- | --- |
| personal menu (+0x8406, triangle) | 11 | PERSONAL: 0 in town, 1 in a field (2 on fields 1-12 and 67, or field 13 while operation 14 is held), 2 in a dungeon (1 on floors of type 8 and 9) |
| chat (+0x8408, square) | 10 | CHAT (3), `firstTime` 1; the only one allowed under `menu_ban` when `forbidChatExcept` is set |
| option (+0x840a, START) | 12 | OPTION (12) |
| action (+0x8404, cross) on `cmndTarget` | 9 | by its type: 0x06 21, 0x08 22, 0x10 23, 0xd00 24, 0x200 26, 0x1000 25, 0x8000000 27, 0x2000 28 (Chaos Gate), 0x4000 - 0x200000 0x1020 - 0x1027 (32-39, no dim), 0x400000 40, 0x800000 0x102b (43), then 44-46 |

The event scripts open menus with the `menu num=` opcode (`ccEvent::Execute`
sets `openReqNum` and waits until the menu shuts) and the item messages
with `item_add_menu` (`itemNum`, `openReqNum = 29`, `firstTime = 1`).
Infection's scripts use six numbers, all in the opening's tutorials:

| event | `menu num=` | handler | what it opens |
| --- | --- | --- | --- |
| 2 | 75 | `PersonalMenuT` | PERSONAL with Party (76) in place of Area Information, walking the player to Party > Add (77) with the tutorial's messages |
| 2 | 78 | `GateMenuT` | the Chaos Gate's list with New Keyword (79) |
| 3 | 80 | `PersonalMenuTS` | PERSONAL with the tutorial Skills (81) |
| 3 | 83 | `ChatMenuT` | the tutorial CHAT |
| 4 | 84, 85 | `ItemBoxMenuT` | the tutorial treasure box |

### Every menu number

The handler of each number (the jump table at 0x006e0300); "port" is what
`crates/piney-fieldui` has. The others are closed at once by the port
(`Request::Unported`).

| menu | handler | list | port |
| ---: | --- | --- | --- |
| 0, 1, 2 | `SystemMenu` | PERSONAL (town, field, dungeon) | yes |
| 3 | `ChatMenu` | CHAT | yes |
| 4 | `SkillMenu` | Skills | yes |
| 5 | `ItemMenu` | Items | yes |
| 6 | `ImportantItemMenu` | Key Items | yes |
| 7 | `ThrowItemMenu` | Discard Item | yes |
| 8 | `StatusMenu` | Status | yes |
| 9 | `PartyMenu` | PARTY | yes |
| 10, 11 | `GateoutMenu`, `LogoutMenu` | Gate Out, Log Out | yes |
| 12 | `SystemMenu` | OPTION | yes |
| 13 - 20 | `ControllerMenu`, `VibrationMenu`, `ScreenMenu`, `SoundMenu`, `ResetMenu`, `DataDrainDemoMenu`, `VoiceMenu`, `StrwinMenu` | the OPTION pages | yes |
| 21 - 27 | `SpcMenu`, `PcMenu`, `NpcMenu`, `VenderMenu`, `RecorderMenu`, `FairyshopMenu`, `BreederMenu` | talking to a character (Talk 47, Trade 48, Gift 50, Sell 51, Buy 52, Save 53, Store 54, Withdraw 55, Give Food 56) | yes |
| 28 | `GateMenu` | the Chaos Gate (Random 57, New Keyword 58, Word List 59, Warp History 60, Other Servers 61) | yes |
| 29, 30 | `GetItemMenu`, `ReplaceItemMenu` | an item got; a full bag | yes |
| 31 | `ItemStatusMenu` | a member's items (Status's square) | yes |
| 32 - 39 | `ItemBoxMenu`, `TrapBoxMenu`, `ItemObjMenu`, `TrapObjMenu`, `SymbolMenu`, `VirusMenu`, `ItemIdolMenu`, `TimeIdolMenu` | the field objects ([below](#the-field-objects-32---39-43-67)) | yes |
| 40 - 46 | `FountainMenu`, `FountainMenu2`, `FountainMenu3`, `FoodMenu`, `NorainuMenu`, `OtonainuMenu`, `InuMenu` | fountains (40 - 42), a Grunty food (43), Dun Loireag's dogs (44) and Grunties (45, 46) | yes |
| 47 - 56 | `TalkMenu`, `TradeMenu`, `TradeSubMenu`, `PresentMenu`, `SellMenu`, `BuyMenu`, `RecordMenu`, `ItemDepositMenu`, `ItemDrawMenu`, `BreedingMenu` | | yes |
| 57 - 62 | `GtRandomMenu`, `GtNewMenu`, `GtListMenu`, `GtRecordMenu`, `GtTownMenu`, `GtHackMenu` | the Chaos Gate's pages | yes |
| 63, 64 | `EquipmentMenu`, `EquipStatusMenu` | Equipment; a piece's status | yes |
| 65 | `TargetMenu` | a skill's or item's target | yes |
| 66, 67 | `DataDrainMenu`, `DataDrainSubMenu` | Data Drain ([below](#data-drain-datadrainmenu-menu-66)) | yes (66 without its movie) |
| 68 - 70 | `PartyInMenu`, `PartyOutMenu`, `PartyDisbandMenu` | Add, Remove, Disband | yes |
| 71 - 73 | `ChatMenu1`, `ChatMenu2`, `ChatMenu3` | a member's own orders, skills, target ([below](#a-members-orders-chatmenu1-71-chatmenu2-72-chatmenu3-73)) | yes |
| 74 | `StreamMenu` | a member drained ([below](#a-member-drained-streammenu-menu-74)) | yes |
| 75 - 77 | `PersonalMenuT`, `PartyMenuT`, `PartyInMenuT` | the party tutorial | yes |
| 78 - 85 | `GateMenuT`, `GtNewMenuT`, `PersonalMenuTS`, `SkillMenuT`, `TargetMenuT`, `ChatMenuT`, `ItemBoxMenuT` (84 and 85) | the other tutorials | yes |
| 86, 87 | `TransFieldMenu`, `AreaInfoMenu` | back to the field; Area Information | yes |

The table's case for 84 has no `break`: `ItemBoxMenuT` runs twice a frame
under menu 84 (the case for 85 follows it), once under 85.

`OpenMenu(t)` (0x00525b40) plays sound 16 except for 75, 78, 80, 83, 84 and
85, puts every other task to sleep and freezes their layers (`still`, and
the noise, still and game layers' `flipFlag` down one) unless `mode` is
set; `OpenMenuNWD` does the same without the sound. `ChangeMenu()`
goes into the list item under the cursor unless it is Gate Out and the
events hold operation 14; `ChangeMenu(m)` into `m`. Both set the target
list's `prev`, so cancel returns there.

## The HUD (`Disp`, 0x0051cfb0)

Once a frame, in this order: the battle banner's clock (in battle, before
the fight, no menu); the target (`cmndTarget`, else `cmndTargetPrev`, else
`dummyTarget`); the window's fades; the open menu's own page
(`ExceptionDisp`); the list window and its rows; the dim; the minimap's
alpha (`WORLD_MAN::SetMapAlpha(mapAlpha / 128)`); the bracelet's gauge;
`nameKanji` (the target's name and action, the three party names, the town
face's name, "You have new mail."); the target cursor and window (name, HP
and SP bars, numbers, element and condition icons); the party panels (face,
name, HP and SP bars coloured by `ccGetNowMaxColor`: under 20 % colour 2,
under 40 % 23, under 60 % 6, else 22); the enemies' life bars and
off-screen arrows; the protect marks; the battle banner; the new-mail notice
(towns); `ccMsg->Disp`; then every sprite's `SendPacket` in a fixed order
(each prepends to the layer, so the last sent is drawn first).

### The bracelet's gauge

While `drainAlpha` is up (`drainStatus`, which the Skills menu raises on
its Data Drain page; forced to 0 by `eventStatus[40]`), `menuDrain`
(`SetPrim(8, 2)`: Gouraud, over `xwin_vir::TEX_xwin_vir`):

```
frame  cell (1280, 0) 48x144 drawn 144x208 at (344, 56), colour 0
bar    cell (0, 2304) 128x32 at (352, 64), colour 2
erosion >= 60:  v = count % 25 folded at 12; 112x32 cells scaled 1 + v/40
       about (-56, -16), at (416, 280), colour 7, alpha a/3 + 2 a v / 36;
       cell (2048, 1280), from erosion 80 (2048, 1792)
virus  four 40x72 cells (0, 0), 2 a row, at (376 | 416, 112 | 184):
       v = count % 49 folded at 24; row = virusCol[erosion / 17]
       X = row[6..9] + (row[0..3] - row[6..9]) v / 24   (r, g, b)
       Y = row[9..12] + (row[3..6] - row[9..12]) v / 24
       each cell's corners Y but one X: cell 0 corner 3, cell 1 corner 1,
       cell 2 corner 2, cell 3 corner 0
```

A Gouraud cell is a textured strip of its four corners (top left, bottom
left, top right, bottom right), each with its colour word (`color[0..7]`
in pairs) and the alpha times `transp`. `targetCursol` is the one rotated
sprite of the menu's (`SetPrim` TRF): its cells are strips of four
corners turned about (cx, cy), as the minimap's second sprites are
([map.md](map.md#the-sprites)); the cursor's diamond turns by pi/4.
`piney_desktop::sprite::Sprite`'s `rot` and `vcol` draw both.

### The noise (`ccNoiz`, +0xe8)

After the sends, `Disp` (0x00522220 - 0x005223e4) acts on `interNoiz`
(+0x104). The events' `noise` instructions set it: `noise1` 1, `noise2` 2,
Infection's `noise3` 3, and from Mutation on `noise level=N` N + 2.

- **2.** `SetNoiz(12, 12, 12)`, then `interNoiz` back to 0: one burst.
- **Infection's 3 in town 4** (`game.area` 0, `game.town` 4). Every 16th
  frame (`ccSys.count & 15`): `SetNoiz(rand() % 10 + 4, 255, rand() % 10 +
  4)`.
- **From Mutation on** (MUT 0x005403c8 - 0x005406fc; Outbreak's and
  Quarantine's are the same code). The strong noise is its own level;
  town 4 no longer matters.
  - **3:** on every 64th frame, always on the 128th and otherwise when
    `rand() & 1`, `SetNoiz(rand() % 10 + 10, rand() % 10 + 15, rand() % 10
    + 4)`.
  - **4:** every 16th frame, `SetNoiz(255, 255, 255)`.
  - **5:** every 32nd frame, a noise and `cameraShake(2, 1, 30, 2)`. The
    noise is level 3's random one when bit 6 of the count is clear or
    `rand() & 3` is 0, else `SetNoiz(255, 255, 255)`. Every 8th frame,
    also sound 258.
  Mutation's event 301 uses level 3. Outbreak's and Quarantine's events
  301 and 313 use level 5.
- **Otherwise** (1, and Infection's 3 anywhere else):
  - On a Grunty, or with the menu shut, no menu type, no ban and no event
    running: every 64th frame, a one-in-four chance (`rand() & 3`) of
    `SetNoiz(n, n, n)`, with `n = rand() % 9 + 4`.
  - In a menu other than Data Drain's (66) and the gate hack's (62):
    `SetNoiz(0, 0, 0)`.

Then `ccNoiz::Draw` (main 0x001bb690). `ccNoiz` (main 0x001bb080, 0x20
bytes) keeps four frame counters. It draws on a layer of its own
(`ccLayer::Init(241, sysLayer's view)`, above the font layer). The effects
are the ones [the streams' effect tasks](stream.md#stream-2s-effect-task)
draw.

| counter | set by | drawn while it runs |
| --- | --- | --- |
| +0 | `SetNoizRn(n)`, `SetNoiz`'s first | the eight `ccRasterNoize` bands. At the set, band i moves to line `rand() % 50 + 48 i`, `rand() % 11 + 2` lines high, and one more number (`rand() & 31 - 16`) is kept but never read. The rows' offsets stay the ones `Init` drew (up to 32 pixels either way) |
| +4 | `SetNoizBs(n)`, `SetNoiz`'s second | `ccBufferSampling` at `SetReflex(1.0284, 0, 0x50808080)`: the last frame 2.8% larger, over this one at alpha 0x50. The set also puts 6 in +8 |
| +8 | once +4 is out | the same at alpha `80 n / 6`, n from 6 down: a six-frame fade |
| +0xc | `SetNoizBr(n)`, `SetNoiz`'s third | `ccBufferReverce`: the frame inverted |

- **The setters.** A value above 0 starts its counter (the inversion's from
  0). 0 stops it; `SetNoizBs(0)` leaves the fade to run. A negative value
  changes nothing.
- **The sound.** `SetNoiz` plays sound 94 when any of the three is nonzero.
- **Each drawn frame,** every running counter goes down one (not below 0).
- **The draw order.** The layer takes each packet at its front, so the GS
  draws the inversion, then the sampling, then bands 7 to 0.
- **The constructor.** Its eight `Init`s draw 216 numbers from `rand`.

The port:
- `piney_desktop::noiz` has `Band`, `Sampling`, `reverse_prim` and
  `Noiz`. `piney_stream::effect` uses the first three too.
- `MenuCtrl::noiz` holds the state. `disp::noiz` makes the menus' calls
  and `disp::inter_noiz` makes Disp's.
- `Draw::Noiz` carries the commands, and `render.rs` puts them on layer
  241.

### The chat balloons (`ccChatMsg`, `ccChat`)

`ccMenuCtrl`'s constructor makes `ccChat` (0x00378a90), a `ccChatMsg`
(main 0x001a6160-0x001a6c68) on the menu's layer. Its window sprite
takes `menuIcon`'s texture. `Disp` draws it after the dim's send and
before the damage numbers', with `still` passed on.

It holds four balloons. `OpenChat(ch, text)` closes any balloon already
over `ch`. The first free slot then takes the text (70 bytes at most) for
120 frames, and its width (`ccKanjiStrWidth(text, 0)`). Each `Disp`:

1. **Count down.** A balloon's frames count down unless `still`. One whose
   speaker is off the command lists (`ccCheckTarget`) closes.
2. **Place.** The rest stand at `ccCalcTagPosChar(ch, p, (0, 0, 0.9
   height), 0)` when that passes: a third of the text's width left of the
   point, 16 above.
3. **Stack** (`CheckScope`). A balloon that overlaps the band of those
   before it moves up above the first one it meets, again until it meets
   none. Each counts as 30 high.
4. **Draw** (`DrawWindow`). 14 x 16 cells from (0, 336) of the texture:
   - corners 0, 2, 8 and 10;
   - edges 1 and 9, as wide as the text;
   - the tail, cell 7, under the first third.

   The player's balloon is in colour 18, the others' in 1. Alpha is 72,
   fading over the last 10 frames. The text is colour 7 at alpha 128 x
   a / 72, drawn at once (`ccKanji::Disp`). The window's packets are sent
   last.

Who opens balloons:
- the CHAT menu's orders (the player's line);
- Kite calling the party's strategy in a fight (`ccSpcShoutOperationName`,
  gcmn 0x005a17e0): `chatActionStr` piece 5, `chatMenuStr` piece
  `tactics + 1`, `chatActionStr` piece 6;
- Mac Anu's walking players (`ccRtownPC`: `rtpcChatTbl`, `rtpcChatMes`,
  `rtpcChatMesShop`);
- the party members' `ChatMessage*` lines, sent by `ccAI::ChatMessageSender`
  ([the chat lines](battle.md#the-chat-lines));
- a Grunty (`ccPGuso::main`, which calls `OpenChat` itself with its own
  texts, `pgChatTbl`'s lines; piney-world's `grunty`, drawn by the town).

**In the port** it is `piney_fieldui::chat_msg::ChatMsg` in
`MenuCtrl.chat`. The runtime answers each speaker's command lists and point
every frame (`World::chat_at`, from `FieldUi::chat_speakers`):
- in a field or dungeon, `FieldWorld::chat_point`;
- in a town, `World::chat_point`.

The town reads its walking players' lines (`take_pc_events`, the texts in
GCMN.PRG) and its members' (`take_party_chats`), and the area mode Kite's
shout (`Show::Shout`) and the members' lines (`Show::Chat`), each over the
member (handle `1 << 24 | id`).
`tools/test_chat_msg_rs.py` runs the game's `ccChatMsg` in eemu against
`examples/chat_probe.rs`. It covers opens, closes, still frames, speakers
leaving the lists and the screen, and crowded balloons. The cells, texts,
colours, alphas and every slot's state match each frame.

## The event windows (`ccMessage`)

The event scripts' `message`, `info`, `info_now` and the tutorials' records
go through the same `ccMsg` the desktop uses
([the desktop page](desktop.md)): `Change` (speech, name and three lines,
typed), `ChangeInfo` (information, four lines at once), `Open` (a tutorial's
window afresh, with its voice), `Check(0)` each frame from the event until
it answers, `Close`. The menus use `DispMsg` (a help line kept up by calling
it every frame, `emode` 0x100) and `OpenInfo` (a refusal the player
dismisses).

## PERSONAL and OPTION (`SystemMenu`, 0x00528810)

```
proccess 0  the items' names (16 glyphs each) into menuKanji; target fixed
            and dropped; the dim in; Gate Out greyed in battle or while the
            events hold operation 14; Skills, Items, Key Items greyed while
            the player is down
proccess 1  Select; cancel (19) to prev or shut; OK (18) into the item; the
            three greyed ones while down: proccess 2
proccess 2  "Cannot be used while dead." once the window is gone
proccess 3  until Check, then 12 frames, then the list again
then        while proccess < 2: the item's help (personalMenuHelp / optionMenuHelp)
```

## The battle menus

### Skills (`SkillMenu`, menu 4)

Pages (`SetSkillList(pc, page)` over `saveData.skillList[pc][20]` at
+0x1ec4, `skillTbl` rows of 0x38 bytes): 0 attack (`type & 1`), 1 magic
(`type & 2`, none of 0x10000 / 0x20000 / 0x40000, not 2-5), 2 recovery
(`type & 0x40000`, or 178-180), 3 strengthen (`type & 0x10000`, not
178-180), 4 weaken (`type & 0x20000`), and with the bracelet
(`saveData.plcol`) 5 Data Drain: 2, then 3 from `drainEvolution` 9, 4 from
10, 5 from 11. Each row: the name, the SP cost in three digits. A row is
greyed in town, when the user's SP is short, or when `ccCheckSkillUseful`
finds nothing to use it on.

`ccCheckSkillUseful(skill)`: a skill for the party (`targetType & 3`) is
useful while a member is there (180 while one has fallen, `dead` not 0 or
5); any other while a character on `cmndSortRoot` has a type in
`targetType`, `cmndDist <= triggerRange + width`, is standing, passes
`ccCheckCameraDeg(pos, 0x1400)` and (Data Drain, 2-5) is an enemy or boss
whose protect is broken (`personality +98` not 0).

OK on a skill: in town `skillMenuHelp[0]`; short of SP `[1]`; Data Drain
while operation 17 is held or `eventStatus[40]` is set `[2]` (the refusal
waits for the window to go, `OpenInfo`, `Check`, 8 frames); else TARGET
(65). Otherwise the skill's name and help line.

### Items (`ItemMenu`, menu 5)

Pages by `SetItemList`'s table (0x00651690: -1, 11, 12, 14, -2): usable
items (categories 10 and 13), scrolls, books, treasure, equipment (0-9),
over `saveData.itemList[pc][40]` (+0x30, `{id, category, num}`); the
counts carried of 40 and in the box of 99 (`plItemList`, +0xb70) at the
top. `ccCheckItemUseful(cat, id)`: 10 and 11 need a target (1), 12 is used
at once (2), 13 is 1 for id 0 and 2 otherwise, 15 is 2 for the key items
that do something, else 0. OK on an item: 0 `itemMenuHelp[0]`; in town
only books and key items (`[1]`); 2 used at once on the player (`DelItem`
unless a key item, `ccUseItemRequest`, the menu shuts) - the Sprite
Ocarina (13/1) only in a dungeon (`[3]`), not on floors of type 8 and 9,
not in battle (`[5]`), not while operation 14 is held (`[4]`), after "Return
to the field." and OK / Cancel; 1 TARGET. Triangle on equipment: its status
(64).

### TARGET (`TargetMenu`, menu 65)

Rebuilt every frame (at most 8): a skill's party members, then
`cmndSortRoot`'s characters as `ccCheckSkillUseful` takes them (the type
may not include 3); for the Fortune Wire (13/0) those within 7000 of type
0x28000; for items 10/18 on the standing party. None: "No target in
range." (Data Drain's own warning for 2-5), and back. The cursor's
character becomes `cmndTarget` (`cmndTargetFix`); an area skill (`type &
0x6000`, or 3 and 5) collects up to 16 `subTarget`s from the target's chain
(`cmndPcRoot`, `cmndEneRoot` or `cmndObjRoot` by its type) within
`targetRange + width` of the target (of the user for `type & 0x2000`),
measured flat (`posP`, z dropped). OK: an item is spent and requested on
the target; Data Drain marks the target and sub-targets
(`EntryAffect(13)`), pays its SP and opens DATA DRAIN (66); a skill is
requested (`ccSkillRequest`). After an item or skill the window shuts, the
tasks wake, the menu waits while `ccSkillCheck(plw) >= 2`, drops the
target, waits six frames and closes.

### CHAT (`ChatMenu`, menu 3)

Alone: "You cannot use this command unless in a party." Else, in town two
pages (the town orders `townChatAct` = 9, 12; the members) and elsewhere
three: Skill Usage (`btInChatAct[0..8]`: seven orders, eight in a dungeon
where the Sprite Ocarina's can be used), Strategy (`btInChatAct[8..12]`,
the current one in colour 6, kept in `saveData.tactics` +0x6773), Members.
OK on an order: `ccSpcChar::RequestChatCmd(member, cmd)` to each member and
the player's line ("Everyone, " + the order) in the chat balloon
(`ccChatMsg::OpenChat`); on a member: its own orders (71). The order rows
are drawn with `ccKanji::Disp` directly (their `settingKanjiSend` cleared).
With `firstTime` set, opening keeps each member's six chat settings
(`personality +200`) at `chatMember + 12 k` and writes -1 at
`equipSpcNum + 2 k`; cancel reads those marks and so tells no member
(`ChangeEquipReport`) anything in normal play (the two strides differ)
until a member's Change Equipment (71) writes its mark: then the cancel
tells that member (`ChangeEquipReport(n)`, n the mark, or 0 when its kept
settings changed). The member answers with its line on the next frame, in
a town as in a field (`World::equip_report`, `FieldWorld::equip_report`).

### A member's orders (`ChatMenu1`, 71; `ChatMenu2`, 72; `ChatMenu3`, 73)

CHAT's Members page opens 71 on the `chatMember`th member (+0x106): the
filled slots 1 and 2 counted in order. Each of the three menus looks the
member up that way every frame; past the count they take slot 2's
character (none when it is empty) and index -1. `chatAction` (+0x108) and
`chatSkill` (+0x10a) carry the order from one to the next.

- **71's rows.** In town Change Equipment alone; in a battle (`inBattle`)
  Designate Skill, Change Equipment, First Aid!, Designate Target,
  Assemble, Standby; elsewhere the first three (`chatMenuStr` 13, 18, 1,
  12, 14, 15). `reverseHead` greys what the member cannot do: a fallen
  member's skill, aid and target (bits 0, 2 and 3 in battle; 0 and 2
  outside), and Change Equipment when `ccSpcChar::CheckChangeEquip`
  (0x0059f3c0) refuses. The list is `disp` 7, the cursor kept within the
  rows.
- **`CheckChangeEquip`.** 1 held, 2 asleep, 3 paralysed, 4 charmed, 5
  confused (by `condition` +0xa, +0x1a, +0x24, +0x1e, +0x1c), 7 a skill
  under way (`skillID` +0x7c 2 or more), 8 in act 12 or 13, 9 in act 14
  (`actNum` +0xee), else 0.
- **71's keys.** `Select(0, 0, 1)` (the help fades in again on a move).
  Cancel (19): back to CHAT. OK (18) on a fallen member's skill, aid or
  target: "Cannot be used while dead." (`deadInfo`). On Change Equipment:
  the check's result is kept in `chatEquipStatus[slot]` (+0x10e); 7 gives
  `chatMenuWarn[1]` (0x0033e8c0: "Equipment cannot be changed / while
  performing skills."), any other refusal `chatMenuWarn[2]`. The
  warnings wait for the window to close, open as information windows,
  wait for Check and then 8 frames, and 71 starts again.
- **71's orders.** `chatAction` becomes 20 (town), or by row 5, 20, 16,
  11, 3, 4 (5, 20, 16 outside a battle); `chatSkill` -1. 20: Equipment
  (63) on the member (`equipSpcNum` = its id, `firstTime` 1). 5: 72; 11:
  73 (their cursors to the top). 16, 3, 4 (First Aid, Assemble,
  Standby): `ccSpcChar::RequestChatCmd(member, cmd, 0, 0)`, "<name>,
  first Aid!" / "come here!" / "standby!" in the player's balloon, and the
  menu shuts (the inline `CloseMenu`). Every frame of `proccess` 1 ends
  with the row's help (`chatMenuHelp` 17 in town; 13, 17, 18, 12, 10, 11
  in battle; 13, 17, 18 outside).
- **72.** No skill in the member's list (`saveData.skillList[id]`): "There
  is no skill you can use." then back. Else its skills on five pages
  (`SetSkillList(id, page)`, `SkillMenuDisp` greying as for the member's
  SP), `SelectScr`; a page change sets `kanjiAlpha` 0. OK on a skill the
  member lacks the SP for: `skillMenuHelp[1]` ("Not enough SP.") and back
  after the window; else `chatSkill` and into 73. The help is the skill's
  name and lines.
- **73's candidates.** A skill for allies (`targetType & 3`): the party's
  standing members (skill 180, the revival: the fallen, but not state 5).
  Otherwise `cmndSortRoot`'s enemies and bosses (0xe0) within 2200, standing
  and in front of the camera, at most 16. None: `targetMenuWarn` ("No
  target in range.") and back after Check.
- **73's keys.** Up to 8 rows (`disp` 6, `bgStatus` 3), scrolled by
  `SelectScr`; the cursor's is the command target, and an area skill
  (`type & 0x6000`) marks the others around it (its chain by type, within
  `targetRange` + width). Cancel: the target dropped, `bgStatus` 1, back to
  72 or 71. OK: `RequestChatCmd(member, chatAction, target, chatSkill)`;
  for 11 "<name>, attack <target>!", for 5 "<name>, use <skill>!" on Kite,
  "... on yourself!!" on the member, "... on <target>!" otherwise (the
  strings' own "!!"); the target dropped and the menu shut. The help while
  `proccess` is 1: `chatMenuHelp` 12, or 13 with a skill.

**The port**: `crates/piney-fieldui/src/menus/chat_member.rs`. The orders
are `Request::ChatOrder`, which the runtime hands to the member's AI
(`FieldWorld::chat_cmd`, `ccAI::RequestChatCmd`); `CharInfo::act` carries
`actNum` for `check_change_equip`. The help after the menu's shut runs
next frame with the rest of the close (`After::ChatHelp` by the menu still
open). The runtime answers the balloons' positions for the party as well
as the balloons up (`hud_world`), so the order's line opened by the menu
in that frame is not dropped.

## The Chaos Gate (`GateMenu`, menu 28)

The action button on the gate (type 0x2000) opens 28 with `firstTime` set:
the gate's circle opens (`ccChar::EntryAffect(gate, plw, 11)`, which runs
`chaosGateInfluence` - `piney_world::gate::Gate::influence` - on the gate)
with a white flash (`ccScFade::EntryFlash(8, 0x3040c0c0)`). The list: Random
(57), New Keyword (58), Word List (59), Warp History (60), Other Servers
(61); the last three are greyed (`reverseHead` bits 2-4) while their lists
are empty, and Other Servers asks operation 16. Cancel closes the circle
(`EntryAffect(gate, 0)`, a flash) and shuts. OK puts the other tasks to
sleep and goes in; each page's help line is `gateMenuHelp` (`DispMsg`).

What the gate reads of the save:

| field | offset | what |
| --- | --- | --- |
| `wordList` | +0x523c | a bit per word ID the player holds (A, B and C words alike) |
| `gateOrderList[server][..]` | +0x5278 | the story areas in the Word List, in the order the events added them (`gate_add`), -1 ends |
| `gateListMark[server]` | +0x5048, 160 bits a server | the areas the events marked (`gate_mark`); Other Servers shows a mark by a town whose server has one |
| `gateRecord[server][20]` | +0x50ac | Warp History: `A * 1000000 + B * 1000 + C`, newest first, no repeats |
| `townMoveFlag` | +0x2234 | a bit per town Other Servers lists |
| `protectArea` | +0x65c8 | the protected story areas already opened |
| `areaCount` | +0x6862 | areas entered through the gate, up to 10000 |

An address is three word IDs (a word's ID is its `wordList` bit; its part,
A, B or C, is its table's index mod 3; the tables and the story areas are in
[area keywords](area-words.md)). Every page shows the address it would go
to on the keyword screen (`GtNewMenuDisp`): the server's symbol and the
three words, each word's attributes coloured by priority
(`ccGetGtNewColor`), the field type, the element (`GetFieldAttrb`'s table
on the merged `WORDPARAM`), the weather, and the battle level
(`ccAnalyzeEnemyList(server, element or 6, rank)`: the highest level among
the three `ccEnemyListInfo` entries from the rank; a story area's own enemy
rank when it has one). The level shown is written to `ccGame.areaLevel`
when that is 0 or in a town. The words go through
`WORLD_MAN::SimGenerateCode` exactly as the area generator does, so the
screen's story area (`eventAreaNumber`) is the one the warp enters.

| page | how the words are chosen |
| --- | --- |
| Random (57) | each part `rand() % n` over the held words of that part; Warp or Cancel |
| New Keyword (58) | each part picked from the held words in a scrolled list (A, then B, then C; cancel steps back) |
| Word List (59) | a story area from `gateOrderList[server]` (its help from `gateWordListMsg`), its three words from `eventAreaInfo` |
| Warp History (60) | a `gateRecord` entry |
| Other Servers (61) | a town of `townMoveFlag`; "Warp" asks yes / no |

"Warp" (the same code in every page):

1. `ccEvent::CheckAreaCode(a, b, c)` records the words in `areaCodeSet`
   (which the events' `gate_words` reads) and tests `areaCode[16]`: a code
   with mode 0 bars its story area, one with mode 1 allows only the areas so
   listed. Barred: the circle closes, the window shuts.
2. A story area on this server with protection (`protect[1, 3, 5, 7]`) and
   not yet in `protectArea`: the gate hack (62, [below](#the-gate-hack-gthackmenu-menu-62)).
3. Else the tasks wake, the window closes and the party leaves:
   `ccSpcChar::TransferOut` on party slot 0, 1, 2 at the menu's frames 5,
   35 and 65; from frame 211 on, every frame until the area changes,
   `ccSPC::DeleteNoPartyMember()` and `WORLD_MAN::SetGenerateCode(a, b, c)`
   (main 0x0019eea0: `ChangeArea(1, story area)` for a field, `ChangeArea(2,
   0)` for a dungeon, a story dungeon's `ChangeScene`). Random records the
   address in the history unless it names a story area; New Keyword records
   it for no story area or areas 36-38, 68-70, 92-99, 118 and 119.
   `areaCount` goes up.

Other Servers' "Warp" is the same leave, then `ccGame::ChangeArea(0, town)`.

What the gate asks of the runtime, as `Request`s, in order: `Affect { gate,
11 }` and `Flash` on opening (`Affect { gate, 0 }` and `Flash` on cancel or
a barred address); `SleepAll`, `Still(true)`, `KeepLayers` going into a
page; `TargetFix`, `Target` / `TargetClear` (the target leaves the gate on
the pages and comes back on the list); `AreaCodeSet([a, b, c])`;
`AreaLevel(v)`; `WakeAll`, `Still(false)`; `TransferOut(member)` three
times; then `DeleteNoPartyMember` and `GoToArea([a, b, c])` each frame (or
`ChangeArea { area: 0, n: town }`). The world gives it `game.server`,
`game.town`, `game.area_level`, `area_codes` (the events' `areaCode`), the
gate as `cmndTarget` (and `cmndTargetPrev` on the pages).

## The gate hack (`GtHackMenu`, menu 62)

A protected story area's gate needs virus cores. The area's `protect` row
is four (core, count) pairs; the cores are key items of category 15 (ids
0-25, the letters A-Z) in `impItemList`. `GtHackMenu` (0x005661e0) runs
the warning, then a screen of its own (`ccHackMenu`) where the player puts
the cores in.

- **proccess 0.** The window closes (`menuStatus` 3), `exceptionDisp` 0,
  `waitCount` 30, the flash count (`temp[4]`) 0, and `ccSndGateHack(0)`:
  the town's music fades out over 20 frames.
- **proccess 1.** Each frame the count goes up.
  - Below 25: `DispInfo` of `gtHackInfo`'s two lines, "#RPROTECTED#W" and
    "You cannot warp to this area.".
  - From 31: a red flash (`EntryFlash2(2, 8, 0x1c4040ff)`: in over 2, out
    over 8), sound 20, the count back to 0.
  - After the fourth flash the dim goes out and proccess 2 begins.
- **proccess 2**, which breathes itself frame by frame:
  1. **The fall.** `ccThGtHackMenu` starts; it loads `flGtHackMenu`
     (`XDHHACK.CCS`) and clears its flag. The count runs from 0. On every
     sixth frame from 26 to 59, `ccNoiz::SetNoiz(n, n, n)` with `n =
     rand() % 4 + 3`. At 45 comes a fade to black over 20
     (`EntryFade(20, 0, 0x80000000)`). It goes on until the fade ends: 65
     frames.
  2. **Black.** The chat balloon closes, the noise stops, `still` goes to 0
     and the layers flip again, while every other task still sleeps. So the
     world draws nothing over `ccSys.bgColor`, which is set to 0. Two
     frames pass without `Disp` (`ccBreathThread(2)`).
  3. **The screen.** The fade is deleted, the load task is waited for and
     deleted, and a flash comes (`EntryFlash(10, 0x50e0c000)`).
     `new ccHackMenu` is built on `xdhhack`, then 55 frames of `Draw` and
     `Disp`.
  4. **The crystals.** Another flash, `crystal` on, 60 frames.
  5. **The slots.** Another flash, `frame` on, the texts in
     (`fontStatus` 1), the window in, and `exceptionDisp` 1: the button bar
     (`GtHackMenuDisp`). Then, each frame, `Select`, and `Draw` while it
     returns 0.
  6. **Complete.** On OK passed: `complete` on, 90 frames of
     `ANM_xdhcomp0`.
  7. **Out.** On OK or cancel: a fade to black over 20, the window out,
     `ANM_xdhfram1` and the texts out (`fontStatus` 3), until the fade ends.
     Then two frames, the screen deleted, two more frames, and the file let
     go.
  8. **Cancel.** `ccSndGateHack(1)` brings the music back, `bgColor` is put
     back, a fade runs from black (`EntryFade(20, 0x80000000, 0)`: `$a2`
     still holds the value `bgColor` was or'd with), and the tasks wake.
     Then 40 frames with noise on every sixth, the noise off,
     `ccSndGateHack(2)`, two frames, and the menu shuts.
  9. **Hacked.** On OK:
     - each needed core's count is taken (`DelItem(0, 15, core, count)`);
     - `protectArea`'s bit for the area is set;
     - `DeleteNoPartyMember`;
     - `ccGame.setupMode = 1` and `ccSetGtHack()` (`gtHackFlag`: the party
       arrives hacking in, [field walk](field-walk.md#the-hacked-arrival));
     - `ccSndGateHack(2)`;
     - `areaCount` up, at most 10000;
     - `WORLD_MAN::SetGenerateCode` on the gate's three words.

     proccess stays 2, so the next frame runs the fall again until the
     area changes.

**`ccHackMenu::Select`** (0x00565ed0):
- Operation lock 15 holds it.
- A turn in progress (`selectCnt`) counts toward 0 first.
- OK returns the OK button when every slot holds its count (sound 222).
  Otherwise it buzzes (220) and returns 0.
- Cancel returns the cancel button.
- Left and right (repeat) turn the ring a slot (221): `select` + 1 or - 1
  wrapped over 4, `selectOld` kept, `selectCnt` -10 or 10.
- Up puts a core in when the player has more than the slot holds: 219, or
  220 when that would pass the slot's count, which it is held to.
- Down takes one out: 219, or 220 at 0.

**`ccHackMenu::Draw`** (0x00565510):
- `ccLayer::active` is its layer 127, whose view takes `ANM_xdhcamer`'s
  camera.
- The background (`ANM_xdhback0`, then from its end `ANM_xdhback1`,
  looping) steps and draws.
- **With `crystal`**, `ANM_xdhcrys0` steps, then:
  - `OBJ_xdhroll`'s matrix becomes the constructor's copy times a Z turn
    of -pi/2 a slot plus the turn's share, wrapped to -pi..pi.
  - Every slot's 4 crystals are set by the slot's count and what it holds.
    Past the count, all three are hidden (`hackCrystalOff`, `On`, `OnF`,
    `dispSW` 0 or 3). A crystal put in shows On and OnF; one still needed
    shows Off.
- **With `frame` and `complete`**, their animations step and draw.
- **The texts' alpha** comes in by 16 to 128 and goes out by 6.
- **While the alpha is above 0:**
  - The global `font` (type 2) shows two columns of 13 at (365 + 61 c,
    58 + 21.4 r): each core's letter, a space, and its count less what its
    slot holds, in two digits. The colour is 22 for a needed core the
    player has enough of, 18 for one they are short of, and 17 for the
    rest.
  - `hackMask` (`TEX_xdhroma1`: 56 x 48 cells, four a row, drawn 56 x 50,
    on layer 128) shows the slot in front: its count still to put in
    (cells 26 + digit, at (180, 272)) and its core (cell = id, at (116,
    272)). Across a turn it fades out and back in.

**`GtHackMenuDisp`** (0x00566d00):
- `gtHackStr`, "Rotate", "Add/Subtract", "Execute", "Cancel", on menuKanji
  (`SetClm(12)`, colour 7) at x 68, 180, 314 and 418, y 402.
- The pad's marks on menuWindow (type 2): the D-pad twice, its left and
  right lit (colour 18) by "Rotate" and its up and down by "Add/Subtract",
  then cross (19) and circle (16).

**The port.** `crates/piney-fieldui/src/menus/hack.rs`:
- The frames are `Resume` steps (`After::Hack`); the screen is
  `MenuCtrl::hack`.
- The animations are `piney_desktop::anm::Anm`, with an owner's
  `dispSW` and matrix writes (`set_disp`, `set_local`, `object_named`,
  `objects_prefixed`).
- `Draw::Hack` carries them for `render.rs`, which draws them on layer 127
  through the camera. The texts go out as `Draw::Send` on `Obj::HackMask`
  (layer 128) and `Obj::SysFont` (the font layer).
- New `Request`s: `GateHackSound(n)`, `WorldHidden(on)` and `GateHacked`.
  The noise is the menu's own ([the noise](#the-noise-ccnoiz-0xe8)).
- The town (`world.rs`) acts on some of them:
  - `WorldHidden` stops drawing the world and the minimap.
  - `GateHackSound(0 / 1)` fades sequences 0 and 2 to nothing, or back to
    256, over 20 frames.

## Data Drain (`DataDrainMenu`, menu 66)

TargetMenu opens 66 once Kite's Data Drain has a target. The skill is in
`itemNum` (+0x238: 2 Data Drain, 3 Drain Arc, 4 2128 Drain, 5 Drain Heart)
and the target is `cmndTarget`. The rules are `piney_battle::drain`'s; the
menu is `crates/piney-fieldui/src/menus/drain.rs`.

- **0, the drain.**
  - The screen breaks up and fades to black: `EntryFade(25)`, noise every
    frame, `rand() % 3 + 3` every sixth. Then `still` goes off and
    `bgColor` is 0; two frames.
  - A boss takes `EntryAffect(21)` from Kite.
  - The movie ([below](#the-drain-movie)), unless the target is an enemy
    and `drainDemo` is off.
  - Then the rules: `AddLvErosion`, the drops into `drainItem` (+0x1ac,
    17) and their count (+0x1f0), and `ccDeleteCmnd` on any target but a
    boss.
  - `cmndTarget` and `cmndTargetPrev` are cleared directly, `bgColor` put
    back, a flash (20, 0x70c0a020) shown, and the tasks woken; three
    frames.
  - Then step 1's roll, so from 0 the roll is made twice.
- **1, the roll.** `infection / 2 < rand() % 100` goes on to 2; anything
  else is a side effect (10).
- **2-3, the warning.** Noise until the count passes 45 (the count goes
  on from 0's fade). Then "Viral infection has spread!", the world asleep
  and its layers kept, until OK; then 20.
- **10-12, the side effect.** It is picked by
  `dataDrainErosionTbl[infection / 25][rand() & 15]`, with `SetNoiz(8)`.
  Step 10 starts what it shows on the characters (below). Then the count
  runs to 61, with noise at 10, 18 and 45; at 30 Kite loses a level if
  the effect takes one, with LEVEL DOWN over him
  (`ccEntryFlyFontNewLevelDown(23, pos, plw)`). The window follows: "Data
  drain out of control!" with the effect's line (`dataDrainWarn` id + 2).
  Effect 29 names the item lost, with sound 79 and a red flash, or says
  "But nothing happened." Effect 30, SYSTEM ERROR, ends the game
  (`compulsionGameOver`). Then 20.
- **What step 10 starts** (0x005337a4-0x0053476c), in order, each
  character's in turn (Kite, or the party's living members):
  - effect 0: `effHeal(ch, 1)` on each;
  - 1-6: `effSkillStartEffect(Kite, 0, 1)`;
  - 7-20: for a resisted condition `ccEntryFlyFontNewMiss(ch's pos, ch)`
    (MISS), then `effSkillStartEffect(ch, 0, 1)` whether resisted or not;
  - 21, 22, 28: `effSkillStartEffect(ch, 0, 1)` on each;
  - 23-27: `ccEntryFlyFontNewExp(23, -loss, Kite's pos, Kite)` (-200 to
    -1000 EXP), then `effAfterDrain(Kite, 0)`;
  - 29, 30: nothing.
  The world runs through step 11's frames, so these play before the window
  opens and the world sleeps. `tools/test_battle_drain_rs.py` checks the
  starts and their order against the game. piney-game's
  `drain_side_effects_show` forces each kind on a drain of Skeith and sees
  the rings (-13, and the heal's -19), MISS, the exp's number, LEVEL DOWN
  and the wave (12) come up during step 11.
- **20, the growth.** `drainCount` goes up, and the bracelet grows at 10,
  20 ... 80 drains, adding key items 273-280. Its pages come from
  `dataDrainEvolutionStr`, four lines a stage (the loop reads a fifth and
  closes over it). Then `cmndTargetFix` goes off, and `ChangeMenu(67)`
  hands the drops out through 29, the last first.

The runtime answers two requests with the rules' results:
- **`Request::DataDrain`** is answered with `FieldUi::drain_drops` before
  the menu's next frame. Nothing reads the drops until 20, and the roll
  only reads the save's infection three frames later.
- **`Request::DrainSideEffect`** is answered with
  `FieldUi::drain_side_effect`. The rules' `SideEffect::starts`
  (`piney_battle::drain::SideStart`) become `Show::DrainSide`, which the
  field's effects start in order; `DrainLevelDown` adds
  `Show::DrainLevelDown`.

It also carries out three others: `DrainLevelDown`, `TargetsCleared` and
`GameOver`. `area.rs` does all of these through `FieldWorld::data_drain`,
`drain_side_effect` and `kite_level_down`.

### The drain movie

Step 0 (0x00532ce4 - 0x00533004) picks a stream by the target:
- **An enemy (type 0x60)**, only with `drainDemo` on: by
  `ccCheckObjectSize` (gcmn 0x0042e120, its `enemyTbl` row's +0x70): 4
  (small) 109, 3 (middle) 110, 1 (large) 111. Every row is one of the
  three.
- **Anything else,** whatever `drainDemo` says, by `base->id`: 0 (Skeith)
  18, 1 37, 2 44, 3 67, 4 69, 7 100, else 111.
- **`cmndTarget` gone** (`ccCheckTarget` fails): 109.

Then:
1. `ccStartThread(ccThExecuteStream, 65)` with the stream in its param,
   and `ccBreathThread(2)`.
2. With the target still there, it waits for the stream to be loaded and
   playing (`ccGetStreamAdrs`, +0x17e bit 3). For an enemy it starts
   `ccThDrainEnemy` (65) on it.
3. `ccBreathThread(1)` (no `Disp`) until the stream's param is -1. Each
   frame, in its scene 0, stream frame 30 sets the enemy task's state to
   1 and 229 to 3.
4. At the end the state goes to 3, and both tasks are deleted.

The streams (`str8100`-`str8300`, 351 frames) hold no models of their
own. Their `#` objects (Kite's body, the bracelet, the target's orb) are
defined by the files a field or dungeon keeps resident: `strcmnFileList`'s
`STR8000E.CCS` (loaded everywhere) and `datadrainFileList`'s
`STR8001E.CCS` (a field's and a dungeon's lists), `DATA.BIN`'s `str8000e`
and `str8001e`.

`ccThDrainEnemy` (gcmn 0x00432000) builds the enemy's clump from its row's
file (`CMP_trall`, its palette) with a `ccAnm` on its animation slot 6,
and a middle boss's second model with the same clip ('x' its eighth
letter). While the stream plays with its layer on, in state 1 it
animates them forward and draws them at (0, -900, 130) turned half about
z, on the stream's layer (+0x154) through its draw environment (+0x158).
It also builds the race's base form for a state 2 that nothing sets.

The port:
- **The menu.** `Request::Affect` (21) for a boss, then
  `Request::DrainMovie(num)`. After two frames comes
  `Request::DrainEnemy(handle)` for an enemy, and the menu waits for
  `FieldUi::drain_movie_done`.
- **The runtime.** `area.rs` plays the stream with
  `StreamPlayer::drain` over the two resident files
  (`piney_stream::Stream::with_resident`). The enemy is
  `piney_world::foe::DrainEnemy`, drawn into the scene through
  `Stream::step_with` with the stream's camera and lights. The minimap is
  hidden with the world.

## A member drained (`StreamMenu`, menu 74)

`ccMenuCtrl::StreamMenu` (gcmn 0x00535340) plays a stream over the field
with party members drawn into it. Skeith's `OnDataDrainAtk` opens it at
its count 45, and the fields it sets are:

| field | value |
| --- | --- |
| `streamNum` (+0xf2) | 20 |
| `streamFlag` (+0xf0) | `1 << slot` of the drained member |
| `openReqNum` | 0x104a: 74 opened by `OpenMenuNWD` |
| `mode` | 0 |
| `firstTime` | 1 |

The handler is one call that breathes its own frames:

1. Unless `streamFlag` 0x100, it makes a fade to black over 12 frames (with
   0x200, to white in 1) and runs `Disp; Breath(1)` until the fade is over.
2. It lets the world's layers go (`still`) and blacks the clear colour
   (`ccSys.bgColor`), then two frames.
3. `ccThExecuteStream` starts on `streamNum`, then two frames. It waits
   until the stream is there and playing.
4. It deletes the fade, then two frames.
5. With `streamFlag & 7`, it starts `ccThStrParty` (gcmn 0x0056ab00). Each
   member the bits name gets:
   - its own `CMP_trall`, posed by its `spcAnmTbl` clip (0x00651890:
     `ANM_ctu1nut2` for Kite ...);
   - each frame, `_AnimateForward` and a draw at the stream's
     `OBJ_dmy_spc0`-`2` (`spcDmyTbl`, counting the members drawn), on
     the stream's layer and lights.
6. It waits until the stream's parameter is -1.
7. It deletes both tasks, puts the clear colour back and wakes the tasks.
   Then it sets:
   - `menu` and `menuNext` to -1;
   - `menuStatus` and `bgStatus` to 3, `panelStatus` to 1;
   - `panelAlpha` to -24;
   - `cmndTargetFix` and `firstTime` to 0.

The boss's frame waits for `CheckMenuType()` to be -1 again before its
affect 13 on the member.

In the port, `menus::stream` asks for `Request::StreamMenu(num)` and
`Request::StrParty(flags)`. The area mode plays the stream as it plays
the drain movie, over the field's resident files, and draws
`piney_world::foe::StrParty` into it. `FieldUi::stream_menu_done` answers
the stream's end, and `FieldUi::open_stream_menu` is the boss's request.

## The tutorials (75 - 85)

Each opens its own lines from the event's message table (`evMsgTblM1[e]`,
`evMsgTblM1p` in Parody Mode; record `n` at `n * 12`, its text at +8) with
the voice `(e, n)`, and lets only one row go on (the others buzz, sound
20).

| menu | event | lines | what |
| ---: | ---: | --- | --- |
| 75 `PersonalMenuT` | 2 | 12, 13 | triangle opens the list; only Party (row 6) |
| 76 `PartyMenuT` | 2 | 14 | only Add (row 0) |
| 77 `PartyInMenuT` | 2 | 15 - 19 | the members who can join (`PartyInMenuDisp`: Orca, member 2, with her panel and level box), OK / Cancel (only OK), "Calling party member.", then `ccThPartyAdd`: `ccParty::AddMember(2)` from a thread, the new slot's face; line 19; the panels in after 7 frames and out after 61 more; shut |
| 78 `GateMenuT` | 2 | 30 - 33 | the gate (`ccCheckTargetTypeId(0x2000, 16)` on the object chain) opens; only New Keyword |
| 79 `GtNewMenuT` | 2 | 35 - 56 | the keyword screen, only each part's first word; the party's leave (no area change: the event takes over) |
| 80 `PersonalMenuTS` | 3 | 34 | triangle; only Skills (row 0) |
| 81 `SkillMenuT` | 3 | 35 | Kite's skills; only the third page's first (recovery) |
| 82 `TargetMenuT` | 3 | 36 | the party as targets; OK: `ccSkillRequest(plw, target, 150)`; shut, panels out |
| 83 `ChatMenuT` | 3 | 45, 46 | square opens CHAT; only the first page's second row: member 1's `ai->manualSW` cleared, the player's line, four more `Disp; Breath` passes, `RequestChatCmd(member 1, 5, plw, 155)`; 11 frames on `ManualModeAI(member 1, 1)` and `SetRemoteCmd(0)`; shut, panels out |
| 84, 85 `ItemBoxMenuT` | 4 | 16 or 20 | the action button: the box opens (`EntryAffect(box, 11)`), its item (+0x14c), 21 frames, sleep, `AreaItem`, `itemBoxCount` up (to 10000), into 29 |

`PartyInMenuDisp` (also menu 68) lists the members of `partyMemberFlag`
(+0x2220) not in the party (`CheckMemberID(k) == -1`), rows `dy` to
`dy + y`, and draws the chosen member's panel at (220, 96) with full HP and
SP from `spcParam` and `LevelDisp` at (220, 192): Level, EXP ("n/1000"),
Money ("nGP") and Class from `spcParam` (+0x0e, +0x10, +0x14, +0xd8).

## An item got (`GetItemMenu`, 29) and a full bag (`ReplaceItemMenu`, 30)

29 shows "You now have #G<item>#W!" (sound 74; a key item adds "It is added
to Key Items"), then adds it (`AddItem(0, cat, id, 1)`; key items count in
`impItemList`). With 99 already: "You already have 99 ...", "Gave up ...".
With the bag's 40 kinds full: "Exchange with a current item?" - Cancel
gives it up, OK opens 30 (`trapNum` = the new item). 30 is the Items pages
(`ItemMenuDisp`, triangle on equipment opens its status, 64): OK on an item
asks "Discard <it> for <new>?" and swaps them (`DelItem(it, 99)`, then
`AddItem(new)`); Cancel asks "Give up <new>?". Both end back in the Data
Drain (67) when 29 came from it, else shut the window (the panels out when
29 was opened by itself, from the box tutorial, or under `menu_ban`).

`AreaItem(n, kind)` (0x00544c00) keeps `n` when it is an item; for -1 it
draws from a 130-entry list by server and element (`ItemBoxList`, or
`DangerItemBoxList`, `SukaItemBoxList`, `idolItemList`, `idolSubItemList`,
`areaItemList` by kind): index the story area's item level (`eventAreaInfo`
+0x1c) in a story field, else `(areaLevel - 1) * 25 + 10 + itemOfs` of the
area's words, plus `floor + 1` and `rand() % 5`, at most 129.

## The field objects (32 - 39, 43, 67)

The action button opens these menus by the target's base type
(`ccThGameCtrl`'s table), without the dim or the sound (`openReqNum`
0x1020 - 0x1027 and 0x102b, `mode` 1, `firstTime` 1). What carries each type in
Infection (`gimmickTbl`, gcmn 0x0061e0f0):

| menu | type | gimmick rows | placed by |
| ---: | --- | --- | --- |
| 32 `ItemBoxMenu` | 0x4000 | 0 Treasure | `SetItemBox`: the dungeons' box slots; story rows type 0 |
| 33 `TrapBoxMenu` | 0x8000 | 1 Risky Treasure | story rows type 3 kind 1 (event area 14) |
| 34 `ItemObjMenu` | 0x10000 | 2, 4, 7-14: Wooden Box, Barrel, Jar, Urn, Skeleton, Warrior's Body, Egg, Small Eggs | `EntryBreakObject` in every room (rows 7-14); story rows type 0 kinds 1 and 2 (rows 2, 4) |
| 35 `TrapObjMenu` | 0x20000 | 3 Wooden Box, 5 Barrel | nothing found |
| 36 `SymbolMenu` | 0x40000 | 17, 18 Symbol (`ccGimSymbol`) | story rows type 3 kind 4 (event areas 17, 18, 20, 24, 120 and later volumes'); 18 in the lake dungeons (types 8, 9: `SetMagicCircle`) |
| 37 `VirusMenu` | 0x80000 | 6 Virus Crystal | story rows type 3 kind 0 and 27 on (event areas 93, 102 - 107: later volumes' story) |
| 38 `ItemIdolMenu` | 0x100000 | 38 - 43 Treasure (the Gott statues) | `SetIDOL` by the area's element |
| 39 `TimeIdolMenu` | 0x200000 | 44 Zeit Statue | `SetIDOL` when `WORLD_MAN.timeSym` (the first word 131, "Chronicling") outside the lake types |
| 43 `FoodMenu` | 0x800000 | 22 - 37 the Grunty foods (`ccGimFood`; 22 Golden Egg) | `SetItemBox`'s box slots (below 30 of `fieldrand(100)`: `GetFood()`'s food; below 10 the Golden Egg, the lakes `GetFood()`'s); story rows type 3 kind 5; `WORLD::SetFood` in a field |

Each of 32, 33, 36 - 39 and 43 fixes the target (`cmndTargetFix`) on its
first frame, opens it (`EntryAffect(cmndTarget, plw, 11)`: the object's
own act, a trapped box's trap on the opener) and drops it
(`ccChangeCmndTarget(0)`), keeping what it needs of it (the item at
+0x14c, `itemNum`; 36 the skill at +0x7c); then 22 frames and the other
tasks asleep (`bgStatus` 1, `mode` 0, `still`, the three layers held;
39 sets `bgStatus` 1 even when they already sleep).

```text
ItemBoxMenu (0x00546450)  trapNum = the box's param[2] (+0x150); a draw
                          from DangerItemBoxList when it was trapped
                          (trapNum >= 0: 3 once disarmed) or rand() & 7 is
                          0, else ItemBoxList; itemBoxCount up (at most
                          10000); ccSpcMessageOpenTreasureBox; into 29
TrapBoxMenu (0x005466d0)  trapNum = the box's skillID (+0x7c): 1, 156, 162
                          show getTrapMenuStr's "Set off trap!" with line
                          1 (the explosion), 2 (the poison gas), 3 (the
                          cursed gas) after 9 frames, until the button,
                          then 7 frames; -1 or another skill none; then a
                          draw from SukaItemBoxList, itemBoxCount up, 29
ItemIdolMenu (0x005477e0) idolCount (+0x746e) up; the party's line;
                          drainItem[2] = AreaItem(item, 4), drainItem[1]
                          and [0] AreaItem(-1, 5), +0x1f0 = 3; into 67
DataDrainSubMenu (67)     while +0x1f0: one less, itemNum =
                          drainItem[+0x1f0], into 29 (which comes back
                          here after 7 frames); then the menu shuts
                          (CloseMenu inline)
```

`ccSpcMessageOpenTreasureBox` (gcmn 0x005a1a10) is `ccAISysMsgSend(0x10010,
-1, plw's AI id, 0xffff, 0, 30)`: a party member's line.

The breakables (34, 35) are broken by Kite first: 4 frames after the fix
`ccPlayer::BreakSomething(0)` (act 25, held), 12 frames on
`AttackCancel`, then the open and the drop; 34 sleeps the others 17
frames later (only on the item's path), 35 27.

```text
ItemObjMenu (0x00546fd0)  itemNum = +0x14c; breakCount (+0x7442) up;
                          rand() % 3: not 0, cmndTargetFix 0 and the menu
                          shuts; 0: 17 frames, asleep, AreaItem(itemNum,
                          3) (areaItemList), into 29
TrapObjMenu (0x00547360)  itemNum, trapNum = +0x7c; 27 frames, asleep;
                          "Set off trap!" with line 1, 2 or 3 as 33's; 12
                          frames; the OK button (sound 18; not Check),
                          Close; AreaItem(itemNum, 3), breakCount up, 29.
                          trapNum -1 or another skill: proccess 4 again,
                          every frame, the menu never ends (the game's)
SymbolMenu (0x00546aa0)   trapNum = +0x7c (ccGimSymbol's random skill,
                          which its own affect 11 casts); "Received
                          effects of #Y<skill>#W!" (getSkillMenuStr);
                          12 frames; Check: symbolCount (+0x7444) up,
                          Close, the menu shut
VirusMenu (0x00546dd0)    itemNum = +0x14c, none: 0xf0000 (key item 0,
                          Virus Core A); into 29
FoodMenu (0x0054a420)     itemNum = 0xf0000 | (base->id + 4): the food's
                          key item (26 - 41); none: 0xf001a; its count
                          (+0x7446 + 2 (item - 26), 16 shorts, at most
                          10000) up; into 29 (menuNext 29 and the list's
                          prev set inline)
TimeIdolMenu (0x00547a50) the time: gameCnt[2] (+0x6c, sixtieths: hours
                          / 216000 to 99, minutes, seconds) as "hh:mm:ss";
                          the best rank so far (+0x6774, 5 when none);
                          CheckTimeIdolRankIn (main 0x001789a0): the first
                          of the five ranks (+0x6775, 40 bytes: the name,
                          then at +20 the time) longer than it, -1 none
                          -1:    one item, 0xb0038; "#YOutside the
                                 Ranking#W"
                          < best: an item per place gained
                                 (timeIdolItem[rank ..], gcmn 0x00651700),
                                 SetTimeIdolRank; "Renewed <place>
                                 #Wrecord.", the place's title
                          else:  one item, 0xb0036, SetTimeIdolRank when
                                 it equals the best; "#YRanked In#W"
                          the message (Open, 0x100, "Current time: #R"
                          and the time first); Check: into 67, which gives
                          the items (no Close)
```

`gameCnt[3]` (+0x64) count sixtieths with the play time (`ccAddPlayTime`,
0x00167740); `ccGame::ChangeRequest` (0x001671e0) zeroes [0] at every
change, [1] when the scene is replaced, and [2] when the area left was a
town (`areaPrev` 0 or -1): [2] is the time since the town's gate.
`SetTimeIdolRank(rank, name, time)` (0x00178870), unless the rank is the
best already, makes it the best and moves the ranks from the old best (4
when none) down to it one place down (`strcpy`, the old bytes past the new
strings kept), then writes the player's name (`saveData` +0) and the time.
A fresh save holds Balmung 00:02:12, Orca 00:02:26, Sieg 00:02:54,
Highlander 00:03:38 and NOG 00:04:45.

The food (`ccGimFood`, gmfood.cpp) is piney-battle's `gimmick::food_new`
and `food_main` ([battle.md](battle.md)'s entry control runs it): it
wobbles and rolls where it lies, turns to face Kite and calls out
(`ccVoicePgFood`) as he comes within 1000, and FoodMenu's affect 11
takes it: off the command lists with `ccSeOn3DNote(179, pos, 70)`, 20
frames on a burst (`effOpenBox`, sound 77, a 30-frame fade and bounce),
gone 30 frames after.

**The port**: `crates/piney-fieldui/src/menus/objects.rs`. Kite's calls
are `Request::BreakSomething` and `Request::AttackCancel`, which the
runtime plays before his next frame (`Combat::kite_menu`, with
piney-battle's `kite::break_something` and `attack_cancel`); `World`'s
`Game::game_cnt` carries `gameCnt`.

## The spring (40 - 42)

The Spring of Myst stands in the lakes (`gimmickTbl` row 20, base type
0x400000; its frames are `ccGimEtc`'s, [battle.md](battle.md) "The spring
and the boss room's warning"). `ccThGameCtrl` opens 40 on it. The three
menus are `FountainMenu` (0x00548060, 40), `FountainMenu2` (0x00548420,
41: the item to throw in) and `FountainMenu3` (0x00548a00, 42: Monsieur's
golden axe game). Their pages are `FountainMenuDisp2` (0x00549ed0) and
`FountainMenuDisp3` (0x0054a260). Each "the spring on" below is
`EntryAffect(spring, plw, 11)`, which moves `ctrlFountain` on a state (0
to 1, 3 to 4, 5 to 6, 7 to 8, 9 to 10). The lines are `fountainMenuHelp[29]`
(main 0x0033ebc0, three pieces each), and the voices are group -20 with
the line's number.

```
40  0  the dim; target fixed and dropped; the greeting
       (line 0) (voice 0); dialogDefault's rows, the first
    1  Select; OK on the first row: into 41; OK on the second, or
       cancel: the menu shuts
41  every frame: the bag's items of categories 0-9 counted (my)
    0  none: the empty-bag line (1), then the menu shuts
       after Check; else y = min(count, 8), dy 0, the page (2)
    2  SelectScr; triangle: sound 18, itemNum, into 64 (prev 41) and
       nothing else that frame; OK: into 42 with the item; cancel: the
       menu shuts. Then the item's name and comment in the window
42  0  the throw-in question (line 2), dialogDefault's rows
    1  OK on "Yes": the spring on; "No" or cancel: back to 41
    2  the menu, dim and map out (a breath if still); the party held,
       the enemies' conditions cleared; the thrown-in line (line 3)
    3  the message shut at 60; when the spring is in state 3: the item
       gone (DelItem), fountainCount up (+0x7470 under area level 4,
       else +0x7472; to 10000) and talkNum 4 or 15; the three answers
       (x 6, y 3, disp 11); the Golden Axe question (talkNum)
    4  OK: the answer kept, the spring on
    5  from frame 11, once the spring is in state 5: Golden or Silver
       Axe: the answer repeated (talkNum + 1, + 2), then 10; Neither: the "neither"
       reply (+ 3)
    6  Check: the pause (+ 4), the guess (+ 5),
       then 7
    7  the item among feTbl[category]'s first f_limitTbl[server (+ 10
       from area level 4, + 5 for category 2)] rows: the spring on, 11;
       else 8
    8  from frame 11: the aside (+ 6)
    9  Check: the level remark (+ 7), the harder-level
       advice (+ 8), the consolation (+ 9)
   10  Check: the spring on
   11  from frame 11, once the spring is in state 7: 12
   12  the spring on; the farewell (+ 10)
   13  Check: the spring on
   14  once the spring is gone: the target free, changeCamera(1), the
       party back
   15  from frame 11: the others asleep, the map in (mapStatus 1);
       Golden Axe: key item 0x3003c; Silver: 0x3003d; Neither: the item
       found in the table becomes the row + 2, + 1 or - 1 on (below),
       held to 0 .. the limit, with the success line (26, sound 92), the
       changed line (27, 91) or the unchanged line (28,
       93); not found: the item back and both axes
   16  Check: 17
   17  frame 7: into 67 (DataDrainSubMenu), which hands the items out
2 <= proccess < 15: SetFountainCamera (0x00526c60) at the end of the
frame; a breath puts it on the next
```

The step for a found item goes by `GetBG` (the lake's bgnum). For weapons
(categories 0-5), bgnum 0 or 3 gives + 2, 1 or 2 gives + 1, and 4 gives -
1. For armour it is the other way round: 0 or 3 gives - 1, 1 or 2 gives + 1,
and 4 gives + 2. `feTbl` (gcmn 0x0069a260) has ten lists of 80 item ids;
`f_limitTbl` (main 0x00307010) has twenty limits. The answers' message
(15) is `Open` with `saveData` itself as the name, so the window has the
player's name above it; the others are `OpenInfo` or `ChangeInfo` with none.

The hold (step 2, gcmn 0x00548d74) is `ccSpcConditionEffectOFF`, then per
party slot `ccStoreSpcCondition`, `ClearCondition`, `ManualModeAI(1)`,
`SetRemoteCmd(0)`, off the command lists, and `transDist` 0 for the
others; then `ccClearConditionAllEnemy`. The release (step 14, 0x005498bc)
is per slot `ccEntryCmnd`, `ccRestoreSpcCondition`,
`ConditionAdjustment`, skill 0, `manualSW` off and `transDist` 1 for the
others unless down; then `ccSpcConditionEffectON`. The port asks for both
as one request (`TalkReq::FountainParty`, piney-world's
`party::fountain_party`).

`SetFountainCamera` puts the event camera (`changeCamera(3)`) on the spring
(`cmndTargetPrev`, z + 100 held to 0 .. 800) from 240 behind Kite, turned 12
degrees (piney-world's `Camera::set_fountain` has the sum). The port asks
for it by `TalkReq::FountainCamera`.

`FountainMenuDisp2` draws `DispSquare(11, y)` (with a scroll bar past
eight) at (39, 96) with the list's `strT`, which is null for 41, so no tab;
then per shown row the cursor, the count (`MakeNum(3)` at 161) and the name
(`menuKanji`, 16 wide). `FountainMenuDisp3` draws `DispTarget` at (35, 96)
with the width of `fountainNameStr[server (+ 5 from level 4)]` ("Monsieur
Lv. n") and the name on `settingKanji[0]` at (60, 101).

`crates/piney-fieldui/src/menus/fountain.rs` is the port.

## Using an item (`ccUseItemRequest`)

TARGET's use (and Items' items used at once, and the Sprite Ocarina) call
`ccUseItemRequest(plw, target, code, 0)` on the menu task, which does not
return until the use is over: it opens and waits on the menu's own window
and breathes the task. For the Fortune Wire (13/0): the target dropped,
sound 228, `effRemoveTrap(pos, -1, -1)`, `CloseMenuDisp` (0x00525ec0:
`menuStatus` and `bgStatus` 3, `panelStatus` 1, `panelAlpha` -24; if
`still`, `WakeAll`, `Disp`, the breath, `still` 0 and the flips back on,
else `Disp` and the breath), the box's word +0x140 put to -1, 20 frames of
`Disp` and breath, "Disarmed trap." (`trapDischargeStr`), `panelStatus` 3,
`bgStatus` 1, `ccSleepAllThread`, `StillOn`, `while (!ccMsg->Check(0))
{Disp; breath}`, `Close`, +0x140 back, and `EntryAffect(box, plw, 12)`.
Only then does TARGET go on (`panelStatus` 1, its tail). piney-battle's
`item::use_item_request` has the whole use as steps; the port's menu task
stops at the call (`Request::UseItem`), the runtime applies the rules and
answers with the steps the same frame (`FieldUi::answer_item`), and the
menu task plays them where the call stood: its own (the windows, the
statuses, the sleeps, `Frames`, `WaitMessage`) itself, the world's (the
affects, the skill, the heal and trap effects, the pauses, the messages to
the party) back to the runtime one at a time (`Request::ItemStep`), in
the game's order. With no answer (the town, `tools/test_fieldui_rs.py`
where the call is hooked) the call does nothing and the menu goes on at
once, as the game's with the call returning. The Grunty Flute's use
(`Request::UseItemArg`, [grunty-ride.md](grunty-ride.md#the-call)) waits
on the task too: `WaitParty` and `WaitRide` breathe with the chat
balloons alone (no `Disp`: the menu shows nothing) while the party is down
or the game over, and while `pgRideFlag` is set; `Pucciguso(n)` goes to
the runtime (`ccPuccigusoStart`) and breathes, drawing nothing, while its
fades run (`World::pg_starting`).

The Fairy's Orb (13/2) waits on the task as well: sound 97, 7 frames, its
information line, 7 frames, then `WORLD_MAN::ShowMap()` once a frame
([map.md](map.md#showmap-the-fairys-orb)) with a `Disp` and breath after
each call that is not yet done, and more `Disp` and breaths until 20
frames have passed in all; then the window closes. The port's `WaitMap`
sends the first call to the runtime (`Request::ItemStep`) and reads each
answer in the next frame's `World::map_showing`.

## PERSONAL's pages

What PERSONAL's rows open, besides Skills and Items: Key Items (6),
Discard Item (7), Status (8, with a member's items 31 and a piece's status
64), Equipment (63), Party (9, in town), Area Information (87, in a field
or dungeon), Gate Out (10) and Log Out (11); and 86, which the field opens
itself. Every page goes back the same way: `menuNext = prev`, the window
out, `proccess` and `waitCount` 0, `InitCursol(1)` on both windows. A
page's decide / back test reads the pad's `push` against
`assignPADcancel` first (sound 19), then `assignPADok` (18).

### Key Items (`ImportantItemMenu`, menu 6)

The list is `saveData.impItemList[291]` (+0xcfc, a count per key item) by
page (`SetImportantItemList`, 0x005274f0): 0 Event Items (ids 42-73 and
281-290), 1 Grunty Food (26-41), 2 Virus Cores (0-25), 3 the Book of 1000
(273-280); two pages before the bracelet's colours (+0x6771), three in
Parody Mode (+0x842b), else four.

```
proccess 0   target fixed and dropped; the pages; the dim in
proccess 1   SelectScr; cancel back; OK by ccCheckItemUseful:
             0: the help; in town only 12 and 15 (help 1);
             2 (used at once): the Grunty Flute (15/49) only in a field
             (help 2), not on fields 13 and 67 (help 4), not in battle
             (help 5): proccess 20; a Ryu Book (15/273-280) only in town
             (help 8); else the window out, proccess 2;
             1: TARGET (65); else the item's name and comment
proccess 2   the window gone: ccUseItemRequest on the player. A Ryu Book:
             menuFade->EntryFade(1, black, black), CloseInstant, the tasks
             woken for one frame, asleep and frozen again (as OpenMenu),
             ContinueFade(10, clear), three frames after CheckFade; else 8
             frames, the list again
proccess 10  the help as an information window, Check, 8 frames
proccess 20  the Grunty Flute: from slot rand() % 3 round the three, the
             first Grunty ccPgAdultCheck(server, slot) finds (n >= 0);
             ccUseItemRequest(plw, plw, 0xf0031, n) - the ride - and the
             menu shut; none: "There are no Grunties you can call ..."
             (two lines)
```

`ImportantItemMenuDisp` (0x005305d0): the tabbed window (`DispSquareTag`,
the page's tag from `impItemMenuTag`) at (39, 96), the scroll bar, L1 / R1
at (55, 64) and past the window, the rows at (67, 112 + 20 r) with their
counts at (235, ...); a row the page cannot use now in colour 0.

### Discard Item (`ThrowItemMenu`, menu 7)

The player's items as the Items menu shows them (`SetItemList`,
`ItemMenuDisp`, five pages):

```
proccess 0   as Items
proccess 1   SelectScr; triangle on equipment: its status (64); cancel
             back; OK on an item: the count (exceptionDisp 2)
proccess 2   up / down one, left / right ten (1 to the count held);
             cancel back; OK: the window out
proccess 3   "Discard N #Gitem#W." with OK / Cancel (list disp 11)
proccess 4   Select; OK on OK: ccSaveData::DelItem(0, cat, id, N)
proccess 5   the list again
```

`ThrowItemMenuDisp` (0x00531820): `ItemMenuDisp`, then with
`exceptionDisp` 2 the count's window (`DispSquareW2(5, 2, 1)` on
menuWindowPr), its cursor, "Discard" (`shopStr`'s cell 6) and the count
in font colour 22.

### Status (`StatusMenu`, menu 8), a member's items (31), a piece (64)

The page is a grid the cursor walks (`list.sx` the column, `list.sy` the
row): the equipment (column 0, rows 0-4), the skills (columns 1-2, rows
0-7), the added effects (rows 8-10), the parameters (rows 11-14, columns
0-3); L1 / R1 through the party's members (the list's `page`).

```
Status   triangle on a piece of equipment: its status (64, itemNum the
         piece: job << 16 | weapon, or 6 - 9 << 16 | armour); square: the
         member's items (31, temp[0] the member); cancel back; on every
         move the row's help (ccMsg->Change at (39, 334)):
         statusMenuHelp[sx * 15 + sy], a skill's own help, an added
         effect's (statusBeffStr) among those the character has
31       Items (SetItemList of the member), five pages; triangle on
         equipment: 64; cancel back
64       the piece: its name and comment, its three skills, its added
         effects, its parameters; cancel back
```

`StatusMenuDisp` (0x005362d0): the frame (39, 16) 29 x 16 with its rules;
the member's panel (`FacePanelDisp` at (39, 0)); `LevelDisp` at (40, 80);
the equipment's labels and names (settingKanji 6) with their icons;
`SkillDisp(220, 30)`, `BeffDisp(220, 196)`, `ParameterDisp(40, 270)`; the
Items button; L1 / R1 by the panels with more than one member; the cursor.
`ItemStatusMenuDisp` (0x005462a0) is `ItemMenuDisp` for the member (the
item box's count only when the member is the player), the member's name
over it. `EquipStatusMenuDisp` (0x00539ca0) draws the piece with the same
boxes.

The boxes all three pages share: `ParameterDisp(x, y, now, was)`
(0x00523e50): the physical and magical abilities, the elements and the
tolerances of `now` (a `ccCharParamElement`, a tenth shown by
`MakeSignedNum(3, v / 10)`), each in colour 20 when it rose over `was`, 23
when it fell (`ccGetParamColor`, both at most 990); `SkillDisp(x, y,
list)` (0x00524e20): "Skills:" and up to sixteen skills, eight a column,
each in its colour (the list is pairs of skill and colour);
`BeffDisp(x, y, cono, con)` (0x005251f0, and 0x00525150 taking
`ccCondition`s): "Added Effects:" and the names of the effects either has,
colour 20 for one only `con` has, 23 for one only `cono` has, else 7.

### Equipment (`EquipmentMenu`, menu 63)

On member `equipSpcNum` (PERSONAL opens it on Kite, 0, with `firstTime`),
one slot a page (`list.page`): the job's weapon (0), head (1), body (2),
hands (3), legs (4). The candidates are the member's items of the page's
category, not the one worn, of a level (`ccEquipParam.level`) the job may
wear: jobs 0-5 wear weapon categories 0-5 up to levels 2, 3, 3, 3, 2
and 1. The cursor is in one of three places (`list.index`): the piece
worn (0), the candidates (1, `list.select`), or the grid of skills, added
effects and parameters (2, `sx` / `sy` as Status's).

```
proccess 0    exceptionDisp 1, the rows shown (at most 8); the first
              time all back to the top
proccess 1    R1 / L1 the page (17, push); the moves; cancel back; OK on a
              candidate (18): refused when 99 of the worn piece are carried
              (warning 1) or, the list full, none of it carried and two or
              more of the new one (warning 0); else DelItem (the new),
              AddItem (the old), ChangeEquipment, CalcReal(1) and
              ccThEquipMenu (ccSPC::ChangeEquip: the model), the cursor
              back on the piece worn; triangle on the piece worn or a
              candidate: its status (64)
proccess 2    the model task done: back to 1
proccess 10   the window closed: equipMenuWarn[waitCount]
proccess 11   Check: the window back, proccess 0
```

On every move the help: a skill's own, an added effect's, the grid's
`statusMenuHelp[sx * 15 + sy]`, `statusMenuHelp[60 + index]` on the piece
worn and the candidates.

`ccEquipChangeMenuSub(id, cond, elm, sklist, category, item)` (0x00538de0)
is what the page shows for the piece under the cursor: on copies of the
member's equipment and skills, `ccChangeEquipment`; the added effects the
pieces would give (the largest of each); `elm` + the pieces + `temp`; the
skills as pairs: the ones had now first (7 kept, 23 lost), then the ones
gained (20). It reads `ccGetEquipParam` rows without a check: an empty
slot (-1) reads the row before its table (the port keeps the tables'
bytes, that row included). A job outside 0-5 takes the leg piece's
effects and the arm piece's parameters again in place of a weapon.

`ccChangeEquipment(equipment, skills, category, id, spc)` (main
0x00177070): the old piece's skills that no other piece worn carries
(`ccCheckSkillCount` < 2) dropped, each from the first place it is in;
the piece put on; its skills added (`ccAddSkill`: Data Drain's 2-5 are
one, the list sorted, lowest first); the sets' skills: all four armour
pieces 60, 61, 62 or 63 add 291-294 (else all four go), head 68, body 67,
arm 66, leg 67 and weapon 8 add 289 (else it goes).

`EquipmentMenuDisp` (0x00538130): the frame (39, 16) 29 x 16 and its
rules; the page's four labels (`equipChangeStr`, colour 17) and the piece
worn at (66, 50) with its icon; the candidates (at most `list.y`, from
`dy`) at (66, 106 + 19 r) with their icons and the scroll bar; then the
preview: `SkillDisp(220, 30)`, `BeffDisp(220, 196)` (the character's
effects against the preview's), `ParameterDisp(40, 270)` (the preview's
against `real`); the grid's cursor; L1 / R1.

### Gate Out, Log Out (10, 11), back to the field (86), Area Information (87)

```
Gate Out  proccess 0   in battle: "Cannot use during battle." and back
                       after Check; else "Return to town." with OK / Cancel
                       (Cancel first)
          proccess 1   OK on OK: the window out, the screen to black over
                       20 (menuFade->EntryFade(20, clear, black)); Cancel
                       or cancel: back
          proccess 2   the fade done: CloseMenu; ccSPC::DeleteNoPartyMember,
                       ccGame::ChangeArea(0, town)
Log Out   as Gate Out without the battle check; at the end
          ccGame::ChangeRequest(4, 7): The World's top page
86        the window and the dim out, the tasks woken; 121 frames; then,
          unless the party is wiped out (checkPartyAnnihilation) or the
          game is over, CloseMenu and WORLD_MAN::GoField
87        exceptionDisp 4, temp[0..3] = WORLD_MAN's A, B, C (the area's
          keywords); the Chaos Gate's keyword page (GtNewMenuDisp) shows
          them; cancel back
```

### The port

`crates/piney-fieldui/src/menus/`: `keyitem.rs` (6, 7), `status.rs` (8,
31, 64 and the shared boxes), `equip.rs` (63), `leave.rs` (10, 11, 86,
87), `personal.rs` (what they share: the texts, the menu fader
`ccScFade`, and the frames a handler resumes after it breathed itself,
`After::Pers`). The fader is `piney_demo::fade::ScFade`, drawn after the
message window. `FieldUi::set_area_words` gives Area Information the
area's keywords (the runtime's `WORLD_MAN`). What the pages ask of the
rest of the game, the save already written:

| request | game call | the runtime |
| --- | --- | --- |
| `UseItemArg { target, code, arg }` | `ccUseItemRequest(plw, target, code, arg)` | the Grunty Flute (0xf0031) calls the Grunty `ccPgAdultCheck` answered (`arg`): the menu held in the call through the ride ([grunty-ride.md](grunty-ride.md)) |
| `DeleteNoPartyMember`, `ChangeArea { area: 0, n }` | Gate Out's | the members not in the party gone; to town `n` |
| `ChangeMode { num: 4, sf: 7 }` | `ccGame::ChangeRequest(4, 7)` | Log Out: The World's top page |
| `GoField` | `WORLD_MAN::GoField` | 86: back to the field |
| `CalcReal { target }` | `ccChar::CalcReal(1)` | Equipment: the member's added effects and maximum HP and SP again from its record (`FieldUi::calc_real` does what the menus see of it) |
| `ChangeEquip { target, cat, item }` | `ccThEquipMenu`: `ccSPC::ChangeEquip` | the new piece's model on the member |

`ccThEquipMenu` is a task in the game; the port takes it as done the next
frame (the shortest the game can take).

**`ccSPC::ChangeEquip(cat, member, item)`** (gcmn 0x0059f9e0) switches on
the category (below 10): the weapons (0-5) go to `ChangeWeapon`, the
armour to `ChangeHelmet`, `ChangeArmor`, `ChangeGlove` and `ChangeBoots`,
which change nothing drawn. `ChangeWeapon` (0x0059fb50) works on the
member's 44-byte row of `ccSPC`:

- +0x18 is the weapon worn, +0x0c the one coming, +0x0e the one going,
  +0x1c the character, +0x24 and +0x28 the worn and the coming file.
- A new weapon: the file of the weapon still coming, if any, is let go
  (`ccFileListDeleteOne` on `spcTempFileList` type 7); the new one's file
  (`ccGetJobWeaponParam(job, item)` +4) is loaded (`ccLoadFLAddOne`), and
  with a character built its address goes to +0x28 and the character's
  `weaponChangeSW` (+0xe1 bits 0-1) to 1. Without one, to +0x24.
- Back to the weapon worn while another was coming: the coming file is let
  go, +0x0c and +0x0e cleared, +0x28 0 and the switch 0.

The character's own frame hangs it (`ccPlayer::Main`, `ccFellow::Main`;
[battle](battle.md#a-party-members-frame)): the switch at 1 runs
`EquipWeapon` on the coming file and leaves -1; the frame after,
`DeleteWeaponCCS` has `ChangeWeaponOldNewCcs` (0x0059fdb0) let the old file
go, the coming file becomes the worn one, and the switch 0.

The port: `FieldWorld::change_equip` and the town's `World::change_equip`
hang the save's weapon file (`body::weapon_of`, the record the menu
wrote) on the character's hands at once, Kite's (`Kite::arm`) or a
member's (`Body::equip_weapon`), a frame before the game's character
frame would. The combat reads back at the start of each frame every
record the menus changed in the save ([battle](battle.md#the-partys-hp-and-conditions-from-scene-to-scene),
the note on the records), so the menu's writes and `CalcReal`'s hold in
a field as in a town.

## Not the game's: the HUD scale

`FieldUi::hud_scale` (the port's `--hud-scale`, 0.5 to 1) draws the HUD
smaller. Each packet carries an anchor (`piney_fieldui::spr::Anchor`, in the
menu's 512 x 448 space), which `Disp` sets on every sprite before each part:

- the party panels (`panels`): the bottom left;
- the target's window (`target_window`) and the new-mail mark: the top left;
- the bracelet's gauge: the top right;
- the battle band: its height, toward the top;
- the battle announcement: the centre.

The menus, the message window, the world-anchored marks (enemy bars, the
target cursor, chat balloons, damage numbers) and the dim keep their size.
The renderer scales a packet's position and size about its anchor, and
pulls its texture coordinates in by half a texel, so that a bilinear
sample at a fractional edge stays in its cell. The field's minimap is
shrunk toward the top right after it is drawn: its sprites, scissors, and
far edges taken from the next tile's start, since its terrain tiles stop
1/16 pixel short of each other. At 1 nothing is touched, and the frames
are the game's.

## The Ryu Books (`BOOK`, gcmn book.cpp)

A Ryu Book (key item 273 + n, read in a town) runs `ccThBook`
(0x0041a990): a fade, the cover stream 112 + n, then `BOOK` (0x4d24
bytes) one frame at a time: `Draw` (the page's `DispNN`), `CheckItemGet`
(the page's rewards, `GetBookItem` and its windows, breathing inside),
`PadControl` (the page's keys, then cancel closes the book unless a
sub-window is open, `cancelFlag` is set or `wait` is not 0). `wait` starts
at 15 and only the page's keys count it down; a reward is checked on the
frame it is 1. Counters past the volume's cap (`CheckBookLimit`) are
drawn in colour 18 and stop giving rewards.

A reward's windows breathe inside `CheckItemGet` (`Draw; Breath` until
`Check(0)` answers). So the frame a reward starts draws the book twice, the
loop's `Draw` and then the first window's, and the frame one ends draws it
not at all: its last `Check` answers and the loop goes on to its next
`Breath`. Run in eemu, Book I's first reward sends 36 window and 2
background packets on its first frame (18 and 1 otherwise) and none on its
last. The port draws the book once on every frame instead (not the game's;
`piney_fieldui::book::Book::as_the_game` restores the game's draws, which
`tools/test_fieldui_rs.py` uses).

| book | page | counts (`saveData`) | keys |
| --- | --- | --- | --- |
| I | `Disp01` | areas visited +0x6862 (one a gate trip), the play time | rows |
| II | `Disp02` | portals opened +0x6864, fields and dungeons all opened +0x6866, +0x6868 | rows |
| III | `Disp03` | characters met (67), trades; a sub-window of a character's items | list |
| IV | `Disp04` | enemy kinds slain of 303 (`enemyKillCount`, +0x68b7); a sub-window of the enemy's `enemyTbl` row and where it was last slain (`enemyKillArea`, +0x69f0) | list |
| V | `Disp05` | gold given to the 17 members (`present`), each one's | list |
| VI | `Disp06` | boxes opened +0x7440, objects broken +0x7442, idols opened +0x746e | rows |
| VII | `Disp07` | springs +0x7470, the mushrooms' grandfather +0x7472, symbols +0x7444 | rows |
| VIII | `Disp08` | a menu: the nine Grunties met (+0x7474), the sixteen foods given (+0x7446) | menu, list |

Rows: down and up pushed (`PadControl01`, `02`, `06`, `07`). The lists
(`PadControl03`-`05`, `08`'s food) put the cursor only on rows known (met,
slain, given); off the list IV and V move with the held buttons and
`keyWait` (every third frame), VIII's food total down by repeat and up
held. Book IV's search runs one past its 303 rows into `checkValue[2]`,
which `Draw` zeroes. Book VIII's food help line is the food's name and
`bookPuchiFood`: its count's `sprintf` is overwritten by the `strcpy`
after it. Opening VIII's pages resizes the window through `BookOfs`
(40, 17, 13 or 12), and going back sets (80, 12, 4); the table is the
game's global and keeps the last values.

The port is `crates/piney-fieldui/src/book`; `tools/test_fieldui_rs.py`'s
`test_book_1` to `test_book_8` run each book against the game's own code.

## PARTY (`PartyMenu`, menu 9; 68 - 70)

PERSONAL's Party row in town (row 6). The members' greetings and
farewells are event records per member (`spcMsgPartyIn10`, `11`, `20`,
`21`, `spcMsgPartyOut10`, `11`, gcmn 0x00637f90 on) opened with
`ccMsg->Open(record, spcParam[id].base.name, -30, id * 3 + n)` (the voice
of group -30: n 0 a greeting, 1 the idle one's, 2 a farewell); the "11"
and "21" tables when `talkNum[id]` (+0x220c) is set.

```
9    outside a town: "You can only form parties in towns." and back; else
     Add greyed with three in the party, Remove and Disband alone
     (reverseHead bits 0; 1 and 2), the rows into menuKanji, the target
     dropped; Select (the help fading in again on a move); cancel back,
     OK into the row; the row's help (partyMenuHelp, DispMsg)
68   refused with three ("Only 3 members to a party ...") or none to call;
     the members of partyMemberFlag (+0x2220) not in the party, at most 8
     rows shown (the page is PartyInMenuDisp, faceNum the one under the
     cursor); OK: the window out, "Add <name> to your party." OK / Cancel;
     OK: "Calling party member.", then
       when partyMemberCall (+0x2224) has the member and its character
       can be loaded (fewer than five in ccSpcManager, or it is one of
       them): ccThPartyAdd (ccParty::AddMember, from a task), the new
       slot's face, 40 frames, the message closed, at 51 the greeting
       (the "20" tables when its character was loaded and idle,
       bootParam 0), Check, the menu shut
       else: 41 frames, "There is no response to the Flash Mail. Player
       is absent from The World." (ChangeInfo), Check, the list again
69   refused alone; the members but Kite (list.page the slot chosen); OK:
     "Remove <name> from your party." OK / Cancel; OK: the farewell,
     Check, ccParty::DelMember(slot), the menu shut
70   refused alone; "Removing all members." OK / Cancel; OK: for slots 1
     and 2 in turn the member's farewell, Check, DelMember; the menu shut
```

`ccParty::DelMember(slot)` (0x0059cf60): for slot 1 or 2 held, the slot's
character cleared, `disbandSpc(id)`, the id -1, the count down (the slots
are not packed).

The port is `menus/party_menus.rs`. `World::spc` is `ccSpcManager`'s
registry (member id and `bootParam` of each character loaded) for Add's
tests; `Request::AddMember(id)` asks for the member (the runtime runs
`ccParty::AddMember` and reads the party again before the next step, as
the game's task has run by then); `Request::DelMember(slot)` for
`ccParty::DelMember`, the menu's own `World` already without the member
that frame (the game's `Disp` of that frame sees it gone).

## The OPTION pages (menus 13 - 20)

OPTION's list (menu 12, `SystemMenu`, START) holds, top to bottom,
Controller (13), Vibrate (14), Adjust Screen (15), Sound (16), Data Drain
(18), Voiceover (19), Movie Text (20) and Title Screen (17). `ChangeMenu`
goes into one; every page comes back the same way: `menuNext = prev` (12),
the window out, `proccess` and `waitCount` 0, `InitCursol(1)` on both
windows. The lists (`menuElementData`): 13 disp 4, x 6, y 4, "CONTROLLER";
14, 15, 18, 19, 20 disp 4 with no rows (the page sets them); 16 disp 4,
"SOUND"; 17 disp 11 (the OK / Cancel dialog), x 6, y 2. The pages with
their own drawing are `ExceptionDisp`'s: 13 `ControllerMenuDisp`, 14
`VibrationMenuDisp`, 15 `ScreenMenuDisp`, 16 `SoundMenuDisp`, 18
`DataDrainDemoMenuDisp`, 19 `VoiceMenuDisp`, 20 `StrwinMenuDisp`. Inside
`Disp`, `font` is menuFont, so what a page writes with `font` goes to
menuFont.

The pages' decide / back test is the menus' usual one: `push &
assignPADcancel` first (sound 19), else `push & assignPADok` (18).

### Controller (`ControllerMenu`, 0x0053d0e0)

```
proccess 0  exceptionDisp 0, menuStatus 3 (the window out); ctrlMenuStr (A-1,
            A-2, B-1, B-2) into menuKanji; select = index = camType (+0x8429)
proccess 1  ccStartThread(ccThControllerMenu, 34, 2048), its tscb +0x14 = 1;
            Disp and breathe while +0x14 is set (the task: ccLoadFLAddOne
            (flContMenu), then +0x14 = 0); ccDeleteThread;
            ccStream::GetCCSAdrs("xcontrol");
            menuMask->SetTex("xcontrol", "TEX_xcontrol", 1);
            menuStatus 1, exceptionDisp 1;
            ccMsg->ChangeInfo(ctrlMenuHelp[index]'s two lines), the window at
            (39, 444)
proccess 2  Select(0, 0, 0); cancel: Close, menuStatus 3;
            OK: index = select, camType = select, setCameraCtrlType(select),
            the help again
proccess 3  when menuStatus is 0: exceptionDisp 0, Disp and breathe twice,
            ccFileListDeleteOne(flContMenu), back to OPTION
```

`setCameraCtrlType(t)` (main 0x001611a0) sets `camCtrlType` and `camRevLR`:
A-1 (0) 1 and 1, A-2 (1) 1 and 0, B-1 (2) 0 and 1, B-2 (3) 0 and 0, and
`cameraControlType` = t (`camRevUD` 0). The page writes only `camType`;
the button assignment (+0x8404 on) is not changed here, the button labels
are fixed texts.

`ControllerMenuDisp` (0x0053d550): the list at (39, 56), rows at (53, 72 +
20 r), the scheme in force (`index`) in colour 6; the picture (menuMask,
xcontrol's 256 x 128 texture, cell at (0, 0)) at (88, 168); boxes round it
(`DispSquare`, colour 0 frames): the buttons at (320, 128), 9 x 4, with
settingKanji 0 (`ctrlMenuStrBtn`); START and SELECT at (179, 300), 10 x
2; the left stick at (39, 300), 8 x 2, settingKanji 1 (`ctrlMenuStrMov`);
the right stick at (347, 300), 7 x 2, settingKanji 2 (`ctrlMenuStrCam`);
the shoulder buttons at (165, 56), 20 x 2. The camera's labels follow
`camType`: A-1 and A-2 zoom with the right stick (Zoom In, Zoom Out),
rotate with L1 and R1 and reset with R2; B-1 and B-2 rotate with the
stick, reset with L1 and zoom with R1 and R2; L2 changes the view in both.
The rotation arrows (grid 1 cells 14 and 32) sit by L1 and R1 for A, round
the right stick for B; the left ones mirrored (ctrl 0x20), A-2 and B-2
upside down (ctrl 0x40, which `MakePacketStr` does by swapping the quad's
two V rows).

### Vibrate, Data Drain, Voiceover, Movie Text

`VibrationMenu` (0x0053ea80), `DataDrainDemoMenu` (0x00540730),
`VoiceMenu` (0x00540d20), `StrwinMenu` (0x005412e0), the same code over
another save byte and texts:

| menu | byte | rows | row 0 is | texts |
| ---: | --- | --- | --- | --- |
| 14 | `vibration` +0x8428 | `dialogOnOff` | not 0 | `vibrationMenuInfo` (one line) |
| 18 | `drainDemo` +0x8427 | `dialogOnOff` | not 0 | `datadrainDemoMenuInfo` (two lines) |
| 19 | `voice` +0x842c | `dialogVoice` (English, Japanese) | 1 | `voiceMenuInfo` (one line) |
| 20 | `strWinMode` +0x8430 | `dialogOnOff` | not 0 | `strwinMenuInfo` (one line) |

```
proccess 0  the rows into menuKanji; the list x 6, y 2; select the row the
            byte stands for; ccMsg->OpenInfo(its text); exceptionDisp 1;
            proccess 1 at once
proccess 1  Select(0, 0, 0); cancel: Close, back to OPTION;
            OK: the byte 1 (row 0) or 0 (row 1), ChangeInfo(its text),
            InitCursol(1) on both windows. Vibrate also sets
            ccPad::actuaterSw the same and, switching on, buzzes once:
            ccPad::SetActuater(&ccSys.pad[0], 1, 160, 200)
```

The pad's motors themselves are `piney_input::actuator`: `SetActuater`'s
queue, `ccSystem::Ctrl`'s countdown and `ccPad::Ctrl`'s sends. The
runtime keeps `actuaterSw` equal to the save's `vibration` (the load, the
new game and both Vibration menus keep the two equal). It turns each
`Event::Actuate` (this buzz, and `ccPlayer::DamageActuate` on a hit on
Kite) into that queue, and plays the sends on a gamepad through gilrs'
force feedback: the large motor as the strong rumble at its power, the
small one as the weak rumble.

The page (`...MenuDisp`): the window at (200, 248), rows at (214, 264 + 20
r), the cursor on `select`; row 0 in colour 6 when the byte is its value
(Voiceover: 1; the others: not 0), row 1 in colour 6 when it is 0 (a
Voiceover value of 2 marks neither).

### Adjust Screen (`ScreenMenu`, 0x0053f060)

```
proccess 0  bgStatus 1 (the dim in again); sx = screenX + 48, sy = screenY +
            16 (+0x841a, +0x841c); exceptionDisp 1; waitCount 0
proccess 1  waitCount = (waitCount + 1) % 30;
            SelectXY(0, 96, 0, 32, 3, 3, 1);
            ccSystem::SetDisplayOffset(sx - 48, sy - 16), and the save the
            same, every frame; cancel: back to OPTION (no OK)
```

`SelectXY(x0, x1, y0, y1, lim, dx, dy)` (0x00526730): the repeat bits
down / up move `sy` by `dy`, right / left `sx` by `dx`, sound 17 each;
past an end it stops there (`lim` bit 2 for y, 1 for x) or wraps.

`ScreenMenuDisp` (0x0053f250): a 6 x 2 box at (200, 188); in menuFont (ccFont
type 1, ctrl 0x10 shadowed, which stays set on menuFont afterwards) "X :"
at (216, 204) with `MakeSignedNum(3, sx / 3 - 16)` and "Y :" at (216, 224)
with `MakeSignedNum(3, sy - 16)`; twelve marks on menuWindowA (colour 10,
the one-texel cell at (1344, 2064) stretched), the screen's corners and
centre, their alpha `min(alpha, 16 + 128 w / 15)` with w the 30-frame
`waitCount` folded at 15.

### Sound (`SoundMenu`, 0x0053f800)

```
proccess 0  exceptionDisp 1; the list y 4; +0x18c = mainVol, bgmVol, seVol
            (+0x841e, +0x8422, +0x8420) * 32 / 256 and output (+0x8424)
proccess 1  Select(0, 0, 0); left: the row's value down to 0, right: up to
            32 (output: 1), sound 17 when it moved; cancel: back to OPTION;
            a value moved (even with cancel): the save = value * 256 / 32
            (the output as it is), ccSaveData::SetSoundEnv
```

`SetSoundEnv` (main 0x00178600) hands the save's volumes and output to
`ccSetMainVol`, `ccSetBgmVol`, `ccSetSeVol` and `ccSetOutputMode`.

`SoundMenuDisp` (0x0053fbc0): the window at (152, 96), 12 x 10,
"SOUND"; `soundMenuStr` in menuKanji (alpha, not kanjiAlpha): Main, BGM
and SE Volume at (166, 116 + 50 k), each with `DispSlideBar(8, value, 32)`
at (178, 132 + 50 k) and the cursor (12 wide) on its row; Output at (166,
264); Mono at (178, 280) and Stereo at (250, 280), the one in force in
colour 6 with the cursor (4 wide) on it when the row is selected.

`ccMenuWindow::DispSlideBar(w, n, max)` (main 0x001b97f0): at (dx, dy)
grid 1 cells 11, 12 (`w` cells wide) and 13; the knob (29) at (dx + 1 +
f, dy + 6) with f = n (14 w + 18) / max; the fill (colour 22, the cell at
(1344, 2048)) f wide and 8 high at (dx + 6, dy + 4); colour 7 and grid 1
after.

### Title Screen (`ResetMenu`, 0x00540270)

```
proccess 0  bgStatus 1; dialogDefault (OK, Cancel) into menuKanji; Cancel
            selected; ccMsg->OpenInfo(resetMenuInfo[0])
proccess 1  Select(0, 0, 0); OK on Cancel, or cancel: Close, back to
            OPTION; OK on OK: Close, menuStatus 3
proccess 2  the window gone: menuStatus 1, OpenInfo(resetMenuInfo[1]'s two
            lines: "Data not saved will be lost." "Proceed?"), Cancel
            selected
proccess 3  as 1, but OK on OK: ccGame::ChangeRequest(1, 7)
proccess 4  nothing: the mother task leaves the field for the title
```

The rows and cursor are `Disp`'s list window (disp 11) at (200, 248).

### The port

`crates/piney-fieldui/src/menus/option.rs` and `option/disp.rs`. The
Controller page's picture is read from the disc with the other textures
(`xcontrol::TEX_xcontrol`, the names from gcmn), so the loading task is
modelled as finishing in its first run: the flag is clear the frame after
the start, the shortest wait the game can have. The page's own breaths
(that wait and the close's two) are `Flow::Breathed` tails
(`After::Option`): they go on before the task's loop next frame, as the
game does, so an `openReqNum` set meanwhile waits for the page. What the
pages ask of the rest of the game comes out as `Request`s, the save
already written:

| request | game call | the runtime |
| --- | --- | --- |
| `DisplayOffset { x, y }` | `ccSystem::SetDisplayOffset` | moves the picture (x -48..48 by 3, GS DISPLAY DX units; y -16..16 lines), every Adjust Screen frame |
| `SoundEnv { main, bgm, se, output }` | `ccSaveData::SetSoundEnv` | sets the sound driver's volumes (0-256) and output (0 mono, 1 stereo) |
| `Vibration { on }` | `ccPad::actuaterSw = on`, `SetActuater(pad 0, 1, 160, 200)` when on | turns the pad's vibration on or off, and buzzes once switching on |
| `CameraType(t)` | `setCameraCtrlType(t)` | the field camera's control scheme (A-1, A-2, B-1, B-2) |
| `ChangeMode { num: 1, sf: 7 }` | `ccGame::ChangeRequest(1, 7)` | leaves the field for the title |

The Data Drain, Voiceover and Movie Text bytes are only written; the code
that reads them (the Data Drain demo, the voices, the movies' text window)
reads the save.

The sprites' ctrl bits 0x40 (upside down) and 0x10 (shadow) are carried
in `Spr` and `Packet` (`flip_v`, `shadow`) and drawn: a queue with an
upside-down cell is sent with each such SPRITE's V rows swapped.

## Talking and the shops (21 - 27, 47 - 56)

The action button on a walking PC opens 22, on a merchant 24 (weapons
0x100, items 0x400, magic 0x800), 25 (the Recorder, 0x1000) or 26 (Elf's
Haven, 0x200), all with `mode` 1 and `firstTime` 1 (`ccThGameCtrl`). The
lists (`menuElementData`: 22 Talk, Trade; 24 Talk, Buy, Sell; 25 Talk,
Save; 26 Talk, Store Items, Withdraw Items) are target lists (`disp` 6).
Everything about the character comes through `cmndTarget->base`, which for
a town NPC is its `npcTbl` row (gcmn 0x00619460, 0x70 bytes: name, type,
id = the row, `msg`, the table of its lines). The shop pages drop the
target (it becomes `cmndTargetPrev`) and put it back on the way out.

### The lists (`PcMenu` 0x00541e90; `VenderMenu` 0x00542840, `RecorderMenu` 0x00542df0, `FairyshopMenu` 0x00543390)

```
merchants   proccess 0: cmndTargetFix; no target: CloseMenu. The rows'
            names (16 glyphs each). firstTime: EntryAffect(cmndTarget, plw,
            14) and ccMsg->Open(base->msg[game.server], base->name, -1, -1),
            the cursor on Talk. talkNum 1; the minimap out (mapStatus 3);
            SetMerchantCamera (0x005269d0)
            proccess 1: Select; the target gone: changeCamera(1), minimap
            back, CloseMenu, ccMsg->Close - then the keys run next frame.
            Cancel (19): EntryAffect 0, changeCamera(1), minimap back,
            CloseMenu, ccMsg->Close. OK (18): off Talk the other tasks
            asleep (ccSleepAllThread and StillOn: the noise, still and game
            layers kept), (Vender only) the list's index = base->type,
            ccChangeCmndTarget(0); ChangeMenu; ccMsg->Close
PcMenu      as the merchants' up to the greeting, by the PC's id: 66-79 (the
            trading PCs) msg[0], talkTradeFlag 0; 30-65 msg[0], talkNum =
            rand() % 3, talkLoopCnt 0, two rows; any other id msg itself as
            the record, talkNum 1, one row (the rows are built before the
            row count changes). The PC's trade count started
            (CheckTradeCount < 0: AddTradeCount). A trading PC's talkNum is
            the first of its three trades still open
            (tpcTradeListSW[id - 66][k], +0x1e7c), from a random one the
            first time, else from talkNum; -1 when none is; two rows.
            proccess 1: no target or in battle: CloseMenu; cancel:
            EntryAffect 0, CloseMenu; OK: off Talk the target dropped;
            ChangeMenu; ccMsg->Close
```

`ccSaveData::CheckTradeCount(type, id)` (main 0x00178680) reads
`pcTradeCount` (+0x686a) at `id - 1` for a party member (`type & 4`) and
at `id - 13` for a PC 30-79, else -1; `AddTradeCount` (0x00178700) adds one
there, capped at 99 as a signed byte, and counts on entry 0 for anyone
else.

`SetMerchantCamera`: `changeCamera(3)` (camID 3, `activeCamPtr =
cameraList[3]`), then camera 3's view = the merchant's pos plus (0, 0,
100); its rotation the merchant's `dirc` with y less 0.1309 (7.5 degrees)
and z less 1.3708 (pi / 2 - 0.2), z wrapped into -pi .. pi; its position
= view + RotZ(z) RotY(y) RotX(x) (500, 0, 0); its rot then from
`cameraGetRot` (x = atan2f(pos.z - view.z, the flat distance), z =
atan2f(view.x - pos.x, pos.y - view.y)). A breeder (0x8000000) takes
`breederCamView` / `breederCamPos[game.server - 1]` instead.
`changeCamera(1)` makes the field camera active again.

### Talk (`TalkMenu`, 0x0054de30, menu 47)

```
proccess 0  no target: CloseMenu (proccess still goes on to 1). base->msg
            0: back to the list. Else EntryAffect(cmndTarget, plw, 15) and
            the line, by base->type:
  4 (party) msg[saveData.talkNum[id]][talkNum] (+0x220c), voice (-31, 2 id + 1)
  8, 66-79  unless talkTradeFlag, talkNum the next open trade after it;
            none: tradeMenuHelp[6]; else the trade in words, built as a
            ccMsgData: tpcTalkStr's first piece, "N #G<item>" of what it
            wants, the second piece; then up to three "N #G<item>#W, "
            offers (the last "#W."), two to a line
  8, others msg[(saveData.talkNum[0] + 1) * 3 + talkNum] for three talks
            (talkLoopCnt; talkNum rounds 0-2), then msg[1]
  0x08001f00 (merchants, breeders) msg[game.server][talkNum]
  others    msg[talkNum], voice (ccCheckVoiceGrp(id), talkNum)
proccess 1  no target: CloseMenu. ccMsg->Check(1) until it answers: back
            to the list (menuNext = prev, proccess 0 then +1: the list
            goes on at its proccess 1, without a new greeting)
```

`Check(1)` differs from `Check(0)` only for a record of emode 1: when its
close count runs out it opens the next record of the table
(`Change(rec + 12, name, grp, msg + 1)`, `ccEvMsgData` being 12 bytes; a
`ccMsgData` chain steps 24). 299 of the 1,460 records in gcmn's line
tables have emode 1 (the merchants' Talk is two records, the second
chained; counted over gcmn's `ccEvMsgData` arrays with an address).
`ccCheckVoiceGrp(id)` (main 0x0017fff0) maps ids 141-157 (the
Grunties) through a jump table to groups -2 .. -16, else -1.

### Trade (`TradeMenu` 0x0054e720, `TradeSubMenu` 0x0054f4b0, `TradeMenuDisp` 0x00551570, menus 48, 49)

Trade on a walking PC's list (`PcMenu`, 22, rows 30-79) drops the target,
so the PC is `cmndTargetPrev` on these pages, and goes into 48: what the
trader offers. OK on an offer goes into 49: what Kite gives for it. Both
pages are drawn by `TradeMenuDisp` (0x00551570), and both build their
lists again every frame:

| list | where | what is listed |
| --- | --- | --- |
| the bag | `saveData.itemList[0][40]` (+0x30) | every held slot but key items (category 15) |
| a party member's offers (type 4) | `spcTradeList[id - 1][16]` (+0xe3c, 64 bytes a row) | every held entry but key items |
| a trading PC's (type 0x02000018, id 66-79) | `tpcTradeList[id - 66][3][4]` (main 0x00347f90, 48 bytes a PC) | trade k (what it gives, then up to three `ccItemList`s it wants) while `tpcTradeListSW[id - 66][k]` (+0x1e7c) is set |
| another PC's | `npcTradeList[row][16]` (+0x127c), row `id - 30` (145-153: `id - 109`) | every held entry but key items |

A new game fills `spcTradeList` and `npcTradeList` from `spcDefTradeList`
and `npcDefTradeList` and sets every switch (`InitTradeItem`, see [the
title screen](title.md)); entering a town, `SetTradeItemTown` (main
0x001767a0) resets the lists from the defaults and, on a coin toss per
row, adds a piece of equipment near what Kite wears.

```
TradeMenu (48)
proccess 0    talkTradeFlag 1 (TalkMenu then keeps the trading PC's line);
              the page (disp 4). An empty bag: 20; nothing offered: 30.
              Unless the list's index is set: temp[8] 0, drainItem[0..4]
              -1, tradeMenuHelp[0] (OpenInfo). The dim; the other tasks
              asleep; the list shut (y, my 0)
proccess 1    until Check (the index 0, then 1), else six in waitCount
proccess 2    waitCount to 7: the offers (my, y at most 8), the window in,
              exceptionDisp 1
proccess 3    SelectScr (select and dy kept in temp[4], temp[5]);
              triangle on equipment: its status (64); cancel (19): 100;
              OK (18): the item in drainItem[0], its count in temp[0],
              into 49 (the index 0). The item's name and comment (DispMsg)
20, 30        tradeMenuHelp[3] (the dim, the tasks asleep); 21, 31 until
              Check: 100
proccess 100  the window and dim out, the index and exceptionDisp 0, the
              target back (ccChangeCmndTarget(cmndTargetPrev)); with mode 1
              and the tasks asleep: WakeAll, Disp, a breath, the flips back
              on; back to prev (22)

TradeSubMenu (49)
proccess 0    the page; unless the index is set tradeMenuHelp[1]
proccess 1-2  as 48's, over the bag (exceptionDisp 2, select and dy in
              temp[6], temp[7])
proccess 3    triangle as 48's; the keys from the pad's repeat, OK over
              cancel:
              cancel on an item offered: one fewer, out of the offer at
                none (sound 19); on another item, pushed: 100
              OK, the trade balanced and OK pushed: 4 (sound 18)
              OK, not balanced: one more of the item offered, up to what is
                carried; a new item takes the first free of drainItem[1..4]
                (sound 18); with three already offered, nothing
proccess 4    the window gone: OK / Cancel (disp 11, x 6, y 2,
              dialogDefault), tradeMenuHelp[2], exceptionDisp 3
proccess 5    Select; cancel, or Cancel: 100; OK: 6
proccess 6    the window gone: 99 of the item with it: 40 (waitCount 5);
              the bag full, the item not carried and no offered stack given
              up whole: 40 (waitCount 4). Else the trade: DelItem of each
              offered, AddItem of the item, sound 74, AddTradeCount(type,
              id), EntryAffect(cmndTargetPrev, 0), the trade off the
              trader's list (DelSpcTradeList / DelNpcTradeList: the entry
              {-1, -1, 0}; a trading PC's switch 0), "You now have #G<item>
              #W!" (OpenInfo)
proccess 7    until Check: a PC: the window shut (CloseMenu inlined; the
              target stays dropped); a member: the dim in, 8
8 - 12        a member, after 10 frames: each book offered (category 12)
              read, AddSpcItem, "<name> used #G<book>#W." (sound 92), until
              Check, 10 frames
15 - 17       each piece of equipment offered with a price: AddSpcItem;
              put on: "<name> equipped #G<item>#W.", until Check, 10 frames
19            the rest: AddSpcItem each; cmndTargetFix 0, EntryAffect 0;
              the window shut
40, 41        tradeMenuHelp[waitCount] (its third line only when it has
              one); until Check: 100
proccess 100  as 48's; 49's prev becomes 48's first, so it goes back to 22
```

The trade balances for a trading PC when each thing the trade wants is
offered in its count (the page's gauge: two glyphs of eight a want met,
all eight when all are). Otherwise it balances when the offer is worth at
least the item: `temp[k] * ccGetItemPrice(item k)` for each (an offered
item the same as the one wanted counts nothing; member 1's Aromatic Grass,
14/10, counts 50000 a piece), each times its rate. `ccGetItemTradeRate(row,
cat, id)` (main 0x00178fb0) reads the trader's row of `spcTradeRateTbl`
(main 0x003457f0, by `id - 1`) or `npcTradeRateTbl` (0x003459d0, by the
list's row), 14 shorts: categories 10-14, armour by its weight class
(`ccGetEquipParam` +0x38: 1, 2, 3), the six weapons; 10 when the row has
none. A member adds `friendship / 200` (at most 5) to the offered items'
rates. The gauge shows `offer * 4 / item` glyph pairs, at most 4.

A party member hands over nothing more but takes what Kite gave through
`AddSpcItem(spc, cat, id, num, 1)` (below, under Gift): a book read, a
piece worn, the rest into its bag.

What is written to the save: Kite's `itemList[0]`; the trader's list
entry (`spcTradeList` / `npcTradeList` +0x127c) or its switch
(`tpcTradeListSW` +0x1e7c); `pcTradeCount` (+0x686a: `AddTradeCount`, at
most 99); for a member its `itemList[id]`, `spcParam[id]` gold (+0x14) and
`equip` (+0xc8), and `skillList[id]`.

The texts: `tradeMenuHelp[7]` (main 0x0033ec40: the help, the question,
the refusals "There is no item to trade.", the 40 kinds and 99 limits),
`presentMenuHelp` [4] " equipped " and [5] " used " (0x0033ec60),
`tradeMenuStr` (.sdata 0x00377e38: the gauge's eight glyphs and "Approve",
16 glyphs a row), `getItemMenuStr` (0x00377e0c: "You now have "), gcmn's
"#G", "#W!", "#W." (0x006e06a0, 0x006e06a8, 0x006e04b0).

`TradeMenuDisp` into `settingKanji[0]`: `tradeMenuStr`'s two rows, the
trader's name, Kite's, the item and the three offered (16 glyphs each).
The trader's name frame at (40, 24) (`DispTarget`), the item and its count
under it (24, 56); the offers (24, 128), eight rows of icon, name
(`settingKanji[1]`) and count less what is being traded, grey unless on
48; Kite's name (322, 24), the three offered and their counts (306, 56),
the one under 49's cursor in colour 22; the bag (278, 128,
`settingKanji[2]`), counts less the offer, grey unless on 49; "Approve"
(214, 56) lit when the trade balances, over the gauge. The font's shadow
bit (ctrl 0x10) is set and left set, as `BuyMenuDisp` does.

What the pages ask of the runtime: `Affect { cmndTargetPrev, 0 }` after the
trade (and after a member's), `Target(prev)` on the way out, `SleepAll`,
`Still`, `KeepLayers`, `WakeAll`; for a member what `AddSpcItem` asks
(`Talk(SpcUseItem { spc, code })`, `CalcReal { target }`, `ChangeEquip {
target, cat, item }`), and `TargetFix(false)`.

### Buy (`BuyMenu` 0x00556480, `BuyMenuDisp` 0x005572e0, menu 52)

The stock is `equipShopItemList`, `magicShopItemList` or `itemShopItemList`
(gcmn 0x00648430, 0x00648650, 0x00648230; a pointer per server to 24 item
codes, -1 past the stock) by the 0x100 and 0x800 bits of the list's
`index` (the merchant's type Vender kept), at `game.server`.

```
proccess 0   the page (disp 4, 18 cells, 8 rows, my = the stock's
             count), the window and dim in; then as 1
proccess 1   SelectScr; triangle on equipment (category 0-9): itemNum, its
             status (64); cancel: ccChangeCmndTarget(cmndTargetPrev), the
             tasks woken (unconditionally), Disp, breath; then back to the
             list and the item's help; OK on an item: exceptionDisp 2,
             waitCount 1 (0 at 99 carried), proccess 2. DispMsg of the
             item's name and comment every frame
proccess 2   the count: up / down one, left up ten, right down ten, 1 ..
             99 - carried; cancel: proccess 0; OK: the window out, then
             not enough gold (buyMenuHelp[2]), no free slot and none
             carried (3), 99 carried (4): temp[0], proccess 10; else 3.
             buyMenuHelp[0] meanwhile
proccess 3   the window gone: "Buy N #G<item>#W for P GP?" (buyMenuHelp[1]
             around the count, the name and min(price * N, 9999999);
             OpenInfo), the list turned into OK / Cancel (disp 11, 6 x 2,
             its index = the row, the cursor on OK, dialogDefault)
proccess 4   Select; OK on OK: gold -= price * N (at most 9999999 left),
             AddItem(0, item, N); else nothing; the window out
proccess 5   the window gone: the row back, proccess 0
proccess 10  the window gone: the refusal (OpenInfo: one line for 2, three
             for 3 and 4); 11 until Check(0); 12 eight frames; proccess 0
```

The gold is `plw.pw->base->gold`: `saveData.spcParam[0].base.gold`
(+0x749c). `ccGetItemPrice` (main 0x00178eb0) is an item table's +0x0c or
an equipment table's +0x40.

`BuyMenuDisp`: the carried count of 40 and the stored of 99 at (87, 56) and
(159, 56) as `ItemMenuDisp`'s; the stock's names into settingKanji[0] from
`dy`; the tabbed frame (one tab, `itemMenuTag`'s 4 equipment, 1 magic, 0
items) at (39, 96), the scroll bar, the cursor (width x - 1 with the bar,
fading only while counting); each row's price (7 digits) and "GP" at 187,
its icon at 51, its name at 67, all colour 0 while counting; the menu
font's ctrl gets 0x10 (a shadow on strings) and keeps it; the money window
(`DispSquareW2(3, 7, 1)` at (39, 282), `shopStr`'s "Money" row, the gold);
while counting a `DispSquareW2(7, 2, 1)` beside the row (x * 14 + 87) with
the total (colour 23 when short or 0, else 22) and the count, and under
it "Possess" and how many are carried. `DispSquareW2(w, h, n, title)`
(main 0x001b9120) is the frame `w + h + 1` cells wide with a divider after
`w` (rows `@1408`, `@1411`, `@1412`: codes 10, 22, 18).

### Sell (`SellMenu` 0x005552f0, `SellMenuDisp` 0x00555ee0, menu 51)

The Items pages of the bag (`SetItemList(0, page, ..)`, five tabs) sold at
half the price (`price / 2`, rounded toward 0).

```
proccess 0  pageNum 5, the page kept in range, the list fitted, the dim
proccess 1  SelectScr over the tabs (kanjiAlpha -24 on a page change);
            triangle on equipment: its status (64); cancel: the merchant the
            target again, the tasks woken, back to the list (no help that
            frame); OK on an item: exceptionDisp 2, waitCount 1, proccess 2.
            The item's help every frame
proccess 2  the count 1 .. carried (as Buy's keys); cancel: proccess 1;
            OK: the window out, 3. sellMenuHelp[0]
proccess 3  the window gone: "Sell N #G<item>#W for P GP?" (sellMenuHelp[1]),
            OK / Cancel (disp 11, its x and y kept in sx and sy)
proccess 4  Select; OK on OK: gold += N * (price / 2) (at most 9999999),
            DelItem(0, item, N)
proccess 5  the window gone: the page back (x, y, disp 4), proccess 1
```

`SellMenuDisp` is `ItemMenuDisp` (which for 51 draws each row's half price
and "GP" at 187 in place of the count at 235), the money window, and while
counting the price window (colour 22, `sn` the row less `dy`) with the
count and "Possess".

### Elf's Haven (`ItemDepositMenu` 0x00559960, `ItemDrawMenu` 0x0055ac40, menus 54, 55)

Store Items moves from the bag (`SetItemList`) into the storage
(`plItemList`, 99 entries, +0xb70); Withdraw Items back
(`SetPlItemList`, gcmn 0x00527740: as SetItemList over the storage, its
page table at gcmn 0x006516b0). The two handlers are the same code with
the two sides swapped:

```
proccess 0  pageNum 5, the list fitted, the window and dim in
proccess 1  SelectScr; reverseHead: a bit per row shown whose item the
            other side cannot take (99 there, or none there and no free
            slot); triangle on equipment: its status; cancel: as Sell's;
            OK on an item: no room (help 2) or 99 there (help 3): the window
            out, waitCount = help, proccess 10; else exceptionDisp 2,
            waitCount 1. The item's help
proccess 2  the count 1 .. min(carried, 99 - there): up one while below,
            down one to 1, left up ten, right down ten; cancel: 1; OK: 3.
            help[0]
proccess 3  the window gone: "... N #G<item>#W." (help[1], OpenInfo),
            OK / Cancel; reverseHead 0
proccess 4  OK on OK: Store AddPlItem then DelItem; Withdraw AddItem then
            DelPlItem
proccess 5  the page back, proccess 1
proccess 10 the refusal (Store two lines, Withdraw three); 11 Check; 12
            eight frames; proccess 0
```

`itemDepositMenuHelp[4]` and `itemDrawMenuHelp[4]` (main 0x0033eca0,
0x0033ecb0). DelItem and DelPlItem leave the emptied slot where it was;
AddItem and AddPlItem sort the list by `addItemCategoryTbl`.
`ItemMenuDisp` greys 54's rows by reverseHead (colour 0 on the live page,
8 while counting; else 7 and 0); `ItemDrawMenuDisp` draws the storage's
page the same way itself. While counting both show three windows beside
the row (`DispSquareW2(7, 2, ..)` at x * 14 + 87): the count (colour 22,
"Store" or "Withdraw"), "In Storage" and "Possess".

### The Recorder's Save (`RecordMenu` 0x00558260, `RecordMenuDisp` 0x00558db0, menu 53)

The page drives `ccSaveSys` (sdmng.cpp, the desktop Data screen's own;
[the save format](../formats/save.md)): `StartReq(3)` starts the
`ccThSaveSys` task (priority 20, so its `MainProccess` runs before
`ccThMenu` from the next frame on) and the page follows `result` (low 12
bits a `saveSysMsg` number; 0x1000 / 0x2000 a message to acknowledge,
0x8000 a YES / NO question, 0x10000 busy).

```
proccess 0    StartReq(3); menuStatus 0, the dim; the list's index =
              saveSys.port, page = fileNum, select = index, sx (the last
              result) 4; then as 1
proccess 1    the window gone and waitCount 0: the card slots (x 11, y 2,
              exceptionDisp 1), SlotSelectReq
proccess 2    Select over the two slots; OK: LoadInfoReq(slot), the window
              out, wait 10, 3; cancel: 100
proccess 3    result 1 (the index read): 4
proccess 4    the window gone and waited: the files (x 27, y 12,
              exceptionDisp 0), select = page, SaveSelectReq
proccess 5    at result 25: exceptionDisp 2, Select; OK: SaveDataReq(file),
              wait 10, 6; cancel: wait 10, 1
proccess 6, 7 result 1 and the wait over: 4
proccess 100  EndReq; the window and dim out
proccess 101  the window gone: the Recorder the target again, the tasks
              woken, Disp, breath; back to the list
every frame   waitCount down. For proccess 1-99: result 0 outside 1:
              NextProccess(0), wait 10, back to 1; message 2: LoadInfoReq;
              a result other than the last: wait 10 (for 28, 34 and 38 with
              sound 74; for 0, 24, 29, 35, 39); an acknowledgement: the
              window out, then OK: NextProccess(0), ccMsg cursor 0; a
              question: once the window is gone, OK / Cancel (disp 11, the
              cursor on Cancel), OK NextProccess(1 - row), cancel
              NextProccess(0). Then, the wait over, the message: DispInfo
              of its four lines (empty ones null) for a flagged result
              (ccMsg cursor 1 for an acknowledgement, 3 for busy), else
              DispMsg of three while the window is up
```

`RecordMenuDisp`: `recordMenuStr` (main .sdata 0x00377e48: "slot 1",
"slot 2", "Unused", "Data", "MEMORY CARD", 16 glyphs each) into menuKanji;
the slots (a frame at (39, 96), "MEMORY CARD" and the slot's name on each
row, the cursor on `index`) or the twelve files (a frame centred at 256 -
(x + 2) * 7, y 64; each row "Data" and the number (two digits) at +14 and
+56, then from the index (`saveSys->info[k]`, 28 bytes) the name at +95,
"Lv." and the level (two digits) at +220 / +250, and the play time as
`h:mm:ss` (hours in 3 digits; "999:59:59" from 0x0cdfe5c4 frames) at +282,
or "Unused"; colour 6 for a record whose clear flag (+2) is at least
`volumeNum` (1), else 18 for a parody one (+3), else 7). The menu font's shadow bit is cleared
here.

### An administrator (`NpcMenu`, 23)

The action button on a character of type 0x10 (`npcTbl` rows 29
"Administrator" and 158 "NPC", both with `sysopeMsg`, gcmn 0x00631cb0, as
their lines) opens 23 (`NpcMenu`, 0x00542510). There is no list: the
character speaks and the window shuts.

```
proccess 0  no target (ccCheckTarget), or base->msg 0: CloseMenu (inline),
            then proccess 1 with the menu shut. Else EntryAffect(cmndTarget,
            plw, 15) and ccMsg->Open(base->msg, base->name, -1, -1)
proccess 1  no target, or a battle (game.inBattle): CloseMenu, ccMsg->Close.
            Check(1) (an emode-1 record chains to the next in its table)
            until it answers: CloseMenu
```

### A party member (`SpcMenu`, 21) and Gift (`PresentMenu`, 50)

A party member walking in town (type 6: 4 party, 2) is a `ccSpcChar` whose
`base` and `personality` are its own `saveData.spcParam[id]` (+0x7488,
0xdc bytes). Its `base.msg` is `spcMsgTbl[id]` (gcmn 0x00637e30): a new
game copies `charTbl`'s pointer (DEMO.PRG's), and `ccSaveData::SetSpcBaseMsg`
(main 0x00176200), called by `ccSetupNewGame` each time the field starts,
sets members 1-17 to the gcmn table. The list (21) is Talk (47), Trade
(48) and Gift (50).

```
SpcMenu (0x005418a0)
proccess 0  cmndTargetFix; no target: CloseMenu. The rows' names (16 glyphs
            each); the first time: EntryAffect(cmndTarget, plw, 14),
            ccMsg->Open(base->msg[saveData.talkNum[id]], base->name, voice
            -31, 2 id) when base->msg is set, the cursor on Talk, and the
            member's trade count started (CheckTradeCount < 0:
            AddTradeCount). talkNum 1 (Talk's record)
proccess 1  Select. No target: CloseMenu, ccMsg->Close. A battle:
            EntryAffect(cmndTarget, 0), the same. Cancel (19):
            ccEvVoiceStop, EntryAffect 0, CloseMenu, ccMsg->Close. OK (18):
            ccEvVoiceStop; off Talk the target dropped
            (ccChangeCmndTarget(0)); ChangeMenu; ccMsg->Close
```

Gift (`PresentMenu`, 0x00553cd0, drawn by `PresentMenuDisp`, 0x00555020)
is the Items pages of Kite's bag; the member is `cmndTargetPrev`.

```
proccess 0   the other tasks asleep unless still (SleepAll, still 1, the
             flips and layers); exceptionDisp 1, the dim; five pages
             (SetItemList(0, page, list, 1))
proccess 1   SelectScr; triangle on equipment (0-9): itemNum, its status
             (64); cancel: 10; OK on an item: the count (exceptionDisp 2,
             waitCount 1). The item's name and comment (DispMsg)
proccess 2   the count, 1 .. carried (up / down one, left up ten, right
             down ten, as the shops'); cancel: 1; OK: 3.
             presentMenuHelp[0] (DispMsg)
proccess 3   the window gone: "Give N #G<item>#W." (presentMenuHelp[1],
             OpenInfo), OK / Cancel (disp 11, the row in index, x and y in
             sx and sy, dialogDefault)
proccess 4   Select; cancel or Cancel: 5. OK:
             spcPresent[id - 1] += price x N (at most 9999999);
             trapNum = AddSpcItem(member, item, N, 1); itemNum = the item;
             DelItem(0, item, N); the dim out; in town (game.area 0)
             WakeAll, Disp, a breath, then still 0 and the flips; 6
proccess 5   the window gone: the page again (1)
proccess 6   the window gone: the list back (disp 4). The gift's worth:
             its price x spcTradeRateTbl[id - 1] / 10, 50000 for Mia (1)
             given Aromatic Grass (14/10). Under 100, 5000, 10000, 20000,
             else: thanks record n = 0-4 of spcMsgPresent10[id] (talkNum[id]
             0) or spcMsgPresent11[id], friendship +1, 10, 20, 50, 100
             (AddFriendship); ccMsg->Open(record, name, voice -32, 5 id + n)
proccess 7   Check(0): Close; the tasks asleep unless still; the dim; 8
proccess 8   ten frames, then "<name><received|equipped|used> #G<item>#W."
             (presentMenuHelp[3 + trapNum], OpenInfo; sound 92 for used)
proccess 9   Check(0): Close, the dim out, WakeAll, Disp, a breath; still 0,
             the flips; cmndTargetFix 0; EntryAffect(member, 0) and
             ccSpcMessagePresentOtherFellow(member); CloseMenu
proccess 10  ccChangeCmndTarget(cmndTargetPrev); with mode 1 and still:
             WakeAll, Disp, a breath, still 0; the dim out, back to the
             list (menuNext = prev)
20 - 22      "No items you can give." (presentMenuHelp[2]) for 16 frames,
             then cancel: 10. Nothing sets proccess 20 on menu 50.
```

`PresentMenuDisp` is `ItemMenuDisp`, and while counting (exceptionDisp 2)
a window beside the row (x = list.x 14 + 87, y = (select - dy) 20 + 112):
menuWindowPr `DispSquareW2(5, 2, 1)` at (x, y - 16), its cursor (8 wide,
df 6), shopStr's row 5 ("Give") at (x + 14, y) and the count in 2 digits
(colour 22) at (x + 106, y).

`AddSpcItem(spc, cat, id, num, tf)` (gcmn 0x00527950) is what the member
does with it (its answer is trapNum):

| gift | what happens | answer |
| --- | --- | --- |
| a book (12) | `ccUseItemRequest(spc, spc, code, 0)` `num` times | 2 |
| a weapon of the member's job (0-5 = job), or armour (6-9) whose weight class (`ccGetEquipParam(cat, id)` +0x38) is at most 2, 3, 3, 3, 2, 1 by job 0-5, dearer (`ccGetItemPrice`) than the piece worn (an empty slot, -1, reads the row before the table) | `ChangeEquipment(id, cat, id)`, `CalcReal(spc, 1)`, with `tf` the thread `ccThEquipMenu` (gcmn 0x00538d80: `ccSPC::ChangeEquip(cat, CheckSpc(id), id)`) and `Disp`, a breath until it has run (one); one fewer then goes to the bag, and the piece taken off | 1 |
| anything else | the bag | 0 |

The bag is `AddSpcItemSub(spc, cat, id, num)` (0x00527d80) over
`itemList[id]`: past 99 of the item, the rest sold in town for half its
price into the member's gold (`spcParam[id].base.gold`, at most 9999999);
with the bag full and none of it, the cheapest thing carried (not the
healing items 10/0-5, nor Mia's 14/10; the last of equal prices) is
dropped (`DelItem`, the whole stack) and sold in town, or, when nothing
is as cheap, the gift itself (0 added); outside town nothing is paid.
Then `AddItem(id, cat, id, num)`.

`ccSaveData::ChangeEquipment(k, cat, id)` (main 0x00177010) is
`ccChangeEquipment(&spcParam[k].equipment, skillList[k], cat, id,
&spcParam[k])` (0x00177070): the old piece's three skills (+0x3a) that no
other piece worn gives (`ccCheckSkillCount`, 0x00176ce0: head, body,
arm, leg and the job's weapon) leave the skill list (the first slot each),
the new piece is worn and its skills added (`ccAddSkill`), then the sets:
four pieces 60, 61, 62 or 63 (the Goblin sets) add 291-294, else all four
are removed (`ccDelSkill`, every slot); head 68, body 67, arm 66, leg 67
with weapon 8 (the Cats set) add 289, else it is removed. The port's is
`menus::equip::change_equipment`, which Equipment (63) uses too, and
`AddSpcItem` is `menus::talk::add_spc_item`, which Trade (49) uses too.

### The Grunty breeders (`BreederMenu`, 27) and Give Food (`BreedingMenu`, 56)

A breeder (`npcTbl` row 10, "Grunt Shop", type 0x8000000) opens 27
(0x00543930): Talk (the list's first item) and three pages
(`breederMenuStr`: About Grunties, About Food, About Breeding).

```
proccess 0  cmndTargetFix; no target: CloseMenu. The rows (16 glyphs
            each); the first time: EntryAffect 14 and the greeting
            (base->msg[game.server]), the cursor on Talk. talkNum 3 with
            key item 49 (impItemList[49]), else 1; the minimap out;
            SetMerchantCamera
proccess 1  Select; cmndTarget 0: changeCamera(1), the minimap back,
            CloseMenu, ccMsg->Close, then the keys (next frame's pad).
            Cancel: EntryAffect 0 (when ccCheckTarget passes it),
            changeCamera(1), the minimap back, CloseMenu, ccMsg->Close.
            OK on Talk: ChangeMenu, ccMsg->Close; on a page: the window
            out, 2
proccess 2  no target: as the lost target (then proccess 3). EntryAffect
            15; breedTeachMsgTbl[row - 1] in town 1 (game.town), else
            breedTeachMsgTbl2's, with the breeder's name
proccess 3  Check(1): the window in, proccess 0, firstTime 0 (no greeting)
```

Give Food (56, 0x0054bdd0, drawn by `BreedingMenuDisp`, 0x0054cd70) is
not on the breeder's list: a Grunty's own menus (`OtonainuMenu`,
`InuMenu`, 45 and 46) drop the target and go into it, so the
Grunty is `cmndTargetPrev`, a `ccPGuso` (0x330 bytes, gcmn pgbreed.cpp)
whose base is its `npcTbl` row (154-157 the young ones). The menu reads
`growthNum` (+0x2ae: 0 idle, 1 eating, 2 growing up, 3 speaking, 4 done,
5 gone), `msgNum` (+0x2af) and `exist` (+0x305) each frame, and writes
`growthNum = 4` once. The foods are Kite's key items 26-41
(`impItemList`, +0xcfc).

```
proccess 0   index, select, dy 0; my the foods held, y at most 8;
             exceptionDisp 1. None: ccChangeCmndTarget(cmndTargetPrev),
             back to the list (menuNext = prev)
proccess 1   SelectScr; cancel: the same; OK: exceptionDisp 2, waitCount 1
proccess 2   itemNum = 15 << 16 | the food; the count, 1 .. held; cancel:
             1; OK: the window out, 3
proccess 3   the window gone: "Give N #G<food>#W." (breedingMenuHelp[1]),
             OK / Cancel (disp 11, the row in sx)
proccess 4   Select; OK on OK: EntryAffect(grunty, plw, 19, food - 26, N)
             and DelItem(0, 15, food, N); 5
proccess 5   the window out; 6
proccess 6   the window gone: growthNum 0: the foods again (0); else 10
proccess 10  growthNum 0: 11 (the foods again); 1: wait; 2: pgEvoMsg (gcmn
             0x00638dc0) opened at waitCount 0 with no name, its second
             record at 60, closed at 120; 3: base->msg[msgNum] with voice
             (ccCheckVoiceGrp(id), msgNum): 12; 5: index 1, 20
proccess 12  Check(1): index 1, growthNum 4; exist: 13; key item 49 held:
             20; else the dim, the tasks asleep unless still: 14
proccess 13  growthNum 5: 20
proccess 14  ten frames, then "You now have #G<15/49>#W!" and
             getItemMenuStr's line 8 (OpenInfo), AddItem(0, 15, 49, 1),
             sound 74
proccess 15  Check(0): Close, WakeAll, Disp, a breath; still 0, the dim
             out; 20
proccess 20  fade = menuFade->EntryFade(10, 0, 0x80000000, 0, 0, 512, 448)
proccess 21  CheckFade(fade) over: EntryAffect(grunty, plw, 11)
proccess 22  ContinueFade(fade, 10, 0)
proccess 23  CheckFade over: DeleteFade; index 0: back as proccess 0; else
             EntryAffect(grunty, 0), the minimap back, cursolOff 0,
             CloseMenu
```

`ccScFade` (0x98 bytes: `flipFlag`, four 0x24-byte elements, `layer`) is
bookkeeping in the menu's calls: `EntryFade(t, c0, c1, ...)` (main
0x00160400) takes the first element with mode 0 (mode 1, count 0, time
t, colours c0 -> c1); `CheckFade(n)` (0x001604d0) is count < time;
`ContinueFade(n, t, c1)` (0x00160490) restarts it from its last colour;
`DeleteFade(n)` (0x00160510) sets mode 0. `SendPacket` (0x0015fb80), from
every `ccMenuCtrl::Disp` just after `ccMsg->Disp`, draws each element on
and counts it up; past its time a fade holds (a flash, mode & 2, ends).
The port keeps it as `MenuCtrl::menu_fade` (`piney_demo::fade::ScFade`,
which PERSONAL's Ryu Book uses too) and draws it itself (`Draw::Fade`).

`BreedingMenuDisp`: the food list (menuWindow at (39, 36), `DispSquareSB`
or `DispSquare` 11 wide with the list's title, colour 7 while choosing
else 0): from `dy`, 8 at most, each food's name (menuKanji at (53, 52 +
20 r)) and count (3 digits at (175, ...)); the cursor on the chosen row
(df 3, 2 while counting). "STATUS" (`DispSquare(9, 6)` at (336, 36)) with
`breedingMenuStr`'s six rows in settingKanji[0] at (350, 52 + 20 r) and
`growth[game.server]` (+0x2194, `GROWTH_PARAM` 0x18 bytes: size, smell,
crooked, cruel, iq, pure at +2..+12) by `ccFont::MakeSignedNum(3, v)` at
(434, 52 + 20 r); while counting each is `food[k] x N + growth` coloured
by the food's `foodTbl` (gcmn 0x005ee450, 12 bytes: six shorts) value:
negative 23, positive 20, 0 colour 7. Then menuWindowPr `DispSquare(2,
1)` at (257, y - 16), its cursor (2 wide, df 6), and the count (2
digits, colour 22) at (277, y), y = (select - dy) 20 + 54.

`MakeSignedNum(n, v)` (main 0x0015cdd0) is `sdec2str(n, v)` (0x0015cfc0):
`n + 1` cells, the digits' leading zeros but the last blank, the sign
(a space for 0 and up) in the cell before the first digit shown; a value
wider than `n` digits keeps its last `n` with the sign in the first cell.

### Dun Loireag's dogs (`NorainuMenu`, 44)

The action button opens 44 on a dog (base type 0x04000000: `npcTbl` rows
141-144, `ccDog`, [town02.md](town02.md#the-dogs)). `defNorainuMenu`
holds Talk (47) alone; `defOtonainuMenu` (45, the grown Grunties) Talk
and Trade (48), `defInuMenu` (46, the young ones) Talk and Give Food
(56).

```
proccess 0  cmndTargetFix; no target: CloseMenu (firstTime 0). The rows:
            the list's items' names; the first time the dog sits and faces
            Kite (EntryAffect 14) and greets (base->msg's first record,
            voice ccCheckVoiceGrp(id), 0), the cursor on the first row;
            talkNum 1, 3 or 5 (ccRand() & 3, 3 read as 0; times 2 plus 1),
            the record Talk shows
proccess 1  Select; cmndTarget gone: CloseMenu; next frame ccMsg->Close,
            ccEvVoiceStop, then the keys
            cancel (19): EntryAffect 0 (it walks on), CloseMenu; next frame
            ccMsg->Close, ccEvVoiceStop
            OK (18): a row past the first drops the target (the tasks
            asleep, ccChangeCmndTarget(0)); ChangeMenu, ccMsg->Close,
            ccEvVoiceStop
```

`TalkMenu` on a dog or a Grunty (no flag of its own) opens `base->msg +
12 talkNum`
with the dog's voice group (`inu0VoiceTbl`-`inu3VoiceTbl`, groups -13 to
-16; a Grunty's `ccCheckVoiceGrp(id)`) and EntryAffect 15 (it sits up).
`tools/test_fieldui_talk_rs.py`'s `DogPages` compare it with the game:
the greeting and cancel, Talk with each `talkNum`, the target lost under
the list and no target. The game draws `talkNum` from `ccRand` (the
town's Mersenne Twister); the port's menu has `rand()` only, so the
harness gives both the same values.

### The Grunties (`OtonainuMenu`, 45; `InuMenu`, 46)

A grown Grunty (base type 0x02000000, `npcTbl` rows 145-153) opens 45
(0x0054ac40, Talk and Trade), a young one (0x01000000, rows 154-157) 46
(0x0054b1d0, Talk and Give Food, drawn by `InuMenuDisp`, 0x0054bb10);
both are `ccPGuso`s ([town02.md](town02.md#the-grunties)).

```
OtonainuMenu
proccess 0  cmndTargetFix; no target: CloseMenu. The rows; the first
            time EntryAffect 14 (dogAction2 / dogActionAdult: line 7)
            and the greeting (base->msg's first record, voice
            ccCheckVoiceGrp(id), 0); the cursor on Talk; talkNum the
            Grunty's msgNum
proccess 1  Select; cmndTarget gone: ccEvVoiceStop, CloseMenu,
            ccMsg->Close, then the keys (next frame's pad)
            cancel (19): EntryAffect 0 (a target still there),
            ccEvVoiceStop, CloseMenu, ccMsg->Close
            OK (18): Trade drops the target (the tasks asleep,
            ccChangeCmndTarget(0)); ccEvVoiceStop, ChangeMenu,
            ccMsg->Close

InuMenu
proccess 0  no target: CloseMenu. cmndTargetFix; the rows; the first
            time the cursor on Talk, menuFade->EntryFade(10, 0,
            0x80000000), the minimap and the window out, cursolOff 1,
            the target dropped (the Grunty now cmndTargetPrev): 1; else
            4
proccess 1  the fade done: EntryAffect(cmndTargetPrev, plw, 11) (the
            fixed camera, Kite and the Grunty at their places)
proccess 2  ContinueFade(fade, 10, 0)
proccess 3  the fade done: DeleteFade, the Grunty the target again,
            EntryAffect 14 and its greeting (base->msg's first record,
            voice ccCheckVoiceGrp(id), its msgNum)
proccess 4  exceptionDisp 1 (the STATUS window), the window in; the
            Grunty's foodMode 0
proccess 5  Select; cancel (19): ccEvVoiceStop, ccMsg->Close, the window
            out, 20
            OK (18): ccEvVoiceStop, ccMsg->Close; Talk: EntryAffect 15,
            talkNum its msgNum, ChangeMenu. Give Food with none of the
            foods (key items 26-41): the window out, the dim, the tasks
            asleep, the target dropped, 10; with some: its foodMode and
            chatFlag 1, the target dropped, ChangeMenu (56)
proccess 10 the window gone: "There is no food." (breedingMenuHelp[0])
proccess 11 Check(0): Close, the tasks woken (a breath), the Grunty the
            target again, the dim out, 4
proccess 20 menuFade->EntryFade(10, 0, 0x80000000)
proccess 21 the fade done: EntryAffect 11 (the field camera back)
proccess 22 ContinueFade(fade, 10, 0)
proccess 23 the fade done: DeleteFade, EntryAffect 0 (it walks on), the
            minimap back, cursolOff 0, CloseMenu
```

`InuMenuDisp` draws the STATUS window (menuWindow at (336, 36),
`DispSquare(9, 6, "STATUS")`): `breedingMenuStr`'s six rows in
settingKanji[0] at (350, 52 + 20 r) and `growth[game.town]`'s size ..
pure by `MakeSignedNum(3, v)` at (434, ...). The Grunty's affect
function changes its `msgNum` at once in the game, which the menu reads
straight after; the port's menus read the world's Grunty as the frame
began, so `inu::after_affect` makes the same change to their copy
(`dogAction`'s line for 15, its 14; `dogAction2`'s 14). Their writes to
the Grunty (`foodMode`, `chatFlag`, `growthNum`) are `TalkReq`s the
runtime carries out.

### The texts and tables of 21, 23, 27, 50 and 56

| what | where |
| --- | --- |
| `sysopeMsg` | gcmn 0x00631cb0 (the administrators' `msg`) |
| `spcMsgTbl[18]` | gcmn 0x00637e30 (`SetSpcBaseMsg`'s) |
| `spcMsgPresent10[18]`, `spcMsgPresent11[18]` | gcmn 0x00638800, 0x00638890: five thanks records a member |
| `spcTradeRateTbl[17][14]` | main 0x003457f0: items 10-14, armour classes 1-3, weapons 0-5 (tenths) |
| `presentMenuHelp[6]` | main 0x0033ec60 |
| `@3218` (the friendship cap by volume) and `volumeNum` | main 0x00307180 (1000, 250, 500, 750, 1000), 0x0034bbf8 (1 on the Infection disc) |
| `breederMenuStr`, `breedingMenuStr`, `breedingMenuHelp[2]`, `getItemMenuStr` | main .sdata 0x00377e24, 0x00377e28, 0x00377e30, 0x00377e0c |
| `breedTeachMsgTbl[3]`, `breedTeachMsgTbl2[3]` | gcmn 0x00631c90, 0x00631ca0 |
| `pgEvoMsg[2]`, `foodTbl[16]` | gcmn 0x00638dc0, 0x005ee450 |
| "STATUS", "#G", "#W!" | gcmn 0x006e0768, 0x006e06a0, 0x006e06a8 |
| equipment rows (`ccGetEquipParam`, 0x48 bytes) | +0x38 weight class (armour 1-3) or 0 (weapons), +0x3a three skills, +0x40 price |

### What 21, 50 and 56 write to the save

`pcTradeCount` (+0x686a) started (SpcMenu); Kite's bag (`DelItem`), the
member's bag (`itemList[id]`), gold (+0x14), equipment (+0xc8), skill list
(`skillList[id]`, +0x1ec4), friendship (+0xda), `spcPresent[id - 1]`
(+0x73fc) (Gift); `impItemList` (+0xcfc) foods down and key item 49 up
(Give Food).

## The port's seam

`FieldUi::step(pad, world, save, count)` is one pass of the task. The
runtime fills a `World` each frame: `game` (status, area, battle flags,
dungeon type), the three party slots (`ccCharBaseParam` and `ccChar` fields:
type, id, name, HP, SP, conditions, abilities, the chat settings, width,
`posP`, the camera check), `cmndTarget` and `cmndTargetPrev` (when
`ccCheckTarget` passes them), `cmndSortRoot`'s chain (with `cmndDist`, the
life bar's projected point and the off-screen arrow, the protect count),
the three `cmndLink` chains, `ccSkillCheck(plw)`, the player's attack
flag, `compulsionGameOver`, the event manager's status and boss entry; for
the gate `game.server`, `game.town`, `game.area_level` and the events'
`area_codes`; for the boxes `game.field`, `game.floor`, `field_attr`,
`event_area`, `area_word` (the area's `areaLevel` and `itemOfs`) and a
box's `item`; `party_id` (`memberID`) for `CheckMemberID`; for the
PERSONAL pages `pg_adult` (`ccPgAdultCheck` for the Grunty Flute),
`party_annihilated` (86) and `spc` (`ccSpcManager`'s registry, for Add).
Screen positions are the world's to project. Outside the towns piney-game
fills it from the battle ([battle](battle.md#how-it-plugs-into-piney-world)):
the party slots and every listed character from the scene's `Char`s (HP,
SP, conditions, width, `posP`), the enemies' chain in `cmndSortRoot`'s
order with their life bars projected through the field camera
(`FieldWorld::hud_geometry`: the tag point, the bar's answer and point,
the arrow, `ccCheckCameraDeg`), `game.inBattle` from `ccThGameCtrl`, and
`ccSkillCheck(plw)`; the requests the menus make (a skill or item on a
target, the target chosen, Data Drain's hold, the chat orders) go back to
the battle as `ccSkillRequest`, `EntryAffect` and
`ccSpcChar::RequestChatCmd`. The save is
`piney_desktop::SaveState` (`ccSaveData` with `ccEvent.operate`), read and
written as the game does (the item lists, `tactics`, the operation lock's
`operateSet`).

What comes back, in the game's order, as `Request`s: sounds, sleep / wake
of the other tasks and the frozen layers, voice calls, `cmndTargetFix`,
target changes (`ccChangeCmndTarget`: the runtime moves `cmndTarget` to
`cmndTargetPrev` and sets the new one), skill and item uses, Data Drain's
marks and SP, the minimap's alpha, the party's chat orders, the chat
balloon's line, the gate's (above), a member joining (`AddMember(id)`: the
runtime makes the character, puts it in the first free slot and its id in
`party_id`, before the next step), ChatMenuT's AI calls (`ManualOff`,
`ChatOrder`, `ManualModeAi`, `RemoteCmd`), the PERSONAL, PARTY and
OPTION pages' (their sections above), the talk and shop menus' (the
seam below), and the unported menus. The event engine's field host
(`piney_event::host`, `Place::Field`) calls `message_open`,
`message_check`, `message_close`, `announce`, `menu_ban`, `open_menu`,
`target_forbid`, `map_on` and `noise`.

### The talk and shop menus' seam (`crates/piney-fieldui/src/talk.rs`)

- `FieldUi::talk_to(Some(TalkTarget { handle, who: Speaker::Npc(row) }))`:
  the character the action button spoke to is `npcTbl[row]` (or
  `Speaker::Spc(id)`, a party member). Called when `ccThGameCtrl` opens
  21-27, before the step that opens the menu; the pages read the base
  parameters of `cmndTarget` / `cmndTargetPrev` through it while the handle
  matches. `game.server` comes in `World.game.server`.
- `FieldUi::set_card(Box<dyn MemoryCard>)` and `set_card_position(port,
  file)`: the cards the Recorder writes to (piney-desktop's card seam) and
  where `ccSaveSys` was left (the game has one `saveSys`, so the title's
  load and the desktop's Data screen leave its port and file). The
  `ccThSaveSys` task runs inside `FieldUi::step` before the menu's frame.
- `FieldUi::set_rand(f)`: `rand()` (PcMenu's lines), the game's one
  generator; unset, the menu keeps its own newlib copy.
- `menus::talk::set_spc_base_msg(save, image)`: where `ccSetupNewGame`
  calls `ccSaveData::SetSpcBaseMsg`, the members' lines in their records.
- `World::grunty`: the Grunty (`ccPGuso`) that is `cmndTargetPrev` on Give
  Food, each frame: its handle, row, `growthNum`, `msgNum` and `exist`.
- What the pages ask (`Request`), besides sounds, voices, `SleepAll` /
  `WakeAll` / `Still` / `KeepLayers`, `TargetFix`, `TargetClear` /
  `Target`: `Affect { target, kind }` (EntryAffect 14 as a list opens, 15
  as a line is said, 0 as it closes; 11 and 0 on a Grunty), and
  `Talk(TalkReq)`: `MerchantCamera` and `Camera(1)`; `SpcUseItem { spc,
  code }` (`ccUseItemRequest(spc, spc, code, 0)`: a member reads a book it
  was given; in the town the port raises the record's stat,
  `piney_battle::item::book_on_record`, as the book's effect writes it
  through `ccChar` +4); `PresentOther(member)`
  (`ccSpcMessagePresentOtherFellow`, gcmn 0x005a1a70: with three in the
  party, the third member is sent AI message 0x10011 with the receiver,
  and answers with `ChatMessagePresentOtherFellow`; `World::present_other`);
  `Feed { target, food, num }` (the Grunty's `EntryAffect` 19);
  `GruntyGrowth { target, growth }` (the menu's write of `growthNum`). A
  member given something to wear asks, as Equipment does, `CalcReal {
  target }` (its record's `real` and `tune` already written) and
  `ChangeEquip { target, cat, item }` (its model).
- The lines, stock, prices, trades and texts are read from the executable
  at run time (`TalkTexts`, with the image kept for `base->msg` tables).

### In the runtime

`piney-game`'s world mode steps the field UI after the world each frame,
from the town's first task frame on, drawing into the world's frame
(`FieldUi::step_into`) under its fade. It fills `World` with Kite in party
slot 0 from `spcParam[0]` (at full HP and SP: the save keeps no current
HP), the command target and the one before it (`ccCalcTagPosChar` points
from the world), and `cmndSortRoot`'s chain. The faces are set as the
constructor sets them (`ccCheckMenuFaceNameParty` gcmn 0x0056a930: member
0 shows face 18 until the bracelet's colours, saveData+0x6771). What
`ccThGameCtrl` asks for (`TalkRequest`) becomes `openReqNum`, `mode` and
`firstTime`; `SleepAll` and `WakeAll` put the world to sleep; sounds and
voices go to the sound driver. The town and the field run the tasks in
their priority order ([overview](overview.md#tasks)):
1. `ccThGameCtrl` (`step_game_ctrl`, with its talks and its game over);
2. the menu task;
3. the camera, the party, the entries and the rest of the world.

So a menu's request (a skill, an item, an order) is acted on, and heard,
in its own frame. What the party and the entries raise (their chat lines,
the battle's shows, the menu's rules from them) reaches the menus the next
frame, as in the game.

## Checks

`tools/test_fieldui_rs.py` runs the game's `ccThMenu` from its start in
eemu (eemu_rs when built) with the leaf calls hooked (sprite packets,
kanji, breaths, sounds, the world's queries) and the same scenario through
`crates/piney-fieldui/examples/fieldui_probe.rs`, and compares every frame:
the menu's state, the message window's state, every packet of every sprite
(cell or string, position, size, offsets, grid, colour and alpha, the
drain gauge's four corners), the kanji rows' text, the message and chat
texts, the sounds, voices, target changes, skill and item requests, chat
orders, the save's item list. Its scenarios: the panels (1-3 members, a
ban), speech and information windows, the tutorials' PERSONAL, OPTION, the
fallen player, five kinds of target, the enemies' bars, the banner, new
mail, low HP, Skills (every page, a skill on a target, the wait), revival
and Data Drain with a sub-target, the refusals, TARGET's warnings, Items
(an item on a member, a book at once, treasure refused, equipment status),
the special items (10/18, the Fortune Wire, the Sprite Ocarina's dialog),
CHAT in a field, a dungeon, a town and alone, ten random pad runs
through the battle menus; the gate (Random with Warp and Cancel, New
Keyword with its cancels, the Word List's protected and barred areas, the
history, Other Servers, empty lists, the tutorial's 78 and 79); the gate
hack to area 19 (two cores put in, a third refused, the ring turned both
ways with an empty slot's down refused, OK through the completion to the
area, the cores gone and `protectArea` set; and one core short: OK
refused, cancel back to the town), with `ccHackMenu`'s animations faked by
their lengths and its crystals' `dispSW` and the ring's turn compared at
each draw; Data Drain on a goblin with and without a side effect (the
rules run natively in the game, and the port is answered with what they
left, recorded as it ran: the infection, the drops, the effect, the
party's HP and SP) through the warning or the side effect's window, the
growth, 67 and the item; the movie (a faked stream of 40 frames: the
stream by an enemy's size, 109-111, with `drainDemo` on, and a boss's with
it off after its `EntryAffect(21)`, and `ccThDrainEnemy`'s start);
`interNoiz` 1, 2 and 3 (in town 4 and away,
over the menu opened and a ban: every `SetNoiz` call Disp makes, and its
sound); the party
tutorial (75 to 77, the member added from a thread, its panel and level
box), the skill and chat lessons (80 to 83), the box tutorials (84 and 85,
an item and a draw), a treasure box (32: its item, a disarmed box's draw
and a plain one's), a trapped box opened as it is (33: each trap's line,
none, an unknown skill), a Gott statue (38: the three items through 67
and 29, `idolCount`), 99 already, and the full bag's exchange, give-up and
status paths; CHAT's member menus (71 - 73: a skill on Kite, an area skill
on the member himself, a target, First Aid, Assemble, Standby, the fallen
member's refusals and her Equipment, a member in a skill, in act 12 and
paralysed, no skill known, short of SP, no enemy in reach, backing out
through each, a town and a field out of battle, eight random pad runs,
with `RequestChatCmd` and the balloon's line compared); the field
objects (34 with each draw and with the menu shut, 35 with each trap and
the two that never end, 36, 37 with and without its item, 39 first with
no best, second after a best of third, fourth with a best of fourth and
outside the ranking, the save's ranks watched). Every packet's ctrl bits 0x20, 0x10 and 0x40 (mirrored,
shadowed, upside down) are compared.

`tools/test_fieldui_personal_rs.py` runs the same harness over PERSONAL's
pages and PARTY, with the world they read (the party's records in the
save, `ccSpcManager`'s registry, `ccPgAdultCheck`, `rand`,
`checkPartyAnnihilation`, the area's keywords through `SimGenerateCode`)
and the calls they make outside the menu logged (`ccUseItemRequest` with
its argument, `ChangeArea`, `ChangeRequest`, `GoField`,
`DeleteNoPartyMember`, `ccChar::CalcReal` run as the game has it,
`ccThEquipMenu`'s `ChangeEquip`, `ccParty::DelMember`), the menu fader's
elements, and the save's bytes they write (the key items, the members'
records and skill lists) after every frame. Its scenarios: Key Items'
pages in the three page counts; each key item's use or refusal in a field,
a battle, a dungeon, the special floors and a town; the Grunty Flute with
and without a Grunty; a Ryu Book's fade; Discard's counts, its dialog both
ways and triangle; Status through the whole grid and the members, a piece's
status and a member's items; Equipment through the list, the grid, the
pages, the statuses, four changes, both refusals and a set's skill; Gate
Out in and out of battle, Log Out, 86 going and wiped out; Area
Information in a field and a dungeon; PARTY outside a town and alone, Add
with no answer, a Cancel and a call, with the member loaded idle, loaded
busy and five loaded; Remove and Disband; sixteen random pad runs through
PERSONAL in towns and fields.

`tools/test_fieldui_option_rs.py` runs the same harness over the pages,
logging on the game's side the loading task (`ccStartThread`, cleared at
the next breath, `ccDeleteThread`), `GetCCSAdrs`, menuMask's `SetTex`,
`ccFileListDeleteOne`, `setCameraCtrlType`, `SetDisplayOffset`,
`SetSoundEnv`, `SetActuater` and `actuaterSw`'s changes, `ChangeRequest`,
every packet's ctrl bits 0x20, 0x40 and 0x10, and the save from +0x841a to
+0x8431 after every frame. Its scenarios: Controller from A-1 and from B-1
through every scheme, cancelled while loading, and with a menu asked for
during its own breaths; Vibrate, Data Drain, Voiceover (1, 0 and 2) and
Movie Text both ways from both values; Adjust Screen to every edge and the
diagonals; Sound, each row both ways past its ends and a move with the
cancel; Title Screen through both questions to `ChangeRequest(1, 7)` and
cancelled; eight random pad runs through OPTION and its pages.

`tools/test_fieldui_shop_rs.py` builds on it for the talk and shop menus:
`cmndTarget` is a `ccChar` whose base is the real `npcTbl` row, the port
is told the row through the probe's `npc` (`FieldUi::talk_to`), `rand()`
returns the same values on both sides, and each frame's state adds
talkNum, talkTradeFlag, talkLoopCnt, dummyTarget, the open list's index,
page, size and scroll, `temp[8]` and chosen runs of the save (the gold,
the bag, the storage, the trade flags and counts). The game's `ccSaveSys`
is built by its own static initialiser and constructor, its task's
`MainProccess` runs at the start of each frame while it lives, and the
memory cards answer in Python with `tools/test_desktop_data_rs.py`'s
`Card` (the probe keeps the same card); the cards' index and slot files
are compared at the end. Its scenarios: each merchant's list (greeting,
every row, another server, reopened), the PCs of each kind (random lines,
the trading PCs with open and closed trades, ids past the tables), a
battle starting; Talk for each merchant (chained lines), PCs through five
talks, the trading PCs' offers, the administrator, the Grunties' voices, a
breeder; Buy from the three shops (count keys, the question's OK and
Cancel, every stock row on another server, short of gold, a full bag, 99
carried, triangle on a weapon); Sell (the pages, the count, all of a
stack, past the gold cap); Store and Withdraw (the count's bound by both
sides, full storage, 99 stored, full bag, 99 carried); the Recorder (a new
file, over a used file with its question, no card and not a PS2 card, a
card without the directory made, a failing write of the file and of the
index); and random pad runs through each.

`tools/test_fieldui_trade_rs.py` runs 48 and 49 from a PC's list, with the
lists `InitTradeItem` and `SetTradeItemTown` (run in eemu) leave in the
save, drainItem[0..4] in the compared state and the save's bag, trade
lists, switches and counts watched: a PC's trade walked through (the
offers browsed, not enough offered, three kinds at most, counts up to what
is carried, taken back, out and in again, a trade made), a longer list
scrolled (a key item and a hole left out), three trading PCs (one and two
wants, a switch off), nothing to trade (an empty bag, only key items, all
trades done), the 99 and full-bag refusals and a full bag that trades a
stack whole, the question's Cancel and cancel, triangle in both pages, the
pages left with mode 0 (no breath), eight random pad runs; party members
(through PcMenu opened on the member, their base and personality their
`spcParam`, with a level and a piece worn in every slot; `CalcReal` runs as
the game has it, so their `real` and `tune` are compared): a book read, armour put on (skills changed, the old piece
into the bag, the excess past 99 sold), member 1's weapon put on with a
full bag (the cheapest sold, gold capped), six random runs.

`tools/test_fieldui_talk_rs.py` (on `test_fieldui_shop_rs.py`'s base): the
administrators (both rows, chained lines, no target, no lines, the target
lost, a battle); a party member (five members and stories, the trade count,
Talk, Trade, Gift, no lines, the
target lost, a battle, opened again, random runs); Gift (every worth band
and both thanks tables, the friendship cap and floor, a book, equipment worn
- weapon, one and two, armour, a class the job refuses, another job's
weapon, a cheaper piece, an empty slot, the Goblin and Cats sets made and
broken - the member's bag full (the cheapest sold, the gift itself sold,
Mia's rule both ways), 99 of it, in a field, every cancel, triangle, an
empty bag, without mode, random runs; the member's `real` and `tune`
compared, `CalcReal` run as the game has it); the breeder (servers, the three
pages in town 1 and elsewhere, Talk with and without key item 49, the target
lost under the list with each key and as a page opens, random runs); Give
Food (fed, eating, growing up, speaking with three Grunties' voices, key item
49 given and already held, staying until it goes, going off, idle again,
every cancel, no food, all sixteen foods scrolled, another server's signed
stats, random runs); the Grunties (`GruntyPages`: InuMenu's fade in, the
camera, the greeting by `msgNum`, the STATUS window and the fade out;
Talk with its line and back; Give Food with no food and with foods, into
56; OtonainuMenu's greeting, Talk, Trade and cancel; the target lost under
45's list with each key, and no target). 33 tests, every frame equal.

`tools/test_fieldui_rs.py`'s `test_food` opens 43 on a target of type
0x800000 with base ids 22, 29 and 37: the affect, the sleep, key items 26,
33 and 41 into 29 and `foodCount` watched, every frame equal. piney-game's
`a_grunty_food_is_picked_up` (session) puts a Golden Egg ahead of Kite in
area 26's dungeon: it calls out as he nears, FoodMenu opens on it, it is
taken (act 1, sound 179 on note 70) and bursts (act 2), key item 26 comes
and `foodCount[0]` goes up; `grunty_food_shots` (ignored) takes three
foods lying there and one being taken.

`test_spring_leave`, `test_spring_list` and `test_spring_throw` open 40 on
a target of base type 0x400000 and give the spring's states (the target's
+0x1d4) at set frames, as `ctrlFountain` would. They cover: cancel, "No",
Yes with nothing to throw; twelve items of which ten can be thrown (the
scroll bar, the cursor past the eighth, triangle into 64 and back); and
eight throws: a known blade under bgnum 0-4 (one or two rows on, or back
one), each answer, an item the spring does not know, "No" first, level 4
on another server. Every frame is equal, including the save's bag and
`fountainCount`. piney-game's `the_spring_of_myst_takes_an_item` (session)
walks Kite to the spring of a Δ lake by night, throws a blade in and
answers "Neither": every menu opens, the spring rises, talks and is gone,
`SetFountain` records the area, `fountainCount[0]` is 1, and the blade
comes back one row on (bgnum 2).

## All portals open (`ccThDfComp`)

When an area's last magic portal has been opened (the circle's act 4,
[battle](battle.md)), the entry control starts `ccThDfComp` (gcmn
0x004008c0, priority 33) and it runs `ccDfComp::Main` (0x00400980) once a
frame until that answers 0. `Init` (0x00400da0) makes a `ccSprite`
(`SetPrim(20, 0)`) on `xwindow::TEX_xwindo01` on the menu's layer
(`ccMenu` +0x58). The texture holds two banners of two lines each, 128
texels wide: "ALL DUNGEON / PORTALS OPEN" at rows 0-31 and "ALL FIELD /
PORTALS OPEN" at 33-63. A field (`game+0x14` 1) and a lake's first
dungeon (field type 4, `game.dungeon` 0) take row 33; another dungeon
takes row 0, its top line 5 to the right.

```text
count = total = 90; scale 2.0; spread 0
count 0     reset, answer 0 (the task ends)
count 1-5   count - 1, nothing drawn
else        alpha: (90 - count) * 96 / 15 above 75 (in over 15 frames),
            96 down to 16, (count - 5) * 96 / 10 below (out over 10)
            slide above 75: from 0.5, count - 75 times s += s / 2.75, the
            sum (the lines come in from the sides)
            below 15: scale + 0.06, spread + 0.5 a frame (they part)
            colour ccSpriteColorTable[6]; each line 128 x 16 (the second
            x 15) texels at scale, pivot (-64, -9) at scale:
            top at (252 - slide + x, 110 - spread),
            bottom at (260 + slide, 147 + spread); count - 1
```

In the port it is `piney_fieldui::dfcomp::DfComp`, run by `FieldUi` after
the menu's task (`FieldUi::df_comp` starts it; the area starts it on the
entry control's `Out::AreaCleared`). Checked: `its_frames` (the alpha,
slide, scale and rows over the 90 frames),
`the_last_portal_puts_up_its_banner` (the area's start of it, 90 frames
up), and the shots of `last_portal_banner_shots`. Not compared with the
game's pictures.

## Not yet known or not ported

- **The field objects the runtime does not make.** No setter places
  rows 3 and 5 (35's). The story dungeons' symbols (36, row 17) are
  `ccGimSymbol` (piney-battle's `gimmick::symbol_new`, worklog 0149): 36
  names their random skill and its close casts it. The lakes' symbols
  (row 18: `SetMagicCircle` puts them at a lake's rolled `OBJ_0ps*`
  slots) run `objMain` (`gimmick::obj_main`, worklog 0155): no body, a
  flickering light, a puff of smoke every other frame, and the same cast.
  The session tests open 35 - 37 and 39 on a breakable given each type.
- **The Zeit statue (39).** "Chronicling" (word 131) is a BBS word of
  Infection's (thread 29, "Zeit Statue": "select Chronicling as your part
  A at the Chaos Gate"). With it first, `WORLD_MAN::time_sym` is set and
  the dungeon's `SetIDOL` makes every idol the statue (row 44). The
  session keeps `ccGame.gameCnt` on its scene (`Scene::game_cnt`,
  `Scene::add_play_time` each frame, the `ChangeRequest` resets in
  `change_scene`), so 39 reads the time since the gate. Checked by
  `chronicling_leads_to_the_zeit_statue` (a warp from Mac Anu with
  Chronicling, Passed Over, Aqua Field: the field's clock running, the
  dungeon's one idol row 44).
- **Data Drain's movie.**
  - The enemy's pose is not compared with the game's pictures.
  - A character's movie by `base->id` other than Skeith's (37, 44, 67,
    69, 100) was not played through.
- **Data Drain's side effects' timing.** The game starts step 10's
  effects and numbers inside the menu task; the port starts them at the
  field's next effect task, so their draws from `rand()` come a frame
  later. Their look is not compared with the game's pictures.
- **The gate hack's pieces outside the menu.**
  - The hacked arrival after the menu (`gtHackFlag`, `setupMode` 1) is
    [field walk](field-walk.md#the-hacked-arrival)'s.
  - The 3D is not compared with the game's pixels: the checks compare its
    calls, not its draw.
- Equipment's model change (`ccThEquipMenu`, `ccSPC::ChangeEquip`) and
  Add's call (`ccThPartyAdd`) are tasks in the game that may take frames
  (a model or a character to load); the port takes each as done a frame
  later, the shortest the game can take. The weapon then hangs at once,
  where the game's character frame hangs it a frame later still.
- `ccGetEquipParam` past a table's rows reads the memory beside it; the
  port keeps the ten tables' bytes (and the row before the first) and
  reads zeros beyond them. `ccChar::CalcReal` as `piney-battle` has it is
  what the port uses for the save's `real` and `tune` (Equipment, and a
  member's trade or gift); a piece worn of -1, which no member has from a
  new game on, the game reads as the row before its table and
  `piney-battle` as nothing.
- What `bootParam` of a loaded character means beyond Add's choice of
  greeting (0: the "20" / "21" tables).
- A trader of neither kind (not type 4 nor 0x02000018) makes the game read
  its trade list from a stale register; the port offers nothing.
- `AddSpcItem`'s thread writes its `ccTscb` into the menu's own `tscb`
  (+0x30); the port does not model it.
- `PresentMenu` 20-22: no code found that sets proccess 20 on menu 50.
- `BreedingMenu` 23 with index 0 (back to the list): index is 1 whenever
  20 is reached.
- A `cmndTargetPrev` that is not a Grunty on 56 (the list re-entered after
  its own cancel made the Grunty the target): the game reads that
  character's bytes at the Grunty's offsets; the port reads an idle Grunty.
- A target list drawn with no target, previous target or dummy (Disp's
  disp-6 case leaves the window's position and the rows' y as the caller's
  registers had them): not reproducible; the checks keep a previous target.
- `ccPGuso` itself (the Grunty's growth, eating and speech) is the world's.
- `ccThPartyAdd` runs as its own thread in the game; the port asks for the
  member (`AddMember`) and reads the party again on its next step, as the
  thread has run by then in the game.
- The minimap itself is the world's (`docs/engine/map.md`) - only its alpha
  is the menu's.
- `SetMerchantCamera` and `changeCamera` are requests
  (`TalkReq::MerchantCamera`, `TalkReq::Camera`); the camera they set is the
  world's to build (the formula above).
- The talk menus' `rand()` is the game's shared generator; the port's own
  copy follows a different sequence unless the runtime hands its generator
  in.
- `ccMcard`'s calls complete at once in the port (see
  [the desktop](desktop.md)): the "Saving" frames the game shows while the
  card works do not happen.
- The fly fonts (damage numbers, `ccCtrlFlyFont`, `ccDamUprStr`),
  `ccChatMsg`'s balloon drawing and `ccNoiz`'s own code are hooked out of
  the menus' checks. The balloon has its own
  ([the chat balloons](#the-chat-balloons-ccchatmsg-ccchat)).
- The Sprite Ocarina's `proccess 12` spends and requests the item every
  frame until the field changes; what ends the menu then is the item's
  (`ccUseItemRequest`), not traced here.
- How many frames `ccLoadFLAddOne(flContMenu)` really takes from the CD; the
  port and the check take the shortest (one).
- `ControllerMenu` indexes `ctrlMenuHelp` with `camType` unchecked (a save
  value past 3 reads past the table); the port keeps to the four rows.
- menuMask keeps xcontrol's texture after `ccFileListDeleteOne` frees it;
  nothing draws menuMask afterwards.
- The meaning of `SetActuater`'s 1, 160, 200 beyond "a short buzz".
- Where `drainDemo`, `voice` and `strWinMode` are read, and what
  `camCtrlType`, `camRevLR` and `cameraControlType` do in the field camera.
