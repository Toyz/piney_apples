//! `battle_probe` requests for Skeith (`piney_battle::boss`), as
//! `tools/test_battle_boss_rs.py` sends them: `bossclips` (every clip of `x11`
//! and `xeffect` as `{name: [frames, looping]}`) and `boss ...`: a scene with
//! the boss, Skeith made on it, then FRAMES frames of `Main` with scripted
//! affects on the boss before each (KIND 0 a hit of P0 from Kite, 1 the
//! protect gauge set to P0, 2 affect 13, 3 affect 21, 4 the break count set
//! to P0). One JSON line: the state after the constructor, then each frame.

use piney_battle::affect::{self, AffectCtx};
use piney_battle::boss::blur::{Blur, BlurFunc, Reflex, Statics};
use piney_battle::boss::{Boss, BossData, BossEnv, CamView, Cx, EffKind, Out, Tbl};
use piney_battle::chara::{AffectFunc, Env};
use piney_battle::enemy_ai::{Genrand, IDENTITY, World};
use piney_battle::event::Events;
use piney_battle::exp::Party;
use piney_battle::rand::Rand;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;

use crate::{Toks, read_char};

/// A clip's name to its frames and whether it loops.
type Clips = std::collections::HashMap<String, (u32, bool)>;

thread_local! {
    static DATA: std::cell::RefCell<Option<(BossData, Clips)>> = const { std::cell::RefCell::new(None) };
}

fn load(iso_path: &str) -> (BossData, Clips) {
    let mut iso = Iso::open(iso_path).expect("the ISO");
    let data = BossData::of(iso.volume().expect("the volume"));
    let arc = Archive::new(iso.read_path("DATA/DATA.BIN").expect("DATA.BIN")).expect("DATA.BIN");
    let mut clips = std::collections::HashMap::new();
    for stem in ["x11", "xeffect"] {
        let ccs = Ccs::parse(arc.inflate_named(stem).expect("the file")).expect("a CCSF");
        let scene = piney_data::scene::Scene::read(&ccs).expect("its scene");
        for a in piney_data::anim::Animation::all(&ccs, &scene).expect("its animations") {
            if let Some(name) = ccs.object_name(a.object) {
                clips.insert(name.to_string(), (a.frames, a.looping));
            }
        }
    }
    (data, clips)
}

/// Answers a command this module knows; None for any other.
pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables, iso: &str) -> Option<String> {
    DATA.with(|d| {
        if d.borrow().is_none() {
            *d.borrow_mut() = Some(load(iso));
        }
    });
    Some(match cmd {
        "bossclips" => DATA.with(|d| {
            let d = d.borrow();
            let (_, clips) = d.as_ref().unwrap();
            let mut v: Vec<_> = clips.iter().collect();
            v.sort();
            let body: Vec<String> = v.iter().map(|(k, (f, l))| format!("\"{k}\":[{f},{}]", u8::from(*l))).collect();
            format!("{{{}}}", body.join(","))
        }),
        "boss" => DATA.with(|d| {
            let d = d.borrow();
            let (data, clips) = d.as_ref().unwrap();
            run(tables, data, clips, t)
        }),
        "bossblur" => blur_main(t),
        _ => return None,
    })
}

fn v4(t: &mut Toks) -> [u32; 4] {
    std::array::from_fn(|_| t.u32())
}

