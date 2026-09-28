//! The interpreter against the game's own event code, run in `tools/eemu.py`
//! by `tools/test_event_vm.py`, which writes `vm_fixture.txt` (numbers
//! only). The scripts come from the disc image at run time; the test is
//! skipped when it is not extracted (set `PINEY_ISO` to point elsewhere).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use piney_data::iso::Iso;
use piney_data::save::{InitText, SaveData, offset};
use piney_event::host::{Host, StoryArea};
use piney_event::official;
use piney_event::vm::{Library, Vm};

fn iso_path() -> Option<PathBuf> {
    let p = std::env::var_os("PINEY_ISO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso"));
    p.exists().then_some(p)
}

fn library() -> Option<Arc<Library>> {
    static LIB: OnceLock<Option<Arc<Library>>> = OnceLock::new();
    LIB.get_or_init(|| {
        let mut iso = Iso::open(iso_path()?).unwrap();
        Some(Arc::new(Library::new(iso.volume().unwrap(), official::events(&mut iso).unwrap())))
    })
    .clone()
}

// ---------------------------------------------------------------------------
// The fixture.

type Runs = Vec<(usize, Vec<u8>)>;

#[derive(Default)]
struct Fixture {
    init: Runs,
    areas: HashMap<i16, StoryArea>,
    area_order: Vec<i16>,
    flagset: Vec<(u64, i32, Runs)>,
    flaghash: Vec<(u64, i32, u64, u64)>,
    startth: Vec<(u64, u64)>,
    startev: Vec<(i32, i32, u64)>,
    open: Vec<(u64, i32, u64)>,
    cond: Vec<(u64, usize, Vec<bool>)>,
    desktop_phases: Vec<i32>,
    desktop_calls: Vec<String>,
    desktop_saves: Vec<(i32, u64, Runs)>,
    sub: Vec<(u64, i32, u64, Runs)>,
    play: Vec<(u64, i32, Vec<u64>, u64, u64)>,
}

fn parse_runs(words: &[&str]) -> Runs {
    words
        .iter()
        .map(|w| {
            let (o, b) = w.split_once(':').unwrap();
            let (b, times) = match b.split_once('*') {
                Some((b, n)) => (b, n.parse().unwrap()),
                None => (b, 1),
            };
            let unit: Vec<u8> =
                (0..b.len()).step_by(2).map(|i| u8::from_str_radix(&b[i..i + 2], 16).unwrap()).collect();
            (usize::from_str_radix(o, 16).unwrap(), unit.repeat(times))
        })
        .collect()
}

fn apply(img: &mut SaveData, runs: &Runs) {
    for (at, bytes) in runs {
        img.bytes_mut()[*at..*at + bytes.len()].copy_from_slice(bytes);
    }
}

fn fixture() -> &'static Fixture {
    static F: OnceLock<Fixture> = OnceLock::new();
    F.get_or_init(|| {
        let text = include_str!("vm_fixture.txt");
        let mut f = Fixture::default();
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let w: Vec<&str> = line.split_whitespace().collect();
            match w[0] {
                "init" => f.init.extend(parse_runs(&w[1..])),
                "area" => {
                    let n = |i: usize| w[i].parse::<i32>().unwrap();
                    let word = |i: usize| if w[i] == "-" { None } else { Some(n(i)) };
                    f.area_order.push(n(1) as i16);
                    f.areas.insert(
                        n(1) as i16,
                        StoryArea {
                            server: n(2),
                            words: [word(3), word(4), word(5)],
                            protect_items: [(n(6), n(7)), (n(8), n(9)), (n(10), n(11)), (n(12), n(13))],
                        },
                    );
                }
                "startth" => f.startth.push((w[1].parse().unwrap(), u64::from_str_radix(w[2], 16).unwrap())),
                "startev" => f.startev.push((
                    w[1].parse().unwrap(),
                    w[2].parse().unwrap(),
                    u64::from_str_radix(w[3], 16).unwrap(),
                )),
                "flaghash" => f.flaghash.push((
                    w[1].parse().unwrap(),
                    w[2].parse().unwrap(),
                    u64::from_str_radix(w[3], 16).unwrap(),
                    u64::from_str_radix(w[4], 16).unwrap(),
                )),
                "flagset" => f.flagset.push((w[1].parse().unwrap(), w[2].parse().unwrap(), parse_runs(&w[3..]))),
                "desktop" => match w[1] {
                    "phases" => f.desktop_phases = w[2..].iter().map(|v| v.parse().unwrap()).collect(),
                    "call" => f.desktop_calls.push(w[2..].join(" ")),
                    "save" => f.desktop_saves.push((w[2].parse().unwrap(), w[3].parse().unwrap(), parse_runs(&w[4..]))),
                    _ => panic!("{line}"),
                },
                "sub" => f.sub.push((
                    w[1].parse().unwrap(),
                    w[2].parse().unwrap(),
                    w[3].parse().unwrap(),
                    parse_runs(&w[4..]),
                )),
                "play" => {
                    let frames =
                        if w[3] == "-" { Vec::new() } else { w[3].split(',').map(|v| v.parse().unwrap()).collect() };
                    f.play.push((
                        w[1].parse().unwrap(),
                        w[2].parse().unwrap(),
                        frames,
                        u64::from_str_radix(w[4], 16).unwrap(),
                        u64::from_str_radix(w[5], 16).unwrap(),
                    ));
                }
                "open" => {
                    f.open.push((w[1].parse().unwrap(), w[2].parse().unwrap(), u64::from_str_radix(w[3], 16).unwrap()))
                }
                "cond" => {
                    let count: usize = w[2].parse().unwrap();
                    let hex = w[3];
                    let bits = (0..count)
                        .map(|i| {
                            let digit = hex.len().checked_sub(1 + i / 4).map(|k| hex.as_bytes()[k] as char);
                            digit.is_some_and(|d| d.to_digit(16).unwrap() >> (i % 4) & 1 != 0)
                        })
                        .collect();
                    f.cond.push((w[1].parse().unwrap(), count, bits));
                }
                _ => {}
            }
        }
        f
    })
}

