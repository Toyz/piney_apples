//! `anim` against `tools/anim.py`, whose values `tools/test_anim.py` checked
//! against the game's own code. The fixture (`anim_fixture.txt`, written by
//! `python3 tools/test_anim.py fixture`) holds only evaluated numbers; the
//! animations and models come from the disc. Skipped when the disc image is
//! not extracted; set PINEY_ISO to point at it elsewhere.
//!
//! Positions, scales, transparencies, morph weights, blended positions,
//! playback times and the notes a step hands on must match exactly; rotation
//! matrices to 1e-6 per element (anim.py prints them as doubles, the port
//! rounds them to f32).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use glam::Mat3;
use piney_data::anim::{self, Animation, MorphTarget};
use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::iso::Iso;
use piney_data::model;
use piney_data::scene::Scene;

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn data_bin() -> Option<Archive> {
    let mut iso = Iso::open(iso_path()?).unwrap();
    Some(Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap())
}

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap()
}

struct File {
    ccs: Ccs,
    scene: Scene,
    animations: HashMap<String, Animation>,
}

impl File {
    fn load(arc: &Archive, member: &str) -> File {
        let ccs = Ccs::parse(arc.inflate_named(member).unwrap()).unwrap();
        let scene = Scene::read(&ccs).unwrap();
        let animations = scene
            .animes
            .iter()
            .map(|an| {
                let a = Animation::read(&ccs, &scene, an).unwrap();
                (ccs.object_name(a.object).unwrap().to_string(), a)
            })
            .collect();
        File { ccs, scene, animations }
    }
}

fn fnv(positions: &[[i16; 3]]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for p in positions {
        for x in p {
            for b in x.to_le_bytes() {
                h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
            }
        }
    }
    h
}

