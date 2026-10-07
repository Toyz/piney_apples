//! The probe `tools/test_fieldui_rs.py` (and the tests built on it)
//! drives: the field UI over a script read from stdin, a JSON line out per
//! frame. The script sets the world (`party`, `target`, `sort`, `game`,
//! `server`, `npc`, `pguso` ...), the save and cards (`save`, `savebytes`,
//! `card`), the inputs due before a frame (`pad`, `open`, `msg`, `check`
//! ...) and what to print (`tk`, `watch`, `trade`); `run N` runs frames
//! 1..N. Between frames the probe does what the runtime would with the
//! requests. Each command's arguments are as `main` reads them.

use std::collections::HashMap;
use std::io::{self, BufRead};
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::SaveState;
use piney_desktop::message::MsgDraw;
use piney_fieldui::ctrl::{Draw, MenuRand};
use piney_fieldui::spr::Packet;
use piney_fieldui::{CharInfo, FieldUi, Request, World};
use piney_input::{Buttons, Pad};

fn hex(s: &str) -> Option<Vec<u8>> {
    if s == "-" {
        return None;
    }
    if s == "=" {
        return Some(Vec::new());
    }
    Some((0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap_or(0)).collect())
}

fn jstr(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &c in b {
        s.push_str(&format!("{c:02x}"));
    }
    s.push('"');
    s
}

fn f(v: f32) -> String {
    format!("{v:?}")
}

fn packet(p: &Packet) -> String {
    let v = p.vcol.map_or(String::new(), |v| {
        let c: Vec<String> = v.iter().map(|c| format!("[{},{},{},{}]", c[0], c[1], c[2], c[3])).collect();
        format!(",[{}]", c.join(","))
    });
    format!(
        "[\"{}\",{},{},{},{},{},{},{},{},{},{},{},{},{},{},[{},{},{},{}],{}{v}]",
        p.obj.name(),
        p.code,
        p.text.as_ref().map_or("null".to_string(), |t| jstr(t)),
        f(p.dx),
        f(p.dy),
        f(p.sx),
        f(p.sy),
        f(p.cx),
        f(p.cy),
        f(p.rot),
        p.su,
        p.sv,
        p.wu,
        p.wv,
        p.wi,
        p.rgba[0],
        p.rgba[1],
        p.rgba[2],
        p.rgba[3],
        // ctrl 0x20, 0x10 and 0x40 as bits 0, 1, 2.
        u8::from(p.flip) | (u8::from(p.shadow) << 1) | (u8::from(p.flip_v) << 2)
    )
}

fn char_from(w: &[&str], with_tag: bool) -> CharInfo {
    let n = |i: usize| w[i].parse::<i64>().unwrap_or(0);
    let mut c = CharInfo {
        handle: n(0) as u32,
        types: n(1) as u32,
        id: n(2) as i16,
        hp: n(3) as i16,
        sp: n(4) as i16,
        max_hp: n(5) as i16,
        max_sp: n(6) as i16,
        name: hex(w[7]).unwrap_or_default(),
        height: 160.0,
        attribute: -1,
        ..CharInfo::default()
    };
    if with_tag {
        let (tx, ty) = (n(8) as i32, n(9) as i32);
        c.tag = if tx == -9999 { None } else { Some((tx, ty)) };
    } else {
        for (k, v) in w[8..].iter().enumerate().take(16) {
            c.condition[k] = v.parse().unwrap_or(0);
        }
    }
    c
}

