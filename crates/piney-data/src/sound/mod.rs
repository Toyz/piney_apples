//! The game's audio data: the `DATA/SNDDATA.BIN` sound banks, PS-ADPCM,
//! `VOICE/BGM.BIN`'s streamed music, and the executable's sound tables.
//!
//! Follows `docs/formats/snddata.md`, `docs/formats/voice.md` and
//! `docs/engine/sound.md`, and `tools/sound.py`, `tools/scei.py` and
//! `tools/adpcm.py`, which established them; `tests/sound.rs` checks this
//! against those tools bit for bit.
//!
//! - [`adpcm`]: PS-ADPCM frames and whole samples.
//! - [`hd`]: the bank header - VAGs, samples, sample sets, programs.
//! - [`sq`]: the sequence file and its MIDI blocks.
//! - [`SndData`] / [`Bank`]: the 79 banks, found through the executable's
//!   loading tables, since the file has no directory.
//! - [`Tables`] / [`INF`] / [`tables_of`]: those loading tables, the
//!   per-sequence port volumes, the sound-effect table `seData`, the
//!   desktop jukebox `Wave` and the `BGM.BIN` track table, read from each
//!   volume's executable by `piney-gen` (`placement::sound`) into the build
//!   (`plans/build-data.md`).
//! - [`voice`]: the voice files' lines (their tables are
//!   [`crate::tables::voice`]).

pub mod adpcm;
pub mod hd;
pub mod sq;
pub mod voice;

pub use hd::{Hd, Program, Sample, SampleSet, Split, Vag};
pub use sq::Sq;

use crate::iso::Iso;
use crate::store::{Load, Reader};
use crate::volume::Volume;
use crate::{Result, format_err};

/// The volume's sound tables (the build's `PINEY/TABLES/sound.bin`, read
/// once a run).
pub fn tables_of(v: Volume) -> &'static Tables {
    static READ: [std::sync::OnceLock<&'static Tables>; 4] = [const { std::sync::OnceLock::new() }; 4];
    READ[v as usize].get_or_init(|| crate::store::group(v, "sound"))
}

/// Infection's sound tables, read on first use.
pub static INF: std::sync::LazyLock<&'static Tables> = std::sync::LazyLock::new(|| tables_of(Volume::Inf));

// Read from the build's file, field by field (`piney_gen::placement::sound`
// writes them).

impl Load for SqLoad {
    fn load(r: &mut Reader) -> Self {
        SqLoad { ofs: Load::load(r), hd_size: Load::load(r), sq_size: Load::load(r), bd_size: Load::load(r) }
    }
}

impl Load for SqTbl {
    fn load(r: &mut Reader) -> Self {
        SqTbl { midi_port: Load::load(r), hd_port: Load::load(r), vol: Load::load(r) }
    }
}

impl Load for Context {
    fn load(r: &mut Reader) -> Self {
        Context { load: Load::load(r), vol: Load::load(r), play: Load::load(r) }
    }
}

impl Load for SeTbl {
    fn load(r: &mut Reader) -> Self {
        SeTbl {
            prog: Load::load(r),
            port: Load::load(r),
            ch: Load::load(r),
            note: Load::load(r),
            velocity: Load::load(r),
            dummy: Load::load(r),
            decay: Load::load(r),
        }
    }
}

impl Load for WaveData {
    fn load(r: &mut Reader) -> Self {
        WaveData { category: Load::load(r), field_type: Load::load(r), bg_num: Load::load(r), no: Load::load(r) }
    }
}

impl Load for BgmTrack {
    fn load(r: &mut Reader) -> Self {
        BgmTrack {
            ofs: Load::load(r),
            size: Load::load(r),
            mode: Load::load(r),
            vol_r: Load::load(r),
            vol_l: Load::load(r),
        }
    }
}

impl Load for Tables {
    fn load(r: &mut Reader) -> Self {
        Tables {
            commse: Load::load(r),
            field: Load::load(r),
            dungeon: Load::load(r),
            town: Load::load(r),
            title: Load::load(r),
            desktop: Load::load(r),
            toppage: Load::load(r),
            event: Load::load(r),
            stream: Load::load(r),
            se: Load::load(r),
            wave: Load::load(r),
            bgm: Load::load(r),
        }
    }
}

pub const SECTOR: usize = 2048;
/// The SPU2's fixed output and streaming rate (`docs/formats/voice.md`).
pub const PCM_RATE: u32 = 48_000;

