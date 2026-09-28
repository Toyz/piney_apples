//! `battle_probe` requests for items (`piney_battle::item`), as
//! `tools/test_battle_items_rs.py` sends them:
//!
//! - `useful CAT ID`: `ccCheckItemUseful`
//! - `skilluseful SCENE PARTY[3] NSORT (CH DIST INVIEW)... SID`:
//!   `ccCheckSkillUseful`
//! - `itemskill SCENE KIND CP TP SID PARAM FLAG RUNNING STATE`: KIND 0
//!   `ccItemSkillRequest`, 1 `ccItemSkillRequestParam`, 2
//!   `ccItemSkillCompel`
//! - `useitem SCENE ENV CP TP CODE PN POS[4] STATE`: `ccUseItemRequest`
//! - `give SCENE ENV CH ID NUM STATE`: `ccMenuCtrl::AddSpcItem` with books
//! - `aiuse SCENE ENV CH TP CODE LIST STATE`: `ccAI::UseItem` past its
//!   checks (the member's list as hex bytes)
//! - `inv OP CH CAT ID NUM LIST PL IMP`: the item lists (`add del num slot
//!   addpl delpl numpl slotpl consume`), the lists as hex bytes
//!
//! A SCENE is `N CHAR... NPC i... NENE i...`; an ENV is `PLAYER PARTY[3]
//! PAUSE PLWPAUSE DNE PARODY CTRLMODE SYSMSGID` (-1 for no player or
//! member). Characters are named `c0`, `c1`, ... in the calls.

use std::sync::OnceLock;

use piney_battle::event::{Event, Who};
use piney_battle::item::{self, Info, ItemEnv, ItemSkill, SortEntry, Step};
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_data::iso::Iso;
use piney_data::save::SaveData;

use crate::{Toks, list, read_char, state};

/// Answers a command this module knows; None for any other.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    Some(match cmd {
        "useful" => {
            let c = t.i32();
            let id = t.i32();
            item::item_useful(c, id).to_string()
        }
        "skilluseful" => skill_useful(t, tables),
        "itemskill" => item_skill(t, tables),
        "useitem" | "give" | "aiuse" => use_item(cmd, t, tables),
        "inv" => inventory(t),
        _ => return None,
    })
}

fn read_scene(t: &mut Toks) -> Scene {
    let n = t.int() as usize;
    let mut s = Scene::default();
    for _ in 0..n {
        let c = read_char(t);
        s.chars.push(c);
    }
    let k = t.int();
    s.pc_list = (0..k).map(|_| t.int() as usize).collect();
    let k = t.int();
    s.ene_list = (0..k).map(|_| t.int() as usize).collect();
    s
}

fn opt(v: i64) -> Option<usize> {
    usize::try_from(v).ok()
}

fn read_party(t: &mut Toks) -> piney_battle::exp::Party {
    let mut p = piney_battle::exp::Party::default();
    for n in 0..3 {
        p.members[n] = opt(t.int());
        if p.members[n].is_some() {
            p.num += 1;
        }
    }
    p
}

fn skill_useful(t: &mut Toks, tables: &Tables) -> String {
    let scene = read_scene(t);
    let party = read_party(t);
    let n = t.int();
    let sorted: Vec<SortEntry> =
        (0..n).map(|_| SortEntry { ch: t.int() as usize, dist: t.u32(), in_view: t.int() != 0 }).collect();
    let sid = t.i32();
    let drain_off = t.int() != 0;
    u8::from(item::skill_useful(tables, &scene, &party, &sorted, sid, drain_off)).to_string()
}

fn name(i: usize) -> String {
    format!("\"c{i}\"")
}

fn who(w: Who, cp: usize, tp: usize) -> String {
    match w {
        Who::Me => name(cp),
        Who::Target => name(tp),
        Who::Char(i) => name(i),
        Who::Nobody => "0".to_string(),
    }
}

/// A character's state as the harness reads it, with `[skillID,
/// skillStatus, noDeath, +0x140, pauseSW]`.
fn char_state(s: &Scene, i: usize, pause: bool) -> String {
    let c = &s.chars[i];
    let st = state(c);
    format!(
        "{},\"extra\":{}}}",
        &st[..st.len() - 1],
        list([
            i64::from(c.skill_id),
            i64::from(c.skill_status),
            i64::from(c.no_death),
            i64::from(c.ent_root),
            i64::from(pause)
        ])
    )
}

fn skill_json(k: Option<&ItemSkill>, cp: usize, tp: usize) -> String {
    let Some(k) = k else { return "null".into() };
    let Some(r) = &k.request else { return "null".into() };
    format!(
        "{{\"sid\":{},\"stype\":{},\"param\":{},\"ac\":{},\"modify\":{},\"hold\":{},\"compel\":{},\"creator\":{},\
         \"target\":{}}}",
        k.sid,
        k.stype,
        k.param.unwrap_or(0),
        r.ac_flag,
        u8::from(r.modify),
        u8::from(r.hold),
        u8::from(k.compel),
        name(cp),
        name(tp)
    )
}

