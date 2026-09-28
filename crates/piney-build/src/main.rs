//! piney-build: one game from the .hack discs.
//!
//!     piney-build [--out DIR] [--no-verify] [--export-symbols DIR] DISC...
//!
//! Each DISC is a disc image (`.iso`), or a folder to look in for them. The
//! discs are told apart by their contents, so names and order do not
//! matter. Every file of every disc is cut into chunks where its content
//! says (FastCDC), and each chunk is kept once however many files and discs
//! hold it: the four discs share their movies' logos, their sound modules,
//! the voices, and most of `DATA.BIN` and the stream archives.
//!
//! The build goes to DIR (by default `game` in the port's folder,
//! `~/.local/share/piney` on Linux; `$PINEY_HOME` moves it), where
//! `piney-game` finds it by itself. A disc of an earlier build in DIR that
//! is not given again is kept, so discs can be added one at a time. Every
//! file is read back from the build and checked against its hash unless
//! `--no-verify`. The format is laid out in `piney_data::pack`.
//!
//! `--export-symbols DIR` also writes each disc's executable's names into
//! DIR, one `<executable>.syms` a disc (`section va size type name how`,
//! tab-separated): Infection's from its own executable, the later volumes'
//! the names the build carries to theirs from Infection's (which needs
//! Infection's disc in the build), for anyone taking the games apart. With
//! no DISC given it reads the build's own discs and leaves the build as it
//! is: `piney-build --export-symbols ./dist/symbols`.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use piney_data::iso::{Entry, Iso};
use piney_data::pack::{self, Chunk, Manifest, PackEntry};
use piney_data::volume::Volume;

/// FastCDC's sizes: a cut is looked for from 16 KiB, around 64 KiB, and
/// forced at 256 KiB.
const MIN_CHUNK: u32 = 16 * 1024;
const AVG_CHUNK: u32 = 64 * 1024;
const MAX_CHUNK: u32 = 256 * 1024;
/// Bytes read from a disc at a time.
const READ_BLOCK: usize = 8 << 20;

fn main() {
    let mut out: Option<PathBuf> = None;
    let mut verify = true;
    let mut export: Option<PathBuf> = None;
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" | "-o" => out = args.next().map(PathBuf::from),
            "--no-verify" => verify = false,
            "--export-symbols" => export = args.next().map(PathBuf::from),
            "-h" | "--help" => {
                println!("piney-build [--out DIR] [--no-verify] [--export-symbols DIR] DISC...");
                println!();
                println!("DISC: a .hack disc image (.iso), or a folder of them; one to four discs,");
                println!("in any order. DIR: where the build goes, by default {}.", show(pack::default_build()));
                println!("A disc of an earlier build in DIR that is not given again is kept.");
                println!("--export-symbols DIR: each executable's names as DIR/<executable>.syms too");
                println!("(the later volumes' carried from Infection's, which needs its disc);");
                println!("with no DISC, from the build's own discs, leaving the build as it is.");
                return;
            }
            s if s.starts_with('-') => {
                eprintln!("unknown option {s} (--help lists them)");
                std::process::exit(2);
            }
            _ => inputs.push(PathBuf::from(a)),
        }
    }
    let Some(out) = out.or_else(pack::default_build) else {
        eprintln!("no folder to build into: give --out DIR");
        std::process::exit(2);
    };
    if let Err(e) = run(&inputs, &out, verify, export.as_deref()) {
        eprintln!("piney-build: {e}");
        std::process::exit(1);
    }
}

fn show(p: Option<PathBuf>) -> String {
    p.map_or_else(|| "(none: set --out)".into(), |p| p.display().to_string())
}

/// `--export-symbols`: each disc's executable's names into `dir`.
fn export_symbols(sources: &[Source], dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let has_inf = sources.iter().any(|s| s.volume == Volume::Inf);
    for s in sources {
        if s.volume != Volume::Inf && !has_inf {
            println!("{}: no names (the carried names need Infection's disc in the build)", s.volume.title());
            continue;
        }
        let v = piney_gen::volume::VOLUMES[s.volume as usize];
        let text = piney_gen::source::symbols_text(v)?;
        let path = dir.join(format!("{}.syms", s.volume.executable()));
        std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
        let n = text.lines().filter(|l| !l.starts_with('#')).count();
        println!("{} names of {} -> {}", n, s.volume.title(), path.display());
    }
    Ok(())
}

/// A disc to put in the build: an image, or a disc of the earlier build.
struct Source {
    path: PathBuf,
    volume: Volume,
    from_build: bool,
}

