//! `battle_probe` requests for the entry control, the magic circle and the
//! race constructors (`piney_battle::entry`, `piney_battle::races`), as
//! `tools/test_battle_spawn_rs.py` sends them: `spawn SCENE OP ARGS` (a scene
//! by [`read_scene`], then one operation, answered by [`scene_json`] with the
//! world's calls and outputs), `fenemies` ([`enemies_request`]: frames of
//! `ccThEntryCtrl` with the enemies' own `main` through [`EnemySeam`]), `sreg`
//! (the registration), `rands` (`ccRandS`), `dust` (`ccCheckDustColor`) and
//! `evsave` (the save's event entries and fountains).

use std::cell::RefCell;
use std::collections::HashMap;

use piney_battle::affect::AffectCtx;
use piney_battle::blocks::InfoRef;
use piney_battle::chara::{AffectFunc, Body, Char, enemy_flag, spc_flag};
use piney_battle::enemy_ai::{self, Enemy, EntryParam, Frame, Genrand, SkillRef};
use piney_battle::enemy_motion::{At, Call, MotionData, MotionWorld};
use piney_battle::entry::{
    self, Cx, DungeonGims, EditGim, EditRoom, EnemySeam, EntryCtrl, EntryGimmickSeam, EntryObj, EvPos, FieldMap, Game,
    GimSlot, Kind, Link, List, MagicCircle, McPart, Obj, Out, Register, Seam, SpawnTables, WorldMan,
};
use piney_battle::event::{Event, Who};
use piney_battle::exp::Party;
use piney_battle::geom::{self, F, V4};
use piney_battle::gimetc::Etc;
use piney_battle::gimmick::{self, Class, Food, Idol};
use piney_battle::param::{Base, Elm};
use piney_battle::prim::{OmniLight, PrimPart, PrimVert, RadOut, Radiate};
use piney_battle::races::{self, Gold};
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_battle::world::{AnmSlot, CharHit, Note, World};
use piney_data::iso::Iso;
use piney_data::save::SaveData;

use crate::enemy_ai::{enemy_json, read_enemy};
use crate::{Names, Toks, calls, list, read_env, state};

thread_local! {
    static ST: RefCell<Option<SpawnTables>> = const { RefCell::new(None) };
    static MD: RefCell<Option<MotionData>> = const { RefCell::new(None) };
}

fn motion_data(tables: &Tables) -> MotionData {
    MD.with(|d| {
        let mut d = d.borrow_mut();
        if d.is_none() {
            let path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
            let mut iso = Iso::open(&path).expect("the ISO");
            *d = Some(MotionData::read_iso(&mut iso, tables).expect("the motion data"));
        }
        d.clone().unwrap()
    })
}

/// Where the volume keeps a block's row: what the game passes.
pub(crate) fn info_va(info: InfoRef) -> u32 {
    with_st(|st| info.va(st.volume)).unwrap_or(0)
}

pub(crate) fn with_st<R>(f: impl FnOnce(&SpawnTables) -> R) -> R {
    ST.with(|s| {
        if s.borrow().is_none() {
            let path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
            let mut iso = Iso::open(&path).expect("the ISO");
            *s.borrow_mut() = Some(SpawnTables::of(iso.volume().expect("the volume")));
        }
        f(s.borrow().as_ref().unwrap())
    })
}

/// Answers a command this module knows; None for any other.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    Some(match cmd {
        "spawn" => with_st(|st| spawn_request(tables, st, t)),
        "sreg" => with_st(|st| register_request(tables, st, t)),
        "rands" => {
            let mut s = t.int() as u16;
            let n = t.int();
            let v: Vec<i16> = (0..n).map(|_| entry::rand_s(&mut s)).collect();
            format!("{{\"v\":{},\"s\":{}}}", list(v), s)
        }
        "dust" => {
            let mut w = ScriptWorld::default();
            w.attr.push(t.u32());
            w.land.push(0);
            let pos = v4(t);
            let d = races::check_dust_color(&mut w, pos);
            format!("{{\"ret\":{},\"calls\":{}}}", d, w.calls_json())
        }
        "evsave" => evsave_request(t),
        _ => return None,
    })
}

fn v4(t: &mut Toks) -> V4 {
    std::array::from_fn(|_| t.u32())
}

fn ix(v: Option<usize>) -> i64 {
    v.map_or(-1, |i| i as i64)
}

fn idx(v: i64) -> Option<usize> {
    if v < 0 { None } else { Some(v as usize) }
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

fn ent_vec(e: &EntryParam) -> Vec<i64> {
    let mut v: Vec<i64> = e.pos.iter().chain(e.dirc.iter()).map(|&x| i64::from(x)).collect();
    v.extend([e.ty, e.id, e.area, e.area_num, e.floor, e.block, e.x, e.y, e.ent_root, e.land].map(i64::from));
    v.extend(e.param.map(i64::from));
    v
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
        ..CharHit::default()
    }
}

fn hit_vec(h: &CharHit) -> Vec<i64> {
    let mut v = vec![
        i64::from(h.sw),
        i64::from(h.mask),
        i64::from(h.mask2),
        i64::from(h.kind),
        i64::from(h.radius),
        i64::from(h.height),
    ];
    v.extend(h.pos.map(i64::from));
    v
}

// the scripted world ------------------------------------------------------------------

/// The world as the harness answers the game's calls: the player's frame
/// an offset `k` on the ground, the other queries from scripts in order
/// (0 once a script runs out), every call but the frame's recorded.
#[derive(Default)]
struct ScriptWorld {
    k: [F; 2],
    land: Vec<F>,
    attr: Vec<u32>,
    camdeg: Vec<bool>,
    camtr: Vec<F>,
    fwd: Vec<i16>,
    calls: Vec<String>,
    /// Objects with a clip set on a slot.
    clips: std::collections::HashSet<(usize, u8)>,
    // The enemies' own frame (`fenemies`): the answers of
    // `CollisionDetection`, `ccHitCheckLM2`, `checkCameraShakeRange` and
    // `_AnimateForward` (its result, the frame it leaves, the notes it
    // passes), the players' frames and notes.
    motion: bool,
    collide: Vec<(i32, V4, u32)>,
    line: Vec<F>,
    shake: Vec<bool>,
    anim: Vec<(i16, u32, Vec<Note>)>,
    frame_now: HashMap<(usize, u8), u32>,
    notes: HashMap<usize, Vec<Note>>,
    /// `game.inBattleDist` as the enemies last set it (a gold goblin
    /// shaking off a hold), `IBD_UNSET` while none has.
    ibd: Option<u32>,
}

/// What the harness leaves in `game.inBattleDist` before an operation.
const IBD_UNSET: u32 = 0x7f7f_7f7f;

fn pop<T: Copy + Default>(v: &mut Vec<T>) -> T {
    if v.is_empty() { T::default() } else { v.remove(0) }
}

impl ScriptWorld {
    fn calls_json(&self) -> String {
        format!("[{}]", self.calls.join(","))
    }
}

fn slot_no(s: AnmSlot) -> u8 {
    match s {
        AnmSlot::Main => 0,
        AnmSlot::Second => 1,
    }
}

impl World for ScriptWorld {
    fn w2p(&mut self, p: V4) -> V4 {
        [geom::sub(p[0], self.k[0]), geom::sub(p[1], self.k[1]), p[2], p[3]]
    }
    fn p2w(&mut self, p: V4) -> V4 {
        [geom::add(p[0], self.k[0]), geom::add(p[1], self.k[1]), p[2], p[3]]
    }
    fn land(&mut self, pos: V4, mask: u32) -> F {
        self.calls.push(format!("[\"land\",{},{}]", list(pos), mask));
        pop(&mut self.land)
    }
    fn hit_attribute(&mut self) -> u32 {
        self.calls.push("[\"attr\"]".into());
        pop(&mut self.attr)
    }
    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> F {
        self.calls.push(format!("[\"line\",{},{},{},{}]", list(from), list(to), mask, kind));
        if self.line.is_empty() { geom::MINUS_ONE } else { self.line.remove(0) }
    }
    fn collide(&mut self, who: usize, hit: &mut CharHit) -> i32 {
        self.calls.push(format!("[\"collide\",{},{},{},{}]", who, hit.radius, hit.height, list(hit.pos)));
        let (r, off, attr) = pop(&mut self.collide);
        hit.offset = off;
        hit.attribute = attr;
        r
    }
    fn hit_char_type(&mut self) -> u32 {
        0
    }
    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        self.calls.push(format!("[\"hit\",{},{}]", who, u8::from(on)));
        hit.sw = on;
    }
    fn camera_deg(&mut self, pos: V4, deg: i16) -> bool {
        self.calls.push(format!("[\"camdeg\",{},{}]", list(pos), deg));
        pop(&mut self.camdeg)
    }
    fn camera_transparency(&mut self, pos: V4, w: F, h: F, far: F, len: F) -> F {
        self.calls.push(format!("[\"camtr\",{},{}]", list(pos), list([w, h, far, len])));
        pop(&mut self.camtr)
    }
    fn anim_set(&mut self, who: usize, slot: AnmSlot, name: &str) {
        self.calls.push(format!("[\"anim_set\",{},{},\"{}\"]", who, slot_no(slot), name));
        self.clips.insert((who, slot_no(slot)));
        self.frame_now.insert((who, slot_no(slot)), 0);
    }
    fn anim_frame(&mut self, who: usize, slot: AnmSlot) -> u16 {
        self.frame_now.get(&(who, slot_no(slot))).copied().unwrap_or(0) as u16
    }
    fn anim_forward(&mut self, who: usize, slot: AnmSlot, step: u16) -> i16 {
        if !self.clips.contains(&(who, slot_no(slot))) {
            return 0;
        }
        self.calls.push(format!("[\"anim_forward\",{},{},{}]", who, slot_no(slot), step));
        if !self.motion {
            return pop(&mut self.fwd);
        }
        let (r, next, notes) = if self.anim.is_empty() { (0, 0, Vec::new()) } else { self.anim.remove(0) };
        self.frame_now.insert((who, slot_no(slot)), next);
        if slot == AnmSlot::Main {
            self.notes.insert(who, notes);
        }
        r
    }
    fn anim_notes(&mut self, who: usize, slot: AnmSlot) -> Vec<Note> {
        if slot != AnmSlot::Main {
            return Vec::new();
        }
        self.notes.remove(&who).unwrap_or_default()
    }
}

