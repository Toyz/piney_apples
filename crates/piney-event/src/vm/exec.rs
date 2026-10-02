//! `ccEvent::Execute` (`INF 0x001a8d20`): one instruction at a level.
//!
//! Level 0 does nothing. Bookkeeping runs at 1 and 2; what the player sees
//! runs only at 2 and may take several frames: the instruction keeps its
//! progress in an [`OpRun`] and returns [`Step::Yield`] at each frame
//! boundary (`ccBreathThread`), to be called again next frame.

use super::{OpRun, Step, Vm};
use crate::host::{
    Announce, CameraCommand, GimmickCommand, Host, MessageCall, MessageKind, NpcCommand, Overlay, PcCommand, Place,
    Wait,
};
use crate::ir::Op;
use crate::state::{CHARACTERS, CLOSED, DONE, ScriptSave, bit32};
use piney_data::save::by_id;
use piney_data::save::offset as off;

/// Counts `n` frames: yields until `run.n` reaches `frames`.
fn frames(run: &mut OpRun, frames: i32) -> bool {
    if run.n < frames {
        run.n += 1;
        return true;
    }
    false
}

impl Vm {
    fn place<H: Host + ?Sized>(&self, host: &mut H) -> Place {
        if self.mng.enable_phase < 4 {
            Place::Setup
        } else if matches!(host.game().status, 2 | 3) {
            Place::Desktop
        } else {
            Place::Field
        }
    }

    fn message_mode(&self, n: i32, msg: i16, parody: bool) -> Option<i32> {
        self.lib.message(n, msg as i32, parody).map(|m| m.mode)
    }

