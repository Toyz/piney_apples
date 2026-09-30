//! Items (`useitem.cpp`, gcmn 0x0057a6d0-0x0057c5e4): what using an item does
//! ([`use_item_request`], `ccUseItemRequest`), which items the menu offers
//! ([`item_useful`], [`skill_useful`]), the skill an item casts
//! ([`item_skill_request`]) and the save's item lists. `ccUseItemRequest` is
//! one blocking call in the game; the port applies its rules to the [`Scene`]
//! and the [`ItemEnv`] and returns the rest as a script of [`Step`]s in the
//! game's order (waits included), which a runtime plays frame by frame. What
//! each category does is in docs/engine/battle.md ("Items").

use piney_data::field::ee;
use piney_data::save::SaveData;

use crate::chara::{Body, Char};
use crate::event::{Event, Who};
use crate::exp::Party;
use crate::param::{cond, elm, ty};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::skill::{self, Request};
use crate::tables::Tables;

/// The item categories, the code's high half.
pub mod category {
    /// Recovery: potions, cures, the bloods (buffs), the souls (SP), wine.
    pub const RECOVERY: i32 = 10;
    /// Spell scrolls: attack, condition and summon spells.
    pub const SPELL: i32 = 11;
    /// Books that raise a stat for good.
    pub const BOOK: i32 = 12;
    /// Tools: the trap wire, the gate-out ocarina, the map orb.
    pub const TOOL: i32 = 13;
    /// Trade items.
    pub const TRADE: i32 = 14;
    /// Important items, the player's alone, counted in `impItemList`.
    pub const IMPORTANT: i32 = 15;
}

/// The runtime's globals the item code reads text from (main): each is a
/// `char *` to a `ccKanjiStrSeparate` list. `statusUpStr` holds "up",
/// "down" and the stat names a book's message is made of (see
/// [`Info::StatusUp`]); the others are one message each.
pub const STATUS_UP_STR: u32 = 0x0037_7e74;
pub const TRAP_DISCHARGE_STR: u32 = 0x0037_7e78;
pub const SHOW_MAP_INFO: u32 = 0x0037_7e70;
pub const INSTALL_WARN_STR: u32 = 0x0037_7e80;
pub const EPITAPH_STR_0X: u32 = 0x0037_7e94;
/// The full stop a book's message ends with (gcmn 0x006e1fb8).
pub const STATUS_UP_STOP: u32 = 0x006e_1fb8;

/// `ccCheckItemUseful(category, id)` (gcmn 0x0057a6d0): how the item menu
/// uses an item: 0 never, 1 on a target chosen next (`TargetMenu`), 2 at
/// once, on the player. Recovery items and spells take a target; books are
/// used at once; of the tools the trap wire (row 0) takes a target and the
/// others are used at once; trade items never; of the important items the
/// ones [`use_item_request`] acts on (42-49, 60, 61, 68, 69, 273-280,
/// 287-290) are used at once and the rest never.
pub fn item_useful(category: i32, id: i32) -> i32 {
    match category {
        category::RECOVERY | category::SPELL => 1,
        category::BOOK => 2,
        category::TOOL => {
            if id == 0 {
                1
            } else {
                2
            }
        }
        category::IMPORTANT => match id {
            42..=49 | 60 | 61 | 68 | 69 | 273..=280 | 287..=290 => 2,
            _ => 0,
        },
        _ => 0,
    }
}

/// A character on the targeting list `ccCheckSkillUseful` walks
/// (`cmndSortRoot`, linked through `ccChar.cmndSort`, nearest first), with
/// what the runtime keeps for it: `cmndDist` (float bits, its distance
/// from the player) and whether it is in the camera's view
/// (`ccCheckCameraDeg(pos, 5120)`, main 0x001da710).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortEntry {
    pub ch: usize,
    pub dist: u32,
    pub in_view: bool,
}

/// `ccCheckSkillUseful(sid)` (gcmn 0x0057a890): whether the skill (or an
/// item's skill) has anyone to use it on, which the skill and item menus show:
/// a party member for a party skill (a fallen one for Resurrect, 180), else
/// someone of its target type alive, in view and in range; the drain skills
/// (2-5) need a broken protect gauge, and from Mutation on the bracelet on
/// (`drain_off`). An id below 1 or past the skill table has no use.
pub fn skill_useful(t: &Tables, scene: &Scene, party: &Party, sorted: &[SortEntry], sid: i32, drain_off: bool) -> bool {
    if sid <= 0 {
        return false;
    }
    let Some(sk) = t.skill(sid) else { return false };
    if sk.target_type & 3 != 0 {
        return party.members.iter().flatten().any(|&m| {
            let d = scene.chars[m].cond[cond::DEAD];
            sid != skill::RESURRECT || (d != 0 && d != 5)
        });
    }
    for e in sorted {
        let ch = &scene.chars[e.ch];
        let base = ch.base();
        if sk.target_type & base.ty == 0
            || !ee::le(e.dist, ee::add(sk.trigger_range, base.width))
            || ch.cond[cond::DEAD] != 0
            || !e.in_view
        {
            continue;
        }
        if !(2..=5).contains(&sid) {
            return true;
        }
        if drain_off && t.volume != piney_data::volume::Volume::Inf {
            continue;
        }
        if base.ty & ty::FOE == 0 {
            return true;
        }
        if ch.foe_state().map_or(0, |f| f.pp_count) != 0 {
            return true;
        }
    }
    false
}

/// A skill an item starts: `_ccSkillRequest(cp, tp, sid, stype)` through
/// `ccItemSkillRequest` (gcmn 0x00572790), `ccItemSkillRequestParam`
/// (0x005727d0) or `ccItemSkillCompel` (0x00572730).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemSkill {
    pub sid: i32,
    /// 1: the item's effect on the target alone; 2: cast through the user.
    pub stype: i32,
    /// `ccSkill.param` (+0x2c), the heal of skill 295, when the request
    /// set it.
    pub param: Option<i32>,
    /// `ccSkill`'s compel bit (byte 13 bit 1), which only
    /// `ccItemSkillCompel` sets.
    pub compel: bool,
    /// What `_ccSkillRequest` did; None when it made no skill (sid 0).
    pub request: Option<Request>,
}

#[allow(clippy::too_many_arguments)]
fn item_skill(
    t: &Tables,
    scene: &mut Scene,
    cp: usize,
    tp: usize,
    sid: i32,
    flag: i32,
    running_attack: bool,
    rng: &mut dyn Rng,
) -> Option<ItemSkill> {
    if sid == -1 {
        return None;
    }
    let stype = if flag == 0 { 1 } else { 2 };
    let request = skill::request(t, scene, cp, tp, sid, stype, running_attack, rng);
    // Cast through the user, `_ccSkillRequest` also sets its `targetChar`
    // (+0x78) to `tp` (gcmn 0x00572b80): the act that casts (Kite's
    // AnimCtrl, 0x005995c0) waits on it, and the skill on the act's end.
    if stype == 2 && request.is_some() {
        scene.chars[cp].target_char = Some(tp);
    }
    Some(ItemSkill { sid, stype, param: None, compel: false, request })
}

