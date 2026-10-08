//! Answers `tools/test_world_rs.py` (`world_probe ISO < requests`): the
//! field's logic tasks - `cameraMain`, then `ccPlayer::Main` - on the pads it
//! is sent, and the town's pieces around them, one JSON line a request, as
//! the game's code does in eemu. Requests: `start`, `pad`, `libm`, `avoid`,
//! `move`, `land`, `weapon`, `lights`, `tagpos`, `party`, the town's (`town`,
//! `gate`), the merchants' (`merch`), the walking PCs' (`pc*`) and `gho`;
//! numbers hex, floats their bits, the fields the harness's.

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_desktop::assets::SceneFile;
use piney_world::camera::{Cam, CamPad, Camera, CameraHits, Scheme, avoid_obstacle};
use piney_world::ee::V4;
use piney_world::hit::{self, HitModel, Hits};
use piney_world::motion::PLAYER_ANIM_TBL;
use piney_world::player::{Body, MoveInput, Player, control_move};
use piney_world::{Rand, ee, tasks};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn mat(m: &[[u32; 4]; 4]) -> String {
    list(&m.iter().flatten().copied().collect::<Vec<_>>())
}

struct Run {
    hits: Hits,
    player: Player,
    camera: Camera,
    rand: Rand,
    mode: i8,
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let volume = iso.volume().unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let kite = SceneFile::read(&archive, "ctu1body").unwrap();
    let anims: Vec<&piney_data::anim::Animation> =
        PLAYER_ANIM_TBL.iter().map(|n| &kite.anims[kite.anim(n).unwrap()]).collect();
    let mut towns: Vec<Option<Vec<HitModel>>> = vec![None, None];
    let mut run: Option<Run> = None;
    let mut props = Props::default();
    let mut merchants = Merchants::default();
    let mut pcs = Pcs::default();
    // `weapon`: Kite as the field draws him, read on first use.
    let mut kite_draw: Option<piney_world::chara::Kite> = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| hex(w[i]);
        match w.first().copied() {
            Some("libm") => {
                let v = match w[1] {
                    "sinf" => piney_data::libm::sinf(n(2)),
                    "cosf" => piney_data::libm::cosf(n(2)),
                    "tanf" => piney_data::libm::tanf(n(2)),
                    "fmodf" => piney_data::libm::fmodf(n(2), n(3)),
                    _ => piney_data::libm::atan2f(n(2), n(3)),
                };
                println!("{v}");
            }
            Some("avoid") => {
                struct Plane([u32; 3]);
                impl CameraHits for Plane {
                    fn line(&mut self, _: V4, _: V4) -> Option<(V4, u32)> {
                        None
                    }
                    fn sphere(&mut self, _: V4) -> Option<V4> {
                        None
                    }
                    fn ground(&mut self, p: V4) -> u32 {
                        let [a, b, c] = self.0;
                        ee::add(ee::add(ee::mul(p[0], a), ee::mul(p[1], b)), c)
                    }
                }
                let cam = Cam { deg: [n(7) as i16, 0], rot: [n(8), 0, 0, 0], ..Cam::default() };
                let tp = [n(1), n(2), n(3), ee::ONE];
                let cp = [n(4), n(5), n(6), ee::ONE];
                let out = avoid_obstacle(volume, tp, cp, &cam, &mut Plane([n(9), n(10), n(11)]));
                println!("{}", list(&out));
            }
            Some("move") => {
                let mut body = Body {
                    dirc: [0, 0, n(8), 0],
                    speed_value: n(9),
                    speed: n(10),
                    target_count: n(11) as i16,
                    inactive: n(12) != 0,
                    ..Body::default()
                };
                let input = MoveInput {
                    pow_l: n(1) as u8,
                    dirc_l: n(2),
                    cmnd_target: n(3) != 0,
                    cam_reset_dirc: n(5),
                    cam_rot_z: n(6),
                    cam_type: n(7) as i32,
                };
                let mut reset = n(4) != 0;
                control_move(&mut body, &input, &mut reset);
                println!(
                    "[{}, {}, {}, {}, {}, {}, {}, {}, {}]",
                    body.dirc[2],
                    body.move_pos[0],
                    body.move_pos[1],
                    body.speed_rate,
                    body.now_speed,
                    u8::from(body.move_flag),
                    u8::from(body.run_flag),
                    body.target_count,
                    u8::from(reset)
                );
            }
            Some("start") => {
                let t = n(1) as usize;
                let models = towns[t].get_or_insert_with(|| {
                    let stem = if t == 0 { "town01" } else { "town01d" };
                    HitModel::read(volume, &Ccs::parse(archive.inflate_named(stem).unwrap()).unwrap()).unwrap()
                });
                let pos = [n(2), n(3), n(4), ee::ONE];
                let dirc = [0, 0, n(5), 0];
                let scheme = Scheme::new(n(6) as i32);
                let mode = n(7) as i8;
                run = Some(Run {
                    hits: Hits::new(volume, models.clone()),
                    player: Player { volume, ..Player::new(pos, dirc, 0x41dc_0000, 0x4234_0000, 0x4320_0000) },
                    camera: Camera { volume, ..Camera::new(pos, dirc, mode, scheme) },
                    rand: Rand(u64::from(n(8))),
                    mode,
                });
                println!("{{}}");
            }
            Some("sphere") => {
                // ccModelHitCheckQZ(off, pos, r, mask, maskType): the push
                // and the result count.
                let r = run.as_mut().unwrap();
                let mut off = [0, 0, 0, ee::ONE];
                r.hits.sphere(&mut off, [n(1), n(2), n(3), ee::ONE], n(4), n(5), n(6) as i32, true);
                println!("[{}, {}]", list(&off), r.hits.num);
            }
            Some("land") => {
                let r = run.as_mut().unwrap();
                let z = r.hits.land([n(1), n(2), n(3), ee::ONE], hit::LAND_MASK);
                println!("[{z}, {}]", r.hits.num);
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
                tasks(&mut r.camera, &mut r.player, &mut r.hits, &anims, &mut r.rand, &pad, &mut r.mode);
                let (p, b, a, c) = (&r.player, &r.player.body, &r.player.acts, &r.camera);
                let t = &c.tcam;
                println!(
                    concat!(
                        "{{\"pos\": {}, \"dirc\": {}, \"move\": {}, \"now_speed\": {}, \"speed_rate\": {}, ",
                        "\"move_flag\": {}, \"run_flag\": {}, \"stop_flag\": {}, \"restraint\": {}, ",
                        "\"act\": {}, \"act_old\": {}, \"act_cnt\": {}, \"anm_flag\": {}, \"react_cnt\": {}, ",
                        "\"transfer_lag\": {}, \"walk_run_cnt\": {}, \"cloak\": {}, \"transparency\": {}, ",
                        "\"frame_spd\": {}, \"time\": {}, \"attribute\": {}, \"stop_cnt\": {}, \"drawn\": {}, ",
                        "\"arms\": {:?}, \"target_count\": {}, \"angle\": {}, ",
                        "\"cam_pos\": {}, \"cam_view\": {}, \"cam_rot\": {}, \"cam_rot2\": {}, \"cam_rot3\": {}, ",
                        "\"cam_dist\": {}, \"cam_deg\": [{}, {}], \"cam_type\": {}, \"cam_reset_flag\": {}, ",
                        "\"cam_reset_dirc\": {}, \"resetting\": {}, \"mem_dirc_z\": {}, \"mode\": {}, ",
                        "\"world_view\": {}, \"world_screen\": {}}}"
                    ),
                    list(&b.pos),
                    list(&b.dirc),
                    list(&b.move_pos),
                    b.now_speed,
                    b.speed_rate,
                    u8::from(b.move_flag),
                    u8::from(b.run_flag),
                    u8::from(a.stop_flag),
                    u8::from(a.restraint),
                    a.act,
                    a.act_old,
                    a.act_cnt,
                    a.anm_flag,
                    a.react_cnt,
                    a.transfer_lag,
                    a.walk_run_cnt,
                    a.cloak,
                    p.transparency,
                    a.frame_spd,
                    a.time,
                    p.hit_attribute,
                    p.stop_cnt,
                    u8::from(p.drawn),
                    match p.arms {
                        Some(piney_world::arms::ArmsCall::Effect) => "effect",
                        Some(piney_world::arms::ArmsCall::Clear) => "clear",
                        None => "none",
                    },
                    b.target_count,
                    list(&p.angle),
                    list(&t.pos),
                    list(&t.view),
                    list(&t.rot),
                    list(&t.rot2),
                    list(&t.rot3),
                    t.dist,
                    t.deg[0],
                    t.deg[1],
                    t.kind,
                    u8::from(t.reset_flag),
                    t.reset_dirc,
                    u8::from(c.resetting),
                    c.mem_dirc_z,
                    r.mode,
                    mat(&c.world_view),
                    mat(&c.world_screen),
                );
            }
            // `weapon`: the weapons' matrices, for test_weapon.
            Some("weapon") => {
                let k = kite_draw.get_or_insert_with(|| piney_world::chara::Kite::read(&archive).unwrap());
                let (pos, dirc) = ([n(3), n(4), n(5), ee::ONE], [n(6), n(7), n(8), 0]);
                let worlds = k.worlds(n(1) as i16, n(2), pos, dirc);
                let (body, weapon) = (&k.file.ccs, &k.weapon.as_ref().unwrap().ccs);
                let hands: Vec<String> = k
                    .hands()
                    .iter()
                    .map(|&(h, m)| {
                        format!("[{:?}, {:?}]", body.object_name(h).unwrap(), weapon.object_name(m).unwrap())
                    })
                    .collect();
                let mats: Vec<String> = k
                    .weapons(&worlds, piney_world::chara::root(pos, dirc))
                    .iter()
                    .map(|(_, w)| list(&w.to_cols_array().map(f32::to_bits)))
                    .collect();
                println!("{{\"hands\": [{}], \"worlds\": [{}]}}", hands.join(", "), mats.join(", "));
            }
            Some(cmd) if props_command(cmd, &w, &archive, volume, &mut props) => {}
            Some(cmd) if merchant_command(cmd, &w, &archive, &iso_path, &mut merchants) => {}
            Some(cmd) if pcs_command(cmd, &w, &archive, &mut iso, &mut pcs) => {}
            // --- talking: ccSortCmnd, ccCheckTargetRange, ccSelectTarget ---
            // `target LX LY LZ DIRCZ LWIDTH EYE MODE PRI N (FLAGS WIDTH X Y Z)*N`:
            // the candidates' [dist, dirc], the sorted order, each one's
            // range, the target index (-1 none) and cmndTargetPriNum after.
            Some("target") => {
                use piney_world::entry::Kind;
                use piney_world::talk::{Cmnd, Leader, check_range, select, sort};
                let leader = Leader { volume, pos_p: [n(1), n(2), n(3), ee::ONE], dirc: [0, 0, n(4), 0], width: n(5) };
                let (eye, mode, mut pri) = (n(6) != 0, n(7) as i32, n(8) as i32);
                let count = n(9) as usize;
                let mut cands: Vec<Cmnd> = (0..count)
                    .map(|k| {
                        let b = 10 + 5 * k;
                        Cmnd::new(Kind::Npc, k as i32, n(b), n(b + 1), [n(b + 2), n(b + 3), n(b + 4), ee::ONE])
                    })
                    .collect();
                let sorted = sort(&leader, &mut cands);
                let ranges: Vec<u32> = cands.iter().map(|c| u32::from(check_range(&leader, c, eye, false))).collect();
                let t = select(&leader, &cands, &sorted, mode, &mut pri, eye, false);
                let dd: Vec<String> = cands.iter().map(|c| format!("[{}, {}]", c.dist, c.dirc)).collect();
                println!(
                    "{{\"dd\": [{}], \"sorted\": {}, \"range\": {}, \"target\": {}, \"pri\": {}}}",
                    dd.join(", "),
                    list(&sorted.iter().map(|&i| i as u32).collect::<Vec<_>>()),
                    list(&ranges),
                    t.map_or(-1, |i| i as i64),
                    pri
                );
            }
            // `lights ...`: the slots SetLightMatrix fills (above).
            Some("lights") => {
                use glam::Vec3;
                let f = |i: usize| f32::from_bits(n(i));
                let count = n(1) as usize;
                let lights = (0..count)
                    .map(|k| {
                        let b = 2 + 16 * k;
                        piney_world::town::Light {
                            kind: n(b) as i16,
                            priority: n(b + 1) as i32 as i8,
                            pos: Vec3::new(f(b + 2), f(b + 3), f(b + 4)),
                            dir: Vec3::new(f(b + 5), f(b + 6), f(b + 7)),
                            colour: Vec3::new(f(b + 8), f(b + 9), f(b + 10)),
                            intensity: f(b + 11),
                            far_start: f(b + 12),
                            far_end: f(b + 13),
                            radius: [f(b + 14), f(b + 15)],
                        }
                    })
                    .collect();
                let at = 2 + 16 * count;
                let group = piney_world::town::TownLights { ambient: Vec3::ZERO, lights, fog: None };
                let world = glam::Mat4::from_translation(Vec3::new(f(at), f(at + 1), f(at + 2)));
                let l = piney_world::chara::light_matrix(&group, world, false);
                let bits = |v: &[[f32; 3]; 3], k: f32| {
                    list(&v.iter().flatten().map(|x| (x / k).to_bits()).collect::<Vec<_>>())
                };
                println!(
                    "{{\"dirs\": {}, \"colours\": {}}}",
                    bits(&l.dirs, 1.0),
                    bits(&l.colours, piney_world::chara::VU_LIGHT_SCALE)
                );
            }
            // `dirc AX AY BX BY`: ccGetDirc's heading from A to B as RAD2DEG.
            Some("dirc") => {
                let d = piney_world::event::dirc_to([n(1), n(2), 0, ee::ONE], [n(3), n(4), 0, ee::ONE]);
                println!("{d}");
            }
            // `tagpos M.. P.. O.. MODE`: ccCalcTagPosChar with world_screen M, the
            // character at P and offset O (x and y 0x7fffffff when ccCalcTagPos left
            // them). `party ...`: the party manager and the registry, then N calls (1
            // `AddMember`, 2 `DelMember`, 3 `expulsionSpc`) on party.rs's `Spcs`.
            Some("party") => println!("{}", party_probe(&w[1..])),
            // `gtclear AREA AREAPREV DTYPE`: ccClearGtHack's rule;
            // `ghoid AREA AREAPREV FIELDTYPE DUNGEON FIELD GT`:
            // ccAddRequestFileListSpc's gateHackingOutID (null: not looked
            // at), the camera file it lists, and ccAI::ccAI's
            // arrivalChatCnt.
            Some("gtclear") => {
                let keep = piney_world::combat::gate_out::keeps_gt_hack(n(1) as i32, n(2) as i32, n(3) as i32);
                println!("{{\"keep\": {keep}}}");
            }
            Some("ghoid") => {
                use piney_world::combat::gate_out::{arrival_chat, ccs_name, hack_out_id};
                let (a, p, ft, d, f, g) = (n(1) as i32, n(2) as i32, n(3) as i32, n(4) as i32, n(5) as i32, n(6) != 0);
                let id = hack_out_id(a, p, ft, d, f, g);
                let file = id.and_then(ccs_name).map_or("null".to_string(), |s| format!("\"{s}\""));
                let id = id.map_or("null".to_string(), |i| i.to_string());
                println!("{{\"id\": {id}, \"file\": {file}, \"chat\": {}}}", arrival_chat(a, p, ft, d, g));
            }
            // `gho STATE CAMERA SET N (ENDED ACT MARKERS)xN`: GateHackingOut
            // frame by frame on a scripted ghoCam (gate_probe below).
            Some("gho") => println!("{}", gate_probe(&w[1..])),
            Some("tagpos") => {
                let m: [[u32; 4]; 4] = std::array::from_fn(|c| std::array::from_fn(|r| n(1 + 4 * c + r)));
                let got = piney_world::char::calc_tag_pos(
                    &m,
                    [n(17), n(18), n(19), ee::ONE],
                    [n(20), n(21), n(22), 0],
                    n(23) as i32,
                );
                let (r, x, y) = got.map_or((0, 0x7fff_ffff, 0x7fff_ffff), |(x, y, r)| (r, x, y));
                println!("[{r}, {x}, {y}]");
            }
            _ => println!("null"),
        }
    }
}

