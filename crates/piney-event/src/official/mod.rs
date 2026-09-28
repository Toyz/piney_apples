//! The adapter for the game's own scripts.
//!
//! The scripts are `short` arrays in the boot executable, and the messages
//! are tables of `{emode, name, text}` records beside them. The port does not
//! read the executable at play time: [`events`] parses a disc's scripts and
//! messages from the text form the build wrote into its port data
//! ([`EVENTS_FILE`], [`events_text`]; `plans/build-data.md`). [`load_iso`]
//! reads them from a disc's executable, for the build and the checks; the
//! tables are found through the game's code ([`locate()`]), so the same
//! reader works on every volume.
//!
//! The encoding (`docs/engine/events.md`): no header, no jumps, every code
//! has a fixed number of `short` operands.
//!
//! ```text
//! open conditions ... 0
//! block, up to 62:
//!   (-2 TAG operands)*      precondition settings
//!   conditions ... 0
//!   instructions ... 0
//! -1
//! ```
//!
//! [`decode`] turns that into [`Script`] and [`encode`] turns a script back
//! into exactly the same shorts; the crate's tests check that for every
//! script on the disc.

pub mod elf;
pub mod locate;

use std::collections::BTreeSet;
use std::fmt;

use crate::ir::{Block, Cond, CondKind, Event, GameText, MAX_BLOCKS, Message, OpKind, Script, TagKind};
pub use elf::Executable;
pub use locate::{Dialect, Layout, locate};