/// The boot's save as the port makes it: `SaveData::boot`, Init's text
/// from the disc (none without it; every test that runs scripts skips then).
fn boot_save() -> SaveData {
    static TEXT: OnceLock<InitText> = OnceLock::new();
    let text = TEXT
        .get_or_init(|| iso_path().and_then(|p| InitText::from_disc(&mut Iso::open(p).ok()?).ok()).unwrap_or_default());
    SaveData::boot(text)
}

/// `timeIdolRankStr`: Init's text, which the fixture leaves out.
const IDOL_TEXT: std::ops::Range<usize> = offset::TIME_IDOL_RANK_STR..offset::TIME_IDOL_RANK_STR + 200;

/// The port's boot save is the game's (the constructor, then `Init(1)`),
/// every byte but Init's text, which `tools/test_save_init_rs.py` checks.
#[test]
fn boot_save_is_the_game_s() {
    let mut want = SaveData::new();
    apply(&mut want, &fixture().init);
    let mut got = boot_save();
    assert!(got.bytes()[IDOL_TEXT].iter().any(|&b| b != 0), "no text read");
    got.bytes_mut()[IDOL_TEXT].fill(0);
    assert!(want == got, "the boot's save differs at +0x{:04x}", {
        (0..want.bytes().len()).find(|&i| want.bytes()[i] != got.bytes()[i]).unwrap()
    });
}

// ---------------------------------------------------------------------------
// The same random saves as the Python side.

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> i64 {
        (self.next() % n) as i64
    }
    fn pick(&mut self, xs: &[i64]) -> i64 {
        xs[self.below(xs.len() as u64) as usize]
    }
}

fn randomise_save(img: &mut SaveData, seed: u64) {
    let mut r = SplitMix(seed);
    let b = img.bytes_mut();
    let mut put = |at: usize, n: usize, v: i64| b[at..at + n].copy_from_slice(&v.to_le_bytes()[..n]);
    for n in 0..512 {
        let x = r.next();
        let mut f = if x & 3 != 0 { r.next() & ((1 << 62) - 1) } else { 0 };
        if (x >> 2) & 7 == 0 {
            f |= 1 << 62;
        }
        if (x >> 5) & 7 == 0 {
            f |= 1 << 63;
        }
        put(0x54f8 + 8 * n, 8, f as i64);
    }
    for i in 0..80 {
        put(0x64f8 + i, 1, r.below(16) - 4);
    }
    for i in 0..512 {
        put(0x2264 + i, 1, r.pick(&[0, 0, 0, 1, 4, 5, 6]));
    }
    let k = r.below(64) as usize;
    for i in 0..512 {
        put(0x2464 + 2 * i, 2, if i < k { r.below(512) } else { -1 });
    }
    for i in 0..128 {
        put(0x2864 + i, 1, r.pick(&[0, 0, 1, 3]));
    }
    for i in 0..128 * 48 {
        put(0x28e4 + i, 1, r.pick(&[0, 0, 0, 1, 3, 7]));
    }
    for i in 0..5 {
        let mut v = r.next() & 0xffff_ffff;
        if i == 1 && r.below(2) != 0 {
            v |= 0x8000_0000;
        }
        put(0x2220 + 4 * i, 4, v as i64);
    }
    put(0x2234, 2, r.next() as i64);
    for i in 0..11 {
        put(0x2238 + 4 * i, 4, r.next() as i64);
    }
    for i in 0..25 {
        put(0x4fe4 + 4 * i, 4, r.next() as i64);
        put(0x5048 + 4 * i, 4, r.next() as i64);
    }
    for i in 0..15 {
        put(0x523c + 4 * i, 4, r.next() as i64);
    }
    for i in 0..5 * 64 {
        put(0x5278 + 2 * i, 2, r.below(151) - 1);
    }
    for i in 0..32 * 4 {
        put(0x6548 + i, 1, r.below(5) - 1);
    }
    for i in 0..5 {
        put(0x65c8 + 4 * i, 4, r.next() as i64);
    }
    for i in 0..18 {
        put(0x220c + i, 1, r.below(8));
    }
    put(0x6771, 1, r.below(2));
    put(0x6772, 1, r.below(2));
    for pc in 0..18 {
        for k in 0..40 {
            let at = 0x30 + 160 * pc + 4 * k;
            if r.below(3) != 0 {
                put(at, 2, -1);
                put(at + 2, 1, -1);
                put(at + 3, 1, 0);
            } else {
                put(at, 2, r.below(100));
                put(at + 2, 1, r.below(16));
                put(at + 3, 1, r.below(100));
            }
        }
        for k in 0..20 {
            put(0x1ec4 + 40 * pc + 2 * k, 2, if k < 4 { r.below(312) - 1 } else { -1 });
        }
        put(0x7488 + 0xdc * pc + 0x14, 4, r.below(10_000_500));
        put(0x7488 + 0xdc * pc + 0xda, 2, r.below(1100) - 50);
    }
    for i in 0..320 {
        let v = if r.below(4) == 0 { r.below(100) } else { 0 };
        put(0xcfc + i, 1, v);
    }
    for i in 0..17 {
        put(0x73b8 + 4 * i, 4, r.below(7560 * 3));
    }
    put(0x8426, 1, r.below(6));
    put(0x842a, 1, r.below(3));
    let parody = if r.below(4) == 0 { 1 } else { 0 };
    put(0x842b, 1, parody);
}

