//! The port against the game's own stream code: `stream_fixture.txt`, written
//! by `python3 tools/test_stream_rs.py fixture` from the game's functions run
//! in `tools/eemu.py` (the table lookup and file list, `WaitEnd`, and
//! `DecodeSetup` / `InitScene` / `DecodeFrameSection` /
//! `ccStreamDrawLayerList::Draw` on streams 0, 106, 2, 15's first scene
//! and 6). The files come
//! from the disc; skipped when the image is not there (PINEY_ISO points
//! elsewhere).
//!
//! Bit for bit: the tables and file lists, the skip rule, the draw list
//! (order, layers, parents), per frame the frame number, the end, the notes
//! and every object drawn (name, transparency, `lwMatrix`). To a tolerance:
//! the view's `world_screen` (the port's is glam's, the game's VU0's) and the
//! light directions and colours.

use std::collections::HashMap;
use std::path::PathBuf;

use glam::Vec3;
use piney_data::iso::Iso;
use piney_desktop::view::View;
use piney_stream::file::StreamFile;
use piney_stream::scene::{Loaded, Scene};
use piney_stream::{draw, load, table};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn fixture() -> Vec<Vec<String>> {
    include_str!("stream_fixture.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split_whitespace().map(str::to_string).collect())
        .collect()
}

fn fnv(h: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(h, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3))
}

#[test]
fn tables_and_file_lists_match_the_game() {
    let Some(p) = iso_path() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let mut iso = Iso::open(p).unwrap();
    let volume = iso.volume().unwrap();
    let mut n = 0;
    for f in fixture().iter().filter(|f| f[0] == "table") {
        let num: usize = f[1].parse().unwrap();
        let d = table::Def::read(volume, num, f[2] == "1").unwrap();
        let h = &d.header;
        let got = [h.name.clone(), h.ofs.to_string(), h.size.to_string(), h.kind.to_string(), h.flag.to_string()];
        assert_eq!(&got[..], &f[3..8], "stream {num} header");
        let files = d.files(0);
        let at = f.iter().position(|x| x == "scenes").unwrap();
        let pre = f.iter().position(|x| x == "preload").unwrap();
        let scenes: Vec<String> = files
            .scenes
            .iter()
            .map(|e| format!("{}:{}:{}:{}:{}:{}", e.name, e.kind, e.flag, e.gzip, e.ofs, e.size))
            .collect();
        let want: Vec<String> = f[at + 1..pre].iter().filter(|x| *x != "-").cloned().collect();
        assert_eq!(scenes, want, "stream {num} {} scenes", f[2]);
        let preload: Vec<String> = files.preload.iter().map(|e| e.name.clone()).collect();
        let want: Vec<String> = f[pre + 1..].iter().filter(|x| *x != "-").cloned().collect();
        assert_eq!(preload, want, "stream {num} {} preloads", f[2]);
        n += 1;
    }
    assert_eq!(n, 2 * table::COUNT);
}

/// `RequestStrPlayGH` (`gate_hack_fixture.txt`, `python3
/// tools/test_stream_rs.py gatehack`): the gate hack's scenes and the files
/// read whole first, for every town, the fields `str7000Out` lists and
/// others, crisis or not, in both languages.
#[test]
fn gate_hack_stream_matches_request_str_play_gh() {
    let Some(p) = iso_path() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let mut iso = Iso::open(p).unwrap();
    let volume = iso.volume().unwrap();
    let mut n = 0;
    for line in include_str!("gate_hack_fixture.txt").lines().filter(|l| l.starts_with("gate ")) {
        let f: Vec<&str> = line.split_whitespace().collect();
        let v: Vec<i32> = f[1..5].iter().map(|x| x.parse().unwrap()).collect();
        let (english, crisis, town, field) = (v[0] != 0, v[1] != 0, v[2], v[3]);
        let d = table::Def::read(volume, table::gate_stream(volume), english).unwrap();
        let files = table::gate_hack_files(volume, &d, town, field, crisis).unwrap();
        let at = f.iter().position(|x| *x == "scenes").unwrap();
        let pre = f.iter().position(|x| *x == "preload").unwrap();
        let scenes: Vec<String> = files
            .scenes
            .iter()
            .map(|e| format!("{}:{}:{}:{}:{}:{}", e.name, e.kind, e.flag, e.gzip, e.ofs, e.size))
            .collect();
        assert_eq!(scenes, f[at + 1..pre].to_vec(), "{line}");
        let preload: Vec<&str> = files.preload.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(preload, f[pre + 1..].to_vec(), "{line}");
        n += 1;
    }
    assert_eq!(n, 768);
}

