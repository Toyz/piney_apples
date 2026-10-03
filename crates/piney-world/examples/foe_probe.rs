//! Answers `tools/test_foe_rs.py` (`foe_probe ISO < requests`: `look`, `pose`,
//! `circle`, `draw` - an enemy row's files, its clump posed, the portal's,
//! and `ccChar::Draw`'s decisions) and renders enemies and a magic portal to
//! PNG (`piney_world::foe`): `foe_probe ISO --shot OUT [--rows 130,151,...]
//! [--frame N] [--circle ANIM,N] [--dist D] [--pitch P] [--yaw Y] [--dead
//! ROW] [--flash ROW]`. Numbers hex, floats their bits.

use std::collections::HashMap;
use std::io::BufRead;
use std::sync::Arc;

use glam::Mat4;
use piney_battle::chara::AffectState;
use piney_battle::tables::Tables;
use piney_data::archive::Archive;
use piney_data::field;
use piney_data::iso::Iso;
use piney_data::volume::Volume;
use piney_desktop::layers::Layers;
use piney_draw::Frame;
use piney_world::camera::{VIEW_SCREEN, pos_target};
use piney_world::draw;
use piney_world::ee::{self, F, ONE, V4};
use piney_world::field_area::{self, DEF_SE, FieldArea};
use piney_world::foe::{self, Camera, CharDraw, Circle, EnemyLook, Files, Model};
use piney_world::pose::Play;

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    format!("[{}]", v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

/// Every clump node: [name, matrix bits, transparency bits].
fn nodes(m: &Model, play: &Play, root: Mat4) -> String {
    let worlds = m.worlds(play, root);
    let alphas = m.node_alphas(play);
    let ccs = &m.file().ccs;
    list(m.body.nodes.iter().map(|&n| {
        let w = worlds.get(&n).copied().unwrap_or(root);
        format!(
            "[{:?}, {}, {}]",
            ccs.object_name(n).unwrap_or("?"),
            list(w.to_cols_array().map(f32::to_bits)),
            alphas.get(&n).copied().unwrap_or(1.0).to_bits()
        )
    }))
}

fn play_at(m: &Model, anim: &str, time: u32) -> Option<Play> {
    let mut p = m.play(anim)?;
    p.time = time;
    p.posed = time;
    Some(p)
}

struct Disc {
    archive: Arc<Archive>,
    volume: Volume,
    t: Tables,
}

fn open(path: &str) -> Disc {
    let mut iso = Iso::open(path).unwrap();
    let archive = Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let volume = iso.volume().unwrap();
    let t = Tables::of(volume);
    Disc { archive, volume, t }
}

fn answer(d: &Disc, files: &mut Files, looks: &mut HashMap<i32, EnemyLook>, circle: &mut Option<Circle>, w: &[&str]) {
    let n = |i: usize| hex(w[i]);
    match w.first().copied() {
        Some("look") => {
            let row = n(1) as i32;
            match EnemyLook::load(files, &d.t, row) {
                Ok(l) => {
                    let swaps = list(l.model.swaps.iter().map(|s| format!("[{:?}, {:?}]", s.material, s.clut)));
                    let second = l.second.as_ref().map_or("null".to_string(), |m| format!("{:?}", m.file().stem));
                    let wait = l.clip(6).to_string();
                    let wait2 = l
                        .second
                        .as_ref()
                        .map_or("null".to_string(), |_| format!("{:?}", piney_battle::races::boss_anm_name(&wait)));
                    println!(
                        "{{\"row\": {row}, \"src\": {}, \"file\": {:?}, \"clump\": {:?}, \"swaps\": {swaps}, \
                         \"anim\": {wait:?}, \"second\": {second}, \"anim2\": {wait2}}}",
                        l.src,
                        l.stem(),
                        foe::CLUMP
                    );
                    looks.insert(row, l);
                }
                Err(e) => println!("{{\"row\": {row}, \"error\": {:?}}}", e.to_string()),
            }
        }
        Some("pose") => {
            let row = n(1) as i32;
            if let std::collections::hash_map::Entry::Vacant(v) = looks.entry(row) {
                v.insert(EnemyLook::load(files, &d.t, row).unwrap());
            }
            let l = &looks[&row];
            let cols: [[u32; 4]; 4] = std::array::from_fn(|c| std::array::from_fn(|r| n(4 + 4 * c + r)));
            match play_at(&l.model, w[2], n(3)) {
                Some(p) => println!("{{\"nodes\": {}}}", nodes(&l.model, &p, draw::mat(&cols))),
                None => println!("{{\"error\": \"no animation {}\"}}", w[2]),
            }
        }
        Some("circle") => {
            let c = circle.get_or_insert_with(|| Circle::load(files, d.volume).unwrap());
            let (pos, dirc) = ([n(3), n(4), n(5), ONE], [n(6), n(7), n(8), 0]);
            let root = Circle::root(pos, dirc);
            match play_at(&c.model, w[1], n(2)) {
                Some(p) => println!(
                    "{{\"root\": {}, \"nodes\": {}}}",
                    list(root.iter().flatten()),
                    nodes(&c.model, &p, draw::mat(&root))
                ),
                None => println!("{{\"error\": \"no animation {}\"}}", w[1]),
            }
        }
        Some("index") => {
            // Each animation of a file: the objects its index names, in order.
            let f = files.get(w[1]).unwrap();
            let anims = f.anims.iter().map(|a| format!("[{}, {}]", a.object, list(a.objects.iter().map(|o| o.0))));
            println!("{{\"anims\": {}}}", list(anims));
        }
        Some("draw") => {
            let mut a = AffectState {
                color_cnt: n(4) as i16,
                color_rate: n(5) as i16,
                color: n(6),
                cond_color_cnt: n(7) as i16,
                cond_color_rate: n(8) as i16,
                cond_color: n(9),
                ..AffectState::default()
            };
            let cam = Camera {
                player: [n(18), n(19), n(20), ONE],
                cam: [n(21), n(22), n(23), ONE],
                deg1: n(24) as i16,
                eye: n(25) != 0,
                bounds: field_area::BOUNDS,
                town: n(26) != 0,
                volume: piney_data::volume::Volume::Inf,
            };
            let c: CharDraw = foe::char_draw_parts(
                n(1) as i16,
                n(2) as i32,
                n(3) != 0,
                &mut a,
                [n(10), n(11), n(12), ONE],
                n(13),
                n(14),
                n(15),
                n(16) != 0,
                n(17),
                &cam,
            );
            let fog = c.blend.fog().map_or("null".into(), |f| {
                list([u32::from(f.f), u32::from_le_bytes([f.colour[0], f.colour[1], f.colour[2], 0])])
            });
            println!(
                "{{\"blend\": [{}, {}], \"fog\": {fog}, \"transparency\": {}, \"hide\": {}, \"shadow_alpha\": {}, \
                 \"shadow_length\": {}, \"drawn\": {}, \"shaded\": {}, \"affect\": [{}, {}, {}, {}, {}, {}]}}",
                c.blend.rate,
                c.blend.colour,
                c.transparency,
                u8::from(c.hide),
                c.shadow_alpha,
                c.shadow_length,
                u8::from(c.drawn),
                u8::from(c.shaded),
                a.color_cnt,
                a.color_rate,
                a.color,
                a.cond_color_cnt,
                a.cond_color_rate,
                a.cond_color
            );
        }
        _ => println!("{{\"error\": \"unknown request\"}}"),
    }
}

// the shot ------------------------------------------------------------------------------

struct Shot {
    out: String,
    rows: Vec<i32>,
    frame: u32,
    circle: (String, u32),
    dist: f32,
    pitch: f32,
    yaw: f32,
    dead: Option<i32>,
    flash: Option<i32>,
}

fn at(x: f32, y: f32, z: f32) -> V4 {
    [x.to_bits(), y.to_bits(), z.to_bits(), ONE]
}

fn render(archive: Arc<Archive>, frame: &Frame) -> Result<(u32, u32, Vec<u8>), String> {
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive))?;
    gs.render(frame);
    let (w, h) = gs.target_size();
    Ok((w, h, gs.read_back()))
}

