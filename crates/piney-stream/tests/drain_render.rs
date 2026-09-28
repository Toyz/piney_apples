//! Stream 5's drain drawn on the GPU (`piney-gs`): at frame 2694 a gold
//! ring (`str0120e` object 0x20c) fades in over the figure at
//! transparency 0.02. Its vertex alpha, `trunc(0x80 * t)`, is 2 and its
//! AREF, `trunc(aref * t)`, is 2 too, so every pixel passes and writes Z:
//! the ring, all but invisible, erases the figure drawn after it (layer 4)
//! from the top down. The renderer once interpolated that alpha as a float
//! that fell a hair under 2 at some pixels, and the erased hat came out
//! dithered with what was behind it. Skipped without the disc image or a
//! GPU.

use std::path::PathBuf;

use piney_data::iso::Iso;
use piney_input::Pad;
use piney_stream::{Options, Stream};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

/// The mean absolute Laplacian of the grey level over a rectangle: high
/// where neighbouring pixels jump, as a dither does.
fn roughness(rgba: &[u8], width: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> f64 {
    let g = |x: u32, y: u32| {
        let i = ((y * width + x) * 4) as usize;
        (f64::from(rgba[i]) + f64::from(rgba[i + 1]) + f64::from(rgba[i + 2])) / 3.0
    };
    let mut sum = 0.0;
    for y in y0..y1 {
        for x in x0..x1 {
            sum += (4.0 * g(x, y) - g(x - 1, y) - g(x + 1, y) - g(x, y - 1) - g(x, y + 1)).abs();
        }
    }
    sum / f64::from((x1 - x0) * (y1 - y0))
}

#[test]
fn the_drain_ring_erases_the_figure_whole() {
    let Some(iso_path) = iso_path() else { return };
    let mut iso = Iso::open(&iso_path).unwrap();
    let mut s = Stream::with_options(&mut iso, 5, Options::default()).unwrap();
    let data = std::sync::Arc::new(piney_data::archive::Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let Ok(mut gs) = piney_gs::Gs::headless(piney_gs::Assets::new(data)) else { return };
    gs.set_overlay(Some(s.archive()));
    let pad = Pad::default();
    while !s.done() && s.frame() < 2694 {
        let frame = s.step(&pad);
        s.take_requests();
        if s.frame() + 4 >= 2694 {
            gs.render(&frame);
        }
    }
    assert_eq!(s.frame(), 2694);
    let rgba = gs.read_back();
    let (w, _) = gs.target_size();
    // The hat's place: 74 with the dither, 24 with the hat cleanly gone.
    let r = roughness(&rgba, w, 181, 191, 339, 259);
    assert!(r < 40.0, "the figure's head is dithered: roughness {r:.1}");
}