// ---------------------------------------------------------------------------
// A host with the fixture's story areas.

struct TestHost {
    save: SaveData,
}

impl Host for TestHost {
    fn save(&mut self) -> &mut SaveData {
        &mut self.save
    }
    fn story_area(&self, area: i16) -> Option<StoryArea> {
        fixture().areas.get(&area).copied()
    }
}

fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xCBF2_9CE4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01B3))
}

/// `mng_digest` in tools/test_event_vm.py: the registers at their offsets in `ccEvent`.
fn mng_digest(m: &piney_event::vm::EventMng) -> u64 {
    let mut raw = Vec::new();
    for v in [m.enable_phase, m.msg_select, m.msg_num, m.entry_npc_num, m.regist_npc_num] {
        raw.extend(v.to_le_bytes());
    }
    for (t, c) in m.targets {
        raw.extend(t.to_le_bytes());
        raw.extend(c.to_le_bytes());
    }
    for e in m.entries.iter().chain(&m.entries_mc) {
        e.iter().for_each(|v| raw.extend(v.to_le_bytes()));
    }
    for (c, x) in m.area_codes {
        raw.extend(c.to_le_bytes());
        raw.extend(x.to_le_bytes());
    }
    for p in m.positions {
        raw.extend(p.floor.to_le_bytes());
        raw.extend(p.block.to_le_bytes());
        raw.extend(p.num.to_le_bytes());
    }
    for p in m.points {
        raw.extend(p.floor.to_le_bytes());
        raw.extend(p.block.to_le_bytes());
        raw.extend(p.num.to_le_bytes());
    }
    let c = m.current;
    for v in [c.phase, c.phase_comp, c.status] {
        raw.extend(v.to_le_bytes());
    }
    c.scene.iter().for_each(|v| raw.extend(v.to_le_bytes()));
    for v in [c.flag, c.status_index, c.status_num, c.status_comp, 0] {
        raw.extend(v.to_le_bytes());
    }
    raw.extend((c.range as i32).to_le_bytes());
    raw.extend(m.operate.to_le_bytes());
    raw.extend(m.operate_set.to_le_bytes());
    m.area_code_set.iter().for_each(|v| raw.extend(v.to_le_bytes()));
    fnv(&raw)
}

fn first_difference(a: &SaveData, b: &SaveData) -> String {
    let (a, b) = (a.bytes(), b.bytes());
    let i = (0..a.len()).find(|&i| a[i] != b[i]).unwrap();
    let j = (i..a.len()).rev().find(|&j| a[j] != b[j]).unwrap();
    format!(
        "bytes 0x{i:x}..=0x{j:x}: port {:02x?} game {:02x?}",
        &a[i..(i + 16).min(a.len())],
        &b[i..(i + 16).min(b.len())]
    )
}

