//! The pad log: every game frame's pad since power-on and the console's
//! commands between them, kept in memory, with the memory card as it was
//! at power-on. `--pad-log FILE` writes it from the start; the console's
//! `pad_log` writes what is kept so far and goes on from there. Either way
//! `--replay FILE` plays the run again from a copy of `FILE.card`.

use std::collections::VecDeque;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use piney_input::{Buttons, Raw};

/// Steps between flushes of the file being written: a second of frames.
const FLUSH_EVERY: usize = 60;

/// One step of a run: a frame's pad, or a console command between frames.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Pad(Raw),
    Console(String),
}

/// What a log line is when read back ([`read`]): a frame's pad carries
/// `| bytes LX LY RX RY buttons HEX`, a command `console LINE`.
fn parse(line: &str) -> Option<Result<Step, ()>> {
    if let Some(c) = line.strip_prefix("console ") {
        return Some(Ok(Step::Console(c.to_string())));
    }
    let at = line.find("| bytes ")?;
    let w: Vec<&str> = line[at + 8..].split_whitespace().collect();
    if w.len() < 6 || w[4] != "buttons" {
        return Some(Err(()));
    }
    let byte = |i: usize| w[i].parse::<u8>().map_err(|_| ());
    let pad = (|| {
        Ok(Raw {
            buttons: Buttons(u32::from_str_radix(w[5], 16).map_err(|_| ())?),
            lx: byte(0)?,
            ly: byte(1)?,
            rx: byte(2)?,
            ry: byte(3)?,
            ..Raw::default()
        })
    })();
    Some(pad.map(Step::Pad))
}

/// A pad log's steps back, in order; other lines (the gamepads' names)
/// are skipped.
pub fn read(path: &str) -> Result<VecDeque<Step>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    text.lines()
        .filter_map(|l| parse(l).map(|s| s.map_err(|()| format!("{path}: a pad line I cannot read: {l}"))))
        .collect()
}

/// A pad as kept in memory: the held buttons and the four stick bytes.
#[derive(Clone, Copy)]
struct Kept {
    buttons: u32,
    sticks: [u8; 4],
}

/// The run so far, and the file it is being written to.
#[derive(Default)]
pub struct Record {
    steps: Vec<Result<Kept, String>>,
    /// The card's files at power-on, by their path under the card.
    card: Vec<(PathBuf, Vec<u8>)>,
    file: Option<(PathBuf, BufWriter<std::fs::File>)>,
}

impl Record {
    /// A record whose card is `dir` as it is now (power-on).
    pub fn new(dir: Option<&Path>) -> Self {
        let mut card = Vec::new();
        if let Some(d) = dir {
            read_tree(d, Path::new(""), &mut card);
        }
        Record { card, ..Record::default() }
    }

    pub fn writing(&self) -> Option<&Path> {
        self.file.as_ref().map(|(p, _)| p.as_path())
    }

    /// Start writing to `path`: `header` (the gamepads), every step kept
    /// so far, and the power-on card into `PATH.card`.
    pub fn start(&mut self, path: &Path, header: &str) -> Result<usize, String> {
        let err = |e: std::io::Error| format!("{}: {e}", path.display());
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        let mut w = BufWriter::new(std::fs::File::create(path).map_err(err)?);
        writeln!(w, "{header}").map_err(err)?;
        let mut frames = 0;
        for s in &self.steps {
            match s {
                Ok(k) => {
                    let [lx, ly, rx, ry] = k.sticks;
                    writeln!(w, "{frames} kept | bytes {lx} {ly} {rx} {ry} buttons {:04x} |", k.buttons)
                        .map_err(err)?;
                    frames += 1;
                }
                Err(c) => writeln!(w, "console {c}").map_err(err)?,
            }
        }
        let card = PathBuf::from(format!("{}.card", path.display()));
        if card.exists() {
            std::fs::remove_dir_all(&card).map_err(err)?;
        }
        for (rel, bytes) in &self.card {
            let to = card.join(rel);
            if let Some(d) = to.parent() {
                std::fs::create_dir_all(d).map_err(err)?;
            }
            std::fs::write(&to, bytes).map_err(err)?;
        }
        std::fs::create_dir_all(&card).map_err(err)?;
        w.flush().map_err(err)?;
        self.file = Some((path.to_path_buf(), w));
        Ok(frames)
    }

