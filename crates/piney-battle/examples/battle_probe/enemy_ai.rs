//! `battle_probe` requests for the enemies' AI (`piney_battle::enemy_ai`), as
//! `tools/test_battle_enemy_ai_rs.py` sends them: `escene` (a scene, each foe's
//! `ccEnemy` state by [`read_enemy`], the globals, generators, environment and
//! enemy book, then one operation: `think`, `interrupt`, `begin`, `routine`,
//! `check`, `crisis`, `setact`, `attack`, `skills`, `target`, `target0`,
//! `interval`, `start`, `affect`, `clearcond`, `skilltarget`), `eskills`,
//! `einit`, `genrand`, `drainid`, `race`, and `epatch`/`erestore`,
//! `eprep`/`eunprep` (table rows changed and put back).

use std::cell::RefCell;
use std::collections::HashMap;

use piney_battle::chara::Char;
use piney_battle::enemy_ai::{
    self, Ai, Begin, Enemy, EnemySkill, EntryParam, Genrand, IDENTITY, Out, SkillRef, SkillUse, World,
};
use piney_battle::param::{EnemyTable, SkillParam};
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_data::save::SaveData;

use crate::{Names, Toks, calls, list, read_char, read_env, read_skill, state};

thread_local! {
    static SAVED: RefCell<HashMap<usize, EnemyTable>> = RefCell::new(HashMap::new());
    static SAVED_SKILLS: RefCell<HashMap<usize, SkillParam>> = RefCell::new(HashMap::new());
}

/// Answers a command this module knows; None for any other.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    Some(match cmd {
        "escene" => scene_request(tables, t),
        "eskills" => {
            let id = t.i32();
            let anm = anm6(t);
            let mut e = Enemy { ene_id: id, ..Enemy::default() };
            enemy_ai::clear_skill_list(&mut e);
            enemy_ai::init_skill_list(tables, &mut e, anm);
            format!("{{\"num\":{},\"list\":{}}}", e.skill_num, skill_list_json(&e))
        }
        "einit" => {
            let ent = read_ent(t);
            let anm = anm6(t);
            let dust = t.i16();
            let mut cc = genrand(t);
            let (ch, e) = enemy_ai::init_enemy(tables, &ent, anm, dust, &IDENTITY, &mut cc);
            format!("{{\"char\":{},\"foe\":{},\"cc\":{}}}", char_json(&ch), enemy_json(&e), cc_json(&cc))
        }
        "genrand" => {
            let mut cc = genrand(t);
            let n = t.int() as usize;
            let v: Vec<u32> = (0..n).map(|_| cc.next_u32()).collect();
            format!("{{\"v\":{},\"cc\":{}}}", list(v), cc_json(&cc))
        }
        "drainid" => enemy_ai::drain_id(tables, t.i32()).to_string(),
        "race" => match enemy_ai::enemy_race(tables, t.i32()) {
            Some((r, k)) => format!("[{r},{k}]"),
            None => "null".into(),
        },
        "epatch" => {
            patch_row(tables, t);
            "0".into()
        }
        "eprep" => {
            if t.int() != 0 {
                patch_row(tables, t);
            }
            for _ in 0..t.int() {
                let sid = t.int() as usize;
                let sk = read_skill(t);
                let row = &mut tables.skills[sid];
                SAVED_SKILLS.with(|s| {
                    s.borrow_mut().entry(sid).or_insert_with(|| row.clone());
                });
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
            }
            "0".into()
        }
        "eunprep" => {
            let has_row = t.int() != 0;
            let id = t.int() as usize;
            if has_row {
                restore_row(tables, id);
            }
            for _ in 0..t.int() {
                let sid = t.int() as usize;
                if let Some(row) = SAVED_SKILLS.with(|s| s.borrow_mut().remove(&sid)) {
                    tables.skills[sid] = row;
                }
            }
            "0".into()
        }
        "erestore" => {
            restore_row(tables, t.int() as usize);
            "0".into()
        }
        _ => return None,
    })
}

