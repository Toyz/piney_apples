//! Answers `tools/test_ride_rs.py`: runs `piney_battle::ride` (the riding
//! Grunty, `ccPucciguso`) on the states and scripts it is sent and prints one
//! JSON line per request (`ride_probe ISO < requests`). The commands:
//! `tables`, `new` (the constructor), `main FRAMES ...`, `fn NAME ...`
//! (`control_move`, `anim_ctrl`, `draw_pg`, `note`, `smoke`, and from
//! Mutation on `seek_move` and `seek`), `lever`
//! (`PadLeverPower`), `place` (`ccPuccigusoExit`'s place) and `adult`
//! (`ccPgAdultCheck`). `RIDE` is read by [`read_ride`], `SCRIPT` by
//! [`read_script`]; the other fields are the harness's.

use std::collections::VecDeque;
use std::io::{BufRead, Write};

use piney_battle::geom::V4;
use piney_battle::kite::{MapBounds, Pad};
use piney_battle::rand::Rand;
use piney_battle::ride::{self, Globals, Input, Out, Ride, RideAnm, RideTables, RideWorld, Seek, SeekEntry, SeekList};
use piney_battle::world::{CharHit, Note};
use piney_data::iso::Iso;

struct Toks<'a>(std::str::SplitWhitespace<'a>);

impl Toks<'_> {
    fn word(&mut self) -> &str {
        self.0.next().expect("more tokens")
    }
    fn int(&mut self) -> i64 {
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
    fn u32(&mut self) -> u32 {
        self.int() as u32
    }
    fn i32(&mut self) -> i32 {
        self.int() as i32
    }
    fn v4(&mut self) -> V4 {
        std::array::from_fn(|_| self.u32())
    }
}

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    let v: Vec<String> = v.into_iter().map(|x| x.to_string()).collect();
    format!("[{}]", v.join(","))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// A name the harness sends as hex (`-` for none), kept for the run.
fn name(t: &mut Toks) -> Option<&'static str> {
    let w = t.word();
    (w != "-").then(|| {
        let b: Vec<u8> = (0..w.len() / 2).map(|i| u8::from_str_radix(&w[2 * i..2 * i + 2], 16).unwrap()).collect();
        &*Box::leak(String::from_utf8(b).expect("ASCII").into_boxed_str())
    })
}

fn anm(a: RideAnm) -> u32 {
    match a {
        RideAnm::Kite => 0,
        RideAnm::Pg => 1,
    }
}

/// The world's answers, in order, and what the calls recorded.
#[derive(Default)]
struct Script {
    land: VecDeque<u32>,
    result: VecDeque<i64>,
    attr: VecDeque<u32>,
    collide: VecDeque<(i32, V4)>,
    line: VecDeque<u32>,
    eye: VecDeque<u32>,
    trans: VecDeque<(u32, bool)>,
    fwd: VecDeque<i16>,
    notes: VecDeque<Vec<Note>>,
    leg: VecDeque<V4>,
    draw: VecDeque<u32>,
    cam_type: i32,
    cam_id: i32,
    cam_rot: V4,
    cam_reset: bool,
    cam_reset_dirc: u32,
    last_result: Option<u32>,
    calls: Vec<String>,
    plw: Option<(V4, V4, bool)>,
    /// From Mutation on, what the search reads: the gimmicks and circles,
    /// `eventAreaNumber`, `dungeonPos[0]`, the player's place (for
    /// `ccTransPosW2P`), the map's bounds.
    gims: Vec<SeekEntry>,
    circles: Vec<SeekEntry>,
    event_area: i32,
    dungeon: V4,
    player: V4,
    bounds: MapBounds,
}