    /// Executes `op` of block `b` of event `n` at level `lv`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn op<H: Host + ?Sized>(
        &mut self,
        host: &mut H,
        n: i32,
        b: usize,
        lv: i32,
        op: Op,
        run: &mut OpRun,
        set_bit: &mut bool,
    ) -> Step {
        if lv <= 0 {
            return Step::Done;
        }
        let play = lv >= 2;
        let volume = self.volume();
        match op {
            // --- Flags ----------------------------------------------------------------------
            Op::EndEvent { grp } => {
                let g = if grp < 0 { n } else { grp as i32 };
                host.save().update_flags(g, |f| f | CLOSED | if grp == -2 { DONE } else { 0 });
            }
            Op::SetBlock { num } => {
                if num >= 30 {
                    self.fault(n, b, "set_block with a bit of 30 or more stores to address 0");
                } else if num >= 0 {
                    host.save().update_flags(n, |f| f | 1u64 << num);
                } else {
                    host.save().update_flags(n, |f| (0..b).fold(f, |f, i| f | 1u64 << (i & 63)));
                }
            }
            Op::ClearBlock { num } => host.save().update_flags(n, |f| f & !(1u64 << (num & 63))),
            Op::Repeatable {} => *set_bit = false,

            // --- Party membership and calls -------------------------------------------------
            Op::MemberAdd { pc } => member_add(host, pc),
            Op::MemberAddMsg { pc } => {
                if play && self.announce(host, run, Announce::Member { pc }) == Step::Yield {
                    return Step::Yield;
                }
                member_add(host, pc);
            }
            Op::CallOn { pc } => {
                let s = host.save();
                let call = s.member_word(off::PARTY_MEMBER_CALL) | bit32(pc as i32);
                s.set_member_word(off::PARTY_MEMBER_CALL, call);
                if call & 0x8000_0000 != 0 {
                    let st = s.member_word(off::PARTY_MEMBER_CALL_STORE);
                    s.set_member_word(off::PARTY_MEMBER_CALL_STORE, st | bit32(pc as i32));
                }
            }
            Op::CallOnLater { pc } => {
                let s = host.save();
                let call = s.member_word(off::PARTY_MEMBER_CALL);
                if call & 0x8000_0000 == 0 {
                    s.set_member_word(off::PARTY_MEMBER_CALL, call | bit32(pc as i32));
                } else {
                    let st = s.member_word(off::PARTY_MEMBER_CALL_STORE);
                    s.set_member_word(off::PARTY_MEMBER_CALL_STORE, st | bit32(pc as i32));
                }
            }
            Op::CallOff { pc } => {
                let s = host.save();
                let call = s.member_word(off::PARTY_MEMBER_CALL) & !bit32(pc as i32);
                s.set_member_word(off::PARTY_MEMBER_CALL, call);
                if call & 0x8000_0000 != 0 {
                    let st = s.member_word(off::PARTY_MEMBER_CALL_STORE);
                    s.set_member_word(off::PARTY_MEMBER_CALL_STORE, st & !bit32(pc as i32));
                }
            }
            Op::CallLock {} => {
                // Characters 0-17 on Infection; from Mutation on 0-20 (INF
                // SLUS_202.67:0x001af6d8 `slti 18`, MUT 0x001c4990 `slti 21`).
                let members = if volume >= 2 { 21 } else { 18 };
                let s = host.save();
                let call = s.member_word(off::PARTY_MEMBER_CALL);
                s.set_member_word(off::PARTY_MEMBER_CALL_STORE, call);
                s.set_member_word(off::PARTY_MEMBER_CALL, (call | 0x8000_0000) & !((1u32 << members) - 1));
            }
            Op::CallUnlock {} => {
                let s = host.save();
                let st = s.member_word(off::PARTY_MEMBER_CALL_STORE);
                s.set_member_word(off::PARTY_MEMBER_CALL, st);
                s.set_member_word(off::PARTY_MEMBER_CALL_STORE, 0);
            }
            Op::ExpOn { pc } => {
                let s = host.save();
                let v = s.member_word(off::PARTY_MEMBER_EXP) | bit32(pc as i32);
                s.set_member_word(off::PARTY_MEMBER_EXP, v);
            }
            Op::ExpOff { pc } => {
                let s = host.save();
                let v = s.member_word(off::PARTY_MEMBER_EXP) & !bit32(pc as i32);
                s.set_member_word(off::PARTY_MEMBER_EXP, v);
            }
            Op::SaveParty {} if play => {
                let party = host.party();
                let s = host.save();
                let mut v = 0u32;
                for id in party.ids {
                    if id >= 0 {
                        v |= bit32(id);
                    }
                }
                s.set_member_word(off::PARTY_MEMBER_SAVE, v);
            }

            // --- The Chaos Gate -------------------------------------------------------------
            Op::GateAdd { area } => gate_add(host, area),
            Op::GateAddMsg { area } => {
                if play && self.announce(host, run, Announce::GateAddress { area }) == Step::Yield {
                    return Step::Yield;
                }
                gate_add(host, area);
            }
            Op::GateMark { area } | Op::GateUnmark { area } => {
                let server = host.story_area(area).unwrap_or_default().server;
                let base = off::GATE_LIST_MARK + 20 * server.clamp(0, 4) as usize;
                if (0..5).contains(&server) {
                    let s = host.save();
                    if matches!(op, Op::GateMark { .. }) {
                        s.set_list_bit(base, 5, area as i32);
                    } else {
                        s.clear_list_bit(base, 5, area as i32);
                    }
                }
            }

            // --- Items, gold, skills --------------------------------------------------------
            Op::ItemAdd { pc, category, id, num } => {
                let menu = play && pc != 0 && (0..10).contains(&category) && host.add_spc_item(pc, category, id, num);
                if !menu {
                    host.save().add_item(pc, category, id, num);
                }
            }
            Op::ItemAddMenu { pc, category, id, num } => {
                if !play {
                    host.save().add_item(pc, category, id, num);
                } else {
                    // Menu 29 gives the item; wait a frame, then until it closes.
                    if run.step == 0 {
                        host.item_get_menu(pc, category, id, num);
                        run.step = 1;
                        return Step::Yield;
                    }
                    if host.field_menu() != -1 {
                        return Step::Yield;
                    }
                    host.item_get_menu_end();
                }
            }
            Op::ItemDel { pc, category, id, num } => host.save().del_item(pc, category, id, num),
            Op::GoldAdd { pc, num } => {
                let s = host.save();
                let g = s.gold(pc as i32).wrapping_add(num as i32);
                s.set_gold(pc as i32, g.clamp(0, 9_999_999));
            }
            Op::DataDrain {} => {
                let s = host.save();
                s.set_u8(off::PLCOL, 1);
                s.add_skill(0, 2);
            }
            Op::Friendship { pc, num } => host.save().add_friendship(pc as i32, num as i32, volume),
            Op::TalkNum { pc, num } => {
                let s = host.save();
                let v = if pc == 1 && s.parody_on() { 0 } else { num };
                if let Some(c) = usize::try_from(pc).ok().filter(|&c| c < CHARACTERS) {
                    s.set_u8(by_id::talk_num(c), v as u8);
                }
            }

            // --- Desktop lists --------------------------------------------------------------
            Op::WallpaperAdd { num } => host.save().set_list_bit(off::DT_WALLPAPER_LIST, 3, num as i32),
            Op::BgmAdd { num } => host.save().set_list_bit(off::DT_BGM_LIST, 3, num as i32),
            Op::DesktopItem { ty, id } => {
                // From Mutation on, a movie is not given in Parody Mode
                // (MUT SLUS_205.62:0x001c51e4).
                if volume >= 2 && ty == 2 && host.save().parody_on() {
                    return Step::Done;
                }
                if play && self.announce(host, run, Announce::DesktopItem { ty, id }) == Step::Yield {
                    return Step::Yield;
                }
                // `list + 4 * (k >> 5) |= 1 << (k & 31)` with no bound
                // (0x001b006c): an id past the list's words sets a bit in
                // what follows it (kept within the save here).
                let k = (id as i32 - 1).max(0);
                let s = host.save();
                let rest = |base: usize| (piney_data::save::SIZE - base) / 4;
                match ty {
                    0 => s.set_list_bit(off::DT_WALLPAPER_LIST, rest(off::DT_WALLPAPER_LIST), k),
                    1 => s.set_list_bit(off::DT_BGM_LIST, rest(off::DT_BGM_LIST), k),
                    2 => s.set_list_bit(off::DT_STR_LIST, rest(off::DT_STR_LIST), k),
                    _ => {}
                }
            }
            Op::BbsPost { thread, post } | Op::BbsPost7 { thread, post } => {
                let s = host.save();
                let (t, p) = (thread as i32, post as i32);
                if play {
                    if s.bbs_state(t, p) == 0 {
                        s.set_bbs_state(t, p, if matches!(op, Op::BbsPost { .. }) { 1 } else { 7 });
                    }
                } else {
                    s.set_bbs_state(t, p, 3);
                }
            }
            Op::BbsRemove { thread, post } => host.save().set_bbs_state(thread as i32, post as i32, 0),
            Op::Mail { mail } => deliver(host, play, mail),
            Op::MailVol { mail } => {
                // This event's volume index (event / 100) against the volumes cleared.
                let vol = n / 50 / 2;
                let s = host.save();
                let skip = (play && s.mail_state(mail as i32) != 0) || vol < s.clear_flag() as i32;
                if !skip {
                    deliver(host, play, mail);
                }
            }
            Op::MailMember { mail, pc } => {
                let s = host.save();
                let skip = (play && s.mail_state(mail as i32) != 0)
                    || s.member_word(off::PARTY_MEMBER_SAVE) & bit32(pc as i32) == 0;
                if !skip {
                    deliver(host, play, mail);
                }
            }
            Op::MailRemove { mail } => host.save().set_byte_at(off::MAIL_LIST, 512, mail as i32, 0),
            Op::NewsAdd { news } => {
                let s = host.save();
                if s.news_state(news as i32) == 0 {
                    s.set_byte_at(off::WEBNEWS_LIST, 128, news as i32, 1);
                }
            }
            Op::NewsRemove { news } => host.save().set_byte_at(off::WEBNEWS_LIST, 128, news as i32, 0),

            // --- World flags ----------------------------------------------------------------
            Op::Crisis { num } => host.save().set_u8(off::CRISIS, (num != 0) as u8),
            Op::TownMove { town } => {
                let s = host.save();
                let v = s.i16(off::TOWN_MOVE_FLAG) as i32 | bit32(town as i32) as i32;
                s.set_i16(off::TOWN_MOVE_FLAG, v as i16);
            }
            Op::VirusCore { area } if lv == 1 => {
                let info = host.story_area(area).unwrap_or_default();
                host.generate_area(info.words);
                let s = host.save();
                s.set_list_bit(off::PROTECT_AREA, 5, area as i32);
                for (id, count) in info.protect_items {
                    if count != 0 {
                        s.del_item(0, 15, id as i16, count as i16);
                    }
                }
            }
            Op::AreaBan { field, dungeon, floor, block } => host.save().set_area_ban([field, dungeon, floor, block]),
            Op::AreaUnban { field, dungeon, floor, block } => {
                host.save().clear_area_ban([field, dungeon, floor, block])
            }
            Op::StatusSet { index, num } => host.save().set_byte_at(off::EVENT_STATUS, 80, index as i32, num as i8),
            Op::StatusAdd { index, num } | Op::StatusSub { index, num } => {
                let s = host.save();
                let v = s.status(index as i32) as i32;
                let v = if matches!(op, Op::StatusAdd { .. }) { v + num as i32 } else { v - num as i32 };
                s.set_byte_at(off::EVENT_STATUS, 80, index as i32, v as i8);
            }
            Op::Protect { area } => host.save().set_list_bit(off::PROTECT_AREA, 5, area as i32),
            Op::LastTown { town } => host.save().set_u8(off::LAST_TOWN, town as u8),
            Op::ClearCount {} if play => {
                let s = host.save();
                if !s.parody_on() {
                    s.set_clear_flag(s.clear_flag().wrapping_add(1));
                }
            }

            _ if !play => {}

            // --- Waits and windows (level 2 from here on) -----------------------------------
            Op::Wait { count } => {
                // `for (i = 0; i <= count; i++) Breath(1)`.
                if run.n <= count as i32 {
                    run.n += 1;
                    return Step::Yield;
                }
            }
            Op::Message { msg } => return self.message(host, run, n, msg),
            Op::ClearAnswer {} => {
                self.mng.msg_num = 0;
                self.mng.msg_select = 0;
            }
            Op::Info { msg } => return self.info(host, run, n, msg, MessageKind::Info),
            Op::InfoNow { msg } => return self.info(host, run, n, msg, MessageKind::InfoNow),
            Op::TeachCamera1 {} => return self.teach(host, run, n, 1),
            Op::TeachCamera2 {} => return self.teach(host, run, n, 2),
            Op::TeachCamera3 {} => return self.teach(host, run, n, 3),
            Op::Stream { num } => {
                if run.step == 0 {
                    host.stream(num);
                    run.step = 1;
                }
                if host.busy(Wait::Stream) {
                    return Step::Yield;
                }
            }
            Op::Overlay { num } => {
                let which = match num {
                    0 => Overlay::Demo,
                    1 => Overlay::Desktop,
                    2 => Overlay::Toppage,
                    3 => Overlay::Gcmn,
                    _ => return Step::Done,
                };
                if run.step == 0 {
                    host.load_overlay(which);
                    run.step = 1;
                }
                if host.busy(Wait::Overlay) {
                    return Step::Yield;
                }
            }
            Op::NameEntry {} => {
                // Make the control; then Breath, Main, until Main says done; Breath(2).
                match run.step {
                    0 => {
                        host.name_entry_start();
                        run.step = 1;
                        return Step::Yield;
                    }
                    1 => {
                        if !host.name_entry_step() {
                            return Step::Yield;
                        }
                        run.step = 2;
                    }
                    _ => {}
                }
                if frames(run, 2) {
                    return Step::Yield;
                }
                host.name_entry_end();
            }
            Op::Menu { num } => {
                if run.step == 0 {
                    host.open_menu(num);
                    run.step = 1;
                }
                if run.step == 1 {
                    if frames(run, 2) {
                        return Step::Yield;
                    }
                    run.step = 2;
                }
                if host.field_menu() != -1 {
                    return Step::Yield;
                }
            }
            Op::ShowMap {} => {
                if run.step == 0 {
                    host.sound_effect(97);
                    run.step = 1;
                }
                if !host.show_map() {
                    return Step::Yield;
                }
            }
            Op::RemoveTrap {} => {
                // ccSeOn(228), the effect, Breath(20), EntryAffect.
                if run.step == 0 {
                    host.sound_effect(228);
                    host.remove_trap();
                    run.step = 1;
                }
                if frames(run, 20) {
                    return Step::Yield;
                }
                host.remove_trap_done();
            }
            Op::PirosColour { code } => {
                if run.step == 0 {
                    // The information line: the first line
                    // (`ccKanjiStrSeparate(text, 0)`) of this event's
                    // message numbered by the save's +0x6510, from the
                    // Parody Mode table when it is on.
                    let save = host.save();
                    let parody = save.parody_on();
                    let msg = i32::from(save.u8(0x6510) as i8);
                    let lib = self.lib.clone();
                    let line = lib
                        .message(n, msg, parody)
                        .and_then(|m| m.lines.first())
                        .map(|t| t.as_bytes().to_vec())
                        .unwrap_or_default();
                    host.piros_colour(code, &line);
                }
                return self.wait_on(host, run, Wait::PirosColour);
            }
            Op::StaffRoll {} => return self.staff_roll(host, run),
            Op::GateHackAnim {} => return self.wait_on(host, run, Wait::GateHackAnim),
            Op::PlayerSkill {} => {
                // Until the command target acts: Breath, then a skill check.
                if run.step == 0 {
                    host.begin(Wait::PlayerSkill);
                    run.step = 1;
                } else {
                    host.player_skill_step();
                }
                if host.busy(Wait::PlayerSkill) {
                    return Step::Yield;
                }
                host.end(Wait::PlayerSkill);
            }

            // --- The event manager's registers ----------------------------------------------
            Op::Entry { ty, code, marker, param } | Op::EntryMc { ty, code, marker, param } => {
                let list =
                    if matches!(op, Op::Entry { .. }) { &mut self.mng.entries } else { &mut self.mng.entries_mc };
                match list.iter_mut().find(|e| e[0] == -1) {
                    Some(e) => *e = [ty, code, marker, param],
                    None => self.fault(n, b, "entry list full: the game stores to address 0"),
                }
            }
            Op::AddOperate { num, except } => self.mng.add_operate(num as i32, except as i32, false),
            Op::DelOperate { num, except } => self.mng.del_operate(num as i32, except as i32, false),
            Op::AddOperateSf { num, except } => self.mng.add_operate(num as i32, except as i32, true),
            Op::DelOperateSf { num, except } => self.mng.del_operate(num as i32, except as i32, true),
            Op::AddTarget { ty, code } => {
                let t = &mut self.mng.targets;
                if !t.contains(&(ty, code))
                    && let Some(slot) = t.iter_mut().find(|s| s.0 == -1)
                {
                    *slot = (ty, code);
                }
            }
            Op::DelTarget { ty, code } => {
                if let Some(slot) = self.mng.targets.iter_mut().find(|s| **s == (ty, code)) {
                    *slot = (-1, -1);
                }
            }
            Op::AddAreaCode { code, except } => {
                let a = &mut self.mng.area_codes;
                if !a.contains(&(code, except))
                    && let Some(slot) = a.iter_mut().find(|s| s.0 < 0)
                {
                    *slot = (code, except);
                }
            }
            Op::DelAreaCode { code, except } => {
                for slot in self.mng.area_codes.iter_mut().filter(|s| **s == (code, except)) {
                    *slot = (-1, -1);
                }
            }
            Op::SetPoint { floor, block, evnum } => {
                if let Some(p) = self.mng.points.iter_mut().find(|p| p.num < 0) {
                    *p = super::EvPoint { floor, block, num: evnum as i32 };
                }
            }
            Op::SetPos { floor, block, posnum, dirc, x, y, z } => {
                let pos = [x as i32 as f32 * 10.0, y as i32 as f32 * 10.0, z as i32 as f32 * 10.0, 1.0];
                self.set_pos(floor, block, posnum as i32, deg2rad(dirc), pos);
                host.event_positions(&self.mng.positions);
            }
            Op::MarkerPos { marker, posnum } => {
                let g = host.game();
                match host.marker(marker) {
                    Some(m) => {
                        self.set_pos(g.floor as i16, g.block as i16, posnum as i32, m.dirc, m.pos);
                        host.event_positions(&self.mng.positions);
                    }
                    None => self.fault(n, b, "marker_pos without a marker in the loaded area"),
                }
            }
            Op::ClearOperate {} => {
                self.mng.operate_set = -1;
                self.mng.operate_target = None;
            }
            Op::ClearAreaCode {} => self.mng.area_code_set = [-1; 3],
            Op::RegistNpc16 {} => self.mng.regist_npc_num = 16,
            Op::BattleReady {} => {
                host.battle_ready();
                self.mng.add_operate(14, 0, false);
            }

            // --- Modes, overlays, sound, the screen -----------------------------------------
            Op::Mode { num } => {
                // Infection's `mode 5` also returns to the last town (INF
                // 0x001b023c); from Mutation on every mode is
                // `ChangeRequest(num, 7)` alone (MUT 0x001c552c).
                if num == 5 && volume == 1 {
                    host.change_request(5, 8);
                    let town = host.save().u8(off::LAST_TOWN) as i8 as i32;
                    host.change_area(0, town);
                } else {
                    host.change_request(num as i32, 7);
                }
                self.disable();
            }
            Op::FrameRate { rate } => host.set_frame_rate(rate),
            Op::Scene { area, town, field, dungeon, floor, block } => {
                // ChangeScene's ChangeRequest(6, 7) may put the task to
                // sleep here (Wait::ChangeRequest); the default host does not.
                if run.step == 0 {
                    host.change_scene(area, town, field, dungeon, floor, block);
                    run.step = 1;
                }
                if host.busy(Wait::ChangeRequest) {
                    return Step::Yield;
                }
            }
            Op::Sound { cmd, p0, p1, p2 } => host.sound(cmd, p0, p1, p2),
            Op::Fade { count, alpha } => host.fade(count, alpha, false, volume >= 2),
            Op::FadeMore { count, alpha } => host.fade(count, alpha, true, volume >= 2),
            Op::Noise2 {} => host.noise(2),
            Op::Noise1 {} => host.noise(1),
            Op::Noise3 {} => host.noise(3),
            Op::Noise { level } => host.noise(if (1..=3).contains(&level) { level as i32 + 2 } else { 0 }),

            // --- The field ------------------------------------------------------------------
            Op::Remove { ty, code } => host.remove(ty, code),
            Op::MenuBan {} => host.menu_ban(true),
            Op::MenuClear {} => host.menu_ban(false),
            Op::CamLookPos { x, y, z } => host.camera(CameraCommand::LookPos { x, y, z, half: false }),
            Op::CamLookPosHalf { x, y, z } => host.camera(CameraCommand::LookPos { x, y, z, half: true }),
            Op::CamLookChar { ty, code, height } => {
                host.camera(CameraCommand::LookChar { ty, code, height, half: false })
            }
            Op::CamLookCharHalf { ty, code, height } => {
                host.camera(CameraCommand::LookChar { ty, code, height, half: true })
            }
            Op::CamFollowChar { ty, code, height } => host.camera(CameraCommand::FollowChar { ty, code, height }),
            Op::CamLookMarker { marker } => host.camera(CameraCommand::LookMarker { marker, half: false }),
            Op::CamLookMarkerHalf { marker } => host.camera(CameraCommand::LookMarker { marker, half: true }),
            Op::CamPanPos { x, y, z, rate } => host.camera(CameraCommand::PanPos { x, y, z, rate }),
            Op::CamPanChar { ty, code, height, rate } => {
                host.camera(CameraCommand::PanChar { ty, code, height, rate, follow: false })
            }
            Op::CamPanFollow { ty, code, height, rate } => {
                host.camera(CameraCommand::PanChar { ty, code, height, rate, follow: true })
            }
            Op::CamPanMarker { marker, rate } => host.camera(CameraCommand::PanMarker { marker, rate }),
            Op::CamOrbit { rotx, roty, dist } => host.camera(CameraCommand::Orbit { rotx, roty, dist }),
            Op::CamOrbitMove { rotx, roty, dist, rate } => {
                host.camera(CameraCommand::OrbitMove { rotx, roty, dist, rate, degrees: false })
            }
            Op::CamOrbitTurn { rotx, roty, dist, rate } => {
                host.camera(CameraCommand::OrbitMove { rotx, roty, dist, rate, degrees: true })
            }
            Op::CamMode4 {} => host.camera(CameraCommand::Mode4),
            Op::CamzSet { vx, vy, vz, cx, cy, cz } => {
                host.camera(CameraCommand::ZSet { v: [vx, vy, vz], c: [cx, cy, cz] })
            }
            Op::CamzMove { vx, vy, vz, cx, cy, cz, vprate, cprate } => {
                host.camera(CameraCommand::ZMove { v: [vx, vy, vz], c: [cx, cy, cz], vprate, cprate })
            }
            Op::CamzSpeed { vstype, cstype } => host.camera(CameraCommand::ZSpeed { vstype, cstype }),
            Op::CamzPoint { num, vx, vy, vz, cx, cy, cz } => {
                host.camera(CameraCommand::ZPoint { num, v: [vx, vy, vz], c: [cx, cy, cz] })
            }
            Op::CamzPath { num, vprate, cprate, alpha } => {
                host.camera(CameraCommand::ZPath { num, vprate, cprate, alpha })
            }
            Op::Camera { ty, code, height, rotx, roty, dist } => {
                host.camera(CameraCommand::Char { ty, code, height, rotx, roty, dist })
            }
            Op::CameraEnd {} => host.camera(CameraCommand::End),
            Op::CameraEndReset {} => host.camera(CameraCommand::EndReset),
            Op::Radiator { rtype, ty, code, x, y, z, roty, rotz } => {
                host.gimmick(GimmickCommand::Radiator { rtype, ty, code, x, y, z, roty, rotz })
            }
            Op::BossSmoke { floor, block, x, y, z } => {
                host.gimmick(GimmickCommand::BossSmoke { floor, block, x, y, z })
            }
            Op::DeleteGimmick19 {} => host.gimmick(GimmickCommand::DeleteAll19),
            Op::OpenDoor {} => host.gimmick(GimmickCommand::OpenDoor),
            Op::CloseDoor {} => host.gimmick(GimmickCommand::CloseDoor),
            Op::NpcAct { npc, param } => host.npc(NpcCommand::Act { npc, param }),
            Op::NpcWalkPos { npc, x, y, z } => host.npc(NpcCommand::WalkPos { npc, x, y, z }),
            Op::NpcWalkDir { npc, rot, dist } => host.npc(NpcCommand::WalkDir { npc, rot, dist }),
            Op::NpcWalkMarker { npc, marker } => host.npc(NpcCommand::WalkMarker { npc, marker }),
            Op::NpcWalkChar { npc, ty, code, rot, dist } => host.npc(NpcCommand::WalkChar { npc, ty, code, rot, dist }),
            Op::NpcPutMarker { npc, marker } => host.npc(NpcCommand::PutMarker { npc, marker }),
            Op::NpcPut { npc, x, y, z } => host.npc(NpcCommand::Put { npc, x, y, z }),
            Op::NpcTurn { npc, dirc, chg } => host.npc(NpcCommand::Turn { npc, dirc, chg }),
            Op::NpcFace { npc, ty, code, chg } => host.npc(NpcCommand::Face { npc, ty, code, chg }),
            Op::PcAct { pc, act } => host.pc(PcCommand::Act { pc, act }),
            Op::PcWalkPos { pc, x, y, z } => host.pc(PcCommand::WalkPos { pc, x, y, z, run: false }),
            Op::PcRunPos { pc, x, y, z } => host.pc(PcCommand::WalkPos { pc, x, y, z, run: true }),
            Op::PcWalkDir { pc, rot, dist } => host.pc(PcCommand::WalkDir { pc, rot, dist }),
            Op::PcWalkMarker { pc, marker } => host.pc(PcCommand::WalkMarker { pc, marker }),
            Op::PcWalkChar { pc, ty, code, rot, dist } => host.pc(PcCommand::WalkChar { pc, ty, code, rot, dist }),
            Op::PcMode { pc, param } => host.pc(PcCommand::Mode { pc, param }),
            Op::PcCommand { pc, on } => host.pc(PcCommand::Command { pc, on }),
            Op::PcPutMarker { pc, marker } => host.pc(PcCommand::PutMarker { pc, marker }),
            Op::PartyPutMarker { pc, marker } => host.pc(PcCommand::PartyPutMarker { pc, marker }),
            Op::PartyPut { pc, x, y, z } => host.pc(PcCommand::PartyPut { pc, x, y, z }),
            Op::PcPut { pc, x, y, z } => host.pc(PcCommand::Put { pc, x, y, z }),
            Op::PcTurn { pc, dirc, chg } => host.pc(PcCommand::Turn { pc, dirc, chg }),
            Op::PcFace { pc, ty, code, chg } => host.pc(PcCommand::Face { pc, ty, code, chg }),
            Op::PcUseSkill { pc, skill } => host.pc(PcCommand::UseSkill { pc, skill }),
            Op::PcTint { pc, r, g, b, rate } => host.pc(PcCommand::Tint { pc, r, g, b, rate }),
            Op::PcTintOff { pc } => host.pc(PcCommand::TintOff { pc }),
            Op::EnemyPut { enemy, posnum } => {
                let pos = self.mng.position(posnum as i32).map(|p| p.pos);
                host.enemy_put(enemy, pos);
            }
            Op::AffectOn { ty, code, bit } => host.affect(ty, code, bit, true),
            Op::AffectOff { ty, code, bit } => host.affect(ty, code, bit, false),
            Op::PartyAdd { pc } => host.party_add(pc),
            Op::PartyRemove { pc } => host.party_remove(pc),
            Op::Area { area } => host.area(area),
            Op::MapOn {} => host.map_on(),
            Op::PrevRoom {} => host.prev_room(),
            Op::Room { floor, block } => host.room(floor, block),
            Op::RoomPoint { num } => {
                if let Some(p) = self.mng.point(num as i32) {
                    host.room(p.floor, p.block);
                }
            }
            Op::Hold { ty, code } => host.hold(Some((ty, code))),
            Op::HoldEnd {} => host.hold(None),
            Op::ConditionFxOn {} => host.condition_effect(true),
            Op::ConditionFxOff {} => host.condition_effect(false),
            Op::TargetForbid { num } => host.target_forbid(num != 0),
            Op::TransOff { ty, code } => host.trans(ty, code, false),
            Op::TransOn { ty, code } => host.trans(ty, code, true),
            Op::GruntyMail {} => host.grunty_mail(),
            Op::EndingKanji {} => host.ending_kanji(),

            // Bookkeeping instructions that do nothing more when playing.
            Op::SaveParty {} | Op::ClearCount {} | Op::VirusCore { .. } => {}
        }
        Step::Done
    }

    /// `teach_camera1..3` (codes 37-39, `INF 0x001ab9b4`): open the prompt,
    /// count frames of camera input (parts 1 and 2: 150 of them; part 3:
    /// one press, then 45 frames), close it.
    fn teach<H: Host + ?Sized>(&mut self, host: &mut H, run: &mut OpRun, n: i32, part: u8) -> Step {
        let k = part as usize - 1;
        let msg = if host.camera_type() != 0 { [55, 56, 57][k] } else { [7, 9, 11][k] };
        if run.step == 0 {
            let parody = host.save().parody_on();
            let lib = self.lib.clone();
            let record = lib.message(n, msg, parody);
            let place = self.place(host);
            host.message_open(&MessageCall {
                event: n as u16,
                msg: msg as i16,
                record,
                kind: MessageKind::Teach(part),
                place,
            });
            run.step = 1;
        }
        if part < 3 {
            // Each frame: count it if there was input, Breath; stop at 150.
            loop {
                if run.step == 1 {
                    if host.teach_input(part) {
                        run.n += 1;
                    }
                    run.step = 2;
                    return Step::Yield;
                }
                if run.n < 150 {
                    run.step = 1;
                    continue;
                }
                break;
            }
        } else {
            if run.step == 1 {
                if !host.teach_input(part) {
                    return Step::Yield;
                }
                run.step = 2;
                run.n = 0;
            }
            if frames(run, 45) {
                return Step::Yield;
            }
        }
        host.message_close();
        Step::Done
    }

    /// `staff_roll` (code 147, `INF 0x001b1ca4`).
    fn staff_roll<H: Host + ?Sized>(&mut self, host: &mut H, run: &mut OpRun) -> Step {
        loop {
            match run.step {
                0 => {
                    host.begin(Wait::StaffRoll);
                    run.step = 1;
                }
                1 => {
                    if host.busy(Wait::StaffRoll) {
                        return Step::Yield;
                    }
                    host.end(Wait::StaffRoll);
                    run.step = 2;
                    run.n = 0;
                }
                2 => {
                    if frames(run, 30) {
                        return Step::Yield;
                    }
                    host.set_frame_rate(2);
                    if host.save().parody_on() {
                        run.step = 5;
                    } else {
                        host.desktop_menu_open(8);
                        run.step = 3;
                        run.n = 0;
                    }
                }
                3 => {
                    if frames(run, 1) {
                        return Step::Yield;
                    }
                    run.step = 4;
                }
                4 => {
                    if host.desktop_menu().menu_type != -1 {
                        return Step::Yield;
                    }
                    run.step = 5;
                }
                5 => {
                    host.bgm_control();
                    run.step = 6;
                    run.n = 0;
                }
                _ => {
                    if frames(run, 1) {
                        return Step::Yield;
                    }
                    host.staff_roll_done();
                    return Step::Done;
                }
            }
        }
    }

    fn set_pos(&mut self, floor: i16, block: i16, num: i32, dirc: f32, pos: [f32; 4]) {
        if let Some(p) = self.mng.positions.iter_mut().find(|p| p.num < 0) {
            *p = super::EvPos { floor, block, num, dirc, pos: [pos[0], pos[1], pos[2], 1.0] };
        }
    }

    /// A blocking effect of no fixed shape: begin, poll each frame from the
    /// same frame, end.
    fn wait_on<H: Host + ?Sized>(&mut self, host: &mut H, run: &mut OpRun, w: Wait) -> Step {
        if run.step == 0 {
            host.begin(w);
            run.step = 1;
        }
        if host.busy(w) {
            return Step::Yield;
        }
        host.end(w);
        Step::Done
    }

    /// `message` (code 6): three shapes, by place.
    fn message<H: Host + ?Sized>(&mut self, host: &mut H, run: &mut OpRun, n: i32, msg: i16) -> Step {
        let parody = host.save().parody_on();
        let mode = self.message_mode(n, msg, parody).unwrap_or(0) & 0xff;
        // step 0: wait for the menus, open; 1: fixed frames; 2: poll; 3.. the tail.
        loop {
            match run.step {
                0 => {
                    let place = self.place(host);
                    run.v = place as i32;
                    let ready = match place {
                        Place::Desktop => {
                            let d = host.desktop_menu();
                            d.request < 0 && matches!(d.menu_type, -1 | 7)
                        }
                        Place::Field => matches!(host.field_menu(), -1 | 62),
                        Place::Setup => true,
                    };
                    if !ready {
                        return Step::Yield;
                    }
                    let lib = self.lib.clone();
                    let record = lib.message(n, msg as i32, parody);
                    host.message_open(&MessageCall { event: n as u16, msg, record, kind: MessageKind::Speech, place });
                    run.step = 1;
                    run.n = 0;
                }
                1 => {
                    let lead = if run.v == Place::Setup as i32 { 10 } else { 5 };
                    if frames(run, lead) {
                        return Step::Yield;
                    }
                    run.step = 2;
                }
                2 => {
                    let v = host.message_check();
                    if v == 0 {
                        return Step::Yield;
                    }
                    let place = place_of(run.v);
                    if place != Place::Desktop && mode == 3 {
                        self.mng.msg_num = msg as i32;
                        self.mng.msg_select = v;
                    }
                    if place == Place::Desktop && mode == 0 {
                        host.desktop_message_done();
                    }
                    // The tail: Desktop and Field wait 10 frames after a
                    // plain line (and Field after a question) and then 1;
                    // Setup waits 15 then 2.
                    let tail = match place {
                        Place::Desktop => {
                            if mode == 0 {
                                11
                            } else {
                                1
                            }
                        }
                        Place::Field => {
                            if mode == 0 || mode == 3 {
                                11
                            } else {
                                1
                            }
                        }
                        Place::Setup => 17,
                    };
                    run.step = 3;
                    run.n = 0;
                    run.v |= tail << 8;
                }
                _ => {
                    let tail = run.v >> 8;
                    if frames(run, tail) {
                        return Step::Yield;
                    }
                    if place_of(run.v & 0xff) == Place::Setup && self.lib.message(n, msg as i32, parody).is_some() {
                        host.message_close();
                    }
                    return Step::Done;
                }
            }
        }
    }

    /// `info` and `info_now` (codes 8, 9).
    fn info<H: Host + ?Sized>(&mut self, host: &mut H, run: &mut OpRun, n: i32, msg: i16, kind: MessageKind) -> Step {
        let parody = host.save().parody_on();
        let lib = self.lib.clone();
        let record = lib.message(n, msg as i32, parody);
        let mode = record.map_or(0, |m| m.mode) & 0xff;
        loop {
            match run.step {
                0 => {
                    let place = self.place(host);
                    run.v = place as i32;
                    let ready = match place {
                        Place::Desktop => matches!(host.desktop_menu().menu_type, -1 | 7),
                        Place::Field => host.field_menu() == -1,
                        Place::Setup => true,
                    };
                    if !ready {
                        return Step::Yield;
                    }
                    host.message_open(&MessageCall { event: n as u16, msg, record, kind, place });
                    run.step = 1;
                    run.n = 0;
                }
                1 => {
                    // After the window opens: 10 frames on the setup screen;
                    // 5 elsewhere, or 10 for info_now (its window has no fade in).
                    let lead = if run.v == Place::Setup as i32 || kind == MessageKind::InfoNow { 10 } else { 5 };
                    if frames(run, lead) {
                        return Step::Yield;
                    }
                    run.step = 2;
                }
                2 => {
                    if host.message_check() == 0 {
                        return Step::Yield;
                    }
                    let place = place_of(run.v);
                    let tail = match place {
                        Place::Desktop => {
                            if mode == 0 {
                                host.message_close();
                                host.desktop_message_done();
                            }
                            10
                        }
                        Place::Field => {
                            host.message_close();
                            10
                        }
                        Place::Setup => {
                            if record.is_some() {
                                10
                            } else {
                                1
                            }
                        }
                    };
                    run.step = 3;
                    run.n = 0;
                    run.v |= tail << 8;
                }
                _ => {
                    if frames(run, run.v >> 8) {
                        return Step::Yield;
                    }
                    if place_of(run.v & 0xff) == Place::Setup && record.is_some() {
                        host.message_close();
                    }
                    return Step::Done;
                }
            }
        }
    }

    /// `ccEvent::DispInfo` for an announcement (sound effect 74 first).
    fn announce<H: Host + ?Sized>(&mut self, host: &mut H, run: &mut OpRun, a: Announce) -> Step {
        if run.step == 0 {
            host.sound_effect(74);
            run.step = 1;
        }
        self.announce_from(host, run, 1, a)
    }

    /// `DispInfo`'s three shapes, with `run.step` counted from `base`.
    fn announce_from<H: Host + ?Sized>(&mut self, host: &mut H, run: &mut OpRun, base: u8, a: Announce) -> Step {
        loop {
            match run.step - base {
                0 => {
                    let place = self.place(host);
                    run.v = place as i32;
                    let ready = match place {
                        Place::Desktop => matches!(host.desktop_menu().menu_type, -1 | 7),
                        Place::Field => host.field_menu() == -1,
                        Place::Setup => true,
                    };
                    if !ready {
                        return Step::Yield;
                    }
                    host.announce(a);
                    run.step = base + 1;
                    run.n = 0;
                }
                1 => {
                    let lead = if run.v == Place::Setup as i32 { 10 } else { 5 };
                    if frames(run, lead) {
                        return Step::Yield;
                    }
                    run.step = base + 2;
                }
                2 => {
                    if host.message_check() == 0 {
                        return Step::Yield;
                    }
                    let place = place_of(run.v);
                    let tail = match place {
                        Place::Desktop => {
                            host.message_close();
                            host.desktop_message_done();
                            10
                        }
                        Place::Field => {
                            host.message_close();
                            10
                        }
                        Place::Setup => 10,
                    };
                    run.step = base + 3;
                    run.n = 0;
                    run.v |= tail << 8;
                }
                _ => {
                    if frames(run, run.v >> 8) {
                        return Step::Yield;
                    }
                    if place_of(run.v & 0xff) == Place::Setup {
                        host.message_close();
                    }
                    return Step::Done;
                }
            }
        }
    }
}