fn skill_code(r: Option<SkillRef>) -> i32 {
    match r {
        None => -1,
        Some(SkillRef::Row(k)) => 1000 + i32::from(k),
        Some(SkillRef::Table(s)) => s,
    }
}

/// The enemies' calls as the harness's hooks record them, characters named
/// `cI` by scene index.
impl MotionWorld for ScriptWorld {
    fn shake_range(&mut self, pos: V4) -> bool {
        self.calls.push(format!("[\"shake\",{}]", list(pos)));
        pop(&mut self.shake)
    }
    fn call(&mut self, who: usize, c: Call, at: &mut At) {
        let n = |i: usize| format!("\"c{i}\"");
        let me = n(who);
        let line = match c {
            // ccSkillRequest(ch, 0, 0): the request it makes
            Call::Rule(enemy_ai::Out::Rule(Event::CancelAttack(w))) => {
                let w = match w {
                    Who::Char(i) => n(i),
                    _ => me.clone(),
                };
                format!("[\"request\",{w},0,0,0]")
            }
            // it deletes an effect only when one is shown, which none is
            Call::Rule(enemy_ai::Out::Rule(Event::ClearConditionEffect(_))) => return,
            Call::Rule(enemy_ai::Out::Rule(ev)) => {
                let me_name = format!("c{who}");
                let chars = (0..at.scene.chars.len()).map(|i| format!("c{i}")).collect();
                let names = Names { me: &me_name, target: "c?", chars, ai: false };
                let s = calls(&[ev], &names);
                s[1..s.len() - 1].to_string()
            }
            Call::Rule(enemy_ai::Out::Exp { level }) => format!("[\"ccExpDistributor\",{level}]"),
            Call::Rule(enemy_ai::Out::SkillStart(r)) => format!("[\"effSkillStart\",{},{},0,0]", me, skill_code(r)),
            Call::Rule(enemy_ai::Out::InBattleDist(d)) => {
                self.ibd = Some(d);
                return;
            }
            Call::Rule(_) => return,
            Call::SkillRequest { target, sid } => {
                format!("[\"request\",{},{},{},0]", me, target.map_or("0".into(), n), sid)
            }
            Call::Sound { param, category } => format!("[\"sound\",{param},{me},{category}]"),
            Call::CameraShake { kind } => format!("[\"cameraShake\",{kind},2,20,2]"),
            Call::WeaponNote { note, .. } => format!("[\"weaponNote\",{},{},{}]", me, note.event, note.param),
            Call::WeaponCtrl(_) => format!("[\"weaponCtrl\",{me}]"),
            Call::DustCtrl { flag, .. } => format!("[\"dustCtrl\",{me},{flag}]"),
            Call::DrawSecond { matrix, alpha } => {
                let m = list(matrix.iter().flatten());
                format!("[\"drawSecond\",{},{},{}]", me, &m[1..m.len() - 1], alpha)
            }
            Call::Draw { matrix } => {
                let m = list(matrix.iter().flatten());
                format!("[\"draw\",{},{}]", me, &m[1..m.len() - 1])
            }
            Call::Sound3d { id, pos } => format!("[\"se3d\",{},{}]", id, list(pos)),
            // ccTransPosFW2LW(p, pos) first: W2P then P2W
            Call::Dust { pos, size, smoke } => {
                let p = self.w2p(pos);
                let p = self.p2w(p);
                format!("[\"dust\",{},1,{},6,{}]", list(p), size, smoke)
            }
            Call::EffDust { pos, n, size, life, smoke } => {
                format!("[\"dust\",{},{n},{size},{life},{smoke}]", list(pos))
            }
            Call::BreathCtrl { slot } => format!("[\"breathCtrl\",{me},{slot}]"),
            // the harness hooks GetObjAdrsF and sees the foot's name
            Call::FootDust { foot, .. } => format!("[\"footDust\",{foot}]"),
            Call::BreathSet { slot, param, .. } => format!("[\"breathSet\",{me},{slot},{}]", info_va(param)),
            Call::TrapDamage { target, sid } => format!("[\"trap_damage\",{},{}]", ix(target), sid),
            Call::TrapSkill { target, sid } => format!("[\"trap_skill\",{},{},0]", ix(target), sid),
        };
        self.calls.push(line);
    }
}

/// The objects' mains and the other constructors as the harness answers
/// them: the mains' results from scripts (recorded), a gimmick or NPC made
/// as `ccGimmick::ccGimmick` leaves it with the entry copied in.
#[derive(Default)]
struct ScriptSeam {
    enemy_main: Vec<bool>,
    gimmick_main: Vec<bool>,
    npc_main: Vec<bool>,
    calls: RefCell<Vec<String>>,
}

fn made_obj(cx: &mut Cx, ent: &EntryParam, who: usize, npc: bool) -> (Char, EntryObj) {
    let mut ch = if npc { entry::npc_char(cx.st, ent.id) } else { entry::gimmick_char(cx.st, ent.id) };
    ch.ent_root = ent.ent_root as u32;
    ch.pos = ent.pos;
    ch.pos_p = cx.world.w2p(ent.pos);
    let o = EntryObj { ent: *ent, dirc: ent.dirc, gim_id: ent.id, ..EntryObj::default() };
    let _ = who;
    (ch, o)
}

impl Seam for ScriptSeam {
    fn enemy_main(&mut self, _: &mut EntryCtrl, _: &mut Cx, who: usize) -> bool {
        self.calls.borrow_mut().push(format!("[\"enemy_main\",{who}]"));
        pop(&mut self.enemy_main)
    }
    /// A treasure box's or an idol's own `main` (their classes are
    /// ported, as the game's natives run); any other gimmick's scripted.
    fn gimmick_main(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx, who: usize) -> bool {
        if let Some(r) = piney_battle::gimmick::main(ctrl, cx, self, 0, who) {
            return r;
        }
        self.calls.borrow_mut().push(format!("[\"gimmick_main\",{who}]"));
        pop(&mut self.gimmick_main)
    }
    fn npc_main(&mut self, _: &mut EntryCtrl, _: &mut Cx, who: usize) -> bool {
        self.calls.borrow_mut().push(format!("[\"npc_main\",{who}]"));
        pop(&mut self.npc_main)
    }
    fn make_enemy(&mut self, _: &mut Cx, race: i32, _: &EntryParam, _: usize) -> (Char, Enemy) {
        panic!("an enemy of race {race}, which the checks do not spawn");
    }
    fn make_gimmick(&mut self, cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj) {
        self.calls.borrow_mut().push(format!("[\"make_gimmick\",{},{}]", who, list(ent_vec(ent))));
        made_obj(cx, ent, who, false)
    }
    fn make_npc(&mut self, cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj) {
        self.calls.borrow_mut().push(format!("[\"make_npc\",{},{}]", who, list(ent_vec(ent))));
        made_obj(cx, ent, who, true)
    }
}

// the scene ---------------------------------------------------------------------------

/// Everything a `spawn` request sets up.
struct Setup {
    game: Game,
    world: ScriptWorld,
    seam: ScriptSeam,
    ctrl: EntryCtrl,
    reg: Register,
    cc: Genrand,
    rnds: u16,
    save: SaveData,
    scene: Scene,
    foes: Vec<Option<Enemy>>,
    pat_num: u16,
    /// `fenemies`: party members (kind 5) and the enemies' full state.
    motion: bool,
}

fn read_list(t: &mut Toks) -> List {
    List { num: t.i32(), head: idx(t.int()), foot: idx(t.int()) }
}

fn list_vec(l: &List) -> [i64; 3] {
    [i64::from(l.num), ix(l.head), ix(l.foot)]
}

fn read_obj_common(t: &mut Toks) -> EntryObj {
    let dirc = v4(t);
    let transparency = t.u32();
    let set_transparency = t.u32();
    let f = t.int();
    EntryObj {
        dirc,
        transparency,
        set_transparency,
        obj_flag: f & 1 != 0,
        init_flag: f & 2 != 0,
        freeze_flag: f & 4 != 0,
        affect_flag: f & 8 != 0,
        disp_sw: f & 0x10 != 0,
        dest_flag: f & 0x20 != 0,
        cmnd_flag: f & 0x40 != 0,
        ccs2_flag: f & 0x80 != 0,
        pl_dist: t.u32(),
        pl_dirc: t.u32(),
        alpha: t.i32(),
        fade_flag: t.i16(),
        fade_cnt: t.i16(),
        grot_deg: t.i16(),
        grot_spd: t.i16(),
        ent: read_ent(t),
        hit: read_hit(t),
        anm_tbl: t.u32(),
        dest_func: t.u32(),
        gim_id: t.i32(),
        act_num: t.i32(),
        act_cnt: t.i32(),
        anm_flag: t.i32(),
        class: Class::None,
    }
}

