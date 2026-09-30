//! Answers `tools/test_battle_rs.py`: runs the port's rules on the states it is
//! sent and prints one JSON line per request, in the shapes the harness reads
//! the game's own results in (`battle_probe ISO < requests`). A request is a
//! command and whitespace-separated integers (hex with `0x`); characters are
//! read by [`read_char`], skills by [`read_skill`]. The commands (`damage`,
//! `skilldmg`, `value`, `recovery`, `cure`, `calcreal`, the levels, the
//! conditions, `erosion`, `exp`, `scene`, `patch`/`restore`, `boss`) are the
//! harness's; the other modules answer the other harnesses.

use std::io::{BufRead, Write};

// Requests of the other rule sets, each in its own module: `handle`
// answers the commands it knows and returns None for the rest.
mod boss;
mod enemy_ai;
mod enemy_motion;
mod evparty;
mod fellow;
mod fidchell;
mod frame;
mod gorre;
mod innis;
mod items;
mod kite;
mod kyvia;
mod magus;
mod navi;
mod party_ai;
mod spawn;

use piney_battle::affect;
use piney_battle::chara::{self, AffectFunc, Body, Char, Env, Foe};
use piney_battle::damage::{self, Roll};
use piney_battle::drain;
use piney_battle::event::{Event, Who};
use piney_battle::exp;
use piney_battle::flow;
use piney_battle::param::*;
use piney_battle::rand;
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::skill;
use piney_battle::tables::Tables;
use piney_data::iso::Iso;
use piney_data::save::SaveData;

pub(crate) struct Toks<'a>(pub(crate) std::str::SplitWhitespace<'a>);

impl Toks<'_> {
    pub(crate) fn word(&mut self) -> &str {
        self.0.next().expect("more tokens")
    }
    pub(crate) fn int(&mut self) -> i64 {
        // Wide enough for a u64 RNG state; callers truncate.
        let s = self.word();
        let v: i128 = if let Some(h) = s.strip_prefix("0x") {
            i128::from_str_radix(h, 16).unwrap()
        } else if let Some(h) = s.strip_prefix("-0x") {
            -i128::from_str_radix(h, 16).unwrap()
        } else {
            s.parse().unwrap_or_else(|_| panic!("not a number: {s}"))
        };
        v as i64
    }
    pub(crate) fn i16(&mut self) -> i16 {
        self.int() as i16
    }
    pub(crate) fn i32(&mut self) -> i32 {
        self.int() as i32
    }
    pub(crate) fn u32(&mut self) -> u32 {
        self.int() as u32
    }
    pub(crate) fn elm(&mut self) -> Elm {
        std::array::from_fn(|_| self.i16())
    }
}

/// A character: `P type id level exp pMaxHP pMaxSP elm real tune temp time
/// equip[6] job`, `E|B type id level exp rowMaxHP rowMaxSP rowElm beff[5]
/// maxPP pDefPP mDefPP exdef item[3] real temp time PP PPcount PPrestore`
/// or `O type real`, then `HP SP maxHP maxSP cond[16] speedValue
/// conditionNum noDeath entRoot partyFlag ai anmFlag skillID skillStatus
/// posP[4] width affectFunc affectType affectMask actNum flags fellowFlags
/// enemyFlags sysMsgID armsEffectSW attack cnt cloak targetChar pos[4]`.
pub(crate) fn read_char(t: &mut Toks) -> Char {
    let kind = t.word().to_string();
    let mut base = Base { ty: t.i32(), ..Base::default() };
    let body = match kind.as_str() {
        "P" => {
            base.id = t.i16();
            base.level = t.i16();
            base.exp = t.i16();
            let mut p = SpcParam { base, max_hp: t.i16(), max_sp: t.i16(), ..SpcParam::default() };
            p.elm = t.elm();
            p.real = t.elm();
            p.tune = t.elm();
            p.temp = t.elm();
            p.time = t.elm();
            p.equipment = std::array::from_fn(|_| t.i16());
            p.job = t.i16();
            Body::Spc(Box::new(p))
        }
        "E" | "B" => {
            base.id = t.i16();
            base.level = t.i16();
            base.exp = t.i16();
            let mut row = FoeRow { base, max_hp: t.i16(), max_sp: t.i16(), elm: t.elm(), ..FoeRow::default() };
            row.beff = std::array::from_fn(|_| t.i16());
            row.max_pp = t.i16();
            row.p_def_pp = t.i16();
            row.m_def_pp = t.i16();
            row.exdefense = t.i16();
            row.item = std::array::from_fn(|_| t.i32());
            let mut f = Foe::new(row);
            f.real = t.elm();
            f.temp = t.elm();
            f.time = t.elm();
            f.pp = t.i16();
            f.pp_count = t.i16();
            f.pp_restore = t.i16();
            Body::Foe(Box::new(f))
        }
        _ => Body::Other { base, real: t.elm() },
    };
    let mut c = match body {
        Body::Spc(p) => Char::pc(*p),
        Body::Foe(f) => {
            let mut c = Char::foe(f.row.clone());
            c.body = Body::Foe(f);
            c
        }
        Body::Other { base, real } => Char::other(base, real),
    };
    c.hp = t.i16();
    c.sp = t.i16();
    c.max_hp = t.i16();
    c.max_sp = t.i16();
    c.cond.v = std::array::from_fn(|_| t.i16());
    c.cond.speed_value = t.u32();
    c.condition_num = t.i32();
    c.no_death = t.int() != 0;
    c.ent_root = t.u32();
    c.party_flag = t.i32();
    c.has_ai = t.int() != 0;
    c.anm_flag = t.i16();
    c.skill_id = t.i16();
    c.skill_status = t.i16();
    c.pos_p = std::array::from_fn(|_| t.u32());
    c.base_mut().width = t.u32();
    c.affect.func = match t.int() {
        1 => AffectFunc::Enemy,
        2 => AffectFunc::Fellow,
        3 => AffectFunc::Player,
        _ => AffectFunc::None,
    };
    c.affect.ty = t.i16();
    c.affect.mask = t.i32();
    let s = &mut c.spc_char;
    s.act_num = t.i16();
    s.flags = t.u32();
    s.fellow_flags = t.int() as u8;
    s.enemy_flags = t.int() as u16;
    s.sys_msg_id = t.i16();
    s.arms_effect_sw = t.i32();
    s.attack = t.i16();
    s.cnt = t.i32();
    s.cloak = t.u32();
    c.target_char = usize::try_from(t.int()).ok();
    c.pos = std::array::from_fn(|_| t.u32());
    c
}

