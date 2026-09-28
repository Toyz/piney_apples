//! The story areas as the event scripts ask for them (`Host::story_area`):
//! `eventAreaInfo` (`EVENTAREA_INFO[126]` in Infection, 127 from Mutation on)
//! and the Chaos Gate's keywords, the volume's (`piney_data::area`), words
//! found as `WORLD_MAN::GetWordParamFromEvCode` (0x001a29a0) finds them. And
//! [`Announcements`], the boxes `ccEvent::Execute` composes for
//! `ccEvent::DispInfo` (0x001b27b0): `member_add_msg`, `gate_add_msg`,
//! `desktop_item`. The layouts and lines are in docs/engine/event-vm.md.

use std::collections::HashMap;

use piney_data::area::AreaTables;
use piney_data::exe::Image;
use piney_data::save::SaveData;
use piney_event::host::{Announce, StoryArea};

/// Every story area of the volume's tables (`piney_data::area`), by code:
/// what the event scripts' area conditions read. Of the records
/// `GetWordParamFromEvCode` searches (`AreaTables::search`: 126 in
/// Infection, 127 from Mutation on), the first of each code; each word's
/// ID is the first word of its slot with that text.
pub fn from_tables(t: &AreaTables) -> HashMap<i16, StoryArea> {
    let mut out = HashMap::new();
    for e in t.events.iter().take(t.search()) {
        let Ok(code) = i16::try_from(e.code) else { continue };
        if out.contains_key(&code) {
            continue;
        }
        let mut area = StoryArea { server: e.server, ..StoryArea::default() };
        for (k, w) in e.words.iter().enumerate() {
            if let Some(text) = w {
                area.words[k] = t.words.iter().find(|x| x.slot == k && x.text == *text).map(|x| x.id);
            }
        }
        for (j, item) in area.protect_items.iter_mut().enumerate() {
            *item = (e.protect[2 * j], e.protect[2 * j + 1]);
        }
        out.insert(code, area);
    }
    out
}

/// `Execute`'s own literals (INF main .rodata 0x00355e08-0x00355e18): the
/// member's colour, the gate address's and the space between its words.
const MEMBER_HEAD: &[u8] = b"#Y";
const GATE_HEAD: &[u8] = b"#B";
const WORD_SEP: &[u8] = b" ";

/// The lists `Execute` cuts its lines from (`ccKanjiStrSeparate`):
/// `serverStr` (INF main .sdata 0x00377e5c, the five servers' letters) and
/// `getItemMenuStr` (0x00377e0c), the volume's (`tables::fieldui`).
#[derive(Clone, Copy, Debug)]
enum List {
    Server,
    GetItem,
}

/// What the announcements read: the story areas and their words' text,
/// `Execute`'s strings, and the image the save's member names point into.
#[derive(Clone, Debug, Default)]
pub struct Announcements {
    /// Every story area by code, as [`read`] reads them.
    pub areas: HashMap<i16, StoryArea>,
    /// The first record of each code: `wordA`, `wordB`, `wordC` (+0x04,
    /// +0x08, +0x0c) and `server` (+0x10), as `GetEventAreaInfo` finds it.
    records: HashMap<i32, ([Vec<u8>; 3], i32)>,
    /// `bookItemAddMsg`, `bookWallPaperAdd`, `bookBgmAdd`, `bookMovieAdd`
    /// (`tables::world::book`).
    book: [Vec<u8>; 4],
    /// The disc's volume, whose lists [`List`] reads.
    volume: piney_data::volume::Volume,
    /// `spcNameList` (main .bss 0x00387840, `char[17][24]`) as `NewGame`
    /// fills it: members 1-17's names from `charTbl` (DEMO.PRG), where the
    /// save's `spcParam[pc]` name pointers point.
    spc_names: Vec<Vec<u8>>,
    /// Where the volume keeps `spcNameList` (the pointers the save holds).
    spc_name_list: u32,
    /// DEMO.PRG's image, which a name pointer outside `spcNameList` and the
    /// save points into.
    image: Image,
}

impl Announcements {
    /// From the volume's area tables and tables, and DEMO.PRG.
    pub fn read(areas: &AreaTables, demo: &Image) -> piney_data::Result<Self> {
        let mut records = HashMap::new();
        for e in areas.events.iter().take(areas.search()) {
            if records.contains_key(&e.code) {
                continue;
            }
            // The texts one char a byte, as the tables keep them.
            let word = |k: usize| e.words[k].map_or(Vec::new(), |t| t.chars().map(|c| c as u8).collect());
            records.insert(e.code, ([word(0), word(1), word(2)], e.server));
        }
        let volume = piney_data::volume::Volume::from_number(areas.volume).unwrap_or_default();
        let b = piney_data::tables::world::of(volume).book();
        let book = [0, 1, 2, 3].map(|k| b.get(k).map_or_else(Vec::new, |s| piney_data::tables::sjis::encode(s)));
        let image = Image::default().with(demo);
        let tables = piney_demo::newgame::NewGameTables::of(volume);
        let spc_names = tables
            .t
            .chars
            .iter()
            .skip(1)
            .map(|c| piney_demo::newgame::copied(c.base.name, piney_demo::newgame::NAME_COPY))
            .collect();
        Ok(Announcements {
            areas: from_tables(areas),
            records,
            book,
            volume,
            spc_names,
            spc_name_list: tables.t.spc_name_list,
            image,
        })
    }