/// A radiate: `FLAGS TYPE PNUM center scale lscale angle bank alpha length
/// width fzoom dpLength dpBank USER PACKET pos[4] rot[4] attr life actNum
/// actCnt param[4] light(matrix[16] matCalc rgb intensity far[3]) then
/// each part's `col0 col1` and four points' `color pos[4]`.
fn read_rad(t: &mut Toks, user: usize) -> Radiate {
    let mut r = Radiate::new(gimmick_rad_info(), Some(user));
    let f = t.int();
    r.rad_flag = f & 1 != 0;
    r.lgt_flag = f & 2 != 0;
    r.ty = t.i16();
    r.pnum = t.i16();
    for w in [
        &mut r.center,
        &mut r.scale,
        &mut r.lscale,
        &mut r.angle,
        &mut r.bank,
        &mut r.alpha,
        &mut r.length,
        &mut r.width,
        &mut r.fzoom,
        &mut r.dp_length,
        &mut r.dp_bank,
    ] {
        *w = t.u32();
    }
    r.user = idx(t.int());
    r.packet = t.i32();
    r.pos = v4(t);
    r.rot = v4(t);
    r.attr = t.i16();
    r.life = t.i16();
    r.act_num = t.i16();
    r.act_cnt = t.i16();
    r.param = std::array::from_fn(|_| t.u32());
    let matrix = std::array::from_fn(|_| v4(t));
    r.light = OmniLight {
        matrix,
        mat_calc: t.int() != 0,
        rgb: t.u32(),
        intensity: t.u32(),
        far_start: t.u32(),
        far_end: t.u32(),
        far_end2: t.u32(),
    };
    r.parts = (0..16)
        .map(|_| {
            let col0 = t.u32();
            let col1 = t.u32();
            let vert = std::array::from_fn(|_| PrimVert { color: t.u32(), pos: v4(t) });
            PrimPart { col0, col1, vert }
        })
        .collect();
    r
}

fn gimmick_rad_info() -> piney_battle::prim::RadInfo {
    piney_battle::prim::BOX_RAD_INFO
}

fn rad_vec(r: &Radiate) -> Vec<i64> {
    let mut v = vec![i64::from(u8::from(r.rad_flag) | u8::from(r.lgt_flag) << 1), i64::from(r.ty), i64::from(r.pnum)];
    v.extend(
        [r.center, r.scale, r.lscale, r.angle, r.bank, r.alpha, r.length, r.width, r.fzoom, r.dp_length, r.dp_bank]
            .map(i64::from),
    );
    v.extend([ix(r.user), i64::from(r.packet)]);
    v.extend(r.pos.map(i64::from));
    v.extend(r.rot.map(i64::from));
    v.extend([r.attr, r.life, r.act_num, r.act_cnt].map(i64::from));
    v.extend(r.param.map(i64::from));
    let l = &r.light;
    v.extend(l.matrix.iter().flatten().map(|&x| i64::from(x)));
    v.extend([i64::from(l.mat_calc), i64::from(l.rgb), i64::from(l.intensity)]);
    v.extend([l.far_start, l.far_end, l.far_end2].map(i64::from));
    for p in &r.parts {
        v.extend([i64::from(p.col0), i64::from(p.col1)]);
        for q in &p.vert {
            v.push(i64::from(q.color));
            v.extend(q.pos.map(i64::from));
        }
    }
    v
}

/// A gimmick's class: `0`, `1 RAD` (a box), `2 EFFSW IDOLID RAD` (an
/// idol), `4` and a food's members (as [`class_vec`] lists them) or `5 ETC`
/// (a `ccGimEtc`: [`etc_vec`]).
fn read_class(t: &mut Toks, who: usize) -> Class {
    match t.int() {
        1 => Class::Box(Box::new(read_rad(t, who))),
        2 => {
            let effsw = t.i32();
            let idol_id = t.i32();
            Class::Idol(Box::new(Idol { effsw, idol_id, rad: read_rad(t, who) }))
        }
        4 => {
            let near = t.i32() != 0;
            let kind = t.i32();
            let scale = [t.u32(), t.u32(), t.u32(), t.u32()];
            let roll_spd = [t.u32(), t.u32()];
            let (amp, step, gain) = (t.u32(), t.u32(), t.u32());
            let rolling = t.i32();
            let roll_rate = [t.i32(), t.i32()];
            let (rand_s, bounce) = (t.i32() as i16, t.i32() as i16);
            let base_dirc = t.u32();
            Class::Food(Box::new(Food {
                near,
                kind,
                scale,
                roll_spd,
                amp,
                step,
                gain,
                rolling,
                roll_rate,
                rand_s,
                bounce,
                base_dirc,
            }))
        }
        5 => Class::Etc(Box::new(read_etc(t))),
        _ => Class::None,
    }
}

/// `EFFSW FLAGS SCALE4 VPOS4 RADCNT WINDOWOFS SCALE0_4 BOUNCECNT BOUNCEDEG4
/// BOUNCEOFS SCALE1_4 SPRINGCNT SPRINGDEC SPRINGZOOM LGTFLAG` and each
/// light's `RGB INTENSITY FARSTART FAREND FAREND2 X Y Z` (FLAGS: bit 0
/// spiritFlag, bit 1 spiritType).
fn read_etc(t: &mut Toks) -> Etc {
    let v4 = |t: &mut Toks| [t.u32(), t.u32(), t.u32(), t.u32()];
    let effsw = t.i32();
    let flags = t.int();
    let mut e = Etc {
        effsw,
        spirit_flag: flags & 1 != 0,
        spirit_type: flags & 2 != 0,
        scale: v4(t),
        vpos: v4(t),
        rad_cnt: t.u32(),
        window_ofs: t.u32(),
        scale0: v4(t),
        bounce_cnt: t.i32(),
        bounce_deg: [t.i32() as i16, t.i32() as i16, t.i32() as i16, t.i32() as i16],
        bounce_ofs: t.u32(),
        scale1: v4(t),
        spring_cnt: t.u32(),
        spring_dec: t.u32(),
        spring_zoom: t.u32(),
        lgt_flag: t.int() != 0,
        ..Etc::default()
    };
    for l in &mut e.lights {
        l.rgb = t.u32();
        l.intensity = t.u32();
        l.far_start = t.u32();
        l.far_end = t.u32();
        l.far_end2 = t.u32();
        l.matrix = geom::unit_matrix();
        l.matrix[3] = [t.u32(), t.u32(), t.u32(), geom::ONE];
    }
    e
}

fn etc_vec(e: &Etc) -> Vec<i64> {
    let mut v = vec![5, i64::from(e.effsw), i64::from(u8::from(e.spirit_flag) | u8::from(e.spirit_type) << 1)];
    v.extend(e.scale.map(i64::from));
    v.extend(e.vpos.map(i64::from));
    v.extend([e.rad_cnt, e.window_ofs].map(i64::from));
    v.extend(e.scale0.map(i64::from));
    v.push(i64::from(e.bounce_cnt));
    v.extend(e.bounce_deg.map(i64::from));
    v.push(i64::from(e.bounce_ofs));
    v.extend(e.scale1.map(i64::from));
    v.extend([e.spring_cnt, e.spring_dec, e.spring_zoom].map(i64::from));
    v.push(i64::from(e.lgt_flag));
    for l in &e.lights {
        v.extend([l.rgb, l.intensity, l.far_start, l.far_end, l.far_end2].map(i64::from));
        v.extend(l.matrix[3][..3].iter().map(|&x| i64::from(x)));
    }
    v
}

fn class_vec(c: &Class) -> Vec<i64> {
    match c {
        Class::None => vec![0],
        Class::Box(r) => {
            let mut v = vec![1];
            v.extend(rad_vec(r));
            v
        }
        Class::Idol(i) => {
            let mut v = vec![2, i64::from(i.effsw), i64::from(i.idol_id)];
            v.extend(rad_vec(&i.rad));
            v
        }
        Class::Symbol(s) => vec![3, i64::from(s.act), i64::from(s.cnt), i64::from(s.light_on)],
        Class::Food(f) => {
            let mut v = vec![4, i64::from(f.near), i64::from(f.kind)];
            v.extend(f.scale.map(i64::from));
            v.extend(f.roll_spd.map(i64::from));
            v.extend([f.amp, f.step, f.gain].map(i64::from));
            v.push(i64::from(f.rolling));
            v.extend(f.roll_rate.map(i64::from));
            v.extend([i64::from(f.rand_s), i64::from(f.bounce), i64::from(f.base_dirc)]);
            v
        }
        Class::Etc(e) => etc_vec(e),
    }
}

fn obj_flags(o: &EntryObj) -> i64 {
    i64::from(
        u8::from(o.obj_flag)
            | u8::from(o.init_flag) << 1
            | u8::from(o.freeze_flag) << 2
            | u8::from(o.affect_flag) << 3
            | u8::from(o.disp_sw) << 4
            | u8::from(o.dest_flag) << 5
            | u8::from(o.cmnd_flag) << 6
            | u8::from(o.ccs2_flag) << 7,
    )
}

fn obj_vec(o: &EntryObj) -> Vec<i64> {
    let mut v: Vec<i64> = o.dirc.iter().map(|&x| i64::from(x)).collect();
    v.extend([i64::from(o.transparency), i64::from(o.set_transparency), obj_flags(o)]);
    v.extend([i64::from(o.pl_dist), i64::from(o.pl_dirc), i64::from(o.alpha)]);
    v.extend([o.fade_flag, o.fade_cnt, o.grot_deg, o.grot_spd].map(i64::from));
    v.extend(ent_vec(&o.ent));
    v.extend(hit_vec(&o.hit));
    v.extend([i64::from(o.anm_tbl), i64::from(o.dest_func)]);
    v.extend([o.gim_id, o.act_num, o.act_cnt, o.anm_flag].map(i64::from));
    v
}