#[test]
fn flag_set_matches_the_game_byte_for_byte() {
    let Some(lib) = library() else {
        eprintln!("no Infection image; skipped");
        return;
    };
    let f = fixture();
    assert!(!f.flagset.is_empty());
    let mut bad = Vec::new();
    for (seed, event, runs) in &f.flagset {
        let mut base = boot_save();
        if *seed != 0 {
            randomise_save(&mut base, *seed);
        }
        let mut want = base.clone();
        apply(&mut want, runs);
        let mut host = TestHost { save: base };
        let mut vm = Vm::new(lib.clone());
        vm.flag_set(*event, &mut host);
        if host.save != want {
            bad.push(format!("seed {seed} event {event}: {}", first_difference(&host.save, &want)));
        }
    }
    for (seed, event, hash, mng) in &f.flaghash {
        let mut base = boot_save();
        randomise_save(&mut base, *seed);
        let mut host = TestHost { save: base.clone() };
        let mut vm = Vm::new(lib.clone());
        vm.flag_set(*event, &mut host);
        if fnv(host.save.bytes()) != *hash {
            bad.push(format!(
                "seed {seed} event {event}: the save differs; the port changed {}",
                first_difference(&host.save, &base)
            ));
        } else if mng_digest(&vm.mng) != *mng {
            bad.push(format!("seed {seed} event {event}: the event manager differs: {:?}", vm.mng));
        }
    }
    let n = f.flagset.len() + f.flaghash.len();
    assert!(bad.is_empty(), "{} of {n} differ:\n{}", bad.len(), bad[..bad.len().min(20)].join("\n"));
    eprintln!("{n} ccEventFlagSet runs match the game");
}

// ---------------------------------------------------------------------------
// Conditions: the same random worlds as the Python side.

use piney_event::host::{Boss, CharRef, EntryList, Game, Marker, Party};
use piney_event::ir::Cond;
use piney_event::vm::EvPoint;

const POSITION_NUMS: [i32; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 31, 40, 50];

struct World {
    game: Game,
    phase: i32,
    msg_num: i32,
    msg_select: i32,
    operate: u64,
    operate_set: i16,
    area_code_set: [i16; 3],
    points: [(i16, i16, i32); 16],
    positions: [(i16, i16, i32, f32); 16],
    party: Party,
    spc: Vec<i32>,
    target: Option<(u32, i16, bool)>,
    chars: Vec<(u32, i16)>,
    gimmicks: Vec<(u32, i16, u8)>,
    mc_num: i32,
    boss_entry: i32,
    boss_task: Option<i32>,
    active: i32,
    menu: i32,
    pad: u32,
    enemy_pp: Vec<i16>,
    town_marker_x: Vec<f32>,
}

impl World {
    fn new(seed: u64, stories: &[(i16, StoryArea)]) -> World {
        let mut r = SplitMix(seed ^ 0x5EED_5EED);
        let status = r.pick(&[1, 2, 3, 5, 5, 5]) as i32;
        let area = r.below(3) as i32;
        let mut server = r.below(5) as i32;
        let town = r.below(6) as i32;
        let field = r.below(4) as i32;
        let dungeon = r.below(4) as i32;
        let floor = r.below(4) as i32;
        let block = r.below(6) as i32;
        let phase = r.below(7) as i32 - 1;
        let msg_num = r.below(13) as i32;
        let msg_select = r.below(4) as i32;
        let operate = r.next() & ((1 << 29) - 1);
        let operate_set = (r.below(22) - 1) as i16;
        let area_code_set = if r.below(2) != 0 {
            let (_, st) = stories[r.below(stories.len() as u64) as usize];
            let set = st.words.map(|w| w.unwrap_or(0) as i16);
            if r.below(2) != 0 {
                server = st.server;
            }
            set
        } else {
            std::array::from_fn(|_| (r.below(40) - 1) as i16)
        };
        let points = std::array::from_fn(|_| {
            if r.below(3) == 0 { (r.below(4) as i16, r.below(6) as i16, r.below(11) as i32) } else { (-1, -1, -1) }
        });
        let positions =
            std::array::from_fn(|k| (r.below(4) as i16, r.below(6) as i16, POSITION_NUMS[k], r.below(500) as f32));
        let mut picks = vec![-1i64, -1];
        picks.extend(1..18);
        let ids = [0, r.pick(&picks) as i32, r.pick(&picks) as i32];
        let num = ids.iter().filter(|&&x| x != -1).count() as i32;
        let mut spc: Vec<i32> = ids.iter().copied().filter(|&x| x != -1).collect();
        for _ in 0..5 {
            spc.push(r.below(20) as i32);
        }
        spc.truncate(5);
        let target = if r.below(2) != 0 {
            let t = 1u32 << r.pick(&[2, 3]);
            Some((t, r.below(140) as i16, r.below(2) != 0))
        } else {
            None
        };
        let chars = (0..r.below(4)).map(|_| ((r.next() & 0xffff_ffff) as u32, (125 + r.below(40)) as i16)).collect();
        let gimmicks = (0..r.below(3))
            .map(|_| ((r.next() & 0xffff_ffff) as u32, r.below(40) as i16, r.below(256) as u8))
            .collect();
        let mc_num = r.below(2) as i32;
        let boss_entry = r.below(17) as i32 - 1;
        let boss_task = match r.below(3) {
            0 => None,
            k => Some(k as i32 - 1),
        };
        let active = r.below(2) as i32;
        let menu = r.pick(&[-1, -1, 0, 62]) as i32;
        let pad = (r.next() & 0xffff_ffff) as u32;
        let enemy_pp = (0..64).map(|_| r.below(2) as i16).collect();
        let town_marker_x = (0..33).map(|_| r.below(500) as f32).collect();
        World {
            game: Game { status, area, area_prev: area, server, town, field, dungeon, floor, block, cnt_stop: 0 },
            phase,
            msg_num,
            msg_select,
            operate,
            operate_set,
            area_code_set,
            points,
            positions,
            party: Party { ids, num },
            spc,
            target,
            chars,
            gimmicks,
            mc_num,
            boss_entry,
            boss_task,
            active,
            menu,
            pad,
            enemy_pp,
            town_marker_x,
        }
    }