/// The calls `_ccSkillRequest` makes, as the harness records them.
fn skill_calls(k: &ItemSkill, cp: usize, tp: usize, out: &mut Vec<String>) {
    let Some(r) = &k.request else { return };
    for &(w, a, b) in &r.skill_start {
        out.push(format!("[\"effSkillStart\",{},{},{},{}]", who(w, cp, tp), k.sid, a, b));
    }
    if r.words {
        out.push(format!("[\"ccWordsPlay\",{},{}]", k.sid, name(cp)));
    }
}

fn item_skill(t: &mut Toks, tables: &Tables) -> String {
    let mut scene = read_scene(t);
    let kind = t.int();
    let cp = t.int() as usize;
    let tp = t.int() as usize;
    let sid = t.i32();
    let param = t.i32();
    let flag = t.i32();
    let running = t.int() != 0;
    let mut rng = Rand(t.int() as u64);
    let k = match kind {
        0 => item::item_skill_request(tables, &mut scene, cp, tp, sid, flag, running, &mut rng),
        1 => item::item_skill_request_param(tables, &mut scene, cp, tp, sid, param, flag, running, &mut rng),
        _ => item::item_skill_compel(tables, &mut scene, cp, tp, sid, flag, running, &mut rng),
    };
    let mut calls = Vec::new();
    if let Some(k) = &k {
        skill_calls(k, cp, tp, &mut calls);
    }
    let chars: Vec<String> = (0..scene.chars.len()).map(|i| char_state(&scene, i, false)).collect();
    format!(
        "{{\"calls\":[{}],\"chars\":[{}],\"skill\":{},\"rand\":{}}}",
        calls.join(","),
        chars.join(","),
        skill_json(k.as_ref(), cp, tp),
        rng.0
    )
}

fn info(i: Info) -> String {
    let sep = |s: &str, n: i32| format!("<{s}:{n}>");
    match i {
        Info::StatusUp { stat, amount } => format!(
            "[\"OpenInfo\",\"{}{}#{}.\",0,0,0,-1,-1]",
            sep("statusUpStr", stat),
            sep("statusUpStr", i32::from(amount < 0)),
            amount.abs()
        ),
        Info::TrapDischarge => "[\"OpenInfo\",\"trapDischargeStr\",0,0,0,-1,-1]".into(),
        Info::ShowMap => "[\"OpenInfo\",\"showMapInfo\",0,0,0,-1,-1]".into(),
        Info::EpitaphUnknown => format!("[\"OpenInfo\",\"{}\",0,0,0,-1,-1]", sep("epitaphStr0X", 0)),
        Info::InstallWarn => format!(
            "[\"OpenInfo\",\"{}\",\"{}\",\"{}\",0,-1,-1]",
            sep("installWarnStr", 0),
            sep("installWarnStr", 1),
            sep("installWarnStr", 2)
        ),
    }
}

/// What the probe tracks while it turns steps into the harness's records.
struct Walk {
    cp: usize,
    tp: usize,
    player: Option<usize>,
    boxent: Vec<u32>,
    no_death: Vec<bool>,
    panel: i16,
    bg: i16,
    page: i32,
    out: Vec<String>,
}

impl Walk {
    fn push(&mut self, s: String) {
        self.out.push(s);
    }

    fn frames(&mut self, n: u32) {
        // Adjacent frame runs read as one in the harness.
        if let Some(last) = self.out.last_mut()
            && let Some(rest) = last.strip_prefix("[\"frames\",")
        {
            let k: u32 = rest.trim_end_matches(']').parse().unwrap();
            *last = format!("[\"frames\",{}]", k + n);
            return;
        }
        self.out.push(format!("[\"frames\",{n}]"));
    }