/// `ID THINK[10] MAG[2] (TYPE TRIGGER COST)x4 ESIZE TYPE GOLD MAXPP`.
fn patch_row(tables: &mut Tables, t: &mut Toks) {
    let id = t.int() as usize;
    let row = &mut tables.enemies[id];
    SAVED.with(|s| {
        s.borrow_mut().entry(id).or_insert_with(|| row.clone());
    });
    let th = &mut row.think;
    th.area = t.u32();
    th.territory = t.u32();
    th.view_range = t.u32();
    th.atk_range_a = t.u32();
    th.atk_range_b = t.u32();
    th.atk_range_c = t.u32();
    th.attack_delay = t.i32();
    th.max_spd = t.u32();
    th.anm_spd = t.u32();
    th.target_type = t.i32();
    row.mag = [t.i32(), t.i32()];
    for k in 0..4 {
        let sk = if k < 2 { &mut row.atc[k] } else { &mut row.ski[k - 2] };
        sk.ty = t.i32();
        sk.trigger_range = t.u32();
        sk.cost = t.i32();
    }
    row.esize = t.u32() as i32;
    row.param.base.ty = t.i32();
    row.param.base.gold = t.i32();
    row.param.max_pp = t.i16();
}

fn restore_row(tables: &mut Tables, id: usize) {
    if let Some(row) = SAVED.with(|s| s.borrow_mut().remove(&id)) {
        tables.enemies[id] = row;
    }
}

fn anm6(t: &mut Toks) -> [bool; 6] {
    std::array::from_fn(|_| t.int() != 0)
}

fn genrand(t: &mut Toks) -> Genrand {
    let mut g = Genrand::seeded(t.u32());
    g.mti = t.i32();
    g
}

/// `mti` and an FNV-1a hash of the 624 words.
fn cc_json(g: &Genrand) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for w in g.mt {
        for b in w.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("[{},{}]", g.mti, h)
}

fn v4(t: &mut Toks) -> [u32; 4] {
    std::array::from_fn(|_| t.u32())
}

fn idx(v: i64) -> Option<usize> {
    if v < 0 { None } else { Some(v as usize) }
}

fn ix(v: Option<usize>) -> i64 {
    v.map_or(-1, |i| i as i64)
}

fn skill_ref(code: i32) -> Option<SkillRef> {
    match code {
        -1 => None,
        1000..=1005 => Some(SkillRef::Row((code - 1000) as u8)),
        c => Some(SkillRef::Table(c)),
    }
}

fn code(r: Option<SkillRef>) -> i32 {
    match r {
        None => -1,
        Some(SkillRef::Row(k)) => 1000 + i32::from(k),
        Some(SkillRef::Table(s)) => s,
    }
}

fn read_ent(t: &mut Toks) -> EntryParam {
    EntryParam {
        pos: v4(t),
        dirc: v4(t),
        ty: t.i32(),
        id: t.i32(),
        area: t.i32(),
        area_num: t.i32(),
        floor: t.i32(),
        block: t.i32(),
        x: t.i32(),
        y: t.i32(),
        ent_root: t.i32(),
        land: t.i32(),
        param: std::array::from_fn(|_| t.i32()),
    }
}

fn ent_json(e: &EntryParam) -> String {
    let mut v: Vec<i64> = e.pos.iter().chain(e.dirc.iter()).map(|&x| i64::from(x)).collect();
    v.extend([e.ty, e.id, e.area, e.area_num, e.floor, e.block, e.x, e.y, e.ent_root, e.land].map(i64::from));
    v.extend(e.param.map(i64::from));
    list(v)
}