/// A skill: `atk hit attr[6] condition cost type dmgRate triggerRange
/// targetRange targetType`.
pub(crate) fn read_skill(t: &mut Toks) -> SkillParam {
    SkillParam {
        atk: t.i16(),
        hit: t.i16(),
        attr: std::array::from_fn(|_| t.i16()),
        condition: t.i32(),
        cost: t.i32(),
        ty: t.i32(),
        dmg_rate: t.i16(),
        trigger_range: t.u32(),
        target_range: t.u32(),
        target_type: t.i32(),
        ..SkillParam::default()
    }
}

pub(crate) fn read_env(t: &mut Toks) -> Env {
    Env {
        plcol: t.int() as u8,
        menu_forbid: t.i16(),
        in_battle: t.i32(),
        count: t.u32(),
        menu_type: t.i32(),
        sp_regene_speed: t.int() != 0,
        area: t.i32(),
    }
}

pub(crate) fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    let v: Vec<String> = v.into_iter().map(|x| x.to_string()).collect();
    format!("[{}]", v.join(","))
}

/// A character's state as the harness reads the game's.
pub(crate) fn state(c: &Char) -> String {
    let mut s = format!(
        "{{\"cond\":{},\"speedValue\":{},\"conditionNum\":{},\"HP/SP\":{}",
        list(c.cond.v),
        c.cond.speed_value,
        c.condition_num,
        list([c.hp, c.sp, c.max_hp, c.max_sp])
    );
    match &c.body {
        Body::Spc(p) => {
            s += &format!(
                ",\"level_exp\":{},\"pmax\":{},\"elm\":{},\"real\":{},\"tune\":{},\"temp\":{},\"time\":{}",
                list([p.base.level, p.base.exp]),
                list([p.max_hp, p.max_sp]),
                list(p.elm),
                list(p.real),
                list(p.tune),
                list(p.temp),
                list(p.time)
            );
        }
        Body::Foe(f) => {
            s += &format!(
                ",\"real\":{},\"temp\":{},\"time\":{},\"PP\":{}",
                list(f.real),
                list(f.temp),
                list(f.time),
                list([f.pp, f.pp_count, f.pp_restore])
            );
        }
        Body::Other { real, .. } => {
            // The harness reads an object's personality as a foe's.
            s += &format!(
                ",\"real\":{},\"temp\":{},\"time\":{},\"PP\":[0,0,0]",
                list(*real),
                list([0; 16]),
                list([0; 16])
            );
        }
    }
    s + "}"
}

/// Names for the events' characters, as the harness's hooks record them.
pub(crate) struct Names<'a> {
    pub(crate) me: &'a str,
    pub(crate) target: &'a str,
    pub(crate) chars: Vec<String>,
    /// Whether the acting character has an AI (its `ai` pointer).
    pub(crate) ai: bool,
}

impl Names<'_> {
    pub(crate) fn who(&self, w: Who) -> String {
        match w {
            Who::Me => format!("\"{}\"", self.me),
            Who::Target => format!("\"{}\"", self.target),
            Who::Char(i) => format!("\"{}\"", self.chars.get(i).cloned().unwrap_or_else(|| i.to_string())),
            Who::Nobody => "0".to_string(),
        }
    }
    pub(crate) fn ai(&self) -> String {
        if self.ai { "\"ai\"".into() } else { "0".into() }
    }
}