    fn step(&mut self, s: &Step) {
        let (cp, tp) = (self.cp, self.tp);
        let idx = |w: Who| match w {
            Who::Me => cp,
            Who::Target => tp,
            Who::Char(i) => i,
            // (no item rule names a null character)
            Who::Nobody => usize::MAX,
        };
        match s {
            Step::Affect(Event::Affect { on, by, kind, p }) => self.push(format!(
                "[\"EntryAffect\",{},{},{},{},{},{}]",
                who(*on, cp, tp),
                who(*by, cp, tp),
                kind,
                p[0],
                p[1],
                p[2]
            )),
            Step::Affect(e) => self.push(format!("[\"event\",\"{e:?}\"]")),
            Step::Skill(k) => {
                let mut v = Vec::new();
                skill_calls(k, cp, tp, &mut v);
                self.out.extend(v);
            }
            Step::Se(n) => self.push(format!("[\"ccSeOn\",{n}]")),
            Step::SeNote(n, c) => self.push(format!("[\"ccSeOnNote\",{n},{c}]")),
            Step::EffHeal(w) => self.push(format!("[\"effHeal\",{},1]", who(*w, cp, tp))),
            Step::CloseMenuDisp => self.push("[\"CloseMenuDisp\"]".into()),
            Step::Frames(n) => self.frames(*n),
            Step::OpenInfo(i) => self.push(info(*i)),
            Step::PanelStatus(v) => self.panel = *v,
            Step::BgStatus(v) => self.bg = *v,
            Step::WaitMessage => self.push("[\"WaitMessage\"]".into()),
            Step::CloseMessage => self.push("[\"Close\"]".into()),
            Step::CloseMessageInstant => self.push("[\"CloseInstant\"]".into()),
            Step::ClearCmndTarget => self.push("[\"ccChangeCmndTarget\",0]".into()),
            Step::RemoveTrap(p) => self.push(format!("[\"effRemoveTrap\",{},{},{},{},-1,-1]", p[0], p[1], p[2], p[3])),
            Step::BoxEnt { on, value } => self.boxent[idx(*on)] = *value,
            // Stores the harness does not read back (ccSnd +0x139,
            // worldman->warpFlag).
            Step::VoicesOff | Step::WarpFlagOff => {}
            Step::UseItemRemark { last, code } => {
                let name = if *last { "ChatMessageUseLastItem" } else { "ChatMessageUseItem" };
                self.push(format!("[\"{name}\",{code}]"))
            }
            Step::SleepAllThread => {
                let v = self.boxent[tp];
                self.push(format!("[\"ccSleepAllThread\",{v}]"))
            }
            Step::WakeAllThread => self.push("[\"ccWakeAllThread\"]".into()),
            Step::StillOn => self.push("[\"StillOn\"]".into()),
            Step::StillOff => self.push("[\"StillOff\"]".into()),
            Step::DisableThEvent => self.push("[\"ccDisableThEvent\"]".into()),
            Step::ManualModeAI(ev) => {
                let p = self.player.map_or("0".into(), name);
                self.push(format!("[\"ManualModeAI\",{p},{ev}]"))
            }
            Step::SysMsg { kind, id, param } => {
                self.push(format!("[\"ccAISysMsgSend\",{kind},-1,{id},65535,0,{param}]"))
            }
            Step::ChangeMenu(n) => self.push(format!("[\"ChangeMenu\",{n}]")),
            Step::WaitMap => self.push("[\"WaitMap\"]".into()),
            Step::NoDeath { on, value } => self.no_death[idx(*on)] = *value,
            Step::WaitParty => {
                let nd = self.player.is_some_and(|p| self.no_death[p]);
                self.push(format!("[\"WaitParty\",{}]", u8::from(nd)))
            }
            Step::ClearConditionAllEnemy => self.push("[\"ccClearConditionAllEnemy\"]".into()),
            Step::Pucciguso(n) => self.push(format!("[\"ccPuccigusoStart\",{n}]")),
            Step::PlwPause(_) | Step::Pause { .. } => {}
            Step::WaitRide => self.push("[\"WaitRide\"]".into()),
            Step::Epitaph { strs, pages } => self.push(format!("[\"ccEpitaphMsg\",{strs},{pages}]")),
            Step::BookStart(page) => {
                self.page = *page;
                self.push("[\"ccStartThread\",\"ccThBook\",35,4096]".into())
            }
            Step::CloseChat => self.push("[\"CloseChat\"]".into()),
            Step::WaitBook => {
                let p = self.page;
                self.push(format!("[\"WaitBook\",{p}]"))
            }
        }
    }
}

fn read_env(t: &mut Toks) -> ItemEnv {
    ItemEnv {
        player: opt(t.int()),
        party: read_party(t),
        player_pause: t.int() != 0,
        plw_pause: t.int() != 0,
        dne_flag: t.i32(),
        parody: t.int() != 0,
        control_mode: t.i32(),
        sys_msg_id: t.int() as u16,
        running_attack: t.int() != 0,
    }
}