fn run(tables: &Tables, data: &BossData, clips: &Clips, t: &mut Toks) -> String {
    let n = t.int() as usize;
    let mut scene = Scene::default();
    for _ in 0..n {
        let c = read_char(t);
        scene.chars.push(c);
    }
    let npc = t.int() as usize;
    scene.pc_list = (0..npc).map(|_| t.int() as usize).collect();
    let members: [Option<usize>; 3] = std::array::from_fn(|_| usize::try_from(t.int()).ok());
    let ids: [i32; 3] = std::array::from_fn(|_| t.i32());
    let me = t.int() as usize;
    let center = [t.u32(), t.u32(), t.u32(), 0x3f80_0000];
    let kpos = v4(t);
    let kdirc = v4(t);
    let mut rand = Rand(t.int() as u64);
    let mut cc = Genrand::seeded(t.u32());
    cc.mti = t.i32();
    let count0 = t.u32();
    let frames = t.int() as usize;
    let ns = t.int() as usize;
    let script: Vec<(usize, i32, i16)> = (0..ns).map(|_| (t.int() as usize, t.i32(), t.i16())).collect();
    let nm = t.int() as usize;
    let menus: Vec<(usize, i32)> = (0..nm).map(|_| (t.int() as usize, t.i32())).collect();
    let mut blur = ProbeBlur::read(t);
    for m in members.iter().flatten() {
        scene.chars[*m].affect.func = AffectFunc::None;
    }
    scene.chars[me].affect.func = AffectFunc::Boss;
    let party = Party { members, ids, num: members.iter().flatten().count() as i32 };
    let clip = |name: &str| clips.get(name).copied();
    let mut forbid = 0i16;
    // ccMenu's fields the boss writes: forbid, forbidChatExcept,
    // cursolOff, openReqNum, +0xf0, +0xf2.
    let mut menu = [0i32, 0, 0, 0, 0, 0];
    let mut lines = Vec::new();
    let menu_at = |f: usize| menus.iter().rfind(|(k, _)| *k <= f).map_or(-1, |(_, v)| *v);
    let env0 = Env { count: count0, menu_type: -1, ..Env::default() };
    let check = |_: usize| 0;
    let benv = BossEnv { t: tables, data, clips: &clip, env: &env0, game_over: false, cc: None };
    let actx =
        AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume: crate::probe_volume() };
    let mut none = |_| None;
    let mut land = |p: [u32; 4]| p[2];
    let mut cx = Cx {
        t: tables,
        data,
        scene: &mut scene,
        party: &party,
        world: World { player: members[0], frame: &IDENTITY, ..World::default() },
        env: &env0,
        actx: &actx,
        rand: &mut rand,
        cc: &mut cc,
        clips: &clip,
        collide: &mut none,
        part_hits: &mut piney_battle::boss::NoHits,
        game_over: false,
        // InitBossCamera is a no-op in the harness: no bossCam, no quake.
        boss_cam: false,
        cam: CamView::default(),
        land: &mut land,
        disc: piney_battle::boss::DiscView::default(),
        me,
        out: Vec::new(),
        ev: Events::new(),
    };
    let mut boss = Boss::new(&mut cx, kpos, kdirc, center);
    let out = std::mem::take(&mut cx.out);
    drop(cx);
    blur.apply(&out);
    lines.push(
        frame_json(&boss, &scene, me, &out, &rand, &cc)
            .replace("\"rand\":", &format!("\"menu\":[0,0,0,0,0,0],\"blur\":{},\"rand\":", blur.json())),
    );
    for f in 0..frames {
        let env = Env { count: count0 + 1 + f as u32, menu_type: menu_at(f), menu_forbid: forbid, ..Env::default() };
        let benv = BossEnv { t: tables, data, clips: &clip, env: &env, game_over: false, cc: None };
        let actx = AffectCtx {
            party: &party,
            menu: true,
            skill_check: &check,
            boss: Some(&benv),
            volume: crate::probe_volume(),
        };
        // The scripted affects, before the boss's frame.
        scene.chars[me].foe_state_mut().unwrap().boss = Some(Box::new(boss));
        for &(sf, kind, p0) in script.iter().filter(|s| s.0 == f) {
            let mut ev = Events::new();
            let kite = members[0];
            match kind {
                0 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 1, [p0, 0, 0], &mut rand, &mut ev),
                1 => scene.param_mut(me).unwrap().pp = p0,
                2 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 13, [0; 3], &mut rand, &mut ev),
                3 => affect::entry_affect(tables, &mut scene, &actx, me, kite, 21, [0; 3], &mut rand, &mut ev),
                4 => scene.param_mut(me).unwrap().pp_count = p0,
                _ => {}
            }
            let _ = sf;
        }
        boss = *scene.chars[me].foe_state_mut().unwrap().boss.take().unwrap();
        let mut out = boss.take_pending();
        // The affects' writes, then the manager's pass, then the boss's.
        blur.apply(&out);
        blur.frame();
        let pre = out.len();
        let mut cx = Cx {
            t: tables,
            data,
            scene: &mut scene,
            party: &party,
            world: World { player: members[0], frame: &IDENTITY, ..World::default() },
            env: &env,
            actx: &actx,
            rand: &mut rand,
            cc: &mut cc,
            clips: &clip,
            collide: &mut none,
            part_hits: &mut piney_battle::boss::NoHits,
            game_over: false,
            boss_cam: false,
            cam: CamView::default(),
            land: &mut land,
            disc: piney_battle::boss::DiscView::default(),
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        boss.main(&mut cx);
        out.extend(std::mem::take(&mut cx.out));
        drop(cx);
        blur.apply(&out[pre..]);
        for o in &out {
            match o {
                Out::MenuForbid { on, chat_except } => {
                    forbid = i16::from(*on);
                    menu[0] = i32::from(*on);
                    if *on {
                        if *chat_except {
                            menu[1] = 1;
                        }
                    } else {
                        menu[1] = 0;
                    }
                }
                Out::CursorOff(on) => menu[2] = i32::from(*on),
                Out::StreamMenu { stream, mask } => {
                    menu[3] = 0x104a;
                    menu[4] = *mask;
                    menu[5] = *stream;
                }
                _ => {}
            }
        }
        lines.push(frame_json(&boss, &scene, me, &out, &rand, &cc).replace(
            "\"rand\":",
            &format!(
                "\"menu\":[{},{},{},{},{},{}],\"blur\":{},\"rand\":",
                menu[0],
                menu[1],
                menu[2],
                menu[3],
                menu[4],
                menu[5],
                blur.json()
            ),
        ));
    }
    format!("[{}]", lines.join(","))
}