/// An `SQ_LOAD` row (24 bytes): where a bank is in `SNDDATA.BIN`. A row of
/// -1 is unused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SqLoad {
    pub ofs: i32,
    pub hd_size: i32,
    pub sq_size: [i32; 3],
    pub bd_size: i32,
}

impl SqLoad {
    /// Rows of -1 are unused, and typeE and typeI end in a zero row.
    pub fn is_used(&self) -> bool {
        self.ofs >= 0 && self.hd_size > 0
    }
}

/// An `SQTBL` (12 bytes): which sequencer plays sequence `i` of a bank, the
/// synthesizer port it drives, and its volume (0x100 is full).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SqTbl {
    pub midi_port: i32,
    pub hd_port: i32,
    pub vol: u16,
}

/// A loading table with its volume rows and play types, row for row.
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub load: &'static [SqLoad],
    pub vol: &'static [[SqTbl; 3]],
    /// `ccSound.playtype` per row where the context has one.
    pub play: &'static [i8],
}

/// A `SETBL` (8 bytes): what `ccSeOn(n)` sends for sound effect `n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeTbl {
    pub prog: i8,
    pub port: i8,
    pub ch: i8,
    pub note: i8,
    pub velocity: i8,
    pub dummy: i8,
    pub decay: i16,
}

/// A `WaveData` row of the desktop jukebox (`Wave`, `INF desktop.prg`),
/// without its title and comment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaveData {
    /// Which loading table: see [`Tables::wave`].
    pub category: i32,
    /// The field type, for category 3.
    pub field_type: i32,
    /// The row of the loading table.
    pub bg_num: i32,
    /// The row's own number; `saveData.dtBgm` holds it.
    pub no: i32,
}

/// A `VOICE/BGM.BIN` track: `bgmWavTbl` (`INF gcmn.prg`) with `bgmParam`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BgmTrack {
    /// Bytes into `BGM.BIN`.
    pub ofs: i32,
    pub size: i32,
    /// `mode & 0xf` non-zero loops.
    pub mode: i32,
    pub vol_r: u16,
    pub vol_l: u16,
}

impl BgmTrack {
    pub fn loops(&self) -> bool {
        self.mode & 0xf != 0
    }
}

/// The sound tables of one executable.
#[derive(Debug)]
pub struct Tables {
    /// `commseTbl`: offset, `.hd` size and `.bd` size of the common
    /// sound-effect bank.
    pub commse: [u32; 3],
    /// By field type (`sqDataField`, `sqVolTblField`, `playTypeTbl`).
    pub field: [Context; 12],
    pub dungeon: Context,
    pub town: Context,
    pub title: Context,
    pub desktop: Context,
    pub toppage: Context,
    pub event: Context,
    pub stream: Context,
    /// `seData`, by sound-effect number.
    pub se: &'static [SeTbl],
    /// The desktop jukebox, by `NO`.
    pub wave: &'static [WaveData],
    pub bgm: &'static [BgmTrack],
}

impl Tables {
    /// The loading row and port volumes a jukebox row selects, as
    /// `ccSndChangeData` (`INF SLUS_202.67:0x001834e0`) finds them through
    /// its two jump tables (0x0034e130 for the bank, 0x0034e110 for the
    /// volumes): category 1 desktop, 2 town, 3 field (by `field_type`),
    /// 4 dungeon, 5 event, 6 stream, 7 title; 0 loads nothing.
    pub fn wave(&self, w: &WaveData) -> Option<(SqLoad, [SqTbl; 3])> {
        let ctx = match w.category {
            1 => &self.desktop,
            2 => &self.town,
            3 => self.field.get(usize::try_from(w.field_type).ok()?)?,
            4 => &self.dungeon,
            5 => &self.event,
            6 => &self.stream,
            7 => &self.title,
            _ => return None,
        };
        let row = usize::try_from(w.bg_num).ok()?;
        let load = *ctx.load.get(row)?;
        load.is_used().then_some((load, *ctx.vol.get(row)?))
    }

    /// The sequence of a jukebox row's bank that `ccSndChangeData` plays:
    /// 1 for rows 47, 27 and 7, else 0.
    pub fn wave_sequence(no: i32) -> usize {
        if matches!(no, 47 | 27 | 7) { 1 } else { 0 }
    }