fn read_part(t: &mut Toks) -> McPart {
    McPart {
        status: t.i32(),
        anm_flag: t.int() != 0,
        mat: std::array::from_fn(|_| v4(t)),
        speed: t.u32(),
        rad_cnt: t.u32(),
        mc_act_cnt: t.i16(),
        eff_anm_pat: t.int() as u16,
        cnt: t.i16(),
        life: t.i16(),
        transparency: t.u32(),
        rnd: t.u32(),
        eff: entry::Eff {
            pos: v4(t),
            scale_x: t.u32(),
            scale_y: t.u32(),
            rotate: t.u32(),
            color: t.u32(),
            pat_num: t.int() as u16,
            transparency: t.u32(),
        },
    }
}

fn part_vec(p: &McPart) -> Vec<i64> {
    let mut v = vec![i64::from(p.status), i64::from(p.anm_flag)];
    for c in &p.mat {
        v.extend(c.map(i64::from));
    }
    v.extend([i64::from(p.speed), i64::from(p.rad_cnt)]);
    v.extend([i64::from(p.mc_act_cnt), i64::from(p.eff_anm_pat), i64::from(p.cnt), i64::from(p.life)]);
    v.extend([i64::from(p.transparency), i64::from(p.rnd)]);
    v.extend(p.eff.pos.map(i64::from));
    v.extend([p.eff.scale_x, p.eff.scale_y, p.eff.rotate, p.eff.color].map(i64::from));
    v.extend([i64::from(p.eff.pat_num), i64::from(p.eff.transparency)]);
    v
}

/// A digest of the particles' words: the sum of each word times its odd
/// position weight, modulo 2^64.
fn parts_hash(mc: &MagicCircle) -> u64 {
    let mut h: u64 = 0;
    let mut i: u64 = 0;
    for p in &mc.parts {
        for x in part_vec(p) {
            h = h.wrapping_add(u64::from(x as u32).wrapping_mul(2 * i + 1));
            i += 1;
        }
    }
    h
}

/// A character: `KIND BASE pos[4] posP[4] HP SP maxHP maxSP cond[16]
/// speedValue conditionNum skillID skillStatus entRoot`, `BASE` the type
/// (kind 0), the `enemyTbl` row (1), the gimmick (2, 3) or NPC (4) id.
fn read_char(t: &mut Toks, t_: &Tables, st: &SpawnTables, kind: i64) -> Char {
    let b = t.i32();
    let mut c = match kind {
        1 => {
            let mut c = Char::foe(t_.enemies[b as usize].param.clone());
            c.affect.func = AffectFunc::Enemy;
            c
        }
        2 | 3 => entry::gimmick_char(st, b),
        4 => entry::npc_char(st, b),
        _ => Char::other(Base { ty: b, ..Base::default() }, Elm::default()),
    };
    c.pos = v4(t);
    c.pos_p = v4(t);
    c.hp = t.i16();
    c.sp = t.i16();
    c.max_hp = t.i16();
    c.max_sp = t.i16();
    for k in 0..16 {
        c.cond[k] = t.i16();
    }
    c.cond.speed_value = t.u32();
    c.condition_num = t.i32();
    c.skill_id = t.i16();
    c.skill_status = t.i16();
    c.ent_root = t.u32();
    c
}

fn char_vec(c: &Char) -> Vec<i64> {
    let mut v = vec![i64::from(c.ty()), i64::from(c.id())];
    v.extend(c.pos.map(i64::from));
    v.extend(c.pos_p.map(i64::from));
    v.extend([c.hp, c.sp, c.max_hp, c.max_sp].map(i64::from));
    v.extend(c.cond.v.map(i64::from));
    v.extend([i64::from(c.cond.speed_value), i64::from(c.condition_num)]);
    v.extend([i64::from(c.skill_id), i64::from(c.skill_status), i64::from(c.ent_root)]);
    v.push(match c.affect.func {
        AffectFunc::Enemy => 1,
        _ => 0,
    });
    v
}

/// An enemy: the enemy AI's `read_enemy` fields, then `plDist plDirc alpha
/// grotDeg grotSpd transparency setTransparency destFunc hit[10] anmTbl`.
fn read_enemy_full(t: &mut Toks, ch: &mut Char) -> Enemy {
    let (mut e, on) = read_enemy(t);
    on.put(ch);
    e.pl_dist = t.u32();
    e.pl_dirc = t.u32();
    e.alpha = t.i32();
    e.grot_deg = t.i16();
    e.grot_spd = t.i16();
    e.transparency = t.u32();
    e.set_transparency = t.u32();
    e.dest_func = t.u32();
    e.hit = read_hit(t);
    e.anm_tbl = t.u32();
    e
}

fn gold_vec(g: &Gold) -> Vec<i64> {
    let mut v = vec![
        i64::from(u8::from(g.flag) | u8::from(g.pre_hold) << 1 | (g.dis_hold & 3) << 2),
        i64::from(g.volume),
        i64::from(g.ty),
        i64::from(g.cnt),
    ];
    v.extend(g.param.map(i64::from));
    v.extend([g.dirc, g.rotate, g.speed, g.accel].map(i64::from));
    v.extend(g.esc_pos.map(i64::from));
    v.push(i64::from(g.esc_cnt));
    v
}

fn enemy_extra_vec(e: &Enemy) -> Vec<i64> {
    let mut v = vec![i64::from(e.pl_dist), i64::from(e.pl_dirc), i64::from(e.alpha)];
    v.extend([i64::from(e.grot_deg), i64::from(e.grot_spd)]);
    v.extend([i64::from(e.transparency), i64::from(e.set_transparency), i64::from(e.dest_func)]);
    v.extend(hit_vec(&e.hit));
    v.push(i64::from(e.anm_tbl));
    let pair = |p: Option<(InfoRef, i32)>| p.map_or([0, 0], |(a, n)| [i64::from(info_va(a)), i64::from(n)]);
    v.extend(pair(e.weapon));
    v.extend(pair(e.dust));
    v.extend(gold_vec(&e.gold));
    v.extend([i64::from(e.spin), i64::from(e.breath)]);
    v.extend(e.l_rand.map(i64::from));
    v
}

/// `GAME K CTRL REG RNG SAVE SCRIPTS NOBJ OBJ... LISTS PATNUM`; see the
/// harness's `ser_scene`.
fn read_scene(t_: &Tables, st: &SpawnTables, t: &mut Toks) -> Setup {
    let game = Game {
        area: t.i32(),
        town: t.i32(),
        field: t.i32(),
        dungeon: t.i32(),
        floor: t.i32(),
        block: t.i32(),
        server: t.i32(),
        field_type: t.i32(),
        area_code: t.i32(),
        player: idx(t.int()),
        area_level: t.i32(),
    };
    let mut world = ScriptWorld { k: [t.u32(), t.u32()], ..ScriptWorld::default() };
    let mut ctrl =
        EntryCtrl { area: t.i32(), area_num: t.i32(), floor: t.i32(), block: t.i32(), ..EntryCtrl::default() };
    for l in ctrl.lists.iter_mut() {
        *l = read_list(t);
    }
    let mut reg = Register::default();
    for c in reg.cells.iter_mut() {
        *c = t.i32();
    }
    reg.enemy_num = t.i32();
    reg.drain_num = t.i32();
    reg.range = t.i32();
    for _ in 0..t.int() {
        let i = t.int() as usize;
        reg.exist[i] = 1;
    }
    let mut cc = Genrand::seeded(t.u32());
    cc.mti = t.i32();
    let rnds = t.int() as u16;
    let mut save = SaveData::new();
    let field = game.field;
    for k in 0..6 {
        let at = entry::SAVE_EVENT_ENTRY + 24 * field.clamp(0, 160) as usize + 4 * k;
        save.set_i32(at, t.i32());
    }
    for k in 0..100 {
        save.set_i32(entry::SAVE_FOUNTAIN + 4 * k, t.i32());
    }
    for at in [entry::SAVE_CIRCLES, entry::SAVE_FIELDS_CLEARED, entry::SAVE_DUNGEONS_CLEARED] {
        save.set_i16(at, t.i16());
    }
    world.land = (0..t.int()).map(|_| t.u32()).collect();
    world.attr = (0..t.int()).map(|_| t.u32()).collect();
    world.camdeg = (0..t.int()).map(|_| t.int() != 0).collect();
    world.camtr = (0..t.int()).map(|_| t.u32()).collect();
    world.fwd = (0..t.int()).map(|_| t.i16()).collect();
    let seam = ScriptSeam {
        enemy_main: (0..t.int()).map(|_| t.int() != 0).collect(),
        gimmick_main: (0..t.int()).map(|_| t.int() != 0).collect(),
        npc_main: (0..t.int()).map(|_| t.int() != 0).collect(),
        ..ScriptSeam::default()
    };
    let n = t.int() as usize;
    let mut scene = Scene::default();
    let mut foes = vec![None; n];
    ctrl.links = vec![Link::default(); n];
    ctrl.objs = vec![Obj::None; n];
    for (i, foe) in foes.iter_mut().enumerate() {
        let kind = t.int();
        ctrl.links[i] = Link { pre: idx(t.int()), next: idx(t.int()) };
        let ch = if kind == 5 { crate::read_char(t) } else { read_char(t, t_, st, kind) };
        scene.chars.push(ch);
        match kind {
            1 => {
                *foe = Some(read_enemy_full(t, &mut scene.chars[i]));
                ctrl.objs[i] = Obj::Enemy;
            }
            2 => {
                let obj = read_obj_common(t);
                let part_flag = t.int() != 0;
                let part_top = t.i32();
                let parts = (0..entry::PARTS).map(|_| read_part(t)).collect();
                ctrl.objs[i] = Obj::Circle(Box::new(MagicCircle { obj, part_flag, part_top, parts }));
                world.clips.insert((i, 0));
            }
            3 => {
                let mut o = read_obj_common(t);
                o.class = read_class(t, i);
                if !matches!(o.class, Class::None) {
                    world.clips.insert((i, 0));
                }
                // affectType, affectPerson, the +0xe0 affect bit's copy
                let c = scene.chars.last_mut().expect("the character");
                c.affect.ty = t.i16();
                c.affect.person = idx(t.int());
                ctrl.objs[i] = Obj::Gimmick(Box::new(o));
            }
            4 => ctrl.objs[i] = Obj::Npc(Box::new(read_obj_common(t))),
            _ => {}
        }
    }
    for l in [&mut scene.pc_list, &mut scene.ene_list, &mut scene.obj_list] {
        *l = (0..t.int()).map(|_| t.int() as usize).collect();
    }
    let pat_num = t.int() as u16;
    Setup { game, world, seam, ctrl, reg, cc, rnds, save, scene, foes, pat_num, motion: false }
}