    /// The event manager's registers as `set_world` writes them.
    fn apply(&self, vm: &mut Vm) {
        let m = &mut vm.mng;
        m.enable_phase = self.phase;
        m.msg_num = self.msg_num;
        m.msg_select = self.msg_select;
        m.operate = self.operate;
        m.operate_set = self.operate_set;
        m.area_code_set = self.area_code_set;
        for (k, &(floor, block, num)) in self.points.iter().enumerate() {
            m.points[k] = EvPoint { floor, block, num };
        }
        for (k, &(floor, block, num, x)) in self.positions.iter().enumerate() {
            m.positions[k].floor = floor;
            m.positions[k].block = block;
            m.positions[k].num = num;
            m.positions[k].pos = [x, 0.0, 0.0, 1.0];
        }
        m.operate_target = self.target.map(|(types, code, _)| CharRef { handle: 1, types, code });
    }
}

struct WorldHost<'a> {
    save: SaveData,
    w: &'a World,
    /// For played blocks: the staff roll's task finishes after one frame.
    roll: u32,
}

impl Host for WorldHost<'_> {
    fn save(&mut self) -> &mut SaveData {
        &mut self.save
    }
    fn story_area(&self, area: i16) -> Option<StoryArea> {
        fixture().areas.get(&area).copied()
    }
    fn game(&self) -> Game {
        self.w.game
    }
    fn party(&self) -> Party {
        self.w.party
    }
    fn spc_present(&self, code: i16) -> bool {
        self.w.spc.contains(&(code as i32))
    }
    fn entry_present(&self, list: EntryList, ty: i16, code: i16) -> bool {
        let bit = 1u32 << (ty as i32 & 31);
        match list {
            EntryList::Characters => self.w.chars.iter().any(|&(t, c)| t & bit != 0 && c == code),
            EntryList::Gimmicks => self.w.gimmicks.iter().any(|&(t, c, _)| t & bit != 0 && c == code),
        }
    }
    fn boss(&self) -> Option<Boss> {
        Some(Boss { entry: self.w.boss_entry, task_param: self.w.boss_task })
    }
    fn no_active_object(&self) -> bool {
        self.w.active & 0xff != 0
            && !self.w.gimmicks.iter().any(|&(t, _, f)| f & 1 != 0 && t & 0x10_0000 != 0 && f & 0x40 == 0)
    }
    fn no_entries(&self) -> bool {
        self.w.chars.is_empty() && self.w.mc_num == 0
    }
    fn field_menu(&self) -> i32 {
        self.w.menu
    }
    fn pad_pushed(&self) -> u32 {
        self.w.pad
    }
    fn enemy_pp(&self, enemy: i16) -> bool {
        self.w.enemy_pp[(enemy & 63) as usize] != 0
    }
    fn marker(&self, marker: i16) -> Option<Marker> {
        let x = usize::try_from(marker).ok().and_then(|i| self.w.town_marker_x.get(i)).copied().unwrap_or(0.0);
        Some(Marker { pos: [x, 0.0, 0.0, 1.0], dirc: 0.0 })
    }
    fn player_distance(&self, pos: Option<[f32; 4]>) -> f32 {
        pos.expect("the test world always has the position")[0]
    }
    fn target_alive(&self, _t: &CharRef) -> bool {
        self.w.target.is_some_and(|t| t.2)
    }
    fn camera_type(&self) -> i32 {
        1
    }
    fn busy(&mut self, w: piney_event::host::Wait) -> bool {
        if w == piney_event::host::Wait::StaffRoll {
            self.roll += 1;
            return self.roll == 1;
        }
        false
    }
    fn begin(&mut self, w: piney_event::host::Wait) {
        if w == piney_event::host::Wait::StaffRoll {
            self.roll = 0;
        }
    }
}