/// An enemy's state, in the order the harness sends it: `dirc[4]`, the affect,
/// the entry, the row and race, the acts and counts, the target, the movement,
/// the animation, the life rate and the skill slots (`(flags atkId skiId
/// percentage skiParam skiTarget)x6`). Pointers are scene indices (-1 none),
/// skill pointers codes (-1 none, 1000 + slot for the row's own, else the
/// `skillTbl` id).
pub(crate) fn read_enemy(t: &mut Toks) -> Enemy {
    let mut e = Enemy { dirc: v4(t), affect_type: t.i16(), affect_param0: t.i16(), ..Enemy::default() };
    let f = t.int();
    e.obj_flag = f & 1 != 0;
    e.init_flag = f & 2 != 0;
    e.freeze_flag = f & 4 != 0;
    e.affect_flag = f & 8 != 0;
    e.disp_sw = f & 0x10 != 0;
    e.dest_flag = f & 0x20 != 0;
    e.cmnd_flag = f & 0x40 != 0;
    e.ccs2_flag = f & 0x80 != 0;
    e.fade_flag = t.i16();
    e.fade_cnt = t.i16();
    e.ent = read_ent(t);
    e.ene_id = t.i32();
    e.race = t.i32();
    e.race_id = t.i32();
    let f = t.int();
    e.target_flag = f & 1 != 0;
    e.action_flag = f & 2 != 0;
    e.drain_flag = f & 4 != 0;
    e.move_flag = ((f >> 3) & 3) as u8;
    e.mad_flag = f & 0x20 != 0;
    e.virus_flag = f & 0x40 != 0;
    e.opt_flag0 = f & 0x80 != 0;
    e.opt_flag1 = f & 0x100 != 0;
    e.ene_type = t.i16();
    e.ene_rand = t.i16();
    e.ene_smoke = t.i16();
    e.ene_part = t.i16();
    e.act_num = t.i16();
    e.act_cnt = t.i16();
    e.atk_num = t.i16();
    e.atk_cnt = t.i16();
    e.atk_dellay = t.i16();
    e.drain_cnt = t.i16();
    e.damage_cnt = t.i16();
    e.target = idx(t.int());
    e.target_dirc = t.u32();
    e.target_dist = t.u32();
    e.crisis_rate = t.u32();
    e.base_dirc = t.u32();
    e.base_dist = t.u32();
    e.bpos = v4(t);
    e.mdirc = v4(t);
    e.max_spd = t.u32();
    e.rad_cnt = t.u32();
    e.hit_cnt = t.i32();
    e.hit_spd = t.u32();
    e.speed = t.u32();
    e.yoffs = t.u32();
    e.zoffs = t.u32();
    e.anm_num = t.i16();
    e.anm_num_old = t.i16();
    e.frame_num = t.int() as u16;
    e.anm_flag = t.i16();
    e.life_rate = t.u32();
    e.skill_id = t.i32();
    e.skill_param = skill_ref(t.i32());
    e.skill_target = idx(t.int());
    e.skill_num = t.i32();
    for s in e.skill_list.iter_mut() {
        let f = t.int();
        *s = EnemySkill {
            atk_flag: f & 1 != 0,
            int_flag: f & 2 != 0,
            range_flag: f & 4 != 0,
            cost_flag: f & 8 != 0,
            atk_id: t.int() as i8,
            ski_id: t.i16(),
            percentage: t.u32(),
            ski_param: skill_ref(t.i32()),
            ski_target: idx(t.int()),
        };
    }
    e
}

fn skill_list_json(e: &Enemy) -> String {
    list(e.skill_list.iter().map(|s| {
        let f =
            u8::from(s.atk_flag) | u8::from(s.int_flag) << 1 | u8::from(s.range_flag) << 2 | u8::from(s.cost_flag) << 3;
        format!("[{},{},{},{},{},{}]", f, s.atk_id, s.ski_id, s.percentage, code(s.ski_param), ix(s.ski_target))
    }))
}

pub(crate) fn enemy_json(e: &Enemy) -> String {
    let ef = u8::from(e.obj_flag)
        | u8::from(e.init_flag) << 1
        | u8::from(e.freeze_flag) << 2
        | u8::from(e.affect_flag) << 3
        | u8::from(e.disp_sw) << 4
        | u8::from(e.dest_flag) << 5
        | u8::from(e.cmnd_flag) << 6
        | u8::from(e.ccs2_flag) << 7;
    let fl = u16::from(e.target_flag)
        | u16::from(e.action_flag) << 1
        | u16::from(e.drain_flag) << 2
        | u16::from(e.move_flag & 3) << 3
        | u16::from(e.mad_flag) << 5
        | u16::from(e.virus_flag) << 6
        | u16::from(e.opt_flag0) << 7
        | u16::from(e.opt_flag1) << 8;
    let shorts = [
        e.ene_type,
        e.ene_rand,
        e.ene_smoke,
        e.ene_part,
        e.act_num,
        e.act_cnt,
        e.atk_num,
        e.atk_cnt,
        e.atk_dellay,
        e.drain_cnt,
        e.damage_cnt,
    ];
    format!(
        "{{\"dirc\":{},\"affect\":{},\"eflags\":{},\"fade\":{},\"ent\":{},\"ids\":{},\"flags\":{},\"shorts\":{},\
         \"target\":{},\"f5\":{},\"bpos\":{},\"mdirc\":{},\"move\":{},\"anm\":{},\"life\":{},\"skill\":{},\"list\":{}}}",
        list(e.dirc),
        list([e.affect_type, e.affect_param0]),
        ef,
        list([e.fade_flag, e.fade_cnt]),
        ent_json(&e.ent),
        list([e.ene_id, e.race, e.race_id]),
        fl,
        list(shorts),
        ix(e.target),
        list([e.target_dirc, e.target_dist, e.crisis_rate, e.base_dirc, e.base_dist]),
        list(e.bpos),
        list(e.mdirc),
        list([
            i64::from(e.max_spd),
            i64::from(e.rad_cnt),
            i64::from(e.hit_cnt),
            i64::from(e.hit_spd),
            i64::from(e.speed),
            i64::from(e.yoffs),
            i64::from(e.zoffs)
        ]),
        list([i32::from(e.anm_num), i32::from(e.anm_num_old), i32::from(e.frame_num), i32::from(e.anm_flag)]),
        e.life_rate,
        list([i64::from(e.skill_id), i64::from(code(e.skill_param)), ix(e.skill_target), i64::from(e.skill_num)]),
        skill_list_json(e)
    )
}