#[test]
fn skips_match_wait_end() {
    let mut n = 0;
    for f in fixture().iter().filter(|f| f[0] == "skip") {
        let v: Vec<u32> = f[1..].iter().map(|x| x.parse().unwrap()).collect();
        let (mode, desk, flag, push, cancel, skipped) = (v[0], v[1], v[2] as i16, v[3], v[4], v[5] != 0);
        assert_eq!(table::skips(flag, push, cancel, mode == 2 && desk == 1), skipped, "{f:?}");
        n += 1;
    }
    assert!(n > 100);
}

/// Stream `num`'s files loaded as the stream does, scene `stem` built.
fn scene(iso: &mut Iso, volume: piney_data::volume::Volume, num: usize, stem: &str) -> (Loaded, Scene) {
    let d = table::Def::read(volume, num, false).unwrap();
    let files = d.files(0);
    let members: Vec<load::Member> =
        files.preload.iter().chain(&files.scenes).map(|e| load::read(iso, &d, e).unwrap()).collect();
    let archive = load::archive(&members.iter().collect::<Vec<_>>()).unwrap();
    let mut loaded = Loaded::default();
    for m in &members {
        loaded.files.push(StreamFile::read(&archive, &m.stem()).unwrap());
    }
    let f = loaded.files.iter().position(|f| f.stem == stem).unwrap_or_else(|| panic!("stream {num}: no {stem}"));
    let s = Scene::new(&loaded, f, 0);
    (loaded, s)
}

