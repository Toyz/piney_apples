//! `sound` against `tools/sound.py`, `tools/scei.py` and `tools/adpcm.py`.
//! The fixture (`sound_fixture.txt`, written by `python3
//! tools/test_sound_rs.py fixture`) holds only numbers and hashes; the banks
//! come from the disc. Every bank's place and parsed header, every VAG's
//! decoded samples and loop, every `.sq`'s chunks, the `BGM.BIN` tracks,
//! every row of the event voice tables and a sample of their lines must
//! match exactly. Skipped when the disc image is not extracted; set
//! PINEY_ISO to point at it elsewhere.

use std::path::PathBuf;

use piney_data::iso::Iso;
use piney_data::sound::{self, Hd, INF, SndData, Tables, adpcm, voice};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x100_0000_01b3;

fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(FNV_OFFSET, |h, &b| (h ^ b as u64).wrapping_mul(FNV_PRIME))
}

fn fnv_values(values: &[i64]) -> u64 {
    let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    fnv(&bytes)
}

/// Slots up to the last filled one, as scei.py's lists see them.
fn filled<T>(v: &[Option<T>]) -> &[Option<T>] {
    let n = v.iter().rposition(|x| x.is_some()).map_or(0, |i| i + 1);
    &v[..n]
}

/// The header's fields in slot order, as tools/test_sound_rs.py lists them.
fn hd_values(hd: &Hd) -> Vec<i64> {
    let mut out = Vec::new();
    for (i, v) in filled(&hd.vags).iter().enumerate() {
        out.push(i as i64);
        match v {
            Some(v) => out.extend([v.offset as i64, v.rate as i64, v.looped as i64, v.reserved as i64]),
            None => out.push(-1),
        }
    }
    for (i, s) in filled(&hd.samples).iter().enumerate() {
        out.push(i as i64);
        let Some(s) = s else {
            out.push(-1);
            continue;
        };
        out.extend([
            s.vag as i64,
            s.vel_low as i64,
            s.vel_crossfade as i64,
            s.vel_high as i64,
            s.vel_follow_pitch as i64,
            s.vel_follow_pitch_center as i64,
            s.vel_follow_pitch_curve as i64,
            s.vel_follow_amp as i64,
            s.vel_follow_amp_center as i64,
            s.vel_follow_amp_curve as i64,
            s.base_note as i64,
            s.detune as i64,
            s.pan as i64,
            s.group as i64,
            s.priority as i64,
            s.volume as i64,
            s.reserved as i64,
            s.adsr1 as i64,
            s.adsr2 as i64,
            s.kf_ar as i64,
            s.kf_ar_center as i64,
            s.kf_dr as i64,
            s.kf_dr_center as i64,
            s.kf_sr as i64,
            s.kf_sr_center as i64,
            s.kf_rr as i64,
            s.kf_rr_center as i64,
            s.kf_sl as i64,
            s.kf_sl_center as i64,
            s.pitch_lfo_delay as i64,
            s.pitch_lfo_fade as i64,
            s.amp_lfo_delay as i64,
            s.amp_lfo_fade as i64,
            s.lfo_attr as i64,
            s.spu_attr as i64,
        ]);
    }
    for (i, s) in filled(&hd.sample_sets).iter().enumerate() {
        out.push(i as i64);
        let Some(s) = s else {
            out.push(-1);
            continue;
        };
        out.extend([s.vel_curve as i64, s.vel_low as i64, s.vel_high as i64, s.samples.len() as i64]);
        out.extend(s.samples.iter().map(|&x| x as i64));
    }
    for (i, p) in filled(&hd.programs).iter().enumerate() {
        out.push(i as i64);
        let Some(p) = p else {
            out.push(-1);
            continue;
        };
        out.extend([
            p.split_offset as i64,
            p.n_split as i64,
            p.split_size as i64,
            p.volume as i64,
            p.pan as i64,
            p.transpose as i64,
            p.detune as i64,
            p.key_follow_pan as i64,
            p.key_follow_pan_center as i64,
            p.attr as i64,
            p.reserved as i64,
            p.lfo_wave as i64,
            p.lfo_wave2 as i64,
            p.lfo_phase as i64,
            p.lfo_phase2 as i64,
            p.lfo_random as i64,
            p.lfo_random2 as i64,
            p.lfo_cycle as i64,
            p.lfo_cycle2 as i64,
            p.lfo_pitch_depth as i64,
            p.lfo_pitch_depth2 as i64,
            p.lfo_midi_pitch_depth as i64,
            p.lfo_midi_pitch_depth2 as i64,
            p.lfo_amp_depth as i64,
            p.lfo_amp_depth2 as i64,
            p.lfo_midi_amp_depth as i64,
            p.lfo_midi_amp_depth2 as i64,
        ]);
        for sp in &p.splits {
            out.extend([
                sp.sample_set as i64,
                sp.key_low as i64,
                sp.key_crossfade as i64,
                sp.key_high as i64,
                sp.number as i64,
                sp.bend_low as i64,
                sp.bend_high as i64,
                sp.key_follow_pitch as i64,
                sp.key_follow_pitch_center as i64,
                sp.key_follow_amp as i64,
                sp.key_follow_amp_center as i64,
                sp.key_follow_pan as i64,
                sp.key_follow_pan_center as i64,
                sp.volume as i64,
                sp.pan as i64,
                sp.transpose as i64,
                sp.detune as i64,
            ]);
        }
    }
    out
}