/// The effect manager's blur as a harness keeps it: made with the heap's
/// word at `m_exit` and the callbacks' statics the case gives, `Main` run
/// before each boss frame and the boss's writes applied after.
pub(crate) struct ProbeBlur {
    pub(crate) blur: Blur,
    pub(crate) statics: Statics,
    pub(crate) reflex: Option<Reflex>,
}

impl ProbeBlur {
    /// The case's three words: `m_exit`'s, then whether
    /// `ccBoss03BlurParamCB` has run (its `init`) and its `blurRad`.
    pub(crate) fn read(t: &mut Toks) -> ProbeBlur {
        let exit = t.u32();
        ProbeBlur::read_statics(Blur::new(exit), t)
    }

    /// `blur` with the statics' two words.
    pub(crate) fn read_statics(blur: Blur, t: &mut Toks) -> ProbeBlur {
        let init = t.int() != 0;
        let rad = t.u32();
        let statics = Statics { boss03_rad: init.then_some(rad), ..Statics::default() };
        ProbeBlur { blur, statics, reflex: None }
    }

    /// `ccBossBlur::Main`, the manager's pass.
    pub(crate) fn frame(&mut self) {
        self.reflex = self.blur.main(&mut self.statics);
    }

    /// The boss's writes this frame, in order.
    pub(crate) fn apply(&mut self, out: &[Out]) {
        for o in out {
            match o {
                Out::Blur(s) => self.blur.set(*s),
                Out::BlurSinRot(r) => self.statics.sin_rot = *r,
                _ => {}
            }
        }
    }

    /// The blur's members, the statics and the frame's reflex, as the
    /// harness reads them.
    pub(crate) fn json(&self) -> String {
        let b = &self.blur;
        let func = match b.func {
            None => "0",
            Some(BlurFunc::Boss03) => "\"boss03\"",
            Some(BlurFunc::Boss06) => "\"boss06\"",
        };
        let reflex = self.reflex.map_or("null".into(), |r| format!("[{},{},{}]", r.scale, r.turn, r.abgr));
        format!(
            "[{},{},{},{},{},{},{},{},{},{},{}]",
            b.scale,
            b.scale0,
            b.turn,
            b.turn0,
            b.abgr,
            b.abgr0,
            u8::from(b.enabled),
            b.exit,
            func,
            self.statics.boss03_rad.unwrap_or(0),
            reflex
        )
    }
}