/// A character as the harness reads it, with its positions.
fn char_json(c: &Char) -> String {
    let st = state(c);
    format!(
        "{},\"pos\":{},\"posP\":{},\"skill\":{}}}",
        &st[..st.len() - 1],
        list(c.pos),
        list(c.pos_p),
        list([c.skill_id, c.skill_status])
    )
}

fn out_json(o: &Out, me: usize, names: &Names) -> Option<String> {
    let n = |i: usize| format!("\"c{i}\"");
    Some(match *o {
        Out::Rule(ev) => {
            let s = calls(&[ev], names);
            s[1..s.len() - 1].to_string()
        }
        Out::Hold { target } => {
            format!("[\"EntryAffect\",{},{},5,0,0,0]", target.map_or("0".into(), n), n(me))
        }
        Out::Exp { level } => format!("[\"ccExpDistributor\",{level}]"),
        Out::SkillStart(r) => format!("[\"effSkillStart\",{},{},0,0]", n(me), code(r)),
        Out::Remove => format!("[\"entryEnemyObject\",{}]", n(me)),
        Out::DrainSpawn(s) => format!("[\"entryObject\",{},1],[\"effAfterDrain\",\"new\",-1]", ent_json(&s.ent)),
        Out::ClearConditionEffect | Out::DeleteCmnd | Out::KillRecord { .. } | Out::InBattleDist(_) => return None,
    })
}

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
    for c in s.chars.iter_mut() {
        c.pos = v4(t);
    }
    let mut foes: Vec<Option<Enemy>> = (0..n).map(|_| if t.int() != 0 { Some(read_enemy(t)) } else { None }).collect();
    let puppet_show = t.int() != 0;
    let ride = t.int() != 0;
    let active_enemies = t.i32();
    let player = idx(t.int());
    let mut rand = Rand(t.int() as u64);
    let mut cc = genrand(t);
    let env = read_env(t);
    let kill_count = t.int() as u8;
    let server = t.i32();
    let area: [i32; 3] = std::array::from_fn(|_| t.i32());
    let op = t.word().to_string();
    let me = t.int() as usize;
    for (c, f) in s.chars.iter_mut().zip(&foes) {
        // A foe's entRoot and ccChar word +0x140 are one.
        if let Some(f) = f {
            c.ent_root = f.ent.ent_root as u32;
        }
    }
    let ene_id = foes[me].as_ref().map_or(0, |e| e.ene_id);
    let mut save = SaveData::new();
    save.set_u8(enemy_ai::SAVE_ENEMY_KILL_COUNT + ene_id as usize, kill_count);
    let world = World { puppet_show, ride, active_enemies, player, frame: &IDENTITY };
    let mut ai = Ai { t: tables, scene: &mut s, foes: &mut foes, world, rand: &mut rand, cc: &mut cc, out: Vec::new() };
    let mut extra: Vec<String> = Vec::new();
    let ret: i64 = match op.as_str() {
        "think" => {
            ai.default_think(me);
            0
        }
        "interrupt" => {
            ai.interrupt_think(me);
            0
        }
        "begin" => match ai.begin_frame(me, &env) {
            Begin::Continue => {
                // The rest of main: think() (empty on ccEnemy), then the
                // stubbed interruptThink, action() (empty), hold cleared,
                // moveEnemy, animEnemy, dispEnemy when displayed.
                extra.push(format!("[\"interruptThink\",\"c{me}\"]"));
                ai.end_action(me);
                extra.push(format!("[\"moveEnemy\",\"c{me}\"]"));
                extra.push(format!("[\"animEnemy\",\"c{me}\"]"));
                if ai.foes[me].as_ref().is_some_and(|e| e.disp_sw) {
                    extra.push(format!("[\"dispEnemy\",\"c{me}\"]"));
                }
                0
            }
            Begin::Removed | Begin::Drained => 1,
            Begin::Frozen { ret } => i64::from(ret),
        },
        "routine" => {
            ai.routine_enemy(me, &env);
            0
        }
        "check" => {
            ai.check_enemy(me);
            0
        }
        "crisis" => {
            ai.check_crisis_rate(me);
            0
        }
        "setact" => {
            let a = t.i16();
            ai.set_act(me, a);
            0
        }
        "attack" => i64::from(ai.select_attack(me)),
        "skills" => i64::from(ai.check_skill_list(me)),
        "target" => {
            let lt = t.i32();
            i64::from(ai.select_target_by(me, lt))
        }
        "target0" => i64::from(ai.select_target(me)),
        "interval" => {
            ai.set_interval(me);
            0
        }
        "start" => {
            if let SkillUse::Request { target, sid } = ai.start_skill(me) {
                extra.push(format!(
                    "[\"request\",\"c{}\",{},{},0]",
                    me,
                    target.map_or("0".into(), |i| format!("\"c{i}\"")),
                    sid
                ));
            }
            0
        }
        "affect" => {
            if let Some((tg, r)) = ai.affect_skill(me) {
                extra.push(format!("[\"ccSkillDamage\",\"c{me}\",\"c{tg}\",{},0]", code(Some(r))));
            }
            0
        }
        "clearcond" => {
            ai.clear_condition_enemy(me);
            0
        }
        "skilltarget" => {
            let sid = t.i32();
            let sk = tables.skills[sid as usize].clone();
            ix(ai.select_skill_target(me, sid, &sk))
        }
        _ => return format!("{{\"error\":\"unknown enemy op {op}\"}}"),
    };
    let me_name = format!("c{me}");
    let names = Names { me: &me_name, target: "c?", chars: (0..n).map(|i| format!("c{i}")).collect(), ai: false };
    let mut new = "null".to_string();
    let mut cl: Vec<String> = Vec::new();
    for o in &ai.out {
        if let Some(j) = out_json(o, me, &names) {
            cl.push(j);
        }
        match o {
            Out::KillRecord { ene_id } => enemy_ai::record_kill(&mut save, *ene_id, server, area),
            Out::DrainSpawn(sp) => {
                let mut e = Enemy::default();
                let mut c = Char::foe(Default::default());
                enemy_ai::after_drain_spawn(&mut e, &mut c, sp);
                new = list([i32::from(c.cond[0]), i32::from(e.drain_cnt)]);
            }
            _ => {}
        }
    }
    cl.extend(extra);
    let at = enemy_ai::SAVE_ENEMY_KILL_AREA + 8 * ene_id as usize;
    let kill = list([
        i32::from(save.u8(enemy_ai::SAVE_ENEMY_KILL_COUNT + ene_id as usize) as i8),
        i32::from(save.i16(at)),
        i32::from(save.i16(at + 2)),
        i32::from(save.i16(at + 4)),
        i32::from(save.i16(at + 6)),
    ]);
    let chars = list(s.chars.iter().map(char_json));
    let foes_json = list(foes.iter().map(|f| f.as_ref().map_or("null".into(), enemy_json)));
    format!(
        "{{\"ret\":{},\"rand\":{},\"cc\":{},\"calls\":[{}],\"chars\":{},\"foes\":{},\"lists\":[{},{}],\"kill\":{},\"new\":{}}}",
        ret,
        rand.0,
        cc_json(&cc),
        cl.join(","),
        chars,
        foes_json,
        list(s.pc_list.iter()),
        list(s.ene_list.iter()),
        kill,
        new
    )
}
