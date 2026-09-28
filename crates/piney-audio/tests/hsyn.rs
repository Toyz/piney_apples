//! The synthesizer against MODHSYN.IRX itself: fixtures from the module run in
//! `tools/eemu.py` by `tools/iopemu.py`, libsd's register functions recording
//! what they are given (ENVX and ENDX read 0, as the [`Recorder`] makes them):
//! `se_fixture.txt` (`se-fixture ELF --ticks 3`, every sound effect),
//! `song_fixture.txt` (`song-fixture ELF 50 0 19 27 46 13 12 26 36 11 7 47
//! --ticks 12000`: jukebox rows, writes hashed per 100 ticks) and
//! `se3d_fixture.txt` (`se3d-fixture ELF`, the positioned effects' messages).
//! Skipped without the disc image; set PINEY_ISO to point at it elsewhere.

use std::path::PathBuf;
use std::sync::Arc;

use piney_audio::hsyn::Synth;
use piney_audio::midi::Sequencer;
use piney_audio::spu::{Recorder, Regs, Write};
use piney_audio::{MUSIC_SPU_BASE, SE_SPU_ADDR};
use piney_data::iso::Iso;
use piney_data::sound::{Bank, INF, SndData};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn fmt(w: &Write) -> String {
    format!("{}:{:x}:{:x}", w.0, w.1, w.2)
}

/// ccSndCommSeLoad's order: the SE bank on port 0, priority 0x40 and 16
/// voices, volume 256, one tick.
fn se_synth(se: &Arc<Bank>) -> (Synth, Recorder) {
    let mut syn = Synth::new();
    let mut rec = Recorder::default();
    syn.load(0, se.clone(), SE_SPU_ADDR);
    syn.set_attr(0, 0x1040);
    syn.set_volume(0, 256);
    syn.tick(&mut rec);
    rec.log.clear();
    (syn, rec)
}

fn se_bytes(n: usize) -> Vec<u8> {
    piney_audio::driver::Driver::se_on(&INF.se[n])
}

#[test]
fn every_sound_effect_writes_what_modhsyn_writes() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let snd = SndData::read(&mut Iso::open(path).unwrap()).unwrap();
    let se = Arc::new(snd.commse(&INF).unwrap());
    let mut state: Option<(Synth, Recorder)> = None;
    let mut last = String::new();
    let mut checked = 0;
    for line in include_str!("se_fixture.txt").lines().filter(|l| l.starts_with("se ")) {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (n, vol, second, tick): (usize, u16, i32, u32) =
            (f[1].parse().unwrap(), f[2].parse().unwrap(), f[3].parse().unwrap(), f[4].parse().unwrap());
        let case = f[1..4].join(" ");
        if tick == 0 || case != last {
            let (mut syn, mut rec) = se_synth(&se);
            if vol != 256 {
                syn.set_volume(0, vol);
                rec.log.clear();
            }
            syn.input(0, &se_bytes(n));
            if second >= 0 {
                syn.input(0, &se_bytes(second as usize));
            }
            state = Some((syn, rec));
            last = case;
        }
        let (syn, rec) = state.as_mut().unwrap();
        syn.tick(rec);
        let got: Vec<String> = rec.log.iter().map(fmt).collect();
        rec.log.clear();
        assert_eq!(got.join(" "), f[5..].join(" "), "{line}");
        checked += 1;
    }
    assert!(checked > 700, "{checked}");
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// The positioned sound effects' messages (`se3d_fixture.txt`, `iopemu.py
/// se3d-fixture ELF`): every sound effect as `ccSeOn3D` sends it (program
/// change, `F9 01` pan, `F9 02` bend when behind, `FD 10` note on), pans
/// 0-126 with and without the bend, other port volumes and notes, loops'
/// note ons by id and their note offs (a wrong id ending nothing), an
/// override waiting for a plain note on, and two at once - every register
/// write of each tick.
#[test]
fn positioned_sound_effects_write_what_modhsyn_writes() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let snd = SndData::read(&mut Iso::open(path).unwrap()).unwrap();
    let se = Arc::new(snd.commse(&INF).unwrap());
    let fixture = include_str!("se3d_fixture.txt");
    let mut lines = fixture.lines().filter(|l| !l.starts_with('#'));
    let mut checked = 0;
    while let Some(head) = lines.next() {
        let f: Vec<&str> = head.split_whitespace().collect();
        assert_eq!(f[0], "case", "{head}");
        let (name, vol, ticks): (&str, u16, u32) = (f[1], f[2].parse().unwrap(), f[3].parse().unwrap());
        let sends: Vec<(u32, Vec<u8>)> = f[4]
            .split(',')
            .map(|s| {
                let (t, b) = s.split_once(':').unwrap();
                (t.parse().unwrap(), unhex(b))
            })
            .collect();
        let (mut syn, mut rec) = se_synth(&se);
        if vol != 256 {
            syn.set_volume(0, vol);
            rec.log.clear();
        }
        for t in 0..ticks {
            for (_, b) in sends.iter().filter(|(w, _)| *w == t) {
                syn.input(0, b);
            }
            syn.tick(&mut rec);
            let got: Vec<String> = rec.log.iter().map(fmt).collect();
            rec.log.clear();
            let line = lines.next().unwrap();
            let w: Vec<&str> = line.split_whitespace().collect();
            assert_eq!((w[0], w[1], w[2].parse::<u32>().unwrap()), ("w", name, t), "{line}");
            assert_eq!(got.join(" "), w[3..].join(" "), "{line}");
            checked += 1;
        }
    }
    assert!(checked > 900, "{checked}");
}