#[derive(Debug)]
pub enum Error {
    Data(piney_data::Error),
    Format(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Data(e) => write!(f, "{e}"),
            Error::Format(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<piney_data::Error> for Error {
    fn from(e: piney_data::Error) -> Self {
        Error::Data(e)
    }
}

// ---------------------------------------------------------------------------
// The code tables.

/// Execute's codes, as Infection numbers them (`tools/evscript.py ops`).
/// Codes 99, 168 and 169 depend on the volume ([`op_kind`]); 134's table
/// entry is the loop head, so it is not an instruction.
const OPS: &[(u16, OpKind)] = {
    use OpKind::*;
    &[
        (1, EndEvent),
        (2, SetBlock),
        (3, ClearBlock),
        (4, Repeatable),
        (5, Wait),
        (6, Message),
        (7, ClearAnswer),
        (8, Info),
        (9, InfoNow),
        (10, Stream),
        (11, Entry),
        (12, EntryMc),
        (13, Remove),
        (14, AddOperate),
        (15, DelOperate),
        (16, AddTarget),
        (17, DelTarget),
        (18, MenuBan),
        (19, MenuClear),
        (20, AddAreaCode),
        (21, DelAreaCode),
        (22, CamLookPos),
        (23, CamLookChar),
        (24, CamFollowChar),
        (25, CamLookMarker),
        (26, CamLookPosHalf),
        (27, CamLookCharHalf),
        (28, CamLookMarkerHalf),
        (29, CamPanPos),
        (30, CamPanChar),
        (31, CamPanFollow),
        (32, CamPanMarker),
        (33, CamOrbit),
        (34, CamOrbitMove),
        (35, CamOrbitTurn),
        (36, CamMode4),
        (37, TeachCamera1),
        (38, TeachCamera2),
        (39, TeachCamera3),
        (40, CamzSet),
        (41, CamzMove),
        (42, CamzSpeed),
        (43, CamzPoint),
        (44, CamzPath),
        (45, Radiator),
        (46, BossSmoke),
        (47, DeleteGimmick19),
        (48, Camera),
        (49, CameraEnd),
        (50, CameraEndReset),
        (51, NpcAct),
        (52, NpcWalkPos),
        (53, NpcWalkDir),
        (54, NpcWalkMarker),
        (55, NpcWalkChar),
        (56, NpcPutMarker),
        (57, NpcPut),
        (58, NpcTurn),
        (59, NpcFace),
        (60, PcAct),
        (61, PcWalkPos),
        (62, PcWalkDir),
        (63, PcWalkMarker),
        (64, PcWalkChar),
        (65, PcMode),
        (66, PcCommand),
        (67, PcPutMarker),
        (68, PartyPutMarker),
        (69, PartyPut),
        (70, PcPut),
        (71, PcTurn),
        (72, PcFace),
        (73, PcUseSkill),
        (74, EnemyPut),
        (75, AffectOn),
        (76, AffectOff),
        (77, PartyAdd),
        (78, PartyRemove),
        (79, MemberAdd),
        (80, MemberAddMsg),
        (81, CallOn),
        (82, CallOnLater),
        (83, CallOff),
        (84, CallLock),
        (85, CallUnlock),
        (86, ExpOn),
        (87, ExpOff),
        (88, SaveParty),
        (89, GateAdd),
        (90, GateAddMsg),
        (91, GateMark),
        (92, GateUnmark),
        (93, ItemAdd),
        (94, ItemAddMenu),
        (95, ItemDel),
        (96, GoldAdd),
        (97, Noise2),
        (98, Noise1),
        (100, Scene),
        (101, Mode),
        (102, WallpaperAdd),
        (103, BgmAdd),
        (104, BbsPost),
        (105, BbsRemove),
        (106, BbsPost7),
        (107, Mail),
        (108, MailVol),
        (109, MailMember),
        (110, MailRemove),
        (111, NewsAdd),
        (112, NewsRemove),
        (113, FrameRate),
        (114, Overlay),
        (115, Crisis),
        (116, SetPoint),
        (117, SetPos),
        (118, Area),
        (119, ClearOperate),
        (120, ClearAreaCode),
        (121, MapOn),
        (122, ShowMap),
        (123, RemoveTrap),
        (124, VirusCore),
        (125, TownMove),
        (126, DataDrain),
        (127, PrevRoom),
        (128, AreaBan),
        (129, AreaUnban),
        (130, Room),
        (131, RoomPoint),
        (132, Hold),
        (133, HoldEnd),
        (135, PcTint),
        (136, PcTintOff),
        (137, PirosColour),
        (138, StatusSet),
        (139, StatusAdd),
        (140, StatusSub),
        (141, Protect),
        (142, TalkNum),
        (143, RegistNpc16),
        (144, MarkerPos),
        (145, Fade),
        (146, FadeMore),
        (147, StaffRoll),
        (148, ClearCount),
        (149, Friendship),
        (150, Menu),
        (151, ConditionFxOn),
        (152, ConditionFxOff),
        (153, PlayerSkill),
        (154, TargetForbid),
        (155, LastTown),
        (156, NameEntry),
        (157, GateHackAnim),
        (158, OpenDoor),
        (159, CloseDoor),
        (160, Sound),
        (161, TransOff),
        (162, TransOn),
        (163, BattleReady),
        (164, PcRunPos),
        (165, AddOperateSf),
        (166, DelOperateSf),
        (167, DesktopItem),
    ]
};

/// CheckOpen's conditions: code `i + 1` is `CONDS[i]`.
const CONDS: &[CondKind] = {
    use CondKind::*;
    &[
        EventDone,
        BlockDone,
        Phase,
        GameStatus,
        InPoint,
        Scene,
        InTown,
        InField,
        InDungeon,
        Status,
        StatusRange,
        GateWords,
        TalkedTo,
        Operate,
        NearMarker,
        Answer,
        BbsRead,
        MailGot,
        Mail4,
        Mail5,
        Mail6,
        NewsRead,
        InParty,
        NotInParty,
        InPartyOf2,
        PartyOther,
        Callable,
        Present,
        Absent,
        NoActive,
        NoEntries,
        NoMenu,
        HasItem,
        Friendship,
        Pad,
        EnemyPp,
        Member,
        MemberSaved,
        Volume,
        InRoom,
    ]
};

/// SetCurrentOpen's tags: tag `i + 2` is `TAGS[i]` (0 and 1 set nothing).
const TAGS: &[TagKind] = {
    use TagKind::*;
    &[BlockDone, Phase, GameStatus, InPoint, Scene, InTown, InField, InDungeon, Status, StatusRange]
};

/// The instruction a code stands for in `d`.
pub fn op_kind(d: Dialect, code: i16) -> Option<OpKind> {
    match (code, d) {
        (99, Dialect::Infection) => Some(OpKind::Noise3),
        (99, _) => Some(OpKind::Noise),
        (168, Dialect::Infection) => None,
        (168, _) => Some(OpKind::GruntyMail),
        (169, Dialect::Outbreak | Dialect::Quarantine) => Some(OpKind::EndingKanji),
        _ => OPS.iter().find(|(c, _)| *c as i16 == code).map(|&(_, k)| k),
    }
}

/// The code for an instruction in `d`, if that volume has it.
pub fn op_code(d: Dialect, k: OpKind) -> Option<i16> {
    match k {
        OpKind::Noise3 => (d == Dialect::Infection).then_some(99),
        OpKind::Noise => (d != Dialect::Infection).then_some(99),
        OpKind::GruntyMail => (d != Dialect::Infection).then_some(168),
        OpKind::EndingKanji => matches!(d, Dialect::Outbreak | Dialect::Quarantine).then_some(169),
        _ => OPS.iter().find(|(_, x)| *x == k).map(|&(c, _)| c as i16),
    }
}

pub fn cond_kind(code: i16) -> Option<CondKind> {
    usize::try_from(code).ok().and_then(|c| c.checked_sub(1)).and_then(|i| CONDS.get(i)).copied()
}

pub fn cond_code(k: CondKind) -> i16 {
    CONDS.iter().position(|&x| x == k).unwrap() as i16 + 1
}

pub fn tag_kind(code: i16) -> Option<TagKind> {
    usize::try_from(code).ok().and_then(|c| c.checked_sub(2)).and_then(|i| TAGS.get(i)).copied()
}

pub fn tag_code(k: TagKind) -> i16 {
    TAGS.iter().position(|&x| x == k).unwrap() as i16 + 2
}

// ---------------------------------------------------------------------------
// Decoding and encoding.

const END: i16 = -1;
const CURRENT: i16 = -2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    /// Index of the short where decoding stopped.
    pub at: usize,
    pub msg: String,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at short {} (+0x{:x}): {}", self.at, 2 * self.at, self.msg)
    }
}

impl std::error::Error for DecodeError {}

struct Reader<'a> {
    s: &'a [i16],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[i16], DecodeError> {
        let at = self.pos;
        let got = self.s.get(at..at + n).ok_or(DecodeError { at, msg: "runs past the end".into() })?;
        self.pos += n;
        Ok(got)
    }
    fn one(&mut self) -> Result<i16, DecodeError> {
        Ok(self.take(1)?[0])
    }
    fn peek(&self) -> Result<i16, DecodeError> {
        self.s.get(self.pos).copied().ok_or(DecodeError { at: self.pos, msg: "runs past the end".into() })
    }
    fn err<T>(&self, at: usize, msg: String) -> Result<T, DecodeError> {
        Err(DecodeError { at, msg })
    }

    fn conds(&mut self) -> Result<Vec<Cond>, DecodeError> {
        let mut out = Vec::new();
        loop {
            let at = self.pos;
            let code = self.one()?;
            if code == 0 {
                return Ok(out);
            }
            let Some(k) = cond_kind(code) else { return self.err(at, format!("condition {code} has no case")) };
            let args = self.take(k.arity())?;
            out.push(k.build(args).unwrap());
        }
    }
}