#[test]
fn animations_match_the_reference() {
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let mut files: HashMap<String, File> = HashMap::new();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut worst_rot = 0f64;
    for line in include_str!("anim_fixture.txt").lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        let (kind, member, anime) = (f[0], f[1], f[2]);
        let file = files.entry(member.to_string()).or_insert_with(|| File::load(&arc, member));
        let a = &file.animations[anime];
        *counts.entry(kind).or_default() += 1;
        match kind {
            "pose" => {
                let (object, target, time): (u32, u32, u32) =
                    (f[3].parse().unwrap(), f[4].parse().unwrap(), f[5].parse().unwrap());
                let track = a.tracks.iter().find(|t| t.object == object && t.target == target).expect(line);
                let p = track.at(time);
                let got = [p.pos.x, p.pos.y, p.pos.z, p.scale.x, p.scale.y, p.scale.z, p.alpha].map(f32::to_bits);
                let want: Vec<u32> = f[6..13].iter().map(|s| hex(s)).collect();
                assert_eq!(got.to_vec(), want, "{line}");
                for r in 0..3 {
                    for c in 0..3 {
                        let want: f64 = f[13 + 3 * r + c].parse().unwrap();
                        let d = (p.rot.col(c)[r] as f64 - want).abs();
                        worst_rot = worst_rot.max(d);
                        assert!(d <= 1e-6, "{line}: rot[{r}][{c}] = {}", p.rot.col(c)[r]);
                    }
                }
            }
            "morph" => {
                let (morpher, frame): (u32, u32) = (f[3].parse().unwrap(), f[4].parse().unwrap());
                let got: Vec<(u32, u32)> = a
                    .morph_weights_at(frame)
                    .into_iter()
                    .find(|(m, _)| *m == morpher)
                    .map(|(_, t)| t.iter().map(|(m, w)| (*m, w.to_bits())).collect())
                    .unwrap_or_default();
                let want: Vec<(u32, u32)> = f[5..]
                    .iter()
                    .map(|s| {
                        let (t, w) = s.split_once(':').unwrap();
                        (t.parse().unwrap(), hex(w))
                    })
                    .collect();
                assert_eq!(got, want, "{line}");
            }
            "blend" => {
                let (morpher, frame, mmat): (u32, u32, usize) =
                    (f[3].parse().unwrap(), f[4].parse().unwrap(), f[5].parse().unwrap());
                let models = model::models(&file.ccs).unwrap();
                let find = |obj: u32| models.iter().find(|m| m.object == obj).unwrap();
                let base = find(anim::morphers(&file.ccs).unwrap()[&morpher]);
                let weights = a.morph_weights_at(frame);
                let (_, pairs) = weights.iter().find(|(m, _)| *m == morpher).unwrap();
                let targets: Vec<MorphTarget> = pairs
                    .iter()
                    .map(|&(t, weight)| {
                        let m = find(t);
                        MorphTarget { positions: &m.mmats[mmat].positions, scale: m.scale, weight }
                    })
                    .collect();
                let out = anim::morph(&base.mmats[mmat].positions, base.scale, &targets);
                assert_eq!(format!("{:016x}", fnv(&out)), f[6], "{line}");
            }
            "forward" => {
                let (speed, steps): (u32, usize) = (f[3].parse().unwrap(), f[4].parse().unwrap());
                let mut time = 0;
                let mut got = Vec::with_capacity(steps);
                for _ in 0..steps {
                    let st = a.forward(time, speed);
                    time = st.time;
                    got.push(time | if st.ended { 0x8000_0000 } else { 0 });
                }
                let want: Vec<u32> = f[5..].iter().map(|s| hex(s)).collect();
                assert_eq!(got, want, "{line}");
            }
            "uv" => {
                let (material, time): (u32, u32) = (f[3].parse().unwrap(), f[4].parse().unwrap());
                let m = a.materials.iter().find(|m| m.material == material).expect(line);
                let [u, v] = m.at(time);
                assert_eq!([u.to_bits(), v.to_bits()], [hex(f[5]), hex(f[6])], "{line}");
                assert_eq!(m.offsets(time, 0x1234, 0xfedc).map(u32::from), [hex(f[7]), hex(f[8])], "{line}");
            }
            "notes" => {
                let (time, step) = (hex(f[3]), hex(f[4]));
                let (st, notes) = a.forward_notes(time, step);
                assert_eq!((st.time, st.ended), (hex(f[5]), f[6] == "1"), "{line}");
                let want: Vec<(u32, u32)> = f[7..]
                    .iter()
                    .map(|s| {
                        let (e, p) = s.split_once(':').unwrap();
                        (hex(e), hex(p))
                    })
                    .collect();
                assert_eq!(notes, want, "{line}");
                *counts.entry("note").or_default() += notes.len();
            }
            k => panic!("unknown fixture line {k}"),
        }
    }
    eprintln!("{counts:?}, worst rotation difference {worst_rot:.2e}");
    assert!(counts["pose"] > 300 && counts["morph"] > 10 && counts["blend"] >= 4 && counts["forward"] > 10);
    assert!(counts["uv"] >= 10);
    assert!(counts["notes"] >= 300 && counts["note"] >= 100);
    // every file's animations read, and their poses evaluate, at a few times
    for file in files.values() {
        for a in file.animations.values() {
            for t in [0, a.last() / 2, a.last()] {
                for p in a.poses_at(t) {
                    assert!(p.rot.is_finite() && p.pos.is_finite(), "{}", a.object);
                }
                let _ = anim::world(&file.scene, &a.locals_at(t));
                assert_eq!(a.controllers_at(t).len(), a.tracks.len());
            }
        }
    }
}