impl RideWorld for Script {
    fn land(&mut self, pos: V4, mask: u32) -> u32 {
        self.calls.push(format!("[\"ccLandHitCheck\",{},{mask}]", list(pos)));
        let r = self.result.pop_front().unwrap_or(-1);
        self.last_result = (r >= 0).then_some(r as u32);
        self.land.pop_front().unwrap_or(0)
    }
    fn hit_attribute(&mut self) -> u32 {
        self.calls.push("[\"checkHitResultAttlibute\"]".into());
        self.attr.pop_front().unwrap_or(0)
    }
    fn hit_result(&mut self) -> Option<u32> {
        self.last_result
    }
    fn line(&mut self, from: V4, to: V4, mask: u32) -> u32 {
        self.calls.push(format!("[\"ccHitCheckLM2\",{},{},{mask}]", list(from), list(to)));
        self.line.pop_front().unwrap_or(0xbf80_0000)
    }
    fn collide(&mut self, hit: &mut CharHit) -> i32 {
        self.calls.push(format!("[\"CollisionDetection\",{},{}]", list(hit.pos), hit.radius));
        let (r, off) = self.collide.pop_front().unwrap_or((0, [0; 4]));
        hit.offset = off;
        r
    }
    fn hit_switch(&mut self, hit: &mut CharHit, on: bool) {
        self.calls.push(format!("[\"SetHitSW\",{}]", u8::from(on)));
        hit.sw = on;
    }
    fn camera_type(&mut self) -> i32 {
        self.cam_type
    }
    fn camera_id(&mut self) -> i32 {
        self.cam_id
    }
    fn camera_rot(&mut self) -> V4 {
        self.cam_rot
    }
    fn camera_reset(&mut self) -> Option<u32> {
        self.cam_reset.then_some(self.cam_reset_dirc)
    }
    fn clear_camera_reset(&mut self) {
        self.cam_reset = false;
    }
    fn camera_set_eye_level(&mut self, pos_eye: V4, angle: &mut V4) {
        self.calls.push(format!("[\"cameraSetEyeLevel\",{},{}]", list(pos_eye), list(*angle)));
        angle[2] = self.eye.pop_front().unwrap_or(0);
    }
    fn camera_set_manual(&mut self, pos_view: V4) {
        self.calls.push(format!("[\"cameraSetManual\",{}]", list(pos_view)));
    }
    fn camera_set(&mut self) {
        self.calls.push("[\"cameraSet\"]".into());
    }
    fn camera_transparency(&mut self, pos: V4, width: u32, height: u32, far: u32, len: u32) -> (u32, bool) {
        self.calls.push(format!("[\"ccGetCameraTransparency\",{},{width},{height},{far},{len}]", list(pos)));
        self.trans.pop_front().unwrap_or((0x3f80_0000, false))
    }
    fn anim_set(&mut self, a: RideAnm, name: &str) {
        self.calls.push(format!("[\"SetAnm\",{},\"{name}\"]", anm(a)));
    }
    fn anim_forward(&mut self, a: RideAnm, step: u16) -> i16 {
        self.calls.push(format!("[\"AnimateForward\",{},{step}]", anm(a)));
        self.fwd.pop_front().unwrap_or(0)
    }
    fn anim_notes(&mut self, _a: RideAnm) -> Vec<Note> {
        self.notes.pop_front().unwrap_or_default()
    }
    fn leg(&mut self, name: &str) -> V4 {
        self.calls.push(format!("[\"GetObjAdrsF\",\"{name}\"]"));
        self.leg.pop_front().unwrap_or([0; 4])
    }
    fn draw(&mut self, _pos: V4, set_transparency: u32) -> u32 {
        self.calls.push(format!("[\"Draw\",{set_transparency}]"));
        self.draw.pop_front().unwrap_or(0x3f80_0000)
    }
    fn seek_list(&mut self, list: SeekList) -> Vec<SeekEntry> {
        match list {
            SeekList::Gimmicks => self.gims.clone(),
            SeekList::Circles => self.circles.clone(),
        }
    }
    fn dungeon(&mut self) -> (i32, V4) {
        (self.event_area, self.dungeon)
    }
    fn w2p(&mut self, pos: V4) -> V4 {
        piney_battle::kite::w2p_pos(&self.bounds, self.player, pos).0
    }
    fn kite_name(&mut self) -> Vec<u8> {
        b"Kite".to_vec()
    }
    fn out(&mut self, o: Out) {
        let s = match o {
            Out::AddCenter { x, y } => format!("[\"AddCenter\",{x},{y}]"),
            Out::Enter { pos } => format!("[\"Enter\",{}]", list(pos)),
            Out::Matrix { anm: a, pos, rot } => format!("[\"SetMatrix\",{},{},{}]", anm(a), list(pos), list(rot)),
            Out::Player { pos, rot, pause } => {
                self.plw = Some((pos, rot, pause));
                self.player = pos;
                return;
            }
            Out::EntryCmnd => "[\"ccEntryCmnd\"]".into(),
            Out::Chat(text) => {
                format!("[\"OpenChat\",\"{}\"]", hex(&text[..text.iter().position(|&b| b == 0).unwrap_or(text.len())]))
            }
            Out::Sound { param, attribute } => format!("[\"ccSeSetParamInu\",{param},{attribute}]"),
            Out::Smoke { pos, v, s, life, t } => {
                format!("[\"effSmoke\",{},{},{s},{life},{t},512,32]", list(pos), list(v))
            }
            Out::DrawPg { transparency, alpha, height, shaded, .. } => {
                format!("[\"DrawPG\",{transparency},{alpha},{height},{}]", u8::from(shaded))
            }
        };
        self.calls.push(s);
    }
}