/// A character as the enemy motion harness reads it: its state, positions,
/// skill, affect words, a party member's `ccSpcChar` words, its target.
fn full_char_json(c: &Char, foe: bool) -> String {
    let st = state(c);
    let a = &c.affect;
    let sc = &c.spc_char;
    let spc = if foe {
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
    format!(
        "{},\"pos\":{},\"posP\":{},\"skill\":{},\"affect\":{},\"spc\":{},\"target\":{}}}",
        &st[..st.len() - 1],
        list(c.pos),
        list(c.pos_p),
        list([c.skill_id, c.skill_status]),
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
}

fn obj_json(s: &Setup, i: usize, full: bool) -> String {
    let ch = list(char_vec(&s.scene.chars[i]));
    let l = s.ctrl.links.get(i).copied().unwrap_or_default();
    let link = list([ix(l.pre), ix(l.next)]);
    match s.ctrl.objs.get(i).unwrap_or(&Obj::None) {
        Obj::None if s.motion && matches!(s.scene.chars[i].body, Body::Spc(_)) => {
            format!("{{\"kind\":5,\"char\":{},\"link\":{link}}}", full_char_json(&s.scene.chars[i], false))
        }
        Obj::None => format!("{{\"kind\":0,\"char\":{ch},\"link\":{link}}}"),
        Obj::Enemy => {
            let e = s.foes[i].as_ref().expect("an enemy");
            let mut extra = enemy_extra_vec(e);
            let ch = if s.motion {
                extra.extend(e.hit.offset.map(i64::from));
                extra.push(i64::from(e.hit.attribute));
                full_char_json(&s.scene.chars[i], true)
            } else {
                ch
            };
            format!(
                "{{\"kind\":1,\"char\":{ch},\"link\":{link},\"enemy\":{},\"extra\":{}}}",
                enemy_json(e, &s.scene.chars[i]),
                list(extra)
            )
        }
        Obj::Circle(c) => {
            let parts = if full { list(c.parts.iter().map(|p| list(part_vec(p)))) } else { parts_hash(c).to_string() };
            format!(
                "{{\"kind\":2,\"char\":{ch},\"link\":{link},\"obj\":{},\"circle\":[{},{}],\"parts\":{parts}}}",
                list(obj_vec(&c.obj)),
                i64::from(c.part_flag),
                c.part_top
            )
        }
        Obj::Gimmick(o) => {
            let c = &s.scene.chars[i];
            // The affect bit the object's frame has not taken yet is the
            // object's affectFlag in the game.
            let mut ov = obj_vec(o);
            if c.spc_char.flags & piney_battle::chara::spc_flag::AFFECT != 0 {
                ov[6] |= 8;
            }
            format!(
                "{{\"kind\":3,\"char\":{ch},\"link\":{link},\"obj\":{},\"cls\":{},\"aff\":{}}}",
                list(ov),
                list(class_vec(&o.class)),
                list([i64::from(c.affect.ty), ix(c.affect.person)])
            )
        }
        Obj::Npc(o) => format!("{{\"kind\":4,\"char\":{ch},\"link\":{link},\"obj\":{}}}", list(obj_vec(o))),
    }
}

fn out_json(o: &Out) -> Option<String> {
    Some(match *o {
        Out::Se { se, pos } => format!("[\"se\",{},{}]", se, list(pos)),
        Out::RemoveTrap { pos } => format!("[\"remove_trap\",{},103,121]", list(pos)),
        Out::TrapRemoved { pos } => format!("[\"remove_trap\",{},-1,-1]", list(pos)),
        Out::Se2d(n) => format!("[\"se2d\",{n}]"),
        Out::OpenBox { pos } => format!("[\"open_box\",{}]", list(pos)),
        Out::OpenTrapBox { pos, kind, trap } => format!("[\"open_trap_box\",{},{kind},{trap}]", list(pos)),
        Out::Crush { what, pos } => format!("[\"crush\",{what},{}]", list(pos)),
        Out::VirusCrystal { pos } => format!("[\"virus_crystal\",{}]", list(pos)),
        Out::StatueOfGod { who, pos } => format!("[\"statue_of_god\",{who},{}]", list(pos)),
        Out::DustRing { who, ofs } => format!("[\"dust_ring\",{who},{}]", list(ofs)),
        Out::SpcMessage(n) => format!("[\"spc_msg\",{n}]"),
        Out::Radiate { who, ref out } => match out {
            RadOut::LightOn(_) => format!("[\"light_on\",{who}]"),
            RadOut::LightOff => format!("[\"light_off\",{who}]"),
            RadOut::Draw(_) => format!("[\"rays\",{who}]"),
        },
        Out::GimDraw { who, layer, transparency, alpha, height, .. } => {
            format!("[\"layer\",{layer}],[\"gim_draw\",{who},{transparency},{alpha},{height}],[\"layer\",0]")
        }
        Out::IdolDraw { who, .. } => format!("[\"idol_draw\",{who}]"),
        Out::FoodDraw { who, .. } => format!("[\"food_draw\",{who}]"),
        Out::FoodVoice { kind } => format!("[\"food_voice\",{kind}]"),
        Out::SeNote { se, pos, note } => format!("[\"se_note\",{se},{},{note}]", list(pos)),
        Out::SymbolLight { who, pos, .. } => format!("[\"symbol_light\",{who},{}]", i32::from(pos.is_some())),
        Out::SymbolFire { pattern, .. } => format!("[\"symbol_fire\",{pattern}]"),
        Out::UseSymbol { pos } => format!("[\"use_symbol\",{}]", list(pos)),
        Out::SymbolSpark { pos, .. } => format!("[\"symbol_spark\",{}]", list(pos)),
        Out::SymbolSmoke { pos, .. } => format!("[\"symbol_smoke\",{}]", list(pos)),
        Out::AreaCleared => "[\"area_cleared\"]".into(),
        Out::Destroyed { who, dest_func } => {
            if dest_func == 0 {
                return None;
            }
            format!("[\"destroyed\",{who},{dest_func}]")
        }
        Out::Weapon { who, info, n } => format!("[\"weapon\",{who},{},{n}]", info_va(info)),
        Out::Breath { who, info, .. } => format!("[\"breath\",{who},{}]", info_va(info)),
        Out::Dust { who, info, n } => format!("[\"dust\",{who},{},{n}]", info_va(info)),
        Out::Clut { who } => format!("[\"clut\",{who}]"),
        Out::Layer(l) => format!("[\"layer\",{l}]"),
        Out::CircleDraw { who, transparency } => format!("[\"circle_draw\",{who},{transparency}]"),
        Out::PartDraw { who, part, transparency, pat } => {
            format!("[\"part_draw\",{who},{part},{transparency},{pat}]")
        }
        // The table's widening is compared as the row's think values (the
        // race check); list membership as the lists.
        Out::DeleteCmnd(_) | Out::CircleParts { .. } | Out::GoldRanges { .. } => return None,
        Out::AfterDrain { who } => format!("[\"after_drain\",{who},-1]"),
        Out::EtcEffect { who, row, pos } => format!("[\"etc_effect\",{row},{who},{}]", list(pos)),
        Out::EtcDraw { who, pos, dirc, scale } => format!(
            "[\"etc_matrix\",{who},{},{},{}],[\"layer\",6],[\"etc_draw\",{who}],[\"layer\",0]",
            list(pos),
            list(dirc),
            list(scale)
        ),
        Out::EtcLights { who, lights: Some(ref ls) } => {
            let each: Vec<String> = ls
                .iter()
                .enumerate()
                .map(|(k, l)| {
                    let p = l.matrix[3];
                    format!(
                        "[\"etc_light\",{who},{k},{},{},{},{},{},{},{},{}]",
                        l.rgb, l.intensity, l.far_start, l.far_end, l.far_end2, p[0], p[1], p[2]
                    )
                })
                .collect();
            each.join(",")
        }
        Out::EtcLights { who, lights: None } => {
            format!("[\"etc_light_off\",{who},0],[\"etc_light_off\",{who},1]")
        }
        Out::Flash { t0, t1: Some(t1), colour } => format!("[\"flash2\",{t0},{t1},{colour}]"),
        Out::Flash { t0, t1: None, colour } => format!("[\"flash\",{t0},{colour}]"),
        Out::DustRingAt { pos, s, r, n, life, tex } => {
            format!("[\"dust_ring_at\",{},{s},{r},{n},{life},{tex}]", list(pos))
        }
    })
}

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

/// The scene after an operation.
fn scene_json(s: &Setup, out: &[Out], full: bool) -> String {
    let lists: Vec<String> = [Kind::Enemy, Kind::Circle, Kind::Gimmick, Kind::Npc]
        .iter()
        .map(|&k| list(s.ctrl.list(k).iter().map(|&i| i as i64)))
        .collect();
    let objs: Vec<String> = (0..s.scene.chars.len()).map(|i| obj_json(s, i, full)).collect();
    let mut calls = s.world.calls.clone();
    calls.extend(s.seam.calls.borrow().iter().cloned());
    let outs: Vec<String> = out.iter().filter_map(out_json).collect();
    // The counters, then the fountains used and fountainCount.
    let mut save: Vec<i64> = [entry::SAVE_CIRCLES, entry::SAVE_FIELDS_CLEARED, entry::SAVE_DUNGEONS_CLEARED]
        .map(|a| i64::from(s.save.i16(a)))
        .to_vec();
    save.extend((0..100).map(|k| i64::from(s.save.i32(entry::SAVE_FOUNTAIN + 4 * k))));
    save.push(i64::from(s.save.i16(piney_battle::gimetc::SAVE_FOUNTAIN_COUNT)));
    let ev: Vec<i32> = (0..6)
        .map(|k| s.save.i32(entry::SAVE_EVENT_ENTRY + 24 * s.game.field.clamp(0, 160) as usize + 4 * k))
        .collect();
    format!(
        "{{\"ctrl\":{},\"lists\":[{}],\"objs\":[{}],\"cmnd\":[{},{},{}],\"calls\":[{}],\"outs\":[{}],\"cc\":{},\
         \"rnds\":{},\"save\":{},\"ev\":{},\"ibd\":{}}}",
        list(
            [s.ctrl.area, s.ctrl.area_num, s.ctrl.floor, s.ctrl.block]
                .iter()
                .map(|&x| i64::from(x))
                .chain(s.ctrl.lists.iter().flat_map(list_vec))
        ),
        lists.join(","),
        objs.join(","),
        list(s.scene.pc_list.iter().map(|&i| i as i64)),
        list(s.scene.ene_list.iter().map(|&i| i as i64)),
        list(s.scene.obj_list.iter().map(|&i| i as i64)),
        calls.join(","),
        outs.join(","),
        cc_json(&s.cc),
        s.rnds,
        list(save),
        list(ev),
        s.world.ibd.unwrap_or(IBD_UNSET),
    )
}

/// Runs `f` with the setup's context (the particles' pattern count is the
/// harness's `ccEff::Init`'s).
fn run<R>(
    t_: &Tables,
    st: &SpawnTables,
    s: &mut Setup,
    out: &mut Vec<Out>,
    f: impl FnOnce(&mut EntryCtrl, &mut Cx, &mut ScriptSeam) -> R,
) -> R {
    let reg = s.reg.clone();
    let mut cx = Cx {
        t: t_,
        st,
        scene: &mut s.scene,
        foes: &mut s.foes,
        world: &mut s.world,
        cc: &mut s.cc,
        rnds: &mut s.rnds,
        save: &mut s.save,
        game: s.game,
        reg: &reg,
        part_pats: s.pat_num,
        out,
    };
    f(&mut s.ctrl, &mut cx, &mut s.seam)
}

/// A random field for `worldmc`: `check[40][40]` (x-major) and the
/// heights through piney-data's `WORLD::GetHeight`.
struct TestField {
    check: Vec<i8>,
    field: piney_data::field::Field,
}

impl TestField {
    fn new(check: Vec<i8>, map: Vec<u32>, check3: Vec<u8>) -> TestField {
        let params = piney_data::field::Params {
            seed: 0,
            field_type: 0,
            weather: 0,
            ground: 0,
            object: 0,
            event: 0,
            protect: false,
            skip_init: true,
        };
        let field = piney_data::field::Field {
            params,
            init_draws: 0,
            map,
            check: vec![0; 3200],
            check3,
            mnt: vec![0; 1600],
            hills: Vec::new(),
            objects: Vec::new(),
            covers: Vec::new(),
            entrance: None,
            dungeon_pos: None,
            start: [0; 2],
            start_pos: [0; 3],
            rng: piney_data::dungeon::Rng::new(0),
        };
        TestField { check, field }
    }
}

impl FieldMap for TestField {
    fn check(&self, cx: i32, cy: i32) -> i8 {
        self.check[(cx * 40 + cy) as usize]
    }
    fn height(&self, x: F, y: F) -> F {
        self.field.get_height(x, y)
    }
}

/// `EntryGimmick`'s setters as the harness answers them: each recorded.
struct GimSeam<'a> {
    calls: &'a RefCell<Vec<String>>,
}

impl EntryGimmickSeam for GimSeam<'_> {
    fn set_food(&mut self, _: &mut EntryCtrl, _: &mut Cx) {
        self.calls.borrow_mut().push("[\"SetFood\"]".into());
    }
    fn world_set_magic_circle(&mut self, _: &mut EntryCtrl, _: &mut Cx) {
        self.calls.borrow_mut().push("[\"WORLD::SetMagicCircle\"]".into());
    }
    fn set_special_obj(&mut self, _: &mut EntryCtrl, _: &mut Cx) {
        self.calls.borrow_mut().push("[\"SetSpecialObj\"]".into());
    }
    fn set_item_box(&mut self, _: &mut EntryCtrl, _: &mut Cx) {
        self.calls.borrow_mut().push("[\"SetItemBox\"]".into());
    }
    fn dungeon_set_magic_circle(&mut self, _: &mut EntryCtrl, _: &mut Cx) {
        self.calls.borrow_mut().push("[\"DUNGEON::SetMagicCircle\"]".into());
    }
    fn set_idol(&mut self, _: &mut EntryCtrl, _: &mut Cx) {
        self.calls.borrow_mut().push("[\"SetIDOL\"]".into());
    }
    fn entry_break_object(&mut self, _: &mut EntryCtrl, _: &mut Cx) {
        self.calls.borrow_mut().push("[\"EntryBreakObject\"]".into());
    }
}