pub(crate) fn calls(ev: &[Event], n: &Names) -> String {
    let v: Vec<String> = ev
        .iter()
        .map(|e| match *e {
            Event::Affect { on, by, kind, p } => {
                format!("[\"EntryAffect\",{},{},{},{},{},{}]", n.who(on), n.who(by), kind, p[0], p[1], p[2])
            }
            Event::AttributeGuard { on, by } => format!("[\"ccParticleAttributeGuard\",{},{}]", n.who(on), n.who(by)),
            Event::Critical(w) => format!("[\"ccParticleCritical\",{}]", n.who(w)),
            Event::Dying(w) => format!("[\"ccParticleDying\",{}]", n.who(w)),
            Event::NoDamage(w) => format!("[\"ccParticleNoDamage\",{}]", n.who(w)),
            Event::DrainCtrl { from, to, kind, a, b } => {
                format!("[\"effDrainCtrl\",{},{},{},{},{}]", n.who(from), n.who(to), kind, a, b)
            }
            Event::Protect { on, broken, kind } => format!("[\"effProtect\",{},{},{}]", n.who(on), broken, kind),
            Event::SetProtect { state, on } => format!("[\"SetProtect\",{},{}]", state, n.who(on)),
            Event::RecoveryReq { on, amount } => format!("[\"ccEntryRecoveryReq\",{},{}]", n.who(on), amount),
            Event::DispCondition(w) => format!("[\"DispConditionEffect\",{}]", n.who(w)),
            Event::AttributeCriticalParticle(w) => format!("[\"ccParticleAttributeCritical\",{}]", n.who(w)),
            Event::ChatAttributeCritical(_) => format!("[\"ChatMessageAttributeCritical\",{}]", n.ai()),
            Event::ChatAttack { on, dmg, sid, .. } => {
                format!("[\"ChatMessageAttack\",{},{},{},{}]", n.ai(), n.who(on), dmg, sid)
            }
            Event::HealSkill { on, sid } => format!("[\"effHealSkill\",{},{}]", n.who(on), sid),
            Event::Cure(w) => format!("[\"effCure\",{}]", n.who(w)),
            Event::Sanity(w) => format!("[\"effSanity\",{}]", n.who(w)),
            Event::Resurrect(w) => format!("[\"effResurrect\",{}]", n.who(w)),
            Event::FlyFont { on, kind, value } => format!("[\"ccEntryFlyFontNew\",{},{},{}]", kind, value, n.who(on)),
            Event::HitMark { on, by } => format!("[\"ccHitMarkDisp\",{},{}]", n.who(on), n.who(by)),
            Event::DamageActuate { value, .. } => format!("[\"DamageActuate\",{value}]"),
            Event::PanelBure { slot, n: k } => format!("[\"SetPanelBure\",{slot},{k}]"),
            Event::CancelAttack(w) => format!("[\"ccSkillRequest\",{},0,0]", n.who(w)),
            Event::HitEnable(w) => format!("[\"HitEnable\",{}]", n.who(w)),
            Event::HitDisable(w) => format!("[\"HitDisable\",{}]", n.who(w)),
            Event::ClearConditionEffect(w) => format!("[\"ClearConditionEffect\",{}]", n.who(w)),
            Event::ChatDamage { value, .. } => format!("[\"ChatMessageDamage\",{},{}]", n.ai(), value),
            Event::ChatResurrectPlz(_) => format!("[\"ChatMessageResurrectPlz\",{}]", n.ai()),
            Event::NoteHit { value, ai, .. } => {
                format!("[\"NoteHit\",{},{}]", if ai { n.ai() } else { "0".into() }, value)
            }
            Event::AffectMessages { kind, by, n: k, .. } => {
                format!("[\"AffectMessages\",{},{},{},{}]", n.ai(), kind, n.who(by), k)
            }
            Event::Greeting { by, n: k, .. } => format!("[\"Greeting\",{},{},{}]", n.ai(), n.who(by), k),
            Event::SysMsgDown { on, id } => format!("[\"ccAISysMsgSendP\",65548,-1,{},65535,{}]", id as u16, n.who(on)),
            Event::SysMsgUp { id, .. } => format!("[\"ccAISysMsgDeleteDelay\",65549,{},{}]", id as u16, id as u16),
            Event::AfterDrain(w) => format!("[\"effAfterDrain\",{},0]", n.who(w)),
            Event::EnemyRetarget(w) => format!("[\"selectTarget\",{}]", n.who(w)),
            Event::ResistantShield { on, magic } => format!("[\"effResistantShield\",{},{},-1]", n.who(on), magic),
            Event::TalkOff(_) => "[\"TalkOff\"]".to_string(),
        })
        .collect();
    format!("[{}]", v.join(","))
}

pub(crate) fn pair_scene(a: Char, b: Char) -> Scene {
    let mut s = Scene::default();
    s.add(a, 0);
    s.add(b, 0);
    s
}

/// The disc's volume, for the rules that differ by it.
static VOLUME: std::sync::OnceLock<piney_data::volume::Volume> = std::sync::OnceLock::new();

