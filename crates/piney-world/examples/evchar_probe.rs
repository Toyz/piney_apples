//! Answers `tools/test_evchar_rs.py` (`evchar_probe ISO < requests`): the
//! characters' event commands - Kite built with a registry `bootParam`, a
//! party member placed by an event, the party instructions and the frames
//! that follow (`cameraMain`, then the battle's tasks as every area runs
//! them: `ccPlayer::Main`, each member's `ccFellow::Main`) - as JSON lines. Requests: `start`, `marker`, `fellow`,
//! `act`, `mode`, `turn`, `face`, `ban`, `add`, `remove`, `talk`, `pad`;
//! numbers hex, floats their bits, the fields the harness's.

use piney_world::camera::CamId;
use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_data::save::{SaveData, offset};
use piney_world::Rand;
use piney_world::camera::{CamPad, Camera, Scheme};
use piney_world::chara::party::{self, SpcChars, Spcs};
use piney_world::combat::spc::SpcRec;
use piney_world::combat::{Combat, NoFx, Tasks};
use piney_world::ee::{self, V4};
use piney_world::hit::{HitModel, Hits};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn mat(m: &[[u32; 4]; 4]) -> String {
    list(&m.iter().flatten().copied().collect::<Vec<_>>())
}

/// A JSON object from (key, value text) pairs.
fn obj(kv: &[(&str, String)]) -> String {
    let body: Vec<String> = kv.iter().map(|(k, v)| format!("\"{k}\": {v}")).collect();
    format!("{{{}}}", body.join(", "))
}

fn b(v: bool) -> String {
    u8::from(v).to_string()
}

struct Run {
    hits: Hits,
    combat: Combat,
    spcs: Spcs,
    save: SaveData,
    camera: Camera,
    rand: Rand,
    mode: i8,
    /// `eventMng.menuBan` (+0x78c): `cameraMain`'s L2 waits while it is set.
    banned: bool,
}

fn ai_json(ai: Option<&piney_world::chara::ai::Ai>) -> String {
    let Some(a) = ai else { return "null".into() };
    obj(&[
        ("manual", b(a.manual_sw)),
        ("talk", b(a.talk_flag)),
        ("remote_flag", b(a.remote_flag)),
        ("remote_cmd", a.remote_cmd.to_string()),
        ("g_deg", a.g_deg.to_string()),
        ("g_rot_sp", a.g_rot_sp.to_string()),
        ("mode", a.mode.to_string()),
        ("mode_old", a.mode_old.to_string()),
        ("battle_flag", a.battle_flag.to_string()),
        ("arrival", a.arrival_chat_cnt.to_string()),
        ("skill_mask", a.skill_mask.to_string()),
        ("invite", b(a.invite_flag)),
        ("detour", a.detour_cnt.to_string()),
    ])
}

/// Kite (the battle's character, as in every area) and the camera.
fn kite_json(r: &Run) -> String {
    let c = &r.combat;
    let k = c.kite.expect("built at start");
    let t = r.camera.list.cam(CamId::Field);
    let kite = fellow_json(c, 0, k);
    let cam = obj(&[
        ("cam_pos", list(&t.pos)),
        ("cam_view", list(&t.view)),
        ("cam_deg", format!("[{}, {}]", t.deg[0], t.deg[1])),
        ("world_screen", mat(&r.camera.world_screen)),
    ]);
    format!("{}, {}", &kite[..kite.len() - 1], &cam[1..])
}

/// The registry's characters (Kite and the members, the battle's) lent to
/// `f` with the collision, written back after, as the world lends them.
fn with_chars<R>(r: &mut Run, f: impl FnOnce(&mut Spcs, &mut dyn SpcChars, &mut Hits) -> R) -> R {
    let c = &r.combat;
    let td = |w: usize| c.cast.get(w).is_none_or(|a| a.trans_dist);
    let mut chars = piney_world::combat::spc::BattleChars::read(&c.scene, &c.crew, &c.members, c.kite, &td);
    let out = f(&mut r.spcs, &mut chars, &mut r.hits);
    let cast = &mut r.combat.cast;
    chars.finish(&mut r.combat.scene, &mut r.combat.crew, &mut |w, t| {
        if let Some(a) = cast.get_mut(w) {
            a.trans_dist = t;
        }
    });
    for &(_, w) in &r.combat.members {
        piney_battle::fellow::sync_in(&r.combat.scene, &mut r.combat.crew, w);
    }
    r.combat.set_party(r.spcs.party());
    out
}

