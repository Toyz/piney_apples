---
title: Battle rules
status: partial
volumes: all
covers: INF gcmn.prg:0x00431970 ccThEntryCtrl, 0x00431d10 ccThEntryCtrlDelete, 0x00431070 ccEntryCtrl::restoreEntry, 0x004312d0 deleteEnemy, 0x004314e0 deleteMagicCircle, 0x004316a0 deleteGimmick, 0x00431860 deleteNpc, 0x0042df10 ccCheckActiveEnemy, 0x0042df70 ccCheckActiveObject, 0x0042fa60 ccEntryObj::routine, 0x0042fe40 ccEntryObj::deleteCmnd, 0x0042f7d0 ccEntryObj::ccEntryObj, 0x00519630 ccEntryCmnd, 0x00519700 ccDeleteCmnd, 0x004307f0 ccEntryCtrl::entryObject, 0x00430c90 entryObject, 0x00430530 entryObjectCheck, 0x00430ec0 initObject, 0x00431220 entryEnemy, 0x004313f0 entryMagicCircle, 0x004315f0 entryGimmick, 0x004317b0 entryNpc, 0x00430360 entryCircleObject, 0x00430250 entryEnemyObject, 0x0042e750 ccInitRegisterEnemy, 0x0042e8d0 ccRegisterEnemyOne, 0x0042ed70 ccRegisterEnemyList, 0x0042f290 ccAnalyzeEnemyList, 0x00455900 ccMagicCircle::ccMagicCircle, 0x00455b60 ccMagicCircle::main, 0x004548e0 ccMcPart::main, 0x005ab610 WORLD::SetMagicCircle, 0x005bf590 DUNGEON::SetMagicCircle, 0x00432a90 ccEnemy::ccEnemy, 0x00442190 ccEnemyB::ccEnemyB, 0x00446070 ccEnemyG::ccEnemyG, 0x0044a7e0 ccEnemyK::ccEnemyK, 0x0044e550 ccEnemyP::ccEnemyP, 0x00451460 ccEnemyV::ccEnemyV, 0x00447200 ccEnemyG::checkGold, 0x0043a250 ccCheckDustColor, 0x0042e580 ccGetNameBossAnm, 0x00433e80 ccEnemy::moveEnemy, 0x00434560 animEnemy, 0x004328b0 ccEnemyCheckNote, 0x004348b0 dispEnemy, 0x004371e0 actMove, 0x00437420 actFollow, 0x00437460 actSlide, 0x004374a0 actEscape, 0x00437610 actEscape(int, float), 0x00437960 actEscapeX, 0x0043f200 ccEnemy1::action, 0x004424e0 ccEnemyB::action, 0x004426d0 moveEB, 0x004467e0 ccEnemyG::action, 0x00446aa0 moveEG, 0x00446d50 moveEGG, 0x00446710 ccEnemyG::freeze, 0x0044aa90 ccEnemyK::action, 0x0044e860 ccEnemyP::action, 0x004518d0 ccEnemyV::action, 0x0043ad60 ccPiLimit, 0x0041b5f0 ccFellow::Main, 0x0041bc50 ccFellow::Move, 0x0041c670 ccFellow::Action, 0x0059ee20 ccSpcChar::HitCheck, 0x00581580 ccAI::FollowPlayer, 0x00581cb0 LeavePlayer, 0x00582090 FollowTarget, 0x00582ad0 FollowTargetDirc, 0x00589e90 CheckFrontObstacle, 0x00589fe0 CheckFrontObstacleF, 0x005148e0 ccNavi::PathFindingInDungeon, 0x00514cc0 HeuristicType1, 0x00514f60 ShortPath, 0x005150c0 MakeBeaconTbl, 0x00515470 SetBeacon, 0x00514770 GetDestination, 0x00515aa0 SetPathFindingMap, 0x005130b0 ccSetNaviMap, 0x00513300 ccNaviSearchNearLandmark, 0x00582d50 ccAI::MoveP2P, 0x00582930 FollowBeacon, 0x005975b0 CheckGoalBeaconPos, 0x00580ef0 ManualControl, 0x0057f660 ActInTown, 0x005826b0 FollowTargetTown, 0x00598310 ccPlayer::Main, 0x00598af0 ControlMove, 0x005993c0 AnimCtrl, 0x0059c580 ccPlayer::Attack, 0x0059cad0 AttackCancel, 0x0059cbd0 BreakSomething, 0x0059b3c0 MapLoopAdjustPos, 0x0059b470 W2MPos, 0x0059b710 P2WPos, 0x005a0530 ccThSpc, 0x005a1930 ccSpcSetOperation, 0x005a17e0 ccSpcShoutOperationName, 0x0056ba30 ccChar::CalcReal, 0x0056bfd0 ccChar::ConditionTimeCount, 0x0056c940 ccChar::ConditionBattleEffect, 0x0056cb50 ccChar::ClearCondition, 0x0056d300 ccClearSpcCondition, 0x0056f950 ccChar::DispConditionEffect, 0x00570180 ccChar::ClearConditionEffect, 0x005703f0 ccClearParamElement, 0x0056d430 ccChar::CheckLevelUp, 0x0056d640 ccChar::LevelDown, 0x0056d8a0 ccChar::CalcBattleDamage(t, sid, mag, h), 0x0056d910 ccChar::CalcBattleDamage, 0x0056b020 ccChar::EntryAffect, 0x005701d0 ccChar::CheckCharAttribute, 0x00570440 ccAddParamElement, 0x00571100 ccGetJobWeaponParam, 0x00571290 ccExpDistributor, 0x00571830 ccCheckConditionSkillSuccess, 0x00571a00 ccSetLevelParam, 0x005722e0 ccThSkill, 0x005723e0 ccSkillCheck, 0x005725a0 ccSkillAttributeCheck, 0x00572860 _ccSkillRequest, 0x00572f80 ccSkill::ccSkill, 0x005731d0 ccSkill::Main, 0x005738e0 ccSkill::NoteEventAffect, 0x00573a10 ccSkillCheckNote, 0x00573af0 ccSkillCheckTypeAttribute, 0x00573c30 _ccSkillCheckType, 0x00573cc0 ccGetArmsEffectAttribute, 0x00573e60 ccSkillDamage, 0x005743c0 ccSkillDamage (point), 0x005746d0 ccSkillDamage2, 0x00574bc0 ccSkillRecovery, 0x00574f70 ccSkillRecovery (point), 0x005752b0 ccSkillHold, 0x00575520 ccSkillHold (point), 0x005756f0 ccSkillModifyCondition, 0x00575a50 ccSkillModifyCondition (point), 0x00575cf0 _ccSkillModifyCondition, 0x00576c50 ccSkill::ConditionModifySystem, 0x00576ea0 ccSkill::HealingSystem, 0x00577070 ccSkill::RecoverySystem, 0x00579750 ccCheckTargetConditionBySkill, 0x00594e90 ccSkillDamageValue, 0x00432840 ccEnemyInfluence, 0x00433a70 ccEnemy::affectEnemy, 0x0041bdb0 ccFellow::Influence, 0x0041e1e0 ccFellow::CheckNote, 0x0059ac50 Influence, 0x0059c300 ccPlayer::CheckNote, 0x0059d080 checkPartyAnnihilation, 0x0059d100 checkPartyMenberNum, 0x0059e860 ccSpcChar::DistanceToTarget, 0x0059ec70 ccSpcChar::ConditionAdjustment, 0x0059f380 ccSpcChar::CheckSpRegeneSpeed, 0x0059f530 ccSpcChar::CheckSysMsgID, 0x005a1630 ccSpcCheckLevelUp, 0x00532ae0 ccMenuCtrl::DataDrainMenu, 0x00544c00 ccMenuCtrl::AreaItem, 0x00453420 ccEntryGimBox, 0x004534a0 ccGimBox::ccGimBox, 0x004545d0 ccGimBox::main, 0x00453d80 ccGimBox::boxMain, 0x00454140 ccGimBox::objectMain, 0x00454410 ccGimBox::virusMain, 0x00453be0 ccGimBox::invokeTrap, 0x00453ac0 ccGimBox::breakObject, 0x00453400 ccGimmickAffect, 0x00459620 ccEntryGimIdol, 0x004596a0 ccGimIdol::ccGimIdol, 0x00459920 ccGimIdol::main, 0x00438ac0 ccPrimRadiate::ccPrimRadiate, 0x0043a1c0 ccPrimRadiate::main, 0x0043c1b0 ccGimBoxRad::ctrl, 0x0043c140 entryBoxRadiate, 0x005beb30 DUNGEON::SetItemBox, 0x005be750 DUNGEON::SetIDOL, 0x005a19b0 ccSpcMessageOpenTrapBox, 0x00432cd0 ccEnemy::main, 0x00434ac0 ccEnemy::defaultThink, 0x004353f0 ccEnemy::interruptThink, 0x00433470 ccEnemy::setAct, 0x00436940 ccEnemy::selectAttack, 0x004360d0 ccEnemy::checkSkillList, 0x004366a0 ccEnemy::selectSkillTarget, 0x00435ef0 ccEnemy::setSkillRate, 0x00436cb0 ccEnemy::selectTarget, 0x00433260 ccEnemy::initEnemy, 0x0042e3c0 ccGetDrainId, 0x00586130 ccAI::ChatMessage, 0x00586190 ChatMessageModify, 0x00586420 ChatMessageSender, 0x00586570 AffectMessages, 0x00583190 Greeting, 0x0058dc40 MessageIndex, 0x0058d7b0 ChatMessageAttackTarget, 0x0058dc80 ChatMessageAccept, 0x0058ddd0 ChatMessageHealStart, 0x0058deb0 ChatMessageCureStart, 0x0058df90 ChatMessageResurrectStart, 0x0058e070 ChatMessageOOM, 0x0058e130 ChatMessageCanNot, 0x0058e1f0 ChatMessageNoBattleModeDeny, 0x0058e2b0 ChatMessageGhost, 0x0058e390 ChatMessageConditionMinus, 0x0058e450 ChatMessageThanksHeal, 0x0058e5b0 ChatMessageThanksResurrect, 0x0058e680 ChatMessageThanksBuff, 0x0058e750 ChatMessageUseOcarina, 0x0058e810 ChatMessageNoOcarinaDeny, 0x0058e8d0 ChatMessageDisableOcarinaDeny, 0x0058e990 ChatMessageEnteredField, 0x0058eae0 ChatMessageVictory, 0x0058ec30 ChatMessageNeutral, 0x0058f480 ChatMessageHealPlz, 0x0058f540 ChatMessageTreatmentPlz, 0x0058f600 ChatMessageResurrectPlz, 0x0058f6e0 ChatMessageReencounter, 0x0058f810 ChatMessageEnteredTown, 0x0058fb40 ChatMessageChatCmdAccept, 0x005901d0 ChatMessageDamage, 0x00590790 ChatMessageAttack, 0x00590c90 ChatMessageEquipOK, 0x00590d50 ChatMessageEquipNOT, 0x00590ea0 ChatMessageAttributeCritical, 0x00590f60 ChatMessageAttributeGuard, 0x00591040 ChatMessageAttributeFollow, 0x00591140 ChatMessageHealPlzAccept, 0x00591200 ChatMessageSkillAccept, 0x005912c0 ChatMessageAssignAccept, 0x00591380 ChatMessageConditionModify, 0x005918f0 ChatMessageWaitPlz, 0x005919d0 ChatMessageGhostCondition, 0x00591a90 ChatMessageLevelDown, 0x00591b50 ChatMessageLevelUp, 0x00591c10 ChatMessagePresentOtherFellow, 0x00591e00 ChatMessageDeadOtherFellow, 0x00591ec0 ChatMessageOpenTrapBox, 0x00592070 ChatMessageQuitPucciguso, 0x00592150 ChatMessageConditionModifyEnemy, 0x00592210 ChatMessageOpenTreasureBox, 0x005923f0 ChatMessageALotOfEnemy, 0x005924b0 ChatMessageBetterWeapon, 0x00592590 ChatMessageWorseWeaponMessages, 0x00592670 ChatMessageFellowIsHighLevel, 0x00592750 ChatMessageGratsLevelUp, 0x00592900 ChatMessageWalkingTalk, 0x00597760 getAttributeStr, 0x00651ae0-0x00654050 the chat tables, 0x0057ca00 ccAI::Brains, 0x0058aeb0 ccAI::ReadSysMsg, 0x00592aa0 ccAI::Reconnoiter, 0x0057cd00 ccAI::ActInField, 0x0057e0c0 ccAI::ActInDungeon, 0x005941f0 ccAI::SelectAttackSkill, 0x00589820 ccAI::CheckNeedHealing, 0x00586630 ccAI::AttackTarget, 0x00586bd0 ccAI::SearchTargetNear, 0x00583440 ccAI::RequestChatCmd, 0x0058ce10 ccAI::ChatCommandExecute, 0x005959c0 ccAI::CheckSolution, 0x00587440 ccAI::levelCheck, 0x0057aa80 ccUseItemRequest, 0x0057a6d0 ccCheckItemUseful, 0x0057a890 ccCheckSkillUseful, 0x00572790 ccItemSkillRequest, 0x005833a0 ccAI::SetGoalPos, 0x0059e920 CheckBootStatus, 0x0059f850 DelSpc, 0x005a02d0 DeleteNoPartyMember, 0x0042f5a0 ccAddRequestFileListEntry, 0x0042feb0 ccEntryCtrl::initEntryCCS, 0x0042f400 ccRegisterGimmick, 0x0042f380 ccInitRegisterGimmick, 0x00432fc0 ccEnemy::initEnemyCCS, 0x0042e270 ccCheckMiddleBoss, 0x006e10d0 ccChar::Draw's condition colours, 0x005a08a0 ccSpcConditionEffectSW, 0x005a06d0 ccThSpcDelete, 0x00456540 ccGimEtc::ccGimEtc, 0x00456ba0 initFountain, 0x00456760 ccGimEtc::main, 0x00456fb0 ctrlFountain, 0x005c9ba0 DUNGEON::GetNearDoorPosition, 0x005aa990 WORLD::SetFood, 0x005aae90 WORLD::SetSpecialObj, 0x005abd20 WORLD::SetDungeonEnter (the in-points), 0x005b63a0 CheckBossEffect, 0x0042e0c0 ccCheckFountain, 0x0043e760 ccEnemyWeaponCtrl::ccEnemyWeaponCtrl, 0x0043e850 ccEnemyWeaponCtrl::~ccEnemyWeaponCtrl, 0x0043e900 ccEnemyWeaponCtrl::ctrl, 0x0043eae0 ccEnemyWeaponCtrl::note, 0x0043c970 ccEnemyWeapon::ccEnemyWeapon, 0x0043cc30 ccEnemyWeapon::~ccEnemyWeapon, 0x0043cda0 ccEnemyWeapon::init, 0x0043ce60 ccEnemyWeapon::setEdgeRate, 0x0043d130 ccEnemyWeapon::initCell, 0x0043d190 ccEnemyWeapon::initCellColorHSV, 0x0043d2c0 ccEnemyWeapon::getNewCell, 0x0043d3b0 ccEnemyWeapon::interpolateCell, 0x0043d720 ccEnemyWeapon::makePacketCell, 0x0043db50 ccEnemyWeapon::setCell, 0x0043de90 ccEnemyWeapon::ctrlCell, 0x0043e110 ccEnemyWeapon::dispCell, 0x0043e3f0 ccEnemyWeapon::ctrlRadiate, 0x0043e680 ccEnemyWeapon::checkWeapon, 0x0043c600 ccEnemyWeaponRad::ctrl, 0x004382a0 eneSplineH, 0x004393a0 ccPrimRadiate::create, 0x00438580 ccPrimPacket::makePacket, 0x00438820 ccPrimPacket::sendPacket; INF SLUS_202.67:0x001da150 ccSetRad, 0x001da5d0 ccSetDist, 0x001da030 ccGetDircChgF, 0x001da0b0 ccSetDirc, 0x001d9c10 ccRandS, 0x00110c28 sceVu0RotMatrix, 0x00153220 ccCharHit::ccCharHit, 0x00153470 ccCharHit::CollisionDetection, 0x00153930 _ccHitCheckLM, 0x001da710 ccCheckCameraDeg, 0x001da880 ccGetCameraTransparency, 0x00152210 ccAnm::NoteProcess, 0x001a1f20 WORLD_MAN::EntryGimmick, 0x001b62e0 ccEntryEventMng, 0x001a22f0 WORLD_MAN::Get2DMapInfo, 0x00178030 ccSaveData::CheckEventEntry, 0x00133a38 rand, 0x001787a0 AddLvErosion, 0x001d9a10 ccRand, 0x001d9620 genrand, 0x001d9a90 ccRandF, 0x00177730 ccSaveData::AddItem, 0x001aa290 remove, 0x001ad8ac pc_walk_pos, 0x001ad9b0 pc_run_pos, 0x001adab0 pc_walk_dir, 0x001adbf4 pc_walk_marker, 0x001adda4 pc_walk_char, 0x001ae164 pc_command, 0x001ae9e8 party_put_marker, 0x001aec5c party_put, 0x001aeb84 pc_put, 0x001aee30 pc_face, 0x001af028 enemy_put, 0x001b09fc hold, 0x001b0a68 hold_end, 0x001b5040 ccThEvHold, 0x001b1f04 player_skill, 0x001b23a8 battle_ready, 0x001da610 ccGetDircPL, 0x001a3c40 WORLD_MAN::SetEventData, 0x0013d610 ccClump::ChangeClut, 0x0013df20 ccSetFogBlendColor, 0x001057f0 ccDrawEnv::SetFogBlend, 0x00378ce4 spcConditionEffectFlag, 0x0015a6a0 ccThControl, 0x001782f0 ccSaveData::CheckFountain, 0x001a3960 WORLD_MAN::GetFood, 0x001a4180 WORLD_MAN::GetDungeonMarkerPoint, 0x001a4130 SetDungeonMarkerPoint, 0x001b71b0 ccRegisterRegularGimmick, 0x0059ddd0 ccSpcChar::ArmsEffect, 0x0059e3b0 ccSpcChar::ClearArmsEffect, 0x0059e460 ccSpcChar::SetArmsEffectColor, 0x0059e4b0 ccSpcChar::_SetArmsEffectColor, 0x0059e530 ccSpcChar::StartArmsEffect, 0x0014d4d0 ccStream::Decode_DummyPos; INF gcmn.prg:0x0045a0d0 ccGimSymbol::ccGimSymbol, 0x0045aa40 ccGimSymbol::main, 0x0045ab00 symMain, 0x0045aed0 objMain, 0x0041e590 ccFellow::Attack, 0x00589410 ccAI::UseItem, 0x00587e20 ccAI::CheckHealParty, 0x00596b00 ccAI::HealSPC, 0x005937d0 ccAI::DebuffForUnusedEnemy, 0x00593ce0 ccAI::BuffForUnusedFellow, 0x005962d0 ccAI::ChatCommandHealPlz, 0x005857c0 ccAI::ChatCommandDeBuffPlz, 0x00585c70 ccAI::ChatCommandBuffPlz, 0x00583750 ccAI::ChatCommandFulfilCheck, 0x00583d60 ccAI::ChatCommand, 0x0041a990 ccThBook, 0x0042e4f0 ccClearConditionAllEnemy, 0x005a0880 ccSpcConditionEffectON, 0x0059aa90 ccPlayer::SetTargetDirc, 0x0059ab80 ccPlayer::SetTargetDist, 0x0059c550 ccPlayerCheckNote, 0x0059ca40 ccPlayer::DamageActuate, 0x0059d630 ccSpcChar::SetActNum, 0x0059d640 ccSpcChar::SetActNumOld, 0x006fa900 ccRegisterEnemyTbl, 0x006fa920 ccRegisterDrainTbl, 0x00443330 ccEnemyC::moveECS, 0x00450b30 ccEnemyU::moveEU, 0x00448b10 ccEnemyG::actEscapeGold, 0x00438d10 ccPrimRadiate::init, 0x00438e30 ccPrimRadiate::setColorRadiate, 0x00439520 ccPrimRadiate::createPlate, 0x00439df0 ccPrimRadiate::disp; INF SLUS_202.67:0x00378190 ccRegisterEnemyRange, 0x00378194 ccRegisterEnemyNum, 0x00378198 ccRegisterDrainNum, 0x0037890c ccCharHitTop, 0x00378910 ccCharHitTail; MUT gcmn.prg:0x00597bb0 ccSkillCheck, 0x005b25f0 ccAI::ReadSysMsg's uses, 0x005a3810 ccAI::ActInField (mode 4), 0x005a5104 ccAI::ActInDungeon (mode 4), 0x005a97e0 ccAI::RequestChatCmd, 0x005acaf0 ccAI::ChatCommandDeBuffPlz, 0x005ad010 ccAI::ChatCommandBuffPlz, 0x00597cb0 the maxSP check, 0x005c2380 an item's heal
worklog: 22, 25, 79, 170, 266, 274, 281
---

# Battle rules

Effective stats, hit and damage, the protect gauge, status effects, skills
from request to effect, what an attack or a heal does to each kind of
character, experience, infection, Data Drain and item drops; and the
motion that carries them on the field: the entry control and its magic
circles, the enemies' movement and animation, Kite's and the party members'
frames and acts, following and path finding. Two implementations check them
against the game's own code run in `tools/eemu.py`:

- `tools/battle.py`, checked by `tools/test_battle.py` on all four volumes
  (the formulas of the first sections): the return value, every struct
  field, the RNG state and the order of calls to effect functions, 240,000
  random cases, 0 mismatches.
