//! The per-frame records of an Anime chunk that `piney_data::anim` does not
//! evaluate: `F_Obj` (0x0101) and `F_Camera` (0x0502).
//!
//! An Anime chunk is u32 object, u32 frames, u32 words, then sub-chunks:
//! `Top` (0xff01, u32 frame) opens a frame and the records after it apply
//! from that frame on (`ccStream::DecodeFrameChunk` 0x0014e1b0).

use glam::Vec3;

use crate::anm::FObj;
use crate::assets::SceneFile;

const TOP: u16 = 0xff01;
const F_OBJ: u16 = 0x0101;
const F_CAMERA: u16 = 0x0502;

fn u32_at(d: &[u8], at: usize) -> u32 {
    d.get(at..at + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
}

fn f32_at(d: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(d, at))
}

/// Every sub-chunk of animation `anim`: (frame, kind, payload offset, words).
fn walk(file: &SceneFile, anim: usize) -> Vec<(u32, u16, usize, usize)> {
    let d = &file.ccs.data;
    let a = &file.anims[anim];
    let words = u32_at(d, a.offset + 16) as usize;
    let mut p = a.offset + 20;
    let end = p + 4 * words;
    let mut frame = 0u32;
    let mut out = Vec::new();
    while p + 8 <= end.min(d.len()) {
        let kind = u32_at(d, p) as u16;
        let n = u32_at(d, p + 4) as usize;
        if kind == TOP {
            frame = u32_at(d, p + 8);
        } else {
            out.push((frame, kind, p + 8, n));
        }
        p += 8 + 4 * n;
    }
    out
}

/// `ccStream::DecodeF_Obj` (0x0014e950): (frame, object, record) for every
/// F_Obj record of the animation, in file order. Rotations stay in degrees;
/// transparency is clamped to 0..1; `dispSW & 1` says drawn.
pub fn fobj_records(file: &SceneFile, anim: usize) -> Vec<(u32, u32, FObj)> {
    let d = &file.ccs.data;
    walk(file, anim)
        .into_iter()
        .filter(|&(_, kind, _, n)| kind == F_OBJ && n >= 13)
        .map(|(frame, _, q, _)| {
            let v = |i: usize| f32_at(d, q + 4 * i);
            let rec = FObj {
                pos: Vec3::new(v(2), v(3), v(4)),
                rot: Vec3::new(v(5), v(6), v(7)),
                scale: Vec3::new(v(8), v(9), v(10)),
                transparency: v(11).clamp(0.0, 1.0),
                disp: u32_at(d, q + 48) & 1 != 0,
            };
            (frame, u32_at(d, q), rec)
        })
        .collect()
}

/// The objects F_Obj records name.
pub fn fobj_objects(file: &SceneFile, anim: usize) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::new();
    for (_, obj, _) in fobj_records(file, anim) {
        if !out.contains(&obj) {
            out.push(obj);
        }
    }
    out
}

/// The F_Camera records: (frame, camera object, fields present, 8 floats).
pub fn camera_records(file: &SceneFile, anim: usize) -> Vec<(u32, u32, [f32; 8])> {
    let d = &file.ccs.data;
    let mut out = Vec::new();
    for (frame, kind, q, n) in walk(file, anim) {
        if kind != F_CAMERA || n < 2 {
            continue;
        }
        let obj = u32_at(d, q);
        let flag = u32_at(d, q + 4);
        let mut vals = [0f32; 8];
        let mut at = q + 8;
        for (bit, v) in vals.iter_mut().enumerate() {
            if flag & (2 << bit) == 0 && at + 4 <= q + 4 * n {
                *v = f32_at(d, at);
                at += 4;
            }
        }
        out.push((frame, obj, vals));
    }
    out
}
