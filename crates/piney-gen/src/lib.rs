//! The table generator as a library: piney-gen's command (`main.rs`) and
//! piney-build use it. See `main.rs` for the commands.

pub mod carry;
pub mod data;
pub mod dtype;
pub mod dwarf;
pub mod elf;
pub mod layout;
pub mod locate;
pub mod manifest;
pub mod placement;
pub mod program;
pub mod race;
pub mod render;
pub mod sinit;
pub mod source;
pub mod syms;
pub mod talk;
pub mod text;
pub mod volume;
pub mod xfer;

/// A fault in the discs or the manifest: said, and the run ends.
pub fn die(msg: &str) -> ! {
    eprintln!("piney-gen: {msg}");
    std::process::exit(2)
}