    /// The string at `p` as the game's memory has it after `NewGame`: in
    /// `spcNameList`, the names it copied (up to 20 bytes of a 24-byte
    /// slot, the rest zero); the save's own address (character 0), the
    /// save's `plName`; anything else, the image.
    fn string_at(&self, p: u32, save: &SaveData) -> Vec<u8> {
        use piney_demo::newgame::{SAVE_VA, SPC_NAME_SIZE};
        let until_nul = |b: &[u8]| b.iter().take_while(|&&c| c != 0).copied().collect::<Vec<u8>>();
        let base = self.spc_name_list;
        let list = base..base + (SPC_NAME_SIZE * self.spc_names.len()) as u32;
        if list.contains(&p) {
            let (k, at) = (((p - base) as usize) / SPC_NAME_SIZE, ((p - base) as usize) % SPC_NAME_SIZE);
            return until_nul(self.spc_names[k].get(at..).unwrap_or_default());
        }
        if (SAVE_VA..SAVE_VA + save.bytes().len() as u32).contains(&p) {
            return until_nul(&save.bytes()[(p - SAVE_VA) as usize..]);
        }
        self.image.cstr(p).unwrap_or_default()
    }

    /// From the disc.
    pub fn from_disc(disc: &mut piney_data::iso::Iso) -> Result<Self, String> {
        use piney_data::exe::Overlay;
        let read = |disc: &mut piney_data::iso::Iso| -> piney_data::Result<Self> {
            let v = disc.volume()?;
            let demo = Overlay::parse(disc.read_path("DATA/DEMO.PRG")?)?.image;
            Announcements::read(AreaTables::of(v), &demo)
        };
        read(disc).map_err(|e| format!("the announcements' text: {e}"))
    }

    /// `ccKanjiStrSeparate(*list, k)`.
    fn piece(&self, list: List, k: i32) -> Vec<u8> {
        let t = piney_data::tables::fieldui::of(self.volume);
        let lines = match list {
            List::Server => t.server_str(),
            List::GetItem => t.get_item_str(),
        };
        lines.get(k.max(0) as usize).map_or_else(Vec::new, |s| piney_data::tables::sjis::encode(s))
    }

    /// The lines `Execute` hands `DispInfo` for `a` (up to three; `save`
    /// holds the members' name pointers).
    pub fn lines(&self, a: Announce, save: &SaveData) -> Vec<Vec<u8>> {
        match a {
            Announce::Member { pc } => {
                let at = piney_data::save::by_id::spc_param(pc as usize);
                let name = self.string_at(save.i32(at) as u32, save);
                let mut l1 = MEMBER_HEAD.to_vec();
                l1.extend_from_slice(&name);
                l1.extend_from_slice(&self.piece(List::GetItem, 6));
                vec![self.piece(List::GetItem, 0), l1]
            }
            Announce::GateAddress { area } => {
                let Some((words, server)) = self.records.get(&i32::from(area)) else { return Vec::new() };
                let mut l0 = GATE_HEAD.to_vec();
                l0.extend_from_slice(&self.piece(List::Server, *server));
                for w in words {
                    l0.extend_from_slice(WORD_SEP);
                    l0.extend_from_slice(w);
                }
                vec![l0, self.piece(List::GetItem, 7)]
            }
            Announce::DesktopItem { ty, id } => {
                let mut l0 = match ty {
                    0..=2 => self.book[1 + ty as usize].clone(),
                    _ => Vec::new(),
                };
                l0.extend_from_slice(&piney_desktop::kanji::dec2sjis(i32::from(id), 3, 0));
                l0.extend_from_slice(&self.book[0]);
                vec![l0]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use piney_data::iso::Iso;

    use super::*;

    /// Every area of `crates/piney-event/tests/vm_fixture.txt` (the game's
    /// own `SetGateList` / `GetWordParamFromEvCode` run in eemu by
    /// `tools/test_event_vm.py`): server, the three words' ids and the
    /// protect items.
    #[test]
    fn story_areas_match_the_game() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let iso = root.join("work/infection/infection.iso");
        let fixture = root.join("crates/piney-event/tests/vm_fixture.txt");
        if !iso.exists() {
            eprintln!("infection.iso not present; skipped");
            return;
        }
        let mut disc = Iso::open(&iso).unwrap();
        let areas = from_tables(AreaTables::of(disc.volume().unwrap()));
        let text = std::fs::read_to_string(fixture).unwrap();
        let mut n = 0;
        for line in text.lines().filter(|l| l.starts_with("area ")) {
            let f: Vec<&str> = line.split_whitespace().collect();
            let num = |s: &str| s.parse::<i32>().unwrap();
            let word = |s: &str| if s == "-" { None } else { Some(num(s)) };
            let code: i16 = f[1].parse().unwrap();
            let a = areas.get(&code).unwrap_or_else(|| panic!("area {code} not read"));
            assert_eq!(a.server, num(f[2]), "area {code} server");
            assert_eq!(a.words, [word(f[3]), word(f[4]), word(f[5])], "area {code} words");
            let items: Vec<(i32, i32)> = (0..4).map(|j| (num(f[6 + 2 * j]), num(f[7 + 2 * j]))).collect();
            assert_eq!(a.protect_items.to_vec(), items, "area {code} protect items");
            n += 1;
        }
        assert!(n > 100, "{n} areas in the fixture");
        // Event 2's area: Bursting Passed Over Aqua Field, server 0.
        assert_eq!(areas[&14].server, 0);
    }
}
