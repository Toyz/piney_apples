//! Print what a stream's scene holds at a frame: every node drawn, its
//! transparency, world position and where it lands on the screen. With
//! `--all`, the nodes not drawn too (their `disp` and transparency).
//!
//! ```text
//! cargo run --release -p piney-stream --example stream_probe -- STREAM FRAME [ISO] [--all]
//! ```

use piney_data::iso::Iso;
use piney_desktop::view::View;
use piney_input::Pad;
use piney_stream::Stream;
use piney_stream::scene::to_mat4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().collect();
    let all = args.iter().any(|a| a == "--all");
    args.retain(|a| a != "--all");
    let num: usize = args.get(1).ok_or("STREAM")?.parse()?;
    let target: u32 = args.get(2).ok_or("FRAME")?.parse()?;
    let iso = args.get(3).cloned().unwrap_or_else(|| "work/infection/infection.iso".into());
    let mut iso = Iso::open(iso)?;
    let mut s = Stream::new(&mut iso, num)?;
    let pad = Pad::default();
    while !s.done() && s.frame() < target {
        s.step(&pad);
    }
    let Some(sc) = s.scene() else { return Ok(()) };
    let l = s.loaded();
    let f = &l.files[sc.file];
    println!("frame {} camera {:?} ambient {:?}", s.frame(), sc.camera, sc.ambient);
    for li in &sc.lights {
        println!("light {:?}", li);
    }
    let mut view = View::default();
    view.set_camera(&sc.camera);
    for (layer, nodes) in &sc.draw_list {
        for &i in nodes {
            let n = &sc.nodes[i];
            let tp = sc.transparency(i);
            let drawn = n.disp == 3 && tp > 1.0 / 128.0;
            if !drawn && !all {
                continue;
            }
            let w = to_mat4(&sc.world(i));
            let q = view.to_screen(w) * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
            let model = n.model.map(|(g, m)| format!("{}:{}", l.files[g].stem, l.files[g].name(m).unwrap_or("?")));
            println!(
                "layer {layer} {:24} {}tp {tp:.3} pos {:?} screen ({:.1}, {:.1}, w {:.1}) model {:?}",
                f.name(n.obj).unwrap_or("?"),
                if drawn { String::new() } else { format!("(not drawn: disp {}) ", n.disp) },
                w.w_axis.truncate(),
                q.x / q.w,
                q.y / q.w,
                q.w,
                model
            );
        }
    }
    Ok(())
}
