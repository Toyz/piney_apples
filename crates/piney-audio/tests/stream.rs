//! The music around the streams against the game's: `stream_fixture.txt`,
//! written by `python3 tools/test_stream_rs.py music` from `ccSndStreamCtrl`,
//! `ccSndStreamSE` / `ccSndStreamBGM` and `strSeInit` run in `tools/eemu.py`
//! between `tools/sound_ee.py`'s steps. The port runs the same steps (the
//! driver, [`stream_ctrl`], [`stream_bgm`] and `piney_stream::table::BgmCursor`)
//! and must send the same, frame for frame; the scenarios are listed in
//! docs/engine/sound.md ("Streams"). Needs the disc image; skipped without it.

use std::path::PathBuf;

use piney_audio::driver::{BgmWorld, Command, Driver, SqContext};
use piney_audio::stream::{StrBgm, StreamGame, stream_bgm, stream_ctrl};
use piney_data::iso::Iso;
use piney_data::sound::SndData;
use piney_stream::table::{self, BgmCursor};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

/// The driver's commands in the fixture's words (`tests/driver.rs`'s).
fn words(snd: &SndData, cmds: &[Command]) -> Vec<String> {
    let mut out = Vec::new();
    let mut bank = None;
    for c in cmds {
        match c {
            Command::Msg(m) => out.push(format!("msg {}", m.iter().map(|b| format!("{b:02x}")).collect::<String>())),
            Command::PortVolume(p, v) => out.push(format!("vol {p} {v}")),
            Command::Master(v) => {
                out.push(format!("master 981 {v}"));
                out.push(format!("master a81 {v}"));
            }
            Command::Load(row) => {
                out.push(format!("load {}", row.ofs));
                bank = Some(snd.bank_at(row.ofs as usize).unwrap());
            }
            Command::Seq(i) => {
                let info = bank.expect("a bank before its sequences");
                out.push(format!("seq {i} {}", info.sequence_offset(*i).unwrap() - info.offset));
            }
            Command::AllSoundOff
            | Command::Reverb(_)
            | Command::OutputMode(_)
            | Command::Voice(_)
            | Command::VoiceStop => {}
            Command::Play(m) => out.push(format!("play {m}")),
            Command::Stop(m) => out.push(format!("stop {m}")),
            Command::Area(n) => out.push(format!("area {n}")),
            Command::PlayType(t) => out.push(format!("playtype {t}")),
            Command::BgmChange => out.push("change".to_string()),
        }
    }
    out
}

/// What the `set NAME V` steps poke (`tests/driver.rs`'s).
#[derive(Default)]
struct World {
    area: i32,
    area_prev: i32,
    town: i32,
    field: i32,
    dungeon: i32,
    bg: i32,
    field_type: i32,
    dtype: [i32; 3],
}

impl World {
    fn set(&mut self, name: &str, v: i32) {
        match name {
            "area" => self.area = v,
            "areaPrev" => self.area_prev = v,
            "town" => self.town = v,
            "field" => self.field = v,
            "dungeon" => self.dungeon = v,
            "bg" => self.bg = v,
            "fieldtype" => self.field_type = v,
            "dtype0" => self.dtype[0] = v,
            "dtype1" => self.dtype[1] = v,
            "dtype2" => self.dtype[2] = v,
            _ => panic!("set {name}"),
        }
    }

    fn context(&self, n: i32) -> SqContext {
        match n {
            1 => SqContext::Desktop,
            2 => SqContext::Town { town: self.town as u8, crisis: false },
            3 => SqContext::Field { field_type: self.field_type as u8, bg: self.bg as u8, piros: false },
            4 => SqContext::Dungeon { dungeon_type: self.dtype[self.dungeon as usize] as u8 },
            5 => SqContext::Event { field: self.field as u16, area_prev: self.area_prev },
            7 => SqContext::Title,
            _ => panic!("load {n}"),
        }
    }

    /// Every scenario starts from a fresh game: the previous scene is 0.
    fn bgm(&self) -> BgmWorld {
        BgmWorld {
            scene_replaced: self.area != 0 || self.town != 0 || self.field != 0 || self.dungeon != 0,
            town: self.town,
            crisis: false,
            dt_bgm: 0,
        }
    }
}

