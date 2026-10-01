//! Answers `tools/test_evchar_rs.py` (`evchar_probe ISO < requests`): the
//! characters' event commands - Kite built with a registry `bootParam`, a
//! party member placed by an event, the party instructions and the frames
//! that follow (`cameraMain`, `ccPlayer::Main`, each member's
//! `ccFellow::Main`) - as JSON lines. Requests: `start`, `marker`, `fellow`,
//! `act`, `mode`, `turn`, `face`, `ban`, `add`, `remove`, `talk`, `pad`;
//! numbers hex, floats their bits, the fields the harness's.

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_data::save::{SaveData, offset};
use piney_desktop::assets::SceneFile;
use piney_world::Rand;
use piney_world::camera::{CamPad, Camera, Scheme};
use piney_world::combat::spc::SpcRec;
use piney_world::ee::{self, V4};
use piney_world::hit::{HitModel, Hits};
use piney_world::motion::{ActEvent, PLAYER_ANIM_TBL};
use piney_world::party::{self, FaceTarget, SpcChars, Spcs};
use piney_world::player::Player;
use piney_world::town_party::{TownFrame, TownParty};

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
    player: Player,
    party: TownParty,
    spcs: Spcs,
    save: SaveData,
    camera: Camera,
    rand: Rand,
    mode: i8,
    /// `eventMng.menuBan` (+0x78c): `cameraMain`'s L2 waits while it is set.
    banned: bool,
}