// --- Mac Anu's props: ROOTTOWN01::Draw's choices ----------------------------
//
// `town CRISIS X Y Z`: `Town::select` for a camera eye there (the town kept
// per CRISIS and stepped by every call); `townobj CRISIS`: each static
// object's root matrix; `townreset`. `gate PX PY PZ CX CY CZ DEG1 EYE
// [CMND]`: one frame of the Chaos Gate after `chaosGateInfluence` of CMND;
// `gatereset`.

#[derive(Default)]
struct Props {
    towns: [Option<piney_world::town::Town>; 2],
    gate: Option<piney_world::gate::Gate>,
}

fn props_command(
    cmd: &str,
    w: &[&str],
    archive: &Arc<Archive>,
    volume: piney_data::volume::Volume,
    props: &mut Props,
) -> bool {
    use piney_data::volume::Volume;
    use piney_world::town::{RootTown, Town};
    use piney_world::town01::{MacAnu, Piece};
    let n = |i: usize| hex(w[i]);
    // Mac Anu as the disc has it (its town01, its tables).
    fn town<'a>(props: &'a mut Props, archive: &Arc<Archive>, volume: Volume, k: u32) -> &'a mut Town {
        props.towns[k as usize].get_or_insert_with(|| Town::open(archive, volume, 0, k == 1).unwrap())
    }
    match cmd {
        "town" => {
            let (t, m) = town(props, archive, volume, n(1)).parts_mut::<MacAnu>().unwrap();
            let pieces = m.select(t, &piney_world::town::TownView::at([n(2), n(3), n(4), ee::ONE]));
            let out: Vec<String> = pieces
                .iter()
                .map(|p| match *p {
                    Piece::Object(row) => format!("[\"obj\", {row}, {}]", t.object(row).unwrap().0),
                    Piece::Sky => "[\"sky\"]".into(),
                    Piece::CrisisSky(k) => format!("[\"crisis\", {k}]"),
                    Piece::Model(row) => {
                        let m = t.model(row).unwrap();
                        format!("[\"model\", {row}, {}]", list(&m.pos.to_array().map(f32::to_bits)))
                    }
                    Piece::Water(k) => format!("[\"water\", {k}, {}]", m.water_time(k)),
                    Piece::Map => "[\"map\"]".into(),
                })
                .collect();
            println!(
                "{{\"clip\": {}, \"uv\": {}, \"bg\": {}, \"pieces\": [{}]}}",
                list(&m.clip.map(|c| c as u32)),
                m.water_uv,
                m.bg_uv,
                out.join(", ")
            );
        }
        "townobj" => {
            let t = &town(props, archive, volume, n(1)).base;
            let rows: Vec<String> =
                (0..11).filter_map(|r| t.object(r).map(|(_, m)| format!("[{r}, {}]", mat(&m)))).collect();
            println!("[{}]", rows.join(", "));
        }
        "townreset" => {
            props.towns = [None, None];
            println!("{{}}");
        }
        "gate" => {
            if props.gate.is_none() {
                let file = town(props, archive, volume, 0).base.file.clone();
                props.gate = Some(piney_world::gate::Gate::new(archive, &file).unwrap());
            }
            let g = props.gate.as_mut().unwrap();
            if w.len() > 9 {
                g.influence(n(9) as i16);
            }
            let f = g.step(volume, [n(1), n(2), n(3), ee::ONE], [n(4), n(5), n(6), ee::ONE], n(7) as i16, n(8) != 0);
            let (bt, rt) = g.times();
            println!(
                concat!(
                    "{{\"near\": {}, \"drawn\": {}, \"ring\": {}, \"t\": {}, \"times\": [{}, {}], ",
                    "\"state\": {}, \"count\": {}, \"ended\": {}, \"sound\": {}, \"root\": {}}}"
                ),
                u8::from(f.near),
                u8::from(f.drawn),
                u8::from(f.ring),
                g.transparency,
                bt,
                rt,
                g.state,
                g.count,
                u8::from(g.ended),
                f.sound.unwrap_or(-1),
                mat(&g.root)
            );
        }
        "gatereset" => {
            props.gate = None;
            println!("{{}}");
        }
        _ => return false,
    }
    true
}

