//! Every PSS on Infection's disc decoded by our decoder and by ffmpeg, compared
//! picture by picture (count, size, planar YUV 4:2:0); the audio against the
//! SPU2 block layout read independently here, and its length against the
//! video's. ffmpeg is only an outside reference, never used by the port. The
//! test is skipped without the disc image or ffmpeg (PINEY_ISO, FFMPEG).

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use piney_data::iso::Iso;
use piney_data::pss;
use piney_mpeg::Movie;

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn ffmpeg() -> Option<PathBuf> {
    let p = std::env::var_os("FFMPEG").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let ok = Command::new(&p).arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status();
    ok.is_ok_and(|s| s.success()).then_some(p)
}

/// What the comparison of one movie found.
struct Report {
    pictures: usize,
    exact: usize,
    worst_psnr: f64,
}

/// Decode `path` both ways and compare. `extra` goes to ffmpeg before its
/// input (for example `-idct int`).
fn compare(ff: &PathBuf, iso: &mut Iso, path: &str, extra: &[&str]) -> Report {
    let bytes = iso.read_path(path).unwrap();
    // One file per comparison: the tests run in parallel, and one writing
    // the file another's ffmpeg is reading cuts that one short.
    let name = format!("{}{}", path.replace('/', "_"), extra.concat());
    let file = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&file, &bytes).unwrap();
    let mut child = Command::new(ff)
        .args(["-v", "error", "-nostdin"])
        .args(extra)
        .arg("-i")
        .arg(&file)
        .args(["-f", "rawvideo", "-pix_fmt", "yuv420p", "-"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = child.stdout.take().unwrap();

    let mut movie = Movie::from_pss(&bytes).unwrap();
    assert_eq!((movie.width(), movie.height()), (640, 448), "{path}");
    let size = movie.width() * movie.height() * 3 / 2;
    let mut theirs = vec![0u8; size];
    let mut r = Report { pictures: 0, exact: 0, worst_psnr: f64::INFINITY };
    while let Some(p) = movie.next_picture().unwrap() {
        out.read_exact(&mut theirs).unwrap_or_else(|e| panic!("{path}: ffmpeg ran out at picture {}: {e}", r.pictures));
        let ours = p.yuv420p();
        if ours == theirs {
            r.exact += 1;
        } else {
            let se: f64 = ours.iter().zip(&theirs).map(|(&a, &b)| (f64::from(a) - f64::from(b)).powi(2)).sum();
            let psnr = 10.0 * (255.0f64 * 255.0 / (se / size as f64)).log10();
            r.worst_psnr = r.worst_psnr.min(psnr);
        }
        r.pictures += 1;
    }
    let mut rest = Vec::new();
    out.read_to_end(&mut rest).unwrap();
    assert!(child.wait().unwrap().success(), "{path}: ffmpeg failed");
    assert!(rest.is_empty(), "{path}: ffmpeg has {} more bytes of pictures", rest.len());
    std::fs::remove_file(&file).ok();
    r
}

/// Every picture of every movie is ffmpeg's to the bit (its default
/// decoder, whose IDCT on x86 is the "simple" one ours computes).
#[test]
fn video_matches_ffmpeg_exactly() {
    let (Some(iso), Some(ff)) = (iso_path(), ffmpeg()) else {
        eprintln!("infection.iso or ffmpeg not present; skipped");
        return;
    };
    let mut iso = Iso::open(iso).unwrap();
    for (path, pictures) in
        [("PSS/LOGO_B.PSS", 146), ("PSS/LOGO_C.PSS", 180), ("PSS/LOGO_H.PSS", 151), ("PSS/OPENING.PSS", 2771)]
    {
        let r = compare(&ff, &mut iso, path, &[]);
        eprintln!("{path}: {} pictures, {} exact, worst PSNR {:.2} dB", r.pictures, r.exact, r.worst_psnr);
        assert_eq!(r.pictures, pictures, "{path}");
        assert_eq!(r.exact, pictures, "{path}: worst PSNR {:.2} dB", r.worst_psnr);
    }
}

/// The later volumes' openings, the one movie each disc does not share
/// with Infection's (the logos are the same bytes on all four): every
/// picture is ffmpeg's too. Each is skipped when its image is missing.
#[test]
fn later_openings_match_ffmpeg_exactly() {
    let Some(ff) = ffmpeg() else {
        eprintln!("ffmpeg not present; skipped");
        return;
    };
    let work = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work");
    for (volume, path, pictures) in [
        ("mutation", "PSS/OPENING2.PSS", 2733),
        ("outbreak", "PSS/OPENING3.PSS", 2630),
        ("quarantine", "PSS/OPENING4.PSS", 2715),
    ] {
        let image = work.join(volume).join(format!("{volume}.iso"));
        if !image.exists() {
            eprintln!("{volume}.iso not present; skipped");
            continue;
        }
        let mut iso = Iso::open(image).unwrap();
        let r = compare(&ff, &mut iso, path, &[]);
        eprintln!("{path}: {} pictures, {} exact, worst PSNR {:.2} dB", r.pictures, r.exact, r.worst_psnr);
        assert_eq!(r.pictures, pictures, "{path}");
        assert_eq!(r.exact, pictures, "{path}: worst PSNR {:.2} dB", r.worst_psnr);
    }
}

/// Against a different IEEE 1180 IDCT (ffmpeg's `-idct int`, the JPEG
/// library's integer one) the pictures differ, but by little: above 45 dB
/// PSNR everywhere, drift through each group of pictures included. This is
/// the margin to expect against the PS2's IPU, whose IDCT is neither.
#[test]
fn video_close_to_another_idct() {
    let (Some(iso), Some(ff)) = (iso_path(), ffmpeg()) else {
        eprintln!("infection.iso or ffmpeg not present; skipped");
        return;
    };
    let mut iso = Iso::open(iso).unwrap();
    for path in ["PSS/LOGO_B.PSS", "PSS/LOGO_C.PSS", "PSS/LOGO_H.PSS", "PSS/OPENING.PSS"] {
        let r = compare(&ff, &mut iso, path, &["-idct", "int"]);
        eprintln!("{path} against -idct int: {} of {} exact, worst PSNR {:.2} dB", r.exact, r.pictures, r.worst_psnr);
        assert!(r.worst_psnr > 45.0, "{path}: {:.2} dB", r.worst_psnr);
    }
}

/// OPENING.PSS's audio: the SShd header, 16-bit PCM in 512-byte blocks
/// per channel, read here straight from the packets and compared with
/// what the player hands out; its length against the video's.
#[test]
fn opening_audio() {
    let Some(iso) = iso_path() else {
        eprintln!("infection.iso not present; skipped");
        return;
    };
    let mut iso = Iso::open(iso).unwrap();
    for path in ["PSS/LOGO_B.PSS", "PSS/LOGO_C.PSS", "PSS/LOGO_H.PSS"] {
        let d = pss::demux(&iso.read_path(path).unwrap()).unwrap();
        assert!(d.audio.is_none(), "{path} has audio");
    }
    let bytes = iso.read_path("PSS/OPENING.PSS").unwrap();
    let packets = pss::packets(&bytes).unwrap();
    let audio: Vec<&pss::Packet> =
        packets.iter().filter(|p| matches!(p.stream, pss::Stream::Audio { kind: 0xa0, channel: 0 })).collect();
    assert_eq!(audio.len(), 4356);
    let mut raw = Vec::new();
    for p in &audio {
        raw.extend_from_slice(&bytes[p.offset..p.offset + p.len]);
    }
    assert_eq!(&raw[0..4], b"SShd");
    assert_eq!(&raw[32..36], b"SSbd");

    let movie = Movie::from_pss(&bytes).unwrap();
    let a = movie.audio().unwrap();
    assert_eq!((a.header.format, a.rate, a.channels, a.header.interleave), (1, 48000, 2, 0x200));
    assert_eq!((a.header.loop_start, a.header.loop_end), (-1, -1));
    let body = &raw[40..];
    assert_eq!(body.len(), 17_727_488);
    assert_eq!(u32::from_le_bytes(raw[36..40].try_into().unwrap()) as usize, body.len());
    // 17,312 blocks of 256 left then 256 right samples.
    assert_eq!(a.frames(), 17_312 * 256);
    for (k, set) in body.as_chunks::<1024>().0.iter().enumerate() {
        for i in 0..256 {
            let l = i16::from_le_bytes([set[2 * i], set[2 * i + 1]]);
            let r = i16::from_le_bytes([set[512 + 2 * i], set[513 + 2 * i]]);
            let at = (k * 256 + i) * 2;
            assert_eq!((a.samples[at], a.samples[at + 1]), (l, r));
        }
    }
    // 92.33 s of sound against 2,771 pictures: 92.37 s at the stream's
    // 30 a second, 92.46 s at the console's 29.97.
    let secs = a.frames() as f64 / 48000.0;
    assert!((secs - 2771.0 / 30.0).abs() < 0.1, "{secs}");
}