fn fnv(h: u64, data: &[u8]) -> u64 {
    data.iter().fold(h, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

#[test]
fn jukebox_rows_play_as_modhsyn_plays_them() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let snd = SndData::read(&mut Iso::open(path).unwrap()).unwrap();
    let se = Arc::new(snd.commse(&INF).unwrap());
    let lines: Vec<Vec<&str>> = include_str!("song_fixture.txt")
        .lines()
        .filter(|l| l.starts_with("song "))
        .map(|l| l.split_whitespace().collect())
        .collect();
    let mut nos: Vec<usize> = lines.iter().map(|f| f[1].parse().unwrap()).collect();
    nos.dedup();
    for no in nos {
        let want: Vec<&Vec<&str>> = lines.iter().filter(|f| f[1] == no.to_string()).collect();
        let (mut syn, mut rec) = se_synth(&se);
        // ccSQDataLoadCD's resets, then ccSndChangeData's all sound off.
        for port in 0..4 {
            syn.reset_all_controllers(port);
        }
        for port in 0..4 {
            syn.reset_all_controllers(port);
            syn.all_note_off(port, &mut rec);
            syn.all_sound_off(port, &mut rec);
            syn.set_volume(port, 0);
        }
        for port in 0..4 {
            syn.all_sound_off(port, &mut rec);
        }
        let w = INF.wave[no];
        let (row, vols) = INF.wave(&w).unwrap();
        let bank = Arc::new(snd.load_row(&row).unwrap());
        let at = MUSIC_SPU_BASE + INF.commse[2];
        for i in 0..3 {
            if i == 0 || row.sq_size[i] != 0 {
                syn.load(i + 1, bank.clone(), at);
                syn.set_attr(i + 1, 0x2010);
            }
        }
        syn.set_volume(0, 256);
        for (i, v) in vols.iter().enumerate() {
            syn.set_volume(i + 1, v.vol);
        }
        let seq = if matches!(no, 27 | 7) { 1 } else { 0 };
        syn.set_volume(vols[seq].hd_port as usize, vols[seq].vol);
        let mut s = Sequencer::new();
        assert!(s.load(bank.sequence(seq).unwrap(), seq));
        assert!(s.play_switch(true));
        rec.log.clear();
        let block = 100;
        let ticks = want.len() * block;
        let (mut h, mut n) = (0xcbf2_9ce4_8422_2325u64, 0);
        for t in 0..ticks {
            for (p, m) in s.tick() {
                syn.input(p + 1, &m);
            }
            syn.tick(&mut rec);
            for e in &rec.log {
                h = fnv(h, format!("{};", fmt(e)).as_bytes());
                n += 1;
            }
            h = fnv(h, b"|");
            rec.log.clear();
            if (t + 1) % block == 0 {
                let f = want[t / block];
                assert_eq!(
                    format!("{} {n} {h:016x}", t + 1 - block),
                    f[2..].join(" "),
                    "Wave[{no}] ticks {}..{}",
                    t + 1 - block,
                    t + 1
                );
                h = 0xcbf2_9ce4_8422_2325;
                n = 0;
            }
        }
    }
}

#[test]
fn recorder_reads_back_what_was_set() {
    let mut r = Recorder::default();
    r.set_param(0x500, 3);
    assert_eq!(r.get_param(0x500), 3);
    assert_eq!(r.get_switch(0x1700), 0);
}
