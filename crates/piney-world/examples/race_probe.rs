//! Answers `tools/test_race_rs.py`: runs `piney_world::race` (the Flag
//! Race, from Mutation on) on the states it is sent and prints one JSON line
//! per request (`race_probe ISO < requests`). The commands: `split T`,
//! `enter SERVER TIME ROW T0 R0 T1 R1 T2 R2` (main 0x0017a860 on the
//! town's three records), `hud RACE` (0x005fee10), `count TICK COUNT
//! RUNNING PHASE` (0x005fdd10's count), `cam ROT EYE DIST HEIGHT`
//! (0x005ff880) and `flag FLAG T` (0x005fce60 with the camera's fade `T`).

use std::io::{BufRead, Write};
use std::rc::Rc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::save::SaveData;
use piney_world::race::{self, Flag, FlagState, Race, RaceEvent, RaceFiles};

struct Toks<'a>(std::str::SplitWhitespace<'a>);

impl Toks<'_> {
    fn int(&mut self) -> i64 {
        let s = self.0.next().expect("more tokens");
        s.parse().unwrap_or_else(|_| panic!("not a number: {s}"))
    }
    fn i32(&mut self) -> i32 {
        self.int() as i32
    }
    fn u32(&mut self) -> u32 {
        self.int() as u32
    }
    fn v4(&mut self) -> [u32; 4] {
        std::array::from_fn(|_| self.u32())
    }
}

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    let v: Vec<String> = v.into_iter().map(|x| x.to_string()).collect();
    format!("[{}]", v.join(","))
}

fn state(f: FlagState) -> i32 {
    match f {
        FlagState::Out => 0,
        FlagState::Taken => 1,
        FlagState::Hidden => 2,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| "work/mutation/mutation.iso".into());
    let mut iso = Iso::open(&path)?;
    let volume = iso.volume()?;
    let archive = Archive::new(iso.read_path("DATA/DATA.BIN")?)?;
    let files = Rc::new(RaceFiles::read(&archive)?);
    let cells = piney_data::tables::race::of(volume).timer_cells();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let mut t = Toks(line.split_whitespace());
        let Some(cmd) = t.0.next() else { continue };
        let answer = match cmd {
            "split" => list(race::split(t.int() as i16)),
            "enter" => {
                let (server, time, row) = (t.i32(), t.int() as i16, t.int() as i16);
                let mut save = SaveData::new();
                let at = race::RACE_RECORDS + 12 * (server - 1) as usize;
                for k in 0..6 {
                    save.set_i16(at + 2 * k, t.int() as i16);
                }
                let ranks: [(i16, i16); 3] = piney_data::tables::fieldui::of(volume)
                    .race_ranks()
                    .get((server - 1) as usize)
                    .map_or([(0, 0); 3], |r| r.map(|x| (x.time, x.row)));
                let rank = race::enter_time(&mut save, &ranks, server, time, row);
                format!("{{\"rank\":{rank},\"recs\":{}}}", list((0..6).map(|k| save.i16(at + 2 * k))))
            }
            "hud" => {
                let mut r = Race::new(files.clone(), 0, 1);
                r.time = t.int() as i16;
                r.splits = std::array::from_fn(|_| t.int() as i16);
                r.digits = std::array::from_fn(|_| t.int() as i8);
                r.taken = t.int() as i8;
                r.order = std::array::from_fn(|_| t.int() as i8);
                r.state = t.int() as i8;
                r.running = t.int() != 0;
                r.hud(cells);
                let v = [vec![i32::from(r.time)], r.splits.iter().map(|&x| i32::from(x)).collect()].concat();
                let more: Vec<i32> = r
                    .digits
                    .iter()
                    .map(|&x| i32::from(x))
                    .chain([i32::from(r.taken)])
                    .chain(r.order.iter().map(|&x| i32::from(x)))
                    .chain([i32::from(r.state), i32::from(r.running)])
                    .collect();
                let cells: Vec<String> = r
                    .hud_cells()
                    .iter()
                    .map(|c| format!("[{},{},{},{},{},{},{},{}]", c.tex, c.code, c.dx, c.dy, c.sx, c.sy, c.su, c.sv))
                    .collect();
                format!("{{\"race\":{},\"cells\":[{}]}}", list(v.into_iter().chain(more)), cells.join(","))
            }
            "count" => {
                let mut r = Race::new(files.clone(), 0, 1);
                r.tick = t.int() as i16;
                r.count = t.int() as i16;
                r.running = t.int() != 0;
                r.phase = t.i32();
                let start = r.countdown();
                let ev: Vec<String> = r
                    .events
                    .iter()
                    .map(|e| match e {
                        RaceEvent::Se(n) => format!("[\"se\",{n}]"),
                        RaceEvent::Bgm(n) => format!("[\"bgm\",{n}]"),
                        RaceEvent::Map(n) => format!("[\"map\",{n}]"),
                        e => format!("[\"{e:?}\"]"),
                    })
                    .collect();
                format!(
                    "{{\"race\":{},\"start\":{},\"ev\":[{}]}}",
                    list([i32::from(r.tick), i32::from(r.count), i32::from(r.running), r.phase]),
                    u8::from(start),
                    ev.join(",")
                )
            }
            "cam" => {
                let (rot, eye) = (t.v4(), t.v4());
                let (dist, height) = (t.u32(), t.u32());
                let (p, r) = race::before_camera_at(rot, eye, dist, height);
                format!("{{\"pos\":{},\"rot\":{}}}", list(p), list(r))
            }
            "flag" => {
                let n = t.int() as usize;
                let pos = t.v4();
                let mut f = Flag::bare(n, pos, t.u32());
                f.state = match t.int() {
                    0 => FlagState::Out,
                    1 => FlagState::Taken,
                    _ => FlagState::Hidden,
                };
                f.cnt = t.i32();
                f.alpha = t.u32();
                f.taken = t.int() != 0;
                f.entry.pl_dist = t.u32();
                f.set_transparency = t.u32();
                f.entry.fade_flag = t.int() as i16;
                f.entry.fade_cnt = t.int() as i16;
                let fade = t.u32();
                let mut r = Race::new(files.clone(), 0, 1);
                r.taken = t.int() as i8;
                r.order = std::array::from_fn(|_| t.int() as i8);
                let fr = f.main(|| fade);
                if fr.taken {
                    r.flag_taken(n);
                }
                // The draw environment the draw leaves: the shadow's alpha
                // and length (`char_shadow`), the clip's alpha.
                let drawn = f.drawn().map_or("null".to_string(), |a| {
                    let (al, len) = piney_world::draw::char_shadow_env(a, f.height);
                    format!("[{al},{len},{a}]")
                });
                format!(
                    "{{\"flag\":{},\"order\":{},\"taken\":{},\"drawn\":{drawn}}}",
                    list([
                        i64::from(state(f.state)),
                        i64::from(f.cnt),
                        i64::from(f.alpha),
                        i64::from(f.taken),
                        i64::from(f.entry.fade_flag),
                        i64::from(f.entry.fade_cnt)
                    ]),
                    list(r.order),
                    u8::from(fr.taken)
                )
            }
            _ => "{}".to_string(),
        };
        writeln!(out, "{answer}")?;
        out.flush()?;
    }
    Ok(())
}