/// A script from the start of its array. Returns the script and the number
/// of shorts it takes, up to and including the -1.
pub fn decode(d: Dialect, shorts: &[i16]) -> Result<(Script, usize), DecodeError> {
    let mut r = Reader { s: shorts, pos: 0 };
    let mut script = Script { open: r.conds()?, blocks: Vec::new() };
    for _ in 0..MAX_BLOCKS {
        if r.peek()? == END {
            return Ok((script, r.pos + 1));
        }
        let mut b = Block::default();
        while r.peek()? == CURRENT {
            let at = r.pos;
            let tag = r.take(2)?[1];
            let Some(k) = tag_kind(tag) else { return r.err(at, format!("setting {tag} is not decoded")) };
            let args = r.take(k.arity())?;
            b.tags.push(k.build(args).unwrap());
        }
        b.conds = r.conds()?;
        loop {
            let at = r.pos;
            let code = r.one()?;
            if code == 0 {
                break;
            }
            let Some(k) = op_kind(d, code) else { return r.err(at, format!("instruction {code} has no case")) };
            let args = r.take(k.arity())?;
            b.ops.push(k.build(args).unwrap());
        }
        script.blocks.push(b);
    }
    if r.peek()? == END {
        return Ok((script, r.pos + 1));
    }
    r.err(r.pos, format!("no -1 after {MAX_BLOCKS} blocks"))
}