fn ai_json(ai: Option<&piney_world::ai::Ai>) -> String {
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

fn kite_json(r: &Run) -> String {
    let (p, bd, a, c) = (&r.player, &r.player.body, &r.player.acts, &r.camera);
    let t = &c.tcam;
    obj(&[
        ("pos", list(&bd.pos)),
        ("dirc", list(&bd.dirc)),
        ("move", list(&bd.move_pos)),
        ("now_speed", bd.now_speed.to_string()),
        ("speed_rate", bd.speed_rate.to_string()),
        ("move_flag", b(bd.move_flag)),
        ("run_flag", b(bd.run_flag)),
        ("stop_flag", b(a.stop_flag)),
        ("restraint", b(a.restraint)),
        ("pause", b(a.pause)),
        ("act", a.act.to_string()),
        ("act_old", a.act_old.to_string()),
        ("act_cnt", a.act_cnt.to_string()),
        ("anm_flag", a.anm_flag.to_string()),
        ("react_cnt", a.react_cnt.to_string()),
        ("transfer_lag", a.transfer_lag.to_string()),
        ("walk_run_cnt", a.walk_run_cnt.to_string()),
        ("cloak", a.cloak.to_string()),
        ("transparency", p.transparency.to_string()),
        ("set_transparency", p.set_transparency.to_string()),
        ("frame_spd", a.frame_spd.to_string()),
        ("time", a.time.to_string()),
        ("attribute", p.hit_attribute.to_string()),
        ("stop_cnt", p.stop_cnt.to_string()),
        ("drawn", b(p.drawn)),
        ("disp", b(p.disp)),
        ("trans_dist", b(p.trans_dist)),
        ("no_death", b(p.no_death)),
        ("party_flag", p.party_flag.to_string()),
        ("listed", b(p.listed)),
        ("hitsw", b(p.hit_body.sw)),
        ("ai", ai_json(p.ai.as_ref())),
        ("cam_pos", list(&t.pos)),
        ("cam_view", list(&t.view)),
        ("cam_deg", format!("[{}, {}]", t.deg[0], t.deg[1])),
        ("world_screen", mat(&c.world_screen)),
    ])
}

/// Bit `n` of `ccSpcChar` +0xe0 (`pauseFlag` bit 0, `exitFlag` 10).
fn bit(w: u32, n: u32) -> String {
    b(w >> n & 1 != 0)
}

/// Member `code` (scene index `w`) of the town's party.
fn fellow_json(party: &TownParty, code: i32, w: usize) -> String {
    let c = &party.combat;
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
    let kite = SceneFile::read(&archive, "ctu1body").unwrap();
    let anims: Vec<&piney_data::anim::Animation> =
        PLAYER_ANIM_TBL.iter().map(|n| &kite.anims[kite.anim(n).unwrap()]).collect();
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
                let player = Player::build(pos, dirc, 0x41dc_0000, 0x4234_0000, 0x4320_0000, boot, &mut hits);
                let mut spcs = Spcs::new_game();
                spcs.registry[0].boot_param = boot;
                spcs.set_party();
                let mut party = TownParty::new(&mut iso, 0, &dummies).unwrap();
                party.combat.add_leader(&save, boot, player.party_flag);
                party.combat.mirror_leader(&player);
                run = Some(Run {
                    hits,
                    player,
                    party,
                    spcs,
                    save,
                    camera: Camera::new(pos, dirc, mode, Scheme::new(n(5) as i32)),
                    rand: Rand(u64::from(n(7))),
                    mode,
                    banned: false,
                });
                println!("{{}}");
            }
            Some("marker") => {
                // markerEvTbl[N] in town01, as World::marker_bits.
                let name = piney_data::tables::world::of(volume).markers()[n(1) as usize].to_string();
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
                let keep = r.rand.0;
                let who = r
                    .party
                    .build(&archive, &r.save, code, file, ee::VF0, 0, boot, pf, i, &mut r.hits, &mut r.rand)
                    .unwrap();
                r.rand.0 = keep;
                if let Some(s) = r.party.combat.crew.spc.get_mut(&who) {
                    s.cycle = 0;
                    s.atk_dellay = 0;
                    s.transfer_lag = lag;
                }
                if let Some(a) = r.party.combat.crew.ais.get_mut(&who) {
                    a.count = 0;
                    a.atk_msg_cnt = 0;
                    a.dmg_msg_cnt = 7;
                }
                r.spcs.set_party();
                // ccEntryEventMng's step for the entry.
                r.party.with_chars(&mut r.player, &mut r.spcs, &mut r.hits, |_, chars, _| {
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
                // piney_world::tasks, with cameraMain's menu_idle clear while
                // menu_ban holds (eventMng +0x78c).
                r.camera.main(&pad, r.player.body.dirc[2], !r.banned, false, &mut r.mode);
                {
                    let rand = &mut r.rand;
                    let mut next = || rand.rand();
                    let mut f = piney_world::player::Frame {
                        pad: &pad,
                        camera: &mut r.camera,
                        hits: &mut r.hits,
                        anims: &anims,
                        rand: &mut next,
                        cmnd_target: false,
                        annihilated: false,
                    };
                    r.player.main(&mut f);
                }
                if r.player.events.contains(&ActEvent::Arrived) {
                    r.hits.hit_enable(&mut r.player.hit_body);
                }
                if let Some(how) = r.player.leave.take() {
                    r.party.with_chars(&mut r.player, &mut r.spcs, &mut r.hits, |spcs, chars, _| {
                        spcs.leave(0, how, chars)
                    });
                }
                // ccThFellowNN as the World runs them.
                let mut x = TownFrame {
                    hits: &mut r.hits,
                    camera: &mut r.camera,
                    save: &mut r.save,
                    rand: &mut r.rand,
                    menu_type: -1,
                    menu_forbid: false,
                    server: 0,
                };
                r.party.frame(&mut r.player, &mut r.spcs, &mut x);
                let fellows: Vec<String> = r.party.members().map(|(id, w)| fellow_json(&r.party, id, w)).collect();
                // The character list's bodies: Kite 0, a member 0x40 + its
                // charTbl row (as the test names them).
                let body_of = |id: u32| {
                    r.party
                        .members()
                        .find(|&(_, w)| piney_world::combat::spc::body_id(w, r.party.combat.kite) == id)
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
            Some(op) => {
                let r = run.as_mut().unwrap();
                let banned = &mut r.banned;
                let ok = r.party.with_chars(&mut r.player, &mut r.spcs, &mut r.hits, |spcs, chars, hits| match op {
                    "act" => party::pc_act(spcs, chars, hits, i32::from(s16(1)), i32::from(s16(2))),
                    "mode" => {
                        spcs.pc_mode(i32::from(s16(1)), i32::from(s16(2)));
                        true
                    }
                    "turn" => party::pc_turn(spcs, chars, i32::from(s16(1)), s16(2), s16(3)),
                    "face" => match FaceTarget::of(s16(2), s16(3)) {
                        FaceTarget::Spc(code) => match spcs.get_spc(code).and_then(|id| chars.pos(id)) {
                            Some(to) => party::pc_face(spcs, chars, i32::from(s16(1)), to, s16(4)),
                            None => false,
                        },
                        FaceTarget::None => true,
                        _ => false,
                    },
                    "ban" => {
                        *banned = n(1) != 0;
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