/// `ccItemSkillRequest(cp, tp, sid, flag)` (gcmn 0x00572790): an item's
/// skill, on the target alone (`flag` 0, stype 1) or cast through `cp`
/// (stype 2). None for sid -1 (an item with no skill). `running_attack`
/// is whether `cp` has a normal attack running (see [`skill::request`]).
#[allow(clippy::too_many_arguments)]
pub fn item_skill_request(
    t: &Tables,
    scene: &mut Scene,
    cp: usize,
    tp: usize,
    sid: i32,
    flag: i32,
    running_attack: bool,
    rng: &mut dyn Rng,
) -> Option<ItemSkill> {
    item_skill(t, scene, cp, tp, sid, flag, running_attack, rng)
}

/// `ccItemSkillRequestParam(cp, tp, sid, param, flag)` (gcmn
/// 0x005727d0): the same, the new skill's `param` set (the Recovery
/// Drink's heal).
#[allow(clippy::too_many_arguments)]
pub fn item_skill_request_param(
    t: &Tables,
    scene: &mut Scene,
    cp: usize,
    tp: usize,
    sid: i32,
    param: i32,
    flag: i32,
    running_attack: bool,
    rng: &mut dyn Rng,
) -> Option<ItemSkill> {
    let mut s = item_skill(t, scene, cp, tp, sid, flag, running_attack, rng)?;
    if s.request.is_some() {
        s.param = Some(param);
    }
    Some(s)
}

/// `ccItemSkillCompel(cp, tp, sid, flag)` (gcmn 0x00572730): the same, the
/// new skill marked compelled (bosses and events use it).
#[allow(clippy::too_many_arguments)]
pub fn item_skill_compel(
    t: &Tables,
    scene: &mut Scene,
    cp: usize,
    tp: usize,
    sid: i32,
    flag: i32,
    running_attack: bool,
    rng: &mut dyn Rng,
) -> Option<ItemSkill> {
    let mut s = item_skill(t, scene, cp, tp, sid, flag, running_attack, rng)?;
    s.compel = s.request.is_some();
    Some(s)
}

/// The globals and runtime state `ccUseItemRequest` reads and writes
/// besides the characters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemEnv {
    /// The player (`plw.pw`, gcmn 0x00730300), by scene index.
    pub player: Option<usize>,
    /// `ccPartyManager` (gcmn 0x00730310).
    pub party: Party,
    /// The player's `ccSpcChar.pauseSW`: set while the player uses an item.
    pub player_pause: bool,
    /// `plw.pauseSW` (gcmn 0x007302e0 bit 0).
    pub plw_pause: bool,
    /// `dneFlag` (main 0x00378cd8): set by the gate-out ocarina.
    pub dne_flag: i32,
    /// `saveData.parodyFlag` (+0x842b): the epitaphs' other texts.
    pub parody: bool,
    /// The player's `ccSpcChar::CheckControlMode()`.
    pub control_mode: i32,
    /// The player's `ccSpcChar::CheckSysMsgID()`.
    pub sys_msg_id: u16,
    /// Whether the user has a normal attack running (`_ccSkillRequest`).
    pub running_attack: bool,
}

/// A message `ccMessage::OpenInfo` shows, by where its text comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Info {
    /// A book's: `statusUpStr` piece `stat` (the stat's name), piece 0
    /// ("up") or, for a negative amount, 1 ("down"), the amount's digits
    /// (`dec2sjis(|amount|, buf, 16, 0)`) and [`STATUS_UP_STOP`], in one
    /// line. `stat` is the `elm` field + 2 (2-15), 16 maxHP, 17 maxSP; a
    /// row past the books says stat 0, amount 0.
    StatusUp { stat: i32, amount: i32 },
    /// `trapDischargeStr`.
    TrapDischarge,
    /// `showMapInfo`.
    ShowMap,
    /// `epitaphStr0X` piece 0.
    EpitaphUnknown,
    /// `installWarnStr` pieces 0, 1 and 2, as three lines.
    InstallWarn,
}

/// One step of an item's use, in the game's order: an effect on the rules
/// (applied already by [`use_item_request`]; the runtime turns affects and
/// the skill into effects) or a call the runtime makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// `ccChar::EntryAffect`: [`Event::Affect`].
    Affect(Event),
    /// A skill request, with its `effSkillStart` and `ccWordsPlay` calls.
    Skill(ItemSkill),
    /// `ccSeOn(n)`, `ccSeOnNote(n, note)`.
    Se(i32),
    SeNote(i32, i32),
    /// `effHeal(ch, 1)` (main 0x001ccd20).
    EffHeal(Who),
    /// `ccMenuCtrl::CloseMenuDisp()`.
    CloseMenuDisp,
    /// From Mutation on, `ccSnd` +0x139 set: the skill words and voices
    /// stay silent (`ccWordsPlay`, `ccVoiceRequest`) until the Grunty is
    /// left (`ccThPucciguso`) or the next file list loads.
    VoicesOff,
    /// From Mutation on, `worldman->warpFlag = 0` as the gate-out ocarina
    /// opens its menu.
    WarpFlagOff,
    /// From Mutation on, after `ccAI::UseItem`'s use: the member's remark
    /// on item `code` ([`crate::party_chat::Remark::UseItem`], or
    /// `UseLastItem` when it was the last).
    UseItemRemark {
        last: bool,
        code: i32,
    },
    /// `n` frames with the menu drawn: `ccMenuCtrl::Disp()`,
    /// `ccBreathThread(1)`.
    Frames(u32),
    /// `ccMessage::OpenInfo(text, ..., 0, -1, -1)`.
    OpenInfo(Info),
    /// `ccMenu->panelStatus`, `ccMenu->bgStatus` set.
    PanelStatus(i16),
    BgStatus(i16),
    /// `while (!ccMsg->Check(0))` a menu frame: until the message is
    /// dismissed.
    WaitMessage,
    /// `ccMessage::Close()`, `ccMessage::CloseInstant()`.
    CloseMessage,
    CloseMessageInstant,
    /// `ccChangeCmndTarget(NULL)`: no command target.
    ClearCmndTarget,
    /// `effRemoveTrap(pos, -1, -1)` (main 0x001d0980) at the target's
    /// `pos`.
    RemoveTrap([u32; 4]),
    /// The target's word at +0x140 (a trap box's entry, `boxent`) set: -1
    /// while the trap's message shows, then put back.
    BoxEnt {
        on: Who,
        value: u32,
    },
    /// `ccSleepAllThread()`, `ccWakeAllThread()`, `ccMenuCtrl::StillOn()`,
    /// `StillOff()`, `ccDisableThEvent()` (main 0x001b5360).
    SleepAllThread,
    WakeAllThread,
    StillOn,
    StillOff,
    DisableThEvent,
    /// `plw.pw->ManualModeAI(ev)`.
    ManualModeAI(i32),
    /// `ccAISysMsgSend(kind, -1, id, 0xffff, 0, param)`.
    SysMsg {
        kind: i32,
        id: u16,
        param: i32,
    },
    /// `ccMenuCtrl::ChangeMenu(n)`.
    ChangeMenu(i32),
    /// Menu frames until `worldman->ShowMap()` is done, at least 20 in all.
    WaitMap,
    /// A character's `noDeathFlag` set (the player's, while the Grunty
    /// comes; put back after).
    NoDeath {
        on: Who,
        value: bool,
    },
    /// While `checkPartyAnnihilation() || compulsionGameOver`, a field
    /// frame (`ccDamUprStr::CtrlAll()`, `ccChat->Disp(0)`, the menu's and
    /// `menuWin`'s `ccSprite::Trans()`, `ccBreathThread(1)`).
    WaitParty,
    /// `ccClearConditionAllEnemy()` (gcmn 0x0042e4f0).
    ClearConditionAllEnemy,
    /// `ccPuccigusoStart(n)` (gcmn 0x005109c0): a Grunty of kind `n`.
    Pucciguso(i32),
    /// `plw.pauseSW` set.
    PlwPause(bool),
    /// While `pgRideFlag`, a field frame: the Grunty ride.
    WaitRide,
    /// `ccEpitaphMsg(strs, pages)` (gcmn 0x0057c3e0): the pages of an
    /// important item's epitaph or note (the build's `fieldui`
    /// `epitaph_*`), the parody mode's when `parody`, each until dismissed.
    Epitaph {
        item: i32,
        parody: bool,
    },
    /// `ccStartThread(ccThBook, 35, 0x1000)` with the book's page (tcb
    /// +0x14) and its state 1 (+0x18).
    BookStart(i32),
    /// `ccChat->CloseChat()`.
    CloseChat,
    /// Frames (the menu drawn while the book's state is 1) until the book
    /// thread's state is 0, then `ccDeleteThread`.
    WaitBook,
    /// The user's `pauseSW` (when the user is the player): set for the
    /// whole use.
    Pause {
        on: Who,
        value: bool,
    },
}