/// `f` on the event's party instructions ([`piney_battle::evparty::EvParty`])
/// in the town (area 0), the registry's characters its roster.
fn ev_party<R>(r: &mut Run, f: impl FnOnce(&mut piney_battle::evparty::EvParty) -> R) -> R {
    let ids = std::array::from_fn(|i| r.spcs.registry[i].id);
    let roster = r.combat.roster(ids);
    let (out, _) = r.combat.with_ev_party(&mut r.hits, &mut r.camera, 0, &roster, &[], &|_| None, &|_| None, f);
    out
}

/// Bit `n` of `ccSpcChar` +0xe0 (`pauseFlag` bit 0, `exitFlag` 10).
fn bit(w: u32, n: u32) -> String {
    b(w >> n & 1 != 0)
}

/// Member `code` (scene index `w`) of the town's party.
fn fellow_json(c: &Combat, code: i32, w: usize) -> String {
    let ch = &c.scene.chars[w];
    let s = c.crew.spc.get(&w).copied().unwrap_or_default();
    let a = c.cast.get(w);
    let td = a.is_none_or(|a| a.trans_dist);
    let r = SpcRec::read(&c.scene, &c.crew, w, c.kite, td);
    let f = ch.spc_char.flags;
    let (anim, time, frame_spd) = a.map_or((String::new(), 0, 0), |a| {
        (a.ch.anim_name().unwrap_or("").to_string(), a.ch.play.time, a.ch.play.frame_spd)
    });
    obj(&[
        ("code", code.to_string()),
        ("pos", list(&ch.pos)),
        ("pos_p", list(&ch.pos_p)),
        ("dirc", list(&s.dirc)),
        ("move", list(&s.move_pos)),
        ("now_speed", s.now_speed.to_string()),
        ("speed_rate", s.speed_rate.to_string()),
        ("move_flag", b(r.move_flag)),
        ("run_flag", b(r.run_flag)),
        ("stop_flag", b(r.stop_flag)),
        ("restraint", b(r.restraint)),
        ("pause", bit(f, 0)),
        ("ghost", b(r.ghost)),
        ("act", r.act.to_string()),
        ("act_old", r.act_old.to_string()),
        ("act_cnt", r.act_cnt.to_string()),
        ("anm_flag", r.anm_flag.to_string()),
        ("react_cnt", s.react_cnt.to_string()),
        ("transfer_lag", r.transfer_lag.to_string()),
        ("walk_run_cnt", s.walk_run_cnt.to_string()),
        ("stop_cnt", s.stop_cnt.to_string()),
        ("cloak", r.cloak.to_string()),
        ("transparency", s.transparency.to_string()),
        ("set_transparency", s.set_transparency.to_string()),
        ("anim", format!("{anim:?}")),
        ("time", time.to_string()),
        ("frame_spd", frame_spd.to_string()),
        ("attribute", s.hit_attribute.to_string()),
        ("drawn", b(a.is_some_and(|a| a.drawn))),
        ("disp", b(r.disp)),
        ("trans_dist", b(td)),
        ("no_death", b(r.no_death)),
        ("party_flag", r.party_flag.to_string()),
        ("recall", b(r.recall)),
        ("exit", bit(f, 10)),
        ("listed", b(r.listed)),
        ("hitsw", b(s.body_hit.sw)),
        ("hitpos", list(&s.body_hit.pos)),
        ("ai", ai_json(r.ai.as_ref())),
    ])
}