pub(crate) fn probe_volume() -> piney_data::volume::Volume {
    VOLUME.get().copied().unwrap_or_default()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path)?;
    let _ = VOLUME.set(iso.volume()?);
    let mut tables = Tables::read(&mut iso)?;
    let mut saved: std::collections::HashMap<usize, SkillParam> = std::collections::HashMap::new();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let line = line?;
        let mut t = Toks(line.split_whitespace());
        let Some(cmd) = t.0.next() else { continue };
        let reply = match cmd {
            "damage" => {
                let att = read_char(&mut t);
                let mut tgt = read_char(&mut t);
                let sk = read_skill(&mut t);
                let mag = t.u32();
                let h = t.i32();
                let env = read_env(&mut t);
                let mut rng = Rand(t.int() as u64);
                let listed = t.int() != 0;
                let mut ev = Vec::new();
                let r = damage::calc_battle_damage(
                    &tables,
                    &att,
                    &mut tgt,
                    &sk,
                    mag,
                    Roll::from_h(h),
                    listed,
                    &mut rng,
                    &env,
                    &mut ev,
                );
                let n = Names { me: "att", target: "t", chars: vec![], ai: att.has_ai };
                format!("{{\"ret\":{},\"rand\":{},\"calls\":{},\"tgt\":{}}}", r.dmg, rng.0, calls(&ev, &n), state(&tgt))
            }
            "skilldmg" => {
                let att = read_char(&mut t);
                let tgt = read_char(&mut t);
                let sk = read_skill(&mut t);
                let sid = t.i32();
                let mut ac = t.i16();
                let env = read_env(&mut t);
                let mut rng = Rand(t.int() as u64);
                let ai = att.has_ai;
                let mut s = pair_scene(att, tgt);
                let mut ev = Vec::new();
                let n = damage::skill_damage(&tables, &mut s, 0, 1, &sk, &mut ac, sid, &mut rng, &env, &mut ev);
                let names = Names { me: "att", target: "t", chars: vec!["att".into(), "t".into()], ai };
                format!(
                    "{{\"ret\":{},\"ac\":{},\"rand\":{},\"calls\":{},\"tgt\":{}}}",
                    n,
                    ac,
                    rng.0,
                    calls(&ev, &names),
                    state(&s.chars[1])
                )
            }
            "value" => {
                let cp = read_char(&mut t);
                let tp = read_char(&mut t);
                let sid = t.i32();
                let ty = t.i32();
                let keep = tables.skills[sid as usize].ty;
                if ty != -1 {
                    tables.skills[sid as usize].ty = ty;
                }
                let v = damage::skill_damage_value(&tables, &cp, &tp, sid, true, true, &Env::default());
                tables.skills[sid as usize].ty = keep;
                format!("[{},{},{}]", v.dmg, u8::from(v.critical), u8::from(v.guard))
            }
            "recovery" => {
                let cr = read_char(&mut t);
                let tg = read_char(&mut t);
                let sid = t.i32();
                let param = t.i32();
                // As the harness lays it out: only the creator is listed.
                let mut s = pair_scene(cr, tg);
                s.pc_list = vec![0];
                let mut ev = Vec::new();
                let n = skill::recovery(&tables, &s, 0, 1, sid, param, &mut ev);
                let names = Names { me: "creator", target: "t", chars: vec!["creator".into(), "t".into()], ai: false };
                format!("{{\"ret\":{},\"calls\":{}}}", n, calls(&ev, &names))
            }
            "cure" => {
                let mut cr = read_char(&mut t);
                let mut tg = read_char(&mut t);
                let sid = t.i32();
                let count = t.i16();
                let stype = t.i32();
                let ann = t.int() != 0;
                cr.anm_flag = t.i16();
                cr.skill_id = 55;
                cr.skill_status = 3;
                let mut ev = Vec::new();
                let r = skill::recovery_system(&tables, Some(&mut cr), &mut tg, sid, count, stype, ann, true, &mut ev);
                let names = Names { me: "creator", target: "t", chars: vec![], ai: false };
                let st = state(&tg);
                format!(
                    "{},\"calls\":{},\"end\":{},\"caster\":{}}}",
                    &st[..st.len() - 1],
                    calls(&ev, &names),
                    u8::from(r.end),
                    list([cr.hp, cr.sp, cr.skill_id, cr.skill_status])
                )
            }
            "calcreal" => {
                let mut ch = read_char(&mut t);
                let flag = t.i32();
                let env = read_env(&mut t);
                let mut ev = Vec::new();
                chara::calc_real(&tables, &mut ch, flag, &env, &mut ev);
                let names = Names { me: "self", target: "t", chars: vec![], ai: false };
                let st = state(&ch);
                format!("{},\"calls\":{}}}", &st[..st.len() - 1], calls(&ev, &names))
            }
            "dispcond" => {
                let mut ch = read_char(&mut t);
                let ep = t.i32();
                let (shown, eye) = (t.i32() != 0, t.i32() != 0);
                let ep = (ep != -2).then_some(ep);
                let act = chara::disp_condition_effect(&mut ch, ep, shown, eye, crate::probe_volume());
                let num = ch.condition_num;
                let mut calls = Vec::new();
                let left = match act {
                    chara::CondFx::Keep => {
                        if ep.is_some() {
                            "\"old\"".to_string()
                        } else {
                            "0".to_string()
                        }
                    }
                    chara::CondFx::Kill => {
                        calls.push("[\"kill\",\"old\"]".to_string());
                        "0".to_string()
                    }
                    chara::CondFx::Delete => {
                        calls.push("[\"delete\",\"old\"]".to_string());
                        "0".to_string()
                    }
                    chara::CondFx::Set => {
                        calls.push(format!("[\"set\",{num}]"));
                        "\"new\"".to_string()
                    }
                    chara::CondFx::Replace => {
                        calls.push("[\"kill\",\"old\"]".to_string());
                        calls.push(format!("[\"set\",{num}]"));
                        "\"new\"".to_string()
                    }
                };
                format!("{{\"num\":{num},\"ep\":{left},\"calls\":[{}]}}", calls.join(","))
            }
            "levelup" | "leveldown" => {
                let mut ch = read_char(&mut t);
                let ret = if cmd == "levelup" {
                    chara::check_level_up(&tables, &mut ch)
                } else {
                    chara::level_down(&tables, &mut ch);
                    0
                };
                let st = state(&ch);
                format!("{},\"ret\":{}}}", &st[..st.len() - 1], ret)
            }
            "setlevel" => {
                let ch = read_char(&mut t);
                let lvs = t.i32();
                let mut p = *ch.spc().unwrap();
                chara::set_level_param(&tables, &mut p, lvs);
                format!(
                    "{{\"level\":{},\"pmax\":{},\"elm\":{}}}",
                    p.base.level,
                    list([p.max_hp, p.max_sp]),
                    list(p.elm)
                )
            }
            "modcond" => {
                let _creator = read_char(&mut t);
                let mut tg = read_char(&mut t);
                let sid = t.i32();
                let mut ev = Vec::new();
                skill::modify_condition(&tables, &mut tg, sid, &mut ev);
                let names = Names { me: "creator", target: "t", chars: vec![], ai: false };
                let st = state(&tg);
                format!("{},\"calls\":{}}}", &st[..st.len() - 1], calls(&ev, &names))
            }
            "condsucc" => {
                let tg = read_char(&mut t);
                let sid = t.i32();
                let mut rng = Rand(t.int() as u64);
                let ok = skill::condition_success(&tg, sid, true, &mut rng);
                format!("{{\"ret\":{},\"rand\":{}}}", u8::from(ok), rng.0)
            }
            "targetcond" => {
                let ch = read_char(&mut t);
                let sid = t.i32();
                skill::target_condition_by_skill(&ch, sid).to_string()
            }
            "erosion" => {
                let e = t.i16();
                let n = t.i32();
                let f = t.u32();
                exp::add_lv_erosion(&tables, e, n, f).to_string()
            }
            "patch" => {
                let sid = t.int() as usize;
                let sk = read_skill(&mut t);
                let row = &mut tables.skills[sid];
                saved.entry(sid).or_insert_with(|| row.clone());
                row.atk = sk.atk;
                row.hit = sk.hit;
                row.attr = sk.attr;
                row.condition = sk.condition;
                row.cost = sk.cost;
                row.ty = sk.ty;
                row.dmg_rate = sk.dmg_rate;
                row.trigger_range = sk.trigger_range;
                row.target_range = sk.target_range;
                row.target_type = sk.target_type;
                "0".into()
            }
            "restore" => {
                let sid = t.int() as usize;
                if let Some(row) = saved.remove(&sid) {
                    tables.skills[sid] = row;
                }
                "0".into()
            }
            "exp" => exp_request(&tables, &mut t),
            "levelabsent" => {
                let area = t.i32();
                let flags = t.u32();
                let exp_flags = t.u32();
                let mut party = exp::Party::default();
                for k in 0..3 {
                    party.ids[k] = t.i32();
                }
                let mut save = SaveData::new();
                save.set_i32(exp::SAVE_PARTY_MEMBER_FLAG, flags as i32);
                save.set_i32(exp::SAVE_PARTY_MEMBER_EXP, exp_flags as i32);
                let n = t.int();
                let mut ids = Vec::new();
                for _ in 0..n {
                    let id = t.i32();
                    let c = read_char(&mut t);
                    c.spc().unwrap().store(&mut save, id as usize);
                    ids.push(id);
                }
                exp::level_up_absent(&tables, &mut save, &party, area);
                let v: Vec<String> = ids
                    .iter()
                    .map(|&id| {
                        let p = SpcParam::from_save(&save, id as usize);
                        format!(
                            "\"{id}\":{}",
                            list([p.base.level, p.base.exp, p.max_hp, p.max_sp].into_iter().chain(p.elm))
                        )
                    })
                    .collect();
                format!("{{{}}}", v.join(","))
            }
            "evolution" => {
                let mut save = SaveData::new();
                save.set_i16(drain::SAVE_DRAIN_COUNT, t.i16());
                save.set_i16(drain::SAVE_DRAIN_EVOLUTION, t.i16());
                save.set_u8(drain::SAVE_PARODY, t.int() as u8);
                let volume = t.i32();
                let (stage, item) = drain::evolution(&mut save, volume);
                format!(
                    "{{\"count\":{},\"evo\":{},\"stage\":{},\"item\":{}}}",
                    save.i16(drain::SAVE_DRAIN_COUNT),
                    save.i16(drain::SAVE_DRAIN_EVOLUTION),
                    stage.unwrap_or(0),
                    item.unwrap_or(-1)
                )
            }
            "areaitem" => {
                let item = t.i32();
                let kind = t.i32();
                let area = exp::AreaInfo { server: t.i32(), field_attr: t.i32(), floor: t.i32(), base: t.i32() };
                let mut rng = Rand(t.int() as u64);
                let v = exp::area_item(&tables, item, kind, &area, &mut rng);
                format!("{{\"ret\":{},\"rand\":{}}}", v, rng.0)
            }
            "scene" => scene_request(&tables, &mut t),
            _ => {
                match enemy_ai::handle(cmd, &mut t, &mut tables)
                    .or_else(|| party_ai::handle(cmd, &mut t, &mut tables))
                    .or_else(|| items::handle(cmd, &mut t, &mut tables))
                    .or_else(|| frame::handle(cmd, &mut t, &mut tables))
                    .or_else(|| kite::handle(cmd, &mut t, &mut tables))
                    .or_else(|| spawn::handle(cmd, &mut t, &mut tables))
                    .or_else(|| navi::handle(cmd, &mut t, &mut tables))
                    .or_else(|| fellow::handle(cmd, &mut t, &mut tables))
                    .or_else(|| enemy_motion::handle(cmd, &mut t, &mut tables))
                    .or_else(|| evparty::handle(cmd, &mut t, &mut tables))
                    .or_else(|| {
                        if cmd.starts_with("boss") { boss::handle(cmd, &mut t, &mut tables, &iso_path) } else { None }
                    })
                    .or_else(|| innis::handle(cmd, &mut t, &mut tables, &iso_path))
                    .or_else(|| kyvia::handle(cmd, &mut t, &mut tables, &iso_path))
                    .or_else(|| magus::handle(cmd, &mut t, &mut tables, &iso_path))
                    .or_else(|| fidchell::handle(cmd, &mut t, &mut tables, &iso_path))
                    .or_else(|| gorre::handle(cmd, &mut t, &mut tables, &iso_path))
                {
                    Some(r) => r,
                    None => format!("{{\"error\":\"unknown command {cmd}\"}}"),
                }
            }
        };
        writeln!(out, "{reply}")?;
        out.flush()?;
    }
    Ok(())
}