fn spawn_request(t_: &Tables, st: &SpawnTables, t: &mut Toks) -> String {
    let mut s = read_scene(t_, st, t);
    let op = t.word().to_string();
    let mut out = Vec::new();
    let mut extra = String::new();
    match op.as_str() {
        "routine" => {
            let w = t.int() as usize;
            run(t_, st, &mut s, &mut out, |c, cx, _| c.run_routine(cx, w));
        }
        "check" | "entry" | "entryn" | "mc" => {
            let mut ep = read_ent(t);
            let n = if op == "entryn" { t.i32() } else { 0 };
            let r: i64 = run(t_, st, &mut s, &mut out, |c, cx, seam| match op.as_str() {
                "check" => i64::from(c.entry_object_check(cx, &mut ep)),
                "entry" => ix(c.entry_object(cx, seam, &mut ep)),
                "entryn" => ix(c.entry_object_n(cx, seam, &mut ep, n)),
                _ => c.entry_magic_circle(cx, &mut ep) as i64,
            });
            extra = format!(",\"ret\":{},\"ep\":{}", r, list(ent_vec(&ep)));
        }
        "circleobj" => {
            let w = t.int() as usize;
            let (ent, d) = {
                let o = s.ctrl.entry_obj(w).expect("a circle");
                (o.ent, o.pl_dirc)
            };
            run(t_, st, &mut s, &mut out, |c, cx, seam| c.entry_circle_object(cx, seam, &ent, d));
        }
        "enemyobj" => {
            let w = t.int() as usize;
            run(t_, st, &mut s, &mut out, |c, cx, seam| c.entry_enemy_object(cx, seam, w));
        }
        "init" => {
            let w = t.int() as usize;
            run(t_, st, &mut s, &mut out, |c, cx, _| c.init_object(cx, w));
        }
        "delete" => {
            let k = t.int();
            let w = t.int() as usize;
            run(t_, st, &mut s, &mut out, |c, cx, _| match k {
                0 => c.delete_enemy(cx, w),
                1 => c.delete_magic_circle(cx, w),
                2 => c.delete_gimmick(cx, w),
                _ => c.delete_npc(cx, w),
            });
        }
        "active" => {
            let (f, b) = (t.i32(), t.i32());
            let r = run(t_, st, &mut s, &mut out, |c, cx, _| {
                [
                    c.check_active_enemy(cx),
                    i32::from(c.check_active_object(cx)),
                    i32::from(c.check_active_object_at(cx, f, b)),
                ]
            });
            extra = format!(",\"ret\":{}", list(r));
        }
        "cmain" => {
            let w = t.int() as usize;
            let r = run(t_, st, &mut s, &mut out, |c, cx, seam| c.circle_main(cx, seam, w));
            extra = format!(",\"ret\":{}", u8::from(r));
        }
        "race" => {
            let race = t.i32();
            let ep = read_ent(t);
            let who = s.scene.chars.len();
            let made = run(t_, st, &mut s, &mut out, |_, cx, _| races::construct(cx, race, &ep, who));
            let (ch, e) = made.expect("a ported race");
            let row = &t_.enemies[ep.id as usize].think;
            let wide = out.iter().any(|o| matches!(o, Out::GoldRanges { ene_id } if *ene_id == ep.id));
            let th = if wide { [geom::k(50000.0); 3] } else { [row.area, row.territory, row.view_range] };
            extra = format!(",\"think\":{}", list(th));
            s.scene.chars.push(ch);
            s.foes.push(Some(e));
            s.ctrl.objs.push(Obj::Enemy);
            s.ctrl.links.push(Link::default());
        }
        "frame" => {
            let n = t.int();
            let path: Vec<[u32; 2]> = (0..n).map(|_| [t.u32(), t.u32()]).collect();
            let mut frames = Vec::new();
            for p in path {
                s.world.k = p;
                s.scene.chars[0].pos[0] = p[0];
                s.scene.chars[0].pos[1] = p[1];
                run(t_, st, &mut s, &mut out, |c, cx, seam| c.frame(cx, seam));
                frames.push(scene_json(&s, &out, false));
                s.world.calls.clear();
                s.seam.calls.borrow_mut().clear();
                out.clear();
            }
            return format!("{{\"frames\":[{}],\"last\":{}}}", frames.join(","), scene_json(&s, &out, true));
        }
        "fenemies" => return enemies_request(t_, st, &mut s, t),
        "worldmc" => {
            let check: Vec<i8> = (0..1600).map(|_| t.int() as i8).collect();
            let map: Vec<u32> = (0..6400).map(|_| t.u32()).collect();
            let check3: Vec<u8> = (0..1600).map(|_| t.int() as u8).collect();
            let field = TestField::new(check, map, check3);
            let mut rng = piney_data::dungeon::Rng { seed: t.u32(), count: 0 };
            let ofs = t.i32();
            let event_area = t.i32();
            let start = v4(t);
            run(t_, st, &mut s, &mut out, |c, cx, seam| {
                entry::world_set_magic_circle(c, cx, seam, &field, &mut rng, ofs, event_area, start)
            });
            extra = format!(",\"seed\":{}", rng.seed);
        }
        "fieldgims" => {
            let mut rng = piney_data::dungeon::Rng { seed: t.u32(), count: 0 };
            let mut g =
                entry::FieldGims { water: t.i32(), event_area: t.i32(), field_type: t.i32(), ..Default::default() };
            g.in_point = [v4(t), v4(t)];
            let objs = |t: &mut Toks| -> Vec<entry::FieldObj> {
                (0..t.int())
                    .map(|_| {
                        let wp = v4(t);
                        let anm = t.int() != 0;
                        let food = (0..t.int()).map(|_| v4(t)).collect();
                        let symb = (0..t.int()).map(|_| v4(t)).collect();
                        entry::FieldObj { wp, anm, food, symb }
                    })
                    .collect()
            };
            g.fobj2 = objs(t);
            g.fobj = objs(t);
            run(t_, st, &mut s, &mut out, |c, cx, seam| {
                entry::world_set_food(c, cx, seam, &g, &mut rng);
                entry::world_set_special_obj(c, cx, seam, &g, &mut rng);
            });
            extra = format!(",\"seed\":{}", rng.seed);
        }
        "dungeonmc" => {
            let mut d = DungeonGims { dtype: t.i32(), story: t.int() != 0, counter: t.i32(), ..DungeonGims::default() };
            d.slots = (0..t.int())
                .map(|_| GimSlot {
                    pos: v4(t),
                    floor: t.int() as i8,
                    block: t.int() as i8,
                    kind: t.int() as i8,
                    ..GimSlot::default()
                })
                .collect();
            let ne = t.int();
            if ne >= 0 {
                d.edit = Some(
                    (0..ne)
                        .map(|_| EditGim {
                            floor: t.i32(),
                            block: t.i32(),
                            x: t.i32(),
                            y: t.i32(),
                            ty: t.i32(),
                            ..EditGim::default()
                        })
                        .collect(),
                );
            }
            run(t_, st, &mut s, &mut out, |c, cx, seam| entry::dungeon_set_magic_circle(c, cx, seam, &mut d));
            extra = format!(",\"counter\":{},\"kinds\":{}", d.counter, list(d.slots.iter().map(|x| i64::from(x.kind))));
        }
        "eventmc" => {
            let kite = v4(t);
            let positions: Vec<EvPos> = (0..16)
                .map(|_| EvPos { floor: t.i16(), block: t.i16(), num: t.i32(), dirc: t.u32(), pos: v4(t) })
                .collect();
            let mcs: Vec<[i16; 4]> = (0..16).map(|_| std::array::from_fn(|_| t.i16())).collect();
            let game = s.game;
            let mut eps = Vec::new();
            run(t_, st, &mut s, &mut out, |c, cx, seam| {
                for e in mcs.iter().filter(|e| e[0] != -1) {
                    if let Some(mut ep) = entry::event_magic_circle(*e, &positions, kite, &game) {
                        eps.push(list(ent_vec(&ep)));
                        c.entry_object(cx, seam, &mut ep);
                    }
                }
            });
            let _ = eps;
        }
        "evenemy" => {
            let kite = v4(t);
            let positions: Vec<EvPos> = (0..16)
                .map(|_| EvPos { floor: t.i16(), block: t.i16(), num: t.i32(), dirc: t.u32(), pos: v4(t) })
                .collect();
            let e: [i16; 4] = std::array::from_fn(|_| t.i16());
            let game = s.game;
            let (mut ep, n) = entry::event_enemy(e, &positions, kite, &game);
            run(t_, st, &mut s, &mut out, |c, cx, seam| {
                c.entry_object_n(cx, seam, &mut ep, n);
            });
        }
        "entrygim" => {
            let mut wm =
                WorldMan { flag: t.i32(), entry_flag: std::array::from_fn(|_| t.i32()), event_area_flag: t.i32() };
            let calls = RefCell::new(Vec::new());
            run(t_, st, &mut s, &mut out, |c, cx, _| {
                let mut g = GimSeam { calls: &calls };
                entry::entry_gimmick(&mut wm, c, cx, &mut g);
            });
            s.seam.calls.borrow_mut().extend(calls.into_inner());
            extra = format!(",\"wm\":{}", list(wm.entry_flag));
        }
        "itembox" | "idol" => {
            let mut d = read_gims(t);
            let seed = t.u32();
            let mut rng = piney_data::dungeon::Rng { seed, count: 0 };
            run(t_, st, &mut s, &mut out, |c, cx, seam| {
                if op == "itembox" {
                    gimmick::set_item_box(c, cx, seam, &mut d, &mut |n| rng.below(n));
                } else {
                    gimmick::set_idol(c, cx, seam, &mut d);
                }
            });
            extra = format!(
                ",\"counter\":{},\"kinds\":{},\"seed\":{}",
                d.counter,
                list(d.slots.iter().map(|x| i64::from(x.kind))),
                rng.seed
            );
        }
        "gframe" => return gframe_request(t_, st, &mut s, t),
        "leave" => {
            let keep = t.int() != 0;
            let saved = run(t_, st, &mut s, &mut out, |c, cx, _| c.leave(cx, keep));
            extra = format!(",\"saved\":{}", list(saved.iter().flat_map(list_vec)));
        }
        "restore" => {
            let saved: [List; 4] = std::array::from_fn(|_| read_list(t));
            run(t_, st, &mut s, &mut out, |c, cx, _| c.restore_entry(cx, saved));
        }
        _ => panic!("unknown spawn op {op}"),
    }
    let j = scene_json(&s, &out, true);
    format!("{}{}}}", &j[..j.len() - 1], extra)
}