fn stories() -> Vec<(i16, StoryArea)> {
    let mut v: Vec<_> = fixture().areas.iter().map(|(k, a)| (*k, *a)).collect();
    // The Python side draws from eventAreaInfo's order; keep it.
    v.sort_by_key(|(k, _)| fixture().area_order.iter().position(|o| o == k).unwrap());
    v
}

#[test]
fn check_open_matches_the_game() {
    let Some(lib) = library() else { return };
    let f = fixture();
    let stories = stories();
    let mut worlds: HashMap<u64, World> = HashMap::new();
    let mut bad = Vec::new();
    for &(seed, event, want) in &f.open {
        let w = worlds.entry(seed).or_insert_with(|| World::new(seed, &stories));
        let mut save = boot_save();
        randomise_save(&mut save, seed);
        let mut host = WorldHost { save, w, roll: 0 };
        let mut vm = Vm::new(lib.clone());
        w.apply(&mut vm);
        let got = vm.open_results(event, &mut host).unwrap();
        let got = got.iter().enumerate().fold(0u64, |v, (i, &x)| v | (x as u64) << i);
        if got != want {
            bad.push(format!("seed {seed} event {event}: port {got:x} game {want:x}"));
        }
    }
    assert!(bad.is_empty(), "{} of {} differ:\n{}", bad.len(), f.open.len(), bad[..bad.len().min(20)].join("\n"));
    eprintln!("{} header-and-block CheckOpen walks match the game", f.open.len());
}

#[test]
fn every_condition_matches_the_game() {
    let Some(lib) = library() else { return };
    let f = fixture();
    let stories = stories();
    let conds: Vec<(i32, Cond)> = (0..500)
        .filter_map(|n| lib.script(n).map(|s| (n, s)))
        .flat_map(|(n, s)| s.open.iter().chain(s.blocks.iter().flat_map(|b| b.conds.iter())).map(move |c| (n, *c)))
        .collect();
    let mut bad = Vec::new();
    let mut passed = 0;
    for (seed, count, want) in &f.cond {
        assert_eq!(*count, conds.len());
        let w = World::new(*seed, &stories);
        let mut save = boot_save();
        randomise_save(&mut save, *seed);
        let mut host = WorldHost { save, w: &w, roll: 0 };
        let mut vm = Vm::new(lib.clone());
        w.apply(&mut vm);
        for (i, (n, c)) in conds.iter().enumerate() {
            let got = vm.check_conditions(*n, std::slice::from_ref(c), &mut host);
            passed += got as usize;
            if got != want[i] {
                bad.push(format!("seed {seed} event {n} {c}: port {got} game {}", want[i]));
            }
        }
    }
    assert!(bad.is_empty(), "{} differ:\n{}", bad.len(), bad[..bad.len().min(30)].join("\n"));
    eprintln!("{} conditions x {} worlds match the game ({passed} passed)", conds.len(), f.cond.len());
}

// ---------------------------------------------------------------------------
// The event task: a new game reaching the desktop.

use piney_event::host::{Announce, DesktopMenu, MessageCall, MessageKind, Overlay};

/// Logs the calls the Python side hooks, with the frame they happen in.
struct DesktopHost {
    save: SaveData,
    frame: u64,
    calls: Vec<String>,
}

impl DesktopHost {
    fn log(&mut self, s: String) {
        self.calls.push(format!("{} {s}", self.frame));
    }
}

impl Host for DesktopHost {
    fn save(&mut self) -> &mut SaveData {
        &mut self.save
    }
    fn game(&self) -> Game {
        Game { status: 2, ..Game::default() }
    }
    fn desktop_menu(&self) -> DesktopMenu {
        DesktopMenu { menu_type: -1, request: -1 }
    }
    fn clear_gate_hack(&mut self) {
        self.log("clear_gate_hack".into());
    }
    fn set_frame_rate(&mut self, rate: i16) {
        self.log(format!("frame_rate {rate}"));
    }
    fn load_overlay(&mut self, which: Overlay) {
        let n = match which {
            Overlay::Demo => 0,
            Overlay::Desktop => 1,
            Overlay::Toppage => 2,
            Overlay::Gcmn => 3,
        };
        self.log(format!("overlay {n}"));
    }
    fn stream(&mut self, num: i16) {
        self.log(format!("stream {num}"));
    }
    fn message_open(&mut self, c: &MessageCall<'_>) {
        let kind = if c.kind == MessageKind::Speech { "message" } else { "info" };
        self.log(format!("{kind} {} {}", c.event, c.msg));
    }
    fn announce(&mut self, _a: Announce) {
        self.log("info -1 -1".into());
    }
    fn name_entry_start(&mut self) {
        self.log("name_entry".into());
    }
    fn sound_effect(&mut self, se: i32) {
        self.log(format!("se {se}"));
    }
    fn sound(&mut self, cmd: i16, p0: i16, p1: i16, p2: i16) {
        self.log(format!("sound {cmd} {p0} {p1} {p2}"));
    }
    fn menu_ban(&mut self, on: bool) {
        self.log(format!("menu_ban {}", on as i32));
    }
    fn change_request(&mut self, num: i32, sf: i32) {
        self.log(format!("change_request {num} {sf}"));
    }
}

