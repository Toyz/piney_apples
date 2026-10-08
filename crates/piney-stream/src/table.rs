//! `streamTbl` / `streamTblE` (INF main 0x0030ef90 and 0x003102f0): what a
//! stream number plays (`docs/engine/stream.md`, "The tables"). Each entry
//! (134 on Infection, 140 from Mutation on: [`count`]) points at a
//! `STREAMDATA` list (20-byte records ended by one with no name): the
//! header (archive, offset, music bits), then the files in order. `ccStreamInit` (0x00198ee0) picks `streamTblE` and the `E` archives
//! when `saveData.voice` (+0x842c) is set; Outbreak and Quarantine have
//! `streamTblE` alone and take it whatever the voice. The tables are the
//! volume's generated ones (`tables::stream`).

use piney_data::tables::stream;
use piney_data::tables::types::Streamdata;
use piney_data::volume::Volume;
use piney_data::{Error, Result};

/// Entries in each of the volume's tables: 134 on Infection, 140 from
/// Mutation on, whose six new streams before `str6100` move the later ones
/// up by six (the gate's 107 to 113, the Ryu Books' covers 112 to 118).
pub fn count(volume: Volume) -> usize {
    stream::of(volume).lists_e().len()
}
/// `sizeof(STREAMDATA)`.
pub const RECORD: u32 = 0x14;

/// `STREAMDATA.flag` bits.
pub const FLAG_PAUSE: i16 = 0x01;
pub const FLAG_ALT: i16 = 0x02;
pub const FLAG_ALT_REST: i16 = 0x04;
pub const FLAG_PRELOAD: i16 = 0x10;
pub const FLAG_SKIP_START: i16 = 0x40;
pub const FLAG_NO_SKIP: i16 = 0x80;

/// `STREAMDATA.type` values.
pub const TYPE_SETUP: i16 = 0;
pub const TYPE_FOLLOWED: i16 = 1;
pub const TYPE_LAST: i16 = -1;
pub const TYPE_ON_MEMORY: i16 = -2;

/// The `ccCd` file records the header's `type` indexes (`ccCdInit`
/// 0x00159620: +0x54 + 36 type, then the English ones): the common
/// streams, each volume's own (streams 0-20 and 1-4's, 21-49, 50-74,
/// 75-111), and from Mutation on the shared `STRSUB` as type 5 (126-133).
/// The later volumes' `ccCdInit`s store each name at its type's record
/// (Mutation's `STRSUB` at +0x108, Outbreak's `STRSUBE` at +0x204). A disc
/// has the archives up to its own volume's.
pub const ARCHIVES: [(&str, &str); 6] = [
    ("STREAM/STRCMN.BIN", "STREAM/STRCMNE.BIN"),
    ("STREAM/STR1.BIN", "STREAM/STR1E.BIN"),
    ("STREAM/STR2.BIN", "STREAM/STR2E.BIN"),
    ("STREAM/STR3.BIN", "STREAM/STR3E.BIN"),
    ("STREAM/STR4.BIN", "STREAM/STR4E.BIN"),
    ("STREAM/STRSUB.BIN", "STREAM/STRSUBE.BIN"),
];

/// Whether `strSndTbl[num]` (INF main 0x0030b950: per stream `{strse,
/// strbgm}`) has a BGM table; a note's event 4 calls `ccSndStreamBGM` only
/// when `strbgm` is set.
pub fn has_stream_bgm(volume: Volume, num: usize) -> bool {
    stream::of(volume).bgm().get(num).is_some_and(|t| t.strbgm.is_some())
}

/// One record of a `strSndTbl` BGM table (`strbgm`, 12 bytes: `sq`, `sq2`,
/// `time`, `vol`, `unknown_8`, `cmd`), what `ccSndStreamBGM` (0x0017cb20) acts
/// on at a note of event 4 (`piney_audio::stream` carries it out; the layout
/// and commands are in docs/engine/sound.md, "Streams"). Three streams have
/// one: 3, 8 and 58.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrBgm {
    pub sq: i16,
    pub sq2: i16,
    pub time: i16,
    pub vol: u16,
    pub unknown_8: i16,
    pub cmd: i16,
}