/// The player's frame the enemies' rules read: an offset on the ground.
struct KFrame([F; 2]);

impl Frame for KFrame {
    fn w2p(&self, v: [u32; 4]) -> [u32; 4] {
        [geom::sub(v[0], self.0[0]), geom::sub(v[1], self.0[1]), v[2], v[3]]
    }
    fn p2w(&self, v: [u32; 4]) -> [u32; 4] {
        [geom::add(v[0], self.0[0]), geom::add(v[1], self.0[1]), v[2], v[3]]
    }
}

/// One frame's input: the player's frame, the party's moves (index,
/// `pos`, `posP`, `condition.dead`), the events on the enemies.
struct Step {
    k: [F; 2],
    moves: Vec<(usize, V4, V4, i16)>,
    events: Vec<(usize, i64, i32, i32)>,
}

/// An event the harness applies to an enemy between frames: 0 its HP, 1
/// an affect taken (kind, first parameter), 2 a condition's timer (index,
/// value), 3 a Data Drain, 4 `freezeFlag`.
fn apply_event(s: &mut Setup, i: usize, kind: i64, a: i32, b: i32) {
    let Some(e) = s.foes.get_mut(i).and_then(Option::as_mut) else { return };
    let ch = &mut s.scene.chars[i];
    match kind {
        0 => ch.hp = a as i16,
        1 => {
            ch.spc_char.flags |= spc_flag::AFFECT;
            ch.affect.ty = a as i16;
            ch.affect.param[0] = b as i16;
        }
        2 => ch.cond[a as usize] = b as i16,
        3 => ch.spc_char.enemy_flags |= enemy_flag::DRAIN,
        _ => e.freeze_flag = a != 0,
    }
}

/// The enemy book's rows with a record: row, count, server, A, B, C.
fn kill_json(save: &SaveData) -> String {
    let mut v = Vec::new();
    for r in 0..313usize {
        let n = save.u8(enemy_ai::SAVE_ENEMY_KILL_COUNT + r);
        if n != 0 {
            let a = enemy_ai::SAVE_ENEMY_KILL_AREA + 8 * r;
            v.push(list([
                r as i64,
                i64::from(n as i8),
                i64::from(save.i16(a)),
                i64::from(save.i16(a + 2)),
                i64::from(save.i16(a + 4)),
                i64::from(save.i16(a + 6)),
            ]));
        }
    }
    format!("[{}]", v.join(","))
}