/// The members `ccMenuCtrl`'s constructor sets that the frame's state
/// leaves out, in the harness's order; a pointer as 0 or 1.
fn ctor_members(c: &piney_fieldui::MenuCtrl) -> Vec<i32> {
    let mut v: Vec<i32> = c.panel_bure.iter().chain(&c.panel_flash).map(|&b| i32::from(b)).collect();
    v.extend(
        [
            c.drain_status,
            c.mode,
            c.exception_disp,
            c.teach_cnt,
            c.cursol_init,
            c.cursol_off,
            c.fade,
            c.bg_col,
            c.face_num,
        ]
        .map(i32::from),
    );
    v.extend([c.drain_alpha, i32::from(c.cursol_target != 0)]);
    let t = &c.talk;
    v.extend(
        [c.pl_attack, t.talk_num, c.first_time, t.talk_trade_flag, t.talk_loop_cnt, c.forbid]
            .into_iter()
            .chain([c.forbid_chat_except, c.target_forbid, c.inter_noiz])
            .map(i32::from),
    );
    v.extend(c.protect.iter().chain(&c.protect_cnt).map(|&p| i32::from(p)));
    v.extend(c.protect_char.iter().chain(&c.sub_target).map(|&h| i32::from(h != 0)));
    v.push(i32::from(c.dummy_target.is_some()));
    v
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path)?;
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN")?)?);
    let mut ui = FieldUi::new(&mut iso, archive)?;
    let mut world = World { boss_entry: -1, party_id: [-1; 3], ..World::default() };
    // The port's fresh save, Init's text from the executable.
    let init_text = piney_desktop::InitText::from_disc(&mut iso)?;
    let volume = iso.volume()?;
    let mut save = SaveState::fresh_with(&init_text);
    let mut pads: HashMap<usize, (u32, u32)> = HashMap::new();
    let mut opens: HashMap<usize, i16> = HashMap::new();
    let mut bans: HashMap<usize, bool> = HashMap::new();
    let mut noises: HashMap<usize, i32> = HashMap::new();
    type Msg = (i32, Option<Vec<u8>>, [Option<Vec<u8>>; 3]);
    let mut msgs: HashMap<usize, Msg> = HashMap::new();
    let mut infos: HashMap<usize, [Option<Vec<u8>>; 3]> = HashMap::new();
    let mut checks: Vec<usize> = Vec::new();
    let mut target_fix = 0;
    let mut gamefs: HashMap<usize, (i32, i32, i32)> = HashMap::new();
    let mut skills: Vec<(usize, i32)> = Vec::new();
    let mut firsts: Vec<usize> = Vec::new();
    let mut extra: Vec<CharInfo> = Vec::new();
    let mut items_of: Vec<(u32, i32)> = Vec::new();
    let mut gim_acts: Vec<(usize, u32, i32)> = Vec::new();
    let mut gones: Vec<(usize, u32)> = Vec::new();
    let mut traps_of: Vec<(u32, i32, i16)> = Vec::new();
    let mut exdefs_of: Vec<(u32, i16, i16)> = Vec::new();
    let mut area_code_set: [i16; 3] = [-1; 3];
    world.area_codes = [(-1, -1); 16];
    // The harness's rand() returns 0; `randlog` traces each draw.
    let draws = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let counted = |mut f: Box<dyn FnMut() -> i32>| -> Box<dyn FnMut() -> i32> {
        let draws = draws.clone();
        Box::new(move || {
            draws.set(draws.get() + 1);
            f()
        })
    };
    let mut randlog = false;
    let mut restarts: Vec<usize> = Vec::new();
    let mut ctor_fields = false;
    ui.ctrl.rng = MenuRand::Fixed(counted(Box::new(|| 0)));
    let mut chains: [Vec<u32>; 3] = Default::default();
    let mut places: Vec<(u32, f32, bool, [f32; 3])> = Vec::new();
    let mut tk = false;
    let sim = sim_card::SimCard::new(volume);
    let mut cards_used = false;
    ui.set_card(Box::new(sim.clone()));
    let mut modes: HashMap<usize, i16> = HashMap::new();
    let mut attacks: HashMap<usize, i16> = HashMap::new();
    let mut directs: HashMap<usize, u32> = HashMap::new();
    let mut book: Option<(i32, usize)> = None;
    let mut watches: Vec<(usize, usize)> = Vec::new();
    // ccPad::actuaterSw as the vibration requests leave it (2: not yet set).
    let mut actuater_sw = 2;
    // The runtime's answers to DataDrain and DrainSideEffect (scripted).
    type DrainSide = ((i32, bool, i32), Vec<(u32, i16, i16)>);
    let mut drain_answers: std::collections::VecDeque<Vec<i32>> = Default::default();
    let mut drain_sides: std::collections::VecDeque<DrainSide> = Default::default();
    let mut drain_target_cleared = false;
    // Data Drain's movie: how many frames a stream plays (the harness's
    // fake ccThExecuteStream), and the frame the current one ends.
    let mut movie_len = 0usize;
    let mut movie_end: Option<usize> = None;
    let mut sizes: HashMap<u32, i32> = HashMap::new();
    let mut trade = false;
    let mut drops: Vec<usize> = Vec::new();
    let mut growths: HashMap<usize, i8> = HashMap::new();
    let mut pg_msgs: HashMap<usize, i8> = HashMap::new();
    let mut breed = false;
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.is_empty() {
            continue;
        }
        let n = |i: usize| w[i].parse::<i64>().unwrap_or(0);
        match w[0] {
            "party" => {
                let slot = n(1) as usize;
                let c = char_from(&w[2..], false);
                world.party_id[slot] = i32::from(c.id);
                world.party[slot] = Some(c);
            }
            "target" => world.target = Some(char_from(&w[1..], true)),
            "sort" => {
                // sort HANDLE TYPES ID HP SP MHP MSP NAMEHEX DIST RES BARX BARY ARROWX ARROWY PP DEAD
                let mut c = char_from(&w[1..], true);
                c.tag = None;
                c.cmnd_dist = w[9].parse().unwrap_or(0.0);
                c.bar_res = n(10) as i32;
                c.bar = (n(11) as i32, n(12) as i32);
                c.arrow = (n(13) as i32, n(14) as i32);
                c.pp = n(15) as i16;
                c.condition[0] = n(16) as i16;
                world.sorted.push(c);
            }
            "gamef" => {
                gamefs.insert(n(1) as usize, (n(2) as i32, n(3) as i32, n(4) as i32));
            }
            "game" => {
                world.game.status = 5;
                world.game.area = n(1) as i32;
                world.game.in_battle = n(2) as i32;
                world.game.in_battle_cnt = n(3) as i32;
                world.game.dungeon_type = n(4) as i32;
            }
            "dead" => {
                // plw.pw->condition.dead: the player is party slot 0.
                world.player_dead = n(1) != 0;
                if let Some(p) = world.party[0].as_mut() {
                    p.condition[0] = n(1) as i16;
                }
            }
            "save" => {
                let (off, size, v) = (n(1) as usize, n(2), n(3));
                match size {
                    1 => save.save.set_u8(off, v as u8),
                    2 => save.save.set_i16(off, v as i16),
                    _ => save.save.set_i32(off, v as i32),
                }
            }
            "operate" => save.operate = n(1) as u64,
            "pad" => {
                pads.insert(n(1) as usize, (n(2) as u32, n(3) as u32));
            }
            "open" => {
                opens.insert(n(1) as usize, n(2) as i16);
            }
            "ban" => {
                bans.insert(n(1) as usize, n(2) != 0);
            }
            "noise" => {
                noises.insert(n(1) as usize, n(2) as i32);
            }
            "msg" => {
                msgs.insert(n(1) as usize, (n(2) as i32, hex(w[3]), [hex(w[4]), hex(w[5]), hex(w[6])]));
            }
            "info" => {
                infos.insert(n(1) as usize, [hex(w[2]), hex(w[3]), hex(w[4])]);
            }
            "check" => checks.push(n(1) as usize),
            "pos" => {
                let fl = |i: usize| w[i].parse::<f32>().unwrap_or(0.0);
                places.push((n(1) as u32, fl(2), n(3) != 0, [fl(4), fl(5), fl(6)]));
            }
            "chain" => {
                let k = (n(1) as usize).min(2);
                chains[k] = w[2..].iter().map(|v| v.parse().unwrap_or(0)).collect();
            }
            "skill" => skills.push((n(1) as usize, n(2) as i32)),
            "item" => items_of.push((n(1) as u32, n(2) as i32)),
            // gimact F HANDLE STATE: a gimmick's actNum (+0x1d4) from frame
            // F (the spring's state, which FountainMenu3 waits on).
            "gimact" => gim_acts.push((n(1) as usize, n(2) as u32, n(3) as i32)),
            // gone F HANDLE: from frame F the character is no more
            // (ccCheckTarget 0: cmndTargetPrev lost).
            "gone" => gones.push((n(1) as usize, n(2) as u32)),
            // bgnum B: WORLD_MAN::GetBG.
            "bgnum" => world.game.bgnum = n(1) as i32,
            // trap HANDLE PARAM2 SKILL: a box's +0x150 and +0x7c.
            "trap" => traps_of.push((n(1) as u32, n(2) as i32, n(3) as i16)),
            // exdef HANDLE EXDEFENSE LOWERED: a foe's row Exdefense and the
            // bits of the defences now below the row's.
            "exdef" => exdefs_of.push((n(1) as u32, n(2) as i16, n(3) as i16)),
            // online ID: an npcTbl row on g_entCtrl's NPC list.
            "online" => world.npcs.push(n(1) as i32),
            "areaitem" => {
                // areaitem FIELD_ATTR EVENT_AREA AREA_LEVEL ITEM_OFS FLOOR FIELD
                world.field_attr = n(1) as i32;
                world.event_area = n(2) as i32;
                world.area_word = (n(3) as i32, n(4) as i32);
                world.game.word_level = n(3) as i32;
                world.game.floor = n(5) as i32;
                world.game.field = n(6) as i32;
            }
            "first" => firsts.push(n(1) as usize),
            "char" => {
                // char HANDLE TYPES ID HP SP MHP MSP NAMEHEX: one only on a chain.
                extra.push(char_from(&w[1..], false));
            }
            "server" => {
                world.game.server = n(1) as i32;
                world.game.town = n(2) as i32;
                world.game.area_level = n(3) as i32;
            }
            "areacode" => {
                let k = (n(1) as usize).min(15);
                world.area_codes[k] = (n(2) as i16, n(3) as i16);
            }
            "npc" => {
                let (h, row) = (n(1) as u32, n(2) as i32);
                let b = piney_fieldui::talk::Base::npc(ui.texts().volume, row).unwrap_or_default();
                let (tx, ty) = (n(3) as i32, n(4) as i32);
                world.target = Some(CharInfo {
                    handle: h,
                    types: b.types,
                    id: b.id,
                    name: b.name,
                    height: 160.0,
                    attribute: -1,
                    tag: if tx == -9999 { None } else { Some((tx, ty)) },
                    ..CharInfo::default()
                });
                use piney_fieldui::talk::{Speaker, TalkTarget};
                ui.talk_to(Some(TalkTarget { handle: h, who: Speaker::Npc(row) }));
            }
            // rand V..: the game's rand() and ccRand() both give these in
            // turn (the harness hooks ccRand to rand), then 0.
            "rand" => {
                let mut v: Vec<i32> = w[1..].iter().map(|x| x.parse().unwrap_or(0)).collect();
                v.reverse();
                let v = Arc::new(std::sync::Mutex::new(v));
                let pop = |v: Arc<std::sync::Mutex<Vec<i32>>>| move || v.lock().map_or(0, |mut v| v.pop().unwrap_or(0));
                ui.ctrl.rng = MenuRand::Fixed(counted(Box::new(pop(v.clone()))));
                ui.set_cc_rand(Box::new(pop(v)));
            }
            "tk" => tk = true,
            "card" => {
                cards_used = true;
                let mut p = sim.port(n(1) as i32);
                let b = |i: usize| n(i) != 0;
                (p.present, p.ps2, p.formatted, p.full) = (b(2), b(3), b(4), b(5));
                (p.fail_write, p.fail_sys, p.fail_fmt) = (b(6), b(7), b(8));
            }
            "dir" => sim.port(n(1) as i32).dir = true,
            "file" => {
                let bytes = hex(w[3]).unwrap_or_default();
                sim.port(n(1) as i32)
                    .files
                    .insert(format!("/{}/{}", piney_desktop::card::own_dir_name(volume), w[2]), bytes);
            }
            "cardpos" => ui.set_card_position(n(1) as i32, n(2) as i32),
            "savebytes" => {
                let b = hex(w[1]).unwrap_or_default();
                for (k, &v) in b.iter().enumerate().take(piney_data::save::SIZE) {
                    save.save.set_u8(k, v);
                }
            }
            "mode" => {
                modes.insert(n(1) as usize, n(2) as i16);
            }
            // attack F V: ccMenuCtrl.plAttack set before frame F.
            "attack" => {
                attacks.insert(n(1) as usize, n(2) as i16);
            }
            // direct F V: the pad's held bits at frame F.
            "direct" => {
                directs.insert(n(1) as usize, n(2) as u32);
            }
            // book P F: Ryu Book P read from frame F (ccUseItemRequest's
            // book branch).
            "book" => book = Some((n(1) as i32, n(2) as usize)),
            // gamecnt A B C: ccGame.gameCnt[3].
            "gamecnt" => world.game.game_cnt = [n(1) as i32, n(2) as i32, n(3) as i32],
            "watch" => watches.push((n(1) as usize, n(2) as usize)),
            // grunty K V: ccPgAdultCheck(server, K) = V (-1 none).
            "grunty" => world.pg_adult[(n(1) as usize).min(2)] = Some(n(2) as i32).filter(|&v| v >= 0),
            // rng V: the menu's own rand() (ccMenuCtrl's, the Grunty Flute's) always V.
            "rng" => {
                let v = n(1) as i32;
                ui.ctrl.rng = MenuRand::Fixed(counted(Box::new(move || v)));
            }
            // randlog: each menu rand() a ["rand"] event in its frame.
            "randlog" => randlog = true,
            // restart F: the set-up's new ccThMenu at frame F.
            "restart" => restarts.push(n(1) as usize),
            // ctorfields: every member the constructor sets, each frame.
            "ctorfields" => ctor_fields = true,
            "wiped" => world.party_annihilated = n(1) != 0,
            // movie K: Data Drain's stream ends K frames after it starts.
            "movie" => movie_len = n(1) as usize,
            // size HANDLE N: ccCheckObjectSize of that character.
            "size" => {
                sizes.insert(n(1) as u32, n(2) as i32);
            }
            // drainanswer EROSION COUNT D0 ...: the runtime's answer to
            // DataDrain (the infection after it, the drops).
            "drainanswer" => {
                let v: Vec<i32> = w[1..].iter().map(|t| t.parse().unwrap()).collect();
                drain_answers.push_back(v);
            }
            // drainside ID LEVELDOWN LOST [HANDLE HP SP]...: the answer to
            // DrainSideEffect, the members as it left them. Both queue in
            // the order the drains come.
            "drainside" => {
                let party =
                    w[4..].chunks(3).map(|c| (c[0].parse().unwrap(), c[1].parse().unwrap(), c[2].parse().unwrap()));
                drain_sides.push_back(((n(1) as i32, n(2) != 0, n(3) as i32), party.collect()));
            }
            // spc ID BOOT: a ccSpcManager registry entry.
            "spc" => world.spc.push((n(1) as i32, n(2) as i32)),
            "areawords" => ui.set_area_words([n(1) as i32, n(2) as i32, n(3) as i32], world.game.server, &save),
            "field" => world.game.field = n(1) as i32,
            "town" => world.game.town = n(1) as i32,
            "trade" => trade = true,
            "talkspc" => {
                use piney_fieldui::talk::{Speaker, TalkTarget};
                ui.talk_to(Some(TalkTarget { handle: n(1) as u32, who: Speaker::Spc(n(2) as i32) }));
            }
            "drop" => drops.push(n(1) as usize),
            "pguso" => {
                let (h, row) = (n(1) as u32, n(2) as i32);
                let b = piney_fieldui::talk::Base::npc(ui.texts().volume, row).unwrap_or_default();
                world.target_prev = Some(CharInfo {
                    handle: h,
                    types: b.types,
                    id: b.id,
                    name: b.name,
                    height: 160.0,
                    attribute: -1,
                    ..CharInfo::default()
                });
                let exist = n(3) as u8;
                world.grunty =
                    Some(piney_fieldui::menus::breeder::Grunty { handle: h, row, exist, ..Default::default() });
            }
            "pgtarget" => {
                // The talk target is a Grunty (ccPGuso): its handle, row,
                // exist, and its record's level, size and food line.
                world.grunty = Some(piney_fieldui::menus::breeder::Grunty {
                    handle: n(1) as u32,
                    row: n(2) as i32,
                    exist: n(3) as u8,
                    level: n(4) as i16,
                    size: n(5) as i16,
                    food_num: n(6) as i32,
                    ..Default::default()
                });
            }
            "growth" => {
                growths.insert(n(1) as usize, n(2) as i8);
            }
            "pgmsg" => {
                pg_msgs.insert(n(1) as usize, n(2) as i8);
            }
            "breed" => breed = true,
            "spcmsg" => {
                piney_fieldui::menus::talk::set_spc_base_msg(&mut save, piney_data::volume::Volume::Inf);
            }
            "busy" => {
                // busy HANDLE SKILLID ACT: a member's +0x7c and +0xee
                // (CheckChangeEquip reads them).
                let h = n(1) as u32;
                for c in world.party.iter_mut().flatten().filter(|c| c.handle == h) {
                    c.skill = n(2) as i16;
                    c.act = n(3) as i16;
                }
            }
            "chat" => {
                // chat HANDLE V0..V5: a member's personality +200 on.
                let h = n(1) as u32;
                for c in world.party.iter_mut().flatten().filter(|c| c.handle == h) {
                    for k in 0..6 {
                        c.chat[k] = n(2 + k) as i16;
                    }
                }
            }
            "run" => {
                let frames = n(1) as usize;
                // The constructor's areaLevel 0 in a town (game+0x14 0),
                // which the town's WorldMode starts each scene with.
                if world.game.area == 0 {
                    world.game.area_level = 0;
                }
                for &(h, code) in &items_of {
                    let set = |c: &mut CharInfo| {
                        if c.handle == h {
                            c.item = code;
                        }
                    };
                    world.party.iter_mut().flatten().for_each(set);
                    world.target.iter_mut().for_each(set);
                    world.sorted.iter_mut().for_each(set);
                    extra.iter_mut().for_each(set);
                }
                for &(h, trap, skill) in &traps_of {
                    let set = |c: &mut CharInfo| {
                        if c.handle == h {
                            c.trap = trap;
                            c.skill = skill;
                        }
                    };
                    world.target.iter_mut().for_each(set);
                    world.sorted.iter_mut().for_each(set);
                    extra.iter_mut().for_each(set);
                }
                for &(h, ex, lowered) in &exdefs_of {
                    let set = |c: &mut CharInfo| {
                        if c.handle == h {
                            c.exdefense = ex;
                            c.exdefense_lowered = lowered;
                        }
                    };
                    world.target.iter_mut().for_each(set);
                    world.sorted.iter_mut().for_each(set);
                    extra.iter_mut().for_each(set);
                }
                for &(h, width, view, p) in &places {
                    let set = |c: &mut CharInfo| {
                        if c.handle == h {
                            c.width = width;
                            c.in_view = view;
                            c.pos_p = p;
                        }
                    };
                    world.party.iter_mut().flatten().for_each(set);
                    world.target.iter_mut().for_each(set);
                    world.sorted.iter_mut().for_each(set);
                    extra.iter_mut().for_each(set);
                }
                let find = |w: &World, h: u32| -> Option<CharInfo> {
                    w.party
                        .iter()
                        .flatten()
                        .chain(w.sorted.iter())
                        .chain(w.target.iter())
                        .chain(w.target_prev.iter())
                        .chain(extra.iter())
                        .find(|c| c.handle == h)
                        .cloned()
                };
                for (k, hs) in chains.iter().enumerate() {
                    let v: Vec<CharInfo> = hs.iter().filter_map(|&h| find(&world, h)).collect();
                    match k {
                        0 => world.pc_chain = v,
                        1 => world.ene_chain = v,
                        _ => world.obj_chain = v,
                    }
                }
                for c in world.sorted.iter_mut().chain(world.ene_chain.iter_mut()).chain(world.target.iter_mut()) {
                    if let Some(&s) = sizes.get(&c.handle) {
                        c.object_size = s;
                    }
                }
                let names = save.names();
                // Infection's glyphs, for the centred lines' widths.
                let fonts = piney_desktop::kanji::Fonts::of(piney_data::volume::Volume::Inf, Vec::new());
                let mut checking = false;
                for fr in 1..=frames {
                    if movie_end == Some(fr) {
                        if ui.book_on() {
                            ui.book_stream_done();
                        } else {
                            ui.drain_movie_done();
                        }
                        movie_end = None;
                    }
                    let (push, rep) = pads.get(&fr).copied().unwrap_or((0, 0));
                    let direct = Buttons(directs.get(&fr).copied().unwrap_or(0));
                    let pad = Pad { direct, push: Buttons(push), repeat: Buttons(rep), ..Pad::default() };
                    if book.is_some_and(|(_, f)| f == fr) {
                        ui.start_book(book.map_or(0, |b| b.0));
                        // The game's own draws, flicker and all.
                        if let Some(t) = ui.ctrl.book.as_mut() {
                            t.as_the_game = true;
                        }
                    }
                    if let Some(&(area, inb, cnt)) = gamefs.get(&fr) {
                        world.game.area = area;
                        world.game.in_battle = inb;
                        world.game.in_battle_cnt = cnt;
                    }
                    if drops.contains(&fr) && world.target.is_some() {
                        world.target_prev = world.target.take();
                    }
                    for &(f, h, st) in &gim_acts {
                        if f == fr {
                            for c in world.target.iter_mut().chain(world.target_prev.iter_mut()) {
                                if c.handle == h {
                                    c.act_num = st;
                                }
                            }
                        }
                    }
                    for &(f, h) in &gones {
                        if f == fr {
                            if world.target.as_ref().is_some_and(|c| c.handle == h) {
                                world.target = None;
                            }
                            if world.target_prev.as_ref().is_some_and(|c| c.handle == h) {
                                world.target_prev = None;
                            }
                        }
                    }
                    if let Some(g) = world.grunty.as_mut() {
                        if let Some(&v) = growths.get(&fr) {
                            g.growth = v;
                        }
                        if let Some(&v) = pg_msgs.get(&fr) {
                            g.msg = v;
                        }
                    }
                    // The event task runs first in a frame.
                    if let Some(&on) = bans.get(&fr) {
                        ui.menu_ban(on);
                    }
                    if let Some(&v) = noises.get(&fr) {
                        ui.noise(v);
                    }
                    if let Some(&num) = opens.get(&fr) {
                        ui.open_menu(num);
                    }
                    if firsts.contains(&fr) {
                        ui.ctrl.first_time = 1;
                    }
                    if let Some(&v) = modes.get(&fr) {
                        ui.ctrl.mode = v;
                    }
                    if let Some(&v) = attacks.get(&fr) {
                        ui.ctrl.pl_attack = v;
                    }
                    let mut ev = Vec::new();
                    draws.set(0);
                    if let Some((emode, name, l)) = msgs.get(&fr) {
                        ui.ctrl.msg.change(
                            *emode,
                            name.as_deref(),
                            [l[0].as_deref(), l[1].as_deref(), l[2].as_deref()],
                            &names,
                        );
                    }
                    if let Some(l) = infos.get(&fr) {
                        ui.ctrl.msg.change_info([l[0].as_deref(), l[1].as_deref(), l[2].as_deref(), None], &names);
                    }
                    if checks.contains(&fr) {
                        checking = true;
                    }
                    if checking {
                        let r = ui.message_check(&pad, &save);
                        ev.push(format!("[\"check\",{r}]"));
                        if r != 0 {
                            checking = false;
                        }
                    }
                    for &(from, v) in &skills {
                        if fr >= from {
                            world.player_skill = v;
                        }
                    }
                    let count = fr as u32;
                    if restarts.contains(&fr) {
                        // A scene's set-up: the new ccThMenu's frame is its
                        // constructor's (areaLevel as at the run's start).
                        ui.menu_task_started(&mut save, world.party_id);
                        if world.game.area == 0 {
                            world.game.area_level = 0;
                        }
                    } else {
                        let _frame = ui.step(&pad, &world, &mut save, count);
                        // ccUseItemRequest is hooked there: it returns at once.
                        if ui.item_asked() {
                            ui.answer_item(Vec::new(), &pad, &mut save, count, None);
                        }
                    }
                    let c = &ui.ctrl;
                    let item_num = c.item_num;
                    let l = &c.lists[c.menu.clamp(0, 88) as usize];
                    let st = [
                        i32::from(c.menu),
                        i32::from(c.menu_next),
                        i32::from(c.menu_status),
                        i32::from(c.proccess),
                        c.alpha,
                        c.kanji_alpha,
                        i32::from(c.panel_status),
                        c.panel_alpha,
                        i32::from(c.bg_status),
                        c.bg_alpha,
                        i32::from(c.map_status),
                        c.map_alpha,
                        c.target_alpha,
                        c.cursol_alpha,
                        i32::from(c.battle_cnt),
                        i32::from(c.still),
                        i32::from(c.open_req),
                        i32::from(c.wait_count),
                        i32::from(c.mail_cnt),
                        i32::from(l.select),
                        c.reverse_head as i32,
                    ];
                    let m = &c.msg;
                    let ms = [
                        m.kanji_status,
                        m.window_status,
                        m.mode(),
                        m.ty,
                        m.cursol,
                        m.str_line,
                        m.kanji_alpha,
                        m.window_alpha,
                        m.wait_cnt,
                        m.wait_close_cnt,
                    ];
                    let mut pk = Vec::new();
                    let mut kanji = Vec::new();
                    let mut mc = Vec::new();
                    let mut mt = Vec::new();
                    for d in ui.draws() {
                        match d {
                            Draw::Send(ps) => pk.extend(ps.iter().map(packet)),
                            Draw::Kanji { obj, text, packets } => {
                                if !packets.is_empty() {
                                    kanji.push(format!("[\"{}\",{}]", obj.name(), jstr(text)));
                                }
                                pk.extend(packets.iter().map(packet));
                            }
                            Draw::Text { text, dx, dy, rgba, count, .. } => mt.push(format!(
                                "[null,{},{count},{},{},[{},{},{},{}]]",
                                jstr(text),
                                f(*dx),
                                f(*dy),
                                rgba[0],
                                rgba[1],
                                rgba[2],
                                rgba[3]
                            )),
                            Draw::Fade { c0, c1, cnt, tcnt } => ev.push(format!("[\"fade\",{c0},{c1},{cnt},{tcnt}]")),
                            // The gate hack's animations are in the trace.
                            Draw::Hack(_) | Draw::Noiz(_) => {}
                            Draw::Msg(ms) => {
                                for x in ms {
                                    match x {
                                        MsgDraw::Cell { code, dx, dy, sx, sy, alpha, .. } => mc.push(format!(
                                            "[{code},{},{},{},{},{alpha}]",
                                            f(*dx),
                                            f(*dy),
                                            f(*sx),
                                            f(*sy)
                                        )),
                                        MsgDraw::Text { line, text, count, dx, dy, rgba, centred } => mt.push(format!(
                                            "[{line},{},{count},{},{},[{},{},{},{}]]",
                                            jstr(text),
                                            f(piney_desktop::message::text_dx(
                                                *dx,
                                                text,
                                                *centred,
                                                &fonts,
                                                &save.names()
                                            )),
                                            f(*dy),
                                            rgba[0],
                                            rgba[1],
                                            rgba[2],
                                            rgba[3]
                                        )),
                                        MsgDraw::Send => {}
                                    }
                                }
                            }
                        }
                    }
                    let reqs = ui.take_requests();
                    for r in &reqs {
                        match r {
                            Request::TargetFix(on) => target_fix = i32::from(*on),
                            Request::PlayerSp(v) => {
                                if let Some(p) = world.party[0].as_mut() {
                                    p.sp = *v;
                                }
                            }
                            Request::AreaCodeSet(w) => area_code_set = *w,
                            Request::Talk(piney_fieldui::talk::TalkReq::GruntyGrowth { target, growth }) => {
                                if let Some(g) = world.grunty.as_mut().filter(|g| g.handle == *target) {
                                    g.growth = *growth;
                                }
                            }
                            Request::Talk(piney_fieldui::talk::TalkReq::GruntyFood {
                                target,
                                food_mode,
                                chat_flag,
                            }) => {
                                if let Some(g) = world.grunty.as_mut().filter(|g| g.handle == *target) {
                                    g.food_mode = *food_mode;
                                    if let Some(c) = chat_flag {
                                        g.chat_flag = *c;
                                    }
                                }
                            }
                            Request::Affect { target, kind } => {
                                // The Grunty's affect function's msgNum, as the
                                // menus make it for the frame.
                                if let Some(g) = world.grunty.as_mut().filter(|g| g.handle == *target) {
                                    piney_fieldui::menus::inu::after_affect(g, *kind);
                                }
                            }
                            Request::AreaLevel(v) => world.game.area_level = *v,
                            Request::AddMember(id) => {
                                // inviteSpc: the member from the scenario's
                                // characters, into the first free slot.
                                if let Some(k) = world.party_id.iter().position(|&v| v == -1)
                                    && let Some(c) = extra.iter().find(|c| c.id == *id).cloned()
                                {
                                    world.party_id[k] = i32::from(*id);
                                    world.party[k] = Some(c);
                                }
                            }
                            Request::CalcReal { target } => {
                                if let Some(c) = world.party.iter_mut().flatten().find(|c| c.handle == *target) {
                                    ui.calc_real(&mut save, c);
                                }
                            }
                            // The runtime's rules: the infection, ccDeleteCmnd
                            // on a target that is not a boss, the drops back.
                            Request::DataDrain { target, .. } => {
                                if let Some(v) = drain_answers.pop_front() {
                                    save.save.set_i16(0x676e, v[0] as i16);
                                    ui.drain_drops(&v[2..], v[1] as usize);
                                }
                                let boss = find(&world, *target).is_some_and(|c| c.is(0x80));
                                if !boss {
                                    // ccDeleteCmnd: the target dropped
                                    // (ccChangeCmndTarget(0)) when it is it.
                                    if world.target.as_ref().is_some_and(|t| t.handle == *target) {
                                        world.target_prev = world.target.take();
                                        drain_target_cleared = true;
                                    }
                                    // Off the command chains; the sort
                                    // chain is ccThGameCtrl's (static here).
                                    world.ene_chain.retain(|c| c.handle != *target);
                                    world.pc_chain.retain(|c| c.handle != *target);
                                    world.obj_chain.retain(|c| c.handle != *target);
                                }
                            }
                            Request::DrainSideEffect => {
                                if let Some(((id, down, lost), party)) = drain_sides.pop_front() {
                                    ui.drain_side_effect(id, down, lost);
                                    for (h, hp, sp) in party {
                                        if let Some(c) = world.party.iter_mut().flatten().find(|c| c.handle == h) {
                                            c.hp = hp;
                                            c.sp = sp;
                                        }
                                    }
                                }
                            }
                            Request::TargetsCleared => {
                                world.target = None;
                                world.target_prev = None;
                            }
                            Request::DelMember(slot) => {
                                if let Some(k) = usize::try_from(*slot).ok().filter(|&k| k < 3) {
                                    world.party[k] = None;
                                    world.party_id[k] = -1;
                                }
                            }
                            Request::Vibration { on } if i32::from(*on) != actuater_sw => {
                                actuater_sw = i32::from(*on);
                                ev.push(format!("[\"actuater_sw\",{actuater_sw}]"));
                            }
                            _ => {}
                        }
                    }
                    // The player's SP shows on the panels, the area words and
                    // level in the state; they are not events.
                    for r in reqs.into_iter().filter(|r| {
                        !matches!(
                            r,
                            Request::TargetFix(_)
                                | Request::PlayerSp(_)
                                | Request::KeepLayers
                                | Request::AreaCodeSet(_)
                                | Request::AreaLevel(_)
                                | Request::Talk(piney_fieldui::talk::TalkReq::GruntyGrowth { .. })
                                | Request::Talk(piney_fieldui::talk::TalkReq::GruntyFood { .. })
                                | Request::WorldHidden(_)
                        )
                    }) {
                        ev.push(match r {
                            Request::Se(s) => format!("[\"se\",{s}]"),
                            Request::SeNote { n, note } => format!("[\"se_note\",{n},{note}]"),
                            Request::CameraShake { power, cycle, time, dirc } => {
                                format!("[\"shake\",{power},{cycle},{time},{dirc}]")
                            }
                            Request::SleepAll => "[\"sleep\"]".into(),
                            Request::WakeAll => "[\"wake\"]".into(),
                            Request::Still(on) => format!("[\"still\",{}]", u8::from(on)),
                            Request::VoiceStop => "[\"voice_stop\"]".into(),
                            Request::VoiceRequest { grp, msg } => format!("[\"voice\",{grp},{msg}]"),
                            Request::TargetFix(on) => format!("[\"target_fix\",{}]", u8::from(on)),
                            Request::TargetClear => {
                                if world.target.is_some() {
                                    world.target_prev = world.target.take();
                                }
                                "[\"target\",0]".into()
                            }
                            Request::Target(h) => {
                                if world.target.as_ref().map_or(0, |t| t.handle) != h {
                                    let c = find(&world, h);
                                    world.target_prev = world.target.take();
                                    world.target = c;
                                }
                                format!("[\"target\",{h}]")
                            }
                            Request::Skill { target, skill } => format!("[\"skill\",{target},{skill}]"),
                            Request::UseItem { target, code } => format!("[\"use_item\",{target},{code}]"),
                            Request::ItemStep(st) => format!("[\"item_step\",{}]", jstr(format!("{st:?}").as_bytes())),
                            Request::SpcMessageOpenTreasureBox => "[\"spc_msg_box\"]".into(),
                            Request::BreakSomething => "[\"break_something\"]".into(),
                            Request::AttackCancel => "[\"attack_cancel\"]".into(),
                            Request::DrainAffect(h) => format!("[\"affect\",{h},13]"),
                            Request::Affect { target, kind } => format!("[\"affect\",{target},{kind}]"),
                            Request::Flash => "[\"flash\"]".into(),
                            Request::KeepLayers | Request::AreaCodeSet(_) | Request::AreaLevel(_) => String::new(),
                            Request::TransferOut(h) => format!("[\"transfer_out\",{h}]"),
                            Request::DeleteNoPartyMember => "[\"delete_no_party\"]".into(),
                            Request::GoToArea(w) => format!("[\"goto\",{},{},{}]", w[0], w[1], w[2]),
                            Request::ChangeArea { area, n } => format!("[\"change_area\",{area},{n}]"),
                            Request::PlayerSp(_) => String::new(),
                            Request::ChatCmd { member, cmd } => format!("[\"chat_cmd\",{member},{cmd},0,0]"),
                            Request::ChatOrder { member, cmd, target, skill } => {
                                format!("[\"chat_cmd\",{member},{cmd},{target},{skill}]")
                            }
                            Request::ManualOff(h) => format!("[\"manual_off\",{h}]"),
                            Request::ManualModeAi { member, ev } => format!("[\"manual_ai\",{member},{ev}]"),
                            Request::RemoteCmd { member, cmd } => format!("[\"remote_cmd\",{member},{cmd}]"),
                            Request::OpenChat(t) => format!("[\"open_chat\",{}]", jstr(&t)),
                            Request::ChangeEquipReport { member, n } => format!("[\"equip_report\",{member},{n}]"),
                            Request::MapAlpha(a) => format!("[\"map_alpha\",{}]", f(a)),
                            Request::Unported(n) => format!("[\"unported\",{n}]"),
                            Request::AddMember(id) => format!("[\"add_member\",{id}]"),
                            Request::Talk(t) => t.trace(),
                            Request::UseItemArg { target, code, arg } => {
                                format!("[\"use_item_arg\",{target},{code},{arg}]")
                            }
                            Request::ChangeMode { num, sf } => format!("[\"change_mode\",{num},{sf}]"),
                            Request::GoField => "[\"go_field\"]".into(),
                            Request::CalcReal { target } => format!("[\"calc_real\",{target}]"),
                            Request::DelMember(slot) => format!("[\"del_member\",{slot}]"),
                            Request::ChangeEquip { target, cat, item } => {
                                format!("[\"change_equip\",{target},{cat},{item}]")
                            }
                            Request::DisplayOffset { x, y } => format!("[\"display_offset\",{x},{y}]"),
                            Request::SoundEnv { main, bgm, se, output } => {
                                format!("[\"sound_env\",{main},{bgm},{se},{output}]")
                            }
                            Request::Vibration { on } => {
                                if on {
                                    "[\"actuater\",1,160,200]".into()
                                } else {
                                    continue;
                                }
                            }
                            Request::CameraType(t) => format!("[\"camera\",{t}]"),
                            Request::GateHackSound(n) => format!("[\"gate_hack_snd\",{n}]"),
                            Request::DrainLevelDown => "[\"level_down\"]".into(),
                            Request::GameOver => "[\"game_over\"]".into(),
                            Request::DrainMovie(num) => {
                                movie_end = Some(fr + movie_len);
                                format!("[\"drain_movie\",{num}]")
                            }
                            // Menu 74's stream and ccThStrParty: the boss's, not
                            // traced by the menus' harness.
                            Request::StreamMenu(num) => format!("[\"stream_menu\",{num}]"),
                            Request::BookStream(Some(page)) => {
                                movie_end = Some(fr + movie_len);
                                format!("[\"drain_movie\",{}]", 112 + page)
                            }
                            Request::BookStream(None) => String::new(),
                            Request::StrParty(flags) => format!("[\"str_party\",{flags}]"),
                            Request::DrainEnemy(h) => format!("[\"drain_enemy\",{h}]"),
                            Request::DataDrain { .. } if std::mem::take(&mut drain_target_cleared) => {
                                "[\"target\",0]".into()
                            }
                            // Their effects are the world's (above).
                            Request::DataDrain { .. } | Request::DrainSideEffect | Request::TargetsCleared => {
                                String::new()
                            }
                            Request::GateHacked => "[\"set_gt_hack\"]".into(),
                            // The noise is the trace's, the screen the runtime's.
                            Request::WorldHidden(_) => String::new(),
                        });
                    }
                    // The OPTION pages' and the gate hack's task, file and
                    // animation calls.
                    for t in ui.ctrl.trace.drain(..) {
                        let tags = [
                            "[\"start_thread\"",
                            "[\"delete_thread\"",
                            "[\"get_ccs\"",
                            "[\"set_tex\"",
                            "[\"file_delete\"",
                            "[\"anm_",
                            "[\"noiz\"",
                            "[\"noiz_",
                            "[\"drain_movie\"",
                            "[\"close_chat\"",
                        ];
                        if tags.iter().any(|g| t.starts_with(g)) {
                            ev.push(t);
                        }
                    }
                    let mut st: Vec<String> = st.iter().map(|v| v.to_string()).collect();
                    st.push(target_fix.to_string());
                    st.push(item_num.to_string());
                    let items: Vec<u8> = save.save.bytes()[0x30..0xd0].to_vec();
                    st.push(jstr(&items));
                    for w in area_code_set {
                        st.push(w.to_string());
                    }
                    st.push(world.game.area_level.to_string());
                    st.push(ui.ctrl.trap_num.to_string());
                    st.push(save.save.i16(0x7440).to_string());
                    if tk {
                        let c = &ui.ctrl;
                        let t = &c.talk;
                        let l = &c.lists[c.menu.clamp(0, 88) as usize];
                        let mut v = vec![
                            i32::from(t.talk_num),
                            i32::from(t.talk_trade_flag),
                            i32::from(t.talk_loop_cnt),
                            c.dummy_target.as_ref().map_or(0, |d| d.handle as i32),
                            i32::from(l.index),
                            i32::from(l.page),
                            i32::from(l.page_num),
                            i32::from(l.x),
                            i32::from(l.y),
                            i32::from(l.dy),
                            i32::from(l.my),
                            i32::from(l.sx),
                            i32::from(l.sy),
                            i32::from(l.disp),
                        ];
                        v.extend(t.temp);
                        st.extend(v.iter().map(|v| v.to_string()));
                    }
                    if trade {
                        st.extend(ui.ctrl.talk.drain_item.iter().take(4).map(|v| v.to_string()));
                    }
                    for &(off, len) in &watches {
                        st.push(jstr(&save.save.bytes()[off..off + len]));
                    }
                    if breed {
                        let c = &ui.ctrl;
                        st.push(c.fade.to_string());
                        st.push(c.cursol_off.to_string());
                        for e in &c.menu_fade.elm {
                            st.extend([e.status, e.cnt, e.tcnt].iter().map(|v| v.to_string()));
                        }
                        st.push(world.grunty.map_or(0, |g| g.growth).to_string());
                        let g = world.grunty.unwrap_or_default();
                        st.extend([g.msg.to_string(), g.food_mode.to_string(), g.chat_flag.to_string()]);
                    }
                    if ctor_fields {
                        st.extend(ctor_members(&ui.ctrl).iter().map(|v| v.to_string()));
                    }
                    if randlog {
                        ev.extend((0..draws.replace(0)).map(|_| "[\"rand\"]".to_string()));
                    }
                    let ms: Vec<String> = ms.iter().map(|v| v.to_string()).collect();
                    println!(
                        "{{\"f\":{fr},\"st\":[{}],\"msg\":[{}],\"pk\":[{}],\"kanji\":[{}],\"mc\":[{}],\"mt\":[{}],\"ev\":[{}]}}",
                        st.join(","),
                        ms.join(","),
                        pk.join(","),
                        kanji.join(","),
                        mc.join(","),
                        mt.join(","),
                        ev.iter().filter(|e| !e.is_empty()).cloned().collect::<Vec<_>>().join(",")
                    );
                }
                if cards_used {
                    println!("{{\"files\":{{{}}}}}", sim.files().join(","));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// The memory card the harness keeps in Python too (tools/test_desktop_data_rs.py's
/// `Card`): ccMcard::CheckPort's answer from the flags, writes to the slot
/// files failing with FAILWRITE, to the index with FAILSYS.
mod sim_card {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use piney_data::volume::Volume;
    use piney_desktop::card::{self, MemoryCard, PortState};
    use piney_desktop::savesys::INDEX_SIZE;

    #[derive(Clone, Default)]
    pub struct SimPort {
        pub present: bool,
        pub ps2: bool,
        pub formatted: bool,
        pub full: bool,
        pub fail_write: bool,
        pub fail_sys: bool,
        pub fail_fmt: bool,
        pub dir: bool,
        pub files: BTreeMap<String, Vec<u8>>,
    }

    #[derive(Clone)]
    pub struct SimCard(Rc<RefCell<[SimPort; 2]>>, Volume);

    fn index_path(v: Volume) -> String {
        format!("/{0}/{0}", card::own_dir_name(v))
    }

    fn slot_path(v: Volume, slot: usize) -> String {
        format!("/{}/{}", card::own_dir_name(v), card::slot_file_name(slot))
    }

    impl SimCard {
        pub fn new(v: Volume) -> Self {
            SimCard(Rc::default(), v)
        }

        pub fn port(&self, port: i32) -> std::cell::RefMut<'_, SimPort> {
            std::cell::RefMut::map(self.0.borrow_mut(), |p| &mut p[port.clamp(0, 1) as usize])
        }

        /// The files as `"port:path":"hex"` (or `zeros:N`).
        pub fn files(&self) -> Vec<String> {
            let mut out = Vec::new();
            for (port, p) in self.0.borrow().iter().enumerate() {
                for (path, b) in &p.files {
                    let v = if b.iter().all(|&c| c == 0) {
                        format!("zeros:{}", b.len())
                    } else {
                        b.iter().map(|c| format!("{c:02x}")).collect()
                    };
                    out.push(format!("\"{port}:{path}\":\"{v}\""));
                }
            }
            out
        }
    }

    impl MemoryCard for SimCard {
        fn check_port(&mut self, port: i32) -> PortState {
            let p = self.port(port);
            if !p.present {
                PortState::NoCard
            } else if !p.ps2 {
                PortState::NotPs2
            } else if !p.formatted {
                PortState::Unformatted
            } else if p.dir {
                PortState::Ready
            } else if p.full {
                PortState::Full
            } else {
                PortState::NoDirectory
            }
        }

        fn read_index(&mut self, port: i32, _vol: i32) -> Option<Vec<u8>> {
            let p = self.port(port);
            p.files.get(&index_path(self.1)).map(|b| b[..b.len().min(INDEX_SIZE)].to_vec())
        }

        fn write_index(&mut self, port: i32, index: &[u8; INDEX_SIZE]) -> bool {
            let mut p = self.port(port);
            if p.fail_sys {
                p.files.entry(index_path(self.1)).or_default();
                return false;
            }
            p.files.insert(index_path(self.1), index.to_vec());
            true
        }

        fn write_slot(&mut self, port: i32, slot: usize, data: &[u8]) -> bool {
            let mut p = self.port(port);
            if p.fail_write {
                p.files.entry(slot_path(self.1, slot)).or_default();
                return false;
            }
            p.files.insert(slot_path(self.1, slot), data.to_vec());
            true
        }

        fn format(&mut self, port: i32) -> bool {
            let mut p = self.port(port);
            if p.fail_fmt {
                return false;
            }
            p.formatted = true;
            p.dir = false;
            p.files.clear();
            true
        }

        fn make_dir(&mut self, port: i32) -> bool {
            let v = self.1;
            let mut p = self.port(port);
            p.dir = true;
            if p.fail_write {
                p.files.entry(slot_path(v, 0)).or_default();
                return false;
            }
            for s in 0..12 {
                p.files.insert(slot_path(v, s), vec![0; piney_desktop::savesys::slot_size(v)]);
            }
            if p.fail_sys {
                p.files.entry(index_path(self.1)).or_default();
                return false;
            }
            p.files.insert(index_path(v), vec![0; INDEX_SIZE]);
            true
        }
    }
}