/// The game's shorts for a script.
pub fn encode(d: Dialect, s: &Script) -> Result<Vec<i16>, String> {
    if s.blocks.len() > MAX_BLOCKS {
        return Err(format!("{} blocks; the game walks at most {MAX_BLOCKS}", s.blocks.len()));
    }
    let mut out = Vec::new();
    let conds = |out: &mut Vec<i16>, cs: &[Cond]| {
        for c in cs {
            out.push(cond_code(c.kind()));
            out.extend(c.args());
        }
        out.push(0);
    };
    conds(&mut out, &s.open);
    for b in &s.blocks {
        for t in &b.tags {
            out.push(CURRENT);
            out.push(tag_code(t.kind()));
            out.extend(t.args());
        }
        conds(&mut out, &b.conds);
        for o in &b.ops {
            let code = op_code(d, o.kind()).ok_or_else(|| format!("{} does not exist in {d:?}", o.name()))?;
            out.push(code);
            out.extend(o.args());
        }
        out.push(0);
    }
    out.push(END);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Reading the tables.

/// One script as it sits in the executable.
#[derive(Clone, Debug)]
pub struct RawScript {
    pub event: u16,
    pub va: u32,
    /// The array up to the next script or table, less trailing zero padding.
    pub shorts: Vec<i16>,
    pub label: GameText,
}

/// The ten `ccEvTbl` group arrays: (group, array address).
fn groups(exe: &Executable, table: u32) -> Vec<(u16, u32)> {
    (0..10u16).filter_map(|g| exe.u32(table + 4 * g as u32).filter(|&a| a != 0).map(|a| (g, a))).collect()
}

/// Every script in `eventTbl`, by event number.
pub fn raw_scripts(exe: &Executable, layout: &Layout) -> Vec<RawScript> {
    let mut entries = Vec::new();
    for (g, base) in groups(exe, layout.event_tbl) {
        for i in 0..50u16 {
            let (Some(script), Some(label)) = (exe.u32(base + 8 * i as u32), exe.u32(base + 8 * i as u32 + 4)) else {
                continue;
            };
            if script != 0 {
                entries.push((50 * g + i, script, label));
            }
        }
    }
    let mut starts: BTreeSet<u32> = entries.iter().map(|e| e.1).collect();
    starts.insert(layout.event_tbl);
    starts.extend(groups(exe, layout.event_tbl).iter().map(|g| g.1));
    let mut out = Vec::new();
    for (event, va, label) in entries {
        let end = starts.range(va + 1..).next().copied().unwrap_or(va + 0x10000);
        let Some(bytes) = exe.read(va, (end - va) as usize) else { continue };
        let mut shorts: Vec<i16> = bytes.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c)).collect();
        while shorts.last() == Some(&0) {
            shorts.pop();
        }
        let label = if label != 0 { exe.cstr(label, 512).unwrap_or_default() } else { Vec::new() };
        out.push(RawScript { event, va, shorts, label: GameText(label) });
    }
    out.sort_by_key(|r| r.event);
    out
}

/// Message arrays are measured the way `tools/evscript.py` does: an array
/// runs to the next array or table of either message table, less trailing
/// all-zero records.
pub struct MessageTables<'a> {
    exe: &'a Executable,
    normal: u32,
    parody: u32,
    starts: Vec<u32>,
}

const RECORD: u32 = 12;

