//! The World outside the towns: a field (`WORLD`, area 1) with its weather
//! and fireflies and the NPCs an event puts there, and [`world::FieldWorld`],
//! which runs a field or its dungeon.

pub mod ambient;
#[doc(alias = "WORLD")]
pub mod area;
#[doc(alias = "FIREFLY")]
pub mod firefly;
pub mod npcs;
pub mod world;
