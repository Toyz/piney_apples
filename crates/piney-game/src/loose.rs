//! Not the game's: `--mode loose:PATH[:N]`, the scenes of an archive no stream
//! table lists, played for looking at them (Outbreak's `STREAM/STRT.BIN` holds
//! six, docs/disc/outbreak.md). The archive is split into streams
//! ([`piney_stream::load::loose_streams`]); stream N plays at 30 frames a
//! second, again from its start at its end; START (Enter) goes to the next.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_draw::Frame;
use piney_input::{Buttons, Pad};
use piney_stream::table::Entry;

use crate::mode::{Event, Mode};
use crate::stream::StreamPlayer;

pub struct LooseMode {
    iso: PathBuf,
    path: String,
    streams: Vec<Vec<Entry>>,
    n: usize,
    player: Option<StreamPlayer>,
    events: Vec<Event>,
    frame: u32,
    /// The stream has played through at least once.
    looped: bool,
}

impl LooseMode {
    /// The archive at `path` on the disc at `iso`, from its stream `n`.
    pub fn new(iso: &Path, path: &str, n: usize) -> Result<LooseMode, String> {
        let mut disc = Iso::open(iso).map_err(|e| format!("{}: {e}", iso.display()))?;
        let raw = disc.read_path(path).map_err(|e| format!("{path}: {e}"))?;
        let entries = piney_stream::load::scan(&raw).map_err(|e| format!("{path}: {e}"))?;
        let streams = piney_stream::load::loose_streams(entries);
        if streams.is_empty() {
            return Err(format!("{path}: no gzip members"));
        }
        for (k, s) in streams.iter().enumerate() {
            let names: Vec<&str> = s.iter().map(|e| e.name.as_str()).collect();
            println!("{path} stream {k}: {}", names.join(" + "));
        }
        let mut m = LooseMode {
            iso: iso.to_path_buf(),
            path: path.to_string(),
            n: n % streams.len(),
            streams,
            player: None,
            events: Vec::new(),
            frame: 0,
            looped: false,
        };
        m.start()?;
        Ok(m)
    }

    fn start(&mut self) -> Result<(), String> {
        let entries = self.streams[self.n].clone();
        self.player = Some(StreamPlayer::loose(&self.iso, &self.path, entries, &mut self.events)?);
        self.frame = 0;
        Ok(())
    }

    fn names(&self) -> String {
        self.streams[self.n].iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join(" + ")
    }
}

impl Mode for LooseMode {
    fn step(&mut self, pad: &Pad) -> Frame {
        if pad.push.contains(Buttons::START) {
            self.n = (self.n + 1) % self.streams.len();
            if let Err(e) = self.start() {
                eprintln!("{e}");
            }
        }
        let Some(p) = &mut self.player else { return Frame::new() };
        match p.step(pad, &mut self.events) {
            Some(f) => {
                self.frame += 1;
                f
            }
            None => {
                self.looped = true;
                if let Err(e) = self.start() {
                    eprintln!("{e}");
                }
                Frame::new()
            }
        }
    }

    fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    fn frame_rate(&self) -> u32 {
        2
    }

    fn archive(&self) -> Option<Arc<Archive>> {
        self.player.as_ref().map(StreamPlayer::archive)
    }

    fn ends(&self) -> bool {
        true
    }

    fn finished(&self) -> bool {
        self.looped
    }

    fn title(&self) -> String {
        format!("{} stream {} of {}: {} - frame {}", self.path, self.n, self.streams.len(), self.names(), self.frame)
    }
}