impl<'a> MessageTables<'a> {
    pub fn new(exe: &'a Executable, layout: &Layout) -> Self {
        let mut starts = BTreeSet::new();
        for t in [layout.msg_tbl, layout.msg_tbl_parody] {
            starts.insert(t);
            for (_, grp) in groups(exe, t) {
                starts.insert(grp);
                for i in 0..50 {
                    if let Some(a) = exe.u32(grp + 4 * i).filter(|&a| a != 0) {
                        starts.insert(a);
                    }
                }
            }
        }
        MessageTables {
            exe,
            normal: layout.msg_tbl,
            parody: layout.msg_tbl_parody,
            starts: starts.into_iter().collect(),
        }
    }

    fn count(&self, va: u32) -> u32 {
        let i = self.starts.partition_point(|&s| s <= va);
        let end = self.starts.get(i).copied().unwrap_or(va + RECORD * 256);
        let mut n = (end - va) / RECORD;
        while n > 0 && self.exe.read(va + RECORD * (n - 1), RECORD as usize).is_none_or(|r| r.iter().all(|&b| b == 0)) {
            n -= 1;
        }
        n
    }

    /// `evMsgTbl[event / 50][event % 50]`, or the Parody Mode table's.
    pub fn messages(&self, event: u16, parody: bool) -> Vec<Message> {
        let exe = self.exe;
        let table = if parody { self.parody } else { self.normal };
        let Some(grp) = exe.u32(table + 4 * (event / 50) as u32).filter(|&g| g != 0) else { return Vec::new() };
        let Some(base) = exe.u32(grp + 4 * (event % 50) as u32).filter(|&b| b != 0) else { return Vec::new() };
        (0..self.count(base))
            .filter_map(|k| {
                let r = exe.read(base + RECORD * k, RECORD as usize)?;
                let mode = i32::from_le_bytes(r[0..4].try_into().unwrap());
                let name = u32::from_le_bytes(r[4..8].try_into().unwrap());
                let text = u32::from_le_bytes(r[8..12].try_into().unwrap());
                let name = (name != 0).then(|| GameText(exe.cstr(name, 128).unwrap_or_default()));
                let mut lines: Vec<GameText> =
                    if text != 0 { (0..3).map(|j| GameText(split_line(exe, text, j))).collect() } else { Vec::new() };
                while lines.last().is_some_and(|l| l.0.is_empty()) {
                    lines.pop();
                }
                Some(Message { mode, name, lines })
            })
            .collect()
    }
}

/// `ccKanjiStrSeparate(text, k)`: skip `k` NUL-ended lines, where a byte
/// below 0x20 or above 0x7f takes the byte after it along.
fn split_line(exe: &Executable, text: u32, k: u32) -> Vec<u8> {
    let mut va = text;
    for _ in 0..k {
        loop {
            let Some(c) = exe.read(va, 1).map(|b| b[0]) else { return Vec::new() };
            if c == 0 {
                va += 1;
                break;
            }
            va += if (0x20..0x80).contains(&c) { 1 } else { 2 };
        }
    }
    exe.cstr(va, 256).unwrap_or_default()
}

/// Every event of an executable: decoded scripts, labels, both message
/// tables.
pub struct Official {
    pub layout: Layout,
    pub events: Vec<Event>,
}

pub fn load(exe: &Executable) -> Result<Official, Error> {
    let layout = locate(exe)?;
    let msgs = MessageTables::new(exe, &layout);
    let mut events = Vec::new();
    for raw in raw_scripts(exe, &layout) {
        let (script, _) = decode(layout.dialect, &raw.shorts)
            .map_err(|e| Error::Format(format!("event {} at 0x{:08x}: {e}", raw.event, raw.va)))?;
        events.push(Event {
            number: raw.event,
            label: raw.label,
            script,
            messages: msgs.messages(raw.event, false),
            parody: msgs.messages(raw.event, true),
        });
    }
    Ok(Official { layout, events })
}

/// The boot executable named by `SYSTEM.CNF` (`BOOT2 = cdrom0:\SLUS_202.67;1`).
pub fn boot_executable(iso: &mut piney_data::iso::Iso) -> Result<Executable, Error> {
    let cnf = iso.read_path("SYSTEM.CNF")?;
    let cnf = String::from_utf8_lossy(&cnf);
    let path = cnf
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "BOOT2").then(|| v.trim().to_string())
        })
        .ok_or_else(|| Error::Format("SYSTEM.CNF has no BOOT2".into()))?;
    let name = path.rsplit(['\\', ':']).next().unwrap_or(&path);
    let name = name.split(';').next().unwrap_or(name);
    Executable::parse(iso.read_path(name)?)
}