/// A book's effect: the `elm` field raised (or lowered), or maxHP/maxSP.
#[derive(Clone, Copy)]
enum Up {
    Stat(usize, i16),
    MaxHp(i16),
    MaxSp(i16),
}

/// The books (category 12, rows 0-33): the effect, the stat the message
/// names and the amount it says.
const BOOKS: [(Up, i32, i32); 34] = {
    use Up::*;
    use elm::*;
    [
        (Stat(P_ATK, 10), 2, 1),
        (Stat(P_DEF, 10), 3, 1),
        (Stat(M_ATK, 10), 6, 1),
        (Stat(M_DEF, 10), 7, 1),
        (Stat(P_HIT, 10), 4, 1),
        (Stat(P_EVA, 10), 5, 1),
        (Stat(SOIL, 10), 10, 1),
        (Stat(WATER, 10), 11, 1),
        (Stat(FIRE, 10), 12, 1),
        (Stat(WIND, 10), 13, 1),
        (Stat(THUNDER, 10), 14, 1),
        (Stat(DARK, 10), 15, 1),
        (Stat(P_ATK, 20), 2, 2),
        (Stat(P_DEF, 20), 3, 2),
        (Stat(M_ATK, 20), 6, 2),
        (Stat(M_DEF, 20), 7, 2),
        (Stat(P_HIT, 20), 4, 2),
        (Stat(P_EVA, 20), 5, 2),
        (Stat(SOIL, 20), 10, 2),
        (Stat(WATER, 20), 11, 2),
        (Stat(FIRE, 20), 12, 2),
        (Stat(WIND, 20), 13, 2),
        (Stat(THUNDER, 20), 14, 2),
        (Stat(DARK, 20), 15, 2),
        (Stat(M_ATK, -10), 6, -1),
        (Stat(WATER, 30), 11, 3),
        (Stat(M_HIT, 10), 8, 1),
        (Stat(M_EVA, 10), 9, 1),
        (Stat(M_HIT, 20), 8, 2),
        (Stat(M_EVA, 20), 9, 2),
        (MaxHp(30), 16, 30),
        (MaxSp(15), 17, 15),
        (MaxHp(10), 16, 10),
        (MaxSp(5), 17, 5),
    ]
};

/// The important items with an epitaph or note to read (`ccEpitaphMsg`).
fn epitaph(id: i32) -> bool {
    matches!(id, 42..=46 | 48 | 68 | 287..=290)
}

/// The short at `off` in what `ch.personality` points at: a party
/// member's `ccSpcParam` (maxHP +0x24, maxSP +0x26, `elm` +0x28), a foe's
/// `ccEnemyParam`/`ccBossParam` (`real` +0x00, `temp` +0x20, `time`
/// +0x40); None past those, or for an object.
fn personality_short(ch: &mut Char, off: usize) -> Option<&mut i16> {
    let i = off / 2;
    match &mut ch.body {
        Body::Spc(p) => match i {
            0x12 => Some(&mut p.max_hp),
            0x13 => Some(&mut p.max_sp),
            0x14..=0x23 => Some(&mut p.elm[i - 0x14]),
            _ => None,
        },
        Body::Foe(f) => match i {
            0..=15 => Some(&mut f.real[i]),
            16..=31 => Some(&mut f.temp[i - 16]),
            32..=47 => Some(&mut f.time[i - 32]),
            _ => None,
        },
        Body::Other { .. } => None,
    }
}

/// A book's effect on `tp`. The game writes through `tp->personality` as
/// a `ccSpcParam`: a party member's own stats, maxHP or maxSP; on a foe
/// (the AI can use any item it carries on any target) the same offsets
/// land in its `ccEnemyParam` (`temp` and `time`). An object's is not
/// modelled and is left alone. HP and SP rise on the `ccChar` whatever
/// the target. From Mutation on (`later`) maxHP stops at 9999 and maxSP
/// at 999.
fn book(scene: &mut Scene, tp: usize, id: i32, later: bool) -> (i32, i32) {
    let Some(&(up, stat, amount)) = usize::try_from(id).ok().and_then(|i| BOOKS.get(i)) else { return (0, 0) };
    let ch = &mut scene.chars[tp];
    match up {
        Up::Stat(f, d) => {
            if let Some(x) = personality_short(ch, 0x28 + 2 * f) {
                let v = x.wrapping_add(d);
                *x = if d < 0 {
                    v.max(0)
                } else if v >= 1000 {
                    999
                } else {
                    v
                };
            }
        }
        Up::MaxHp(d) => {
            if let Some(x) = personality_short(ch, 0x24) {
                *x = x.wrapping_add(d);
                if later && *x >= 10000 {
                    *x = 9999;
                }
            }
            let v = i32::from(ch.hp) + i32::from(d);
            ch.hp = if v < 10000 { v as i16 } else { 9999 };
        }
        Up::MaxSp(d) => {
            if let Some(x) = personality_short(ch, 0x26) {
                *x = x.wrapping_add(d);
                if later && *x >= 1000 {
                    *x = 999;
                }
            }
            let v = i32::from(ch.sp) + i32::from(d);
            ch.sp = if v < 1000 { v as i16 } else { 999 };
        }
    }
    (stat, amount)
}