#[test]
fn scenes_play_as_the_game_plays_them() {
    let Some(p) = iso_path() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let mut iso = Iso::open(p).unwrap();
    let volume = iso.volume().unwrap();
    let fx = fixture();
    let mut groups: Vec<usize> = Vec::new();
    for f in fx.iter().filter(|f| f[0] == "scene") {
        groups.push(f[1].parse().unwrap());
    }
    let (mut frames, mut worst_cam, mut worst_light) = (0, 0f64, 0f64);
    for &num in &groups {
        let lines: Vec<&Vec<String>> =
            fx.iter().filter(|f| f[0] != "table" && f[0] != "skip" && f[1] == num.to_string()).collect();
        let head = lines.iter().find(|f| f[0] == "scene").unwrap();
        let (loaded, mut sc) = scene(&mut iso, volume, num, &head[2]);
        let file = &loaded.files[sc.file];
        let name = |obj: u32| file.index_name(obj).unwrap_or_else(|| "?".to_string());
        // A name as the fixture's lines write it, one word.
        let word = |n: String| n.replace('%', "%25").replace(' ', "%20");
        // The draw list.
        assert_eq!(sc.frame_end.to_string(), head[4], "stream {num} frameEnd");
        let want: Vec<(String, String, String)> =
            lines.iter().filter(|f| f[0] == "draw").map(|f| (f[3].clone(), f[4].clone(), f[5].clone())).collect();
        let got: Vec<(String, String, String)> = sc
            .walk()
            .map(|(l, i)| {
                let n = &sc.nodes[i];
                (l.to_string(), word(name(n.obj)), n.parent.map_or("-".to_string(), |p| word(name(sc.nodes[p].obj))))
            })
            .collect();
        assert_eq!(got, want, "stream {num} draw list");
        // The frames.
        let by_step: HashMap<(&str, u32), &Vec<String>> =
            lines.iter().filter(|f| f.len() > 3).map(|f| ((f[0].as_str(), f[3].parse().unwrap_or(0)), *f)).collect();
        sc.decode(&loaded);
        let mut notes = std::mem::take(&mut sc.notes);
        let mut k = 1u32;
        while let Some(f) = by_step.get(&("frame", k)) {
            let mut h = 0xcbf2_9ce4_8422_2325u64;
            let mut drawn = 0;
            let mut first = None;
            for (_, i) in sc.walk() {
                let Some(tp) = sc.visible(i) else { continue };
                let w = sc.world(i);
                first.get_or_insert(w);
                h = fnv(h, name(sc.nodes[i].obj).as_bytes());
                h = fnv(h, &tp.to_bits().to_le_bytes());
                for c in w {
                    for x in c {
                        h = fnv(h, &x.to_le_bytes());
                    }
                }
                drawn += 1;
            }
            let note_text: Vec<String> =
                notes.iter().rev().map(|n| format!("{}:{:x}:{}", n.frame, n.event, n.param)).collect();
            let note_text = if note_text.is_empty() { "-".to_string() } else { note_text.join(" ") };
            let got = format!("{} {:x} {} {:016x} notes {}", sc.frame_now, sc.state & 0x14, drawn, h, note_text);
            assert_eq!(got, f[4..].join(" "), "stream {num} step {k}");
            // The effect nodes that draw: pattern, place, scale, turn and
            // transparency, hashed in sorted order.
            let mut effs: Vec<Vec<u8>> = sc
                .effs
                .iter()
                .enumerate()
                .filter(|(_, e)| e.pattern != piney_stream::scene::PATTERN_STOP)
                .map(|(j, e)| {
                    let w = sc.eff_world(j);
                    let mut b = e.pattern.to_le_bytes().to_vec();
                    for x in [w[3][0], w[3][1], w[3][2], e.scale[0], e.scale[1], e.rotate, e.transparency] {
                        b.extend(x.to_le_bytes());
                    }
                    b
                })
                .collect();
            effs.sort();
            match by_step.get(&("eff", k)) {
                Some(l) => {
                    let mut he = 0xcbf2_9ce4_8422_2325u64;
                    for e in &effs {
                        he = fnv(he, e);
                    }
                    assert_eq!(
                        format!("{} {:016x}", effs.len(), he),
                        l[4..].join(" "),
                        "stream {num} step {k} effects"
                    );
                }
                None => {
                    assert!(effs.is_empty(), "stream {num} step {k}: {} effects the game does not draw", effs.len())
                }
            }
            // The camera.
            let c = by_step[&("camera", k)];
            let mut view = View::default();
            view.set_camera(&sc.camera);
            let ws = view.world_screen().to_cols_array();
            let game: Vec<f64> = c[4..20].iter().map(|x| x.parse().unwrap()).collect();
            for (j, x) in ws.iter().enumerate() {
                let want = game[j];
                // Relative to the column's size: glam's inverse leaves a
                // few 1e-4 where VU0 has an exact 0 beside entries of 1e3.
                let scale = game[j / 4 * 4..j / 4 * 4 + 4].iter().fold(1f64, |a, v| a.max(v.abs()));
                let err = (f64::from(*x) - want).abs() / scale;
                worst_cam = worst_cam.max(err);
                assert!(err < 1e-4, "stream {num} step {k} world_screen[{j}] {x} vs {want}");
            }
            // The light at the first object drawn.
            if let Some(l) = by_step.get(&("light", k)) {
                let v: Vec<f64> = l[5..].iter().map(|x| x.parse().unwrap()).collect();
                let at = first.map(|w| piney_stream::scene::to_mat4(&w).w_axis.truncate()).unwrap_or(Vec3::ZERO);
                for (slot, s) in draw::light_slots(&sc, at).iter().enumerate() {
                    let Some((toward, colour)) = s else { continue };
                    for c in 0..3 {
                        let e = (f64::from(toward[c]) - v[4 * c + slot]).abs();
                        let ec = (f64::from(colour[c]) - v[16 + 4 * slot + c]).abs();
                        worst_light = worst_light.max(e).max(ec);
                        assert!(
                            e < 1e-4 && ec < 1e-5,
                            "stream {num} step {k} light slot {slot}: toward {toward:?} colour {colour:?} vs {:?}",
                            (v[slot], v[4 + slot], v[8 + slot], &v[16 + 4 * slot..19 + 4 * slot])
                        );
                    }
                }
                for c in 0..3 {
                    assert!((f64::from(sc.ambient[c]) - v[28 + c]).abs() < 1e-6, "stream {num} step {k} ambient");
                }
            }
            frames += 1;
            if sc.ended() {
                break;
            }
            sc.decode(&loaded);
            notes = std::mem::take(&mut sc.notes);
            k += 1;
        }
        assert!(!by_step.contains_key(&("frame", k + 1)), "stream {num}: the port ended at step {k}");
    }
    eprintln!("{frames} frames; worst world_screen error {worst_cam:.2e}, light {worst_light:.2e}");
    assert_eq!(groups, vec![0, 106, 2, 15, 6]);
}