// --- Mac Anu's merchants: ccSetMerchant(0), routine and ccMerchan::main ------
//
// `merchstart PX PY PZ [TOWN [TYPE CODE MARKER]...]`: `ccEntryEventMng` in
// town TOWN (0 unless given) with Kite at P: the event entries' merchants
// (`event_merchant`), then `ccSetMerchant(0)`. `merch ...`: one frame of
// `ccThEntryCtrl` for the merchants, after the OPs on merchant K (`inf`,
// `breed`, `grot`, `fade`); each merchant's state. `merchreset`.
// `merchstatus V`: `eventStatus[52]` (from Mutation on, the Event NPC).

#[derive(Default)]
struct Merchants {
    town: Option<piney_world::town::Town>,
    list: Vec<piney_world::merchant::Merchant>,
    /// `eventStatus[52]` (`merchstatus V`): the Event NPC's.
    status: bool,
    /// Kite's body (radius 45, kind 7) on the character list after theirs
    /// while KITEHIT.
    kite: piney_world::hit::Body,
}

fn merchant_command(cmd: &str, w: &[&str], archive: &Arc<Archive>, iso_path: &str, ms: &mut Merchants) -> bool {
    use piney_world::camera::kind;
    use piney_world::char::View;
    use piney_world::entry::{Npc, NpcCtx};
    use piney_world::merchant;
    let n = |i: usize| hex(w[i]);
    let v3 = |i: usize| [n(i), n(i + 1), n(i + 2), ee::ONE];
    match cmd {
        "merchstart" => {
            let volume = Iso::open(iso_path).unwrap().volume().unwrap();
            let no = if w.len() > 4 { n(4) as i32 } else { 0 };
            let mut town = piney_world::town::Town::open(archive, volume, no, false).unwrap();
            let t = &mut town.base;
            ms.list.clear();
            for e in w.get(5..).unwrap_or_default().chunks(3) {
                let [ty, code, marker] = [0, 1, 2].map(|k| hex(e[k]) as i16);
                let row = piney_world::npc::NpcRow::of(volume, code as usize).unwrap();
                if row.event_class(ty) == Some(piney_world::npc::EventClass::Merchant) {
                    let at = piney_world::event::event_marker_in(&t.file, volume, marker);
                    let m = merchant::event_merchant(archive, &row, &mut t.hits, no, v3(1), at);
                    ms.list.push(m.unwrap());
                }
            }
            let (file, written, hits) = (&t.file, &mut t.written, &mut t.hits);
            let made = merchant::set_merchants(archive, volume, file, written, hits, no, v3(1), ms.status).unwrap();
            ms.list.extend(made);
            ms.town = Some(town);
            let out: Vec<String> = ms
                .list
                .iter()
                .map(|m| {
                    format!(
                        concat!(
                            "{{\"id\": {}, \"name\": {:?}, \"pos\": {}, \"pos_p\": {}, \"dirc\": {}, ",
                            "\"default_dirc\": {}, \"anim\": {:?}, \"hit_sw\": {}}}"
                        ),
                        m.id,
                        m.name,
                        list(&m.ch.pos),
                        list(&m.ch.pos_p),
                        list(&m.ch.dirc),
                        list(&m.default_dirc),
                        m.ch.anim_name().unwrap_or(""),
                        u8::from(m.ch.hit.sw)
                    )
                })
                .collect();
            println!("[{}]", out.join(", "));
        }
        "merch" => {
            let (player, cam_pos, view_at) = (v3(1), v3(4), v3(7));
            let (deg1, eye, kite_hit) = (n(10) as i16, n(11) != 0, n(12) != 0);
            let mut k = 13;
            while k < w.len() {
                let m = &mut ms.list[n(k + 1) as usize];
                match w[k] {
                    "inf" => m.influence(n(k + 2) as i16),
                    "breed" => m.breeder_influence(n(k + 2) as i16),
                    "grot" => {
                        (m.entry.grot_deg, m.entry.grot_spd) = (n(k + 2) as i16, n(k + 3) as i16);
                        k += 1;
                    }
                    "fade" => {
                        (m.entry.fade_flag, m.entry.fade_cnt) = (n(k + 2) as i16, n(k + 3) as i16);
                        k += 1;
                    }
                    op => panic!("merch: {op}?"),
                }
                k += 3;
            }
            let cam = Cam {
                pos: cam_pos,
                view: view_at,
                deg: [0, deg1],
                kind: if eye { kind::EYE } else { kind::FOLLOW },
                ..Cam::default()
            };
            let view = View { player, cam: cam_pos, deg1, eye };
            let mut rand = Rand(1);
            let hits = &mut ms.town.as_mut().unwrap().base.hits;
            ms.kite.pos = player;
            ms.kite.radius = 0x4234_0000;
            hits.set_hit_sw(&mut ms.kite, kite_hit);
            hits.sync(&ms.kite);
            let mut ctx = NpcCtx { player, player_dirc: [0; 4], view, cam: &cam, hits, rand: &mut rand };
            for m in &mut ms.list {
                m.status_on = ms.status;
            }
            merchant::step_all(&mut ms.list, &mut ctx);
            // A merchant whose main returned 1 is deleted from the list.
            ms.list.retain(|m| !m.gone);
            let out: Vec<String> = ms
                .list
                .iter()
                .map(|m| {
                    let (e, c, f) = (&m.entry, &m.ch, &m.frame);
                    format!(
                        concat!(
                            "{{\"pos\": {}, \"pos_p\": {}, \"dirc\": {}, \"pl_dist\": {}, \"pl_dirc\": {}, ",
                            "\"disp\": {}, \"freeze\": {}, \"cmnd\": {}, \"affect\": {}, \"listed\": {}, ",
                            "\"fade_flag\": {}, \"grot_spd\": {}, \"t\": {}, \"set_t\": {}, \"in_view\": {}, ",
                            "\"stepped\": {}, \"drawn\": {}, \"anim\": {:?}, \"time\": {}, \"act\": [{}, {}, {}], ",
                            "\"hit\": [{}, {}], \"offset\": {}, \"root\": {}}}"
                        ),
                        list(&c.pos),
                        list(&c.pos_p),
                        list(&c.dirc),
                        e.pl_dist,
                        e.pl_dirc,
                        u8::from(e.disp_sw),
                        u8::from(e.freeze),
                        u8::from(e.cmnd_flag),
                        u8::from(e.affect),
                        u8::from(e.listed),
                        e.fade_flag,
                        e.grot_spd,
                        c.transparency,
                        c.set_transparency,
                        u8::from(f.in_view),
                        u8::from(f.stepped),
                        u8::from(f.drawn),
                        c.anim_name().unwrap_or(""),
                        c.play.time,
                        m.act_num,
                        m.act_process,
                        m.anm_old,
                        m.body_hit_flag,
                        m.body_hit_cnt,
                        list(&c.hit.offset),
                        if f.stepped { mat(&m.root()) } else { "null".into() }
                    )
                })
                .collect();
            println!("[{}]", out.join(", "));
        }
        "merchreset" => {
            *ms = Merchants::default();
            println!("{{}}");
        }
        "merchstatus" => {
            ms.status = n(1) != 0;
            println!("{{}}");
        }
        _ => return false,
    }
    true
}