/// A BGM table's end: `sq` below 0 and `cmd` 5.
pub const BGM_END: i16 = 5;
/// `strSndTbl[num].strbgm` up to and with its end record (empty for none).
pub fn stream_bgm_table(volume: Volume, num: usize) -> Vec<StrBgm> {
    let rows = stream::of(volume).bgm().get(num).and_then(|t| t.strbgm).unwrap_or_default();
    rows.iter()
        .map(|r| StrBgm {
            sq: r.bgm1,
            sq2: r.bgm2,
            time: r.fade_time,
            vol: r.vol1 as u16,
            unknown_8: r.vol2,
            cmd: r.param,
        })
        .collect()
}

/// `ccSnd +0xe8`, the cursor `strSeInit` (0x00183f40) sets on
/// `strSndTbl[num].strbgm` and `ccSndStreamBGM` walks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BgmCursor {
    records: Vec<StrBgm>,
    pos: usize,
}

impl BgmCursor {
    pub fn new(records: Vec<StrBgm>) -> BgmCursor {
        BgmCursor { records, pos: 0 }
    }

    /// `ccSnd +0xe8` is set: `ccSndStreamSE` calls `ccSndStreamBGM`.
    pub fn is_set(&self) -> bool {
        !self.records.is_empty()
    }

    /// `ccSndStreamBGM`'s walk: the record to carry out, if any. A record
    /// with `sq` below 0 is passed over (nothing done), unless it is the
    /// end, where the cursor stays; any other is carried out and passed.
    pub fn step(&mut self) -> Option<StrBgm> {
        let r = *self.records.get(self.pos)?;
        if r.sq < 0 {
            if r.cmd != BGM_END {
                self.pos += 1;
            }
            return None;
        }
        self.pos += 1;
        Some(r)
    }
}

/// One `STREAMDATA` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub ofs: i32,
    pub size: i32,
    pub kind: i16,
    pub flag: i16,
    pub gzip: u32,
}

impl Entry {
    /// A record; None for one with no name (a list's end).
    fn of(r: &Streamdata) -> Option<Entry> {
        Some(Entry { name: r.name?.to_string(), ofs: r.ofs, size: r.size, kind: r.kind, flag: r.flag, gzip: r.gzip })
    }
}

/// `streamTbl[num]` (or `streamTblE[num]`) read whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Def {
    pub num: usize,
    pub english: bool,
    /// Record 0.
    pub header: Entry,
    /// The files, in table order.
    pub entries: Vec<Entry>,
}

/// The files a stream reads, split as `ccStreamLoadPlay` splits them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Files {
    /// `searchPreLoad` (0x00199690): every `type` -2 record, then every
    /// `flag` 16 record, each read whole into memory
    /// (`ccLoadStreamOnMem`) before the scenes start.
    pub preload: Vec<Entry>,
    /// `RequestStrPlay` (0x00198fc0): the `ccsTbl`, the files the decoder
    /// plays one after another.
    pub scenes: Vec<Entry>,
}

impl Def {
    pub fn read(volume: Volume, num: usize, english: bool) -> Result<Def> {
        if num >= count(volume) {
            return Err(Error::NotFound(format!("stream {num}")));
        }
        let t = stream::of(volume);
        // A disc with only `streamTblE` and the `E` archives (Outbreak on,
        // where the voice picks a PCM track instead) reads them whatever
        // the voice.
        let english = english || t.lists().is_empty();
        let lists = if english { t.lists_e() } else { t.lists() };
        let list = lists.get(num).copied().flatten().unwrap_or_default();
        let mut rows = list.iter().map(Entry::of);
        let header = rows.next().flatten().ok_or_else(|| Error::Format(format!("stream {num} has no header")))?;
        Ok(Def { num, english, header, entries: rows.map_while(|e| e).collect() })
    }