    /// Every row that names a bank, the common bank first: (ofs, hd size,
    /// sq sizes, table bd size, name of the row).
    fn rows(&self) -> Vec<(SqLoad, String)> {
        let mut out = vec![(
            SqLoad {
                ofs: self.commse[0] as i32,
                hd_size: self.commse[1] as i32,
                sq_size: [0; 3],
                bd_size: self.commse[2] as i32,
            },
            "commseTbl".to_string(),
        )];
        let mut add = |name: &str, ctx: &Context| {
            for (i, r) in ctx.load.iter().enumerate() {
                if r.is_used() {
                    out.push((*r, format!("{name}[{i}]")));
                }
            }
        };
        for (t, ctx) in self.field.iter().enumerate() {
            add(&format!("field{t}"), ctx);
        }
        for (name, ctx) in [
            ("dungeon", &self.dungeon),
            ("town", &self.town),
            ("title", &self.title),
            ("desktop", &self.desktop),
            ("toppage", &self.toppage),
            ("event", &self.event),
            ("stream", &self.stream),
        ] {
            add(name, ctx);
        }
        out
    }
}

/// Where one bank lies in `SNDDATA.BIN`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BankInfo {
    /// In file order.
    pub index: usize,
    pub offset: usize,
    pub hd_size: usize,
    pub sq_sizes: [usize; 3],
    /// From the bank's own `Head` chunk.
    pub bd_size: usize,
    /// What the loading table says; one row (typeI[2]) carries another
    /// bank's.
    pub table_bd_size: usize,
    /// The table rows that load it, the first one naming it.
    pub refs: Vec<String>,
}

impl BankInfo {
    /// The `.sq` files: (offset, size), the empty ones left out.
    pub fn sq_offsets(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut p = self.offset + self.hd_size;
        for &n in &self.sq_sizes {
            if n > 0 {
                out.push((p, n));
            }
            p += n;
        }
        out
    }

    /// Where sequencer `i` reads its `.sq`, as `ccSndSQLoad` and
    /// `ccSndChangeData` (`INF SLUS_202.67:0x001821d0`, `0x001834e0`) hand
    /// it out: sequence 0 always starts right after the `.hd`, sequence
    /// `i > 0` after the first `i` sizes, and exists only when its own size
    /// is non-zero. So a bank whose first size is 0 plays its second file
    /// as sequence 0 and again as sequence 1.
    pub fn sequence_offset(&self, i: usize) -> Option<usize> {
        if i >= 3 || (i > 0 && self.sq_sizes[i] == 0) || self.sq_sizes.iter().all(|&n| n == 0) {
            return None;
        }
        Some(self.offset + self.hd_size + self.sq_sizes[..i].iter().sum::<usize>())
    }

    /// `ccSQDataLoadCD` (`SNDBASE.IRX:0x1e44`): the `.bd` starts at the
    /// sector after the `.hd` and the `.sq`s.
    pub fn bd_offset(&self) -> usize {
        (self.offset + self.hd_size + self.sq_sizes.iter().sum::<usize>()).div_ceil(SECTOR) * SECTOR
    }

    pub fn end(&self) -> usize {
        self.bd_offset() + self.bd_size
    }
}

/// One bank, parsed, with its sample data.
#[derive(Clone, Debug)]
pub struct Bank {
    pub info: BankInfo,
    pub hd: Hd,
    /// The `.sq` files in table order (1 to 3, or none for the SE bank).
    pub sq: Vec<Sq>,
    /// The `.bd`: PS-ADPCM, addressed by `Vag::offset`.
    pub bd: Vec<u8>,
}

impl Bank {
    /// The `.sq` sequencer `i` plays (see [`BankInfo::sequence_offset`]).
    pub fn sequence(&self, i: usize) -> Option<&Sq> {
        let at = self.info.sequence_offset(i)?;
        let k = self.info.sq_offsets().iter().position(|&(o, _)| o == at)?;
        self.sq.get(k)
    }

    /// A VAG's ADPCM, from its `Vagi` offset to the next VAG's.
    pub fn vag_data(&self, vag: usize) -> Option<&[u8]> {
        let (a, b) = (*self.hd.vag_ranges().get(vag)?)?;
        self.bd.get(a as usize..b as usize)
    }
}