fn to_audio(r: table::StrBgm) -> StrBgm {
    StrBgm { sq: r.sq, sq2: r.sq2, time: r.time, vol: r.vol, cmd: r.cmd }
}

#[test]
fn the_music_around_the_streams_is_the_games() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    check(&path, include_str!("stream_fixture.txt"));
}

/// Mutation's, Outbreak's and Quarantine's own `ccSndStreamCtrl` (each
/// volume's switch, `PINEY_VOLUME=... python3 tools/test_stream_rs.py
/// music`), on the same steps.
#[test]
fn the_later_volumes_music_around_the_streams() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work");
    for (disc, fixture) in [
        ("mutation/mutation.iso", include_str!("stream_fixture_mut.txt")),
        ("outbreak/outbreak.iso", include_str!("stream_fixture_out.txt")),
        ("quarantine/quarantine.iso", include_str!("stream_fixture_qua.txt")),
    ] {
        let path = root.join(disc);
        if !path.exists() {
            eprintln!("skipped: no {disc}");
            continue;
        }
        check(&path, fixture);
    }
}

fn check(path: &std::path::Path, fixture: &str) {
    let mut iso = Iso::open(path).unwrap();
    let snd = SndData::read(&mut iso).unwrap();
    let volume = iso.volume().unwrap();
    let t = piney_data::sound::tables_of(volume);
    let mut n = 0;
    for line in fixture.lines().filter(|l| l.starts_with("seq ")) {
        let (head, want) = line.split_once(" | ").unwrap_or((line.trim_end_matches(" |"), ""));
        let want: Vec<String> = want.split("; ").filter(|s| !s.is_empty()).map(String::from).collect();
        let mut d = Driver::new();
        let mut out = Vec::new();
        let mut world = World::default();
        let mut cursor = BgmCursor::default();
        let (mut status, mut movie) = (0, false);
        let steps = head.split_once(' ').unwrap().1;
        for step in steps.split('+') {
            let w: Vec<&str> = step.split_whitespace().collect();
            let a: Vec<i32> = w[1..].iter().filter_map(|s| s.parse().ok()).collect();
            let note = |d: &mut Driver, cursor: &mut BgmCursor, out: &mut Vec<Command>, event: i32| {
                // ccSndStreamSE: event 4 with a table, not under the movie
                // player.
                if !movie
                    && event == 4
                    && cursor.is_set()
                    && let Some(r) = cursor.step()
                {
                    stream_bgm(d, to_audio(r), out);
                }
            };
            match w[0] {
                "load" => d.sq_load(t, world.context(a[0]), &mut out),
                "set" => world.set(w[1], a[0]),
                "bgm" => {
                    d.bgm_ctrl(&world.bgm(), &mut out);
                }
                "start" => d.game_start = a[0] != 0,
                "frames" => {
                    for _ in 0..a[0] {
                        d.frame(t, &mut out);
                    }
                }
                "strinit" => cursor = BgmCursor::new(table::stream_bgm_table(volume, a[0] as usize)),
                "strctrl" => {
                    if !movie {
                        let game = StreamGame { status, field: world.field };
                        stream_ctrl(&mut d, t, volume, a[0] as usize, a[2], a[1] == 1, game, &mut out);
                    }
                }
                "strnote" => note(&mut d, &mut cursor, &mut out, a[0]),
                "bgmrec" => {
                    let rec = table::StrBgm {
                        sq: a[0] as i16,
                        sq2: a[1] as i16,
                        time: a[2] as i16,
                        vol: a[3] as u16,
                        unknown_8: 0,
                        cmd: a[4] as i16,
                    };
                    let end =
                        table::StrBgm { sq: -1, sq2: -1, time: -1, vol: 0xffff, unknown_8: -1, cmd: table::BGM_END };
                    cursor = BgmCursor::new(vec![rec, end]);
                    note(&mut d, &mut cursor, &mut out, 4);
                }
                "status" => status = a[0],
                "movie" => movie = a[0] != 0,
                "loop" => d.loop_id[a[0] as usize] = a[1] as i8,
                s => panic!("step {s}"),
            }
        }
        assert_eq!(words(&snd, &out), want, "{volume:?}: {head}");
        n += 1;
    }
    // Each stream before and after over five banks, playing or not; 59
    // others.
    assert_eq!(n, 59 + 10 * table::count(volume));
}