// --- Mac Anu's walking PCs (rtownpc.rs, navi.rs, mt.rs) and character collision
//
// `pcsel` (`ccInitRand`, `ccRegisterRandomNpc`), `pcroute`
// (`RouteSearchByMap`, `GetDestination`), `pcstart` (Kite and the town's
// walking PCs), `pchit` (`HitCheck` against scripted bodies), `pcpad` (one
// frame, then each PC's `routine` and `main`), `pcinfl` (`rTownNPCInfluence`),
// `pcadd` (`ccSetRtownPC`), `pcev` (an event instruction), `pcpos`. From
// Mutation on: `pcsel C R SERVER DONE` and `pcstart ... MANUAL SERVER DONE`
// (DONE's bits events 300, 358, 315), `pcsearch ROW STAGE` (a SEARCH PC),
// `pcrace V` (`ccSnd +0x13a`), `pcstatus I V` (`eventStatus[I]`).

struct PcRun {
    hits: Hits,
    player: Player,
    camera: Camera,
    rand: Rand,
    mode: i8,
    town: std::rc::Rc<std::cell::RefCell<piney_world::rtownpc::TownPcs>>,
    pcs: Vec<piney_world::rtownpc::RtownPc>,
    kite_char: u32,
}

#[derive(Default)]
struct Pcs {
    town01: Option<SceneFile>,
    models: Option<Vec<HitModel>>,
    run: Option<PcRun>,
}

