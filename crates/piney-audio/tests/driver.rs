//! The driver against the game's own EE sound code. `driver_fixture.txt` (from
//! `python3 tools/sound_ee.py fixture ELF`) is what `ccSeOn`, `ccSeOnNote`,
//! the desktop starting, the volume options, `ccSndSQLoad` with the sequence
//! calls and fades send, run in `tools/eemu.py`; the `seq` scenarios also load
//! every area's bank with `ccSndBgmCtrl`, `sound 10`'s hold and the battle
//! switch both ways. The driver must send the same. Needs the disc image only
//! for the banks' sequence offsets; skipped without it.

use std::path::PathBuf;

use piney_audio::driver::{BgmWorld, Command, Driver, SqContext};
use piney_data::iso::Iso;
use piney_data::sound::{INF, SndData};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

/// The driver's commands in the fixture's words.
fn words(snd: &SndData, cmds: &[Command]) -> Vec<String> {
    let mut out = Vec::new();
    let mut bank = None;
    for c in cmds {
        match c {
            Command::Msg(m) => {
                // Two messages: program change, then note on.
                out.push(format!("msg {}", hex(&m[..2])));
                out.push(format!("msg {}", hex(&m[2..])));
            }
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
            Command::AllSoundOff => {}
            Command::Play(m) => out.push(format!("play {m}")),
            Command::Stop(m) => out.push(format!("stop {m}")),
            // tools/sound_ee.py stubs ccEvVoiceStop out here; tests/voice.rs
            // checks the voice against its own fixture.
            Command::Voice(_) | Command::VoiceStop => {}
            Command::Area(n) => out.push(format!("area {n}")),
            Command::PlayType(t) => out.push(format!("playtype {t}")),
            Command::BgmChange => out.push("change".to_string()),
        }
    }
    out
}

/// What `tools/sound_ee.py`'s `set NAME V` steps poke: `ccGame`,
/// `WORLD_MAN` and save fields, and whether Piros is in the party.
#[derive(Default)]
struct World {
    area: i32,
    area_prev: i32,
    town: i32,
    town_prev: i32,
    field: i32,
    field_prev: i32,
    dungeon: i32,
    dungeon_prev: i32,
    bg: i32,
    field_type: i32,
    dtype: [i32; 3],
    crisis: i32,
    dt_bgm: i32,
    piros: bool,
}

impl World {
    fn set(&mut self, name: &str, v: i32) {
        match name {
            "area" => self.area = v,
            "areaPrev" => self.area_prev = v,
            "town" => self.town = v,
            "townPrev" => self.town_prev = v,
            "field" => self.field = v,
            "fieldPrev" => self.field_prev = v,
            "dungeon" => self.dungeon = v,
            "dungeonPrev" => self.dungeon_prev = v,
            "bg" => self.bg = v,
            "fieldtype" => self.field_type = v,
            "dtype0" => self.dtype[0] = v,
            "dtype1" => self.dtype[1] = v,
            "dtype2" => self.dtype[2] = v,
            "crisis" => self.crisis = v,
            "dtBgm" => self.dt_bgm = v,
            "piros" => self.piros = v != 0,
            _ => panic!("set {name}"),
        }
    }

    /// `ccSndSQLoad(n)`'s context as the world picks it.
    fn context(&self, n: i32) -> SqContext {
        match n {
            0 => SqContext::Toppage,
            1 => SqContext::Desktop,
            2 => SqContext::Town { row: (self.town + if self.crisis != 0 { 5 } else { 0 }) as u8 },
            3 => SqContext::Field { field_type: self.field_type as u8, bg: self.bg as u8, piros: self.piros },
            4 => SqContext::Dungeon { dungeon_type: self.dtype[self.dungeon as usize] as u8 },
            5 => SqContext::Event { field: self.field as u16, area_prev: self.area_prev },
            7 => SqContext::Title,
            _ => panic!("load {n}"),
        }
    }

