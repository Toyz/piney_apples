//! Answers `tools/test_effect_draw_rs.py`: `ccEff::Draw` and `mc_DrawEff`
//! as the port runs them ([`Eff::packet`], [`Eff::render`]) on the sprites
//! it is sent, one JSON line each, so the test can compare them with the
//! game's own `Draw` in eemu and `mc_DrawEff` in `tools/vu.py`.
//!
//! ```text
//! cargo build -p piney-effect --example draw_probe
//! draw_probe ISO < requests
//! ```
//!
//! Numbers are hex; floats travel as their bit patterns, matrices as 16
//! words, column by column. Requests, one a line:
//! - `view WV.. WS..`: the view's `world_view` and `world_screen`; answers
//!   the `fview` the port derives.
//! - `env NEAR FAR FOGNEAR FOGFAR NEARRATE FARRATE COLOUR TEX1 ZBUF`: the
//!   view's w range, `ccDrawEnv::SetFog`'s arguments, the draw
//!   environment's TEX1 and ZBUF; answers fMin, fMax, fogA, fogB, the
//!   colour.
//! - `eff FILE CHUNK PAT X0 Y0 X1 Y1 PX PY PZ PW SX SY ROT COLOUR TRANSP WH
//!   ALPHA TEST FLAG PRIM`: a `ccEff` of Eff chunk CHUNK of effect file
//!   FILE with those fields drawn with pattern PAT; answers the packet (or
//!   null) and the primitives `render` puts in a layer.
//! - `clump FILE NAME M.. ALPHA`: the CMP_ object NAME of effect file FILE
//!   at matrix M after `SetTransparency(ALPHA)`; answers each model drawn:
//!   its Obj and MDL_ names, world matrix and transparency.
//! - `anm FILE NAME STEPS SPEED M.. ALPHA`: the ANM_ object NAME stepped
//!   STEPS times at SPEED, at matrix M with localtp ALPHA; answers the same.