/// The pool `pcsel` and `pcstart` draw from: on `server`, the events
/// `done`'s bits name done (300, 358, 315).
fn pc_pool(volume: piney_data::volume::Volume, server: u32, done: u32) -> piney_world::rtownpc::Pool {
    use piney_world::rtownpc::{Pool, Tables};
    let bit = |n: usize| match n {
        300 => done & 1 != 0,
        358 => done & 2 != 0,
        _ => done & 4 != 0,
    };
    Pool::with(&Tables::of(volume), server as i32, bit)
}

fn pcs_json(town: &piney_world::rtownpc::TownPcs, pcs: &[piney_world::rtownpc::RtownPc]) -> String {
    use piney_world::rtownpc::PcEvent;
    let t = format!(
        "{{\"shown\": {}, \"chatnum\": {}, \"chatcnt\": {}, \"chatmes\": {}, \"talk\": {}, \"mti\": {}}}",
        town.shown,
        town.chat_num,
        list(&town.chat_cnt.map(|c| c as u32)),
        town.chat_mes,
        u8::from(town.talk_flag),
        town.mt.mti
    );
    let p: Vec<String> = pcs
        .iter()
        .map(|pc| {
            let c = &pc.char;
            let route: Vec<u32> =
                pc.navi.route[..=(pc.navi.step.clamp(0, 47) as usize)].iter().map(|&b| u32::from(b)).collect();
            let events: Vec<String> = pc
                .events
                .iter()
                .filter_map(|e| match e {
                    PcEvent::Chat(t) => Some(format!("[\"chat\", {t:?}]")),
                    PcEvent::Transfer => Some("[\"transfer\"]".into()),
                    PcEvent::Vanish { crystal, .. } => Some(format!("[\"vanish\", {}]", u8::from(*crystal))),
                    // The steps' sounds and dust are presentation, which
                    // the probe's game side does not record.
                    PcEvent::Step { .. } => None,
                })
                .collect();
            format!(
                concat!(
                    "{{\"row\": {}, \"pos\": {}, \"posp\": {}, \"dirc\": {}, \"act\": {}, \"proc\": {}, \"cnt\": {}, ",
                    "\"poscnt\": {}, \"hitflag\": {}, \"hitcnt\": {}, \"chatcnt\": {}, \"target\": [{}, {}, {}], ",
                    "\"step\": {}, \"lm\": {}, \"route\": {}, \"next\": {}, \"old\": {}, \"trans\": {}, \"tp\": {}, ",
                    "\"stp\": {}, \"attr\": {}, \"pldist\": {}, \"pldirc\": {}, \"dist\": {}, \"disp\": {}, ",
                    "\"freeze\": {}, \"listed\": {}, \"drawn\": {}, \"anim\": {:?}, \"time\": {}, \"hitsw\": {}, ",
                    "\"hitpos\": {}, \"offset\": {}, \"mask2\": {}, \"shop\": {}, \"mdeg\": {}, \"crflag\": {}, ",
                    "\"events\": [{}]{}}}"
                ),
                pc.row.id,
                list(&c.pos),
                list(&c.pos_p),
                list(&c.dirc),
                pc.act_num,
                pc.act_process,
                pc.act_cnt,
                pc.pos_cnt,
                pc.body_hit_flag,
                pc.body_hit_cnt,
                pc.chat_cnt,
                pc.target_num,
                pc.target_num_old,
                pc.target_num_start,
                pc.navi.step,
                pc.navi.landmark,
                list(&route),
                list(&pc.next_pos),
                list(&pc.old_pos),
                pc.transrate,
                c.transparency,
                c.set_transparency,
                c.hit_attribute,
                pc.pl_dist,
                pc.pl_dirc,
                pc.dist,
                u8::from(pc.disp_sw),
                u8::from(pc.freeze),
                u8::from(pc.listed),
                u8::from(c.drawn),
                c.anim_name().unwrap_or(""),
                c.play.time,
                u8::from(c.hit.sw),
                list(&c.hit.pos),
                list(&c.hit.offset),
                c.hit.mask2,
                pc.shop_num,
                pc.merchan_deg,
                pc.change_route_flag,
                events.join(", "),
                later(pc)
            )
        })
        .collect();
    format!("\"town\": {t}, \"pcs\": [{}]", p.join(", "))
}

/// From Mutation on, a PC's fields of the later class.
fn later(pc: &piney_world::rtownpc::RtownPc) -> String {
    if pc.volume == piney_data::volume::Volume::Inf {
        return String::new();
    }
    format!(
        ", \"search\": {}, \"stage\": {}, \"glimmer\": {}, \"racehide\": {}, \"evact\": [{}, {}], \"kind\": {}",
        u8::from(pc.search),
        pc.stage,
        pc.glimmer,
        u8::from(pc.race_hide),
        pc.ev_act_num,
        pc.ev_act_process,
        pc.char.hit.kind
    )
}