/// Every event on a disc image, read from its executable (the generator's
/// and the checks' path; the port uses [`events`]).
pub fn load_iso(iso: &mut piney_data::iso::Iso) -> Result<Official, Error> {
    load(&boot_executable(iso)?)
}

/// The port's file of a disc's scripts and messages, in the text form.
pub const EVENTS_FILE: &str = "PINEY/EVENTS.EVS";

/// The text form of a volume's events, as the build writes
/// [`EVENTS_FILE`]: a comment line, then every event.
pub fn events_text(v: piney_data::volume::Volume, events: &[Event]) -> String {
    let mut s = format!(
        "# {} {}: the event scripts and messages, read from the disc by piney-build.\n",
        v.title(),
        v.executable()
    );
    for e in events {
        s.push_str(&crate::text::print_event(e));
    }
    s
}

/// Each volume's events once read ([`events`]).
static PARSED: [std::sync::OnceLock<Result<Vec<Event>, String>>; 4] =
    [std::sync::OnceLock::new(), std::sync::OnceLock::new(), std::sync::OnceLock::new(), std::sync::OnceLock::new()];

/// Every event of the disc's volume, from its port data ([`EVENTS_FILE`]);
/// a disc without it (a tool's or a check's image) from its executable.
/// Read once a run; later calls copy it.
pub fn events(iso: &mut piney_data::iso::Iso) -> Result<Vec<Event>, Error> {
    let v = iso.volume()?;
    PARSED[v as usize]
        .get_or_init(|| match iso.read_path(EVENTS_FILE) {
            Ok(bytes) => String::from_utf8(bytes)
                .map_err(|e| e.to_string())
                .and_then(|t| crate::text::parse_events(&t).map_err(|e| e.to_string()))
                .map_err(|e| format!("{v}'s {EVENTS_FILE}: {e}")),
            Err(_) => load_iso(iso).map(|o| o.events).map_err(|e| format!("{v}'s scripts: {e}")),
        })
        .clone()
        .map_err(Error::Format)
}

/// A volume's events as [`events`] read them earlier this run.
pub fn events_of(v: piney_data::volume::Volume) -> Result<Vec<Event>, Error> {
    PARSED[v as usize]
        .get()
        .cloned()
        .unwrap_or_else(|| Err(format!("{v}'s scripts are not read yet")))
        .map_err(Error::Format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_tables_cover_every_kind() {
        for &k in OpKind::ALL {
            let codes: Vec<_> = [Dialect::Infection, Dialect::Mutation, Dialect::Outbreak, Dialect::Quarantine]
                .map(|d| op_code(d, k))
                .into();
            assert!(codes.iter().any(|c| c.is_some()), "{k:?} has no code");
            for (d, c) in
                [Dialect::Infection, Dialect::Mutation, Dialect::Outbreak, Dialect::Quarantine].iter().zip(codes)
            {
                if let Some(c) = c {
                    assert_eq!(op_kind(*d, c), Some(k));
                }
            }
        }
        assert_eq!(OPS.len(), 165);
        assert_eq!(cond_code(CondKind::InRoom), 40);
        assert_eq!(tag_code(TagKind::StatusRange), 11);
        for &k in CondKind::ALL {
            assert_eq!(cond_kind(cond_code(k)), Some(k));
        }
        for &k in TagKind::ALL {
            assert_eq!(tag_kind(tag_code(k)), Some(k));
        }
    }

    #[test]
    fn decode_encode_small() {
        let s = [1, 0, 4, 2, 0, -2, 3, 0, 0, 0, 113, 2, 6, 1, 0, -1];
        let (script, n) = decode(Dialect::Infection, &s).unwrap();
        assert_eq!(n, s.len());
        assert_eq!(encode(Dialect::Infection, &script).unwrap(), s);
        assert!(decode(Dialect::Infection, &[0, 134, 0, -1]).is_err());
    }
}