/// `RIDE`: pos[4] posP[4] rot[4] hitAttribute transparency setTransparency
/// flags act actOld anmEnd idle speed speedRate nowSpeed ownSet kind cycle
/// posView[4] posEye[4] angle[4] movePos[4] moveEase[4] hitSW hitRadius
/// hitHeight hitPos[4] hitOffset[4] frameSpd[2].
fn read_ride(t: &mut Toks, later: bool) -> Ride {
    let pos = t.v4();
    let pos_p = t.v4();
    let rot = t.v4();
    let hit_attribute = t.u32();
    let transparency = t.u32();
    let char_set_transparency = t.u32();
    let flags = t.u32() as u8;
    let act = t.i32() as i16;
    let act_old = t.i32() as i16;
    let anm_end = t.i32() as i16;
    let idle = t.i32() as i16;
    let speed = t.u32();
    let speed_rate = t.u32();
    let now_speed = t.u32();
    let set_transparency = t.u32();
    let kind = t.i32();
    let cycle = t.i32();
    let pos_view = t.v4();
    let pos_eye = t.v4();
    let angle = t.v4();
    let move_pos = t.v4();
    let move_ease = t.v4();
    let sw = t.int() != 0;
    let radius = t.u32();
    let height = t.u32();
    let hpos = t.v4();
    let offset = t.v4();
    let frame_spd = [t.int() as u16, t.int() as u16];
    // From Mutation on: lead idle hold target[4] range name chat.
    let (seek, chat) = if later {
        let (lead, idle, hold) = (t.i32(), t.i32(), t.i32());
        let (target, range) = (t.v4(), t.u32());
        let seek = Seek { lead, idle, hold, target, range, name: name(t) };
        let mut chat = [0u8; ride::CHAT_LEN];
        if let Some(n) = name(t) {
            chat[..n.len()].copy_from_slice(n.as_bytes());
        }
        (seek, chat)
    } else {
        (Seek::default(), [0; ride::CHAT_LEN])
    };
    Ride {
        pos,
        pos_p,
        rot,
        hit_attribute,
        transparency,
        char_set_transparency,
        flags,
        act,
        act_old,
        anm_end,
        idle,
        speed,
        speed_rate,
        now_speed,
        set_transparency,
        kind,
        cycle,
        pos_view,
        pos_eye,
        angle,
        move_pos,
        move_ease,
        hit: CharHit {
            sw,
            mask2: ride::WALL,
            kind: ride::BODY_KIND,
            radius,
            height,
            pos: hpos,
            offset,
            ..CharHit::default()
        },
        frame_spd,
        later,
        seek,
        chat,
    }
}

fn ride_json(r: &Ride) -> String {
    let mut v: Vec<u32> = Vec::new();
    v.extend(r.pos);
    v.extend(r.pos_p);
    v.extend(r.rot);
    let flags = if r.later { r.flags } else { r.flags & 0x3f };
    v.extend([r.hit_attribute, r.transparency, r.char_set_transparency, u32::from(flags)]);
    v.extend([r.act, r.act_old, r.anm_end, r.idle].map(|x| x as i32 as u32));
    v.extend([r.speed, r.speed_rate, r.now_speed, r.set_transparency, r.kind as u32, r.cycle as u32]);
    v.extend(r.pos_view);
    v.extend(r.pos_eye);
    v.extend(r.angle);
    v.extend(r.move_pos);
    v.extend(r.move_ease);
    v.extend([u32::from(r.hit.sw), r.hit.radius, r.hit.height]);
    v.extend(r.hit.pos);
    v.extend(r.hit.offset);
    v.extend([u32::from(r.frame_spd[0]), u32::from(r.frame_spd[1])]);
    if r.later {
        v.extend([r.seek.lead, r.seek.idle, r.seek.hold].map(|x| x as u32));
        v.extend(r.seek.target);
        v.push(r.seek.range);
    }
    list(v)
}

