//! The driver against the game's own EE sound code. `driver_fixture.txt` (from
//! `python3 tools/sound_ee.py fixture ELF`) is what `ccSeOn`, `ccSeOnNote`,
//! the desktop starting, the volume options, `ccSndSQLoad` with the sequence
//! calls and fades send, run in `tools/eemu.py`; the `seq` scenarios also load
//! every area's bank with `ccSndBgmCtrl`, `sound 10`'s hold, the battle
//! switch both ways and the scene sounds (the breeder's tune, the church).
//! The driver must send the same; `driver_fixture_mut.txt`, `_out` and `_qua`
//! are each later volume's own. Needs the disc images only for the banks'
//! sequence offsets; skipped without them.

use std::path::{Path, PathBuf};

use piney_audio::driver::{BgmWorld, Command, Driver, SqContext};
use piney_audio::scene::{SceneInput, scene_sound};
use piney_data::iso::Iso;
use piney_data::sound::{SndData, tables_of};
use piney_data::volume::Volume;

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
            Command::Msg(m) if m.is_empty() => {}
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
            Command::AllSoundOff | Command::OutputMode(_) => {}
            Command::Reverb(on) => {
                let mode = if *on { 0x105 } else { 0x100 };
                out.push(format!("reverb 0 {mode:x}"));
                out.push(format!("reverb 1 {mode:x}"));
            }
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
    /// The scene sounds' input: `breeder X Y`, `kite X Y`, `camera X Y`
    /// and `block B`.
    scene: SceneInput,
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

    /// A scene step's place: (x, y, 0, 1) as floats.
    fn place(n: &[i32]) -> [u32; 4] {
        [n[0] as f32, n[1] as f32, 0.0, 1.0].map(f32::to_bits)
    }

    /// `ccSndSQLoad(n)`'s context as the world picks it.
    fn context(&self, n: i32) -> SqContext {
        match n {
            0 => SqContext::Toppage,
            1 => SqContext::Desktop,
            2 => SqContext::Town { town: self.town as u8, crisis: self.crisis != 0 },
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
    assert_eq!(check(include_str!("driver_fixture.txt"), &path, Volume::Inf), [237, 30, 51, 6, 10, 376]);
}

/// The later volumes' own code (`driver_fixture_mut.txt`, `_out`, `_qua`,
/// from `tools/sound_ee.py fixture` on each executable) against the driver
/// on each disc: the same scenarios, their banks, crisis rows, Flag Race
/// hold and empty sound-effect rows. A disc not extracted is skipped.
#[test]
fn the_driver_sends_what_the_later_volumes_send() {
    let fixtures = [
        (Volume::Mut, "mutation", include_str!("driver_fixture_mut.txt")),
        (Volume::Out, "outbreak", include_str!("driver_fixture_out.txt")),
        (Volume::Qua, "quarantine", include_str!("driver_fixture_qua.txt")),
    ];
    for (volume, disc, fixture) in fixtures {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
        if !path.exists() {
            eprintln!("skipped: no {disc} disc image");
            continue;
        }
        assert_eq!(check(fixture, &path, volume), [237, 30, 51, 6, 10, 376], "{disc}");
    }
}

/// A driver for `volume`'s disc.
fn driver(volume: Volume) -> Driver {
    let mut d = Driver::new();
    d.volume = volume;
    d
}

/// Every line of `fixture` played on `volume`'s tables; the scenarios
/// counted by kind.
fn check(fixture: &str, path: &Path, volume: Volume) -> [usize; 6] {
    let t = tables_of(volume);
    let snd = SndData::read(&mut Iso::open(path).unwrap()).unwrap();
    let mut counts = [0; 6];
    for line in fixture.lines().filter(|l| !l.starts_with('#')) {
        let (head, want) = line.split_once(" | ").unwrap_or((line.trim_end_matches(" |"), ""));
        let want: Vec<String> = want.split("; ").filter(|s| !s.is_empty()).map(String::from).collect();
        let a: Vec<i32> = head.split_whitespace().skip(1).filter_map(|s| s.parse().ok()).collect();
        let mut out = Vec::new();
        let kind = head.split_whitespace().next().unwrap();
        match kind {
            "se" => {
                out.push(Command::Msg(driver(volume).se_on(&t.se[a[0] as usize])));
                counts[0] += 1;
            }
            "note" => {
                out.push(Command::Msg(driver(volume).se_note(&t.se[a[0] as usize], a[1] as i8)));
                counts[1] += 1;
            }
            "desktop" => {
                let mut d = driver(volume);
                d.desktop(t, t.wave[a[0] as usize], &mut out);
                counts[2] += 1;
            }
            "volume" => {
                let mut d = driver(volume);
                d.desktop(t, t.wave[a[0] as usize], &mut Vec::new());
                d.set_volumes(a[1], a[2], a[3], &mut out);
                d.frame(t, &mut out);
                counts[3] += 1;
            }
            "jukebox" => {
                let mut d = driver(volume);
                d.desktop(t, t.wave[a[0] as usize], &mut Vec::new());
                d.change(t, t.wave[a[1] as usize], &mut out);
                for _ in 0..20 {
                    d.frame(t, &mut out);
                }
                counts[4] += 1;
            }
            "seq" => {
                let mut d = driver(volume);
                let mut world = World::default();
                let steps = head.split_once(' ').unwrap().1;
                for step in steps.split('+') {
                    let w: Vec<&str> = step.split_whitespace().collect();
                    let n: Vec<i32> = w[1..].iter().filter_map(|s| s.parse().ok()).collect();
                    match w[0] {
                        "load" => d.sq_load(t, world.context(n[0]), &mut out),
                        "set" => world.set(w[1], n[0]),
                        "bgm" => {
                            d.bgm_ctrl(&world.bgm(), &mut out);
                        }
                        "hold" => d.hold_bgm(),
                        "desktop" => {
                            world.set("dtBgm", n[0]);
                            d.desktop(t, t.wave[n[0] as usize], &mut out);
                        }
                        "battle" => d.in_battle = n[0] != 0,
                        "ride" => d.riding = n[0] != 0,
                        "pginit" => d.pg_bgm_init(),
                        "pgend" => d.pg_bgm_end(n[0], &mut out),
                        "breeder" => world.scene.breeder = Some(World::place(&n)),
                        "kite" => world.scene.kite = World::place(&n),
                        "camera" => world.scene.camera = Some(World::place(&n)),
                        "block" => world.scene.block = n[0],
                        // waterTest (Mac Anu) is not run by the harness.
                        "scene" => scene_sound(&mut d, t, &world.scene, &mut out),
                        "play" => d.sq_play(n[0] as usize, &mut out),
                        "stop" => d.sq_stop(n[0] as usize, &mut out),
                        "fade" => d.sq_fade(n[0] as usize, n[1] as u16, n[2], n[3] as u8),
                        "main" => d.set_main_volume(n[0]),
                        "start" => d.game_start = n[0] != 0,
                        "off" => d.all_sound_off(),
                        "frames" => {
                            for _ in 0..n[0] {
                                d.frame(t, &mut out);
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
    counts
}