/// `SNDDATA.BIN` with the banks the tables place in it.
pub struct SndData {
    data: Vec<u8>,
    banks: Vec<BankInfo>,
    volume: Volume,
    tables: &'static Tables,
}

impl SndData {
    /// From the file's bytes, placed by the volume's executable's tables.
    pub fn new(data: Vec<u8>, volume: Volume) -> Result<SndData> {
        let tables = tables_of(volume);
        let mut banks: Vec<BankInfo> = Vec::new();
        for (row, name) in tables.rows() {
            let offset = row.ofs as usize;
            let sq_sizes = row.sq_size.map(|n| n.max(0) as usize);
            if let Some(b) = banks.iter_mut().find(|b| b.offset == offset) {
                if (b.hd_size, b.sq_sizes) != (row.hd_size as usize, sq_sizes) {
                    return format_err(format!("{name} disagrees with {} about bank 0x{offset:x}", b.refs[0]));
                }
                b.refs.push(name);
                continue;
            }
            banks.push(BankInfo {
                index: 0,
                offset,
                hd_size: row.hd_size as usize,
                sq_sizes,
                bd_size: row.bd_size as usize,
                table_bd_size: row.bd_size as usize,
                refs: vec![name],
            });
        }
        banks.sort_by_key(|b| b.offset);
        for (i, b) in banks.iter_mut().enumerate() {
            b.index = i;
            let hd = data
                .get(b.offset..b.offset + b.hd_size)
                .ok_or_else(|| crate::Error::Format(format!("bank at 0x{:x} runs past the file", b.offset)))?;
            b.bd_size = Hd::parse(hd)?.bd_size as usize;
        }
        Ok(SndData { data, banks, volume, tables })
    }

    /// `DATA/SNDDATA.BIN` from the disc, placed by its volume's tables.
    pub fn read(iso: &mut Iso) -> Result<SndData> {
        let volume = crate::volume::of_disc(iso)?;
        SndData::new(iso.read_path("DATA/SNDDATA.BIN")?, volume)
    }

    /// The disc's volume.
    pub fn volume(&self) -> Volume {
        self.volume
    }

    /// The tables that placed the banks: the disc's volume's.
    pub fn tables(&self) -> &'static Tables {
        self.tables
    }

    pub fn banks(&self) -> &[BankInfo] {
        &self.banks
    }

    pub fn bank_at(&self, offset: usize) -> Option<&BankInfo> {
        self.banks.iter().find(|b| b.offset == offset)
    }

    /// The whole file.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn load(&self, info: &BankInfo) -> Result<Bank> {
        let slice = |a: usize, n: usize| {
            self.data.get(a..a + n).ok_or_else(|| crate::Error::Format(format!("0x{a:x}+{n} runs past SNDDATA.BIN")))
        };
        let hd = Hd::parse(slice(info.offset, info.hd_size)?)?;
        let sq = info.sq_offsets().into_iter().map(|(o, n)| Sq::parse(slice(o, n)?.to_vec())).collect::<Result<_>>()?;
        let bd = slice(info.bd_offset(), info.bd_size)?.to_vec();
        Ok(Bank { info: info.clone(), hd, sq, bd })
    }

    /// The bank a loading row names.
    pub fn load_row(&self, row: &SqLoad) -> Result<Bank> {
        let info =
            self.bank_at(row.ofs as usize).ok_or_else(|| crate::Error::NotFound(format!("bank at 0x{:x}", row.ofs)))?;
        self.load(info)
    }

    /// The common sound-effect bank (`commseTbl`).
    pub fn commse(&self, tables: &Tables) -> Result<Bank> {
        let info = self
            .bank_at(tables.commse[0] as usize)
            .ok_or_else(|| crate::Error::NotFound("the common SE bank".into()))?;
        self.load(info)
    }
}

/// A `BGM.BIN` track's samples: interleaved stereo (L, R) signed 16-bit at
/// 48 kHz, as `wavPlay` (`INF SLUS_202.67:0x0017e6e0`) streams it.
pub fn bgm_pcm(bgm_bin: &[u8], track: &BgmTrack) -> Result<Vec<i16>> {
    let (a, n) = (track.ofs as usize, track.size as usize);
    let raw = bgm_bin
        .get(a..a + n)
        .ok_or_else(|| crate::Error::Format(format!("BGM.BIN track 0x{a:x}+{n} runs past the file")))?;
    Ok(raw.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c)).collect())
}
