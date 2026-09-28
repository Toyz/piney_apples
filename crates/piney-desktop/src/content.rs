//! The desktop's content: wallpapers, background music, movies, news
//! headlines, and the mails and their replies (`docs/engine/text.md`), from
//! the volume's generated tables (`piney_data::tables::desktop`,
//! `piney_data::tables::mail`), their text as the Shift-JIS the desktop
//! draws.

use piney_data::tables::sjis::encode;
use piney_data::tables::{desktop, mail as mails};
use piney_data::volume::Volume;

fn text(s: Option<&str>) -> Vec<u8> {
    s.map(encode).unwrap_or_default()
}

/// A two-line comment's lines (the second empty when there is one line).
fn two(c: Option<&[&str]>) -> (Vec<u8>, Vec<u8>) {
    let line = |k: usize| c.and_then(|l| l.get(k)).map_or(Vec::new(), |t| encode(t));
    (line(0), line(1))
}

/// `WallData` (0x18 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WallData {
    pub title: Vec<u8>,
    /// +0x04 `No`.
    pub no: i32,
    /// +0x0c: the CCS file (`DATA.BIN` stem), e.g. `xddwal00`.
    pub name: String,
    /// +0x10: its animation, e.g. `ANM_xddwal00_a`.
    pub anm_name: String,
    /// +0x14: the artist, and after its NUL the caption
    /// (`ccKanjiStrSeparate(comment, 1)`).
    pub comment: Vec<u8>,
    pub comment2: Vec<u8>,
}

/// The volume's wallpapers (`WallTbl`, up to its terminator).
pub fn walls(v: Volume) -> Vec<WallData> {
    desktop::of(v)
        .walls
        .iter()
        .map(|w| {
            let (comment, comment2) = two(w.comment);
            WallData {
                title: text(w.title),
                no: w.no,
                name: w.name.unwrap_or_default().to_string(),
                anm_name: w.anm_name.unwrap_or_default().to_string(),
                comment,
                comment2,
            }
        })
        .collect()
}

/// `WallTbl[i]` (the last one past the end).
pub fn wall(v: Volume, i: usize) -> WallData {
    let mut all = walls(v);
    let i = i.min(all.len() - 1);
    all.swap_remove(i)
}

/// `WaveData` (0x18 bytes): one piece of background music.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveData {
    pub title: Vec<u8>,
    pub category: i32,
    pub field_type: i32,
    pub bg_num: i32,
    /// Two lines, NUL-separated, as `WallData`'s.
    pub comment: Vec<u8>,
    pub comment2: Vec<u8>,
    pub no: i32,
}

/// The volume's jukebox (`Wave`, up to its terminator).
pub fn waves(v: Volume) -> Vec<WaveData> {
    desktop::of(v)
        .waves
        .iter()
        .map(|w| {
            let (comment, comment2) = two(w.comment);
            WaveData {
                title: text(w.title),
                category: w.category,
                field_type: w.field_type,
                bg_num: w.bg_num,
                comment,
                comment2,
                no: w.no,
            }
        })
        .collect()
}

/// `StrData` (0xc bytes): one movie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrData {
    pub name: Vec<u8>,
    /// Two lines, NUL-separated.
    pub comment: Vec<u8>,
    pub comment2: Vec<u8>,
    /// The stream `SimplePlayStream` plays.
    pub str_num: i32,
}

/// The volume's movies (`Stream`, up to its terminator).
pub fn streams(v: Volume) -> Vec<StrData> {
    desktop::of(v)
        .streams
        .iter()
        .map(|m| {
            let (comment, comment2) = two(m.comment);
            StrData { name: text(m.name), comment, comment2, str_num: m.str_num }
        })
        .collect()
}

/// A movie's volume: 1 + the number of `AddStrList`'s bounds it is at or
/// above.
pub fn stream_volume(i: usize) -> i32 {
    1 + desktop::STREAM_VOLUMES.iter().take_while(|&&b| i as i32 >= b).count() as i32
}

/// `HtmlData` (0x1c bytes): one news headline and its page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlData {
    pub no: i32,
    pub flg: i32,
    pub title: Vec<u8>,
    /// The CCS file holding the page image, e.g. `xddn_010`.
    pub ccs_name: String,
    /// The page's texture in it.
    pub chunk: String,
    /// Page height in pixels (`hight`).
    pub height: i32,
}

/// The volume's news (`HtmlTbl`, up to its terminator).
pub fn news(v: Volume) -> Vec<HtmlData> {
    desktop::of(v)
        .news
        .iter()
        .map(|h| HtmlData {
            no: h.no,
            flg: h.flg,
            title: text(h.title),
            ccs_name: h.ccs_name.unwrap_or_default().to_string(),
            chunk: h.chunk.unwrap_or_default().to_string(),
            height: h.hight,
        })
        .collect()
}

/// `ccReMailData` (0x14 bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReMail {
    pub mail_flg: i32,
    pub title: Vec<u8>,
    pub from: Vec<u8>,
    /// The body's lines, NUL-separated in the table.
    pub lines: Vec<Vec<u8>>,
}

/// `ccMailData` (0x48 bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mail {
    pub no: i32,
    pub re_flg: i32,
    pub mail_flg: i32,
    pub title: Vec<u8>,
    pub from: Vec<u8>,
    pub from_no: i32,
    pub lines: Vec<Vec<u8>>,
    /// The two replies `__sinit_mailtbl.cpp` copies in from `ReMail`.
    pub one_res: Option<ReMail>,
    pub two_res: Option<ReMail>,
}

fn remail_of(r: &mails::ReMailData) -> ReMail {
    ReMail {
        mail_flg: r.mail_flg,
        title: text(r.title),
        from: text(r.from),
        lines: r.sentence.iter().map(|l| encode(l)).collect(),
    }
}

/// How many mails the volume's `MailTbl` holds: 326 on Infection, 375 from
/// Mutation on.
pub fn mail_count(v: Volume) -> usize {
    mails::of(v).mails.len()
}

/// `ReMail[k]` or `ReMailp[k]` (the last one past the end).
pub fn remail(v: Volume, k: usize, parody: bool) -> ReMail {
    let t = mails::of(v);
    let rows = if parody { t.remails_parody } else { t.remails };
    remail_of(&rows[k.min(rows.len() - 1)])
}

/// `MailTbl[i]` or `MailTblp[i]`, with the replies its static initialiser
/// links in.
pub fn mail(v: Volume, i: usize, parody: bool) -> Mail {
    let t = mails::of(v);
    let (rows, replies) = if parody { (t.mails_parody, t.replies_parody) } else { (t.mails, t.replies) };
    let i = i.min(rows.len() - 1);
    let m = &rows[i];
    let [one, two] = replies[i];
    Mail {
        no: m.no,
        re_flg: m.re_flg,
        mail_flg: m.mail_flg,
        title: text(m.title),
        from: text(m.from),
        from_no: m.from_no,
        lines: m.sentence.iter().map(|l| encode(l)).collect(),
        one_res: one.map(|k| remail(v, k as usize, parody)),
        two_res: two.map(|k| remail(v, k as usize, parody)),
    }
}