/// `bossblur`: a blur in any state (scale, its start, turn, its start,
/// colour, its copy, enabled, `m_exit`, callback 0 or 1, then the statics
/// and the frames), `Main` run frame by frame; each frame's state.
fn blur_main(t: &mut Toks) -> String {
    let mut b = Blur::new(0);
    (b.scale, b.scale0, b.turn, b.turn0, b.abgr, b.abgr0) = (t.u32(), t.u32(), t.u32(), t.u32(), t.u32(), t.u32());
    b.enabled = t.int() != 0;
    b.exit = t.u32();
    b.func = (t.int() != 0).then_some(BlurFunc::Boss03);
    let mut p = ProbeBlur::read_statics(b, t);
    let frames = t.int();
    let lines: Vec<String> = (0..frames)
        .map(|_| {
            p.frame();
            p.json()
        })
        .collect();
    format!("[{}]", lines.join(","))
}

/// A frame's line with the blur's state put in before `"rand"`.
pub(crate) fn with_blur(line: String, blur: &ProbeBlur) -> String {
    line.replacen("\"rand\":", &format!("\"blur\":{},\"rand\":", blur.json()), 1)
}

fn tbl_num(t: Tbl) -> i32 {
    match t {
        Tbl::None => -1,
        Tbl::Normal => 0,
        Tbl::Super => 1,
        Tbl::Epitaph => 2,
    }
}

pub(crate) fn eff_num(k: EffKind) -> i32 {
    use piney_battle::boss::innis::Element;
    match k {
        EffKind::WaveShock | EffKind::WaveShockScaled { .. } => 0,
        EffKind::MagicSquare { .. } => 1,
        EffKind::ForceGenerator { .. } => 2,
        EffKind::AutoSamonRing { .. } | EffKind::AutoSamonRingOf { .. } => 3,
        EffKind::IceBreak => 4,
        EffKind::Dead => 5,
        EffKind::SamonRing { .. } => 6,
        EffKind::Missile { element: Element::Ice, .. } => 7,
        EffKind::Missile { element: Element::Lightning, .. } => 8,
        EffKind::Missile { element: Element::Blaze, .. } => 9,
        EffKind::Meteorite { .. } => 10,
        EffKind::Needle { .. } => 11,
        EffKind::LeafRing => 12,
        EffKind::MeteoSworm { .. } => 13,
        EffKind::ThunderStorm { .. } => 14,
        EffKind::RockTower { .. } => 15,
        EffKind::FinalPhotonFlash => 16,
        EffKind::Thunderbolt { .. } => 17,
        EffKind::MagicalBall => 18,
        EffKind::HellExpress => 19,
        EffKind::LaserShot => 20,
        EffKind::LightBallShock => 21,
        EffKind::SpikeRain => 22,
        EffKind::Phantom => 23,
        EffKind::Eruption => 24,
    }
}

pub(crate) fn out_json(o: &Out) -> Option<String> {
    use piney_battle::boss::innis::Gen;
    Some(match o {
        Out::Se3d { se, .. } => format!("[\"se3d\",{se}]"),
        Out::Se { se } => format!("[\"se\",{se}]"),
        Out::Se3dNote { se, note, .. } => format!("[\"se3dnote\",{se},{note}]"),
        Out::Flash { t, colour } => format!("[\"flash\",{t},{colour}]"),
        Out::Effect { id, kind, .. } => format!("[\"eff\",{},{id}]", eff_num(*kind)),
        Out::Cinema(n) => format!("[\"cinema\",{}]", n.unwrap_or(-99)),
        Out::SwitchLayer => "[\"switch_layer\"]".into(),
        Out::StageBegin { rgba, t } => format!("[\"stage_begin\",{rgba},{},{},{}]", t[0], t[1], t[2]),
        Out::StageEnd { rgba, t } => format!("[\"stage_end\",{rgba},{t}]"),
        Out::Skill(_, s) => format!("[\"skill\",{}]", s.sid),
        Out::HitMark { .. } => "[\"hitmark\"]".into(),
        Out::FlyFont { kind, n } => format!("[\"flyfont\",{kind},{n}]"),
        Out::ClearSpcCondition => "[\"clear_spc\"]".into(),
        Out::AfterImage => "[\"afterimage\"]".into(),
        Out::Shield { magic, .. } => format!("[\"shield\",{magic}]"),
        Out::Particles { which, .. } => match which {
            Gen::InisField(r) => format!("[\"particles\",\"InisField\",{r}]"),
            Gen::Tornado => "[\"particles\",\"Tornado\",0]".into(),
            Gen::Burst(r) => format!("[\"particles\",\"Burst\",{r}]"),
        },
        // The camera's quake and the reverse layer's draw: the runtime's
        // (the harness has no boss camera); the menu's writes are compared
        // as the menu's state; the rest by their effects.
        Out::Quake(_)
        | Out::DrawWave
        | Out::Reverse
        | Out::CursorOff(_)
        | Out::MenuForbid { .. }
        | Out::StreamMenu { .. }
        | Out::DeadCamera { .. }
        | Out::CamMode { .. }
        | Out::FreeCam { .. }
        | Out::CamPitch { .. }
        | Out::Blur { .. }
        | Out::DeleteCmnd => return None,
        _ => return None,
    })
}