/// From Mutation on, the balloon's line (hex, to its NUL) and the name.
fn seek_json(r: &Ride) -> String {
    if !r.later {
        return "null".into();
    }
    let end = r.chat.iter().position(|&b| b == 0).unwrap_or(r.chat.len());
    let name = r.seek.name.map_or("null".into(), |n| format!("\"{}\"", hex(n.as_bytes())));
    format!("{{\"chat\":\"{}\",\"name\":{name}}}", hex(&r.chat[..end]))
}

fn read_input(t: &mut Toks, later: bool) -> Input {
    Input {
        pad: Pad { pow_l: t.int() as u8, dirc_l: t.u32() },
        pause: t.int() != 0,
        dne: t.int() != 0,
        bounds: {
            let b = t.v4();
            MapBounds { min: [b[0], b[1]], max: [b[2], b[3]] }
        },
        area: t.i32(),
        // From Mutation on: 1 and the race's top speed, acceleration and
        // easings, or 0.
        race: (t.int() != 0).then(|| ride::RaceRide {
            max: t.u32(),
            accel: t.u32(),
            ease_on: t.u32(),
            ease_off: t.u32(),
        }),
        push: if later { t.u32() } else { 0 },
        pow_r: if later { t.int() as u8 } else { 0 },
    }
}

/// `SEEK`: event area, dungeon[4], player[4], then each list as a count
/// and its entries (pos[4] type active going name).
fn read_seek(t: &mut Toks, s: &mut Script) {
    s.event_area = t.i32();
    s.dungeon = t.v4();
    s.player = t.v4();
    let entries = |t: &mut Toks| {
        let n = t.int();
        (0..n)
            .map(|_| SeekEntry { pos: t.v4(), ty: t.i32(), active: t.int() != 0, going: t.int() != 0, name: name(t) })
            .collect::<Vec<_>>()
    };
    s.gims = entries(t);
    s.circles = entries(t);
}

fn read_camera(t: &mut Toks, s: &mut Script) {
    s.cam_type = t.i32();
    s.cam_id = t.i32();
    s.cam_rot = t.v4();
    s.cam_reset = t.int() != 0;
    s.cam_reset_dirc = t.u32();
}

/// `SCRIPT`: each queue as a count and its items: land (z), result (-1
/// none, else the nearest's attribute), attr, collide (r offset[4]), line,
/// eye (angle.z), trans (fade hide), fwd, notes (per frame: n, then n
/// event param pairs), leg (pos[4]), draw (transparency).
fn read_script(t: &mut Toks, s: &mut Script) {
    let n = t.int();
    s.land = (0..n).map(|_| t.u32()).collect();
    let n = t.int();
    s.result = (0..n).map(|_| t.int()).collect();
    let n = t.int();
    s.attr = (0..n).map(|_| t.u32()).collect();
    let n = t.int();
    s.collide = (0..n).map(|_| (t.i32(), t.v4())).collect();
    let n = t.int();
    s.line = (0..n).map(|_| t.u32()).collect();
    let n = t.int();
    s.eye = (0..n).map(|_| t.u32()).collect();
    let n = t.int();
    s.trans = (0..n).map(|_| (t.u32(), t.int() != 0)).collect();
    let n = t.int();
    s.fwd = (0..n).map(|_| t.int() as i16).collect();
    let n = t.int();
    s.notes = (0..n)
        .map(|_| {
            let k = t.int();
            (0..k).map(|_| Note { event: t.u32(), param: t.u32() }).collect()
        })
        .collect();
    let n = t.int();
    s.leg = (0..n).map(|_| t.v4()).collect();
    let n = t.int();
    s.draw = (0..n).map(|_| t.u32()).collect();
}