/// A book read by a party member outside a fight: `AddSpcItem`'s
/// `ccUseItemRequest(spc, spc, code, 0)` for a book given in a town, where
/// what lasts is the member's record (`ccChar` +4 is the save's
/// `spcParam[id]`). [`book`] on the record; (stat, amount) for its
/// message.
pub fn book_on_record(p: &mut crate::param::SpcParam, id: i32, volume: piney_data::volume::Volume) -> (i32, i32) {
    let mut scene = Scene { chars: vec![Char::pc(*p)], ..Scene::default() };
    let r = book(&mut scene, 0, id, volume != piney_data::volume::Volume::Inf);
    if let Some(q) = scene.chars[0].spc() {
        *p = *q;
    }
    r
}

/// `ccUseItemRequest(cp, tp, code, pn)` (gcmn 0x0057aa80): `cp` uses item
/// `code` on `tp`; `pn` is the kind of Grunty the flute calls (0-8, else
/// 0). Applies the item's effects (see the module's overview) and returns
/// the whole use as [`Step`]s. A negative code does nothing. A recovery
/// item needs `tp` on a command list (`ccCheckTarget`). An item code
/// outside categories 10-15, or a recovery or spell row past its table,
/// casts no skill here (the game reads a skill id from unrelated memory).
#[allow(clippy::too_many_arguments)]
pub fn use_item_request(
    t: &Tables,
    scene: &mut Scene,
    env: &mut ItemEnv,
    cp: usize,
    tp: usize,
    code: i32,
    pn: i32,
    rng: &mut dyn Rng,
) -> Vec<Step> {
    let mut out = Vec::new();
    if code < 0 {
        return out;
    }
    let by_player = env.player == Some(cp);
    if by_player {
        env.player_pause = true;
        out.push(Step::Pause { on: Who::Char(cp), value: true });
    }
    let cat = code >> 16;
    let id = code & 0xffff;
    let pos = scene.chars[tp].pos;
    let (ctp, ccp) = (Who::Char(tp), Who::Char(cp));
    let item_sid = |t: &Tables| t.item(code).map(|p| p.skill_id);
    let later = t.volume != piney_data::volume::Volume::Inf;
    let running = env.running_attack;
    match cat {
        category::RECOVERY => {
            if scene.listed(tp) {
                let (max_hp, max_sp) = (scene.chars[tp].max_hp, scene.chars[tp].max_sp);
                match id {
                    18..=20 => {
                        let amount = [100, 250, max_sp][(id - 18) as usize];
                        out.push(Step::Affect(Event::affect(ctp, ccp, 8, amount, 0, 0)));
                        out.push(Step::EffHeal(ctp));
                    }
                    21 | 22 => {
                        out.push(Step::EffHeal(ctp));
                        out.push(Step::Affect(Event::affect(ctp, ccp, 7, max_hp, 0, 0)));
                        // From Mutation on the SP follows at once.
                        if !later {
                            out.push(Step::CloseMenuDisp);
                            out.push(Step::Frames(10));
                        }
                        out.push(Step::Affect(Event::affect(ctp, ccp, 8, max_sp, 0, 0)));
                    }
                    23 => {
                        if let Some(sid) = item_sid(t)
                            && let Some(s) = item_skill_request_param(t, scene, cp, tp, sid, 800, 0, running, rng)
                        {
                            out.push(Step::Skill(s));
                        }
                    }
                    _ => {
                        if let Some(sid) = item_sid(t)
                            && let Some(s) = item_skill_request(t, scene, cp, tp, sid, 0, running, rng)
                        {
                            out.push(Step::Skill(s));
                        }
                    }
                }
            }
        }
        category::SPELL => {
            if let Some(sid) = item_sid(t)
                && let Some(s) = item_skill_request(t, scene, cp, tp, sid, 1, running, rng)
            {
                out.push(Step::Skill(s));
            }
        }
        category::BOOK => {
            let told = by_player && env.player == Some(tp);
            if told {
                out.push(Step::Se(92));
                out.push(Step::Frames(8));
            }
            let (stat, amount) = book(scene, tp, id, later);
            if told {
                out.push(Step::OpenInfo(Info::StatusUp { stat, amount }));
                out.push(Step::PanelStatus(3));
                out.push(Step::BgStatus(1));
                out.push(Step::WaitMessage);
                out.push(Step::CloseMessage);
            }
        }
        category::TOOL => match id {
            0 => {
                out.push(Step::ClearCmndTarget);
                out.push(Step::Se(228));
                out.push(Step::RemoveTrap(pos));
                out.push(Step::CloseMenuDisp);
                let boxent = scene.chars[tp].ent_root;
                out.push(Step::BoxEnt { on: ctp, value: u32::MAX });
                out.push(Step::Frames(20));
                out.push(Step::OpenInfo(Info::TrapDischarge));
                out.push(Step::PanelStatus(3));
                out.push(Step::BgStatus(1));
                out.push(Step::SleepAllThread);
                out.push(Step::StillOn);
                out.push(Step::WaitMessage);
                out.push(Step::CloseMessage);
                out.push(Step::BoxEnt { on: ctp, value: boxent });
                out.push(Step::Affect(Event::affect(ctp, ccp, 12, 0, 0, 0)));
            }
            1 => {
                out.push(Step::Se(96));
                env.dne_flag = 1;
                out.push(Step::DisableThEvent);
                if by_player && env.control_mode == 0 {
                    out.push(Step::ManualModeAI(1));
                    out.push(Step::SysMsg { kind: 6, id: env.sys_msg_id, param: 1 });
                    out.push(Step::SysMsg { kind: 7, id: env.sys_msg_id, param: 50 });
                }
                for &m in env.party.members.iter().flatten() {
                    scene.chars[m].no_death = true;
                }
                if later {
                    out.push(Step::WarpFlagOff);
                }
                out.push(Step::ChangeMenu(86));
            }
            2 => {
                out.push(Step::Se(97));
                out.push(Step::Frames(7));
                out.push(Step::OpenInfo(Info::ShowMap));
                out.push(Step::Frames(7));
                out.push(Step::WaitMap);
                out.push(Step::CloseMessage);
            }
            _ => {}
        },
        category::TRADE => {}
        category::IMPORTANT => match id {
            49 => {
                if later {
                    out.push(Step::VoicesOff);
                }
                grunty(scene, env, pn, &mut out)
            }
            47 => {
                out.push(Step::OpenInfo(Info::EpitaphUnknown));
                out.push(Step::Frames(8));
                out.push(Step::WaitMessage);
                out.push(Step::CloseMessage);
            }
            60 | 61 | 69 => {
                out.push(Step::OpenInfo(Info::InstallWarn));
                out.push(Step::Frames(8));
                out.push(Step::SeNote(196, 52));
                out.push(Step::Frames(8));
                out.push(Step::WaitMessage);
                out.push(Step::CloseMessage);
            }
            273..=280 => {
                out.push(Step::BookStart(id - 273));
                out.push(Step::CloseChat);
                out.push(Step::BgStatus(3));
                out.push(Step::Frames(1));
                out.push(Step::WaitBook);
            }
            _ => {
                if epitaph(id) {
                    out.push(Step::Epitaph { item: id, parody: env.parody });
                    out.push(Step::CloseMessage);
                }
            }
        },
        _ => {}
    }
    if by_player {
        env.player_pause = false;
        out.push(Step::Pause { on: Who::Char(cp), value: false });
    }
    out
}