/// Stream 2's ribbons are morphed (`ccMorpher`, `F_Morpher`): at frame 822
/// each of the four draws with its two targets, whose weights sum to 1, and
/// the blend moves the positions away from the base shape.
#[test]
fn ribbons_are_morphed() {
    let Some(iso) = iso_path() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let mut iso = Iso::open(iso).unwrap();
    let mut s = piney_stream::Stream::new(&mut iso, 2).unwrap();
    let pad = piney_input::Pad::default();
    let mut frame = piney_draw::Frame::new();
    for _ in 0..822 {
        frame = s.step(&pad);
        s.take_requests();
    }
    let archive = s.archive();
    let morphed: Vec<&piney_draw::ModelDraw> = frame
        .cmds
        .iter()
        .filter_map(|c| match c {
            piney_draw::Cmd::Model(m) if !m.morph.is_empty() => Some(&**m),
            _ => None,
        })
        .collect();
    assert_eq!(morphed.len(), 4);
    for m in morphed {
        let sum: f32 = m.morph.iter().map(|t| t.1).sum();
        assert_eq!(m.morph.len(), 2);
        assert!((sum - 1.0).abs() < 1e-6, "{:?}", m.morph);
        let ccs = piney_data::ccs::Ccs::parse(archive.inflate_named(&m.file).unwrap()).unwrap();
        let models = piney_data::model::models(&ccs).unwrap();
        let find = |o: u32| models.iter().find(|x| x.object == o).unwrap();
        let targets: Vec<_> = m.morph.iter().map(|&(t, w)| (find(t), w)).collect();
        let base = find(m.model);
        let blended = base.morph(0, &targets).unwrap();
        assert_ne!(blended, base.mmats[0].positions, "{}", ccs.object_name(m.model).unwrap_or("?"));
    }
}

/// Outbreak's and Quarantine's streams carry a PCM track a voice language
/// (each Pcm chunk and F_Pcm record its language; `DecodeF_Pcm`, OUT
/// 0x0014d240, reads past the other): the opening (Outbreak's 52,
/// Quarantine's 77) plays its sound through all its frames, about 1600
/// stereo samples a frame, on either voice, and the two differ.
#[test]
fn later_volumes_play_their_voice_track() {
    use piney_input::Pad;
    use piney_stream::{Options, Stream};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work");
    for (disc, num) in [("outbreak/outbreak.iso", 52), ("quarantine/quarantine.iso", 77)] {
        let path = root.join(disc);
        if !path.exists() {
            eprintln!("skipped: no {disc}");
            continue;
        }
        let mut tracks = Vec::new();
        for english in [false, true] {
            let mut iso = Iso::open(&path).unwrap();
            let mut s = Stream::with_options(&mut iso, num, Options { english, ..Options::default() }).unwrap();
            let pad = Pad::default();
            let (mut frames, mut pcm) = (0u32, Vec::new());
            while !s.done() && frames < 20_000 {
                s.step(&pad);
                frames += 1;
                for r in s.take_requests() {
                    if let piney_stream::Request::Pcm(p) = r {
                        pcm.extend(p);
                    }
                }
            }
            let per_frame = pcm.len() as f64 / 2.0 / f64::from(frames);
            assert!((1500.0..1700.0).contains(&per_frame), "{disc} {num} english {english}: {per_frame:.1} a frame");
            tracks.push(pcm);
        }
        let n = tracks[0].len().min(tracks[1].len());
        assert_ne!(tracks[0][..n], tracks[1][..n], "{disc} {num}: the two voices are the same");
    }
}