/// `exp LEVEL ERO COUNT FLAGS EXPFLAGS NPARTY (ID CHAR)... NSAVED (ID
/// CHAR)...`: `ccExpDistributor` over up to three party characters and
/// the saved records of others. Prints every exp and the infection.
fn exp_request(tables: &Tables, t: &mut Toks) -> String {
    let level = t.i16();
    let erosion = t.i16();
    let count = t.u32();
    let flags = t.u32();
    let exp_flags = t.u32();
    let mut save = SaveData::new();
    save.set_i16(exp::SAVE_EROSION, erosion);
    save.set_i32(exp::SAVE_PARTY_MEMBER_FLAG, flags as i32);
    save.set_i32(exp::SAVE_PARTY_MEMBER_EXP, exp_flags as i32);
    let mut scene = Scene::default();
    let mut party = exp::Party::default();
    let np = t.int();
    let mut ids = Vec::new();
    for n in 0..np as usize {
        let id = t.i32();
        let c = read_char(t);
        let i = scene.add(c, 0);
        party.members[n] = Some(i);
        party.ids[n] = id;
        party.num += 1;
        ids.push((id, Some(i)));
    }
    let ns = t.int();
    for _ in 0..ns {
        let id = t.i32();
        let c = read_char(t);
        c.spc().unwrap().store(&mut save, id as usize);
        ids.push((id, None));
    }
    exp::exp_distributor(tables, &mut scene, &party, &mut save, level, count);
    let v: Vec<String> = ids
        .iter()
        .map(|&(id, i)| {
            let e = match i {
                Some(i) => scene.chars[i].spc().unwrap().base.exp,
                None => SpcParam::from_save(&save, id as usize).base.exp,
            };
            format!("\"{id}\":{e}")
        })
        .collect();
    format!("{{\"exp\":{{{}}},\"erosion\":{}}}", v.join(","), save.i16(exp::SAVE_EROSION))
}