#[test]
fn a_new_game_reaches_the_desktop_as_in_the_game() {
    let Some(lib) = library() else { return };
    let f = fixture();
    assert!(!f.desktop_phases.is_empty());
    let mut host = DesktopHost { save: boot_save(), frame: 1, calls: Vec::new() };
    let mut vm = Vm::new(lib);
    // ccThMother: ccStartEvent(1, 0). ccSetupDesktop: ccStartThEvent, ccEnableThEvent(0).
    vm.start_event(1, 0, &mut host);
    vm.start_thread(&mut host);
    vm.enable(0);
    // tools/test_event_vm.py DESKTOP_ACTIONS: after the Nth frame of play.
    let actions: [(i32, &str, i32); 5] =
        [(3, "operate", 3), (40, "operate", 1), (45, "read", 4), (46, "read", 5), (60, "operate", 3)];
    let extra = 90;
    let (mut stage, mut left) = (0, extra);
    let mut phases = Vec::new();
    let mut saves = Vec::new();
    'run: for frame in 1..=10_000u64 {
        host.frame = frame;
        vm.frame(&mut host);
        let phase = vm.phase();
        phases.push(phase);
        match (stage, phase) {
            (0, 1) => {
                saves.push((0, frame, host.save.clone()));
                vm.enable(2);
                stage = 1;
            }
            (1, 3) => {
                saves.push((2, frame, host.save.clone()));
                vm.enable(4);
                stage = 2;
            }
            (2, 5) => {
                left -= 1;
                let done = extra - left;
                for &(at, what, v) in &actions {
                    if at != done {
                        continue;
                    }
                    if what == "operate" {
                        let go = vm.check_operate(v, 0, &mut host);
                        host.calls.push(format!("{frame} check_operate {v} {}", go as i32));
                    } else {
                        host.save.set_mail(v as usize, 4);
                    }
                }
                if left == 0 {
                    saves.push((4, frame, host.save.clone()));
                    break 'run;
                }
            }
            _ => {}
        }
    }
    assert_eq!(phases, f.desktop_phases, "the phase after each frame");
    assert_eq!(host.calls, f.desktop_calls, "calls and the frames they happen in");
    assert_eq!(saves.len(), f.desktop_saves.len());
    for ((stage, frame, save), (want_stage, want_frame, runs)) in saves.iter().zip(&f.desktop_saves) {
        assert_eq!((stage, frame), (want_stage, want_frame));
        let mut want = boot_save();
        apply(&mut want, runs);
        assert!(*save == want, "phase {stage}: {}", first_difference(save, &want));
    }
    eprintln!("new game to the desktop: {} frames, {} calls, 3 saves match the game", phases.len(), host.calls.len());
}

/// The world for playing whole events and blocks: menus closed, every
/// button held (as `write_fixture` sets it for `sub` and `play`).
fn play_world(seed: u64, stories: &[(i16, StoryArea)]) -> World {
    let mut w = World::new(seed, stories);
    w.menu = -1;
    w.pad = 0xffff_ffff;
    w
}

#[test]
fn played_events_match_the_game() {
    let Some(lib) = library() else { return };
    let f = fixture();
    let stories = stories();
    let mut bad = Vec::new();
    let mut frames_total = 0;
    for (seed, event, frames, runs) in &f.sub {
        let w = play_world(*seed, &stories);
        let mut base = boot_save();
        randomise_save(&mut base, *seed);
        let mut want = base.clone();
        apply(&mut want, runs);
        let mut host = WorldHost { save: base, w: &w, roll: 0 };
        let mut vm = Vm::new(lib.clone());
        w.apply(&mut vm);
        let got = vm.run_event(*event, &mut host);
        frames_total += got;
        if got != *frames || host.save != want {
            bad.push(format!(
                "seed {seed} event {event}: frames port {got} game {frames}; {}",
                if host.save == want { "save ok".into() } else { first_difference(&host.save, &want) }
            ));
        }
    }
    assert!(bad.is_empty(), "{} of {} differ:\n{}", bad.len(), f.sub.len(), bad[..bad.len().min(20)].join("\n"));
    eprintln!("{} eventSub runs at lv 2 match the game ({frames_total} frames)", f.sub.len());
}