    fn bgm(&self) -> BgmWorld {
        BgmWorld {
            scene_replaced: self.area != self.area_prev
                || self.town != self.town_prev
                || self.field != self.field_prev
                || self.dungeon != self.dungeon_prev,
            town: self.town,
            crisis: self.crisis == 1,
            dt_bgm: self.dt_bgm,
        }
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn the_driver_sends_what_the_game_sends() {
    let Some(path) = iso_path() else {
        eprintln!("skipped: no disc image");
        return;
    };
    let snd = SndData::read(&mut Iso::open(path).unwrap()).unwrap();
    let mut counts = [0; 6];
    for line in include_str!("driver_fixture.txt").lines().filter(|l| !l.starts_with('#')) {
        let (head, want) = line.split_once(" | ").unwrap_or((line.trim_end_matches(" |"), ""));
        let want: Vec<String> = want.split("; ").filter(|s| !s.is_empty()).map(String::from).collect();
        let a: Vec<i32> = head.split_whitespace().skip(1).filter_map(|s| s.parse().ok()).collect();
        let mut out = Vec::new();
        let kind = head.split_whitespace().next().unwrap();
        match kind {
            "se" => {
                out.push(Command::Msg(Driver::se_on(&INF.se[a[0] as usize])));
                counts[0] += 1;
            }
            "note" => {
                out.push(Command::Msg(Driver::se_note(&INF.se[a[0] as usize], a[1] as i8)));
                counts[1] += 1;
            }
            "desktop" => {
                let mut d = Driver::new();
                d.desktop(&INF, INF.wave[a[0] as usize], &mut out);
                counts[2] += 1;
            }
            "volume" => {
                let mut d = Driver::new();
                d.desktop(&INF, INF.wave[a[0] as usize], &mut Vec::new());
                d.set_volumes(a[1], a[2], a[3], &mut out);
                d.frame(&INF, &mut out);
                counts[3] += 1;
            }
            "jukebox" => {
                let mut d = Driver::new();
                d.desktop(&INF, INF.wave[a[0] as usize], &mut Vec::new());
                d.change(&INF, INF.wave[a[1] as usize], &mut out);
                for _ in 0..20 {
                    d.frame(&INF, &mut out);
                }
                counts[4] += 1;
            }
            "seq" => {
                let mut d = Driver::new();
                let mut world = World::default();
                let steps = head.split_once(' ').unwrap().1;
                for step in steps.split('+') {
                    let w: Vec<&str> = step.split_whitespace().collect();
                    let n: Vec<i32> = w[1..].iter().filter_map(|s| s.parse().ok()).collect();
                    match w[0] {
                        "load" => d.sq_load(&INF, world.context(n[0]), &mut out),
                        "set" => world.set(w[1], n[0]),
                        "bgm" => {
                            d.bgm_ctrl(&world.bgm(), &mut out);
                        }
                        "hold" => d.hold_bgm(),
                        "battle" => d.in_battle = n[0] != 0,
                        "ride" => d.riding = n[0] != 0,
                        "pginit" => d.pg_bgm_init(),
                        "pgend" => d.pg_bgm_end(n[0], &mut out),
                        "play" => d.sq_play(n[0] as usize, &mut out),
                        "stop" => d.sq_stop(n[0] as usize, &mut out),
                        "fade" => d.sq_fade(n[0] as usize, n[1] as u16, n[2], n[3] as u8),
                        "main" => d.set_main_volume(n[0]),
                        "start" => d.game_start = n[0] != 0,
                        "off" => d.all_sound_off(),
                        "frames" => {
                            for _ in 0..n[0] {
                                d.frame(&INF, &mut out);
                            }
                        }
                        s => panic!("step {s}"),
                    }
                }
                counts[5] += 1;
            }
            other => panic!("scenario {other}"),
        }
        assert_eq!(words(&snd, &out), want, "{head}");
    }
    assert_eq!(counts, [237, 30, 51, 6, 10, 367]);
}