/// `dispEnemy`'s matrix for an enemy at `pos` facing `dirc_z`, `yoffs` 0:
/// the facing's rotation moved to the position.
fn disp_matrix(pos: V4, dirc_z: F) -> [V4; 4] {
    let m = piney_battle::geom::rot_matrix(&piney_battle::geom::unit_matrix(), [0, 0, dirc_z, 0]);
    piney_battle::geom::trans_matrix(&m, [pos[0], pos[1], pos[2], ONE])
}

fn shot(d: &Disc, s: &Shot) -> Result<(), Box<dyn std::error::Error>> {
    let mut files = Files::new(d.archive.clone());
    // Story area 14's field, as WORLD_MAN::GO(1) makes it.
    let params = field::Params {
        seed: 1_420_855,
        field_type: 10,
        weather: 0,
        ground: 1,
        object: 1,
        event: 14,
        protect: false,
        skip_init: false,
    };
    let mut area = FieldArea::new(&d.archive, params, DEF_SE[10])?;
    let heights = area.hits.heights.clone().ok_or("no heights")?;
    let ground = |x: f32, y: f32| ee::f(heights.height(x.to_bits(), y.to_bits()));
    // The portal somewhere on dry ground near the middle, the enemies round
    // it.
    let dry = |cx: f32, cy: f32| {
        (0..16).all(|k| {
            let a = std::f32::consts::TAU * k as f32 / 16.0;
            [400.0f32, 900.0, 1400.0].iter().all(|r| ground(cx + r * a.cos(), cy - 300.0 + r * a.sin()) >= 0.0)
        }) && ground(cx, cy) >= 0.0
    };
    let (cx, cy) = (0..40)
        .flat_map(|i| (0..40).map(move |j| (24000.0 + 600.0 * (i - 20) as f32, 24000.0 + 600.0 * (j - 20) as f32)))
        .filter(|&(x, y)| dry(x, y))
        .min_by(|a, b| {
            let d = |p: &(f32, f32)| (p.0 - 24000.0).hypot(p.1 - 24000.0);
            d(a).total_cmp(&d(b))
        })
        .ok_or("no dry ground")?;
    eprintln!("portal at ({cx}, {cy})");
    let cz = ground(cx, cy);
    let centre = at(cx, cy, cz + 250.0);
    let yaw = s.yaw.to_radians();
    let pitch = s.pitch.to_radians();
    let target = at(cx, cy, cz + 150.0);
    let eye = at(
        cx - s.dist * pitch.cos() * yaw.sin(),
        cy + s.dist * pitch.cos() * yaw.cos(),
        cz + 150.0 + s.dist * pitch.sin(),
    );
    let world_screen = piney_data::anim::vu_mul(&VIEW_SCREEN, &pos_target(eye, target));
    let to_screen = draw::screen(&world_screen);
    let mut layers = Layers::default();
    let player = at(cx, cy + 600.0, ground(cx, cy + 600.0));
    area.set_center(player[0], player[1]);
    area.hits.center = [player[0], player[1]];
    area.draw(&mut layers, to_screen, player, eye, true);
    let lights = area.lights.clone();
    let cam = Camera {
        player,
        cam: eye,
        deg1: 1512,
        eye: false,
        bounds: field_area::BOUNDS,
        town: false,
        volume: piney_data::volume::Volume::Inf,
    };
    // The portal.
    let circle = Circle::load(&mut files, d.volume)?;
    if let Some(p) = play_at(&circle.model, &s.circle.0, s.circle.1 << 8) {
        let t = cam.transparency5(centre, 0, 0, foe::AREA_FAR, foe::AREA_FADE);
        circle.draw(&mut layers, to_screen, &p, centre, [0; 4], t, &lights);
    }
    // The enemies in a row across the view in front of the portal, facing
    // the camera (first on the left).
    let n = s.rows.len().max(1) as f32;
    for (k, &row) in s.rows.iter().enumerate() {
        let look = EnemyLook::load(&mut files, &d.t, row)?;
        let side = (k as f32 - (n - 1.0) / 2.0) * s.dist * 0.6 / n.max(3.0);
        let ahead = 0.3 * s.dist;
        let (x, y) = (cx - side * yaw.cos() - ahead * yaw.sin(), cy - side * yaw.sin() + ahead * yaw.cos());
        let pos = at(x, y, ground(x, y));
        eprintln!("row {row} {} at ({x:.0}, {y:.0}, {:.0})", look.stem(), ee::f(pos[2]));
        // Face the camera: heading 0 faces -y, growing toward +x.
        let dirc = (ee::f(eye[0]) - x).atan2(y - ee::f(eye[1]));
        let clip = look.clip(6).to_string();
        let Some(mut play) = look.play(&clip) else { continue };
        let f = s.frame % (look.model.file().anims[play.anim].frames.max(1));
        play.time = f << 8;
        play.posed = f << 8;
        let mut ch = look_char(&look, pos);
        if s.dead == Some(row) {
            ch.cond.v[piney_battle::param::cond::DEAD] = 2;
        }
        if s.flash == Some(row) {
            ch.affect.color_rate = 40;
            ch.affect.color_cnt = -4;
            ch.affect.color = 0x00ff_ffff;
        }
        let how = foe::char_draw(&mut ch, ONE, true, false, 0, &cam);
        look.draw(&mut layers, to_screen, &play, &disp_matrix(pos, dirc.to_bits()), &how, &lights);
    }
    let mut frame = Frame::new();
    frame.clear = piney_draw::Rgba([0x30, 0x40, 0x60, 0x80]);
    frame.cmds = layers.flatten();
    let (w, h, rgba) = render(d.archive.clone(), &frame)?;
    std::fs::write(&s.out, piney_gs::png::encode(w, h, &rgba))?;
    println!("{}: {w}x{h}, {} commands", s.out, frame.cmds.len());
    Ok(())
}