fn fixture() -> Vec<Vec<String>> {
    include_str!("sound_fixture.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split_whitespace().map(String::from).collect())
        .collect()
}

fn num(s: &str) -> i64 {
    s.parse().unwrap()
}

#[test]
fn banks_vags_and_sequences_match_the_tools() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let mut iso = Iso::open(path).unwrap();
    let snd = SndData::read(&mut iso).unwrap();
    let bgm_bin = iso.read_path("VOICE/BGM.BIN").unwrap();
    let lines = fixture();
    let mut bank = None;
    let (mut banks, mut vags, mut sqs, mut bgms, mut evrows, mut voices) = (0, 0, 0, 0, 0, 0);
    for f in &lines {
        match f[0].as_str() {
            "bank" => {
                let info = &snd.banks()[num(&f[1]) as usize];
                let want: Vec<i64> = f[2..14].iter().map(|s| num(s)).collect();
                let got = [
                    info.offset,
                    info.hd_size,
                    info.sq_sizes[0],
                    info.sq_sizes[1],
                    info.sq_sizes[2],
                    info.bd_offset(),
                    info.bd_size,
                    info.table_bd_size,
                ]
                .map(|x| x as i64);
                assert_eq!(&got[..], &want[..8], "bank {}", f[1]);
                let b = snd.load(info).unwrap();
                let counts = [b.hd.vags.len(), b.hd.samples.len(), b.hd.sample_sets.len(), b.hd.programs.len()];
                assert_eq!(counts.map(|x| x as i64), [want[8], want[9], want[10], want[11]], "bank {} slots", f[1]);
                assert_eq!(format!("{:016x}", fnv_values(&hd_values(&b.hd))), f[14], "bank {} header", f[1]);
                bank = Some(b);
                banks += 1;
            }
            "vag" => {
                let b = bank.as_ref().unwrap();
                let slot = num(&f[2]) as usize;
                let v = b.hd.vags[slot].unwrap();
                let (a, e) = b.hd.vag_ranges()[slot].unwrap();
                let d = adpcm::decode(b.vag_data(slot).unwrap());
                let (ls, le) = d.loop_range.map_or((-1, -1), |(s, e)| (s as i64, e as i64));
                let pcm: Vec<u8> = d.pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
                let got = format!(
                    "{a} {e} {} {} {} {} {ls} {le} {} {:016x}",
                    v.rate,
                    v.looped as u8,
                    d.pcm.len(),
                    d.frames,
                    d.clipped,
                    fnv(&pcm)
                );
                assert_eq!(got, f[3..].join(" "), "bank {} vag {slot}", f[1]);
                vags += 1;
            }
            "sq" => {
                let b = bank.as_ref().unwrap();
                let off = num(&f[2]) as usize;
                let k = b.info.sq_offsets().iter().position(|&(o, _)| o == off).expect("sq offset");
                let sq = &b.sq[k];
                let ch = sq.chunks.map(|c| c.map_or(-1, |c| c as i64));
                let mut got = vec![sq.data.len() as i64, sq.size as i64];
                got.extend(ch);
                got.extend(sq.midi.iter().map(|&m| m as i64));
                let want: Vec<i64> = f[3..].iter().map(|s| num(s)).collect();
                assert_eq!(got, want, "sq at {off}");
                sqs += 1;
            }
            "bgm" => {
                let t = &INF.bgm[num(&f[1]) as usize];
                assert_eq!([t.ofs, t.size, t.mode].map(|x| x as i64), [num(&f[2]), num(&f[3]), num(&f[4])]);
                let pcm = sound::bgm_pcm(&bgm_bin, t).unwrap();
                assert_eq!(pcm.len() as i64, num(&f[5]));
                let head: Vec<u8> = pcm[..pcm.len().min(1 << 17)].iter().flat_map(|s| s.to_le_bytes()).collect();
                assert_eq!(format!("{:016x}", fnv(&head)), f[6], "BGM.BIN track {}", f[1]);
                bgms += 1;
            }
            "evrow" => {
                let vol1 = &piney_data::tables::voice::of(piney_data::volume::Volume::Inf).events[0];
                let (main, side) = (vol1.main.unwrap(), vol1.side.unwrap());
                let table = match f[1].as_str() {
                    "evVoiceDataVol1M" => main.jp,
                    "evVoiceDataVol1S" => side.jp,
                    "evVoiceDataVol1ME" => main.en.unwrap(),
                    "evVoiceDataVol1SE" => side.en.unwrap(),
                    t => panic!("table {t}"),
                };
                let (event, msg) = (num(&f[2]) as usize, num(&f[3]) as usize);
                let row = table[event % 50].expect("a table for the event")[msg];
                assert_eq!((row.ofs, row.siz), (num(&f[4]) as i32, num(&f[5]) as i32), "{}", f.join(" "));
                evrows += 1;
            }
            "voice" => {
                let (ofs, size) = (num(&f[2]) as u64, num(&f[3]) as usize);
                let bytes = voice::read_voice(&mut iso, &f[1], ofs, size).unwrap();
                let pcm = voice::voice_pcm(&bytes);
                let raw: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
                assert_eq!(format!("{} {:016x}", pcm.len(), fnv(&raw)), f[4..].join(" "), "{}", f.join(" "));
                voices += 1;
            }
            other => panic!("fixture line {other}"),
        }
    }
    assert_eq!((banks, vags, sqs, bgms), (79, 1013, 150, 2));
    assert_eq!((evrows, voices), (1292, 42));
    // Every row the fixture does not list is absent: the tables hold 1,292.
    let vol1 = &piney_data::tables::voice::of(piney_data::volume::Volume::Inf).events[0];
    let (main, side) = (vol1.main.unwrap(), vol1.side.unwrap());
    let all: usize = [main.jp, side.jp, main.en.unwrap(), side.en.unwrap()]
        .iter()
        .flat_map(|t| t.iter().flatten())
        .map(|r| r.len())
        .sum();
    assert_eq!(all, 1292);
    assert_eq!(snd.banks().len(), 79);
}

