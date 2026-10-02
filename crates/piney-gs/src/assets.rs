//! What frames refer to by name: `DATA.BIN` members, read once and kept.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

use piney_data::archive::Archive;
use piney_data::ccs::Ccs;
use piney_data::model::{self, Model};
use piney_data::scene::Scene;
use piney_data::texture::{self, Clut, Texture};
use piney_draw::{Rgba, Upload, UploadFormat};

/// One CCSF file, read.
pub struct File {
    pub ccs: Ccs,
    pub scene: Scene,
    /// MDL_ object -> model.
    pub models: HashMap<u32, Model>,
    pub textures: Vec<Texture>,
    pub cluts: HashMap<u32, Clut>,
}

impl File {
    pub fn texture(&self, object: u32) -> Option<&Texture> {
        self.textures.iter().find(|t| t.object == object)
    }
}

/// An image ready to upload: RGBA8, rows in stored order, alpha as
/// `piney_data::texture` scales it (0x80 -> 255).
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// `DATA.BIN`'s members by stem, read on first use.
pub struct Assets {
    archive: Arc<Archive>,
    /// Files looked up before the archive's: a stream's own, while it
    /// plays.
    overlay: Option<Arc<Archive>>,
    files: HashMap<String, Option<Arc<File>>>,
    /// The overlay's files, read on a thread since it was set: a stream's
    /// models are first drawn well into it (Skeith's in stream 5 at frame
    /// 1834), and reading their file then held that frame 135 ms.
    warm: Option<Receiver<(String, Arc<File>)>>,
}

/// A member's bytes read as a file.
fn read_file(bytes: Vec<u8>) -> piney_data::Result<File> {
    let ccs = Ccs::parse(bytes)?;
    let scene = Scene::read(&ccs)?;
    let models = model::models(&ccs)?.into_iter().map(|m| (m.object, m)).collect();
    let (textures, cluts) = texture::read(&ccs)?;
    Ok(File { ccs, scene, models, textures, cluts })
}

/// Every member of `archive` read in order on a thread, sent as it is
/// read; the thread stops when the receiver is gone. Members that do not
/// read are left to [`Assets::file`], which says so.
fn warm(archive: Arc<Archive>) -> Receiver<(String, Arc<File>)> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        for m in archive.members() {
            let stem = m.stem();
            let Ok(f) = archive.inflate_named(&stem).and_then(read_file) else { continue };
            if tx.send((stem, Arc::new(f))).is_err() {
                break;
            }
        }
    });
    rx
}

impl Assets {
    pub fn new(archive: Arc<Archive>) -> Self {
        Assets { archive, overlay: None, files: HashMap::new(), warm: None }
    }

    pub fn archive(&self) -> &Arc<Archive> {
        &self.archive
    }

    /// Another disc's `DATA.BIN` (the launcher's choice): every file read
    /// so far is dropped, and the overlay with them.
    pub fn set_archive(&mut self, archive: Arc<Archive>) {
        self.archive = archive;
        self.overlay = None;
        self.warm = None;
        self.files.clear();
    }

    /// Look members up in `overlay` first; true when that changed what is
    /// looked up (and every file read so far is dropped). The overlay's
    /// files start reading on a thread.
    pub fn set_overlay(&mut self, overlay: Option<Arc<Archive>>) -> bool {
        let same = match (&self.overlay, &overlay) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        if !same {
            self.warm = overlay.clone().map(warm);
            self.overlay = overlay;
            self.files.clear();
        }
        !same
    }

    /// The member `stem`, or None when it is missing or does not read.
    pub fn file(&mut self, stem: &str) -> Option<Arc<File>> {
        if let Some(rx) = &self.warm {
            while let Ok((name, f)) = rx.try_recv() {
                self.files.entry(name).or_insert(Some(f));
            }
        }
        if let Some(f) = self.files.get(stem) {
            return f.clone();
        }
        let file = (|| -> piney_data::Result<File> {
            let bytes = match self.overlay.as_ref().map(|o| o.inflate_named(stem)) {
                Some(Ok(b)) => b,
                _ => self.archive.inflate_named(stem)?,
            };
            read_file(bytes)
        })();
        let file = match file {
            Ok(f) => Some(Arc::new(f)),
            Err(e) => {
                tracing::warn!("{stem}: {e}");
                None
            }
        };
        self.files.insert(stem.to_string(), file.clone());
        file
    }

    /// Texture `texture` of `stem` through palette `clut`, level 0.
    pub fn image(&mut self, stem: &str, texture: u32, clut: u32) -> Option<Image> {
        let f = self.file(stem)?;
        let t = f.texture(texture)?;
        let c = f.cluts.get(&clut).or_else(|| f.cluts.get(&t.clut))?;
        let rgba = t.rgba(c, 0).ok()?;
        Some(Image { width: t.width(0), height: t.height(0), rgba })
    }
}

/// A run-time texture to RGBA, its palette's alpha scaled as the file
/// textures' are.
pub fn upload_image(u: &Upload) -> Image {
    let palette: Vec<[u8; 4]> =
        u.clut.iter().map(|&Rgba([r, g, b, a])| [r, g, b, (u16::from(a) * 2).min(255) as u8]).collect();
    let (w, h) = (u32::from(u.width), u32::from(u.height));
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    if u.format == UploadFormat::Psmct32 {
        for i in 0..(w * h) as usize {
            let t: [u8; 4] = u.pixels.get(4 * i..4 * i + 4).and_then(|t| t.try_into().ok()).unwrap_or([0; 4]);
            rgba.extend_from_slice(&[t[0], t[1], t[2], (u16::from(t[3]) * 2).min(255) as u8]);
        }
        return Image { width: w, height: h, rgba };
    }
    for i in 0..(w * h) as usize {
        let index = match u.format {
            UploadFormat::Psmt8 => u.pixels.get(i).copied().unwrap_or(0),
            UploadFormat::Psmt4 => u.pixels.get(i / 2).map_or(0, |b| if i % 2 == 0 { b & 0xf } else { b >> 4 }),
            UploadFormat::Psmct32 => unreachable!(),
        };
        rgba.extend_from_slice(&palette.get(index as usize).copied().unwrap_or([0; 4]));
    }
    Image { width: w, height: h, rgba }
}