    /// Stop writing; the file's path.
    pub fn stop(&mut self) -> Option<PathBuf> {
        self.file.take().map(|(p, mut w)| {
            let _ = w.flush();
            p
        })
    }

    /// A frame's pad: kept, and written as `line` gives it when writing;
    /// the file flushed once a second, so it is whole while the game runs
    /// (and after a crash) up to the last second.
    pub fn pad(&mut self, raw: &Raw, line: impl FnOnce() -> String) {
        self.steps.push(Ok(Kept { buttons: raw.buttons.bits(), sticks: [raw.lx, raw.ly, raw.rx, raw.ry] }));
        let second = self.steps.len().is_multiple_of(FLUSH_EVERY);
        if let Some((_, w)) = &mut self.file {
            let _ = writeln!(w, "{}", line());
            if second {
                let _ = w.flush();
            }
        }
    }

    /// A console command run between frames, flushed at once.
    pub fn console(&mut self, line: &str) {
        self.steps.push(Err(line.to_string()));
        if let Some((_, w)) = &mut self.file {
            let _ = writeln!(w, "console {line}");
            let _ = w.flush();
        }
    }
}

fn read_tree(dir: &Path, rel: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let (path, name) = (e.path(), rel.join(e.file_name()));
        if path.is_dir() {
            read_tree(&path, &name, out);
        } else if let Ok(bytes) = std::fs::read(&path) {
            out.push((name, bytes));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kept steps written by `start` read back as the same steps, the
    /// commands in their places, and the card beside the log.
    #[test]
    fn a_kept_run_reads_back() {
        let dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../target/padlog-test-{}", std::process::id()));
        let card = dir.join("card");
        std::fs::create_dir_all(card.join("SAVE")).unwrap();
        std::fs::write(card.join("SAVE/slot"), b"abc").unwrap();
        let mut r = Record::new(Some(&card));
        let a = Raw { buttons: Buttons::CROSS, lx: 0, ..Raw::default() };
        r.pad(&a, String::new);
        r.console("god");
        let log = dir.join("run.log");
        assert_eq!(r.start(&log, "gamepads").unwrap(), 1);
        r.pad(&Raw::default(), || "1 live | bytes 128 128 128 128 buttons 0000 | x".into());
        r.stop();
        let steps: Vec<Step> = read(log.to_str().unwrap()).unwrap().into();
        assert_eq!(steps, [Step::Pad(a), Step::Console("god".into()), Step::Pad(Raw::default())]);
        assert_eq!(std::fs::read(dir.join("run.log.card/SAVE/slot")).unwrap(), b"abc");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// While the game still runs the file holds the run: the kept steps as
    /// soon as `pad_log` starts, a console command at once, the pads up to
    /// the last second. It had stayed empty until the game closed.
    #[test]
    fn the_log_is_on_disk_while_it_is_written() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../target/padlog-live-test-{}", std::process::id()));
        let mut r = Record::new(None);
        let a = Raw { buttons: Buttons::CROSS, ..Raw::default() };
        r.pad(&a, String::new);
        let log = dir.join("run.log");
        r.start(&log, "gamepads").unwrap();
        let on_disk = || read(log.to_str().unwrap()).unwrap().len();
        assert_eq!(on_disk(), 1, "the kept frame");
        r.console("god");
        assert_eq!(on_disk(), 2, "the command");
        let line = || "live | bytes 128 128 128 128 buttons 0000 |".to_string();
        while !r.steps.len().is_multiple_of(FLUSH_EVERY) {
            r.pad(&Raw::default(), line);
        }
        assert_eq!(on_disk(), FLUSH_EVERY, "a second of frames");
        drop(r);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
