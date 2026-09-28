//! The event script engine: [`ir`], our own instruction set, and [`text`], its
//! readable form, which round-trips; [`official`], the adapter that finds the
//! game's scripts and message tables through a disc's executable and converts
//! them to and from the IR without loss; [`vm`], the interpreter
//! (`ccEvent::CheckOpen` and `Execute`, `eventSub`, `ccEventFlagSet`, the task
//! `ccThEvent`, as Infection runs them); [`host`], what it asks of the rest of
//! the port; [`state`], the save record the scripts use, which the caller owns.
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