fn place_of(v: i32) -> Place {
    match v & 0xff {
        0 => Place::Setup,
        1 => Place::Desktop,
        _ => Place::Field,
    }
}

/// `DEG2RAD(short)`: `3.1415927 * deg / 32768.0`, the game's single-precision
/// arithmetic (`INF 0x001dabb0`): the argument is a 16-bit angle, not degrees.
///
/// In the EE's arithmetic (`piney_data::field::ee`): the host's IEEE
/// multiply and divide round the last bit differently for half the inputs.
/// This form equals the game's `DEG2RAD` for all 65536 of them (run in
/// `tools/eemu.py`).
pub fn deg2rad(v: i16) -> f32 {
    use piney_data::field::ee::{div, from_int, mul};
    const PI: u32 = 0x4049_0fdb;
    const HALF_TURN: u32 = 0x4700_0000;
    f32::from_bits(div(mul(PI, from_int(i32::from(v))), HALF_TURN))
}

fn member_add<H: Host + ?Sized>(host: &mut H, pc: i16) {
    let s = host.save();
    let f = s.member_word(off::PARTY_MEMBER_FLAG) | bit32(pc as i32);
    s.set_member_word(off::PARTY_MEMBER_FLAG, f);
    let e = s.member_word(off::PARTY_MEMBER_EXP) | bit32(pc as i32);
    s.set_member_word(off::PARTY_MEMBER_EXP, e);
}