fn use_item(cmd: &str, t: &mut Toks, tables: &Tables) -> String {
    let mut scene = read_scene(t);
    let mut env = read_env(t);
    let mut extra = String::new();
    let (cp, tp, steps, rng) = match cmd {
        "useitem" => {
            let cp = t.int() as usize;
            let tp = t.int() as usize;
            let code = t.i32();
            let pn = t.i32();
            scene.chars[tp].pos = std::array::from_fn(|_| t.u32());
            let mut rng = Rand(t.int() as u64);
            let steps = item::use_item_request(tables, &mut scene, &mut env, cp, tp, code, pn, &mut rng);
            (cp, tp, steps, rng)
        }
        "aiuse" => {
            let ch = t.int() as usize;
            let tp = t.int() as usize;
            let code = t.i32();
            let at = piney_data::save::by_id::item_list(scene.chars[ch].id() as usize);
            let mut save = SaveData::new();
            hex_in(&mut save, at, t.word());
            let mut rng = Rand(t.int() as u64);
            let (steps, _left) = item::ai_use_item(tables, &mut scene, &mut save, &mut env, ch, tp, code, &mut rng);
            extra = format!(",\"list\":\"{}\"", hex_out(&save, at, 160));
            (ch, tp, steps, rng)
        }
        _ => {
            let ch = t.int() as usize;
            let id = t.i32();
            let num = t.i32();
            let mut rng = Rand(t.int() as u64);
            let steps = item::give_stat_items(tables, &mut scene, &mut env, ch, id, num, &mut rng);
            // AddSpcItem answers 2 for books.
            extra = ",\"ret\":2".into();
            (ch, ch, steps, rng)
        }
    };
    // The walk starts from the state before the call: every temporary
    // write in the steps is put back by the end, so the final state serves.
    let mut w = Walk {
        cp,
        tp,
        player: env.player,
        boxent: scene.chars.iter().map(|c| c.ent_root).collect(),
        no_death: scene.chars.iter().map(|c| c.no_death).collect(),
        panel: 0,
        bg: 0,
        page: 0,
        out: Vec::new(),
    };
    for s in &steps {
        w.step(s);
    }
    let chars: Vec<String> =
        (0..scene.chars.len()).map(|i| char_state(&scene, i, env.player == Some(i) && env.player_pause)).collect();
    format!(
        "{{\"calls\":[{}],\"chars\":[{}],\"skill\":{},\"rand\":{},\"env\":{},\"menu\":{}{}}}",
        w.out.join(","),
        chars.join(","),
        skill_json(item::skill_of(&steps), cp, tp),
        rng.0,
        list([i32::from(env.plw_pause), env.dne_flag]),
        list([w.panel, w.bg]),
        extra
    )
}

/// `addItemCategoryTbl`, read once from the disc the probe was given.
fn order() -> &'static item::CategoryOrder {
    static ORDER: OnceLock<item::CategoryOrder> = OnceLock::new();
    ORDER.get_or_init(|| {
        let path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
        let volume = Iso::open(&path).and_then(|mut iso| iso.volume()).expect("the disc");
        item::category_order(volume)
    })
}

fn hex_in(save: &mut SaveData, at: usize, s: &str) {
    for (k, i) in (0..s.len()).step_by(2).enumerate() {
        save.set_u8(at + k, u8::from_str_radix(&s[i..i + 2], 16).unwrap());
    }
}

fn hex_out(save: &SaveData, at: usize, n: usize) -> String {
    save.record()[at..at + n].iter().map(|b| format!("{b:02x}")).collect()
}

fn inventory(t: &mut Toks) -> String {
    let op = t.word().to_string();
    let ch = t.i32();
    let cat = t.i32();
    let id = t.i32();
    let num = t.i32();
    let list_at = item::SAVE_ITEM_LIST + 160 * ch as usize;
    let mut save = SaveData::new();
    hex_in(&mut save, list_at, t.word());
    hex_in(&mut save, item::SAVE_PL_ITEM_LIST, t.word());
    hex_in(&mut save, item::SAVE_IMP_ITEM_LIST, t.word());
    let ret = match op.as_str() {
        "add" => {
            item::add_item(&mut save, order(), ch, cat, id, num);
            0
        }
        "del" => {
            item::del_item(&mut save, ch, cat, id, num);
            0
        }
        "num" => item::get_item_num(&save, ch, cat, id),
        "slot" => item::get_item_slot(&save, ch),
        "addpl" => {
            item::add_pl_item(&mut save, order(), cat, id, num);
            0
        }
        "delpl" => {
            item::del_pl_item(&mut save, cat, id, num);
            0
        }
        "numpl" => item::get_pl_item_num(&save, cat, id),
        "slotpl" => item::get_pl_item_slot(&save),
        "consume" => item::consume_item_list(&mut save, ch, (cat << 16) | (id & 0xffff)),
        _ => return format!("{{\"error\":\"unknown inventory op {op}\"}}"),
    };
    format!(
        "{{\"ret\":{},\"list\":\"{}\",\"pl\":\"{}\",\"imp\":\"{}\"}}",
        ret,
        hex_out(&save, list_at, 160),
        hex_out(&save, item::SAVE_PL_ITEM_LIST, 4 * item::PL_ITEM_SLOTS),
        hex_out(&save, item::SAVE_IMP_ITEM_LIST, item::IMP_ITEMS)
    )
}
