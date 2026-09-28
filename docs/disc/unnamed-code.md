---
title: The later volumes' unnamed code
status: solid
volumes: MUT, OUT, QUA
covers: every stretch of MUT SLUS_205.62, OUT SLUS_205.63, QUA SLUS_205.64 main, gcmn.prg, desktop.prg, toppage.prg and demo.prg text that no carried name covers
worklog: 239
---

# The later volumes' unnamed code

Every stretch of each later volume's code that no name carried from
Infection covers (`<elf>.syms`, [the four discs](volumes.md#names-for-the-stripped-executables)),
listed with what can be read of it: where it is, the named functions either
side, how many functions start in it, the strings it builds and the named
functions it calls. [The four discs](volumes.md#what-the-later-volumes-code-adds)
describe the large ones.

The tables are generated:
`tools/voldiff.py report work/<volume>/disc/<elf> --title "<name>"`
(`gaps` with no minimum, then `look` on each stretch).
- **at, bytes.** The stretch, between two named functions more than 16
  bytes apart.
- **after, before.** The named functions either side (`-` at a section's
  end).
- **fns.** The function prologues (`addiu $sp, $sp, -N`) in it: 0 is data,
  a leaf function, or a function's tail that a carried size cut short.
- **strings.** The strings its code builds (`lui`/`addiu` pairs), most used
  first, at most four.
- **calls.** The named functions it calls, most called first, at most five.
  Calls to unnamed functions are left out.
- A stretch whose words are all zero is marked as padding.

Counts: Mutation 80 stretches, Outbreak 173, Quarantine 170; 351 of the 423
start at least one function, and 17 are all padding. Main's largest on each
volume is the stretch from `DEG2RAD` to the static constructors, which holds
the bitmap fonts ([text rendering](../engine/font.md#the-kanji-fonts)) and
data.

The functions in these stretches have no names to carry. What they do is
read from their strings and calls here, and from their code where a system
is ported.

## Mutation (SLUS_205.62)

### main

Text 0x288868 bytes, 0xd50a8 named, 0x1b1ad4 unnamed (66.9%) in 32 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00100000` | 0xb8 | `-` | `_exit` | 0 |  | `_InitSys`, `FlushCache`, `main` |
| `0010f3a0` | 0x180 | `sceDmaPutEnv` | `sceDmaGetEnv` | 0 |  |  |
| `00110d18` | 0x28 | `sceVu0RotMatrix` | `sceVu0CameraMatrix` | 0 |  |  |
| `00110f18` | 0x108 | `sceVu0LightColorMatrix` | `sceVu0DropShadowMatrix` | 1 |  | `sceVu0UnitMatrix`, `sceVu0MulMatrix` |
| `001150f0` | 0xc8 | `_groupOfPicturesHeader` | `_pictureDisplayExtension` | 1 | `load_chroma_intra_quantizer_matrix == 1`, `load_chroma_non_intra_quantizer_matrix =` | `_nextBit`, `_waitIpuIdle`, `_sendIpuCommand`, `_Error` |
| `00119250` | 0x68 | `sceIpuSync` | `sceIpuInit` | 1 |  | `DIntr` |
| `0011c698` | 0x158 | `_request_rdata` | `_search_svdata` | 1 |  | `_sceRpcFreePacket`, `sceSifSendCmd`, `DeleteSema`, `_sceRpcGetPacket`, `CreateSema` |
| `0011ce58` | 0x98 | `sceSifRegisterRpc` | `sceSifRemoveRpcQueue` | 1 |  | `DIntr`, `EIntr` |
| `00121c6c` | 0x6c | `SetDebugHandler` | `InitTLBFunctions` | 0 |  |  |
| `00122084` | 0x3c | `InitTLB32MB` | `_kTLBException` | 0 | (zero words: padding) |  |
| `001221dc` | 0x124 | `_kTLBException` | `_kDebugException` | 0 |  |  |
| `0012254c` | 0x14 | `GetSystemCallTableEntry` | `_setup` | 0 |  |  |
| `001225bc` | 0x6c | `_InitSys` | `PatchIsNeeded` | 0 |  |  |
| `00122818` | 0x38 | `Exit` | `sceSdRemoteInit` | 1 |  | `TerminateLibrary` |
| `0012afe0` | 0x70 | `cdvd_exit` | `_sceCd_Poff_Intr` | 1 |  | `PowerOffCB`, `DIntr`, `EIntr` |
| `00171e30` | 0x110 | `BootCheckProccess__9ccSaveSysFv` | `MainProccess__9ccSaveSysFv` | 1 |  | `CheckPort__7ccMcardFii`, `ReadSys__7ccMcardFiiPvii`, `CheckRightInfo__9ccSaveSysFP14ccSaveDataInfo` |
| `00179eb4` | 0x53c | `SetSoundEnv__10ccSaveDataFv` | `AddLvErosion__10ccSaveDataFif` | 0 |  |  |
| `0017a860` | 0x980 | `ReadNewMail__10ccSaveDataFi` | `ccGetItemName__Fii` | 5 |  | `GetFrameRate__8ccSystemFv` |
| `0017c1e0` | 0x110 | `ccSqFade__FiUsiUc` | `ccSeOn__Fi` | 1 |  | `ccSqPlayVol__FiUs` |
| `00184208` | 0x468 | `ccSndMoviePlayer__Fii` | `ccLoad_SndModules__Fv` | 1 |  | `ccBreathThread__Fi`, `ccSndCmd__Fii` |
| `0018795c` | 0x4b4 | `ccSndGetStatus__Fv` | `SetObj__10ccEventObjFUi` | 3 |  | `__nw__FUi`, `__ct__8ccSpriteFv`, `Init__7ccKanjiFii`, `__dt__7ccKanjiFv`, `__dl__FPv` |
| `00188e24` | 0xccc | `Func_str8000__FP6ccTscb` | `Func_str9001sub__FUiP15ccStrEffectCtrlP8` | 1 |  | `rand`, `__nw__FUi`, `__dl__FPv`, `DeleteFade__8ccScFadeFi`, `EntryFade__8ccScFadeFiiiffff` |
| `0018acc0` | 0x20c0 | `Func_str9000__FP6ccTscb` | `Func_str0001__FP6ccTscb` | 2 |  | `__dl__FPv`, `rand`, `__nw__FUi`, `MakePacket__16ccBufferSamplingFP7ccLayer`, `Breath__6ccTscbFi` |
| `0018d210` | 0x8c0 | `Func_str0001__FP6ccTscb` | `Func_str0150__FP6ccTscb` | 0 |  | `rand`, `__dl__FPv`, `SetReflex__16ccBufferSamplingFffUi`, `MakePacket__16ccBufferSamplingFP7ccLayer`, `SetYH__13ccRasterNoizeFii` |
| `0018e5b4` | 0xdac | `Func_str0150__FP6ccTscb` | `Func_str0240__FP6ccTscb` | 1 |  | `rand`, `__nw__FUi`, `__dl__FPv`, `SetReflex__16ccBufferSamplingFffUi`, `DeleteFade__8ccScFadeFi` |
| `0019c13c` | 0xffb4 | `Func_str0610__FP6ccTscb` | `ccInitStreamDemoThread__Fv` | 18 | `str0710e`, `str1070e`, `EFF_x001` | `__nw__FUi`, `__dl__FPv`, `rand`, `MakePacket__16ccBufferSamplingFP7ccLayer`, `DeleteFade__8ccScFadeFi` |
| `001ac300` | 0x140 | `ccSetStreamDemoThread__FP8ccStream` | `__dt__13ccEffPart0580Fv` | 2 |  | `__dl__FPv`, `__dt__5ccObjFv` |
| `001b15f8` | 0x128 | `Quit__9WORLD_MANFv` | `Init__9WORLD_MANFv` | 0 |  |  |
| `001b7230` | 0x30 | `SetLightDirection__9WORLD_MANFP14ccDista` | `SleepDistantLight__9WORLD_MANFv` | 1 |  | `sceVu0CopyVector` |
| `001c4440` | 0x3490 | `Execute__7ccEventFRPsiii` | `MenuBan__7ccEventFv` | 0 | `cdrom0:\DATA\DEMO.PRG`, `cdrom0:\DATA\DESKTOP.PRG`, `cdrom0:\DATA\TOPPAGE.PRG`, `cdrom0:\DATA\GCMN.PRG` | `ccBreathThread__Fi`, `GetWordParamFromEvCode__9WORLD_MANFii`, `strcat`, `fptosi`, `EntryFlash__8ccScFadeFiiffff` |
| `001ca7f8` | 0x88 | `ccThEvHold__FP6ccTscb` | `ccStartThEvent__Fv` | 0 |  | `Breath__6ccTscbFi` |
| `001f04ec` | 0x197c04 | `DEG2RAD__Fs` | `__sinit_iostream.cpp` | 10 |  | `effWork+0xca08`, `__register_global_object`, `dungeonCCSTbl+0x98`, `effWork+0xf924`, `eventAreaInfo+0x2a4` |

### gcmn

Text 0x1ec240 bytes, 0x1d5e1c named, 0x112c0 unnamed (3.5%) in 42 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00413840` | 0x40 | `-` | `ccThDfCompDelete__FPv` | 0 | (zero words: padding) |  |
| `00416efc` | 0x154 | `Draw__11EVENTAREA03Fv` | `__ct__11EVENTAREA04Fv` | 2 |  | `__ct__9EVENTAREAFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `0041cb0c` | 0x34 | `Draw__11EVENTAREAB0Fv` | `NextStage__11EVENTAREAB0Fv` | 0 |  |  |
| `0041e8b4` | 0x47c | `HitEnable__11EVENTAREAB8Fi` | `Draw__11EVENTAREAB8Fv` | 1 | `CAM_camera2_4`, `OBJ_dummy2_4`, `CAM_camera3_3`, `OBJ_dummy3_3` | `_AnimateForward__5ccAnmFUi`, `GetSubstAdrsF__5ccAnmFPCcb`, `sceVu0CopyVector`, `sceVu0CopyMatrix`, `_SetLWMatrix__7ccCoordFv` |
| `0047375c` | 0x24 | `DrawAfterImages__6ccBossFv` | `SetBlendTypeAfterImage__6ccBossFUc` | 0 |  |  |
| `00476b34` | 0x56c | `__dt__9ccBossCamFv` | `ccBoss03BlurParamCB__FPfPfPUi` | 0 |  |  |
| `004778f4` | 0x9c | `OnCinemaMode__16ccBossEffManagerFi` | `OffCinemaMode__16ccBossEffManagerFv` | 1 |  | `checkPartyAnnihilation__Fv` |
| `004806e8` | 0x58 | `CinemaOn__19ccBossEffCinemaFadeFi` | `CinemaOff__19ccBossEffCinemaFadeFv` | 1 |  | `SetupSkillName__19ccBossEffCinemaFadeFP17CINEMAS` |
| `004899e8` | 0x1a8 | `Draw__13ccBossEffWaveFv` | `__dt__20ccBossEffEnergyGrow2Fv` | 1 | `particle`, `CLT_x011`, `CLT_x011c2`, `EFF_x011` | `GetChunkAdrsF__8ccStreamFPCci`, `sceVu0CopyVector`, `GetCCSAdrs__8ccStreamFPCc`, `__nw__FUi`, `Init__5ccEffFP10ccEffChunki` |
| `00489c10` | 0x290 | `__dt__20ccBossEffEnergyGrow2Fv` | `__ct__20ccBossEffEnergyGrow2FPf` | 2 |  | `ccRandF__Ff`, `Draw__5ccEffFPfUs`, `sceVu0UnitMatrix`, `sceVu0RotMatrix`, `sceVu0ApplyMatrix` |
| `0048a0a0` | 0x200 | `__ct__20ccBossEffEnergyGrow2FPf` | `Draw__14ccBossEffDrainFv` | 2 |  | `ccCheckParticleGenerator__FP19ccParticleGenerato`, `ParticleKill__19ccParticleGeneratorFv`, `__dl__FPv` |
| `0049e7b4` | 0x22c | `__dt__8ccBoss03Fv` | `CalcCamera__8ccBoss03Fv` | 2 |  | `GetSubstAdrsF__5ccAnmFPCcb` |
| `0049f400` | 0x180 | `SetupLaserShock__8ccBoss03Fv` | `Main__8ccBoss03Fv` | 1 |  | `sceVu0AddVector`, `sceVu0CopyVector`, `ccTransPosW2P__FPfPf`, `sinf`, `cosf` |
| `004a1c18` | 0x368 | `OnThinkDataDrainAtk__8ccBoss03Fv` | `OnThinkNeedle__8ccBoss03Fv` | 1 |  | `sceVu0RotMatrixZ`, `changeCamera__Fi`, `ccCheckTarget__FP6ccChar`, `BeginStageEffect__6ccBossFiiii`, `LockPlayer__6ccBossFi` |
| `004a2ebc` | 0x244 | `OnThinkWave__8ccBoss03Fv` | `OnThinkLeafDrop__8ccBoss03Fv` | 1 |  | `EraseCmndTarget__6ccBossFv`, `sceVu0CopyVector`, `EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T`, `ccTransPosW2P__FPfPf`, `ccGetDist__FPfPf` |
| `004b69d0` | 0x210 | `OnThinkChase__8ccBoss06Fv` | `OnThinkSkill__8ccBoss06Fv` | 1 |  | `ccSeOn3D__FiPf`, `EraseCmndTarget__6ccBossFv`, `EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T`, `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector` |
| `004c71a4` | 0x16c | `OnThinkBodyPress__10ccBoss08_bFv` | `OnThinkDie__10ccBoss08_bFv` | 1 |  | `checkPartyAnnihilation__Fv`, `LockPlayer__6ccBossFi`, `EraseCmndTarget__6ccBossFv`, `CheckMenuType__10ccMenuCtrlFv`, `UnlockPlayer__6ccBossFv` |
| `004c7ce8` | 0x208 | `OnThinkSetupBarrier__10ccBoss08_bFv` | `OnThinkRandomDrive__10ccBoss08_bFv` | 1 |  | `ccSeOn3D__FiPf`, `EraseCmndTarget__6ccBossFv`, `EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T`, `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector` |
| `004cb708` | 0x698 | `OnThinkNeutral__10ccBoss08_cFv` | `OnExit__10ccBoss08_cFv` | 1 |  | `sceVu0CopyVector`, `sceVu0RotMatrixZ`, `__nw__FUi`, `__ct__19ccParticleGeneratorFP24ccParticleGenerat`, `startParticleGenerator__FP19ccParticleGenerator` |
| `004d0d08` | 0xc8 | `Affect__17ccBoss08KillerEyeFv` | `Generate__17ccBoss08KillerEyeFv` | 1 |  | `SetMatrix_PosRotZYX__7ccCoordFPfPf`, `_AnimateForward__5ccAnmFUi`, `PreDrawAnm__6ccBossFv` |
| `004d1440` | 0x340 | `ExecPatternIndex__17ccBoss08KillerEyeFPi` | `OnThinkGenerate__17ccBoss08KillerEyeFv` | 2 |  | `ccSetDirc__FPffi`, `checkPartyAnnihilation__Fv`, `ccCheckTarget__FP6ccChar` |
| `004edea0` | 0x1e0 | `KyviaSwitchActionPattern__13ccBossKyvia0` | `SwitchActionPattern__13ccBossKyvia04Fv` | 1 |  | `SetCoreBaseParam__9kyviaCoreFii`, `EntrySlave__13ccBossKyvia04Fii` |
| `004eeef0` | 0x9b0 | `KyviaScene__13ccBossKyvia04Fv` | `CheckDmgSmokeEff__13ccBossKyvia04Fv` | 2 |  | `ParticleKill__19ccParticleGeneratorFv`, `sceVu0CopyVector`, `LockPlayer__6ccBossFi`, `EntryFlash3__8ccScFadeFiiiiffff`, `__nw__FUi` |
| `00521988` | 0x368 | `breederInfluence__FP6ccChar` | `ccSetRtownPC__Fii` | 1 |  | `GetChunkAdrsF__8ccStreamFPCci`, `SetAnm__5ccAnmFP10ccAnmChunkUi`, `ccSetDirc__FPffi`, `effTransfer__FP6ccChar`, `deleteCmnd__10ccEntryObjFi` |
| `00521e4c` | 0x1a4 | `ccSetRtownPC__Fii` | `ccEntryRtownPC__FP7ccEntry` | 1 |  | `sceVu0CopyVector`, `ccEntryParamClear__FP12ccEntryParam`, `ccNaviGetLandmarkPos__FPfi`, `GetChunkAdrsF__8ccStreamFPCci`, `entryObject__11ccEntryCtrlFP12ccEntryParam` |
| `005243e0` | 0x970 | `normalMode__9ccRtownPCFv` | `eventMode__9ccRtownPCFv` | 1 |  | `GetChunkAdrsF__8ccStreamFPCci`, `SetAnm__5ccAnmFP10ccAnmChunkUi`, `HitDisable__9ccCharHitFv`, `changeRoute__9ccRtownPCFi`, `ccEntryCmnd__FP6ccChar` |
| `0052f244` | 0xa1c | `ControlMove__11ccPuccigusoFv` | `PadLeverPower__11ccPuccigusoFf` | 5 |  | `sceVu0InnerProduct`, `sqrtf`, `sceVu0CopyVector`, `ccTransPosW2P__FPfPf`, `PadLeverPower__11ccPuccigusoFf` |
| `00530dc8` | 0x1c8 | `DrawPG__11ccPuccigusoFv` | `ccPuccigusoCheckNote__FP9ccAnmNote` | 2 |  | `strcpy`, `OpenChat__9ccChatMsgFP6ccCharPc`, `getPartyMenberChar__Fi` |
| `00531088` | 0x88 | `ccPuccigusoCheckNote__FP9ccAnmNote` | `ccSetNaviMap__Fv` | 1 |  | `sceVu0CopyVector` |
| `0058a4ec` | 0x59c4 | `AreaInfoMenu__10ccMenuCtrlFv` | `ccCheckMenuFaceName__Fi` | 8 |  | `InitCursol__12ccMenuWindowFi`, `ccSeOn__Fi`, `ccKanjiStrSeparate__FPci`, `MakePacket__8ccSpriteFii`, `Close__9ccMessageFv` |
| `00597ca8` | 0x68 | `ccSkillCostCheck__FP6ccCharP12ccSkillPar` | `ccSkillAttributeCheck__Fi` | 1 |  | `ccGetSkillParam__Fi` |
| `005ac3f0` | 0x700 | `CurePoisonSPC__4ccAIFv` | `ChatCommandDeBuffPlz__4ccAIFv` | 1 |  | `CheckSysMsgID__9ccSpcCharFv`, `CheckSolution__4ccAIFiP6ccChar`, `SysMsgFromMeToMe__4ccAIFiP6ccChariiP6ccChar`, `ccSkillRecoveryCheck__Fi`, `CheckSkillList__4ccAIFi` |
| `005b401c` | 0xe34 | `SearchHistoryMessageP__10ccAISystemFUiii` | `CalcEstimateDamage__10ccAISystemFP6ccCha` | 4 |  | `ccSkillCheckType__Fi`, `ccGetItemParam__Fi` |
| `005b5580` | 0x90 | `ccAISysMsgCheckEntryChar__Fi` | `ccAISysMsgDeleteDelay__FUiUsUs` | 3 |  |  |
| `005bb75c` | 0x524 | `ChatMessageWalkingTalk__4ccAIFv` | `Reconnoiter__4ccAIFv` | 6 |  | `ChatMessageModify__4ccAIFPcPcPcPc`, `ccGetItemParam__Fi` |
| `005c1a54` | 0x9ec | `CheckEyeLineToTarget__4ccAIFP6ccChar` | `getAttributeStr__Fii` | 4 |  | `DebuffForUnusedEnemy__4ccAIFi`, `GetFieldAttrb__9WORLD_MANFv`, `BuffForUnusedFellow__4ccAIFi`, `ccSkillCheckType__Fi`, `ccGetItemParam__Fi` |
| `005c24ac` | 0x494 | `getAttributeStr__Fii` | `ccThPlayer__FPv` | 1 |  | `ccGetEnemyParam__Fi`, `ccGetBossParam__Fi`, `ccCheckTarget__FP6ccChar`, `ccGetSkillParam__Fi`, `ccSkillAttributeCheck__Fi` |
| `005c82f4` | 0x7c | `checkPartyAnnihilation__Fv` | `checkPartyMenberNum__Fi` | 0 |  |  |
| `005cb554` | 0x12c | `DeleteNoPartyMember__5ccSPCFv` | `Wakeup__5ccSPCFv` | 1 |  | `ccDeleteCmnd__FP6ccChar`, `ccSleepThread__FP6ccTscb` |
| `005cbb68` | 0x38 | `ccSpcWakeup__Fv` | `ccSpcConditionEffectON__Fv` | 1 |  | `Wakeup__5ccSPCFv` |
| `005fbd88` | 0x10d8 | `Move__4BIRDFv` | `Draw__19ccBossEffLaserRain2Fv` | 10 | `town06`, `CMP_sr6met1`, `CMP_sr6bac1`, `CMP_sr6cro1` | `GetChunkAdrsF__8ccStreamFPCci`, `__nw__FUi`, `__ct__7ccCoordFv`, `Init__7ccClumpFP12ccClumpChunk`, `SetFogSw__7ccClumpFi` |
| `005fd100` | 0x2980 | `Draw__19ccBossEffLaserRain2Fv` | `-` | 15 | `xp_text`, `TEX_xp_tim_1`, `xp_cup`, `ANM_xp_count_a` | `__nw__FUi`, `ccBreathThread__Fi`, `GetChunkAdrsF__8ccStreamFPCci`, `SetAnm__5ccAnmFP10ccAnmChunkUi`, `CheckFade__8ccScFadeFi` |

### desktop

Text 0x1ae40 bytes, 0x1a850 named, 0x60 unnamed (0.1%) in 2 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00413840` | 0x40 | `-` | `ccThDesktopDelete__FPv` | 0 | (zero words: padding) |  |
| `0042e660` | 0x20 | `PlayKigou__17NameEntry_ControlFv` | `-` | 0 | (zero words: padding) |  |

### toppage

Text 0x4140 bytes, 0x3f78 named, 0x94 unnamed (0.9%) in 2 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00413840` | 0x70 | `-` | `ccThToppageDelete__FPv` | 1 |  | `ccAddFileList__FP10ccFileList` |
| `0041795c` | 0x24 | `SetPos__11ccScrollBarFi` | `-` | 0 | (zero words: padding) |  |

### demo

Text 0xd3c0 bytes, 0xd1f8 named, 0xec unnamed (0.4%) in 2 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00413840` | 0x90 | `-` | `ccThDemoDelete__FPv` | 1 |  | `ccAddFileListOne__FP10ccFileList` |
| `00419624` | 0x5c | `Delete__17ccOpening_ControlFv` | `__ct__17ccOpening_ControlFv` | 0 |  |  |

## Outbreak (SLUS_205.63)

### main

Text 0x283be4 bytes, 0xd0d3c named, 0x1b0c60 unnamed (67.2%) in 39 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00100000` | 0xb8 | `-` | `_exit` | 0 |  | `_InitSys`, `FlushCache`, `main` |
| `0010d800` | 0x180 | `sceDmaPutEnv` | `sceDmaGetEnv` | 0 |  |  |
| `0010f178` | 0x28 | `sceVu0RotMatrix` | `sceVu0CameraMatrix` | 0 |  |  |
| `0010f378` | 0x108 | `sceVu0LightColorMatrix` | `sceVu0DropShadowMatrix` | 1 |  | `sceVu0UnitMatrix`, `sceVu0MulMatrix` |
| `00113550` | 0xc8 | `_groupOfPicturesHeader` | `_pictureDisplayExtension` | 1 | `load_chroma_intra_quantizer_matrix == 1`, `load_chroma_non_intra_quantizer_matrix =` | `_nextBit`, `_waitIpuIdle`, `_sendIpuCommand`, `_Error` |
| `001176b0` | 0x68 | `sceIpuSync` | `sceIpuInit` | 1 |  | `DIntr` |
| `00117950` | 0x30 | `sceIpuInit` | `RFU000_FullReset` | 0 | (zero words: padding) |  |
| `0011ab18` | 0x158 | `_request_rdata` | `_search_svdata` | 1 |  | `_sceRpcFreePacket`, `sceSifSendCmd`, `DeleteSema`, `_sceRpcGetPacket`, `CreateSema` |
| `0011b2d8` | 0x98 | `sceSifRegisterRpc` | `sceSifRemoveRpcQueue` | 1 |  | `DIntr`, `EIntr` |
| `001200ec` | 0x6c | `SetDebugHandler` | `InitTLBFunctions` | 0 |  |  |
| `00120504` | 0x3c | `InitTLB32MB` | `_kTLBException` | 0 | (zero words: padding) |  |
| `0012065c` | 0x124 | `_kTLBException` | `_kDebugException` | 0 |  |  |
| `001209cc` | 0x14 | `GetSystemCallTableEntry` | `_setup` | 0 |  |  |
| `00120a3c` | 0x6c | `_InitSys` | `PatchIsNeeded` | 0 |  |  |
| `00120c98` | 0x38 | `Exit` | `sceSdRemoteInit` | 1 |  | `TerminateLibrary` |
| `00129348` | 0x70 | `cdvd_exit` | `_sceCd_Poff_Intr` | 1 |  | `PowerOffCB`, `DIntr`, `EIntr` |
| `001713bc` | 0x114 | `BootCheckProccess__9ccSaveSysFv` | `MainProccess__9ccSaveSysFv` | 1 |  | `CheckPort__7ccMcardFii`, `ReadSys__7ccMcardFiiPvii`, `CheckRightInfo__9ccSaveSysFP14ccSaveDataInfo` |
| `00179814` | 0x58c | `SetSoundEnv__10ccSaveDataFv` | `AddLvErosion__10ccSaveDataFif` | 0 |  |  |
| `0017a224` | 0x9cc | `ReadNewMail__10ccSaveDataFi` | `ccGetItemName__Fii` | 5 |  | `GetFrameRate__8ccSystemFv` |
| `0017bcbc` | 0x2c4 | `ccSqFade__FiUsiUc` | `ccSeOn__Fi` | 2 |  | `ccSqPlayVol__FiUs` |
| `00184150` | 0x480 | `ccSndMoviePlayer__Fii` | `ccLoad_SndModules__Fv` | 1 |  | `ccBreathThread__Fi`, `ccSndCmd__Fii` |
| `00187a58` | 0xcd8 | `ccSndGetStatus__Fv` | `SetObj__10ccEventObjFUi` | 5 |  | `__nw__FUi`, `__construct_array`, `__ct__8ccSpriteFv`, `Init__7ccKanjiFii`, `__dt__7ccKanjiFv` |
| `00188928` | 0x7a8 | `Ctrl__12ccPartCreateFv` | `ccSetStreamDemoNote__FP9ccAnmNote` | 2 |  | `__nw__FUi`, `rand`, `MakePacket__16ccBufferSamplingFP7ccLayer`, `__ct__16ccBufferSamplingFv`, `SendPacket__8ccSpriteFv` |
| `001891e0` | 0xb30 | `ccGetStreamDemoMsg__Fv` | `Func_str9001sub__FUiP15ccStrEffectCtrlP8` | 2 |  | `__dl__FPv`, `rand`, `DeleteFade__8ccScFadeFi`, `EntryFade__8ccScFadeFiiiffff`, `Breath__6ccTscbFi` |
| `0018b9c0` | 0xa60 | `Func_str0001__FP6ccTscb` | `Func_str0240__FP6ccTscb` | 2 |  | `__dl__FPv`, `Breath__6ccTscbFi`, `DeleteFade__8ccScFadeFi`, `EntryFade__8ccScFadeFiiiffff`, `__dt__7ccLayerFv` |
| `00191f3c` | 0x374 | `Func_str0120__FP6ccTscb` | `Func_str0610__FP6ccTscb` | 1 |  | `__dl__FPv`, `SetFrame__6ccViewFffffffff`, `Breath__6ccTscbFi`, `__dt__7ccLayerFv`, `__nw__FUi` |
| `00192960` | 0xfb60 | `Func_str0610__FP6ccTscb` | `ccInitStreamDemoThread__Fv` | 38 | `str0710e`, `str1070e`, `str1430e`, `EFF_x001` | `__dl__FPv`, `__dt__7ccLayerFv`, `Breath__6ccTscbFi`, `__nw__FUi`, `DeleteFade__8ccScFadeFi` |
| `001a277c` | 0xb4 | `__dt__13ccEffPart0580Fv` | `__dt__19ccObjPartCreate0580Fv` | 1 |  | `__dt__5ccObjFv`, `__dl__FPv` |
| `001a28bc` | 0x94 | `__dt__19ccObjPartCreate0580Fv` | `__dt__13ccObjPart0580Fv` | 1 |  | `__dl__FPv` |
| `001a2b50` | 0x1a0 | `__ct__16ccStreamLoadPlayFv` | `WaitEnd__16ccStreamLoadPlayFv` | 1 |  |  |
| `001a7398` | 0x128 | `Quit__9WORLD_MANFv` | `Init__9WORLD_MANFv` | 0 |  |  |
| `001ad290` | 0x30 | `SetLightDirection__9WORLD_MANFP14ccDista` | `SleepDistantLight__9WORLD_MANFv` | 1 |  | `sceVu0CopyVector` |
| `001ad2d4` | 0x2c | `SleepDistantLight__9WORLD_MANFv` | `Get2DPos__9WORLD_MANFPfPf` | 0 |  |  |
| `001bdd80` | 0xc0 | `Execute__7ccEventFRPsiii` | `MenuBan__7ccEventFv` | 1 |  | `ChangeInfo__9ccMessageFPcPcPcPcii` |
| `001c42b4` | 0x190 | `SetType__12ccMenuWindowFi` | `DispLine__12ccMenuWindowFiiPc` | 1 |  | `MakePacket__8ccSpriteFii`, `SetType__12ccMenuWindowFi` |
| `001cf198` | 0x278 | `ccSetQuaternion__FPfPff` | `ccQuaternionToMatrix__FPA4_fPf` | 3 |  | `fptodp`, `dptofp`, `sceVu0CopyVector`, `acosf`, `sceVu0OuterProduct` |
| `001d025c` | 0x94 | `ccNewEffect__Fi` | `InitEffect__8ccEffectFii` | 1 |  | `InitEffect__9ccEffect2Fi` |
| `001d8afc` | 0x14 | `Main__8ccEffectFv` | `FadeIn__8ccEffectFii` | 0 |  |  |
| `001e7c3c` | 0x19b844 | `DEG2RAD__Fs` | `__sinit_libcc3d.cpp` | 10 |  | `effWork+0x10fa8`, `DispLine__12ccMenuWindowFiiPc`, `__register_global_object`, `str1610Tbl+0x18`, `ALCNumTbl+0x12b4` |

### gcmn

Text 0x1ee340 bytes, 0x19d3dc named, 0x4be00 unnamed (15.4%) in 125 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `0040f240` | 0x40 | `-` | `ccThDfCompDelete__FPv` | 0 | (zero words: padding) |  |
| `0040fae0` | 0x80 | `__dt__9EVENTAREAFv` | `__ct__11EVENTAREA01Fv` | 0 |  |  |
| `004101d0` | 0x5e0 | `__ct__11EVENTAREA01Fv` | `__ct__11EVENTAREA02Fv` | 6 |  | `Draw__11STATICMODELFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `fieldrand__FUi` |
| `00410b80` | 0xc0 | `__ct__11EVENTAREA02Fv` | `ChangeBlock__11EVENTAREA02Fi` | 1 |  | `__dl__FPv`, `__dt__5ccAnmFv`, `__dt__9EVENTAREAFv` |
| `00411b00` | 0x270 | `DrawBG__11EVENTAREA02Fv` | `Draw__11EVENTAREA02Fv` | 3 |  | `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `Draw__12STATICOBJECTFv` |
| `00412320` | 0x1d0 | `__ct__11EVENTAREA03Fv` | `Draw__11EVENTAREA03Fv` | 3 |  | `__dt__9EVENTAREAFv`, `__dl__FPv`, `SetMatrix_PosRotZYX__7ccCoordFPfPf`, `Draw__7ccClumpFv`, `LockBlt__13ccBltGrpChunkFv` |
| `00412820` | 0x160 | `Draw__11EVENTAREA03Fv` | `__ct__11EVENTAREA04Fv` | 2 |  | `__ct__9EVENTAREAFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `00413040` | 0xd0 | `__ct__11EVENTAREA04Fv` | `DrawBG__11EVENTAREA04Fv` | 1 |  | `__dt__9LENSFLAREFv`, `__dt__5CLOUDFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `004131f0` | 0x4a0 | `DrawBG__11EVENTAREA04Fv` | `__ct__11EVENTAREA05Fv` | 4 | `DMY_dummy5` | `Draw__11STATICMODELFv`, `Draw__12STATICOBJECTFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer` |
| `00413cb0` | 0xd0 | `__ct__11EVENTAREA05Fv` | `DrawBG__11EVENTAREA05Fv` | 1 |  | `__dt__5CLOUDFv`, `__dt__9LENSFLAREFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `00413e60` | 0x4c0 | `DrawBG__11EVENTAREA05Fv` | `__ct__11EVENTAREA07Fv` | 4 | `DMY_dummy5` | `Draw__11STATICMODELFv`, `Draw__12STATICOBJECTFv`, `SetActiveLayer__9WORLD_MANFi`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv` |
| `00414480` | 0xd0 | `__ct__11EVENTAREA07Fv` | `ChangeBlock__11EVENTAREA07Fi` | 1 |  | `__dt__5CLOUDFv`, `__dt__9LENSFLAREFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `004151c0` | 0x300 | `DrawBG__11EVENTAREA07Fv` | `Draw__11EVENTAREA07Fv` | 3 |  | `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `Draw__12STATICOBJECTFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer` |
| `00415c40` | 0xc0 | `__ct__11EVENTAREA06Fv` | `DrawBG__11EVENTAREA06Fv` | 1 |  | `__dt__5CLOUDFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `00415de0` | 0x4b0 | `DrawBG__11EVENTAREA06Fv` | `__ct__11EVENTAREAB0Fv` | 4 |  | `Draw__11STATICMODELFv`, `Draw__12STATICOBJECTFv`, `SetActiveLayer__9WORLD_MANFi`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv` |
| `004178d0` | 0xe0 | `__ct__11EVENTAREAB0Fv` | `DrawBG__11EVENTAREAB0Fv` | 1 |  | `__dt__8FIREFLY2Fv`, `__dt__5ccAnmFv`, `__dt__7ccClumpFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `00417c40` | 0x680 | `DrawBG__11EVENTAREAB0Fv` | `SetFloatRockParam__FP9FLOATROCK` | 4 |  | `SetActiveLayer__9WORLD_MANFi`, `Draw__11STATICMODELFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer` |
| `00418370` | 0x500 | `SetFloatRockParam__FP9FLOATROCK` | `__ct__11EVENTAREAB8Fv` | 0 |  | `fieldrand__FUi`, `ccRand__Fv` |
| `004197e0` | 0x10f0 | `__ct__11EVENTAREAB8Fv` | `SetCharInfo__FP8CHARINFO` | 9 | `MAT_se1_6clo1`, `CAM_camera2_4`, `OBJ_dummy2_4`, `CAM_camera3_3` | `_AnimateForward__5ccAnmFUi`, `Draw__7ccClumpFv`, `GetSubstAdrsF__5ccAnmFPCcb`, `SetMatrix_PosRotZYX__7ccCoordFPfPf`, `SetActiveLayer__9WORLD_MANFi` |
| `0041e034` | 0x2dc | `GetBook04Item__4BOOKFP8BOOKITEMiiii` | `GetBook06Item__4BOOKFP8BOOKITEMiiii` | 1 | `%s%d%s` | `Draw__4BOOKFv`, `ccBreathThread__Fi`, `memset`, `OpenInfo__9ccMessageFPcPcPcPcii`, `Check__9ccMessageFi` |
| `0042dc40` | 0x130 | `ccFellowCheckNote__FP9ccAnmNote` | `Attack__8ccFellowFP6ccChari` | 1 |  | `DistanceToTarget__9ccSpcCharFP6ccChar`, `ccSkillCostCheck__FP6ccChari`, `ccSkillRangeCheck__Fif`, `ccSkillRequest__FP6ccCharP6ccChari` |
| `0042e310` | 0x1690 | `Attack__8ccFellowFP6ccChari` | `ccThFellow10__FPv` | 36 | `SPC_01`, `SPC_02`, `SPC_03`, `SPC_04` | `Breath__6ccTscbFi`, `expulsionSpc__Fi`, `ccDeleteThread__FP6ccTscb`, `__nw__FUi`, `ccStartThread__FPFPv_vii` |
| `0042fa50` | 0x1350 | `ccThFellow10__FPv` | `__ct__10ROOTTOWN01Fv` | 31 | `SPC_11`, `SPC_12`, `SPC_13`, `SPC_14` | `__nw__FUi`, `ccStartThread__FPFPv_vii`, `__dt__8ccFellowFv`, `__dl__FPv`, `DelSpc__5ccSPCFi` |
| `00431970` | 0xd0 | `__ct__10ROOTTOWN01Fv` | `DrawBG__10ROOTTOWN01Fv` | 1 |  | `__dt__5ccAnmFv`, `__dt__10ccTexChunkFv`, `__dt__8ROOTTOWNFv`, `__dl__FPv` |
| `00431cd0` | 0x7f0 | `DrawBG__10ROOTTOWN01Fv` | `DrawMap__10ROOTTOWN01Fv` | 3 |  | `Draw__11STATICMODELFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `Draw__12STATICOBJECTFv` |
| `004332f0` | 0x5a0 | `DrawMap__10ROOTTOWN01Fv` | `__ct__10ROOTTOWN02Fv` | 1 |  | `SetActiveLayer__9WORLD_MANFi`, `_AnimateForward__5ccAnmFUi`, `Draw__5ccAnmFv`, `sceVu0CopyVector`, `SetUV__5ccAnmFiiP15ccMaterialChunki` |
| `004347f0` | 0x170 | `__ct__10ROOTTOWN02Fv` | `DrawBG__10ROOTTOWN02Fv` | 1 |  | `__dt__7ccClumpFv`, `__dt__7ccLayerFv`, `__dt__9LENSFLAREFv`, `__dt__5CLOUDFv`, `__dt__5ccAnmFv` |
| `00434da0` | 0x280 | `DrawBG__10ROOTTOWN02Fv` | `DrawMap__10ROOTTOWN02Fv` | 3 |  | `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `Draw__12STATICOBJECTFv` |
| `004363a0` | 0x3f0 | `DrawMap__10ROOTTOWN02Fv` | `__ct__7AIRSHIPFP8ccStream` | 1 | `OBJ_sr2wat00`, `DMY_sr2lig_1point` | `SetActiveLayer__9WORLD_MANFi`, `_AnimateForward__5ccAnmFUi`, `Draw__5ccAnmFv`, `SetUV__5ccAnmFiiP15ccMaterialChunki`, `Init__5CLOUDFP8ccStreami` |
| `00436940` | 0x8f0 | `__ct__7AIRSHIPFP8ccStream` | `__ct__10ROOTTOWN03Fv` | 1 |  | `sceVu0CopyVector`, `fieldrand__FUi`, `sceVu0ApplyMatrix`, `sceVu0AddVector`, `sceVu0UnitMatrix` |
| `004380d0` | 0x160 | `__ct__10ROOTTOWN03Fv` | `DrawBG__10ROOTTOWN03Fv` | 1 |  | `__dt__7ccClumpFv`, `__dt__5ccAnmFv`, `__dt__7ccLayerFv`, `__dl__FPv`, `__dt__10ccTexChunkFv` |
| `004384e0` | 0x1f0 | `DrawBG__10ROOTTOWN03Fv` | `DrawMap__10ROOTTOWN03Fv` | 3 |  | `Draw__11STATICMODELFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer` |
| `00439a10` | 0x240 | `DrawMap__10ROOTTOWN03Fv` | `__ct__10ROOTTOWN04Fv` | 1 | `OBJ_sr3wat00` | `DrawWithOutFog__11STATICMODELFv`, `SetActiveLayer__9WORLD_MANFi`, `Draw__5ccAnmFv`, `cameraGetPos__FPfi`, `GetSubstAdrsF__5ccAnmFPCcb` |
| `0043a900` | 0x140 | `__ct__10ROOTTOWN04Fv` | `DrawBG__10ROOTTOWN04Fv` | 1 |  | `__dt__7ccClumpFv`, `__dt__7ccLayerFv`, `__dt__9LENSFLAREFv`, `__dt__5CLOUDFv`, `__dt__8ROOTTOWNFv` |
| `0043ae50` | 0x280 | `DrawBG__10ROOTTOWN04Fv` | `DrawMap__10ROOTTOWN04Fv` | 3 |  | `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `Draw__12STATICOBJECTFv` |
| `0043c430` | 0x1b0 | `DrawMap__10ROOTTOWN04Fv` | `__ct__10ROOTTOWN05Fv` | 1 | `DMY_sr4lig1point` | `SetActiveLayer__9WORLD_MANFi`, `Init__5CLOUDFP8ccStreami`, `cameraGetPos__FPfi`, `Draw__9LENSFLAREFP8ccStreamPci`, `Move__5CLOUDFv` |
| `0043d0c0` | 0x490 | `__ct__10ROOTTOWN05Fv` | `DrawMap__10ROOTTOWN05Fv` | 5 |  | `__dt__7ccClumpFv`, `__dl__FPv`, `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `UnlockBlt__13ccBltGrpChunkFv` |
| `0043e8e0` | 0x220 | `DrawMap__10ROOTTOWN05Fv` | `ccCheckActiveEnemy__Fv` | 1 |  | `SetActiveLayer__9WORLD_MANFi`, `fieldrand__FUi`, `Draw__5ccEffFPfUs`, `Draw__11STATICMODELFv` |
| `0044171c` | 0x54 | `deleteAllObject__11ccEntryCtrlFv` | `initObject__11ccEntryCtrlFP10ccEntryObj` | 1 |  | `deleteAllEnemy__11ccEntryCtrlFv`, `deleteAllMagicCircle__11ccEntryCtrlFv`, `deleteAllGimmick__11ccEntryCtrlFv`, `deleteAllNpc__11ccEntryCtrlFv` |
| `00441c28` | 0xc8 | `deleteEnemy__11ccEntryCtrlFP10ccEntryObj` | `main__10ccEntryObjFv` | 1 |  | `routine__10ccEntryObjFv`, `deleteEnemy__11ccEntryCtrlFP10ccEntryObj` |
| `00441f18` | 0xc8 | `deleteMagicCircle__11ccEntryCtrlFP10ccEn` | `deleteAllMagicCircle__11ccEntryCtrlFv` | 1 |  | `routine__10ccEntryObjFv`, `deleteMagicCircle__11ccEntryCtrlFP10ccEntryObj` |
| `004421a8` | 0xc8 | `deleteGimmick__11ccEntryCtrlFP10ccEntryO` | `deleteAllGimmick__11ccEntryCtrlFv` | 1 |  | `routine__10ccEntryObjFv`, `deleteGimmick__11ccEntryCtrlFP10ccEntryObj` |
| `00442438` | 0xc8 | `deleteNpc__11ccEntryCtrlFP10ccEntryObj` | `deleteAllNpc__11ccEntryCtrlFv` | 1 |  | `routine__10ccEntryObjFv`, `deleteNpc__11ccEntryCtrlFP10ccEntryObj` |
| `00459f24` | 0x43c | `actEscapeGold__8ccEnemyGFf` | `__ct__8ccEnemyHFP7ccEntry` | 4 |  | `ccSeOn3D__FiPf`, `ccRand__Fv`, `ctrl__15ccEnemyDustCtrlFP10ccEntryObji`, `ctrl__17ccEnemyWeaponCtrlFP10ccEntryObj`, `sceVu0CopyVector` |
| `0045a750` | 0x5a0 | `__ct__8ccEnemyHFP7ccEntry` | `exclEHK__8ccEnemyHFv` | 4 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `defaultThink__7ccEnemyFv`, `ccRand__Fv`, `ccSetRadDisperse__FPff` |
| `0045adc0` | 0xf0 | `exclEHK__8ccEnemyHFv` | `__ct__8ccEnemyIFP7ccEntry` | 3 |  | `note__17ccEnemyWeaponCtrlFP10ccEntryObjP9ccAnmNo`, `__nw__FUi`, `__ct__8ccEnemyIFP7ccEntry`, `__dt__17ccEnemyWeaponCtrlFv`, `__dt__15ccEnemyDustCtrlFv` |
| `0045b080` | 0x720 | `__ct__8ccEnemyIFP7ccEntry` | `__ct__8ccEnemyKFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `ctrl__15ccEnemyDustCtrlFP10ccEntryObji`, `defaultThink__7ccEnemyFv` |
| `0045ba20` | 0x840 | `__ct__8ccEnemyKFP7ccEntry` | `__ct__8ccEnemyLFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `ctrl__15ccEnemyDustCtrlFP10ccEntryObji`, `defaultThink__7ccEnemyFv` |
| `0045ca30` | 0x9b0 | `__ct__8ccEnemyLFP7ccEntry` | `exclELW__8ccEnemyLFv` | 5 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccSetRadDisperse__FPff`, `defaultThink__7ccEnemyFv`, `ccRand__Fv` |
| `0045d480` | 0x18c0 | `exclELW__8ccEnemyLFv` | `initELG__8ccEnemyLFv` | 7 |  | `DEG2RAD__Fs`, `sinf`, `ccRand__Fv`, `SetRenderState__7ccModelF20CC_RENDER_STATE_TYPEi`, `fptodp` |
| `0045f040` | 0x600 | `initELG__8ccEnemyLFv` | `__ct__8ccEnemyPFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `Draw__6ccCharFv`, `SetMatrix_PosRotZYX__7ccCoordFPfPf`, `ccSetRadDisperse__FPff` |
| `0045f910` | 0x710 | `__ct__8ccEnemyPFP7ccEntry` | `__ct__8ccEnemySFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `defaultThink__7ccEnemyFv`, `ccSetRadDisperse__FPff` |
| `004602e0` | 0x830 | `__ct__8ccEnemySFP7ccEntry` | `__ct__8ccEnemyTFP7ccEntry` | 7 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `defaultThink__7ccEnemyFv`, `ccRand__Fv`, `ccSetRadDisperse__FPff` |
| `00460cd0` | 0x880 | `__ct__8ccEnemyTFP7ccEntry` | `__ct__8ccEnemyUFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `defaultThink__7ccEnemyFv`, `ccSetRadDisperse__FPff`, `actEscape__7ccEnemyFv` |
| `00461a80` | 0xbc0 | `__ct__8ccEnemyUFP7ccEntry` | `__ct__8ccEnemyVFP7ccEntry` | 7 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `actEscape__7ccEnemyFif`, `RAD2DEG__Ff` |
| `00462a70` | 0x7e0 | `__ct__8ccEnemyVFP7ccEntry` | `__ct__8ccEnemyWFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `defaultThink__7ccEnemyFv`, `ccSetRadDisperse__FPff` |
| `00463510` | 0x7d0 | `__ct__8ccEnemyWFP7ccEntry` | `__ct__8ccEnemyZFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `RAD2DEG__Ff`, `DEG2RAD__Fs` |
| `00463ff0` | 0x600 | `__ct__8ccEnemyZFP7ccEntry` | `__ct__9ccGimmickFv` | 5 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ctrl__15ccEnemyDustCtrlFP10ccEntryObji`, `defaultThink__7ccEnemyFv`, `ccRand__Fv` |
| `0046f058` | 0x28 | `DrawAfterImages__6ccBossFv` | `SetBlendTypeAfterImage__6ccBossFUc` | 0 |  |  |
| `00472544` | 0x64c | `__dt__9ccBossCamFv` | `ccBoss03BlurParamCB__FPfPfPUi` | 0 |  |  |
| `004733a4` | 0x9c | `OnCinemaMode__16ccBossEffManagerFi` | `OffCinemaMode__16ccBossEffManagerFv` | 1 |  | `checkPartyAnnihilation__Fv` |
| `0047c090` | 0x70 | `CinemaOn__19ccBossEffCinemaFadeFi` | `CinemaOff__19ccBossEffCinemaFadeFv` | 1 |  | `SetupSkillName__19ccBossEffCinemaFadeFP17CINEMAS` |
| `0047ca18` | 0x128 | `Draw__19ccBossEffCinemaFadeFv` | `__dt__18ccBossEffWaveShockFv` | 1 | `xeffect`, `ANM_ex31lhit` | `GetCCSAdrs__8ccStreamFPCc`, `sceVu0CopyVector`, `__nw__FUi`, `__ct__5ccAnmFv`, `GetChunkAdrsF__8ccStreamFPCci` |
| `00481da0` | 0x160 | `Draw__18ccBossEffLaserShotFv` | `__dt__23ccBossEffLightBallShockFv` | 1 | `xeffect`, `CMP_ex31exp1` | `GetCCSAdrs__8ccStreamFPCc`, `__nw__FUi`, `__ct__7ccCoordFv`, `GetChunkAdrsF__8ccStreamFPCci`, `Init__7ccClumpFP12ccClumpChunk` |
| `00485648` | 0x668 | `__dt__20ccBossEffEnergyGrow2Fv` | `Draw__14ccBossEffDrainFv` | 5 | `particle` | `ccRandF__Ff`, `sceVu0CopyVector`, `__nw__FUi`, `__ct__19ccParticleGeneratorFP24ccParticleGenerat`, `startParticleGenerator__FP19ccParticleGenerator` |
| `00499ce8` | 0x228 | `__dt__8ccBoss03Fv` | `CalcCamera__8ccBoss03Fv` | 2 |  | `GetSubstAdrsF__5ccAnmFPCcb` |
| `0049a944` | 0x18c | `SetupLaserShock__8ccBoss03Fv` | `Main__8ccBoss03Fv` | 1 |  | `sceVu0AddVector`, `sceVu0CopyVector`, `ccTransPosW2P__FPfPf`, `sinf`, `cosf` |
| `0049d210` | 0x390 | `OnThinkDataDrainAtk__8ccBoss03Fv` | `OnThinkNeedle__8ccBoss03Fv` | 1 |  | `sceVu0RotMatrixZ`, `changeCamera__Fi`, `ccCheckTarget__FP6ccChar`, `BeginStageEffect__6ccBossFiiii`, `LockPlayer__6ccBossFi` |
| `0049e54c` | 0x254 | `OnThinkWave__8ccBoss03Fv` | `OnThinkLeafDrop__8ccBoss03Fv` | 1 |  | `EraseCmndTarget__6ccBossFv`, `sceVu0CopyVector`, `EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T`, `ccTransPosW2P__FPfPf`, `ccGetDist__FPfPf` |
| `004b2574` | 0x21c | `OnThinkChase__8ccBoss06Fv` | `OnThinkSkill__8ccBoss06Fv` | 1 |  | `ccSeOn3D__FiPf`, `EraseCmndTarget__6ccBossFv`, `EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T`, `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector` |
| `004c3260` | 0x3f0 | `OnThinkBodyPress__10ccBoss08_bFv` | `OnThinkMagic__10ccBoss08_bFv` | 2 |  | `LockPlayer__6ccBossFi`, `EraseCmndTarget__6ccBossFv`, `CheckMenuType__10ccMenuCtrlFv`, `UnlockPlayer__6ccBossFv`, `sceVu0CopyVector` |
| `004c3e34` | 0x21c | `OnThinkSetupBarrier__10ccBoss08_bFv` | `OnThinkRandomDrive__10ccBoss08_bFv` | 1 |  | `ccSeOn3D__FiPf`, `EraseCmndTarget__6ccBossFv`, `EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T`, `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector` |
| `004c78f8` | 0x778 | `OnThinkNeutral__10ccBoss08_cFv` | `OnThinkDmg__10ccBoss08_cFv` | 2 |  | `sceVu0CopyVector`, `sceVu0RotMatrixZ`, `ccDeleteCmnd__FP6ccChar`, `__nw__FUi`, `__ct__19ccParticleGeneratorFP24ccParticleGenerat` |
| `004cd074` | 0xdc | `Affect__17ccBoss08KillerEyeFv` | `Generate__17ccBoss08KillerEyeFv` | 1 |  | `SetMatrix_PosRotZYX__7ccCoordFPfPf`, `_AnimateForward__5ccAnmFUi`, `PreDrawAnm__6ccBossFv` |
| `004cd7cc` | 0x344 | `ExecPatternIndex__17ccBoss08KillerEyeFPi` | `OnThinkGenerate__17ccBoss08KillerEyeFv` | 2 |  | `ccSetDirc__FPffi`, `checkPartyAnnihilation__Fv`, `ccCheckTarget__FP6ccChar` |
| `004ced30` | 0x120 | `OnThinkDead__17ccBoss08KillerEyeFv` | `__ct__13ccBossKyvia01Fi` | 2 | `Kyvia01` | `Breath__6ccTscbFi`, `__nw__FUi`, `__ct__13ccBossKyvia01Fi` |
| `004cf510` | 0x6430 | `__ct__13ccBossKyvia01Fi` | `__ct__13ccBossKyvia02Fi` | 19 | `ANM_ex0batc0`, `Kyvia02` | `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector`, `ccRandF__Ff`, `SetMode__9ccBossCamFifPfP6ccChar`, `sceVu0AddVector` |
| `004d6010` | 0x8140 | `__ct__13ccBossKyvia02Fi` | `ccThKyvia03__FP6ccTscb` | 21 | `ANM_ex0batc0` | `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector`, `ccSeOn__Fi`, `ccRandF__Ff`, `EntryFlash__8ccScFadeFiiffff` |
| `004de900` | 0x8a30 | `__ct__13ccBossKyvia03Fi` | `__ct__13ccBossKyvia04Fi` | 24 | `ANM_ex0batc0`, `Kyvia04` | `ccSeOn__Fi`, `ccSeOnNote__Fic`, `sceVu0CopyVector`, `ccRandF__Ff`, `SetMode__9ccBossCamFifPfP6ccChar` |
| `004e7ce0` | 0x2780 | `__ct__13ccBossKyvia04Fi` | `CheckDiscMove__13ccBossKyvia04Fv` | 11 |  | `sceVu0CopyVector`, `ccSeOnNote__Fic`, `sceVu0AddVector`, `__nw__FUi`, `__ct__19ccParticleGeneratorFP24ccParticleGenerat` |
| `004ea630` | 0xac0 | `CheckDiscMove__13ccBossKyvia04Fv` | `KyviaScene__13ccBossKyvia04Fv` | 3 |  | `OnCinemaMode__16ccBossEffManagerFi`, `ParticleKill__19ccParticleGeneratorFv`, `UnlockPlayer__6ccBossFv`, `sceVu0CopyVector`, `EntryFlash3__8ccScFadeFiiiiffff` |
| `004eb530` | 0xcd0 | `KyviaScene__13ccBossKyvia04Fv` | `KyviaRndTarget__13ccBossKyvia04Fv` | 3 |  | `ParticleKill__19ccParticleGeneratorFv`, `sceVu0CopyVector`, `__nw__FUi`, `__ct__19ccParticleGeneratorFP24ccParticleGenerat`, `startParticleGenerator__FP19ccParticleGenerator` |
| `004ec350` | 0x7160 | `KyviaRndTarget__13ccBossKyvia04Fv` | `Skill_Atk__13ccBossKyvia04Fv` | 15 | `ANM_ex0batc0` | `sceVu0CopyVector`, `SetMode__9ccBossCamFifPfP6ccChar`, `ccRandF__Ff`, `ccSeOn3DNote__FiPfc`, `sceVu0AddVector` |
| `004f39e0` | 0xe80 | `Skill_Atk__13ccBossKyvia04Fv` | `InitData__9kyviaCoreFv` | 4 |  | `ccSeOn__Fi`, `EntryFlash__8ccScFadeFiiffff`, `ccSeOnNote__Fic`, `ccBossSkillDamage__FP6ccBossPfi`, `SetEnterPos__9kyviaCoreFPf` |
| `004f4a60` | 0x70 | `InitData__9kyviaCoreFv` | `ResetData__9kyviaCoreFv` | 0 |  |  |
| `00501b2c` | 0x154 | `__dt__13ccMoveElementFv` | `__dt__13ccFallElementFv` | 1 |  | `memset` |
| `0050527c` | 0x1b4 | `Main__22ccSummonsSystemElementFv` | `__dt__25ccDarkUpheavalMngrElementFv` | 1 |  | `sceVu0CopyVector`, `ccTransPosW2P__FPfPf`, `ccTransPosP2W__FPfPf`, `ccHitCheckLM2__FPfPfUi` |
| `0050ae6c` | 0x1e4 | `__ct__Q220ccDarkSummonsElement6BALL_TFv` | `__dt__Q220ccDarkSummonsElement9ELEMENT_T` | 1 |  | `memset` |
| `005173c0` | 0x1b0 | `Draw__20ccThunderBoltElementFv` | `__dt__24ccResistantShieldElementFv` | 1 | `particle`, `ANM_x070`, `ANM_x069` | `SetAnm__5ccAnmFP8ccStreamPcUi`, `memset`, `GetCCSAdrs__8ccStreamFPCc`, `__nw__FUi`, `__ct__5ccAnmFv` |
| `0051c7a4` | 0x39c | `breederInfluence__FP6ccChar` | `ccSetRtownPC__Fii` | 1 |  | `GetChunkAdrsF__8ccStreamFPCci`, `SetAnm__5ccAnmFP10ccAnmChunkUi`, `ccSetDirc__FPffi`, `effTransfer__FP6ccChar`, `deleteCmnd__10ccEntryObjFi` |
| `0051ccb0` | 0x1b0 | `ccSetRtownPC__Fii` | `ccEntryRtownPC__FP7ccEntry` | 1 |  | `sceVu0CopyVector`, `ccEntryParamClear__FP12ccEntryParam`, `ccNaviGetLandmarkPos__FPfi`, `GetChunkAdrsF__8ccStreamFPCci`, `entryObject__11ccEntryCtrlFP12ccEntryParam` |
| `0051f0b4` | 0x99c | `normalMode__9ccRtownPCFv` | `eventMode__9ccRtownPCFv` | 1 |  | `GetChunkAdrsF__8ccStreamFPCci`, `SetAnm__5ccAnmFP10ccAnmChunkUi`, `HitDisable__9ccCharHitFv`, `changeRoute__9ccRtownPCFi`, `ccEntryCmnd__FP6ccChar` |
| `00526e6c` | 0x3a4 | `ccSetChibiGuso__Fv` | `dogAction2__FP6ccChar` | 1 |  | `sceVu0CopyVector`, `CloseChat__9ccChatMsgFv`, `changeCamera__Fi`, `ccGetDirc0__FPfPf`, `cameraSetPos__FPfi` |
| `005279d0` | 0x90 | `dogActionAdult__FP6ccChar` | `burpEff__7ccPGusoFv` | 1 |  | `effect__7ccPGusoFv`, `ccSeSetParamInu__FUiP6ccChar` |
| `0052a1e4` | 0xa2c | `ControlMove__11ccPuccigusoFv` | `PadLeverPower__11ccPuccigusoFf` | 5 |  | `sceVu0InnerProduct`, `sceVu0CopyVector`, `ccTransPosW2P__FPfPf`, `PadLeverPower__11ccPuccigusoFf`, `fptodp` |
| `0052beb8` | 0x1c8 | `DrawPG__11ccPuccigusoFv` | `ccPuccigusoCheckNote__FP9ccAnmNote` | 2 |  | `strcpy`, `OpenChat__9ccChatMsgFP6ccCharPc`, `getPartyMenberChar__Fi` |
| `0052c184` | 0x8c | `ccPuccigusoCheckNote__FP9ccAnmNote` | `ccSetNaviMap__Fv` | 1 |  | `sceVu0CopyVector` |
| `0053b3c0` | 0x2a0 | `Disp__10ccMenuCtrlFv` | `SetPanelBure__10ccMenuCtrlFis` | 0 |  |  |
| `00556518` | 0xa58 | `PartyOutMenu__10ccMenuCtrlFv` | `ItemStatusMenu__10ccMenuCtrlFv` | 2 |  | `InitCursol__12ccMenuWindowFi`, `Close__9ccMessageFv`, `OpenInfo__9ccMessageFPcPcPcPcii`, `ccSeOn__Fi`, `Disp__10ccMenuCtrlFv` |
| `00557304` | 0x19ec | `ItemStatusMenu__10ccMenuCtrlFv` | `GateoutMenu__10ccMenuCtrlFv` | 3 | `xcontrol`, `TEX_xcontrol`, `CONTROLLER LOAD` | `MakePacket__8ccSpriteFii`, `SetType__12ccMenuWindowFi`, `DispSquare__12ccMenuWindowFiiPc`, `Extract__7ccKanjiFPc`, `ccKanjiStrSeparate__FPci` |
| `00559004` | 0x2dc | `GateoutMenu__10ccMenuCtrlFv` | `LogoutMenu__10ccMenuCtrlFv` | 1 |  | `DispSquare__12ccMenuWindowFiiPc`, `SetClm__7ccKanjiFiiii`, `DispSelectCursol__12ccMenuWindowFffiiii`, `MakePacket__8ccSpriteFii` |
| `005594d4` | 0x99c | `LogoutMenu__10ccMenuCtrlFv` | `DataDrainDemoMenuDisp__10ccMenuCtrlFv` | 2 |  | `MakePacket__8ccSpriteFii`, `ccSeOn__Fi`, `MakePacketStr__8ccSpriteFPci`, `MakeSignedNum__6ccFontFil`, `InitCursol__12ccMenuWindowFi` |
| `00560530` | 0x610 | `FairyshopMenu__10ccMenuCtrlFv` | `BreederMenu__10ccMenuCtrlFv` | 2 |  | `InitCursol__12ccMenuWindowFi`, `ccSeOn__Fi`, `ccKanjiStrSeparate__FPci`, `SetItemList__10ccMenuCtrlFiiP10ccItemListi`, `SelectScr__10ccMenuCtrlFiii` |
| `0057a590` | 0x150 | `ItemDepositMenu__10ccMenuCtrlFv` | `FountainMenuDisp3__10ccMenuCtrlFv` | 1 |  |  |
| `0057f998` | 0x418 | `BreedingMenu__10ccMenuCtrlFv` | `GtRandomMenu__10ccMenuCtrlFv` | 1 | `WORD` | `strcat`, `GetWordParamPtr__9WORLD_MANFi`, `ccKanjiStrlen__FPc`, `ccKanjiStrcat__FPcPci`, `FountainMenuDisp3__10ccMenuCtrlFv` |
| `00580778` | 0x208 | `GtRandomMenu__10ccMenuCtrlFv` | `__ct__10ccHackMenuFP8ccStream` | 1 |  | `SetType__12ccMenuWindowFi`, `MakePacket__8ccSpriteFii` |
| `00581880` | 0x330 | `Draw__10ccHackMenuFv` | `GtNewMenu__10ccMenuCtrlFv` | 1 |  | `ccSeOn__Fi`, `CheckOperate__7ccEventFii`, `GetEventAreaInfo__9WORLD_MANFi`, `GetItemNum__10ccSaveDataFiii` |
| `00582e1c` | 0xfa4 | `PartyDisbandMenu__10ccMenuCtrlFv` | `PersonalMenuT__10ccMenuCtrlFv` | 3 |  | `ccSeOn__Fi`, `Disp__10ccMenuCtrlFv`, `ccBreathThread__Fi`, `Close__9ccMessageFv`, `Change__9ccMessageFP11ccEvMsgDataPcii` |
| `00586440` | 0x5b20 | `AreaInfoMenu__10ccMenuCtrlFv` | `ccCheckMenuFaceName__Fi` | 8 |  | `InitCursol__12ccMenuWindowFi`, `ccSeOn__Fi`, `ccKanjiStrSeparate__FPci`, `MakePacket__8ccSpriteFii`, `Close__9ccMessageFv` |
| `005941e8` | 0x68 | `ccSkillCostCheck__FP6ccCharP12ccSkillPar` | `ccSkillAttributeCheck__Fi` | 1 |  | `ccGetSkillParam__Fi` |
| `005a9050` | 0x730 | `CurePoisonSPC__4ccAIFv` | `ChatCommandDeBuffPlz__4ccAIFv` | 1 |  | `CheckSysMsgID__9ccSpcCharFv`, `CheckSolution__4ccAIFiP6ccChar`, `SysMsgFromMeToMe__4ccAIFiP6ccChariiP6ccChar`, `ccSkillRecoveryCheck__Fi`, `CheckSkillList__4ccAIFi` |
| `005b0de4` | 0xe7c | `SearchHistoryMessageP__10ccAISystemFUiii` | `CalcEstimateDamage__10ccAISystemFP6ccCha` | 4 |  | `ccSkillCheckType__Fi`, `ccGetItemParam__Fi` |
| `005b23cc` | 0x94 | `ccAISysMsgCheckEntryChar__Fi` | `ccAISysMsgDeleteDelay__FUiUsUs` | 3 |  |  |
| `005b36b0` | 0x2a0 | `ChatMessageAccept__4ccAIFv` | `ChatMessageHealStart__4ccAIFv` | 1 |  | `rand`, `ChatMessageModify__4ccAIFPcPcPcPc` |
| `005b88cc` | 0x544 | `ChatMessageWalkingTalk__4ccAIFv` | `Reconnoiter__4ccAIFv` | 6 |  | `ChatMessageModify__4ccAIFPcPcPcPc`, `ccGetItemParam__Fi` |
| `005beec0` | 0xa20 | `CheckEyeLineToTarget__4ccAIFP6ccChar` | `getAttributeStr__Fii` | 4 |  | `DebuffForUnusedEnemy__4ccAIFi`, `GetFieldAttrb__9WORLD_MANFv`, `BuffForUnusedFellow__4ccAIFi`, `ccSkillCheckType__Fi`, `ccGetItemParam__Fi` |
| `005bf94c` | 0x4c4 | `getAttributeStr__Fii` | `ccPlayerStart__Fii` | 1 |  | `ccGetEnemyParam__Fi`, `ccGetBossParam__Fi`, `ccCheckTarget__FP6ccChar`, `ccGetSkillParam__Fi`, `ccSkillAttributeCheck__Fi` |
| `005c5980` | 0x80 | `checkPartyAnnihilation__Fv` | `checkPartyMenberNum__Fi` | 0 |  |  |
| `005c817c` | 0x234 | `EntrySpc__5ccSPCFi` | `DelSpc__5ccSPCFi` | 1 |  | `ccGetCharParam__Fi`, `ChangeHelmet__5ccSPCFii`, `ChangeArmor__5ccSPCFii`, `ChangeGlove__5ccSPCFii`, `ChangeBoots__5ccSPCFii` |
| `005c8468` | 0x88 | `DelSpc__5ccSPCFi` | `DelBodyCcs__5ccSPCFi` | 1 |  | `ccGetCharParam__Fi`, `ccLoadFLAddOne__FP10ccFileList`, `GetCCSAdrs__8ccStreamFPCc` |
| `005c8f34` | 0x12c | `DeleteNoPartyMember__5ccSPCFv` | `Wakeup__5ccSPCFv` | 1 |  | `ccDeleteCmnd__FP6ccChar`, `ccSleepThread__FP6ccTscb` |
| `005c9518` | 0x38 | `ccSpcWakeup__Fv` | `ccSpcConditionEffectON__Fv` | 1 |  | `Wakeup__5ccSPCFv` |
| `005ee2c0` | 0x260 | `__dt__9ROOMLIGHTFv` | `SetLight__7DUNGEONFii` | 1 |  | `__nw__FUi`, `GetChunkAdrsF__8ccStreamFPCci`, `Init__5ccEffFP10ccEffChunki`, `sceVu0CopyMatrix`, `_SetLWMatrix__7ccCoordFv` |
| `005f98ec` | 0xf64 | `Move__4BIRDFv` | `Draw__19ccBossEffLaserRain2Fv` | 10 | `town06`, `CMP_sr6met1`, `CMP_sr6bac1`, `CMP_sr6cro1` | `GetChunkAdrsF__8ccStreamFPCci`, `__nw__FUi`, `__ct__7ccCoordFv`, `Init__7ccClumpFP12ccClumpChunk`, `SetFogSw__7ccClumpFi` |
| `005faae0` | 0x2aa0 | `Draw__19ccBossEffLaserRain2Fv` | `-` | 15 | `xp_text`, `TEX_xp_tim_1`, `xp_cup`, `ANM_xp_count_a` | `__nw__FUi`, `ccBreathThread__Fi`, `GetChunkAdrsF__8ccStreamFPCci`, `SetAnm__5ccAnmFP10ccAnmChunkUi`, `CheckFade__8ccScFadeFi` |

### desktop

Text 0x1bcc0 bytes, 0x1af08 named, 0x628 unnamed (1.4%) in 4 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `0040f240` | 0x40 | `-` | `ccThDesktopDelete__FPv` | 0 | (zero words: padding) |  |
| `00414250` | 0x350 | `at__Q23std38__vector_pod<Ui,Q23std13allo` | `MainAudio_control__13Audio_controlFv` | 1 | `Memory allocation failure` | `fprintf`, `abort`, `__nw__FUi`, `__dl__FPv` |
| `00416038` | 0x228 | `StartStream__13Audio_controlFv` | `SimplePlayStream__13Audio_controlFi` | 1 |  | `ccKanjiStrSeparate__FPci`, `Breath__6ccTscbFi`, `ChangeInfo__9ccMessageFPcPcPcPcii`, `Check__9ccMessageFi`, `ccSeOn__Fi` |
| `0042ae90` | 0x70 | `PlayKigou__17NameEntry_ControlFv` | `-` | 0 | (zero words: padding) |  |

### toppage

Text 0x42c0 bytes, 0x4020 named, 0xf4 unnamed (1.4%) in 2 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `0040f240` | 0x70 | `-` | `ccThToppageDelete__FPv` | 1 |  | `ccAddFileList__FP10ccFileList` |
| `0041347c` | 0x84 | `SetPos__11ccScrollBarFi` | `-` | 0 | (zero words: padding) |  |

### demo

Text 0xd8c0 bytes, 0xd2d4 named, 0x160 unnamed (0.6%) in 3 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `0040f240` | 0x90 | `-` | `ccThDemoDelete__FPv` | 1 |  | `ccAddFileListOne__FP10ccFileList` |
| `004152d0` | 0x60 | `Delete__17ccOpening_ControlFv` | `__ct__17ccOpening_ControlFv` | 0 |  |  |
| `0041ca90` | 0x70 | `changeInputVolume__FUi` | `-` | 0 | (zero words: padding) |  |

## Quarantine (SLUS_205.64)

### main

Text 0x1764e4 bytes, 0xd0d48 named, 0xa354c unnamed (43.6%) in 39 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00100000` | 0xb8 | `-` | `_exit` | 0 |  | `_InitSys`, `FlushCache`, `main` |
| `0010d5d0` | 0x180 | `sceDmaPutEnv` | `sceDmaGetEnv` | 0 |  |  |
| `0010ef48` | 0x28 | `sceVu0RotMatrix` | `sceVu0CameraMatrix` | 0 |  |  |
| `0010f148` | 0x108 | `sceVu0LightColorMatrix` | `sceVu0DropShadowMatrix` | 1 |  | `sceVu0UnitMatrix`, `sceVu0MulMatrix` |
| `00113320` | 0xc8 | `_groupOfPicturesHeader` | `_pictureDisplayExtension` | 1 | `load_chroma_intra_quantizer_matrix == 1`, `load_chroma_non_intra_quantizer_matrix =` | `_nextBit`, `_waitIpuIdle`, `_sendIpuCommand`, `_Error` |
| `00117480` | 0x68 | `sceIpuSync` | `sceIpuInit` | 1 |  | `DIntr` |
| `00117720` | 0x20 | `sceIpuInit` | `RFU000_FullReset` | 0 | (zero words: padding) |  |
| `0011a8d8` | 0x158 | `_request_rdata` | `_search_svdata` | 1 |  | `_sceRpcFreePacket`, `sceSifSendCmd`, `DeleteSema`, `_sceRpcGetPacket`, `CreateSema` |
| `0011b098` | 0x98 | `sceSifRegisterRpc` | `sceSifRemoveRpcQueue` | 1 |  | `DIntr`, `EIntr` |
| `0011feac` | 0x6c | `SetDebugHandler` | `InitTLBFunctions` | 0 |  |  |
| `001202c4` | 0x3c | `InitTLB32MB` | `_kTLBException` | 0 | (zero words: padding) |  |
| `0012041c` | 0x124 | `_kTLBException` | `_kDebugException` | 0 |  |  |
| `0012078c` | 0x14 | `GetSystemCallTableEntry` | `_setup` | 0 |  |  |
| `001207fc` | 0x6c | `_InitSys` | `PatchIsNeeded` | 0 |  |  |
| `00120a58` | 0x38 | `Exit` | `sceSdRemoteInit` | 1 |  | `TerminateLibrary` |
| `00129108` | 0x70 | `cdvd_exit` | `_sceCd_Poff_Intr` | 1 |  | `PowerOffCB`, `DIntr`, `EIntr` |
| `00158684` | 0xbc | `Read2__6ccCdvdFUiUiPv` | `ccLoadModule__FPciPc` | 1 |  | `sceCdSearchFile`, `ccMalloc__FUi`, `sceCdRead`, `sceCdSync` |
| `0017124c` | 0x114 | `BootCheckProccess__9ccSaveSysFv` | `MainProccess__9ccSaveSysFv` | 1 |  | `CheckPort__7ccMcardFii`, `ReadSys__7ccMcardFiiPvii`, `CheckRightInfo__9ccSaveSysFP14ccSaveDataInfo` |
| `00179774` | 0x58c | `SetSoundEnv__10ccSaveDataFv` | `AddLvErosion__10ccSaveDataFif` | 0 |  |  |
| `0017a184` | 0x9cc | `ReadNewMail__10ccSaveDataFi` | `ccGetItemName__Fii` | 5 |  | `GetFrameRate__8ccSystemFv` |
| `0017bc1c` | 0x2c4 | `ccSqFade__FiUsiUc` | `ccSeOn__Fi` | 2 |  | `ccSqPlayVol__FiUs` |
| `00184210` | 0x470 | `ccSndMoviePlayer__Fii` | `ccLoad_SndModules__Fv` | 1 |  | `ccBreathThread__Fi`, `ccSndCmd__Fii` |
| `00187b08` | 0xcd8 | `ccSndGetStatus__Fv` | `SetObj__10ccEventObjFUi` | 5 |  | `__nw__FUi`, `__construct_array`, `__ct__8ccSpriteFv`, `Init__7ccKanjiFii`, `__dt__7ccKanjiFv` |
| `001889d8` | 0x7a8 | `Ctrl__12ccPartCreateFv` | `ccSetStreamDemoNote__FP9ccAnmNote` | 2 |  | `__nw__FUi`, `rand`, `MakePacket__16ccBufferSamplingFP7ccLayer`, `__ct__16ccBufferSamplingFv`, `SendPacket__8ccSpriteFv` |
| `00189290` | 0xb30 | `ccGetStreamDemoMsg__Fv` | `Func_str9001sub__FUiP15ccStrEffectCtrlP8` | 2 |  | `__dl__FPv`, `rand`, `DeleteFade__8ccScFadeFi`, `EntryFade__8ccScFadeFiiiffff`, `Breath__6ccTscbFi` |
| `0018ba70` | 0xa60 | `Func_str0001__FP6ccTscb` | `Func_str0240__FP6ccTscb` | 2 |  | `__dl__FPv`, `Breath__6ccTscbFi`, `DeleteFade__8ccScFadeFi`, `EntryFade__8ccScFadeFiiiffff`, `__dt__7ccLayerFv` |
| `00191fec` | 0x374 | `Func_str0120__FP6ccTscb` | `Func_str0610__FP6ccTscb` | 1 |  | `__dl__FPv`, `SetFrame__6ccViewFffffffff`, `Breath__6ccTscbFi`, `__dt__7ccLayerFv`, `__nw__FUi` |
| `00192a10` | 0x172c0 | `Func_str0610__FP6ccTscb` | `ccInitStreamDemoThread__Fv` | 61 | `str0710e`, `str1070e`, `str1430e`, `EFF_x001` | `__dl__FPv`, `__dt__7ccLayerFv`, `Breath__6ccTscbFi`, `EntryFade__8ccScFadeFiiiffff`, `__nw__FUi` |
| `001a9f8c` | 0xb4 | `__dt__13ccEffPart0580Fv` | `__dt__19ccObjPartCreate0580Fv` | 1 |  | `__dt__5ccObjFv`, `__dl__FPv` |
| `001aa0cc` | 0x94 | `__dt__19ccObjPartCreate0580Fv` | `__dt__13ccObjPart0580Fv` | 1 |  | `__dl__FPv` |
| `001aa360` | 0x1a0 | `__ct__16ccStreamLoadPlayFv` | `WaitEnd__16ccStreamLoadPlayFv` | 1 |  |  |
| `001aeba8` | 0x128 | `Quit__9WORLD_MANFv` | `Init__9WORLD_MANFv` | 0 |  |  |
| `001b4aa0` | 0x30 | `SetLightDirection__9WORLD_MANFP14ccDista` | `SleepDistantLight__9WORLD_MANFv` | 1 |  | `sceVu0CopyVector` |
| `001b4ae4` | 0x2c | `SleepDistantLight__9WORLD_MANFv` | `Get2DPos__9WORLD_MANFPfPf` | 0 |  |  |
| `001c5400` | 0xc0 | `Execute__7ccEventFRPsiii` | `MenuBan__7ccEventFv` | 1 |  | `ChangeInfo__9ccMessageFPcPcPcPcii` |
| `001d6818` | 0x278 | `ccSetQuaternion__FPfPff` | `ccQuaternionToMatrix__FPA4_fPf` | 3 |  | `fptodp`, `dptofp`, `sceVu0CopyVector`, `acosf`, `sceVu0OuterProduct` |
| `001d78dc` | 0x94 | `ccNewEffect__Fi` | `InitEffect__8ccEffectFii` | 1 |  | `InitEffect__9ccEffect2Fi` |
| `001e017c` | 0x14 | `Main__8ccEffectFv` | `FadeIn__8ccEffectFii` | 0 |  |  |
| `001ef2bc` | 0x86ac4 | `DEG2RAD__Fs` | `__sinit_libcc3d.cpp` | 10 |  | `__register_global_object` |

### gcmn

Text 0x1f2dc0 bytes, 0x19cb74 named, 0x510d0 unnamed (16.2%) in 124 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00301b40` | 0x40 | `-` | `ccThDfCompDelete__FPv` | 0 | (zero words: padding) |  |
| `003023e0` | 0x80 | `__dt__9EVENTAREAFv` | `__ct__11EVENTAREA01Fv` | 0 |  |  |
| `00302ad0` | 0x5e0 | `__ct__11EVENTAREA01Fv` | `__ct__11EVENTAREA02Fv` | 6 |  | `Draw__11STATICMODELFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `fieldrand__FUi` |
| `00303480` | 0xc0 | `__ct__11EVENTAREA02Fv` | `ChangeBlock__11EVENTAREA02Fi` | 1 |  | `__dl__FPv`, `__dt__5ccAnmFv`, `__dt__9EVENTAREAFv` |
| `00304400` | 0x270 | `DrawBG__11EVENTAREA02Fv` | `Draw__11EVENTAREA02Fv` | 3 |  | `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `Draw__12STATICOBJECTFv` |
| `00304c20` | 0x1d0 | `__ct__11EVENTAREA03Fv` | `Draw__11EVENTAREA03Fv` | 3 |  | `__dt__9EVENTAREAFv`, `__dl__FPv`, `SetMatrix_PosRotZYX__7ccCoordFPfPf`, `Draw__7ccClumpFv`, `LockBlt__13ccBltGrpChunkFv` |
| `00305120` | 0x160 | `Draw__11EVENTAREA03Fv` | `__ct__11EVENTAREA04Fv` | 2 |  | `__ct__9EVENTAREAFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `00305940` | 0xd0 | `__ct__11EVENTAREA04Fv` | `DrawBG__11EVENTAREA04Fv` | 1 |  | `__dt__9LENSFLAREFv`, `__dt__5CLOUDFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `00305af0` | 0x4a0 | `DrawBG__11EVENTAREA04Fv` | `__ct__11EVENTAREA05Fv` | 4 | `DMY_dummy5` | `Draw__11STATICMODELFv`, `Draw__12STATICOBJECTFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer` |
| `003065b0` | 0xd0 | `__ct__11EVENTAREA05Fv` | `DrawBG__11EVENTAREA05Fv` | 1 |  | `__dt__5CLOUDFv`, `__dt__9LENSFLAREFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `00306760` | 0x4c0 | `DrawBG__11EVENTAREA05Fv` | `__ct__11EVENTAREA07Fv` | 4 | `DMY_dummy5` | `Draw__11STATICMODELFv`, `Draw__12STATICOBJECTFv`, `SetActiveLayer__9WORLD_MANFi`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv` |
| `00306d80` | 0xd0 | `__ct__11EVENTAREA07Fv` | `ChangeBlock__11EVENTAREA07Fi` | 1 |  | `__dt__5CLOUDFv`, `__dt__9LENSFLAREFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `00307ac0` | 0x300 | `DrawBG__11EVENTAREA07Fv` | `Draw__11EVENTAREA07Fv` | 3 |  | `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `Draw__12STATICOBJECTFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer` |
| `00308540` | 0xc0 | `__ct__11EVENTAREA06Fv` | `DrawBG__11EVENTAREA06Fv` | 1 |  | `__dt__5CLOUDFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `003086e0` | 0x4b0 | `DrawBG__11EVENTAREA06Fv` | `__ct__11EVENTAREAB0Fv` | 4 |  | `Draw__11STATICMODELFv`, `Draw__12STATICOBJECTFv`, `SetActiveLayer__9WORLD_MANFi`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv` |
| `0030a1d0` | 0xe0 | `__ct__11EVENTAREAB0Fv` | `DrawBG__11EVENTAREAB0Fv` | 1 |  | `__dt__8FIREFLY2Fv`, `__dt__5ccAnmFv`, `__dt__7ccClumpFv`, `__dt__9EVENTAREAFv`, `__dl__FPv` |
| `0030a540` | 0x680 | `DrawBG__11EVENTAREAB0Fv` | `SetFloatRockParam__FP9FLOATROCK` | 4 |  | `SetActiveLayer__9WORLD_MANFi`, `Draw__11STATICMODELFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer` |
| `0030ac70` | 0x500 | `SetFloatRockParam__FP9FLOATROCK` | `__ct__11EVENTAREAB8Fv` | 0 |  | `fieldrand__FUi`, `ccRand__Fv` |
| `0030c0e0` | 0x10f0 | `__ct__11EVENTAREAB8Fv` | `SetCharInfo__FP8CHARINFO` | 9 | `MAT_se1_6clo1`, `CAM_camera2_4`, `OBJ_dummy2_4`, `CAM_camera3_3` | `_AnimateForward__5ccAnmFUi`, `Draw__7ccClumpFv`, `GetSubstAdrsF__5ccAnmFPCcb`, `SetMatrix_PosRotZYX__7ccCoordFPfPf`, `SetActiveLayer__9WORLD_MANFi` |
| `00310934` | 0x2dc | `GetBook04Item__4BOOKFP8BOOKITEMiiii` | `GetBook06Item__4BOOKFP8BOOKITEMiiii` | 1 | `%s%d%s` | `Draw__4BOOKFv`, `ccBreathThread__Fi`, `memset`, `OpenInfo__9ccMessageFPcPcPcPcii`, `Check__9ccMessageFi` |
| `00320550` | 0x130 | `ccFellowCheckNote__FP9ccAnmNote` | `Attack__8ccFellowFP6ccChari` | 1 |  | `DistanceToTarget__9ccSpcCharFP6ccChar`, `ccSkillCostCheck__FP6ccChari`, `ccSkillRangeCheck__Fif`, `ccSkillRequest__FP6ccCharP6ccChari` |
| `00320c20` | 0x1690 | `Attack__8ccFellowFP6ccChari` | `ccThFellow10__FPv` | 36 | `SPC_01`, `SPC_02`, `SPC_03`, `SPC_04` | `Breath__6ccTscbFi`, `expulsionSpc__Fi`, `ccDeleteThread__FP6ccTscb`, `__nw__FUi`, `ccStartThread__FPFPv_vii` |
| `00322360` | 0x1350 | `ccThFellow10__FPv` | `__ct__10ROOTTOWN01Fv` | 31 | `SPC_11`, `SPC_12`, `SPC_13`, `SPC_14` | `__nw__FUi`, `ccStartThread__FPFPv_vii`, `__dt__8ccFellowFv`, `__dl__FPv`, `DelSpc__5ccSPCFi` |
| `00324280` | 0xd0 | `__ct__10ROOTTOWN01Fv` | `DrawBG__10ROOTTOWN01Fv` | 1 |  | `__dt__5ccAnmFv`, `__dt__10ccTexChunkFv`, `__dt__8ROOTTOWNFv`, `__dl__FPv` |
| `003245e0` | 0x7f0 | `DrawBG__10ROOTTOWN01Fv` | `DrawMap__10ROOTTOWN01Fv` | 3 |  | `Draw__11STATICMODELFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `Draw__12STATICOBJECTFv` |
| `00325c00` | 0x5a0 | `DrawMap__10ROOTTOWN01Fv` | `__ct__10ROOTTOWN02Fv` | 1 |  | `SetActiveLayer__9WORLD_MANFi`, `_AnimateForward__5ccAnmFUi`, `Draw__5ccAnmFv`, `sceVu0CopyVector`, `SetUV__5ccAnmFiiP15ccMaterialChunki` |
| `00327100` | 0x170 | `__ct__10ROOTTOWN02Fv` | `DrawBG__10ROOTTOWN02Fv` | 1 |  | `__dt__7ccClumpFv`, `__dt__7ccLayerFv`, `__dt__9LENSFLAREFv`, `__dt__5CLOUDFv`, `__dt__5ccAnmFv` |
| `003276b0` | 0x280 | `DrawBG__10ROOTTOWN02Fv` | `DrawMap__10ROOTTOWN02Fv` | 3 |  | `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `Draw__12STATICOBJECTFv` |
| `00328cb0` | 0x3f0 | `DrawMap__10ROOTTOWN02Fv` | `__ct__7AIRSHIPFP8ccStream` | 1 | `OBJ_sr2wat00`, `DMY_sr2lig_1point` | `SetActiveLayer__9WORLD_MANFi`, `_AnimateForward__5ccAnmFUi`, `Draw__5ccAnmFv`, `SetUV__5ccAnmFiiP15ccMaterialChunki`, `Init__5CLOUDFP8ccStreami` |
| `00329250` | 0x8f0 | `__ct__7AIRSHIPFP8ccStream` | `__ct__10ROOTTOWN03Fv` | 1 |  | `sceVu0CopyVector`, `fieldrand__FUi`, `sceVu0ApplyMatrix`, `sceVu0AddVector`, `sceVu0UnitMatrix` |
| `0032a9e0` | 0x160 | `__ct__10ROOTTOWN03Fv` | `DrawBG__10ROOTTOWN03Fv` | 1 |  | `__dt__7ccClumpFv`, `__dt__5ccAnmFv`, `__dt__7ccLayerFv`, `__dl__FPv`, `__dt__10ccTexChunkFv` |
| `0032adf0` | 0x1f0 | `DrawBG__10ROOTTOWN03Fv` | `DrawMap__10ROOTTOWN03Fv` | 3 |  | `Draw__11STATICMODELFv`, `LockBlt__13ccBltGrpChunkFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer` |
| `0032c320` | 0x240 | `DrawMap__10ROOTTOWN03Fv` | `__ct__10ROOTTOWN04Fv` | 1 | `OBJ_sr3wat00` | `DrawWithOutFog__11STATICMODELFv`, `SetActiveLayer__9WORLD_MANFi`, `Draw__5ccAnmFv`, `cameraGetPos__FPfi`, `GetSubstAdrsF__5ccAnmFPCcb` |
| `0032d210` | 0x140 | `__ct__10ROOTTOWN04Fv` | `DrawBG__10ROOTTOWN04Fv` | 1 |  | `__dt__7ccClumpFv`, `__dt__7ccLayerFv`, `__dt__9LENSFLAREFv`, `__dt__5CLOUDFv`, `__dt__8ROOTTOWNFv` |
| `0032d760` | 0x280 | `DrawBG__10ROOTTOWN04Fv` | `DrawMap__10ROOTTOWN04Fv` | 3 |  | `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `UnlockBlt__13ccBltGrpChunkFv`, `MakePacketLoadData__13ccBltGrpChunkFP7ccLayer`, `Draw__12STATICOBJECTFv` |
| `0032ed40` | 0x1b0 | `DrawMap__10ROOTTOWN04Fv` | `__ct__10ROOTTOWN05Fv` | 1 | `DMY_sr4lig1point` | `SetActiveLayer__9WORLD_MANFi`, `Init__5CLOUDFP8ccStreami`, `cameraGetPos__FPfi`, `Draw__9LENSFLAREFP8ccStreamPci`, `Move__5CLOUDFv` |
| `0032f9d0` | 0x490 | `__ct__10ROOTTOWN05Fv` | `DrawMap__10ROOTTOWN05Fv` | 5 |  | `__dt__7ccClumpFv`, `__dl__FPv`, `LockBlt__13ccBltGrpChunkFv`, `Draw__11STATICMODELFv`, `UnlockBlt__13ccBltGrpChunkFv` |
| `003311f0` | 0x220 | `DrawMap__10ROOTTOWN05Fv` | `ccCheckActiveEnemy__Fv` | 1 |  | `SetActiveLayer__9WORLD_MANFi`, `fieldrand__FUi`, `Draw__5ccEffFPfUs`, `Draw__11STATICMODELFv` |
| `0033402c` | 0x54 | `deleteAllObject__11ccEntryCtrlFv` | `initObject__11ccEntryCtrlFP10ccEntryObj` | 1 |  | `deleteAllEnemy__11ccEntryCtrlFv`, `deleteAllMagicCircle__11ccEntryCtrlFv`, `deleteAllGimmick__11ccEntryCtrlFv`, `deleteAllNpc__11ccEntryCtrlFv` |
| `00334538` | 0xc8 | `deleteEnemy__11ccEntryCtrlFP10ccEntryObj` | `main__10ccEntryObjFv` | 1 |  | `routine__10ccEntryObjFv`, `deleteEnemy__11ccEntryCtrlFP10ccEntryObj` |
| `00334828` | 0xc8 | `deleteMagicCircle__11ccEntryCtrlFP10ccEn` | `deleteAllMagicCircle__11ccEntryCtrlFv` | 1 |  | `routine__10ccEntryObjFv`, `deleteMagicCircle__11ccEntryCtrlFP10ccEntryObj` |
| `00334ab8` | 0xc8 | `deleteGimmick__11ccEntryCtrlFP10ccEntryO` | `deleteAllGimmick__11ccEntryCtrlFv` | 1 |  | `routine__10ccEntryObjFv`, `deleteGimmick__11ccEntryCtrlFP10ccEntryObj` |
| `00334d48` | 0xc8 | `deleteNpc__11ccEntryCtrlFP10ccEntryObj` | `deleteAllNpc__11ccEntryCtrlFv` | 1 |  | `routine__10ccEntryObjFv`, `deleteNpc__11ccEntryCtrlFP10ccEntryObj` |
| `0034c834` | 0x43c | `actEscapeGold__8ccEnemyGFf` | `__ct__8ccEnemyHFP7ccEntry` | 4 |  | `ccSeOn3D__FiPf`, `ccRand__Fv`, `ctrl__15ccEnemyDustCtrlFP10ccEntryObji`, `ctrl__17ccEnemyWeaponCtrlFP10ccEntryObj`, `sceVu0CopyVector` |
| `0034d060` | 0x5a0 | `__ct__8ccEnemyHFP7ccEntry` | `exclEHK__8ccEnemyHFv` | 4 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `defaultThink__7ccEnemyFv`, `ccRand__Fv`, `ccSetRadDisperse__FPff` |
| `0034d6d0` | 0xf0 | `exclEHK__8ccEnemyHFv` | `__ct__8ccEnemyIFP7ccEntry` | 3 |  | `note__17ccEnemyWeaponCtrlFP10ccEntryObjP9ccAnmNo`, `__nw__FUi`, `__ct__8ccEnemyIFP7ccEntry`, `__dt__17ccEnemyWeaponCtrlFv`, `__dt__15ccEnemyDustCtrlFv` |
| `0034d990` | 0x720 | `__ct__8ccEnemyIFP7ccEntry` | `__ct__8ccEnemyKFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `ctrl__15ccEnemyDustCtrlFP10ccEntryObji`, `defaultThink__7ccEnemyFv` |
| `0034e330` | 0x840 | `__ct__8ccEnemyKFP7ccEntry` | `__ct__8ccEnemyLFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `ctrl__15ccEnemyDustCtrlFP10ccEntryObji`, `defaultThink__7ccEnemyFv` |
| `0034f340` | 0x9b0 | `__ct__8ccEnemyLFP7ccEntry` | `exclELW__8ccEnemyLFv` | 5 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccSetRadDisperse__FPff`, `defaultThink__7ccEnemyFv`, `ccRand__Fv` |
| `0034fd90` | 0x18c0 | `exclELW__8ccEnemyLFv` | `initELG__8ccEnemyLFv` | 7 |  | `DEG2RAD__Fs`, `sinf`, `ccRand__Fv`, `SetRenderState__7ccModelF20CC_RENDER_STATE_TYPEi`, `fptodp` |
| `00351950` | 0x600 | `initELG__8ccEnemyLFv` | `__ct__8ccEnemyPFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `Draw__6ccCharFv`, `SetMatrix_PosRotZYX__7ccCoordFPfPf`, `ccSetRadDisperse__FPff` |
| `00352220` | 0x710 | `__ct__8ccEnemyPFP7ccEntry` | `__ct__8ccEnemySFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `defaultThink__7ccEnemyFv`, `ccSetRadDisperse__FPff` |
| `00352bf0` | 0x830 | `__ct__8ccEnemySFP7ccEntry` | `__ct__8ccEnemyTFP7ccEntry` | 7 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `defaultThink__7ccEnemyFv`, `ccRand__Fv`, `ccSetRadDisperse__FPff` |
| `003535e0` | 0x880 | `__ct__8ccEnemyTFP7ccEntry` | `__ct__8ccEnemyUFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `defaultThink__7ccEnemyFv`, `ccSetRadDisperse__FPff`, `actEscape__7ccEnemyFv` |
| `00354390` | 0xbc0 | `__ct__8ccEnemyUFP7ccEntry` | `__ct__8ccEnemyVFP7ccEntry` | 7 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `actEscape__7ccEnemyFif`, `RAD2DEG__Ff` |
| `00355380` | 0x7e0 | `__ct__8ccEnemyVFP7ccEntry` | `__ct__8ccEnemyWFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `defaultThink__7ccEnemyFv`, `ccSetRadDisperse__FPff` |
| `00355e20` | 0x7d0 | `__ct__8ccEnemyWFP7ccEntry` | `__ct__8ccEnemyZFP7ccEntry` | 6 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ccRand__Fv`, `RAD2DEG__Ff`, `DEG2RAD__Fs` |
| `00356900` | 0x600 | `__ct__8ccEnemyZFP7ccEntry` | `__ct__9ccGimmickFv` | 5 |  | `actFollow__7ccEnemyFfffff`, `ccSetDist__FPfff`, `ctrl__15ccEnemyDustCtrlFP10ccEntryObji`, `defaultThink__7ccEnemyFv`, `ccRand__Fv` |
| `00361968` | 0x28 | `DrawAfterImages__6ccBossFv` | `SetBlendTypeAfterImage__6ccBossFUc` | 0 |  |  |
| `00364e54` | 0x64c | `__dt__9ccBossCamFv` | `ccBoss03BlurParamCB__FPfPfPUi` | 0 |  |  |
| `00365cb4` | 0x9c | `OnCinemaMode__16ccBossEffManagerFi` | `OffCinemaMode__16ccBossEffManagerFv` | 1 |  | `checkPartyAnnihilation__Fv` |
| `0036e9a0` | 0x70 | `CinemaOn__19ccBossEffCinemaFadeFi` | `CinemaOff__19ccBossEffCinemaFadeFv` | 1 |  | `SetupSkillName__19ccBossEffCinemaFadeFP17CINEMAS` |
| `0036f328` | 0x128 | `Draw__19ccBossEffCinemaFadeFv` | `__dt__18ccBossEffWaveShockFv` | 1 | `xeffect`, `ANM_ex31lhit` | `GetCCSAdrs__8ccStreamFPCc`, `sceVu0CopyVector`, `__nw__FUi`, `__ct__5ccAnmFv`, `GetChunkAdrsF__8ccStreamFPCci` |
| `003746b0` | 0x160 | `Draw__18ccBossEffLaserShotFv` | `__dt__23ccBossEffLightBallShockFv` | 1 | `xeffect`, `CMP_ex31exp1` | `GetCCSAdrs__8ccStreamFPCc`, `__nw__FUi`, `__ct__7ccCoordFv`, `GetChunkAdrsF__8ccStreamFPCci`, `Init__7ccClumpFP12ccClumpChunk` |
| `00377f58` | 0x668 | `__dt__20ccBossEffEnergyGrow2Fv` | `Draw__14ccBossEffDrainFv` | 5 | `particle` | `ccRandF__Ff`, `sceVu0CopyVector`, `__nw__FUi`, `__ct__19ccParticleGeneratorFP24ccParticleGenerat`, `startParticleGenerator__FP19ccParticleGenerator` |
| `0038c5f8` | 0x228 | `__dt__8ccBoss03Fv` | `CalcCamera__8ccBoss03Fv` | 2 |  | `GetSubstAdrsF__5ccAnmFPCcb` |
| `0038d254` | 0x18c | `SetupLaserShock__8ccBoss03Fv` | `Main__8ccBoss03Fv` | 1 |  | `sceVu0AddVector`, `sceVu0CopyVector`, `ccTransPosW2P__FPfPf`, `sinf`, `cosf` |
| `0038fb20` | 0x390 | `OnThinkDataDrainAtk__8ccBoss03Fv` | `OnThinkNeedle__8ccBoss03Fv` | 1 |  | `sceVu0RotMatrixZ`, `changeCamera__Fi`, `ccCheckTarget__FP6ccChar`, `BeginStageEffect__6ccBossFiiii`, `LockPlayer__6ccBossFi` |
| `00390e5c` | 0x254 | `OnThinkWave__8ccBoss03Fv` | `OnThinkLeafDrop__8ccBoss03Fv` | 1 |  | `EraseCmndTarget__6ccBossFv`, `sceVu0CopyVector`, `EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T`, `ccTransPosW2P__FPfPf`, `ccGetDist__FPfPf` |
| `003a4e84` | 0x21c | `OnThinkChase__8ccBoss06Fv` | `OnThinkSkill__8ccBoss06Fv` | 1 |  | `ccSeOn3D__FiPf`, `EraseCmndTarget__6ccBossFv`, `EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T`, `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector` |
| `003b5b70` | 0x3f0 | `OnThinkBodyPress__10ccBoss08_bFv` | `OnThinkMagic__10ccBoss08_bFv` | 2 |  | `LockPlayer__6ccBossFi`, `EraseCmndTarget__6ccBossFv`, `CheckMenuType__10ccMenuCtrlFv`, `UnlockPlayer__6ccBossFv`, `sceVu0CopyVector` |
| `003b6744` | 0x21c | `OnThinkSetupBarrier__10ccBoss08_bFv` | `OnThinkRandomDrive__10ccBoss08_bFv` | 1 |  | `ccSeOn3D__FiPf`, `EraseCmndTarget__6ccBossFv`, `EntryAfterImage__6ccBossFP17ENTRYAFTERIMAGE_T`, `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector` |
| `003ba208` | 0x778 | `OnThinkNeutral__10ccBoss08_cFv` | `OnThinkDmg__10ccBoss08_cFv` | 2 |  | `sceVu0CopyVector`, `sceVu0RotMatrixZ`, `ccDeleteCmnd__FP6ccChar`, `__nw__FUi`, `__ct__19ccParticleGeneratorFP24ccParticleGenerat` |
| `003bf984` | 0xdc | `Affect__17ccBoss08KillerEyeFv` | `Generate__17ccBoss08KillerEyeFv` | 1 |  | `SetMatrix_PosRotZYX__7ccCoordFPfPf`, `_AnimateForward__5ccAnmFUi`, `PreDrawAnm__6ccBossFv` |
| `003c00dc` | 0x344 | `ExecPatternIndex__17ccBoss08KillerEyeFPi` | `OnThinkGenerate__17ccBoss08KillerEyeFv` | 2 |  | `ccSetDirc__FPffi`, `checkPartyAnnihilation__Fv`, `ccCheckTarget__FP6ccChar` |
| `003c1640` | 0x120 | `OnThinkDead__17ccBoss08KillerEyeFv` | `__ct__13ccBossKyvia01Fi` | 2 | `Kyvia01` | `Breath__6ccTscbFi`, `__nw__FUi`, `__ct__13ccBossKyvia01Fi` |
| `003c1e20` | 0x6430 | `__ct__13ccBossKyvia01Fi` | `__ct__13ccBossKyvia02Fi` | 19 | `ANM_ex0batc0`, `Kyvia02` | `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector`, `ccRandF__Ff`, `SetMode__9ccBossCamFifPfP6ccChar`, `sceVu0AddVector` |
| `003c8920` | 0x8140 | `__ct__13ccBossKyvia02Fi` | `ccThKyvia03__FP6ccTscb` | 21 | `ANM_ex0batc0` | `ccSeOn3DNote__FiPfc`, `sceVu0CopyVector`, `ccSeOn__Fi`, `ccRandF__Ff`, `EntryFlash__8ccScFadeFiiffff` |
| `003d1210` | 0xbb60 | `__ct__13ccBossKyvia03Fi` | `CheckDiscMove__13ccBossKyvia04Fv` | 36 | `ANM_ex0batc0`, `Kyvia04`, `ANM_ex04nut0`, `DMY_marker01` | `ccSeOnNote__Fic`, `sceVu0CopyVector`, `ccSeOn__Fi`, `sceVu0AddVector`, `ccRandF__Ff` |
| `003dcf40` | 0xac0 | `CheckDiscMove__13ccBossKyvia04Fv` | `KyviaScene__13ccBossKyvia04Fv` | 3 |  | `OnCinemaMode__16ccBossEffManagerFi`, `ParticleKill__19ccParticleGeneratorFv`, `UnlockPlayer__6ccBossFv`, `sceVu0CopyVector`, `EntryFlash3__8ccScFadeFiiiiffff` |
| `003dde40` | 0xcd0 | `KyviaScene__13ccBossKyvia04Fv` | `KyviaRndTarget__13ccBossKyvia04Fv` | 3 |  | `ParticleKill__19ccParticleGeneratorFv`, `sceVu0CopyVector`, `__nw__FUi`, `__ct__19ccParticleGeneratorFP24ccParticleGenerat`, `startParticleGenerator__FP19ccParticleGenerator` |
| `003dec60` | 0x7160 | `KyviaRndTarget__13ccBossKyvia04Fv` | `Skill_Atk__13ccBossKyvia04Fv` | 15 | `ANM_ex0batc0` | `sceVu0CopyVector`, `SetMode__9ccBossCamFifPfP6ccChar`, `ccRandF__Ff`, `ccSeOn3DNote__FiPfc`, `sceVu0AddVector` |
| `003e62f0` | 0xe80 | `Skill_Atk__13ccBossKyvia04Fv` | `InitData__9kyviaCoreFv` | 4 |  | `ccSeOn__Fi`, `EntryFlash__8ccScFadeFiiffff`, `ccSeOnNote__Fic`, `ccBossSkillDamage__FP6ccBossPfi`, `SetEnterPos__9kyviaCoreFPf` |
| `003e7370` | 0x70 | `InitData__9kyviaCoreFv` | `ResetData__9kyviaCoreFv` | 0 |  |  |
| `003f443c` | 0x154 | `__dt__13ccMoveElementFv` | `__dt__13ccFallElementFv` | 1 |  | `memset` |
| `003f7b8c` | 0x1b4 | `Main__22ccSummonsSystemElementFv` | `__dt__25ccDarkUpheavalMngrElementFv` | 1 |  | `sceVu0CopyVector`, `ccTransPosW2P__FPfPf`, `ccTransPosP2W__FPfPf`, `ccHitCheckLM2__FPfPfUi` |
| `003fd77c` | 0x1e4 | `__ct__Q220ccDarkSummonsElement6BALL_TFv` | `__dt__Q220ccDarkSummonsElement9ELEMENT_T` | 1 |  | `memset` |
| `00409cd0` | 0x1b0 | `Draw__20ccThunderBoltElementFv` | `__dt__24ccResistantShieldElementFv` | 1 | `particle`, `ANM_x070`, `ANM_x069` | `SetAnm__5ccAnmFP8ccStreamPcUi`, `memset`, `GetCCSAdrs__8ccStreamFPCc`, `__nw__FUi`, `__ct__5ccAnmFv` |
| `0040f0b4` | 0x39c | `breederInfluence__FP6ccChar` | `ccSetRtownPC__Fii` | 1 |  | `GetChunkAdrsF__8ccStreamFPCci`, `SetAnm__5ccAnmFP10ccAnmChunkUi`, `ccSetDirc__FPffi`, `effTransfer__FP6ccChar`, `deleteCmnd__10ccEntryObjFi` |
| `0040f5c0` | 0x1b0 | `ccSetRtownPC__Fii` | `ccEntryRtownPC__FP7ccEntry` | 1 |  | `sceVu0CopyVector`, `ccEntryParamClear__FP12ccEntryParam`, `ccNaviGetLandmarkPos__FPfi`, `GetChunkAdrsF__8ccStreamFPCci`, `entryObject__11ccEntryCtrlFP12ccEntryParam` |
| `004119c4` | 0x99c | `normalMode__9ccRtownPCFv` | `eventMode__9ccRtownPCFv` | 1 |  | `GetChunkAdrsF__8ccStreamFPCci`, `SetAnm__5ccAnmFP10ccAnmChunkUi`, `HitDisable__9ccCharHitFv`, `changeRoute__9ccRtownPCFi`, `ccEntryCmnd__FP6ccChar` |
| `0041977c` | 0x3a4 | `ccSetChibiGuso__Fv` | `dogAction2__FP6ccChar` | 1 |  | `sceVu0CopyVector`, `CloseChat__9ccChatMsgFv`, `changeCamera__Fi`, `ccGetDirc0__FPfPf`, `cameraSetPos__FPfi` |
| `0041a2e0` | 0x90 | `dogActionAdult__FP6ccChar` | `burpEff__7ccPGusoFv` | 1 |  | `effect__7ccPGusoFv`, `ccSeSetParamInu__FUiP6ccChar` |
| `0041caf4` | 0xa2c | `ControlMove__11ccPuccigusoFv` | `PadLeverPower__11ccPuccigusoFf` | 5 |  | `sceVu0InnerProduct`, `sceVu0CopyVector`, `ccTransPosW2P__FPfPf`, `PadLeverPower__11ccPuccigusoFf`, `fptodp` |
| `0041e7c8` | 0x1c8 | `DrawPG__11ccPuccigusoFv` | `ccPuccigusoCheckNote__FP9ccAnmNote` | 2 |  | `strcpy`, `OpenChat__9ccChatMsgFP6ccCharPc`, `getPartyMenberChar__Fi` |
| `0041ea94` | 0x8c | `ccPuccigusoCheckNote__FP9ccAnmNote` | `ccSetNaviMap__Fv` | 1 |  | `sceVu0CopyVector` |
| `0042dcd0` | 0x2a0 | `Disp__10ccMenuCtrlFv` | `SetPanelBure__10ccMenuCtrlFis` | 0 |  |  |
| `00448e58` | 0xa58 | `PartyOutMenu__10ccMenuCtrlFv` | `ItemStatusMenu__10ccMenuCtrlFv` | 2 |  | `InitCursol__12ccMenuWindowFi`, `Close__9ccMessageFv`, `OpenInfo__9ccMessageFPcPcPcPcii`, `ccSeOn__Fi`, `Disp__10ccMenuCtrlFv` |
| `00449c44` | 0x19ec | `ItemStatusMenu__10ccMenuCtrlFv` | `GateoutMenu__10ccMenuCtrlFv` | 3 | `xcontrol`, `TEX_xcontrol`, `CONTROLLER LOAD` | `MakePacket__8ccSpriteFii`, `SetType__12ccMenuWindowFi`, `DispSquare__12ccMenuWindowFiiPc`, `Extract__7ccKanjiFPc`, `ccKanjiStrSeparate__FPci` |
| `0044b944` | 0x2dc | `GateoutMenu__10ccMenuCtrlFv` | `LogoutMenu__10ccMenuCtrlFv` | 1 |  | `DispSquare__12ccMenuWindowFiiPc`, `SetClm__7ccKanjiFiiii`, `DispSelectCursol__12ccMenuWindowFffiiii`, `MakePacket__8ccSpriteFii` |
| `0044be14` | 0x99c | `LogoutMenu__10ccMenuCtrlFv` | `DataDrainDemoMenuDisp__10ccMenuCtrlFv` | 2 |  | `MakePacket__8ccSpriteFii`, `ccSeOn__Fi`, `MakePacketStr__8ccSpriteFPci`, `MakeSignedNum__6ccFontFil`, `InitCursol__12ccMenuWindowFi` |
| `00452e70` | 0x610 | `FairyshopMenu__10ccMenuCtrlFv` | `BreederMenu__10ccMenuCtrlFv` | 2 |  | `InitCursol__12ccMenuWindowFi`, `ccSeOn__Fi`, `ccKanjiStrSeparate__FPci`, `SetItemList__10ccMenuCtrlFiiP10ccItemListi`, `SelectScr__10ccMenuCtrlFiii` |
| `0046ced0` | 0x150 | `ItemDepositMenu__10ccMenuCtrlFv` | `FountainMenuDisp3__10ccMenuCtrlFv` | 1 |  |  |
| `004722d8` | 0x418 | `BreedingMenu__10ccMenuCtrlFv` | `GtRandomMenu__10ccMenuCtrlFv` | 1 | `WORD` | `strcat`, `GetWordParamPtr__9WORLD_MANFi`, `ccKanjiStrlen__FPc`, `ccKanjiStrcat__FPcPci`, `FountainMenuDisp3__10ccMenuCtrlFv` |
| `004730b8` | 0x208 | `GtRandomMenu__10ccMenuCtrlFv` | `__ct__10ccHackMenuFP8ccStream` | 1 |  | `SetType__12ccMenuWindowFi`, `MakePacket__8ccSpriteFii` |
| `004741c0` | 0x330 | `Draw__10ccHackMenuFv` | `GtNewMenu__10ccMenuCtrlFv` | 1 |  | `ccSeOn__Fi`, `CheckOperate__7ccEventFii`, `GetEventAreaInfo__9WORLD_MANFi`, `GetItemNum__10ccSaveDataFiii` |
| `0047575c` | 0xfa4 | `PartyDisbandMenu__10ccMenuCtrlFv` | `PersonalMenuT__10ccMenuCtrlFv` | 3 |  | `ccSeOn__Fi`, `Disp__10ccMenuCtrlFv`, `ccBreathThread__Fi`, `Close__9ccMessageFv`, `Change__9ccMessageFP11ccEvMsgDataPcii` |
| `00478d80` | 0x5b20 | `AreaInfoMenu__10ccMenuCtrlFv` | `ccCheckMenuFaceName__Fi` | 8 |  | `InitCursol__12ccMenuWindowFi`, `ccSeOn__Fi`, `ccKanjiStrSeparate__FPci`, `MakePacket__8ccSpriteFii`, `Close__9ccMessageFv` |
| `00486b28` | 0x68 | `ccSkillCostCheck__FP6ccCharP12ccSkillPar` | `ccSkillAttributeCheck__Fi` | 1 |  | `ccGetSkillParam__Fi` |
| `0049b9c0` | 0x730 | `CurePoisonSPC__4ccAIFv` | `ChatCommandDeBuffPlz__4ccAIFv` | 1 |  | `CheckSysMsgID__9ccSpcCharFv`, `CheckSolution__4ccAIFiP6ccChar`, `SysMsgFromMeToMe__4ccAIFiP6ccChariiP6ccChar`, `ccSkillRecoveryCheck__Fi`, `CheckSkillList__4ccAIFi` |
| `004a3754` | 0xe7c | `SearchHistoryMessageP__10ccAISystemFUiii` | `CalcEstimateDamage__10ccAISystemFP6ccCha` | 4 |  | `ccSkillCheckType__Fi`, `ccGetItemParam__Fi` |
| `004a4d3c` | 0x94 | `ccAISysMsgCheckEntryChar__Fi` | `ccAISysMsgDeleteDelay__FUiUsUs` | 3 |  |  |
| `004a6050` | 0x2a0 | `ChatMessageAccept__4ccAIFv` | `ChatMessageHealStart__4ccAIFv` | 1 |  | `rand`, `ChatMessageModify__4ccAIFPcPcPcPc` |
| `004ab26c` | 0x544 | `ChatMessageWalkingTalk__4ccAIFv` | `Reconnoiter__4ccAIFv` | 6 |  | `ChatMessageModify__4ccAIFPcPcPcPc`, `ccGetItemParam__Fi` |
| `004b1920` | 0xa20 | `CheckEyeLineToTarget__4ccAIFP6ccChar` | `getAttributeStr__Fii` | 4 |  | `DebuffForUnusedEnemy__4ccAIFi`, `GetFieldAttrb__9WORLD_MANFv`, `BuffForUnusedFellow__4ccAIFi`, `ccSkillCheckType__Fi`, `ccGetItemParam__Fi` |
| `004b23ac` | 0x4c4 | `getAttributeStr__Fii` | `ccPlayerStart__Fii` | 1 |  | `ccGetEnemyParam__Fi`, `ccGetBossParam__Fi`, `ccCheckTarget__FP6ccChar`, `ccGetSkillParam__Fi`, `ccSkillAttributeCheck__Fi` |
| `004b83e0` | 0x80 | `checkPartyAnnihilation__Fv` | `checkPartyMenberNum__Fi` | 0 |  |  |
| `004babdc` | 0x234 | `EntrySpc__5ccSPCFi` | `DelSpc__5ccSPCFi` | 1 |  | `ccGetCharParam__Fi`, `ChangeHelmet__5ccSPCFii`, `ChangeArmor__5ccSPCFii`, `ChangeGlove__5ccSPCFii`, `ChangeBoots__5ccSPCFii` |
| `004baec8` | 0x88 | `DelSpc__5ccSPCFi` | `DelBodyCcs__5ccSPCFi` | 1 |  | `ccGetCharParam__Fi`, `ccLoadFLAddOne__FP10ccFileList`, `GetCCSAdrs__8ccStreamFPCc` |
| `004bb994` | 0x12c | `DeleteNoPartyMember__5ccSPCFv` | `Wakeup__5ccSPCFv` | 1 |  | `ccDeleteCmnd__FP6ccChar`, `ccSleepThread__FP6ccTscb` |
| `004bbf78` | 0x38 | `ccSpcWakeup__Fv` | `ccSpcConditionEffectON__Fv` | 1 |  | `Wakeup__5ccSPCFv` |
| `004e0d20` | 0x260 | `__dt__9ROOMLIGHTFv` | `SetLight__7DUNGEONFii` | 1 |  | `__nw__FUi`, `GetChunkAdrsF__8ccStreamFPCci`, `Init__5ccEffFP10ccEffChunki`, `sceVu0CopyMatrix`, `_SetLWMatrix__7ccCoordFv` |
| `004ec34c` | 0xf64 | `Move__4BIRDFv` | `Draw__19ccBossEffLaserRain2Fv` | 10 | `town06`, `CMP_sr6met1`, `CMP_sr6bac1`, `CMP_sr6cro1` | `GetChunkAdrsF__8ccStreamFPCci`, `__nw__FUi`, `__ct__7ccCoordFv`, `Init__7ccClumpFP12ccClumpChunk`, `SetFogSw__7ccClumpFi` |
| `004ed540` | 0x73c0 | `Draw__19ccBossEffLaserRain2Fv` | `-` | 57 | `xp_text`, `ending`, `TEX_xp_tim_1`, `xp_cup` | `__nw__FUi`, `__dl__FPv`, `sceVu0CopyVector`, `ccBreathThread__Fi`, `_AnimateForward__5ccAnmFUi` |

### desktop

Text 0x19d40 bytes, 0x190b4 named, 0x5b8 unnamed (1.4%) in 3 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00301b40` | 0x40 | `-` | `ccThDesktopDelete__FPv` | 0 | (zero words: padding) |  |
| `00306b50` | 0x350 | `at__Q23std38__vector_pod<Ui,Q23std13allo` | `MainAudio_control__13Audio_controlFv` | 1 | `Memory allocation failure` | `fprintf`, `abort`, `__nw__FUi`, `__dl__FPv` |
| `00308938` | 0x228 | `StartStream__13Audio_controlFv` | `SimplePlayStream__13Audio_controlFi` | 1 |  | `ccKanjiStrSeparate__FPci`, `Breath__6ccTscbFi`, `ChangeInfo__9ccMessageFPcPcPcPcii`, `Check__9ccMessageFi`, `ccSeOn__Fi` |

### toppage

Text 0x42c0 bytes, 0x4020 named, 0xf4 unnamed (1.4%) in 2 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00301b40` | 0x70 | `-` | `ccThToppageDelete__FPv` | 1 |  | `ccAddFileList__FP10ccFileList` |
| `00305d7c` | 0x84 | `SetPos__11ccScrollBarFi` | `-` | 0 | (zero words: padding) |  |

### demo

Text 0xd8c0 bytes, 0xd32c named, 0xf0 unnamed (0.4%) in 2 stretches.

| at | bytes | after | before | fns | strings | calls |
| --- | ---: | --- | --- | ---: | --- | --- |
| `00301b40` | 0x90 | `-` | `ccThDemoDelete__FPv` | 1 |  | `ccAddFileListOne__FP10ccFileList` |
| `00307c30` | 0x60 | `Delete__17ccOpening_ControlFv` | `__ct__17ccOpening_ControlFv` | 0 |  |  |