fn kite_json(r: &PcRun) -> String {
    let (p, b, a, c) = (&r.player, &r.player.body, &r.player.acts, &r.camera);
    let t = &c.tcam;
    format!(
        concat!(
            "{{\"pos\": {}, \"dirc\": {}, \"move\": {}, \"now_speed\": {}, \"speed_rate\": {}, ",
            "\"move_flag\": {}, \"run_flag\": {}, \"stop_flag\": {}, \"restraint\": {}, ",
            "\"act\": {}, \"act_old\": {}, \"act_cnt\": {}, \"anm_flag\": {}, \"react_cnt\": {}, ",
            "\"transfer_lag\": {}, \"walk_run_cnt\": {}, \"cloak\": {}, \"transparency\": {}, ",
            "\"frame_spd\": {}, \"time\": {}, \"attribute\": {}, \"stop_cnt\": {}, \"drawn\": {}, ",
            "\"target_count\": {}, \"angle\": {}, \"stress\": {}, \"hitpos\": {}, \"hitsw\": {}, ",
            "\"cam_pos\": {}, \"cam_view\": {}, \"cam_rot\": {}, \"cam_dist\": {}, \"cam_deg\": [{}, {}], ",
            "\"cam_type\": {}, \"kitechar\": {}}}"
        ),
        list(&b.pos),
        list(&b.dirc),
        list(&b.move_pos),
        b.now_speed,
        b.speed_rate,
        u8::from(b.move_flag),
        u8::from(b.run_flag),
        u8::from(a.stop_flag),
        u8::from(a.restraint),
        a.act,
        a.act_old,
        a.act_cnt,
        a.anm_flag,
        a.react_cnt,
        a.transfer_lag,
        a.walk_run_cnt,
        a.cloak,
        p.transparency,
        a.frame_spd,
        a.time,
        p.hit_attribute,
        p.stop_cnt,
        u8::from(p.drawn),
        b.target_count,
        list(&p.angle),
        p.stress,
        list(&p.hit_body.pos),
        u8::from(p.hit_body.sw),
        list(&t.pos),
        list(&t.view),
        list(&t.rot),
        t.dist,
        t.deg[0],
        t.deg[1],
        t.kind,
        r.kite_char,
    )
}