fn frame_json(r: &Ride, g: &Globals, rand: &Rand, s: &mut Script, ret: Option<i32>) -> String {
    let plw =
        s.plw.take().map_or("null".into(), |(p, r, pause)| format!("[{},{},{}]", list(p), list(r), u8::from(pause)));
    let calls = std::mem::take(&mut s.calls);
    let ret = ret.map_or("null".into(), |v| v.to_string());
    format!(
        "{{\"ride\":{},\"seek\":{},\"ret\":{ret},\"g\":[{},{}],\"rand\":{},\"plw\":{plw},\"reset\":{},\"calls\":[{}]}}",
        ride_json(r),
        seek_json(r),
        g.pg_r,
        g.pg_din,
        rand.0,
        u8::from(s.cam_reset),
        calls.join(",")
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&path)?;
    let tables = RideTables::of(iso.volume()?);
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let mut t = Toks(line.split_whitespace());
        let Some(cmd) = t.0.next() else { continue };
        let answer = match cmd {
            "tables" => format!(
                "{{\"anims\":{:?},\"anims_pg\":{:?},\"files\":{:?},\"angles\":{},\"seek_types\":{}}}",
                tables.anims,
                (0..ride::ACTS as i16).map(|a| tables.anim_pg(a, 0)).collect::<Vec<_>>(),
                tables.files,
                list(tables.angles),
                tables.seek.as_ref().map_or("null".into(), |s| list(s.types.iter()))
            ),
            "new" => {
                let kind = t.i32();
                let mut rand = Rand(t.int() as u64);
                let (pos, rot) = (t.v4(), t.v4());
                let mut s = Script::default();
                read_camera(&mut t, &mut s);
                read_script(&mut t, &mut s);
                let r = Ride::new(kind, pos, rot, &tables, &mut s, &mut rand);
                let names: Vec<String> = (0..ride::ACTS as i16).map(|a| tables.anim_pg(a, kind)).collect();
                format!(
                    "{{\"file\":\"{}\",\"pg\":{names:?},{}",
                    tables.file(kind),
                    &frame_json(&r, &Globals::default(), &rand, &mut s, None)[1..]
                )
            }
            "main" | "fn" => {
                let (frames, name) = if cmd == "main" { (t.int(), String::new()) } else { (1, t.word().to_string()) };
                let later = tables.volume != piney_data::volume::Volume::Inf;
                let mut r = read_ride(&mut t, later);
                let input = read_input(&mut t, later);
                let mut g = Globals { pg_r: t.i32(), pg_din: t.i32() };
                let mut rand = Rand(t.int() as u64);
                let mut s = Script { bounds: input.bounds, ..Script::default() };
                read_camera(&mut t, &mut s);
                read_script(&mut t, &mut s);
                if later {
                    read_seek(&mut t, &mut s);
                }
                let mut each = Vec::new();
                for _ in 0..frames {
                    let mut ret = None;
                    match name.as_str() {
                        "" => ride::main(&mut r, &mut s, &input, &mut g, &tables, &mut rand),
                        "control_move" => {
                            // Its answer is compared from Mutation on (the search's 0).
                            let v = ride::control_move(&tables, &mut r, &mut s, &input, &mut rand);
                            ret = later.then_some(v);
                        }
                        "seek_move" => ret = Some(ride::seek_move(&tables, &mut r, &mut s, &input, &mut rand)),
                        "seek" => ret = Some(ride::seek(&tables, &mut r, &mut s, &input)),
                        "anim_ctrl" => ride::anim_ctrl(&mut r, &mut s, &tables, &input, &mut rand),
                        "draw_pg" => ride::draw_pg(&mut r, &mut s, &input),
                        "note" => {
                            let n = Note { event: t.u32(), param: t.u32() };
                            ride::check_note(&mut r, &mut s, n, &mut rand);
                        }
                        "smoke" => {
                            let (d, legs) = (t.u32(), t.i32());
                            ride::paw_smoke(&r, &mut s, d, legs, &mut rand);
                        }
                        other => panic!("unknown function {other}"),
                    }
                    each.push(frame_json(&r, &g, &rand, &mut s, ret));
                }
                format!("{{\"frames\":[{}]}}", each.join(","))
            }
            "lever" => format!("{}", ride::pad_lever_power(t.u32())),
            "place" => {
                let (pos, rot) = (t.v4(), t.v4());
                let slot = t.int() as usize;
                list(ride::exit_place(&tables, pos, rot, slot))
            }
            "adult" => {
                let (server, slot) = (t.i32(), t.i32());
                let mut save = piney_data::save::SaveData::new();
                for i in 0..60 {
                    save.set_i16(ride::SAVE_GROWTH + 2 * i, t.int() as i16);
                }
                format!("{}", ride::adult_check(&save, server, slot))
            }
            other => format!("{{\"error\":\"unknown request {other}\"}}"),
        };
        writeln!(out, "{answer}")?;
        out.flush()?;
    }
    Ok(())
}