fn run(inputs: &[PathBuf], out: &Path, verify: bool, export: Option<&Path>) -> Result<(), String> {
    let mut sources = find_discs(inputs)?;
    // The earlier build's discs that were not given again.
    for (v, path) in pack::volumes_in(out) {
        if !sources.iter().any(|s| s.volume == v) {
            println!("keeping {} from the build in {}", v.title(), out.display());
            sources.push(Source { path, volume: v, from_build: true });
        }
    }
    if sources.is_empty() {
        return Err("no .hack disc given (--help)".into());
    }
    sources.sort_by_key(|s| s.volume);
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;

    // Written beside the old build and put in its place at the end, so an
    // old build being read from stays whole until then.
    let pak_part = out.join(format!("{}.part", pack::PAK_NAME));
    let mut pak = Pak::create(&pak_part)?;
    let started = Instant::now();
    // The generator reads each volume's files from its own disc, and
    // Infection's for the later volumes' carried names and the tables'
    // layouts.
    for s in &sources {
        piney_build::use_disc(s.volume, &s.path);
    }
    if let Some(dir) = export {
        export_symbols(&sources, dir)?;
        // With no disc given, only the names: the build stays as it is.
        if inputs.is_empty() {
            return Ok(());
        }
    }
    let mut pending = Vec::new();
    for s in &sources {
        pending.push(add_disc(&mut pak, s)?);
    }
    pak.finish()?;
    let manifests: Vec<Manifest> = pending.into_iter().map(|p| p.manifest(&pak)).collect();

    // The new build in place: chunks.pak, then each .disc.
    let pak_path = out.join(pack::PAK_NAME);
    std::fs::rename(&pak_part, &pak_path).map_err(|e| format!("{}: {e}", pak_path.display()))?;
    for m in &manifests {
        let path = out.join(pack::disc_name(m.volume));
        let part = path.with_extension("disc.part");
        std::fs::write(&part, m.encode()).map_err(|e| format!("{}: {e}", part.display()))?;
        std::fs::rename(&part, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let total: u64 = manifests.iter().flat_map(|m| &m.entries).map(|e| u64::from(e.entry.size)).sum();
    let summary = format!(
        "{} disc(s): {}\n{} of files: {} of distinct chunks, {} stored ({:.1}%), {} chunks, built in {:.0} s\n",
        manifests.len(),
        manifests.iter().map(|m| format!("{} ({})", m.volume.title(), m.label)).collect::<Vec<_>>().join(", "),
        mib(total),
        mib(pak.raw),
        mib(pak.bytes),
        100.0 * pak.bytes as f64 / total.max(1) as f64,
        pak.chunks.len(),
        started.elapsed().as_secs_f64(),
    );
    print!("{summary}");
    let _ = std::fs::write(out.join("build.txt"), &summary);

    if verify {
        for m in &manifests {
            verify_disc(&out.join(pack::disc_name(m.volume)), m)?;
        }
    }
    println!("the build is in {}; piney-game finds it there", out.display());
    Ok(())
}

/// The discs among `inputs`: images, and folders' images, each told
/// apart by its `DATA/GCMN.PRG`.
fn find_discs(inputs: &[PathBuf]) -> Result<Vec<Source>, String> {
    let mut paths = Vec::new();
    for p in inputs {
        if p.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(p)
                .map_err(|e| format!("{}: {e}", p.display()))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|f| f.extension().is_some_and(|x| x.eq_ignore_ascii_case("iso")))
                .collect();
            found.sort();
            if found.is_empty() {
                return Err(format!("{}: no .iso in it", p.display()));
            }
            paths.extend(found);
        } else {
            paths.push(p.clone());
        }
    }
    let mut out: Vec<Source> = Vec::new();
    for path in paths {
        let mut disc = Iso::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let volume =
            disc.volume().map_err(|e| format!("{}: not a .hack disc this port knows ({e})", path.display()))?;
        if let Some(prev) = out.iter().find(|s| s.volume == volume) {
            return Err(format!("{} and {} are both {}", prev.path.display(), path.display(), volume.title()));
        }
        println!("{}: {}", path.display(), volume.title());
        out.push(Source { path, volume, from_build: false });
    }
    Ok(out)
}

/// The chunk file being written, and every chunk in it by its hash.
///
/// A new chunk waits in a batch; a full batch is compressed on up to
/// [`THREADS`] threads and written. Files name their chunks by index until
/// the end, when every chunk has its place.
struct Pak {
    file: BufWriter<File>,
    path: PathBuf,
    /// Where the next chunk goes.
    at: u64,
    /// Chunk bytes written (without the header), and their files' bytes.
    bytes: u64,
    raw: u64,
    /// Every chunk by index, its place once written.
    chunks: Vec<Chunk>,
    index: HashMap<[u8; 32], usize>,
    /// New chunks not written yet: (index, bytes).
    batch: Vec<(usize, Vec<u8>)>,
    batch_bytes: usize,
}

