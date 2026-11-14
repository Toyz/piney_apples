//! The World outside the towns: a field (`WORLD`, area 1) with its weather
//! and fireflies, and [`world::world::World`], which runs a field or its
//! dungeon.

pub mod ambient;
#[doc(alias = "WORLD")]
pub mod area;
pub mod def;
#[doc(alias = "FIREFLY")]
pub mod firefly;