/// `mail` when playing: `NewMail` if the mail never came; when replaying, `ReadNewMail`.
fn deliver<H: Host + ?Sized>(host: &mut H, play: bool, mail: i16) {
    let s = host.save();
    let Ok(m) = usize::try_from(mail) else { return };
    if play {
        if s.mail_state(mail as i32) == 0 {
            s.new_mail(m);
        }
    } else {
        s.read_new_mail(m);
    }
}

/// `gate_add`: the story area into its server's gate list and order, its
/// three words into the word list.
fn gate_add<H: Host + ?Sized>(host: &mut H, area: i16) {
    let info = host.story_area(area).unwrap_or_default();
    let s = host.save();
    s.set_gate_list(info.server, area as i32);
    for w in info.words {
        s.set_list_bit(off::WORD_LIST, 15, w.unwrap_or(0));
    }
}

#[cfg(test)]
mod tests {
    use super::deg2rad;

    /// Values of the game's `DEG2RAD` (run in `tools/eemu.py`) where the
    /// host's IEEE arithmetic comes out one bit higher.
    #[test]
    fn deg2rad_is_the_games() {
        assert_eq!(deg2rad(-32767).to_bits(), 0xc049_0e48);
        assert_eq!(deg2rad(-32766).to_bits(), 0xc049_0cb6);
        assert_eq!(deg2rad(-32765).to_bits(), 0xc049_0b24);
        assert_eq!(deg2rad(32767).to_bits(), 0x4049_0e48);
        assert_eq!(deg2rad(0).to_bits(), 0);
    }
}