/// The flute (important item 49): the player cannot die while the Grunty
/// comes (the party may be down: the game waits until it is not), the
/// enemies' conditions are cleared, the Grunty of kind `pn` rides in.
fn grunty(scene: &mut Scene, env: &mut ItemEnv, pn: i32, out: &mut Vec<Step>) {
    let pl = env.player;
    let ndf = pl.is_some_and(|p| scene.chars[p].no_death);
    if let Some(p) = pl {
        scene.chars[p].no_death = true;
        out.push(Step::NoDeath { on: Who::Char(p), value: true });
    }
    out.push(Step::CloseMenuDisp);
    out.push(Step::PanelStatus(3));
    out.push(Step::WakeAllThread);
    out.push(Step::Frames(1));
    out.push(Step::StillOff);
    out.push(Step::Frames(8));
    out.push(Step::CloseMessageInstant);
    out.push(Step::WaitParty);
    out.push(Step::Se(81));
    out.push(Step::ClearConditionAllEnemy);
    out.push(Step::Pucciguso(if (0..9).contains(&pn) { pn } else { 0 }));
    env.plw_pause = false;
    out.push(Step::PlwPause(false));
    out.push(Step::WaitRide);
    if let Some(p) = pl {
        scene.chars[p].no_death = ndf;
        out.push(Step::NoDeath { on: Who::Char(p), value: ndf });
    }
}

/// The skill request among an item use's steps, if any.
pub fn skill_of(steps: &[Step]) -> Option<&ItemSkill> {
    steps.iter().find_map(|s| match s {
        Step::Skill(k) => Some(k),
        _ => None,
    })
}

/// `ccMenuCtrl::AddSpcItem(ch, 12, id, num, ...)` (gcmn 0x00527950), a
/// party member given books: the member uses all `num` at once on itself
/// (`ccUseItemRequest(ch, ch, 12 << 16 | id, 0)` each), none of them kept.
pub fn give_stat_items(
    t: &Tables,
    scene: &mut Scene,
    env: &mut ItemEnv,
    ch: usize,
    id: i32,
    num: i32,
    rng: &mut dyn Rng,
) -> Vec<Step> {
    let code = (category::BOOK << 16) | (id & 0xffff);
    let mut out = Vec::new();
    for _ in 0..num.max(0) {
        out.extend(use_item_request(t, scene, env, ch, ch, code, 0, rng));
    }
    out
}

/// The player uses an item from the item menu: one is taken from Kite's
/// list (`DelItem(0, category, id, 1)`), then `ccUseItemRequest(player,
/// target, code, 0)`. `ccMenuCtrl::ItemMenu` (gcmn 0x0052db30) does this
/// for the items used at once (`target` the player) and keeps important
/// items (category 15); `ccMenuCtrl::TargetMenu` (gcmn 0x00531af0) does it
/// for the items used on a chosen target, taking one whatever its category
/// (`from_target_menu`). Returns the use's steps; nothing happens without
/// a player.
#[allow(clippy::too_many_arguments)]
pub fn menu_use_item(
    t: &Tables,
    scene: &mut Scene,
    save: &mut SaveData,
    env: &mut ItemEnv,
    target: usize,
    code: i32,
    from_target_menu: bool,
    rng: &mut dyn Rng,
) -> Vec<Step> {
    let Some(pl) = env.player else { return Vec::new() };
    let (cat, id) = (code >> 16, i32::from(code as i16));
    if from_target_menu || cat != category::IMPORTANT {
        del_item(save, 0, cat, id, 1);
    }
    use_item_request(t, scene, env, pl, target, code, 0, rng)
}

/// A party member the AI drives uses an item: `ccUseItemRequest(ch, target,
/// code, 0)`, then one is taken from the member's list
/// ([`consume_item_list`]). Returns the steps and what is left. This is
/// `ccAI::UseItem` (gcmn 0x00589410) once the party AI's checks pass, and the
/// ocarina command of `ChatCommandExecute` (the call at 0x0058d558: code 13:1
/// on itself). From Mutation on the member then remarks on it
/// ([`Step::UseItemRemark`]).
#[allow(clippy::too_many_arguments)]
pub fn ai_use_item(
    t: &Tables,
    scene: &mut Scene,
    save: &mut SaveData,
    env: &mut ItemEnv,
    ch: usize,
    target: usize,
    code: i32,
    rng: &mut dyn Rng,
) -> (Vec<Step>, i32) {
    let mut steps = use_item_request(t, scene, env, ch, target, code, 0, rng);
    let char_id = i32::from(scene.chars[ch].id());
    let (cat, id) = (code >> 16, code & 0xffff);
    let n = list_at(char_id)
        .and_then(|at| (0..ITEM_SLOTS).map(|k| get(save, at + 4 * k)).find(|&s| matches(s, cat, id)))
        .map_or(0, |s| s.num);
    let left = consume_item_list(save, char_id, code);
    if t.volume != piney_data::volume::Volume::Inf {
        steps.push(Step::UseItemRemark { last: n == 1, code });
    }
    (steps, left)
}

// The item lists ---------------------------------------------------------------

/// `saveData.itemList[18][40]` (+0x30): each character's items, a
/// `ccItemList` (`short id; char category; char num`) per slot, an empty
/// slot `{-1, -1, 0}`. Kite's (0) is the player's inventory.
pub const SAVE_ITEM_LIST: usize = 0x30;
pub const ITEM_SLOTS: usize = 40;
/// `saveData.plItemList[99]` (+0xb70): the player's storage.
pub const SAVE_PL_ITEM_LIST: usize = 0xb70;
pub const PL_ITEM_SLOTS: usize = 99;
/// `saveData.impItemList[320]` (+0xcfc): the player's important items, a
/// count per row of category 15.
pub const SAVE_IMP_ITEM_LIST: usize = 0xcfc;
pub const IMP_ITEMS: usize = 320;