    /// The archive on the disc, when this disc has it.
    pub fn archive(&self) -> Option<&'static str> {
        let (jp, en) = ARCHIVES.get(usize::try_from(self.header.kind).ok()?)?;
        Some(if self.english { en } else { jp })
    }

    /// Where a file starts in the archive: `ReadThread` (0x00198330) adds
    /// the header's and the record's offsets, each rounded up to a sector.
    pub fn offset(&self, e: &Entry) -> u64 {
        let sectors = |o: i32| (i64::from(o) + 2047) >> 11;
        ((sectors(self.header.ofs) + sectors(e.ofs)) as u64) * 2048
    }

    /// `searchPreLoad` and `RequestStrPlay`'s split. `param` picks among
    /// alternatives (a `flag` 2 record and the `flag` 4 records after it);
    /// `ccRequestLoadStream` passes 0.
    pub fn files(&self, param: usize) -> Files {
        let mut f = Files::default();
        f.preload.extend(self.entries.iter().filter(|e| e.kind == TYPE_ON_MEMORY).cloned());
        f.preload.extend(self.entries.iter().filter(|e| e.flag == FLAG_PRELOAD).cloned());
        let mut i = 0;
        while i < self.entries.len() {
            let e = &self.entries[i];
            if e.flag == FLAG_ALT {
                if let Some(pick) = self.entries.get(i + param) {
                    f.scenes.push(pick.clone());
                }
                i += param + 1;
                while self.entries.get(i).is_some_and(|e| e.flag == FLAG_ALT_REST) {
                    i += 1;
                }
                continue;
            }
            if e.flag != FLAG_PRELOAD {
                f.scenes.push(e.clone());
            }
            i += 1;
        }
        f
    }

    /// Whether `ccSndStreamCtrl` looks at the stream before and after it
    /// plays: header `size` bit 0 / bit 1. Which streams it then changes
    /// the music for is each volume's own switch
    /// (`piney_audio::stream::stream_ctrl`).
    pub fn music(&self) -> (bool, bool) {
        (self.header.size & 1 != 0, self.header.size & 2 != 0)
    }
}

/// The Chaos Gate's own stream (`str7000`: `str7200`, then `str7300`
/// looping), which `ccSetupGameCtrl` starts with `setupMode` set: 107 on
/// Infection, 113 from Mutation on (where 107 is empty; each volume's
/// `ccSndStreamCtrl` names it too).
pub fn gate_stream(volume: Volume) -> usize {
    if volume == Volume::Inf { 107 } else { 113 }
}
/// `str7000Out` (INF main 0x0030f1b0: `{field, arrival}` shorts, ended by
/// a negative field)'s arrival for `field`; 0 when it is not listed.
pub fn gate_hack_arrival(volume: Volume, field: i32) -> u32 {
    let rows = stream::of(volume).gate_out();
    rows.iter().find(|r| i32::from(r.area_num) == field).map_or(0, |r| r.tbl_num as u32)
}