/// `scene N CHAR... PCLIST ENELIST` (each list `K i...`) then an area command:
/// `skilldmg`, `skilldmgat`, `skilldmg2`, `recovery`, `recoveryat`, `hold`,
/// `holdat`, `modify`, `modifyat` or `request`, with the arguments
/// `tools/test_battle_rs.py` sends. Characters are named `c0`, `c1`, ... in the
/// events. Prints the return, the RNG, the events and every character's state.
fn scene_request(tables: &Tables, t: &mut Toks) -> String {
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
    let op = t.word().to_string();
    let me = t.int() as usize;
    let names =
        Names { me: "c?", target: "c?", chars: (0..n).map(|i| format!("c{i}")).collect(), ai: s.chars[me].has_ai };
    let pos4 = |t: &mut Toks| -> [u32; 4] { std::array::from_fn(|_| t.u32()) };
    let mut ev = Vec::new();
    let mut rng = Rand(0);
    let mut extra = String::new();
    let ret = match op.as_str() {
        "skilldmg" => {
            let target = t.int() as usize;
            let sk = read_skill(t);
            let sid = t.i32();
            let mut ac = t.i16();
            let env = read_env(t);
            rng = Rand(t.int() as u64);
            let r = damage::skill_damage(tables, &mut s, me, target, &sk, &mut ac, sid, &mut rng, &env, &mut ev);
            extra = format!(",\"ac\":{ac}");
            r
        }
        "skilldmgat" => {
            let pos = pos4(t);
            let ttype = t.i32();
            let sk = read_skill(t);
            let sid = t.i32();
            let env = read_env(t);
            rng = Rand(t.int() as u64);
            damage::skill_damage_at(tables, &mut s, me, pos, ttype, &sk, sid, &mut rng, &env, &mut ev)
        }
        "skilldmg2" => {
            let target = t.int() as usize;
            let pos = pos4(t);
            let ttype = t.i32();
            let sk = read_skill(t);
            let sid = t.i32();
            let mut ac = t.i16();
            let env = read_env(t);
            rng = Rand(t.int() as u64);
            let r = damage::skill_damage2(
                tables, &mut s, me, target, pos, ttype, &sk, &mut ac, sid, &mut rng, &env, &mut ev,
            );
            extra = format!(",\"ac\":{ac}");
            r
        }
        "recovery" => {
            let target = t.int() as usize;
            let sid = t.i32();
            let param = t.i32();
            skill::recovery(tables, &s, me, target, sid, param, &mut ev)
        }
        "recoveryat" => {
            let pos = pos4(t);
            let sid = t.i32();
            let param = t.i32();
            skill::recovery_at(tables, &s, me, pos, sid, param, &mut ev)
        }
        "hold" => {
            let target = t.int() as usize;
            let sk = read_skill(t);
            skill::hold(tables.volume, &s, me, target, &sk, &mut ev)
        }
        "holdat" => {
            let pos = pos4(t);
            let ttype = t.i32();
            let sk = read_skill(t);
            skill::hold_at(tables.volume, &s, me, pos, ttype, &sk, &mut ev)
        }
        "modify" | "modifyat" => {
            let (target, pos, ttype) = if op == "modify" {
                (Some(t.int() as usize), [0; 4], 0)
            } else {
                let p = pos4(t);
                (None, p, t.i32())
            };
            let sid = t.i32();
            let stype = t.i32();
            let force = t.int() != 0;
            rng = Rand(t.int() as u64);
            let mut m = skill::ModifyEvents::default();
            let r = match target {
                Some(target) => {
                    skill::skill_modify_condition(tables, &mut s, me, target, sid, stype, force, &mut rng, &mut m)
                }
                None => skill::skill_modify_condition_at(
                    tables, &mut s, me, pos, ttype, sid, stype, force, &mut rng, &mut m,
                ),
            };
            ev = m.events;
            extra = format!(",\"started\":{},\"resisted\":{}", list(m.started), list(m.resisted));
            r
        }
        "request" => {
            let target = t.int() as usize;
            let sid = t.i32();
            let stype = t.i32();
            let running = t.int() != 0;
            rng = Rand(t.int() as u64);
            let mut skills = flow::Skills::default();
            if running {
                let mut r = flow::SkillRun::new(1);
                r.creator = Some(me);
                skills.runs.push(r);
            }
            match skills.request(tables, &mut s, me, Some(target), sid, stype as i8, &mut rng) {
                (Some(i), Some(r)) => {
                    let run = &skills.runs[i];
                    extra =
                        format!(
                            ",\"ac\":{},\"modify\":{},\"hold\":{},\"words\":{},\"start\":{},\"new\":{},\"old\":{}",
                            r.ac_flag,
                            u8::from(r.modify),
                            u8::from(r.hold),
                            if r.words { format!("[[{sid},{me}]]") } else { "[]".to_string() },
                            list(r.skill_start.iter().map(|&(w, a, b)| {
                                format!("[{},{},{}]", if w == Who::Me { me } else { target }, a, b)
                            })),
                            list(
                                [
                                    i64::from(run.id),
                                    i64::from(run.stype),
                                    i64::from(run.target_type),
                                    i64::from(run.ty),
                                    i64::from(run.height)
                                ]
                                .into_iter()
                                .chain(run.pos.iter().map(|&v| i64::from(v)))
                                .chain(run.target_pos.iter().map(|&v| i64::from(v)))
                            ),
                            skills.runs.first().filter(|_| running).map_or(0, |r| r.status)
                        );
                    1
                }
                _ => 0,
            }
        }
        "skillmain" | "noteaffect" => {
            let mut run = read_run(t);
            let env = read_env(t);
            let (anim_done, annihilated, kind) =
                if op == "skillmain" { (t.int() != 0, t.int() != 0, 0) } else { (false, false, t.u32()) };
            rng = Rand(t.int() as u64);
            let id = |p: [u32; 4]| p;
            let f = flow::Frame { env: &env, w2p: &id, anim_done, notes: &[], annihilated };
            let mut out = flow::MainOut::default();
            let r = if op == "skillmain" {
                i32::from(run.main(tables, &mut s, &f, &mut rng, &mut ev, &mut out))
            } else {
                run.note_event_affect(tables, &mut s, &f, kind, &mut rng, &mut ev);
                0
            };
            extra = format!(
                ",\"run\":{},\"started\":{},\"resisted\":{},\"spell\":{},\"heal\":{}",
                run_state(&run),
                list(out.modify.started.iter()),
                list(out.modify.resisted.iter()),
                out.spell.map_or(-1, |s| s as i32),
                u8::from(out.heal_sound)
            );
            r
        }
        "skillcheck" => {
            let n = t.int() as usize;
            let mut sk = flow::Skills::default();
            for _ in 0..n {
                let mut r = read_run(t);
                r.creator = Some(me);
                sk.runs.push(r);
            }
            let act = t.i16();
            sk.check(tables, &s, me, act)
        }
        "drain" => {
            let target = t.int() as usize;
            let sid = t.i32();
            let mut save = SaveData::new();
            save.set_i16(exp::SAVE_EROSION, t.i16());
            rng = Rand(t.int() as u64);
            // The fade's noise draws once before the drain.
            let _ = rand::Rng::rand(&mut rng);
            drain::drain_start(&s, me, target, &mut ev);
            let d = drain::drain(tables, &s, &mut save, me, target, sid, &mut rng);
            let side = drain::side_effect_happens(&save, &mut rng);
            extra = format!(
                ",\"erosion\":{},\"drops\":{},\"count\":{},\"remove\":{},\"side\":{}",
                d.erosion,
                list(d.drops),
                d.count,
                u8::from(d.remove_target),
                u8::from(side)
            );
            0
        }
        "drainfx" => {
            let mut party = exp::Party::default();
            for k in 0..3 {
                party.members[k] = usize::try_from(t.int()).ok();
            }
            let mut save = SaveData::new();
            save.set_i16(exp::SAVE_EROSION, t.i16());
            for i in 0..40 {
                let a = drain::SAVE_ITEM_LIST + 4 * i;
                save.set_i16(a, t.i16());
                save.set_u8(a + 2, t.int() as u8);
                save.set_u8(a + 3, t.int() as u8);
            }
            rng = Rand(t.int() as u64);
            let fx = drain::side_effect(tables, &mut s, &party, me, &mut save, &mut rng);
            // Each start as [kind, who, value]: 0 effHeal, 1 the MISS, 2
            // effSkillStartEffect, 3 the exp's number, 4 effAfterDrain.
            let starts = fx.starts.iter().flat_map(|st| match *st {
                drain::SideStart::Heal(c) => [0, c as i32, 0],
                drain::SideStart::Miss(c) => [1, c as i32, 0],
                drain::SideStart::Effect(c) => [2, c as i32, 0],
                drain::SideStart::Exp { who, value } => [3, who as i32, value],
                drain::SideStart::AfterDrain(c) => [4, c as i32, 0],
            });
            extra = format!(
                ",\"fx\":{},\"items\":{},\"missed\":{},\"shown\":{},\"healed\":{},\"starts\":{}",
                list([fx.id, i32::from(fx.level_down), fx.lost, fx.exp_lost]),
                list((0..40).flat_map(|i| {
                    let (id, cat, num) = drain::item_slot(&save, i);
                    [i32::from(id), i32::from(cat), i32::from(num)]
                })),
                list(fx.missed().iter()),
                list(fx.shown().iter()),
                list(fx.healed().iter()),
                list(starts)
            );
            0
        }
        "condadj" => {
            s.chars[me].spc_char.act_num_old = t.i16();
            let mut e = Vec::new();
            affect::condition_adjustment(&mut s.chars[me], Who::Char(me), &mut e);
            ev = e;
            let sc = &s.chars[me].spc_char;
            extra = format!(
                ",\"spc\":{}",
                list([
                    i64::from(sc.act_num),
                    i64::from(sc.act_num_old),
                    i64::from(sc.flags),
                    i64::from(sc.cnt),
                    i64::from(sc.cloak)
                ])
            );
            0
        }
        "playernote" | "fellownote" => {
            let (mut last, arms, dist_tg) = if op == "playernote" { (t.i32(), 0, 0) } else { (0, t.u32(), t.u32()) };
            s.chars[me].target_char = usize::try_from(t.int()).ok();
            let env = read_env(t);
            rng = Rand(t.int() as u64);
            if op == "playernote" {
                flow::player_attack_note(tables, &mut s, me, &mut last, &mut rng, &env, &mut ev);
                extra = format!(",\"dist\":{last}");
            } else {
                flow::fellow_attack_note(tables, &mut s, me, arms, dist_tg, &mut rng, &env, &mut ev);
            }
            0
        }
        "affect" => {
            let by = usize::try_from(t.int()).ok();
            let kind = t.i16();
            let p = [t.i16(), t.i16(), t.i16()];
            let running = t.i32();
            let menu = t.int() != 0;
            let mut party = exp::Party::default();
            for k in 0..3 {
                party.ids[k] = t.i32();
            }
            rng = Rand(t.int() as u64);
            let check = move |_: usize| running;
            let ctx = affect::AffectCtx {
                party: &party,
                menu,
                skill_check: &check,
                boss: None,
                volume: crate::probe_volume(),
            };
            affect::entry_affect(tables, &mut s, &ctx, me, by, kind, p, &mut rng, &mut ev);
            let c = &s.chars[me];
            let a = &c.affect;
            let sc = &c.spc_char;
            extra = format!(
                ",\"affect\":{},\"spc\":{},\"target\":{}",
                list([
                    i64::from(a.ty),
                    i64::from(a.param[0]),
                    i64::from(a.param[1]),
                    i64::from(a.param[2]),
                    a.person.map_or(-1, |i| i as i64),
                    i64::from(a.color_cnt),
                    i64::from(a.color_rate),
                    i64::from(a.color),
                    i64::from(a.cond_color_cnt),
                    i64::from(a.cond_color_rate),
                    i64::from(a.cond_color)
                ]),
                list([
                    i64::from(sc.act_num),
                    i64::from(sc.flags),
                    i64::from(sc.fellow_flags),
                    i64::from(sc.enemy_flags),
                    i64::from(sc.arms_effect_sw),
                    i64::from(sc.attack),
                    i64::from(sc.cnt),
                    i64::from(sc.cloak)
                ]),
                c.target_char.map_or(-1, |i| i as i64)
            );
            0
        }
        _ => return format!("{{\"error\":\"unknown scene op {op}\"}}"),
    };
    format!(
        "{{\"ret\":{},\"rand\":{},\"calls\":{},\"chars\":{}{}}}",
        ret,
        rng.0,
        calls(&ev, &names),
        list(s.chars.iter().map(state)),
        extra
    )
}