/// `addItemCategoryTbl`: the order `AddItem` sorts a list in, 15
/// (category, rows) pairs; an item of a category not listed, or of a row
/// not below the count, is dropped by the sort.
pub type CategoryOrder = [(i16, i16); 15];

/// The volume's `addItemCategoryTbl` (`tables::fieldui`).
pub fn category_order(volume: piney_data::volume::Volume) -> CategoryOrder {
    let t = piney_data::tables::fieldui::of(volume).category_order();
    std::array::from_fn(|k| t.get(k).map_or((0, 0), |r| (r[0], r[1])))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Slot {
    id: i16,
    cat: i8,
    num: i8,
}

const EMPTY: Slot = Slot { id: -1, cat: -1, num: 0 };

fn get(save: &SaveData, at: usize) -> Slot {
    Slot { id: save.i16(at), cat: save.u8(at + 2) as i8, num: save.u8(at + 3) as i8 }
}

fn put(save: &mut SaveData, at: usize, s: Slot) {
    save.set_i16(at, s.id);
    save.set_u8(at + 2, s.cat as u8);
    save.set_u8(at + 3, s.num as u8);
}

/// Where character `ch`'s list starts, if it lies in the save (the game
/// does not check `ch`): characters 18-20's in the extension (Mutation's
/// `GetItemList` 0x0017a9d0).
fn list_at(ch: i32) -> Option<usize> {
    if ch >= 18 {
        let at = piney_data::save::by_id::item_list(ch as usize);
        return (at + 4 * ITEM_SLOTS <= piney_data::save::FULL).then_some(at);
    }
    let at = SAVE_ITEM_LIST as i64 + 160 * i64::from(ch);
    (at >= 0 && at as usize + 4 * ITEM_SLOTS <= piney_data::save::SIZE).then_some(at as usize)
}

/// The important-item count of row `id`, if it lies in the save.
fn imp_at(id: i32) -> Option<usize> {
    let at = SAVE_IMP_ITEM_LIST as i64 + i64::from(id);
    (at >= 0 && (at as usize) < piney_data::save::SIZE).then_some(at as usize)
}

fn matches(s: Slot, cat: i32, id: i32) -> bool {
    i32::from(s.id) == id && i32::from(s.cat) == cat
}

/// `AddItem`'s and `AddPlItem`'s common part: the stack of `(cat, id)`
/// (the last one in the list) or else the first empty slot takes `num`
/// (`cap` counts the sum; the storage caps the stored byte instead), then
/// the list is rebuilt in `order`: for each category, its items by row
/// ascending, the first of equal rows first, from the list's front.
#[allow(clippy::too_many_arguments)]
fn add_to_list(save: &mut SaveData, at: usize, n: usize, order: &CategoryOrder, cat: i32, id: i32, num: i32, pl: bool) {
    let mut tmp: Vec<Slot> = Vec::with_capacity(n);
    let mut pick: Option<usize> = None;
    for k in 0..n {
        let s = get(save, at + 4 * k);
        tmp.push(s);
        // A later stack of the item wins over an earlier one; an empty
        // slot only while nothing has been picked.
        if matches(s, cat, id) || (pick.is_none() && s.id < 0) {
            pick = Some(k);
        }
        put(save, at + 4 * k, EMPTY);
    }
    if let Some(k) = pick {
        let s = &mut tmp[k];
        s.id = id as i16;
        s.cat = cat as i8;
        if pl {
            let v = s.num.wrapping_add(num as i8);
            s.num = if v >= 100 { 99 } else { v };
        } else {
            let v = i32::from(s.num) + num;
            s.num = if v < 100 { v as i8 } else { 99 };
        }
    }
    for &(c, rows) in order {
        for _ in 0..n {
            let mut best = i32::from(rows);
            let mut count = 0i8;
            let mut found = None;
            for (k, s) in tmp.iter().enumerate() {
                if i32::from(c) == i32::from(s.cat) && i32::from(s.id) < best {
                    best = i32::from(s.id);
                    count = s.num;
                    found = Some(k);
                }
            }
            let Some(k) = found else { break };
            tmp[k] = EMPTY;
            if let Some(j) = (0..n).find(|&j| get(save, at + 4 * j).id < 0) {
                put(save, at + 4 * j, Slot { id: best as i16, cat: c as i8, num: count });
            }
        }
    }
}

fn del_from_list(save: &mut SaveData, at: usize, n: usize, cat: i32, id: i32, num: i32) {
    for k in 0..n {
        let mut s = get(save, at + 4 * k);
        if matches(s, cat, id) {
            if num < i32::from(s.num) {
                s.num = (i32::from(s.num) - num) as i8;
                put(save, at + 4 * k, s);
            } else {
                put(save, at + 4 * k, EMPTY);
            }
            return;
        }
    }
}

fn num_in_list(save: &SaveData, at: usize, n: usize, cat: i32, id: i32) -> i32 {
    (0..n).map(|k| get(save, at + 4 * k)).find(|&s| matches(s, cat, id)).map_or(0, |s| i32::from(s.num))
}

fn free_slot(save: &SaveData, at: usize, n: usize) -> i32 {
    (0..n)
        .find(|&k| {
            let s = get(save, at + 4 * k);
            s.id < 0 && s.cat < 0
        })
        .map_or(-1, |k| k as i32)
}

/// `ccSaveData::AddItem(ch, category, id, num)` (main 0x00177730): `num`
/// more of an item in character `ch`'s list. Kite's important items
/// (category 15) are counted in `impItemList`, up to 99. Otherwise the
/// item's stack (or else the first empty slot) grows to at most 99 and the
/// list is sorted by [`CategoryOrder`], which drops what it does not list;
/// a full list without the item gains nothing.
pub fn add_item(save: &mut SaveData, order: &CategoryOrder, ch: i32, category: i32, id: i32, num: i32) {
    if ch == 0 && category == category::IMPORTANT {
        if let Some(at) = imp_at(id) {
            let v = i32::from(save.u8(at) as i8) + num;
            save.set_u8(at, if v < 100 { v as u8 } else { 99 });
        }
        return;
    }
    if let Some(at) = list_at(ch) {
        add_to_list(save, at, ITEM_SLOTS, order, category, id, num, false);
    }
}

/// `ccSaveData::DelItem(ch, category, id, num)` (main 0x00177af0): `num`
/// fewer of an item: Kite's important items down to 0; otherwise the first
/// stack of it loses `num`, emptied when that is all it had.
pub fn del_item(save: &mut SaveData, ch: i32, category: i32, id: i32, num: i32) {
    if ch == 0 && category == category::IMPORTANT {
        if let Some(at) = imp_at(id) {
            let v = i32::from(save.u8(at) as i8) - num;
            save.set_u8(at, if v < 0 { 0 } else { v as u8 });
        }
        return;
    }
    if let Some(at) = list_at(ch) {
        del_from_list(save, at, ITEM_SLOTS, category, id, num);
    }
}

/// `ccSaveData::GetItemNum(ch, category, id)` (main 0x00177990): how many
/// of an item a character carries.
pub fn get_item_num(save: &SaveData, ch: i32, category: i32, id: i32) -> i32 {
    if ch == 0 && category == category::IMPORTANT {
        return imp_at(id).map_or(0, |at| i32::from(save.u8(at) as i8));
    }
    list_at(ch).map_or(0, |at| num_in_list(save, at, ITEM_SLOTS, category, id))
}

/// `ccSaveData::GetItemSlot(ch)` (main 0x00177a20): the first empty slot
/// of a character's list, -1 when full.
pub fn get_item_slot(save: &SaveData, ch: i32) -> i32 {
    list_at(ch).map_or(-1, |at| free_slot(save, at, ITEM_SLOTS))
}

/// `ccSaveData::AddPlItem(category, id, num)` (main 0x00177bc0): the same
/// as [`add_item`] on the storage (99 slots); the stack's count is kept as
/// a byte before it is capped at 99.
pub fn add_pl_item(save: &mut SaveData, order: &CategoryOrder, category: i32, id: i32, num: i32) {
    add_to_list(save, SAVE_PL_ITEM_LIST, PL_ITEM_SLOTS, order, category, id, num, true);
}

/// `ccSaveData::DelPlItem(category, id, num)` (main 0x00177dd0).
pub fn del_pl_item(save: &mut SaveData, category: i32, id: i32, num: i32) {
    del_from_list(save, SAVE_PL_ITEM_LIST, PL_ITEM_SLOTS, category, id, num);
}

/// `ccSaveData::GetPlItemNum(category, id)` (main 0x00177e50).
pub fn get_pl_item_num(save: &SaveData, category: i32, id: i32) -> i32 {
    num_in_list(save, SAVE_PL_ITEM_LIST, PL_ITEM_SLOTS, category, id)
}

/// `ccSaveData::GetPlItemSlot()` (main 0x00177a90).
pub fn get_pl_item_slot(save: &SaveData) -> i32 {
    free_slot(save, SAVE_PL_ITEM_LIST, PL_ITEM_SLOTS)
}

/// `ccAI::ConsumeItemList(code)` (gcmn 0x00588860): one of item `code`
/// taken from character `char_id`'s list (its first stack; emptied at 0 or
/// below). Returns what is left, -1 when it carries none. The row is the
/// code's low half as unsigned, so a row of 0x8000 up is never found.
pub fn consume_item_list(save: &mut SaveData, char_id: i32, code: i32) -> i32 {
    let (cat, id) = (code >> 16, code & 0xffff);
    let Some(at) = list_at(char_id) else { return -1 };
    for k in 0..ITEM_SLOTS {
        let mut s = get(save, at + 4 * k);
        if matches(s, cat, id) {
            s.num = s.num.wrapping_sub(1);
            if s.num <= 0 {
                s = EMPTY;
            }
            put(save, at + 4 * k, s);
            return i32::from(s.num);
        }
    }
    -1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A book read outside a fight raises the record: book 0 physical
    /// attack by 10, capped at 999; the rest of the record as it was.
    #[test]
    fn a_book_on_the_record() {
        let mut p = crate::param::SpcParam::default();
        p.elm[elm::P_ATK] = 50;
        p.max_hp = 100;
        let before = p;
        assert_eq!(book_on_record(&mut p, 0, piney_data::volume::Volume::Inf), (2, 1));
        assert_eq!(p.elm[elm::P_ATK], 60);
        assert_eq!((p.max_hp, p.elm[elm::P_DEF]), (before.max_hp, before.elm[elm::P_DEF]));
        p.elm[elm::P_ATK] = 995;
        book_on_record(&mut p, 0, piney_data::volume::Volume::Inf);
        assert_eq!(p.elm[elm::P_ATK], 999);
    }
    use crate::chara::Char;
    use crate::param::{Base, SpcParam};
    use crate::rand::Rand;

    const ORDER: CategoryOrder = [
        (10, 24),
        (13, 3),
        (11, 72),
        (12, 34),
        (14, 22),
        (0, 82),
        (1, 77),
        (2, 97),
        (3, 75),
        (4, 74),
        (5, 76),
        (6, 69),
        (7, 68),
        (8, 67),
        (9, 68),
    ];

    fn pc(id: i16) -> Char {
        let p = SpcParam {
            base: Base { ty: 1, id, level: 1, ..Base::default() },
            max_hp: 100,
            max_sp: 50,
            ..Default::default()
        };
        Char::pc(p)
    }

    fn slots(save: &SaveData, ch: i32) -> Vec<(i16, i8, i8)> {
        let at = list_at(ch).unwrap();
        (0..ITEM_SLOTS).map(|k| get(save, at + 4 * k)).filter(|s| s.id >= 0).map(|s| (s.id, s.cat, s.num)).collect()
    }

    fn empty_lists() -> SaveData {
        let mut save = SaveData::new();
        for ch in 0..18 {
            let at = list_at(ch).unwrap();
            for k in 0..ITEM_SLOTS {
                put(&mut save, at + 4 * k, EMPTY);
            }
        }
        save
    }

    #[test]
    fn useful_by_category() {
        assert_eq!(item_useful(10, 5), 1);
        assert_eq!(item_useful(12, 40), 2);
        assert_eq!(item_useful(13, 0), 1);
        assert_eq!(item_useful(13, 2), 2);
        assert_eq!(item_useful(14, 0), 0);
        assert_eq!(item_useful(15, 49), 2);
        assert_eq!(item_useful(15, 50), 0);
        assert_eq!(item_useful(15, 280), 2);
        assert_eq!(item_useful(3, 1), 0);
    }

    #[test]
    fn add_sorts_and_caps() {
        let mut save = empty_lists();
        add_item(&mut save, &ORDER, 1, 11, 5, 3);
        add_item(&mut save, &ORDER, 1, 10, 7, 98);
        add_item(&mut save, &ORDER, 1, 10, 2, 1);
        add_item(&mut save, &ORDER, 1, 10, 7, 5);
        // Recovery before spells, rows ascending; the stack capped at 99.
        assert_eq!(slots(&save, 1), [(2, 10, 1), (7, 10, 99), (5, 11, 3)]);
        // Category 15 is not in the order: dropped from anyone but Kite.
        add_item(&mut save, &ORDER, 1, 15, 3, 1);
        assert_eq!(slots(&save, 1).len(), 3);
        add_item(&mut save, &ORDER, 0, 15, 3, 120);
        assert_eq!(get_item_num(&save, 0, 15, 3), 99);
    }

    #[test]
    fn delete_and_consume() {
        let mut save = empty_lists();
        add_item(&mut save, &ORDER, 2, 10, 0, 3);
        del_item(&mut save, 2, 10, 0, 1);
        assert_eq!(get_item_num(&save, 2, 10, 0), 2);
        assert_eq!(consume_item_list(&mut save, 2, 10 << 16), 1);
        assert_eq!(consume_item_list(&mut save, 2, 10 << 16), 0);
        assert_eq!(consume_item_list(&mut save, 2, 10 << 16), -1);
        assert_eq!(get_item_slot(&save, 2), 0);
    }

    #[test]
    fn books_raise_stats() {
        let t = Tables::default();
        let mut scene = Scene::default();
        let mut c = pc(0);
        c.spc_mut().unwrap().elm[elm::P_ATK] = 995;
        c.hp = 9990;
        scene.add(c, 0);
        let mut env = ItemEnv { player: Some(0), ..ItemEnv::default() };
        let mut rng = Rand::default();
        let steps = use_item_request(&t, &mut scene, &mut env, 0, 0, 12 << 16, 0, &mut rng);
        assert_eq!(scene.chars[0].spc().unwrap().elm[elm::P_ATK], 999);
        assert!(steps.contains(&Step::OpenInfo(Info::StatusUp { stat: 2, amount: 1 })));
        use_item_request(&t, &mut scene, &mut env, 0, 0, 12 << 16 | 30, 0, &mut rng);
        assert_eq!(scene.chars[0].hp, 9999);
        assert_eq!(scene.chars[0].spc().unwrap().max_hp, 130);
        assert!(!env.player_pause);
    }

    /// Issue #8: a Speed Charm (skill 177) Kite casts on himself (stype 2)
    /// aims his cast at himself, as `_ccSkillRequest` does; with no
    /// target his act never casts and the skill never ends. On the target
    /// alone (stype 1) the caster's aim is left as it was.
    #[test]
    fn an_item_cast_through_the_user_aims_it() {
        let t = Tables::default();
        let mut scene = Scene::default();
        scene.add(pc(0), 0);
        scene.add(pc(1), 0);
        let mut rng = Rand::default();
        let s = item_skill_request(&t, &mut scene, 0, 0, 177, 1, false, &mut rng).unwrap();
        assert!(s.request.is_some());
        let k = &scene.chars[0];
        assert_eq!((k.skill_id, k.skill_status, k.target_char), (177, 9, Some(0)));
        item_skill_request(&t, &mut scene, 1, 0, 177, 0, false, &mut rng);
        assert_eq!(scene.chars[1].target_char, None);
    }

    #[test]
    fn soul_and_wine_affects() {
        let t = Tables::default();
        let mut scene = Scene::default();
        scene.add(pc(0), 0);
        scene.add(pc(1), 0);
        let mut env = ItemEnv::default();
        let mut rng = Rand::default();
        let steps = use_item_request(&t, &mut scene, &mut env, 0, 1, 10 << 16 | 21, 0, &mut rng);
        let affects: Vec<_> = steps.iter().filter(|s| matches!(s, Step::Affect(_))).collect();
        assert_eq!(affects.len(), 2);
        assert_eq!(steps[0], Step::EffHeal(Who::Char(1)));
        // Off the command lists: nothing.
        scene.pc_list.clear();
        assert!(use_item_request(&t, &mut scene, &mut env, 0, 1, 10 << 16 | 18, 0, &mut rng).is_empty());
    }

    #[test]
    fn ocarina_protects_the_party() {
        let t = Tables::default();
        let mut scene = Scene::default();
        scene.add(pc(0), 0);
        scene.add(pc(1), 0);
        let mut env = ItemEnv { player: Some(0), ..ItemEnv::default() };
        env.party.members = [Some(0), Some(1), None];
        let mut rng = Rand::default();
        let steps = use_item_request(&t, &mut scene, &mut env, 0, 0, 13 << 16 | 1, 0, &mut rng);
        assert!(scene.chars.iter().all(|c| c.no_death));
        assert_eq!(env.dne_flag, 1);
        assert!(steps.contains(&Step::ChangeMenu(86)));
        assert!(steps.contains(&Step::ManualModeAI(1)));
    }

    #[test]
    fn menu_use_takes_one_first() {
        let t = Tables::default();
        let mut scene = Scene::default();
        scene.add(pc(0), 0);
        let mut save = empty_lists();
        add_item(&mut save, &ORDER, 0, 10, 18, 2);
        add_item(&mut save, &ORDER, 0, 15, 42, 1);
        let mut env = ItemEnv { player: Some(0), ..ItemEnv::default() };
        let mut rng = Rand::default();
        let steps = menu_use_item(&t, &mut scene, &mut save, &mut env, 0, 10 << 16 | 18, false, &mut rng);
        assert_eq!(get_item_num(&save, 0, 10, 18), 1);
        assert!(matches!(steps[1], Step::Affect(Event::Affect { kind: 8, p: [100, 0, 0], .. })));
        // Important items are kept.
        menu_use_item(&t, &mut scene, &mut save, &mut env, 0, 15 << 16 | 42, false, &mut rng);
        assert_eq!(get_item_num(&save, 0, 15, 42), 1);
    }

    #[test]
    fn ai_use_consumes_after() {
        let t = Tables::default();
        let mut scene = Scene::default();
        scene.add(pc(3), 0);
        let mut save = empty_lists();
        add_item(&mut save, &ORDER, 3, 13, 1, 1);
        let mut env = ItemEnv::default();
        let mut rng = Rand::default();
        let (steps, left) = ai_use_item(&t, &mut scene, &mut save, &mut env, 0, 0, 13 << 16 | 1, &mut rng);
        assert_eq!(left, 0);
        assert_eq!(get_item_slot(&save, 3), 0);
        assert!(steps.contains(&Step::ChangeMenu(86)));
    }

    #[test]
    fn books_on_a_foe_land_in_its_blocks() {
        let t = Tables::default();
        let mut scene = Scene::default();
        let mut row = crate::param::FoeRow::default();
        row.base.ty = 0x20;
        scene.add(Char::foe(row), 1);
        let mut env = ItemEnv::default();
        let mut rng = Rand::default();
        // Power Book: elm[0] at +0x28, temp[4] of a ccEnemyParam.
        use_item_request(&t, &mut scene, &mut env, 0, 0, 12 << 16, 0, &mut rng);
        // An energy sutra: maxHP at +0x24, temp[2].
        use_item_request(&t, &mut scene, &mut env, 0, 0, 12 << 16 | 32, 0, &mut rng);
        let f = scene.chars[0].foe_state().unwrap();
        assert_eq!(f.temp[4], 10);
        assert_eq!(f.temp[2], 10);
        assert_eq!(scene.chars[0].hp, 10);
    }
}
