//! `battle_probe` requests for the enemies' motion
//! (`piney_battle::enemy_motion`), as `tools/test_battle_enemy_motion_rs.py`
//! sends them: `mhelp FN ARGS` (one of the helpers: `setrad`, `setdist`,
//! `dircchgf`, `setdirc`, `getdirc`, `getdist`, `disperse`, `pilimit`, the
//! rotations, `boss`) and `mscene`: `enemy_ai`'s `escene`, each foe's motion
//! members, the world's script (the answers of the collision, camera and
//! animation calls in order), then one operation (`move`, `anim`, `note`,
//! `disp`, `act`, the escapes, `action`, the races' movers, `main FRAMES`...).

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use piney_battle::affect::AffectCtx;
use piney_battle::blocks::InfoRef;
use piney_battle::chara::{AffectFunc, Char};
use piney_battle::enemy_ai::{self, Ai, Enemy, Frame, Out, SkillRef};
use piney_battle::enemy_motion::{self, At, Call, Kind, Motion, MotionData, MotionWorld};
use piney_battle::event::Event;
use piney_battle::exp::Party;
use piney_battle::geom::{self, F, M4, V4};
use piney_battle::rand::{Genrand, Rand};
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_battle::world::{AnmSlot, CharHit, Note, World};
use piney_data::save::SaveData;

use crate::{Names, Toks, calls, list, read_char, read_env, state};

thread_local! {
    static DATA: RefCell<Option<MotionData>> = const { RefCell::new(None) };
}

/// Answers a command this module knows; None for any other.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    Some(match cmd {
        "mhelp" => helper(t),
        "mscene" => {
            // the acting row's attacks' targetRange, changed for the case
            let saved = if t.int() != 0 {
                let id = t.int() as usize;
                let row = &mut tables.enemies[id];
                let old = [
                    row.atc[0].target_range,
                    row.atc[1].target_range,
                    row.ski[0].target_range,
                    row.ski[1].target_range,
                ];
                row.atc[0].target_range = t.u32();
                row.atc[1].target_range = t.u32();
                row.ski[0].target_range = t.u32();
                row.ski[1].target_range = t.u32();
                Some((id, old))
            } else {
                None
            };
            let r = scene_request(tables, t);
            if let Some((id, old)) = saved {
                let row = &mut tables.enemies[id];
                row.atc[0].target_range = old[0];
                row.atc[1].target_range = old[1];
                row.ski[0].target_range = old[2];
                row.ski[1].target_range = old[3];
            }
            r
        }
        _ => return None,
    })
}

fn data(tables: &Tables) -> MotionData {
    DATA.with(|d| {
        let mut d = d.borrow_mut();
        if d.is_none() {
            let path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
            let mut iso = piney_data::iso::Iso::open(&path).expect("the ISO");
            *d = Some(MotionData::read_iso(&mut iso, tables).expect("the motion data"));
        }
        d.clone().unwrap()
    })
}

fn v4(t: &mut Toks) -> V4 {
    std::array::from_fn(|_| t.u32())
}

fn m4(t: &mut Toks) -> M4 {
    std::array::from_fn(|_| v4(t))
}

/// Values separated by commas, for a flat JSON list.
fn flat<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

fn m4_json(m: &M4) -> String {
    list(m.iter().flatten())
}