/// `fenemies ENV RUNNING MENU IDS[3] PUPPET RIDE PLAYER AREA[3] RAND NE
/// EP... NF (KX KY NM MOVE... NEV EVENT...)... COLLIDE LINE SHAKE ANIM`:
/// the entries `entryObject(ep)` makes before the first frame, then
/// `ccThEntryCtrl`'s frames with the enemies' `main` run by
/// [`EnemySeam`]; the scene after each (the entries' as frame 0), with
/// newlib's `rand()` and the enemy book.
fn enemies_request(t_: &Tables, st: &SpawnTables, s: &mut Setup, t: &mut Toks) -> String {
    s.motion = true;
    s.world.motion = true;
    let data = motion_data(t_);
    let env = read_env(t);
    let running = t.i32();
    let menu = t.int() != 0;
    let mut party = Party::default();
    for k in 0..3 {
        party.ids[k] = t.i32();
    }
    let puppet_show = t.int() != 0;
    let ride = t.int() != 0;
    let player = idx(t.int());
    let book_area: [i32; 3] = std::array::from_fn(|_| t.i32());
    let mut rand = Rand(t.int() as u64);
    let entries: Vec<EntryParam> = (0..t.int()).map(|_| read_ent(t)).collect();
    let steps: Vec<Step> = (0..t.int())
        .map(|_| {
            let k = [t.u32(), t.u32()];
            let moves = (0..t.int()).map(|_| (t.int() as usize, v4(t), v4(t), t.i16())).collect();
            let events = (0..t.int()).map(|_| (t.int() as usize, t.int(), t.i32(), t.i32())).collect();
            Step { k, moves, events }
        })
        .collect();
    s.world.collide = (0..t.int()).map(|_| (t.i32(), v4(t), t.u32())).collect();
    s.world.line = (0..t.int()).map(|_| t.u32()).collect();
    s.world.shake = (0..t.int()).map(|_| t.int() != 0).collect();
    s.world.anim = (0..t.int())
        .map(|_| {
            let r = t.i16();
            let next = t.u32();
            let notes = (0..t.int()).map(|_| Note { event: t.u32(), param: t.u32() }).collect();
            (r, next, notes)
        })
        .collect();
    let check = move |_: usize| running;
    let actx = AffectCtx { party: &party, menu, skill_check: &check, boss: None, volume: crate::probe_volume() };
    let reg = s.reg.clone();
    let mut out = Vec::new();
    let mut frames = Vec::new();
    let step = |s: &mut Setup, out: &mut Vec<Out>, rand: &mut Rand, entries: Option<&[EntryParam]>| {
        let frame = KFrame(s.world.k);
        let mut cx = Cx {
            t: t_,
            st,
            scene: &mut s.scene,
            foes: &mut s.foes,
            world: &mut s.world,
            cc: &mut s.cc,
            rnds: &mut s.rnds,
            save: &mut s.save,
            game: s.game,
            reg: &reg,
            part_pats: s.pat_num,
            out,
        };
        let mut seam = EnemySeam {
            data: &data,
            affect: &actx,
            env: &env,
            rand,
            frame: &frame,
            puppet_show,
            ride,
            player,
            book_area,
            inner: &mut s.seam,
        };
        match entries {
            None => s.ctrl.frame(&mut cx, &mut seam),
            Some(entries) => {
                for ep in entries {
                    let mut ep = *ep;
                    s.ctrl.entry_object(&mut cx, &mut seam, &mut ep);
                }
            }
        }
    };
    let frame_json = |s: &mut Setup, out: &mut Vec<Out>, rand: &Rand| {
        let j = scene_json(s, out, false);
        s.world.calls.clear();
        s.seam.calls.borrow_mut().clear();
        out.clear();
        format!("{},\"rand\":{},\"kill\":{}}}", &j[..j.len() - 1], rand.0, kill_json(&s.save))
    };
    step(s, &mut out, &mut rand, Some(&entries));
    frames.push(frame_json(s, &mut out, &rand));
    for st_ in steps {
        s.world.k = st_.k;
        for (i, pos, pos_p, dead) in st_.moves {
            let c = &mut s.scene.chars[i];
            c.pos = pos;
            c.pos_p = pos_p;
            c.cond[0] = dead;
        }
        for (i, kind, a, b) in st_.events {
            apply_event(s, i, kind, a, b);
        }
        step(s, &mut out, &mut rand, None);
        frames.push(frame_json(s, &mut out, &rand));
    }
    format!("{{\"frames\":[{}],\"last\":{}}}", frames.join(","), scene_json(s, &out, true))
}

/// `sreg N (OP ARGS)...`: `0` ccInitRegisterEnemy, `1 S T R`
/// ccRegisterEnemyList, `2 ID` ccRegisterEnemyOne, `3 S T R`
/// ccAnalyzeEnemyList (its result collected).
fn register_request(t_: &Tables, st: &SpawnTables, t: &mut Toks) -> String {
    let mut r = Register::default();
    let mut an = Vec::new();
    for _ in 0..t.int() {
        match t.int() {
            0 => r.init(),
            1 => {
                let (a, b, c) = (t.i32(), t.i32(), t.i32());
                r.register_list(t_, st, a, b, c);
            }
            2 => r.register_one(t_, t.i32()),
            _ => {
                let (a, b, c) = (t.i32(), t.i32(), t.i32());
                an.push(entry::analyze_enemy_list(t_, st, a, b, c, r.range));
            }
        }
    }
    let exist: Vec<i64> = r.exist.iter().enumerate().filter(|&(_, &e)| e != 0).map(|(i, _)| i as i64).collect();
    format!(
        "{{\"cells\":{},\"nums\":{},\"exist\":{},\"an\":{}}}",
        list(r.cells),
        list([r.enemy_num, r.drain_num, r.range]),
        list(exist),
        list(an)
    )
}

/// `evsave FIELD SERVER WORDS[6] FOUNTAINS[100] N (OP ARG)...`: op 0
/// CheckEventEntry, 1 CheckFountain (ARG the code), 2 ClearEventEntry, 3
/// deleteEventEntry (ARG `param[0]`, then `entRoot`).
fn evsave_request(t: &mut Toks) -> String {
    let field = t.i32();
    let server = t.i32();
    let mut save = SaveData::new();
    for k in 0..6 {
        save.set_i32(entry::SAVE_EVENT_ENTRY + 24 * field.clamp(0, 160) as usize + 4 * k, t.i32());
    }
    for k in 0..100 {
        save.set_i32(entry::SAVE_FOUNTAIN + 4 * k, t.i32());
    }
    let mut r = Vec::new();
    for _ in 0..t.int() {
        let op = t.int();
        let a = t.i32();
        r.push(match op {
            0 => i32::from(entry::check_event_entry(&save, field, a)),
            1 => i32::from(entry::check_fountain(&save, server, a)),
            2 => {
                entry::clear_event_entry(&mut save, field, a);
                0
            }
            _ => {
                let ep = EntryParam { ent_root: t.i32(), param: [a, -1, -1, -1], ..EntryParam::default() };
                entry::delete_event_entry(&mut save, field, &ep);
                0
            }
        });
    }
    let words: Vec<i32> =
        (0..6).map(|k| save.i32(entry::SAVE_EVENT_ENTRY + 24 * field.clamp(0, 160) as usize + 4 * k)).collect();
    format!("{{\"r\":{},\"words\":{}}}", list(r), list(words))
}

/// What the dungeon's gimmick setters read: `DTYPE STORY COUNTER LAKE
/// TIMESYM ATTR FOOD NSLOT (pos[4] dirc[4] floor room kind)... NEDIT
/// (floor room x y type kind flag direc)... (-1: no edit data) NROOM
/// (floor room type event item)... rotate[10][15]`.
fn read_gims(t: &mut Toks) -> DungeonGims {
    let mut d = DungeonGims {
        dtype: t.i32(),
        story: t.int() != 0,
        counter: t.i32(),
        lake: t.int() != 0,
        time_sym: t.i32(),
        field_attr: t.i32(),
        food: t.i32(),
        ..DungeonGims::default()
    };
    d.slots = (0..t.int())
        .map(|_| GimSlot { pos: v4(t), dirc: v4(t), floor: t.int() as i8, block: t.int() as i8, kind: t.int() as i8 })
        .collect();
    let ne = t.int();
    if ne >= 0 {
        d.edit = Some(
            (0..ne)
                .map(|_| EditGim {
                    floor: t.i32(),
                    block: t.i32(),
                    x: t.i32(),
                    y: t.i32(),
                    ty: t.i32(),
                    kind: t.i32(),
                    flag: t.i32(),
                    direc: t.i32(),
                })
                .collect(),
        );
    }
    d.rooms = (0..t.int())
        .map(|_| EditRoom { floor: t.i32(), block: t.i32(), ty: t.i32(), event: t.i32(), item: t.i32() })
        .collect();
    d.rotate = (0..10).map(|_| std::array::from_fn(|_| t.u32())).collect();
    d
}

/// `gframe ENV RAND N (kx ky)...`: `ccThEntryCtrl`'s frames with the
/// objects' own mains through [`EnemySeam`] (the boxes' and idols'
/// natively, the others the scripts'), Kite following the path; the scene
/// after each frame.
fn gframe_request(t_: &Tables, st: &SpawnTables, s: &mut Setup, t: &mut Toks) -> String {
    let data = motion_data(t_);
    let env = read_env(t);
    let mut rand = Rand(t.int() as u64);
    let n = t.int();
    let path: Vec<[u32; 2]> = (0..n).map(|_| [t.u32(), t.u32()]).collect();
    let party = Party::default();
    let check = |_: usize| 0;
    let actx = AffectCtx { party: &party, menu: true, skill_check: &check, boss: None, volume: crate::probe_volume() };
    let reg = s.reg.clone();
    let mut out = Vec::new();
    let mut frames = Vec::new();
    for p in path {
        s.world.k = p;
        s.scene.chars[0].pos[0] = p[0];
        s.scene.chars[0].pos[1] = p[1];
        {
            let frame = KFrame(s.world.k);
            let mut cx = Cx {
                t: t_,
                st,
                scene: &mut s.scene,
                foes: &mut s.foes,
                world: &mut s.world,
                cc: &mut s.cc,
                rnds: &mut s.rnds,
                save: &mut s.save,
                game: s.game,
                reg: &reg,
                part_pats: s.pat_num,
                out: &mut out,
            };
            let mut seam = EnemySeam {
                data: &data,
                affect: &actx,
                env: &env,
                rand: &mut rand,
                frame: &frame,
                puppet_show: false,
                ride: false,
                player: Some(0),
                book_area: [0; 3],
                inner: &mut s.seam,
            };
            s.ctrl.frame(&mut cx, &mut seam);
        }
        frames.push(scene_json(s, &out, true));
        s.world.calls.clear();
        s.seam.calls.borrow_mut().clear();
        out.clear();
    }
    let _ = rand;
    format!("{{\"frames\":[{}]}}", frames.join(","))
}
