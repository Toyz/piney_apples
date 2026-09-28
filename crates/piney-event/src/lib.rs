//! The event script engine.
//!
//! - [`ir`]: our own instruction set, a typed form of a script with no tie
//!   to the game's bytecode, and [`text`], its readable form, which parses
//!   back to exactly the same IR. A clean-room script set is written in it.
//! - [`official`]: the adapter for the game's own scripts. It reads them and
//!   their message tables from the boot executable on a disc image at run
//!   time, finding the tables through the game's code, and converts them to
//!   and from the IR without loss. Nothing read from a disc is kept here.
//! - [`vm`]: the interpreter, `ccEvent::CheckOpen` and `Execute`, `eventSub`,
//!   `ccEventFlagSet` and the event task `ccThEvent`, as Infection runs them.
//! - [`host`]: everything the interpreter asks of the rest of the port, as
//!   named methods, and [`host::LogHost`], which records them.
//! - [`state`]: the save record the scripts read and write, which the
//!   caller owns.
//!
//! The reference is `docs/engine/events.md` and `docs/engine/event-vm.md`.

pub mod extras;
pub mod host;
pub mod ir;
pub mod official;
pub mod state;
pub mod text;
pub mod vm;

pub use host::{Host, LogHost};
pub use state::{SaveData, ScriptSave};
pub use vm::{Library, Vm};