fn helper(t: &mut Toks) -> String {
    let f = t.word().to_string();
    match f.as_str() {
        "setrad" => {
            let mut r = t.u32();
            let (to, rate) = (t.u32(), t.u32());
            geom::set_rad(&mut r, to, rate);
            r.to_string()
        }
        "setdist" => {
            let mut v = t.u32();
            let (to, rate) = (t.u32(), t.u32());
            geom::set_dist(&mut v, to, rate);
            v.to_string()
        }
        "dircchgf" => {
            let (a, b, m) = (t.u32(), t.u32(), t.i32());
            geom::get_dirc_chg_f(a, b, m).to_string()
        }
        "setdirc" => {
            let (r, to, m) = (t.u32(), t.u32(), t.i32());
            geom::set_dirc(r, to, m).to_string()
        }
        "getdirc" => {
            let (a, b) = (v4(t), v4(t));
            geom::get_dirc(a, b).to_string()
        }
        "getdist" => {
            let (a, b) = (v4(t), v4(t));
            geom::get_dist_on(crate::probe_volume(), a, b).to_string()
        }
        "disperse" => {
            let (v, x) = (t.u32(), t.u32());
            let mut cc = Genrand::seeded(t.u32());
            cc.mti = t.i32();
            let r = geom::rad_disperse(v, x, &mut cc);
            format!("[{},{}]", r, cc_json(&cc))
        }
        "pilimit" | "pilimitp" => geom::pi_limit(t.u32()).to_string(),
        "rotz" | "rotx" | "roty" => {
            let m = m4(t);
            let r = t.u32();
            let out = match f.as_str() {
                "rotz" => geom::rot_matrix_z(&m, r),
                "rotx" => geom::rot_matrix_x(&m, r),
                _ => geom::rot_matrix_y(&m, r),
            };
            m4_json(&out)
        }
        "rot" => {
            let m = m4(t);
            let r = v4(t);
            m4_json(&geom::rot_matrix(&m, r))
        }
        "apply" => {
            let m = m4(t);
            let v = v4(t);
            list(geom::apply_matrix(&m, v))
        }
        "trans" => {
            let m = m4(t);
            let v = v4(t);
            m4_json(&geom::trans_matrix(&m, v))
        }
        "unit" => m4_json(&geom::unit_matrix()),
        _ => format!("{{\"error\":\"unknown helper {f}\"}}"),
    }
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

// The scripted world -----------------------------------------------------------------

/// What the harness answers the game's world functions with, in the order
/// they are asked, and the calls, as the harness records them.
#[derive(Default)]
struct Script {
    ox: F,
    oy: F,
    land: VecDeque<F>,
    collide: VecDeque<(i32, V4, u32)>,
    line: VecDeque<F>,
    shake: VecDeque<bool>,
    trans: VecDeque<F>,
    anim: VecDeque<(i16, u32, Vec<Note>)>,
    frame_now: HashMap<(usize, u8), u32>,
    notes: HashMap<usize, Vec<Note>>,
    log: Vec<String>,
    /// The rules' outputs are named by the acting enemy.
    names: Vec<String>,
}

struct SWorld(RefCell<Script>);

fn slot_no(s: AnmSlot) -> u8 {
    match s {
        AnmSlot::Main => 0,
        AnmSlot::Second => 1,
    }
}

impl Script {
    fn w2p(&self, v: V4) -> V4 {
        [geom::sub(v[0], self.ox), geom::sub(v[1], self.oy), v[2], v[3]]
    }
    fn p2w(&self, v: V4) -> V4 {
        [geom::add(v[0], self.ox), geom::add(v[1], self.oy), v[2], v[3]]
    }
    fn n(&self, i: usize) -> String {
        format!("\"c{i}\"")
    }
}

impl Frame for SWorld {
    fn w2p(&self, v: [u32; 4]) -> [u32; 4] {
        self.0.borrow().w2p(v)
    }
    fn p2w(&self, v: [u32; 4]) -> [u32; 4] {
        self.0.borrow().p2w(v)
    }
}

impl World for &SWorld {
    fn w2p(&mut self, pos: V4) -> V4 {
        let mut s = self.0.borrow_mut();
        s.log.push(format!("[\"w2p\",{}]", flat(pos)));
        s.w2p(pos)
    }
    fn p2w(&mut self, pos: V4) -> V4 {
        let mut s = self.0.borrow_mut();
        s.log.push(format!("[\"p2w\",{}]", flat(pos)));
        s.p2w(pos)
    }
    fn land(&mut self, pos: V4, mask: u32) -> F {
        let mut s = self.0.borrow_mut();
        s.log.push(format!("[\"land\",{},{}]", flat(pos), mask));
        s.land.pop_front().unwrap_or(0)
    }
    fn hit_attribute(&mut self) -> u32 {
        0
    }
    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> F {
        let mut s = self.0.borrow_mut();
        s.log.push(format!("[\"line\",{},{},{},{}]", flat(from), flat(to), mask, kind));
        s.line.pop_front().unwrap_or(geom::MINUS_ONE)
    }
    fn collide(&mut self, who: usize, hit: &mut CharHit) -> i32 {
        let mut s = self.0.borrow_mut();
        let n = s.n(who);
        s.log.push(format!("[\"collide\",{},{},{},{}]", n, hit.radius, hit.height, flat(hit.pos)));
        let (r, off, attr) = s.collide.pop_front().unwrap_or((0, [0; 4], 0));
        hit.offset = off;
        hit.attribute = attr;
        r
    }
    fn hit_char_type(&mut self) -> u32 {
        0
    }
    fn hit_switch(&mut self, _who: usize, hit: &mut CharHit, on: bool) {
        hit.sw = on;
    }
    fn camera_deg(&mut self, _pos: V4, _deg: i16) -> bool {
        false
    }
    fn camera_transparency(&mut self, pos: V4, width: F, height: F, far: F, len: F) -> F {
        let mut s = self.0.borrow_mut();
        s.log.push(format!("[\"transparency\",{},{},{},{},{}]", flat(pos), width, height, far, len));
        s.trans.pop_front().unwrap_or(geom::ONE)
    }
    fn anim_set(&mut self, who: usize, slot: AnmSlot, name: &str) {
        let mut s = self.0.borrow_mut();
        let n = s.n(who);
        s.log.push(format!("[\"anim_set\",{},{},\"{}\"]", n, slot_no(slot), name));
        s.frame_now.insert((who, slot_no(slot)), 0);
    }
    fn anim_frame(&mut self, who: usize, slot: AnmSlot) -> u16 {
        self.0.borrow().frame_now.get(&(who, slot_no(slot))).copied().unwrap_or(0) as u16
    }
    fn anim_forward(&mut self, who: usize, slot: AnmSlot, step: u16) -> i16 {
        let mut s = self.0.borrow_mut();
        let n = s.n(who);
        s.log.push(format!("[\"anim_forward\",{},{},{}]", n, slot_no(slot), step));
        let (r, next, notes) = s.anim.pop_front().unwrap_or((0, 0, Vec::new()));
        s.frame_now.insert((who, slot_no(slot)), next);
        if slot == AnmSlot::Main {
            s.notes.insert(who, notes);
        }
        r
    }
    fn anim_notes(&mut self, who: usize, slot: AnmSlot) -> Vec<Note> {
        if slot != AnmSlot::Main {
            return Vec::new();
        }
        self.0.borrow_mut().notes.remove(&who).unwrap_or_default()
    }
}

fn code(r: Option<SkillRef>) -> i32 {
    match r {
        None => -1,
        Some(SkillRef::Row(k)) => 1000 + i32::from(k),
        Some(SkillRef::Table(s)) => s,
    }
}

fn ent_json(e: &enemy_ai::EntryParam) -> String {
    let mut v: Vec<i64> = e.pos.iter().chain(e.dirc.iter()).map(|&x| i64::from(x)).collect();
    v.extend([e.ty, e.id, e.area, e.area_num, e.floor, e.block, e.x, e.y, e.ent_root, e.land].map(i64::from));
    v.extend(e.param.map(i64::from));
    list(v)
}

/// An output of the rules as the harness's hooks record it (as
/// `enemy_ai`'s requests print them); None for those that call nothing.
fn out_json(o: &Out, me: usize, names: &Names) -> Option<String> {
    let n = |i: usize| format!("\"c{i}\"");
    Some(match *o {
        Out::Rule(ev) => {
            let s = calls(&[ev], names);
            s[1..s.len() - 1].to_string()
        }
        Out::Hold { target } => format!("[\"EntryAffect\",{},{},5,0,0,0]", target.map_or("0".into(), n), n(me)),
        Out::Exp { level } => format!("[\"ccExpDistributor\",{level}]"),
        Out::SkillStart(r) => format!("[\"effSkillStart\",{},{},0,0]", n(me), code(r)),
        Out::Remove => format!("[\"entryEnemyObject\",{}]", n(me)),
        Out::DrainSpawn(s) => format!("[\"entryObject\",{},1],[\"effAfterDrain\",\"new\",-1]", ent_json(&s.ent)),
        Out::ClearConditionEffect | Out::DeleteCmnd | Out::KillRecord { .. } | Out::InBattleDist(_) => return None,
    })
}

/// The side effects the probe keeps of the rules' outputs: the enemy book
/// and the drained form.
#[derive(Default)]
struct Book {
    kills: Vec<i32>,
    new: Option<String>,
}

impl MotionWorld for &SWorld {
    fn shake_range(&mut self, pos: V4) -> bool {
        let mut s = self.0.borrow_mut();
        s.log.push(format!("[\"shake\",{}]", flat(pos)));
        s.shake.pop_front().unwrap_or(false)
    }
    fn call(&mut self, who: usize, c: Call, _at: &mut At) {
        let mut s = self.0.borrow_mut();
        let n = s.n(who);
        let line = match c {
            // ccSkillRequest(ch, 0, 0): the harness records the request it
            // makes.
            Call::Rule(Out::Rule(Event::CancelAttack(w))) => {
                let w = match w {
                    piney_battle::event::Who::Char(i) => s.n(i),
                    _ => n.clone(),
                };
                format!("[\"request\",{w},0,0,0]")
            }
            // ClearConditionEffect runs: it deletes an effect only when one
            // is shown, which no case sets up.
            Call::Rule(Out::Rule(Event::ClearConditionEffect(_))) => return,
            Call::Rule(o) => {
                if let Out::KillRecord { ene_id } = o {
                    BOOK.with(|b| b.borrow_mut().kills.push(ene_id));
                }
                if let Out::DrainSpawn(sp) = o {
                    let mut e = Enemy::default();
                    let mut ch = Char::foe(Default::default());
                    enemy_ai::after_drain_spawn(&mut e, &mut ch, &sp);
                    BOOK.with(|b| b.borrow_mut().new = Some(list([i32::from(ch.cond[0]), i32::from(e.drain_cnt)])));
                }
                let me_name = format!("c{who}");
                let names = Names { me: &me_name, target: "c?", chars: s.names.clone(), ai: false };
                match out_json(&o, who, &names) {
                    Some(j) => j,
                    None => return,
                }
            }
            Call::SkillRequest { target, sid } => {
                format!("[\"request\",{},{},{},0]", n, target.map_or("0".into(), |i| s.n(i)), sid)
            }
            Call::Sound { param, category } => format!("[\"sound\",{},{},{}]", param, n, category),
            Call::CameraShake { kind } => format!("[\"cameraShake\",{kind},2,20,2]"),
            Call::WeaponNote { note, .. } => format!("[\"weaponNote\",{},{},{}]", n, note.event, note.param),
            Call::WeaponCtrl(_) => format!("[\"weaponCtrl\",{n}]"),
            Call::DustCtrl { flag, .. } => format!("[\"dustCtrl\",{n},{flag}]"),
            Call::DrawSecond { matrix, alpha } => {
                format!("[\"drawSecond\",{},{},{}]", n, flat(matrix.iter().flatten()), alpha)
            }
            Call::Draw { matrix } => format!("[\"draw\",{},{}]", n, flat(matrix.iter().flatten())),
            Call::Sound3d { id, pos } => format!("[\"se3d\",{},{}]", id, flat(pos)),
            Call::Dust { pos, size, smoke } => format!("[\"dust\",{},1,{},6,{}]", flat(pos), size, smoke),
            Call::EffDust { pos, n: k, size, life, smoke } => {
                format!("[\"dust\",{},{},{},{},{}]", flat(pos), k, size, life, smoke)
            }
            Call::BreathCtrl { slot } => format!("[\"breathCtrl\",{},{}]", n, slot),
            // the harness hooks GetObjAdrsF and sees the foot's name
            Call::FootDust { foot, .. } => format!("[\"footDust\",{foot}]"),
            Call::BreathSet { slot, param, .. } => {
                format!("[\"breathSet\",{},{},{}]", n, slot, crate::spawn::info_va(param))
            }
            // A box's trap: no box in the enemies' checks.
            Call::TrapDamage { .. } | Call::TrapSkill { .. } => return,
        };
        s.log.push(line);
    }
}

thread_local! {
    static BOOK: RefCell<Book> = RefCell::new(Book::default());
}

// The scene ---------------------------------------------------------------------------

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

fn hit_json(h: &CharHit) -> String {
    let mut v: Vec<u32> = vec![u32::from(h.sw), h.mask, h.mask2, h.kind, h.radius, h.height];
    v.extend(h.pos);
    v.extend(h.offset);
    v.push(h.attribute);
    list(v)
}

fn read_hit(t: &mut Toks) -> CharHit {
    CharHit {
        sw: t.int() != 0,
        mask: t.u32(),
        mask2: t.u32(),
        kind: t.u32(),
        radius: t.u32(),
        height: t.u32(),
        pos: v4(t),
        offset: v4(t),
        attribute: t.u32(),
    }
}

fn read_note(t: &mut Toks) -> Note {
    Note { event: t.u32(), param: t.u32() }
}

fn scene_request(tables: &Tables, t: &mut Toks) -> String {
    let data = data(tables);
    let n = t.int() as usize;
    let mut s = Scene::default();
    for _ in 0..n {
        s.chars.push(read_char(t));
    }
    let k = t.int();
    s.pc_list = (0..k).map(|_| t.int() as usize).collect();
    let k = t.int();
    s.ene_list = (0..k).map(|_| t.int() as usize).collect();
    for c in s.chars.iter_mut() {
        c.pos = v4(t);
    }
    let mut foes: Vec<Option<Enemy>> =
        (0..n).map(|_| if t.int() != 0 { Some(crate::enemy_ai::read_enemy(t)) } else { None }).collect();
    let puppet_show = t.int() != 0;
    let ride = t.int() != 0;
    let active_enemies = t.i32();
    let player = usize::try_from(t.int()).ok();
    let mut rand = Rand(t.int() as u64);
    let mut cc = Genrand::seeded(t.u32());
    cc.mti = t.i32();
    let env = read_env(t);
    let kill_count = t.int() as u8;
    let server = t.i32();
    let area: [i32; 3] = std::array::from_fn(|_| t.i32());
    for (c, f) in s.chars.iter_mut().zip(&foes) {
        if let Some(f) = f {
            c.ent_root = f.ent.ent_root as u32;
        }
    }
    // extras
    for c in s.chars.iter_mut() {
        c.base_mut().height = t.u32();
    }
    let mut script = Script { names: (0..n).map(|i| format!("c{i}")).collect(), ..Script::default() };
    for (i, f) in foes.iter_mut().enumerate() {
        if let Some(e) = f {
            e.hit = read_hit(t);
            e.anm_tbl = t.u32();
            e.set_transparency = t.u32();
            // Only whether the race made its controllers matters here.
            let made = InfoRef::new("", 0);
            e.weapon = (t.int() != 0).then_some((made, 0));
            e.dust = (t.int() != 0).then_some((made, 0));
            e.gold.flag = t.int() != 0;
            e.breath = t.int() != 0;
            script.frame_now.insert((i, 0), t.u32());
            script.frame_now.insert((i, 1), t.u32());
        }
    }
    script.ox = t.u32();
    script.oy = t.u32();
    script.land = (0..t.int()).map(|_| t.u32()).collect();
    script.collide = (0..t.int()).map(|_| (t.i32(), v4(t), t.u32())).collect();
    script.line = (0..t.int()).map(|_| t.u32()).collect();
    script.shake = (0..t.int()).map(|_| t.int() != 0).collect();
    script.trans = (0..t.int()).map(|_| t.u32()).collect();
    script.anim = (0..t.int())
        .map(|_| {
            let r = t.i16();
            let next = t.u32();
            let notes = (0..t.int()).map(|_| read_note(t)).collect();
            (r, next, notes)
        })
        .collect();
    // what the affects read: ccSkillCheck's answer, ccMenu, the party
    let running = t.i32();
    let menu = t.int() != 0;
    let mut party = Party::default();
    for k in 0..3 {
        party.ids[k] = t.i32();
    }
    let check = move |_: usize| running;
    let actx = AffectCtx { party: &party, menu, skill_check: &check, boss: None, volume: crate::probe_volume() };
    // an enemy's copies of its ccChar and ccEnemy words
    for (c, f) in s.chars.iter_mut().zip(&foes) {
        if let Some(e) = f {
            c.affect.ty = e.affect_type;
            c.affect.param[0] = e.affect_param0;
            c.spc_char.enemy_flags = u16::from(e.virus_flag) << 6 | u16::from(e.drain_flag) << 2;
            if c.affect.func == AffectFunc::None {
                c.affect.func = AffectFunc::Enemy;
            }
        }
    }
    let op = t.word().to_string();
    let me = t.int() as usize;
    let ene_id = foes[me].as_ref().map_or(0, |e| e.ene_id);
    let mut save = SaveData::new();
    save.set_u8(enemy_ai::SAVE_ENEMY_KILL_COUNT + ene_id as usize, kill_count);
    BOOK.with(|b| *b.borrow_mut() = Book::default());
    let sw = SWorld(RefCell::new(script));
    let world = enemy_ai::World { puppet_show, ride, active_enemies, player, frame: &sw };
    let mut ret: i64 = 0;
    let mut frames: Vec<String> = Vec::new();
    if op == "main" {
        let nframes = t.int();
        let mut live: Vec<usize> = (0..n)
            .filter(|&i| foes[i].as_ref().is_some_and(|e| !matches!(Kind::of_row(tables, e.ene_id), Kind::Other(_))))
            .collect();
        for _ in 0..nframes {
            // the party's moves, then events on the enemies
            for _ in 0..t.int() {
                let i = t.int() as usize;
                let pos = v4(t);
                let pos_p = v4(t);
                let c = &mut s.chars[i];
                c.pos = pos;
                c.pos_p = pos_p;
                c.cond[0] = t.i16();
            }
            for _ in 0..t.int() {
                let i = t.int() as usize;
                let kind = t.int();
                let (a, b) = (t.i32(), t.i32());
                apply_event(&mut s, &mut foes, i, kind, a, b);
            }
            let mut rets = Vec::new();
            {
                let mut ai = Ai {
                    t: tables,
                    scene: &mut s,
                    foes: &mut foes,
                    world,
                    rand: &mut rand,
                    cc: &mut cc,
                    out: Vec::new(),
                };
                let mut w = &sw;
                let mut m = Motion { ai: &mut ai, w: &mut w, data: &data, affect: &actx, env: &env };
                for &i in live.clone().iter() {
                    let r = m.enemy_main(i);
                    rets.push(format!("[{i},{r}]"));
                    if r == 1 {
                        // removed, drained or frozen off the lists: the
                        // harness stops calling it
                        live.retain(|&x| x != i);
                    }
                }
            }
            let log = std::mem::take(&mut sw.0.borrow_mut().log);
            frames.push(format!(
                "{{\"rets\":[{}],\"calls\":[{}],\"rand\":{},\"cc\":{},\"state\":{}}}",
                rets.join(","),
                log.join(","),
                rand.0,
                cc_json(&cc),
                state_json(&s, &foes)
            ));
        }
    } else {
        let mut ai =
            Ai { t: tables, scene: &mut s, foes: &mut foes, world, rand: &mut rand, cc: &mut cc, out: Vec::new() };
        let mut w = &sw;
        let mut m = Motion { ai: &mut ai, w: &mut w, data: &data, affect: &actx, env: &env };
        match op.as_str() {
            "move" => m.move_enemy(me),
            "anim" => m.anim_enemy(me),
            "note" => {
                for _ in 0..t.int() {
                    let note = read_note(t);
                    m.check_note(me, note);
                }
            }
            "disp" => m.disp_enemy(me),
            "act" => {
                let which = t.int();
                let a: [u32; 7] = std::array::from_fn(|_| t.u32());
                let e = m.ai.foes[me].as_mut().unwrap();
                match which {
                    0 => enemy_motion::act_move(e, a[0], a[1], a[2], a[3], a[4], a[5], a[6]),
                    1 => enemy_motion::act_follow(e, a[0], a[1], a[2], a[3], a[4]),
                    _ => enemy_motion::act_slide(e, a[0], a[1], a[2], a[3], a[4]),
                }
            }
            "escape" => m.act_escape(me),
            "escapeby" => {
                let ty = t.i32();
                let red = t.u32();
                m.act_escape_by(me, ty, red);
            }
            "escapex" => m.act_escape_x(me),
            "action" => m.action(me),
            "moveeg" => m.move_eg(me),
            "moveegg" => m.move_egg(me),
            "moveeb" => m.move_eb(me),
            "excl" => m.exclusive(me),
            "rnote" => {
                let note = read_note(t);
                m.race_note(me, note);
            }
            "freeze" => {
                let mut e = m.ai.foes[me].take().unwrap();
                ret = i64::from(enemy_motion::freeze_g(m.ai.scene, me, &mut e));
                m.ai.foes[me] = Some(e);
            }
            _ => return format!("{{\"error\":\"unknown motion op {op}\"}}"),
        }
        // what the rules pushed outside a flush (the act helpers' own calls)
        m.flush(me);
    }
    let log = std::mem::take(&mut sw.0.borrow_mut().log);
    let book = BOOK.with(|b| std::mem::take(&mut *b.borrow_mut()));
    for id in &book.kills {
        enemy_ai::record_kill(&mut save, *id, server, area);
    }
    let at = enemy_ai::SAVE_ENEMY_KILL_AREA + 8 * ene_id as usize;
    let kill = list([
        i32::from(save.u8(enemy_ai::SAVE_ENEMY_KILL_COUNT + ene_id as usize) as i8),
        i32::from(save.i16(at)),
        i32::from(save.i16(at + 2)),
        i32::from(save.i16(at + 4)),
        i32::from(save.i16(at + 6)),
    ]);
    if op == "main" {
        return format!("{{\"frames\":[{}],\"kill\":{}}}", frames.join(","), kill);
    }
    format!(
        "{{\"ret\":{},\"rand\":{},\"cc\":{},\"calls\":[{}],\"state\":{},\"kill\":{},\"new\":{}}}",
        ret,
        rand.0,
        cc_json(&cc),
        log.join(","),
        state_json(&s, &foes),
        kill,
        book.new.unwrap_or_else(|| "null".into())
    )
}

/// The state the harness compares: every character, every foe's
/// `ccEnemy` fields and body hit, the command lists.
fn state_json(s: &Scene, foes: &[Option<Enemy>]) -> String {
    let chars = list(s.chars.iter().zip(foes).map(|(c, f)| {
        let a = &c.affect;
        let sc = &c.spc_char;
        let spc = if f.is_some() {
            "null".to_string()
        } else {
            list([
                i64::from(sc.act_num),
                i64::from(sc.flags),
                i64::from(sc.fellow_flags),
                i64::from(sc.enemy_flags),
                i64::from(sc.arms_effect_sw),
                i64::from(sc.attack),
                i64::from(sc.cnt),
                i64::from(sc.cloak),
            ])
        };
        let st = char_json(c);
        format!(
            "{},\"affect\":{},\"spc\":{},\"target\":{}}}",
            &st[..st.len() - 1],
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
            spc,
            c.target_char.map_or(-1, |i| i as i64)
        )
    }));
    let foes_json = list(foes.iter().map(|f| f.as_ref().map_or("null".into(), crate::enemy_ai::enemy_json)));
    let hits = list(foes.iter().map(|f| f.as_ref().map_or("null".into(), |e| hit_json(&e.hit))));
    format!(
        "{{\"chars\":{},\"foes\":{},\"hits\":{},\"lists\":[{},{}]}}",
        chars,
        foes_json,
        hits,
        list(s.pc_list.iter()),
        list(s.ene_list.iter())
    )
}

/// An event the harness applies to an enemy between frames: 0 its HP, 1
/// an affect taken (kind, first parameter), 2 a condition's timer
/// (index, value), 3 a Data Drain, 4 `freezeFlag`.
fn apply_event(s: &mut Scene, foes: &mut [Option<Enemy>], i: usize, kind: i64, a: i32, b: i32) {
    let e = foes[i].as_mut().expect("an enemy");
    let ch = &mut s.chars[i];
    match kind {
        0 => ch.hp = a as i16,
        1 => {
            // ccEnemyInfluence's marks, on the enemy's copy and its ccChar
            e.note_affect(a as i16, b as i16, false);
            ch.affect.ty = a as i16;
            ch.affect.param[0] = b as i16;
        }
        2 => ch.cond[a as usize] = b as i16,
        3 => e.drain_flag = true,
        _ => e.freeze_flag = a != 0,
    }
}