/// zstd's level: about the size of keeping the archives' members inflated
/// and deduplicated, which the port could not read without new code.
const ZSTD_LEVEL: i32 = 15;
/// Threads compressing a batch.
const THREADS: usize = 8;
/// Bytes of new chunks gathered before they are compressed.
const BATCH: usize = 32 << 20;

impl Pak {
    fn create(path: &Path) -> Result<Pak, String> {
        let f = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut file = BufWriter::with_capacity(4 << 20, f);
        let mut head = Vec::from(&pack::PAK_MAGIC[..]);
        head.extend_from_slice(&pack::VERSION.to_le_bytes());
        head.extend_from_slice(&0u32.to_le_bytes());
        file.write_all(&head).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Pak {
            file,
            path: path.to_path_buf(),
            at: pack::PAK_HEADER,
            bytes: 0,
            raw: 0,
            chunks: Vec::new(),
            index: HashMap::new(),
            batch: Vec::new(),
            batch_bytes: 0,
        })
    }

    /// The chunk's index; new chunks are kept, an equal one's index reused.
    fn put(&mut self, data: Vec<u8>) -> Result<usize, String> {
        let h = *blake3::hash(&data).as_bytes();
        if let Some(&i) = self.index.get(&h) {
            return Ok(i);
        }
        let i = self.chunks.len();
        self.chunks.push(Chunk { offset: 0, stored: 0, len: data.len() as u32, zstd: false });
        self.index.insert(h, i);
        self.batch_bytes += data.len();
        self.batch.push((i, data));
        if self.batch_bytes >= BATCH {
            self.flush_batch()?;
        }
        Ok(i)
    }

    /// The batch compressed (kept as it is where zstd does not shrink it)
    /// and written in order.
    fn flush_batch(&mut self) -> Result<(), String> {
        let batch = std::mem::take(&mut self.batch);
        self.batch_bytes = 0;
        let per = batch.len().div_ceil(THREADS).max(1);
        let packed: Vec<Vec<Option<Vec<u8>>>> = std::thread::scope(|s| {
            let jobs: Vec<_> = batch
                .chunks(per)
                .map(|part| {
                    s.spawn(move || {
                        part.iter()
                            .map(|(_, d)| zstd::bulk::compress(d, ZSTD_LEVEL).ok().filter(|z| z.len() < d.len()))
                            .collect()
                    })
                })
                .collect();
            jobs.into_iter().map(|j| j.join().unwrap()).collect()
        });
        for ((i, data), z) in batch.iter().zip(packed.into_iter().flatten()) {
            let (bytes, zstd) = match &z {
                Some(z) => (&z[..], true),
                None => (&data[..], false),
            };
            self.file.write_all(bytes).map_err(|e| format!("{}: {e}", self.path.display()))?;
            self.chunks[*i] = Chunk { offset: self.at, stored: bytes.len() as u32, len: data.len() as u32, zstd };
            self.at += bytes.len() as u64;
            self.bytes += bytes.len() as u64;
            self.raw += data.len() as u64;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        self.flush_batch()?;
        self.file.flush().map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.file.get_ref().sync_all().map_err(|e| format!("{}: {e}", self.path.display()))
    }
}

/// A disc's file read in blocks, for the chunker.
struct FileReader<'a> {
    disc: &'a mut Iso,
    entry: piney_data::iso::Entry,
    at: u64,
    buf: Vec<u8>,
    pos: usize,
}