fn pcs_command(cmd: &str, w: &[&str], archive: &Arc<Archive>, iso: &mut Iso, pcs: &mut Pcs) -> bool {
    use piney_world::mt::Mt;
    use piney_world::navi::{Navi, NaviMap};
    use piney_world::rtownpc;
    let n = |i: usize| hex(w[i]);
    let ours = ["pcsel", "pcroute", "pcstart", "pcpad", "pchit", "pcinfl", "pcadd", "pcev", "pcpos"];
    if !ours.contains(&cmd) && !matches!(cmd, "pcsearch" | "pcrace" | "pcstatus" | "pcsearchat") {
        return false;
    }
    let volume = iso.volume().unwrap();
    if pcs.town01.is_none() {
        pcs.town01 = Some(SceneFile::read(archive, "town01").unwrap());
        let c = Ccs::parse(archive.inflate_named("town01").unwrap()).unwrap();
        pcs.models = Some(HitModel::read(volume, &c).unwrap());
    }
    let town01 = pcs.town01.as_ref().unwrap();
    match cmd {
        "pcsel" => {
            let mut mt = Mt::init(n(1));
            let pool = if w.len() > 4 { pc_pool(volume, n(3), n(4)) } else { rtownpc::Pool::default() };
            let rows = rtownpc::register_random_npc(&mut mt, n(2) as i32, &pool);
            let next = mt.rand();
            println!("{{\"rows\": {}, \"next\": {}}}", list(&rows.map(|r| i32::from(r) as u32)), next as u32);
        }
        "pcroute" => {
            let map = NaviMap::read(volume, 0, town01).unwrap();
            let mut hits = Hits::new(volume, pcs.models.clone().unwrap());
            let (s, g) = ([n(1), n(2), n(3), ee::ONE], [n(4), n(5), n(6), ee::ONE]);
            let mut navi = Navi::default();
            let ret = navi.route_search(&map, s, g, &mut hits);
            let mut dest = [0u32; 4];
            navi.destination(&map, &mut dest);
            let route: Vec<u32> =
                navi.route[..=(navi.step.clamp(0, 47) as usize)].iter().map(|&b| u32::from(b)).collect();
            println!(
                concat!(
                    "{{\"ret\": {}, \"route\": {}, \"step\": {}, \"landmark\": {}, \"name\": {}, \"dist\": {}, ",
                    "\"dirc\": {}, \"dest\": {}, \"near\": [{}, {}]}}"
                ),
                u8::from(ret),
                list(&route),
                navi.step,
                navi.landmark,
                navi.name,
                navi.dist,
                navi.dirc,
                list(&dest),
                map.search_near(s),
                map.search_near(g)
            );
        }
        "pcstart" => {
            let pos = [n(3), n(4), n(5), ee::ONE];
            let dirc = [0, 0, n(6), 0];
            let mode = n(8) as i8;
            let mut hits = Hits::new(volume, pcs.models.clone().unwrap());
            let mut player = Player::new(pos, dirc, 0x41dc_0000, 0x4234_0000, 0x4320_0000);
            player.hit_body.manual = w.len() > 10 && n(10) != 0;
            player.volume = volume;
            let mut camera = Camera::new(pos, dirc, mode, Scheme::new(n(7) as i32));
            camera.volume = volume;
            let pool = if w.len() > 12 { pc_pool(volume, n(11), n(12)) } else { rtownpc::Pool::default() };
            let (count, reserved) = (n(1), n(2) as i32);
            let (town, list_) =
                rtownpc::enter(archive, volume, town01, 0, count, reserved, &pool, pos, &mut hits).unwrap();
            let r = PcRun { hits, player, camera, rand: Rand(u64::from(n(9))), mode, town, pcs: list_, kite_char: 0 };
            println!("{{{}}}", pcs_json(&r.town.borrow(), &r.pcs));
            pcs.run = Some(r);
        }
        "pchit" => {
            let mut hits = Hits::new(volume, pcs.models.clone().unwrap());
            let count = n(10) as usize;
            for k in 0..count {
                let a = 11 + 5 * k;
                let mut b = hit::Body {
                    pos: [n(a), n(a + 1), n(a + 2), ee::ONE],
                    radius: n(a + 3),
                    kind: n(a + 4),
                    id: 1 + k as u32,
                    ..hit::Body::default()
                };
                hits.hit_enable(&mut b);
            }
            let mut body = hit::Body { manual: n(1) != 0, ..hit::Body::default() };
            hits.hit_enable(&mut body);
            let pos = [n(2), n(3), n(4), ee::ONE];
            let mv = [n(5), n(6), 0, ee::ONE];
            let (r, mv) = hits.hit_check(&mut body, pos, mv, n(7), n(8), n(9) != 0);
            println!(
                "{{\"r\": {r}, \"move\": {}, \"hitpos\": {}, \"offset\": {}, \"chartype\": {}}}",
                list(&mv),
                list(&body.pos),
                list(&body.offset),
                hits.char_type
            );
        }
        "pcpos" => {
            let r = pcs.run.as_mut().unwrap();
            r.pcs[n(1) as usize].char.pos = [n(2), n(3), n(4), n(5)];
            println!("{{}}");
        }
        "pcinfl" => {
            let r = pcs.run.as_mut().unwrap();
            r.pcs[n(1) as usize].influence(n(2) as i16);
            println!("{{}}");
        }
        "pcadd" => {
            let r = pcs.run.as_mut().unwrap();
            let row = piney_world::npc::NpcRow::of(volume, n(1) as usize).unwrap();
            let model = {
                let t = r.town.borrow();
                let mut files = rtownpc::Files::new();
                rtownpc::load_model(archive, &mut files, &t.tables, &row).unwrap()
            };
            let pos = r.player.body.pos;
            let pc = rtownpc::RtownPc::place(&r.town, &row, &model, n(2) as i32, pos, &mut r.hits);
            r.pcs.push(pc);
            println!("{{{}}}", pcs_json(&r.town.borrow(), &r.pcs));
        }
        "pcsearch" => {
            let r = pcs.run.as_mut().unwrap();
            let (code, stage) = (n(1) as i32, n(2) as i16);
            let row = piney_world::npc::NpcRow::of(volume, code as usize).unwrap();
            let set_up = rtownpc::Search::set_up(&r.town.borrow(), town01, code, stage).unwrap();
            let model = {
                let t = r.town.borrow();
                let mut files = rtownpc::Files::new();
                rtownpc::load_model(archive, &mut files, &t.tables, &row).unwrap()
            };
            let (pos, origin) = (r.player.body.pos, rtownpc::Origin::Search(set_up));
            let pc = rtownpc::RtownPc::place_as(&r.town, &row, &model, origin, pos, &mut r.hits);
            r.pcs.push(pc);
            println!("{{{}}}", pcs_json(&r.town.borrow(), &r.pcs));
        }
        "pcsearchat" => {
            // The SEARCH set-up alone in town TOWN, Kite at (0, 0, 0, 0).
            let (no, code, stage) = (n(1) as i32, n(2) as i32, n(3) as i16);
            let mut town = piney_world::town::Town::open(archive, volume, no, false).unwrap();
            let t = rtownpc::TownPcs::new(volume, no, &town.base.file, Mt::init(0)).unwrap();
            let t = std::rc::Rc::new(std::cell::RefCell::new(t));
            let set_up = rtownpc::Search::set_up(&t.borrow(), &town.base.file, code, stage).unwrap();
            let row = piney_world::npc::NpcRow::of(volume, code as usize).unwrap();
            let model = rtownpc::load_model(archive, &mut rtownpc::Files::new(), &t.borrow().tables, &row).unwrap();
            let origin = rtownpc::Origin::Search(set_up);
            let pc = rtownpc::RtownPc::place_as(&t, &row, &model, origin, [0; 4], &mut town.base.hits);
            println!("{{{}}}", pcs_json(&t.borrow(), &[pc]));
        }
        "pcrace" => {
            pcs.run.as_mut().unwrap().town.borrow_mut().race_hold = n(1) != 0;
            println!("{{}}");
        }
        "pcstatus" => {
            pcs.run.as_mut().unwrap().town.borrow_mut().event_status[n(1) as usize] = n(2) as u8;
            println!("{{}}");
        }
        "pcev" => {
            use piney_event::host::NpcCommand;
            let r = pcs.run.as_mut().unwrap();
            let pc = &mut r.pcs[n(1) as usize];
            let (a, b, c) = (n(3) as i16, n(4) as i16, n(5) as i16);
            let npc = pc.row.id;
            let cmd = match n(2) {
                0 => NpcCommand::Act { npc, param: a },
                1 => NpcCommand::WalkPos { npc, x: a, y: b, z: c },
                2 => NpcCommand::WalkDir { npc, rot: a, dist: b },
                _ => NpcCommand::Turn { npc, dirc: a, chg: b },
            };
            println!("{{\"ok\": {}}}", u8::from(pc.command(&cmd, None, None)));
        }
        "pcpad" => {
            use piney_world::entry::{Npc, NpcCtx};
            let r = pcs.run.as_mut().unwrap();
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
            tasks(&mut r.camera, &mut r.player, &mut r.hits, &anims_of(archive), &mut r.rand, &pad, &mut r.mode);
            r.kite_char = r.hits.char_type;
            if r.player.events.contains(&piney_world::motion::ActEvent::Arrived) {
                r.hits.hit_enable(&mut r.player.hit_body);
            }
            let t = r.camera.tcam.clone();
            let view = piney_world::char::View {
                player: r.player.body.pos,
                cam: t.pos,
                deg1: t.deg[1],
                eye: t.kind == piney_world::camera::kind::EYE,
            };
            for pc in r.pcs.iter_mut() {
                let mut ctx = NpcCtx {
                    player: r.player.body.pos,
                    player_dirc: r.player.body.dirc,
                    view,
                    cam: &t,
                    hits: &mut r.hits,
                    rand: &mut r.rand,
                };
                pc.step(&mut ctx);
            }
            println!("{{\"kite\": {}, {}}}", kite_json(r), pcs_json(&r.town.borrow(), &r.pcs));
        }
        _ => return false,
    }
    true
}

/// Kite's animations by act, read once.
fn anims_of(archive: &Arc<Archive>) -> Vec<&'static piney_data::anim::Animation> {
    use std::sync::OnceLock;
    static KITE: OnceLock<SceneFile> = OnceLock::new();
    let kite = KITE.get_or_init(|| SceneFile::read(archive, "ctu1body").unwrap());
    PLAYER_ANIM_TBL.iter().map(|n| &kite.anims[kite.anim(n).unwrap()]).collect()
}

