//! The four discs' executables (extracted under `work/`) and a volume's
//! program with an overlay loaded, which a layout reads through.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::program::Program;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Vol {
    Inf,
    Mut,
    Out,
    Qua,
}

pub const VOLUMES: [Vol; 4] = [Vol::Inf, Vol::Mut, Vol::Out, Vol::Qua];

impl Vol {
    /// The manifest's tag: INF, MUT, OUT, QUA.
    pub fn tag(self) -> &'static str {
        match self {
            Vol::Inf => "INF",
            Vol::Mut => "MUT",
            Vol::Out => "OUT",
            Vol::Qua => "QUA",
        }
    }

    pub fn from_tag(t: &str) -> Option<Vol> {
        VOLUMES.into_iter().find(|v| v.tag() == t)
    }

    fn dir(self) -> &'static str {
        match self {
            Vol::Inf => "infection",
            Vol::Mut => "mutation",
            Vol::Out => "outbreak",
            Vol::Qua => "quarantine",
        }
    }

    pub fn exe(self) -> &'static str {
        match self {
            Vol::Inf => "SLUS_202.67",
            Vol::Mut => "SLUS_205.62",
            Vol::Out => "SLUS_205.63",
            Vol::Qua => "SLUS_205.64",
        }
    }

    /// The carry's file name: inf, mut_, out, qua.
    pub fn module(self) -> &'static str {
        match self {
            Vol::Inf => "inf",
            Vol::Mut => "mut_",
            Vol::Out => "out",
            Vol::Qua => "qua",
        }
    }

    /// `volumeNum - 1`.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// The repository's root (this crate is `crates/piney-gen`).
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the repository")
}

/// The tools' and checks' copy of a volume's built tables
/// (`piney_data::store::work_tables`).
pub fn work_tables(v: Vol) -> PathBuf {
    root().join("work").join("data").join(v.dir()).join("TABLES")
}

pub fn elf_path(v: Vol) -> PathBuf {
    root().join("work").join(v.dir()).join("disc").join(v.exe())
}

/// A volume's executable with an overlay (or none) loaded.
#[derive(Clone)]
pub struct Ctx {
    pub volume: Vol,
    pub overlay: Option<&'static str>,
    pub p: Rc<Program>,
}

type Programs = HashMap<(Vol, Option<&'static str>), Rc<Program>>;

thread_local! {
    static PROGRAMS: RefCell<Programs> = RefCell::new(HashMap::new());
}

pub fn ctx(volume: Vol, overlay: Option<&'static str>) -> Ctx {
    if let Some(p) = PROGRAMS.with(|m| m.borrow().get(&(volume, overlay)).cloned()) {
        return Ctx { volume, overlay, p };
    }
    let open = || -> Result<Program, String> {
        let exe = volume.exe();
        let elf = crate::elf::Elf::parse(crate::source::read(volume, exe)?, exe)?;
        Program::from_parts(elf, || crate::source::syms(volume), |path| crate::source::read(volume, path), overlay)
    };
    let p = Rc::new(open().unwrap_or_else(|e| crate::die(&format!("{}: {e}", volume.tag()))));
    PROGRAMS.with(|m| m.borrow_mut().insert((volume, overlay), p.clone()));
    Ctx { volume, overlay, p }
}

/// Drop the volume's programs read so far (their names changed).
pub fn forget(volume: Vol) {
    PROGRAMS.with(|m| m.borrow_mut().retain(|k, _| k.0 != volume));
}