fn party_json(s: &Spcs) -> String {
    let reg: Vec<String> = s
        .registry
        .iter()
        .map(|r| format!("[{}, {}, {}]", r.id as u32, r.party_flag as u32, r.boot_param as u32))
        .collect();
    obj(&[
        ("member", list(&s.member_id.map(|m| m as u32))),
        ("num", s.num.to_string()),
        ("registry", format!("[{}]", reg.join(", "))),
        ("registry_num", s.registry_num.to_string()),
    ])
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let volume = iso.volume().unwrap();
    let char_tbl = piney_world::load_char_tbl(&mut iso).unwrap();
    let kite = std::rc::Rc::new(
        piney_world::chara::body::Body::read(&archive, "ctu1body", piney_world::chara::body::TRALL).unwrap(),
    );
    let town = HitModel::read(volume, &Ccs::parse(archive.inflate_named("town01").unwrap()).unwrap()).unwrap();
    let dummies = piney_desktop::assets::SceneFile::read(&archive, "town01").unwrap();
    let mut run: Option<Run> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        let s16 = |i: usize| hex(w[i]) as i16;
        match w.first().copied() {
            Some("start") => {
                let pos = [n(1), n(2), n(3), ee::ONE];
                let dirc = [0, 0, n(4), 0];
                let mode = n(6) as i8;
                let boot = n(8) as i32;
                let mut hits = Hits::new(volume, town.clone());
                // Kite's spcParam as the test's machine has it: flags 7,
                // height 160, width 45, velocity 27.5.
                let mut save = SaveData::new();
                spc_row(&mut save, 0, 7, None, 0x4320_0000, 0x4234_0000, 0x41dc_0000);
                let mut spcs = Spcs::new_game();
                spcs.registry[0].boot_param = boot;
                spcs.set_party();
                let data = std::rc::Rc::new(piney_world::combat::BattleData::read(&mut iso).unwrap());
                let mut combat = Combat::new(data, 1);
                combat.town_nav =
                    Some(piney_world::combat::stage::TownNavData::read(volume, 0, &dummies, &mut combat.keep).unwrap());
                // ccSPC::Reboot's Kite, arriving in the town (area 0).
                combat.add_kite(&save, pos, dirc[2], true, boot, 1, kite.clone(), Vec::new(), &mut hits, 0);
                combat.set_party(spcs.party());
                run = Some(Run {
                    hits,
                    combat,
                    spcs,
                    save,
                    camera: Camera::new(volume, pos, dirc, mode, Scheme::new(n(5) as i32)),
                    rand: Rand(u64::from(n(7))),
                    mode,
                    banned: false,
                });
                println!("{{}}");
            }
            Some("marker") => {
                // markerEvTbl[N] in town01, as World::marker_bits.
                let name = piney_data::tables::world::of(volume).markers[n(1) as usize].to_string();
                let o = dummies.ccs.find_object(&name).unwrap();
                let d = &dummies.scene.dummies[&o];
                let pos: V4 = [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ee::ONE];
                let rot = d.rot.map_or(0, |r| piney_data::anim::const_radians([0, 0, r.z.to_bits()])[2]);
                println!("{}", obj(&[("pos", list(&pos)), ("rot", rot.to_string())]));
            }
            Some("fellow") => {
                let r = run.as_mut().unwrap();
                let code = n(1) as i32;
                let (boot, lag, param) = (n(2) as i32, n(3) as i16, n(4) as i16);
                let pos = [n(5), n(6), n(7), n(8)];
                let dd = n(9) as u16;
                let (vel, height, width, flags) = (n(10), n(11), n(12), n(13));
                spc_row(&mut r.save, code as usize, flags, Some(code as i16), height, width, vel);
                let i = r.spcs.entry_spc(code);
                r.spcs.registry[i as usize].boot_param = boot;
                let pf = (((r.spcs.registry[i as usize].party_flag & 7) << 5) as i8) >> 5;
                let file = &char_tbl[code as usize].1;
                // ccSPC::Reboot's build, at StartPos (the origin: not in
                // the party); the draws as the test scripts them.
                let keep = r.combat.rand.0;
                let who =
                    r.combat.build_member(&archive, &r.save, code, file, ee::VF0, 0, boot, pf, i, &mut r.hits).unwrap();
                r.combat.rand.0 = keep;
                if let Some(s) = r.combat.crew.spc.get_mut(&who) {
                    s.cycle = 0;
                    s.atk_dellay = 0;
                    s.transfer_lag = lag;
                }
                if let Some(a) = r.combat.crew.ais.get_mut(&who) {
                    a.count = 0;
                    a.atk_msg_cnt = 0;
                    a.dmg_msg_cnt = 7;
                }
                r.spcs.set_party();
                // ccEntryEventMng's step for the entry.
                with_chars(r, |_, chars, _| {
                    let mut c = chars.spc(code).unwrap();
                    *c.pos = pos;
                    c.set_dirc_z(dd);
                    if param == 5 {
                        *c.listed = true;
                    }
                });
                println!("{}", obj(&[("list", i.to_string())]));
            }
            Some("pad") => {
                let r = run.as_mut().unwrap();
                let mut pow = [0u8; 12];
                for (k, p) in pow.iter_mut().enumerate() {
                    *p = n(7 + k) as u8;
                }
                let pad = CamPad {
                    direct: n(1),
                    push: n(2),
                    pow_l: n(3) as u8,
                    dirc_l: n(4),
                    pow_r: n(5) as u8,
                    dirc_r: n(6),
                    pow,
                };
                // cameraMain, with its menu_idle clear while menu_ban holds
                // (eventMng +0x78c), then the battle's tasks as the world
                // runs them in every area.
                // A leaver's task wakes to its exit and ends (World::fellows_gone).
                let exit = piney_battle::fellow::flag::EXIT;
                let gone: Vec<(i32, usize)> = (r.combat.members.iter().copied())
                    .filter(|&(id, w)| id != 0 && r.combat.scene.chars[w].spc_char.flags & exit != 0)
                    .collect();
                for (id, w) in gone {
                    let pf = (((r.combat.scene.chars[w].party_flag & 7) << 5) as i8) >> 5;
                    with_chars(r, |spcs, chars, _| {
                        if let Some(l) = spcs.find(id) {
                            spcs.expulsion(l, pf, chars);
                        }
                    });
                    r.combat.drop_member(w, &mut r.hits);
                    r.combat.set_party(r.spcs.party());
                }
                let heading = r.combat.kite_ref().map_or(0, |k| k.spc.dirc[2]);
                r.camera.main(&pad, heading, !r.banned, false, &mut r.mode);
                r.combat.rand = r.rand;
                {
                    let mut x = Tasks {
                        hits: &mut r.hits,
                        camera: &mut r.camera,
                        pad,
                        save: &mut r.save,
                        scene: piney_world::area::Scene::init(),
                        wm: piney_world::area::WorldMan::default(),
                        menu_type: -1,
                        menu_forbid: false,
                        puppet_show: false,
                        cmnd_target: false,
                        count: 0,
                        event_area: false,
                        dungeon: [0; 4],
                        map2d: Vec::new(),
                        map2d_info: [0; 3],
                        path_map: false,
                        effect_sw: true,
                        spcs: Some(&mut r.spcs),
                        own: None,
                    };
                    r.combat.frame(&mut x, &mut NoFx);
                }
                r.combat.shows.clear();
                r.rand = r.combat.rand;
                let fellows: Vec<String> =
                    r.combat.party_members().map(|(id, w)| fellow_json(&r.combat, id, w)).collect();
                // The character list's bodies: Kite 0, a member 0x40 + its
                // charTbl row (as the test names them).
                let body_of = |id: u32| {
                    r.combat
                        .party_members()
                        .find(|&(_, w)| piney_world::combat::spc::body_id(w, r.combat.kite) == id)
                        .map_or(id, |m| 0x40 + m.0 as u32)
                };
                println!(
                    "{}",
                    obj(&[
                        ("kite", kite_json(r)),
                        ("fellows", format!("[{}]", fellows.join(", "))),
                        ("party", party_json(&r.spcs)),
                        ("chars", list(&r.hits.chars.iter().map(|c| body_of(c.id)).collect::<Vec<_>>())),
                    ])
                );
            }
            // pc_turn and pc_face: the event's party instructions as every
            // area runs them (EvParty over the battle's characters).
            Some(op @ ("turn" | "face")) => {
                let r = run.as_mut().unwrap();
                let (pc, x, y) = (s16(1), s16(2), s16(3));
                let ok = ev_party(r, |p| match op {
                    "turn" => p.pc_turn(pc, x, y),
                    _ => p.pc_face(pc, x, y, s16(4)),
                });
                println!("{}", obj(&[("ok", b(ok))]));
            }
            Some(op) => {
                let r = run.as_mut().unwrap();
                let mut ban = None;
                let ok = with_chars(r, |spcs, chars, hits| match op {
                    "act" => party::pc_act(spcs, chars, hits, i32::from(s16(1)), i32::from(s16(2))),
                    "mode" => {
                        spcs.pc_mode(i32::from(s16(1)), i32::from(s16(2)));
                        true
                    }
                    "ban" => {
                        ban = Some(n(1) != 0);
                        if n(1) != 0 {
                            party::menu_ban(spcs, chars, hits);
                        } else {
                            party::menu_clr(spcs, chars, hits, 0);
                        }
                        true
                    }
                    "add" => spcs.party_add(s16(1).into(), chars) >= 0,
                    "talk" => match chars.spc(i32::from(s16(1))).and_then(|c| c.ai) {
                        Some(ai) => {
                            ai.talk_flag = n(2) != 0;
                            ai.g_deg = n(3) as u16;
                            true
                        }
                        None => false,
                    },
                    "remove" => {
                        spcs.party_remove(s16(1).into(), chars);
                        true
                    }
                    _ => panic!("unknown request {line}"),
                });
                if let Some(on) = ban {
                    r.banned = on;
                }
                println!("{}", obj(&[("ok", b(ok))]));
            }
            None => {}
        }
    }
}

/// `spcParam[row]` in the save cleared, then `type` FLAGS, `id` (when
/// given, with maxHP 120 and maxSP 40), height, width and velocity.
fn spc_row(save: &mut SaveData, row: usize, flags: u32, code: Option<i16>, height: u32, width: u32, vel: u32) {
    let at = offset::SPC_PARAM + offset::SPC_PARAM_SIZE * row;
    save.bytes_mut()[at..at + offset::SPC_PARAM_SIZE].fill(0);
    save.set_i32(at + 8, flags as i32);
    save.set_i32(at + 0x18, height as i32);
    save.set_i32(at + 0x1c, width as i32);
    save.set_i32(at + 0xd4, vel as i32);
    if let Some(c) = code {
        save.set_i16(at + 0xc, c);
        save.set_i16(at + 0x24, 120);
        save.set_i16(at + 0x26, 40);
    }
}