fn frame_json(b: &Boss, scene: &Scene, me: usize, out: &[Out], rand: &Rand, cc: &Genrand) -> String {
    let ch = &scene.chars[me];
    let f = scene.param(me).unwrap();
    let outs: Vec<String> = out.iter().filter_map(out_json).collect();
    let effs: Vec<String> = b
        .effects
        .slots
        .iter()
        .enumerate()
        .filter_map(|(k, s)| s.as_ref().map(|e| format!("[{k},{},{}]", eff_num(e.kind), u8::from(e.enabled))))
        .collect();
    let party: Vec<String> = scene
        .pc_list
        .iter()
        .map(|&c| format!("[{},{},{}]", scene.chars[c].hp, scene.chars[c].cond[1], scene.chars[c].cond[0]))
        .collect();
    format!(
        "{{\"act\":[{},{},{}],\"pat\":[{},{},{},{}],\"stop\":[{},{},{},{}],\"move\":[{},{},{},{},{}],\
         \"tgt\":[{},{},{}],\"pos\":[{},{},{}],\"posp\":[{},{},{}],\"dirc\":{},\"hp\":[{},{}],\"pp\":[{},{}],\
         \"flags\":[{},{},{},{},{},{},{},{},{},{},{}],\"anm\":[\"{}\",{}],\"wave\":[\"{}\",{},{}],\
         \"tr\":[{},{}],\"center\":[{},{}],\"eff\":[{}],\"party\":[{}],\"out\":[{}],\"rand\":{},\"cc\":{}}}",
        b.act_num,
        b.act_proccess,
        b.act_count,
        tbl_num(b.pat_tbl),
        b.pat_index,
        b.pat_num,
        b.pat_mode,
        b.stop_time,
        b.stop_counter,
        b.stop,
        b.stop_count,
        b.move_spd,
        b.move_dirc,
        b.move_vector[0],
        b.move_vector[1],
        b.move_vector[2],
        ch.target_char.map_or(-1, |t| t as i64),
        b.target_dist,
        b.target_dirc,
        ch.pos[0],
        ch.pos[1],
        ch.pos[2],
        ch.pos_p[0],
        ch.pos_p[1],
        ch.pos_p[2],
        b.dirc[2],
        ch.hp,
        ch.max_hp,
        f.pp,
        f.pp_count,
        b.epitaph,
        b.cheat_hp,
        b.lock_player,
        b.erase_target,
        b.body_hit_sw,
        b.draw_sw,
        b.exit,
        b.act_forbid,
        b.reserve_forbid_menu,
        b.reserve_forbid_chat_except,
        b.anm_status,
        b.anm.clip.as_deref().unwrap_or(""),
        b.anm.frame(),
        b.anm_wave.clip.as_deref().unwrap_or(""),
        b.anm_wave.frame(),
        b.anm_wave.speed,
        b.animate.transparency,
        b.set_transparency,
        b.center_dist,
        b.center_dirc,
        effs.join(","),
        party.join(","),
        outs.join(","),
        rand.0,
        cc.mti,
    )
}