/// A `ccSkill`: `id stype status modify report hold force level count type
/// targetType param pos[4] targetPos[4] acFlag creator target hasAnm`
/// (creator and target -1 for none).
fn read_run(t: &mut Toks) -> flow::SkillRun {
    let mut r = flow::SkillRun::new(t.i32());
    r.stype = t.int() as i8;
    r.status = t.int() as i8;
    r.modify = t.int() != 0;
    r.report = t.int() != 0;
    r.hold = t.int() != 0;
    r.force = t.int() != 0;
    r.level = t.i32();
    r.count = t.i16();
    r.ty = t.i32();
    r.target_type = t.i32();
    r.param = t.i32();
    r.pos = std::array::from_fn(|_| t.u32());
    r.target_pos = std::array::from_fn(|_| t.u32());
    r.ac_flag = t.i16();
    r.creator = usize::try_from(t.int()).ok();
    r.target = usize::try_from(t.int()).ok();
    r.has_anm = t.int() != 0;
    r
}

/// The `ccSkill` fields the harness reads back.
fn run_state(r: &flow::SkillRun) -> String {
    list([
        i64::from(r.status),
        i64::from(r.modify),
        i64::from(r.hold),
        i64::from(r.force),
        i64::from(r.count),
        i64::from(r.ac_flag),
        r.creator.map_or(-1, |i| i as i64),
        r.target.map_or(-1, |i| i as i64),
        i64::from(r.pos[0]),
        i64::from(r.pos[1]),
        i64::from(r.pos[2]),
        i64::from(r.pos[3]),
        i64::from(r.target_pos[0]),
        i64::from(r.target_pos[1]),
        i64::from(r.target_pos[2]),
        i64::from(r.target_pos[3]),
    ])
}
