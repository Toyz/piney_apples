//! The sequencer against `tools/midi.py`, which matched MODMIDI.IRX run in
//! `tools/eemu.py` byte for byte on every tick of every sequence. The
//! fixture (`midi_fixture.txt`, from `python3 tools/midi.py fixture`) holds
//! per sequence the message count, bytes, last tick with output, final
//! status and a hash of every (tick, buffer, message) over 40,000 ticks -
//! 2 min 47 s, past every sequence's first loop jump. Skipped without the
//! disc image; set PINEY_ISO to point at it elsewhere.

use std::path::PathBuf;

use piney_audio::midi::{ST_PLAY, Sequencer};
use piney_data::iso::Iso;
use piney_data::sound::{SndData, Sq};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn fnv(h: u64, data: &[u8]) -> u64 {
    data.iter().fold(h, |h, &b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

#[test]
fn every_sequence_plays_as_modmidi_does() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let snd = SndData::read(&mut Iso::open(path).unwrap()).unwrap();
    let mut checked = 0;
    for line in include_str!("midi_fixture.txt").lines().filter(|l| l.starts_with("sq ")) {
        let f: Vec<&str> = line.split_whitespace().collect();
        let bank = &snd.banks()[f[1].parse::<usize>().unwrap()];
        let slot: usize = f[2].parse().unwrap();
        let (off, n) = bank.sq_offsets()[slot];
        assert_eq!(off.to_string(), f[3]);
        let sq = Sq::parse(snd.bytes()[off..off + n].to_vec()).unwrap();
        let mut s = Sequencer::new();
        assert!(s.load(&sq, 0));
        assert!(s.play_switch(true));
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        let (mut count, mut size, mut last) = (0, 0, 0);
        for t in 0..40_000u32 {
            for (p, m) in s.tick() {
                let mut rec = t.to_le_bytes().to_vec();
                rec.extend([p as u8, m.len() as u8]);
                rec.extend(&m);
                h = fnv(h, &rec);
                count += 1;
                size += m.len();
                last = t;
            }
            if s.status & ST_PLAY == 0 {
                break;
            }
        }
        let got = format!("{count} {size} {last} {} {h:016x}", s.status);
        assert_eq!(got, f[4..].join(" "), "bank {} slot {slot}", f[1]);
        checked += 1;
    }
    assert_eq!(checked, 150);
}