#[test]
fn every_block_played_matches_the_game() {
    let Some(lib) = library() else { return };
    let f = fixture();
    let stories = stories();
    let mut bad = Vec::new();
    let (mut blocks, mut frames_total) = (0, 0);
    for (seed, event, frames, hash, mng) in &f.play {
        let w = play_world(*seed, &stories);
        let mut base = boot_save();
        randomise_save(&mut base, *seed);
        let mut host = WorldHost { save: base.clone(), w: &w, roll: 0 };
        let mut vm = Vm::new(lib.clone());
        w.apply(&mut vm);
        let nblocks = lib.script(*event).map_or(0, |s| s.blocks.len());
        let got: Vec<u64> = (0..nblocks).map(|b| vm.play_block(*event, b, &mut host)).collect();
        blocks += got.len();
        frames_total += got.iter().sum::<u64>();
        if got != *frames {
            let at = got.iter().zip(frames).position(|(a, b)| a != b).unwrap_or(0);
            bad.push(format!(
                "seed {seed} event {event} block {at}: frames port {:?} game {:?}",
                got.get(at),
                frames.get(at)
            ));
        } else if fnv(host.save.bytes()) != *hash {
            bad.push(format!(
                "seed {seed} event {event}: the save differs; the port changed {}",
                first_difference(&host.save, &base)
            ));
        } else if mng_digest(&vm.mng) != *mng {
            bad.push(format!("seed {seed} event {event}: the event manager differs: {:?}", vm.mng));
        }
    }
    assert!(bad.is_empty(), "{} of {} differ:\n{}", bad.len(), f.play.len(), bad[..bad.len().min(30)].join("\n"));
    eprintln!("{blocks} blocks played at lv 2 match the game ({frames_total} frames)");
}

#[test]
fn start_thread_and_start_event_match_the_game() {
    let Some(lib) = library() else { return };
    let f = fixture();
    assert!(!f.startth.is_empty() && !f.startev.is_empty());
    for &(seed, hash) in &f.startth {
        let mut save = boot_save();
        randomise_save(&mut save, seed);
        let mut host = TestHost { save };
        Vm::new(lib.clone()).start_thread(&mut host);
        assert_eq!(fnv(host.save.bytes()), hash, "ccStartThEvent on seed {seed}");
    }
    for &(vol, flag, hash) in &f.startev {
        let mut host = TestHost { save: boot_save() };
        Vm::new(lib.clone()).start_event(vol, flag, &mut host);
        assert_eq!(fnv(host.save.bytes()), hash, "ccStartEvent({vol}, {flag})");
    }
}

#[test]
fn the_task_runs_every_mode_on_random_saves() {
    let Some(lib) = library() else { return };
    let stories = stories();
    let (mut blocks, mut faults) = (0, 0);
    for seed in 300..310u64 {
        for status in [2, 3, 5] {
            let mut w = play_world(seed, &stories);
            w.game.status = status;
            let mut save = boot_save();
            randomise_save(&mut save, seed);
            let mut host = WorldHost { save, w: &w, roll: 0 };
            let mut vm = Vm::new(lib.clone());
            vm.trace = Some(Vec::new());
            vm.start_thread(&mut host);
            for phase in [0, 2, 4] {
                vm.enable(phase);
                for _ in 0..100_000 {
                    if vm.enable_settled(phase) {
                        break;
                    }
                    vm.frame(&mut host);
                }
            }
            for _ in 0..600 {
                vm.frame(&mut host);
            }
            blocks +=
                vm.trace.as_ref().unwrap().iter().filter(|t| matches!(t, piney_event::vm::Trace::Block { .. })).count();
            faults += vm.faults.len();
        }
    }
    assert!(blocks > 0);
    eprintln!("30 runs: {blocks} blocks played, {faults} faults");
}

/// A new game on Mutation: `ccStartEvent(2, 0)` counts Infection cleared
/// and marks event 100 done, and the setup's first pass walks Mutation's
/// own main story (100-149, after S1), which opens M201 (name entry, the
/// opening streams, the first mails).
#[test]
fn mutation_s_setup_opens_its_own_story() {
    let v = piney_data::volume::Volume::Mut;
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    let Ok(mut iso) = piney_data::iso::Iso::open(&path) else { return };
    let lib = Arc::new(Library::new(v, official::events(&mut iso).unwrap()));
    let mut host = TestHost { save: boot_save() };
    let mut vm = Vm::new(lib);
    vm.trace = Some(Vec::new());
    vm.start_event(2, 0, &mut host);
    assert!(host.save.event_flag(100) & piney_event::state::DONE != 0);
    vm.start_thread(&mut host);
    vm.enable(0);
    for _ in 0..1000 {
        if vm.enable_settled(0) {
            break;
        }
        vm.frame(&mut host);
    }
    let first = vm.trace.as_ref().unwrap().iter().find_map(|t| match *t {
        piney_event::vm::Trace::Block { event, block, .. } => Some((event, block)),
        _ => None,
    });
    assert_eq!(first, Some((101, 0)));
}