#[test]
fn every_animation_reads() {
    let Some(arc) = data_bin() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let (mut animations, mut tracks, mut looping, mut morphs) = (0, 0, 0, 0);
    let (mut notes, mut with_notes) = (0, 0);
    let mut worst = 0f32;
    for m in arc.members() {
        let c = Ccs::parse(arc.inflate(m).unwrap()).unwrap();
        let scene = Scene::read(&c).unwrap();
        for a in Animation::all(&c, &scene).unwrap() {
            animations += 1;
            tracks += a.tracks.len();
            looping += a.looping as usize;
            morphs += a.morphs.iter().map(|m| m.keys.len()).sum::<usize>();
            notes += a.notes.len();
            with_notes += !a.notes.is_empty() as usize;
            assert!(a.notes.windows(2).all(|w| w[0].frame <= w[1].frame) && a.notes.iter().all(|n| n.frame < a.frames));
            // frame 0 agrees with Scene::anime_frame0 (the first track for a
            // target, the first key's values), rotations up to the key
            // quantisation and the VU0 sine's coarseness near 0
            let an = scene.animes.iter().find(|an| an.offset == a.offset).unwrap();
            let frame0 = scene.anime_frame0(&c, an).unwrap();
            let mut seen = HashSet::new();
            for (t, p) in a.tracks.iter().zip(a.poses_at(0)) {
                if !seen.insert(t.target) {
                    continue;
                }
                let want = frame0[&t.target];
                // the EE reads denormals (some keys hold them) as zero
                let bits =
                    |v: glam::Vec3| v.to_array().map(|x| if x.to_bits() & 0x7f80_0000 == 0 { 0 } else { x.to_bits() });
                assert_eq!(bits(p.pos), bits(want.pos), "{} {}", m.name, t.object);
                assert_eq!(bits(p.scale), bits(want.scale), "{} {}", m.name, t.object);
                let r = Mat3::from_rotation_x(want.rot.x.to_radians())
                    * Mat3::from_rotation_y(want.rot.y.to_radians())
                    * Mat3::from_rotation_z(want.rot.z.to_radians());
                let d = (0..3).map(|k| (r.col(k) - p.rot.col(k)).abs().max_element()).fold(0f32, f32::max);
                worst = worst.max(d);
                assert!(d < 2e-3, "{} {}: {d}", m.name, t.object);
            }
        }
    }
    // tools/anim.py list over DATA.BIN: 4,091 Anime chunks, 953 of them
    // looping, 229,107 object records, 11,086 F_Morpher records, 2,697
    // F_Note records in 944 animations
    eprintln!("worst frame-0 rotation difference from Scene::anime_frame0: {worst:.2e}");
    assert_eq!((animations, looping, tracks, morphs), (4091, 953, 229_107, 11_086));
    assert_eq!((notes, with_notes), (2697, 944));
}

/// F_Obj (0x0101, `DecodeF_Obj`): `eex1`'s damage clip moves the body by
/// one record a frame (its only object track is `OBJ_xnote`'s). Frame 0's
/// record is never read; from frame 1 the last record read holds, as
/// `T(pos) * sceVu0RotMatrix(pi * deg / 180) * S(scale)` and `localtp`.
#[test]
fn f_obj_records_pose_the_body() {
    let Some(arc) = data_bin() else { return };
    let f = File::load(&arc, "eex1.cmp");
    let a = &f.animations["ANM_eex1dmg0"];
    assert_eq!(a.objs.len(), 31, "one record a frame");
    assert!(a.objs.iter().all(|r| f.ccs.object_name(r.object) == Some("OBJ_eex1body")));
    let body = a.objs[0].target;
    assert!(a.objs.iter().all(|r| r.target == body));
    assert!(a.tracks.iter().all(|t| t.target != body), "no track drives the body");
    assert!(a.obj_poses_at(0).is_empty(), "frame 0's record is never read");
    let at = |frame: u32| a.obj_poses_at(frame << 8).into_iter().find(|(t, _)| *t == body).map(|(_, p)| p).unwrap();
    // Frame 3's record: z up 7.75, turned about 15 degrees about y and -15
    // about z (the file's floats are those to a few ulps).
    let p = at(3);
    assert!(p.pos.x.abs() < 1e-4 && p.pos.y.abs() < 1e-4 && p.pos.z == 7.75, "{:?}", p.pos);
    assert!((p.scale - glam::Vec3::ONE).abs().max_element() < 1e-5, "{:?}", p.scale);
    assert_eq!(p.alpha, 1.0);
    let rad = anim::const_radians([0f32.to_bits(), 15f32.to_bits(), (-15f32).to_bits()]);
    let m = anim::rot_bits(rad);
    let want = Mat3::from_cols_array(&[
        f32::from_bits(m[0][0]),
        f32::from_bits(m[0][1]),
        f32::from_bits(m[0][2]),
        f32::from_bits(m[1][0]),
        f32::from_bits(m[1][1]),
        f32::from_bits(m[1][2]),
        f32::from_bits(m[2][0]),
        f32::from_bits(m[2][1]),
        f32::from_bits(m[2][2]),
    ]);
    assert!((p.rot - want).to_cols_array().iter().all(|d| d.abs() < 1e-4), "{:?} vs {want:?}", p.rot);
    // Between frames the pose holds; past the end, the last frame's.
    assert_eq!(a.obj_poses_at((3 << 8) + 200)[0].1, p);
    assert_eq!(a.obj_poses_at(1 << 20)[0].1, at(30));
    // The locals take it for the body.
    let locals = a.locals_at(3 << 8);
    assert_eq!(locals[&body], p.matrix());
}