#[test]
fn tables_hang_together() {
    // Every jukebox row names a used loading row whose bank has the
    // sequence the game plays, and NO is its index.
    let snd = iso_path().map(|p| SndData::read(&mut Iso::open(p).unwrap()).unwrap());
    for (i, w) in INF.wave.iter().enumerate() {
        assert_eq!(w.no, i as i32);
        let (load, vol) = INF.wave(w).unwrap_or_else(|| panic!("Wave[{i}] loads nothing"));
        assert!(load.is_used());
        let seq = Tables::wave_sequence(w.no);
        assert_eq!(vol[seq].midi_port as usize, seq);
        assert_eq!(vol[seq].hd_port as usize, seq + 1);
        if let Some(snd) = &snd {
            let bank = snd.load_row(&load).unwrap();
            assert!(bank.sequence(seq).is_some(), "Wave[{i}] plays sequence {seq} of a bank without it");
        }
    }
    assert_eq!(INF.se.len(), 237);
    assert!(INF.se.iter().all(|s| s.port == 0));
    assert_eq!(INF.commse, [0, 0x6260, 0x12a560]);
    let _ = sound::PCM_RATE;
}

/// The later volumes' tables (`piney-gen`'s `placement::sound`, each volume's own
/// executable) place their disc's `SNDDATA.BIN`: every bank's header,
/// sequences and samples load, every `BGM.BIN` track lies in the file and
/// every jukebox row names a bank.
#[test]
fn later_volumes_tables_place_their_discs() {
    use piney_data::volume::Volume;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work");
    for (dir, volume) in [("mutation", Volume::Mut), ("outbreak", Volume::Out), ("quarantine", Volume::Qua)] {
        let path = root.join(dir).join(format!("{dir}.iso"));
        if !path.exists() {
            eprintln!("skipped {dir}: no disc image");
            continue;
        }
        let mut iso = Iso::open(&path).unwrap();
        let snd = SndData::read(&mut iso).unwrap();
        let t = snd.tables();
        assert!(std::ptr::eq(t, sound::tables_of(volume)), "{dir}: its own tables");
        for b in snd.banks() {
            snd.load(b).unwrap_or_else(|e| panic!("{dir}: bank 0x{:x} ({}): {e}", b.offset, b.refs[0]));
        }
        snd.commse(t).unwrap();
        let bgm_len = iso.read_path("VOICE/BGM.BIN").unwrap().len();
        assert!(!t.bgm.is_empty());
        for (i, track) in t.bgm.iter().enumerate() {
            assert!((track.ofs + track.size) as usize <= bgm_len, "{dir}: BGM.BIN track {i} runs past the file");
        }
        for w in t.wave {
            if w.category != 0 {
                assert!(t.wave(w).is_some(), "{dir}: jukebox row {} names no bank", w.no);
            }
        }
        eprintln!("{dir}: {} banks, {} BGM tracks", snd.banks().len(), t.bgm.len());
    }
}