/// The `party` request ([`piney_world::party::Spcs`] against `ccParty` and
/// `ccSPC`).
fn party_probe(w: &[&str]) -> String {
    use piney_world::combat::spc::SpcRec;
    use piney_world::party::{Registry, SpcChars, Spcs};
    let mut at = 0usize;
    let mut next = || {
        let v = hex(w[at]) as i32;
        at += 1;
        v
    };
    let mut s = Spcs::new_game();
    for r in s.registry.iter_mut() {
        *r = Registry { id: next(), party_flag: next(), boot_param: 0 };
    }
    let mc: [i32; 3] = std::array::from_fn(|_| next());
    s.member_id = std::array::from_fn(|_| next());
    s.num = next();
    s.member_char = std::array::from_fn(|k| (mc[k] >= 0).then(|| s.registry[mc[k] as usize].id));
    struct Chars {
        ids: [i32; 5],
        recs: Vec<Option<SpcRec>>,
    }
    impl SpcChars for Chars {
        fn spc(&mut self, id: i32) -> Option<piney_world::ai::SpcRef<'_>> {
            let k = self.ids.iter().position(|&x| x == id)?;
            self.recs[k].as_mut().map(|r| r.spc_ref())
        }
        fn pos(&self, _: i32) -> Option<ee::V4> {
            None
        }
    }
    let rec = |pf: i32, recall: bool| SpcRec {
        who: 0,
        ai: None,
        pos: [0; 4],
        dirc: [0; 4],
        dead: 0,
        skill_id: 0,
        skill_status: 0,
        transparency: 0,
        set_transparency: 0,
        trans_dist: false,
        disp: false,
        restraint: false,
        move_flag: false,
        stop_flag: false,
        run_flag: false,
        ghost: false,
        no_death: false,
        recall,
        party_flag: pf as i8,
        act: 0,
        act_old: 0,
        anm_flag: 0,
        act_cnt: 0,
        transfer_lag: 0,
        cloak: 0,
        hit: hit::Body::default(),
        listed: false,
    };
    let mut recs = Vec::new();
    for _ in 0..5 {
        let (built, pf, recall) = (next(), next(), next());
        recs.push((built != 0).then(|| rec(pf, recall != 0)));
    }
    let mut chars = Chars { ids: std::array::from_fn(|k| s.registry[k].id), recs };
    let n = next();
    let mut ret = Vec::new();
    for _ in 0..n {
        let (op, arg) = (next(), next());
        match op {
            1 => ret.push(s.add_member(arg, &mut chars)),
            2 => {
                s.del_member(arg, &mut chars);
                ret.push(0);
            }
            _ => {
                // expulsionSpc alone (the fellow task's delete, which may
                // free the registry slot, is not the function).
                s.expulsion(arg as usize, 1, &mut chars);
                ret.push(0);
            }
        }
    }
    let mc: Vec<String> = s
        .member_char
        .iter()
        .map(|c| c.and_then(|id| s.registry.iter().position(|r| r.id == id)).map_or(-1, |k| k as i32).to_string())
        .collect();
    let reg: Vec<String> = s.registry.iter().map(|r| format!("[{}, {}]", r.id, r.party_flag)).collect();
    let cs: Vec<String> = chars
        .recs
        .iter()
        .map(|r| r.as_ref().map_or("null".to_string(), |r| format!("[{}, {}]", r.party_flag, u8::from(r.recall))))
        .collect();
    let ints = |v: &[i32]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ");
    format!(
        "{{\"ret\": [{}], \"reg\": [{}], \"mc\": [{}], \"mid\": [{}], \"num\": {}, \"chars\": [{}]}}",
        ints(&ret),
        reg.join(", "),
        mc.join(", "),
        ints(&s.member_id),
        s.num,
        cs.join(", ")
    )
}

/// The `gho` request: `ccPlayer::GateHackingOut` over a scripted `ghoCam`.
/// In: the player's state (prog, counters, flags, act, positions, camID), then
/// `tcam` and `ecam` (pos, view, rot, dist, deg0, deg1), the animation set,
/// N, and per frame: ended, act (ffffffff as it is) and the three markers'
/// x y z. Out, per frame: the same state after the call.
fn gate_probe(w: &[&str]) -> String {
    use piney_world::combat::gate_out::{GateState, GhoCam, Kite, Marker, gate_hacking_out};
    let mut at = 0usize;
    let mut next = || {
        let v = hex(w[at]);
        at += 1;
        v
    };
    struct Scripted {
        set: bool,
        ended: bool,
        markers: [V4; 3],
    }
    impl GhoCam for Scripted {
        fn start(&mut self) -> bool {
            self.set
        }
        fn forward(&mut self) -> bool {
            self.set && self.ended
        }
        fn marker(&self, m: Marker) -> Option<V4> {
            Some(self.markers[m as usize])
        }
        fn stop(&mut self) {}
    }
    let mut g = GateState::default();
    let mut prog = next() as i32;
    g.cnt = next() as i32;
    g.k224 = next();
    let mut speed_rate = next();
    g.arms_t = next();
    g.arms_disp = next() != 0;
    g.flag = next() != 0;
    let mut camera_flag = next() != 0;
    let mut flags = next();
    let mut act = next() as i32 as i16;
    let v4 = |next: &mut dyn FnMut() -> u32| -> V4 { [next(), next(), next(), next()] };
    let mut pos = v4(&mut next);
    let mut pos_cam = v4(&mut next);
    let mut pos_view = v4(&mut next);
    let mut camera = Camera::new([0; 4], [0; 4], 3, Scheme::new(0));
    camera.cam_id = next() as i32 as i16;
    for n in [1i16, 3] {
        let (p, v, r) = (v4(&mut next), v4(&mut next), v4(&mut next));
        let dist = next();
        let d = [next() as i32 as i16, next() as i32 as i16];
        let c = camera.cam_mut(n);
        (c.pos, c.view, c.rot, c.dist, c.deg) = (p, v, r, dist, d);
    }
    let mut cam = Scripted { set: next() != 0, ended: false, markers: [[0; 4]; 3] };
    let frames = next();
    let mut out = Vec::new();
    let v = |v: V4| format!("[{}, {}, {}, {}]", v[0], v[1], v[2], v[3]);
    for _ in 0..frames {
        cam.ended = next() != 0;
        let a = next();
        if a != 0xffff_ffff {
            act = a as i32 as i16;
        }
        for m in 0..3 {
            cam.markers[m] = [next(), next(), next(), ee::ONE];
        }
        let k = Kite {
            pos: &mut pos,
            flags: &mut flags,
            act: &mut act,
            speed_rate: &mut speed_rate,
            prog: &mut prog,
            camera_flag: &mut camera_flag,
            pos_cam: &mut pos_cam,
            pos_view: &mut pos_view,
        };
        gate_hacking_out(&mut g, &mut cam, &mut camera, k);
        let c = |n: i16| {
            let c = camera.cam(n);
            format!("[{}, {}, {}, {}, {}, {}]", v(c.pos), v(c.view), v(c.rot), c.dist, c.deg[0], c.deg[1])
        };
        out.push(format!(
            "[{prog}, {}, {}, {speed_rate}, {}, {}, {}, {}, {flags}, {act}, {}, {}, {}, {}, {}, {}]",
            g.cnt,
            g.k224,
            g.arms_t,
            u8::from(g.arms_disp),
            u8::from(g.flag),
            u8::from(camera_flag),
            v(pos),
            v(pos_cam),
            v(pos_view),
            camera.cam_id,
            c(1),
            c(3)
        ));
    }
    format!("[{}]", out.join(", "))
}
