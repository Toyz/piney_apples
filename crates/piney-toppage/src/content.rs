//! The bulletin board's tables: the volume's `bbsThreadTbl` and its parody
//! twin `bbsThreadTblP` (`piney_data::tables::toppage`), as the Shift-JIS
//! the board draws (`docs/engine/text.md`, `docs/engine/toppage.md`).
//!
//! `ccBBSThreadMsgList::CheckThreadData` (0x00402220) picks the parody
//! table when `saveData.parodyFlag` (+0x842b) is set. A post's `maxLines`
//! is the number of its lines.

use piney_data::tables::sjis::encode;
use piney_data::tables::toppage::{self, BBSMsgList, BBSThreadList};
use piney_data::volume::Volume;

/// The loops over the threads run to 63 (`_CreateReadTable` 0x004021e8,
/// `CheckNewMessage` 0x00402308).
pub const THREAD_COUNT: usize = 63;

/// One post (`ccBBSMsgList`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgList {
    pub dateindex: i32,
    pub title: Vec<u8>,
    pub transname: Vec<u8>,
    pub max_lines: i32,
    /// `GetLineText(i)` for `i < maxLines`: the body's lines.
    pub lines: Vec<Vec<u8>>,
}

/// One thread (`ccBBSThreadList`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadList {
    pub title: Vec<u8>,
    pub msgs: Vec<MsgList>,
}

fn text(s: Option<&str>) -> Vec<u8> {
    s.map_or_else(Vec::new, encode)
}

fn msg(m: &BBSMsgList) -> MsgList {
    MsgList {
        dateindex: m.dateindex,
        title: text(m.title),
        transname: text(m.transname),
        max_lines: m.message.len() as i32,
        lines: m.message.iter().map(|l| encode(l)).collect(),
    }
}

fn thread(t: &BBSThreadList) -> ThreadList {
    ThreadList { title: text(t.title), msgs: t.msg_list.unwrap_or_default().iter().map(msg).collect() }
}

/// The volume's 63 threads (or Parody Mode's) with their posts.
pub fn threads(volume: Volume, parody: bool) -> Vec<ThreadList> {
    let t = toppage::of(volume);
    let tbl = if parody { t.threads_parody } else { t.threads };
    tbl.iter().map(thread).collect()
}