- `crates/piney-battle`, the Rust port of the rules and the motion (no
  drawing), checked on Infection by `tools/test_battle_rs.py` and its
  companions (see [The port](#the-port)).

Integer arithmetic is C's: division truncates toward zero, products wrap at
32 bits, stored fields at 16. Addresses are `gcmn.prg` unless marked main.
Times are in frames; 60 frames per second is assumed from the timer
constants. The tables the rules read are on the
[game data](../content/game-data.md) page.

## RNG

Battle uses newlib `rand()` (main `0x00133a38`), not the area generator's:

```
state (u64) at *_impure_ptr + 168
state = state * 6364136223846793005 + 1
return (state >> 32) & 0x7fffffff
```

One generator serves the whole game, so every draw anywhere (the Data Drain
movie's noise, the party AI, the menus) moves the battle rolls.

The enemies draw from a second generator, `ccRand()` (main 0x001d9a10), which
returns `genrand()` (main 0x001d9620) as a signed `int`, so half its values
are negative and `ccRand() % 3` runs from -2 to 2. `genrand` is MT19937 with
the 1998 `sgenrand` seeding: `mt[624]` at main 0x003ff400 (64-bit words
holding 32-bit values), `mti` at main 0x00377fd0, 625 in the executable, so
the first call seeds with 4352 (as `ccInitRand`, main 0x001d9900, does).
`ccRandF(x)` (main 0x001d9a90) is `x * ccRand() / 2^31` in double
precision. Spawning and the enemies' movement draw from it too.

## Targets and areas

A character takes part in the rules only while it is on one of the three
command lists (`cmndPcRoot`, `cmndEneRoot`, `cmndObjRoot`, linked through
`ccChar` +0xbc); `ccCheckTarget(ch)` walks them. `CalcBattleDamage` checks
both the attacker and the target, most other rules the target. A drained
enemy is taken off (`ccDeleteCmnd`, 0x00519700).

Type bits (`base.type`): 0x1, 0x2 and 0x4 a party member (Kite's `charTbl`
row carries 7, the others 6), 0x20 and 0x40 an enemy (every `enemyTbl` row
carries 0x20), 0x80 a boss.

An area rule takes every living character of the target's side whose
distance on the ground from the centre, less its own width, is within the
skill's `targetRange`:

```
d = sqrtf(|posP - centre|^2, lane 2 of the difference zeroed) - base.width
```

`posP` is the position in the player's frame (`sceVu0SubVector`,
`sceVu0InnerProduct`, newlib `sqrtf`). The centre is the target, or the
caster for type bit 0x2000; the side is the party for a target of type
0x2 or 0x4, the foes for 0xe0. The point variants take a world position
converted with `ccTransPosW2P` (0x0059b940).

## Skill types

`ccSkillParam.type` (+0x24):

| bit | meaning |
| --- | --- |
| 0x1 | physical: pAtk, pDef, pHit, pEva |
| 0x2 | magic: the m- stats |
| 0x4-0x80 | soil, water, fire, wind, thunder, dark (`4 << i`) |
| 0x100, 0x200 | set on attack and support spells; no rule reads them |
| 0x400, 0x800, 0x1000 | the three arts of each weapon class |
| 0x2000 | the area is centred on the caster |
| 0x4000 | goes on when its target dies or leaves |
| 0x8000 | splash: the others in the area take x0.5 |
| 0x10000 | buff |
| 0x20000 | debuff |
| 0x40000 | heal |

`_ccSkillCheckType` (0x00573c30): -1 a plain physical attack, 0 an art, 3 a
heal spell, 2 a buff, -2 a debuff, 1 an attack spell. The skill's element is
the last element bit set (`ccSkillAttributeCheck`, 0x005725a0) or the first
(`ccSkillCheckTypeAttribute`, 0x00573af0), numbered 2 soil to 7 dark.
Opposing elements: soil and wind, water and fire, thunder and dark.
`CheckCharAttribute` (0x005701d0) gives a character's strongest element from
its table row (-1 on a tie at the top).

## Effective stats: `ccChar::CalcReal` (0x0056ba30)

Each character has a 16-entry `elm` block of stats (pAtk, pDef, pHit, pEva,
mAtk, mDef, mHit, mEva, then six element attributes and two more), plus
`temp` (buffs and debuffs) and `real` (the result).

```
party member:  tune = head + body + leg + arm + weapon     each addition clamped to +/-999
               real = clamp(clamp(elm + tune, +/-999) + temp, +/-999)
enemy, boss:   real = clamp(table row elm + temp, 0, 32767)
```

- The pieces are `ccSpcParam.equipment` (+0xc8); each `ccEquipmentParam`'s
  `elm` block is at +0xa.
- The weapon row is `equipmentWeapon<job+1>Tbl[weapon]` (`ccGetJobWeaponParam`,
  0x00571100).
- `maxHP`/`maxSP` come from `ccSpcParam` +0x24/+0x26.
- `ccAddParamElement` (0x00570440) stores the wrapped short, then clamps.
- A new game copies `charTbl` into `saveData.spcParam` (`NewGame`, main
  `0x00174d70`), so level-1 stats are the table row.
- `flag == 0` first runs `ConditionTimeCount`, and for a foe counts down a
  protect break.
- Every frame each character's own `Main` calls it: `ccPlayer::Main` and
  `ccFellow::Main` with the character's `dead` condition (`ccChar` +0x08),
  so a party member down runs no timers; `ccEnemy::routineEnemy` and
  `ccBoss::Main` with 0. The equipment and item menus call it after a
  change.

`ConditionBattleEffect` (0x0056c940) sets `drainHP`, `drainSP`, `critical`,
`dying` and `invincible` to the largest `beff` value of any worn piece, or of
the foe's table row.

## Hit and damage: `ccChar::CalcBattleDamage(t, sk, mag, h)` (0x0056d910)

Physical skills (type bit 0x1) use pAtk, pDef, pHit and pEva. Magic skills
(bit 0x2) use the m- stats. `h < 0` draws the roll and applies side effects,
`h == 0` is a sure hit at 100 with none, and `h > 0` is a given roll with side
effects. Normal attacks by the player, party members and traps pass
`mag = 1.0`, `h = -1`.

```
roll = rand() % 101
roll < 5:   miss
roll >= 95: hit, hit = 100
otherwise:  hit = roll + (atk.Hit + sk.hit - 2*tgt.Eva/3) / 10
            hit < 50 misses; hits cap at 100

a   = max(atk.Atk + sk.atk, 1);   d = max(tgt.Def, 1)
dmg = max(a*a / (2*d) * hit / 100, 1)
for each element i with type bit (4 << i) and sk.attr[i] != 0:
    a = max((sk.attr[i] + atk.attr[i]) / 10, 1);   d = max(tgt.attr[i] / 10, 1)
    dmg += a*a / (2*d)                                     (after the hit scaling)
dmg = dmg * sk.dmgRate / 100
dmg = (int)((float)dmg * mag)                              cvt.s.w, mul.s, fptosi
dmg = min(dmg, 9999)
```

A trap or other non-character attacker never misses and brings no stats.
Nothing happens (-1) unless both characters are on the command lists.

**Absorption.** For each element term where the target's attribute is 999
or more, the target also recovers `max(term / 10, 1)` through
`ccEntryRecoveryReq`, and still takes the damage. A skill with the element
bit but attribute 0 gives 1.

**Exdefense** (table row +0x64: bit 0x1 pDef, 0x2 mDef, 0x4-0x80 elements).
When a bit is set in both Exdefense and the skill type, and the target's
current `real` value for it is not below the table's, the damage is 0.
Lowering that stat below the table value removes the immunity. The
"attribute guard" particle shows only when the matching bits are elements.

**Equipment effects.** Only on a live physical hit by a skill that costs 0 SP
(`sk.cost`, +0x28: the normal attack). Each is a roll `rand() % 100 <= value`,
in this order:

| effect | result |
| --- | --- |
| critical | dmg x 2 |
| dying | dmg at least HP*3/4; not on bosses, only when target +0x140 is set |
| drainHP | attacker gains (dmg+1)/2, or (HP+1)/2 when HP < dmg |
| drainSP | the same, capped at the target's SP |
| invincible | if the *target* has it, dmg = 0; the chance is the *attacker's* value |

The last is a game bug as written. Each drain's `EntryAffect` on the
attacker (kinds 9 and 10) runs at once, so on a character hitting itself
(charmed or confused) the SP drain's half reads the HP the HP drain left,
capped at maxHP (measured).

**Attribute critical.** `_ccSkillRequest` (0x00572860) rolls it when a skill
is started against a foe: the skill's element opposes the foe's strongest
(`CheckCharAttribute`), no unbroken Exdefense immunity stands in the way
(bit 0x1 with a physical skill, 0x2 with magic), and `(rand() >> 3) % 100 <
50`. In `ccSkillDamage` (0x00573e60) the aimed target of a skill with the
flag then takes a sure hit (`h = 100`) at `mag = 2.0`, and later hits of the
same skill stay doubled (`acFlag` left at -1; in an area the sure hit carries
to the characters after it too). In an area, the others of a splash skill (bit
0x8000) take `mag = 0.5`.

## The protect gauge

The gauge fills on a live hit when `saveData.plcol` (+0x6771) is set, except:
- when `ccMenuCtrl.forbid` is set;
- when the attacker is a party member with `noDeathFlag`;
- against the drained form of an enemy (`ccCheckDrainEnemy`, 0x0042e330);
- while a break is running.

```
pp  = max(a*a / (2*PPdef) * hit / (boss ? 80 : 100), 1) + element terms
pp  = pp * sk.dmgRate / 100, then mag and the 9999 cap
      (a physical hit on a boss caps before mag, everything else after)
PP += pp                                   ccEnemyParam +0x60, starts at 0
PP >= maxPP (+0x52): break - PPcount = 300 frames, PP = 0
```

`PPdef` is `pDefPP`/`mDefPP` (+0x54/+0x56). An enemy breaks only if the hit
does not kill it; a boss always breaks. In `CalcReal` with flag 0 the break
counts down, then PP restarts at maxPP/2 (a boss: maxPP*3/4 from its second
recovery on).

## A skill's life

Every skill use, the normal attack included, is a `ccSkill` (0xb0 bytes)
on the list `SkillEntryTop` (main 0x00378c94).

**Request.** `_ccSkillRequest(cp, tp, sid, stype)` (0x00572860) starts one.
`stype` 0 is a character's own skill (`ccSkillRequest`), 1 an item's effect
on the target alone and 2 an item used through the caster
(`ccItemSkillRequest`). For stype 0 and 2 a normal attack of the caster's
that is running ends. A caster using a skill (stype 0, sid 2 up, not a
buff, heal or cure) pays the SP here; buffs, heals and cures pay when they
take effect. The caster's `skillID`, `skillStatus` (1, or 9 for stype 2)
and target are set, the attribute critical is rolled, and for stype 0 and
2 and ids 6 and up the skill's name is shouted (`ccWordsPlay(sid, cp)`,
gcmn 0x00572b98; [sound](sound.md#voice)). The new `ccSkill` holds its targets while it
runs (`ccSkillHold`) unless it is a normal attack (sids below 6), a buff, a
debuff or a heal; a condition or buff skill (type 0x30000) is flagged for
`ConditionModifySystem`.

**Frames.** `ccThSkill` (0x005722e0) runs every skill's `Main` (0x005731d0)
once a frame in list order and deletes those whose status (flag byte bits
4-5) turned non-zero. `Main`:

1. drops a caster that died or left the lists (clearing its skill fields
   when the skill is the normal attack);
2. ends the skill when its target died or left, unless the skill has type
   bit 0x4000;
3. plays the skill's animation, if it has one, handling each note it
   passes (`ccSkillCheckNote`, 0x00573a10): 0x8005 is `NoteEventAffect`
   with the note's value, 0x8002 a shock wave at the target, 1 and 2 a
   sound of a party caster;
4. by kind: a condition or buff skill runs `ConditionModifySystem`
   (0x00576c50) on its first frame; a heal (150-155, 295) `HealingSystem`
   (0x00576ea0) at frame 75, at once for an item's; a cure (178-180)
   `RecoverySystem` (0x00577070) on its first frame; an attack spell its
   element's system (jump table at 0x006e1210, by skill id): Fall
   (0x00577a30), Tornado (0x00577540), Convergence (0x00578060), Upheaval
   (0x005787c0), Summons (0x00579000). The systems' own frames and releases
   are on the [effects](effects.md) page ("The attack spells"), checked
   frame by frame on Infection and Mutation by
   `tools/test_effect_spell_rs.py`.

**Notes.** `NoteEventAffect(kind)` (0x005738e0): 1 and 3 deal the skill's
damage (`ccSkillDamage`) on the target, or around where it was when it is
gone or down; 2 and 4 are `EntryAffect(kind, 1)` on the target; 5 and 6
start and stop holding.

**Interrupts.** `ccSkillCheck(ch)` (0x005723e0) finds the first running
skill of `ch`'s, status 0, that a hit interrupts: one flagged so (flag
bit 7), a physical skill, or a spell while Kite is in a casting act (17,
18). Only characters of type 0x0700000f have one. From Mutation on
(0x00597bb0) the rule is another: any such skill of a character without
those type bits, and of one with them only the skill that is its
`skillID`.

**Healing** (`ccSkillRecovery`, 0x00574bc0): the Repth skills heal 150, 400,
and the target's maxHP; skill 295 heals its parameter. A single target
takes `EntryAffect(7, amount)` whatever its state; an area heals every
living member of the *caster's* side. `HealingSystem` then has a caster of
its own skill pay the SP.

**Holding** (`ccSkillHold`, 0x005752b0): `EntryAffect(5)` on the target, or
every living character of its side in the area, which sets `cond.hold`.

**Cures** (`ccSkill::RecoverySystem`, 0x00577070), nothing while the target
is off the lists or down (except Resurrect):
- Antidote (178): poison, paralysis, slow, and lowered physical and element
  stats.
- Restorative (179): charm, confusion, sleep, curse, and lowered magic stats.
- Resurrect (180): full HP and 0 SP, but not when the whole party is down.

A party caster of its own skill then pays the SP. The skill ends, unless a
party caster's attack animation is still running.

## Affects

`ccChar::EntryAffect(ch, kind, p0, p1, p2)` (0x0056b020) is the one way
damage, healing, SP changes, revival and a Data Drain reach a character.
Nothing happens to a character off the lists or to a drained foe (its last
affect was 13). Kind 5 sets `hold` and stops; kind 6, and a character
without an `affectFunc`, stop. Otherwise the affect is stored with its
flash (kinds 1-4 red, 7-10 green) and, unless the character's mask has the
kind's bit, applied at once by the character's `affectFunc`:

| kind | meaning |
| ---: | --- |
| 0 | a party member's AI stops talking |
| 1, 3 | damage, poison (`p0` HP, -1 a miss; `p1` the skill) |
| 2, 4 | SP loss |
| 5 | hold |
| 7, 9 | HP gain |
| 8, 10 | SP gain |
| 13 | Data Drain |
| 14, 15 | greetings |
| 16, 17, 18 | cure, buff, debuff: messages only |
| 20 | revival |
| 21 | a boss is drained (its own `Affect`) |

**Enemies** (`ccEnemyInfluence` 0x00432840, `ccEnemy::affectEnemy`
0x00433a70):
- Damage and poison: the number and the hit mark, then HP falls by the
  amount, not below 0. A party member of type 4 out of the party does no
  damage.
- A virus-flagged enemy (enemy flag bit 6) takes a tenth and never falls
  below half its maxHP.
- Hit or missed, the enemy picks its target again (`selectTarget`).
- HP gain (7, 9) and SP gain (10) stop at the maxima, SP loss (4) at 0.
- Revival (20) of an enemy down (`dead` 2) restores full HP and no SP.
- A hit on a defence the enemy is immune to shows its shield.
- 13 marks the enemy drained and changes nothing else.

**Kite and party members** (`Influence` 0x0059ac50, `ccFellow::Influence`
0x0041bdb0), one body with differences:
- Damage and poison: the number (-1 a miss, and nothing more), the hit
  mark, the party panel shakes, Kite's pad rumbles. HP falls, not below 0;
  a party member loses it even when already down, Kite does not.
- A party member hit by a foe's attack turns to it.
- Poison that leaves HP stops there. Otherwise the hit interrupts the
  running normal attack (`ccSkillCheck`), and a survivor of an attack
  (kind 1) plays a hurt act (7 or 8, `rand() >> 3 & 1`) unless already past
  act 7.
- HP 0 is death unless the character has 1000 exp waiting (a level up) or
  `noDeathFlag`: the down act (9), `dead` 2, conditions cleared, SP 0, the
  "down" message to the other members' AI.
- `noDeathFlag`: HP and SP only ever rise.
- HP and SP gain do nothing to a character down.
- Revival (20): `dead` 5 and the getting-up act (2).
- A Data Drain on a member (13): poison and curse 5400 frames, paralysis and
  sleep 450, charm and confusion 300, slow 900 at 0.5; HP and SP cut to half
  their maxima.

**Down and up.** `ccSpcChar::ConditionAdjustment` (0x0059ec70), when a
character leaves an area (`ccSpcChar::TransferOut`), is set up in one
(`ccSPC::Wakeup`) or at a fountain (`ccMenuCtrl::FountainMenu3`): a character
down (`dead` 2-4) becomes a ghost (`dead` 4: HP and SP 0, half transparent);
one revived (`dead` 5) stands up with full HP and no SP.

## The normal attack

The normal attack (skill 1) and the arts deal their damage from the
attacker's own animation, note 0x8005:
- Kite (`ccPlayer::CheckNote`, 0x0059c300): the target is his `targetChar`
  (listed, alive, HP left). Within 450 on the ground, less the target's
  width, the hit is `CalcBattleDamage(target, skillID, 1.0, -1)` and
  `EntryAffect(1, dmg, skillID)`; farther, a miss (-1).
- A party member (`ccFellow::CheckNote`, 0x0041e1e0): in range when
  `ccSpcChar::DistanceToTarget` (0x0059e860: the distance less both widths)
  is within the AI's `armsRange`; a hit on a foe is reported to its AI.

## Status effects

`_ccSkillModifyCondition` (0x00575cf0), durations in frames, on a foe / on
the party:

| condition | foe | party |
| --- | ---: | ---: |
| poison | 5400 | 5400 |
| paralysis, sleep | 900 | 450 |
| charm, confusion | 600 | 300 |
| curse | 5400 | 5400 |
| slow (x0.5) | 1800 | 900 |
| haste (x1.75) | 9000 | 9000 |
| regenerate HP / SP | 5400 | 5400 |
| stat debuff (skill's atk, hit or attribute into `temp`) | 1800 | 900 |
| stat buff | 9000 | 9000 |

A paralysis, charm, confusion or sleep already running is not renewed
(`EntryAffect(3, -1)` instead). `ccSkillModifyCondition` (0x005756f0) applies
it to the target or every living character of its side in the area, after
the resistance roll (not for an item's poison or curse, nor a forced one).

**Resistance** (`ccCheckConditionSkillSuccess`, 0x00571830): only party
members and foes on the lists; resisted when `rand() % 1000 + 100 <
tolerance`, so 1000 or more always resists. Body tolerance covers skills
156-158, 163-165, 169-174; spirit covers 159-162, 166-168.

`ccCheckTargetConditionBySkill` (0x00579750) tells the AI whether a character
already has what a skill gives.

**Every frame** (`ConditionTimeCount`, 0x0056bfd0):

| effect | amount | every |
| --- | --- | ---: |
| poison | -max(maxHP/100, 1) HP, only while no menu is open | 90 |
| regenerate HP | +max(maxHP/50, 1) | 60 |
| regenerate SP | +max(maxSP/50, 1) | 60 |
| curse | -max(maxSP/100, 1) SP; stops natural SP regeneration | 90 |
| natural SP, party | maxSP/100 in battle, maxSP/33 outside; maxSP/50 and maxSP/20 when `CheckSpRegeneSpeed` | 60 |
| natural SP, enemies | maxSP/50 | 60 |

`CheckSpRegeneSpeed` (0x0059f380) holds while the character stands still and
uses no skill. Buffs and debuffs expire; pEva, mEva and the tolerances are
never timed.

## Experience and levels

- **Levels.** A flat 1000 exp per level. `CheckLevelUp` (0x0056d430) adds the
  character's `LevelUpParamTbl` row per level; stats clamp to 0..999, maxHP
  to 9999, maxSP to 999. Level 99 zeroes exp. `LevelDown` (0x0056d640) is the
  Data Drain level loss. `ccSetLevelParam` (0x00571a00) is a one-step debug
  version. `ccSpcCheckLevelUp` (0x005a1630), every frame of `ccThSpc`
  outside the Root Town, levels up in the save each member away from the
  party that shares experience (maxHP and maxSP unclamped there).
- **Per kill** (`ccExpDistributor`, 0x00571290, from `ccEnemy::interruptThink`):
  `expCalcTbl[clamp(enemy level - member level, -10, 10) + 10]`, a u16 table
  (0x006518f0).

  | diff | -10 | -9 | -8 | -7 | -6 | -5 | -4 | -3 | -2 | -1 | 0 |
  | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | exp | 1 | 2 | 3 | 4 | 6 | 8 | 13 | 28 | 40 | 50 | 60 |

  | diff | +1 | +2 | +3 | +4 | +5 | +6 | +7 | +8 | +9 | +10 |
  | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | exp | 70 | 80 | 100 | 130 | 170 | 220 | 280 | 350 | 430 | 520 |

  - Members outside the party but flagged in both `partyMemberFlag` and
    `partyMemberExp` get 60%.
  - Dead members and level-99 members get nothing.
  - The enemy table's own `exp` field is never read.
  - Each kill lowers `saveData.erosion` by `count % 3 + 2`, not below 0.
- **Infection** (`AddLvErosion`, main `0x001787a0`):
  `erosion += erosionTbl[clamp(target level - Kite's level, -5, 10) + 5] x factor`,
  capped at 100. The factor is 1 for Data Drain, 1.5 for Drain Arc and 2128
  Drain, and 3 for Drain Heart.

## Data Drain

`ccMenuCtrl::DataDrainMenu` (0x00532ae0) plays a drain once Kite picks it; its
step is `ccMenuCtrl` +0x1a:

| step | what happens |
| ---: | --- |
| 0 | the movie, the enemy's transformation, then the drain proper; at once step 1 |
| 1 | a side effect when `rand() % 100 <= infection / 2`: step 10, else 2 |
| 2, 3 | the warning over 45 frames of noise, then 20 |
| 10 | the side effect |
| 11 | its message; at frame 30 a level lost |
| 12 | the message closed, then 20; a lost key item ends the game (`compulsionGameOver`) |
| 20 | the drain count and the bracelet's growth |

Before the movie a boss takes `EntryAffect(21)` from Kite.

**The drain** (step 0 after the movie, 0x00533010-0x00533470):
1. The infection rises (`AddLvErosion`, above).
2. The aimed enemy drops by `rand() % 100 + 60` for 2128 Drain and Drain
   Heart, else `rand() % 100 + infection / 2`: 96 and up its row's
   `item[2]`, 50 and up `item[1]`, below `item[0]`. A boss always gives
   `item[0]`.
3. Drain Arc (3) and Drain Heart (5) also take each other living foe on the
   list whose protect is broken and whose ground distance from the target's
   `posP` is within `targetRange` plus its width, at most 16. Each rolls
   `rand() % 100 + infection / 2`, with the items the other way round.
4. Every target but a boss leaves the command lists.

The item menu (`DataDrainSubMenu`, 0x00535210) hands the drops out one at a
time.

**Side effects** (step 10): `dataDrainErosionTbl[infection / 25][rand() &
15]` (0x00651430) picks one of 31, then one roll `rand() % 1001` is tested
against the tolerance of every condition it brings (resisted below it).
Nothing happens to Kite while he is down. The effects and numbers it
starts on each character, in order, are in
[the field menus](field-ui.md#data-drain-datadrainmenu-menu-66)
(`piney_battle::drain::SideStart`).

| effect | what |
| ---: | --- |
| 0 | every living member's HP and SP full |
| 1-6 | Kite's pAtk, pDef, pHit, mAtk, mDef, mHit -20 for 900 frames |
| 7-13 | Kite poisoned (5400), paralysed (450), slowed (x0.5, 900), charmed (300), confused (300), asleep (450), cursed (5400); the first three resisted by body, the rest by spirit |
| 14-20 | the same on every living member |
| 21, 22 | every living member's HP, SP halved, `(v + 1) / 2` |
| 23-27 | Kite loses 200, 400, 600, 800, 1000 exp; below 0 he gets 1000 back and loses a level from level 2, else stops at 0 |
| 28 | every living member left with 1 HP and 1 SP |
| 29 | an item lost: the first of Kite's list from slot `rand() % 20` on, unless a key item comes first; failing that the first from the start that is not a key item |
| 30 | nothing |

**Growth** (step 20, 0x00534c6c): `saveData.drainCount` (+0x685e) rises, at
most 10000. The bracelet (`+0x6860`) grows one stage at 10, 20, ... 80
drains, each adding key item 273-280 to Kite's list; stages 8-10 at 80, 160
and 240 drains belong to the later volumes and add nothing. No growth in a
Parody game (`+0x842b`).

## Item boxes, traps and idols

`ccMenuCtrl::AreaItem(item, kind)` (0x00544c00): a fixed item (`item >= 0`)
as it is; otherwise entry `base + floor + 1 + rand() % 5` (at most 129) of a
130-entry list chosen by `kind`:

| kind | list |
| ---: | --- |
| 0 | `ItemBoxList[server * 6 + field attribute]` (0x0064c450) |
| 1 | `DangerItemBoxList` (0x006a1ed0) |
| 2 | `SukaItemBoxList[server]` (0x006a29a0) |
| 3, other | `areaItemList` (0x0069e070), a trap's |
| 4 | `idolItemList` (0x006502b0) |
| 5 | `idolSubItemList[server]` (0x00650d80) |

`base` is the event area's level when `ccGame.field` is set, else the field's
`(level - 1) x 25 + 10 + offset`.

## The dungeon's objects

Treasure boxes, the breakables and the virus crystal are `ccGimBox`
(gimmick rows 0-14, gcmn 0x00453400-0x00454838), the Gott and Zeit statues
`ccGimIdol` (rows 38-44, 0x00459620-0x00459d4c), the Grunty foods
`ccGimFood` (rows 22-37, `FOOD_00`-`0F`, 0x00458220-0x00458de4).

**The box.** `ccEntryGimBox` makes a `ccGimBox`: the entry copied in, its
model by row, a body in the collision list, the opening's rays and, for a
trapped row (1, 3, 5) whose `param[2]` is not 0-2, a trap from `ccRand()
& 3` (3 gives 2, 2 gives 1, else 0). Trap 0 is skill 1 with the dying
condition 100, trap 1 skill 156, trap 2 skill 162 (`skillID`, +0x7c). It
fades in over 10 frames, radiates, and its clean-up is 0x00453470. Its
`main` (0x004545d0) runs the rays, then `boxMain` for rows 0 and 1,
`virusMain` for 6, `objectMain` for the rest, then the animation a step.

```text
boxMain (0x00453d80)
  affect 11 (opened)      act 2
  affect 12 (disarmed)    row 1: a row-0 box made in its place (param[2]
                          3), effRemoveTrap(pos, -1, -1), sound 104, gone
  act 0                   a box a foe or portal left (entRoot 1-3) goes
                          after 902 frames out of battle
  act 2, first frame      row 1's trap (invokeTrap); off the command list
                          (deleteCmnd); its event entry used (param[1] not
                          -1); ANM_xgbbox02, sound 73, effOpenBox, the rays
                          shine; act 3 fades it out
objectMain (0x00454140)   affect 11 breaks it (rows 3, 5 their trap);
                          affect 12 on rows 3, 5: the untrapped row in its
                          place
virusMain (0x00454410)    affect 11: off the list, 29 frames, then sound
                          75, effVirusCrystal, a 30-frame fade
invokeTrap (0x00453be0)   ccSpcMessageOpenTrapBox; trap 0:
                          CalcBattleDamage(box, opener, skill, 1.0, -1) and
                          the opener's EntryAffect(1, damage); traps 1, 2:
                          ccItemSkillRequest(box, opener, skill, 0); sound
                          39, effOpenTrapBox
```

The opener is `affectPerson`. `ccGimmickAffect` (0x00453400, the
gimmicks' `affectFunc`) only raises `affectFlag`. The trap's
`CalcBattleDamage` runs before `deleteCmnd` in the same frame, so both are
still on the command lists and it lands (an object attacker never misses:
[the damage](#hit-and-damage-cccharcalcbattledamaget-sk-mag-h-0x0056d910)).

**The rays.** `ccPrimRadiate` (0x00438ac0) with `ccGimBoxRad::ctrl`
(0x0043c1b0): gouraud strips (`PRIM` 0x4c, `ALPHA` 0x44, `TEST` 0x73001,
`ZMSK`) on layer priority 30, from the object's position, turning about x
and pulsing for 52 frames with an omni light in the scene's group. Act 0
takes the user's position, the colours `ccFractionalHsv(0x802affff, 1.0,
0.4)` inside and `(.., 0, 0)` outside, length 80, width 60, zoom 4,
`dpLength` and `dpBank` 0.1 and the height it shines from (`param[2]` its
z), and goes on to act 1 at once. Act 1, for 52 frames, turns it about x
(0.18 a frame), pulses it with `param[0]` (alpha 0.5 sin, the light 3 sin,
length 30 + 60 sin, width 30 + 30 sin, zoom 1 + 2 sin, `dpLength` 0.2 sin,
`dpBank` 0.1 sin; +0.0628 a frame) and sways it with `param[1]` (bank pi/2
sin, z `param[2]` - 10 + 60 cos; +0.0314 a frame); act 2 puts it out.

**The idol.** `ccEntryGimIdol` makes a `ccGimIdol`: `CMP_trall` with
`idolAnimTbl[row - 38]`'s clips (standing, opening, open); one whose event
entry is used (`param[2]` 0) stands open. `main` (0x00459920): the
statue's glow once (`effStatueOfGod`, at (0, 800, -530) turned by its
heading), affect 11 opens it (sound 73, effOpenBox, the rays; the Zeit
statue sound 164 and effRemoveTrap), its entry used, `param[2]` 0; at frame
100 sound 56, then every other frame a dust ring until the clip ends. It
is drawn with `ccChar::Draw`.

**The symbol.** `ccEntryGimSymbol` (rows 17 and 18) makes a `ccGimSymbol`
(0x0045a0d0): `trapNum` (+0x7c) a skill of `symbolSkillTbl`
(`ccRand() & 15`), an omni light over it, and for row 17 (the story
dungeons', `XGSYMBOL.CCS`) a body, `CMP_xgsymbo0` playing `ANM_xgsymbol`
and 40 fires (`ccSymFire`, `EFF_x008`) run 20 frames ahead. Its `main`
(0x0045aa40) does nothing while frozen, then runs row 17's `symMain`
(0x0045ab00) or row 18's `objMain` (0x0045aed0, the lakes', no body or
fires):

```text
symMain  a symbol already used (act not 0) starts spent (act 2, its fires
         out). Acts 0, 1: the light at 1 + ccRandF(0.2), in the group
  act 0  a fire every other frame; affect 11 (SymbolMenu): act 1
  act 1  frame 0 off the command lists, param[2] 0, sound 35,
         effUseSymbol(light), ccItemSkillRequest(this, affectPerson,
         trapNum, 0); each frame a fire and invokeEff (a spark,
         ccParticleExplode); frame 30 act 2, the light out
  in view (ccCheckCameraDeg 12288): the clip forward, ccChar::Draw, the
         fires on layer 3
objMain  every frame the light as above; act 0 the count up, affect 11:
         act 1; act 1 as symMain's (deleteCmnd(1)); act 2 the object goes
         with its light; on an even count a puff: effSmoke from 10 out
         along a heading of three ccRand() turns, flying out along it and
         2.5 up
```

**The food.** `ccEntryGimFood` makes a `ccGimFood` (0x240 bytes): the
entry copied in, a body (the base's width, half its height, its type) in
the collision list, `CMP_trall` with `foodAnimTbl[row - 22]`
(`ANM_xgfood00a`-`0fa`), the scale (1, 1, 1). Its `main` (0x00458710)
turns it back to its heading and away from Kite beyond 1000, toward him
within (the first frame within, `ccVoicePgFood(food)`: its call, out of
battle); wobbles (scale z, a damped sine by `ccRand`) and rolls about y
(`actRolling`, 0x00458580) now and then, always near Kite (sound 164 with
each wobble); affect 11 (FoodMenu, 43) takes it: act 1 off the command
lists with `ccSeOn3DNote(179, pos, 70)`, act 2 20 frames on
(`effOpenBox`, sound 77, a 30-frame fade while it bounces), gone 30
frames later. It is drawn in view (`ccCheckCameraDeg(pos, 0x3000)`) at
`SetMatrix_PosRotZYXScale` with `ccChar::Draw`
([the field UI](field-ui.md#the-field-objects-32---39-43-67)).

**Where they come from.** `DUNGEON::SetItemBox` (0x005beb30) puts a box at
each kind-0 slot of `SetAllGim` (a golden egg, id 22, when `fieldrand(100)`
is under 10, the area's food under 30, else row 0), then, in a story
dungeon, each `GIMMICKDATA` row of type 0 (a box at (x, y, 250), rows 0, 2,
4 by kind, its item `ccGetItemEditCode` of the row's code, `param[0]` the
event entry counter) and type 3 (the table at 0x006965f0 by kind: 6, 1, 0,
0, 17, the food (`GetFood()` overwrites word 5), 19, ...; kinds 7-26 are
the warps; kinds 27 and up a virus crystal holding the item `(kind - 27) |
0xf0000`; kind 6 is the boss room's warning, below). A row faces by its
`direc` (0: 0, 1: pi, 2: pi/2, 3: -pi/2). `DUNGEON::SetIDOL`
(0x005be750) puts an idol at each kind-2 slot: 44 in a time-symbol
dungeon, 20 at a lake, else by the field's element 38-43, turned by the
room's `rotate` plus pi, and in a story dungeon the room's item.

**The spring and the boss room's warning.** `ccGimEtc` (gcmn gmetc.cpp,
0x00456450-0x00458218, 0x270 bytes) serves rows 19 (`WARNING`), 20 (the
lakes' Spring of Myst, `FOUNTAIN`, `XGWATER0.CCS`) and 21 (`ENTRANCE`,
the fields' dungeon entrance swirls, `WORLD::SetSpecialObj`). Its fields past `ccGimmick`: `effsw`
+0x1e0, `spiritFlag` and `spiritType` +0x1e4 (bits 0, 1), `scale` +0x1f0,
`vpos` +0x200, `radCnt` +0x210, `windowOfs` +0x214, `scale0` +0x220,
`bounceCnt` +0x230, `bounceDeg[4]` +0x234, `bounceOfs` +0x23c, `scale1`
+0x240, `springCnt`, `springDec`, `springZoom` +0x250, `lgtFlag` +0x25c,
`lgt[2]` +0x260, `gimRad` +0x268.

```text
ccGimEtc(ent)   (0x00456540) the entry copied in, posP, dirc, gimId, the
                row's base; act 0; effsw 1, the rest 0
  row 19        off the command lists; param[2] 0: the rays (param[3]
                their user); SetItemBox leaves param[2] -1
  row 20        initFountain (0x00456ba0): spirit off, every scale 1,
                vpos the place, bounceCnt 1, bounceDeg four ccRandS();
                area level under 4: CMP_trall1 playing ANM_xgwater1a,
                else (spiritType 1) CMP_trall2 and ANM_xgwater2a, blend
                4; two omni lights
  row 21        off the command lists
main            (0x00456760) nothing while frozen; the first frame with
                effsw 1: row 19 (param[2] not 0)
                effBossRoomEntrance(pos, &effsw) (generator row 122), 20
                effFountain(pos, &effsw) (rows 120, 121), 21
                effDungeonEntrance(pos, &effsw, 2); row 20 then
                ctrlFountain
```

`ctrlFountain` (0x00456fb0) keeps its state and count in `actNum` and
`actCnt`. "Affect 11" is `FountainMenu3`'s `EntryAffect(spring, plw, 11)`
([the field UI](field-ui.md#the-spring-40---42)).

```text
0   affect 11: 1
1   frame 0 sounds 85, 216, 217; 10 EntryFlash2(30, 30, white); 60: 2
2   frame 0: faded in over 10, turned away from Kite, 400 under the
    ground, a spring (6, over 90), the spirit on, sound 215, both
    lights (farDown 0 to 1500; blue, or red for spiritType 1:
    ccSetColor's low byte is red), lgtFlag down; 90: 3. Each frame the
    spirit rises to 150 + 230 sin(2 radCnt) and turns by 0.6
    sin(radCnt), radCnt on by 2 degrees
3   frame 0 the bounce on; a spring of 1-3 now and then (ccRand() & 7
    0) when none runs; down to 150, turned toward plDirc; affect 11: 4
4   frame 0 a spring of 7 (0.5, over 90); 30: 5
5   affect 11: 6
6   frame 0 a spring of 7, sound 104; frames 4-28 every fourth
    effRemoveTrap; to frame 30 a turn of 24 degrees a frame; 80: 7
7   affect 11: 8
8   as 4; 30: 9
9   affect 11: 10
10  frame 0 effsw 0, a spring of 3 (0.9, over 180); 30 faded out over
    60, a spring of 7, sound 215; 90 EntryFlash(10, white): 11. To
    frame 30 down to -200 (rate 0.02), then up to 1800 (0.08), the
    bounce off, scale0 to 0 (0.12); frames 6-59, when the count is 2 mod 4, effOpenBox
11  SetFountain(the area's code), off the command lists, the lights
    out: gone
then (states 0-10): under 10, every fourth count a dust ring (3.0,
    30.0, 4, 35, 110) ccRandF(100) out at a ccRandF(pi) heading, at z
    -50; windowOfs to 150 (rate 0.04) in states 3-10, else to 0 (0.01);
    the bounce (bounceDeg on by ccRandS() & 0xff, & 0x7f the fourth,
    plus 837, 1110, 564, 1110; scale0 1.2 + 0.22 cos, 1.1 + 0.23 cos,
    1 + 0.12 sin, z + 20 sin); the
    spring (scale1 x 1 + zoom cnt cos(pi/2 + 2 pi cnt), z 1 + zoom cnt
    sin(2 pi cnt), springCnt down by springDec); scale = scale0 * scale1;
    in view with the spirit on: drawn on layer 6 (refLayer, priority 30)
    with blend 4 (alphaBlendTbl[4] 0x09), the lights 30 under and 100
    over vpos, intensity (1 + sin(bounceDeg[3])) + 0.5 in double,
    farDown 1000 + 1000 (1 + sin)
```

`SetFountain(code | server << 29)` records the area in the save's 100
words at +0x65dc (count +0x676c). `entryObjectCheck` then makes no row 20
in an area `ccSaveData::CheckFountain(code)` (main 0x001782f0) finds
there, so a spring is used once. `ccCheckFountain()` (0x0042e0c0) is the
minimap's: whether any entry is of row 20.

The warning is kind 6 of a story dungeon's type-3 rows. `SetItemBox`
builds each row's room (`SetRoom`, then `DeleteRoom`; the roomless types
16-23 not), and for kind 6: `GetBanRoom` finding a banned room skips the
row without taking a counter number; else `GetNearDoorPosition` (gcmn
0x005c9ba0) moves the row's place to the nearest `OBJ_w_0g10_*` of the
room (with none, `OBJ_0pae0_*`) within 12,000, `param[0]` takes the
counter, and `entryObject` runs only if `CheckBossEffect()` (0x005b63a0)
holds. That is false in story areas 23, 25, 48, 71, 77 and 101. In areas
17, 18, 19, 21, 22, 44, 45, 50, 72, 74, 75, 76, 100 and 103-106 it holds
until bit 62 of the area's save word is set (the boss beaten; +0x5568,
+0x5580, +0x5588, +0x55a0, +0x55b8, +0x5838, +0x5840, +0x5880, +0x5b68,
+0x5b80, +0x5ba0, +0x5ba8, +0x5e68, +0x5eb0). In any other area it is
true. `GetNearDoorPosition` first copies `in` to `out`; with no marks it
returns 0 (the row's place stays), and with marks all farther than 12,000
it leaves `out` as its stack's words.

The port is `crates/piney-battle/src/gimetc.rs` (`etc_new`, `main`) and
`gimmick::check_boss_effect`; piney-world's `DungeonArea::near_door`
gives `SetItemBox` the places, and draws the spring (both clumps loaded,
blend 4) with its two lights. `tools/test_battle_spawn_rs.py`'s
`etc_frame` makes rows 19-21 by their own constructor, puts the spring in
a random state and count (some given affect 11) and compares 150 scenes'
frames of `ccGimEtc::main` with the port's: the words, the effects, the
matrix, the draws, the lights, the flashes, the dust rings and the save's
fountains. `tools/test_dungeon_rt.py`'s `test_near_door` builds rooms by
the game's `SetRoom` and gives each of the port's marks back as `in` (338
marks, every one returned), then random places near them (617 picks, and
493 beyond 12,000).

**Opening one.** The action button on a box (base type 0x4000) opens menu
32, on a trapped box (0x8000) 33, on an idol (0x100000) 38: they open it
with `EntryAffect(target, plw, 11)` and give its item ([the field
UI](field-ui.md#the-field-objects-32-33-38-67)). The Fortune Wire (13:0)
from TARGET disarms a trapped box (`EntryAffect(box, plw, 12)` after
"Disarmed trap.": [the field UI](field-ui.md#using-an-item-ccuseitemrequest)).
Event 4's tutorial disarms its own trapped box (`remove_trap`: "Orca used
the Fortune Wire!").

**The port.** `crates/piney-battle/src/gimmick.rs` (`box_new`,
`idol_new`, `food_new`, `main`, `set_item_box`, `set_idol`) and `prim.rs`
(the rays), from the entry control's seams (`EnemySeam::gimmick_main`);
piney-world draws the boxes, idols and foods with their rows' palettes
(`GimLook`; a food at its scaled matrix), the rays on the effect layer,
and carries the traps out after the entry frame with the command lists as
the call saw them. `tools/test_battle_spawn_rs.py`'s `item_box` compares
the foods `SetItemBox` makes (their constructor) and `gim_frame` their
frames (`ccGimFood::main` and `actRolling` run natively, taken or not,
Kite near and far) with the port's.

## Enemy AI

`ccEnemy::main` (0x00432cd0) runs an enemy once a frame: the drained form's
grace, removal, `routineEnemy` (0x00433990: `CalcReal(0)`, the life rate,
the timers), `checkEnemy` (0x00433710: the headings and distances home and
to the target), then the race's `think()`, which is `defaultThink`
(0x00434ac0) for every race, `interruptThink` (0x004353f0), the race's
`action()` (movement), and `hold` cleared. Acts (`actNum`): 0 wait, 1 close
in, 2 wander, 3 chase, 4 return home, 5 stop, 6 attack, 7 flinch, 8 dying,
9 dead.

`defaultThink` moves between acts by the row's think parameters: every
fourth frame of an act an enemy with a close range retargets; a waiting
enemy chases a target beyond `atkRangeC` after 30 frames, closes in within
`atkRangeA`, else attacks once its delay is over; closing in, it attacks
beyond `atkRangeB` after 16 frames or after 240; a chase stops within
`atkRangeB`; a wanderer stops 3 times in 4 every 256th frame.
`interruptThink` then drops a target beyond `viewRange`, sends the enemy
home beyond `area`, handles charm and confusion (`madFlag`) and holds, and
reacts to the affect just taken: a revival wakes a dying enemy, damage or
poison with no HP left starts dying, damage flinches (from level 31 only if
it has not flinched lately). Dying sets `dead` 2, then 3 after 90 + 30
frames, with `ccExpDistributor` and the enemy book's record
(`saveData.enemyKillCount` +0x68b7, `enemyKillArea` +0x69f0).

- **Targets** (`selectTarget`, 0x00436cb0): among the first six of the
  party within `viewRange`, the nearest (type 0), lowest HP (1) or lowest
  level (2); charmed, the nearest other foe; confused, the nearest of
  either side.
- **Attacks** (`selectAttack`, 0x00436940): nothing while paralysed or
  asleep; charmed or confused, slot 0; else a usable heal or revival first,
  else a usable slot at random by its share, `0.5 + ccRandF(0.5)` against
  the running sum. `checkSkillList` (0x004360d0) says which of the six slots
  are usable (SP, range, a target); `setSkillRate` (0x00435ef0) weights the
  shares: heals and revival 0, 175-177 x1.5, 156-162 x1.3.
- **Spell targets** (`selectSkillTarget`, 0x004366a0): a heal on the foe with
  the lowest life rate under half; a buff on the first foe without it; a
  debuff on the party member it would take on
  (`ccCheckConditionSkillSuccess`, which draws `rand()`) with the lowest
  tolerance.
- **Delay** (`setInterval`, 0x00436c40): the row's `attackDelay` plus 8
  frames for every active enemy; a new enemy waits 32-95 frames.
- **Skills.** Note 0x8003 of the attack animation is `startSkill`
  (0x00435ac0: a spell through `_ccSkillRequest`, a special attack's SP);
  note 0x8005 is `affectSkill` (0x00435b80: `ccSkillDamage` with the slot's
  row).
- **Spawn** (`initEnemy`, 0x00433260): the row's stats, full HP and SP, home
  at the spawn point. A middle boss (row type 0x40, its base form in
  `gold`) with a protect gauge is virus-flagged: it takes a tenth of the
  damage and never falls below half its HP (above). A drained enemy becomes
  its middle boss's base form, else the first row of its race group
  (`ccGetDrainId`, 0x0042e3c0).

**`defaultThink`** in full. Every fourth frame of an act (not an attack) an
enemy with a close range retargets the nearest, then its preferred kind if
the nearest is not close. Then by act:

```text
0 wait    paralysed or asleep: nothing; held: strike back (6) if it was just
          flinching, with no delay, a target within atkRangeC and a race row past
          the first; with a target: beyond atkRangeC chase (3) after 30 frames,
          within atkRangeA close in (1), else attack (6) once the delay is over;
          without: a target found means chase, else every 64th frame a coin toss
          wanders (2)
1 close   no target: stop (5); beyond atkRangeB after 16 frames: attack, or (a
          later race row with a delay, healthy, a coin toss) halve the delay and
          stop; after 240 frames (or 90 with 90 hit frames): attack; then with no
          delay (later rows, 16 frames): attack if one is ready (when held: if
          flinching lately)
2 wander  every 256th frame 3 in 4 stop; a target found means chase
3 chase   within atkRangeB or no target: stop; after 240 frames 1 in 8
4 return  home within territory: stop; after 240 frames, every 32nd, a coin toss
5 stop    below speed 1: wait
6 attack  a special attack holds its target in range; the animation's end (or
          240 frames): wait, and the delay restarts
7 flinch  counts; at the animation's end: wait if paralysed, asleep or held in a
          puppet show; with a close range: close in on a random kind of target
          (first rows, below half life, 1 in 4) else wait; else attack a target
          found once the delay is over
8, 9      count
```

**`interruptThink`** in full, after `think()` every frame:

- a target beyond `viewRange` (not mid-attack) is dropped: stop, wait;
- beyond `area` from home while waiting, closing, wandering or chasing:
  drop the target and return (4);
- charm or confusion starting sets `madFlag`; ending clears it and
  retargets (from Mutation on also waits); while mad it retargets every
  frame, and if it targets itself its delay runs down and it attacks (from
  closing or chasing);
- held, paralysed or asleep: speed 0; paralysed, asleep or held in a puppet
  show, and not attacking, flinching or dying: drop the target and wait;
- the affect just taken: a revival (20) of a dying enemy wakes it (`dead`
  0, wait); poison (3) or damage (1) with no HP left starts dying (8);
  damage of 1 or more flinches (7) unless attacking or flinching, and from
  level 31 only if not flinching lately;
- no HP: dying (8). Dying's first frame sets `dead` 2 and clears the
  conditions; after 90 frames it is dead (9), off the lists, the experience
  given, fading out over 30 frames, the book's record kept; dead for 30
  frames, `dead` 3, removed the next frame.

**`selectTarget(lttype)`** in full: with the whole party down (or none)
while waiting, no target, conditions cleared, `actCnt` 1 (unless riding).
Charmed: the nearest other foe; confused: the nearest of the party or the
foes (`ccSearchNearPerson`, types 0x60 and 0x63, within `viewRange`). Else
among the first six of the party in `viewRange` (on the ground from its
world position), living and not a character that cannot die while others
are there: type 0 the nearest, 1 the lowest HP, 2 the lowest level;
failing that the nearest player (`ccSearchNearPerson(me, 3, 0,
viewRange)`). A target found or kept is measured at once (`checkEnemy`).
Other types (the `rand() % 3` of a flinch can be -2 or -1) compare an
uninitialised register instead of HP or level; every call `main` makes
leaves it 0, which picks the first in list order.

## Party AI

A party member the game drives has a `ccAI` (`personal.cpp`, gcmn
0x0057c5f0-0x005977cc). `ccAI::Brains` (0x0057ca00) runs it once a frame from
`ccFellow::Main` (0x0041b5f0): the distance to Kite and the heading to the
target, the message queue (`ReadSysMsg`, 0x0058aeb0), `levelCheck`
(0x00587440), the player's command (`ChatCommand`), then by area
`ActInField` (0x0057cd00), `ActInDungeon` (0x0057e0c0) or `ActInTown`, and
the chat. A member down acts only as a ghost (`dead` 4 or 5, mode 6).

**Messages.** Members coordinate through `ccAISystem` (`ccAISys`, main
0x00378ca0) rather than directly: one that decides to heal, cure, revive,
buff or debuff someone posts a message delivered 30 frames later, and every
member checks the waiting messages and the recent history
(`CheckSolution`, 0x005959c0: 90 frames for a skill, 180 for an item) before
deciding the same thing twice. A message's name is a kind in the high half
and, for the healing kinds, the patient's AI id in the low half:

| kind | meaning |
| ---: | --- |
| 0x2, 0x8 | heal HP with a skill, an item |
| 0x3, 0x9 | cure with Rip Teyn (178), its item |
| 0x4, 0xa | cure with Rip Synk (179), its item |
| 0x5, 0xb | revive with Rip Maen (180), its item |
| 0x6, 0x7 | a buff or debuff skill, item |
| 0xc | a mode change |
| 0x10008 | damage about to be dealt (the estimate, the target) |
| 0x10009, 0x1000a | a level lost, gained |
| 0x10013 | going back to Kite |

**Duties.** `ActInField` and `ActInDungeon` carry out the player's command
(`ChatCommandExecute`, 0x0058ce10), then, unless asleep, held or paralysed,
the heal, debuff and buff duties, then by mode: 1 idle (look for a fight
with `Reconnoiter`, 0x00592aa0; follow Kite beyond 280, turning back from
3500 in a field), 2 wary, 3 fighting (`AttackTarget`, 0x00586630), 4 closing
in for a commanded skill, 6 a ghost. `CheckNeedHealing` (0x00589820) heals
below `maxHP * healRate / 100` of the member's `spcAIParam` row in battle
(the whole maxHP outside).

**Targets.** `Reconnoiter` picks by strategy: 0 and 3 the nearest foe
(`SearchTargetNear`, 0x00586bd0, within `cautionRange`), 1 Kite's target, 2 a
foe only while Kite fights, 4 the commanded target. A boss or 0x40 enemy is
fought at once; others beyond `cautionRange` are let go.

**Attacks.** `SelectAttackSkill` (0x005941f0) keeps the four best
candidates by `ccSkillDamageValue` (0x00594e90: `CalcBattleDamage` as a sure
hit, x2 for the opposing element, times a hit count per job and art bit; 0
for a target off the lists or down) among the member's attack spells (when
allowed magic), arts and attack items, within the SP it may spend (down to
`maxSP * savingSP / 400` once below `maxSP * savingSP / 100`). A candidate
displaces a kept one it matches unless that one does more damage per SP;
`mercy` holds back overkill. The best is taken, or 22% of the time
(`(rand() >> 3) % 100 < 22`) another rank. A zero-cost candidate divides by
zero in the damage-per-SP ratio (the EE's quotient), and the fifth rank is
never filled.

**Commands** (`RequestChatCmd`, 0x00583440): 7-10 set and pin strategies 0-3,
1-4 set them; 0, 13, 14 allow arts and spells, arts, spells; 15-18 heal,
debuff, buff duties; 11 attack the commanded target; 19 the Sprite Ocarina
(the party made unkillable, Kite's AI under manual control). A member
paralysed, asleep, confused or charmed only answers that it cannot.

### The decisions

Function by function; "from Mutation on" names what the later volumes
change (their addresses are Mutation's gcmn). A member's frame around
`Brains` is [a party member's frame](#a-party-members-frame).

**`Brains`** (0x0057ca00), not before a mode is set nor without a body:

```text
not Kite         distPl = distance to Kite (party slot 0); dircTg = heading to the target
talking          turn to gDeg and stop here
off the bus      enter it (SysMsgEntry); on it: ReadSysMsg
levelCheck
in the party     ChatCommand; not: skillMask 3
alive            out of mode 6 (to 1)
down (4, 5)      a ghost goes to mode 6 and acts; dead otherwise: no action
act              manual: ManualControl; else by area: dungeon ActInDungeon, field
                 ActInField, Root Town ActInTown (ActInField for ccGame +0x24 == 13)
ChatMessageSender, count + 1, detourCnt down to 0
```

It returns what the act returned (0 without one: the game leaves a stale
register there), -1 with no mode or body.

**`ReadSysMsg`** (0x0058aeb0) reads the whole queue, front first, and
carries each message out (none while the ocarina plays): 2 heals the
patient (the id in the name) with `SearchHealSkill`'s skill; 3, 4, 5 cure
(178, 179) or revive (180) the patient; 6 casts the buff or debuff skill
`param` on `pointer`, 7 uses the item `param` on `pointer`; 8-0xb use the
item `param` on the patient. A skill out of reach is asked again in 45
frames (kinds 2-6); a heal, cure or revive command (16), or out of battle a
buff command (17, 18, for kind 6), is done after trying. The broadcasts
answer calls (1, 4: come in mode 1 or 2 with 0x10002, else 0x10003), level
changes (9-11), a death (12), a ghost's lament (13, every 90 frames),
treasure and traps (15, 16) and meetings (17), and repeat the idle talk
(18-22: every 150, 300, 300-322, 500-522 frames); the ocarina's 6 and 7
make the member leave. Once a message the member sent itself is read, the
rest of the batch is read as its own too. It returns the last use's
result. From Mutation on (0x005b25f0) a use is skipped unless its patient
needs it: a revive one down (`dead` 2-4), anything else one alive or
reviving (0, 5), not at full HP for a heal (kinds 2, 8, a skill of type
0x40000), with a condition `CheckConditionMinus` finds (-1 for 178, -2 for
179). A skill use by id (2-5) the member did not send itself is announced,
the item uses say nothing (`UseItem` remarks), and only a debuff command
(17) is done after 6 and 7.

**`ActInField`, `ActInDungeon`** (0x0057cd00, 0x0057e0c0), one function
with the dungeon's differences: the command (`ChatCommandExecute`); nothing
more while asleep, held or paralysed; the heal, debuff and buff duties;
then by mode (it returns 0):

```text
1  idle: Reconnoiter (a strategy-3 healer keeps watch); follow Kite beyond 280 (in a
   dungeon also when out of sight or with a route to him), turning back from 3500 in a
   field; walking talk out of battle
2  wary: look again, keep to Kite (or face the target)
3  fighting: a healer (strategy 3) goes back to 2, strategy 2 falls back (1) when Kite
   attacks from beyond 280, strategy 1 lets its target go when Kite has another;
   AttackTarget while the target is up (in a field only within cautionRange, unless a
   boss or 0x40 enemy, strategy 1 or 4, or commands 5 and 11), else drop it
4  a skill command: in range (50 beyond the skill's triggerRange) UseSkill (a hit foe
   hears the damage estimate, mode 3), else close in
5  called over: step away from Kite within 150, look within 260, else stand down (1)
6  down: follow Kite beyond 280 (in a field only as a ghost)
```

From Mutation on mode 4 (`ActInField` 0x005a3810, `ActInDungeon`
0x005a5104) is a member holding its place (strategy 6): unless charmed or
confused, a party member of another strategy goes to fighting (3) and one
heading back falls in (1). Its target, while up, is attacked (with the
field's `cautionRange` rule of mode 3), else let go and the member halted;
a target that is the member itself is let go at once. With none it looks
around (`Reconnoiter`) and halts.

**`Reconnoiter`** (0x00592aa0) chooses the target and mode. A member out
of the party searches for any foe (`SearchTarget` 224); a party member none
while the ocarina plays, while heading back or while riding; otherwise by
strategy: 0 and 3 (and a charmed or confused member) the nearest foe, 1
Kite's target (a listed foe of neither side's own, up), 2 a foe only when
Kite is neither attacking nor standing still (`stopCnt` 15 up) in modes 1
and 3, 4 the commanded target (taken at once, mode 3, when no skill runs).
With a target and able to act: a charmed, confused or unallied member, or
strategy 1, goes for it (mode 3; strategy 1 says so); strategies 3 and 4
only aim; the others go for it within `territory` (mode 3, saying so), let
a foe that is not a boss or 0x40 enemy go beyond `cautionRange` in mode 2
(mode 1), and otherwise aim, going to mode 3 for a boss or 0x40 enemy and 2
for the rest. Without one the member stands down (1) unless in mode 1 or 5.
`distTg` is kept up to date; it returns 1 when the mode changed. From
Mutation on a charmed or confused member searches before anything else
holds it back, strategy 6 searches and only aims as 3 does, and a member
holding its place with no target stays so.

**`AttackTarget(tp)`** (0x00586630): a member neither confused nor charmed
lets go of a target of its own side (a charmed one of a foe), ending a
normal attack. One in the party (`partyFlag` 1), not already heading back,
in a field with no boss fight and more than 3500 from Kite turns back to
him: `goBackFlag` and `followSW` set, a 0x10013 note to itself in 40
frames, the target let go. Otherwise it closes in (`FollowTarget` once the
navigation has arrived, else `FollowBeacon`, arriving within 150 or, every
90th count, when the route's goal is at the target) and attacks: Kite
through `ccPlayer::Attack`, a member of type 4 through `ccFellow::Attack`,
with its strategy.

**`ccFellow::Attack(tp, n)`** (0x0041e590), while no skill runs: before its
first action, or every 180th frame of its task (`cycle`), a member neither
confused nor charmed, able to act and allowed arts or spells chooses
(`SelectAttackSkill`; none is the normal attack). It faces `tp` and
measures the distance; the normal attack starts within `armsRange` (the
AI's `distTg` for a distance of exactly -1, or within 30 of its stride
while moving under its own power) and a running one ends on a target at
`dead` 1; a skill starts within its `triggerRange`; an item is used
(`UseItem`). Every attempt counts in `atkTargetCnt`, and a skill or item
announces its estimate at once (0x10008, param the estimate, pointer the
target). It returns 1 done, -1 out of range, 0 unable.

**`SelectAttackSkill(tp)`** in full: with no affordable art (type 0), attack
spell (type 1) nor attack item, none. The candidates are its attack spells
(not 2-5) if allowed magic (`skillMask` 2), its arts (the normal attack
included) if allowed and not of job 5, then its attack spell items. A
spell or art costs no more than the SP it may spend (all of it, or down to
`maxSP * savingSP / 400` once below `maxSP * savingSP / 100`; items only
below that). A candidate displaces the first kept one it matches in damage
unless that one did more damage per SP, and unless (with `mercy`, before
its first action, not job 5) both overkill the target's HP left (its HP
less the damage announced against it in the last 5 frames); mercy also
holds back altogether when a normal attack would do a third of that. The
other rank is `(rand() >> 3) % 4 + 1` (rank 4 redrawn as 1-3; in a boss
fight rank 4 is rank 1, and an empty or zero-damage pick climbs to a better
rank). It returns 1 for a skill, 2 for an item (the id or code in
`SelectAttackSkillResult`), -1 for none. From Mutation on the 22% goes with
the level difference, and of two candidates that do no damage the cheaper
is kept.

**`UseItem(code, target)`** (0x00589410): not asleep, held, paralysed,
charmed or confused (from Mutation on free to act, `CheckAction(7)`, and
held only with a boss in, which returns -1; it then remarks on the item),
the member uses an item it carries (categories 10-15) on a listed target
not down past 1 (or down, for a Rip Maen item): `ccUseItemRequest(body,
target, code, 0)`, one used up, and a command to use it (99) done. It
returns 1 for an item of categories 11 and 12 and of category 10 other
than ids 18-22, 0 otherwise; failing, the command is dropped and the
target let go.

**`CheckHealParty(flag)`** (0x00587e20): the need is a bit per kind over
the three members: 1 someone down, 2 someone hurt (`CheckNeedHealing`), 4 a
body condition, 8 a spirit condition (`CheckConditionMinus` -1 and -2).
Each need the member's skills (180, 150-155, 178, 179, with the SP) or
items (Rip Maen items, healing items, 178 and 179 items; counts not
consulted) cover is cleared. It returns -2 when nothing is needed, 2 when
every need is covered, 1 when some are, -1 when none is but a skill lacked
only the SP, 0 otherwise. From Mutation on the argument is the rate for
`CheckNeedHealing` (its callers give 100.0), and a healing item's category
is looked for as 10 (Infection's code looks for 0, which no item has).

**`HealSPC(tp, rate)`** (0x00596b00) heals `tp` (itself with `selfFlag`;
with none, the party member alive or down, in the party, below `rate`% of
its maxHP, whom no one is healing, with the least HP) when below `rate`%
(`fptosi(rate * maxHP / 100)`). With a healing skill it plans it (0x20000)
or, out of battle and not keeping to itself, hands it half the time to a
member that heals better with a skill; with a healing item for the loss it
plans the item (0x80000) or, mostly (`rand() >> 3` not 0), hands it to such
a member. It returns 1 when planned. From Mutation on (a third argument,
`item_first`) the least-HP member wins ties (in battle the one with more
SP); the skill is handed only when it heals less than the loss, to a
member healing more than it does (an item's heal, 0x005c2380, likewise);
and with `item_first` the item is tried before the skill.

**`DebuffForUnusedEnemy(sid)`** (0x005937d0) and **`BuffForUnusedFellow`**
(0x00593ce0) plan `sid` on the nearest foe (member) without it: with the
skill and the SP the member posts itself a 0x60000 message (param the
skill, pointer the target), with an item casting it a 0x70000 one (param
the item), due in 30 frames, unless the same plan waits or went out within
90 (skill) or 180 (item) frames. They return 1 when planned (or with no
free slot). From Mutation on (a third argument, `check`): with `check` 2
the maxSP alone need cover the skill (0x00597cb0); an area skill (type bits
0x6000) planned by anyone within 90 frames (either form) is not planned
again; a target already planned gives way to the next unplanned one; with
`check` non-zero a target found is only reported (1); and the message is
due in 30 frames on the member's first turn (`firstTime`), else in 1.

**`ChatCommandHealPlz`** (0x005962d0): a member allowed to heal
(`skillMask` 4), able (not confused, asleep, charmed, held, paralysed or
dead), not in a skill and with nothing scheduled looks after the party (the
battle or the normal mode's check). Out of battle it then goes back to its
strategy (a strategy-3 healer to keeping to itself and healing only) and a
heal command (16) is done. From Mutation on a member that has acted looks
only every tenth frame, a held member heals too, the member must be free to
use a skill or an item (`CheckAction(6)` or `(7)`), and out of battle the
strategy comes back only once no one is below full HP (`CheckHealParty`).

**`ChatCommandDeBuffPlz`** (0x005857c0) and **`ChatCommandBuffPlz`**
(0x00585c70): a member allowed debuffs (8) or buffs (16), on its first
action or every 55th (90th) count, able, idle, with nothing scheduled and
with such skills or items, plans the first of its character's priority
list (`spcDebuffPriorityType[debuffTableIndex[id]]`, nine entries, or the
buff table's five) that some foe (member) still lacks: a skill id; -1 the
six stat skills of `debuffSkillAbilityTable`; -2 the element skill of the
field's element, then the other five. A debuff planned returns 1 at once;
out of battle the member then (buffs: in any case) goes back to its
strategy and a command 17 (18) is done. From Mutation on (0x005acaf0,
0x005ad010): every 35th count, a held member too, only when free to use a
skill or an item. A plan returns 1 at once. With none, in battle nothing
more (a buffer's next action is no longer its first); out of battle, only
once no one needs one (and, for buffs, no buff for the member waits or came
in the last 37 frames), the member goes back to its strategy, drops the
command and remarks that there is nothing to do (`OnlyDebuff`,
`OnlyBuff`).

**`RequestChatCmd(cmd, tp, sid)`** in full: the command takes effect on the
settings at once and is noted for `ChatCommand` (`chatCmdNew`, the skill,
the target, `chatCmdFlag` 6):

```text
7 8 9 10   strategy 0 1 2 3, commanded (strategyCMD) and applied
1 2 3 4    strategy 0 1 2 3, applied only
           (3 and 10 also set skillMask 4 and selfFlag; the rest clear skillMask)
0 13 14    skillMask 3, 1, 2 (arts and spells, arts, spells); a commanded strategy
           0, 2 or 3 reset to 0
15 16      skillMask 0; 4 (heal) clearing selfFlag
17 18      skillMask 8 (debuff); 16 (buff) clearing selfFlag
11         strategy 4, skillMask 3 for job 5
5 12 19    nothing more
6, 20 up   strategy 0, skillMask 0
```

Mutation's (0x005a97e0), past the condition check, is the same table with
a standing hold: `strategyCMD` 6 becomes 3 before most commands; the arts
and spells commands (0, 13) keep a standing 1 or 4 (applied) and otherwise
clear it; the spells command (14) makes a standing 3, or 0 or 2 with
strategy 3, into 6.

**`ChatCommandFulfilCheck`** (0x00583750): can the member do what it was
told? A dead member says so a quarter of the time (`ChatMessageGhost`), a
troubled one always (`ChatMessageConditionMinus`); then by command: 5 needs
the skill and its SP (else an item casting it, which turns the command
into 99); 13, 14 and 0 need a battle and arts or attack spells (or items)
aimed at foes; 16 a heal the party needs and the member can give
(`CheckHealParty`; nothing needed is told `Accept`); 18 buffs; 17 a battle
and debuffs; 19 a place the ocarina works, the ocarina and a free moment.
It returns 1 accepted (`firstTime` set), 0 refused (back to the standing
strategy, the command dropped).

**`ChatCommand`** (0x00583d60), once a frame: a member that cannot have
strategies keeps strategy 0. A command waiting for the member
(`chatCmdFlag` 2) starts once it can act (1). A new one (-2) waits while the
member heads back to Kite from beyond 1750, then is checked (the same
command again is taken as it is when the member is able) and accepted:
`chatCmd`, `chatCmdTime` 0, `chatCmdFlag` 2, `ChatMessageChatCmdAccept`,
the navigation reset, and for 11 `skillMask` 2 on job 5, for 3 and 9
outside the Root Town heading back. A command in force ends: 11 when its
target is gone or dead or Kite is more than 3500 away outside a dungeon; 5
once the skill runs on its target, or on confusion or charm; 3 and 9 after
900 frames in the Root Town or out of battle elsewhere; 1, 2, 4, 13, 14 and
17 out of battle (strategy 3 keeping its mask). The battle's start
(`spcBattleCondition` 3) resets the strategy to the party's (dropping
commands other than 16 and 18); during it (1) a strategy-3 member heals; at
the win (5) a member in battle mode says so (`ChatMessageVictory`) and
leaves it.

**`ChatCommandExecute`** carries out the command in force (`chatCmdFlag` 1
or 2) for a member in the party, not in mode 6, not troubled or held and
not heading back, a running normal attack ended first:

```text
11     attack the commanded target (mode 3), once no skill runs
2 8    let go of the target when Kite's target is a living foe
4 10   in battle, stop following and fall in (mode 1) unless in modes 1, 2, 5; stop moving
5      close in on the target for the skill (mode 4); a target gone ends it (mode 1)
99     use the item (UseItem)
19     play the Sprite Ocarina (not twice, not in an event, not with a menu open): the
       item used on itself, Kite's AI under manual control, messages 6 (next frame) and
       7 (in 50) to everyone, ocarinaUseFlag the member's id
```

It returns 0 when the command was carried out or does not apply (the
caller goes on with its own decisions), 1 while it holds the member.

## The chat lines

A member speaks through `ccAI::ChatMessage*` (`personal.cpp`, gcmn
0x00586130-0x00592a98): 57 functions, each choosing a line for its
situation from the 77 tables in GCMN.PRG's data (0x00651ae0-0x00654050:
`acceptMessages`, `fatalDamagedMessages`, `arriveDeltaMessages` and the
rest). A line is not shown where it is picked. The function writes it into
the AI (+0x24, 80 bytes) and sets `chatRequest` (byte 2 bit 0). The
member's next `ChatMessageSender` (0x00586420, the end of `Brains`) opens it
over the member with `ccChatMsg::OpenChat(body, text)` ([the
balloons](field-ui.md#the-chat-balloons-ccchatmsg-ccchat)) and clears the
request. A later line in the same frame overwrites an earlier one.

**The shared end.** Most functions follow one template:

```text
gate      partyFlag (ccSpcChar +0xe0 bits 14-16, signed) is 1; else nothing,
          no draw
row       MessageIndex() (0x0058dc40): the character id; 18 for id 1 while
          saveData +0x220d is set (id 1's lines, garbled)
19 table  one line a character
57 table  three a character, 3 row + v: v from (rand() >> 3) % 100, below
          60 the first, below 90 the second, else the third ("variant");
          or (rand() >> 3) % 3 where noted
say       unless manualSW (byte 0 bit 0) is set or the entry is null:
          ChatMessageModify(line, a1, a2, a3), chatRequest 1
```

The draws come before the manual check, so a member under manual control
still draws. `ChatMessage(line, a1, a2, a3)` (0x00586130) is the last step
alone; `ActInTown` and the following call it on the town's tables
(`byebyeMessages`, the `goto*Messages`) without the party check.

**`ChatMessageModify`** (0x00586190) copies the line with its `#` codes
filled in:

| code | text |
| --- | --- |
| `#0` | the leader's name (`getPartyMenberChar(0)->base->name`: the save's player name for Kite, `charTbl`'s for the others) |
| `#1` | a1, or the leader's name |
| `#a` | a2, or " " (`@3726`, 0x006f0210) |
| `#b` | a3, or " " |

The copy stops once 79 bytes are written. A name that crosses the 79th
byte is copied whole, but no table line comes near: the longest is 61
bytes before its names. Any other `#` code is never passed over. Its branch
leaves the read pointer where it is, so the loop spins for good. Two lines
have one: `fatalDamagedMessages[8]` (Orca's third) and
`charmConfusionMessages[12]` (Rachel's), each with a stand-in for a curse. The port stops the copy at the code. A name passed for no
character (`ch->base->name` of a null `ch`, which the game does not check)
is read through address 0. eemu holds zeros there, so the name is null
and `#1` gives the leader's; the port passes no name.

**The lines.** Each call is made where the game makes it. The decisions
call them through `Call::Chat` (`ReadSysMsg`'s broadcasts,
`ChatCommandFulfilCheck`, `ChatCommand`, `ChatCommandExecute`,
`Reconnoiter`, the town's movers), and the rules raise the rest as events.
"Chance" is the draw that must pass before the row is read:

| function | when | chance, draws | also |
| --- | --- | --- | --- |
| `ConditionMinus`, `CanNot`, `OOM`, `NoBattleModeDeny`, `DisableOcarinaDeny`, `NoOcarinaDeny` | an order refused (asleep and the like, nothing to do, no SP, not fighting, the Ocarina) | none | |
| `Accept` (0x0058dc80) | an order taken | `& 7` non-zero, then the variant | |
| `EquipNOT(v)` (0x00590d50), `EquipOK` (0x00590c90) | `ChangeEquipReport` (the CHAT menu's equipment order); `EquipNOT` also from an order's check | none; v 1 the skill table, else the other | |
| `ChatCmdAccept(cmd)` (0x0058fb40) | `RequestChatCmd`'s answer | 0, 13, 14 by the AI's `chatCmd`: 0 `% 3` of `useAllAttackSkill`, 13 and 14 `& 7` non-zero; 4 and 10 standby, 5 skill, 11 assign, 15 not-use-skill, 16-18 heal, debuff, buff (template); others as `Accept` | |
| `Victory` (0x0058eae0) | the fight won | `& 3` zero, then the variant | |
| `UseOcarina` (0x0058e750) | the Sprite Ocarina | none | |
| `HealStart`, `CureStart`, `ResurrectStart`, `Ghost` | a heal, cure or revive begun (`ReadSysMsg`); a ghost's refusal | `& 7` non-zero | |
| `WaitPlz` (0x005918f0), `QuitPucciguso` (0x00592070) | broadcasts 0x10013, 0x1000e | `& 1` non-zero | |
| `LevelDown`, `LevelUp`, `GhostCondition`, `TreatmentPlz`, `HealPlz` | broadcasts: 0x10009 and 0x1000a its own level; 0x1000d; 0x10012; 0x10016 below a third of maxHP, not a ghost | none | |
| `ResurrectPlz` (0x0058f600), `AttributeCritical` (0x00590ea0) | a hit on the downed member; a critical element hit | none | `ResurrectPlz` withdraws the member's 0x10016 |
| `GratsLevelUp(ch)` (0x00592750), `DeadOtherFellow(name)` (0x00591e00) | another member's level up (0x1000b), fall (0x1000c) | none; `#1` its name | Grats turns to face it |
| `PresentOtherFellow(ch)` (0x00591c10) | broadcast 0x10011 | `ch` on the command lists, then `& 1` non-zero; `#1` its name | turns to the leader |
| `OpenTrapBox` (0x00591ec0), `OpenTreasureBox` (0x00592210) | broadcasts 0x1000f, 0x10010 | none; the treasure `% 3` zero | turns to the leader |
| `WalkingTalk` (0x00592900) | broadcast 0x10014 | another member (below), `#1` its name, `% 3` of three a character | |
| `Neutral` (0x0058ec30) | broadcast 0x10015, out of a fight, not manual | below | below |
| `EnteredField` (0x0058e990), `EnteredTown` (0x0058f810), `Reencounter` (0x0058f6e0) | the arrival (below); meeting Kite again in a town | field `& 7` then the variant; town none, the server's table; reencounter the variant | |
| `AttackTarget(tp)` (0x0058d7b0) | `Reconnoiter` taking on a foe | below | returns the kind |
| `Attack(tp, dmg, sid)` (0x00590790) | the member's hit | below | `atkMsgCnt` |
| `Damage(dmg)` (0x005901d0) | a foe's hit on the member (`ccFellow::Influence`) | below | `dmgMsgCnt`, 0x10016 |
| `AffectMessages(kind, from, n)` (0x00586570) | an affect on the member | 7, 8, 16 `ThanksHeal(from, n)`: `& 7` non-zero; 17 `ThanksBuff`, 20 `ThanksResurrect`: none; 18 `ConditionModify(n)` | `#1` the giver's name |
| `ConditionModify(sid)` (0x00591380) | a condition from a skill | none: 156 and 162 poison and curse, 157 and 160 paralysis and sleep, 159 and 161 charm and confusion, no line for the rest | 0x10012 withdrawn and sent again to itself in 150 |
| `AttributeGuard(attr)` (0x00590f60), `AttributeFollow(attr)` (0x00591040) | from `Attack`, `AttackTarget` | none; `#a` `getAttributeStr(attr, 0)`, `#b` `(attr, 1)` (0x00597760: below 2 "!", 8 and over "?", else the element) | |

`Greeting(from, n)` (0x00583190) says nothing. It points the AI at the
greeter (`target`, `talkFlag`, `gDeg` = `RAD2DEG(atan2f(d.x, -d.y))`) and
stops the body (`moveFlag`, `runFlag`). The lines that turn the member
(`CheckAction(1)` passing) set its heading to `atan2f(d.x, -d.y)` toward
the one named, else the AI's target.

**The ones with more to them:**

- **`AttackTarget(tp)`** returns what it said. A `tp` that is no foe (`type
  & 0xe0` clear) gets nothing: 0. A foe that is neither boss nor 0x40 is
  looked at first. Its element (`CheckCharAttribute(tp,
  0)`) and `& 1` non-zero give `AttributeFollow`: 4. Else, with 6 or more foes
  within 1000 (`CountTargetInArea(0xe0, 1000)`), ids 3, 9-12 and 15 say
  `aLotOfEnemy`: 5. With fewer, ids 2, 5, 6, 9, 13, 14 and 17 say
  `conditionModifyEnemy` if the foe has a condition skill
  (`skillList[0..skillNum]`, `skiParam->type & 0x20000`): 6. Then (tp, or
  the AI's target) `% 3` zero says nothing; else the level difference
  picks `timid` (3 or more above: 1), `bull` (3 or more below: 2) or
  `dashForEnemy` (3), `% 3` of three.
- **`Attack(tp, dmg, sid)`**: nothing for dmg below 0. `atkMsgCnt` counts to
  3 and wraps to 0 before the party check. A skill (sid 2 and up) on a foe
  that did no damage, whose element (`skillAttribute(skiParam->type)`) is
  the foe's own, says `AttributeGuard` while the count is not negative, and
  sets the count to -3: no line for the next two hits. Otherwise
  only a count of 0 speaks. Damage over a third of the foe's maxHP needs
  `& 1` non-zero (`attackFatalHit`). Below 1% (at least 1) of a maxHP of 31
  or more gives `attackSmallHit`: a boss or 0x40 foe needs `& 3` zero, else
  `& 1` non-zero. The rest need `& 1` non-zero (`attackHit`). Then `% 3` of
  three.
- **`Damage(dmg)`**: `dmgMsgCnt` the same way. At 0, in the party, over a
  third of maxHP is `fatalDamaged`, below 1% (maxHP 31 or more) is
  `smallDamaged`, 10 and up is `damaged`, less is nothing; `& 1` non-zero,
  then `% 3`. Then, with `hp` the HP before the hit (`ccFellow::Influence`
  stores the new HP after the line): `hp - dmg` below a third of maxHP,
  without self-heal (`CheckSkillMask()` bit 4), files the "heal me" request.
  That is 0x10016 from and to itself, parameter -1, in `11 ((rand() >> 3)
  & 3)` frames, and none while a 0x1xxxx message to it waits. Else `hp / 2
  < hp - dmg`, or self-heal, withdraws it (`DeleteDelayMessage`).
  `ThanksHeal` withdraws it too, when `hp / 2 < n` or self-heal.
- **`Neutral()`**: first the same file-or-withdraw on the present HP. Then
  `& 7` zero says nothing. Rachel (id 12) and Natsume (id 11) take `& 1`
  non-zero for their own line. They pick another member, then Rachel
  compares weapon prices (`EquipParam` +0x40 of `equipment[4]`):
  `betterWeapon` for a dearer one, `worseWeaponMessages` for a cheaper one.
  Natsume notes a member two levels above her
  (`fellowIsHighLevelMessages`). Either line is `% 3` of three with `#1`
  the name, and the speaker turns to that member. Everyone else, or a tie,
  gives the variant of `neutralMessages`.
- **Another member** (`WalkingTalk`, `Neutral`): with fewer than three
  in the party, the leader. Otherwise `(rand() >> 3) % num` again while it
  is the speaker's own slot, and the leader after five draws.

**`ChatMessageSender`** once a frame: nothing once the party is wiped out
(`checkPartyAnnihilation`); the kept line opened; then `arrivalChatCnt`
(+0x80) below 0 stops there. At 0 and out of a fight, the arrival's line:
in a town `EnteredTown` (`ccGame` +0x1c, the server: Delta, Theta, Lambda,
Sigma, Omega's `arrive*Messages`); in a field reached from a town
(`areaPrev` +0x18 is 0) `EnteredField`; in a dungeon of field type 4
outside a story dungeon (+0x28 is 0), from a town, `EnteredField` too. Then
the count goes down one. `ccAI::ccAI` starts it at 150 in a town or in a
field reached from one, else at -1 (after a gate hack, for one).

**In the port** it is `piney_battle::party_chat`. `ChatTexts` reads the 77
tables and `getAttributeStr`'s from GCMN.PRG. `Ctx::chat_line` is the
functions, `Ctx::chat_sender` the sender and `Ctx::greeting` the greeting,
and the stage's runtime answers `Call::Chat` and `Call::ChatMessageSender`
with them. The sender's text goes out as `Show::Chat(body, text)`. The area
opens it in the member's balloon (handle `1 << 24 | id`), and the town
likewise through `take_party_chats`. The lines the rules raise are events
(`ChatAttack`, `ChatAttributeCritical`, `ChatDamage` and `AffectMessages`
with the HP before the hit, `ChatResurrectPlz`, `Greeting`), which
`combat::chat` runs on the member's AI. The game makes those calls inside
the hit, while the port runs them after the task that raised them: a
member's frame, Kite's, the entry control's pass (the enemies' hits), the
skills' pass, a box's trap, the boss. Those raised by menus and events wait
for the next frame's start. Their draws come from the same `rand()` a
little later in the frame than the game's. `levelOld` (+0xb6) is left by
the constructor as the heap had it (`_ccMalloc` does not clear). The
runtime starts it at the character's level, so an arrival outside act 14
says no level line.

**Checked** by `tools/test_battle_chat_rs.py` (eemu, `examples/battle_probe`
`chat`). It runs each of the 45 entry points (the 30 plain ones, the ones
with arguments, `AttackTarget`, `Attack`, `AffectMessages`, `Greeting`,
`ChatMessage` and `ChatMessageSender`) on 120 random party worlds each
(`bulk 500`: 500 each).
The worlds vary names, enemy skill lists, `saveData` +0x220d, `areaPrev`,
the server, the kept text, the counters and manual control. The tests
compare the text kept at +0x24, every AI's fields (flags, counters,
target, `gDeg`, the message queue), the bus, the bodies' flags and
headings, the balloon opened, and the `rand()` count. All match. A case
that picks one of the two lines with a stray `#` code runs out of its
instruction limit in the game and is drawn again.
`a_member_speaks_in_a_fight` (piney-game) has Orca speak in event 3's
portal fight, and `member_chat_shot` draws it.
`piros_says_his_arrival_line_in_mac_anu` has Piros say his
`arriveDeltaMessages` line 151 frames after arriving in Mac Anu.

A Grunty's line (`ccPGuso::main`) is not this machinery. It calls
`OpenChat` itself with its own texts.

## Items

`ccUseItemRequest(cp, tp, code, pn)` (0x0057aa80) is one blocking call: it
changes the characters, starts a skill, and in between draws the menu,
shows messages and waits. It never takes the item out of the inventory:
the item menu does that first (`DelItem`, in `ItemMenu` 0x0052db30 and
`TargetMenu` 0x00531af0), the party AI after (`ccAI::ConsumeItemList`,
0x00588860). An item code is `category << 16 | row`: 0-5 weapons by job,
6-9 armour, 10-15 the items.

| category | what using it does |
| ---: | --- |
| 10 recovery | on a listed target: rows 18-20 `EntryAffect(8)` of 100, 250, maxSP; 21-22 full HP, then 10 frames later full SP; 23 its skill with a heal of 800; the rest their skill on the target alone (stype 1) |
| 11 spells | the skill through the user (stype 2) |
| 12 books | a stat of the target's for good (`ccSpcParam.elm`, or maxHP/maxSP): +10 or +20 (capped at 999), +30 water, -10 magic attack, maxHP +30/+10 (HP rises too, up to 9999), maxSP +15/+5; the player's on the player names the stat in a message; on a foe the same offsets land in its `temp` and `time` blocks |
| 13 tools | row 0 disarms the targeted trap (`EntryAffect(12)`), row 1 leaves the dungeon (`dneFlag`, the party made unkillable, menu 86), row 2 shows the map |
| 14 trade | nothing |
| 15 important | epitaphs and notes (42-48, 68, 287-290) show their text (normal or Parody tables), the flute (49) calls a Grunty (`ccPuccigusoStart` 0x005109c0), 60, 61 and 69 warn, the Ryu books 273-280 open the book viewer (`ccThBook` 0x0041a990) |

`ccCheckItemUseful` (0x0057a6d0) tells the menu how an item is used: 0 never,
1 on a target chosen next, 2 at once on the user. `ccCheckSkillUseful`
(0x0057a890) greys out a skill with no target: party-aimed skills need a
member (Resurrect one who is down), others a target of the right type within
`triggerRange` plus its width, alive and in view, and the drain skills a foe
whose protect is broken (`PPcount` running) or a target that is no foe;
from Mutation on none at all while the bracelet is off
(`saveData.eventStatus[40]`). An id below 1 or past the skill table reads
past the table (the port gives no use). `ccItemSkillRequest` (0x00572790),
`ccItemSkillRequestParam` (0x005727d0) and `ccItemSkillCompel` (0x00572730)
start an item's skill (stype 1, or 2 with the flag), with its amount and the
compel bit. The save's lists: `AddItem` (main 0x00177730) stacks on the last
stack of the item or takes the first empty slot, caps at 99, then sorts by
`addItemCategoryTbl` (main 0x00307140); `DelItem` 0x00177af0 and the
player's own lists (`AddPlItem` 0x00177bc0, ...).

The party AI's use is `ccAI::UseItem` (0x00589410) once its checks pass,
and the ocarina command of `ChatCommandExecute` (the call at 0x0058d558:
the member uses 13:1 on itself); from Mutation on the member then remarks
on it. Around the rules the call also clears the enemies' conditions
(`ccClearConditionAllEnemy`, 0x0042e4f0), shows the trap's removal and
sets the player's AI mode and system messages.

## The motion layer

Where the rules stop, the game's code goes on moving and animating the
characters the rules act on: the entry control spawns the enemies and runs
their frames, the magic portals open, Kite's and the party members' acts
play their clips, the party AI follows and finds its way. The sections from
here to "Enemy movement and animation" describe that code; `piney-battle`
ports it with the rules and checks it the same way ([The port](#the-port)).

Everything it calls that is not logic - the collision, the camera, the
player's frame, the animation players - is the world's, asked at the point
the game asks it and as often:

| query | the game function |
| --- | --- |
| player's frame | `ccTransPosW2P` (0x0059b940, `ccPlayer::W2PPos` 0x0059b5a0), `ccTransPosP2W` (0x0059b980, `P2WPos` 0x0059b710) |
| ground | `ccLandHitCheck(pos, mask)` (0x00571e00), `checkHitResultAttlibute` (main 0x00153e80) |
| walls | `_ccHitCheckLM(from, to, mask, kind)` (main 0x00153930): `ccHitCheckLM` (0x001538d0) kind 0, `ccHitCheckLM2` (0x00153900) kind 1 |
| bodies | `ccCharHit::CollisionDetection` (main 0x00153470), `hitResultCharType` (main 0x0037892c), `HitEnable`/`HitDisable` (main 0x00153310/0x00153360) |
| camera | `ccCheckCameraDeg(pos, deg)` (main 0x001da710), `ccGetCameraTransparency(pos, w, h, far, len)` (main 0x001da880) |
| animation | `ccStream::GetChunkAdrsF` + `ccAnm::SetAnm` (main 0x00150f50), `ccAnm.frameNow` (+0x98), `ccAnm::_AnimateForward` (main 0x00152270), `ccAnm::NoteProcess` (main 0x00152210) |

A character's body in the collision is its `ccCharHit` (0x50 bytes: +0x00
`hitSW`, +0x04 `mask`, +0x08 `mask2`, +0x0c `type`, +0x10 `next`, +0x14
`radius`, +0x18 `height`, +0x20 `pos`, +0x30 `offset`, +0x40 `attribute`;
the constructor, main 0x00153220, sets both masks to -1 and the radius to
1). An enemy's is at `ccEntryObj` +0x160, a party member's `bodyHit` at
+0x1a0; the list of bodies is `ccCharHitTop`/`ccCharHitTail` (main
0x0037890c, 0x00378910). A note an
animation passes is a `ccAnmNote` (+0x4 `event`, +0x8 `param`), from the
clip's `F_Note` records; `NoteProcess` hands each to the character's note
function (`ccEnemyCheckNote`, `ccFellowCheckNote`, `ccPlayerCheckNote`).

**Affects land at once.** `ccChar::EntryAffect` runs the target's
`affectFunc` before it returns, and the code after it reads what it left:
a poison tick that downs a member inside its own `CalcReal` makes it fall
(act 9) in the same frame's `Action`; an enemy's hit on note 0x8005 hurts
the party member before the next target's roll. The motion code applies
each `EntryAffect` where the game calls it (`affect::entry_affect`), with
what the affect functions call beyond the characters (a running normal
attack called off, the body's switch, the pad's rumble, the AI bus's
"down" and "up" messages, an enemy's `selectTarget`), and hands the rest
(numbers, hit marks, the AI's lines) on in order.

### A frame of the field

The field's tasks run in priority order each frame (`ccSetupGameCtrl`,
main 0x00168960, starts them):

| prio | task | what it runs |
| ---: | --- | --- |
| 32 | `ccThEvent` | the event scripts |
| 33 | `ccThGameCtrl` (0x00517800) | targets and the menu buttons (`ccPlayerMenuCheck` 0x0059cd70, `CheckControlMode` 0x0059f550) |
| 34 | `ccThMenu` (0x005280d0) | the menus: items, skills, Data Drain |
| 40 | `ccThCamera` (main 0x00160610) | the camera |
| 48 | `ccThSpc` (0x005a0530) | the party's bookkeeping (below) |
| 48 | `ccThAISystem` (0x0058c850) | the AI bus's clock and deliveries |
| 49 | `ccThPlayer` (0x005977d0) | `ccPlayer::Main` (0x00598310): Kite |
| 50 | `ccThFellowNN` (0x0041eaf0 ...) | `ccFellow::Main` (0x0041b5f0): each party member |
| 64 | `ccThEntryCtrl` (0x00431970) | enemies, magic circles, gimmicks, NPCs |
| 80 | `ccThEffect` (main 0x001c2e60) | effects |
| 82 | `ccThSkill` (0x005722e0) | every running skill (`ccSkill::Main`) |
| 96 | `ccThFieldDisp` (main 0x001a4430) | the field or the dungeon room |
| 98 | `ccThParticle` (main 0x001bc2f0) | particles |

`ccThSpc` and `ccThAISystem` share priority 48; `ccThSpc` starts the other
from its own set-up, so it runs first (tasks of one priority taken to run
in the order they were started, as in [the town](field-game.md)). `ccThSpc`'s set-up also turns the
condition effects on (`ccSpcConditionEffectON`, 0x005a0880) and builds the
dungeon room's path-finding map (`SetPathFindingMap`, 0x00515aa0), which
runs again whenever a room is built. When an area starts, the entry
control registers the area's enemy rows (`ccRegisterEnemyList`,
0x0042ed70) and places the field's or dungeon's magic circles
(`WORLD_MAN::EntryGimmick`, main 0x001a1f20) and the event's entries
(`ccEntryEventMng`, main 0x001b62e0), or restores what it kept
(`restoreEntry`, 0x00431070).

### The party's bookkeeping

`ccThSpc` (0x005a0530), once a frame: `ccSpcCheckLevelUp` (0x005a1630, the
members away from the party), `ccSpcSetOperation` (0x005a1930: the party's
strategy `partyStrategy`, main 0x00378ce0, from the save's operation at
+0x6773: 8 is 1, 9 is 2, 10 is 3, anything else 0), `ccSpcShoutOperationName`
(0x005a17e0: Kite's chat balloon naming the strategy, text only, not a
voice line), then the battle condition `spcBattleCondition` (main
0x00378d00) the party AI reads, from `ccGame.inBattle` (+0x58) and its value
a frame ago (`spcCheckInBattleOld`, main 0x00378cfc):

| `inBattle` | condition |
| ---: | --- |
| 0 | 5 if it was 2, else 0 |
| 1 | 1 if it was 0, else 2 |
| 2 | 3 if it was 1, else 4 |
| other | unchanged |

**The shout.** In a field or dungeon with two or more in the party, a fight
(`inBattle` 1) counts `spcOpnCnt` (main 0x003782c0, 60 in the executable)
down to 0, one a frame, and when it passes 20 Kite opens a chat
(`ccChatMsg::OpenChat`) of three pieces: `ccKanjiStrSeparate(chatActionStr,
5)`, `ccKanjiStrSeparate(chatMenuStr, operation + 1)`,
`ccKanjiStrSeparate(chatActionStr, 6)`: a call to the party and the
operation's name. An event's scene (`eventMng` +0x78c) stops the count; `inBattle` 0
or 2, the Root Town or Kite alone put it back to 60 (measured).

## Kite

`ccPlayer` (`player.cpp`, 0x005977d0-0x0059ce00) is Kite. `ccPlayer::Main` (0x00598310) runs him once a frame on `ccThPlayer` (priority 49): after the camera (40) and the AI bus (48), before the party members (50):

```text
his AI onto the bus (ccAI::SysMsgEntry); a weapon change (EquipWeapon, DeleteWeaponCCS)
CalcReal(dead): ConditionTimeCount's poison, curse and regeneration reach him through
    EntryAffect and Influence at once; ResultOfConditions (0x0059c270); CheckLevelUp
    (effLevelUp); ccAI::levelCheck
movePos = (0, 0, 0, 1)
acts 23, 24: GateHackingOut (0x0059bc00), then the camera
on a moving floor: carried (diskOffset, W2PPos, P2WPos)
dead 2: cnt counts down from Influence's 90; once below 0: act 10, dead 3, effOpenBox twice
the party wiped out: still
manual mode, charmed or confused: ccAI::Brains drives him (held, asleep or
    paralysed: moveFlag 0); speed 140/140 x tsp walking, 100/100 x speed running
else ControlMove (0x00598af0) when alive, a ghost or reviving and no cutscene;
    the AI's targetFlag cleared, its messages dropped (ReadSysMsg2 0x0058be10)
HitCheck; stressMeter +2 while pressed against a wall, else -8; at 101 message 0x10004
pos += movePos; CollisionTest (0x0059b340); hitAttribute; MapLoopAdjustPos (0x0059b3c0)
CameraPosCalc, CameraPosSet; AnimCtrl (0x005993c0)
the matrix; plw (attack, pauseSW); transparency (0 when the eye view hides him)
the draw and the weapon's trails; diskOffset; hold cleared; stopCnt; cycle
```

A poison tick that leaves him at 0 HP inside `CalcReal` downs him there (`Influence`: act 9, `dead` 2, conditions cleared, SP 0, message 0x1000c). The same frame's `ControlMove` is skipped and `AnimCtrl` plays the fall. The rest of `ConditionTimeCount` sees the cleared conditions: the curse no longer blocks natural SP, and `CheckSpRegeneSpeed` is asked after his skill and `moveFlag` were cleared.

**Conditions.**
- Asleep, paralysed, held, or down (`dead` 2, 3): `inactiveSW` (`ResultOfConditions`), and the stick's power is 0.
- Held, asleep or paralysed: `ControlMove` stops him (`moveFlag` 0).
- Charmed or confused: the party AI drives him through `ccAI::Brains`, and a normal attack on the wrong side ends in `AnimCtrl`. Charmed, only a party member is a target; confused, anything with side bits.
- Asleep or paralysed: the combo stops.
- `hold` lasts one frame: `Main` clears it.

**The stick** (`ControlMove`, `ctrlType` 0): nothing moves him while a skill runs, during the first swing (`attack` 1), or under `pauseSW` / `restraintSW`. Leaning during the second swing ends it. `ctrlType` 1 (steering: the stick's x turns him by `5 (|sin| - 0.4) power` a frame past 0.4, its y drives him at `|power cos| / 255` in double precision, `pos` moved at once as well as `movePos`) and other types (backing along the stick) are ported, but only the constructor writes `ctrlType`, and it writes 0 (*inferred*).

**Acts** (`actNum`, each a clip of `playerAnimTbl` 0x006f0560):

| act | clip | what |
| ---: | --- | --- |
| 0, 1 | nut0, nut1 | standing in a field or dungeon, its fidget (and standing under manual control) |
| 2, 3, 4 | nut2, nut3, nut4 | standing in a town, a ghost, getting up; the fidget and its loop |
| 5, 6 | run0, wal0 | running, walking |
| 7, 8 | dmg0, dmg1 | hurt (`Influence`) |
| 9, 10 | dwn0 | falling; lying, fading out over 50 frames into a ghost (`dead` 4, act 2) |
| 12, 14 | nut2 | leaving through a gate: effTransfer and the body off the collision at frame 1, fading out from 22 to 41, then 14 at 141 |
| 13 | nut2 | arriving: transparent until 50 (20 by warp), effTransfer at 30 (0), fading in to 70 (40) |
| 15, 16 | atc0, atc1 | the normal attack's swings |
| 17-21 | mag0, mag1, ski2, ski3, ski4 | a skill by type bit 0x100, 0x200, 0x400, 0x800, 0x1000 |
| 24, 23 | hac4b, nut2 | a skill with none of those bits |
| 25 | atc0 | breaking a box or trap open (`BreakSomething` 0x0059cbd0; `AttackCancel` 0x0059cad0 ends it) |

- **The fidget:** after 451 frames standing (not in the eye view, manual mode or battle, act below 4, no condition), `reactCnt` becomes `rand() % 60`. When a clip ends, the jump table at 0x006f07e0 picks the next act.
- **A ghost:** fades to 0.5 over 30 frames, then `ghostFlag` is set.
- **A revival** (`dead` 5): transparent for 18 frames, fades in until frame 78, then `dead` 0.
- **Clip speed:** walking and running play at `256 x 1.1` and `256 x 1.375` times `speedRate x speedValue`.

**Skills and the combo.** `skillStatus` bit 0 (a request) starts the act and sets bit 1. For a target that is not listed the skill ends (`ccSkillRequest(this, 0, 0)`), except for skill 180.
- **Skill 1:** swings 15, or 16 while `attack` is 1.
- **Starting a skill:** faces the target (`SetTargetDirc` 0x0059aa90: `atan2f(dx, -dy)`, not in the eye view; `SetTargetDist` 0x0059ab80 keeps `distTg`, the ground distance less the target's width, and drops a target that is down), colours the trails, and, for an art or attack spell when not in manual mode, charmed or confused, sends 0x10008 to his own AI id with `ccSkillDamageValue`.
- **Ending the normal attack:** it ends when its swing ends or he stands (act 0), and `actOneTwoCnt` then allows 30 frames for a second press to swing 16; when the count runs out, `attack` goes back to 0.
- **The third swing:** `attack` 3 moves to 4 after 50 frames, and 4 swings again within `armsRange`, but no gcmn code stores 3 (*inferred*).
- **The hit:** note 0x8005 of his swing lands the normal attack's or art's hit (`ccPlayer::CheckNote` 0x0059c300, through `ccPlayerCheckNote` 0x0059c550), and its `EntryAffect` on the target runs the target's `affectFunc` at once.
- **The rumble:** `DamageActuate(dmg)` (0x0059ca40) rumbles the pad `DamActuTbl[0]` under 10 damage, `[1]` under 100, else `[2]` (nothing in act 14 or for no damage). `Influence` calls it before it sets the hurt or down act, so it sees Kite's act from before the hit.

**Kite's AI attack** (`ccPlayer::Attack(tp, n)` 0x0059c580, asked for while his AI drives him): it is `ccFellow::Attack` without `firstTime`. While no skill runs, every 180th frame of his task (`cycle % 180`), neither confused nor charmed, able to act and allowed arts or spells, he chooses (`ccAI::SelectAttackSkill`; none is the normal attack). `tp` becomes his target (`targetChar` set on the character). The normal attack starts within `armsRange` (the AI's `distTg` for a distance of exactly -1, or within 30 of his stride while moving), and a running one ends on a target at `dead` 1; a skill starts within its `triggerRange`; an item is used (`ccAI::UseItem`). Every attempt counts in the AI's `atkTargetCnt`, and a skill or item announces its damage to the party (0x10008). It returns 1 done, -1 out of range, 0 unable.

**Frames.**
- `W2MPos` (0x0059b470) wraps a world position into the map's bounds (`WORLD_MAN` +0x420: minX minY maxX maxY).
- `W2PPos` (0x0059b5a0) is the position less Kite's, taken the short way round (within half the map), with the world's z and w 1.
- `P2WPos` (0x0059b710) adds his position back, wrapping from the half of the map he stands in. The result can lie past an edge, which `W2MPos` wraps later.
- The `ccTransPos*` functions (0x0059b900-0x0059b9c0) call these with `plw+0x20`. `FW2LW` is `P2W(W2P(p))`.

**The port** (`kite`): the functions above. `KiteWorld` stands for the collision, the camera, the clip player, the draw, `ccSkillCheck` and the gate cutscene; `Out` carries effects, sounds, trails, `WORLD_MAN` updates and the rules' events, in order.
- Every `EntryAffect` of Kite's code is applied where the game makes it, with the character's `affectFunc` (`affect::entry_affect`).
- Of what an affect leads to, the stateful calls are made there too: ending a running normal attack, his body's switch, the pad's rumble, the down and up messages on the AI bus, a member's `talkFlag`.
- An enemy's retarget and the AI's lines are handed out.
- Kite's state is split across his `Char`, his `Spc` and a `kite::Player`; after an affect, each touched character's `Spc` copies are refreshed from its `Char` for the party AI. The `ccSpcChar` members the movers share with a party member (`atkAnmCnt`, `actCnt`, `reactCnt`, `transferLag`, `bodyHit`, `hitAttribute`, `transparency`, `setTransparency`) are his `Spc`'s; `kite::Player` keeps `ccPlayer`'s own.
- `kite::Host` is a `KiteWorld` over any world that is also a `NaviWorld`: each call borrows the world (shared through a `RefCell`) for itself, and its runtime is `party_motion::Movement`, so when his AI drives him (charmed, confused, under remote control) the following, `ccPlayer::Attack` and `ManualControl` are the ported code; the rest goes to the world's own runtime.
- From `ccPlayer::Main`, `ccAI::Brains` reaches `FollowTarget`, `FollowTargetDirc`, `ccPlayer::Attack` (the approach only), `ManualControl`, and `FollowPlayer` only as a charmed or confused ghost (mode 6); `LeavePlayer` never (measured by instruction trace).

## A party member's frame

`ccFellow::Main` (0x0041b5f0; `ccFellow` is 0x0041b5f0-0x0041e46c) runs a
member the AI drives once a frame, in this order. It sets acts through
`ccSpcChar::SetActNum` and `SetActNumOld` (0x0059d630, 0x0059d640).

1. The weapon swap: `weaponChangeSW` (+0xe1 bits 0-1) 1 loads the weapon
   (`ccSpcChar::EquipWeapon`, 0x0059d650) and leaves -1, which the next frame
   turns into `DeleteWeaponCCS` (0x0059dda0) and 0.
2. `CalcReal` with the member's `dead` condition, or -1 for character id 8
   under manual control with `noDeathFlag` while the event lock
   (`eventMng` +0x78c) is 1. `CheckSpRegeneSpeed` (0x0059f380) is "standing
   still with no skill" at this point. The timers' affects
   (`ConditionTimeCount`: 9 regeneration, 3 poison, 10 SP, 4 curse) land
   where they are made, so a member poisoned down here is down for the rest
   of the frame (see [Affects in the frame](#affects-in-the-frame)).
3. `CheckLevelUp`; a level gained shows `effLevelUp` unless the member is
   away (act 14).
4. `posP = W2P(pos)`, `pos = P2W(posP)`; `movePos` (+0x230) is cleared. Out of
   manual control, while `WORLD_MAN::GetTransMode()` (main 0x001a3ad0) is
   set, the member rides the carrier: `posP = W2P(GetTransCenter()) +
   diskOffset` (+0x220).
5. A member down (`dead` 2) counts `cnt` below 0, then lies still: `dead` 3,
   act 10, `anmFlag` 1, two `effOpenBox` at its feet.
6. Recall: `recallFlag` (+0xe1 bit 4) with `partyFlag` below 0 in act 14
   joins the party (`partyFlag` 1, act 13, the AI's `inviteFlag`, `actType`
   95, `arrivalChatCnt` -1). Exit: `partyFlag` -2 in act 14 leaves the bus
   (`ccAI::SysMsgWithdrawal`), the command lists (`ccDeleteCmnd`, 0x00519700)
   and the collision, sets `exitFlag` (+0xe1 bit 2) and ends the frame.
7. `ccAI::Brains` unless `ghoFlag` (main 0x00378cc0, which
   `ccCheckGtHackAnm` reads): set while a gate-hacked arrival's cutscene
   runs ([field walk](field-walk.md#the-hacked-arrival)).
8. `Move` (0x0041bc50), unless held, asleep or paralysed (then `moveFlag`
   and `nowSpeed` 0): `movePos = speedRate x speedValue x (3.0, or speed
   running) x (sin h, -cos h)` for heading `h = dirc.z`, only while
   `moveFlag`; `nowSpeed` the same length. Walking into it with a normal
   attack running (`skillStatus` bit 1) calls the attack off
   (`ccSkillRequest(this, 0, 0)`). Nothing at `dead` 2 or 3.
9. `ccSpcChar::HitCheck(movePos)` (0x0059ee20), below.
10. `pos.xy += movePos.xy`, `pos.z = ccLandHitCheck(pos, 0x20000001)`,
    `hitAttribute` from `checkHitResultAttlibute`, then W2P and P2W again.
11. `Action` (0x0041c670), below.
12. `ccAnm::NoteProcess` hands each note the animation passed to
    `ccFellowCheckNote` (0x0041e440) and `ccFellow::CheckNote` (0x0041e1e0):
    notes 1 and 2 are footsteps (`ccSeSetParamSPC` while shown, running dust
    `ccEffPawSmoke` in act 5); 0x8005 lands the normal attack (see
    [The normal attack](#the-normal-attack)), its `EntryAffect(1)` on the
    target applied at once; the others do nothing.
13. The body's matrix, `transparency = setTransparency = cloak`, and while
    shown (`dispSW`, +0xe0 bit 1) the draw (`ccChar::Draw`, 0x0056b1c0): the
    weapon's trail (`ArmsEffect`, 0x0059ddd0) runs while drawn, alive (or a
    ghost at `dead` 4) and not in acts 12-14; otherwise `ClearArmsEffect`.
14. `dispWait` (+0x204) counts down to showing the member and putting it on
    the lists (`ccEntryCmnd`, 0x00519630); the carrier offset is kept (`pos -
    GetTransCenter()`); `hold` is cleared; `stopCnt` counts frames standing
    (to 32767); `cycle` counts frames.

### Affects in the frame

Every `ccChar::EntryAffect` a member's frame makes runs the target's
`affectFunc` then and there (`ccFellow::Influence` 0x0041bdb0 for a member,
`Influence` 0x0059ac50 for Kite, `ccEnemyInfluence` 0x00432840 /
`affectEnemy` for a foe), and what follows in the frame reads the result
(*measured*: the frame runs in the checks with the real affect functions,
only their presentation stubbed). Two places make them: `CalcReal`'s
`ConditionTimeCount` on the member itself, and the normal attack's note
(0x8005) on its target, one per note, so a second hit in a frame sees the
first. A poison tick that downs the member (`dead` 2, act 9, conditions
cleared, off the collision, the "down" message 0x1000c on the bus) leaves
the SP ticks after it, `Brains`, `Move` and `Action` a downed member. What
the affect functions call beyond the characters: `ccSkillRequest(ch, 0, 0)`
(a hit, or a tick that downs, on a character whose `ccSkillCheck` is
skill 1, the normal attack: the attack is called off),
`ccCharHit::HitDisable`/`HitEnable`, the bus's 0x1000c sent and 0x1000d
withdrawn (`ccAI::SysMsgDown`/`SysMsgUp`), the AI's `talkFlag` cleared;
they read whether `ccMenu` exists (the panel shakes) and `ccSkillCheck`.
The rest is presentation (numbers, hit marks, `SetPanelBure`, the pad's
rumble; the AI's lines are [the chat lines](#the-chat-lines)), and so is a
foe's `ccEnemy::selectTarget` from `ccEnemyInfluence`, which the port hands
to the enemy's own frame.

**Collision** (`ccSpcChar::HitCheck`): the body (`bodyHit`, +0x1a0, radius
`width + nowSpeed`) stands where the move lands on the ground. In a field,
out of the event area (`WORLD_MAN::CheckEventArea`, main 0x001a2180), a
member beyond 7000 of the player on the ground is not checked. A member away
(act 14) or a ghost leaves the collision list. Under manual control the body
ignores the party's bodies (mask -8). A push (`ccCharHit::CollisionDetection`,
main 0x00153470) is added to the move and the body tried again; pushed twice
it goes halfway between. A wall (`ccHitCheckLM2`, mask 0x40000001) between the
feet raised by the body's height and where it lands, or where the move takes
it, cancels the move. Returns 1 when a party member's body (kind 2) was
touched, plus 2 when a push left less than 1 (8 running) of move on each
axis. The town version in `piney-world` (`hit.rs`) is the same without the
field's distance test and the list exit.

**Acts** (`ccFellow::Action`): the clip of act `n` is `motionTbl[n]`
(+0x240, the member's `fellowAnimTbl`, gcmn 0x005d4050, one per
`fellowNN.cpp`): 0 standing in battle (`nut0`), 1 its fidget, 2 at ease
(`nut2`), 3 and 4 its fidget, 5 running, 6 walking, 7 and 8 hurt, 9 down, 10
lying, 12 transferring out, 13 in, 14 away, 15 and 16 the two swings of the
normal attack, 17-21 spells and arts. Each frame, in order:

- A normal attack aimed at what it should not hit is called off: a
  non-foe while in its right mind, a non-party member while charmed, a
  non-character while confused.
- A requested skill (`skillStatus` bit 0) starts, unless its target is off
  the lists or down (then called off; Resurrect, 180, excepted): the normal
  attack in act 15 when the act is below 7, a spell or art in act 17-21 by
  its type bit (0x100 ... 0x1000; none: it waits). `skillStatus` becomes
  running (bit 1), the member stops (`moveFlag` 0, `stopFlag` 1, `runFlag`
  0) and faces the target (`dirc.z = atan2f(dx, -dy)`; under manual control
  `ccAI::SetDircZ` too), the weapon glows (`SetArmsEffectColor`,
  `effSkillStart` and `StartArmsEffect` for a spell), and the damage
  estimate goes to the party at once (0x10008, `ccSkillDamageValue`; for a
  spell only an attack or art, and not charmed, confused or under manual
  control).
- The normal attack's combo, while running and neither asleep nor
  paralysed, on `attack` (+0xfe): 1 and 2, at the end of the act-15 swing,
  swing again (act 16, `attack` 3) if the target is up and within
  `armsRange` (`ccSpcChar::DistanceToTarget`, the AI's `distTg` for exactly
  -1.0), else call it off; a swing from act below 7 otherwise only moves on
  to 3. 3 waits `consecutiveCnt` frames (`atkAnmCnt`, counted each frame of
  the attack) in an act below 5, then 4. 4 starts the next combo (act 15,
  `attack` 2) under the same test. Each swing announces its damage (0x10008).
- A skill over (`skillStatus` 0): the normal attack's `skillID` goes to 0.
- The idle fidget: free, alive, in act 0-3 and not in battle, every
  `reactCnt` past 450 (restarting at `rand() % 60`) act 0 turns to 1 and 2 to
  3, and a member not a ghost away from the Root Town queues 0x10015 to
  itself in 0, 11 or 22 frames (`(rand() >> 3) % 3 x 11`).
- The act's clip over (`anmFlag`): a swing or spell (15-21) goes to running
  or standing as the flags say, with the next attack's delay `atkDellay =
  ((rand() >> 3) & 31) + 70`; 1 -> 2, 3 -> 4; 12 -> 14 (away); 13 -> 2 (back
  on the lists, into the collision when alive); 4, 6-8, 14 and anything past
  21 -> 0.
- Down: at `dead` 3 or 4 lying (act 10) the member fades over 50 frames,
  then stands as a ghost (`dead` 4, act 2; the whole party down hides it);
  a ghost standing fades in to half over 30 frames (`ghostFlag`); `dead` 5
  gets up over 78 frames (`dead` 0).
- Walking: `moveFlag` with `stopFlag` walks (6) from an act below 5; neither
  stands (0; 2 in the Root Town, under manual control or as a ghost) from 5
  or 6; moving, `runFlag` picks 5 or 6. `walkRunCnt` counts frames moving.
- A new act sets its clip (`ccAnm::SetAnm`); the clip steps 256/256 frames a
  frame, `256 x speedRate x speedValue` walking and 1.375 times that
  running (`fptoui`), and `anmFlag` is whether a play-once clip ended.
- The transfers fade the body: out (12) with the effect and off the
  collision at frame 1, fading from frame 22 to 41 and done at 141; in (13)
  with the effect at frame 30 (0 for a warp, `WORLD_MAN` +0x168), fading in
  over 50-70 (20-40), done after.
- `actCnt` counts frames unless `transferLag` holds it; `atkDellay` counts
  down.

## Following

The AI's following (`personal.cpp`) turns the body and sets `moveFlag` and
`runFlag`; `Move` does the walking. What each writes of the AI is on the
decisions' side (`party_ai`): `followSW`, `goBackFlag`, `detourCnt`,
`distTg`, `dircTg`, and the global `aiOpenDirc` (main 0x00378ca4).

- **`FollowPlayer`** (0x00581580): a member walking heads for the point 200
  out from Kite's position along his heading turned by `fpAngleOffset[slot]`
  (main 0x003782a0: slots 0-2 at 0, 20480, 45056; a member out of the party
  reads the halfword before the array, 110), turning to it unless within 200
  of him; standing, it measures `distPl`. In a dungeon a wall ahead
  (`CheckFrontObstacle`) asks the navigation for a route to Kite's feet
  (`ccNavi::PathFindingInDungeon`, 0x005148e0): found, nothing more; none,
  stop. In a field `CheckFrontObstacleF` toward his feet may start a detour
  (below). Walking: within 70 of the point, or `distPl` within `fpOkRange`
  (120), it has arrived (`goBackFlag` 0) - beside a walking Kite beyond 20 it
  keeps walking, else it stops and takes his heading; otherwise it walks,
  stops running within 50, runs beyond 250 unless Kite walks. Standing beyond 280 it sets off if it can
  (`CheckAction(1)`), running unless Kite walks, or beyond 350. Kite gone
  (off the lists): stop, `followSW` and `goBackFlag` 0.
- **`LeavePlayer`** (0x00581cb0): steps away from Kite (mode 5) until
  beyond 260 (`goBackFlag` 0, it turns about and stops); starts within 150.
  In a dungeon a wall that way asks for a route to the point 280 along the
  way out, which the code passes as the goal unconverted (a direction, not
  a position).
- **`FollowTarget(tg)`** (0x00582090): closes in on a target on the lists.
  `dircTg` is the heading to its `pos` (in the player's frame), `distTg` the
  ground distance less both widths, truncated to a whole number; the member
  turns when able (`CheckAction(2)`) beyond `noTurnRange` (from Mutation on
  at any distance) and while no skill runs. A dungeon's wall asks for a route; a field's obstacle a detour.
  Wary (mode 2) it runs after the target beyond `stopRange` or while Kite is
  within `territory`, stops within `stopRange` or with Kite beyond
  `territory`; otherwise it runs beyond `attackRange` and stops within
  `stopRange`.
- **`FollowTargetDirc(tg)`** (0x00582ad0): faces a target on the lists not
  dying; `distTg` less the target's width only.
- **`CheckFrontObstacle(h)`** (0x00589e90): a line (`ccHitCheckLM`, mask 1)
  from the feet raised 99 to `bodyHit.radius + 1.1 speed` along `h` (-999.0:
  the body's heading).
- **`CheckFrontObstacleF(goal)`** (0x00589fe0): from the feet raised by
  `bodyHit.height`, a line of `1.5 r` toward the goal (`r = CFOFP x
  bodyHit.radius`, CFOFP 2.5 at main 0x003782ac): clear, 0. Else six probes
  in pairs either side, `r sqrt 2` at 45 degrees (8192), `r sqrt 5` at
  `atan 2` (11547), `2r` at 101 degrees (18432), each blocked on the way out
  (2) or on to the goal (1) or open (0). The first pair with an open probe
  sets `aiOpenDirc`, taking the next probe's side when this one is half open
  and that one fully; returns 1, 2 for the widest pair, -1 when all are
  blocked. The lengths and `atan 2` are computed in doubles (`sqrt`,
  `atan2`, `dpmul`, `dptofp`, main), which round as IEEE doubles do
  (*measured*).
- **Detours** (a field): blocked with no detour running, the member takes
  `aiOpenDirc` at once for 30 frames (`detourCnt`, 60 when only the widest
  pair was open) and holds that heading while it runs; a clear way cuts a
  detour of 6 or more to 5. `Brains` counts it down.
- **`SetDircZ(dd)`** (0x00581500): `gDeg = dd`, `dirc.z = DEG2RAD(dd)`.

## Party navigation

A party member's `ccAI` holds a `ccNavi` at +0xf0 (0x70 bytes: `goalPos`,
the last way's ends in map cells, `name`, `finishFlag` +0x1a, `step`,
`landmark`, `dist`, `dirc`, the window `mapX`/`mapY`/`mapS` +0x28,
`route[48]` +0x2e, the beacon table `num` +0x60 and pointer +0x64). The
movers the decisions call walk what it holds; `crates/piney-battle`'s
`navi` and `ai_move` are the port.

**The dungeon map.** Each floor has a map of 300-unit cells
(`u8[256][256]`, `[x][y]`, at `DUNGEON` +0x33538 + floor * 0x10000;
`WORLD_MAN::Get2DMapPtr`, main 0x001a22b0). `DUNGEON::MakeMiniMap(room)`
(0x005cd850, from `DUNGEON::Draw` 0x005ce930) fills a room's square once,
marking its minimap byte +2: 12, 22 or 42 cells for a small, medium or large
room starting 1800, 3300 or 6300 units below its position, 82 cells from
6300 below for a story room whose `ROOMDATA.type` is 16 or more. Each cell
drops a line from z 1500 to -500 through its centre (`ccHitCheckLM`, mask
-1); on a hit, a cell still 0 becomes 3 for a ground attribute
(`hitResultNearest` +0x48) with 0x20000, 2 with 0x80000, 4 for
`attr & 0xf0f0f0 == 0xc0c0`, 0 for 0x303030, else 1; cells at 200 or more
are skipped (read, not checked). `SetPathFindingMap` (0x00515aa0), run when
`ccThSpc` (0x005a0530) starts and after a room is built, turns the current
room's window (`WORLD_MAN::Get2DMapInfo`, main 0x001a22f0:
`DUNGEON::GetRoom2DPos` 0x005cf080, the room's position / 300 through
`fptosi` less half its size, 10, 20 or 40 by the minimap size code, 80 for
a story room of type 16 up) into `buf` (0x006faa70): for each walkable cell
(not 0 or 4) a bit per walkable neighbour, 1 at y - 1, 2 at x + 1, 4 at
y + 1, 8 at x - 1; then `bufFlag` (0x00378c30) is 1.

**Ways.** `ccNavi::PathFindingInDungeon(s, g)` (0x005148e0) takes both ends'
cells (`WORLD_MAN::Get2DPos`, main 0x001a2210: / 300, `fptosi`) within the
member's window, set when its `ccAI` is made in a dungeon
(`SetDungeonMapInfo`, 0x00514890). It returns -1 before `SetPathFindingMap`
has run or for a goal cell with no link; 0 outside a dungeon, for one cell,
for an end outside the window, or when no way is found; 1 with a way. The
wave (`HeuristicType1`, 0x00514cc0) spreads over `buf2` (0x0070aa70: the
window at -1, the start 0, the goal -2) one step per pass for
`mapS * 14 / 10` passes, into any neighbour at -1 whose `buf` byte is not 0
(across the whole map, so one cell past the window too). It stops once two
neighbours of the goal in the window are reached (`LinkNum`, 0x005157f0,
which counts `buf2 > 0` with `buf2 & 0xf` not 0, so a neighbour 16 or 32
steps out does not count). `ShortPath` (0x00514f60) walks back from the goal
to the lowest neighbour (`MinimumLinkDirc`, 0x005156b0: y - 1, x + 1, y + 1,
x - 1, the first of equals), marking `buf` 0x80. `MakeBeaconTbl`
(0x005150c0) resets the window's other cells to -1, marks the goal and each
corner (two perpendicular neighbours on the way, `LinkInfo` 0x00515580)
0x20, and lists the way's cells from the one after the start to the goal.
`SetBeacon` (0x00515470) copies them into a new beacon table, with `step`
and `beacon.num` the count and `route[i] = i`. Then `name` and `landmark`
are 0 and `finishFlag` 0.

- A way is at most 14, 28, 56 or 112 cells in a 10-, 20-, 40- or 80-cell
  window.
- `HeuristicType1` returns 0 on both of its ways out:
  `PathFindingInDungeon`'s "found" branch (return 2) is dead code.
- `ShortPath` fails on a `buf2` value with bit 7 (it tests `buf2` where it
  means `buf`); the wave never produces one.
- `route` has 48 bytes. A way of 49-54 cells runs into the padding and over
  `beacon.num` (which ends as 50, 0x3332, 0x343332 or 0x35343332); from 55
  cells it overwrites the table's pointer, and from 67 the AI's message
  queue.

`GetDestination` (0x00514770) gives the point being walked to while
`landmark < step`: a beacon's cell centre in a dungeon
(`WORLD_MAN::Get3DPos`, main 0x001a2250: 150 + 300 x, 150 + 300 y),
`naviMapPtr[route[landmark]]` in the Root Town; after the last, `goalPos`;
in a field it writes nothing. `CheckGoalBeaconPos` (0x005975b0) is 0 when a
position's cell is the last beacon's (`route[beacon.num - 1]`), else 1.

**The Root Town.** A town's landmarks are 0x30-byte records (position,
`name` +0x10, links, junction, main lines): Mac Anu's are `naviMapTown1`
(0x006144a0), 72 of them (`naviMarkTable`, 0x00619408). `ccSetNaviMap`
(0x005130b0, from `ccThPlayer`) moves landmarks 1-71 to their
`DMY_markerNN` dummies. `ccNaviSearchLandmark(name)` (0x00513240) finds the
first landmark with a name; in Mac Anu names 1-6 are landmarks 44, 48, 49,
46, 47, 50, the shops `ActInTown` walks to. `ccNaviSearchNearLandmark`
(0x00513300) gives the nearest within 10000 on the ground and 301 in height,
and `ccNaviSearchNearLandmarkN(pos, n)` (0x00513420) the nth nearest (-1
without n). Routes between landmarks are `ccNavi::RouteSearchByMap`
(0x00513720).

**Movers.** `ccAI::MoveP2P(p1, p2, run)` (0x00582d50) returns the ground
distance less `bodyHit.radius`, truncated to a whole number. It turns the
body to face `p2` (through the 16-bit angle) when it may act
(`CheckAction(2)`) and is not within its stride (`personality->velocity`),
starts it walking beyond its stride (`CheckAction(1)`), and makes a walking
body run when asked. `FollowBeacon` (0x00582930) walks the route running,
taking the next landmark within 150. At the end it faces the goal
landmark's point (a dummy named by `naviPointNameTable`, 0x00653c80) and
stands, and in the Root Town it rests (`actType` 11) for
`((rand() >> 3) % 15) * 30 + 100` frames. `SetTargetPosDirc` (0x005830f0)
turns at once and sets `dircTg`.

**Remote control.** Events set up a member with `ManualMode` (0x00583270),
`SetRemoteCmd` (0x005832e0) and `SetGoalPos` (0x005833a0). `ManualControl`
(0x00580ef0) acts on `remoteCmd`:

| cmd | what |
| ---: | --- |
| 0 | stand, turn toward `gDeg` by `gRotSp` (`ccSetDirc`); `remoteFlag` 0 once facing it while `stopFlag` is set, else 1 |
| 1, 2 | walk, run to `gPos`: straight for `gPoint` -1, else along the town route to landmark `gPoint` (`TownNavigatorPoint`, then `gPoint` -2); within 50 of the end: stand, `remoteCmd` 0, `remoteFlag` 1, `gDeg` the heading it had this frame |
| 3 | in the area (`actNum` 2): done; transferred out (14): `TransferIn` |
| 4 | out (14): done; else, unless leaving (12), `TransferOut` |
| 5 | unless out or leaving: `TransferOut` and out of the party (`resignParty` 0x0059d150, or `disbandSpc` 0x005a0f50 for a member of none); `partyFlag` -2 |
| 6 | hide: `actNum` 14, transparency and cloak 0 |
| 7 | show: `actNum` 2 if it was 14 (its body into the world, `HitEnable`, while alive or down), transparency and cloak 1, `dispSW` |

"Done" clears `remoteCmd` and `remoteFlag`; the others leave `remoteFlag` 1
until done. The turn compares `gDeg`,
read unsigned, with the new heading's sign-extended 16-bit angle, and
`RAD2DEG(DEG2RAD(s))` is not `s` for any positive `s` (measured over every
angle), so a turn only ever counts as done facing heading 0. A `gRotSp` of
0 divides by zero in `ccSetDirc`.

**The town walk.** `ActInTown` (0x0057f660) runs a member in the Root Town
(Orca in Mac Anu once he has joined). First the party's orders: a recalled
member rejoins (`partyFlag` 1, `actType` 3); a member leaving
(`partyFlag` -1) walks to landmark 1 (`actType` 2, then -2) and, with five
members registered (`ccSpcRegistryNum`, 0x005a1620), says goodbye and
leaves; the commands 12 (stop, `actType` 0) and 9 (follow, 3) change mode
to 1. Mode 5 keeps its distance from Kite (`LeavePlayer` from 150, turning
away from 170; back to mode 1 beyond 260). Mode 1 goes by `actType`:

| actType | what |
| ---: | --- |
| 0 | stand; rest (99) for 30-300 frames |
| 1, 98 | wait, then `TransferOut` (98) |
| 2 | walk the route to landmark 1; goodbye line, then 1 |
| 3 | to Kite by the route; within 200 follow him (94, the greeting when invited) |
| 4, 12 | to landmark `actDummy`; then rest |
| 5-9 | to the weapon, magic, record, goods or fairy shop (landmark names 2, 3, 6, 4, 5), with its line |
| 10 | to a landmark at random, with its line |
| 11 | rest, then 99 |
| 20 | walk the route |
| 94 | follow Kite (`FollowTargetTown`, 0x005826b0); within 150 stay with him (97) |
| 95, 96 | wait for Kite: follow him (94) or rest |
| 97 | `FollowPlayer` |
| 99 | rest, then a shop at random (5-10, not the same twice, not 7 for character 1) |
| 100 | once back up (`actNum` 2), 0 |
| 101 | after a pause (at once after 97), back to `actTypeOld` |

A walk ends within twice the stride of its landmark, facing the landmark's
point, resting (11) for 100-520 frames. Once `noMoveCnt` (frames moving
without a whole unit of progress, counted every frame of mode 1) passes 200,
the member looks for the nearest landmark (12), or gives up following (3)
beyond 500 from Kite. When it reaches Kite (3 and 94) it stands only if Kite
has `stopFlag` set (bit 4 of +0xe0; *measured*, not `moveFlag`). The lines
are `ccAI::ChatMessage` with `byebyeMessages` and the `goto*Messages` tables
(0x00653020-0x006533b0), indexed by `MessageIndex` (0x0058dc40: the
character id, 18 for id 1 while `saveData` +0x220d is set).

**The movement composed.** `party_motion::Movement` performs the party AI's
movement calls with the ported code (the following, the path finding, the
movers, `HitEnable`). `tools/test_battle_navi_rs.py`'s composed runs check it
against the game's own code run end to end: Orca in Mac Anu for 60-300 frames
(`ccAI::ActInTown` with `FollowPlayer`, `LeavePlayer`, `FollowTargetTown`, the
`TownNavigator`s and `RouteSearchByMap` over Mac Anu's own tables, the
landmarks first moved to town01's dummies by `ccSetNaviMap`), 30-120 frames
of `ManualControl` under changing event commands (`HitEnable` through the
world), and 60-300 frames of the whole `ccAI::Brains` in a dungeon room (the
eye line, `PathFindingInDungeon`, `FollowBeacon`, `CheckGoalBeaconPos`,
`FollowPlayer`). The port's world answers the route search with what the
game's own search found, and one script answers every `ccHitCheckLM`.

## Enemies appearing

### The entry control

`g_entCtrl` (`ccEntryCtrl`, 0x40 bytes) keeps four doubly linked lists of `ccEntryObj`s: enemies, magic circles, gimmicks and NPCs, each with `head`, `foot` and `num`, each object's `pre` and `next` at +0x1c0 and +0x1c4. `ccThEntryCtrl` (gcmn 0x00431970), a task at priority 64, sets the lists up when an area starts and then, once a frame after `ccTscb::Breath`, walks the four lists in that order. Each list is walked from its head for as many objects as it held when its turn began, the next object taken before each runs, so objects made during the walk wait a frame and a deleted object does not break it (measured). An object with `objFlag` runs `ccEntryObj::routine` (gcmn 0x0042fa60) and then its class's `main` through vtable slot +8; a non-zero result deletes it with `deleteEnemy` (gcmn 0x004312d0), `deleteMagicCircle` (0x004314e0), `deleteGimmick` (0x004316a0) or `deleteNpc` (0x00431860): unlinked, its body out of the collision, off the command list (`ccDeleteCmnd`, gcmn 0x00519700), its class's clean-up hook (`destFunc`, +0x1c8). The port calls an enemy's `ccEnemy::main` (gcmn 0x00432cd0), a gimmick's and an NPC's `main` through a seam at exactly that point.

Set-up in a field or dungeon: `restoreEntry` (gcmn 0x00431070) when the area was only left for a moment (every kept object `initObject`ed again, so only those of the current place come back on), else `initEntryCCS` (resource lookups only), `WORLD_MAN::EntryGimmick` (main 0x001a1f20) and the event manager's entries (`ccEntryEventMng`, main 0x001b62e0). The Root Town runs the same frame loop; it does not call `EntryGimmick`, and the event manager places the merchants, the Chaos Gate and the walking PCs (read from the code, not checked).

`ccThEntryCtrlDelete` (gcmn 0x00431d10) runs when the task ends. When the area is left for a moment (`ccGame::CheckSceneReplace()` false and the game's mode word 5), it deletes the enemies whose `entRoot` is neither -1 nor 0 (those made by circles and corpses), the gimmicks likewise and those fading out (`fadeFlag` 2), and every NPC, keeping the magic circles and the rest in `g_entryList`; otherwise it deletes every object (measured).

`ccCheckActiveEnemy` (gcmn 0x0042df10) counts the listed enemies with `objFlag` set and `freezeFlag` clear. `ccCheckActiveObject()` (gcmn 0x0042df70) is true when no enemy and no circle has `objFlag` set. `ccCheckActiveObject(floor, block)` (gcmn 0x0042e010) is true when none belongs to that floor and block; `DUNGEON::SetRoom` opens a room's doors then (measured).

The enemies' own `main` is run for the entry control by `entry::EnemySeam`: at each enemy's turn in `ccThEntryCtrl` (gcmn 0x00431970) it counts `ccCheckActiveEnemy` (gcmn 0x0042df10) on the lists, runs `ccEnemy::main` (gcmn 0x00432cd0) as `enemy_motion::Motion::enemy_main` on the control's scene and world, and carries out what that frame asks of the control and the save where the game does: the enemy book's record of a Data Drain into the save at once, and, as `main` returns (both are the last thing the game's `main` does before returning 1), `entryEnemyObject` (gcmn 0x00430250) for a corpse taken away and `entryDrainEnemy`'s (gcmn 0x004359e0) spawn of the drained form (`entryObject(ep, 1)`, then `transparency` and `setTransparency` 1.0, `condition.dead` 1, `drainCnt` 60 or 30, `effAfterDrain(obj, -1)`). The rest of the frame's calls go to the world, so the entry control's context holds a `MotionWorld`. Measured: `tools/test_battle_spawn_rs.py`'s `frame_enemies` runs `ccThEntryCtrl` with the game's own `ccEnemy::main`, the races' code and the affect functions against the port frame by frame.

### ccEntryObj::routine

`ccEntryObj::routine` (gcmn 0x0042fa60) runs every frame before the object's `main`:

- With `grotSpd` set, the heading turns toward `grotDeg` (16-bit units) through `ccSetDirc`. The turn ends when `fabs((double)(dirc.z - target)) < (double)0.003f`, a comparison in double.
- `plDist` is Kite's ground distance and `plDirc` the heading from Kite. The object's position goes through the player's frame (`W2P`) and is negated; `plDirc` is pi/2 + atan2, wrapped to -pi..pi.
- `dispSW` is on within 7000 (`<=`).
- Beyond 10000 the object is frozen (`freezeFlag`) and taken off the command list (`deleteCmnd`, gcmn 0x0042fe40). `cmndFlag` is then set for an entry whose `entRoot` is neither -1 nor 0, so that object stays off. Within 10000 it is unfrozen and put back (`ccEntryCmnd`, gcmn 0x00519630) unless `cmndFlag`.
- A fade in (`fadeFlag` 1) or out (2) moves `setTransparency` by 1/`fadeCnt` a frame, clamped at 1 or 0, ending the fade.

All of this is measured.

### Spawning

`entryObject(ep)` (gcmn 0x00430c90) decides the count. An enemy of row -1 first takes a registered row at random. It then makes `abs(ccRand() % esize) + 1` by its `enemyTbl` row's `esize`: with `esize` 3 a count of 1 becomes 2 when `(ccRand() >> 2) & 1`, and with `esize` 4 a count of 1 always becomes 2. A gimmick or NPC makes its table row's `esize` (`gimmickTbl` gcmn 0x0061e0f0, `npcTbl` 0x00619460).

`entryObject(ep, n)` (gcmn 0x004307f0) first calls `entryObjectCheck`. With `n` 1 it makes one object on the spot: put on the ground when `land` is -1, and an enemy gets `param[3]` 0. Otherwise it makes `n` objects 300 away round the spot. The `i`th stands at angle `a/2 + start + i a`, where `a` is 2 pi / n through `RAD2DEG`/`DEG2RAD` and `start` is 90 degrees before the entry's heading, rotated with `sceVu0RotMatrixZ`; it faces 90 degrees on, with `param[3]` = i. After each enemy whose `entRoot` is neither 0 nor 3, a registered row drawn at random replaces the entry's row when that row's `esize` is 3 or 4 (measured).

`entryObjectCheck` (gcmn 0x00430530) returns true when nothing more is made. An enemy of row -1 takes a registered row. An event-placed gimmick (`entRoot` 0, `param[0]` an event entry number) whose save bit `ccSaveData::CheckEventEntry` (main 0x00178030) finds cleared is changed: ids 0-5 get `param[1]` = -1, ids 38-44 get `param[2]` = 0, and id 6 is not made at all. Id 15 becomes a magic circle (`entryMagicCircle`). A fountain (id 20) already used (`ccSaveData::CheckFountain`, main 0x001782f0: `code | server << 29` among 100 words) is not made (measured).

`entryEnemy` (gcmn 0x00431220) calls the row's `entry.func`, which `ccInitRegisterEnemy` (gcmn 0x0042e750) sets from `ccEntryRaceTbl` (gcmn 0x005f1d60), then `initObject`, then appends to the enemy list. `entryMagicCircle` (gcmn 0x004313f0) sets the entry 250 above the ground (`ccLandHitCheck(pos, 0x20000002)`) before making the circle. `initObject` (gcmn 0x00430ec0) switches an object on (`objFlag`, `initFlag`, body into the collision) when its area, area number, floor and block are the control's, and off otherwise. With `land` -1 it puts the object on the ground, a circle 250 higher (measured).

`entryCircleObject` (gcmn 0x00430360) is what an opened circle gives, read from its entry's params:

- `param[1]` is the kind: -1 means enemies, or a gimmick 1 time in 8 (`(ccRand() & 7) == 3`).
- `param[2]` is the id: -1 means a registered row, or gimmick `ccRand() & 1`.
- The objects are made from a copy of the entry with `entRoot` 2, `land` -1, heading `plDirc` and params -1. An enemy from an event circle keeps `entRoot` 0 and gets `param[2]` 0.
- `param[3]` is the count: -1 means `entryObject(ep)`'s count.

`entryEnemyObject` (gcmn 0x00430250) runs when a dead enemy is taken away and its `param[2]` is not 0. With `(ccRand() & 3) == 0` it leaves a treasure box (gimmick `ccRand() & 1`, `entRoot` 1) where the enemy stood, with the trap-removal effect and sound 215 (measured).

Registration: `ccRegisterEnemyList(server, type, rank)` (gcmn 0x0042ed70) registers `ccRegisterEnemyRange` (main 0x00378190, 3) rows of `ccEnemyListInfo[server][type]` (gcmn 0x005da200) from `rank` into `ccRegisterEnemyTbl` (gcmn 0x006fa900, `int[8]`, count `ccRegisterEnemyNum` main 0x00378194), staying on the last row past the end. Each registered row also registers its drained form into `ccRegisterDrainTbl` (gcmn 0x006fa920, `int[16]`, count `ccRegisterDrainNum` main 0x00378198) once, and a middle boss registers its base form's drained form as well. `ccRegisterEnemyOne` (gcmn 0x0042e8d0) registers one row. The two tables are contiguous and unchecked, so a ninth registered row lands in the drain table (measured). `ccAnalyzeEnemyList` (gcmn 0x0042f290) is the highest level among the three rows that would be registered.

### The magic circle

`ccMagicCircle::ccMagicCircle` (gcmn 0x00455900) makes a `ccGimmick` with 128 free particles, model `CMP_xmagcir0` playing `ANM_xmagcir1`, clean-up `ccDestMagicCircle` (0x00455870), kept off the command lists for good (`deleteCmnd(1)`).

`ccMagicCircle::main` (gcmn 0x00455b60): while closed (act 0) and frozen, it frees its particles and does nothing else. Otherwise it makes the particles' effects if they are not made, passes its position through the player's frame and back, and steps by act:

- act 0: when Kite is on the command list and `plDist <= 3000`, it plays sounds 216 and 217 and moves to act 1.
- act 1: from frame 31 it plays `ANM_xmagcir2` and moves to act 2.
- act 2: from frame 21 it plays sound 215 and gives what its entry says (`entryCircleObject`), then moves to act 3.
- act 3: when its animation has ended it moves to act 4.
- act 4: from frame 65 it goes. An entry with `entRoot` other than 0 counts a circle opened in the save (+0x6864). When it is the area's last circle, it starts `ccThDfComp` (the "PORTALS OPEN" banner, [field UI](field-ui.md#all-portals-open-ccthdfcomp)) and counts the area cleared: +0x6866 for a field, or a type-4 field's first dungeon, else +0x6868. The counts are held at 10000.

Then come `createPart` (gcmn 0x00455710): 3 particles of kind 1 while waiting, `actCnt/4 + 1` of kind 2 while opening, 8 of kind 4 while giving, 2 of kind 3 for 16 frames after. Every live particle runs `ccMcPart::main` (gcmn 0x004548e0), which draws `ccRand` for its first-frame turn, and `ccMcPart::fade` (gcmn 0x00454840). When the circle is open or `plDist < 7000`, the animation steps by 256. When it is shown and `ccCheckCameraDeg(pos, 12288)` passes, it is drawn at `ccGetCameraTransparency(pos, 0, 0, 7000, 600)` times its own transparency. Each particle's draw multiplies that running value by its own transparency, clamped to 0..1, and the product carries on to the next particle (measured, frame by frame, with Kite scripted along a path to a circle and back).

### Where the entries come from

- **A field:** `WORLD::SetMagicCircle` (gcmn 0x005ab610) places 16 entries from `fieldrand`.
  - `circleOfs` 0, 1 or 2 (any other but 3 means 12) marks 4, 8 or 12 of the 16 slots, drawn with `fieldrand(16)` until a free slot turns up, as circles. `circleOfs` 3 places nothing, and a story area makes every slot a circle.
  - Each of the 4 x 4 blocks of 10 x 10 chips (x inner) takes two `fieldrand(10)` draws per try until a site passes. A site passes when it is a free chip's centre (`FIELD::CalcWorldMeshPosition`, gcmn 0x005ae940: 600 + 1200 x chip), on the ground (`WORLD::GetHeight`), and either within 6000 of an earlier entry (the distance is then set to 4000) or more than 3500 from the field's start.
  - A circle is gimmick 15 with `x`, `y` its chip. Anything else is an enemy of row -1.
  - Event area 14, the tutorial field, draws the positions but makes no entries (measured).
- **A dungeon:** `DUNGEON::SetMagicCircle` (gcmn 0x005bf590) makes a circle (z 0, `entRoot` -1) for every magic-circle slot in the generator's `gimPos` list.
  - A story dungeon also makes one for every `GIMMICKDATA` row of type 1. `param[0]` is the next event-entry number, and `w` is a caller's stack word, taken as 1.
  - Dungeon types 8 and 9 add special objects (gimmick 18, `land` 0) for slots of kinds 3 to 6 (measured).
- **A field's other setters:**
  - `WORLD::SetFood` (gcmn 0x005aa990) first puts the Spring of Myst (gimmick 20) at the lake's `wp`, z 0, when the field has one: `WORLD.water` (+0x30) is the lake's index among the `FOBJECT2`s (`fobj2`), and the dungeon entrance is always index 0, so 0 means no lake.
  - Then, for each `fobj2[0 .. KeyObjNum]` and each chip's `fobj[a][b]` (b the outer loop), it counts the object's `OBJ_xgfood0a*` nodes: its anm's (`GetSubstAdrs`, the chunk's order), or with no anm its clump's (`GetObjAdrsF`). With one to three of them, `fieldrand(n) + 1` foods (`GetFood()`: 23, 24, 25, 26, 33, 27, 28, 29, 30, 31, 32 by field type 0-10) go to nodes picked by `ccRand() % n` until a free one.
  - A node's place is its world matrix's last row (`_SetLWMatrix`; the anm's root is still the identity, since the field has not been drawn yet) plus the object's `wp`, w 1.
  - `ccRand()` is signed. A negative remainder indexes the words below the `used` array (the search's count and table pointer, never 0), so the game simply draws again.
  - `WORLD::SetSpecialObj` (gcmn 0x005aae90) puts gimmick 21 (`ENTRANCE`, the swirl `effDungeonEntrance(pos, &effsw, 2)`) at each of `WORLD_MAN.inPoint[2]` (+0xd0, `GetDungeonMarkerPoint`). `SetDungeonEnter` set them to `DMY_inpoint0` and `DMY_inpoint1` of the field's file (w 1) plus the entrance's `wp`. Areas 28, 39-42, 53-57, 78-82 and 109-113 get none.
  - It then gives each `fobj2` object with `OBJ_xgsymb*` nodes (its anm's, else its clump's), and each chip's `fobj` with them in its anm, a symbol (gimmick 18) at node `fieldrand(n)` when `fieldrand(100) >= 41`. Last it records type 7's fires (`firePos`).
  - All of these are `entRoot` -1, `area` 1, `areaNum` `game.field`, `land` 0 (measured).
  - `tools/test_battle_spawn_rs.py`'s `field_gims` runs the game's `SetFood` and `SetSpecialObj` on a `WORLD` of random objects: `fobj2` and chips' `fobj`, with and without anms, 0-5 food and 0-3 symbol nodes, a lake or none, the in-points, and areas inside and outside the list. The node searches are hooked to answer the objects' nodes. The port's `entry::world_set_food` and `world_set_special_obj` make the same entries in 300 cases. piney-game's `a_field_has_its_portals_entrance_and_foods` and `a_lake_field_has_its_spring` (session) enter random fields of Δ. They find the portals and their enemies, the two swirls, the symbols, the foods, and the lake's spring. The fields' places come from piney-world's `FieldArea::field_gims`.
- **Which setters run:** `WORLD_MAN::EntryGimmick` (main 0x001a1f20) runs `SetFood`, `SetMagicCircle` and `SetSpecialObj` each time a field is entered, but not on a hand-made event map. For a dungeon it runs `SetItemBox`, `SetMagicCircle` (except in volume 2's field 27) and `SetIDOL` once per dungeon, and `EntryBreakObject` every time (measured).
- **An event:** `ccEntryEventMng` (main 0x001b62e0) turns `entry_mc TYPE CODE MARKER PARAM` into a circle (`entRoot` 0) at the marker's `evPos`, relative to Kite when its floor and block are 9999, giving `param[1]` = TYPE, `param[2]` = CODE and `param[3]` = PARAM (0 and 1 mean 1). In a dungeon the room's doors also close. An `entry` of type 5 or 6 makes PARAM enemies of row CODE (`entRoot` 0, `param[2]` 0) at the marker (measured, except the doors).

Field 14 (read from its data, not run in the game): TEACH-F's `entry_mc` makes 4 circles of code 130 at (0, 4000), (0, -1500), (4000, 0) and (-4000, 0) from Kite. TEACH-D places a circle on floor 0, block 3 at (24000, 12000, 250), marker 100. The area's dungeon is a story dungeon of 2 floors whose `GIMMICKDATA` puts type-1 circles on floor index 1 at (21093, 13500) in room 2 and (11906, 13500) in room 3.

### The race constructors

`ccEntryRaceTbl` (gcmn 0x005f1d60) gives each of the 22 races a `ccEntryEnemyX` wrapper (the table below), which allocates the object (0x350 bytes; `ccEnemyG` 0x380, `ccEnemyL` 0x3b0) and runs the race's constructor. The port makes every race, but not `ccEnemyL`'s type 3 (rows 203-206), whose constructor also sets up lights, scaling and colours (`initELG`).

Every constructor starts with `ccEnemy::ccEnemy` (gcmn 0x00432a90): display on, alpha 128, fading in over 30 frames. Then `ccEnemy::initEnemy` (gcmn 0x00433260) sets:

- the animation: `anmTbl[6]` in the main slot, or the second slot for a middle boss, whose name comes from `ccGetNameBossAnm`, gcmn 0x0042e580, byte 7 made 'x';
- the body: radius the base's width, height half its height, kind the base's type, ground mask 0x40000002 in `mask2`, switched into the collision;
- the dust colour from `ccCheckDustColor` (gcmn 0x0043a250): the ground's attribute under the enemy, type bits 0x00f0f0f0, is 116 for the dark grounds and 117 otherwise;
- the enemy AI's initial state;
- `ccEnemyInfluence`.

Then the race's own part: act 0, animation 6 with `anmNumOld` -1, `eneType` from the race's row table (a jump table on the row), the weapon trail and dust controllers of that type (in a few races by row: `ccEnemy1`, `ccEnemyA`, `ccEnemyU` and `ccEnemyV` give some rows of one type their own weapon), and the race's clean-up hook (`destFunc`). `ccEnemy1`, `A`, `E`, `F`, `G`, `I`, `P` and `V` set `anmFlag` 1; all of those but `ccEnemy1` and `G` also set `frameNum` 0xffff. The controllers sit at +0x340 and +0x344 (`ccEnemyG` +0x374, `ccEnemyC` +0x348).

- G's gold goblins run `ccEnemyG::checkGold` (gcmn 0x00447200). Their gold type comes from the row; `goldParam` comes from the volume and type, and the larger hoards draw `ccRandS` (main 0x001d9c10: `lastRnd = rotl2(((lastRnd ^ 0x1100) - 25939) & 0xffff)`, state at main 0x00378aec). Rows 154-157 swap their palette.
- The spellcasters' attacks are reweighted to 0.1 (the first two) and 0.4 (the next two): G type 4, and types 1 and 2 of `ccEnemy3` and `ccEnemy4`.
- `ccEnemyH` types 3 and 4 (Cerberus, Flame Heads, Black Death) make two fire breaths (`ccEnemyBreath` at +0x348 and +0x34c, `initBreath` with `ehkBreathInfo`, gcmn 0x005e27a0, 28 bytes each: the node they come from is looked up in the clump).
- `ccEnemyL` (0x0044b270): types 2 and 4 (the wyrms, rows 197-202, and the dragons, 207-217) make one breath at +0x348 from `elBrInfo` (gcmn 0x005e5ae0, row `id - 197` or `id - 201`); every type then draws `ccRandS` four times into +0x390 (the type 3's colours), sets its scale (+0x380) to 1 and clears the rest.

Every row of the 22 races but L's type 3 was compared field by field, with the controllers and breaths made (measured: 299 rows).

## Enemy movement and animation

`ccEnemy::main` (0x00432cd0) ends an enemy's frame with its movement and
animation: after `think()` and `interruptThink`, the race's `action()`,
`hold` cleared, `moveEnemy` (0x00433e80), `animEnemy` (0x00434560),
`dispEnemy` (0x004348b0) while `dispSW` is set, and the race's
`exclusive()`. The race's functions come from the vtable its constructor
sets: `ccEntryRaceTbl` (0x005f1d60) by the row's race group (the rows' own
`ccEntry.func` is 0 on the disc). Every race's `think()` is
`defaultThink`; `ccEnemyG::think` (0x004467a0) calls `thinkGold`
(0x00447b20) instead for a gold goblin. Measured by running each
constructor:

| rows | race | class | eneType by row | dust controller |
| --- | ---: | --- | --- | --- |
| 0-12 | 0 | `ccEnemy1` (`ccEntryEnemy1` 0x0043ecf0) | 0: 0; 1-2: 1; 3-4: 2; 5-6: 3; 7-9: 4; 10-12: 5 | 5-12 |
| 13-21 | 1 | `ccEnemy2` (0x0043f910) | 13: 0; 14-15: 1; 16-18: 2; 19-21: 3 | none |
| 22-34 | 2 | `ccEnemy3` (0x004402d0) | 22: 0; 23-28: 1; 29-34: 2 | none |
| 35-47 | 3 | `ccEnemy4` (0x00440b60) | 35: 0; 36-41: 1; 42-47: 2 | none |
| 48-65 | 4 | `ccEnemyA` (0x00441550) | 48: 0; 49-51: 1; 52-55: 2; 56-60: 3; 61-65: 4 | 49-65 |
| 66-78 | 5 | `ccEnemyB` (0x004420e0) | 66: 0; 67-69: 1; 70-72: 2; 73-78: 3 | 67-78 |
| 79-88 | 6 | `ccEnemyC` (0x00442a80) | 79: 0; 80-82: 1; 83-85: 2; 86-88: 3 | 80-82, 86-88 |
| 89-103 | 7 | `ccEnemyD` (0x004438e0) | 89: 0; 90-92: 1; 93-96: 2; 97-99: 3; 100-103: 4 | 90-103 |
| 104-115 | 8 | `ccEnemyE` (0x00444810) | 104: 0; 105-107: 1; 108-112: 2; 113-115: 3 | 105-107, 113-115 |
| 116-128 | 9 | `ccEnemyF` (0x004454a0) | 116: 0; 117-119: 1; 120-121: 2; 122-123: 3; 124-128: 4 | 120-128 |
| 129-166 | 10 | `ccEnemyG` (0x00445fc0) | 129: 0; 130-138: 1; 139-143: 2; 144-150: 3; 151-157: 4; 158-160: 5; 161-166: 6 | 130-166 |
| 167-177 | 11 | `ccEnemyH` (0x004492d0) | 167: 0; 168-170: 1; 171-173: 2; 174-175, 177: 3; 176: 4 | 171-177 |
| 178-183 | 12 | `ccEnemyI` (0x00449e60) | 178: 0; 179-183: 1 | 179-183 |
| 184-192 | 13 | `ccEnemyK` (0x0044a730) | 184: 0; 185-186: 1; 187-189: 2; 190-192: 3 | 190-192 |
| 193-217 | 14 | `ccEnemyL` (0x0044b130) | 193: 0; 194-196: 1; 197-202: 2; 203-206: 3 (not ported); 207-217: 4 | 197-202, 207-217 |
| 218-228 | 15 | `ccEnemyP` (0x0044e4a0) | 218: 0; 219-221: 1; 222-225: 2; 226-228: 3 | 222-228 |
| 229-238 | 16 | `ccEnemyS` (0x0044ee70) | 229: 0; 230-232: 1; 233-235: 2; 236-238: 3 | 233-238 |
| 239-244 | 17 | `ccEnemyT` (0x0044f940) | 239: 0; 240-244: 1 | 240-244 |
| 245-266 | 18 | `ccEnemyU` (0x00450330) | 245: 0; 246-248: 1; 249-251: 2; 252-257: 3; 258-261: 4; 262: 5; 263-266: 6 | 246-266 |
| 267-278 | 19 | `ccEnemyV` (0x004513b0) | 267: 0; 268-270: 1; 271-273: 2; 274-275: 3; 276-278: 4 | 268-270, 274-278 |
| 279-287 | 20 | `ccEnemyW` (0x00451f90) | 279: 0; 280-281: 1; 282-284: 2; 285-287: 3 | 282-287 |
| 288-302 | 21 | `ccEnemyZ` (0x00452a00) | 288: 0; 289-291: 1; 292-296: 2; 297-302: 3 | 288-291, 297-302 |

Every row gets a weapon controller. Gold goblins (`goldFlag`) are rows
131-138, 140-143, 147-150 and 154-157; the first field and dungeon's
goblins (130, 151) are not. Middle bosses (a second model, body hit type
64 instead of 32) are rows 72, 76-78, 164-166 and 224-225. The body hit
comes out with every mask and `mask2` 0x40000002. The sound categories
(`seCategory`, +8) are 0, 15, 2, 13, 7 and 12 for races 0, 5, 10, 13, 15
and 19.

The Delta server's lists (`ccEnemyListInfo`, gcmn 0x005da200) register
most races; by the lowest rank a race first appears at, over the seven
field types: G, P (0), K (1), V (3), 1 (4), B (5), F (6), 2 (7), C (9), U
(20), I (26), H (27), L (29), W (36), D (43), E (45), T (50), A (67), 4
(78), S (93). A field's rank is `25 (areaLevel - 1) + 10 + enemyOfs`
(`WORLD_MAN::enemy_rank`), `areaLevel` 1-5.

**The helpers.** `ccSetRad(&r, to, rate)` (main 0x001da150) turns `r`
`rate` (held to -1..1) of the short way round to `to`; a negative rate
aims at `to + pi`, then takes the whole step; a zero step stores nothing.
`ccSetDist(&v, to, rate)` (0x001da5d0) is `v + (to - v) rate`, not stored
when `to - v` is 0. `ccPiLimit` (0x0043ad60, 0x0043ace0) wraps once each
way. `ccGetDircChgF` (0x001da030) is `ccGetDircChg` (0x001d9eb0) in
radians; only a mode below 0x10000 divides.

**Acts.** `actMove(tMd, pMd, tDd, pDd, tSpd, pSpd, pRed)` (0x004371e0)
turns the walk (`mdirc.z`) toward `tMd` by `pMd` and the facing (`dirc.z`)
toward `tDd` by `pDd`, and moves `speed` toward `tSpd` by `pSpd`, each
only when its rate is not 0. With `pRed`, the target speed first falls by
how far the walk still has to turn (`mdirc.z` turned 0.3 of the way; `d`
the difference): `tSpd (1 - d / (pi pRed))`, not below 0.
- `actFollow` (0x00437420) walks and faces the same way; `actSlide`
  (0x00437460) walks where it faces while turning to `tMd`.
- `actEscape()` (0x004374a0) backs off from the target. On the act's first
  frame two ccRand coins pick the way (type 0 on the first coin's tails,
  else 2 or 1), with a reduction by size (4: 1.0, 3: 0.8, else 0.4).
- `actEscape(type, red)` (0x00437610): a close range under 100 counts as
  pressed (`crisisRate` 0.8), a hold divides `red` by 1.5, and the speed
  aims at `maxSpd (1 + (10 crisis)^2 / 100)`. The heading is `targetDirc +
  pi`, scattered below crisis 0.4 by `2pi/5 (1 - crisis) +
  ccRandF(pi/10)`, below 0.8 by `2pi/5 crisis`, to the side bit 1 of
  `eneRand` picks. Type 2 slides; type 1 walks the heading facing the
  target; otherwise it walks and faces the heading.
- The first row of a race (`raceId` 0) uses `actEscapeX` (0x00437960):
  after 60 frames it attacks if `selectAttack` finds an attack, else
  stops; until then it runs at `maxSpd (1 + (1.1 + ccRandF(0.2))
  (10 crisis)^2 / 100)`.

The races' acts (turn rate / target speed / speed rate / reduction;
toward the target unless marked home; "stop r" is `speed` toward 0 at r):

| act | G `moveEG` | P | K | V | B `moveEB` | 1 |
| --- | --- | --- | --- | --- | --- | --- |
| 0 wait | .125/0/0/0 (speed kept); none: stop .3 | .08/0/.3/0; none: stop .3 | as P | as P | as P | as P |
| 1 close | `actEscape()` | 2 coins, `actEscape()` | as P | as P | `actEscape()` | 1 coin, `actEscape(2 or 0, 0.8; types 3-5 0.5)` |
| 2 wander | every 16 frames `ccSetRadDisperse(pi/2)`; .05/half/.02/1 | type 2 .005/half/.03/1, 0,1,3 .05/half/.05/1 | 3 .01/half/.06/1, 0-2 .1/half/.06/1 | 3 .005/half/.03/.8, 1,4 .02/half/.06/1.5, 2 .005/half/.06/2, 0 .05/half/.06/1 | .01/half/.06/1 | .05/half/.06/1 |
| 3 chase | .0625/max/.03/1 | .03/max/.02/1 | as P | .03/max/.02/1.5 | as V | as V |
| 4 return | home .04/half/.03/1 | home .06/half/.1/1 | as P | home .01/.6 max/.06/0 | as P | home .06/.6 max/.1/1 |
| 5 stop | .125/0/.3/0; stop .3 | .08/0/.3/0; stop .3 | as P | .08/0/.2/0; stop .2 | .08/0/.3/1; stop .3 | as P |
| 6 attack | .125/0/.08/0 | .125/0/.3/0; row 218 .08, 40 for 16 frames | .125/0/.3/0; row 184 snaps to the target, then 40 for 32 frames beyond 200 | .08/0/.3/0; row 267 .125, 40 at .1 for 32 frames beyond 200 | .08/0/.3/0 | as B |
| 7, 8 | stop .5 | flinch: stop .5; dying: speed 0 | as P | as P | stop .5 (dying also 0) | as P |

The other races with their motion ported (`action()` at 0x0043fbf0 (2),
0x00442e30 (C), 0x00445910 (F), 0x00449810 (H), 0x0044a120 (I),
0x00450940 (U)):

| act | 2 `moveE2` | C `moveEC` (types 0-2) | C `moveECS` (type 3) | F | H `moveEH` | I | U `moveEU` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0 wait | as B | as B | slide .03/0/.06/0 (none: nothing) | as P | as B | as P | as B |
| 1 close | type 2 `actEscape()`; 0, 1, 3 one coin, `actEscape(2 or 0, 0.8)` | `actEscape()` | sound 179, `actEscape(2, 1.8)` | 2 coins, `actEscape()` | as B | as F | 0-2 `actEscape()`; 3-6 2 coins: heads `actEscape(1, 0.8)`, tails `actEscape(2, 0.8)` with the walk turned aside first (below) |
| 2 wander | .01/half/.06/1 | .01/half/.06/1 | slide .03/half/.06/1 | .01/.6 max/.06/1 | as B | .02/half/.06/1 | 0-1 .05/half/.03/1, 2-3 .02/half/.03/.8, 4-6 .005/half/.03/.6; the facing follows the walk (.05) |
| 3 chase | .03/max/.02/1.5 | .03/max/.02/1 | slide .09/max/.28/1 | .03/max/.02/1.5 | as B | .03/max/.03/1 | .03/max/.02/1 |
| 4 return | as B | as B | slide home .06/half/.1/1 | home .01/.6 max/.06/0 | as B | home .02/half/.06/0 | as B |
| 5 stop | as B | as B | spin (below) | .08/0/.2/0; stop .2 | as B | as F | as B |
| 6 attack | as B | as B | slide .24/0/.3/0 | .08/0/.3/0; row 116 .125, 40 at .1 for 32 frames beyond 200 | as B | .125/0/.3/0; row 178 snaps to the target, then .16, 40 at .1 for 24 frames beyond 250 | .08/0/.3/0; row 245 snaps to the target, then 1.5 max for 32 frames beyond its `atkRangeA` |
| 7, 8 | as B | as B | stop .5 | as P | as B | as P | stop .5 |

The clips: F and I as P; 2, C, H and U as B. `ccEnemy2` types 0, 1 and 3
turn the facing after the walk as `ccEnemy1` does.

- **The scorpion's spin** (`moveECS` 0x00443330, act 5): on the act's first frame
  `pi/2 speed / maxSpd`, to a side by a coin (`optFlag0`), from the
  target's heading (or its own facing without one), wrapped once, is kept
  at +0x340. Then `actMove(dirc.z, 0, spin, 0.1, 0, 0.1, 0)`: the facing
  turns there as it slows, with sound 179 on the first frame and 36 every
  8th. Under speed 5 it stops dead and goes back to act 0 (count 0). Its
  `exclusive()` (0x00443780) kicks up dust at the four `wheelPos` (gcmn
  0x005dec10: (200, -300), (-200, -300), (130, 200), (-130, 200)) through
  the model's world matrix (`anm` +0, what the last draw left), every
  other frame of the spin: `ccEnemyEffDust(p, 2, 3.0, 20, eneSmoke)`.
- **`moveEU`** (0x00450b30) first cuts the target's heading to its top bits: `td >>
  r << r` (arithmetic), `r = 18 + (ccRand() & 3)`, every frame. The tails
  escape of types 3-6, on the act's first frame, runs `actEscape(2, 0.8)`
  once only to see which way it turns the walk, puts the walk, facing and
  speed back, turns the walk by `|ccRandF(3pi/4, 3pi/4)|` that way
  (`ccPiLimit`), then escapes for real.
- **Death Head** (row 245) bobs: `radCnt` goes on by 584 (a 16-bit angle)
  a frame and `zoffs = 80 + 30 sinf(radCnt)`; in attacks 0 and 1 by 291
  with no bob; a flinch or dying sinks `zoffs` to 0 at 0.08.
- **The rays** (F type 2, rows 120-121) dive: in attacks 2 and 3, for the
  act's first 50 frames, `yoffs` sinks toward -300 at 0.04; otherwise it
  rises back to 0 at 0.1 (`ccEnemyF::exclusive`, 0x00445e20).
- **The breath** (H types 3-4, `exclEHK` 0x00449d50): while displayed,
  both breaths run (`ctrlBreath`, 0x0043bee0: the flames move, and burst
  on the ground or a wall with `ccRandF` and `effSmoke`); in attack 4 with
  a target, breath `actCnt & 1` breathes (`setBreath(ehkBrParam)`,
  0x0043bfe0). "The fire breath" below has the flames.

The rest but L (`action()` at 0x00440610 (3), 0x00440ea0 (4), 0x00441a70
(A), 0x00443d80 (D), 0x00444bd0 (E), 0x0044f220 (S), 0x0044fbf0 (T),
0x00452340 (W), 0x00452e00 (Z)). `ccEnemy3` (`moveE3`) and `ccEnemyZ`
(`moveEZ`) move exactly as B, and `ccEnemy4` (`moveE4`) as 2 with the coin
for every type:

| act | A | D `moveED` (not 2, 4) | D `moveEDD` (2, 4) | E | S `moveES` | T | W |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0 wait | as P | as B | as B | as P | as B | as P (lid, below) | as P |
| 1 close | clip 7, 2 coins; by type: 0 `actEscape()`, 1 (2 or 0, .8), 2 (2 or 0, .6), 3 (2 or 1, .8), 4 (1, 1.0) | `actEscape()` | a coin, `actEscape(1 or 2, 0.8)` | no clip; 0, 1, 3 `actEscape()`, 2 a coin, `actEscape(2 or 1, 0.5)` | 0, 1, 3 `actEscape()`, 2 `actEscape(2, 0.3)` | `actEscape()`; a mimic gives up after 48 frames (act 5, count 0) | as P |
| 2 wander | scatter pi/4; .01/.6 max/.06/1 | .01/half/.06/1 | .004/.6 max/.2/1 | 0 .05/half/.03/1, 1 .02/half/.03/1, 2-3 .005/half/.03/.5 | 0-1 .05/half/.06/1, 2 .02/half/.02/1.5, 3 .03/half/.03/1.2 | .1/half/.06/1 | scatter pi; .004/half/.06/1 |
| 3 chase | .03/max/.02/1.5 | .03/max/.02/1 | .0625/max/.3/1 | .03/max/.02/1.5 | .03/max/.02/1 | .03/max/.02/1 | the act's count bumped on its first frame; .02/max/.02/1 |
| 4 return | home .01/.6 max/.06/0 | as B | home .04/.6 max/.3/1 | home .01/.6 max/.06/0 | as B | as B | home .06/.8 max/.1/1 |
| 5 stop | as P | as B | .125/0/.3/0; stop .3 | .08/0/.2/0; stop .2 | as B | as P | as B |
| 6 attack | .08/0/.3/0 | .08/0/.3/0; row 89 (Bat) 30 for 48 frames beyond 200 | .125/0/.3/0 | type 0 (Moai) as F's row 116; else .08/0/.3/0 | as D `moveED` (its row 89 test never true) | .125/0/.3/0 | types 0-1 in attack 0 .08/1.2 max/.2/1; else .08/0/.3/0 |
| 7, 8 | as P | as B | as B | as P | as B | flinch clip 8 (the act's count bumped); dying as P | as T |

- `ccEnemy3` and `ccEnemy4` set `moveFlag` 2 in clips 2 and 3; D sets it
  for type 0 in clip 0. `ccEnemy4` turns after the walk as `ccEnemy1`.
- **The mimics** (T type 1) have a lid: waiting after a walk (clip 7) or
  stopping plays 12 (closing), then 6 when it ends; a move opens it (11,
  unless it was walking) and walks (7) once the opening ends.
- **The ghosts** (W types 0-1) bob as the Death Head: `zoffs = 100 + 60
  sinf(radCnt)`, 584 a frame, 291 in attack 0 without the bob, sinking to
  0 at 0.1 when dying.
- **The eyes' kin** (D type 3: Ark Prince, Skate Rat Ark, Alucard) float at
  `zoffs` 0 (0.2) and sink to -50 (0.08) when dying (`ccEnemyD::exclusive`,
  0x004446a0).
- **The turtles** (E type 3) walking (clip 7) raise a ring of dust at two
  feet on notes 1 and 2 (`ccEnemyE::note`, 0x004452b0): before frame 46
  `footObjTbl` 0 and 1 (gcmn 0x005e0640) with `eet1DustInfo` rows 8 and 9,
  from frame 46 feet 2 and 3 with rows 10 and 11, each at its clump node's
  place (`GetObjAdrsF`, `ccEnemyEffDustRing`).
- D's type switch has no default: any type but 2 and 4 runs `moveED`. A's
  escape reads uninitialised registers for types above 4 (none exist).

`ccEnemyL` (`action()` 0x0044bae0): the clips as B, then by type
`moveEL` (0x0044bd60; types 0-1: the snakoids), `moveEL2` (0x0044c010;
2 and 4: the wyrms and dragons) or `moveELG` (3, not ported; its
`dispSW` is cleared every frame and `exclusive()` draws it through
`dispELG`, with its own lights, scaling, colours and texture scrolling).

| act | `moveEL` | `moveEL2` |
| --- | --- | --- |
| 0, 5 | .08/0/.3/0; stop .3 | as `moveEL` |
| 1 close | `actEscape()` | `actEscape(1, 0.3)` |
| 2 wander | .02/half/.03/1, the facing following the walk (.05) | scatter pi/4; .01/half/.06/1 |
| 3 chase | .03/max/.02/1 | .03/max/.02/1.5 |
| 4 return | home .03/.6 max/.3/1 | home .01/half/.06/0 |
| 6 attack | .08/0/.3/0 | .08/0/.3/0, with or without a target |
| 7 flinch | stop .5 | stop .5 |
| 8 dying | nothing | nothing |

Its `exclusive()` (0x0044c300), while displayed: a wyrm (`exclELW`)
runs its breath and breathes in attack 4 at a target (`elBrParam` +0x80,
gcmn 0x005e5aa0); a dragon (`exclELD`) rises in attack 4 (`yoffs` toward
300 at .04 for 190 frames, else back to 0 at .1), runs its breath and
breathes in attacks 4 and 5 (`elBrParam`, +0x40). Its note is the dust's
(as G's).

- `ccEnemyG` quantises `targetDirc` to `& 0xffe00000` first.
- `moveEGG` (0x00446d50, eneType 5-6) uses its own rates (wait .08, stop
  .05, attack .05; one coin: `actEscape(1 or 0, 0.7)` for type 6, `(2 or
  0, 0.8)` for type 5).
- With `moveFlag` 1 and more than pi/4 to turn, `moveEGG` and
  `ccEnemy1::action` turn the facing after the walk by `0.1 d / 0.45pi`;
  `ccEnemy1` also slows by it.
- The clips: wait 6, walks 7, a flinch 8 or 9 (ccRand coin), dying 10. An
  attack counts `atkCnt` and sets `actionFlag` when its clip ends.
- `moveFlag` is 1 while the clip is 7. It is 2 for a gold goblin
  attacking, and for `ccEnemy1` types 2-3 in clips 2-3.

**`moveEnemy`.** The step is `speed (sin, -cos)(mdirc.z)`, added in the
player's frame (`ccTransPosW2P`/`P2W`) and put on the ground
(`ccLandHitCheck(mask 0x20000002)`, less and plus `zoffs`).
- The body hit is tried against the others (`CollisionDetection`), up to
  six times. Each time a body is touched, the step turns toward the
  push (`ccSetDirc` by 128, or 32 above speed 30, where the body first
  swells by `0.6 + speed/30`, at most 600). The step also slows by `1 -
  0.0005 width`, and the body shrinks by 0.9 (the fifth time to 200, with
  a doubled, sharper step). A body of height 300 or more flattens by 0.9.
- Then the line from where the enemy stands to the step's end, half its
  height up, is tried against the walls (`ccHitCheckLM2`, mask
  0x40000002). The enemy moves only if nothing is in the way. Touching
  the ground (result 2 or 3) slows it by 0.1 and takes back half the step.
- A blocked walk swerves: `mdirc.z` moves toward the step's heading, then
  on by `pi/24` per try, with the turn's sign, the speed's share of
  `maxSpd` and `1 + 0.05 hitCnt`, at rates by size (4: /4, /1; 3: /2, x2;
  else x4, x16).
- `hitCnt` counts blocked frames. An unblocked frame clears it and draws
  one ccRand coin into `optFlag1`.
- `hitSpd` moves toward 0.8 once per try, or toward 1. The body returns
  to the width and half the height.

**`animEnemy`.** A new `anmNum` plays `anmTbl + 30 anmNum` (a middle
boss's second model plays the same name with its eighth letter `x`,
`ccGetNameBossAnm` 0x0042e580). The frame speed (1/256 frames) is 256
unless the row's `anmSpd` is 1:
- `moveFlag` 1: `anmSpd 256 speed / maxSpd`.
- `moveFlag` 2: `256 + 512 (1 - d/400)`, `d` the nearest target's
  distance (`selectTarget(0)` with `actNum` set to 0, then
  `selectTarget()`).
- Otherwise, while alive and not attacking: `256 (1 + crisisRate)`.

The speed is at least 64. `frameNum` is the frame before the step;
`anmFlag` is whether a play-once clip ended. The notes the step passed
go to `ccEnemyCheckNote` (0x004328b0):
- 1, 2: a sound (`ccSeSetParamEnemy`).
- 0x8005: `affectSkill`.
- 0x8003: `startSkill`.
- 0x8002: near the camera (`checkCameraShakeRange`), `cameraShake(n, 2,
  20, 2)` for a note value n of 0-2.

Then the race's `note()`: the weapon trail, and dust on notes 1 and 2
for G, K, B, 1, A, D, F, I, L, U and Z (not P, V, 2, 3, 4, C, E, H, S,
T, W; E has its turtles' foot dust instead).

**`dispEnemy`.** The model's matrix is the facing's rotation
(`sceVu0RotMatrix` of `dirc`) moved to `pos` plus `(yoffs, 0, 0)` turned
a quarter turn back about z and then by the facing. A middle boss's second
model is drawn at `ccGetCameraTransparency(pos, width, height, 7000, 600)
x setTransparency`, on layer 5. Then `ccChar::Draw` (0x0056b1c0). The
fade is `ccEntryObj::routine`'s (0x0042fa60), not `dispEnemy`'s.
`exclusive()` runs the weapon and dust controllers (`ccEnemy2`, 3 and 4
only the weapon); a gold goblin walking also raises dust and clinks, and
C, D, F, H and L do the more above.

### The fire breath (`ccEnemyBreath`, gcmn 0x0043ae90-0x0043c134)

A breath (0x10 bytes: +0 its flames alive, +4 its node's matrix, +8 its
`ccEnemyBrInfo`, +0xc the flames) holds 64 flames (`ccEnemyBrPart`, 0x48
bytes, each with a `ccEff` of the info's effect chunk, `Init(chunk, 0)`,
and its palette chunk when named). `ccEnemyBrInfo` (0x1c bytes): +0 kind,
+4 the bursts' smoke kind, +0x10 the node, +0x14 the effect, +0x18 the
palette. `ccEnemyBrParam` (0x40 bytes): +0 a flame's life, +2 and +4 the
attack's frames that breathe, +6 their mask, +0x10 the start speed,
+0x1c the start size, +0x20 the flying speed's growth, +0x2c the flying
size's, +0x30 the fallen speed's (times the frames fallen), +0x3c the
fallen size's.

| row | info | param |
| --- | --- | --- |
| H 174-177 | `ehkBreathInfo` 0x005e27a0: kind 1, smoke 0xcc, `OBJ_neckr04` / `OBJ_neckl04`, `EFF_ehk1fire` | `ehkBrParam` 0x005e2760: life 24, frames 48-108 every one, speed 50, size 2.5 |
| L 197-202, 207-217 | `elBrInfo` 0x005e5ae0 (a row each): kind 0, the head, `EFF_plane01` with a fire palette | `elBrParam` 0x005e5a20: the dragons' attack 4 (life 48, frames 122-168 every other) and 5 (+0x40), the wyrms' attack 4 (+0x80) |

- **setBreath(param, obj)**: the enemy displayed, `transparency` 0.05 or
  more, `actCnt` in start..=end with `actCnt & mask` 0: the first free
  flame starts (`init`), its phase the flames alive over `(end - start) /
  (mask + 1)`, and the count goes up.
- **init**: life, speed and size from the row, alpha 0, pattern 0,
  flying; at the node's place; kind 0 first turns the node's matrix by
  `(pi/2, pi/2, 0)` (the node's own world matrix, in place); the matrix's
  translation is cleared and the flame's turn read off it (`ccSetMat2Rot`
  0x0043ae90: undone about x by `atan2(y', z')`, then y, then z).
- **ctrlBreath**: each live flame moves, animates and is drawn
  (`ccEff::Draw(pattern)` on layer 3); its life counts down, and at 0 it
  is freed.
- **move**: flying, on its first frame the forward speed times `cos(pi/2
  phase - pi/4)` (the flames fan out), then the speed grows by itself
  times the row's growth; fallen, the speed grows by the row's times the
  frames fallen. Never backwards. The step (the speed turned by the
  flame) is tried against the walls (`ccHitCheckLM2`, 0x40000000; a hit
  stops it forward and makes it fallen) and the ground (`ccLandHitCheck`,
  0x20000002, 15 sizes above what it answers; under it the flame lands:
  fallen, kept on the ground). A landed or blocked flame bursts on frames
  1, 3 and 5 after (`explode`). It faces its step (`mov2rot` 0x0043b000)
  and wraps through the player's frame.
- **anim**: flying, the pattern steps by 4 through 0-15, the size grows,
  the alpha goes to 1 at 0.36; fallen, the pattern steps by 1 to 31, the
  size by the fallen growth, the alpha to -0.5 at 0.02 (0 under 0.01).
- **explode**: `effSmoke(place + ccRandF(50) on each axis, half the
  turned speed, size cos(pi/2 phase - pi/4), 8, the smoke kind, 512, 32)`.

In the port the breaths run where the effects start the frame's shows
(piney-world `combat::breath`), so their bursts draw ccRand after the
enemies' frame instead of inside it. The flames are drawn as the
field's ambient sprites, from the enemy's own file.
`tools/test_enemy_breath_rs.py` runs the game's `initBreath`,
`setBreath` and `ctrlBreath` against [`piney_battle::breath`].

### The gold goblins (`thinkGold` 0x00447b20, `moveGold` 0x00448520)

A gold goblin (`goldFlag`) thinks with `thinkGold` and moves with
`moveGold` (from `ccEnemyG::action`, before its `moveFlag`). It runs from
the party, wears off conditions faster, and shakes off holds. The
hoard's size is `goldVolume` (1-4 by row), and `goldParam[4]` is set by
`checkGold`. `checkGold` also widens a volume 3 or 4 row's `area`,
`territory` and `viewRange` to 50000 in the table itself. Every goblin of
that row has the same volume, so the port reads the widened ranges for
those rows.

```text
thinkGold
  selectTarget(0); beyond atkRangeA selectTarget()
  sleep, confusion, charm, paralysis: each held one less goldVolume
    a frame (4, 3, 2; volume 1 not at all), down to 0
  goldDisHold 1: game.inBattleDist -2, off the command lists, 2;
    2: inBattleDist 2200, back on them, 3; 3: 0
  volume 2 on: hit this frame (affectFlag) while held, not paralysed or
    asleep, no puppet show, affect type 1 or 3: goldDisHold 1;
    else from volume 3: held, the first frame marks goldEscPos (and
    goldEscCnt 1), later ones count, and once more than 500 from the
    mark goldDisHold 1; not held, goldEscCnt counts down;
    volume 2: let go: goldDisHold 1
  goldPreHold = hold
  0 wait    held: flee unless a puppet show; target beyond atkRangeC:
            chase; within atkRangeA or an attack delay left: flee;
            else attack (setActGold(6): selectAttack, else wait);
            no target: find one and chase; every 64th frame a coin:
            wander
  1 flee    no target, or beyond atkRangeB from the 17th frame: stop
  2 wander  every 256th frame 3 in 4 stop; a target: chase
  3 chase   within atkRangeB: wait; no target: stop
  4 home    inside the territory: stop
  5 stop    slower than 1: speed 0, wait; from the 33rd frame one in
            four: chase, flee, or (half) speed 0 and wait
  6 attack  its end or 300 frames: wait, setInterval; no target: wait
  7 flinch  its end: wait
moveGold (goldParam[2], [3] in hundredths)
  0 wait    a target: turn to it (0.125), slow (0.2); none: slow (0.3)
  1 flee    held: volume 3-4: base 2.5 + d^2 / 30 / goldEscCnt (d =
            dist / 500 under 1) or - d / 30 goldEscCnt, crisisRate
            max(1 - d, 0.7) + ccRandF(0.1); volume 1-2: zoom at most
            2.5; actEscapeGold(goldParam[3] - base); not held:
            actEscapeGold(goldParam[2])
  2 wander  as moveEG's; 3 chase as moveEG's
  4 home    actMove(baseDirc, 0.05, mdirc.z, 0.2, goldSpeed, goldAccel, 1)
  5 stop    a target: turn to it (0.125), slow (0.05); none: slow (0.3)
  6 attack  actEscapeGold(min(goldParam[2], 1.6) - 1.6)
  7 flinch  paralysed, asleep, or held in a puppet show: slow (0.5);
            volume 3-4: crisisRate 0.8, actEscapeGold(min(goldParam[3],
            3) - 1.5), goldDisHold 1 on the act's first frame; volume 1:
            slow; else actEscapeGold(-0.8)
  8 dying   slow (0.5)
actEscapeGold(zoom)  (0x00448b10)
  away = piLimit(pi + targetDirc) dispersed by goldParam[0]: 0
    ccRandF(0.3 pi (1 - c)), 1 ccRandF(pi (1 - c), pi/10 (1 - c)),
    else none (under crisisRate 0.3 a ccRandF(pi/10) drawn and dropped);
    goldDirc on frames 2 mod 4
  turn by goldParam[1]: 0 1.5 ccRandF(0.2 (1 - c)); else c / 5 - (a +
    ccRandF(a c)), a = 0.1 + ccRandF(0.1) (beyond 1200 0.1 a); a
    quarter held; goldRotate on frames 1 mod 4
  volume 1: goldSpeed = maxSpd + zoom maxSpd c (zoom < 0: 1.1 - c),
    goldAccel c / 10; else q = (20 c)^2 / 400: maxSpd + zoom maxSpd q, q
  actMove(goldDirc, goldRotate, mdirc.z, 0.2, goldSpeed, goldAccel, 1)
```

`ccRandF(x, y)` (main 0x001d9b20) is `ccRandF(x)` held to `-y..y`. The
port is `enemy_ai::think_gold` and `enemy_motion::Motion::move_gold`.
`tools/test_battle_spawn_rs.py`'s `frame_enemies` now also spawns gold
goblins (rows 131-157 of each volume and type). Between frames it holds,
lets go, puts to sleep, paralyses, confuses and charms them, besides
the hits. Frames are compared with `game.inBattleDist` included: 200
cases, 0 mismatches. In a 60-case sample the goblins flee, chase, stop,
attack, flinch and die, and shake off holds (inBattleDist set to -2 and
2200). A gold goblin's walking dust is taken through `ccTransPosFW2LW`
(the probe had printed it raw).

## The events' battle instructions

The event scripts walk, place, list and hold characters through cases of
`ccEvent::Execute` (main 0x001a8d20; [event scripts](events.md), [the
interpreter](event-vm.md)). All run at level 2 only. Main addresses:

| case | instruction | at | what |
| ---: | --- | --- | --- |
| 13 | `remove 5\|6 code` | 0x001aa290 | `GetEnemy(code)` to `deleteEnemy` (gcmn 0x004312d0); -1 `deleteAllObject` (0x00430e70) |
| 61 | `pc_walk_pos pc x y z` | 0x001ad8ac | a remote walk to (10 x, 10 y, 10 z, 1), remote command 1 |
| 164 | `pc_run_pos pc x y z` | 0x001ad9b0 | the same, remote command 2 |
| 62 | `pc_walk_dir pc rot dist` | 0x001adab0 | a remote walk 10 `dist` from where it stands toward `rot` |
| 63 | `pc_walk_marker pc marker` | 0x001adbf4 | a remote walk to a marker |
| 64 | `pc_walk_char pc type code rot dist` | 0x001adda4 | a remote walk to a point near a character |
| 66 | `pc_command pc on` | 0x001ae164 | `ccEntryCmnd` (gcmn 0x00519630), or `ccDeleteCmnd` (0x00519700) for `on` 0 |
| 67 | `pc_put_marker pc marker` | 0x001ae7d8 | placed at a marker, facing its way |
| 68 | `party_put_marker pc marker` | 0x001ae9e8 | the companion that is not `pc`, the same |
| 69 | `party_put pc x y z` | 0x001aec5c | the companion that is not `pc` to (10 x, 10 y, 10 z, 1) |
| 70 | `pc_put pc x y z` | 0x001aeb84 | to (10 x, 10 y, 10 z, 1); nothing else |
| 71, 72 | `pc_turn`, `pc_face` | 0x001aed5c, 0x001aee30 | ([field game](field-game.md#the-party-under-the-event-scripts)) |
| 74 | `enemy_put enemy posnum` | 0x001af028 | an enemy to an event position |
| 132 | `hold type code` | 0x001b09fc | start `ccThEvHold` (0x001b5040) |
| 133 | `hold_end` | 0x001b0a68 | stop it |
| 153 | `player_skill` | 0x001b1f04 | Kite attacks the command target on the confirm button until it dies |
| 163 | `battle_ready` | 0x001b23a8 | `game.inBattleDist` (+0x60) = -1.0; `AddOperate(14, 0, 0)` |

Events 3 (TEACH-F) and 4 (TEACH-D) use `pc_walk_dir`, `pc_command`,
`hold`, `hold_end`, `player_skill` and `pc_put` from this table, and
`pc_turn`, `pc_face` and `pc_mode`. They use neither `remove`,
`enemy_put`, `battle_ready`, nor the other walks and puts.

### Who `pc` names

`ccSpcManager`'s registry (gcmn 0x00730340: `id` +0x00, `charPtr` +0x1c,
0x2c a slot, 5 slots) and `ccPartyManager` (0x00730310: `memberChar[3]`
+0x00, `memberID[3]` +0x0c, `num` +0x18) are read five ways:

```
GetSpc(code)     main 0x001b2bb0   code >= 0: the first slot whose id is code, its charPtr.
                                   code < 0: id = memberID[-code] (-3 reads num); id < 0: null;
                                   else the first slot of that id
named(pc)        walk_pos, run_pos, put, put_marker, turn, face
                                   pc >= 0: GetSpc(pc); else memberChar[-pc]
                                   (-3 reads memberID[0], Kite's 0, as the pointer: null)
registered(pc)   walk_dir, walk_marker, walk_char
                                   the first slot whose id equals pc (sign extended);
                                   a negative pc matches only a free slot
commanded(pc)    pc_command        pc >= 0: the first slot of id pc (null: nothing);
                                   -3: for each party slot with memberID >= 0, the first
                                   slot of that id (null: skipped); -1/-2: that slot's
companion(pc)    party_put, party_put_marker
                                   memberChar[1]; memberChar[2] if slot 1 is empty or
                                   its base id is pc; nothing if that one is pc too
```

Below -3 each of them reads past `ccParty` (no script does it).

`GetEnemy(code)` (main 0x001b2d50) walks the entry control's enemy list
for its count (`g_entCtrl` +0x10, head +0x14, `next` +0x1c4) and returns
the first object with base type & 0x60 and base id `code`. `GetNpc(code)`
(0x001b2cc0) walks the NPC list (+0x34, +0x38) with type bit 0x10 for codes
29 and 158, 0x08 otherwise. `pc_walk_char`, `pc_face` and the other type
operands pick by `1 << type` (the shift's low five bits): 4 `GetSpc`, 0x18
`GetNpc`, 0x60 `GetEnemy`, nothing else.

### The remote walks

Each walk runs, in order:

1. `ccSpcChar::ManualModeAI(1)` (gcmn 0x0059eba0). Unless the party is
   wiped out (`checkPartyAnnihilation`, 0x0059d080: every `memberChar`
   down, `dead` neither 0 nor 5, as many as `num`), a character with `dead`
   not 0 gets `dead` 0, `cloak` 1.0, `ghostFlag` (+0xe0 bit 6) off and its
   `bodyHit` into the collision (`HitEnable`, main 0x00153310). One in act
   9-11 gets act 2 and old act -1. Then `ccAI::ManualMode` (0x00583270):
   `manualSW` 1, `gDeg` = `RAD2DEG(dirc.z)`, `gRotSp` 64, `remoteCmd` 0.
2. `ccAI::SetRemoteCmd(cmd)` (0x005832e0): `remoteCmd`, `remoteFlag` 1.
3. `ccAI::SetGoalPos(v, 0)` (0x005833a0): `gPoint` -1, `gPos` = v.

`ManualControl` walks there from the next frame on. Remote command 1 moves
with `runFlag` set and 2 without it, so `pc_walk_pos` runs and `pc_run_pos`
walks; the names are the instruction set's. The goals, each lane an FPU
operation in this order:

```
pc_walk_pos     (10.0 x, 10.0 y, 10.0 z, 1.0)                  (10.0f * (float)v)
pc_walk_dir     a = DEG2RAD(rot), d = 10.0 * dist
                pos.x + d sinf(a), pos.y - d cosf(a), pos.z, pos.w      (its own position)
pc_walk_char    the same from the other character's position, its w kept
pc_walk_marker  a field or a dungeon (game.area 1, 2): the first of the 16 event positions
                whose number is marker (eventMng +0x1c0, 0x20 each, pos at +0x10);
                a Root Town (area 0): the marker's dummy, markerEvTbl[marker] (0x00317da0)
                found with GetChunkAdrsF in worldman +0x430's stream +0x1a4, pos at +0x10;
                w 1.0
```

A `pc_walk_char` whose other character is not found still does steps 1
and 2, with the goal left as it was. A `pc_walk_marker` with no such
event position (or in another area) walks to what Execute's stack frame
holds at +0x360.

### Placing

- `pc_put` and `party_put` write the position (+0x40) only: not the
  heading, `posP` or the ground.
- `pc_put_marker` works by area:
  - field or dungeon, `marker` >= 0: the first event position of that
    number gives the position; then `ccAI::SetDircZ(RAD2DEG(its dirc))`
    (gcmn 0x00581500: `gDeg` = dd, and the body's `dirc.z` =
    `DEG2RAD(dd)`). No such position: nothing.
  - field or dungeon, `marker` < 0: the position is copied, and `a` =
    `ccGetDircPL(position)` (main 0x001da610: the position in the player's
    frame (`ccTransPosW2P`), x and y times -1.0, `atan2f(y, x) + pi/2`,
    wrapped). x becomes -200.0 `sinf(a)` and y 200.0 `cosf(a)` (z and w
    kept), and the result goes back through `ccTransPosP2W`. The heading
    is not touched.
  - Root Town: the marker's dummy, its position and `SetDircZ(RAD2DEG(rot.z
    at +0x28))`.
  - Any other area: nothing.
- `party_put_marker` is the same for the companion, without the negative
  case: in a field or a dungeon any number, -1 included, is looked up.
- `SetDircZ` with no AI (null) writes through null; the character is not
  turned.
- `enemy_put`: `GetEnemy(enemy)` takes the first event position numbered
  `posnum`: its position, and `dirc` = (0, 0, its dirc, 1.0).

### The command lists and removal

`pc_command` calls `ccEntryCmnd` (the end of the list the base type names,
unless listed: `cmndPcLast`/`EneLast`/`ObjLast` 0x00378c4c/54/5c) or
`ccDeleteCmnd` (off the list; when that was `cmndTarget`,
`ccChangeCmndTarget(0)`: `cmndTargetPrev` = it, `cmndTarget` = 0) on each
character `commanded(pc)` names. `remove 5|6 code` passes `GetEnemy(code)`
to `deleteEnemy`; nothing when there is none.

### `hold` and `ccThEvHold`

`hold type code` checks `eventMng.holdTscb` (+0x7c0). If it is set, it
does nothing: a second `hold` is ignored, type and code included.
Otherwise it runs `ccStartThread(ccThEvHold, 33, 2048)` and stores the task
and its `type` and `code` (the tscb's +0x14 and +0x18). `hold_end` runs
`ccDeleteThread` on the task and clears `holdTscb`.

`ccThEvHold` (main 0x001b5040, "EVENT HOLD") names itself, reads type and
code, and then loops `Breath(1)` and a pass. Priority 33 puts it after
`ccThEvent` (32), `ccThGameCtrl` and any event camera task started before
it (both 33), and before `ccThMenu` (34), the camera (40), the party
(48-50) and the entry control (64). `GoThread` starts it without
preempting the event task, so in the frame of `hold` it only reaches its
first `Breath`. The first pass is the next frame. After `hold_end` it does
not pass again, that frame included (`Breath` sleeps while `del` is set).
A pass, by type:

```
5, 6   for the count g_entCtrl's enemy list has at the pass's start, from its head:
         base.type & (1 << type) and base.id == code:
           EntryAffect(obj, 0, 5, 0, 0, 0)       (gcmn 0x0056b020: off the command lists,
                                                  or a foe whose affectType is 13: nothing;
                                                  else condition.hold (+0x0a) = 1)
           code 130: obj.dirc = (0, 0, ccGetDirc(obj.pos, plw.pw.pos), 1.0)
                                                  (main 0x001d9ce0; turned at once)
7      eventMng.bossEntry (+0x08) == code, eventMng.bossTscb (+0x7bc) set and its +0x14
       not 0: EntryAffect(ccCheckTargetTypeId(0x80, code), 0, 5, 0, 0, 0)
       (gcmn 0x005199e0: the first character on the party's, then the foes', then the
       objects' command list, as the mask's bits 0x7, 0xe0 and the rest name them,
       whose base type has a bit of the mask and whose base id matches; none: nothing)
other  nothing
```

TEACH-F's `hold 5 130` holds every goblin of its four circles (`enemyTbl`
row 130, Goblin: base type 0x20, id 130) and keeps it facing Kite from the
frame after the instruction until `hold_end`, through the whole of
`player_skill` and the skill and chat-command lessons after it.

### `player_skill`

Set-up: `ccMenu.panelStatus` (+0x0c) = 1 (the party panels fade in),
`targetForbid` (+0x102) = 0, `cmndTargetFix` (gcmn 0x00378c6c) = 1, and
`cmndTarget` +0x150 = 0. On an entry object +0x150 is
`entParam.param[2]` (`entParam` is at +0x100), and an enemy with it 0
leaves no treasure box when it is taken away (`entryEnemyObject`, gcmn
0x00430250). The goblin of the lesson never drops one.

Then, while `cmndTarget`'s `condition.dead` (+0x08) is 0 (read again each
time, `cmndTarget` too):

```
ccBreathThread(1)                                  a frame
if ccSkillCheck(plw.pw) == 0:                      (gcmn 0x005723e0)
    if ccSys.pad[0].push (+0x2d0) & saveData.assignPADok (+0x840e, a short, sign extended):
        ccSkillRequest(plw.pw, cmndTarget, 1); ccMenu.plAttack (+0xf4) = 1
    else: plAttack = 0
```

The test comes first: a target already dead ends it at once, in the same
frame. The end: `cmndTargetFix` 0, `plAttack` 0, `panelStatus` and
`mapStatus` (+0x10) 3 (fading out). With `cmndTarget` null the game writes
through null at the set-up.

### `battle_ready`

`ccThGameCtrl` (gcmn 0x00517c54) reads `inBattleDist` while an enemy is on
a list. Below 0 it sets `inBattle` outright on fields 1-12 (`game.field`,
+0x24). Otherwise -2.0 sets it anywhere, and any other value is the radius
of `ccCheckInAreaCmnd` around each party member. So -1.0 means a fight in a
story field and none elsewhere. The interpreter makes the `AddOperate`.

### The party's `bootParam` from area to area

`pc_mode -3 4` in TEACH-F's set-up pass writes `bootParam` 4 into the
registry slot of every party member: Kite and Orca (case 65, main
0x001adf74; the stores at 0x001adfdc, 0x001ae084 and 0x001ae134). Every
area's set-up rebuilds them from the registry (`rebootSpcManager`, called
unconditionally by `ccSetupGameCtrl` at main 0x001694c8; `ccSPC::Reboot`
gcmn 0x005a00d0). The constructors read `bootParam` through
`CheckBootStatus` (0x0059e920) into `SetBootStatus` (0x0059e950). With bit 2
they skip `ccEntryCmnd`, and Reboot gives the new `ccAI` `ManualMode`
(0x005a0268). So both start each area under manual control (remote command
0) and off the command list.

Its writers:

- `ccSPC::Initialise` (0x0059f5f0) writes 0. `ccSetupNewGame` (main
  0x001687a0) calls it on every entry to The World from the TOPPAGE
  (request 5).
- `ccThSpcDelete` (0x005a06d0), the delete hook `ccThSpc` (0x005a0530)
  puts on its own task, writes 0 into all five slots. `ccThControl` (main
  0x0015a6a0) runs a task's hook as it takes the task off the thread
  list, and the next area's `ccSetupGameCtrl` deletes every task
  (`ccDeleteAllThread`) before its event passes.
- `DelSpc` (0x0059f850) writes 0 when a slot is freed (a member's task
  deleted).
- `ccRegisterEventMng` (main 0x001b6ec8) writes an event `entry`'s `param`
  for a party character.
- `pc_mode`.

So a `pc_mode` holds for the one area whose passes wrote it:
TEACH-F's `bootParam` 4 is gone by the dungeon's set-up, and Kite and
Orca arrive there on the command list and under the AI. TEACH-D ends with
`mode 3`, back to the desktop.

`manualSW` goes off only through these:

- a new `ccAI` (its constructor 0x0057c5f0);
- `pc_act` 0 or 1 (Execute 0x001ad190-0x001ad6d8);
- `MenuClr` (main 0x001b25c0, `menu_clear`): outside a Root Town it
  switches manual control off for every party slot. It also puts every
  registered character back on the command list unless in act 14;
- `ChatMenuT` (gcmn 0x0056a0e4) and `FountainMenu3` (0x00549904).

In field 14 TEACH-F's `menu_clear`s release both characters: at the end of
block 5, and after talking to Orca (blocks 3, 4). In the dungeon, event
point 4 is the entrance room: `D0001_room` floor 0 room 0 has `eventFlag`
4, which `WORLD_MAN::SetEventData` (main 0x001a3c40) makes an event point.
So TEACH-D's block 4 plays on arrival. Its `menu_ban` takes both off the
command list and its `pc_command 0 1` puts Kite back on it before the
treasure-box lesson; its `menu_clear` frees both. (Read from the code and
the tables. `SetEventData` is checked in the interpreter:
[dungeon.md](dungeon.md#the-doors-and-the-events).)

### The party's HP and conditions from scene to scene

Every scene change builds the party again (`rebootSpcManager`), and
`ccChar::SetBaseParam(ccSpcParam *)` (gcmn 0x0056b690) gives a new
character full HP and SP, clears its conditions and its `ccSpcParam`'s
`tune`, `temp` and `time`. What carries the party's state over is
`storeCondition[18]` (gcmn 0x0072fb00, `ccStoreCondition`, 0x70 bytes, by
`charTbl` row, common.cpp):

```text
+0x00 condition (16 shorts)   +0x20 speedValue      +0x24 ccSpcParam.temp
+0x44 ccSpcParam.time         +0x64 HP  +0x66 SP    +0x68 maxHP  +0x6a maxSP
+0x6c conditionNum
```

- **Store.** `ccStoreSpcCondition()` (0x0056cc40) runs
  `ccStoreSpcConditionOne` (0x0056ccd0) over every character the registry
  has built (`ccSPC::CheckSpc` for ids 0-17, `charPtr` set). The next
  scene's `ccSetupGameCtrl` calls it before it deletes the old scene's
  tasks. `ccEvent::MenuBan` and `FountainMenu3` call it as well.
- **Restore.** `ccRestoreSpcCondition(ch)` (0x0056cfc0) copies it back.
  `ccPlayer::ccPlayer` (0x00597a08) and `ccFellow::Initialize`
  (0x0041af2c) call it right after `SetBaseParam`, for a character in the
  party (`partyFlag` 1); `MenuClr` and `FountainMenu3` do too.
- **When the restore is skipped.** It does nothing in a town
  (`game.area` 0: the party arrives whole), when no area came before
  (`game.areaPrev` -1: a log-in), and in field 13 while `eventStatus[39]`
  (`saveData` +0x651f) is 1.
- **A dead character.** Both constructors check HP after the restore. A
  character with none stands as a ghost: SP 0, `dead` 4, `ghostFlag`,
  cloak 0.5. It is the state act 10 leaves a dead Kite in, and he can
  walk until someone revives him.

The port keeps the table in `party::Spcs::store`, which travels with the
registry from scene to scene. `FieldWorld::store_conditions` and
`World::store_conditions` store the built party as the session leaves the
scene. `party::restores_condition` makes the test, and `Combat`'s
`add_kite` and `add_member` restore through `Combat::restore`.
`piney-game`'s `the_party_keeps_its_hp_through_a_door` takes Kite
through a door in area 26's dungeon: at 17 HP he is still at 17; with no
HP he comes back a ghost. Before this, every door healed the party
whole. `MenuBan`/`MenuClr`'s store and restore around the events are
ported (worklog 0135), and so is `ccClearSpcCondition` (0x0056d300),
which a boss's death calls (Skeith's `ccBoss::Affect`, when a hit takes
its HP to 0): `ccSpcConditionEffectOFF`, then for each character built
`ManualModeAI(1)` and `SetRemoteCmd(0)` (the revival of the fallen taken
back: `condition.dead` is saved round the calls), `noDeathFlag` on,
`ccDeleteCmnd`, `ClearCondition` and `ccClearParamElement` on the
record's +0x88 and +0xa8 (`FieldWorld::clear_spc_condition`;
`event_30_ends_with_skeith` poisons Kite and raises a stat in his record
before the end, and sees both gone at the death).

The party's records are the save's (`ccChar` +4 points at
`saveData.spcParam[id]`); the port's battle keeps copies. Each frame ends
by storing them in the save (`Combat::store_records`), and so does
anything that changes them between frames. A frame starts by reading
back each record the save no longer holds as the last store left it (a
menu wrote it: Equipment, a book read, an item given); the others stay
as the world left them.

### In the port

`crates/piney-battle/src/evparty.rs`:

- `Roster` is the registry and the party as scene indices. `EvParty`
  borrows the scene, the crew (the AIs, and each `Spc`'s heading and
  `bodyHit`), the entry control, the enemies' state, the world
  (`hit_switch`, `w2p`, `p2w`), `game.area`, the event positions and the
  town's markers. It has one method per instruction above, and
  `manual_mode_ai`, `set_remote_cmd`, `set_goal_pos`, `set_dirc_z`.
- `hold`, `hold_end` and `hold_frame` (a `Hold`, armed after its first
  frame) cover the task; `player_skill_begin`, `_busy`, `_step`, `_apply`
  and `_end` cover `player_skill`.
- `get_enemy`, `get_npc`, `check_target_type_id`, `get_dirc_pl`,
  `remove_enemy` and `battle_ready` are free functions.

The act, the ghost flag, `cloak` and `dead` are written to the `Char`, and
the `Spc` copies of the act and the ghost flag with them.

`pc_act` and `pc_mode` are not in this crate: piney-world ports them over
its own characters. `menu_ban`'s and `menu_clear`'s condition handling
(`ccStoreSpcCondition`, `ClearCondition`, `ccRestoreSpcCondition`,
`ConditionAdjustment`) is `Combat::menu_ban_conditions` and
`menu_clear_conditions` there.

`tools/test_battle_event_rs.py` runs each instruction through the game's own
`Execute` at level 2 on a short array, over random worlds. A world has up
to ten characters, the registry and the party with free slots, repeated
ids and null characters, the command lists, the entry control's lists, 16
event positions, 33 town markers, and a player's frame moved by an offset.
The same case goes through `battle_probe`, and the check compares every
field above of every character, its AI's flags, `remoteCmd`, `gDeg`,
`gRotSp`, `gPoint` and `gPos`, the three lists in order, `cmndTarget`, and
the calls in order.

`ccThEvHold` runs from its start for 20-120 frames of types 3, 5, 6 and 7.
Between frames holds are cleared, positions moved, `affectType` set to 13,
foes taken off and put back on their list, and the enemy list reordered.
`EntryAffect`, `ccCheckTarget`, `ccGetDirc` and `ccCheckTargetTypeId` run
natively. `player_skill` runs for 1-60 frames with the pad, `ccSkillCheck`'s
answer, the target's death and `cmndTarget` changing. Results:

| check | cases | mismatches |
| --- | ---: | ---: |
| `pc_command` | 5,000 | 0 |
| `pc_walk_pos`, `pc_run_pos` | 5,000 | 0 |
| `pc_walk_dir` | 5,000 | 0 |
| `pc_walk_marker` | 5,000 | 0 |
| `pc_walk_char` | 5,000 | 0 |
| `pc_put`, `party_put` | 5,000 | 0 |
| `pc_put_marker`, `party_put_marker` | 5,000 | 0 |
| `pc_turn`, `pc_face` | 5,000 | 0 |
| `enemy_put` | 5,000 | 0 |
| `remove` (whom `deleteEnemy` gets) | 5,000 | 0 |
| `hold`, `hold_end` | 5,000 | 0 |
| `ccThEvHold` | 5,000 runs, 306,500 frames | 0 |
| `player_skill` | 5,000 runs, 23,198 frames | 0 |
| `battle_ready` | 5,000 | 0 |

### Unknown

- `pc_walk_marker` with no event position of that number, or outside areas
  0-2, walks to whatever Execute's stack frame holds (+0x360). The port
  takes (0, 0, 0, 1). The checks do not make these cases.
- What `eventMng.bossTscb`'s +0x14 counts. `hold 7` holds the boss only
  while it is not 0, and the `present` condition treats 0 as "the boss is
  there".
- The deletion `remove 5|6` hands over is `EntryCtrl::delete_enemy`,
  checked by `tools/test_battle_spawn_rs.py`. `remove -1`
  (`deleteAllObject`) is not ported here.

## Drawing the enemies and the portals

What an enemy draws with is fixed when the area loads its files and the
race's constructor makes it; `dispEnemy` then hands `ccChar::Draw` the
matrix each frame.

**The files.** `ccAddRequestFileListEntry` (gcmn 0x0042f5a0) loads the
`fileList` (category 13) of every row with `entry.exist` set. A middle boss
(`base.type` 0x40; its base form's row in `base.gold`) first takes that
row's `anm` table, `clut` and `fileList` into its own entry, and loads a
second file, the name with its fourth letter made `X`. Then
`ccEntryCtrl::initEntryCCS` (0x0042feb0) sets `entry.ccsc` to
`ccStream::GetCCSAdrs` of the name cut at its first '.' and lower-cased:
`EGN1` is the `DATA.BIN` member `egn1`. A middle boss's `ccsc2` is the `X`
file (row 72: `ebq1` and `ebqx`). So in the game a middle boss's
constructor finds its base form's animation table in its entry, not the
empty one of its own row.

**The gimmicks registered.** `initEntryCCS` does the same for each
`gimmickTbl` row whose +0x28 is set, into the row's +0x38.
`ccRegisterGimmick(row)` (gcmn 0x0042f400) sets it and adds the row to
`ccRegisterGimmickTbl` (a list only it reads, to skip a row twice);
`ccInitRegisterGimmick` clears both. `ccRegisterEventMng` calls
`ccRegisterRegularGimmick` (main 0x001b71b0), which registers the regular
rows of the place `game+0x14` names:

- 0: row 16 only;
- 1, a field: 0, 1, 6, 15, 18, 20, 21, and by field type (the jump table
  at 0x00356350) one of 23-26 and 27-32 (none for type 4);
- 2, a dungeon: 0-6, 15, 17, 19, 22; by dungeon type (0x00356320) 34, 7,
  8, 10, 12 (types 0 and 4), 35, 7, 8, 9, 12 (1, 5), 36, 9, 10, 11, 12
  (2, 6), 37, 11, 12, 13, 14 (3, 7) or 33 (8, 9); then 44 when
  `worldman+0x134` is set, else by `GetFieldAttrb` (0x00356300) 42, 38,
  39, 40, 41 or 43; and 20 when the field type is 4.

`entryGimmick` does not look at the flag: the row's constructor finds its
models through +0x38, so a gimmick of a row the place did not register
would have no file. The event scripts only make the place's own. The
port reads a gimmick's file when one is made, so it keeps no list.

**The constructor's models.** `ccEnemy::initEnemyCCS` (0x00432fc0), from
`initEnemy`:

- the clump `CMP_trall` of `ccsc` (`ccClump::Init`);
- `ccEntryChangeCLUT` (0x0042e670): with an `entry.clut`, the clump is
  duplicated (`ccClump::Duplicate(0x2000)`) and `ccClump::ChangeClut(clut,
  MAT_clut)` (main 0x0013d610) points every node model's material
  `MAT_clut` at that palette;
- a `ccAnm` on the clump playing `anmTbl[6]` (the wait), its notes going to
  `ccEnemyCheckNote`;
- a middle boss: `CMP_trall` of `ccsc2` with a second `ccAnm` playing the
  same name with its eighth letter `x` (`ANM_ebq1nut0`, `ANM_ebqxnut0`),
  its shadow off, the enemy fully opaque.

`ccEnemyG::ccEnemyG` (0x00446070) adds `ChangeClut(CLT_egmfrod1c1,
MAT_clut2)` for rows 154-157. Nothing else is hung on a node: the weapon
trail and dust controllers are effects. The first field's and dungeon's
rows (measured by running the constructors):

| row | file | palette | wait |
| --- | --- | --- | --- |
| 67 Chicken Hand | `ebl1` | | `ANM_ebl1nut0` |
| 130 Goblin | `egn1` | | `ANM_egn1nut0` |
| 151 Magical Goblin | `egmf` | | `ANM_egmfnut0` |
| 185 Disco Knife | `ekd1` | | `ANM_ekd1nut0` |
| 189 Sword of Chaos | `eks1` | `MAT_clut`: `CLT_eks1bodyc2` | `ANM_eks1nut0` |
| 219 Mad Grass | `epg1` | | `ANM_epg1nut0` |
| 268 Deadly Moth | `evm1` | | `ANM_evm1nut0` |

In the files of these rows, of rows 154 and 187 and of the portal, only
the swapped material uses its texture, so a swap of the texture's own
palette is a swap by material.

**The pose.** `dispEnemy` stores its matrix into the anm (`ccAnm` +0x40,
`matCalcSW` set) as `SetMatrix_PosRotZYX` would. `ccAnm::SetAnm` puts each
object of the animation under its own ExtObj parent (what that copy
drives; the first under the anm), which is not always the clump's Obj
parent: in `ebl1` the dummies 05, 11, 12 and 13 hang from other nodes in
its animations than in its clump (`OBJ_dummy05` from the head, not the
tail), and are drawn where the animation puts them. A clump node no animation
drives keeps the anm's matrix (`eks1`'s `OBJ_trall`, a small rigid model;
*inferred*, not run). `ccAnm::Draw` (main 0x001524d0) draws each node with
`ccObj::Draw(anm transparency)` (0x0013f220), which multiplies in the
node's `localtp` and its parents' (`ccCoord::_GetTransparency`, 0x00138490)
and draws nothing at or below 1/128.

**`ccChar::Draw`** (gcmn 0x0056b1c0):

1. `WORLD_MAN::SetActiveLayer(5)` (+0x4cc, priority 10). The layers by
   number: 0 `sysLayer` (0), 1 +0x4bc (0), 2 +0x4c0 (-10), 3 +0x4c8 (20),
   4 +0x4c4 (-20), 5 +0x4cc (10), 6 +0x4d0 (30).
2. The fog blend, `ccDrawEnv::SetFogBlend(rate, colour)` (main
   0x001057f0), by `condition.dead`:
   - 0 and 1: while `affectColorRate` is not 0 (an affect's flash), the
     rate moves by `affectColorCnt`; at or below 0 the rate and
     `affectColor` are cleared; the blend is (rate, `affectColor`).
     Otherwise `conditionColor` is cleared and, with a `conditionNum` of
     0-6 and the condition effects on (`spcConditionEffectFlag`, main
     0x00378ce4: set by `ccSPC::Initialise` and `ccThSpc`, cleared while an
     event bans the menus, `ccEvent::MenuBan`, and at a fountain), taken
     from the jump table at gcmn 0x006e10d0: 0x7000e0,
     0xe0e0, none, 0xb070e0, 0xe0, 0xe0e0, 0x100010 (red in the low byte).
     With a tint, `conditionColorRate` moves by `conditionColorCnt`: from
     60 it is held at 60 and the count becomes -2, from 0 or below it is
     0 and the count 12; the blend is (rate, tint).

**Which condition shows.** `conditionNum` (+0x30) is set by
`ccChar::DispConditionEffect` (gcmn 0x0056f950), which the characters'
own frames call after their condition counts (Kite's `ccPlayer::Main`,
`ccFellow::Main`, the enemies' `main`, outside the Root Town), and
`ConditionBattleEffect` after a hit:

```text
dead other than 0 and 1        an effect (+0x2c) killed, num -1
a party character (type & 4)   ccSpcConditionEffectSW() off, or Kite
                               (base id 0) in the eye view: an effect
                               deleted, num -1 (from Mutation on num -1
                               with no effect too)
otherwise, in this order, each active condition: if it is the current
num, it stays (its effect kept, or remade if the effect's own number
+0x1c differs, or made if there is none); else the first active one is
the new num:
  paralysis 1, sleep 5, confusion 4, charm 3,
  speed 22 (speedValue above 1) or 2 (below),
  the record's stat changes with a timer (temp/time: pAtk pDef pHit
    mAtk mDef mHit, the six elements): 23-34 raised, 7-18 lowered,
  HP regain 20, SP regain 21, poison 0, curse 6
none active: an effect killed (only when num was not -1), num -1
a new num: the old effect killed (again only when num was not -1: one
  left over is left running), setConditionEffect(ch)
```

`ccChar::ClearConditionEffect` (0x00570180) deletes the effect and sets
num -1 (a death, a Data Drain's end).

The port: piney-battle's `chara::disp_condition_effect` (checked by
`tools/test_battle_rs.py`'s `disp_condition`, 1,000 random characters,
effects and switches against the game's own function), run by the combat
on the characters' `DispCondition` rules (`Combat::disp_condition`), which
keeps each character's effect number; piney-game's effects hold the
`ccConditionEffect`s. `a_poisoned_kite_shows_it` poisons Kite in event 3's
field: num 0 and the particles, and -1 once cured. Before this nothing
set `conditionNum` in a fight, so neither the tints nor the condition
effects showed.
   - 4: (60, 0xc0c060). 5: no blend, the flash cleared.
   - 2, 3 and anything else: (60, black).

   `affectColorFix` (+0xa6), which would hold the flash, is set only by
   `ccRestoreSpcCondition` (0x0056cfc0) and is 0 for an enemy.
3. `hide` starts set when `transDist` (+0x90) is 0 or in the eye view
   (`checkCameraType() == 1`). The transparency is `setTransparency` times
   `ccGetCameraTransparency(pos, width, height, far, fade, &hide)` (main
   0x001da8b0), a town (`game.area` 0) 4000 and 400, a field or dungeon
   7000 and 600, both points taken into the player's frame
   (`ccTransPosW2P`). It becomes the anm's.
4. The shadow's alpha, `(fptosi(256 t) + 1) >> 1` of `setTransparency`
   when `hide`, else of the transparency (`ccDrawEnv::shadowAlpha`).
5. Under 0.05 without `hide`: nothing is drawn. Otherwise
   `shadowLength` is three times the height, and `ccAnm::Draw` runs, with
   `WORLD_MAN::SleepDistantLight` around it on shaded ground
   (`hitAttribute` 0x40000; the ambient saved and put back).
6. `shadowAlpha` 128, `ResetFogBlend`, `SetActiveLayer(0)`.

`ccSetMatrixPacket` (main 0x0013e350) turns a blend into each model's fog
through `ccSetFogBlendColor` (0x0013df20). A model whose fog is on (FGE:
`ccObj::Init` sets every node's fog switch, 0x0013b7b4) gets the
coefficient `2.55 (100 - rate)` and the blend's colour, and its fog is
forced on for the draw: rate 60 gives 101, the EE's product falling just
short of 102. A model whose fog is off mixes the blend with the draw
environment's fog by its depth instead. A rate of 0 draws as without a
blend. The shadow model (`ccShadowModel::Draw`) takes `shadowAlpha` and
`shadowLength`.

**The portal.** `gimmickTbl[15]` (gcmn 0x0061e0f0, 0x70 bytes a row) loads
`XMAGCIR.CCS` (`xmagcir`, `DATA.BIN` member 623) with the palette
`CLT_x031c2`. `ccMagicCircle::ccMagicCircle` makes the clump `CMP_xmagcir0`
with its fog off (`ccClump::SetFogSw(0)`), the entry's `ccEntryChangeCLUT`
(`MAT_clut`), and a `ccAnm` playing `ANM_xmagcir1`; `main` changes to
`ANM_xmagcir2`. It draws on layer 3 (priority 20): `SetMatrix_PosRotZYX(pos,
dirc)`, the anm's transparency the camera's (`ccGetCameraTransparency(pos,
0, 0, 7000, 600)`, `hide` clear) times `setTransparency`, then `ccAnm::Draw`;
its animations fade their objects (`localtp`). The particles' effect
`EFF_xmagpat1` has 111 patterns (`patNum`, the Eff chunk's halfword at +14
of its payload, `ccStream::Decode_Eff` main 0x0014cca0), which the
circle's particles step through.

**In the port.** `piney-world`'s `foe` module: `EnemyLook::load` (the
files, clump, swaps, animation table and a middle boss's second model of a
row), `Model::worlds` and `node_alphas` (the pose with the animation's
parents), `char_draw` (`ccChar::Draw` up to the draw, moving the affect
members on), `EnemyLook::draw` and `draw_second` (the field's cast draws
a middle boss's second model where `dispEnemy` asks, `Call::DrawSecond`,
before its body), `Camera` (the
transparency in the field's frame), `Circle` (the portal's model, `patNum`
and draw). `tools/test_foe_rs.py` checks them against the game's code in
the interpreter (the constructors' world is `test_battle_spawn_rs.py`'s):

| check | what runs natively | cases | mismatches |
| --- | --- | ---: | ---: |
| looks | `ccAddRequestFileListEntry`, `initEntryCCS`, the B, G, K, P, V constructors: the files, clumps, palette swaps, animations, a middle boss's second file and animation | 24 rows (field 14's, 154-157, first forms, middle bosses 72, 76, 164, 165, 224, 225) | 0 |
| pose | the rows' animations compiled and played (`ConvLCNum2ALCNum`, `_AnimateForward`), the anm at a `dispEnemy` matrix, `ccObj::Draw` of every node: its world matrix and alpha | 994 frames, 17,362 nodes | 0 (rotation within 1.9e-5, position 2.3e-3 over the float step) |
| circle | `CMP_xmagcir0` under `ANM_xmagcir1`/`2`, `SetMatrix_PosRotZYX` | 1,000 frames, 4,350 nodes | 0 (root exact; alpha within 6e-8) |
| draw | `ccChar::Draw` with `ccGetCameraTransparency` (random state, camera, area; `ccAnm::Draw` and the lights recorded): the blend, transparency, whether it draws, the shade, the shadow's alpha and length when drawn, the affect members after | 1,000 (449 drawn) | 0 |

The shadow model is drawn as [the shadow volumes](shadow.md) set out, with
the alpha and length above. Not drawn by the port: the area's fog on
characters (as for Kite).

### The weapon trails and flashes

A race's constructor makes a `ccEnemyWeaponCtrl` over its type's
`ccEnemyWpInfo` rows ([the race constructors](#the-race-constructors)).
The race's `exclusive()` runs it every frame, and its `note()` hands it
every note. Each row is one weapon: a trail of cells swept by up to four
points on a node while the enemy attacks, and a flash (`ccEnemyWeaponRad`,
a [`ccPrimRadiate`](#the-dungeons-objects) plate) where a hit lands.

```text
ccEnemyWpInfo (0x60)  +0 chunk name (one row's is set), +4 node name,
                      +8 flags, +0xc the flash's scale, +0x10 tension,
                      +0x14 cells between two (u16), +0x16 life (u16),
                      +0x18 edges (u16, 1-4), +0x20 the edges' points (vec4 x 4)
flags                 1 rgb colours (attrRgbTable 0x005d58e0), 2 hsv colours
                      (attrHsvTable 0x005d5900), 4 the strip packet, 8 the
                      line packet, 0x10 the flash, 0x20 bezier cells, 0x40
                      spline cells, 0x80 << atkNum the attacks it shows on
ccEnemyWeapon (0x74)  +0 bits: 0 the node is the animation's, 1 a flash
                      asked for, 2 the flash follows the node; +4 flags;
                      +8 element; +0xc cells; +0xe edges; +0x10 each edge's
                      share; +0x20 the cells; +0x24 newest, +0x28 oldest,
                      +0x2c live; +0x30 info; +0x34 enemy; +0x38 chunk;
                      +0x3c node (ccCoord); +0x40 the flash's life; +0x44 the
                      flash; +0x48, +0x5c the two ccPrimPackets; +0x70 next
ccEnemyCell (0xd0)    +0 bit 0 put between two; +2 life; +4 alpha; +8 newer;
                      +0xc older; +0x10 + 0x30 e the edge's ccPrimVert (+0
                      bit 0 on the screen this frame, +4 RGBA, +0x10 place,
                      +0x20 its screen place)
```

| function | gcmn | what |
| --- | --- | --- |
| `ccEnemyWeaponCtrl(n, info, obj)` | 0x0043e760 | a `new ccEnemyWeapon(info + 0x60 i, obj)` a row, chained through +0x70 |
| `ccEnemyWeapon(info, obj)` | 0x0043c970 | cells: 1 for a life below 2, else life x (between + 1); `setEdgeRate`; `init(0)`; the node: a name whose part before the first `_` is `EXT` (`ccCheckNameExtObject` 0x0042e5d0) is `ccAnm::GetSubstAdrsF` of the enemy's animation, looked up again each frame; any other `ccClump::GetObjAdrsF`; a flash from `wpRadInfo` (0x005d59b0: type 5, a plate facing the camera, 8 rays) |
| `setEdgeRate` | 0x0043ce60 | each edge's distance from the first along the axis on which the last is farthest, over that distance, in doubles (`fptodp`, `fabs`, `dpdiv`, `dptofp`) |
| `ctrl(obj)` | 0x0043e900 | each weapon: `ctrlCell`; with `dispSW`: when `checkWeapon(actNum, atkNum)`, at `actCnt` 0 `init(ccSkillCheckTypeAttribute(skillParam->type))`, then `setCell`; `dispCell`; always `ctrlRadiate` |
| `note(obj, note)` | 0x0043eae0 | `atkNum` 0, 1, 4, 5 and note 0x8005: weapon `param` (1 at least, the count at most) flashes for 20 frames; `atkNum` 2, 3 and 0x8003: every weapon for 80, following the node; only where `checkWeapon` holds |
| `checkWeapon(act, atk)` | 0x0043e680 | act 6 and flag `0x80 << atk` (atk 0-5) |
| `getNewCell` | 0x0043d2c0 | the first cell with life 0 (one more live), else the oldest taken off the list; life 0, alpha 1, colours afresh (`initCellColorHSV`), unlinked |
| `initCellColorHSV(cell)` | 0x0043d190 | each edge: flag 1 `attrRgbTable[attr]`, else flag 2 `ccFractionalHsv(attrHsvTable[attr] \| 0x80000000, rate, 0.7 rate)` |
| `setCell` | 0x0043db50 | a new cell at the front, life the row's, each edge's point under the node's matrix (the unit matrix without a node); bezier or spline rows with 4 live `interpolateCell`; with flag 0x10 and a flash asked for, the flash starts (`init`, at the last edge or the middle of the last two, the element, the flash's life, the row's scale) |
| `interpolateCell` | 0x0043d3b0 | `between` new cells between the second and third newest at t = i / (between + 1), life the second's less t of the difference (at least 1), each edge a bezier over the four newest (`((a u^3 + b 3u^2 t) + c 3u t^2) + d t^3`, as the FPU's accumulator adds) or `eneSplineH` (0x004382a0, a Hermite curve from the second to the third, tangents `(c - a)(1 - tension)/2`, `(d - b)(1 - tension)/2`), the four cells' points put into Kite's frame first |
| `ctrlCell` | 0x0043de90 | each live cell from the newest: alpha less 0.4 / life (not below 0), each edge's alpha byte times it (truncated, so it compounds), life less 1; at 0 off the list |
| `dispCell` | 0x0043e110 | with more than one cell listed: each point's screen flag cleared, each packet's buffer flipped and its work taken (`GetWork`, 6 + 4 per pair qwords); on layer 6, for each pair of edges each live cell from the newest `makePacketCell(v, k != 0)`; `sendPacket` the lines, then the strips |
| `makePacketCell(v, cont)` | 0x0043d720 | edges e and e + 1: each the first time this frame `ccTransPosFW2LW` in place and `sceVu0RotTransPers` on `ccLayer::active`'s view; the strip packet as they are, the line packet with the alphas 1.1 times (held to 0-255) |
| `ctrlRadiate` | 0x0043e3f0 | a skill's flash moved to the node's edges each frame; while it shines `ccEnemyWeaponRad::ctrl`, `create`, `disp` |
| `ccEnemyWeaponRad::ctrl` | 0x0043c600 | act 0: colours `ccFractionalHsv(hsv, 1, 0.7)` inside and `(hsv, 0, 0)` outside, length and zoom 0, width 2 pi / pnum; act 1: 0.24 a frame about x, alpha `sin p`, the light 1.5 `sin p` in `cc3d`'s group at its place in Kite's frame, length 40 `sin p`, width 20 `sin p`, zoom 10 `sin p`, p on by pi / life; after life + 2 frames act 2 puts it out |

`ccPrimRadiate` (gcmn prim.cpp, 0x00438ac0-0x0043a248) builds rays of
light round a point each frame and draws them as triangle strips on layer
6, with a `ccOmniLight` in the scene's light group while it shines:

| function | gcmn | what |
| --- | --- | --- |
| `ccPrimRadiate(inf, chr)` | 0x00438ac0 | every member 0; user `chr`, info `inf`; `init`; `pnum` `ccPrimPart`s (`new[]`; the centre's too for type bit 0x10); a `ccOmniLight`; `init` again |
| `init` | 0x00438d10 | pos (0, 0, 0, 1), rot 0; type, `pnum`, centre, length, width, zoom and the colours from the info; angle 2 pi / `pnum`; bank 0; scale, lscale, alpha 1; `dpLength`, `dpBank` 0; `radFlag`, `lgtFlag`, life, act, count, param 0 |
| `setColorRadiate(c0, c1)` | 0x00438e30 | (no centre part) each ray's inner points c0, outer c1; the light's reach (0, 1000, 1e6) and colour (c0's RGB) |
| `main` | 0x0043a1c0 | with `radFlag`: the class's `ctrl` (vtable +8), `create`, `disp` of the rays (and of the centre) |
| `create` | 0x004393a0 | the turn (below); m = RotZ RotY RotX of the unit; with parts, type bit 4 `createPlate(m)`, else bit 8 `createRing(m)` |
| `createPlate(m)` | 0x00439520 | ray i: RotX(angle i) under m at pos; inner (0, -+w, c), outer (0, -+w zoom, c + l), l drawn afresh a ray with `dpLength` (l + ccRandF(l dp)); the outer points turned by bank (+ ccRandF(bank dpBank)) unless facing; each ray's alpha |
| `disp(part)` | 0x00439df0 | each point into Kite's frame (in place) and through the view: a strip of two triangles a ray, on layer 6 |

Only the plate (type bit 4) is ported: the ring (bit 8) and the centre part
(bit 0x10) belong to rays nothing the port makes uses.

`ccPrimRadiate::create` (0x004393a0) turns a plate that faces the camera
(type bit 1): y `ccPiLimit(-cam.x)`, z `ccPiLimit(pi/2 + cam.z)`, from
`cameraGetRot2(camID)` in the eye view (`checkCameraType()` 1), else
`cameraGetRot(camID)`.

`ccPrimPacket::makePacket` (0x00438580) adds a pair to the packet: each
point's colour (`[r, g, b, a]`) and screen place with ADC set when the pair
does not continue the strip. A pair does not continue when it starts an
edge's run, when either point is behind the eye, when both points are more
than 5632 (352 pixels) across from the screen's centre (0x8000), or when
both are more than 4608 (288 pixels) down from it. The depth sort takes
the points' mean z. `sendPacket` (0x00438820) sends nothing under 2 pairs.
It sends a GIF tag of `cczEffGSprim` (TEST 0x73001, ZBUF no write, ALPHA
0x44) and `cczGsPrimPoly` (PRIM 0x4c, a gouraud blended strip) or
`cczGsPrimLine` (PRIM 0x49, lines), each vertex RGBAQ and XYZ2.

**No random draws.** Nothing here calls `rand()` or `ccRand()`. The
flash's plate has no `dpLength` or `dpBank`, so `createPlate` draws no
`ccRandF`.

**The one-edge share.** A one-edge row divides 0 by 0 in `setEdgeRate`.
libgcc's soft float (built without NaNs) packs that result with an
exponent `__unpack_d` never set, so it is whatever the stack held. Such
rows never draw a trail, and nothing else reads the share.

Every race's constructor names its tables (`races.rs` holds which table
each row takes: 97 tables, 225 rows). One row names a chunk: 0x005db9c0's
third, `DMY_xdummy_w05` under `OBJ_t0`, a flash. With a chunk (`ccStream::
GetChunkAdrsF(name, 1)` at construction), `setCell` and `ctrlRadiate`
turn the node's matrix by the chunk's turn (+0x20, `sceVu0MulMatrix(m, m,
RotMatrix(unit, rot))`). Then its last row becomes the chunk's place
(+0x10) under the node: a copy of `ApplyMatrix(m, pos)`, meant for a
spare vector, lands at +0x30 of the matrix. The port reads the place and
turn from the enemy file's dummy, turning the degrees into radians and
giving the place w 1. The first five races' rows:

| race, type | table | edges, life, between | flags | nodes |
| --- | --- | --- | --- | --- |
| B 0-2 | 0x005ddd30-0x005ddfd0 | 1, 1, 0 | flash | `OBJ_ebx1bird`, toes, head, dummies |
| B 3 | 0x005de030 | 4, 16, 4 | spline, strips, flash | `OBJ_r_wingdummy` and 5 flash rows |
| G 0 | 0x005e1bf0 | 1, 1, 0 | flash | `OBJ_dummy06`, `OBJ_dummy12` |
| G 1-4 | 0x005e1cb0-0x005e1dd0 | 4, 16, 1 | spline, strips, flash, every attack | the sword, club, sword and rod (`OBJ_egn1swor` ...) |
| G 5 | 0x005e1e30 | 4, 16, 2 | spline, both packets, flash | `OBJ_hammer` |
| G 6 | 0x005e1e90 | 4, 16, 2 | spline, strips, flash | both hands, and a toe's flash |
| K 0-3 | 0x005e3540-0x005e3780 | 3-4, 10-16, 1-4 | spline, strips (and lines), flash | bodies, `OBJ_egkswo1`, hands |
| P 0-3 | 0x005e6200-0x005e63e0 | 1, 1, 0 | flash | bodies, fingers, hands |
| V 0, 1, 3 | 0x005e91c0-0x005e9880 | 1, 1, 0 | flash | bodies, mouth, head |
| V 2 (rows 271-273) | 0x005e9340-0x005e9760 | 1, 1, 0 | flash | the animation's `EXT_evb1body01` ... |
| V 4 | 0x005e98e0 | 1-4, 1-24, 0-2 | bezier (the dummies) and spline, both packets, flash | head, tail, dummies |

**In the port.** `piney_battle::weapon` holds the rules: `WeaponCtrl`
(`ctrl`, `note`), `Weapon` (the cells as an array with indices for the
links) and `rad_ctrl`. Its world (`WeaponWorld`, over `prim::RadWorld`)
answers the node's matrix, `ccTransPosFW2LW` and the camera's turn. The
enemy's `exclusive()` and `note()` call it with what they read of the enemy
(`WeaponAt`). `piney_world::combat::weapon` runs it on the frame's shows
after the entry control. The node is the enemy's body posed as this
frame's `Call::Draw` left it. The trails go to `Combat::trails`, and
`field_world` draws them as `sendPacket` does (a line is one pixel wide).
The flashes' rays are drawn as the boxes'. Their lights, like the boxes',
are not put in the light group.

`tools/test_enemy_weapon_rs.py` builds controllers with the game's
constructor over every table `races.rs` names and a made-up one (rgb
colours, bezier, both packets, two and three edges, a missing node), a
chunk given a random place and turn. It runs 360 frames of
notes and `ctrl` natively on each, then compares every weapon, cell and
flash and the `makePacket` calls against `weapon_probe`.

| check | what runs natively | cases | mismatches |
| --- | --- | ---: | ---: |
| weapons | `ccEnemyWeaponCtrl` constructor, `note`, `ctrl` with everything under them (`setCell`, `interpolateCell`, `eneSplineH`, `ctrlCell`, `dispCell`, `makePacketCell`, `ctrlRadiate`, `ccEnemyWeaponRad::ctrl`, `ccPrimRadiate::init`, `create`, `createPlate`, `ccFractionalHsv`); `ccTransPosFW2LW` a shift, `RotTransPers` a copy, `makePacket`, `disp` and the light group recorded | 98 tables x 360 frames (1,188,443 `makePacket` calls, 20,882 flash frames, 6,036 notes) | 0 |

### The party's weapon trails

Kite and the members sweep a ribbon behind the weapon as they swing, and
an art with an element lights the weapon with its particles. This is
`ccSpcChar` (gcmn spcchar.cpp) over [`ccLattice`](boss.md), not the
enemies' cells above.

`EquipWeapon` (0x0059d650) finds the weapon file's `DMY_xdummy_w01` to
`_w04` (`GetChunkAdrsF(name, 1)`) besides its models. Each is a
DummyPos chunk decoded with its place at +0x10 and w 1.0
(`Decode_DummyPos` 0x0014d4d0). It makes the lattices the job needs,
keeping any already made:

| job | dummies (+0x140..+0x14c) | lattices |
| --- | --- | --- |
| 0 (twin blades) | w01, w02, w03, w04 | +0x150 and +0x154, `ccLattice(2, 0)` |
| 1, 2, 5 | w01, w02 | +0x154, `ccLattice(2, 0)` |
| 3 | w01, w02 | +0x150, `ccLattice(2, 0)` |
| 4 | w02, w01, w03 | +0x154, `ccLattice(3, 0)` |

`ArmsEffect` (0x0059ddd0) runs from the draw of Kite's and the members'
frames (step 13 above). It does nothing when all four dummies are null.
It puts the points `weaponEffPos` (+0x160..+0x190) at the hands' world
matrices (+0x130 the right hand's node, +0x12c the left's) times the
dummies, w 1:

| job | points | rows (after `ClearCnt`) |
| --- | --- | --- |
| 0 | P0 = R d0, P1 = L d1, P2 = R d2, P3 = L d3 | +0x154: P0, P2; +0x150: P1, P3 |
| 4 | P0..P2 = R d0..d2 | +0x154: P1, P0, P2 |
| 1, 2, 5 | P0, P1 = R d0, d1 | +0x154: P1, P0 |
| 3 | P0, P1 = L d0, d1 | +0x150: P1, P0 |

The aura is `ccGetEquipParam(job, weapon)` +8. A row is laid only with
`trajectorySW` (+0xe1 bit 3, set through a swing) on or an aura (not -1).
The aura becomes the colour type (`_SetArmsEffectColor`, the lattice's
+4) with `trajectorySW` off, or on with the normal attack (skill +0x7c 1).
Then each lattice's `Disp`.

- `ClearArmsEffect` (0x0059e3b0) sets the lattices' +0x72c, so the next
  `ClearCnt` (0x00579c50) kills every row and puts the tail on the head.
- `SetArmsEffectColor(sid)` (0x0059e460) is `_SetArmsEffectColor(
  ccSkillCheckTypeAttribute(skill's type))`. It runs as a skill starts.
  `_SetArmsEffectColor` (0x0059e4b0) has no null check.
- `StartArmsEffect(sid)` (0x0059e530) starts an art's particles. It does
  nothing for a skill that is not an art. The element picks the row,
  `ccGetArmsEffectAttribute`: 40 + element, 0 for none. With no element
  it turns `armsEffectSW` (+0xe4) off. Otherwise, with the switch off, it
  turns the switch on and calls `startParticleEffect2(&weaponEffPos[a],
  &weaponEffPos[b], row, &armsEffectSW)`: job 0 on (0, 2) and (1, 3), the
  others on (0, 1). The generators follow the points and stop when the
  switch goes 0.

**In the port.** `piney_world::arms::Arms` is the state: the dummies from
the file hung on the actor's hands, the lattices, the points and this
frame's strips. Each party `Actor` keeps one (`weapon`), made at its
first call and re-read by `change_equip`. `Combat::arms_shown` runs the
frame's `ArmsEffect`, `ClearArmsEffect` and `SetArmsEffectColor` shows
in order, before `ccThEffect`. It poses the hands as the actor's play
stands. `field_world` sends the strips on the effect layer as the cross's
(`trail_packet`). `piney_battle::skill::start_arms_effect` is
`StartArmsEffect`: Kite's `AnimCtrl` calls it in piney-battle, a member's
`fellow::Out::StartArmsEffect` in the combat. `Effects::arms_particles`
starts the particles. The host answers `weaponEffPos` from the actor's
`Arms` and `armsEffectSW` from the character.

**Checked.**

- `test_boss_effect_rs.py`'s `test_weapon_trails` drives the game's
  `ccLattice(2, 0)` and `(3, 0)` as `ArmsEffect` does: `ClearCnt` every
  frame, `ClearArmsEffect`'s flag and colour types between frames. It
  compares every call as the cross's check does, with the flag and the
  type in the state. 0 mismatches over 600 frames each; 374 of the
  3-column frames send a packet (both strips).
- `arms::tests::kites_blades_leave_trails`: Kite's blades have all four
  dummies. Every weapon's aura is -1 or 0-7.
- `orcas_swings_leave_trails`: Orca's heavy blade (job 1) sends a white
  trail on 30 frames as he fights event 3's goblin. `party_trail_shots`
  shows the ribbon following his blade.
- `flame_dance_lights_his_blades`: Kite's Flame Dance (art 9) on the
  goblin. `armsEffectSW` goes on, particle row 42's generators start on
  both blades, and both trails turn red (type 4). `flame_dance_shots`
  shows the fire on his blades.

**Still unknown.** Neither the trails nor the particles have been
compared with the game's own pictures. A member's `StartArmsEffect` has
not been seen in play. The hands' matrices come from the port's pose,
not the game's `ccObj` matrices.

## The port

`crates/piney-battle` holds the rules and the motion layer as logic. The
combat loop owns the characters (`Scene`: the `Char`s and the three command
lists), newlib's `rand` (`Rand`) and `ccRand` (`Genrand`), calls a rule when
the game would, and turns the `Event`s it returns (in call order) into
effects; `affect::apply` runs every `EntryAffect` among them on the scene.
The motion layer runs whole frames of the field's tasks, asks the world
through `world::World` and its sub-traits, applies the affects it makes
where the game makes them, and hands presentation back as outputs in the
game's order.

| module | holds |
| --- | --- |
| `tables` | every table, read from `SLUS_202.67`, `DATA/GCMN.PRG` and `DATA/DEMO.PRG` at run time |
| `param`, `chara` | the structures; `CalcReal`, `ConditionTimeCount`, levels |
| `damage` | `CalcBattleDamage`, the protect gauge, `ccSkillDamage`, `ccSkillDamageValue` |
| `skill` | types, costs, healing, holding, conditions, cures, `_ccSkillRequest` |
| `flow` | `ccSkill` and `ccThSkill`, the normal attack's hit |
| `affect` | `EntryAffect` and each character's `affectFunc` |
| `exp`, `drain` | experience, infection, drops, Data Drain, `AreaItem` |
| `item` | `useitem.cpp`, the save's item lists |
| `enemy_ai` | `enemy.cpp`'s decisions, spawning and draining an enemy |
| `party_ai` | `ccAI` and the message bus, `ccFellow::UseSkill`/`Attack` |
| `party_chat` | `ccAI::ChatMessage*`, `ChatMessageModify`, `ChatMessageSender`, `Greeting`; the chat tables (`ChatTexts`) |
| `world`, `geom` | the world's queries (`World`, `CharHit`, `Note`); the EE arithmetic: libvu0's vectors and matrices, the angle helpers (`ccSetDirc`, `ccSetRad`, `ccSetDist`, `ccGetDircChgF`, `ccPiLimit` ...) |
| `frame` | `ccThSpc`: the strategy, the shout, `spcBattleCondition` |
| `kite` | `ccPlayer::Main`, `ControlMove`, `AnimCtrl`, `CheckNote`, `Attack`, `AttackCancel`, `BreakSomething`, the player's frame conversions (`W2MPos`, `W2PPos`, `P2WPos`); `KiteWorld` |
| `fellow`, `follow` | `ccFellow::Main`, `Move`, `Action`, `CheckNote`, `ccSpcChar::HitCheck`; `ccAI::FollowPlayer`, `LeavePlayer`, `FollowTarget`, `FollowTargetDirc`, `CheckFrontObstacle(F)`; `FellowWorld`, `Follow` (a runtime performing the follow calls) |
| `navi`, `ai_move` | `ccNavi` (dungeon ways, town landmarks), `SetPathFindingMap`; `MoveP2P`, `FollowBeacon`, `CheckGoalBeaconPos`, `ManualControl`, `ActInTown`; `NaviWorld` |
| `party_motion` | `Movement`: every movement call of the party AI performed by the modules above, as one runtime; `Share`, the world shared through a `RefCell` |
| `entry`, `races` | the entry control (`ccThEntryCtrl`, `ccEntryObj::routine`, the entry functions, `ccMagicCircle`), where the entries come from (`EntryGimmick`, `WORLD`/`DUNGEON::SetMagicCircle`, the event's `entry_mc`), registration; the race constructors |
| `enemy_motion` | `moveEnemy`, `animEnemy`, `ccEnemyCheckNote`, `dispEnemy`, the act helpers, the races' `action()`, `exclusive()`, `note()`; `ccEnemy::main` as a whole frame; `MotionWorld` |
| `evparty` | the events' party instructions (`pc_command`, the walks and puts, `pc_turn`, `pc_face`, `enemy_put`, `remove`), `hold` and `ccThEvHold`, `player_skill`, `battle_ready` |

Once a frame the loop runs the field's tasks in their order ([A frame of the
field](#a-frame-of-the-field)): `frame::SpcThread::frame` (`ccThSpc`);
`party_ai::Crew::tick` (`ccThAISystem`); `kite::main` (`ccThPlayer`); for
each party member `fellow::Frame::main`, its runtime a
`party_motion::Movement` around the loop's own; `entry::EntryCtrl::frame`
(`ccThEntryCtrl`), whose enemies each run
`enemy_motion::Motion::enemy_main` through `entry::EnemySeam` (the rules of
`enemy_ai` inside it);
`flow::Skills::frame` with each running skill's animation state (finished,
notes passed). Per action: `flow::Skills::request` when anyone starts a
skill (synchronously: the code reads `skillStatus` right after);
`item::menu_use_item` from the item menu; the `drain` steps from the Data
Drain menu. At an area's start: the entry control's registration and
`EntryGimmick`, the event's entries, `navi::PathMap::set_path_finding_map`
(again after each dungeon room is built), a dungeon member's
`Navi::set_dungeon_map_info`, the town's `TownMap::set_navi_map`.

The world is one object behind `world::World` and the sub-traits
(`navi::NaviWorld`, `fellow::FellowWorld`, `enemy_motion::MotionWorld`,
`kite::KiteWorld`, `entry::Seam`, `entry::EntryGimmickSeam`); the frames and
the movement runtime call it in turn, never at once, so the loop shares it
through a `RefCell` (`party_motion::Share`; `kite::Host` is Kite's world
over it). The following's `aiOpenDirc` and the path finding's `buf`,
`buf2` and `bufFlag` live in `party_motion::Keep`, kept from frame to frame.
`ccSkillCheck`
(`flow::Skills::check`) is asked while the skills are lent to the frame, so
the loop keeps `Skills` in a `RefCell` too.

**Checks.** Each check runs the game's function natively in `tools/eemu.py`
on a random case (gcmn.prg over the executable, the real tables in memory,
effect, particle, menu and movement functions stubbed to record their
calls) and the same case through the crate's `battle_probe` example, and
compares the return value, every field of every character and AI touched,
both generators' states and the calls in order:

| harness | covers | checks |
| --- | --- | ---: |
| `tools/test_battle_rs.py` | `CalcBattleDamage` (protect, Exdefense), `ccSkillDamage` (single, area, point, `ccSkillDamage2`), `ccSkillDamageValue`, `ccSkillRecovery`, `RecoverySystem`, `ccSkillHold`, `ccSkillModifyCondition`, `_ccSkillRequest` (with its `ccWordsPlay(sid, caster)`), `CalcReal`, levels, `_ccSkillModifyCondition`, `ccCheckConditionSkillSuccess`, `ccCheckTargetConditionBySkill`, `ccExpDistributor`, `AddLvErosion`, `EntryAffect` with each `affectFunc` | 20 |
| `tools/test_battle_flow_rs.py` | `ccSkill::Main` with its systems, `NoteEventAffect`, `ccSkillCheck`, both `CheckNote`s, `ConditionAdjustment`, `ccSpcCheckLevelUp`, `AreaItem` | 7 |
| `tools/test_battle_drain_rs.py` | `DataDrainMenu` steps 0-1, 10 (with the effects and numbers it starts, in order) and 20 | 3 |
| `tools/test_battle_items_rs.py` | `ccCheckItemUseful`, `ccCheckSkillUseful`, the item skill requests, `ccUseItemRequest`, `AddSpcItem`, `ccAI::UseItem`, the item lists | 7 |
| `tools/test_battle_enemy_ai_rs.py` | `defaultThink`, `interruptThink`, `ccEnemy::main` to `think()`, `routineEnemy`, `checkEnemy`, `setAct`, `selectAttack`, the skill lists and targets, `selectTarget`, `startSkill`, `affectSkill`, `initEnemy`, `genrand`, `ccGetDrainId` | 15 |
| `tools/test_battle_party_ai_rs.py` | the message bus, every decision above, `Brains`, `ReadSysMsg`, `Reconnoiter`, `ActInField`, `ActInDungeon`, `ccFellow::UseSkill`/`Attack` | 73 |
| `tools/test_battle_frame_rs.py` | `ccThSpc` from its start over 20-200 frames: `ccSpcSetOperation`, `ccSpcShoutOperationName`, the battle condition | 1 |
| `tools/test_battle_kite_rs.py` | `ccPlayer::AnimCtrl` and `Main`, each once and for 60-300 frames (the AI, `CalcReal`, `EntryAffect` with every character's own `affectFunc`, and the notes' `CheckNote` run natively), `ControlMove`, `Attack`, `CheckNote`, `AttackCancel`, `BreakSomething`, `DamageActuate`, `ResultOfConditions`, `ccPlayerMenuCheck`, `CheckControlMode`, `SetTargetDist`, `SetTargetDirc`, `ReadSysMsg2`, `W2MPos`/`W2PPos`/`P2WPos` and `ccTransPos*`; and 60-300 frames of `Main` with Kite charmed, confused or under remote control, `ccAI::Brains` driving him through `party_motion::Movement` (the following, `ccPlayer::Attack`, `ManualControl` and `ccSetDirc` native in the game) | 10 |
| `tools/test_battle_fellow_rs.py` | `ccFellow::Main`, `Move`, `Action`, `CheckNote`, `ccSpcChar::HitCheck`, `ccAI::FollowPlayer`, `LeavePlayer`, `FollowTarget`, `FollowTargetDirc`, `CheckFrontObstacle`, `CheckFrontObstacleF`, `SetDircZ`; runs of 60-300 frames of `ccThAISystem` and `ccFellow::Main` (Orca after Kite in a field and a dungeon, with and without foes), compared after every frame (every `Char`, `Spc` and AI field, the bus, `rand`, the world's calls), `EntryAffect` run natively with the real affect functions; and the party's movement assembled (`party_motion::Movement` in the port; `PathFindingInDungeon`, `SetPathFindingMap`, `FollowBeacon`, `CheckGoalBeaconPos` native in the game) in rooms of a dungeon's 2D map with walls, doors and pillars, the `ccNavi` and `buf`/`buf2` compared too | 18 |
| `tools/test_battle_navi_rs.py` | `SetPathFindingMap`, `PathFindingInDungeon` (both) and its pieces, `CheckBeaconPos`/`2D`, `GetDestination`, `ccNavi`, `SetDungeonMapInfo`, the tutorial dungeon's and random dungeons' rooms in sequence, the town landmark searches, `ccSetNaviMap`, `MoveP2P`, `SetTargetPosDirc`, `FollowBeacon`, `CheckGoalBeaconPos`, `ManualControl` with `ManualMode`/`SetRemoteCmd`/`SetGoalPos`, `FollowTargetTown`, `TownNavigator*`, `MessageIndex`, `ActInTown`, 60-300-frame runs of `ActInTown` and `FollowBeacon`, and the composed runs of `party_motion::Movement` (`ActInTown` in Mac Anu, `ManualControl`, `ccAI::Brains` in a dungeon room) | 23 |
| `tools/test_battle_spawn_rs.py` | the entry control and `ccEntryObj::routine`, `entryObject`/`entryObjectCheck`, `entryEnemy`/`MagicCircle`/`CircleObject`/`EnemyObject`, `initObject`, the deletes, the active checks, `ccMagicCircle::main` with its particles, multi-frame `ccThEntryCtrl` runs with Kite walking to a circle, `WORLD`/`DUNGEON::SetMagicCircle`, `ccEntryEventMng`'s `entry_mc` and enemy entries, `EntryGimmick`, `ccThEntryCtrlDelete`, `restoreEntry`, registration, the save checks, every race's constructor but L's type 3, with `checkGold`, `ccCheckDustColor`, `ccRandS`; and `ccThEntryCtrl` frames with the enemies' own `ccEnemy::main`, the races' code and `EntryAffect` run natively (`entry::EnemySeam` in the port) while the party approaches a circle and the enemies spawn, chase, attack, die and are drained; the boxes' and idols' own constructors and `main`s run natively (boxes opened, disarmed, trapped, their traps on the opener, the virus crystal, the rays, the draws) and `DUNGEON::SetItemBox` and `SetIDOL` over random slots and story rows | 23 |
| `tools/test_battle_enemy_motion_rs.py` | `ccSetRad`, `ccSetDist`, `ccGetDircChgF`, `ccSetDirc`, `ccGetDirc`, `ccGetDist`, `ccSetRadDisperse`, `ccPiLimit`, libvu0's rotation matrices; the act helpers; `moveEnemy`; `animEnemy` with `ccEnemyCheckNote`; `dispEnemy`; each race's `action()` (L's but type 3), `moveEG`/`moveEGG`/`moveEB`, `exclusive()` (with H's breaths, C's spin, D's float and F's dive), `note()` (with E's foot dust), `ccEnemyG::freeze`; `ccEnemy::main` over 60-300 frames of 1-3 enemies with the affects run natively | 11 |

At least 1,000 cases of each: 0 mismatches. The motion checks stub the
world's functions in the interpreter with answers drawn at random (the probe
answers the same, and both record the calls); their long runs play 60-300
frames of a task's whole frame with Kite and the party on scripted paths
and compare every field after every frame. `cargo test -p piney-battle`
checks the tables read from the disc and the worked example below.

## How it plugs into piney-world

The field and the dungeon run the battle: `piney-world`'s `combat` module
holds one `Combat` per area (the scene's characters, the enemies' state,
the crew, the party, `Keep`, the entry control and its registration,
`ccThSpc`, Kite's player, `Skills` in a `RefCell`, both generators) and
`FieldWorld` runs it inside its frame in the game's task order:

```text
ccThEvent (32)          the event VM (piney-game's area host), which calls
                        the events' party instructions (piney-battle's
                        evparty over the combat's characters)
ccThGameCtrl (33)       talk::Targeting::frame: the target, the buttons,
                        inBattle (SetInBattle / ccCheckInAreaCmnd), the
                        delayed recoveries, X on an enemy -> ccSkillRequest
event camera (33), ccThEvHold (33)  evparty::hold_frame
ccThCamera (40)         cameraMain with Kite's heading
ccThSpc (48)            SpcThread::frame
ccThAISystem (48)       Crew::tick
ccThPlayer (49)         kite::main
ccThFellowNN (50)       fellow::Frame::main for each member, its runtime a
                        party_motion::Movement over the field
ccThEntryCtrl (64)      EntryCtrl::frame with entry::EnemySeam (the enemies'
                        ccEnemy::main); then ccChar::Draw of each enemy
                        dispEnemy placed (foe::char_draw); the kills'
                        ccExpDistributor
ccThEffect (80)         the effects (FxTasks::effect, piney-effect)
ccThSkill (82)          Skills::frame, its events applied
ccThParticle (98)       FxTasks::particle
ccThFieldDisp (96)      the characters, the effects, WORLD::Draw /
                        DUNGEON::Draw
```

| piece | where |
| --- | --- |
| the world traits (`World`, `NaviWorld`, `FellowWorld`, `MotionWorld`, `KiteWorld`, `entry::Seam`, `party_ai::Runtime`) | `combat::Stage` over the area's `Hits`, the camera, the pad and the `Cast`; lent to the frames through a `RefCell` as the crate expects |
| a party character's `ccSpcChar` as piney-world's `SpcRef` (`ManualModeAI`, `SetRemoteCmd`, `pc_act`, `menu_ban`) | `combat::spc` (`SpcRec`, `BattleChars`), read from and written back to the scene and the crew |
| the animation players (`ccAnm`: `SetAnm`, `frameNow`, `_AnimateForward` and the `F_Note`s it passed, which `NoteProcess` hands on) | `combat::cast` (`Actor`, `Play::forward_notes`, [animation](animation.md)) |
| the player's frame (`ccTransPosW2P`/`P2W`) | `kite::w2p_pos`/`p2w_pos` with `WORLD_MAN`'s bounds: a field 0..48000, a dungeon 0..60000 (*inferred* from `WORLD_MAN::GO`), an event area -48000..48000 |
| Kite and the members | `Char::pc` from the save's `spcParam`, built when the party is (`rebootSpcManager`), the registry's `bootParam` kept across areas |
| the enemies and portals of an area | the entry control's registration: `ccRegisterDifficultyEnemy` (`WorldMan::enemy_rank`: a story area's type 6 and rank `eventAreaInfo.enemy` unless 255, else `25 (areaLevel - 1) + 10 + enemyOfs`, not below 0, plus floor + 1 in a dungeon), the field's circles, the event's `entry_mc` and `entry` positions |
| the enemies' and the portal's looks and draws | [Drawing the enemies and the portals](#drawing-the-enemies-and-the-portals) (`foe`); the portal's sparks through piney-effect's `MagicCircle::render` |
| the HUD | piney-game fills `piney_fieldui::World` from the combat: the enemy chain sorted by `cmndDist`, `CharInfo` with HP and conditions, the life bars' points and the off-screen arrows, `inBattle`; the menus' requests (skills, items, targets, Data Drain's hold, the chat orders) call back into it |
| the event instructions | [The events' battle instructions](#the-events-battle-instructions) over the combat (`FieldWorld::pc_command`, `player_skill_*`, `set_event_hold`, `battle_ready`) |
| the members' chat lines | the stage's runtime runs `Call::Chat` and `ChatMessageSender` (`Show::Chat`); `combat::chat` runs the rules' chat events on the members' AIs; piney-game opens the text in the member's balloon ([the chat lines](#the-chat-lines)) |
| the battle music | `inBattle` each frame into the sound's `bgmChange` ([sound](sound.md#battle-music-bgmchange)) |
| the effects | piney-game's `fx.rs` (`AreaFx`), installed into the world as `FieldFx`: [effects](effects.md#driving-it) |

The effects' starters run where the next effect task is (the start of
`ccThEffect`, or of `ccThParticle` for what `ccThSkill` raised), for the
shows the tasks left since, in order; the game calls each inside the task
that raises it, so their draws from `rand()` fall after the rest of that
task's in the port (the effects step in the same frame either way).

What the runtime still lacks:

- **Sounds, in part.** The frame's sounds are played through piney-audio's
  positioned sound effects ([sound](sound.md)): the enemies' notes
  (`ccSeSetParamEnemy`), Kite's and the members' (`ccSeSetParamSPC`, the
  footsteps by the ground), `ccSeOn3D` of the portals, the gold goblins
  and the effects; heard from the active camera as the frame leaves it
  (the game reads `activeCamPtr` at each call), with `ccThSkill`'s heal
  sound (`ccSeOn3D(75)` at the target) and its notes' `ccSeSetParamSPC`.
  The effects' camera shakes, flashes and screen noise are carried out:
  the shakes on the field's camera, the flashes on `scFadeDef`, and a
  blast's `ccMenu->noiz->SetNoizBs(20)` on the field UI's noise
  (`a_spell_lands_through_the_effects` casts Tornado from within the
  camera's reach and sees it).
- **Spells, in part.** `ccThSkill` walks the runs itself (as
  `Skills::frame` does) so that an attack spell's element system runs where
  `ccSkill::Main` runs it: `FxTasks::spell` makes the effects' `Spell` at
  its first call (`Effects::spell_request`), syncs it from the run
  (`Spell::sync`, the count before Main's increment) and steps
  `Effects::spell_system`; the run takes back `endFlag`, `holdFlag`,
  `level` and the casters released, and the damage calls
  (`SkillDamage`, `SkillDamageAt`, `SkillDamage2`) go to
  `damage::skill_damage`, `skill_damage_at` and `skill_damage2` with the
  run's `acFlag` and the position through `ccTransPosW2P`. The game makes
  those calls inside the system, between the effects' own draws from
  `rand()`; the port makes them after it returns, in the same order.
  `effSkillStart`, the exec rings, the shock wave, heals and cures are not
  in piney-effect yet.
- **Data Drain.** The movie, the enemy's transformation
  (`ccThDrainEnemy`, 0x00432000), the fades and noise.
- The items' steps (the book viewer, the Grunty ride).
- The objects' own code the entry control calls and the port does not
  hold (gimmicks' and NPCs' `main` and constructors other than
  `ccGimBox`, `ccGimIdol`, `ccGimSymbol` and `ccGimFood`; `EntryBreakObject`'s
  placement is the dungeon's, [dungeon](dungeon.md#the-breakables)).
  The objects' effects (`effOpenTrapBox`,
  `effStatueOfGod`, `effVirusCrystal`, the breakables' `effCrush*`) are
  piney-effect's ([the effects](effects.md#the-dungeons-objects-crystals-breakables-traps-the-statue)).

**Runtime checks** (`cargo test -p piney-game`): `event_3_plays_its_fights`
plays a new game from Log in through events 2 and 3: the portal south of
the start opens on a goblin the event holds, `player_skill` has the
player's attack button hit it until it is down (its HP falls from 50 to 0
over several blows), Kite gains experience, menus 80-83 open and close,
the event ends; then Kite walks east to the next portal, `inBattle` goes
to 1, Orca targets its goblin and the attack button sends Kite's normal
attack at it, and it goes down. `a_spell_lands_through_the_effects` has
Kite cast Tornado (197) on the next portal's goblin: its system sets level
1, releases Kite, and its blows from frame 35 take the goblin's HP.
`event_4_plays_in_the_dungeon` walks on into the dungeon and plays event 4
through: each block in its room (`SetEventData`, [dungeon](dungeon.md)),
the trap room's portal with its goblin, which Kite and Orca hit until it
falls, and the doors opening after it, down to block 8's `mode 3`.
`event_3_shots`, `portal_fight_shots` and `event_4_shots` draw the fights
to PNGs (`PINEY_SHOTS=DIR`, marks in `PINEY_MARKS`).

## Example

Kite at level 1 with his starting gear: pAtk 35, pDef 54, pHit 43, pEva 73,
mAtk 24, mDef 54, mHit 36, mEva 66. Against a Goblin (row 130: pDef 37, pEva
4, HP 50): `hit = roll + 4`, so a hit needs a roll of 46 or more (55 of 101
rolls, 54.5%). A hit deals 8 to 16 damage, 11.87 on average, 6.47 per swing.

## Other volumes

`test_battle.py` runs the same checks against each volume's own code: 1,000
cases of each of 13 checks per volume, 0 mismatches. `battle.py` finds every
table from its accessor function, and the row counts from the table's
neighbours; on Infection this reproduces the DWARF sizes.

The sections the Rust port added (targets, skill types, a skill's life,
affects, the normal attack, Data Drain, item boxes) are checked on Infection
only.

From Mutation on:
- **Exdefense.** Infection keeps the whole Exdefense value while any
  matching defence is not lowered. The later volumes keep only the matching
  bits 0x1-0x80 whose defence is not lowered, and clear them all when the
  row has 0x100 (MUT `0x005933f8`-`0x005935bc`). No row on any volume has
  0x100.
- **`ccSkillDamage`.** The dead-target early return moved into the
  single-target branch (MUT `0x00599988`), so an area skill no longer stops
  when its aimed target is dead (*read*).
- **21 party members.** `charTbl`, `LevelUpParamTbl` and `ccExpDistributor`
  cover Tsukasa, Subaru and Sora. `ccGetCharParam` sends ids 18-20 to the save
  extension ([save](../formats/save.md#the-extension-mutation-on)).
- `ccSkill`'s creator and target move from +0x70/+0x74 to +0x80/+0x84.
- **The bosses.** `ccBoss02` (Innis) and `ccBoss03` (Magus) are in
  Infection's gcmn but fought in Mutation; both are ported from Mutation's
  code ([Innis](boss-innis.md), [Magus](boss-magus.md)). `ccBoss04` is not.

OUT and QUA differ from MUT only in compilation: the durations come from a
literal pool, and `fptosi` is inlined. The tables change from volume to volume;
see [game data](../content/game-data.md#other-volumes).

## Unknown

- The labels of type bits 0x100/0x200 (set on spells, read by no rule) and
  the art bits' names.
- That the protect break is the Data Drain window is *inferred* from the
  gauge's rules below. `plcol` is the bracelet: the event instruction
  `data_drain` sets it to 1 together with Kite's skill 2, Data Drain (event
  11's book, [events](events.md)). Kite's menu face follows it.
- `ccBoss04`'s `Affect` and AI: in Infection's gcmn but fought only in a
  later volume, and not ported.
- Kite's third swing: `attack` 3 and 4 have code in `AnimCtrl`, but no
  gcmn code stores 3 (*inferred* from a search of the stores to +0xfe).
  `ControlMove`'s `ctrlType` 1 and 2 are only reachable if something
  writes +0x204, and only the constructor does, with 0 (*inferred*).
- What the party AI does with message 0x10004, which Kite sends after about
  50 frames pressed against a wall (`stressMeter` 101).
- That `WORLD_MAN`'s trans mode is a carrier the party
  rides, and that `weaponChangeSW` -1 frees the weapon's file, are
  *inferred* too. `fpAngleOffset[-1]` (110, the halfword before the array)
  is measured; that a member out of the party reads it is all that is known.
- A dungeon way of 55 cells or more overwrites `ccNavi`'s beacon-table
  pointer, and from 67 the AI's message queue; the port keeps its table and
  its checks stop at 54 cells. Where the game walks to a destination it
  never set (a field's `GetDestination`, a landmark `ActInTown` cannot find)
  it reads stack garbage; the port starts from (0, 0, 0, 0). A `gRotSp` of 0
  divides by zero in `ccSetDirc`.
- *Inferred:* that `ccHitCheckLM` moves the z of its ends in
  `DUNGEON::MakeMiniMap` (the loop resets them every cell); the meaning of
  `saveData` +0x220d (`MessageIndex` 18 for character 1) and of the ground
  attributes 0x20000 and 0x80000 in the 2D map.
- A story dungeon's circles take `w` from a word on the caller's stack,
  taken as 1.0. A middle boss's constructor reads `anmTbl` at EE 0xb4
  (empty). With no enemy row registered, a circle's random row divides by
  zero in the game.
- `ccEnemy1::action` with an `eneType` outside 0-5 reads an uninitialised
  register (the port takes 0.8); no constructor makes one (measured).
- A new `ccAI`'s `levelOld` is whatever the heap held (`_ccMalloc` does
  not clear). A member whose first `levelCheck` falls outside act 14, as in
  a town's arrival, may announce a level change in the game. The port
  starts it at the character's level. What the game's heap holds there was
  not measured.
- Whether the two lines with a stray `#` code (Orca's third
  `fatalDamagedMessages`, Rachel's `charmConfusionMessages`) freeze a real
  console as they freeze eemu's run of `ChatMessageModify`. They need Orca
  hit for over a third of his maxHP, or Rachel charmed or confused by a
  skill, while in the party.
- Whether `ccChar::Draw` leaves an enemy's node coordinates as they were
  on a frame it culls the body. If so, the weapon controller reads the last
  drawn pose. The port reads the enemy's matrix from the last frame that
  drew it, and its pose from this frame.