impl Read for FileReader<'_> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.pos == self.buf.len() {
            self.buf = self.disc.read_at(&self.entry, self.at, READ_BLOCK).map_err(std::io::Error::other)?;
            self.at += self.buf.len() as u64;
            self.pos = 0;
        }
        let n = out.len().min(self.buf.len() - self.pos);
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// Every file and directory of the disc into the build.
fn add_disc(pak: &mut Pak, s: &Source) -> Result<Pending, String> {
    let err = |e: &dyn std::fmt::Display| format!("{}: {e}", s.path.display());
    let mut disc = Iso::open(&s.path).map_err(|e| err(&e))?;
    // The images' volume identifiers are blank: the boot file's name
    // (SYSTEM.CNF's BOOT2, `SLUS_202.67`) names the disc instead.
    let mut label = disc.label().map_err(|e| err(&e))?;
    if label.is_empty() {
        label = disc.read_path("SYSTEM.CNF").ok().and_then(|cnf| boot_name(&cnf)).unwrap_or_default();
    }
    let mut entries = disc.list().map_err(|e| err(&e))?;
    // An earlier build's port data is made again below, from the disc.
    entries.retain(|e| !is_port(&e.path));
    entries.sort_by_key(|e| (e.lba, e.path.clone()));
    let from = if s.from_build { "the build" } else { "the image" };
    println!("{} ({label}), {} files from {from}", s.volume.title(), entries.iter().filter(|e| !e.dir).count());
    let before = pak.chunks.len();
    let mut total = 0u64;
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.dir {
            out.push((entry, [0; 32], Vec::new()));
            continue;
        }
        let t = Instant::now();
        let fresh = pak.chunks.len();
        let mut hasher = blake3::Hasher::new();
        let mut ids = Vec::new();
        let mut got = 0u64;
        let reader = FileReader { disc: &mut disc, entry: entry.clone(), at: 0, buf: Vec::new(), pos: 0 };
        for c in fastcdc::v2020::StreamCDC::new(reader, MIN_CHUNK, AVG_CHUNK, MAX_CHUNK) {
            let c = c.map_err(|e| err(&format!("{}: {e}", entry.path)))?;
            hasher.update(&c.data);
            got += c.data.len() as u64;
            ids.push(pak.put(c.data)?);
        }
        if got != u64::from(entry.size) {
            return Err(err(&format!("{}: read {got} bytes of {}", entry.path, entry.size)));
        }
        total += got;
        if entry.size >= 32 << 20 {
            let new: u64 = pak.chunks[fresh..].iter().map(|c| u64::from(c.len)).sum();
            println!(
                "  {:<24} {:>10}  {:>10} new  {:.1} s",
                entry.path,
                mib(u64::from(entry.size)),
                mib(new),
                t.elapsed().as_secs_f64()
            );
        }
        out.push((entry, *hasher.finalize().as_bytes(), ids));
    }
    // The port's own files (`PINEY/`), chunked as the disc's are. They sit
    // at no place on the disc: their LBA is out of its range.
    let t = Instant::now();
    let files = piney_build::port_files(&mut disc).map_err(|e| err(&e))?;
    out.push((Entry { path: pack::PORT_DIR.to_string(), lba: u32::MAX, size: 0, dir: true }, [0; 32], Vec::new()));
    let mut port = 0u64;
    for (path, bytes) in files {
        let mut ids = Vec::new();
        for c in fastcdc::v2020::FastCDC::new(&bytes, MIN_CHUNK, AVG_CHUNK, MAX_CHUNK) {
            ids.push(pak.put(bytes[c.offset..c.offset + c.length].to_vec())?);
        }
        port += bytes.len() as u64;
        let hash = *blake3::hash(&bytes).as_bytes();
        out.push((Entry { path, lba: u32::MAX, size: bytes.len() as u32, dir: false }, hash, ids));
    }
    println!("  the port's data ({}): {} in {:.1} s", pack::PORT_DIR, mib(port), t.elapsed().as_secs_f64());
    let new: u64 = pak.chunks[before..].iter().map(|c| u64::from(c.len)).sum();
    println!("  {} of files, {} of it new to the build", mib(total + port), mib(new));
    Ok(Pending { volume: s.volume, label, files: out })
}

/// Whether a path is one of the port's own files (`PINEY/` and under).
fn is_port(path: &str) -> bool {
    let top = path.split(['/', '\\']).next().unwrap_or("");
    top.eq_ignore_ascii_case(pack::PORT_DIR)
}

/// A disc's files as chunk indices, until every chunk has its place.
struct Pending {
    volume: Volume,
    label: String,
    files: Vec<(piney_data::iso::Entry, [u8; 32], Vec<usize>)>,
}

impl Pending {
    fn manifest(self, pak: &Pak) -> Manifest {
        let entries = self
            .files
            .into_iter()
            .map(|(entry, hash, ids)| PackEntry::new(entry, hash, ids.iter().map(|&i| pak.chunks[i]).collect()))
            .collect();
        Manifest::new(self.volume, self.label, entries)
    }
}

/// Each file read back through the build and its hash compared.
fn verify_disc(path: &Path, m: &Manifest) -> Result<(), String> {
    let mut disc = Iso::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    for e in m.entries.iter().filter(|e| !e.entry.dir) {
        let mut hasher = blake3::Hasher::new();
        let mut at = 0u64;
        while at < u64::from(e.entry.size) {
            let b = disc.read_at(&e.entry, at, READ_BLOCK).map_err(|x| format!("{}: {x}", e.entry.path))?;
            if b.is_empty() {
                break;
            }
            at += b.len() as u64;
            hasher.update(&b);
        }
        if *hasher.finalize().as_bytes() != e.hash {
            return Err(format!("{}: {} reads back wrong from the build", m.volume.title(), e.entry.path));
        }
    }
    println!("{}: every file reads back as it was", m.volume.title());
    Ok(())
}

/// `BOOT2 = cdrom0:\\SLUS_202.67;1` -> `SLUS_202.67`.
fn boot_name(cnf: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(cnf);
    let line = text.lines().find(|l| l.trim_start().starts_with("BOOT2"))?;
    let name = line.rsplit(['\\', ':']).next()?.split(';').next()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn mib(n: u64) -> String {
    format!("{:.1} MiB", n as f64 / (1 << 20) as f64)
}