use std::io::BufRead;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_desktop::layers::Layers;
use piney_draw::{AlphaTest, Cmd, PrimKind, TexRef};
use piney_effect::Effects;
use piney_effect::draw::Camera;
use piney_effect::eff::Eff;
use piney_effect::files::ObjRef;
use piney_effect::nodes;
use piney_effect::sprite::{self, Packet};
use piney_world::pose::Play;

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn list<T: ToString>(v: &[T]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn mat(w: &[&str]) -> [[u32; 4]; 4] {
    std::array::from_fn(|c| std::array::from_fn(|r| hex(w[4 * c + r]) as u32))
}

fn packet(p: &Option<Packet>) -> String {
    let Some(p) = p else { return "null".into() };
    let gs = match &p.gs {
        None => "null".into(),
        Some(g) => {
            let verts: Vec<String> =
                g.verts.iter().map(|v| list(&[v.s, v.t, u32::from(v.x), u32::from(v.y), v.z])).collect();
            format!(
                "{{\"alpha\": {}, \"test\": {}, \"tex1\": {}, \"clamp\": {}, \"tex0fmt\": {}, \"tex\": {}, \"rgbaq\": {}, \
                 \"fogcol\": {}, \"zbuf\": {}, \"prim\": {}, \"fog\": {}, \"verts\": [{}]}}",
                g.alpha,
                g.test,
                g.tex1,
                g.clamp,
                g.tex.map_or(0, |t| t.tex0_format()),
                g.tex.map_or("null".into(), |t| list(&[t.texture, t.clut])),
                g.rgbaq,
                g.fog_color,
                g.zbuf,
                g.prim,
                g.fog,
                verts.join(", ")
            )
        }
    };
    format!("{{\"key\": {}, \"m\": {}, \"gs\": {}}}", p.key, list(&p.matrix.concat()), gs)
}

fn prims(layers: Layers) -> String {
    let out: Vec<String> = layers
        .flatten()
        .iter()
        .map(|c| {
            let Cmd::Prim(p) = c else { return "\"model\"".into() };
            let s = &p.state;
            let atest = match s.alpha_test {
                AlphaTest::Off => "null".into(),
                AlphaTest::On { method, reference, fail } => list(&[method as u32, u32::from(reference), fail as u32]),
            };
            let tex = match &s.texture {
                None => "null".into(),
                Some(t) => {
                    let TexRef::Ccs { file, texture, clut } = &t.tex else { panic!("not a scene file texture") };
                    format!(
                        "[\"{file}\", {texture}, {clut}, {}, {}, {}, {}]",
                        t.func as u32,
                        u32::from(t.use_alpha),
                        matches!(t.filter, piney_draw::Filter::Linear) as u32,
                        matches!(t.wrap, piney_draw::Wrap::Clamp) as u32
                    )
                }
            };
            let verts: Vec<String> = p
                .verts
                .iter()
                .map(|v| list(&[v.x.to_bits(), v.y.to_bits(), v.z, v.u.to_bits(), v.v.to_bits(), u32::from_le_bytes(v.rgba.0)]))
                .collect();
            format!(
                "{{\"kind\": {}, \"gouraud\": {}, \"blend\": {}, \"atest\": {atest}, \"depth\": [{}, {}], \"tex\": {tex}, \
                 \"verts\": [{}]}}",
                match p.kind {
                    PrimKind::Sprite => 6,
                    PrimKind::Triangles => 3,
                    PrimKind::Strip => 4,
                    PrimKind::Fan => 5,
                },
                u32::from(p.gouraud),
                s.blend.map_or("null".into(), |b| b.to_reg().to_string()),
                s.depth.test as u32,
                u32::from(s.depth.write),
                verts.join(", ")
            )
        })
        .collect();
    format!("[{}]", out.join(", "))
}

fn main() {
    let iso_path = std::env::args().nth(1).unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(&iso_path).unwrap();
    let archive = Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
    let mut fx = Effects::new(&archive, iso.volume().unwrap()).unwrap();
    // The magic portal's file after the effect files (as the field adds
    // it): its circle and its two animations are drawn the effects' way.
    fx.assets.add_file(&archive, piney_effect::portal::FILE).unwrap();
    let mut camera = Camera::default();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let w: Vec<&str> = line.split_whitespace().collect();
        if w.is_empty() {
            continue;
        }
        let n = |i: usize| hex(w[i]) as u32;
        match w[0] {
            "view" => {
                camera.world_view = mat(&w[1..17]);
                camera.world_screen = mat(&w[17..33]);
                println!("{{\"fview\": {}}}", list(&sprite::fview(&camera.world_view).concat()));
            }
            "env" => {
                let e = &mut camera.env;
                e.near = n(1);
                e.far = n(2);
                e.set_fog(n(3), n(4), n(5), n(6), n(7));
                e.tex1 = hex(w[8]);
                e.zbuf = hex(w[9]);
                println!("{{\"fog\": {}}}", list(&[e.fog_min, e.fog_max, e.fog_a, e.fog_b, e.fog_color]));
            }
            "eff" => {
                let (file, chunk, pat) = (n(1) as usize, n(2) as usize, n(3) as u16);
                let c = &fx.assets.effs[file][chunk];
                let e = Eff {
                    file,
                    chunk,
                    x0: n(4),
                    y0: n(5),
                    x1: n(6),
                    y1: n(7),
                    pos: [n(8), n(9), n(10), n(11)],
                    scale_x: n(12),
                    scale_y: n(13),
                    rotate: n(14),
                    color: n(15),
                    pat_num: c.pat_num(),
                    zoffs: c.zoffs,
                    transparency: n(16),
                    wh: n(17),
                    alpha: hex(w[18]),
                    test: hex(w[19]),
                    flag: n(20) as u16,
                    prim: n(21) as u16,
                    clut: None,
                };
                let p = e.packet(&fx.assets, pat, &camera);
                let mut layers = Layers::default();
                e.render(&fx.assets, &mut layers, 20, pat, &camera);
                println!("{{\"packet\": {}, \"prims\": {}}}", packet(&p), prims(layers));
            }
            "clump" | "anm" => {
                let file = n(1) as usize;
                let f = &fx.assets.files[file];
                let obj = ObjRef { file, object: f.ccs.find_object(w[2]).unwrap() };
                let draws = if w[0] == "clump" {
                    nodes::clump(&fx.assets, obj, &mat(&w[3..19]), n(19))
                } else {
                    let mut play = Play::new(f, w[2]).unwrap();
                    play.frame_spd = n(4);
                    for _ in 0..n(3) {
                        play.forward(f);
                    }
                    nodes::anm(&fx.assets, obj, &play, &mat(&w[5..21]), n(21))
                };
                let name = |o: u32| f.ccs.object_name(o).unwrap_or("");
                let out: Vec<String> = draws
                    .iter()
                    .map(|d| {
                        format!(
                            "[\"{}\", \"{}\", {}, {}]",
                            name(d.obj),
                            name(d.model),
                            list(&d.world.concat()),
                            d.alpha
                        )
                    })
                    .collect();
                println!("[{}]", out.join(", "));
            }
            other => panic!("unknown request {other}"),
        }
    }
}