/// A battle character standing for `look`'s row at `pos`, as `ccChar::Draw`
/// reads one.
fn look_char(look: &EnemyLook, pos: V4) -> piney_battle::Char {
    let base = piney_battle::param::Base { height: look.height, width: look.width, ..Default::default() };
    let mut ch = piney_battle::Char::other(base, piney_battle::param::Elm::default());
    ch.pos = pos;
    ch.condition_num = -1;
    ch
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let iso_path = args.next().unwrap_or_else(|| "work/infection/infection.iso".into());
    let d = open(&iso_path);
    let rest: Vec<String> = args.collect();
    if rest.first().map(String::as_str) == Some("--shot") {
        let mut s = Shot {
            out: rest.get(1).cloned().unwrap_or_else(|| "foe.png".into()),
            rows: vec![130, 151, 185, 189, 219, 268, 67],
            frame: 10,
            circle: ("ANM_xmagcir1".into(), 20),
            dist: 1800.0,
            pitch: 20.0,
            yaw: 0.0,
            dead: None,
            flash: None,
        };
        let mut it = rest.iter().skip(2);
        while let Some(a) = it.next() {
            let v = it.next().ok_or("missing value")?;
            match a.as_str() {
                "--rows" => s.rows = v.split(',').map(str::parse).collect::<Result<_, _>>()?,
                "--frame" => s.frame = v.parse()?,
                "--circle" => {
                    let (name, f) = v.split_once(',').ok_or("--circle ANIM,N")?;
                    s.circle = (name.into(), f.parse()?);
                }
                "--dist" => s.dist = v.parse()?,
                "--pitch" => s.pitch = v.parse()?,
                "--yaw" => s.yaw = v.parse()?,
                "--dead" => s.dead = Some(v.parse()?),
                "--flash" => s.flash = Some(v.parse()?),
                _ => return Err(format!("unknown argument {a}").into()),
            }
        }
        return shot(&d, &s);
    }
    let mut files = Files::new(d.archive.clone());
    let mut looks = HashMap::new();
    let mut circle = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let w: Vec<&str> = line.split_whitespace().collect();
        answer(&d, &mut files, &mut looks, &mut circle, &w);
    }
    Ok(())
}