/// What `ccRequestLoadStreamGateHack(107, town, field)` (0x00199f00) plays
/// through `ccStreamLoadPlay::RequestStrPlayGH(town, field)` (0x0019a0e0), the
/// gate hack's arrival as `ccSetupGameCtrl` runs it (`setupMode` 1): the files
/// read whole from `str7000TblPre`, the town's row (`TownC` in the crisis),
/// the arrival's row and stream 107's records, then the scenes of the town
/// row, stream 107 and the arrival row. The tables and their `E` versions are
/// in docs/engine/stream.md ("The gate hack's movie").
pub fn gate_hack_files(volume: Volume, def: &Def, town: i32, field: i32, crisis: bool) -> Result<Files> {
    let t = stream::of(volume);
    let english = def.english || t.gate_pre().is_empty();
    let pick = |e: &'static [[Streamdata; 2]], j: &'static [[Streamdata; 2]]| if english { e } else { j };
    let towns =
        if crisis { pick(t.gate_town_crisis_e(), t.gate_town_crisis()) } else { pick(t.gate_town_e(), t.gate_town()) };
    let afters = pick(t.gate_after_e(), t.gate_after());
    let row = |rows: &[[Streamdata; 2]], n: u32, k: usize| -> Result<Entry> {
        rows.get(n as usize)
            .and_then(|r| Entry::of(&r[k]))
            .ok_or_else(|| Error::Format(format!("str7000 table row {n}: empty")))
    };
    let town = town.max(0) as u32;
    let after = gate_hack_arrival(volume, field);
    let pre = if english { t.gate_pre_e() } else { t.gate_pre() };
    let pre = pre.first().and_then(Entry::of).ok_or_else(|| Error::Format("str7000TblPre: empty".into()))?;
    let mut f = Files::default();
    f.preload.push(pre);
    f.preload.push(row(towns, town, 0)?);
    f.preload.push(row(afters, after, 0)?);
    f.preload.extend(def.entries.iter().filter(|e| e.kind == TYPE_ON_MEMORY).cloned());
    f.preload.extend(def.entries.iter().filter(|e| e.flag == FLAG_PRELOAD).cloned());
    f.scenes.push(row(towns, town, 1)?);
    f.scenes.extend(def.entries.iter().cloned());
    f.scenes.push(row(afters, after, 1)?);
    Ok(f)
}

/// `WaitEnd` (0x00197f40): whether this frame's `push` skips the scene
/// playing, whose record has `flag`. `title_after_desktop`: `game.status` 2
/// (the title) with `DESKTOP_FLG` 1, where cancel always skips.
pub fn skips(flag: i16, push: u32, cancel: u32, title_after_desktop: bool) -> bool {
    if title_after_desktop {
        push & cancel != 0
    } else if flag & FLAG_NO_SKIP != 0 {
        false
    } else if flag & FLAG_SKIP_START != 0 {
        push & 0x800 != 0
    } else {
        push & cancel != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str, kind: i16, flag: i16) -> Entry {
        Entry { name: name.into(), ofs: 0, size: 0, kind, flag, gzip: 0 }
    }

    #[test]
    fn preloads_come_out_of_the_scene_list() {
        let d = Def {
            num: 107,
            english: false,
            header: e("str7000", 0, 0),
            entries: vec![e("str7200", 1, 64), e("str7300", -2, 193)],
        };
        let f = d.files(0);
        assert_eq!(f.preload, vec![e("str7300", -2, 193)]);
        assert_eq!(f.scenes, vec![e("str7200", 1, 64), e("str7300", -2, 193)]);
        let d = Def {
            num: 2,
            english: false,
            header: e("str0001", 1, 0),
            entries: vec![e("str0001e", 0, 16), e("str0001", -1, 64)],
        };
        let f = d.files(0);
        assert_eq!(f.preload, vec![e("str0001e", 0, 16)]);
        assert_eq!(f.scenes, vec![e("str0001", -1, 64)]);
    }

    #[test]
    fn alternatives_pick_one() {
        let d = Def {
            num: 0,
            english: false,
            header: e("x", 0, 0),
            entries: vec![e("a", 1, 2), e("b", 1, 4), e("c", 1, 4), e("d", -1, 0)],
        };
        assert_eq!(d.files(0).scenes, vec![e("a", 1, 2), e("d", -1, 0)]);
        assert_eq!(d.files(1).scenes, vec![e("b", 1, 4), e("d", -1, 0)]);
    }

    #[test]
    fn offsets_round_to_sectors() {
        let mut d = Def { num: 2, english: false, header: e("str0001", 1, 0), entries: vec![] };
        d.header.ofs = 188_416;
        let mut f = e("str0001", -1, 64);
        f.ofs = 845_824;
        assert_eq!(d.offset(&f), 188_416 + 845_824);
        d.header.ofs = 1;
        f.ofs = 1;
        assert_eq!(d.offset(&f), 4096);
    }
}
