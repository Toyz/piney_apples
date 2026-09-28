//! Answers `tools/test_map_rs.py` (`map_probe ISO < requests`): the minimap
//! (`piney_world::map`) of Mac Anu and Dun Loireag, of a field and of a
//! dungeon, frame by frame, one JSON line a request. Requests: `town`,
//! `tframe`, `field`, `fframe`, `dungeon`, `droom`, `dframe`, `modes`,
//! `showmap`, `dshow`, `dset`, `dsquares`; numbers hex, floats their bits.
//! What was sent: `["send", spr, sub, packets]` (each packet's fields and the
//! corners `MakePacketStr` writes, null when culled) or `["text", spr, bytes,
//! dx, dy, rgba, transp]`.

use std::io::BufRead;
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_world::area::{Scene, WorldMan};
use piney_world::dungeon_area::DungeonArea;
use piney_world::ee::ONE;
use piney_world::field_area::{DEF_SE, FieldArea};
use piney_world::map::sprite::{Out, Place, corners};
use piney_world::map::{self, dungeon, field, town};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    format!("[{}]", v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

/// What a frame sent, each packet with its corners through `place`.
fn outs(o: &[Out], place: &dyn Fn(u8) -> Option<Place>) -> String {
    list(o.iter().map(|o| match o {
        Out::Send(s) => {
            let p = place(s.spr);
            let packets = list(s.packets.iter().map(|k| {
                let c = p.as_ref().and_then(|p| corners(k, &p.view, p.tex_h));
                let c = match c {
                    Some((cs, rgba)) => {
                        format!("[{}, {}]", list(cs.iter().map(|c| list([c.x, c.y, c.u, c.v]))), list(rgba))
                    }
                    None => "null".into(),
                };
                format!(
                    "[{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}]",
                    k.code,
                    k.transp,
                    k.cx,
                    k.cy,
                    k.rot,
                    k.dx,
                    k.dy,
                    k.sx,
                    k.sy,
                    k.su,
                    k.sv,
                    k.wu,
                    k.wv,
                    k.wi,
                    list(k.rgba),
                    k.ctrl,
                    c
                )
            }));
            format!("[\"send\", {}, {}, {packets}]", s.spr, u8::from(s.sub))
        }
        Out::Text(t) => format!(
            "[\"text\", {}, {}, {}, {}, {}, {}]",
            t.spr,
            list(t.text.iter().copied()),
            t.dx,
            t.dy,
            list(t.rgba),
            t.transp
        ),
    }))
}

fn icons(t: &town::TownMap) -> String {
    list(t.icons.iter().map(|i| list([i.x, i.y, i.u, i.v, i.tmp[0], i.tmp[1], i.tmp2[0], i.tmp2[1], i.alpha])))
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let volume = iso.volume().unwrap();
    let mut tmap: Option<town::TownMap> = None;
    let mut fmap: Option<field::FieldMap> = None;
    let mut darea: Option<DungeonArea> = None;
    let tables = piney_data::area::AreaTables::of(iso.volume().unwrap());
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        let Some(&cmd) = w.first() else { continue };
        let n = |i: usize| hex(w[i]);
        match cmd {
            "town" => {
                // The town by its file: town01(d) Mac Anu, town02(d) Dun
                // Loireag, town03(d) Carmina Gade.
                let no = match &w[1][..6] {
                    "town02" => 1,
                    "town03" => 2,
                    _ => 0,
                };
                let t = town::TownMap::open(&archive, volume, no, w[1]).unwrap();
                println!("{{\"icons\": {}}}", icons(&t));
                tmap = Some(t);
            }
            "tframe" => {
                let t = tmap.as_mut().unwrap();
                // Then, while the flag race runs, "race" and each racer as
                // its x, y and fade ("-" for none).
                let race = (w.get(7) == Some(&"race")).then(|| {
                    std::array::from_fn(|i| match w[8 + 3 * i] {
                        "-" => None,
                        _ => Some(town::Racer { pos: [n(8 + 3 * i), n(9 + 3 * i), 0, ONE], fade: n(10 + 3 * i) }),
                    })
                });
                let inp = town::Input {
                    mode: n(1) as i32,
                    alpha: n(2),
                    pos: [n(3), n(4), n(5), ONE],
                    dirc: [0, 0, n(6), 0],
                    race,
                };
                let o = t.draw(&inp);
                let t = tmap.as_ref().unwrap();
                println!("{{\"outs\": {}, \"icons\": {}}}", outs(&o, &|s| t.place(s).cloned()), icons(t));
            }
            "field" => {
                let params = piney_data::field::Params {
                    seed: n(1),
                    field_type: n(2),
                    weather: n(3),
                    ground: n(4),
                    object: n(5),
                    event: n(6) as i32,
                    protect: false,
                    skip_init: false,
                };
                let a = FieldArea::new(&archive, params, DEF_SE[params.field_type as usize]).unwrap();
                let m = field::FieldMap::new(&archive, volume, &a.field, &a.file).unwrap();
                let two = list(
                    m.fobj2.iter().map(|o| list([o.kind, o.idx as u32, o.mx as u32, o.my as u32, o.wp[0], o.wp[1]])),
                );
                let one =
                    list(a.field.objects.iter().filter(|o| o.kind.number() == 3 || o.kind.number() == 4).map(|o| {
                        list([o.x, o.y, o.kind.number(), o.index as u32, o.cell[0] as u32, o.cell[1] as u32])
                    }));
                let mut rows = Vec::new();
                for y in 0..80usize {
                    for x in 0..80usize {
                        rows.push(m.pixels[(255 - y) * 256 + 79 - x]);
                    }
                }
                println!(
                    "{{\"fobj2\": {two}, \"fobj\": {one}, \"water\": {}, \"dungeon\": {}, \"map\": {}, \"texels\": {}, \"type\": {}}}",
                    m.water,
                    list(m.dungeon),
                    list(a.field.map.iter().copied()),
                    list(rows),
                    m.field_type
                );
                fmap = Some(m);
            }
            "fframe" => {
                let m = fmap.as_mut().unwrap();
                m.map_flag = n(9) != 0;
                let count = n(10) as usize;
                let mut circles: Vec<field::Circle> = (0..count)
                    .map(|k| field::Circle {
                        mx: n(11 + 4 * k) as i32,
                        my: n(12 + 4 * k) as i32,
                        fading: n(13 + 4 * k) != 0,
                        alpha: n(14 + 4 * k) as i32,
                    })
                    .collect();
                let mut inp = field::Input {
                    mode: n(1) as i32,
                    alpha: n(2),
                    pos: [n(3), n(4), n(5), ONE],
                    dirc: [0, 0, n(6), 0],
                    event_area: n(7) as i32,
                    fountain: n(8) != 0,
                    circles: &mut circles,
                };
                let o = m.draw(&mut inp);
                let m = fmap.as_ref().unwrap();
                println!(
                    "{{\"outs\": {}, \"circles\": {}}}",
                    outs(&o, &|s| m.place(s).cloned()),
                    list(circles.iter().map(|c| c.alpha))
                );
            }
            "dungeon" => {
                let d = |i: usize| -> i64 { w[i].parse().unwrap() };
                let wm = WorldMan {
                    field_seed: 0,
                    dungeon_seed: [d(1) as u32, d(2) as u32, d(3) as u32],
                    field_type: d(4) as u32,
                    weather: d(5) as u32,
                    ground: 0,
                    object: 0,
                    event: d(9) as i32,
                    protect: false,
                    level_max: d(6) as u32,
                    room_max: d(7) as u32,
                    hack: d(8) as u32,
                    field_model: 0,
                    dungeon_type: [0; 3],
                    words: [0; 3],
                    ..WorldMan::default()
                };
                let mut scene = Scene::init();
                scene.area = 2;
                scene.field = d(9) as i32;
                scene.dungeon = d(10) as i32;
                scene.floor = 0;
                scene.block = 0;
                scene.server = d(11) as i32;
                let mut a = DungeonArea::new(&archive, &wm, &scene).unwrap();
                let edit = if a.event != 0 { a.tables.edit(a.event, scene.dungeon) } else { None };
                let floors = dungeon::floors_of(&a, edit);
                let file = a.file.clone();
                let m = dungeon::DungeonMap::new(&archive, volume, &file, i32::from(a.dtype), floors).unwrap();
                let sizes = list(m.floors.iter().map(|f| {
                    list(f.rooms.iter().map(|r| format!("[{}, {}, {}]", u8::from(r.made), r.size, u8::from(r.special))))
                }));
                println!(
                    "{{\"dtype\": {}, \"event\": {}, \"file\": \"{}\", \"clut\": {}, \"tex\": {}, \"fog_va\": {}, \
                     \"fog_index\": {}, \"fog\": {}, \"rooms\": {sizes}}}",
                    a.dtype,
                    a.event,
                    file.stem,
                    a.tex_clut.clut_type,
                    a.tex_clut.tex_type,
                    a.fog_table.0,
                    a.fog_table.1,
                    a.fog.packed_colour()
                );
                a.map = Some(Box::new(m));
                darea = Some(a);
            }
            "droom" => {
                let a = darea.as_mut().unwrap();
                let (f, i) = (n(1) as usize, n(2) as usize);
                a.set_room(f, i);
                let mut m = a.map.take().unwrap();
                let new = m.make_mini_map(&mut a.hits, f, i);
                if new {
                    m.paint(f, false);
                }
                let mut sq = Vec::new();
                for x in 0..dungeon::SIDE {
                    for y in 0..dungeon::SIDE {
                        let v = m.square(f, x, y);
                        if v != 0 {
                            sq.push(format!("[{x}, {y}, {v}]"));
                        }
                    }
                }
                println!("{{\"new\": {}, \"squares\": [{}]}}", u8::from(new), sq.join(", "));
                a.map = Some(m);
            }
            "dframe" => {
                let a = darea.as_mut().unwrap();
                let f = n(1) as usize;
                let nc = n(11) as usize;
                let ent = |at: usize| dungeon::Ent {
                    id: n(at) as i32,
                    floor: n(at + 1) as i32,
                    block: n(at + 2) as i32,
                    pos: [n(at + 3), n(at + 4)],
                    param2: n(at + 5) as i32,
                };
                let circles: Vec<dungeon::Ent> = (0..nc).map(|k| ent(12 + 6 * k)).collect();
                let ng_at = 12 + 6 * nc;
                let ng = n(ng_at) as usize;
                let gims: Vec<dungeon::Ent> = (0..ng).map(|k| ent(ng_at + 1 + 6 * k)).collect();
                let m = a.map.as_mut().unwrap();
                m.map_hide = n(9) as i32;
                let inp = dungeon::Input {
                    mode: n(2) as i32,
                    special_room: n(3) as i32,
                    alpha: n(4),
                    pos: [n(5), n(6), n(7), ONE],
                    dirc: [0, 0, n(8), 0],
                    event_hold: n(10) != 0,
                    circles: &circles,
                    gims: &gims,
                };
                let d = m.draw(f, &inp);
                let m = a.map.as_ref().unwrap();
                let sum: u64 = m.pixels.iter().enumerate().map(|(i, &p)| (i as u64 + 1) * u64::from(p)).sum();
                println!(
                    "{{\"outs\": {}, \"map_status\": {}, \"texsum\": {sum}}}",
                    outs(&d.outs, &|s| m.place(s).cloned()),
                    u8::from(d.map_status)
                );
            }
            "modes" => {
                let mut m = map::MapModes { town: n(2) as i32, field: n(3) as i32, dungeon: n(4) as i32 };
                m.change(n(1) as i32);
                println!("{}", list([m.town, m.field, m.dungeon]));
            }
            "showmap" => {
                let event = n(1) as i32;
                let model = if event == 0 { 0 } else { tables.event_area_info(event, false).map_or(0, |i| i.model) };
                let m = fmap.as_mut().unwrap();
                m.map_flag = false;
                let done = map::show_field_map(m, model);
                println!("[{}, {}]", u8::from(done), u8::from(m.map_flag));
            }
            "dshow" => {
                let a = darea.as_mut().unwrap();
                a.level = n(1) as usize;
                let done = dungeon::show_map_step(a, [n(2), n(3), 0, ONE]);
                println!("[{}, {}]", u8::from(done), list(a.room_at.map_or([-1, -1], |(f, i)| [f as i32, i as i32])));
            }
            "dset" => {
                darea.as_mut().unwrap().set_room(n(1) as usize, n(2) as usize);
                println!("null");
            }
            "dsquares" => {
                let m = darea.as_ref().unwrap().map.as_ref().unwrap();
                let f = n(1) as usize;
                let mut sq = Vec::new();
                for x in 0..dungeon::SIDE {
                    for y in 0..dungeon::SIDE {
                        let v = m.square(f, x, y);
                        if v != 0 {
                            sq.push(format!("[{x}, {y}, {v}]"));
                        }
                    }
                }
                println!("[{}]", sq.join(", "));
            }
            _ => println!("null"),
        }
    }
}
